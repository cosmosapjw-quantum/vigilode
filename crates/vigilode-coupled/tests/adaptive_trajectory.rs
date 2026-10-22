use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveOutputSession, AdaptiveStepConfig, MaxStepPolicy, OdeProblem,
    OutputSchedule, ParallelExecution, Rodas5pMfFastWorkspace,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode_coupled::{Config, FailureKind, RootErrorPolicy, integrate_coupled_observed};

fn linear() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        atol: 1e-14,
        rtol: 1e-12,
        ..LinearSolverConfig::default()
    }
}

fn decay() -> OdeProblem {
    OdeProblem::new(
        "adaptive-three-mode-decay",
        6,
        Arc::new(|_, y, f| {
            for i in 0..6 {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, _, v, out| {
            for i in 0..6 {
                out[i] = [-1.0, -20.0, -100.0][i % 3] * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn constant() -> OdeProblem {
    OdeProblem::new(
        "constant",
        1,
        Arc::new(|_, _, f| {
            f[0] = 1.0;
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, _, _, out| {
            out[0] = 0.0;
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn adaptive_candidates_commit_only_accepted_states_and_land_on_outputs() {
    let p = decay();
    let schedule = OutputSchedule::new(vec![0.0, 0.13, 0.5]).unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1e-10,
        rtol: 1e-7,
        initial_step: 0.5,
        max_step: 0.5,
        ..AdaptiveStepConfig::default()
    };
    let r = integrate_coupled_observed(
        &p,
        (0.0, 0.5),
        &[1.0; 6],
        &adaptive,
        &schedule,
        &Config::default(),
        &ParallelExecution::rayon(2).unwrap(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(r.observed.success, "{}", r.observed.message);
    assert_eq!(r.observed.t, schedule.times());
    assert!(r.attempts.iter().any(|a| !a.accepted));
    let mut committed_t = 0.0;
    for a in &r.attempts {
        assert_eq!(a.t, committed_t);
        if a.accepted {
            committed_t += a.h;
        }
    }
    assert_eq!(committed_t, 0.5);
    assert_eq!(
        r.observed.counters.accepted_steps as usize,
        r.attempts.iter().filter(|a| a.accepted).count()
    );
    assert_eq!(
        r.observed.counters.rejected_steps as usize,
        r.attempts.iter().filter(|a| !a.accepted).count()
    );
    for (&t, y) in r.observed.t.iter().zip(&r.observed.y) {
        for (i, actual) in y.iter().enumerate() {
            let exact = ([-1.0, -20.0, -100.0][i % 3] * t).exp();
            assert!(
                (actual - exact).abs() <= 2e-7,
                "at t={t} component {i}: {actual} vs {exact}"
            );
        }
    }
    let rhs: u64 = r
        .attempts
        .iter()
        .map(|a| {
            a.candidate_work.rhs_attempted
                + a.fallback_work.rhs_calls
                + a.fallback_validation_work.rhs_attempted
        })
        .sum();
    assert_eq!(r.observed.counters.rhs_calls, rhs);
    println!(
        "ADAPTIVE_DECAY attempts={} accepted={} rejected={} rhs={} jvp={}",
        r.attempts.len(),
        r.observed.counters.accepted_steps,
        r.observed.counters.rejected_steps,
        rhs,
        r.observed.counters.jvp_calls
    );
}

#[test]
fn forced_raw_fallback_matches_native_raw_u_driver() {
    let p = decay();
    let schedule = OutputSchedule::new(vec![0.0, 0.001]).unwrap();
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.001,
        max_step: 0.001,
        ..AdaptiveStepConfig::default()
    };
    let cfg = Config {
        max_rhs: 0,
        ..Config::default()
    };
    let result = integrate_coupled_observed(
        &p,
        (0.0, 0.001),
        &[1.0; 6],
        &adaptive,
        &schedule,
        &cfg,
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    let reference = Rodas5pMfFastWorkspace::new(&p, &linear())
        .unwrap()
        .integrate_observed(&p, (0.0, 0.001), &[1.0; 6], &adaptive, &schedule)
        .unwrap();
    assert!(result.observed.success);
    assert!(reference.observed.success);
    assert!(result.attempts.iter().all(|a| a.fallback_attempted));
    assert!(
        result
            .attempts
            .iter()
            .all(|a| a.candidate_work.rhs_attempted == 0)
    );
    assert!(result.observed.counters.linear_solves > 0);
    for (a, b) in result
        .last_y
        .iter()
        .zip(reference.observed.y.last().unwrap())
    {
        assert!((a - b).abs() < 1e-12);
    }
}

#[test]
fn callback_failure_retains_work_and_the_committed_prefix() {
    let p = OdeProblem::new(
        "callback-failure",
        1,
        Arc::new(|_, _, _| Err(CoreError::InvalidInput("client domain failure".into()))),
        None,
        None,
        Some(Arc::new(|_, _, v, out| {
            out.copy_from_slice(v);
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let r = integrate_coupled_observed(
        &p,
        (0.0, 1.0),
        &[1.0],
        &AdaptiveStepConfig::default(),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(!r.observed.success);
    assert_eq!(r.last_t, 0.0);
    assert_eq!(r.last_y, vec![1.0]);
    assert_eq!(r.observed.t, vec![0.0]);
    assert_eq!(r.attempts.len(), 1);
    assert_eq!(
        r.terminal_failure.as_ref().unwrap().kind,
        FailureKind::Callback
    );
    assert_eq!(r.observed.counters.rhs_calls, 1);
    assert!(!r.attempts[0].fallback_attempted);
}

#[test]
fn changed_epoch_cannot_commit_a_candidate() {
    let epoch = Arc::new(AtomicU64::new(0));
    let in_jvp = epoch.clone();
    let observed_epoch = epoch.clone();
    let p = OdeProblem::new(
        "epoch-change",
        1,
        Arc::new(|_, y, f| {
            f[0] = -y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            in_jvp.fetch_add(1, Ordering::SeqCst);
            out[0] = -v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_model_epoch(Arc::new(move || observed_epoch.load(Ordering::SeqCst)));
    let r = integrate_coupled_observed(
        &p,
        (0.0, 1.0),
        &[1.0],
        &AdaptiveStepConfig::default(),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(!r.observed.success);
    assert_eq!(r.last_t, 0.0);
    assert_eq!(r.last_y, vec![1.0]);
    assert_eq!(
        r.terminal_failure.as_ref().unwrap().kind,
        FailureKind::ModelChanged
    );
    assert!(!r.attempts[0].fallback_attempted);
}

#[test]
fn coarse_clock_uses_represented_intervals_and_strict_cap_refuses() {
    let p = constant();
    let start = 1e12_f64;
    let ulp = start.next_up() - start;
    let end = start + 4.0 * ulp;
    let schedule = OutputSchedule::new((0..=4).map(|i| start + i as f64 * ulp).collect()).unwrap();
    let adaptive = AdaptiveStepConfig {
        initial_step: 1e-4,
        max_step: 1e-4,
        min_step: 1e-10,
        ..AdaptiveStepConfig::default()
    };
    let r = integrate_coupled_observed(
        &p,
        (start, end),
        &[0.0],
        &adaptive,
        &schedule,
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(r.observed.success, "{}", r.observed.message);
    assert_eq!(r.observed.t, schedule.times());
    assert!((r.last_y[0] - (end - start)).abs() < 1e-15);
    assert!(r.attempts.iter().all(|a| a.h == ulp));
    let strict = AdaptiveStepConfig {
        max_step_policy: MaxStepPolicy::StrictRepresentedCap,
        ..adaptive
    };
    let r = integrate_coupled_observed(
        &p,
        (start, end),
        &[0.0],
        &strict,
        &schedule,
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(!r.observed.success);
    assert_eq!(r.last_t, start);
    assert!(r.attempts.is_empty());
    assert!(r.terminal_failure.is_some());
}

#[test]
fn attempt_budget_returns_partial_result_without_relabelling_final_time() {
    let p = constant();
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 1,
        ..AdaptiveStepConfig::default()
    };
    let r = integrate_coupled_observed(
        &p,
        (0.0, 1.0),
        &[0.0],
        &adaptive,
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(!r.observed.success);
    assert_eq!(r.last_t, 0.1);
    assert!((r.last_y[0] - 0.1).abs() < 1e-13);
    assert_eq!(r.observed.t, vec![0.0]);
    assert_eq!(r.attempts.len(), 1);
}

#[test]
fn output_session_commits_only_its_issued_finite_dimensioned_endpoint() {
    let adaptive = AdaptiveStepConfig {
        initial_step: 0.1,
        max_step: 0.1,
        ..AdaptiveStepConfig::default()
    };
    let mut session = AdaptiveOutputSession::new(
        &adaptive,
        &OutputSchedule::new(vec![0.0, 0.1, 0.5]).unwrap(),
        (0.0, 0.5),
        &[0.0],
    )
    .unwrap();
    assert!(session.accept(0.1, &[0.1], false).is_err());
    let trial = session
        .trial(0.0, 0.1, &AdaptiveControllerState::default())
        .unwrap()
        .unwrap();
    assert!(session.accept(0.2, &[0.2], trial.clipped).is_err());
    assert!(session.accept(0.1, &[0.1, 0.1], trial.clipped).is_err());
    assert!(session.accept(0.1, &[f64::NAN], trial.clipped).is_err());
    session.accept(0.1, &[0.1], trial.clipped).unwrap();
    assert!(session.accept(0.2, &[0.2], false).is_err());
    let result = session
        .finish(false, "test partial".into(), Default::default(), 1)
        .unwrap();
    assert_eq!(result.t, vec![0.0, 0.1]);
    assert_eq!(result.y, vec![vec![0.0], vec![0.1]]);
}

#[test]
fn singular_trial_operator_retries_a_smaller_physical_step() {
    let gamma = rodas5p_core::rodas5p_coefficients().unwrap().gamma;
    let reciprocal = gamma.recip();
    let rate = [reciprocal.next_down(), reciprocal, reciprocal.next_up()]
        .into_iter()
        .find(|r| gamma * r == 1.0)
        .expect("a neighboring binary64 rate must construct exactly singular W");
    // The initial h=1 makes W=I-h*gamma*J exactly zero in this fixture.
    assert_eq!(gamma * rate, 1.0);
    let p = OdeProblem::new(
        "singular-initial-W",
        1,
        Arc::new(move |_, y, f| {
            f[0] = rate * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            out[0] = rate * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let adaptive = AdaptiveStepConfig {
        initial_step: 1.0,
        max_step: 1.0,
        atol: 1e-10,
        rtol: 1e-7,
        ..AdaptiveStepConfig::default()
    };
    let r = integrate_coupled_observed(
        &p,
        (0.0, 1.0),
        &[1.0],
        &adaptive,
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        &Config {
            max_rhs: 0,
            ..Config::default()
        },
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(r.observed.success, "{}", r.observed.message);
    assert!(!r.attempts[0].accepted);
    assert!(r.attempts[0].retryable_failure);
    assert_eq!(
        r.attempts[0].retry_failure_kind,
        Some(rodas5p_integrators::AdaptiveFailureKind::LinearSolve)
    );
    assert!(r.observed.counters.linear_solve_failures > 0);
    assert_eq!(r.attempts[1].t, 0.0);
    assert!(r.attempts[1].h < r.attempts[0].h);
    assert!(r.terminal_failure.is_none());
    assert!((r.last_y[0] / rate.exp() - 1.0).abs() < 1e-6);
}

#[test]
fn callback_cannot_impersonate_a_retryable_linear_solver_failure() {
    let p = OdeProblem::new(
        "callback-typed-linear-failure",
        1,
        Arc::new(|_, _, _| Err(CoreError::LinearSolve("actually a client callback".into()))),
        None,
        None,
        Some(Arc::new(|_, _, v, out| {
            out.copy_from_slice(v);
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let r = integrate_coupled_observed(
        &p,
        (0.0, 1.0),
        &[1.0],
        &AdaptiveStepConfig::default(),
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        &Config {
            max_rhs: 0,
            ..Config::default()
        },
        &ParallelExecution::sequential(),
        &linear(),
        &RootErrorPolicy::Operational,
    )
    .unwrap();
    assert!(!r.observed.success);
    assert_eq!(r.attempts.len(), 1);
    assert!(r.attempts[0].fallback_attempted);
    assert!(!r.attempts[0].retryable_failure);
    assert_eq!(r.terminal_failure.unwrap().kind, FailureKind::Callback);
    assert_eq!(r.last_t, 0.0);
    assert_eq!(r.last_y, vec![1.0]);
    assert!(r.observed.counters.rhs_calls > 0);
}
