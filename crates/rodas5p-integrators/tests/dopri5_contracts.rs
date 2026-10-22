use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5Config, Dopri5StopReason, Dopri5Workspace, MaxStepPolicy,
    OutputSchedule, integrate_dopri5_observed,
};

fn schedule(end: f64) -> OutputSchedule {
    OutputSchedule::new(vec![0.0, end]).unwrap()
}
fn fixed_cap(h: f64) -> Dopri5Config {
    Dopri5Config {
        adaptive: AdaptiveStepConfig {
            initial_step: h,
            max_step: h,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn fixed_step_matches_independent_scalar_stability_polynomial() {
    let mut ws = Dopri5Workspace::new(1).unwrap();
    let mut endpoint = [0.0];
    let mut embedded = [0.0];
    let mut counters = WorkCounters::default();
    for z in [-2.0_f64, -0.4, 0.0, 0.2, 1.0] {
        ws.step_into(
            &|_, y, out| {
                out[0] = z * y[0];
                Ok(())
            },
            0.0,
            &[1.0],
            1.0,
            &mut endpoint,
            &mut embedded,
            &mut counters,
        )
        .unwrap();
        let exact = 1.0
            + z
            + z.powi(2) / 2.0
            + z.powi(3) / 6.0
            + z.powi(4) / 24.0
            + z.powi(5) / 120.0
            + z.powi(6) / 600.0;
        assert!(
            (endpoint[0] - exact).abs() < 3.0e-14,
            "z={z}, endpoint={:?}, polynomial={exact}",
            endpoint
        );
    }
    assert_eq!(counters.rhs_calls, 35);
    assert_eq!(counters.jacobian_builds, 0);
    assert_eq!(counters.jvp_calls, 0);
    assert_eq!(counters.linear_solves, 0);
    assert_eq!(counters.nonlinear_solves, 0);
}

#[test]
fn nonlinear_fixed_steps_converge_at_fifth_order() {
    let rhs = |_: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0] * y[0];
        Ok(())
    };
    let mut errors = Vec::new();
    for steps in [5_usize, 10, 20] {
        let h = 0.5 / steps as f64;
        let mut state = [1.0];
        let mut out = [0.0];
        let mut error = [0.0];
        let mut ws = Dopri5Workspace::new(1).unwrap();
        let mut counters = WorkCounters::default();
        for k in 0..steps {
            ws.step_into(
                &rhs,
                k as f64 * h,
                &state,
                h,
                &mut out,
                &mut error,
                &mut counters,
            )
            .unwrap();
            state = out;
        }
        errors.push((state[0] - 2.0).abs());
    }
    assert!(errors[0] / errors[1] > 20.0, "{errors:?}");
    assert!(errors[1] / errors[2] > 20.0, "{errors:?}");
}

#[test]
fn nonautonomous_solution_and_observation_landing_are_correct() {
    // y' = y+t, y(0)=1 => y(t)=2 exp(t)-t-1.
    let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[0] + t;
        Ok(())
    };
    let mut config = fixed_cap(0.2);
    config.adaptive.rtol = 1e-10;
    config.adaptive.atol = 1e-12;
    let times = vec![0.0, 0.017, 0.4, 0.81, 1.0];
    let run = integrate_dopri5_observed(
        &rhs,
        (0.0, 1.0),
        &[1.0],
        &config,
        &OutputSchedule::new(times.clone()).unwrap(),
    )
    .unwrap();
    assert!(run.observed.success, "{}", run.observed.message);
    assert_eq!(run.observed.t, times);
    for (&t, y) in run.observed.t.iter().zip(&run.observed.y) {
        assert!((y[0] - (2.0 * t.exp() - t - 1.0)).abs() < 1e-9);
    }
    assert_eq!(run.final_time, 1.0);
}

#[test]
fn fsal_and_rejected_base_reuse_have_exact_rhs_costs() {
    let rhs = |_: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -y[0];
        Ok(())
    };
    let mut config = fixed_cap(1.0);
    config.adaptive.rtol = 1e-10;
    config.adaptive.atol = 1e-12;
    let run = integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).unwrap();
    assert!(run.observed.success);
    assert!(run.rejected_steps > 0);
    assert_eq!(run.observed.counters.rhs_calls, 1 + 6 * run.attempts as u64);
    assert_eq!(run.fsal_reuses, run.attempts as u64 - 1);
    assert_eq!(run.rhs_completed, run.observed.counters.rhs_calls);
    config.fsal = false;
    let uncached =
        integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).unwrap();
    assert!(uncached.observed.success);
    assert_eq!(uncached.attempts, run.attempts);
    assert_eq!(uncached.final_state, run.final_state);
    assert_eq!(
        uncached.observed.counters.rhs_calls,
        7 * uncached.attempts as u64
    );
    assert_eq!(uncached.fsal_reuses, 0);
}

