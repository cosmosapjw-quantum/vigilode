use vigilode::{
    Atol, CallbackError, DerivativeFallback, EntryProbeReason, Jacobian, Options, Problem,
    SwitchingConfig, SwitchingEvidence, SwitchingMethod, SwitchingSolver,
};

fn rotation() -> Problem {
    Problem::new(2, |_, y, out| {
        out[0] = -100.0 * y[1];
        out[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, out| {
        out[0] = -100.0 * v[1];
        out[1] = 100.0 * v[0];
        Ok(())
    }))
    .autonomous()
}

fn options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 50_000,
        ..Default::default()
    }
}

#[test]
fn adaptive_entry_does_not_stack_legacy_rate_rearm_or_backoff() {
    let mut config = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    config.pilot_backoff_factor = 1;
    config.rearm_rate_ratio = 1.01;
    let a =
        SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &config).unwrap();
    config.pilot_backoff_factor = usize::MAX;
    config.rearm_rate_ratio = 1e100;
    let b =
        SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &config).unwrap();
    assert!(a.success && b.success);
    assert_eq!(a.stats.switches, 0);
    assert_eq!(b.stats.switches, 0);
    assert_eq!(a.final_state, b.final_state);
    assert!((a.final_state[0] - 100.0_f64.cos()).abs() < 1e-5);
    assert!((a.final_state[1] - 100.0_f64.sin()).abs() < 1e-5);
    assert_eq!(
        serde_json::to_value(&a.stats).unwrap(),
        serde_json::to_value(&b.stats).unwrap()
    );
    assert!(
        a.stats
            .segments
            .iter()
            .any(|s| s.is_probe && s.decision.contains("ceiling"))
    );
}

#[test]
fn legacy_explicit_tiny_representable_span_uses_native_endpoint_clipping() {
    let p = Problem::new(1, |_, _, out| {
        out[0] = 1.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| {
        panic!("short explicit span must not enter derivatives")
    }))
    .autonomous();
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 1,
        pressure_steps: usize::MAX,
        carry_explicit_step: false,
        ..Default::default()
    };
    let end = 1e-15;
    let r = SwitchingSolver::solve(&p, (0.0, end), &[0.0], &options(), &config).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(r.stats.accounting_complete);
    assert_eq!(r.final_time, end);
    assert!((r.final_state[0] - end).abs() < 1e-29);
    assert_eq!(r.stats.attempts, 1);
    assert_eq!(r.stats.jacobian_builds, 0);
}

#[test]
fn adaptive_bounded_entry_preserves_fatal_derivative_failure() {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| {
        Err(CallbackError::Fatal("fatal pilot Jacobian".into()))
    }))
    .autonomous();
    let config = SwitchingConfig {
        explicit_attempt_budget: 8,
        pressure_steps: 1,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(), &config).unwrap();
    assert!(!r.success);
    assert!(r.message.contains("fatal implicit pilot"), "{}", r.message);
    assert!(r.stats.accounting_complete);
    assert_eq!(r.stats.switches, 0);
    let pilot = r.stats.segments.iter().find(|s| s.is_probe).unwrap();
    assert_eq!(pilot.method, SwitchingMethod::Rodas5p);
    assert!(!pilot.success && !pilot.committed);
    assert_eq!(pilot.jacobian_builds, 1);
    assert_eq!(pilot.evidence, SwitchingEvidence::ImplicitPilotIncomplete);
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
}

#[test]
fn completed_winning_entry_pilot_can_seed_only_the_next_implicit_proposal() {
    assert!(!SwitchingConfig::default().reuse_entry_pilot_step);
    assert!(
        !SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly).reuse_entry_pilot_step
    );
    let p = Problem::new(1, |t, y, out| {
        out[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10000.0;
        Ok(())
    }))
    .time_derivative(|t, _, out| {
        out[0] = -10000.0 * t.sin() - t.cos();
        Ok(())
    });
    let opts = Options {
        h0: 1e-7,
        ..options()
    };
    let mut config = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    assert!(config.reuse_entry_pilot_step);
    let seeded = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts, &config).unwrap();
    config.reuse_entry_pilot_step = false;
    let unseeded = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &opts, &config).unwrap();
    let mut first_pilot_work = Vec::new();
    for (r, reuse) in [(&seeded, true), (&unseeded, false)] {
        assert!(r.success, "{}", r.message);
        assert!(r.stats.accounting_complete);
        assert!((r.final_state[0] - 1.0_f64.cos()).abs() < 1e-6);
        let i = r
            .stats
            .segments
            .iter()
            .position(|s| s.is_probe && s.evidence == SwitchingEvidence::ImplicitCheaperOnPrefix)
            .expect("fixture must enter through a completed economical pilot");
        let pilot = &r.stats.segments[i];
        assert!(pilot.success && !pilot.committed && pilot.accepted_steps > 0);
        let accepted_explicit = &r.stats.segments[i + 1];
        assert!(accepted_explicit.committed && accepted_explicit.method == SwitchingMethod::Dopri5);
        assert_eq!(pilot.target_time, accepted_explicit.reached_time);
        let next = r.stats.segments[i + 2..]
            .iter()
            .find(|s| !s.is_probe && s.method == SwitchingMethod::Rodas5p)
            .expect("winning pilot must be followed by productive implicit integration");
        assert_eq!(next.start_time, accepted_explicit.reached_time);
        let proposal = if reuse {
            (pilot.reached_time - pilot.start_time) / pilot.accepted_steps as f64
        } else {
            opts.h0
        };
        let expected = proposal.min(next.target_time - next.start_time).max(1e-14);
        assert_eq!(next.initial_step.to_bits(), expected.to_bits());
        if reuse {
            assert!(next.initial_step > 2.0 * opts.h0);
        }
        first_pilot_work.push((pilot.attempts, pilot.rhs_calls, pilot.work_units));
        assert_eq!(
            r.stats.attempts,
            r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
        assert_eq!(
            r.stats.accepted_steps,
            r.stats
                .segments
                .iter()
                .map(|s| s.accepted_steps)
                .sum::<usize>()
        );
        assert_eq!(
            r.stats.rejected_steps,
            r.stats
                .segments
                .iter()
                .map(|s| s.rejected_steps)
                .sum::<usize>()
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
            r.stats.jvp_vectors,
            r.stats.segments.iter().map(|s| s.jvp_vectors).sum::<u64>()
        );
        assert_eq!(
            r.stats.ft_calls,
            r.stats.segments.iter().map(|s| s.ft_calls).sum::<u64>()
        );
        assert_eq!(
            r.stats.linear_solves,
            r.stats
                .segments
                .iter()
                .map(|s| s.linear_solves)
                .sum::<u64>()
        );
    }
    assert_eq!(first_pilot_work[0], first_pilot_work[1]);
}

