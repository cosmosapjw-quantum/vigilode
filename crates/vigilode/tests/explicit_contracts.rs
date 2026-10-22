use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use vigilode::{
    Atol, CallbackError, Direction, Dopri5, ExplicitSolveError, ExplicitUnsupported, Jacobian,
    Options, Problem,
};

fn options() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.01,
        ..Options::default()
    }
}

#[test]
fn explicit_nonautonomous_solution_never_enters_derivative_providers() {
    let calls = Arc::new(AtomicU64::new(0));
    let seen = calls.clone();
    let problem = Problem::new(2, move |t, y, out| {
        seen.fetch_add(1, Ordering::Relaxed);
        out[0] = 2.0 * t;
        out[1] = -y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| {
        panic!("explicit solve evaluated Jacobian")
    }))
    .time_derivative(|_, _, _| panic!("explicit solve evaluated df/dt"));
    let times = [0.0, 0.13, 0.47, 0.9, 1.0];
    let result = Dopri5::solve_observed(&problem, (0.0, 1.0), &[0.0, 1.0], &options(), &times)
        .unwrap()
        .ensure_success()
        .unwrap();
    assert_eq!(result.t, times);
    for (&t, y) in result.t.iter().zip(&result.y) {
        assert!((y[0] - t * t).abs() < 1e-12);
        assert!((y[1] - (-t).exp()).abs() < 5e-8);
    }
    assert_eq!(result.final_time, 1.0);
    assert_eq!(result.final_state, *result.y.last().unwrap());
    let c = result.stats.counters;
    assert_eq!(c.rhs_calls, calls.load(Ordering::Relaxed));
    assert_eq!(result.stats.callback_work.attempted, c.rhs_calls);
    assert_eq!(
        result.stats.callback_work.completed,
        result.stats.rhs_completed
    );
    assert_eq!(
        c.jacobian_builds + c.jvp_calls + c.ft_calls + c.linear_solves + c.nonlinear_solves,
        0
    );
    assert_eq!(c.direct_factorizations + c.mass_matvecs, 0);
    assert_eq!(result.stats.method, "dopri5-5(4)-fsal");
}

#[test]
fn explicit_uniform_vector_atol_reproduces_scalar_and_records_vector() {
    let problem = Problem::new(2, |t, y, out| {
        out[0] = -y[0];
        out[1] = 2.0 * t;
        Ok(())
    });
    let scalar = options();
    let vector = Options {
        atol: Atol::Vector(vec![1e-10; 2]),
        ..scalar.clone()
    };
    let a = Dopri5::solve(&problem, (0.0, 2.0), &[1.0, 0.0], &scalar)
        .unwrap()
        .ensure_success()
        .unwrap();
    let b = Dopri5::solve(&problem, (0.0, 2.0), &[1.0, 0.0], &vector)
        .unwrap()
        .ensure_success()
        .unwrap();
    assert_eq!(a.t, b.t);
    assert_eq!(a.y, b.y);
    assert_eq!(a.stats.counters, b.stats.counters);
    assert_eq!(b.stats.options.atol, Atol::Vector(vec![1e-10; 2]));
    let encoded = serde_json::to_value(&b.stats).unwrap();
    assert_eq!(encoded["schema"], "vigilode.explicit-stats.v1");
    assert_eq!(encoded["method"], "dopri5-5(4)-fsal");
    assert!(encoded.get("jacobian").is_none());
}

#[test]
fn explicit_features_are_refused_before_rhs() {
    let problem = Problem::new(1, |_, _, _| panic!("unsupported request entered RHS"));
    assert!(matches!(
        Dopri5::solve(
            &problem.clone().mass(vec![1.0]),
            (0.0, 1.0),
            &[1.0],
            &options()
        ),
        Err(ExplicitSolveError::Unsupported(
            ExplicitUnsupported::MassMatrix
        ))
    ));
    let dense = Options {
        dense_output: true,
        ..options()
    };
    assert!(matches!(
        Dopri5::solve(&problem, (0.0, 1.0), &[1.0], &dense),
        Err(ExplicitSolveError::Unsupported(
            ExplicitUnsupported::DenseOutput
        ))
    ));
    let events = problem.event(|_, y| y[0], Direction::Either);
    assert!(matches!(
        Dopri5::solve(&events, (0.0, 1.0), &[1.0], &options()),
        Err(ExplicitSolveError::Unsupported(ExplicitUnsupported::Events))
    ));
}

