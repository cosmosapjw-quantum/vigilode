//! V03 contracts of the unified frontend (research loop L1, node V03 part A).
//!
//! Part 1: reproductions of the `PeerOde` defects of audit findings COD-02
//! and COD-04, built from the audit probe
//! (`research/production_loop_20261116/audit/findings_code.json`, cases
//! `p1`..`p7`). They use only the public API that exists at the base commit,
//! so the same assertions run before and after the fix. Part 1b adds the
//! typed assertions that the fix makes possible.
//!
//! Part 2: the frozen discriminators of node V03
//! (`research/production_loop_20261116/unified_frontend/PREDECLARED.json`):
//! D1 decision equality of `route(General)` and `GeneralOde::select` on the
//! exhaustive golden grid with zero callbacks, D2 bitwise equality of every
//! adapter with its direct call, D3 refusal before any callback. D4 (the
//! default-path golden) is the unchanged test target `default_path_golden`.
//!
//! Every test prints `QUAL <json>` lines; the receipt script parses them.

use std::{
    collections::BTreeMap,
    f64::consts::PI,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BdfConfig, Dopri5StopReason, FusedPhiKrylovConfig, OdeProblem,
    OutputSchedule, ParallelExecution, RadauConfig, Rodas5pLaggedOptions,
    integrate_bdf_adaptive_observed, integrate_pexprb54s4_fused_adaptive_observed,
    integrate_radau_adaptive_observed, integrate_rodas5p_lagged_observed, ppsw1,
};
use serde_json::{Value, json};
use vigilode::{
    Atol, Callback, CallbackError, CapabilitySet, DampedRotationBlock, DampedRotationFlow,
    DerivativeFallback, Direction, Dopri5, FacadeRoute, FailureReason, GeneralOde,
    GeneralOdePolicy, GeneralResult, GeneralSelection, InteriorError, Jacobian, JacobianForm,
    LawsonDopri5, METHOD_ID_LAWSON, METHOD_ID_PEXPRB, METHOD_ID_RODAS5P_LAGGED, MethodAdmission,
    MethodAutoPolicy, MethodAvailability, MethodChoice, MethodChoiceKind, MethodCostEvidence,
    MethodFailure, MethodFallback, MethodOutputGrid, MethodParams, MethodPhase,
    MethodQualification, MethodRefusalReason, MethodRegistry, MethodResult, MethodRouteRefusal,
    MethodSolveError, Options, PeerFailure, PeerOde, PeerPhase, PeerSolveError, PeerSolveOptions,
    Problem, ProblemRegime, ProductionMethod, RegistryRefusal, Rodas5p, SelectionRefusal,
    Unsupported, WorkRecord,
    coloring::SparsityPattern,
    selection_evidence::{
        CostEvidence, CostEvidenceKind, CostInterval, CostUnit, SelectionIdentity,
    },
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
            let s = PeerOde::solve(
                &pr_problem(kind),
                (0.0, 1.0),
                &pr_phi(0.0),
                &options,
                allowed,
            )
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
    println!(
        "QUAL {}",
        json!({"test": "peer_baseline_digests", "data": rows})
    );
}

// --------------------------------------- part 1b: typed assertions of the fix

