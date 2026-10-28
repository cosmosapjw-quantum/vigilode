use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, ProductionConfig,
    ProductionSolver, SwitchingConfig, SwitchingSolver,
};

fn options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 50_000,
        ..Options::default()
    }
}

fn selective() -> ProductionConfig {
    ProductionConfig::selective_jvp(
        options(),
        SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
        true,
    )
}

#[test]
fn rhs_only_preserves_native_states_work_and_derivative_prohibition() {
    let p = Problem::new(2, |t, y, out| {
        out[0] = -y[0];
        out[1] = 2.0 * t;
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, _, _| panic!("forbidden JVP")))
    .time_derivative(|_, _, _| panic!("forbidden time derivative"));
    let setup = ProductionConfig::rhs_only(options(), 8);
    let a = ProductionSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &setup).unwrap();
    let b = SwitchingSolver::solve(
        &p,
        (0.0, 1.0),
        &[1.0, 0.0],
        &setup.options,
        &setup.switching,
    )
    .unwrap();
    assert!(a.success && a.stats.accounting_complete);
    assert_eq!(a.t, b.t);
    assert_eq!(a.y, b.y);
    assert_eq!(a.stats.rhs_calls, b.stats.rhs_calls);
    assert_eq!(a.stats.attempts, b.stats.attempts);
    assert_eq!(
        a.stats.jvp_vectors + a.stats.jacobian_builds + a.stats.ft_calls,
        0
    );
}

#[test]
fn invalid_provider_policy_is_refused_before_any_callback() {
    let calls = Arc::new(AtomicU64::new(0));
    let entered = calls.clone();
    let fd = Problem::new(1, move |_, y, out| {
        entered.fetch_add(1, Ordering::SeqCst);
        out[0] = -y[0];
        Ok(())
    })
    .autonomous();
    let dense = fd.clone().jacobian(Jacobian::dense(|_, _, out| {
        out[0] = -1.0;
        Ok(())
    }));
    let jvp = fd.clone().jacobian(Jacobian::jvp(|_, _, v, out| {
        out[0] = -v[0];
        Ok(())
    }));
    for problem in [&fd, &dense] {
        assert!(ProductionSolver::solve(problem, (0.0, 1.0), &[1.0], &selective()).is_err());
    }
    let mut bad = Vec::new();
    let mut c = selective();
    c.schema_version = 2;
    bad.push(c);
    let mut c = selective();
    c.replay_safe_callbacks = false;
    bad.push(c);
    let mut c = selective();
    c.switching.implicit_pilot_attempt_budget = 0;
    bad.push(c);
    let mut c = selective();
    c.switching.strict_entry_work_improvement = false;
    bad.push(c);
    let mut c = selective();
    c.switching.strict_return_work_improvement = false;
    bad.push(c);
    let mut c = selective();
    c.switching.carry_explicit_step = false;
    bad.push(c);
    let mut c = selective();
    c.switching.fallback = DerivativeFallback::AllowFiniteDifference;
    bad.push(c);
    for config in bad {
        assert!(ProductionSolver::solve(&jvp, (0.0, 1.0), &[1.0], &config).is_err());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn complete_json_roundtrip_preserves_permissions_and_refuses_unknown_controls() {
    let setup = selective();
    let original = serde_json::to_value(&setup).unwrap();
    let decoded: ProductionConfig = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), original);
    let mut unknown = original.clone();
    unknown["allow_derivatives"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProductionConfig>(unknown).is_err());
    let mut missing = original.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("replay_safe_callbacks");
    assert!(serde_json::from_value::<ProductionConfig>(missing).is_err());
    let mut nested = original.clone();
    nested["switching"]["pretend_strict"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProductionConfig>(nested).is_err());
    let mut options_typo = original.clone();
    options_typo["options"]["max_stepz"] = serde_json::json!(1);
    assert!(serde_json::from_value::<ProductionConfig>(options_typo).is_err());
    let mut implicit_default = original.clone();
    implicit_default["options"]
        .as_object_mut()
        .unwrap()
        .remove("interior_error");
    assert!(serde_json::from_value::<ProductionConfig>(implicit_default).is_err());
    let mut atol_typo = original;
    atol_typo["options"]["atol"]["ignored_scale"] = serde_json::json!(0.1);
    assert!(serde_json::from_value::<ProductionConfig>(atol_typo).is_err());
}

#[test]
fn fatal_rhs_preserves_accepted_prefix_and_all_entered_work() {
    let calls = Arc::new(AtomicU64::new(0));
    let entered = calls.clone();
    let p = Problem::new(1, move |t, _, out| {
        entered.fetch_add(1, Ordering::SeqCst);
        if t > 0.2 {
            return Err(CallbackError::Fatal("fixture failure".to_owned()));
        }
        out[0] = 1.0;
        Ok(())
    });
    let setup = ProductionConfig::rhs_only(options(), 10);
    let result = ProductionSolver::solve(&p, (0.0, 1.0), &[0.0], &setup).unwrap();
    assert!(!result.success);
    assert!(result.stats.accounting_complete);
    assert!(result.final_time > 0.0 && result.final_time <= 0.2);
    assert_eq!(result.stats.rhs_calls, calls.load(Ordering::SeqCst));
    assert!((result.final_state[0] - result.final_time).abs() < 1e-12);
}

#[test]
fn supplied_jvp_profile_has_complete_cost_and_common_grid_accuracy() {
    let rate = 1000.0;
    let p = Problem::new(2, move |_, y, out| {
        for (d, x) in out.iter_mut().zip(y) {
            *d = -rate * x;
        }
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_, _, v, out| {
        for (d, x) in out.iter_mut().zip(v) {
            *d = -rate * x;
        }
        Ok(())
    }))
    .autonomous();
    let setup = selective();
    let result = ProductionSolver::solve(&p, (0.0, 1.0), &[1.0; 2], &setup).unwrap();
    assert!(result.success && result.stats.accounting_complete);
    assert_eq!(result.final_time, 1.0);
    assert!(result.stats.jvp_vectors > 0);
    assert_eq!(result.stats.jacobian_builds + result.stats.ft_calls, 0);
    assert!(result.stats.attempts <= setup.options.max_steps);
    for (t, state) in result.t.iter().zip(&result.y) {
        assert!(state.iter().all(|x| (x - (-rate * t).exp()).abs() <= 1e-6));
    }
}
