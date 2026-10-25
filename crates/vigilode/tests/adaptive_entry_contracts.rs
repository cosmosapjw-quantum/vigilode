use vigilode::{
    Atol, AutomaticOde, AutomaticOdePolicy, CallbackError, DerivativeFallback,
    EntryProbeAdaptation, EntryProbeReason, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingEvidence, SwitchingMethod, SwitchingSolver,
};

fn options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 50000,
        ..Default::default()
    }
}
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
fn transitioning(tau: f64) -> Problem {
    // Smooth changing damping with exactly known rotating solution. The time
    // derivative includes lambda' and the moving equilibrium derivative.
    Problem::new(2, move |t, y, o| {
        let l = 5e4 * (1.0 + (60.0 * (t - tau)).tanh());
        let (s, c) = (100.0 * t).sin_cos();
        o[0] = -100.0 * y[1] - l * (y[0] - c);
        o[1] = 100.0 * y[0] - l * (y[1] - s);
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |t, _, v, o| {
        let l = 5e4 * (1.0 + (60.0 * (t - tau)).tanh());
        o[0] = -100.0 * v[1] - l * v[0];
        o[1] = 100.0 * v[0] - l * v[1];
        Ok(())
    }))
    .time_derivative(move |t, y, o| {
        let q = (60.0 * (t - tau)).tanh();
        let l = 5e4 * (1.0 + q);
        let lp = 3e6 * (1.0 - q * q);
        let (s, c) = (100.0 * t).sin_cos();
        o[0] = -lp * (y[0] - c) - 100.0 * l * s;
        o[1] = -lp * (y[1] - s) + 100.0 * l * c;
        Ok(())
    })
}
fn error(y: &[f64]) -> f64 {
    (y[0] - 100.0_f64.cos())
        .abs()
        .max((y[1] - 100.0_f64.sin()).abs())
}
fn pilots(r: &vigilode::SwitchingSolution) -> usize {
    r.stats
        .segments
        .iter()
        .filter(|s| s.is_probe && s.method == SwitchingMethod::Rodas5p)
        .count()
}
#[test]
fn stable_rotation_reuses_negative_evidence_and_reduces_discarded_work() {
    let p = rotation();
    let o = options();
    let old = SwitchingSolver::solve(
        &p,
        (0.0, 1.0),
        &[1.0, 0.0],
        &o,
        &SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly),
    )
    .unwrap();
    let new = SwitchingSolver::solve(
        &p,
        (0.0, 1.0),
        &[1.0, 0.0],
        &o,
        &SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
    )
    .unwrap();
    assert!(old.success && new.success);
    assert_eq!(new.stats.switches, 0);
    assert_eq!(old.final_state, new.final_state);
    assert!(error(&new.final_state) < 1e-5);
    assert!(
        pilots(&new) < pilots(&old),
        "old={} new={}",
        pilots(&old),
        pilots(&new)
    );
    assert!(new.stats.rhs_calls < old.stats.rhs_calls);
    assert!(
        new.stats
            .segments
            .iter()
            .any(|s| s.entry_probe_reason == Some(EntryProbeReason::StableEvidenceBackoff))
    );
    assert_eq!(
        new.stats.rhs_calls,
        new.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
    );
}
#[test]
fn shifted_smooth_stiffness_onsets_renew_old_negative_evidence() {
    for tau in [0.35, 0.65, 0.82] {
        let p = transitioning(tau);
        let o = options();
        // The legacy lifetime count is deliberately one. Adaptive policy may
        // renew a stale comparison and remains bounded by total attempts.
        let c = SwitchingConfig {
            implicit_pilot_limit: 1,
            ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
        };
        let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &o, &c).unwrap();
        assert!(r.success, "tau={tau}: {}", r.message);
        assert!(
            error(&r.final_state) < 1e-5,
            "tau={tau} error={}",
            error(&r.final_state)
        );
        assert!(
            r.stats
                .segments
                .iter()
                .any(|s| s.committed && s.method == SwitchingMethod::Rodas5p),
            "tau={tau}"
        );
        assert!(
            r.stats.segments.iter().any(|s| s.is_probe
                && matches!(
                    s.entry_probe_reason,
                    Some(
                        EntryProbeReason::WorkDeteriorated
                            | EntryProbeReason::RejectionDeteriorated
                            | EntryProbeReason::SentinelExpired
                            | EntryProbeReason::BackoffExpired
                    )
                )),
            "tau={tau}"
        );
        assert!(r.stats.attempts <= o.max_steps);
        assert_eq!(
            r.stats.attempts,
            r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
    }
}
#[test]
fn adaptive_sentinel_never_exhausts_its_observation_right_in_stable_regime() {
    let c = SwitchingConfig {
        implicit_pilot_limit: 1,
        entry_pilot_cooldown_batches: 2,
        entry_probe_adaptation: Some(EntryProbeAdaptation {
            max_backoff_batches: 4,
            ..Default::default()
        }),
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    };
    let r = SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &c).unwrap();
    assert!(r.success && pilots(&r) > 1);
    assert!(
        r.stats
            .segments
            .iter()
            .any(|s| s.is_probe && s.entry_probe_reason == Some(EntryProbeReason::SentinelExpired))
    );
    assert_eq!(r.stats.switches, 0);
}
#[test]
fn adaptive_probes_remain_inside_global_attempt_budget() {
    let o = Options {
        max_steps: 40,
        ..options()
    };
    let r = SwitchingSolver::solve(
        &rotation(),
        (0.0, 1.0),
        &[1.0, 0.0],
        &o,
        &SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
    )
    .unwrap();
    assert!(!r.success);
    assert_eq!(r.stats.attempts, 40);
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
    assert!(
        r.stats
            .segments
            .iter()
            .filter(|s| s.is_probe)
            .all(|s| !s.committed)
    );
}
#[test]
fn adaptive_factory_and_frontdoor_preserve_derivative_permission() {
    let p = Problem::new(1, |_, y, o| {
        o[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("unauthorized derivative")))
    .autonomous();
    let r = AutomaticOde::solve_adaptive(
        &p,
        (0.0, 1.0),
        &[1.0],
        &options(),
        &AutomaticOdePolicy::default(),
    )
    .unwrap();
    assert!(r.success);
    assert_eq!(r.stats.jacobian_builds, 0);
    assert_eq!(r.stats.segments.len(), 1);
}
#[test]
fn invalid_adaptation_is_refused_before_callbacks() {
    let p = Problem::new(1, |_, _, _| panic!("must validate first"));
    for a in [
        EntryProbeAdaptation {
            work_rate_growth: f64::NAN,
            ..Default::default()
        },
        EntryProbeAdaptation {
            max_backoff_batches: 0,
            ..Default::default()
        },
        EntryProbeAdaptation {
            step_shrink_ratio: 1.0,
            ..Default::default()
        },
    ] {
        let c = SwitchingConfig {
            entry_probe_adaptation: Some(a),
            ..SwitchingConfig::adaptive_evidence(DerivativeFallback::Forbidden)
        };
        assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(), &c).is_err());
    }
}

