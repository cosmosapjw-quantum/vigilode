use vigilode::execution_policy::*;
fn key() -> WorkloadKey {
    WorkloadKey {
        model_epoch: 1,
        method_tag: 2,
        dimension: 4,
        batch_len: 32,
        layer: ExecutionLayer::Ensemble,
        environment_epoch: 3,
    }
}
fn trained(s: f64, p: f64, overhead: f64) -> ExecutionSelector {
    let mut c = ExecutionSelector::new(2, ExecutionPolicyConfig::default()).unwrap();
    c.begin_calibration(key(), overhead).unwrap();
    for _ in 0..3 {
        c.record_pair(key(), s, p, true).unwrap();
    }
    c
}
#[test]
fn measured_gain_and_overhead_payback() {
    let mut c = trained(0.01, 0.004, 0.03);
    let d = c.decide(key(), 3, false);
    assert_eq!(d.mode, ExecutionMode::Parallel);
    assert!(!d.horizon_recovers_calibration);
    assert_eq!(d.estimated_cost_recovery_calls, Some(5.0));
    assert!(c.decide(key(), 20, false).horizon_recovers_calibration);
}
#[test]
fn slow_parallel_stays_serial() {
    let mut c = trained(0.001, 0.01, 0.0);
    assert_eq!(c.decide(key(), 1000, false).mode, ExecutionMode::Serial);
}
#[test]
fn exact_key_and_nesting_cannot_reuse_unsafe_evidence() {
    let mut c = trained(0.01, 0.004, 0.0);
    assert_eq!(
        c.decide(key(), 100, true).reason,
        ExecutionReason::NestedParallelism
    );
    let mut changed = key();
    changed.method_tag += 1;
    assert_eq!(
        c.decide(changed, 100, false).reason,
        ExecutionReason::UncalibratedWorkload
    );
    changed = key();
    changed.environment_epoch += 1;
    assert_eq!(c.decide(changed, 100, false).mode, ExecutionMode::Serial);
}
#[test]
fn invalid_or_wrong_output_clears_prior_evidence() {
    for bad in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
        let mut c = trained(0.01, 0.004, 0.0);
        c.begin_calibration(key(), 0.0).unwrap();
        assert!(c.record_pair(key(), 0.01, bad, true).is_err());
        assert_eq!(c.decide(key(), 100, false).mode, ExecutionMode::Serial);
    }
    let mut c = trained(0.01, 0.004, 0.0);
    c.begin_calibration(key(), 0.0).unwrap();
    assert!(c.record_pair(key(), 0.01, 0.004, false).is_err());
    assert_eq!(c.decide(key(), 100, false).mode, ExecutionMode::Serial);
}
#[test]
fn expired_calibration_falls_back_and_requests_probe() {
    let mut c = ExecutionSelector::new(
        2,
        ExecutionPolicyConfig {
            max_calibration_age: 2,
            ..Default::default()
        },
    )
    .unwrap();
    c.begin_calibration(key(), 0.0).unwrap();
    for _ in 0..3 {
        c.record_pair(key(), 0.01, 0.004, true).unwrap();
    }
    assert_eq!(c.decide(key(), 100, false).mode, ExecutionMode::Parallel);
    assert_eq!(c.decide(key(), 100, false).mode, ExecutionMode::Parallel);
    let d = c.decide(key(), 100, false);
    assert_eq!(d.reason, ExecutionReason::ExpiredCalibration);
    assert!(d.probe_due);
}
#[test]
fn actual_dispatch_preserves_order_and_no_implicit_replay() {
    let mut e = AdaptiveBatchExecutor::new(2, Default::default()).unwrap();
    let v: Vec<_> = (0..32).collect();
    e.selector.begin_calibration(key(), 0.0).unwrap();
    for _ in 0..3 {
        e.selector.record_pair(key(), 0.01, 0.004, true).unwrap();
    }
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let (out, d) = e
        .map_ordered(key(), 100, false, &v, |x| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(x * x)
        })
        .unwrap();
    assert_eq!(d.mode, ExecutionMode::Parallel);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 32);
    assert_eq!(out, v.iter().map(|x| x * x).collect::<Vec<_>>());
}
#[test]
fn internal_nested_wrapper_is_forced_serial() {
    let mut outer = AdaptiveBatchExecutor::new(2, Default::default()).unwrap();
    outer.selector.begin_calibration(key(), 0.0).unwrap();
    for _ in 0..3 {
        outer
            .selector
            .record_pair(key(), 0.01, 0.004, true)
            .unwrap();
    }
    let (_, decision) = outer
        .map_ordered(key(), 100, false, &(0..32).collect::<Vec<_>>(), |_| {
            let mut inner = AdaptiveBatchExecutor::new(2, Default::default())?;
            inner.selector.begin_calibration(key(), 0.0).unwrap();
            for _ in 0..3 {
                inner
                    .selector
                    .record_pair(key(), 0.01, 0.004, true)
                    .unwrap();
            }
            let (_, d) =
                inner.map_ordered(key(), 100, false, &(0..32).collect::<Vec<_>>(), |x| {
                    Ok(x * x)
                })?;
            assert_eq!(d.reason, ExecutionReason::NestedParallelism);
            Ok(())
        })
        .unwrap();
    assert_eq!(decision.mode, ExecutionMode::Parallel);
}
#[test]
fn real_calibration_and_shape_validation() {
    let mut e = AdaptiveBatchExecutor::new(2, Default::default()).unwrap();
    let v: Vec<_> = (0..32).collect();
    assert!(
        e.calibrate_replay_safe(key(), &v, |x| Ok(x * x), |a, b| a == b)
            .is_ok()
    );
    assert_eq!(e.selector.telemetry().calibration_pairs, 3);
    assert!(
        e.map_ordered(key(), 100, false, &v[..31], |x| Ok(x * x))
            .is_err()
    );
}

#[test]
fn single_item_calibration_never_replays_or_builds_pool() {
    let mut e = AdaptiveBatchExecutor::new(2, Default::default()).unwrap();
    let mut k = key();
    k.batch_len = 1;
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let elapsed = e
        .calibrate_replay_safe(
            k,
            &[3],
            |x| {
                calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(x * x)
            },
            |a, b| a == b,
        )
        .unwrap();
    assert_eq!(elapsed, 0.0);
    assert_eq!(e.startup_seconds, 0.0);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    let (result, d) = e.map_ordered(k, 20, false, &[3], |x| Ok(x * x)).unwrap();
    assert_eq!(result, vec![9]);
    assert_eq!(d.reason, ExecutionReason::SingleWorker);
    assert_eq!(e.startup_seconds, 0.0);
}
