//! Contracts for the switcher-v2 remediation tasks (design:
//! docs/agent/switcher_v2_remediation/DESIGN.md). Each task appends its own
//! test functions below the marker for that task. Helpers are shared.
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingMethod,
    SwitchingSolution, SwitchingSolver,
};

/// Registered `rotation` model of `examples/evidence_switch_bench.rs`.
fn rotation() -> Problem {
    Problem::new(2, |_, y, o| {
        o[0] = -100.0 * y[1];
        o[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        o[0] = -100.0 * v[1];
        o[1] = 100.0 * v[0];
        Ok(())
    }))
    .autonomous()
}

/// Registered `stiff-forced` model of `examples/evidence_switch_bench.rs`.
fn stiff_forced() -> Problem {
    Problem::new(1, |t, y, o| {
        o[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10000.0;
        Ok(())
    }))
    .time_derivative(|t, _, o| {
        o[0] = -10000.0 * t.sin() - t.cos();
        Ok(())
    })
}

/// Options of the registered campaign (`evidence_switch_bench`, h0 = 1e-7).
fn bench_options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-7,
        max_steps: 20000,
        ..Default::default()
    }
}

fn solve(p: &Problem, y0: &[f64], cfg: &SwitchingConfig) -> SwitchingSolution {
    SwitchingSolver::solve(p, (0.0, 1.0), y0, &bench_options(), cfg).unwrap()
}

/// Sum of JVP vectors spent in discarded (uncommitted) RODAS pilots/comparators.
fn discarded_rodas_jvp(sol: &SwitchingSolution) -> u64 {
    sol.stats
        .segments
        .iter()
        .filter(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe && !s.committed)
        .map(|s| s.jvp_vectors)
        .sum()
}

fn provided() -> DerivativeFallback {
    DerivativeFallback::ProvidedOnly
}

// ---- T-F1 ------------------------------------------------------------------

#[test]
fn f1_adaptive_evidence_factory_has_no_loss_margin_backoff() {
    let a = SwitchingConfig::adaptive_evidence(provided());
    let adaptation = a
        .entry_probe_adaptation
        .expect("adaptive_evidence sets an adaptation");
    assert!(!adaptation.loss_margin_backoff);
    let r = SwitchingConfig::route_registry(provided());
    assert!(
        r.entry_probe_adaptation
            .expect("route_registry sets an adaptation")
            .loss_margin_backoff
    );
}

// ---- T-F2 ------------------------------------------------------------------

#[test]
fn f2_v2_entry_ceiling_stops_losing_rotation_pilots_like_v1() {
    let mut v2 = SwitchingConfig::route_registry(provided());
    v2.work_model = vigilode::WorkModelVersion::V2;
    let mut v1 = v2.clone();
    v1.work_model = vigilode::WorkModelVersion::V1;
    let a = solve(&rotation(), &[1.0, 0.0], &v2);
    let b = solve(&rotation(), &[1.0, 0.0], &v1);
    assert!(a.success && b.success);
    assert_eq!(a.stats.switches, b.stats.switches);
    assert!(
        discarded_rodas_jvp(&a) <= discarded_rodas_jvp(&b) + 50,
        "V2 pilot JVPs {} vs V1 {}",
        discarded_rodas_jvp(&a),
        discarded_rodas_jvp(&b)
    );
}

// ---- T-F3 ------------------------------------------------------------------

#[test]
fn f3_route_registry_defaults_to_work_model_v1() {
    let r = SwitchingConfig::route_registry(provided());
    assert_eq!(r.work_model, vigilode::WorkModelVersion::V1);
    assert_eq!(r.entry_seed, vigilode::EntrySeed::ExplicitCheckpoint);
    let sf = solve(&stiff_forced(), &[1.0], &r);
    assert!(sf.success);
    assert_eq!(sf.stats.switches, 1);
    assert!(
        sf.stats.rhs_calls <= 420,
        "stiff-forced RHS {}",
        sf.stats.rhs_calls
    );
}

// ---- T-F4 ------------------------------------------------------------------

