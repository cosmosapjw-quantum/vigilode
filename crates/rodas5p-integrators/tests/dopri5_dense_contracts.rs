use std::cell::Cell;

use rodas5p_core::CoreError;
use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5Config, Dopri5StopReason, MaxStepPolicy, OutputSchedule,
    integrate_dopri5_dense_observed, integrate_dopri5_observed,
};

fn config(h: f64, rtol: f64, atol: f64) -> Dopri5Config {
    Dopri5Config {
        adaptive: AdaptiveStepConfig {
            initial_step: h,
            max_step: h,
            rtol,
            atol,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn dense_observations_do_not_change_steps_rhs_or_final_state() {
    // Nonautonomous analytic holdout, not a polynomial exactness fixture.
    let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0] + t;
        Ok(())
    };
    let cfg = config(0.2, 1e-10, 1e-12);
    let sparse = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let dense = OutputSchedule::new((0..=1000).map(|i| i as f64 / 1000.0).collect()).unwrap();
    let baseline = integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &cfg, &sparse).unwrap();
    let samples = integrate_dopri5_dense_observed(&rhs, (0.0, 1.0), &[1.0], &cfg, &dense).unwrap();
    assert!(baseline.observed.success && samples.observed.success);
    assert_eq!(samples.final_state, baseline.final_state);
    assert_eq!(samples.attempts, baseline.attempts);
    assert_eq!(samples.accepted_steps, baseline.accepted_steps);
    assert_eq!(samples.rejected_steps, baseline.rejected_steps);
    assert_eq!(
        samples.observed.counters.rhs_calls,
        baseline.observed.counters.rhs_calls
    );
    assert_eq!(samples.observed.output_clipped_steps, 0);
    assert_eq!(samples.observed.t, dense.times());
    assert_eq!(samples.observed.y.first().unwrap(), &[1.0]);
    assert_eq!(samples.observed.y.last().unwrap(), &baseline.final_state);
    for (&t, y) in samples.observed.t.iter().zip(&samples.observed.y) {
        assert!((y[0] - (2.0 * t.exp() - t - 1.0)).abs() < 1e-9, "t={t}");
    }
}

#[test]
fn dense_extension_integrates_cubic_time_polynomials_at_interior_points() {
    let schedule = OutputSchedule::new(vec![0.0, 0.07, 0.37, 0.81, 1.0]).unwrap();
    for degree in 0..=3_i32 {
        let rhs = |t: f64, _: &[f64], out: &mut [f64]| {
            out[0] = t.powi(degree);
            Ok(())
        };
        let run = integrate_dopri5_dense_observed(
            &rhs,
            (0.0, 1.0),
            &[0.0],
            &config(1.0, 1.0, 1.0),
            &schedule,
        )
        .unwrap();
        assert!(run.observed.success);
        assert_eq!(run.accepted_steps, 1);
        assert_eq!(run.observed.counters.rhs_calls, 7);
        for (&t, y) in run.observed.t.iter().zip(&run.observed.y) {
            let exact = t.powi(degree + 1) / (degree + 1) as f64;
            assert!(
                (y[0] - exact).abs() < 2e-14,
                "degree={degree}, t={t}, got={:?}",
                y
            );
        }
    }
}

#[test]
fn dense_interior_error_has_fifth_power_scaling() {
    // The quartic extension is not exact for this degree-five solution.
    // This checks interpolation's own local error, not merely endpoints.
    let rhs = |t: f64, _: &[f64], out: &mut [f64]| {
        out[0] = t.powi(4);
        Ok(())
    };
    let mut errors = Vec::new();
    for h in [0.4_f64, 0.2, 0.1] {
        let query = 0.37 * h;
        let schedule = OutputSchedule::new(vec![0.0, query, h]).unwrap();
        let run = integrate_dopri5_dense_observed(
            &rhs,
            (0.0, h),
            &[0.0],
            &config(h, 1.0, 1.0),
            &schedule,
        )
        .unwrap();
        assert!(run.observed.success);
        assert_eq!(run.accepted_steps, 1);
        errors.push((run.observed.y[1][0] - query.powi(5) / 5.0).abs());
        assert!((run.final_state[0] - h.powi(5) / 5.0).abs() < 1e-15);
    }
    for pair in errors.windows(2) {
        assert!((pair[0] / pair[1] - 32.0).abs() < 1e-7, "{errors:?}");
    }
}

#[test]
fn dense_nonlinear_interior_convergence_uses_the_actual_stage_field() {
    let rhs = |_: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0] * y[0];
        Ok(())
    };
    let mut errors = Vec::new();
    for h in [0.1_f64, 0.05, 0.025] {
        let query = 0.37 * h;
        let schedule = OutputSchedule::new(vec![0.0, query, h]).unwrap();
        let run = integrate_dopri5_dense_observed(
            &rhs,
            (0.0, h),
            &[1.0],
            &config(h, 1.0, 1.0),
            &schedule,
        )
        .unwrap();
        assert!(run.observed.success);
        errors.push((run.observed.y[1][0] - 1.0 / (1.0 - query)).abs());
    }
    for pair in errors.windows(2) {
        assert!(pair[0] / pair[1] > 20.0, "{errors:?}");
    }
}