fn relaxing_problem(fail_comparator_at: Option<f64>) -> Problem {
    Problem::new(1, |t, y, o| {
        let l = 1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh());
        o[0] = -l * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _, j| {
        if fail_comparator_at.is_some_and(|at| t.to_bits() == at.to_bits()) {
            return Err(CallbackError::Fatal(
                "injected return comparator error".into(),
            ));
        }
        j[0] = -(1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh()));
        Ok(())
    }))
    .time_derivative(|t, y, o| {
        let q = (40.0 * (t - 0.4)).tanh();
        let l = 1.0 + 4999.5 * (1.0 - q);
        let lp = -199980.0 * (1.0 - q * q);
        o[0] = -lp * (y[0] - t.cos()) - l * t.sin() - t.cos();
        Ok(())
    })
}
fn relaxing_config() -> SwitchingConfig {
    SwitchingConfig {
        window_count: 12,
        implicit_residence: 1,
        probe_interval: 1,
        probe_attempt_budget: 64,
        return_cost_ratio: 0.99,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    }
}
fn relaxing_options() -> Options {
    Options {
        h0: 1e-7,
        ..options()
    }
}
#[test]
fn return_to_explicit_requires_same_checkpoint_comparison() {
    let r = SwitchingSolver::solve(
        &relaxing_problem(None),
        (0.0, 1.0),
        &[1.0],
        &relaxing_options(),
        &relaxing_config(),
    )
    .unwrap();
    assert!(r.success, "{}", r.message);
    assert!((r.final_state[0] - 1.0_f64.cos()).abs() < 1e-5);
    let indices: Vec<_> = r
        .stats
        .segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_probe && s.committed && s.method == SwitchingMethod::Dopri5)
        .map(|(i, _)| i)
        .collect();
    assert!(
        !indices.is_empty(),
        "{:?}",
        r.stats
            .segments
            .iter()
            .map(|s| (s.method, s.start_time, s.evidence))
            .collect::<Vec<_>>()
    );
    for i in indices {
        let explicit = &r.stats.segments[i];
        let paired = &r.stats.segments[i + 1];
        assert_eq!(
            explicit.evidence,
            SwitchingEvidence::ExplicitCheaperOnSameWindow
        );
        assert_eq!(paired.method, SwitchingMethod::Rodas5p);
        assert!(paired.is_probe && !paired.committed && paired.success);
        assert_eq!(explicit.start_time, paired.start_time);
        assert_eq!(explicit.target_time, paired.target_time);
        assert!(explicit.work_units <= 0.99 * paired.work_units);
    }
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
}
#[test]
fn exhausted_return_comparison_budget_keeps_previous_committed_state() {
    let c = relaxing_config();
    let mut o = relaxing_options();
    let full = SwitchingSolver::solve(&relaxing_problem(None), (0.0, 1.0), &[1.0], &o, &c).unwrap();
    let i = full
        .stats
        .segments
        .iter()
        .position(|s| s.evidence == SwitchingEvidence::ExplicitCheaperOnSameWindow)
        .expect("reference must exercise return comparison");
    let before = full.stats.segments[..=i]
        .iter()
        .map(|s| s.attempts)
        .sum::<usize>();
    for remaining in [0, 1] {
        o.max_steps = before + remaining;
        let r =
            SwitchingSolver::solve(&relaxing_problem(None), (0.0, 1.0), &[1.0], &o, &c).unwrap();
        assert!(!r.success);
        assert_eq!(r.stats.attempts, o.max_steps);
        assert_eq!(r.final_time, full.stats.segments[i].start_time);
        assert!(
            !r.stats
                .segments
                .iter()
                .any(|s| s.evidence == SwitchingEvidence::ExplicitCheaperOnSameWindow)
        );
        assert_eq!(
            r.stats.attempts,
            r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
        assert_eq!(
            r.stats.rhs_calls,
            r.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
        );
    }
}
#[test]
fn fatal_return_comparator_does_not_publish_the_discarded_probe_state() {
    let c = relaxing_config();
    let o = relaxing_options();
    let full = SwitchingSolver::solve(&relaxing_problem(None), (0.0, 1.0), &[1.0], &o, &c).unwrap();
    let target = full
        .stats
        .segments
        .iter()
        .find(|s| s.evidence == SwitchingEvidence::ExplicitCheaperOnSameWindow)
        .unwrap();
    let r = SwitchingSolver::solve(
        &relaxing_problem(Some(target.start_time)),
        (0.0, 1.0),
        &[1.0],
        &o,
        &c,
    )
    .unwrap();
    assert!(!r.success);
    assert!(r.message.contains("fatal same-window"), "{}", r.message);
    assert_eq!(r.final_time, target.start_time);
    assert!(!r.stats.segments.last().unwrap().committed);
    assert!(r.stats.accounting_complete);
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
}

fn fast_transient_problem(eps: f64) -> Problem {
    Problem::new(2, move |t, y, o| {
        o[1] = -y[1] / (1.0 + t) + 0.2 * y[0];
        o[0] = -(y[0] - y[1].cos()) / eps - y[1].sin() * o[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, y, j| {
        let f = -y[1] / (1.0 + t) + 0.2 * y[0];
        j[0] = -1.0 / eps - 0.2 * y[1].sin();
        j[1] = -y[1].sin() / eps - y[1].cos() * f + y[1].sin() / (1.0 + t);
        j[2] = 0.2;
        j[3] = -1.0 / (1.0 + t);
        Ok(())
    }))
    .time_derivative(|t, y, o| {
        o[1] = y[1] / (1.0 + t).powi(2);
        o[0] = -y[1].sin() * o[1];
        Ok(())
    })
}
#[test]
fn completed_fast_transients_renew_uneconomical_startup_comparisons() {
    use vigilode::{Dopri5, ExponentialCofactor};
    for eps in [1e-3, 1e-6] {
        let p = fast_transient_problem(eps);
        let o = options();
        let old = SwitchingSolver::solve(
            &p,
            (0.0, 2.0),
            &[0.0, 1.0],
            &o,
            &SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly),
        )
        .unwrap();
        let new = SwitchingSolver::solve(
            &p,
            (0.0, 2.0),
            &[0.0, 1.0],
            &o,
            &SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
        )
        .unwrap();
        assert!(
            old.success && new.success,
            "eps={eps} old={} new={}",
            old.message,
            new.message
        );
        let reference = if eps == 1e-6 {
            // Reuse the published independent WP40/WP50 endpoint.
            let value:serde_json::Value=serde_json::from_str(include_str!(
                "../../../research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json")).unwrap();
            [
                value["reference_state"][0].as_f64().unwrap(),
                value["reference_state"][1].as_f64().unwrap(),
            ]
        } else {
            // Different epsilon: check the independently reduced equation at
            // tighter tolerance, using the same native RK method explicitly.
            let carrier = ExponentialCofactor::new(0.0, -1.0_f64.cos(), 1.0 / eps, 0.2).unwrap();
            let reduced = Problem::new(1, move |t, v, out| {
                let y = carrier.physical_slow(t, v[0]).unwrap();
                out[0] = -y / (1.0 + t) + 0.2 * y.cos();
                Ok(())
            });
            let precise = Options {
                rtol: 1e-12,
                atol: Atol::Scalar(1e-14),
                h0: 1e-6,
                ..options()
            };
            let r = Dopri5::solve(&reduced, (0.0, 2.0), &[1.0], &precise).unwrap();
            assert!(r.success);
            let y = carrier.physical_slow(2.0, r.final_state[0]).unwrap();
            [y.cos() + carrier.value(2.0).unwrap(), y]
        };
        assert!(
            new.final_state
                .iter()
                .zip(reference)
                .all(|(y, z)| y.is_finite() && (y - z).abs() < 1e-5)
        );
        assert_eq!(
            new.stats.attempts,
            new.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
        if eps == 1e-6 {
            let crossed = new
                .stats
                .segments
                .iter()
                .find(|s| {
                    !s.is_probe && s.entry_probe_reason == Some(EntryProbeReason::WorkRegimeRelaxed)
                })
                .unwrap();
            let compared = new
                .stats
                .segments
                .iter()
                .find(|s| {
                    s.is_probe
                        && s.entry_probe_reason == Some(EntryProbeReason::PendingRegimeComparison)
                })
                .unwrap();
            assert_eq!(compared.start_time, crossed.reached_time);
            assert!(compared.target_time > compared.start_time);
            assert!(
                new.stats.rhs_calls < old.stats.rhs_calls,
                "new={} old={}",
                new.stats.rhs_calls,
                old.stats.rhs_calls
            );
        }
    }
}
