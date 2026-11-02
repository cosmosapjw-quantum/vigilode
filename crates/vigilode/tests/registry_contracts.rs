use std::sync::atomic::{AtomicUsize, Ordering};
use vigilode::{
    CallbackError, DerivativeFallback, Options, Problem, RouteId, SolverState, SwitchingConfig,
    SwitchingSolver, TransitionCause, compute_provider_epoch, next_solver_state,
};

#[test]
fn state_machine_table_driven_all_transitions() {
    use SolverState::*;
    use TransitionCause::*;

    // 1. All valid transitions specified in §5.3
    let valid_cases: &[(SolverState, TransitionCause, SolverState)] = &[
        (Initial, InitialStart, ExplicitTenure),
        (ExplicitTenure, ExplicitBatchDone, ExplicitTenure),
        (ExplicitTenure, PressureAndDue, EntryTrial),
        (ExplicitTenure, FatalStop, Stopped),
        (ExplicitTenure, BudgetStop, Stopped),
        (EntryTrial, PilotWonStrict, ImplicitTenure),
        (EntryTrial, PilotLostOrCeiling, ExplicitTenure),
        (EntryTrial, FatalStop, Stopped),
        (ImplicitTenure, ImplicitBatchDone, ImplicitTenure),
        (ImplicitTenure, RecheckDue, ReturnTrial),
        (ImplicitTenure, SentinelDue, ReturnSentinel),
        (ImplicitTenure, FatalStop, Stopped),
        (ImplicitTenure, BudgetStop, Stopped),
        (ReturnTrial, ReturnWonStrict, ExplicitTenure),
        (ReturnTrial, ReturnLost, ImplicitTenure),
        (ReturnSentinel, SentinelWon, ExplicitTenure),
        (ReturnSentinel, SentinelLost, ImplicitTenure),
    ];

    for &(from, cause, expected) in valid_cases {
        let actual = next_solver_state(from, cause);
        assert_eq!(
            actual,
            Ok(expected),
            "transition from {from:?} via {cause:?} should yield {expected:?}"
        );
    }

    // 2. Stopped cannot transition to anything
    let all_causes = [
        InitialStart,
        ExplicitBatchDone,
        PressureAndDue,
        PilotWonStrict,
        PilotLostOrCeiling,
        ImplicitBatchDone,
        RecheckDue,
        SentinelDue,
        ReturnWonStrict,
        ReturnLost,
        SentinelWon,
        SentinelLost,
        FatalStop,
        BudgetStop,
    ];

    for cause in all_causes {
        assert_eq!(
            next_solver_state(Stopped, cause),
            Err("cannot transition out of Stopped state")
        );
    }

    // 3. Test illegal transitions
    let all_states = [
        Initial,
        ExplicitTenure,
        EntryTrial,
        ImplicitTenure,
        ReturnTrial,
        ReturnSentinel,
    ];

    let valid_set: std::collections::HashSet<(SolverState, TransitionCause)> = valid_cases
        .iter()
        .map(|&(from, cause, _)| (from, cause))
        .collect();

    for &from in &all_states {
        for &cause in &all_causes {
            if !valid_set.contains(&(from, cause)) {
                assert!(
                    next_solver_state(from, cause).is_err(),
                    "transition from {from:?} via {cause:?} should be illegal"
                );
            }
        }
    }
}

#[test]
fn state_machine_batch_stop_coverage() {
    // Section 10.6: add one transition test per BatchStop variant
    use SolverState::*;
    use TransitionCause::*;

    // Completed
    assert_eq!(
        next_solver_state(ExplicitTenure, ExplicitBatchDone),
        Ok(ExplicitTenure)
    );
    // Budget
    assert_eq!(next_solver_state(ExplicitTenure, BudgetStop), Ok(Stopped));
    // Ceiling / ProgressBound
    assert_eq!(
        next_solver_state(EntryTrial, PilotLostOrCeiling),
        Ok(ExplicitTenure)
    );
    // Fatal
    assert_eq!(next_solver_state(EntryTrial, FatalStop), Ok(Stopped));
}

#[test]
fn distinct_routes_have_distinct_provider_epochs() {
    // Section 6.1: evidence from two different routes can never compare as equal-provider
    let problem = Problem::new(2, |_t, _y, dy| {
        dy[0] = 0.0;
        dy[1] = 0.0;
        Ok(())
    });

    let routes = [
        RouteId::Dopri5Rhs,
        RouteId::Rodas5pDense,
        RouteId::Rodas5pBanded,
        RouteId::Rodas5pFiniteDifference,
        RouteId::Rodas5pMatrixFree,
        RouteId::Rodas5pJvpAssembled,
        RouteId::Pexprb54s4Fused,
    ];

    let mut epochs = Vec::new();
    for &route in &routes {
        epochs.push(compute_provider_epoch(route, &problem));
    }

    for i in 0..epochs.len() {
        for j in (i + 1)..epochs.len() {
            assert_ne!(
                epochs[i], epochs[j],
                "routes {:?} and {:?} must have distinct provider epochs",
                routes[i], routes[j]
            );
        }
    }
}

#[test]
fn audit_replay_safety_detects_non_deterministic_callbacks() {
    // Section 10.4: evaluate RHS twice on checkpoint; mismatch refuses competitive switching
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    CALL_COUNT.store(0, Ordering::SeqCst);
    let problem = Problem::new(1, |_t, _y, dy| {
        let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        // Returns different values on successive evaluations of the exact same state!
        dy[0] = count as f64 * 0.1;
        Ok::<(), CallbackError>(())
    })
    .jacobian(vigilode::Jacobian::dense(|_t, _y, jac| {
        jac[0] = -1.0;
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        h0: 0.1,
        max_steps: 100,
        ..Default::default()
    };

    let mut config = SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly);
    config.audit_replay_safety = true;
    config.window_count = 2;
    config.pressure_steps = 1; // force work pressure to trigger pilot

    let sol = SwitchingSolver::solve(&problem, (0.0, 1.0), &[1.0], &opts, &config).unwrap();
    // Verify that the switcher detected the replay mismatch and refused switching
    let has_mismatch_decision = sol.stats.segments.iter().any(|s| {
        s.decision
            .contains("callback replay mismatch: RHS or JVP non-deterministic")
    });
    assert!(
        has_mismatch_decision,
        "solver should have recorded callback replay mismatch decision"
    );
    // Solver should never have committed a Rodas5p segment
    assert!(
        sol.stats
            .segments
            .iter()
            .all(|s| s.method == vigilode::SwitchingMethod::Dopri5 || !s.committed),
        "non-deterministic callbacks must never be committed to an implicit route"
    );
}

#[test]
fn audit_replay_safety_allows_deterministic_callbacks() {
    // Deterministic problem passes audit replay safety
    let problem = Problem::new(1, |_t, y, dy| {
        dy[0] = -100.0 * y[0];
        Ok::<(), CallbackError>(())
    })
    .jacobian(vigilode::Jacobian::dense(|_t, _y, jac| {
        jac[0] = -100.0;
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        h0: 0.001,
        max_steps: 1000,
        ..Default::default()
    };

    let mut config = SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly);
    config.audit_replay_safety = true;
    config.window_count = 4;
    config.pressure_steps = 2;

    let sol = SwitchingSolver::solve(&problem, (0.0, 1.0), &[1.0], &opts, &config).unwrap();
    // Solver must succeed without callback replay mismatch
    assert!(sol.success);
    assert!(
        !sol.stats
            .segments
            .iter()
            .any(|s| { s.decision.contains("callback replay mismatch") })
    );
}
