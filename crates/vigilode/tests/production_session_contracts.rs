use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use vigilode::{
    Atol, CallbackError, Direction, Dopri5, ExplicitSolution, ExplicitSolveError,
    ExplicitUnsupported, Jacobian, Options, Problem, SessionAdvance, SessionEpochs,
};

fn options() -> Options {
    Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-12),
        h0: 0.01,
        ..Options::default()
    }
}

fn same_run(a: &ExplicitSolution, b: &ExplicitSolution) {
    assert_eq!(a.t, b.t);
    assert_eq!(a.y, b.y);
    assert_eq!(a.final_time.to_bits(), b.final_time.to_bits());
    assert_eq!(a.final_state, b.final_state);
    assert_eq!(a.success, b.success);
    assert_eq!(a.stats.counters, b.stats.counters);
    assert_eq!(a.stats.callback_work, b.stats.callback_work);
    assert_eq!(a.stats.attempts, b.stats.attempts);
    assert_eq!(a.stats.accepted_steps, b.stats.accepted_steps);
    assert_eq!(a.stats.rejected_steps, b.stats.rejected_steps);
    assert_eq!(a.stats.fsal_reuses, b.stats.fsal_reuses);
    assert_eq!(a.stats.rhs_completed, b.stats.rhs_completed);
    assert_eq!(a.stats.output_clipped_steps, b.stats.output_clipped_steps);
}

#[test]
fn pausing_between_trials_and_observations_preserves_the_one_shot_path() {
    let problem = Problem::new(2, |t, y, out| {
        out[0] = -y[0];
        out[1] = 2.0 * t;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("Jacobian entered")))
    .time_derivative(|_, _, _| panic!("time derivative entered"));
    let opts = Options {
        h0: 0.6,
        atol: Atol::Vector(vec![1e-12, 2e-12]),
        ..options()
    };
    let times = [0.0, 0.37, 0.62, 1.0];
    let reference =
        Dopri5::solve_observed(&problem, (0.0, 1.0), &[1.0, 0.0], &opts, &times).unwrap();
    let (yields, result) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[1.0, 0.0],
        &opts,
        &times,
        SessionEpochs::default(),
        |session| {
            let mut yields = 0;
            for &target in &times[1..] {
                loop {
                    match session
                        .advance_to(target, 1, SessionEpochs::default())
                        .unwrap()
                    {
                        SessionAdvance::Yield => yields += 1,
                        SessionAdvance::Checkpoint | SessionAdvance::Complete => break,
                        SessionAdvance::Stopped => panic!("unexpected stop"),
                    }
                }
                assert_eq!(session.checkpoint().t, target);
            }
            yields
        },
    )
    .unwrap();
    assert!(result.success);
    assert!(yields > 1);
    assert!(result.stats.rejected_steps > 0);
    same_run(&reference, &result);
    assert_eq!(result.stats.error_control.atol, vec![1e-12, 2e-12]);
    assert!((result.final_state[0] - (-1.0_f64).exp()).abs() < 1e-9);
    assert!((result.final_state[1] - 1.0).abs() < 1e-13);
    let c = result.stats.counters;
    assert_eq!(
        c.jacobian_builds + c.jvp_calls + c.ft_calls + c.linear_solves,
        0
    );
}

#[test]
fn zero_quota_and_bad_targets_or_epochs_do_not_change_state_or_work() {
    let problem = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    });
    let epochs = SessionEpochs {
        model: 4,
        ..SessionEpochs::default()
    };
    let (_, result) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[1.0],
        &options(),
        &[0.0, 0.5, 1.0],
        epochs,
        |session| {
            assert_eq!(
                session.advance_to(0.5, 0, epochs).unwrap(),
                SessionAdvance::Yield
            );
            assert_eq!(session.checkpoint().attempts, 0);
            assert_eq!(session.callback_work().attempted, 0);
            assert_eq!(
                session.advance_to(0.5, usize::MAX, epochs).unwrap(),
                SessionAdvance::Checkpoint
            );
            let before = (
                session.checkpoint().t,
                session.checkpoint().y.to_vec(),
                session.checkpoint().proposed_h,
                session.checkpoint().attempts,
                *session.checkpoint().counters,
                session.callback_work(),
            );
            for target in [f64::NAN, 0.4, 0.7, 1.1] {
                assert!(session.advance_to(target, 5, epochs).is_err());
            }
            assert!(
                session
                    .advance_to(1.0, 5, SessionEpochs { model: 5, ..epochs })
                    .is_err()
            );
            let after = (
                session.checkpoint().t,
                session.checkpoint().y.to_vec(),
                session.checkpoint().proposed_h,
                session.checkpoint().attempts,
                *session.checkpoint().counters,
                session.callback_work(),
            );
            assert_eq!(before, after);
            assert_eq!(
                session.advance_to(1.0, usize::MAX, epochs).unwrap(),
                SessionAdvance::Complete
            );
        },
    )
    .unwrap();
    assert!(result.success);
}