fn decay_problem_failing_after(
    t_fail: f64,
    jacobian_error: Option<CallbackError>,
    rhs_error: Option<CallbackError>,
    failing_calls: Arc<AtomicUsize>,
) -> Problem {
    let (rhs_failures, jacobian_failures) = (failing_calls.clone(), failing_calls);
    Problem::new(1, move |t, y, d| {
        if t > t_fail
            && let Some(error) = &rhs_error
        {
            rhs_failures.fetch_add(1, Ordering::SeqCst);
            return Err(error.clone());
        }
        d[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _y, m| {
        if t > t_fail
            && let Some(error) = &jacobian_error
        {
            jacobian_failures.fetch_add(1, Ordering::SeqCst);
            return Err(error.clone());
        }
        m[0] = -1.0;
        Ok(())
    }))
    .autonomous()
}

/// COD-04 (c): a recoverable Jacobian error on a Peer step stops the run with
/// that reason after one failing call; the accepted prefix ends at the
/// linearization point.
#[test]
fn peer_recoverable_jacobian_error_on_a_peer_step_is_typed_and_stops() {
    let failing_calls = Arc::new(AtomicUsize::new(0));
    // the starter covers at most [0, 2]: every Jacobian call with t > 5 is a Peer step
    let problem = decay_problem_failing_after(
        5.0,
        Some(CallbackError::Recoverable("user says shrink".into())),
        None,
        failing_calls.clone(),
    );
    let s = PeerOde::solve(
        &problem,
        (0.0, 10.0),
        &[1.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    )
    .expect("the run starts");
    assert!(!s.success);
    let reason = s.reason.clone().expect("a failed run has a typed reason");
    match &reason {
        PeerFailure::Callback {
            phase: PeerPhase::Step,
            callback: Callback::Jacobian,
            recoverable: true,
            t,
            message,
        } => {
            assert!(*t > 5.0, "t = {t}");
            assert_eq!(message, "user says shrink");
            assert_eq!(
                s.t.last().copied(),
                Some(*t),
                "the prefix ends at the Jacobian point"
            );
        }
        other => panic!("unexpected reason {other:?}"),
    }
    assert_eq!(s.failure.as_deref(), Some(reason.to_string().as_str()));
    assert_eq!(
        failing_calls.load(Ordering::SeqCst),
        1,
        "the step is not retried at the unchanged Jacobian point"
    );
    assert!(s.step_work.accepted_steps > 0 && s.y.iter().flatten().all(|v| v.is_finite()));
    println!(
        "QUAL {}",
        json!({"test": "peer_typed_recoverable_jacobian", "data": {
            "reason": format!("{reason:?}"),
            "failing_jacobian_calls": failing_calls.load(Ordering::SeqCst),
            "accepted": s.step_work.accepted_steps,
            "rejected": s.step_work.rejected_steps,
        }})
    );
}

/// COD-04 (c, d): a fatal right-hand-side error on a Peer step is a typed fatal
/// callback failure with the accepted prefix and the charged work.
#[test]
fn peer_fatal_rhs_error_on_a_peer_step_is_typed_with_prefix() {
    let failing_calls = Arc::new(AtomicUsize::new(0));
    let problem = decay_problem_failing_after(
        5.0,
        None,
        Some(CallbackError::Fatal("user fatal".into())),
        failing_calls.clone(),
    );
    let s = PeerOde::solve(
        &problem,
        (0.0, 10.0),
        &[1.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    )
    .expect("the run starts");
    assert!(!s.success);
    let reason = s.reason.clone().expect("a failed run has a typed reason");
    assert!(
        matches!(
            &reason,
            PeerFailure::Callback {
                phase: PeerPhase::Step,
                callback: Callback::Rhs,
                recoverable: false,
                message,
                ..
            } if message == "user fatal"
        ),
        "{reason:?}"
    );
    assert_eq!(
        failing_calls.load(Ordering::SeqCst),
        1,
        "no call after the fatal one"
    );
    assert!(!s.t.is_empty() && *s.t.last().unwrap() <= 5.0);
    assert!(
        s.step_work.rhs_calls > 6 * s.step_work.accepted_steps,
        "failed attempt charged"
    );
    println!(
        "QUAL {}",
        json!({"test": "peer_typed_fatal_rhs", "data": {
            "reason": format!("{reason:?}"),
            "last_t": s.t.last(),
            "accepted": s.step_work.accepted_steps,
        }})
    );
}

/// COD-02: the starter record, a starter that covers the span, and the typed
/// failure of a starter whose estimate never passes.
#[test]
fn peer_starter_record_and_typed_starter_failure() {
    // zero Jacobian, fast forcing: the first step 0.5 is halved until the estimate passes
    let forced = Problem::new(1, |t, _y, d| {
        d[0] = 20.0 * (20.0 * t).cos();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, m| {
        m[0] = 0.0;
        Ok(())
    }));
    let s = PeerOde::solve(
        &forced,
        (0.0, 1.0),
        &[0.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    )
    .unwrap();
    assert!(s.success);
    assert_eq!(s.starter.h0_requested, 0.5);
    assert!(s.starter.halvings > 0);
    assert_eq!(s.h0, 0.5 / f64::from(1u32 << s.starter.halvings));
    assert!(s.starter.estimate <= 1.0 && !s.starter.covers_span);
    // 1 Jacobian for nothing (zero-Jacobian rule), then 1 + 2 (halvings + 1) starter solves
    assert_eq!(
        s.startup_work.jacobian_builds,
        1 + 1 + 2 * (s.starter.halvings as u64 + 1)
    );
    let halvings = s.starter.halvings;

    // a smooth problem whose starter is accepted at once and covers the whole span
    let smooth = Problem::new(1, |_t, _y, d| {
        d[0] = 1.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, m| {
        m[0] = 0.0;
        Ok(())
    }))
    .autonomous();
    let s = PeerOde::solve(
        &smooth,
        (0.3, 0.9),
        &[0.0],
        &PeerSolveOptions::default(),
        dense_caps(),
    )
    .unwrap();
    assert!(s.success && s.starter.covers_span && s.starter.halvings == 0);
    assert_eq!((s.t.len(), s.step_work.accepted_steps), (1, 0));

    // a zero tolerance scale (atol = 0 at y = 0) leaves the estimate undefined: typed failure
    let zero = Problem::new(1, |_t, y, d| {
        d[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, m| {
        m[0] = -1.0;
        Ok(())
    }))
    .autonomous();
    let options = PeerSolveOptions {
        atol: 0.0,
        ..PeerSolveOptions::default()
    };
    let s = PeerOde::solve(&zero, (0.0, 1.0), &[0.0], &options, dense_caps()).unwrap();
    assert!(!s.success && s.t.is_empty());
    assert!(
        matches!(
            s.reason,
            Some(PeerFailure::StarterErrorAboveTolerance { halvings: 40, estimate, .. })
                if estimate.is_nan()
        ),
        "{:?}",
        s.reason
    );
    println!(
        "QUAL {}",
        json!({"test": "peer_starter_record", "data": {
            "forcing_halvings": halvings,
            "zero_scale_reason": format!("{:?}", s.reason),
            "zero_scale_startup_jacobian_builds": s.startup_work.jacobian_builds,
        }})
    );
}

// =============================================================================
// Part 2: the entry point (MethodChoice, MethodRegistry::route / solve, view)
// =============================================================================

/// Sorted-key JSON without whitespace: the identity of a decision.
fn canonical(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("a key serializes"),
                        canonical(&map[*key])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        scalar => serde_json::to_string(scalar).expect("a scalar serializes"),
    }
}

fn outcome(result: &Result<GeneralSelection, SelectionRefusal>) -> String {
    canonical(&match result {
        Ok(selection) => json!({ "selected": serde_json::to_value(selection).unwrap() }),
        Err(refusal) => json!({ "refused": serde_json::to_value(refusal).unwrap() }),
    })
}

// ------------------------------------------------------------------ the grid
// The enumeration of `default_path_golden.rs::decision_grid`, in its order,
// plus two sparse-JVP providers for the extension grid.

const GRID_DIMENSIONS: [usize; 4] = [8, 64, 65, 500];
const GRID_DENSE_CEILINGS: [usize; 2] = [64, 1000];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GridJacobian {
    Dense,
    Banded,
    Jvp,
    FiniteDifference,
    SparseBanded,
    SparseGeneral,
}

const GOLDEN_JACOBIANS: [GridJacobian; 4] = [
    GridJacobian::Dense,
    GridJacobian::Banded,
    GridJacobian::Jvp,
    GridJacobian::FiniteDifference,
];
const EXTENSION_JACOBIANS: [GridJacobian; 6] = [
    GridJacobian::Dense,
    GridJacobian::Banded,
    GridJacobian::Jvp,
    GridJacobian::FiniteDifference,
    GridJacobian::SparseBanded,
    GridJacobian::SparseGeneral,
];

#[derive(Clone, Copy, Debug)]
struct Features {
    jacobian: GridJacobian,
    autonomous: bool,
    time_derivative: bool,
    mass: bool,
    events: bool,
    band: bool,
}

impl Features {
    fn all(jacobians: &[GridJacobian]) -> Vec<Self> {
        let mut all = Vec::new();
        for &jacobian in jacobians {
            for autonomous in [false, true] {
                for time_derivative in [false, true] {
                    for mass in [false, true] {
                        for events in [false, true] {
                            for band in [false, true] {
                                all.push(Self {
                                    jacobian,
                                    autonomous,
                                    time_derivative,
                                    mass,
                                    events,
                                    band,
                                });
                            }
                        }
                    }
                }
            }
        }
        all
    }

    fn key(&self) -> String {
        let jacobian = match self.jacobian {
            GridJacobian::Dense => "dense",
            GridJacobian::Banded => "banded",
            GridJacobian::Jvp => "jvp",
            GridJacobian::FiniteDifference => "finite-difference",
            GridJacobian::SparseBanded => "sparse-jvp-banded",
            GridJacobian::SparseGeneral => "sparse-jvp-general",
        };
        format!(
            "jacobian={jacobian} autonomous={} time_derivative={} mass={} events={} declared_band={}",
            u8::from(self.autonomous),
            u8::from(self.time_derivative),
            u8::from(self.mass),
            u8::from(self.events),
            u8::from(self.band),
        )
    }

    /// The problem with these features. Every callback counts its entries.
    fn problem(&self, n: usize, entries: &Arc<AtomicUsize>) -> Problem {
        let count = |entries: &Arc<AtomicUsize>| {
            let entries = entries.clone();
            move || {
                entries.fetch_add(1, Ordering::Relaxed);
            }
        };
        let rhs = count(entries);
        let mut problem = Problem::new(n, move |_t, _y, _dy| {
            rhs();
            Ok(())
        });
        let tridiagonal = |far: bool| {
            let rows: Vec<Vec<usize>> = (0..n)
                .map(|i| {
                    let mut row: Vec<usize> = (i.saturating_sub(1)..=(i + 1).min(n - 1)).collect();
                    if far && i == 0 && !row.contains(&(n - 1)) {
                        row.push(n - 1);
                    }
                    row
                })
                .collect();
            SparsityPattern::from_rows(n, &rows).expect("a valid pattern")
        };
        problem = match self.jacobian {
            GridJacobian::Dense => {
                let entered = count(entries);
                problem.jacobian(Jacobian::dense(move |_t, _y, _jac| {
                    entered();
                    Ok(())
                }))
            }
            GridJacobian::Banded => {
                let entered = count(entries);
                problem.jacobian(Jacobian::banded(1, 1, move |_t, _y, _band| {
                    entered();
                    Ok(())
                }))
            }
            GridJacobian::Jvp => {
                let entered = count(entries);
                problem.jacobian(Jacobian::jvp(move |_t, _y, _v, _out| {
                    entered();
                    Ok(())
                }))
            }
            GridJacobian::FiniteDifference => problem.jacobian(Jacobian::FiniteDifference),
            GridJacobian::SparseBanded | GridJacobian::SparseGeneral => {
                let entered = count(entries);
                let pattern = tridiagonal(self.jacobian == GridJacobian::SparseGeneral);
                assert_eq!(
                    pattern.is_banded(),
                    self.jacobian == GridJacobian::SparseBanded
                );
                problem.jacobian(Jacobian::sparse_jvp(pattern, move |_t, _y, _v, _out| {
                    entered();
                    Ok(())
                }))
            }
        };
        if self.autonomous {
            problem = problem.autonomous();
        }
        if self.time_derivative {
            let entered = count(entries);
            problem = problem.time_derivative(move |_t, _y, _out| {
                entered();
                Ok(())
            });
        }
        if self.mass {
            let mut identity = vec![0.0; n * n];
            for index in 0..n {
                identity[index * n + index] = 1.0;
            }
            problem = problem.mass(identity);
        }
        if self.events {
            let entered = count(entries);
            problem = problem.event(
                move |_t, _y| {
                    entered();
                    0.0
                },
                Direction::Either,
            );
        }
        if self.band {
            problem = problem.banded(1, 1);
        }
        problem
    }
}

#[derive(Clone, Copy)]
struct GridOptions {
    dense_output: bool,
    interior_error: InteriorError,
    vector_atol: bool,
}

impl GridOptions {
    fn all() -> Vec<Self> {
        let mut all = Vec::new();
        for dense_output in [false, true] {
            for interior_error in [
                InteriorError::WFilteredEnforce,
                InteriorError::Enforce,
                InteriorError::Off,
            ] {
                for vector_atol in [false, true] {
                    all.push(Self {
                        dense_output,
                        interior_error,
                        vector_atol,
                    });
                }
            }
        }
        all
    }

    fn key(&self) -> String {
        let interior = match self.interior_error {
            InteriorError::WFilteredEnforce => "w-filtered-enforce",
            InteriorError::Enforce => "enforce",
            InteriorError::Off => "off",
            _ => unreachable!("the grid enumerates the three known modes"),
        };
        format!(
            "dense_output={} interior_error={interior} atol={}",
            u8::from(self.dense_output),
            if self.vector_atol { "vector" } else { "scalar" },
        )
    }

    fn options(&self, n: usize) -> Options {
        Options {
            atol: if self.vector_atol {
                Atol::Vector(vec![1.0e-9; n])
            } else {
                Atol::Scalar(1.0e-9)
            },
            dense_output: self.dense_output,
            interior_error: self.interior_error,
            ..Options::default()
        }
    }
}

fn grid_policies() -> Vec<GeneralOdePolicy> {
    let mut all = Vec::new();
    for regime in [
        ProblemRegime::Unknown,
        ProblemRegime::Nonstiff,
        ProblemRegime::Stiff,
    ] {
        for derivatives in [
            DerivativeFallback::Forbidden,
            DerivativeFallback::ProvidedOnly,
            DerivativeFallback::AllowFiniteDifference,
        ] {
            for allow_matrix_free in [false, true] {
                for max_dense_dimension in GRID_DENSE_CEILINGS {
                    all.push(GeneralOdePolicy {
                        regime,
                        derivatives,
                        max_dense_dimension,
                        allow_matrix_free,
                    });
                }
            }
        }
    }
    all
}

/// The `select` table of the golden file, decoded: features key -> options
/// key -> one canonical outcome per column, and the entry count it declares.
fn golden_select_table() -> (BTreeMap<String, BTreeMap<String, Vec<String>>>, usize) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/default_path/decision_grid.json");
    let text = std::fs::read_to_string(&path).expect("the golden decision grid is readable");
    let document: Value = serde_json::from_str(&text).expect("the golden file is JSON");
    let table = &document["select"];
    let width = table["code_width"].as_u64().expect("code width") as usize;
    let legend = table["legend"].as_object().expect("legend");
    let mut rows = BTreeMap::new();
    for (features, by_options) in table["rows"].as_object().expect("rows") {
        let mut decoded = BTreeMap::new();
        for (options, cells) in by_options.as_object().expect("option rows") {
            let outcomes: Vec<String> = cells
                .as_str()
                .expect("a code string")
                .as_bytes()
                .chunks(width)
                .map(|code| canonical(&legend[std::str::from_utf8(code).expect("ascii codes")]))
                .collect();
            decoded.insert(options.clone(), outcomes);
        }
        rows.insert(features.clone(), decoded);
    }
    (
        rows,
        table["n_entries"].as_u64().expect("entry count") as usize,
    )
}

/// The decision of `route(General)` in the shape of `GeneralOde::select`.
fn general_via_route(
    problem: &Problem,
    options: &Options,
    policy: &GeneralOdePolicy,
) -> Result<GeneralSelection, SelectionRefusal> {
    match MethodRegistry::route(problem, options, &MethodChoice::General(*policy)) {
        Ok(plan) => {
            let selection = plan.general.expect("a general plan carries its selection");
            // the plan's route is the selection's method and facade route
            match selection.method {
                ProductionMethod::Dopri5 => {
                    assert_eq!(plan.route.id(), "dopri5/rhs-only");
                    assert_eq!(plan.route.facade_route, None);
                }
                ProductionMethod::Rodas5p => {
                    assert_eq!(plan.route.method_id, "rodas5p");
                    assert_eq!(plan.route.facade_route, selection.implicit_route);
                }
            }
            assert_eq!(plan.choice, MethodChoiceKind::General);
            assert!(plan.fallbacks.is_empty() && plan.ranking.is_empty());
            Ok(selection)
        }
        Err(MethodRouteRefusal::General(refusal)) => Err(refusal),
        Err(other) => panic!("a general choice returned the refusal {other:?}"),
    }
}

/// D1 (frozen): `route(General)` equals `GeneralOde::select` on every entry of
/// the golden decision grid, both equal the golden file, and no callback is
/// entered.
#[test]
fn d1_route_general_equals_select_on_the_golden_decision_grid() {
    let entries = Arc::new(AtomicUsize::new(0));
    let (golden, golden_entries) = golden_select_table();
    let policies = grid_policies();
    let grid_options = GridOptions::all();
    let (mut compared, mut equal_select, mut equal_golden) = (0usize, 0usize, 0usize);
    let mut equal_reference = 0usize;
    let mut differing = Vec::new();
    for feature in Features::all(&GOLDEN_JACOBIANS) {
        for (dimension_index, &n) in GRID_DIMENSIONS.iter().enumerate() {
            let problem = feature.problem(n, &entries);
            for grid_option in &grid_options {
                let options = grid_option.options(n);
                let golden_row = &golden[&feature.key()][&grid_option.key()];
                for (policy_index, policy) in policies.iter().enumerate() {
                    let by_select = outcome(&GeneralOde::select(&problem, &options, policy));
                    let by_route = outcome(&general_via_route(&problem, &options, policy));
                    let by_golden = &golden_row[dimension_index * policies.len() + policy_index];
                    // the body `select` had before phase B, kept as the oracle
                    let by_reference =
                        outcome(&GeneralOde::select_reference(&problem, &options, policy));
                    compared += 1;
                    equal_select += usize::from(by_route == by_select);
                    equal_golden += usize::from(&by_route == by_golden);
                    equal_reference += usize::from(by_route == by_reference);
                    if (by_route != by_select || &by_route != by_golden) && differing.len() < 20 {
                        differing.push(format!(
                            "{} | {} | n={n} | policy {policy_index}: route {by_route} / select \
                             {by_select} / golden {by_golden}",
                            feature.key(),
                            grid_option.key()
                        ));
                    }
                }
            }
        }
    }
    let callbacks = entries.load(Ordering::SeqCst);
    println!(
        "QUAL {}",
        json!({"test": "d1_decision_grid", "data": {
            "entries": compared,
            "golden_file_entries": golden_entries,
            "equal_to_select": equal_select,
            "equal_to_golden_file": equal_golden,
            "equal_to_reference_body": equal_reference,
            "callback_entries": callbacks,
            "differing_examples": differing,
        }})
    );
    assert_eq!(compared, 221_184);
    assert_eq!(golden_entries, 221_184);
    assert_eq!(equal_select, compared, "{differing:#?}");
    assert_eq!(equal_golden, compared, "{differing:#?}");
    assert_eq!(equal_reference, compared, "{differing:#?}");
    assert_eq!(callbacks, 0, "a routing query entered a callback");
}

/// D1, phase B: the catalog-derived `GeneralOde::select` equals the retained
/// reference body on every combination of the facts either of them reads (the
/// golden grid plus the two sparse-JVP provider forms the golden grid does not
/// enumerate), with zero callbacks.
#[test]
fn d1_phase_b_select_equals_the_reference_body_on_the_extension_grid() {
    let entries = Arc::new(AtomicUsize::new(0));
    let policies = grid_policies();
    let (mut compared, mut equal_select, mut equal_derived) = (0usize, 0usize, 0usize);
    let mut differing = Vec::new();
    let mut outcomes = BTreeMap::<String, usize>::new();
    for feature in Features::all(&EXTENSION_JACOBIANS) {
        for &n in &GRID_DIMENSIONS {
            let problem = feature.problem(n, &entries);
            for grid_option in GridOptions::all() {
                let options = grid_option.options(n);
                for (policy_index, policy) in policies.iter().enumerate() {
                    let reference =
                        outcome(&GeneralOde::select_reference(&problem, &options, policy));
                    let selected = outcome(&GeneralOde::select(&problem, &options, policy));
                    let derived = outcome(&MethodRegistry::general_selection(
                        &problem, &options, policy,
                    ));
                    compared += 1;
                    equal_select += usize::from(selected == reference);
                    equal_derived += usize::from(derived == reference);
                    if selected != reference && differing.len() < 20 {
                        differing.push(format!(
                            "{} | {} | n={n} | policy {policy_index}: select {selected} / \
                             reference {reference}",
                            feature.key(),
                            grid_option.key()
                        ));
                    }
                    *outcomes.entry(reference).or_default() += 1;
                }
            }
        }
    }
    // the catalog names exactly one selectable method per regime
    for (regime, id) in [
        (ProblemRegime::Unknown, "dopri5"),
        (ProblemRegime::Nonstiff, "dopri5"),
        (ProblemRegime::Stiff, "rodas5p"),
    ] {
        assert_eq!(
            MethodRegistry::general_method(regime).map(|e| e.descriptor.method_id),
            Some(id)
        );
    }
    let callbacks = entries.load(Ordering::SeqCst);
    println!(
        "QUAL {}",
        json!({"test": "d1_phase_b_extension_grid", "data": {
            "entries": compared,
            "select_equal_to_reference": equal_select,
            "general_selection_equal_to_reference": equal_derived,
            "distinct_outcomes": outcomes.len(),
            "outcome_counts": outcomes,
            "callback_entries": callbacks,
            "differing_examples": differing,
        }})
    );
    assert_eq!(compared, 6 * 32 * 12 * 4 * 36);
    assert_eq!(compared, 331_776);
    assert_eq!(equal_select, compared, "{differing:#?}");
    assert_eq!(equal_derived, compared);
    assert_eq!(callbacks, 0, "a selection query entered a callback");
}

fn full_permission() -> CapabilitySet {
    CapabilitySet {
        provided_jvp: true,
        finite_difference: true,
        automatic_differentiation: true,
        explicit_jacobian: true,
        dense_matrix: true,
        newton: true,
    }
}

fn lawson_flow(damping: f64, frequency: f64) -> DampedRotationFlow {
    DampedRotationFlow::repeated(DampedRotationBlock { damping, frequency }, 1).unwrap()
}

/// D3 (frozen): routing never enters user code, for every choice kind, every
/// registered method id and every grid problem (extension providers included).
/// The explicit RODAS5P admission is `Rodas5p::capabilities` on every entry.
#[test]
fn d3_route_is_pure_for_every_choice_over_the_extension_grid() {
    let entries = Arc::new(AtomicUsize::new(0));
    let ids: Vec<&'static str> = MethodRegistry::catalog()
        .iter()
        .map(|e| e.descriptor.method_id)
        .collect();
    let with_flow = MethodParams {
        lawson_flow: Some(lawson_flow(0.1, 3.0)),
        ..MethodParams::default()
    };
    let mut auto_policies = Vec::new();
    for regime in [
        ProblemRegime::Unknown,
        ProblemRegime::Nonstiff,
        ProblemRegime::Stiff,
    ] {
        for allowed in [CapabilitySet::NONE, full_permission()] {
            for admit_unqualified in [false, true] {
                for fallback in [
                    MethodFallback::Never,
                    MethodFallback::NextAdmitted { max_attempts: 3 },
                ] {
                    auto_policies.push(MethodAutoPolicy {
                        regime,
                        allowed,
                        admit_unqualified,
                        fallback,
                        use_measured_evidence: false,
                        evidence: &[],
                    });
                }
            }
        }
    }
    let rodas5p_variants = MethodRegistry::entry("rodas5p")
        .expect("the RODAS5P entry")
        .route_variants;
    let (mut routed, mut plans, mut refusals) = (0usize, 0usize, 0usize);
    let (mut rodas_checked, mut rodas_equal, mut rodas_query_defects) = (0usize, 0usize, 0usize);
    let mut tally = |result: &Result<vigilode::MethodRoutePlan, MethodRouteRefusal>| {
        routed += 1;
        match result {
            Ok(_) => plans += 1,
            Err(_) => refusals += 1,
        }
    };
    for feature in Features::all(&EXTENSION_JACOBIANS) {
        for &n in &GRID_DIMENSIONS {
            let problem = feature.problem(n, &entries);
            for grid_option in GridOptions::all() {
                let options = grid_option.options(n);
                for id in ids.iter().copied().chain(["no-such-method"]) {
                    for params in [&MethodParams::default(), &with_flow] {
                        let choice = MethodChoice::Explicit(id);
                        let result =
                            MethodRegistry::route_with(&problem, &options, &choice, params);
                        tally(&result);
                        if id == "rodas5p" {
                            // the explicit RODAS5P admission is the facade capability query
                            rodas_checked += 1;
                            let same = match (&result, Rodas5p::capabilities(&problem, &options)) {
                                (Ok(plan), Ok(capabilities)) => {
                                    // Independent statement of what `Rodas5p::solve` runs: the
                                    // route of the capability query, except that a general
                                    // sparse-JVP request leaves the router with dense output
                                    // (fast dense driver) or a vector atol (dense fast driver).
                                    let sparse_general =
                                        feature.jacobian == GridJacobian::SparseGeneral;
                                    let executed = match capabilities.route {
                                        FacadeRoute::MatrixFree
                                            if sparse_general && grid_option.dense_output =>
                                        {
                                            FacadeRoute::FastDense
                                        }
                                        FacadeRoute::MatrixFree
                                            if sparse_general && grid_option.vector_atol =>
                                        {
                                            FacadeRoute::DenseFast
                                        }
                                        queried => queried,
                                    };
                                    rodas_query_defects +=
                                        usize::from(executed != capabilities.route);
                                    // the catalog row of the planned route is that driver
                                    // family and supports what the request asks for
                                    let row = rodas5p_variants
                                        .iter()
                                        .find(|v| v.facade_route == executed)
                                        .expect("one row per facade route");
                                    let dense_provider = feature.jacobian == GridJacobian::Dense;
                                    plan.route.facade_route == Some(executed)
                                        && plan.route.variant == row.label
                                        && plan.route.needs.dense_matrix
                                            == (row.dense_matrix || dense_provider)
                                        && (!feature.mass || row.supports_mass)
                                        && (!feature.events || row.supports_events)
                                        && (!grid_option.dense_output || row.supports_dense_output)
                                }
                                (Err(MethodRouteRefusal::Method { reasons, .. }), Err(refusal)) => {
                                    reasons == &[MethodRefusalReason::Implicit(refusal)]
                                }
                                _ => false,
                            };
                            rodas_equal += usize::from(same);
                        }
                    }
                }
                for policy in &auto_policies {
                    let result =
                        MethodRegistry::route(&problem, &options, &MethodChoice::Auto(*policy));
                    tally(&result);
                }
            }
        }
    }
    let callbacks = entries.load(Ordering::SeqCst);
    println!(
        "QUAL {}",
        json!({"test": "d3_route_purity", "data": {
            "route_calls": routed,
            "plans": plans,
            "refusals": refusals,
            "method_ids": ids.len(),
            "auto_policies": auto_policies.len(),
            "rodas5p_admissions_compared_with_capabilities": rodas_checked,
            "rodas5p_admissions_equal_to_capabilities": rodas_equal,
            "rodas5p_capability_query_names_another_driver_than_runs": rodas_query_defects,
            "callback_entries": callbacks,
        }})
    );
    assert_eq!(
        routed,
        192 * 4 * 12 * (2 * (ids.len() + 1) + auto_policies.len())
    );
    assert_eq!(rodas_equal, rodas_checked);
    // general sparse-JVP requests whose capability query names a driver that does not run:
    // dense output on a fast route (16 feature rows * 4 dimensions * 4 option rows) and a
    // vector atol without dense output (8 * 4 * 3), each with 2 parameter sets
    assert_eq!(rodas_query_defects, (16 * 4 * 4 + 8 * 4 * 3) * 2);
    assert!(plans > 0 && refusals > 0);
    assert_eq!(callbacks, 0, "a routing query entered a callback");
}

// ------------------------------------------------------------------ fixtures
// Plain functions, so that the facade problem and the direct driver problem
// of a test evaluate the same code.

type RhsPointer = fn(f64, &[f64], &mut [f64]);
type JvpPointer = fn(f64, &[f64], &[f64], &mut [f64]);

#[derive(Clone)]
struct Fixture {
    name: &'static str,
    n: usize,
    rhs: RhsPointer,
    dense: Option<RhsPointer>,
    jvp: Option<JvpPointer>,
    autonomous: bool,
    y0: Vec<f64>,
    span: (f64, f64),
}

impl Fixture {
    /// The facade problem, with every callback counting its entries.
    fn facade(&self, entries: &Arc<AtomicUsize>) -> Problem {
        let (rhs, counter) = (self.rhs, entries.clone());
        let mut problem = Problem::new(self.n, move |t, y, dy| {
            counter.fetch_add(1, Ordering::Relaxed);
            rhs(t, y, dy);
            Ok(())
        });
        if let Some(fill) = self.dense {
            let counter = entries.clone();
            problem = problem.jacobian(Jacobian::dense(move |t, y, jac| {
                counter.fetch_add(1, Ordering::Relaxed);
                fill(t, y, jac);
                Ok(())
            }));
        }
        if let Some(product) = self.jvp {
            let counter = entries.clone();
            problem = problem.jacobian(Jacobian::jvp(move |t, y, v, out| {
                counter.fetch_add(1, Ordering::Relaxed);
                product(t, y, v, out);
                Ok(())
            }));
        }
        if self.autonomous {
            problem = problem.autonomous();
        }
        problem
    }

    /// The driver-level problem a direct caller of the integrator crate builds.
    fn direct(&self) -> OdeProblem {
        type CoreRhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
        type CoreJacobian = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
        type CoreJvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
        let (n, rhs) = (self.n, self.rhs);
        let rhs: CoreRhs = Arc::new(move |t, y, out| {
            rhs(t, y, out);
            Ok(())
        });
        let jacobian = self.dense.map(|fill| {
            let jacobian: CoreJacobian = Arc::new(move |t, y| {
                let mut matrix = DenseMatrix::zeros(n, n);
                fill(t, y, matrix.as_mut_slice());
                Ok(matrix)
            });
            jacobian
        });
        let jvp = self.jvp.map(|product| {
            let jvp: CoreJvp = Arc::new(move |t, y, v, out| {
                product(t, y, v, out);
                Ok(())
            });
            jvp
        });
        OdeProblem::new(
            "direct",
            n,
            rhs,
            None,
            jacobian,
            jvp,
            None,
            self.autonomous,
            None,
            None,
        )
        .expect("a valid driver problem")
    }
}

fn robertson_rhs(_t: f64, y: &[f64], dy: &mut [f64]) {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
}

fn robertson_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) {
    jac.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        0.0,
        6.0e7 * y[1],
        0.0,
    ]);
}

