//! Native tests for the opt-in, source-bound single-stage value-only bridge.
//! Run: cargo test -p vigilode-coupled --locked --offline --test n01_df_value_stage_contract
use rodas5p_core::N01MassActionOwner;
use vigilode_coupled::n01_derivative_free::N01ValueStage;
use vigilode_coupled::rhs_homotopy::{RhsHomotopyConfig, RhsOnlyMap};

fn model() -> N01MassActionOwner {
    N01MassActionOwner::new(
        [0.5, 1.0, 0.5],
        [35.0 / 64.0, 27.0 / 128.0, 31.0 / 128.0],
        [1.0; 3],
        0.25,
        [1.0; 3],
        [1.0; 3],
        0.0,
    )
    .unwrap()
}

#[test]
fn exact_manufactured_root_certifies_both_gates() {
    let source = model();
    let trial = source.freeze();
    let map = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let mut out = [7.0; 3];
    let report = map.try_commit(0.25, 1.0e-12, 1.0e-12, &mut out).unwrap();
    assert!(report.accepted, "{report:?}");
    assert!((out[0] - 0.5).abs() < 1.0e-15);
    assert_eq!(out[1], 0.25);
    assert!((out[2] - 0.25).abs() < 1.0e-15);
}

#[test]
fn native_homotopy_uses_read_only_source_and_commits_only_after_certificate() {
    let source = model();
    let trial = source.freeze();
    let map = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let mut out = [43.0; 3];
    let report = map.solve(&RhsHomotopyConfig::default(), 1.0e-8, 1.0e-8, &mut out);
    assert!(report.admitted, "homotopy did not admit a certified root");
    assert!(report.homotopy.work.map_attempted > 0);
    assert_eq!(source.model_epoch(), 0);
    assert!((out[1] - 0.25).abs() < 1.0e-8);
}

#[test]
fn deliberately_wrong_candidate_never_mutates_output() {
    let source = model();
    let trial = source.freeze();
    let map = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let mut out = [11.0, 12.0, 13.0];
    let report = map.try_commit(0.6, 1.0e-12, 1.0e-12, &mut out).unwrap();
    assert!(!report.accepted);
    assert_eq!(out, [11.0, 12.0, 13.0]);
}

#[test]
fn stale_trial_refuses_after_explicit_owner_state_change() {
    let mut source = model();
    let prior = source.freeze();
    source.advance(0.125, [0.5, 0.25, 0.25]).unwrap();
    assert!(N01ValueStage::new(&source, &prior, 0.25).is_err());
}

#[test]
fn rhs_map_evaluation_never_automatically_advances_epoch() {
    let source = model();
    let trial = source.freeze();
    let map = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let mut buffer = [f64::NAN];
    map.evaluate(&[0.2], &mut buffer).unwrap();
    assert!(buffer[0].is_finite());
    assert_eq!(source.model_epoch(), 0);
}

#[test]
fn budget_exhaustion_preserves_output_and_failed_work() {
    let source = model();
    let trial = source.freeze();
    let map = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let cfg = RhsHomotopyConfig {
        max_map_evaluations: 1,
        ..RhsHomotopyConfig::default()
    };
    let mut out = [101.0; 3];
    let report = map.solve(&cfg, 1.0e-12, 1.0e-12, &mut out);
    assert!(!report.admitted);
    assert_eq!(out, [101.0; 3]);
    assert_eq!(report.homotopy.work.map_attempted, 1);
    assert!(report.homotopy.failure.is_some());
}
