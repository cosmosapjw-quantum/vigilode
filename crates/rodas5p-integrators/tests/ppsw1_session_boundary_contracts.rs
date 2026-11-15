//! Regressions for independently reproduced transaction/time-boundary defects.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::ppsw1::{self, Config, Session, State};
use rodas5p_integrators::{ParallelExecution, scalar_linear_problem};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn state(t: f64, value: f64, h: f64) -> State {
    State {
        t,
        y: std::array::from_fn(|_| vec![value]),
        f: std::array::from_fn(|_| vec![0.0]),
        h_prev: h,
    }
}

#[test]
fn foreign_session_attempt_cannot_commit_or_reject() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    let initial = state(0.0, 1.0, 0.1);
    let mut a = Session::new(
        &p,
        initial.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let mut b = Session::new(
        &p,
        state(0.0, 2.0, 0.1),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let foreign = b.attempt(0.1).unwrap();
    assert!(a.accept(foreign).is_err());
    let foreign = b.attempt(0.1).unwrap();
    assert!(a.reject(foreign).is_err());
    assert_eq!(a.state(), &initial);
    assert_eq!(a.counters(), WorkCounters::default());
    assert!(a.controller().accepted_ratios.is_empty());
}

#[test]
fn rejected_attempt_invalidates_other_attempts_from_that_generation() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    let initial = state(0.0, 1.0, 0.1);
    let mut a = Session::new(
        &p,
        initial.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let rejected = a.attempt(0.1).unwrap();
    let sibling = a.attempt(0.1).unwrap();
    a.reject(rejected).unwrap();
    let after_rejection = a.counters();
    assert!(a.accept(sibling).is_err());
    assert_eq!(a.state(), &initial);
    assert_eq!(a.counters(), after_rejection);
    // A fresh retry still works after invalidating the discarded generation.
    let retry = a.attempt(0.05).unwrap();
    a.accept(retry).unwrap();
    assert_eq!(a.state().t, 0.05);
}

#[test]
fn malformed_history_is_refused_before_any_callbacks() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    let base = state(0.0, 1.0, 0.1);
    let mut bad = Vec::new();
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut s = base.clone();
        s.t = t;
        bad.push(s);
    }
    for h in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut s = base.clone();
        s.h_prev = h;
        bad.push(s);
    }
    let mut s = base.clone();
    s.y[0].clear();
    bad.push(s);
    let mut s = base.clone();
    s.f[2].push(1.0);
    bad.push(s);
    let mut s = base.clone();
    s.y[3][0] = f64::NAN;
    bad.push(s);
    let mut s = base.clone();
    s.f[4][0] = f64::INFINITY;
    bad.push(s);
    for (i, s) in bad.into_iter().enumerate() {
        assert!(
            Session::new(
                &p,
                s.clone(),
                Config::default(),
                ParallelExecution::sequential()
            )
            .is_err(),
            "case {i}"
        );
        let mut work = WorkCounters::default();
        assert!(
            ppsw1::advance(&p, &s, 0.1, &ParallelExecution::sequential(), &mut work).is_err(),
            "case {i}"
        );
        assert_eq!(
            work,
            WorkCounters::default(),
            "case {i} ran numerical callbacks"
        );
        assert!(ppsw1::stage_inputs(&s, 0.1).is_err(), "case {i}");
    }
}

#[test]
fn nonadvancing_and_overflowing_step_times_are_refused_before_work() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    // Last case has a finite forward endpoint but a nonfinite negative-node time.
    for (t, h) in [(1e16, 0.01), (f64::MAX, f64::MAX), (-f64::MAX, f64::MAX)] {
        let s = state(t, 0.0, h);
        let mut work = WorkCounters::default();
        assert!(ppsw1::advance(&p, &s, h, &ParallelExecution::sequential(), &mut work).is_err());
        assert_eq!(work, WorkCounters::default());
        assert!(ppsw1::stage_inputs(&s, h).is_err());
    }
}

#[test]
fn starter_requires_two_finite_representably_advancing_intervals() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    for (t, h) in [(1e16, 0.01), (0.0, f64::MAX), (f64::MAX, 1.0)] {
        let mut work = WorkCounters::default();
        assert!(ppsw1::start(&p, t, &[0.0], h, &mut work).is_err());
        assert_eq!(
            work,
            WorkCounters::default(),
            "invalid startup did numerical work"
        );
    }
}

#[test]
fn nonfinite_try_step_rejection_invalidates_pending_sibling_attempt() {
    let fail = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&fail);
    let p = rodas5p_integrators::OdeProblem::new(
        "conditional-nonfinite",
        1,
        Arc::new(move |_, _, out| {
            out[0] = if flag.load(Ordering::SeqCst) {
                f64::NAN
            } else {
                0.0
            };
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| {
            rodas5p_core::DenseMatrix::from_vec_rows(vec![vec![0.0]])
        })),
        None,
        None,
        false,
        None,
        None,
    )
    .unwrap();
    let mut session = Session::new(
        &p,
        state(0.0, 0.0, 0.1),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let pending = session.attempt(0.1).unwrap();
    fail.store(true, Ordering::SeqCst);
    let decision = session
        .try_step(
            0.1,
            &ppsw1::Tolerances {
                atol: 1e-6,
                rtol: 1e-6,
            },
        )
        .unwrap();
    assert!(matches!(decision, ppsw1::Decision::Rejected { .. }));
    fail.store(false, Ordering::SeqCst);
    assert!(session.accept(pending).is_err());
    assert_eq!(session.state().t, 0.0);
    assert_eq!(session.counters().accepted_steps, 0);
    assert_eq!(session.counters().rejected_steps, 1);
}