fn robertson() -> Fixture {
    Fixture {
        name: "robertson",
        n: 3,
        rhs: robertson_rhs,
        dense: Some(robertson_jacobian),
        jvp: None,
        autonomous: true,
        y0: vec![1.0, 0.0, 0.0],
        span: (0.0, 40.0),
    }
}

fn robertson_dae_rhs(_t: f64, y: &[f64], dy: &mut [f64]) {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = y[0] + y[1] + y[2] - 1.0;
}

fn robertson_dae_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) {
    jac.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        1.0,
        1.0,
        1.0,
    ]);
}

const VDP_MU: f64 = 10.0;

fn vanderpol_rhs(_t: f64, y: &[f64], dy: &mut [f64]) {
    dy[0] = y[1];
    dy[1] = VDP_MU * (1.0 - y[0] * y[0]) * y[1] - y[0];
}

fn vanderpol_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) {
    jac.copy_from_slice(&[
        0.0,
        1.0,
        -2.0 * VDP_MU * y[0] * y[1] - 1.0,
        VDP_MU * (1.0 - y[0] * y[0]),
    ]);
}

fn vanderpol() -> Fixture {
    Fixture {
        name: "vanderpol",
        n: 2,
        rhs: vanderpol_rhs,
        dense: Some(vanderpol_jacobian),
        jvp: None,
        autonomous: true,
        y0: vec![2.0, 0.0],
        span: (0.0, 5.0),
    }
}

fn pr_rhs_pointer(t: f64, y: &[f64], dy: &mut [f64]) {
    pr_rhs(t, y, dy);
}

fn pr_jacobian(_t: f64, _y: &[f64], jac: &mut [f64]) {
    jac.copy_from_slice(&[PR_L[0][0], PR_L[0][1], PR_L[1][0], PR_L[1][1]]);
}

fn pr_jvp(_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]) {
    out[0] = PR_L[0][0] * v[0] + PR_L[0][1] * v[1];
    out[1] = PR_L[1][0] * v[0] + PR_L[1][1] * v[1];
}

fn prothero_robinson(jvp: bool) -> Fixture {
    Fixture {
        name: if jvp {
            "prothero_robinson_nonnormal_jvp"
        } else {
            "prothero_robinson_nonnormal_dense"
        },
        n: 2,
        rhs: pr_rhs_pointer,
        dense: (!jvp).then_some(pr_jacobian as RhsPointer),
        jvp: jvp.then_some(pr_jvp as JvpPointer),
        autonomous: false,
        y0: pr_phi(0.0).to_vec(),
        span: (0.0, 1.0),
    }
}

const BRUSS_CELLS: usize = 20;

fn bruss_alpha() -> f64 {
    (1.0 / 50.0) * ((BRUSS_CELLS + 1) as f64).powi(2)
}

fn bruss_rhs(_t: f64, y: &[f64], dy: &mut [f64]) {
    let alpha = bruss_alpha();
    for i in 0..BRUSS_CELLS {
        let (u, v) = (y[2 * i], y[2 * i + 1]);
        let (u_left, v_left) = if i == 0 {
            (1.0, 3.0)
        } else {
            (y[2 * i - 2], y[2 * i - 1])
        };
        let (u_right, v_right) = if i + 1 == BRUSS_CELLS {
            (1.0, 3.0)
        } else {
            (y[2 * i + 2], y[2 * i + 3])
        };
        dy[2 * i] = 1.0 + u * u * v - 4.0 * u + alpha * (u_left - 2.0 * u + u_right);
        dy[2 * i + 1] = 3.0 * u - u * u * v + alpha * (v_left - 2.0 * v + v_right);
    }
}

fn bruss_entries(y: &[f64], mut set: impl FnMut(usize, usize, f64)) {
    let alpha = bruss_alpha();
    for i in 0..BRUSS_CELLS {
        let (u, v) = (y[2 * i], y[2 * i + 1]);
        let (p, q) = (2 * i, 2 * i + 1);
        set(p, p, 2.0 * u * v - 4.0 - 2.0 * alpha);
        set(p, q, u * u);
        set(q, p, 3.0 - 2.0 * u * v);
        set(q, q, -u * u - 2.0 * alpha);
        if i > 0 {
            set(p, p - 2, alpha);
            set(q, q - 2, alpha);
        }
        if i + 1 < BRUSS_CELLS {
            set(p, p + 2, alpha);
            set(q, q + 2, alpha);
        }
    }
}

fn bruss_dense(_t: f64, y: &[f64], jac: &mut [f64]) {
    let n = 2 * BRUSS_CELLS;
    bruss_entries(y, |i, j, value| jac[i * n + j] = value);
}

fn bruss_band(_t: f64, y: &[f64], band: &mut [f64]) {
    // lower = upper = 2: band[i * 5 + (j + 2 - i)]
    bruss_entries(y, |i, j, value| band[i * 5 + (j + 2 - i)] = value);
}

fn brusselator() -> Fixture {
    Fixture {
        name: "brusselator_1d",
        n: 2 * BRUSS_CELLS,
        rhs: bruss_rhs,
        dense: Some(bruss_dense),
        jvp: None,
        autonomous: true,
        y0: (0..BRUSS_CELLS)
            .flat_map(|i| {
                let x = (i + 1) as f64 / (BRUSS_CELLS + 1) as f64;
                [1.0 + (2.0 * PI * x).sin(), 3.0]
            })
            .collect(),
        span: (0.0, 2.0),
    }
}

const HEAT_N: usize = 16;

