use rodas5p_integrators::AdaptiveStepConfig;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingEvidence,
    SwitchingMethod, SwitchingSolver,
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
fn config(enabled: bool) -> SwitchingConfig {
    SwitchingConfig {
        window_count: 8,
        stop_dominated_return_comparator: true,
        seed_entry_pilot_from_explicit_checkpoint: enabled,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    }
}
fn decay(rate: f64, rhs: Arc<AtomicU64>, jvp: Arc<AtomicU64>) -> Problem {
    Problem::new(16, move |_, y, out| {
        rhs.fetch_add(1, Ordering::SeqCst);
        for (o, y) in out.iter_mut().zip(y) {
            *o = -rate * y;
        }
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_, _, v, out| {
        jvp.fetch_add(1, Ordering::SeqCst);
        for (o, v) in out.iter_mut().zip(v) {
            *o = -rate * v;
        }
        Ok(())
    }))
    .autonomous()
}

#[test]
fn checkpoint_seed_is_an_independent_default_false_option() {
    assert!(!SwitchingConfig::default().seed_entry_pilot_from_explicit_checkpoint);
    assert!(
        !SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
            .seed_entry_pilot_from_explicit_checkpoint
    );
    assert!(
        !SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
            .seed_entry_pilot_from_explicit_checkpoint
    );
    for policy in [
        SwitchingConfig::default(),
        SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly),
        SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
    ] {
        assert!(!policy.strict_entry_work_improvement);
        assert_eq!(policy.entry_cost_ratio, 0.8);
    }
}

#[test]
fn each_entry_pilot_uses_its_own_start_checkpoint_and_same_replay_endpoint() {
    let p = decay(
        1000.0,
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
    );
    let o = options();
    let c = config(true);
    let out = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0; 16], &o, &c).unwrap();
    assert!(out.success);
    let mut seeded_later = 0;
    let mut pilots = 0;
    for pilot in out.stats.segments.iter().filter(|s| {
        s.is_probe
            && s.method == SwitchingMethod::Rodas5p
            && matches!(
                s.evidence,
                SwitchingEvidence::ImplicitCheaperOnPrefix
                    | SwitchingEvidence::ImplicitNotCheaperOnPrefix
                    | SwitchingEvidence::ImplicitPilotIncomplete
            )
    }) {
        pilots += 1;
        let explicit = out
            .stats
            .segments
            .iter()
            .find(|s| {
                s.method == SwitchingMethod::Dopri5
                    && !s.is_probe
                    && s.start_time == pilot.start_time
                    && s.reached_time == pilot.target_time
            })
            .unwrap();
        let expected =
            o.h0.max(explicit.initial_step)
                .min(pilot.target_time - pilot.start_time)
                .max(AdaptiveStepConfig::default().min_step);
        assert_eq!(pilot.initial_step.to_bits(), expected.to_bits());
        assert!(pilot.attempts <= c.implicit_pilot_attempt_budget);
        assert!(!pilot.committed);
        if pilot.start_time == 0.0 {
            assert_eq!(pilot.initial_step, o.h0);
        }
        if pilot.start_time > 0.0 && pilot.initial_step > o.h0 {
            seeded_later += 1;
        }
        if pilot.evidence == SwitchingEvidence::ImplicitCheaperOnPrefix {
            assert!(pilot.success);
            assert!(pilot.work_units <= c.entry_cost_ratio * explicit.work_units);
            assert_eq!(pilot.reached_time, explicit.reached_time);
        }
    }
    assert!(pilots > 0 && seeded_later > 0);
}

#[test]
fn original_accuracy_and_complete_charged_work_survive_new_seed() {
    for rate in [1.0, 1000.0, 10000.0] {
        let rhs = Arc::new(AtomicU64::new(0));
        let jvp = Arc::new(AtomicU64::new(0));
        let p = decay(rate, rhs.clone(), jvp.clone());
        let out =
            SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0; 16], &options(), &config(true)).unwrap();
        assert!(out.success && out.final_time == 1.0 && out.stats.accounting_complete);
        for (&t, y) in out.t.iter().zip(&out.y) {
            for &yi in y {
                assert!((yi - (-rate * t).exp()).abs() <= 1e-6);
            }
        }
        assert_eq!(rhs.load(Ordering::SeqCst), out.stats.rhs_calls);
        assert_eq!(jvp.load(Ordering::SeqCst), out.stats.jvp_vectors);
        assert_eq!(
            out.stats.rhs_calls,
            out.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
        );
        assert_eq!(
            out.stats.jvp_vectors,
            out.stats
                .segments
                .iter()
                .map(|s| s.jvp_vectors)
                .sum::<u64>()
        );
        assert_eq!(
            out.stats.attempts,
            out.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
        assert!(out.stats.attempts <= options().max_steps);
        assert_eq!(out.stats.jacobian_builds, 0);
        assert_eq!(out.stats.ft_calls, 0);
    }
}

#[test]
fn checkpoint_seed_does_not_authorize_a_derivative_callback() {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -1000.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, _, _| {
        panic!("unauthorized derivative")
    }))
    .autonomous();
    let c = SwitchingConfig {
        seed_entry_pilot_from_explicit_checkpoint: true,
        strict_entry_work_improvement: true,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::Forbidden)
    };
    let out = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(), &c).unwrap();
    assert!(out.success);
    assert_eq!(out.stats.jvp_vectors, 0);
    assert_eq!(out.stats.switches, 0);
}

#[test]
fn strict_entry_admits_only_a_completed_lower_integer_work_replay() {
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let p = decay(1000.0, rhs.clone(), jvp.clone());
    let mut c = config(true);
    c.strict_entry_work_improvement = true;
    let out = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0; 16], &options(), &c).unwrap();
    assert!(out.success && out.stats.accounting_complete);
    assert!(out.stats.switches > 0);
    assert_eq!(rhs.load(Ordering::SeqCst), out.stats.rhs_calls);
    assert_eq!(jvp.load(Ordering::SeqCst), out.stats.jvp_vectors);
    assert!(out.stats.attempts <= options().max_steps);
    let cost = |s: &vigilode::SwitchingSegment| -> u128 {
        s.rhs_calls as u128
            + s.jvp_vectors as u128
            + s.ft_calls as u128
            + 16 * s.jacobian_builds as u128
            + s.linear_solves as u128
    };
    let mut winners = 0;
    for pilot in out
        .stats
        .segments
        .iter()
        .filter(|s| s.is_probe && s.evidence == SwitchingEvidence::ImplicitCheaperOnPrefix)
    {
        let explicit = out
            .stats
            .segments
            .iter()
            .find(|s| {
                s.method == SwitchingMethod::Dopri5
                    && !s.is_probe
                    && s.start_time == pilot.start_time
                    && s.reached_time == pilot.target_time
            })
            .unwrap();
        assert!(pilot.success && explicit.accepted_steps > 0);
        assert_eq!(pilot.reached_time, explicit.reached_time);
        assert!(cost(pilot) < cost(explicit));
        // The exact criterion can admit this qualified improvement while
        // retaining the untouched legacy ratio in the configuration.
        assert!(pilot.work_units > c.entry_cost_ratio * explicit.work_units);
        winners += 1;
    }
    assert!(winners > 0);
}