#[test]
fn early_driver_return_preserves_the_last_unscheduled_accepted_checkpoint() {
    let problem = Problem::new(1, |_, _, out| {
        out[0] = 1.0;
        Ok(())
    });
    let (checkpoint, result) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[0.0],
        &options(),
        &[0.0, 1.0],
        SessionEpochs::default(),
        |session| {
            assert_eq!(
                session
                    .advance_to(1.0, 1, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Yield
            );
            (session.checkpoint().t, session.checkpoint().y.to_vec())
        },
    )
    .unwrap();
    assert!(!result.success);
    assert!(result.stop_reason.is_none());
    assert!(result.final_time > 0.0 && result.final_time < 1.0);
    assert_eq!(result.final_time, checkpoint.0);
    assert_eq!(result.final_state, checkpoint.1);
    assert_eq!(result.t.last(), Some(&checkpoint.0));
    assert_eq!(result.y.last(), Some(&checkpoint.1));
    assert!((result.final_state[0] - result.final_time).abs() < 1e-14);
    match result.ensure_success() {
        Err(ExplicitSolveError::Failed { partial, .. }) => {
            assert_eq!(partial.final_time, checkpoint.0)
        }
        other => panic!("expected paused partial solution: {other:?}"),
    }
}

#[test]
fn fatal_callback_stops_once_and_the_session_cannot_retry_it() {
    let entered = Arc::new(AtomicU64::new(0));
    let calls = entered.clone();
    let problem = Problem::new(1, move |t, _, out| {
        calls.fetch_add(1, Ordering::Relaxed);
        if t > 0.2 {
            return Err(CallbackError::Fatal("model unavailable".into()));
        }
        out[0] = 1.0;
        Ok(())
    });
    let (_, result) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[0.0],
        &options(),
        &[0.0, 0.1, 1.0],
        SessionEpochs::default(),
        |session| {
            assert_eq!(
                session
                    .advance_to(0.1, usize::MAX, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Checkpoint
            );
            assert_eq!(
                session
                    .advance_to(1.0, usize::MAX, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Stopped
            );
            let before = (session.checkpoint().attempts, session.callback_work());
            for _ in 0..3 {
                assert_eq!(
                    session
                        .advance_to(1.0, 10, SessionEpochs::default())
                        .unwrap(),
                    SessionAdvance::Stopped
                );
            }
            assert_eq!(
                before,
                (session.checkpoint().attempts, session.callback_work())
            );
            assert_eq!(
                session.callback_failure().unwrap().message,
                "model unavailable"
            );
        },
    )
    .unwrap();
    assert!(!result.success);
    assert!(result.final_time >= 0.1 && result.final_time <= 0.2);
    assert!((result.final_state[0] - result.final_time).abs() < 1e-14);
    assert_eq!(result.stats.callback_work.fatal_errors, 1);
    assert_eq!(
        result.stats.counters.rhs_calls,
        entered.load(Ordering::Relaxed)
    );
}

#[test]
fn malformed_callback_output_is_fatal_even_after_an_accepted_checkpoint() {
    let problem = Problem::new(2, |t, _, out| {
        out[0] = 1.0;
        if t <= 0.2 {
            out[1] = 2.0;
        }
        Ok(())
    });
    let (_, result) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[0.0, 0.0],
        &options(),
        &[0.0, 0.1, 1.0],
        SessionEpochs::default(),
        |session| {
            assert_eq!(
                session
                    .advance_to(1.0, usize::MAX, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Stopped
            );
            assert!(session.callback_failure().unwrap().malformed_output);
            let before = session.callback_work();
            assert_eq!(
                session
                    .advance_to(1.0, 5, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Stopped
            );
            assert_eq!(session.callback_work(), before);
        },
    )
    .unwrap();
    assert_eq!(result.stats.callback_work.malformed_outputs, 1);
    assert_eq!(result.stats.callback_work.fatal_errors, 0);
    assert!(result.final_time >= 0.1);
    assert!(result.final_state.iter().all(|x| x.is_finite()));
    assert!((result.final_state[1] - 2.0 * result.final_time).abs() < 1e-14);
}

fn recoverable_problem() -> Problem {
    let calls = AtomicU64::new(0);
    Problem::new(1, move |_, _, out| {
        if calls.fetch_add(1, Ordering::Relaxed) == 1 {
            return Err(CallbackError::Recoverable(
                "temporary domain failure".into(),
            ));
        }
        out[0] = 1.0;
        Ok(())
    })
}

#[test]
fn recoverable_failure_and_rejected_work_survive_a_pause() {
    let times = [0.0, 0.5, 1.0];
    let reference = Dopri5::solve_observed(
        &recoverable_problem(),
        (0.0, 1.0),
        &[0.0],
        &options(),
        &times,
    )
    .unwrap();
    let (_, result) = Dopri5::with_session(
        &recoverable_problem(),
        (0.0, 1.0),
        &[0.0],
        &options(),
        &times,
        SessionEpochs::default(),
        |session| {
            assert_eq!(
                session
                    .advance_to(1.0, 1, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Yield
            );
            assert_eq!(session.checkpoint().t, 0.0);
            assert_eq!(session.checkpoint().rejected_steps, 1);
            assert_eq!(session.callback_work().recoverable_errors, 1);
            assert_eq!(
                session
                    .advance_to(1.0, usize::MAX, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Complete
            );
        },
    )
    .unwrap();
    same_run(&reference, &result);
}

#[test]
fn unsupported_requests_never_enter_the_driver_or_user_callbacks() {
    let problem = Problem::new(1, |_, _, _| panic!("unsupported request entered RHS"));
    let requests = [
        (
            problem.clone().mass(vec![1.0]),
            options(),
            ExplicitUnsupported::MassMatrix,
        ),
        (
            problem.clone(),
            Options {
                dense_output: true,
                ..options()
            },
            ExplicitUnsupported::DenseOutput,
        ),
        (
            problem.event(|_, y| y[0], Direction::Either),
            options(),
            ExplicitUnsupported::Events,
        ),
    ];
    for (problem, opts, expected) in requests {
        let result = Dopri5::with_session(
            &problem,
            (0.0, 1.0),
            &[1.0],
            &opts,
            &[0.0, 1.0],
            SessionEpochs::default(),
            |_| panic!("unsupported request entered driver"),
        );
        match result {
            Err(ExplicitSolveError::Unsupported(actual)) => assert_eq!(actual, expected),
            _ => panic!("expected unsupported request"),
        }
    }
}

#[test]
fn a_global_attempt_budget_is_not_reset_by_each_advance() {
    let problem = Problem::new(1, |_, _, out| {
        out[0] = 1.0;
        Ok(())
    });
    let opts = Options {
        max_steps: 2,
        h0: 0.01,
        ..options()
    };
    let (_, result) = Dopri5::with_session(
        &problem,
        (0.0, 1.0),
        &[0.0],
        &opts,
        &[0.0, 1.0],
        SessionEpochs::default(),
        |session| {
            assert_eq!(
                session
                    .advance_to(1.0, 1, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Yield
            );
            let state = session
                .advance_to(1.0, 1, SessionEpochs::default())
                .unwrap();
            // The budget is detected before another trial, including when a
            // call has just exhausted its local quota.
            assert_eq!(state, SessionAdvance::Stopped);
            assert_eq!(session.checkpoint().attempts, 2);
            assert_eq!(
                session
                    .advance_to(1.0, usize::MAX, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Stopped
            );
        },
    )
    .unwrap();
    assert!(!result.success);
    assert_eq!(result.stats.attempts, 2);
    assert_eq!(
        result.stop_reason,
        Some(rodas5p_integrators::Dopri5StopReason::AttemptBudget)
    );
}

#[test]
fn zero_length_session_is_complete_without_callback_work() {
    let problem = Problem::new(1, |_, _, _| panic!("zero span entered RHS"));
    let (_, result) = Dopri5::with_session(
        &problem,
        (2.0, 2.0),
        &[3.0],
        &options(),
        &[2.0],
        SessionEpochs::default(),
        |session| {
            assert_eq!(
                session
                    .advance_to(2.0, 0, SessionEpochs::default())
                    .unwrap(),
                SessionAdvance::Complete
            )
        },
    )
    .unwrap();
    assert!(result.success);
    assert_eq!(result.t, vec![2.0]);
    assert_eq!(result.y, vec![vec![3.0]]);
    assert_eq!(result.stats.callback_work.attempted, 0);
}