fn heat_diffusion() -> f64 {
    1.0e-2 * ((HEAT_N + 1) as f64).powi(2)
}

fn heat_rhs(_t: f64, y: &[f64], dy: &mut [f64]) {
    let d = heat_diffusion();
    for i in 0..HEAT_N {
        let left = if i == 0 { 0.0 } else { y[i - 1] };
        let right = if i + 1 == HEAT_N { 0.0 } else { y[i + 1] };
        dy[i] = d * (left - 2.0 * y[i] + right) + y[i] * (1.0 - y[i]);
    }
}

fn heat_jvp(_t: f64, y: &[f64], v: &[f64], out: &mut [f64]) {
    let d = heat_diffusion();
    for i in 0..HEAT_N {
        let left = if i == 0 { 0.0 } else { v[i - 1] };
        let right = if i + 1 == HEAT_N { 0.0 } else { v[i + 1] };
        out[i] = d * (left - 2.0 * v[i] + right) + (1.0 - 2.0 * y[i]) * v[i];
    }
}

fn heat_reaction() -> Fixture {
    Fixture {
        name: "heat_reaction_jvp",
        n: HEAT_N,
        rhs: heat_rhs,
        dense: None,
        jvp: Some(heat_jvp),
        autonomous: true,
        y0: (0..HEAT_N)
            .map(|i| (PI * (i + 1) as f64 / (HEAT_N + 1) as f64).sin())
            .collect(),
        span: (0.0, 1.0),
    }
}

fn lotka_volterra_rhs(_t: f64, y: &[f64], dy: &mut [f64]) {
    dy[0] = 1.5 * y[0] - y[0] * y[1];
    dy[1] = -3.0 * y[1] + y[0] * y[1];
}

fn lotka_volterra() -> Fixture {
    Fixture {
        name: "lotka_volterra",
        n: 2,
        rhs: lotka_volterra_rhs,
        dense: None,
        jvp: None,
        autonomous: true,
        y0: vec![1.0, 1.0],
        span: (0.0, 10.0),
    }
}

fn tolerances(rtol: f64, atol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        ..Options::default()
    }
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn row_bits(rows: &[Vec<f64>]) -> Vec<Vec<u64>> {
    rows.iter().map(|row| bits(row)).collect()
}

fn hex_state(values: &[f64]) -> Vec<String> {
    values
        .iter()
        .map(|v| format!("{:016x}", v.to_bits()))
        .collect()
}

fn explicit_plan(problem: &Problem, options: &Options, id: &str) -> vigilode::MethodRoutePlan {
    MethodRegistry::route(problem, options, &MethodChoice::Explicit(id))
        .unwrap_or_else(|refusal| panic!("{id}: {refusal}"))
}

// ----------------------------------------------- D2: adapters, bit for bit

/// One RODAS5P facade route through the registry against `Rodas5p::solve`.
fn rodas5p_case(
    name: &str,
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    options: &Options,
    expected: FacadeRoute,
) -> Value {
    let plan = explicit_plan(problem, options, "rodas5p");
    assert_eq!(plan.route.facade_route, Some(expected), "{name}");
    assert_eq!(plan.route.qualification, MethodQualification::Production);
    let unified = MethodRegistry::solve(problem, span, y0, options, &plan).expect(name);
    let direct = Rodas5p::solve(problem, span, y0, options).expect(name);
    let view = unified.view();
    let MethodResult::Rodas5p(native) = view.native() else {
        panic!("{name}: not a RODAS5P result");
    };
    assert_eq!(bits(view.times()), bits(&direct.t), "{name}: times");
    assert_eq!(
        row_bits(view.states()),
        row_bits(&direct.y),
        "{name}: states"
    );
    assert_eq!(
        serde_json::to_string(native).unwrap(),
        serde_json::to_string(&direct).unwrap(),
        "{name}: the whole run record"
    );
    assert_eq!(view.is_success(), direct.is_success(), "{name}");
    assert!(view.is_success() && view.failure().is_none(), "{name}");
    assert_eq!(view.work(), direct.stats.counters, "{name}: work counters");
    assert_eq!(
        view.steps(),
        (
            direct.stats.accepted_steps as u64,
            direct.stats.rejected_steps as u64
        )
    );
    assert_eq!(view.executed_driver(), direct.stats.route.driver_id);
    // the route read from the run record is the planned one
    assert_eq!(view.route(), plan.route, "{name}: executed route");
    assert_eq!(view.planned_route(), &plan.route);
    assert_eq!(unified.attempts().len(), 1);
    if options.dense_output {
        assert_eq!(view.output_grid(), MethodOutputGrid::AcceptedSteps);
        for fraction in [0.1, 0.5, 0.9] {
            let t = span.0 + fraction * (span.1 - span.0);
            assert_eq!(
                bits(&view.at(t).expect("inside the span")),
                bits(&direct.at(t).expect("inside the span")),
                "{name}: continuous extension at {t}"
            );
        }
    } else {
        assert_eq!(view.output_grid(), MethodOutputGrid::Endpoints);
        assert_eq!(view.times(), &[span.0, span.1]);
    }
    json!({
        "case": name,
        "route": view.route_id(),
        "driver": view.executed_driver(),
        "entry_point": native.stats.route.entry_point,
        "accepted": direct.stats.accepted_steps,
        "rejected": direct.stats.rejected_steps,
        "rhs_calls": direct.stats.counters.rhs_calls,
        "jacobian_builds": direct.stats.counters.jacobian_builds,
        "n_outputs": direct.t.len(),
        "final_state_bits": hex_state(direct.final_state().unwrap()),
        "bitwise_equal": true,
    })
}

/// D2 (frozen): the RODAS5P adapter is `Rodas5p::solve` bit for bit on every
/// facade route.
#[test]
fn d2_rodas5p_adapter_is_bitwise_the_direct_call_on_every_facade_route() {
    let entries = Arc::new(AtomicUsize::new(0));
    let options = tolerances(1e-6, 1e-9);
    let dense_output = Options {
        dense_output: true,
        ..options.clone()
    };
    let robertson = robertson();
    let bruss = brusselator();
    let heat = heat_reaction();
    let banded_provider = || {
        Problem::new(bruss.n, |t, y, dy| {
            bruss_rhs(t, y, dy);
            Ok(())
        })
        .jacobian(Jacobian::banded(2, 2, |t, y, band| {
            bruss_band(t, y, band);
            Ok(())
        }))
        .autonomous()
    };
    let finite_difference_band = Problem::new(bruss.n, |t, y, dy| {
        bruss_rhs(t, y, dy);
        Ok(())
    })
    .banded(2, 2)
    .autonomous();
    let dae = Problem::new(3, |t, y, dy| {
        robertson_dae_rhs(t, y, dy);
        Ok(())
    })
    .jacobian(Jacobian::dense(|t, y, jac| {
        robertson_dae_jacobian(t, y, jac);
        Ok(())
    }))
    .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
    .autonomous();
    let rows = vec![
        rodas5p_case(
            "robertson/dense-fast",
            &robertson.facade(&entries),
            robertson.span,
            &robertson.y0,
            &options,
            FacadeRoute::DenseFast,
        ),
        rodas5p_case(
            "brusselator_1d/banded-provider/banded-fast",
            &banded_provider(),
            bruss.span,
            &bruss.y0,
            &options,
            FacadeRoute::BandedFast,
        ),
        rodas5p_case(
            "brusselator_1d/finite-difference-band/banded-fast",
            &finite_difference_band,
            bruss.span,
            &bruss.y0,
            &options,
            FacadeRoute::BandedFast,
        ),
        rodas5p_case(
            "heat_reaction_jvp/matrix-free",
            &heat.facade(&entries),
            heat.span,
            &heat.y0,
            &options,
            FacadeRoute::MatrixFree,
        ),
        rodas5p_case(
            "robertson/dense-output/fast-dense",
            &robertson.facade(&entries),
            robertson.span,
            &robertson.y0,
            &dense_output,
            FacadeRoute::FastDense,
        ),
        rodas5p_case(
            "brusselator_1d/banded-provider/dense-output/fast-banded-dense",
            &banded_provider(),
            bruss.span,
            &bruss.y0,
            &dense_output,
            FacadeRoute::FastBandedDense,
        ),
        rodas5p_case(
            "robertson_dae/mass/sequential-dense",
            &dae,
            (0.0, 40.0),
            &[1.0, 0.0, 0.0],
            &options,
            FacadeRoute::SequentialDense,
        ),
    ];
    let routes: std::collections::BTreeSet<String> = rows
        .iter()
        .map(|row| row["route"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(routes.len(), 6, "every facade route is covered: {routes:?}");
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_rodas5p", "data": {"method": "rodas5p", "cases": rows}})
    );
}

/// D2 (frozen): the DOPRI5 adapter is `Dopri5::solve`, and a general plan is
/// `GeneralOde::solve`, bit for bit.
#[test]
fn d2_dopri5_adapter_and_general_plan_are_bitwise_the_direct_calls() {
    let entries = Arc::new(AtomicUsize::new(0));
    let options = tolerances(1e-6, 1e-9);
    let fixture = lotka_volterra();
    let problem = fixture.facade(&entries);
    let plan = explicit_plan(&problem, &options, "dopri5");
    assert_eq!(plan.route.id(), "dopri5/rhs-only");
    let unified =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan).unwrap();
    let direct = Dopri5::solve(&problem, fixture.span, &fixture.y0, &options).unwrap();
    let view = unified.view();
    let MethodResult::Explicit(native) = view.native() else {
        panic!("not an explicit result");
    };
    assert_eq!(
        serde_json::to_string(native).unwrap(),
        serde_json::to_string(&direct).unwrap()
    );
    assert_eq!(bits(view.times()), bits(&direct.t));
    assert_eq!(row_bits(view.states()), row_bits(&direct.y));
    assert_eq!(view.work(), WorkRecord::from(&direct.stats.counters));
    assert_eq!(
        view.steps(),
        (
            direct.stats.accepted_steps as u64,
            direct.stats.rejected_steps as u64
        )
    );
    assert_eq!(view.executed_driver(), direct.stats.method);
    assert_eq!(view.output_grid(), MethodOutputGrid::Endpoints);
    let mut rows = vec![json!({
        "case": "lotka_volterra/dopri5", "route": view.route_id(),
        "accepted": direct.stats.accepted_steps, "rejected": direct.stats.rejected_steps,
        "rhs_calls": direct.stats.counters.rhs_calls,
        "final_state_bits": hex_state(&direct.final_state), "bitwise_equal": true,
    })];

    // General: the default policy selects DOPRI5, a stiff policy RODAS5P
    let robertson = robertson();
    let stiff = robertson.facade(&entries);
    for (name, problem, fixture, policy) in [
        (
            "lotka_volterra/general-default",
            &problem,
            &fixture,
            GeneralOdePolicy::default(),
        ),
        (
            "robertson/general-stiff-provided",
            &stiff,
            &robertson,
            GeneralOdePolicy {
                regime: ProblemRegime::Stiff,
                derivatives: DerivativeFallback::ProvidedOnly,
                ..GeneralOdePolicy::default()
            },
        ),
    ] {
        let plan =
            MethodRegistry::route(problem, &options, &MethodChoice::General(policy)).unwrap();
        let unified =
            MethodRegistry::solve(problem, fixture.span, &fixture.y0, &options, &plan).unwrap();
        let direct =
            GeneralOde::solve(problem, fixture.span, &fixture.y0, &options, &policy).unwrap();
        let view = unified.view();
        assert_eq!(unified.choice(), MethodChoiceKind::General);
        let (direct_t, direct_y, method) = match (&direct.result, view.native()) {
            (GeneralResult::Explicit(d), MethodResult::Explicit(u)) => {
                assert_eq!(
                    serde_json::to_string(u).unwrap(),
                    serde_json::to_string(d).unwrap()
                );
                (&d.t, &d.y, "dopri5")
            }
            (GeneralResult::Implicit(d), MethodResult::Rodas5p(u)) => {
                assert_eq!(
                    serde_json::to_string(u).unwrap(),
                    serde_json::to_string(d).unwrap()
                );
                (&d.t, &d.y, "rodas5p")
            }
            _ => panic!("{name}: the plan executed another method than GeneralOde::solve"),
        };
        assert_eq!(view.route().method_id, method, "{name}");
        assert_eq!(bits(view.times()), bits(direct_t), "{name}");
        assert_eq!(row_bits(view.states()), row_bits(direct_y), "{name}");
        rows.push(json!({
            "case": name, "route": view.route_id(),
            "final_state_bits": hex_state(view.final_state().unwrap()), "bitwise_equal": true,
        }));
    }
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_dopri5_general", "data": {"method": "dopri5", "cases": rows}})
    );
}

