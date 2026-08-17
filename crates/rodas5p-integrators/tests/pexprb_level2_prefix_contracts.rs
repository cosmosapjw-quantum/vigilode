use std::sync::Arc;

use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
    pexprb54s4_fused_step_resume_level2, pexprb54s4_fused_step_with_tolerance_scaled_telemetry,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
    pexprb54s4_level2_prefix_resume_level1,
};

fn square_problem() -> OdeProblem {
    OdeProblem::new(
        "pexprb-level2-prefix-square",
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
fn level2_prefix_is_resumable_without_repeating_level1_or_level2_work() {
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

    let level1 = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
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
    let level1_report = level1.report().clone();
    let level2 = pexprb54s4_level2_prefix_resume_level1(level1, &execution).unwrap();
    let level2_report = level2.report().clone();

    assert_eq!(level2_report.method, "pexprb54s4-fused-level2");
    assert_eq!(level2_report.logical_critical_depth, 2);
    assert_eq!(level2_report.level2_fused_phi_reports.len(), 2);
    assert_eq!(level2_report.level1_report, level1_report);
    assert_eq!(level2_report.cumulative_work, {
        let mut expected = level1_report.work;
        expected.accumulate(level2_report.level2_incremental_work);
        expected
    });
    let stage3 = level2_report.stage3_flow_defect.as_ref().unwrap();
    let stage4 = level2_report.stage4_flow_defect.as_ref().unwrap();
    assert_eq!(stage3.stage_fraction.to_bits(), (0.5_f64).to_bits());
    assert_eq!(stage4.stage_fraction.to_bits(), (0.9_f64).to_bits());
    assert!(stage3.tolerance_scaled_defect_wrms.is_some());
    assert!(stage4.tolerance_scaled_defect_wrms.is_some());
    assert!(level2_report.level2_incremental_work.rhs_evaluations > 0);
    assert!(level2_report.level2_incremental_work.jvp_vectors > 0);
    assert!(level2_report.level2_incremental_work.phi_actions > 0);

    let resumed = pexprb54s4_fused_step_resume_level2(level2, &execution).unwrap();
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
}