#[test]
fn rejected_entry_pilots_never_supply_a_seed() {
    let mut config = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    let a =
        SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &config).unwrap();
    config.reuse_entry_pilot_step = false;
    let b =
        SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &config).unwrap();
    assert!(a.success && b.success && a.stats.switches == 0 && b.stats.switches == 0);
    assert_eq!(a.final_state, b.final_state);
    assert_eq!(
        serde_json::to_value(&a.stats).unwrap(),
        serde_json::to_value(&b.stats).unwrap()
    );
}

#[test]
fn unresolved_entry_retry_is_once_bounded_and_preserves_accepted_explicit_states() {
    let p = Problem::new(2, |_, y, out| {
        out[0] = -100.0 * y[1];
        out[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| {
        Err(CallbackError::Recoverable(
            "pilot cannot accept this trial".into(),
        ))
    }))
    .autonomous();
    let mut config = SwitchingConfig {
        implicit_pilot_attempt_budget: 1,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    };
    assert!(config.retry_unresolved_entry_once);
    assert!(!SwitchingConfig::default().retry_unresolved_entry_once);
    assert!(
        !SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
            .retry_unresolved_entry_once
    );
    let retried = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &options(), &config).unwrap();
    config.retry_unresolved_entry_once = false;
    let baseline =
        SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &options(), &config).unwrap();
    assert!(retried.success && baseline.success);
    assert_eq!(retried.t, baseline.t);
    assert_eq!(retried.y, baseline.y);
    assert_eq!(retried.final_state, baseline.final_state);
    assert!((retried.final_state[0] - 100.0_f64.cos()).abs() < 1e-5);
    assert!((retried.final_state[1] - 100.0_f64.sin()).abs() < 1e-5);
    assert_eq!(retried.stats.switches, 0);
    let pilots: Vec<_> = retried
        .stats
        .segments
        .iter()
        .filter(|s| s.is_probe)
        .collect();
    assert!(pilots.len() >= 2);
    assert!(
        pilots
            .iter()
            .all(|s| s.accepted_steps == 0 && !s.committed && !s.success)
    );
    assert_eq!(
        pilots
            .iter()
            .filter(|s| s.entry_probe_reason == Some(EntryProbeReason::UnresolvedEntryRetry))
            .count(),
        1
    );
    let first_i = retried
        .stats
        .segments
        .iter()
        .position(|s| s.is_probe)
        .unwrap();
    let first_explicit = &retried.stats.segments[first_i + 1];
    assert!(first_explicit.committed);
    assert_eq!(pilots[1].start_time, first_explicit.reached_time);
    assert!(
        !baseline
            .stats
            .segments
            .iter()
            .any(|s| s.entry_probe_reason == Some(EntryProbeReason::UnresolvedEntryRetry))
    );
    config.retry_unresolved_entry_once = true;
    let limited_opts = Options {
        max_steps: 70,
        ..options()
    };
    let limited =
        SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &limited_opts, &config).unwrap();
    assert!(!limited.success);
    assert_eq!(limited.stats.attempts, limited_opts.max_steps);
    for r in [&retried, &baseline, &limited] {
        assert!(r.stats.accounting_complete);
        assert_eq!(
            r.stats.attempts,
            r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
        );
        assert_eq!(
            r.stats.accepted_steps,
            r.stats
                .segments
                .iter()
                .map(|s| s.accepted_steps)
                .sum::<usize>()
        );
        assert_eq!(
            r.stats.rejected_steps,
            r.stats
                .segments
                .iter()
                .map(|s| s.rejected_steps)
                .sum::<usize>()
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
            r.stats.jvp_vectors,
            r.stats.segments.iter().map(|s| s.jvp_vectors).sum::<u64>()
        );
        assert_eq!(
            r.stats.ft_calls,
            r.stats.segments.iter().map(|s| s.ft_calls).sum::<u64>()
        );
        assert_eq!(
            r.stats.linear_solves,
            r.stats
                .segments
                .iter()
                .map(|s| s.linear_solves)
                .sum::<u64>()
        );
    }
}
