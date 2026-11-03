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

// ---- T-F7 ------------------------------------------------------------------

// ---- T-F8 ------------------------------------------------------------------

// ---- T-F9 ------------------------------------------------------------------

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
