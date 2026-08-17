use std::sync::Arc;

use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem,
    OutputSchedule, ParallelExecution, integrate_pexprb54s4_fused_adaptive_observed,
    integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry,
    pexprb54s4_fused_step, pexprb54s4_fused_step_with_tolerance_scaled_telemetry,
};

fn square_problem() -> OdeProblem {
    OdeProblem::new(
        "square-tolerance-scaled-early-defect",
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

fn adaptive_config() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1e-10,
        rtol: 1e-8,
        initial_step: 0.1,
        min_step: 1e-14,
        max_step: 0.25,
        max_attempts: 10_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 4.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Pi,
    }
}

fn assert_same_nested_bits(left: &[Vec<f64>], right: &[Vec<f64>]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_same_bits(a, b);
    }
}

#[test]
fn tolerance_scaled_score_matches_sealed_stage_wrms_formula_and_is_numerically_neutral() {
    let problem = square_problem();
    let execution = ParallelExecution::sequential();
    let h = 0.05_f64;
    let y0 = [1.0_f64];
    let atol = 1e-8_f64;
    let rtol = 1e-6_f64;

    let disabled = pexprb54s4_fused_step(&problem, 0.0, &y0, h, phi_config(), &execution).unwrap();
    let scaled = pexprb54s4_fused_step_with_tolerance_scaled_telemetry(
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

    assert_same_bits(&disabled.y_new, &scaled.y_new);
    assert_same_bits(
        disabled.y_embedded.as_ref().unwrap(),
        scaled.y_embedded.as_ref().unwrap(),
    );
    assert_same_bits(
        disabled.error_estimate.as_ref().unwrap(),
        scaled.error_estimate.as_ref().unwrap(),
    );
    assert_eq!(disabled.fused_phi_reports, scaled.fused_phi_reports);
    assert_eq!(disabled.work, scaled.work);

    let telemetry = scaled.early_flow_defect.unwrap();
    let z = 0.25 * h * 2.0;
    let phi1 = z.exp_m1() / z;
    let u2 = 1.0 + h * 0.25 * phi1;
    let d2 = (u2 - 1.0).powi(2);
    let scale = atol + rtol * y0[0].abs().max(u2.abs());
    let expected = h.abs() * d2.abs() / scale;
    let observed = telemetry.tolerance_scaled_defect_wrms.unwrap();
    let relative = (observed - expected).abs() / expected.abs().max(1e-300);
    assert!(
        relative <= 5e-13,
        "observed={observed:e} expected={expected:e} relative={relative:e}"
    );
    assert_eq!(telemetry.tolerance_scale_atol, Some(atol));
    assert_eq!(telemetry.tolerance_scale_rtol, Some(rtol));
    assert_eq!(telemetry.tolerance_scaled_nonfinite, Some(false));
    assert_eq!(telemetry.diagnostic_work.wrms_norm_evaluations, 1);
    assert_eq!(telemetry.diagnostic_work.component_scale_evaluations, 1);
    assert_eq!(telemetry.diagnostic_work.added_rhs_calls, 0);
    assert_eq!(telemetry.diagnostic_work.added_jvp_calls, 0);
    assert_eq!(telemetry.diagnostic_work.added_phi_actions, 0);
}

#[test]
fn tolerance_scaled_score_breaks_the_raw_cross_tolerance_degeneracy_directionally() {
    let problem = square_problem();
    let execution = ParallelExecution::sequential();
    let h = 0.0625_f64;
    let mut scores = Vec::new();
    let mut raw = Vec::new();
    for rtol in [1e-4_f64, 1e-6_f64, 1e-8_f64] {
        let report = pexprb54s4_fused_step_with_tolerance_scaled_telemetry(
            &problem,
            0.0,
            &[1.0],
            h,
            phi_config(),
            &execution,
            1,
            0.01 * rtol,
            rtol,
        )
        .unwrap();
        let telemetry = report.early_flow_defect.unwrap();
        raw.push(telemetry.normalized_defect.unwrap());
        scores.push(telemetry.tolerance_scaled_defect_wrms.unwrap());
    }
    assert_eq!(raw[0].to_bits(), raw[1].to_bits());
    assert_eq!(raw[1].to_bits(), raw[2].to_bits());
    assert!(
        scores[0] < scores[1] && scores[1] < scores[2],
        "scores={scores:?}"
    );
}

#[test]
fn adaptive_tolerance_scaled_telemetry_preserves_controller_outputs_and_existing_work() {
    let problem = square_problem();
    let adaptive = adaptive_config();
    let output = OutputSchedule::new(vec![0.0, 0.125, 0.25]).unwrap();
    let execution = ParallelExecution::sequential();
    let disabled = integrate_pexprb54s4_fused_adaptive_observed(
        &problem,
        (0.0, 0.25),
        &[1.0],
        &adaptive,
        &output,
        phi_config(),
        &execution,
    )
    .unwrap();
    let scaled = integrate_pexprb54s4_fused_adaptive_observed_with_tolerance_scaled_telemetry(
        &problem,
        (0.0, 0.25),
        &[1.0],
        &adaptive,
        &output,
        phi_config(),
        &execution,
        1,
    )
    .unwrap();

    assert_same_bits(&disabled.observed.t, &scaled.observed.t);
    assert_same_nested_bits(&disabled.observed.y, &scaled.observed.y);
    assert_eq!(disabled.observed.counters, scaled.observed.counters);
    assert_eq!(disabled.observed.success, scaled.observed.success);
    assert_eq!(disabled.diagnostics.attempts, scaled.diagnostics.attempts);
    assert_same_bits(
        &disabled.diagnostics.accepted_step_sizes,
        &scaled.diagnostics.accepted_step_sizes,
    );
    assert_same_bits(
        &disabled.diagnostics.rejected_step_sizes,
        &scaled.diagnostics.rejected_step_sizes,
    );
    assert_same_bits(
        &disabled.diagnostics.total_error_norms,
        &scaled.diagnostics.total_error_norms,
    );
    assert_eq!(
        scaled.diagnostics.early_flow_defect_attempts.len(),
        scaled.diagnostics.attempts
    );
    assert!(
        scaled
            .diagnostics
            .early_flow_defect_attempts
            .iter()
            .all(|row| {
                row.telemetry.as_ref().is_some_and(|telemetry| {
                    telemetry.tolerance_scaled_defect_wrms.is_some()
                        && telemetry.tolerance_scale_atol == Some(adaptive.atol)
                        && telemetry.tolerance_scale_rtol == Some(adaptive.rtol)
                        && telemetry.diagnostic_work.added_rhs_calls == 0
                        && telemetry.diagnostic_work.added_jvp_calls == 0
                        && telemetry.diagnostic_work.added_phi_actions == 0
                }) && row.trial_work.is_some()
            })
    );
}
