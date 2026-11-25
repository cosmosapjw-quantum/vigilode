use vigilode::{
    Atol, BatchRequest, BatchStop, DerivativeFallback, EntryPolicy, EntrySeed, Jacobian, Options,
    PexprbWindowDriver, Problem, ProductionConfig, ProductionSolver, ProgressModel, RouteId,
    RouteRequirements, SwitchingConfig, SwitchingSolver, WindowDriver, WorkModelVersion,
    selection_evidence::SelectionIdentity,
};

#[test]
fn pexprb_window_driver_metadata_contract() {
    let driver = PexprbWindowDriver;
    assert_eq!(driver.route(), RouteId::Pexprb54s4Fused);
    assert_eq!(
        driver.requires(),
        RouteRequirements {
            needs_derivatives: true,
            max_dense_dimension: None,
            replay_safety: true,
        }
    );
    assert_eq!(
        driver.progress_model(2, true),
        Some(ProgressModel {
            max_step_growth: 5.0,
            min_work_per_attempt: 12,
        })
    );
    assert_eq!(
        driver.progress_model(2, false),
        Some(ProgressModel {
            max_step_growth: 5.0,
            min_work_per_attempt: 12,
        })
    );
}

#[test]
fn pexprb_window_driver_execution_contract_harmonic_and_decay() {
    let mut driver = PexprbWindowDriver;

    // Harmonic oscillator: y' = [-y1, y0]^T, with analytic JVP
    let p_harmonic = Problem::new(2, |_t, y, out| {
        out[0] = -y[1];
        out[1] = y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -v[1];
        out[1] = v[0];
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-8),
        h0: 0.05,
        max_steps: 1000,
        ..Default::default()
    };

    let req = BatchRequest {
        start_time: 0.0,
        target_time: 0.5,
        solve_end: 1.0,
        state: &[1.0, 0.0],
        proposed_h: Some(0.05),
        options: &opts,
        landings: &[],
        ceiling: None,
        carry_explicit_step: false,
        is_probe: false,
        is_entry_pilot: false,
        identity: SelectionIdentity::default(),
    };

    let outcome = driver
        .run(&p_harmonic, &req)
        .expect("pexprb driver succeeds on harmonic oscillator");

    assert_eq!(outcome.stop, BatchStop::Completed);
    assert!(outcome.record.success);
    let exact_0 = 0.5f64.cos();
    let exact_1 = 0.5f64.sin();
    assert!(
        (outcome.state[0] - exact_0).abs() < 1e-4,
        "y[0] err = {:e}",
        (outcome.state[0] - exact_0).abs()
    );
    assert!(
        (outcome.state[1] - exact_1).abs() < 1e-4,
        "y[1] err = {:e}",
        (outcome.state[1] - exact_1).abs()
    );
    assert!(outcome.counters.jvp_vectors > 0);

    // Exponential decay: y' = -5 y
    let p_decay = Problem::new(1, |_t, y, out| {
        out[0] = -5.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -5.0 * v[0];
        Ok(())
    }))
    .autonomous();

    let req_decay = BatchRequest {
        start_time: 0.0,
        target_time: 0.2,
        solve_end: 1.0,
        state: &[1.0],
        proposed_h: Some(0.02),
        options: &opts,
        landings: &[],
        ceiling: None,
        carry_explicit_step: false,
        is_probe: false,
        is_entry_pilot: false,
        identity: SelectionIdentity::default(),
    };

    let outcome_decay = driver
        .run(&p_decay, &req_decay)
        .expect("pexprb driver succeeds on decay");
    assert_eq!(outcome_decay.stop, BatchStop::Completed);
    let exact_decay = (-5.0 * 0.2f64).exp();
    assert!(
        (outcome_decay.state[0] - exact_decay).abs() < 1e-4,
        "decay err = {:e}",
        (outcome_decay.state[0] - exact_decay).abs()
    );

    // Negative control: attempt budget exhausted
    let opts_budget = Options {
        max_steps: 1,
        ..opts.clone()
    };
    let req_budget = BatchRequest {
        options: &opts_budget,
        ..req
    };
    let outcome_budget = driver
        .run(&p_harmonic, &req_budget)
        .expect("driver returns budget stop");
    assert_eq!(outcome_budget.stop, BatchStop::Budget);
    assert!(!outcome_budget.record.success);

    // Negative control: progress bound screening on small ceiling
    let req_pb = BatchRequest {
        ceiling: Some(12),
        is_entry_pilot: true,
        proposed_h: Some(1e-5),
        ..req
    };
    let outcome_pb = driver
        .run(&p_harmonic, &req_pb)
        .expect("driver returns progress bound stop");
    assert_eq!(outcome_pb.stop, BatchStop::ProgressBound);
    assert_eq!(
        outcome_pb.record.decision,
        "same-checkpoint implicit pilot discarded: unreachable under step growth bound"
    );
}