#[test]
fn f4_progress_screen_preserves_entry_decisions() {
    for (p, y0) in [(stiff_forced(), vec![1.0]), (rotation(), vec![1.0, 0.0])] {
        let on = SwitchingConfig::route_registry(provided());
        let mut off = on.clone();
        off.progress_bound = false;
        let a = solve(&p, &y0, &on);
        let b = solve(&p, &y0, &off);
        assert!(a.success && b.success);
        assert_eq!(a.stats.switches, b.stats.switches);
        let entry = |s: &SwitchingSolution| {
            s.stats
                .segments
                .iter()
                .find(|x| x.method == SwitchingMethod::Rodas5p && x.committed)
                .map(|x| x.start_time.to_bits())
        };
        assert_eq!(entry(&a), entry(&b));
    }
}

#[test]
fn f4_progress_reach_upper_bound_boundary_table() {
    use vigilode::progress_reach_upper_bound as reach;
    assert_eq!(reach(1e-3, 5.0, 6, 7), 0.0);
    assert_eq!(reach(1e-3, 5.0, 7, 7), 1e-3f64.next_up());
    assert_eq!(reach(1e-3, 5.0, 14, 7), (1e-3f64 * 6.0).next_up());
    assert_eq!(reach(1e-3, 1.0, 21, 7), (1e-3f64 * 3.0).next_up());
    assert_eq!(reach(1e-3, 5.0, 7 * 401, 7), f64::INFINITY);
    assert!(reach(1e-3, 5.0, 7 * 400, 7).is_finite());
    assert_eq!(reach(1e-3, 5.0, 100, 0), f64::INFINITY);
    assert_eq!(reach(0.0, 5.0, 100, 7), f64::INFINITY);
    assert_eq!(reach(1e-3, 0.5, 100, 7), f64::INFINITY);
    assert_eq!(reach(f64::NAN, 5.0, 100, 7), f64::INFINITY);

    use vigilode::registry::WindowDriver;
    let driver = vigilode::registry::Rodas5pWindowDriver::with_route(
        vigilode::registry::RouteId::Rodas5pMatrixFree,
    );
    let pm = driver
        .progress_model(2, false)
        .expect("progress model present");
    assert_eq!(pm.min_work_per_attempt, 7);
}

// ---- T-F5 ------------------------------------------------------------------

#[test]
fn f5_first_explicit_batch_keeps_configured_budget_on_small_dense_routes() {
    let cfg = SwitchingConfig::route_registry(provided());
    assert_eq!(cfg.explicit_attempt_budget, 32);
    let sf = solve(&stiff_forced(), &[1.0], &cfg);
    let first = sf
        .stats
        .segments
        .iter()
        .find(|s| s.method == SwitchingMethod::Dopri5 && !s.is_probe)
        .expect("an explicit batch");
    assert_eq!(first.attempts, 32);
}

// ---- T-F6 ------------------------------------------------------------------

#[test]
fn f6_replay_mismatch_keeps_explicit_progress_and_disables_pilots() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    CALLS.store(0, Ordering::SeqCst);
    let p = Problem::new(1, |t, y, o| {
        let k = CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
        if k % 2 == 1 {
            o[0] = f64::from_bits(o[0].to_bits() ^ 1);
        }
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10000.0;
        Ok(())
    }))
    .time_derivative(|t, _, o| {
        o[0] = -10000.0 * t.sin() - t.cos();
        Ok(())
    });
    let mut cfg = SwitchingConfig::route_registry(provided());
    cfg.audit_replay_safety = true;
    let sol = solve(&p, &[1.0], &cfg);
    assert!(sol.success);
    assert_eq!(sol.final_time, 1.0);
    assert!(
        !sol.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe),
        "no RODAS pilot may run after a replay mismatch"
    );
    assert!(
        sol.stats
            .segments
            .iter()
            .any(|s| s.decision.contains("competitive trials disabled"))
    );
    let segment_rhs: u64 = sol.stats.segments.iter().map(|s| s.rhs_calls).sum();
    assert_eq!(
        sol.stats.rhs_calls,
        segment_rhs + 2,
        "the two audit RHS calls must be charged"
    );
    assert_eq!(CALLS.load(Ordering::SeqCst) as u64, sol.stats.rhs_calls);
}

