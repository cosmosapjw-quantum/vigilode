//! U07 contracts: explicit PPSW1-6b end-to-end adapter over the facade `Problem`.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use serde_json::json;
use vigilode::{
    CallbackError, CapabilitySet, DerivativeFallback, Direction, GeneralOde, GeneralOdePolicy,
    Jacobian, MethodAdmission, MethodPhase, Options, PeerOde, PeerSolveError, PeerSolveOptions,
    Problem, ProblemRegime, ProductionMethod, RegistryRefusal,
};

const L: [[f64; 2]; 2] = [[-2.0, 30.0], [0.0, -400.0]];

fn phi(t: f64) -> [f64; 2] {
    [t.sin() + 1.0, (2.0 * t).cos()]
}
fn dphi(t: f64) -> [f64; 2] {
    [t.cos(), -2.0 * (2.0 * t).sin()]
}
fn pr_rhs(t: f64, y: &[f64], dy: &mut [f64]) {
    let (p, d) = (phi(t), dphi(t));
    for i in 0..2 {
        dy[i] = d[i] + L[i][0] * (y[0] - p[0]) + L[i][1] * (y[1] - p[1]);
    }
}
fn dense_l(_t: f64, _y: &[f64], j: &mut [f64]) -> Result<(), CallbackError> {
    j.copy_from_slice(&[L[0][0], L[0][1], L[1][0], L[1][1]]);
    Ok(())
}

/// Nonnormal stiff Prothero-Robinson system with exact solution phi.
fn pr_problem(jacobian_kind: &str, calls: Arc<AtomicUsize>) -> Problem {
    let c = calls.clone();
    let p = Problem::new(2, move |t, y, dy| {
        c.fetch_add(1, Ordering::SeqCst);
        pr_rhs(t, y, dy);
        Ok(())
    });
    match jacobian_kind {
        "dense" => {
            let c = calls.clone();
            p.jacobian(Jacobian::dense(move |t, y, j| {
                c.fetch_add(1, Ordering::SeqCst);
                dense_l(t, y, j)
            }))
        }
        "jvp" => {
            let c = calls.clone();
            p.jacobian(Jacobian::jvp(move |_t, _y, v, o| {
                c.fetch_add(1, Ordering::SeqCst);
                o[0] = L[0][0] * v[0] + L[0][1] * v[1];
                o[1] = L[1][0] * v[0] + L[1][1] * v[1];
                Ok(())
            }))
        }
        _ => p,
    }
}

fn caps(jvp: bool, explicit: bool, dense: bool, newton: bool) -> CapabilitySet {
    CapabilitySet {
        provided_jvp: jvp,
        explicit_jacobian: explicit,
        dense_matrix: dense,
        newton,
        ..CapabilitySet::NONE
    }
}

fn opts() -> PeerSolveOptions {
    PeerSolveOptions {
        rtol: 1e-8,
        atol: 1e-10,
        h0_rule_h_max: 0.01,
        ..PeerSolveOptions::default()
    }
}

#[test]
fn e1_end_to_end_dense_and_jvp_reach_the_endpoint_accurately() {
    let mut rows = Vec::new();
    for (kind, allowed) in [
        ("dense", caps(false, true, true, true)),
        ("jvp", caps(true, false, true, true)),
    ] {
        for threads in [1usize, 4] {
            let p = pr_problem(kind, Arc::new(AtomicUsize::new(0)));
            let o = PeerSolveOptions { threads, ..opts() };
            let s = PeerOde::solve(&p, (0.0, 1.0), &phi(0.0), &o, allowed).unwrap();
            assert!(s.success, "{kind}: {:?}", s.failure);
            assert_eq!(*s.t.last().unwrap(), 1.0, "endpoint is hit exactly");
            assert!(s.t.windows(2).all(|w| w[1] > w[0]));
            let e = phi(1.0);
            let y = s.y.last().unwrap();
            let err = (y[0] - e[0]).abs().max((y[1] - e[1]).abs());
            assert!(err < 1e-6, "{kind} error {err:e}");
            assert!(s.startup_work.nonlinear_iterations > 0);
            assert_eq!(s.step_work.nonlinear_iterations, 0, "steps are Newton-free");
            assert!(matches!(s.admission, MethodAdmission::Admitted { .. }));
            rows.push(json!({"jacobian": kind, "threads": threads, "h0": s.h0, "error": err,
                             "accepted": s.step_work.accepted_steps,
                             "rejected": s.step_work.rejected_steps,
                             "startup_rhs": s.startup_work.rhs_calls, "step_rhs": s.step_work.rhs_calls,
                             "startup_jacobian": s.startup_work.jacobian_builds,
                             "step_jacobian": s.step_work.jacobian_builds,
                             "step_jvp": s.step_work.jvp_calls}));
        }
    }
    println!("QUAL {}", json!({"test": "e1_end_to_end", "data": rows}));
}