#[test]
fn dense_callback_failure_keeps_only_outputs_of_accepted_intervals() {
    let rhs = |t: f64, _: &[f64], out: &mut [f64]| {
        if t > 0.31 {
            return Err(CoreError::InvalidInput("deliberate fatal callback".into()));
        }
        out[0] = 1.0;
        Ok(())
    };
    let schedule = OutputSchedule::new(vec![0.0, 0.03, 0.17, 0.25, 0.33, 0.6]).unwrap();
    let run = integrate_dopri5_dense_observed(
        &rhs,
        (0.0, 0.6),
        &[0.0],
        &config(0.1, 1e-8, 1e-10),
        &schedule,
    )
    .unwrap();
    assert!(!run.observed.success);
    assert_eq!(run.stop_reason, Some(Dopri5StopReason::Callback));
    assert!(run.final_time > 0.25 && run.final_time < 0.33);
    assert_eq!(run.observed.t, vec![0.0, 0.03, 0.17, 0.25]);
    assert!((run.final_state[0] - run.final_time).abs() < 1e-14);
    for (&t, y) in run.observed.t.iter().zip(&run.observed.y) {
        assert!(t <= run.final_time);
        assert!((y[0] - t).abs() < 1e-14);
    }
}

#[test]
fn dense_invalid_inputs_are_rejected_before_rhs() {
    let calls = Cell::new(0_usize);
    let rhs = |_: f64, _: &[f64], out: &mut [f64]| {
        calls.set(calls.get() + 1);
        out[0] = 1.0;
        Ok(())
    };
    for times in [vec![0.1, 1.0], vec![0.0, 0.9], vec![1.0]] {
        let schedule = OutputSchedule::new(times).unwrap();
        assert!(
            integrate_dopri5_dense_observed(
                &rhs,
                (0.0, 1.0),
                &[0.0],
                &config(0.1, 1e-6, 1e-9),
                &schedule
            )
            .is_err()
        );
    }
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    assert!(
        integrate_dopri5_dense_observed(
            &rhs,
            (1.0, 0.0),
            &[0.0],
            &config(0.1, 1e-6, 1e-9),
            &schedule
        )
        .is_err()
    );
    assert!(OutputSchedule::new(vec![0.0, 0.1, 0.1, 1.0]).is_err());
    assert!(OutputSchedule::new(vec![0.0, f64::NAN, 1.0]).is_err());
    assert_eq!(calls.get(), 0);
}

#[test]
fn dense_zero_span_returns_initial_state_without_rhs() {
    let rhs = |_: f64, _: &[f64], _: &mut [f64]| -> Result<(), CoreError> { panic!("no calls") };
    let schedule = OutputSchedule::new(vec![3.0]).unwrap();
    let run = integrate_dopri5_dense_observed(
        &rhs,
        (3.0, 3.0),
        &[7.0],
        &config(0.1, 1e-6, 1e-9),
        &schedule,
    )
    .unwrap();
    assert!(run.observed.success);
    assert_eq!(run.observed.t, vec![3.0]);
    assert_eq!(run.observed.y, vec![vec![7.0]]);
    assert_eq!(run.attempts, 0);
}

#[test]
fn dense_represented_clock_keeps_adjacent_requests_distinct() {
    let start = 1e12_f64;
    let ulp = start.next_up() - start;
    let end = start + 4.0 * ulp;
    let schedule = OutputSchedule::new((0..=4).map(|i| start + i as f64 * ulp).collect()).unwrap();
    let rhs = |_: f64, _: &[f64], out: &mut [f64]| {
        out[0] = 1.0;
        Ok(())
    };
    let mut cfg = config(2.0 * ulp, 1e-8, 1e-10);
    cfg.adaptive.min_step = 1e-10;
    cfg.adaptive.max_step_policy = MaxStepPolicy::StrictRepresentedCap;
    let run = integrate_dopri5_dense_observed(&rhs, (start, end), &[0.0], &cfg, &schedule).unwrap();
    assert!(run.observed.success, "{}", run.observed.message);
    assert_eq!(run.accepted_steps, 2);
    assert_eq!(run.observed.t, schedule.times());
    for (&t, y) in run.observed.t.iter().zip(&run.observed.y) {
        assert!((y[0] - (t - start)).abs() < 1e-17);
    }
}

#[test]
fn dense_cross_zero_interval_accepts_query_whose_theta_rounds_to_one() {
    let start = -1.0e16_f64;
    let end = 2.0_f64;
    let query = end.next_down();
    let h = end - start;
    assert_eq!((query - start) / h, 1.0);
    let schedule = OutputSchedule::new(vec![start, query, end]).unwrap();
    let rhs = |_: f64, _: &[f64], out: &mut [f64]| {
        out[0] = 1.0e-16;
        Ok(())
    };
    let run = integrate_dopri5_dense_observed(
        &rhs,
        (start, end),
        &[-1.0],
        &config(h, 1.0, 1.0),
        &schedule,
    )
    .unwrap();
    assert!(run.observed.success, "{}", run.observed.message);
    assert_eq!(run.accepted_steps, 1);
    assert_eq!(run.observed.t, schedule.times());
    // Exact endpoint anchoring is the appropriate local floating-point
    // contract. The large starting-state cancellation limits absolute
    // trajectory accuracy separately to ordinary floating-point roundoff.
    let anchored = run.final_state[0] - (end - query) * 1.0e-16;
    assert!((run.observed.y[1][0] - anchored).abs() < 1.0e-29);
    assert!((run.observed.y[1][0] - query * 1.0e-16).abs() < 3.0e-15);
}
