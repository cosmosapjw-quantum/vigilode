use vigilode::execution_policy::*;
use vigilode::selection_evidence::SelectionIdentity;

fn id() -> ExecutionIdentity {
    ExecutionIdentity {
        workload: WorkloadKey {
            model_epoch: 1,
            method_tag: 5,
            dimension: 1,
            batch_len: 8,
            layer: ExecutionLayer::Ensemble,
            environment_epoch: 1,
        },
        comparison: SelectionIdentity {
            chart_epoch: 4,
            physical_norm_epoch: 3,
            ..Default::default()
        },
    }
}
fn trained() -> ExecutionSelector {
    let mut s = ExecutionSelector::new(2, ExecutionPolicyConfig::monitored()).unwrap();
    s.begin_calibration_for(id(), 0.0).unwrap();
    for _ in 0..3 {
        s.record_pair_for(id(), 0.01, 0.004, true).unwrap();
    }
    s
}
#[test]
fn every_physical_epoch_invalidates_and_does_not_resurrect() {
    for which in 0..8 {
        let mut s = trained();
        let mut changed = id();
        match which {
            0 => changed.comparison.model_epoch += 1,
            1 => changed.comparison.provider_epoch += 1,
            2 => changed.comparison.chart_epoch += 1,
            3 => changed.comparison.mesh_epoch += 1,
            4 => changed.comparison.physical_norm_epoch += 1,
            5 => changed.comparison.tolerance_epoch += 1,
            6 => changed.comparison.window_epoch += 1,
            _ => changed.comparison.environment_epoch += 1,
        }
        assert_eq!(
            s.decide_for(changed, 10, false).reason,
            ExecutionReason::UncalibratedWorkload
        );
        assert_eq!(
            s.decide_for(id(), 10, false).reason,
            ExecutionReason::UncalibratedWorkload
        );
    }
}
#[test]
fn stale_pair_cannot_finish_calibration() {
    let mut s = trained();
    let mut changed = id();
    changed.comparison.chart_epoch += 1;
    s.begin_calibration_for(changed, 0.0).unwrap();
    assert!(s.record_pair_for(id(), 0.01, 0.004, true).is_err());
    assert_eq!(s.decide_for(changed, 10, false).mode, ExecutionMode::Serial);
}
#[test]
fn consecutive_cost_drift_expires_only_future_choices() {
    let mut s = trained();
    assert_eq!(s.decide_for(id(), 20, false).mode, ExecutionMode::Parallel);
    assert!(!s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 0.02, false));
    assert_eq!(s.decide_for(id(), 19, false).mode, ExecutionMode::Parallel);
    assert!(s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 0.02, false));
    let d = s.decide_for(id(), 18, false);
    assert_eq!(d.mode, ExecutionMode::Serial);
    assert_eq!(d.reason, ExecutionReason::ObservedCostDrift);
    assert!(d.probe_due);
    assert_eq!(s.telemetry().drift_invalidations, 1);
}
#[test]
fn in_envelope_call_resets_streak_and_nested_calls_do_not_contaminate() {
    let mut s = trained();
    assert!(!s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 0.02, false));
    assert!(!s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 0.004, false));
    assert!(!s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 0.02, false));
    assert!(!s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 100.0, true));
    assert_eq!(s.decide_for(id(), 20, false).mode, ExecutionMode::Parallel);
    assert!(s.observe_dispatch_cost(id(), ExecutionMode::Parallel, 0.02, false));
}
#[test]
fn legacy_path_has_no_dispatch_clock_and_context_dispatch_runs_once() {
    let mut executor = AdaptiveBatchExecutor::new(2, Default::default()).unwrap();
    let jobs: Vec<_> = (0..8).collect();
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let (result, d) = executor
        .map_ordered_for(id(), 10, false, &jobs, |x| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(x * x)
        })
        .unwrap();
    assert_eq!(result, jobs.iter().map(|x| x * x).collect::<Vec<_>>());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 8);
    assert!(d.observed_dispatch_seconds.is_none());
}
#[test]
fn monitored_wrapper_emits_actual_cost_and_keeps_order() {
    let mut executor = AdaptiveBatchExecutor::new(2, ExecutionPolicyConfig::monitored()).unwrap();
    let jobs: Vec<_> = (0..8).collect();
    let (result, d) = executor
        .map_ordered_for(id(), 10, false, &jobs, |x| Ok(x * x))
        .unwrap();
    assert_eq!(result, jobs.iter().map(|x| x * x).collect::<Vec<_>>());
    assert!(d.observed_dispatch_seconds.unwrap().is_finite());
}
