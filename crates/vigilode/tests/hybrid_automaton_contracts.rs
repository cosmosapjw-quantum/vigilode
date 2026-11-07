//! Contracts for Node P8: Hybrid Route Automaton, Anti-Chattering,
//! Route Epoch Tracking, Speculative Work Retention, and Zero-Allocation (VG-1).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rodas5p_core::WorkCounters;
use vigilode::hybrid_automaton::{
    AntiChatteringPolicy, HybridAutomatonError, HybridRouteAutomaton, ProtectedFallbackGuarantor,
    RouteEpochTracker, RouteFsmEvent, RouteFsmState, SpeculativeWorkTracker,
};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingSolver,
};

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAlloc;

fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let result = f();
    TRACK.with(|track| track.set(false));
    (result, COUNT.with(Cell::get))
}

#[test]
fn test_permitted_transitions_complete_lifecycle() {
    let mut automaton = HybridRouteAutomaton::new();
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);
    assert_eq!(automaton.transition_count(), 0);

    // 1. Start solve
    automaton
        .handle_event(RouteFsmEvent::StartSolve)
        .expect("StartSolve should succeed on ExplicitActive");
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);

    // 2. Pressure or stiffness detected -> Screening
    automaton
        .handle_event(RouteFsmEvent::PressureOrStiffnessDetected)
        .expect("PressureOrStiffnessDetected should transition to ScreeningActive");
    assert_eq!(automaton.state(), RouteFsmState::ScreeningActive);

    // 3. Screen passed with evidence -> Implicit Pilot Trial
    automaton
        .handle_event(RouteFsmEvent::ScreenPassedWithEvidence)
        .expect("ScreenPassedWithEvidence should transition to ImplicitPilotTrial");
    assert_eq!(automaton.state(), RouteFsmState::ImplicitPilotTrial);

    // 4. Pilot completed cheaper -> Implicit Active
    automaton
        .handle_event(RouteFsmEvent::PilotCompletedCheaper)
        .expect("PilotCompletedCheaper should transition to ImplicitActive");
    assert_eq!(automaton.state(), RouteFsmState::ImplicitActive);

    // 4b. Implicit window accepted while in implicit active
    automaton
        .handle_event(RouteFsmEvent::ImplicitWindowAccepted)
        .expect("ImplicitWindowAccepted should succeed on ImplicitActive");
    assert_eq!(automaton.state(), RouteFsmState::ImplicitActive);

    // 5. Return sentinel due -> Explicit Return Pilot
    automaton
        .handle_event(RouteFsmEvent::ReturnSentinelDue)
        .expect("ReturnSentinelDue should transition to ExplicitReturnPilot");
    assert_eq!(automaton.state(), RouteFsmState::ExplicitReturnPilot);

    // 6. Return pilot cheaper -> Explicit Active
    automaton
        .handle_event(RouteFsmEvent::ReturnPilotCheaper)
        .expect("ReturnPilotCheaper should transition to ExplicitActive");
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);

    // 7. Complete solve -> Terminated
    automaton
        .handle_event(RouteFsmEvent::CompleteSolve)
        .expect("CompleteSolve should transition to Terminated");
    assert_eq!(automaton.state(), RouteFsmState::Terminated);
    assert!(automaton.state().is_terminal());

    // Verify history and transition count
    assert_eq!(automaton.transition_count(), 8);
    let last = automaton.last_transition().expect("last transition exists");
    assert_eq!(last.from, RouteFsmState::ExplicitActive);
    assert_eq!(last.to, RouteFsmState::Terminated);
    assert_eq!(last.event, RouteFsmEvent::CompleteSolve);
}

