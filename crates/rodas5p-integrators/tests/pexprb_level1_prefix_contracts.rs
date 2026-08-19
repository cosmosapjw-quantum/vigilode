use std::sync::Arc;

use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
    pexprb54s4_fused_step_resume_level1, pexprb54s4_fused_step_with_tolerance_scaled_telemetry,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
};

fn square_problem() -> OdeProblem {
    OdeProblem::new(
        "pexprb-level1-prefix-square",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn phi_config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 12,
        dimension_increment: 1,
        relative_tolerance: 1e-11,
        absolute_tolerance: 1e-14,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

fn assert_same_bits(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

#[test]
fn level1_prefix_is_resumable_without_repeating_work_and_matches_one_shot_step() {
    let problem = square_problem();
    let execution = ParallelExecution::sequential();
    let y0 = [1.0_f64];
    let h = 0.0625_f64;
    let atol = 1e-10_f64;
    let rtol = 1e-8_f64;

    let one_shot = pexprb54s4_fused_step_with_tolerance_scaled_telemetry(
        &problem,
        0.0,
        &y0,
        h,
        phi_config(),
        &execution,
        1,
        atol,
        rtol,
    )
    .unwrap();

    let prefix = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
        &problem,
        0.0,
        &y0,
        h,
        phi_config(),
        1,
        atol,
        rtol,
    )
    .unwrap();
    let prefix_report = prefix.report().clone();

    assert_eq!(prefix_report.method, "pexprb54s4-fused-level1");
    assert_eq!(prefix_report.logical_critical_depth, 1);
    assert_eq!(prefix_report.fused_phi_reports.len(), 1);
    assert!(prefix_report.early_flow_defect.is_some());
    assert!(prefix_report.work.rhs_evaluations > 0);
    assert!(prefix_report.work.jvp_vectors > 0);
    assert!(prefix_report.work.phi_actions > 0);
    assert!(prefix_report.work.rhs_evaluations < one_shot.work.rhs_evaluations);
    assert!(prefix_report.work.phi_actions < one_shot.work.phi_actions);

    let resumed = pexprb54s4_fused_step_resume_level1(prefix, &execution).unwrap();

    assert_same_bits(&one_shot.y_new, &resumed.y_new);
    assert_same_bits(
        one_shot.y_embedded.as_ref().unwrap(),
        resumed.y_embedded.as_ref().unwrap(),
    );
    assert_same_bits(
        one_shot.error_estimate.as_ref().unwrap(),
        resumed.error_estimate.as_ref().unwrap(),
    );
    assert_eq!(one_shot.fused_phi_reports, resumed.fused_phi_reports);
    assert_eq!(one_shot.early_flow_defect, resumed.early_flow_defect);
    assert_eq!(one_shot.work, resumed.work);
    assert_eq!(
        resumed.fused_phi_reports[0],
        prefix_report.fused_phi_reports[0]
    );
}
