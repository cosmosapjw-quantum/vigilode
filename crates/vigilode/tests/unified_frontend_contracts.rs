//! V03 contracts of the unified frontend (research loop L1, node V03 part A).
//!
//! Part 1 (this section): reproductions of the `PeerOde` defects of audit
//! findings COD-02 and COD-04, built from the audit probe
//! (`research/production_loop_20261116/audit/findings_code.json`, cases
//! `p1`..`p7`). They use only the public API that exists at the base commit,
//! so the same assertions run before and after the fix.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use serde_json::json;
use vigilode::{
    CallbackError, CapabilitySet, Jacobian, MethodAdmission, PeerOde, PeerSolveError,
    PeerSolveOptions, Problem,
};

// ------------------------------------------------------------------ helpers

fn dense_caps() -> CapabilitySet {
    CapabilitySet {
        explicit_jacobian: true,
        dense_matrix: true,
        newton: true,
        ..CapabilitySet::NONE
    }
}

fn jvp_caps() -> CapabilitySet {
    CapabilitySet {
        provided_jvp: true,
        dense_matrix: true,
        newton: true,
        ..CapabilitySet::NONE
    }
}

/// FNV-1a over the little-endian bytes of 64-bit words.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
    fn word(&mut self, word: u64) {
        for byte in word.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn floats(&mut self, values: &[f64]) {
        self.word(values.len() as u64);
        for value in values {
            self.word(value.to_bits());
        }
    }
    fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

fn trajectory_digest(t: &[f64], y: &[Vec<f64>]) -> String {
    let mut digest = Fnv::new();
    digest.floats(t);
    digest.word(y.len() as u64);
    for row in y {
        digest.floats(row);
    }
    digest.hex()
}

/// One tolerance unit of the scalar problems below.
fn tolerance_unit(options: &PeerSolveOptions, exact: f64) -> f64 {
    options.atol + options.rtol * exact.abs()
}

// ------------------------------------------- PeerOde defects (COD-02, COD-04)

/// COD-04 (a), probe case p1: a JVP provider must not be lowered when the
/// caller did not permit provided JVPs.
#[test]
fn cod04a_p1_jvp_provider_needs_the_jvp_permission() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (rhs_calls, jvp_calls) = (calls.clone(), calls.clone());
    let problem = Problem::new(2, move |_t, y, d| {
        rhs_calls.fetch_add(1, Ordering::SeqCst);
        d[0] = -y[0];
        d[1] = -2.0 * y[1];
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_t, _y, v, out| {
        jvp_calls.fetch_add(1, Ordering::SeqCst);
        out[0] = -v[0];
        out[1] = -2.0 * v[1];
        Ok(())
    }))
    .autonomous();
    let admission = PeerOde::admission(&problem, dense_caps());
    let result = PeerOde::solve(
        &problem,
        (0.0, 1.0),
        &[1.0, 1.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    );
    let user_callbacks = calls.load(Ordering::SeqCst);
    println!(
        "QUAL {}",
        json!({"test": "cod04a_p1", "data": {
            "admission": format!("{admission:?}"),
            "solve_is_err": result.is_err(),
            "error": result.as_ref().err().map(|e| format!("{e:?}")),
            "user_callbacks": user_callbacks,
        }})
    );
    assert!(
        matches!(admission, MethodAdmission::Refused { .. }),
        "a JVP provider without the provided_jvp permission was admitted: {admission:?}"
    );
    assert!(
        matches!(result, Err(PeerSolveError::Refused(_))),
        "solve did not refuse: {:?}",
        result.as_ref().map(|s| s.success)
    );
    assert_eq!(user_callbacks, 0, "user callbacks before the refusal");
    // the same problem with the JVP permission is admitted and runs
    let admitted = PeerOde::solve(
        &problem,
        (0.0, 1.0),
        &[1.0, 1.0],
        &PeerSolveOptions::default(),
        jvp_caps(),
    )
    .expect("admitted with the JVP permission");
    assert!(admitted.success, "{:?}", admitted.failure);
}

/// COD-04 (b), probe case p2a: when the starter end point `t0 + 2 h0` rounds
/// above `t_end`, the run must not end with `success == false` and no reason.
#[test]
fn cod04b_p2a_starter_endpoint_above_t_end_is_a_success_with_exact_endpoint() {
    let problem = Problem::new(1, |_t, _y, d| {
        d[0] = 1.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, m| {
        m[0] = 0.0;
        Ok(())
    }))
    .autonomous();
    let mut rows = Vec::new();
    for span in [(0.3, 0.9), (0.2, 0.9), (0.0, 1.0)] {
        let s = PeerOde::solve(
            &problem,
            span,
            &[0.0],
            &PeerSolveOptions::default(),
            dense_caps(),
        )
        .expect("the run starts");
        rows.push(json!({
            "span": [span.0, span.1],
            "success": s.success,
            "failure": s.failure,
            "t_last": s.t.last().map(|t| format!("{t:?}")),
            "y_last": s.y.last().map(|y| y[0]),
        }));
        assert!(
            s.success || s.failure.is_some(),
            "span {span:?}: success == false without a reason"
        );
        assert!(s.success, "span {span:?}: {:?}", s.failure);
        let t_last = *s.t.last().expect("an output");
        assert_eq!(
            t_last.to_bits(),
            span.1.to_bits(),
            "span {span:?}: the last output time {t_last:?} is not t_end"
        );
        let y_last = s.y.last().expect("an output")[0];
        assert!(
            (y_last - (span.1 - span.0)).abs() <= 1e-12,
            "span {span:?}: y_end {y_last}"
        );
    }
    println!("QUAL {}", json!({"test": "cod04b_p2", "data": rows}));
}

fn forcing_case(name: &str, omega: f64) {
    let problem = Problem::new(1, move |t, _y, d| {
        d[0] = omega * (omega * t).cos();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, m| {
        m[0] = 0.0;
        Ok(())
    }));
    let options = PeerSolveOptions {
        rtol: 1e-6,
        atol: 1e-8,
        ..PeerSolveOptions::default()
    };
    let s = PeerOde::solve(&problem, (0.0, 1.0), &[0.0], &options, dense_caps())
        .expect("the run starts");
    let exact = omega.sin();
    let error = s.y.last().map(|y| (y[0] - exact).abs());
    let units = error.map(|e| e / tolerance_unit(&options, exact));
    println!(
        "QUAL {}",
        json!({"test": name, "data": {
            "omega": omega,
            "success": s.success,
            "failure": s.failure,
            "h0": s.h0,
            "y_end": s.y.last().map(|y| y[0]),
            "exact": exact,
            "abs_error": error,
            "error_in_tolerance_units": units,
            "accepted_peer_steps": s.step_work.accepted_steps,
            "rejected_peer_steps": s.step_work.rejected_steps,
            "startup_rhs_calls": s.startup_work.rhs_calls,
            "startup_jacobian_builds": s.startup_work.jacobian_builds,
        }})
    );
    if s.success {
        let units = units.expect("a successful run has an output");
        assert!(
            units <= 1000.0,
            "{name}: success with an error of {units:.3e} tolerance units"
        );
    } else {
        assert!(s.failure.is_some(), "{name}: failure without a reason");
    }
}

/// COD-02, probe case p3: the starter interval is error controlled.
#[test]
fn cod02_p3_fast_forcing_20_is_error_controlled_or_refused() {
    forcing_case("cod02_p3", 20.0);
}

/// COD-02, probe case p3b.
#[test]
fn cod02_p3b_fast_forcing_200_is_error_controlled_or_refused() {
    forcing_case("cod02_p3b", 200.0);
}

/// COD-02, probe cases p5a, p5b, p5c: a caller-chosen first step that is not
/// finite and positive is an input error, found before any callback.
#[test]
fn cod02_p5_invalid_h0_is_refused_before_any_callback() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut rows = Vec::new();
    let mut wrong = Vec::new();
    for (name, h0) in [
        ("negative", -1.0),
        ("nan", f64::NAN),
        ("zero", 0.0),
        ("infinite", f64::INFINITY),
    ] {
        let (rhs_calls, jac_calls) = (calls.clone(), calls.clone());
        let problem = Problem::new(1, move |_t, y, d| {
            rhs_calls.fetch_add(1, Ordering::SeqCst);
            d[0] = -y[0];
            Ok(())
        })
        .jacobian(Jacobian::dense(move |_t, _y, m| {
            jac_calls.fetch_add(1, Ordering::SeqCst);
            m[0] = -1.0;
            Ok(())
        }))
        .autonomous();
        let options = PeerSolveOptions {
            h0: Some(h0),
            ..PeerSolveOptions::default()
        };
        let before = calls.load(Ordering::SeqCst);
        let result = PeerOde::solve(&problem, (0.0, 1.0), &[1.0], &options, dense_caps());
        let entered = calls.load(Ordering::SeqCst) - before;
        let outcome = match &result {
            Ok(s) => format!(
                "Ok(success={}, failure={:?}, h0={:?})",
                s.success, s.failure, s.h0
            ),
            Err(e) => format!("Err({e:?})"),
        };
        rows.push(json!({"h0": name, "outcome": outcome, "user_callbacks": entered}));
        if !matches!(result, Err(PeerSolveError::InvalidInput(_))) || entered != 0 {
            wrong.push(format!("h0 {name}: {outcome}, {entered} user callbacks"));
        }
    }
    println!("QUAL {}", json!({"test": "cod02_p5", "data": rows}));
    assert!(wrong.is_empty(), "invalid h0 was not refused: {wrong:#?}");
}

