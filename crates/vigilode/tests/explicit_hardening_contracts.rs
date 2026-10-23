use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vigilode::{AffineErrorNorm, Atol, CallbackError, Dopri5, Options, Problem};

fn opts() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 0.03,
        ..Options::default()
    }
}

#[test]
fn dense_observations_preserve_trajectory_and_count_actual_work() {
    let problem = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    });
    let times: Vec<_> = (0..=1000).map(|i| i as f64 / 1000.0).collect();
    let sampled = Dopri5::solve_dense_observed(&problem, (0.0, 1.0), &[1.0], &opts(), &times)
        .unwrap()
        .ensure_success()
        .unwrap();
    let sparse = Dopri5::solve_dense_observed(&problem, (0.0, 1.0), &[1.0], &opts(), &[0.0, 1.0])
        .unwrap()
        .ensure_success()
        .unwrap();
    assert_eq!(sampled.final_state, sparse.final_state);
    assert_eq!(sampled.stats.counters, sparse.stats.counters);
    assert_eq!(sampled.t, times);
    assert_eq!(sampled.stats.observation_mode, "quartic-dense-observations");
    assert_eq!(sampled.stats.error_control.mode, "state-wrms");
    assert_eq!(
        sampled.stats.callback_work.completed,
        sampled.stats.rhs_completed
    );
    assert_eq!(
        sampled.stats.callback_work.attempted,
        sampled.stats.counters.rhs_calls
    );
    assert_eq!(sampled.stats.output_clipped_steps, 0);
    for (&t, y) in sampled.t.iter().zip(&sampled.y) {
        assert!((y[0] - (-t).exp()).abs() < 1e-7);
    }
}

#[test]
fn physical_norm_is_the_executed_tolerance_and_keeps_reduced_outputs() {
    let problem = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    });
    let loose = Options {
        rtol: 1e-2,
        atol: Atol::Scalar(1e-4),
        h0: 0.3,
        ..opts()
    };
    let norm = AffineErrorNorm::new(
        1,
        vec![0.0, 3.0],
        vec![1.0, 100.0],
        vec![1e-12, 1e-10],
        1e-9,
    )
    .unwrap();
    let plain = Dopri5::solve(&problem, (0.0, 2.0), &[1.0], &loose)
        .unwrap()
        .ensure_success()
        .unwrap();
    for dense in [false, true] {
        let result = if dense {
            Dopri5::solve_dense_observed_with_norm(
                &problem,
                (0.0, 2.0),
                &[1.0],
                &loose,
                &[0.0, 0.7, 2.0],
                &norm,
            )
        } else {
            Dopri5::solve_observed_with_norm(
                &problem,
                (0.0, 2.0),
                &[1.0],
                &loose,
                &[0.0, 0.7, 2.0],
                &norm,
            )
        }
        .unwrap()
        .ensure_success()
        .unwrap();
        assert!(result.stats.accepted_steps > plain.stats.accepted_steps);
        assert_eq!(result.stats.error_control.mode, "affine-physical-wrms");
        assert_eq!(result.stats.error_control.dimension, 2);
        assert_eq!(result.stats.error_control.atol, vec![1e-12, 1e-10]);
        assert_eq!(result.stats.error_control.rtol, 1e-9);
        assert_eq!(result.stats.options.rtol, 1e-2);
        assert!(result.y.iter().all(|y| y.len() == 1));
        for (&t, y) in result.t.iter().zip(&result.y) {
            assert!((y[0] - (-t).exp()).abs() < 5e-9);
        }
    }
}

#[test]
fn invalid_physical_dimension_and_schedule_enter_no_callbacks() {
    let problem = Problem::new(1, |_, _, _| panic!("invalid input reached callback"));
    let norm = AffineErrorNorm::new(2, vec![0.0], vec![1.0, 1.0], vec![1e-9], 1e-6).unwrap();
    assert!(
        Dopri5::solve_observed_with_norm(&problem, (0.0, 1.0), &[1.0], &opts(), &[0.0, 1.0], &norm)
            .is_err()
    );
    for times in [
        vec![0.0, 0.7, 0.5, 1.0],
        vec![0.0, 0.5, 0.5, 1.0],
        vec![0.1, 1.0],
    ] {
        assert!(
            Dopri5::solve_dense_observed(&problem, (0.0, 1.0), &[1.0], &opts(), &times).is_err()
        );
    }
}

#[test]
fn removed_facade_poison_preserves_malformed_output_detection() {
    let calls = Arc::new(AtomicUsize::new(0));
    let c = calls.clone();
    let problem = Problem::new(2, move |_, y, out| {
        let k = c.fetch_add(1, Ordering::Relaxed);
        out[0] = -y[0];
        if k == 0 {
            out[1] = -y[1];
        }
        Ok(())
    });
    let result =
        Dopri5::solve_dense_observed(&problem, (0.0, 1.0), &[1.0, 2.0], &opts(), &[0.0, 0.5, 1.0])
            .unwrap();
    assert!(!result.success);
    assert_eq!(result.final_state, vec![1.0, 2.0]);
    assert_eq!(result.final_time, 0.0);
    assert_eq!(result.stats.callback_work.malformed_outputs, 1);
    assert_eq!(result.stats.callback_work.completed, 1);
    assert_eq!(result.stats.callback_work.attempted, 2);
    assert!(result.callback_failure.unwrap().malformed_output);
}

#[test]
fn dense_fatal_callback_keeps_only_accepted_outputs_and_partial_state() {
    let problem = Problem::new(1, |t, _, out| {
        if t > 0.4 {
            return Err(CallbackError::Fatal("domain ended".into()));
        }
        out[0] = 1.0;
        Ok(())
    });
    let result = Dopri5::solve_dense_observed(
        &problem,
        (0.0, 1.0),
        &[0.0],
        &opts(),
        &[0.0, 0.01, 0.1, 0.5, 1.0],
    )
    .unwrap();
    assert!(!result.success);
    assert!(result.final_time > 0.0 && result.final_time <= 0.4);
    assert_eq!(*result.t.last().unwrap(), result.final_time);
    assert_eq!(*result.y.last().unwrap(), result.final_state);
    assert!(result.t.iter().all(|t| *t <= result.final_time));
    assert!(result.ensure_success().is_err());
}
