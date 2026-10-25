use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingEvidence,
    SwitchingMethod, SwitchingSolver,
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
        h0: 1e-7,
        max_steps: 20000,
        ..Default::default()
    }
}
fn config() -> SwitchingConfig {
    SwitchingConfig {
        window_count: 8,
        explicit_attempt_budget: 16,
        pressure_steps: 12,
        implicit_pilot_attempt_budget: 8,
        implicit_pilot_limit: 2,
        entry_pilot_cooldown_batches: 3,
        ..SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
    }
}
#[test]
fn native_explicit_step_proposal_is_carried_only_from_committed_state() {
    let r = SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &config())
        .unwrap();
    assert!(r.success, "{}", r.message);
    assert_eq!(r.stats.switches, 0);
    let committed: Vec<_> = r.stats.segments.iter().filter(|s| s.committed).collect();
    assert!(committed.len() > 4);
    for pair in committed.windows(2) {
        assert_eq!(pair[0].method, SwitchingMethod::Dopri5);
        assert_eq!(
            pair[1].initial_step,
            pair[0]
                .proposed_next_step
                .unwrap()
                .min(pair[1].target_time - pair[1].start_time)
        );
    }
    assert!((r.final_state[0] - 100.0_f64.cos()).hypot(r.final_state[1] - 100.0_f64.sin()) < 1e-5);
}
#[test]
fn carry_removes_repeated_startup_work_without_relaxing_tolerance() {
    let c = config();
    let mut old = c.clone();
    old.carry_explicit_step = false;
    let o = options();
    let p = rotation();
    let baseline = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &o, &old).unwrap();
    let pilot = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &o, &c).unwrap();
    assert!(baseline.success && pilot.success);
    assert_eq!(baseline.stats.switches, 0);
    assert_eq!(pilot.stats.switches, 0);
    assert!(
        pilot.stats.rhs_calls < baseline.stats.rhs_calls,
        "pilot {} baseline {}",
        pilot.stats.rhs_calls,
        baseline.stats.rhs_calls
    );
    for r in [&baseline, &pilot] {
        assert!(
            (r.final_state[0] - 100.0_f64.cos()).hypot(r.final_state[1] - 100.0_f64.sin()) < 1e-5
        );
    }
}
#[test]
fn entry_pilots_are_separated_bounded_and_fully_charged() {
    let c = config();
    let r = SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(), &c).unwrap();
    let mut committed_explicit: usize = 0;
    let mut previous: Option<usize> = None;
    let mut pilots: usize = 0;
    for s in &r.stats.segments {
        if s.is_probe && s.method == SwitchingMethod::Rodas5p {
            pilots += 1;
            if let Some(last) = previous {
                assert!(committed_explicit - last >= c.entry_pilot_cooldown_batches);
            }
            previous = Some(committed_explicit);
            assert!(!s.committed);
            assert!(s.attempts <= c.implicit_pilot_attempt_budget);
            assert!(s.comparator_work_ratio.is_some());
            assert!(matches!(
                s.evidence,
                SwitchingEvidence::ImplicitPilotIncomplete
                    | SwitchingEvidence::ImplicitNotCheaperOnPrefix
            ));
        }
        if s.committed && s.method == SwitchingMethod::Dopri5 {
            committed_explicit += 1;
        }
    }
    assert!(pilots > 0 && pilots <= c.implicit_pilot_limit);
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
fn evidence_factory_never_grants_derivative_permission() {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("unauthorized derivative")))
    .autonomous();
    let c = SwitchingConfig::evidence_driven(DerivativeFallback::Forbidden);
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(), &c).unwrap();
    assert!(r.success);
    assert_eq!(r.stats.switches, 0);
    assert_eq!(r.stats.jacobian_builds, 0);
    assert_eq!(r.stats.segments.len(), 1);
}

#[test]
fn implicit_advance_invalidates_saved_explicit_proposal() {
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
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        carry_explicit_step: true,
        implicit_pilot_attempt_budget: 0,
        explicit_attempt_budget: 12,
        pressure_steps: 8,
        implicit_residence: 1,
        probe_interval: 1,
        ..Default::default()
    };
    let o = options();
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &o, &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(
        r.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p)
    );
    let mut probes = 0;
    for s in &r.stats.segments {
        if s.method == SwitchingMethod::Rodas5p {
            assert!(s.proposed_next_step.is_none());
        } else if s.is_probe {
            probes += 1;
            assert_eq!(s.initial_step, o.h0.min(s.target_time - s.start_time));
        }
    }
    assert!(probes > 0);
}

#[test]
fn completed_window_carries_post_acceptance_proposal_not_terminal_snapshot() {
    let p = Problem::new(1, |_, _, out| {
        out[0] = 1.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = 0.0;
        Ok(())
    }))
    .autonomous();
    let o = Options {
        h0: 0.1,
        ..options()
    };
    let c = SwitchingConfig {
        window_count: 10,
        pressure_steps: usize::MAX,
        ..SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[0.0], &o, &c).unwrap();
    assert!(r.success, "{}", r.message);
    let first = &r.stats.segments[0];
    assert_eq!(first.reached_time, 0.1);
    assert_eq!(first.initial_step, 0.1);
    assert!(first.success);
    assert!((first.proposed_next_step.unwrap() - 0.5).abs() < 1e-15);
    assert!(
        r.stats
            .segments
            .last()
            .unwrap()
            .proposed_next_step
            .is_none()
    );
    assert!((r.final_state[0] - 1.0).abs() < 1e-14);
}

#[test]
fn one_attempt_completed_window_is_success_without_renewing_global_budget() {
    let p = Problem::new(1, |_, _, out| {
        out[0] = 1.0;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = 0.0;
        Ok(())
    }))
    .autonomous();
    let c = SwitchingConfig {
        window_count: 10,
        explicit_attempt_budget: 1,
        pressure_steps: usize::MAX,
        ..SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
    };
    for budget in [1, 20] {
        let o = Options {
            h0: 0.1,
            max_steps: budget,
            ..options()
        };
        let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[0.0], &o, &c).unwrap();
        assert_eq!(r.success, budget > 1);
        assert_eq!(r.stats.switches, 0);
        assert_eq!(r.stats.attempts, if budget == 1 { 1 } else { 10 });
        assert!(
            r.stats
                .segments
                .iter()
                .all(|s| s.success && !s.is_probe && s.attempts == 1)
        );
        if budget == 1 {
            assert_eq!(r.final_time, 0.1);
            assert!(r.message.contains("total attempt budget exhausted"));
        } else {
            assert_eq!(r.final_time, 1.0);
        }
    }
}
