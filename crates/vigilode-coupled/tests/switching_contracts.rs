use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode_coupled::{
    CandidateRoute, CausalSchedule, CausalSwitchPolicy, Config, FailureKind, RawStepSession,
    RootErrorPolicy, integrate_causal_observed, integrate_sequential_raw_observed,
    integrate_switching_causal_observed,
};

fn linear() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        atol: 1e-14,
        rtol: 1e-12,
        maxiter: 256,
        restart: 64,
        ..Default::default()
    }
}
fn problem(rhs: Arc<AtomicU64>, jvp: Arc<AtomicU64>, fail: bool) -> OdeProblem {
    OdeProblem::new(
        "switch-contract",
        2,
        Arc::new(move |_, y, f| {
            rhs.fetch_add(1, Ordering::SeqCst);
            if fail {
                return Err(CoreError::LinearSolve(
                    "callback-masquerading-as-solver".into(),
                ));
            }
            f[0] = -y[0];
            f[1] = -10.0 * y[1];
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| panic!("full Jacobian forbidden"))),
        Some(Arc::new(move |_, _, v, f| {
            jvp.fetch_add(1, Ordering::SeqCst);
            f[0] = -v[0];
            f[1] = -10.0 * v[1];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1e-10,
        rtol: 1e-7,
        initial_step: 0.001,
        max_step: 0.001,
        max_attempts: 1000,
        ..Default::default()
    }
}
fn output() -> OutputSchedule {
    OutputSchedule::new((0..=32).map(|i| i as f64 * 0.001).collect()).unwrap()
}

#[test]
fn declining_projection_is_bypassed_and_all_attempted_callbacks_are_charged() {
    let run = |switching: bool| {
        let rhs = Arc::new(AtomicU64::new(0));
        let jvp = Arc::new(AtomicU64::new(0));
        let p = problem(rhs.clone(), jvp.clone(), false);
        // A valid but insufficient rank-one basis exercises actual leakage,
        // not an artificial zero callback or storage budget.
        let c = Config {
            initial_basis: 1,
            max_basis: 1,
            max_expansions: 0,
            ..Default::default()
        };
        let r = if switching {
            integrate_switching_causal_observed(
                &p,
                (0., 0.032),
                &[1., 1.],
                &adaptive(),
                &output(),
                &c,
                &ParallelExecution::sequential(),
                &linear(),
                &RootErrorPolicy::Operational,
                &CausalSwitchPolicy::default(),
            )
            .unwrap()
        } else {
            integrate_causal_observed(
                &p,
                (0., 0.032),
                &[1., 1.],
                &adaptive(),
                &output(),
                &c,
                &ParallelExecution::sequential(),
                &linear(),
                &RootErrorPolicy::Operational,
                CausalSchedule::Forward,
            )
            .unwrap()
        };
        assert!(r.observed.success, "{}", r.observed.message);
        assert_eq!(r.observed.counters.rhs_calls, rhs.load(Ordering::SeqCst));
        assert_eq!(r.observed.counters.jvp_calls, jvp.load(Ordering::SeqCst));
        r
    };
    let fixed = run(false);
    let switched = run(true);
    let probes = switched
        .attempts
        .iter()
        .filter(|a| !a.candidate_skipped)
        .count();
    assert!(probes > 0 && probes <= 17, "probes={probes}");
    assert!(fixed.attempts.iter().all(|a| a.fallback_attempted));
    assert!(switched.observed.counters.rhs_calls < fixed.observed.counters.rhs_calls);
    assert!(switched.observed.counters.jvp_calls < fixed.observed.counters.jvp_calls);
    for (a, b) in fixed.observed.y.iter().zip(&switched.observed.y) {
        assert_eq!(
            a, b,
            "same protected target should produce identical outputs"
        );
    }
    assert!(switched.attempts.iter().any(|a| {
        a.switch_decision
            .as_ref()
            .is_some_and(|s| s.phase == "reentry-probe")
    }));
    for a in &switched.attempts {
        if a.candidate_skipped {
            assert_eq!(a.candidate_work.rhs_attempted, 0);
            assert_eq!(a.candidate_work.jvp_attempted, 0);
            assert!(a.candidate_failure.is_none());
        }
    }
}

#[test]
fn all_explicitly_selected_candidate_roots_keep_accuracy_and_work_contracts() {
    for candidate in [
        CandidateRoute::Forward,
        CandidateRoute::Picard,
        CandidateRoute::PicardReuse,
        CandidateRoute::ResidualFirst,
    ] {
        let rhs = Arc::new(AtomicU64::new(0));
        let jvp = Arc::new(AtomicU64::new(0));
        let p = problem(rhs.clone(), jvp.clone(), false);
        let r = integrate_switching_causal_observed(
            &p,
            (0., 0.032),
            &[1., 1.],
            &adaptive(),
            &output(),
            &Config::default(),
            &ParallelExecution::sequential(),
            &linear(),
            &RootErrorPolicy::Operational,
            &CausalSwitchPolicy {
                candidate,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            r.observed.success,
            "{:?}: {}",
            candidate, r.observed.message
        );
        assert!(r.attempts.iter().any(|a| !a.candidate_skipped));
        assert_eq!(r.observed.counters.rhs_calls, rhs.load(Ordering::SeqCst));
        assert_eq!(r.observed.counters.jvp_calls, jvp.load(Ordering::SeqCst));
        for (&t, y) in r.observed.t.iter().zip(&r.observed.y) {
            assert!((y[0] - (-t).exp()).abs() < 1e-8);
            assert!((y[1] - (-10. * t).exp()).abs() < 1e-8);
        }
    }
}

#[test]
fn fatal_protected_callback_does_not_become_a_retry_or_candidate_trial() {
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let p = problem(rhs.clone(), jvp.clone(), true);
    let r = integrate_switching_causal_observed(
        &p,
        (0., 0.032),
        &[1., 1.],
        &adaptive(),
        &output(),
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
        &CausalSwitchPolicy::default(),
    )
    .unwrap();
    assert!(!r.observed.success);
    assert_eq!(r.last_t, 0.);
    assert_eq!(r.attempts.len(), 1);
    assert_eq!(
        r.terminal_failure.as_ref().unwrap().kind,
        FailureKind::Callback
    );
    assert!(!r.attempts[0].retryable_failure);
    assert!(r.attempts[0].candidate_skipped);
    assert_eq!(r.observed.counters.rhs_calls, rhs.load(Ordering::SeqCst));
    assert_eq!(rhs.load(Ordering::SeqCst), 1);
}

#[test]
fn invalid_policy_and_invalid_protected_inputs_do_no_callback_work() {
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let p = problem(rhs.clone(), jvp.clone(), false);
    let policy = CausalSwitchPolicy {
        initial_cooldown: 0,
        ..Default::default()
    };
    assert!(
        integrate_switching_causal_observed(
            &p,
            (0., 0.032),
            &[1., 1.],
            &adaptive(),
            &output(),
            &Config::default(),
            &ParallelExecution::sequential(),
            &linear(),
            &RootErrorPolicy::Operational,
            &policy
        )
        .is_err()
    );
    let mut session = RawStepSession::new(&p, &linear()).unwrap();
    let r = session.attempt_protected(
        &p,
        0.,
        &[1., 1.],
        0.001,
        &Config {
            full_residual_wrms_limit: f64::NAN,
            ..Default::default()
        },
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert_eq!(r.terminal_failure.unwrap().kind, FailureKind::Input);
    let r = session.attempt_protected(
        &p,
        0.,
        &[f64::NAN, 1.],
        0.001,
        &Config::default(),
        &ParallelExecution::sequential(),
        &RootErrorPolicy::Operational,
    );
    assert!(r.terminal_failure.is_some());
    assert_eq!(rhs.load(Ordering::SeqCst) + jvp.load(Ordering::SeqCst), 0);
}

#[test]
fn direct_protected_route_is_independent_of_candidate_storage_budget() {
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let p = problem(rhs, jvp, false);
    let c = Config {
        max_storage_bytes: 0,
        ..Default::default()
    };
    let r = integrate_sequential_raw_observed(
        &p,
        (0., 0.032),
        &[1., 1.],
        &adaptive(),
        &output(),
        &c,
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(r.observed.success);
    assert!(
        r.attempts
            .iter()
            .all(|a| a.candidate_skipped && a.candidate_failure.is_none())
    );
}

#[test]
fn strict_state_guard_recalibrates_on_the_real_accepted_state_and_keeps_raw_solution() {
    let p = problem(
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
        false,
    );
    let c = Config::default();
    let execution = ParallelExecution::sequential();
    let raw = integrate_sequential_raw_observed(
        &p,
        (0., 0.032),
        &[1., 1.],
        &adaptive(),
        &output(),
        &c,
        &execution,
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    let switched = integrate_switching_causal_observed(
        &p,
        (0., 0.032),
        &[1., 1.],
        &adaptive(),
        &output(),
        &c,
        &execution,
        &linear(),
        &RootErrorPolicy::Operational,
        &CausalSwitchPolicy {
            reference_guard: Some(vigilode_coupled::CausalReferenceGuard {
                state_absolute_scale: 1e-12,
                state_relative_scale: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(switched.observed.success && raw.observed.success);
    assert_eq!(switched.observed.y, raw.observed.y);
    assert!(switched.attempts.iter().all(|a| a.candidate_skipped));
    assert!(switched.attempts.iter().skip(1).all(|a| {
        a.switch_decision
            .as_ref()
            .is_some_and(|s| s.reference_status == "state-displacement-changed")
    }));
    assert_eq!(
        switched.observed.counters.rhs_calls,
        raw.observed.counters.rhs_calls
    );
    assert_eq!(
        switched.observed.counters.jvp_calls,
        raw.observed.counters.jvp_calls
    );
}