/// One integrator-crate adapter against the direct call of its driver on an
/// independently built driver problem.
fn observed_case(method_id: &'static str, fixture: &Fixture, options: &Options) -> Value {
    let name = format!("{}/{method_id}", fixture.name);
    let entries = Arc::new(AtomicUsize::new(0));
    let problem = fixture.facade(&entries);
    let plan = explicit_plan(&problem, options, method_id);
    let unified =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, options, &plan).expect(&name);
    let view = unified.view();
    let MethodResult::Observed(run) = view.native() else {
        panic!("{name}: not an integrator-crate record");
    };

    let ode = fixture.direct();
    let Atol::Scalar(atol) = options.atol else {
        panic!("scalar atol");
    };
    let adaptive = AdaptiveStepConfig {
        atol,
        rtol: options.rtol,
        initial_step: options.h0,
        max_attempts: options.max_steps,
        ..AdaptiveStepConfig::default()
    };
    let schedule = OutputSchedule::new(vec![fixture.span.0, fixture.span.1]).unwrap();
    let (span, y0) = (fixture.span, fixture.y0.as_slice());
    let (direct, attempts) = if method_id == "bdf1-bdf2-variable" {
        let run = integrate_bdf_adaptive_observed(
            &ode,
            span,
            y0,
            &BdfConfig::default(),
            &adaptive,
            &schedule,
        )
        .expect(&name);
        (run.observed, run.diagnostics.attempts)
    } else if method_id == "radau-iia3" {
        let run = integrate_radau_adaptive_observed(
            &ode,
            span,
            y0,
            &RadauConfig::default(),
            &adaptive,
            &schedule,
        )
        .expect(&name);
        (run.observed, run.diagnostics.attempts)
    } else if method_id == METHOD_ID_PEXPRB {
        let run = integrate_pexprb54s4_fused_adaptive_observed(
            &ode,
            span,
            y0,
            &adaptive,
            &schedule,
            FusedPhiKrylovConfig::default(),
            &ParallelExecution::sequential(),
        )
        .expect(&name);
        (run.observed, run.diagnostics.attempts)
    } else if method_id == METHOD_ID_RODAS5P_LAGGED {
        let run = integrate_rodas5p_lagged_observed(
            &ode,
            span,
            y0,
            &adaptive,
            &schedule,
            Rodas5pLaggedOptions::default(),
        )
        .expect(&name);
        (run.fast.observed, run.fast.attempts)
    } else {
        panic!("{method_id} is not an integrator-crate adapter");
    };

    assert!(
        direct.success,
        "{name}: the direct call failed: {}",
        direct.message
    );
    assert_eq!(bits(&run.t), bits(&direct.t), "{name}: times");
    assert_eq!(row_bits(&run.y), row_bits(&direct.y), "{name}: states");
    assert_eq!(run.success, direct.success, "{name}");
    assert_eq!(run.message, direct.message, "{name}");
    assert_eq!(
        run.counters,
        WorkRecord::from(&direct.counters),
        "{name}: work counters"
    );
    assert!(run.counters_from_driver, "{name}");
    assert_eq!(run.attempts, attempts, "{name}: attempts");
    assert_eq!(
        (run.accepted_steps, run.rejected_steps),
        (
            direct.counters.accepted_steps,
            direct.counters.rejected_steps
        ),
        "{name}: step counts"
    );
    assert_eq!(run.internal_steps, direct.internal_steps, "{name}");
    assert_eq!(
        run.output_clipped_steps, direct.output_clipped_steps,
        "{name}"
    );
    // the view agrees with the native record
    assert!(view.is_success() && view.failure().is_none());
    assert_eq!(view.work(), run.counters);
    assert_eq!(view.times(), &[span.0, span.1]);
    assert_eq!(view.output_grid(), MethodOutputGrid::Endpoints);
    assert_eq!(view.executed_driver(), run.driver);
    // the callback entries counted at the facade boundary are the user-code entries
    let boundary = run.callbacks_entered;
    assert_eq!(
        (boundary.rhs + boundary.jacobian + boundary.jvp + boundary.time_derivative) as usize,
        entries.load(Ordering::SeqCst),
        "{name}: boundary counts"
    );
    json!({
        "case": name,
        "route": view.route_id(),
        "driver": run.driver,
        "rtol": options.rtol,
        "atol": atol,
        "attempts": attempts,
        "accepted": direct.counters.accepted_steps,
        "rejected": direct.counters.rejected_steps,
        "rhs_calls": direct.counters.rhs_calls,
        "jacobian_builds": direct.counters.jacobian_builds,
        "jvp_calls": direct.counters.jvp_calls,
        "nonlinear_iterations": direct.counters.nonlinear_iterations,
        "final_state_bits": hex_state(direct.y.last().unwrap()),
        "bitwise_equal": true,
    })
}

/// D2 (frozen): the BDF adapter is `integrate_bdf_adaptive_observed`.
#[test]
fn d2_bdf_adapter_is_bitwise_the_direct_call() {
    let options = tolerances(1e-4, 1e-8);
    let rows = vec![
        observed_case("bdf1-bdf2-variable", &robertson(), &options),
        observed_case("bdf1-bdf2-variable", &vanderpol(), &options),
        observed_case("bdf1-bdf2-variable", &prothero_robinson(true), &options),
    ];
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_bdf", "data": {"method": "bdf1-bdf2-variable", "cases": rows}})
    );
}

/// D2 (frozen): the Radau IIA adapter is `integrate_radau_adaptive_observed`.
#[test]
fn d2_radau_adapter_is_bitwise_the_direct_call() {
    let options = tolerances(1e-6, 1e-9);
    let rows = vec![
        observed_case("radau-iia3", &robertson(), &options),
        observed_case("radau-iia3", &vanderpol(), &options),
        observed_case("radau-iia3", &prothero_robinson(true), &options),
    ];
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_radau", "data": {"method": "radau-iia3", "cases": rows}})
    );
}

/// D2 (frozen): the PEXPRB adapter is
/// `integrate_pexprb54s4_fused_adaptive_observed`.
#[test]
fn d2_pexprb_adapter_is_bitwise_the_direct_call() {
    let options = tolerances(1e-6, 1e-9);
    let rows = vec![observed_case(METHOD_ID_PEXPRB, &heat_reaction(), &options)];
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_pexprb", "data": {"method": METHOD_ID_PEXPRB, "cases": rows}})
    );
}

/// D2 (frozen): the lagged RODAS5P adapter is
/// `integrate_rodas5p_lagged_observed`.
#[test]
fn d2_lagged_rodas5p_adapter_is_bitwise_the_direct_call() {
    let options = tolerances(1e-6, 1e-9);
    let rows = vec![
        observed_case(METHOD_ID_RODAS5P_LAGGED, &robertson(), &options),
        observed_case(METHOD_ID_RODAS5P_LAGGED, &brusselator(), &options),
        observed_case(
            METHOD_ID_RODAS5P_LAGGED,
            &prothero_robinson(false),
            &options,
        ),
    ];
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_lagged", "data": {"method": METHOD_ID_RODAS5P_LAGGED, "cases": rows}})
    );
}

/// D2 (frozen): the PPSW1 adapter is `PeerOde::solve`.
#[test]
fn d2_ppsw1_adapter_is_bitwise_the_direct_call() {
    let options = Options::default();
    let mut rows = Vec::new();
    for (jvp, variant) in [
        (false, "explicit-jacobian-dense"),
        (true, "jvp-columns-dense"),
    ] {
        let fixture = prothero_robinson(jvp);
        for peer in [
            None,
            Some(PeerSolveOptions {
                rtol: 1e-8,
                atol: 1e-10,
                h0_rule_h_max: 0.01,
                ..PeerSolveOptions::default()
            }),
        ] {
            let entries = Arc::new(AtomicUsize::new(0));
            let problem = fixture.facade(&entries);
            let params = MethodParams {
                peer,
                ..MethodParams::default()
            };
            let choice = MethodChoice::Explicit(ppsw1::METHOD_ID);
            let plan = MethodRegistry::route_with(&problem, &options, &choice, &params).unwrap();
            assert_eq!(plan.route.variant, variant);
            assert_eq!(
                plan.route.qualification,
                MethodQualification::ScopedCalibration
            );
            let unified =
                MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan)
                    .unwrap();
            // the direct call: the caller's Peer options, or the shared options lowered
            let direct_options = peer.unwrap_or(PeerSolveOptions {
                rtol: options.rtol,
                atol: 1e-9,
                max_attempts: options.max_steps,
                ..PeerSolveOptions::default()
            });
            let direct = PeerOde::solve(
                &problem,
                fixture.span,
                &fixture.y0,
                &direct_options,
                plan.route.needs,
            )
            .unwrap();
            let view = unified.view();
            let MethodResult::Peer(native) = view.native() else {
                panic!("not a PPSW1 result");
            };
            assert!(direct.success, "{:?}", direct.failure);
            assert_eq!(bits(&native.t), bits(&direct.t));
            assert_eq!(row_bits(&native.y), row_bits(&direct.y));
            assert_eq!(native.h0.to_bits(), direct.h0.to_bits());
            assert_eq!(native.success, direct.success);
            assert_eq!(native.starter, direct.starter);
            for (ours, theirs) in [
                (&native.startup_work, &direct.startup_work),
                (&native.step_work, &direct.step_work),
            ] {
                assert_eq!(
                    serde_json::to_string(ours).unwrap(),
                    serde_json::to_string(theirs).unwrap()
                );
            }
            let mut total = direct.startup_work;
            total.accumulate(direct.step_work);
            assert_eq!(view.work(), WorkRecord::from(&total));
            assert_eq!(
                view.steps(),
                (
                    direct.step_work.accepted_steps,
                    direct.step_work.rejected_steps
                )
            );
            assert_eq!(
                view.output_grid(),
                MethodOutputGrid::AcceptedStepsWithoutInitial
            );
            // the variant of the native admission is the planned one
            assert_eq!(view.route(), plan.route);
            assert_eq!(view.final_time(), Some(fixture.span.1));
            rows.push(json!({
                "case": format!("{}/{}", fixture.name, if peer.is_some() { "peer-options" } else { "shared-options" }),
                "route": view.route_id(),
                "rtol": direct_options.rtol, "atol": direct_options.atol,
                "accepted": direct.step_work.accepted_steps,
                "rejected": direct.step_work.rejected_steps,
                "starter_halvings": direct.starter.halvings,
                "n_outputs": direct.t.len(),
                "final_state_bits": hex_state(direct.y.last().unwrap()),
                "bitwise_equal": true,
            }));
        }
    }
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_ppsw1", "data": {"method": ppsw1::METHOD_ID, "cases": rows}})
    );
}

/// D2 (frozen): the Lawson adapter is `LawsonDopri5::solve`; without the
/// declared flow it is refused.
#[test]
fn d2_lawson_adapter_is_bitwise_the_direct_call_and_needs_its_declaration() {
    let options = tolerances(1e-8, 1e-11);
    let flow = lawson_flow(0.1, 50.0);
    let residual = Problem::new(2, |_t, y, out| {
        out[0] = -0.1 * y[0] * y[0] * y[0];
        out[1] = -0.1 * y[1] * y[1] * y[1];
        Ok(())
    });
    let (span, y0) = ((0.0, 5.0), [1.0, 0.0]);
    let choice = MethodChoice::Explicit(METHOD_ID_LAWSON);
    assert_eq!(
        MethodRegistry::route(&residual, &options, &choice).err(),
        Some(MethodRouteRefusal::Method {
            method_id: METHOD_ID_LAWSON,
            reasons: vec![MethodRefusalReason::MissingDeclaration],
        })
    );
    let params = MethodParams {
        lawson_flow: Some(flow.clone()),
        ..MethodParams::default()
    };
    let plan = MethodRegistry::route_with(&residual, &options, &choice, &params).unwrap();
    let unified = MethodRegistry::solve(&residual, span, &y0, &options, &plan).unwrap();
    let direct = LawsonDopri5::solve(&flow, &residual, span, &y0, &options).unwrap();
    let view = unified.view();
    let MethodResult::Lawson(native) = view.native() else {
        panic!("not a Lawson result");
    };
    assert!(direct.integration.success, "{}", direct.integration.message);
    assert_eq!(
        serde_json::to_string(native).unwrap(),
        serde_json::to_string(&direct).unwrap()
    );
    assert_eq!(bits(view.times()), bits(&direct.integration.t));
    assert_eq!(row_bits(view.states()), row_bits(&direct.integration.y));
    assert_eq!(
        view.work(),
        WorkRecord::from(&direct.integration.stats.counters)
    );
    assert_eq!(view.executed_driver(), direct.integration.stats.method);
    println!(
        "QUAL {}",
        json!({"test": "d2_adapter_lawson", "data": {"method": METHOD_ID_LAWSON, "cases": [{
            "case": "damped_rotation/lawson", "route": view.route_id(),
            "accepted": direct.integration.stats.accepted_steps,
            "rejected": direct.integration.stats.rejected_steps,
            "rhs_calls": direct.integration.stats.counters.rhs_calls,
            "final_state_bits": hex_state(&direct.integration.final_state),
            "bitwise_equal": true,
        }]}})
    );
}

// ------------------------------------------- D3: refusal before any callback

/// One malformed request: its name, span, initial state and options.
type MalformedInput<'a> = (&'a str, (f64, f64), &'a [f64], &'a Options);