#[test]
fn e2_refusals_happen_before_any_callback() {
    let calls = Arc::new(AtomicUsize::new(0));
    let full = caps(true, true, true, true);
    let y0 = phi(0.0);
    let cases: Vec<(&str, Problem, CapabilitySet)> = vec![
        (
            "mass",
            pr_problem("dense", calls.clone()).mass(vec![1.0, 0.0, 0.0, 1.0]),
            full,
        ),
        (
            "events",
            pr_problem("dense", calls.clone()).event(|_t, y| y[0] - 1.5, Direction::Up),
            full,
        ),
        (
            "no_newton",
            pr_problem("dense", calls.clone()),
            caps(false, true, true, false),
        ),
        (
            "no_dense",
            pr_problem("dense", calls.clone()),
            caps(false, true, false, true),
        ),
        (
            "finite_difference_only",
            pr_problem("fd", calls.clone()),
            full,
        ),
        (
            "banded",
            pr_problem("dense", calls.clone()).banded(1, 1),
            full,
        ),
    ];
    let mut rows = Vec::new();
    for (name, p, allowed) in cases {
        let r = PeerOde::solve(&p, (0.0, 1.0), &y0, &opts(), allowed);
        rows.push(json!({"case": name, "outcome": format!("{:?}", r.as_ref().err())}));
        match (name, r) {
            ("mass", Err(PeerSolveError::Refused(v))) => {
                assert!(v.contains(&RegistryRefusal::MassUnsupported))
            }
            ("events", Err(PeerSolveError::Refused(v))) => {
                assert!(v.contains(&RegistryRefusal::EventsUnsupported))
            }
            ("no_newton", Err(PeerSolveError::Refused(v))) => {
                assert!(v.iter().any(|r| matches!(
                    r,
                    RegistryRefusal::Permission {
                        phase: MethodPhase::Startup,
                        ..
                    }
                )))
            }
            ("no_dense", Err(PeerSolveError::Refused(_))) => {}
            ("finite_difference_only" | "banded", Err(PeerSolveError::UnsupportedJacobian)) => {}
            (n, other) => panic!("{n}: unexpected {:?}", other.err()),
        }
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no callback before refusal"
    );
    println!("QUAL {}", json!({"test": "e2_refusals", "data": rows}));
}

#[test]
fn e3_short_span_caps_the_starter_at_the_endpoint() {
    let p = pr_problem("dense", Arc::new(AtomicUsize::new(0)));
    let o = PeerSolveOptions {
        h0: Some(0.5),
        ..opts()
    };
    let s = PeerOde::solve(
        &p,
        (0.0, 1e-3),
        &phi(0.0),
        &o,
        caps(false, true, true, true),
    )
    .unwrap();
    assert!(s.h0 <= 0.5e-3, "h0 {}", s.h0);
    assert!(s.success, "{:?}", s.failure);
    assert_eq!(*s.t.last().unwrap(), 1e-3);
    assert!(s.t[0] <= 1e-3, "starter endpoint {} passed t_end", s.t[0]);
}

#[test]
fn e4_fatal_callback_keeps_the_accepted_prefix_and_charged_work() {
    let p = Problem::new(2, move |t, y, dy| {
        if t > 0.5 {
            return Err(CallbackError::Fatal("model left its domain".into()));
        }
        pr_rhs(t, y, dy);
        Ok(())
    })
    .jacobian(Jacobian::dense(dense_l));
    let s = PeerOde::solve(
        &p,
        (0.0, 1.0),
        &phi(0.0),
        &opts(),
        caps(false, true, true, true),
    )
    .unwrap();
    assert!(!s.success);
    let failure = s.failure.clone().unwrap();
    assert!(failure.contains("fatal callback error"), "{failure}");
    assert!(!s.t.is_empty() && *s.t.last().unwrap() <= 0.5 + 1e-12);
    assert!(s.y.iter().flatten().all(|v| v.is_finite()));
    assert!(
        s.step_work.rhs_calls > 6 * s.step_work.accepted_steps,
        "failed attempt charged"
    );
    println!(
        "QUAL {}",
        json!({"test": "e4_fatal_prefix", "data": {"failure": failure, "last_t": s.t.last(),
               "accepted": s.step_work.accepted_steps, "rhs_calls": s.step_work.rhs_calls}})
    );
}

#[test]
fn e5_recoverable_callback_rejects_then_recovers() {
    let tripped = Arc::new(AtomicBool::new(false));
    let tr = tripped.clone();
    let p = Problem::new(2, move |t, y, dy| {
        if t > 0.3 && !tr.swap(true, Ordering::SeqCst) {
            return Err(CallbackError::Recoverable("transient".into()));
        }
        pr_rhs(t, y, dy);
        Ok(())
    })
    .jacobian(Jacobian::dense(dense_l));
    let s = PeerOde::solve(
        &p,
        (0.0, 1.0),
        &phi(0.0),
        &opts(),
        caps(false, true, true, true),
    )
    .unwrap();
    assert!(tripped.load(Ordering::SeqCst));
    assert!(s.success, "{:?}", s.failure);
    assert!(s.step_work.rejected_steps >= 1);
}

#[test]
fn e6_no_switching_and_no_default_routing_change() {
    assert_eq!(
        PeerOde::switch_history_to("rodas5p"),
        Err(PeerSolveError::MethodSwitchUnsupported)
    );
    // selection only (no solve): the stiff general surface still selects RODAS5P
    let p = pr_problem("dense", Arc::new(AtomicUsize::new(0))).autonomous();
    let policy = GeneralOdePolicy {
        regime: ProblemRegime::Stiff,
        derivatives: DerivativeFallback::ProvidedOnly,
        max_dense_dimension: 64,
        allow_matrix_free: false,
    };
    let sel = GeneralOde::select(&p, &Options::default(), &policy).unwrap();
    assert_eq!(sel.method, ProductionMethod::Rodas5p);
}