#[test]
fn f6_replay_audit_charges_deterministic_jvp_calls() {
    let mut cfg = SwitchingConfig::route_registry(provided());
    cfg.audit_replay_safety = true;
    let sol = solve(&rotation(), &[1.0, 0.0], &cfg);
    assert!(sol.success);
    let segment_rhs: u64 = sol.stats.segments.iter().map(|s| s.rhs_calls).sum();
    let segment_jvp: u64 = sol.stats.segments.iter().map(|s| s.jvp_vectors).sum();
    assert_eq!(sol.stats.rhs_calls, segment_rhs + 2, "two audit RHS calls");
    assert_eq!(
        sol.stats.jvp_vectors,
        segment_jvp + 2,
        "two audit JVP calls"
    );
    assert!(
        sol.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe),
        "deterministic callbacks keep competitive pilots enabled"
    );

    let mut limit_cfg = cfg.clone();
    limit_cfg.entry_probe_adaptation = None;
    limit_cfg.implicit_pilot_limit = 1;
    limit_cfg.entry_policy = Some(vigilode::EntryPolicy::Evidence);
    let limit_sol = solve(&rotation(), &[1.0, 0.0], &limit_cfg);
    let pilot_count = limit_sol
        .stats
        .segments
        .iter()
        .filter(|s| s.is_probe)
        .count();
    assert_eq!(
        pilot_count, 1,
        "implicit_pilot_limit strictly caps pilot count"
    );
}

#[test]
fn f6_nondeterministic_jvp_disables_competitive_trials() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static JVP_CALLS: AtomicUsize = AtomicUsize::new(0);
    let p = Problem::new(2, |_, y, o| {
        o[0] = -100.0 * y[1];
        o[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        let k = JVP_CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -100.0 * v[1];
        o[1] = 100.0 * v[0];
        if k % 2 == 1 {
            o[0] = f64::from_bits(o[0].to_bits() ^ 1);
        }
        Ok(())
    }))
    .autonomous();
    let mut cfg = SwitchingConfig::route_registry(provided());
    cfg.audit_replay_safety = true;
    let sol = solve(&p, &[1.0, 0.0], &cfg);
    assert!(sol.success);
    assert!(
        !sol.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe),
        "no RODAS pilot may run after a JVP replay mismatch"
    );
    assert!(
        sol.stats
            .segments
            .iter()
            .any(|s| s.decision.contains("competitive trials disabled"))
    );
}

// ---- T-WP02 (Supersedes T-F7) ----------------------------------------------

#[test]
fn wp02_window_epoch_is_checkpoint_sensitive_sha256_stable() {
    use vigilode::compute_window_epoch;
    // Governed by docs/agent/switcher_v2_remediation/EPOCH_001_AUTHORITY_DECISION.md.
    // The test asserts the authoritative WP-02 SHA-256 golden values and index-sensitivity.
    assert_eq!(compute_window_epoch(0, 0.0, 1.0), 0x5718_834c_1304_ae04);
    assert_eq!(compute_window_epoch(3, 0.25, 0.5), 0x27a1_3285_42cc_99d0);
    assert_eq!(
        compute_window_epoch(17, -1.5, 2e-300),
        0x8b1b_1407_ca5e_33aa
    );
    assert_eq!(compute_window_epoch(7, 0.0, 1.0), 0xe9b6_ee9b_f31a_9fa7);
    assert_ne!(
        compute_window_epoch(0, 0.0, 1.0),
        compute_window_epoch(7, 0.0, 1.0),
        "WP-02 specification is checkpoint-index sensitive"
    );

    // Negative zero and subnormal boundary stability (-0.0 vs +0.0 distinct bit patterns)
    assert_ne!(
        compute_window_epoch(0, 0.0, 1.0),
        compute_window_epoch(0, -0.0, 1.0),
        "-0.0 and +0.0 must yield distinct epoch digests"
    );

    // Large index check (> 2^32) ensures 64-bit index fidelity
    let large_idx = (1usize << 33) + 42;
    assert_ne!(
        compute_window_epoch(42, 0.0, 1.0),
        compute_window_epoch(large_idx, 0.0, 1.0),
        "Index bits above 32-bit boundary must affect the digest"
    );
}

#[test]
fn f7_tolerance_epoch_is_fnv_stable() {
    use vigilode::{InteriorError, compute_tolerance_epoch};
    let o = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        interior_error: InteriorError::WFilteredEnforce,
        ..Default::default()
    };
    assert_eq!(compute_tolerance_epoch(&o), 0xd700_4aac_7795_c1b0);
    let off = Options {
        interior_error: InteriorError::Off,
        ..o.clone()
    };
    assert_eq!(compute_tolerance_epoch(&off), 0xd700_4cac_7795_c516);
    let vector = Options {
        atol: Atol::Vector(vec![1e-9, 1e-8]),
        ..o.clone()
    };
    assert_eq!(compute_tolerance_epoch(&vector), 0xa1d2_d72c_3b87_d01d);

    use vigilode::registry::{RouteId, compute_provider_epoch};
    let ep = compute_provider_epoch(RouteId::Dopri5Rhs, &rotation());
    assert_ne!(ep, 0);
    assert_ne!(ep, 1);
}