/// COD-04 (c), probe case p4: a recoverable Jacobian error is reported as
/// itself, not replaced by the step-size underflow that follows 52 halvings
/// at the same Jacobian point.
#[test]
fn cod04c_p4_recoverable_jacobian_error_is_reported_as_itself() {
    let jacobian_calls = Arc::new(AtomicUsize::new(0));
    let counter = jacobian_calls.clone();
    let problem = Problem::new(1, |_t, y, d| {
        d[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, m| {
        if counter.fetch_add(1, Ordering::SeqCst) >= 2 {
            return Err(CallbackError::Recoverable("user says shrink".into()));
        }
        m[0] = -1.0;
        Ok(())
    }))
    .autonomous();
    let s = PeerOde::solve(
        &problem,
        (0.0, 10.0),
        &[1.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    )
    .expect("the run starts");
    let calls = jacobian_calls.load(Ordering::SeqCst);
    println!(
        "QUAL {}",
        json!({"test": "cod04c_p4", "data": {
            "success": s.success,
            "failure": s.failure,
            "user_jacobian_callbacks": calls,
            "rejected_peer_steps": s.step_work.rejected_steps,
            "accepted_peer_steps": s.step_work.accepted_steps,
        }})
    );
    assert!(!s.success);
    let failure = s.failure.clone().expect("a failed run has a reason");
    assert!(
        failure.contains("user says shrink"),
        "the reason does not carry the callback's message: {failure}"
    );
    assert!(
        calls <= 6,
        "{calls} Jacobian callbacks: the step was halved at an unchanged Jacobian point"
    );
}

/// COD-04 (c, d), probe case p6: a fatal right-hand-side error is reported as
/// a fatal callback error, not as invalid input.
#[test]
fn cod04cd_p6_fatal_rhs_error_is_reported_as_a_fatal_callback() {
    let rhs_calls = Arc::new(AtomicUsize::new(0));
    let counter = rhs_calls.clone();
    let problem = Problem::new(1, move |_t, y, d| {
        if counter.fetch_add(1, Ordering::SeqCst) >= 40 {
            return Err(CallbackError::Fatal("user fatal".into()));
        }
        d[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, m| {
        m[0] = -1.0;
        Ok(())
    }))
    .autonomous();
    let s = PeerOde::solve(
        &problem,
        (0.0, 10.0),
        &[1.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    )
    .expect("the run starts");
    println!(
        "QUAL {}",
        json!({"test": "cod04cd_p6", "data": {
            "success": s.success,
            "failure": s.failure,
            "n_outputs": s.t.len(),
            "user_rhs_callbacks": rhs_calls.load(Ordering::SeqCst),
        }})
    );
    assert!(!s.success);
    let failure = s.failure.clone().expect("a failed run has a reason");
    assert!(failure.contains("user fatal"), "{failure}");
    assert!(
        !failure.contains("InvalidInput"),
        "a fatal callback error is reported as invalid input: {failure}"
    );
    assert!(s.y.iter().flatten().all(|v| v.is_finite()));
}

/// COD-04 (e), probe case p7: a declared band is refused by `admission` too,
/// not admitted there and refused afterwards by the lowering.
#[test]
fn cod04e_p7_declared_band_is_refused_by_admission_too() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (rhs_calls, jac_calls) = (calls.clone(), calls.clone());
    let problem = Problem::new(3, move |_t, y, d| {
        rhs_calls.fetch_add(1, Ordering::SeqCst);
        d.copy_from_slice(y);
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, m| {
        jac_calls.fetch_add(1, Ordering::SeqCst);
        m.fill(0.0);
        Ok(())
    }))
    .banded(1, 1)
    .autonomous();
    let admission = PeerOde::admission(&problem, dense_caps());
    let result = PeerOde::solve(
        &problem,
        (0.0, 1.0),
        &[1.0, 1.0, 1.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    );
    println!(
        "QUAL {}",
        json!({"test": "cod04e_p7", "data": {
            "admission": format!("{admission:?}"),
            "solve": format!("{:?}", result.as_ref().err()),
            "user_callbacks": calls.load(Ordering::SeqCst),
        }})
    );
    assert!(result.is_err(), "a declared band was solved");
    assert!(
        matches!(admission, MethodAdmission::Refused { .. }),
        "admission says Admitted while solve refuses: {admission:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

// ----------------------------- baseline digests of the U07 contract problem

const PR_L: [[f64; 2]; 2] = [[-2.0, 30.0], [0.0, -400.0]];

fn pr_phi(t: f64) -> [f64; 2] {
    [t.sin() + 1.0, (2.0 * t).cos()]
}

fn pr_dphi(t: f64) -> [f64; 2] {
    [t.cos(), -2.0 * (2.0 * t).sin()]
}

fn pr_rhs(t: f64, y: &[f64], dy: &mut [f64]) {
    let (p, d) = (pr_phi(t), pr_dphi(t));
    for i in 0..2 {
        dy[i] = d[i] + PR_L[i][0] * (y[0] - p[0]) + PR_L[i][1] * (y[1] - p[1]);
    }
}

/// Nonnormal stiff Prothero-Robinson system of the U07 contracts.
fn pr_problem(jacobian_kind: &str) -> Problem {
    let p = Problem::new(2, |t, y, dy| {
        pr_rhs(t, y, dy);
        Ok(())
    });
    match jacobian_kind {
        "dense" => p.jacobian(Jacobian::dense(|_t, _y, j| {
            j.copy_from_slice(&[PR_L[0][0], PR_L[0][1], PR_L[1][0], PR_L[1][1]]);
            Ok(())
        })),
        "jvp" => p.jacobian(Jacobian::jvp(|_t, _y, v, o| {
            o[0] = PR_L[0][0] * v[0] + PR_L[0][1] * v[1];
            o[1] = PR_L[1][0] * v[0] + PR_L[1][1] * v[1];
            Ok(())
        })),
        other => panic!("unknown Jacobian kind {other}"),
    }
}

/// Exploratory regression record (PREDECLARED.json, `regression_check_exploratory`):
/// digests of the `PeerOde` trajectories of the four U07 cases. The receipt
/// script compares the lines printed on the base sources with the lines
/// printed after the fix.
#[test]
fn peer_baseline_digests_of_the_u07_cases() {
    let mut rows = Vec::new();
    for (kind, allowed) in [("dense", dense_caps()), ("jvp", jvp_caps())] {
        for threads in [1usize, 4] {
            let options = PeerSolveOptions {
                rtol: 1e-8,
                atol: 1e-10,
                h0_rule_h_max: 0.01,
                threads,
                ..PeerSolveOptions::default()
            };
            let s = PeerOde::solve(&pr_problem(kind), (0.0, 1.0), &pr_phi(0.0), &options, allowed)
                .expect("the U07 case runs");
            assert!(s.success, "{kind}/{threads}: {:?}", s.failure);
            rows.push(json!({
                "jacobian": kind,
                "threads": threads,
                "h0_bits": format!("{:016x}", s.h0.to_bits()),
                "n_outputs": s.t.len(),
                "trajectory_fnv1a64": trajectory_digest(&s.t, &s.y),
                "accepted": s.step_work.accepted_steps,
                "rejected": s.step_work.rejected_steps,
                "step_rhs_calls": s.step_work.rhs_calls,
                "step_jacobian_builds": s.step_work.jacobian_builds,
                "step_jvp_calls": s.step_work.jvp_calls,
                "startup_rhs_calls": s.startup_work.rhs_calls,
                "startup_jacobian_builds": s.startup_work.jacobian_builds,
                "startup_jvp_calls": s.startup_work.jvp_calls,
            }));
        }
    }
    println!("QUAL {}", json!({"test": "peer_baseline_digests", "data": rows}));
}