#[test]
fn test_illegal_transitions_hard_fail() {
    let mut automaton = HybridRouteAutomaton::new();
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);

    // From ExplicitActive, jumping directly to ExplicitReturnPilot or Terminated without event fails
    let err = automaton
        .handle_event(RouteFsmEvent::ReturnSentinelDue)
        .expect_err("ReturnSentinelDue should fail from ExplicitActive");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::ExplicitActive,
            event: RouteFsmEvent::ReturnSentinelDue,
        }
    );

    let err = automaton
        .handle_event(RouteFsmEvent::ReturnPilotCheaper)
        .expect_err("ReturnPilotCheaper should fail from ExplicitActive");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::ExplicitActive,
            event: RouteFsmEvent::ReturnPilotCheaper,
        }
    );

    let err = automaton
        .handle_event(RouteFsmEvent::PilotCompletedCheaper)
        .expect_err("PilotCompletedCheaper should fail from ExplicitActive");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::ExplicitActive,
            event: RouteFsmEvent::PilotCompletedCheaper,
        }
    );

    let err = automaton
        .handle_event(RouteFsmEvent::ImplicitWindowAccepted)
        .expect_err("ImplicitWindowAccepted should fail from ExplicitActive");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::ExplicitActive,
            event: RouteFsmEvent::ImplicitWindowAccepted,
        }
    );

    // Terminate the solver
    automaton
        .handle_event(RouteFsmEvent::CompleteSolve)
        .unwrap();
    assert_eq!(automaton.state(), RouteFsmState::Terminated);

    // From Terminated, regular transitions fail closed
    let err = automaton
        .handle_event(RouteFsmEvent::StartSolve)
        .expect_err("StartSolve must fail from Terminated");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::Terminated,
            event: RouteFsmEvent::StartSolve,
        }
    );

    let err = automaton
        .handle_event(RouteFsmEvent::PressureOrStiffnessDetected)
        .expect_err("PressureOrStiffnessDetected must fail from Terminated");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::Terminated,
            event: RouteFsmEvent::PressureOrStiffnessDetected,
        }
    );
}

#[test]
fn test_anti_chattering_dwell_and_hysteresis() {
    let policy = AntiChatteringPolicy::default();
    assert_eq!(policy.min_dwell_batches, 3);
    assert!((policy.hysteresis_enter_threshold - 0.8).abs() < 1e-12);
    assert!((policy.hysteresis_exit_threshold - 0.4).abs() < 1e-12);

    // 1. Dwell time blocks premature switching to implicit
    assert!(!policy.can_switch_to_implicit(0, 0.95));
    assert!(!policy.can_switch_to_implicit(1, 0.95));
    assert!(!policy.can_switch_to_implicit(2, 0.95));
    // Dwell >= 3 allows switch only if metric >= 0.8
    assert!(!policy.can_switch_to_implicit(3, 0.79));
    assert!(policy.can_switch_to_implicit(3, 0.80));
    assert!(policy.can_switch_to_implicit(4, 0.90));

    // 2. Dwell time blocks premature switching to explicit
    assert!(!policy.can_switch_to_explicit(0, 0.1));
    assert!(!policy.can_switch_to_explicit(1, 0.1));
    assert!(!policy.can_switch_to_explicit(2, 0.1));
    // Dwell >= 3 allows switch only if metric <= 0.4
    assert!(!policy.can_switch_to_explicit(3, 0.45));
    assert!(policy.can_switch_to_explicit(3, 0.40));
    assert!(policy.can_switch_to_explicit(5, 0.35));

    // 3. Hysteresis gap [0.4, 0.8] prevents chattering under oscillation
    for dwell in 3..10 {
        let in_gap = 0.6;
        assert!(!policy.can_switch_to_implicit(dwell, in_gap));
        assert!(!policy.can_switch_to_explicit(dwell, in_gap));
    }
}