#[test]
fn vector_absolute_tolerance_retains_zero_components_and_scalar_parity() {
    let rhs = |_: f64, y: &[f64], out: &mut [f64]| {
        out[0] = 0.0;
        out[1] = -y[1];
        Ok(())
    };
    let mut config = fixed_cap(0.1);
    config.adaptive.rtol = 1e-8;
    config.adaptive.atol = 1e-11;
    let scalar =
        integrate_dopri5_observed(&rhs, (0.0, 1.0), &[0.0, 1.0], &config, &schedule(1.0)).unwrap();
    config.atol = Some(vec![1e-11, 1e-11]);
    let vector =
        integrate_dopri5_observed(&rhs, (0.0, 1.0), &[0.0, 1.0], &config, &schedule(1.0)).unwrap();
    assert_eq!(scalar.final_state, vector.final_state);
    assert_eq!(scalar.observed.counters, vector.observed.counters);
    config.atol = Some(vec![0.0, 1e-11]);
    let zero =
        integrate_dopri5_observed(&rhs, (0.0, 1.0), &[0.0, 1.0], &config, &schedule(1.0)).unwrap();
    assert!(zero.observed.success);
    assert_eq!(zero.final_state[0], 0.0);
    assert!((zero.final_state[1] - (-1.0_f64).exp()).abs() < 1e-8);
    config.atol = Some(vec![0.0, 0.0]);
    let stationary =
        integrate_dopri5_observed(&rhs, (0.0, 1.0), &[0.0, 0.0], &config, &schedule(1.0)).unwrap();
    assert!(stationary.observed.success);
    assert_eq!(stationary.final_state, vec![0.0, 0.0]);
}

#[test]
fn preflight_rejects_bad_dimensions_tolerances_and_clock_before_callbacks() {
    let calls = AtomicUsize::new(0);
    let rhs = |_: f64, _: &[f64], _: &mut [f64]| {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    };
    let mut config = Dopri5Config::default();
    assert!(integrate_dopri5_observed(&rhs, (0.0, 1.0), &[], &config, &schedule(1.0)).is_err());
    assert!(
        integrate_dopri5_observed(&rhs, (0.0, 1.0), &[f64::NAN], &config, &schedule(1.0)).is_err()
    );
    config.atol = Some(vec![1e-9, 1e-9]);
    assert!(integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).is_err());
    config.atol = Some(vec![-1e-9]);
    assert!(integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).is_err());
    config.atol = None;
    assert!(integrate_dopri5_observed(&rhs, (1.0, 0.0), &[1.0], &config, &schedule(1.0)).is_err());
    config.adaptive.rtol = 0.0;
    config.adaptive.atol = 0.0;
    assert!(integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).is_err());
    config.adaptive.atol = 1e-9;
    config.atol = Some(vec![0.0]);
    assert!(integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).is_err());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn callback_failure_preserves_last_accepted_state_and_attempted_work() {
    let calls = AtomicUsize::new(0);
    let rhs = |t: f64, _: &[f64], out: &mut [f64]| {
        calls.fetch_add(1, Ordering::Relaxed);
        if t > 0.15 {
            return Err(CoreError::InvalidInput("model callback stopped".into()));
        }
        out[0] = 1.0;
        Ok(())
    };
    let run = integrate_dopri5_observed(&rhs, (0.0, 0.5), &[0.0], &fixed_cap(0.1), &schedule(0.5))
        .unwrap();
    assert!(!run.observed.success);
    assert_eq!(run.stop_reason, Some(Dopri5StopReason::Callback));
    assert_eq!(run.final_time, 0.1);
    assert!((run.final_state[0] - 0.1).abs() < 1e-15);
    assert_eq!(run.observed.t, vec![0.0]);
    assert_eq!(
        run.observed.counters.rhs_calls,
        calls.load(Ordering::Relaxed) as u64
    );
    assert_eq!(run.rhs_completed + 1, run.observed.counters.rhs_calls);
    assert_eq!(run.attempts, run.accepted_steps + run.rejected_steps);
}

