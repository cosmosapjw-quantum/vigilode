use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingMethod, SwitchingSolver,
};

fn opts() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 0.002,
        max_steps: 50_000,
        ..Default::default()
    }
}

fn stiff() -> Problem {
    Problem::new(1, |t, y, dy| {
        dy[0] = -10_000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10_000.0;
        Ok(())
    }))
    .time_derivative(|t, _, f| {
        f[0] = -10_000.0 * t.sin() - t.cos();
        Ok(())
    })
}

#[test]
fn default_is_rhs_only_even_when_derivatives_are_declared() {
    let p = Problem::new(2, |t, y, dy| {
        dy[0] = -y[0];
        dy[1] = t.cos();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("unauthorized Jacobian")))
    .time_derivative(|_, _, _| panic!("unauthorized df/dt"));
    let c = SwitchingConfig {
        pressure_steps: 1,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &opts(), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!((r.final_state[0] - (-1.0f64).exp()).abs() < 2e-7);
    assert!((r.final_state[1] - 1.0f64.sin()).abs() < 2e-7);
    assert_eq!(
        r.stats.jacobian_builds + r.stats.jvp_vectors + r.stats.ft_calls + r.stats.linear_solves,
        0
    );
    assert_eq!(r.stats.switches, 0);
    assert!(
        r.stats
            .segments
            .iter()
            .all(|s| s.method == SwitchingMethod::Dopri5)
    );
}

#[test]
fn stiff_work_pressure_enters_authorized_fallback_from_accepted_state() {
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        explicit_attempt_budget: 12,
        pressure_steps: 10,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&stiff(), (0.0, 1.0), &[1.0], &opts(), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(
        (r.final_state[0] - 1.0f64.cos()).abs() < 1e-6,
        "{:?}",
        r.final_state
    );
    assert!(r.stats.switches > 0);
    assert!(r.stats.jacobian_builds > 0);
    assert!(
        r.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p)
    );
    assert_eq!(r.stats.segments[0].method, SwitchingMethod::Dopri5);
    assert!(r.t.windows(2).all(|w| w[1] > w[0]));
    let mut t = 0.0;
    for s in &r.stats.segments {
        assert_eq!(s.start_time, t);
        if s.committed {
            t = s.reached_time;
        }
    }
    assert_eq!(t, 1.0);
}

#[test]
fn fatal_callback_is_not_retried_with_another_method() {
    let calls = Arc::new(AtomicU64::new(0));
    let seen = calls.clone();
    let p = Problem::new(1, move |t, _, dy| {
        seen.fetch_add(1, Ordering::Relaxed);
        if t > 0.03 {
            return Err(CallbackError::Fatal("stop now".into()));
        }
        dy[0] = 1.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("fallback after fatal")))
    .time_derivative(|_, _, _| panic!("time derivative after fatal"));
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[0.0], &opts(), &c).unwrap();
    assert!(!r.success);
    assert_eq!(r.stats.rhs_calls, calls.load(Ordering::Relaxed));
    assert_eq!(r.stats.jacobian_builds, 0);
    assert!(r.final_time <= 0.03);
    assert!((r.final_state[0] - r.final_time).abs() < 1e-14);
}

#[test]
fn finite_difference_requires_its_separate_opt_in() {
    let p = Problem::new(1, |_, _, _| panic!("invalid policy entered RHS"));
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        ..Default::default()
    };
    assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts(), &c).is_err());
    let p = p.jacobian(Jacobian::dense(|_, _, _| {
        panic!("unprovided df/dt entered Jacobian")
    }));
    assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts(), &c).is_err());
    let p = Problem::new(1, |t, y, dy| {
        dy[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    });
    let c = SwitchingConfig {
        fallback: DerivativeFallback::AllowFiniteDifference,
        explicit_attempt_budget: 8,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 0.2), &[1.0], &opts(), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(r.stats.jacobian_builds > 0);
}

#[test]
fn all_attempts_and_discarded_probes_share_one_budget() {
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        explicit_attempt_budget: 6,
        probe_attempt_budget: 1,
        implicit_residence: 1,
        probe_interval: 1,
        ..Default::default()
    };
    let o = Options {
        max_steps: 160,
        ..opts()
    };
    let r = SwitchingSolver::solve(&stiff(), (0.0, 1.0), &[1.0], &o, &c).unwrap();
    assert!(r.stats.attempts <= o.max_steps);
    assert!(r.stats.segments.iter().any(|s| s.is_probe && !s.committed));
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
    assert_eq!(
        r.stats.rhs_calls,
        r.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
    );
    assert_eq!(
        r.stats.jacobian_builds,
        r.stats
            .segments
            .iter()
            .map(|s| s.jacobian_builds)
            .sum::<u64>()
    );
    assert_eq!(
        r.stats.accepted_steps + r.stats.rejected_steps,
        r.stats.attempts
    );
    assert!(r.stats.accounting_complete);
}