#[test]
fn test_route_epoch_invalidates_stale_evidence() {
    let mut tracker = RouteEpochTracker::new();
    assert_eq!(tracker.current_epoch(), 0);

    let ticket0 = tracker.issue_ticket();
    assert_eq!(ticket0, 0);
    assert!(tracker.is_evidence_valid(ticket0));

    // Route switch increments epoch
    let new_epoch = tracker.increment_epoch();
    assert_eq!(new_epoch, 1);
    assert_eq!(tracker.current_epoch(), 1);

    // Stale ticket from epoch 0 is invalidated
    assert!(!tracker.is_evidence_valid(ticket0));

    // New ticket from epoch 1 is valid
    let ticket1 = tracker.issue_ticket();
    assert_eq!(ticket1, 1);
    assert!(tracker.is_evidence_valid(ticket1));

    // Verify automaton epoch tracking on route switches
    let mut automaton = HybridRouteAutomaton::new();
    let initial_ticket = automaton.epoch_tracker().issue_ticket();
    assert_eq!(initial_ticket, 0);
    assert!(automaton.epoch_tracker().is_evidence_valid(initial_ticket));

    // Transition explicit -> implicit pilot -> implicit active
    automaton
        .handle_event(RouteFsmEvent::PressureOrStiffnessDetected)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::ScreenPassedWithEvidence)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::PilotCompletedCheaper)
        .unwrap();
    assert_eq!(automaton.state(), RouteFsmState::ImplicitActive);

    // Route switch incremented epoch: stale ticket is rejected
    assert_eq!(automaton.epoch_tracker().current_epoch(), 1);
    assert!(!automaton.epoch_tracker().is_evidence_valid(initial_ticket));
}

#[test]
fn test_speculative_work_retained_on_rollback() {
    let mut tracker = SpeculativeWorkTracker::new();

    // 1. Record committed baseline work
    let init = WorkCounters {
        rhs_calls: 25,
        accepted_steps: 4,
        ..Default::default()
    };
    tracker.record_committed(init);

    assert_eq!(tracker.committed_work().rhs_calls, 25);
    assert_eq!(tracker.total_work().rhs_calls, 25);

    // 2. Perform speculative pilot trial
    let pilot = WorkCounters {
        rhs_calls: 12,
        jvp_calls: 3,
        linear_solves: 4,
        direct_factorizations: 1,
        ..Default::default()
    };
    tracker.record_speculative(pilot);

    // Before rollback: total work includes pending speculative work
    assert_eq!(tracker.total_work().rhs_calls, 37);
    assert_eq!(tracker.total_work().jvp_calls, 3);
    assert_eq!(tracker.total_work().linear_solves, 4);
    assert_eq!(tracker.committed_work().rhs_calls, 25);
    assert_eq!(tracker.refused_work().rhs_calls, 0);

    // 3. Roll back speculative trial
    tracker.rollback_and_retain_work();

    // After rollback: pending is cleared, refused has accumulated all pilot work,
    // and total work retains 100% of spent work
    assert_eq!(tracker.pending_speculative_work().rhs_calls, 0);
    assert_eq!(tracker.committed_work().rhs_calls, 25);
    assert_eq!(tracker.refused_work().rhs_calls, 12);
    assert_eq!(tracker.refused_work().jvp_calls, 3);
    assert_eq!(tracker.refused_work().linear_solves, 4);
    assert_eq!(tracker.refused_work().direct_factorizations, 1);

    assert_eq!(tracker.total_work().rhs_calls, 37);
    assert_eq!(tracker.total_work().jvp_calls, 3);
    assert_eq!(tracker.total_work().linear_solves, 4);
    assert_eq!(tracker.total_work().direct_factorizations, 1);

    assert!(tracker.accounting_complete());
}

#[test]
fn test_protected_rodas5p_fallback_availability() {
    let all_states = [
        RouteFsmState::ExplicitActive,
        RouteFsmState::ScreeningActive,
        RouteFsmState::ImplicitPilotTrial,
        RouteFsmState::ImplicitActive,
        RouteFsmState::ExplicitReturnPilot,
        RouteFsmState::RollbackRecovery,
        RouteFsmState::ProtectedRodasFallback,
        RouteFsmState::Terminated,
    ];

    let mut guarantor = ProtectedFallbackGuarantor::new();

    for state in all_states {
        assert!(
            guarantor.is_fallback_permitted_from(state),
            "Protected RODAS5P fallback must be permitted from {:?}",
            state
        );

        let mut automaton = HybridRouteAutomaton::new();
        // Force automaton to `state` for test coverage
        automaton.set_state_for_testing(state);
        assert_eq!(automaton.state(), state);

        guarantor
            .trigger_emergency_fallback(&mut automaton)
            .unwrap_or_else(|_| panic!("Emergency fallback must succeed from {:?}", state));
        assert_eq!(
            automaton.state(),
            RouteFsmState::ProtectedRodasFallback,
            "State after emergency fallback must be ProtectedRodasFallback"
        );
    }

    assert_eq!(guarantor.fallback_invocations(), all_states.len() as u64);
}