/// D3 (frozen): `solve` refuses malformed input and a foreign plan before any
/// callback, for every available method.
#[test]
fn d3_solve_refuses_malformed_input_before_any_callback() {
    let entries = Arc::new(AtomicUsize::new(0));
    let options = tolerances(1e-6, 1e-9);
    let params = MethodParams {
        lawson_flow: Some(lawson_flow(0.1, 3.0)),
        ..MethodParams::default()
    };
    let mut rows = Vec::new();
    let mut methods_checked = std::collections::BTreeSet::new();
    for fixture in [prothero_robinson(false), heat_reaction(), lotka_volterra()] {
        let problem = fixture.facade(&entries);
        for entry in MethodRegistry::catalog() {
            let id = entry.descriptor.method_id;
            let choice = MethodChoice::Explicit(id);
            let Ok(plan) = MethodRegistry::route_with(&problem, &options, &choice, &params) else {
                continue;
            };
            methods_checked.insert(id);
            let mut short = fixture.y0.clone();
            short.pop();
            let mut nan = fixture.y0.clone();
            nan[0] = f64::NAN;
            let bad_rtol = Options {
                rtol: f64::NAN,
                ..options.clone()
            };
            let reversed = (fixture.span.1, fixture.span.0);
            let cases: [MalformedInput<'_>; 4] = [
                ("short y0", fixture.span, &short, &options),
                ("nan y0", fixture.span, &nan, &options),
                ("reversed span", reversed, &fixture.y0, &options),
                ("nan rtol", fixture.span, &fixture.y0, &bad_rtol),
            ];
            for (what, span, y0, opts) in cases {
                let result = MethodRegistry::solve(&problem, span, y0, opts, &plan);
                assert!(
                    result.is_err(),
                    "{id} on {}: {what} was accepted",
                    fixture.name
                );
                rows.push(json!({"method": id, "fixture": fixture.name, "input": what,
                                 "error": result.err().map(|e| e.to_string())}));
            }
            // a plan routed for other facts is refused
            let other_facts = Options {
                dense_output: true,
                ..options.clone()
            };
            assert!(matches!(
                MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &other_facts, &plan),
                Err(MethodSolveError::PlanMismatch)
            ));
        }
    }
    let callbacks = entries.load(Ordering::SeqCst);
    let available = MethodRegistry::catalog()
        .iter()
        .filter(|e| e.availability == MethodAvailability::Available)
        .count();
    println!(
        "QUAL {}",
        json!({"test": "d3_solve_refusals", "data": {
            "refused_solve_calls": rows.len(),
            "methods_checked": methods_checked.len(),
            "available_methods": available,
            "callback_entries": callbacks,
        }})
    );
    assert_eq!(methods_checked.len(), available, "{methods_checked:?}");
    assert_eq!(callbacks, 0, "a refused solve entered a callback");
}

/// D3: the refusal matrix of the explicit choice, with the typed reason of
/// every refused request.
#[test]
fn d3_explicit_refusals_are_typed() {
    use MethodRefusalReason as Reason;
    let entries = Arc::new(AtomicUsize::new(0));
    let options = tolerances(1e-6, 1e-9);
    let dense = prothero_robinson(false);
    let refusal = |problem: &Problem, options: &Options, id: &str| match MethodRegistry::route(
        problem,
        options,
        &MethodChoice::Explicit(id),
    ) {
        Err(MethodRouteRefusal::Method { reasons, .. }) => reasons,
        other => panic!("{id}: expected a method refusal, got {other:?}"),
    };
    // unknown id
    assert_eq!(
        MethodRegistry::route(
            &dense.facade(&entries),
            &options,
            &MethodChoice::Explicit("rodas6")
        )
        .err(),
        Some(MethodRouteRefusal::UnknownMethod {
            method_id: "rodas6".into()
        })
    );
    // no adapter
    assert_eq!(
        refusal(
            &dense.facade(&entries),
            &options,
            "ppsw1-6b-matrix-free-common-w"
        ),
        vec![Reason::AdapterUnavailable]
    );
    let mass = dense.facade(&entries).mass(vec![1.0, 0.0, 0.0, 1.0]);
    let event = dense
        .facade(&entries)
        .event(|_t, y| y[0] - 1.5, Direction::Up);
    let banded = dense.facade(&entries).banded(1, 1);
    let finite_difference = Problem::new(2, |t, y, dy| {
        pr_rhs(t, y, dy);
        Ok(())
    });
    let dense_output = Options {
        dense_output: true,
        ..options.clone()
    };
    let vector_atol = Options {
        atol: Atol::Vector(vec![1e-9; 2]),
        ..options.clone()
    };
    let mut rows = Vec::new();
    for id in [
        "bdf1-bdf2-variable",
        "radau-iia3",
        METHOD_ID_RODAS5P_LAGGED,
        ppsw1::METHOD_ID,
    ] {
        let registry = |r| Reason::Registry(r);
        let mut checks: Vec<(&str, Vec<Reason>, Vec<Reason>)> = vec![
            (
                "mass",
                refusal(&mass, &options, id),
                vec![registry(RegistryRefusal::MassUnsupported)],
            ),
            (
                "events",
                refusal(&event, &options, id),
                vec![registry(RegistryRefusal::EventsUnsupported)],
            ),
            (
                "dense output",
                refusal(&dense.facade(&entries), &dense_output, id),
                vec![registry(RegistryRefusal::DenseOutputUnsupported)],
            ),
            (
                "vector atol",
                refusal(&dense.facade(&entries), &vector_atol, id),
                vec![Reason::VectorAtolUnsupported],
            ),
            (
                "declared band",
                refusal(&banded, &options, id),
                vec![registry(RegistryRefusal::ProviderFormUnsupported)],
            ),
        ];
        if matches!(id, METHOD_ID_RODAS5P_LAGGED | ppsw1::METHOD_ID) {
            checks.push((
                "finite-difference provider",
                refusal(&finite_difference, &options, id),
                vec![registry(RegistryRefusal::ProviderFormUnsupported)],
            ));
        }
        for (what, got, expected) in checks {
            assert_eq!(got, expected, "{id}: {what}");
            rows.push(json!({"method": id, "request": what, "reasons": format!("{got:?}")}));
        }
    }
    // the exponential method lowers a JVP provider of an autonomous problem only
    assert_eq!(
        refusal(&dense.facade(&entries), &options, METHOD_ID_PEXPRB),
        vec![
            Reason::NonAutonomousUnsupported,
            Reason::Registry(RegistryRefusal::ProviderFormUnsupported)
        ]
    );
    assert_eq!(
        refusal(
            &prothero_robinson(true).facade(&entries),
            &options,
            METHOD_ID_PEXPRB
        ),
        vec![Reason::NonAutonomousUnsupported]
    );
    // the explicit drivers and the RODAS5P facade keep their own typed refusals
    assert_eq!(
        refusal(&mass, &options, "dopri5"),
        vec![Reason::Explicit(vigilode::ExplicitUnsupported::MassMatrix)]
    );
    assert_eq!(
        refusal(&event, &options, "rodas5p"),
        vec![Reason::Implicit(Unsupported::EventsWithoutDenseOutput)]
    );
    assert_eq!(entries.load(Ordering::SeqCst), 0);
    println!(
        "QUAL {}",
        json!({"test": "d3_explicit_refusal_matrix", "data": {"rows": rows.len(), "refusals": rows}})
    );
}

// ------------------------------------- the view: typed failure, prefix, work

/// A failed attempt returns a typed reason, its accepted prefix and its work.
#[test]
fn view_reports_typed_failure_with_accepted_prefix_and_work() {
    let entries = Arc::new(AtomicUsize::new(0));
    let budget = Options {
        max_steps: 5,
        ..tolerances(1e-6, 1e-9)
    };
    let mut rows = Vec::new();

    // DOPRI5: attempt budget
    let fixture = lotka_volterra();
    let problem = fixture.facade(&entries);
    let plan = explicit_plan(&problem, &budget, "dopri5");
    let solution =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &budget, &plan).unwrap();
    let view = solution.view();
    assert!(!view.is_success());
    assert!(
        matches!(
            view.failure(),
            Some(MethodFailure::Explicit {
                stop: Some(Dopri5StopReason::AttemptBudget),
                ..
            })
        ),
        "{:?}",
        view.failure()
    );
    let t_last = view.final_time().unwrap();
    assert!(
        view.times()[0] == fixture.span.0 && t_last > fixture.span.0 && t_last < fixture.span.1
    );
    assert!(view.work().rhs_calls > 0 && view.steps().0 > 0);
    rows.push(
        json!({"method": "dopri5", "failure": view.failure().map(|f| f.to_string()),
                     "prefix_end": t_last, "rhs_calls": view.work().rhs_calls}),
    );

    // RODAS5P: attempt budget
    let fixture = robertson();
    let problem = fixture.facade(&entries);
    let plan = explicit_plan(&problem, &budget, "rodas5p");
    let solution =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &budget, &plan).unwrap();
    let view = solution.view();
    assert!(
        matches!(
            view.failure(),
            Some(MethodFailure::Driver(FailureReason::MaxStepsReached {
                max_steps: 5
            }))
        ),
        "{:?}",
        view.failure()
    );
    let t_last = view.final_time().unwrap();
    assert!(t_last > fixture.span.0 && t_last < fixture.span.1);
    assert!(view.work().rhs_calls > 0);
    rows.push(
        json!({"method": "rodas5p", "failure": view.failure().map(|f| f.to_string()),
                     "prefix_end": t_last, "rhs_calls": view.work().rhs_calls}),
    );

    // Radau: attempt budget; the driver exposes the reached output samples only
    let plan = explicit_plan(&problem, &budget, "radau-iia3");
    let solution =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &budget, &plan).unwrap();
    let view = solution.view();
    assert!(
        matches!(
            view.failure(),
            Some(MethodFailure::Driver(FailureReason::MaxStepsReached {
                max_steps: 5
            }))
        ),
        "{:?}",
        view.failure()
    );
    assert_eq!(view.times(), &[fixture.span.0]);
    assert!(view.work().rhs_calls > 0 && view.steps().0 > 0);
    rows.push(
        json!({"method": "radau-iia3", "failure": view.failure().map(|f| f.to_string()),
                     "prefix_end": view.final_time(), "rhs_calls": view.work().rhs_calls}),
    );

    // BDF: a fatal right-hand-side error; the driver returns no record, the adapter keeps
    // the typed reason, the initial state and the callback entries
    let rhs_calls = Arc::new(AtomicUsize::new(0));
    let counter = rhs_calls.clone();
    let fatal = Problem::new(3, move |t, y, dy| {
        if counter.fetch_add(1, Ordering::SeqCst) >= 30 {
            return Err(CallbackError::Fatal("model left its domain".into()));
        }
        robertson_rhs(t, y, dy);
        Ok(())
    })
    .jacobian(Jacobian::dense(|t, y, jac| {
        robertson_jacobian(t, y, jac);
        Ok(())
    }))
    .autonomous();
    let options = tolerances(1e-4, 1e-8);
    let plan = explicit_plan(&fatal, &options, "bdf1-bdf2-variable");
    let solution =
        MethodRegistry::solve(&fatal, fixture.span, &fixture.y0, &options, &plan).unwrap();
    let view = solution.view();
    assert!(!view.is_success());
    match view.failure() {
        Some(MethodFailure::Driver(FailureReason::CallbackFailed {
            callback: Callback::Rhs,
            message,
            ..
        })) => assert_eq!(message, "model left its domain"),
        other => panic!("unexpected failure {other:?}"),
    }
    let MethodResult::Observed(run) = view.native() else {
        panic!("not an integrator-crate record");
    };
    assert!(!run.counters_from_driver && run.driver_error.is_some());
    assert_eq!(run.callbacks_entered.rhs, 31);
    assert_eq!(
        rhs_calls.load(Ordering::SeqCst),
        31,
        "no call after the fatal one"
    );
    assert_eq!(
        view.work().rhs_calls,
        31,
        "the work of the failed run is kept"
    );
    assert_eq!(view.times(), &[fixture.span.0]);
    assert_eq!(run.callback_outcomes.fatal_errors, 1);
    rows.push(
        json!({"method": "bdf1-bdf2-variable", "failure": view.failure().map(|f| f.to_string()),
                     "prefix_end": view.final_time(), "rhs_calls": view.work().rhs_calls}),
    );

    // PPSW1: the typed Peer failure passes through the view
    let failing_calls = Arc::new(AtomicUsize::new(0));
    let peer_problem = decay_problem_failing_after(
        5.0,
        Some(CallbackError::Recoverable("user says shrink".into())),
        None,
        failing_calls,
    );
    let plan = explicit_plan(&peer_problem, &Options::default(), ppsw1::METHOD_ID);
    let solution = MethodRegistry::solve(
        &peer_problem,
        (0.0, 10.0),
        &[1.0],
        &Options::default(),
        &plan,
    )
    .unwrap();
    let view = solution.view();
    assert!(
        matches!(
            view.failure(),
            Some(MethodFailure::Peer(PeerFailure::Callback {
                phase: PeerPhase::Step,
                callback: Callback::Jacobian,
                recoverable: true,
                ..
            }))
        ),
        "{:?}",
        view.failure()
    );
    assert!(view.final_time().unwrap() > 5.0 && view.work().rhs_calls > 0);
    rows.push(
        json!({"method": ppsw1::METHOD_ID, "failure": view.failure().map(|f| f.to_string()),
                     "prefix_end": view.final_time(), "rhs_calls": view.work().rhs_calls}),
    );

    println!(
        "QUAL {}",
        json!({"test": "view_typed_failures", "data": rows})
    );
}