#[test]
fn bounded_probe_can_return_to_the_rhs_only_route() {
    let p = Problem::new(1, |_, y, dy| {
        dy[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -1.0;
        Ok(())
    }))
    .autonomous();
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 12,
        pressure_steps: 1,
        implicit_residence: 2,
        probe_interval: 2,
        probe_attempt_budget: 16,
        return_cost_ratio: 0.99,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.2), &[1.0], &opts(), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(
        r.stats.segments.iter().any(|s| s.is_probe && s.committed),
        "{:?}",
        r.stats.segments
    );
    assert!(r.stats.switches >= 2);
    assert!((r.final_state[0] - (-1.2f64).exp()).abs() < 2e-7);
}

#[test]
fn budget_failure_preserves_accepted_state_and_never_forges_success() {
    let o = Options {
        max_steps: 3,
        ..opts()
    };
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        explicit_attempt_budget: 2,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&stiff(), (0.0, 1.0), &[1.0], &o, &c).unwrap();
    assert!(!r.success);
    assert_eq!(r.stats.attempts, 3);
    assert_eq!(r.final_time, *r.t.last().unwrap());
    assert_eq!(r.final_state, *r.y.last().unwrap());
}

#[test]
fn malformed_output_stops_before_fallback() {
    let p = Problem::new(2, |_, _, dy| {
        dy[0] = 0.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| {
        panic!("malformed output fell back")
    }))
    .autonomous();
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 1.0], &opts(), &c).unwrap();
    assert!(!r.success);
    assert_eq!(r.final_time, 0.0);
    assert_eq!(r.stats.jacobian_builds, 0);
}

#[test]
fn request_preflight_rejects_unsupported_and_resource_contracts_without_callbacks() {
    let p = Problem::new(1, |_, _, _| panic!("invalid input entered RHS"));
    for c in [
        SwitchingConfig {
            window_count: 0,
            ..Default::default()
        },
        SwitchingConfig {
            return_cost_ratio: f64::NAN,
            ..Default::default()
        },
        SwitchingConfig {
            probe_attempt_budget: 0,
            ..Default::default()
        },
    ] {
        assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts(), &c).is_err());
    }
    assert!(
        SwitchingSolver::solve(
            &p.clone().mass(vec![1.0]),
            (0.0, 1.0),
            &[1.0],
            &opts(),
            &Default::default()
        )
        .is_err()
    );
    assert!(
        SwitchingSolver::solve(
            &p,
            (0.0, 1.0),
            &[1.0],
            &Options {
                dense_output: true,
                ..opts()
            },
            &Default::default()
        )
        .is_err()
    );
    assert!(
        SwitchingSolver::solve(
            &p,
            (-f64::MAX, f64::MAX),
            &[1.0],
            &opts(),
            &Default::default()
        )
        .is_err()
    );
    let p = Problem::new(65, |_, _, _| panic!("dense cap entered RHS"));
    let c = SwitchingConfig {
        fallback: DerivativeFallback::AllowFiniteDifference,
        ..Default::default()
    };
    assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &vec![0.0; 65], &opts(), &c).is_err());
}

#[test]
fn zero_span_and_representationally_tiny_windows_do_not_loop() {
    let p = Problem::new(1, |_, _, dy| {
        dy[0] = 1.0;
        Ok(())
    });
    let r = SwitchingSolver::solve(&p, (2.0, 2.0), &[3.0], &opts(), &Default::default()).unwrap();
    assert!(r.success);
    assert_eq!(r.stats.attempts, 0);
    assert_eq!(r.final_state, vec![3.0]);
    let end = f64::from_bits(1.0f64.to_bits() + 4);
    let r = SwitchingSolver::solve(&p, (1.0, end), &[0.0], &opts(), &Default::default()).unwrap();
    // The underlying integrator may refuse its minimum step; either result
    // retains a finite, strictly monotone accepted trajectory.
    assert!(r.t.windows(2).all(|w| w[0] < w[1]));
    assert!(r.stats.attempts <= opts().max_steps);
}

#[test]
fn endpoint_accuracy_for_rotation_is_checked_separately_from_stiffness_routing() {
    let w = 80.0;
    let p = Problem::new(2, move |_, y, dy| {
        dy[0] = -w * y[1];
        dy[1] = w * y[0];
        Ok(())
    });
    let r =
        SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &opts(), &Default::default()).unwrap();
    assert!(r.success, "{}", r.message);
    assert!((r.final_state[0] - w.cos()).hypot(r.final_state[1] - w.sin()) < 3e-6);
    assert_eq!(r.stats.jvp_vectors + r.stats.jacobian_builds, 0);
}

#[test]
fn serialization_preserves_route_and_original_driver_records() {
    let p = Problem::new(1, |_, y, dy| {
        dy[0] = -y[0];
        Ok(())
    });
    let r = SwitchingSolver::solve(&p, (0.0, 0.1), &[1.0], &opts(), &Default::default()).unwrap();
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["schema"], "vigilode.switching.v1");
    assert_eq!(j["config"]["fallback"], "forbidden");
    assert_eq!(j["stats"]["segments"][0]["driver"]["method"], "dopri5");
}