#[test]
fn test_vg1_zero_allocation() {
    let mut automaton = HybridRouteAutomaton::new();
    let policy = AntiChatteringPolicy::default();

    // Warm up
    let _ = automaton.handle_event(RouteFsmEvent::StartSolve);

    let ((), allocs) = measured(|| {
        for _ in 0..10_000 {
            // Anti-chattering checks
            let _ = policy.can_switch_to_implicit(3, 0.85);
            let _ = policy.can_switch_to_explicit(4, 0.35);

            // Epoch checks
            let epoch = automaton.epoch_tracker().current_epoch();
            let _ = automaton.epoch_tracker().is_evidence_valid(epoch);

            // Speculative tracker updates
            automaton
                .speculative_tracker_mut()
                .record_speculative(WorkCounters {
                    rhs_calls: 1,
                    ..Default::default()
                });
            automaton
                .speculative_tracker_mut()
                .rollback_and_retain_work();

            // Automaton state inspection
            let _ = automaton.state();
            let _ = automaton.transition_count();
            let _ = automaton.last_transition();
        }
    });

    assert_eq!(
        allocs, 0,
        "VG-1 violation: 10,000 FSM evaluations must perform 0 heap allocations, got {allocs}"
    );
}

#[test]
fn test_positive_and_negative_routing_controls_with_invariants() {
    let mut automaton = HybridRouteAutomaton::new();
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);
    assert_eq!(automaton.transition_count(), 0);

    // Initial epoch and ticket
    let epoch0 = automaton.epoch_tracker().current_epoch();
    assert_eq!(epoch0, 0);
    let ticket0 = automaton.epoch_tracker().issue_ticket();
    assert!(automaton.epoch_tracker().is_evidence_valid(ticket0));

    // Baseline work
    automaton
        .speculative_tracker_mut()
        .record_committed(WorkCounters {
            rhs_calls: 50,
            accepted_steps: 10,
            ..Default::default()
        });
    assert_eq!(
        automaton.speculative_tracker().committed_work().rhs_calls,
        50
    );
    assert!(automaton.speculative_tracker().accounting_complete());

    // --- Negative Routing Control 1: Illegal transition from ExplicitActive ---
    let err = automaton
        .handle_event(RouteFsmEvent::ImplicitWindowAccepted)
        .expect_err("ImplicitWindowAccepted must be rejected from ExplicitActive");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::ExplicitActive,
            event: RouteFsmEvent::ImplicitWindowAccepted,
        }
    );
    // Invariants preserved after rejected transition
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);
    assert_eq!(automaton.transition_count(), 0);
    assert_eq!(automaton.epoch_tracker().current_epoch(), 0);
    assert!(automaton.speculative_tracker().accounting_complete());

    // --- Positive Routing Control 1: Advance to ScreeningActive ---
    automaton
        .handle_event(RouteFsmEvent::PressureOrStiffnessDetected)
        .expect("PressureOrStiffnessDetected permitted");
    assert_eq!(automaton.state(), RouteFsmState::ScreeningActive);
    assert_eq!(automaton.transition_count(), 1);
    assert_eq!(automaton.epoch_tracker().current_epoch(), 0); // intra-family: no epoch increment

    // --- Negative Routing Control 2: Illegal transition from ScreeningActive ---
    let err = automaton
        .handle_event(RouteFsmEvent::ReturnSentinelDue)
        .expect_err("ReturnSentinelDue must be rejected from ScreeningActive");
    assert_eq!(
        err,
        HybridAutomatonError::IllegalTransition {
            from: RouteFsmState::ScreeningActive,
            event: RouteFsmEvent::ReturnSentinelDue,
        }
    );
    assert_eq!(automaton.state(), RouteFsmState::ScreeningActive);
    assert_eq!(automaton.transition_count(), 1);

    // --- Positive Routing Control 2: Screen passed -> ImplicitPilotTrial ---
    automaton
        .handle_event(RouteFsmEvent::ScreenPassedWithEvidence)
        .expect("ScreenPassedWithEvidence permitted");
    assert_eq!(automaton.state(), RouteFsmState::ImplicitPilotTrial);
    assert_eq!(automaton.transition_count(), 2);

    // Record speculative pilot work
    automaton
        .speculative_tracker_mut()
        .record_speculative(WorkCounters {
            rhs_calls: 20,
            jvp_calls: 5,
            linear_solves: 6,
            ..Default::default()
        });
    assert_eq!(
        automaton
            .speculative_tracker()
            .pending_speculative_work()
            .rhs_calls,
        20
    );
    assert_eq!(automaton.speculative_tracker().total_work().rhs_calls, 70);
    assert!(automaton.speculative_tracker().accounting_complete());

    // --- Positive Routing Control 3: Pilot won -> ImplicitActive ---
    automaton
        .handle_event(RouteFsmEvent::PilotCompletedCheaper)
        .expect("PilotCompletedCheaper permitted");
    assert_eq!(automaton.state(), RouteFsmState::ImplicitActive);
    assert_eq!(automaton.transition_count(), 3);
    // Route family changed (Explicit -> Implicit): epoch must increment!
    assert_eq!(automaton.epoch_tracker().current_epoch(), 1);
    // Stale ticket0 must now be invalidated
    assert!(!automaton.epoch_tracker().is_evidence_valid(ticket0));
    // Speculative work committed
    assert_eq!(
        automaton
            .speculative_tracker()
            .pending_speculative_work()
            .rhs_calls,
        0
    );
    assert_eq!(
        automaton.speculative_tracker().committed_work().rhs_calls,
        70
    );
    assert_eq!(
        automaton.speculative_tracker().committed_work().jvp_calls,
        5
    );
    assert_eq!(
        automaton
            .speculative_tracker()
            .committed_work()
            .linear_solves,
        6
    );
    assert!(automaton.speculative_tracker().accounting_complete());

    // Issue ticket in epoch 1
    let ticket1 = automaton.epoch_tracker().issue_ticket();
    assert_eq!(ticket1, 1);
    assert!(automaton.epoch_tracker().is_evidence_valid(ticket1));

    // --- Positive Routing Control 4: ReturnSentinelDue -> ExplicitReturnPilot ---
    automaton
        .handle_event(RouteFsmEvent::ReturnSentinelDue)
        .expect("ReturnSentinelDue permitted");
    assert_eq!(automaton.state(), RouteFsmState::ExplicitReturnPilot);
    assert_eq!(automaton.transition_count(), 4);

    // Record speculative explicit recheck work
    automaton
        .speculative_tracker_mut()
        .record_speculative(WorkCounters {
            rhs_calls: 15,
            ..Default::default()
        });
    assert_eq!(
        automaton
            .speculative_tracker()
            .pending_speculative_work()
            .rhs_calls,
        15
    );
    assert_eq!(automaton.speculative_tracker().total_work().rhs_calls, 85);

    // --- Negative Routing Control 3 (Speculative Discard): Return pilot failed/expensive -> ImplicitActive ---
    automaton
        .handle_event(RouteFsmEvent::ReturnPilotFailedOrExpensive)
        .expect("ReturnPilotFailedOrExpensive permitted");
    assert_eq!(automaton.state(), RouteFsmState::ImplicitActive);
    assert_eq!(automaton.transition_count(), 5);
    // Pending work rolled into refused work, NOT lost
    assert_eq!(
        automaton
            .speculative_tracker()
            .pending_speculative_work()
            .rhs_calls,
        0
    );
    assert_eq!(automaton.speculative_tracker().refused_work().rhs_calls, 15);
    assert_eq!(
        automaton.speculative_tracker().committed_work().rhs_calls,
        70
    );
    assert_eq!(automaton.speculative_tracker().total_work().rhs_calls, 85);
    assert!(automaton.speculative_tracker().accounting_complete());

    // Anti-chattering negative controls:
    let policy = automaton.anti_chattering();
    assert!(!policy.can_switch_to_implicit(1, 0.9)); // dwell too small
    assert!(!policy.can_switch_to_implicit(5, 0.7)); // threshold not met
    assert!(!policy.can_switch_to_explicit(1, 0.2)); // dwell too small
    assert!(!policy.can_switch_to_explicit(5, 0.5)); // threshold not met
}