#[test]
fn route_registry_factory_defaults_contract() {
    let cfg = SwitchingConfig::route_registry(DerivativeFallback::ProvidedOnly);

    assert_eq!(cfg.fallback, DerivativeFallback::ProvidedOnly);
    assert_eq!(cfg.entry_policy, Some(EntryPolicy::AdaptiveEvidence));
    assert_eq!(cfg.entry_seed, EntrySeed::ExplicitCheckpoint);
    assert_eq!(cfg.work_model, WorkModelVersion::V1);
    assert!(cfg.progress_bound);
    assert!(cfg.strict_entry_work_improvement);
    assert!(cfg.strict_return_work_improvement);
    assert!(cfg.carry_explicit_step);
    assert!(cfg.reuse_entry_pilot_step);
    assert!(cfg.seed_entry_pilot_from_explicit_checkpoint);

    let adaptation = cfg
        .entry_probe_adaptation
        .as_ref()
        .expect("adaptation must be present");
    assert!(adaptation.loss_margin_backoff);
    assert_eq!(adaptation.max_backoff_batches, 64);
    assert_eq!(adaptation.work_rate_growth, 2.0);
    assert_eq!(adaptation.step_shrink_ratio, 0.5);
    assert_eq!(adaptation.rejection_fraction_increase, 0.2);

    // Matches adaptive_evidence fields
    let adaptive = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    assert_eq!(
        cfg.implicit_pilot_attempt_budget,
        adaptive.implicit_pilot_attempt_budget
    );
    assert_eq!(cfg.implicit_pilot_limit, adaptive.implicit_pilot_limit);
    assert_eq!(
        cfg.entry_pilot_cooldown_batches,
        adaptive.entry_pilot_cooldown_batches
    );
    assert_eq!(cfg.window_count, adaptive.window_count);
    assert_eq!(
        cfg.retry_unresolved_entry_once,
        adaptive.retry_unresolved_entry_once
    );
}

#[test]
fn production_refusal_of_pressure_only_entry_policy() {
    let problem = Problem::new(1, |_t, y, out| {
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_t, _y, v, out| {
        out[0] = -v[0];
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        rtol: 1e-4,
        atol: Atol::Scalar(1e-6),
        h0: 0.1,
        max_steps: 100,
        ..Default::default()
    };

    // Refusal: entry_policy == PressureOnly && fallback != Forbidden
    let mut bad_config =
        ProductionConfig::selective_jvp(opts.clone(), SwitchingConfig::default(), true);
    bad_config.switching.entry_policy = Some(EntryPolicy::PressureOnly);
    bad_config.switching.fallback = DerivativeFallback::ProvidedOnly;

    let err = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &bad_config)
        .expect_err("production solver must refuse pressure-only entry policy with fallback");
    assert!(
        err.0.contains(
            "production configuration refuses implicit pressure-only entry: campaign measured excessive overhead on rotation and nonstiff cells"
        ),
        "actual error: {}",
        err.0
    );

    // Negative control: forbidden fallback with pressure-only does NOT trigger this error
    let mut allowed_config = ProductionConfig::rhs_only(opts.clone(), 8);
    allowed_config.switching.entry_policy = Some(EntryPolicy::PressureOnly);
    let res = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &allowed_config);
    assert!(res.is_ok(), "rhs_only with forbidden fallback must succeed");

    // Negative control: adaptive evidence entry policy does NOT trigger this error
    let mut good_config = ProductionConfig::selective_jvp(
        opts,
        SwitchingConfig::route_registry(DerivativeFallback::ProvidedOnly),
        true,
    );
    good_config.switching.entry_policy = Some(EntryPolicy::AdaptiveEvidence);
    let res_good = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &good_config);
    assert!(res_good.is_ok(), "adaptive evidence must succeed");
}