// ---- T-F8 ------------------------------------------------------------------

#[test]
fn f8_declared_entry_policy_must_match_effective_schedule() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vigilode::EntryPolicy;
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let counting = Problem::new(1, |_, y, o| {
        CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        o[0] = -v[0];
        Ok(())
    }))
    .autonomous();
    assert_eq!(
        SwitchingConfig::route_registry(provided()).effective_entry_policy(),
        EntryPolicy::AdaptiveEvidence
    );
    let mut c = SwitchingConfig::evidence_driven(provided());
    assert_eq!(c.effective_entry_policy(), EntryPolicy::Evidence);
    c.implicit_pilot_attempt_budget = 0;
    assert_eq!(c.effective_entry_policy(), EntryPolicy::PressureOnly);
    c.entry_policy = Some(EntryPolicy::Evidence);
    CALLS.store(0, Ordering::SeqCst);
    let err = SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &c)
        .expect_err("a declared policy that the schedule does not implement is refused");
    assert!(
        err.0.contains("declared entry_policy does not match"),
        "{}",
        err.0
    );
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        0,
        "refusal happens before callbacks"
    );
    c.entry_policy = Some(EntryPolicy::PressureOnly);
    assert!(SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &c).is_ok());
    c.entry_policy = None;
    assert!(SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &c).is_ok());
}

// ---- T-F9 ------------------------------------------------------------------

#[test]
fn f9_switcher_route_selection_is_limited_to_legacy_routes() {
    use vigilode::{Rodas5pWindowDriver, RouteId, WindowDriver};
    assert_eq!(
        Rodas5pWindowDriver::for_problem(&stiff_forced()).route(),
        RouteId::Rodas5pDense
    );
    assert_eq!(
        Rodas5pWindowDriver::for_problem(&rotation()).route(),
        RouteId::Rodas5pMatrixFree
    );
}

#[test]
fn f9_reference_counters_after_remediation() {
    let cases = [
        (
            "evidence_driven",
            SwitchingConfig::evidence_driven(provided()),
            (892, 5371, 355),
            (66, 456),
        ),
        (
            "adaptive_evidence",
            SwitchingConfig::adaptive_evidence(provided()),
            (892, 5371, 355),
            (58, 394),
        ),
        (
            "route_registry",
            SwitchingConfig::route_registry(provided()),
            (883, 5307, 149),
            (59, 400),
        ),
    ];
    for (name, cfg, rot_expected, sf_expected) in cases {
        let rot = solve(&rotation(), &[1.0, 0.0], &cfg);
        assert!(rot.success, "{name}");
        assert_eq!(
            (
                rot.stats.attempts,
                rot.stats.rhs_calls,
                discarded_rodas_jvp(&rot)
            ),
            rot_expected,
            "{name} rotation"
        );
        let sf = solve(&stiff_forced(), &[1.0], &cfg);
        assert!(sf.success, "{name}");
        assert_eq!(
            (sf.stats.attempts, sf.stats.rhs_calls),
            sf_expected,
            "{name} stiff-forced"
        );
        assert_eq!(sf.stats.switches, 1, "{name} stiff-forced switches");
    }
}

#[test]
fn skeleton_helpers_reproduce_registered_counters() {
    // Guards the helpers themselves: the evidence-driven factory must give the
    // registered L-0171 counters on both models (COMPARISON.md, "after" column).
    let rot = solve(
        &rotation(),
        &[1.0, 0.0],
        &SwitchingConfig::evidence_driven(provided()),
    );
    assert!(rot.success);
    assert_eq!((rot.stats.attempts, rot.stats.rhs_calls), (892, 5371));
    assert_eq!(discarded_rodas_jvp(&rot), 355);
    let sf = solve(
        &stiff_forced(),
        &[1.0],
        &SwitchingConfig::evidence_driven(provided()),
    );
    assert!(sf.success);
    assert_eq!((sf.stats.attempts, sf.stats.rhs_calls), (66, 456));
}