/// The plan names the route that runs, and the view reads it back from the run
/// record. For a general sparse-JVP provider `Rodas5p::capabilities` answers
/// `matrix-free` although `Rodas5p::solve` leaves the router with dense output
/// (fast dense driver) or a vector atol (dense fast driver), assembling the
/// Jacobian from `n` JVP columns per build. The registry plans the driver that
/// runs, with the dense-matrix need it has; the adapter is still the direct
/// call bit for bit.
#[test]
fn view_reports_the_executed_route_when_the_capability_query_disagrees() {
    let fixture = heat_reaction();
    let n = fixture.n;
    let rows: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let mut row: Vec<usize> = (i.saturating_sub(1)..=(i + 1).min(n - 1)).collect();
            if i == 0 {
                row.push(n - 1);
            }
            row
        })
        .collect();
    let pattern = SparsityPattern::from_rows(n, &rows).expect("a valid pattern");
    assert!(!pattern.is_banded());
    let problem = Problem::new(n, |t, y, dy| {
        heat_rhs(t, y, dy);
        Ok(())
    })
    .jacobian(Jacobian::sparse_jvp(pattern, |t, y, v, out| {
        heat_jvp(t, y, v, out);
        Ok(())
    }))
    .autonomous();
    let base = tolerances(1e-6, 1e-9);
    let cases = [
        ("plain", base.clone(), "rodas5p/matrix-free"),
        (
            "dense output",
            Options {
                dense_output: true,
                ..base.clone()
            },
            "rodas5p/fast-dense",
        ),
        (
            "vector atol",
            Options {
                atol: Atol::Vector(vec![1e-9; n]),
                ..base.clone()
            },
            "rodas5p/dense-fast",
        ),
    ];
    let mut data = Vec::new();
    for (name, options, expected) in cases {
        let queried = Rodas5p::capabilities(&problem, &options)
            .expect("supported")
            .route;
        assert_eq!(
            queried,
            FacadeRoute::MatrixFree,
            "{name}: the capability query"
        );
        let plan = explicit_plan(&problem, &options, "rodas5p");
        let unified =
            MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan).unwrap();
        let direct = Rodas5p::solve(&problem, fixture.span, &fixture.y0, &options).unwrap();
        let view = unified.view();
        assert!(view.is_success(), "{name}");
        assert_eq!(bits(view.times()), bits(&direct.t), "{name}");
        assert_eq!(row_bits(view.states()), row_bits(&direct.y), "{name}");
        assert_eq!(view.work(), direct.stats.counters, "{name}");
        // the executed route is read from the run record, and the plan named it
        assert_eq!(view.route_id(), expected, "{name}: executed route");
        assert_eq!(plan.route.id(), expected, "{name}: planned route");
        assert_eq!(view.route(), *view.planned_route(), "{name}");
        let dense = expected != "rodas5p/matrix-free";
        assert_eq!(
            plan.route.needs.dense_matrix, dense,
            "{name}: dense-matrix need"
        );
        assert!(plan.route.needs.provided_jvp, "{name}");
        let counters = direct.stats.counters;
        if dense {
            // the Jacobian is assembled from n JVP columns per build
            assert!(counters.jacobian_builds > 0, "{name}");
            assert!(
                counters.jvp_calls >= n as u64 * counters.jacobian_builds,
                "{name}"
            );
        } else {
            assert_eq!(counters.jacobian_builds, 0, "{name}");
        }
        data.push(json!({
            "request": name,
            "capability_query_route": format!("{queried:?}"),
            "planned_route": plan.route.id(),
            "executed_route": view.route_id(),
            "executed_driver": view.executed_driver(),
            "entry_point": direct.stats.route.entry_point,
            "jacobian_builds": counters.jacobian_builds,
            "jvp_calls": counters.jvp_calls,
            "accepted": direct.stats.accepted_steps,
        }));
    }
    // Auto does not admit the dense drivers without the dense-matrix permission
    let policy = MethodAutoPolicy {
        regime: ProblemRegime::Stiff,
        allowed: CapabilitySet {
            provided_jvp: true,
            ..CapabilitySet::NONE
        },
        admit_unqualified: false,
        fallback: MethodFallback::Never,
        use_measured_evidence: false,
        evidence: &[],
    };
    let auto = |options: &Options| {
        MethodRegistry::route(&problem, options, &MethodChoice::Auto(policy))
            .map(|plan| plan.route.id())
            .ok()
    };
    assert_eq!(auto(&base).as_deref(), Some("rodas5p/matrix-free"));
    assert_eq!(
        auto(&Options {
            dense_output: true,
            ..base.clone()
        }),
        None,
        "the fast dense driver needs the dense-matrix permission"
    );
    // The general surface keeps its receipt on the capability query (default routing is
    // not changed): with a vector atol it selects this problem as matrix-free and does not
    // apply the dense-dimension ceiling, although the dense fast driver runs. The plan of
    // route(General) names the driver that runs and its dense-matrix need.
    let vector = Options {
        atol: Atol::Vector(vec![1e-9; n]),
        ..base.clone()
    };
    let policy = GeneralOdePolicy {
        regime: ProblemRegime::Stiff,
        derivatives: DerivativeFallback::ProvidedOnly,
        max_dense_dimension: 4,
        allow_matrix_free: true,
    };
    let selection = GeneralOde::select(&problem, &vector, &policy).expect("selected");
    let reference = GeneralOde::select_reference(&problem, &vector, &policy).expect("selected");
    assert_eq!(selection.implicit_route, Some(FacadeRoute::MatrixFree));
    assert_eq!(reference.implicit_route, selection.implicit_route);
    let plan = MethodRegistry::route(&problem, &vector, &MethodChoice::General(policy)).unwrap();
    assert_eq!(plan.route.id(), "rodas5p/dense-fast");
    assert!(plan.route.needs.dense_matrix && n > policy.max_dense_dimension);
    println!(
        "QUAL {}",
        json!({"test": "executed_route_vs_capability_query", "data": {
            "requests": data,
            "general_surface_vector_atol": {
                "dimension": n,
                "max_dense_dimension": policy.max_dense_dimension,
                "select_implicit_route": format!("{:?}", selection.implicit_route),
                "select_equals_reference_body": true,
                "planned_route_of_route_general": plan.route.id(),
                "plan_needs_dense_matrix": plan.route.needs.dense_matrix,
            },
        }})
    );
}

// ------------------------------------------------ Auto: ranking and fallback

fn ranked_ids(plan: &vigilode::MethodRoutePlan) -> Vec<&'static str> {
    let mut admitted: Vec<(usize, &'static str)> = plan
        .ranking
        .iter()
        .filter_map(|c| c.rank.map(|rank| (rank, c.method_id)))
        .collect();
    admitted.sort();
    admitted.into_iter().map(|(_, id)| id).collect()
}

/// Auto is a transparent ranking from declared facts; it refuses before any
/// callback, and measured evidence is used only when the policy says so.
#[test]
fn auto_ranking_is_transparent_and_refuses_before_any_callback() {
    let entries = Arc::new(AtomicUsize::new(0));
    let options = tolerances(1e-6, 1e-9);
    let fixture = robertson();
    let problem = fixture.facade(&entries);
    let policy = MethodAutoPolicy {
        regime: ProblemRegime::Stiff,
        allowed: full_permission(),
        admit_unqualified: true,
        fallback: MethodFallback::NextAdmitted { max_attempts: 3 },
        use_measured_evidence: false,
        evidence: &[],
    };
    let plan = MethodRegistry::route(&problem, &options, &MethodChoice::Auto(policy)).unwrap();
    // regime fit, then qualification, then catalog order
    assert_eq!(
        ranked_ids(&plan),
        vec![
            "rodas5p",
            ppsw1::METHOD_ID,
            "bdf1-bdf2-variable",
            METHOD_ID_RODAS5P_LAGGED,
            "radau-iia3"
        ]
    );
    assert_eq!(plan.route.id(), "rodas5p/dense-fast");
    assert_eq!(
        plan.fallbacks
            .iter()
            .map(|r| r.method_id)
            .collect::<Vec<_>>(),
        vec![ppsw1::METHOD_ID, "bdf1-bdf2-variable"],
        "max_attempts bounds the fallback chain"
    );
    assert_eq!(plan.ranking.len(), MethodRegistry::catalog().len());
    let reasons_of = |plan: &vigilode::MethodRoutePlan, id: &str| {
        plan.ranking
            .iter()
            .find(|c| c.method_id == id)
            .map(|c| c.refusals.clone())
            .expect("one row per catalog entry")
    };
    assert_eq!(
        reasons_of(&plan, "dopri5"),
        vec![MethodRefusalReason::RegimeMismatch]
    );
    assert_eq!(
        reasons_of(&plan, METHOD_ID_LAWSON),
        vec![MethodRefusalReason::ExplicitChoiceOnly]
    );
    assert_eq!(
        reasons_of(&plan, "ppsw1-6b-matrix-free-common-w"),
        vec![MethodRefusalReason::AdapterUnavailable]
    );
    assert_eq!(
        reasons_of(&plan, METHOD_ID_PEXPRB),
        vec![MethodRefusalReason::Registry(
            RegistryRefusal::ProviderFormUnsupported
        )]
    );

    // qualified methods only, and only what the General policy permits (no Newton)
    let general = GeneralOdePolicy {
        regime: ProblemRegime::Stiff,
        derivatives: DerivativeFallback::ProvidedOnly,
        ..GeneralOdePolicy::default()
    };
    let from_general = MethodAutoPolicy::from_general(&problem, &general);
    let narrow =
        MethodRegistry::route(&problem, &options, &MethodChoice::Auto(from_general)).unwrap();
    assert_eq!(ranked_ids(&narrow), vec!["rodas5p"]);
    assert!(narrow.fallbacks.is_empty());
    assert!(
        reasons_of(&narrow, ppsw1::METHOD_ID)
            .iter()
            .any(|r| matches!(
                r,
                MethodRefusalReason::Registry(RegistryRefusal::Permission {
                    phase: MethodPhase::Startup,
                    ..
                })
            )),
        "the PPSW1 starter needs Newton, which the policy does not permit"
    );
    assert_eq!(
        reasons_of(&narrow, "radau-iia3")[0],
        MethodRefusalReason::Registry(RegistryRefusal::Unqualified)
    );

    // nothing admitted: a typed refusal with the whole ranking, before any callback
    let none = MethodAutoPolicy {
        allowed: CapabilitySet::NONE,
        ..policy
    };
    match MethodRegistry::route(&problem, &options, &MethodChoice::Auto(none)) {
        Err(MethodRouteRefusal::NoAdmittedMethod { ranking }) => {
            assert_eq!(ranking.len(), MethodRegistry::catalog().len());
            assert!(
                ranking
                    .iter()
                    .all(|c| c.rank.is_none() && !c.refusals.is_empty())
            );
        }
        other => panic!("expected NoAdmittedMethod, got {other:?}"),
    }

    // measured evidence: supplied data, used only when the policy says so, and charged
    let interval = |lower: f64, upper: f64| CostEvidence {
        identity: SelectionIdentity::default(),
        cost: CostInterval { lower, upper },
        unit: CostUnit::Work,
        kind: CostEvidenceKind::ObservedWork,
        admissible: true,
    };
    let evidence = [
        MethodCostEvidence {
            method_id: "bdf1-bdf2-variable",
            evidence: interval(100.0, 120.0),
            callback_units: 7,
        },
        MethodCostEvidence {
            method_id: METHOD_ID_RODAS5P_LAGGED,
            evidence: interval(10.0, 20.0),
            callback_units: 11,
        },
    ];
    let ignored = MethodAutoPolicy {
        evidence: &evidence,
        ..policy
    };
    let plan_ignored =
        MethodRegistry::route(&problem, &options, &MethodChoice::Auto(ignored)).unwrap();
    assert_eq!(ranked_ids(&plan_ignored), ranked_ids(&plan));
    assert_eq!(plan_ignored.evidence_callback_units, 0);
    let used = MethodAutoPolicy {
        use_measured_evidence: true,
        ..ignored
    };
    let plan_used = MethodRegistry::route(&problem, &options, &MethodChoice::Auto(used)).unwrap();
    assert_eq!(
        ranked_ids(&plan_used),
        vec![
            "rodas5p",
            ppsw1::METHOD_ID,
            METHOD_ID_RODAS5P_LAGGED,
            "bdf1-bdf2-variable",
            "radau-iia3"
        ],
        "the cheaper measured interval moves ahead inside its tier only"
    );
    assert_eq!(plan_used.evidence_callback_units, 18);
    assert!(
        plan_used
            .ranking
            .iter()
            .all(|c| c.moved_by_evidence == (c.method_id == METHOD_ID_RODAS5P_LAGGED))
    );
    assert_eq!(
        entries.load(Ordering::SeqCst),
        0,
        "routing entered a callback"
    );
    let solved =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan_used).unwrap();
    assert!(solved.is_success());
    assert_eq!(
        solved.evidence_callback_units(),
        18,
        "the evidence is charged"
    );
    println!(
        "QUAL {}",
        json!({"test": "auto_ranking", "data": {
            "stiff_full_permission": ranked_ids(&plan),
            "stiff_general_policy": ranked_ids(&narrow),
            "with_measured_evidence": ranked_ids(&plan_used),
            "evidence_callback_units": plan_used.evidence_callback_units,
            "ranking_rows": serde_json::to_value(&plan.ranking).unwrap(),
        }})
    );
}