#[test]
fn explicit_invalid_inputs_and_schedules_do_not_enter_rhs() {
    let p = Problem::new(2, |_, _, _| panic!("invalid request entered RHS"));
    for atol in [
        Atol::Scalar(f64::NAN),
        Atol::Scalar(-1.0),
        Atol::Vector(vec![1e-9]),
        Atol::Vector(vec![1e-9, f64::INFINITY]),
    ] {
        let opts = Options { atol, ..options() };
        assert!(matches!(
            Dopri5::solve(&p, (0.0, 1.0), &[1.0, 2.0], &opts),
            Err(ExplicitSolveError::InvalidInput(_))
        ));
    }
    assert!(Dopri5::solve(&p, (1.0, 0.0), &[1.0, 2.0], &options()).is_err());
    assert!(Dopri5::solve(&p, (0.0, 1.0), &[1.0], &options()).is_err());
    assert!(
        Dopri5::solve_observed(
            &p,
            (0.0, 1.0),
            &[1.0, 2.0],
            &options(),
            &[0.0, 0.8, 0.4, 1.0]
        )
        .is_err()
    );
    assert!(Dopri5::solve_observed(&p, (0.0, 1.0), &[1.0, 2.0], &options(), &[0.0, 1.5]).is_err());
}

#[test]
fn explicit_recoverable_trial_retries_and_counts_failed_user_work() {
    let once = Arc::new(AtomicU64::new(0));
    let entered = once.clone();
    let p = Problem::new(1, move |_, _, out| {
        let call = entered.fetch_add(1, Ordering::Relaxed);
        if call == 1 {
            return Err(CallbackError::Recoverable("trial domain".into()));
        }
        out[0] = 1.0;
        Ok(())
    });
    let r = Dopri5::solve(&p, (0.0, 1.0), &[0.0], &options())
        .unwrap()
        .ensure_success()
        .unwrap();
    assert!((r.final_state[0] - 1.0).abs() < 1e-12);
    assert_eq!(r.stats.callback_work.recoverable_errors, 1);
    assert_eq!(r.stats.callback_work.fatal_errors, 0);
    assert!(r.stats.rejected_steps >= 1);
    assert_eq!(r.stats.counters.rhs_calls, once.load(Ordering::Relaxed));
    assert_eq!(r.stats.rhs_completed + 1, r.stats.counters.rhs_calls);
}

#[test]
fn explicit_fatal_rhs_stops_once_and_retains_accepted_state() {
    let calls = Arc::new(AtomicU64::new(0));
    let entered = calls.clone();
    let p = Problem::new(1, move |t, _, out| {
        entered.fetch_add(1, Ordering::Relaxed);
        if t > 0.2 {
            return Err(CallbackError::Fatal("model unavailable".into()));
        }
        out[0] = 1.0;
        Ok(())
    });
    let r = Dopri5::solve_observed(&p, (0.0, 1.0), &[0.0], &options(), &[0.0, 0.1, 1.0]).unwrap();
    assert!(!r.success);
    assert!(r.final_time >= 0.1 && r.final_time <= 0.2);
    assert!((r.final_state[0] - r.final_time).abs() < 1e-14);
    assert_eq!(r.stats.callback_work.fatal_errors, 1);
    assert_eq!(r.stats.counters.rhs_calls, calls.load(Ordering::Relaxed));
    assert_eq!(
        r.callback_failure.as_ref().unwrap().message,
        "model unavailable"
    );
    match r.ensure_success() {
        Err(ExplicitSolveError::Failed { partial, .. }) => {
            assert!(!partial.success);
            assert!(partial.final_time < 1.0);
        }
        other => panic!("expected preserved partial run, got {other:?}"),
    }
}

#[test]
fn explicit_partial_or_nonfinite_ok_callback_is_fatal() {
    for write_nan in [false, true] {
        let p = Problem::new(2, move |_, _, out| {
            out[0] = 1.0;
            if write_nan {
                out[1] = f64::NAN;
            }
            Ok(())
        });
        let r = Dopri5::solve(&p, (0.0, 1.0), &[2.0, 3.0], &options()).unwrap();
        assert!(!r.success);
        assert_eq!(r.final_time, 0.0);
        assert_eq!(r.final_state, vec![2.0, 3.0]);
        assert_eq!(r.stats.callback_work.attempted, 1);
        assert_eq!(r.stats.callback_work.completed, 0);
        assert_eq!(r.stats.callback_work.malformed_outputs, 1);
        assert!(r.callback_failure.unwrap().malformed_output);
    }
}

#[test]
fn explicit_budget_and_zero_span_have_unambiguous_status() {
    let p = Problem::new(1, |_, _, out| {
        out[0] = 1.0;
        Ok(())
    });
    let opts = Options {
        max_steps: 1,
        h0: 0.01,
        ..options()
    };
    let r = Dopri5::solve(&p, (0.0, 1.0), &[0.0], &opts).unwrap();
    assert!(!r.success);
    assert_eq!(r.stats.attempts, 1);
    assert!((r.final_state[0] - r.final_time).abs() < 1e-14);
    assert!(r.final_time < 1.0);
    let p = Problem::new(1, |_, _, _| panic!("zero-span solve entered RHS"));
    let r = Dopri5::solve(&p, (3.0, 3.0), &[2.0], &options())
        .unwrap()
        .ensure_success()
        .unwrap();
    assert_eq!(r.final_time, 3.0);
    assert_eq!(r.final_state, vec![2.0]);
    assert_eq!(r.stats.counters.rhs_calls, 0);
}