// ---- T-X9 ------------------------------------------------------------------

#[test]
fn x9_hysteresis_band_validation_and_default_neutrality() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vigilode::margin::MarginRatio;

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let counting = Problem::new(1, |_, y, o| {
        CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        o[0] = -v[0];
        Ok(())
    }))
    .autonomous();

    let mut cfg = SwitchingConfig::route_registry(provided());
    assert_eq!(cfg.entry_margin, None);
    assert_eq!(cfg.return_margin, None);

    // Closed band (1/1, 1/1) is refused before callbacks.
    cfg.entry_margin = MarginRatio::new(1, 1);
    cfg.return_margin = MarginRatio::new(1, 1);
    CALLS.store(0, Ordering::SeqCst);
    let err = SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &cfg)
        .expect_err("closed hysteresis band must be refused");
    assert!(
        err.0.contains("hysteresis band must be open"),
        "got: {}",
        err.0
    );
    assert_eq!(CALLS.load(Ordering::SeqCst), 0);

    // Malformed ratio is refused before callbacks.
    cfg.entry_margin = Some(MarginRatio { num: 0, den: 5 });
    cfg.return_margin = None;
    let err = SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &cfg)
        .expect_err("zero numerator must be refused");
    assert!(err.0.contains("entry_margin"), "got: {}", err.0);
    assert_eq!(CALLS.load(Ordering::SeqCst), 0);

    // Open band (4/5, 4/5) preserves stiff-forced entry and rotation explicit residence.
    cfg.entry_margin = MarginRatio::new(4, 5);
    cfg.return_margin = MarginRatio::new(4, 5);
    let sf = solve(&stiff_forced(), &[1.0], &cfg);
    assert!(sf.success);
    assert_eq!(sf.stats.switches, 1);
    let rot = solve(&rotation(), &[1.0, 0.0], &cfg);
    assert!(rot.success);
    assert_eq!(rot.stats.switches, 0);
}

// ---- T-X5 ------------------------------------------------------------------

#[test]
fn x5_stiffness_gate_flags_stiff_forced_and_backs_off_on_rotation() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let counting = Problem::new(1, |_, y, o| {
        CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        o[0] = -v[0];
        Ok(())
    }))
    .autonomous();

    let mut cfg = SwitchingConfig::adaptive_evidence(provided());
    assert_eq!(cfg.stiffness_gate_fraction, None);

    // Out-of-range fraction is refused before any RHS callback.
    for bad_frac in [0.0, -0.5, 1.01, f64::NAN, f64::INFINITY] {
        cfg.stiffness_gate_fraction = Some(bad_frac);
        CALLS.store(0, Ordering::SeqCst);
        let err = SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &cfg)
            .expect_err("invalid stiffness_gate_fraction must be refused");
        assert!(err.0.contains("stiffness_gate_fraction"), "got: {}", err.0);
        assert_eq!(CALLS.load(Ordering::SeqCst), 0);
    }

    // With fraction = 0.8:
    // - stiff-forced is flagged on the first batch and switches immediately (58 attempts, 394 RHS calls).
    // - rotation is unflagged and uses maximal backoff, lowering discarded pilot JVPs by >= 90%.
    let base_rot = solve(
        &rotation(),
        &[1.0, 0.0],
        &SwitchingConfig::adaptive_evidence(provided()),
    );
    let base_discarded_jvp = discarded_rodas_jvp(&base_rot);
    assert_eq!(base_discarded_jvp, 355);

    cfg.stiffness_gate_fraction = Some(0.8);
    let sf = solve(&stiff_forced(), &[1.0], &cfg);
    assert!(sf.success);
    assert_eq!(sf.stats.switches, 1);
    assert_eq!((sf.stats.attempts, sf.stats.rhs_calls), (58, 394));

    let gated_rot = solve(&rotation(), &[1.0, 0.0], &cfg);
    assert!(gated_rot.success);
    assert_eq!(gated_rot.stats.switches, 0);
    let gated_discarded_jvp = discarded_rodas_jvp(&gated_rot);
    assert!(
        gated_discarded_jvp * 10 <= base_discarded_jvp,
        "expected >= 90% reduction in discarded pilot JVPs on rotation: base={base_discarded_jvp}, gated={gated_discarded_jvp}"
    );
}
