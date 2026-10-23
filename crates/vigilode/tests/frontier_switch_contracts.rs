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
        h0: 0.0001,
        max_steps: 100_000,
        ..Default::default()
    }
}
fn screened() -> SwitchingConfig {
    SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        implicit_pilot_attempt_budget: 12,
        implicit_pilot_limit: 1,
        explicit_attempt_budget: 32,
        pressure_steps: 24,
        ..Default::default()
    }
}
fn rotation() -> Problem {
    Problem::new(2, |_, y, out| {
        out[0] = -200.0 * y[1];
        out[1] = 200.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, out| {
        out[0] = -200.0 * v[1];
        out[1] = 200.0 * v[0];
        Ok(())
    }))
    .autonomous()
}
#[test]
fn rapid_rotation_rejects_uneconomical_jvp_entry_and_retains_accuracy() {
    let r =
        SwitchingSolver::solve(&rotation(), (0.0, 0.2), &[1.0, 0.0], &opts(), &screened()).unwrap();
    assert!(r.success, "{}", r.message);
    assert_eq!(r.stats.switches, 0);
    let pilots: Vec<_> = r
        .stats
        .segments
        .iter()
        .filter(|s| s.is_probe && s.method == SwitchingMethod::Rodas5p)
        .collect();
    assert_eq!(pilots.len(), 1);
    assert!(!pilots[0].committed);
    assert!(pilots[0].attempts <= 12);
    assert!(
        r.stats
            .segments
            .iter()
            .filter(|s| s.committed)
            .all(|s| s.method == SwitchingMethod::Dopri5)
    );
    assert!((r.final_state[0] - 40.0_f64.cos()).hypot(r.final_state[1] - 40.0_f64.sin()) < 2e-6);
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
    assert_eq!(
        r.stats.rhs_calls,
        r.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
    );
}
#[test]
fn stiff_decay_pilot_must_finish_the_same_prefix_before_entry() {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -10_000.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10_000.0;
        Ok(())
    }))
    .autonomous();
    let c = SwitchingConfig {
        implicit_pilot_attempt_budget: 64,
        explicit_attempt_budget: 256,
        pressure_steps: 32,
        ..screened()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 0.2), &[1.0], &opts(), &c).unwrap();
    assert!(r.success, "{}", r.message);
    let pilot_index = r
        .stats
        .segments
        .iter()
        .position(|s| s.is_probe && s.method == SwitchingMethod::Rodas5p)
        .unwrap();
    let pilot = &r.stats.segments[pilot_index];
    let explicit = &r.stats.segments[pilot_index + 1];
    assert_eq!(pilot.start_time, explicit.start_time);
    assert_eq!(pilot.target_time, explicit.reached_time);
    if r.stats.switches > 0 {
        assert!(pilot.success);
        assert!(pilot.work_units <= c.entry_cost_ratio * explicit.work_units);
    }
    assert!(r.final_state[0].abs() < 1e-8);
}
#[test]
fn implicit_pilot_failure_is_fatal_and_counts_every_callback() {
    let rhs_calls = Arc::new(AtomicU64::new(0));
    let seen = rhs_calls.clone();
    let jac_calls = Arc::new(AtomicU64::new(0));
    let jac_seen = jac_calls.clone();
    let p = Problem::new(1, move |_, y, out| {
        seen.fetch_add(1, Ordering::Relaxed);
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_, _, _| {
        jac_seen.fetch_add(1, Ordering::Relaxed);
        Err(CallbackError::Fatal("pilot stop".into()))
    }))
    .autonomous();
    let c = SwitchingConfig {
        pressure_steps: 1,
        ..screened()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts(), &c).unwrap();
    assert!(!r.success);
    assert!(r.message.contains("fatal implicit pilot"));
    assert_eq!(jac_calls.load(Ordering::Relaxed), 1);
    assert_eq!(r.stats.rhs_calls, rhs_calls.load(Ordering::Relaxed));
    // The native fatal signal stops the pilot on its first failed build.
    assert_eq!(r.stats.jacobian_builds, 1);
    assert!(r.final_time > 0.0 && r.final_time < 1.0);
    assert!((r.final_state[0] - (-r.final_time).exp()).abs() < 1e-7);
    assert_eq!(r.stats.switches, 0);
}
#[test]
fn pilot_shares_global_budget_and_no_partial_pilot_state_is_committed() {
    let o = Options {
        max_steps: 8,
        ..opts()
    };
    let c = SwitchingConfig {
        explicit_attempt_budget: 3,
        implicit_pilot_attempt_budget: 20,
        pressure_steps: 1,
        ..screened()
    };
    let r = SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &o, &c).unwrap();
    assert!(!r.success);
    assert_eq!(r.stats.attempts, 8);
    assert!(
        r.stats
            .segments
            .iter()
            .filter(|s| s.is_probe)
            .all(|s| !s.committed)
    );
    assert_eq!(r.final_time, *r.t.last().unwrap());
    assert_eq!(r.final_state, *r.y.last().unwrap());
}
#[test]
fn screening_configuration_is_checked_before_callbacks() {
    let p = Problem::new(1, |_, _, _| panic!("invalid request RHS"));
    for c in [
        SwitchingConfig {
            entry_cost_ratio: f64::NAN,
            ..Default::default()
        },
        SwitchingConfig {
            implicit_pilot_limit: 0,
            ..Default::default()
        },
    ] {
        assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts(), &c).is_err());
    }
}
#[test]
fn derivative_forbidden_ignores_even_configured_implicit_pilots() {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("forbidden pilot")))
    .autonomous();
    let c = SwitchingConfig {
        fallback: DerivativeFallback::Forbidden,
        implicit_pilot_attempt_budget: 64,
        pressure_steps: 1,
        ..screened()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts(), &c).unwrap();
    assert!(r.success);
    assert_eq!(r.stats.segments.len(), 1);
    assert_eq!(r.stats.jacobian_builds, 0);
}