fn rotation_problem() -> Problem {
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

fn robertson_problem() -> Problem {
    Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, j| {
        j.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            0.0,
            6.0e7 * y[1],
            0.0,
        ]);
        Ok(())
    }))
    .autonomous()
}

#[test]
fn test_switching_solver_runtime_traces_and_work_preservation() {
    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-8),
        h0: 1e-4,
        max_steps: 100_000,
        ..Default::default()
    };

    // Case 1: Live dispatch on rotation (nonstiff) system
    // Live controller (DriveState) retains explicit execution without spurious switches
    let config_nonstiff = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    let sol_rot = SwitchingSolver::solve(
        &rotation_problem(),
        (0.0, 1.0),
        &[1.0, 0.0],
        &opts,
        &config_nonstiff,
    )
    .expect("SwitchingSolver rotation solve should succeed");

    assert!(sol_rot.success);
    assert_eq!(sol_rot.final_time, 1.0);
    // Physical invariant: energy/radius conservation |r^2 - 1| < 1e-4
    let r2 = sol_rot.final_state[0].powi(2) + sol_rot.final_state[1].powi(2);
    assert!(
        (r2 - 1.0).abs() < 1e-4,
        "Radius invariant violated: r2 = {r2}"
    );
    // Work accounting and preservation
    assert!(sol_rot.stats.rhs_calls > 0);
    assert!(sol_rot.stats.attempts > 0);
    assert!(sol_rot.stats.accounting_complete);
    // Negative routing control: nonstiff rotation does not switch to Rodas5p
    assert_eq!(sol_rot.stats.switches, 0);

    // Case 2: Live dispatch on Robertson chemical kinetics (stiff)
    // Positive routing control: pre-screen detects stiffness and routes to Rodas5p
    let mut config_stiff = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    config_stiff.prescreen_initial_stiffness = true;
    let sol_rob = SwitchingSolver::solve(
        &robertson_problem(),
        (0.0, 10.0),
        &[1.0, 0.0, 0.0],
        &opts,
        &config_stiff,
    )
    .expect("SwitchingSolver Robertson solve should succeed");

    assert!(sol_rob.success);
    assert_eq!(sol_rob.final_time, 10.0);
    // Physical invariant: total mass conservation y0 + y1 + y2 = 1.0
    let mass = sol_rob.final_state[0] + sol_rob.final_state[1] + sol_rob.final_state[2];
    assert!(
        (mass - 1.0).abs() < 1e-6,
        "Mass conservation invariant violated: mass = {mass}"
    );
    // Work accounting and preservation
    assert!(sol_rob.stats.rhs_calls > 0);
    assert!(sol_rob.stats.jacobian_builds > 0);
    assert!(sol_rob.stats.linear_solves > 0);
    assert!(sol_rob.stats.accounting_complete);

    // Case 3: Negative routing control under forbidden derivatives
    // Solver must never perform unauthorized Jacobian builds when DerivativeFallback::Forbidden
    let config_forbidden = SwitchingConfig {
        prescreen_initial_stiffness: false,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::Forbidden)
    };
    let opts_short = Options {
        max_steps: 1_000,
        ..opts
    };
    let sol_forbidden = SwitchingSolver::solve(
        &robertson_problem(),
        (0.0, 1.0),
        &[1.0, 0.0, 0.0],
        &opts_short,
        &config_forbidden,
    )
    .expect("Forbidden solve returns result with failure or success");

    // Negative routing verification: zero Jacobian builds authorized or performed
    assert_eq!(sol_forbidden.stats.jacobian_builds, 0);
    assert!(sol_forbidden.stats.accounting_complete);
}