#[test]
fn incomplete_or_nonfinite_successful_callbacks_are_fatal() {
    let mut config = fixed_cap(0.1);
    config.recover_nonfinite_rhs = true;
    let incomplete = |_: f64, _: &[f64], out: &mut [f64]| {
        out[0] = 1.0;
        Ok(())
    };
    let run = integrate_dopri5_observed(
        &incomplete,
        (0.0, 1.0),
        &[0.0, 0.0],
        &config,
        &schedule(1.0),
    )
    .unwrap();
    assert_eq!(run.stop_reason, Some(Dopri5StopReason::NonFinite));
    assert_eq!(run.attempts, 1);
    assert_eq!(run.observed.counters.rhs_calls, 1);
    assert_eq!(run.final_state, vec![0.0, 0.0]);
    let nonfinite = |_: f64, _: &[f64], out: &mut [f64]| {
        out.fill(f64::INFINITY);
        Ok(())
    };
    let run =
        integrate_dopri5_observed(&nonfinite, (0.0, 1.0), &[0.0], &config, &schedule(1.0)).unwrap();
    assert_eq!(run.attempts, 1);
    assert_eq!(run.rhs_completed, 0);
}

#[test]
fn explicitly_recoverable_domain_error_retries_with_work_retained() {
    let calls = AtomicUsize::new(0);
    let rhs = |_: f64, y: &[f64], out: &mut [f64]| {
        calls.fetch_add(1, Ordering::Relaxed);
        if y[0] < 0.0 {
            return Err(CoreError::NonFinite("outside positive domain".into()));
        }
        out[0] = -10.0 * y[0];
        Ok(())
    };
    let mut config = fixed_cap(1.0);
    config.recover_nonfinite_rhs = true;
    let run = integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &config, &schedule(1.0)).unwrap();
    assert!(run.observed.success, "{}", run.observed.message);
    assert!(run.rejected_steps > 0);
    assert!(run.observed.counters.nonfinite_step_failures > 0);
    assert_eq!(
        run.observed.counters.rhs_calls,
        calls.load(Ordering::Relaxed) as u64
    );
    assert!((run.final_state[0] - (-10.0_f64).exp()).abs() < 1e-8);
    assert_eq!(run.attempts, run.accepted_steps + run.rejected_steps);
}

#[test]
fn attempt_budget_is_a_partial_result() {
    let mut config = fixed_cap(0.1);
    config.adaptive.max_attempts = 1;
    let rhs = |_: f64, _: &[f64], out: &mut [f64]| {
        out[0] = 1.0;
        Ok(())
    };
    let run = integrate_dopri5_observed(&rhs, (0.0, 1.0), &[0.0], &config, &schedule(1.0)).unwrap();
    assert_eq!(run.stop_reason, Some(Dopri5StopReason::AttemptBudget));
    assert_eq!(run.final_time, 0.1);
    assert_eq!(run.attempts, 1);
    assert_eq!(run.observed.counters.rhs_calls, 7);
}

#[test]
fn represented_time_landing_and_strict_cap_follow_shared_clock_contract() {
    let start = 1e12_f64;
    let ulp = start.next_up() - start;
    let end = start + 4.0 * ulp;
    let times: Vec<_> = (0..=4).map(|k| start + k as f64 * ulp).collect();
    let output = OutputSchedule::new(times.clone()).unwrap();
    let rhs = |_: f64, _: &[f64], out: &mut [f64]| {
        out[0] = 1.0;
        Ok(())
    };
    let mut config = fixed_cap(1e-4);
    let run = integrate_dopri5_observed(&rhs, (start, end), &[0.0], &config, &output).unwrap();
    assert!(run.observed.success);
    assert_eq!(run.observed.t, times);
    assert!((run.final_state[0] - (end - start)).abs() < 1e-18);
    config.adaptive.max_step_policy = MaxStepPolicy::StrictRepresentedCap;
    let run = integrate_dopri5_observed(&rhs, (start, end), &[0.0], &config, &output).unwrap();
    assert_eq!(run.stop_reason, Some(Dopri5StopReason::StepUnderflow));
    assert_eq!(run.observed.counters.rhs_calls, 0);
}

#[test]
fn zero_interval_returns_initial_state_without_rhs() {
    let rhs =
        |_: f64, _: &[f64], _: &mut [f64]| -> Result<(), CoreError> { panic!("zero interval RHS") };
    let run = integrate_dopri5_observed(
        &rhs,
        (2.0, 2.0),
        &[3.0],
        &Dopri5Config::default(),
        &OutputSchedule::new(vec![2.0]).unwrap(),
    )
    .unwrap();
    assert!(run.observed.success);
    assert_eq!(run.final_state, vec![3.0]);
    assert_eq!(run.attempts, 0);
}