/// The typed fallback: off by default, inside the caller's capabilities only,
/// never silent, and the failed attempt keeps its reason, prefix and work.
#[test]
fn auto_fallback_is_opt_in_bounded_by_permissions_and_never_silent() {
    let entries = Arc::new(AtomicUsize::new(0));
    // Robertson is stiff: DOPRI5 cannot finish within 500 attempts, RODAS5P can
    let options = Options {
        max_steps: 500,
        ..tolerances(1e-6, 1e-9)
    };
    let fixture = robertson();
    let problem = fixture.facade(&entries);
    let general = GeneralOdePolicy {
        regime: ProblemRegime::Unknown,
        derivatives: DerivativeFallback::ProvidedOnly,
        ..GeneralOdePolicy::default()
    };
    let no_fallback = MethodAutoPolicy::from_general(&problem, &general);
    assert_eq!(no_fallback.fallback, MethodFallback::Never);

    // without fallback: the failed explicit attempt is returned as it is
    let plan = MethodRegistry::route(&problem, &options, &MethodChoice::Auto(no_fallback)).unwrap();
    assert_eq!(plan.route.id(), "dopri5/rhs-only");
    assert!(plan.fallbacks.is_empty());
    let alone =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan).unwrap();
    assert!(!alone.is_success());
    assert_eq!(alone.attempts().len(), 1);

    // with fallback: DOPRI5 fails, RODAS5P runs from t0, both attempts are reported
    let with_fallback = MethodAutoPolicy {
        fallback: MethodFallback::NextAdmitted { max_attempts: 2 },
        ..no_fallback
    };
    let plan =
        MethodRegistry::route(&problem, &options, &MethodChoice::Auto(with_fallback)).unwrap();
    assert_eq!(
        plan.fallbacks.iter().map(|r| r.id()).collect::<Vec<_>>(),
        vec!["rodas5p/dense-fast"]
    );
    let solution =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan).unwrap();
    assert!(solution.is_success());
    assert_eq!(solution.choice(), MethodChoiceKind::Auto);
    let attempts: Vec<_> = solution.attempts().collect();
    assert_eq!(attempts.len(), 2, "the fallback is visible");
    let (failed, reported) = (attempts[0], attempts[1]);
    assert_eq!(failed.route_id(), "dopri5/rhs-only");
    assert!(
        matches!(
            failed.failure(),
            Some(MethodFailure::Explicit {
                stop: Some(Dopri5StopReason::AttemptBudget),
                ..
            })
        ),
        "{:?}",
        failed.failure()
    );
    let prefix_end = failed.final_time().unwrap();
    assert!(prefix_end > fixture.span.0 && prefix_end < fixture.span.1);
    assert!(failed.final_state().unwrap().iter().all(|v| v.is_finite()));
    assert_eq!(reported.route_id(), "rodas5p/dense-fast");
    assert_eq!(solution.view().route_id(), "rodas5p/dense-fast");
    // the work of the failed attempt stays in the totals
    let (failed_work, reported_work, total) = (
        failed.work_summary(),
        reported.work_summary(),
        solution.total_work(),
    );
    assert!(failed_work.rhs_calls > 0 && failed_work.accepted_steps > 0);
    assert_eq!(
        total.rhs_calls,
        failed_work.rhs_calls + reported_work.rhs_calls
    );
    assert_eq!(
        total.accepted_steps,
        failed_work.accepted_steps + reported_work.accepted_steps
    );
    assert_eq!(total.jacobian_builds, reported_work.jacobian_builds);
    // the reported attempt is the direct RODAS5P call from t0, bit for bit
    let direct = Rodas5p::solve(&problem, fixture.span, &fixture.y0, &options).unwrap();
    assert_eq!(bits(reported.times()), bits(&direct.t));
    assert_eq!(row_bits(reported.states()), row_bits(&direct.y));
    // and the failed attempt is the direct DOPRI5 call
    let direct_explicit = Dopri5::solve(&problem, fixture.span, &fixture.y0, &options).unwrap();
    assert_eq!(bits(failed.times()), bits(&direct_explicit.t));
    assert_eq!(row_bits(failed.states()), row_bits(&direct_explicit.y));

    // a fallback stays inside the caller's capabilities: without derivative permission
    // there is no implicit candidate, so nothing follows the failed attempt
    let no_derivatives = MethodAutoPolicy {
        allowed: CapabilitySet::NONE,
        ..with_fallback
    };
    let plan =
        MethodRegistry::route(&problem, &options, &MethodChoice::Auto(no_derivatives)).unwrap();
    assert!(plan.fallbacks.is_empty());
    let bounded =
        MethodRegistry::solve(&problem, fixture.span, &fixture.y0, &options, &plan).unwrap();
    assert!(!bounded.is_success() && bounded.attempts().len() == 1);
    println!(
        "QUAL {}",
        json!({"test": "auto_fallback", "data": {
            "attempts": attempts.iter().map(|a| json!({
                "route": a.route_id(), "success": a.is_success(),
                "failure": a.failure().map(|f| f.to_string()),
                "prefix_end": a.final_time(), "work": serde_json::to_value(a.work_summary()).unwrap(),
            })).collect::<Vec<_>>(),
            "total_work": serde_json::to_value(total).unwrap(),
            "without_permission_attempts": bounded.attempts().len(),
        }})
    );
}

// ------------------------------------------------------- the method catalog

/// The catalog is the registry plus the methods node V03 made reachable; the
/// table printed here is the source of the report's method table.
#[test]
fn catalog_extends_the_registry_and_states_what_is_unavailable() {
    let catalog = MethodRegistry::catalog();
    let descriptors = MethodRegistry::descriptors();
    assert_eq!(catalog.len(), descriptors.len() + 3);
    for (entry, descriptor) in catalog.iter().zip(&descriptors) {
        assert_eq!(
            &entry.descriptor, descriptor,
            "the U06 descriptors are carried unchanged"
        );
    }
    let ids: Vec<&str> = catalog.iter().map(|e| e.descriptor.method_id).collect();
    let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "method ids are unique");
    assert_eq!(
        &ids[descriptors.len()..],
        &[METHOD_ID_PEXPRB, METHOD_ID_RODAS5P_LAGGED, METHOD_ID_LAWSON]
    );
    for id in &ids {
        assert_eq!(
            MethodRegistry::entry(id).map(|e| e.descriptor.method_id),
            Some(*id)
        );
    }
    assert!(MethodRegistry::entry("no-such-method").is_none());
    let unavailable: Vec<(&str, &str)> = catalog
        .iter()
        .filter_map(|e| match e.availability {
            MethodAvailability::Unavailable(reason) => Some((e.descriptor.method_id, reason)),
            MethodAvailability::Available => None,
        })
        .collect();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0].0, "ppsw1-6b-matrix-free-common-w");
    assert!(!unavailable[0].1.is_empty());
    // descriptor facts that the code states
    let radau = MethodRegistry::entry("radau-iia3").unwrap().descriptor;
    let stages = RadauConfig::default().stages;
    assert_eq!(
        (radau.stages, radau.order),
        (stages.count(), stages.order())
    );
    // the RODAS5P entry carries the six driver families as exact route variants, the banded
    // and the sequential one included (the U06 descriptor has neither); no other entry
    // needs them
    for entry in &catalog {
        let labels: Vec<&str> = entry.route_variants.iter().map(|v| v.label).collect();
        if entry.descriptor.method_id == "rodas5p" {
            assert_eq!(
                labels,
                vec![
                    "dense-fast",
                    "banded-fast",
                    "matrix-free",
                    "fast-dense",
                    "fast-banded-dense",
                    "sequential-dense"
                ]
            );
            assert!(
                entry
                    .route_variants
                    .iter()
                    .all(|v| v.dense_matrix != v.label.contains("banded") || v.matrix_free)
            );
        } else {
            assert!(labels.is_empty(), "{}", entry.descriptor.method_id);
        }
    }
    let peer = MethodRegistry::entry(ppsw1::METHOD_ID).unwrap();
    assert_eq!(
        peer.jacobian_forms,
        vec![JacobianForm::Dense, JacobianForm::Jvp]
    );
    assert!(!peer.declared_band && !peer.vector_atol);
    // every route names a real export
    let _ = rodas5p_integrators::integrate_pexprb54s4_fused_adaptive_observed;
    let _ = rodas5p_integrators::integrate_rodas5p_lagged_observed;
    let _ = vigilode::LawsonDopri5::solve;
    println!(
        "QUAL {}",
        json!({"test": "method_catalog", "data": serde_json::to_value(&catalog).unwrap()})
    );
}

/// The nine disagreeing cases of audit finding COD-03 (probe cases c01..c13):
/// what `route` answers for each, next to `GeneralOde::select`.
#[test]
fn cod03_audit_cases_agree_through_route() {
    use DerivativeFallback::{AllowFiniteDifference, ProvidedOnly};
    use ProblemRegime::{Stiff, Unknown};
    let entries = Arc::new(AtomicUsize::new(0));
    let policy = |regime, derivatives, allow_matrix_free| GeneralOdePolicy {
        regime,
        derivatives,
        max_dense_dimension: 64,
        allow_matrix_free,
    };
    let base = |n: usize| {
        let counter = entries.clone();
        Problem::new(n, move |_t, _y, d| {
            counter.fetch_add(1, Ordering::SeqCst);
            d.fill(0.0);
            Ok(())
        })
    };
    let dense = || {
        let counter = entries.clone();
        Jacobian::dense(move |_t, _y, m| {
            counter.fetch_add(1, Ordering::SeqCst);
            m.fill(0.0);
            Ok(())
        })
    };
    let band = || {
        let counter = entries.clone();
        Jacobian::banded(1, 1, move |_t, _y, m| {
            counter.fetch_add(1, Ordering::SeqCst);
            m.fill(0.0);
            Ok(())
        })
    };
    let jvp = || {
        let counter = entries.clone();
        Jacobian::jvp(move |_t, _y, _v, out| {
            counter.fetch_add(1, Ordering::SeqCst);
            out.fill(0.0);
            Ok(())
        })
    };
    let plain = Options::default();
    let dense_output = Options {
        dense_output: true,
        ..Options::default()
    };
    let vector = Options {
        atol: Atol::Vector(vec![1e-8; 3]),
        ..Options::default()
    };
    let identity = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let cases: Vec<(&str, Problem, &Options, GeneralOdePolicy)> = vec![
        (
            "c01_banded_provider_n1000_stiff_provided",
            base(1000).jacobian(band()).autonomous(),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c02_fd_declared_band_n1000_stiff_allowfd",
            base(1000).banded(1, 1).autonomous(),
            &plain,
            policy(Stiff, AllowFiniteDifference, false),
        ),
        (
            "c03_jvp_n3_stiff_allowfd_no_matrix_free",
            base(3).jacobian(jvp()).autonomous(),
            &plain,
            policy(Stiff, AllowFiniteDifference, false),
        ),
        (
            "c04_dense_n3_nonautonomous_no_ft_stiff_provided",
            base(3).jacobian(dense()),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c05_dense_n3_event_stiff_provided",
            base(3)
                .jacobian(dense())
                .autonomous()
                .event(|_t, y| y[0], Direction::Either),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c06_dense_n3_mass_stiff_provided",
            base(3).jacobian(dense()).autonomous().mass(identity),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c07_dense_n3_regime_unknown_provided",
            base(3).jacobian(dense()).autonomous(),
            &plain,
            policy(Unknown, ProvidedOnly, false),
        ),
        (
            "c08_dense_n3_dense_output_stiff_provided",
            base(3).jacobian(dense()).autonomous(),
            &dense_output,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c09_jvp_n3_vector_atol_stiff_provided_matrix_free",
            base(3).jacobian(jvp()).autonomous(),
            &vector,
            policy(Stiff, ProvidedOnly, true),
        ),
        (
            "c10_dense_provider_declared_band_n100_control",
            base(100).jacobian(dense()).banded(1, 1).autonomous(),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c11_dense_n3_stiff_provided_control",
            base(3).jacobian(dense()).autonomous(),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c12_banded_provider_n10_label",
            base(10).jacobian(band()).autonomous(),
            &plain,
            policy(Stiff, ProvidedOnly, false),
        ),
        (
            "c13_dense_n3_event_dense_output",
            base(3)
                .jacobian(dense())
                .autonomous()
                .event(|_t, y| y[0], Direction::Either),
            &dense_output,
            policy(Stiff, ProvidedOnly, false),
        ),
    ];
    let mut rows = Vec::new();
    for (name, problem, options, policy) in &cases {
        let select = GeneralOde::select(problem, options, policy);
        let routed = general_via_route(problem, options, policy);
        assert_eq!(outcome(&routed), outcome(&select), "{name}");
        // The Auto choice under the permissions of the same policy: where select selects, it
        // ranks the same route first and labels the route that runs. Where select refuses a
        // derivative, a route or a resource, Auto admits nothing either. The general surface
        // itself is narrower than the methods (identity mass, no events, no dense output):
        // those three refusals belong to the surface, and Auto may admit RODAS5P there.
        let auto = MethodAutoPolicy::from_general(problem, policy);
        let auto_plan = MethodRegistry::route(problem, options, &MethodChoice::Auto(auto));
        let auto_route = auto_plan.as_ref().ok().map(|p| p.route.id());
        if let Ok(selection) = &select {
            let expected = match (selection.method, selection.implicit_route) {
                (ProductionMethod::Dopri5, _) => "dopri5/rhs-only".to_owned(),
                (ProductionMethod::Rodas5p, Some(route)) => {
                    MethodRegistry::route(problem, options, &MethodChoice::Explicit("rodas5p"))
                        .map(|plan| {
                            assert_eq!(plan.route.facade_route, Some(route));
                            plan.route.id()
                        })
                        .unwrap()
                }
                (ProductionMethod::Rodas5p, None) => unreachable!(),
            };
            assert_eq!(auto_route.as_deref(), Some(expected.as_str()), "{name}");
        } else if let Err(refusal) = &select
            && !matches!(
                refusal,
                SelectionRefusal::MassMatrix
                    | SelectionRefusal::Events
                    | SelectionRefusal::DenseOutput
            )
        {
            assert_eq!(policy.regime, Stiff);
            assert!(
                auto_route.is_none(),
                "{name}: select refuses with {refusal:?} and Auto admitted {auto_route:?}"
            );
        }
        rows.push(json!({
            "case": name,
            "select": format!("{select:?}"),
            "route_general_equal": true,
            "auto_under_the_same_policy": auto_route,
        }));
    }
    assert_eq!(entries.load(Ordering::SeqCst), 0);
    println!(
        "QUAL {}",
        json!({"test": "cod03_audit_cases", "data": rows})
    );
}