#[test]
fn bitwise_preservation_of_switching_config_default() {
    let default_cfg = SwitchingConfig::default();
    assert_eq!(default_cfg.entry_policy, None);
    assert_eq!(default_cfg.entry_seed, EntrySeed::Legacy);
    assert_eq!(default_cfg.work_model, WorkModelVersion::V1);
    assert!(!default_cfg.progress_bound);
    assert!(!default_cfg.strict_entry_work_improvement);
    assert!(!default_cfg.strict_return_work_improvement);
    assert_eq!(default_cfg.fallback, DerivativeFallback::Forbidden);
    assert_eq!(default_cfg.window_count, 8);
    assert_eq!(default_cfg.explicit_attempt_budget, 32);
    assert_eq!(default_cfg.pressure_steps, 24);
    assert_eq!(default_cfg.rejection_fraction, 0.3);
    assert_eq!(default_cfg.implicit_residence, 2);
    assert_eq!(default_cfg.probe_interval, 4);
    assert_eq!(default_cfg.probe_attempt_budget, 16);
    assert_eq!(default_cfg.return_cost_ratio, 0.8);
    assert_eq!(default_cfg.max_dense_dimension, 64);
    assert_eq!(default_cfg.implicit_pilot_attempt_budget, 0);
    assert_eq!(default_cfg.implicit_pilot_limit, 1);
    assert_eq!(default_cfg.entry_cost_ratio, 0.8);
    assert!(!default_cfg.carry_explicit_step);
    assert_eq!(default_cfg.entry_pilot_cooldown_batches, 0);
    assert!(!default_cfg.reuse_entry_pilot_step);
    assert!(!default_cfg.seed_entry_pilot_from_explicit_checkpoint);
    assert!(!default_cfg.reuse_observed_entry_step);
    assert!(!default_cfg.retry_unresolved_entry_once);
    assert_eq!(default_cfg.pilot_backoff_factor, 2);
    assert_eq!(default_cfg.probe_backoff_factor, 2);
    assert_eq!(default_cfg.rearm_rate_ratio, 2.0);
    assert_eq!(default_cfg.return_sentinel_interval, 0);
    assert!(default_cfg.entry_probe_adaptation.is_none());

    // Serde roundtrip preserves exact bitwise equality
    let serialized = serde_json::to_string(&default_cfg).expect("serialize default config");
    let deserialized: SwitchingConfig =
        serde_json::from_str(&serialized).expect("deserialize default config");
    assert_eq!(deserialized.entry_policy, None);
    assert_eq!(deserialized.entry_seed, EntrySeed::Legacy);
    assert_eq!(deserialized.explicit_attempt_budget, 32);
}

#[test]
fn b7_route_sized_first_batch_dense_contract() {
    // Dense route with dimension 64:
    // c_R = 64 + 16 = 80 (autonomous)
    // c_D = 6.0
    // rho = 0.8
    // 3 * c_R / (rho * c_D) = 240 / 4.8 = 50 attempts.
    // Base explicit_attempt_budget is 32.
    // Under route_registry (progress_bound: true), the first batch attempt budget must be >= 50.
    let n = 64;
    let p_dense = Problem::new(n, |_t, y, out| {
        out.fill(0.0);
        out[0] = -50.0 * y[1];
        out[1] = 50.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, jac| {
        jac.fill(0.0);
        jac[1] = -50.0;
        jac[n] = 50.0;
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        rtol: 1e-4,
        atol: Atol::Scalar(1e-6),
        h0: 1e-3,
        max_steps: 1000,
        ..Default::default()
    };

    let mut cfg = SwitchingConfig::route_registry(DerivativeFallback::ProvidedOnly);
    cfg.explicit_attempt_budget = 32;
    cfg.window_count = 1;

    let y0 = vec![1.0; n];
    let sol = SwitchingSolver::solve(&p_dense, (0.0, 1.0), &y0, &opts, &cfg).unwrap();
    // Verify that the explicit segment exists
    let explicit_segment = sol
        .stats
        .segments
        .iter()
        .find(|s| s.method == vigilode::SwitchingMethod::Dopri5)
        .expect("must contain an explicit Dopri5 segment");
    assert_eq!(explicit_segment.method, vigilode::SwitchingMethod::Dopri5);

    // Now test with tiny h0 so that DOPRI5 hits the attempt limit of the batch
    let opts_tiny = Options {
        rtol: 1e-4,
        atol: Atol::Scalar(1e-6),
        h0: 1e-7,
        max_steps: 1000,
        ..Default::default()
    };
    let sol_budget = SwitchingSolver::solve(&p_dense, (0.0, 10.0), &y0, &opts_tiny, &cfg).unwrap();
    let seg = sol_budget
        .stats
        .segments
        .iter()
        .find(|s| s.method == vigilode::SwitchingMethod::Dopri5)
        .expect("must contain explicit Dopri5 segment");
    assert!(
        seg.attempts >= 50,
        "first explicit batch attempts {} must be route-sized to >= 50 (3 * 80 / (0.8 * 6))",
        seg.attempts
    );

    // Negative control: without progress_bound and with legacy entry_seed, default 32 is used
    let mut cfg_legacy = SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly);
    cfg_legacy.explicit_attempt_budget = 32;
    cfg_legacy.progress_bound = false;
    cfg_legacy.window_count = 1;

    let sol_legacy =
        SwitchingSolver::solve(&p_dense, (0.0, 10.0), &y0, &opts_tiny, &cfg_legacy).unwrap();
    let seg_legacy = sol_legacy
        .stats
        .segments
        .iter()
        .find(|s| s.method == vigilode::SwitchingMethod::Dopri5)
        .expect("must contain legacy explicit Dopri5 segment");
    assert_eq!(
        seg_legacy.attempts, 32,
        "legacy explicit batch attempts must remain capped at explicit_attempt_budget (32)"
    );
}
