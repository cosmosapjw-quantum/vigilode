//! Same-step interpolant audit (audit F-007, part C).
//!
//! For every interior output time of a dense run, the audit compares the
//! interpolant with one hard-stop step restarted from the accepted state that
//! produced it. Both approximations start from the same state, so their
//! difference is bounded by the interpolant defect plus the sub-step's local
//! error; it needs no reference and stays within a few tolerance units for a
//! solver whose local control holds.

use rodas5p_core::{LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSamplingPlan, OutputSchedule,
    integrate_sequential_matrix_free_adaptive_dense_fixed_inner_observed,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit,
    integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit_and_evaluator,
    prothero_robinson_problem, rodas5p_dense_output,
};

/// Delta limit of the two-arm protocol, in tolerance units.
const DELTA_LIMIT: f64 = 2.0;

fn setup(
    lambda: f64,
    rtol: f64,
) -> (
    rodas5p_integrators::OdeProblem,
    Vec<f64>,
    LinearSolverConfig,
    AdaptiveStepConfig,
    OutputSamplingPlan,
) {
    let (problem, y0) = prothero_robinson_problem(lambda, 0.0, 0.0);
    let matrix_free = problem.jvp_only_clone().unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: rtol * 1.0e-2,
        rtol,
        initial_step: 0.01,
        min_step: 1.0e-12,
        max_step: 1.0,
        max_attempts: 2048,
        ..AdaptiveStepConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        ..LinearSolverConfig::default()
    };
    let times = (0..=100).map(|index| index as f64 * 0.01).collect();
    let sampling = OutputSamplingPlan::dense(OutputSchedule::new(times).unwrap());
    (matrix_free, y0, linear, adaptive, sampling)
}

#[test]
fn the_audit_leaves_the_dense_run_bit_for_bit_unchanged() {
    let (problem, y0, linear, adaptive, sampling) = setup(-50.0, 1.0e-6);
    let plain = integrate_sequential_matrix_free_adaptive_dense_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &linear,
        &adaptive,
        &sampling,
    )
    .unwrap();
    let (audited, audit) = integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit(
        &problem,
        (0.0, 1.0),
        &y0,
        &linear,
        &adaptive,
        &sampling,
    )
    .unwrap();
    assert!(plain.observed.success);
    assert_eq!(audited.observed.counters, plain.observed.counters);
    assert_eq!(
        audited.observed.internal_steps,
        plain.observed.internal_steps
    );
    assert_eq!(audited.observed.t.len(), plain.observed.t.len());
    for (a, b) in audited.observed.y.iter().zip(&plain.observed.y) {
        assert_eq!(
            a.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
    }
    // The sub-steps are charged to the audit only.
    assert!(audit.counters.rhs_evaluations > 0);
    assert!(!audit.samples.is_empty());
    assert_eq!(audit.failed_substeps, 0);
}

#[test]
fn an_intact_interpolant_stays_within_the_delta_limit() {
    for (lambda, rtol) in [(-1.0, 1.0e-4), (-50.0, 1.0e-6), (-1.0e3, 1.0e-8)] {
        let (problem, y0, linear, adaptive, sampling) = setup(lambda, rtol);
        let (result, audit) =
            integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit(
                &problem,
                (0.0, 1.0),
                &y0,
                &linear,
                &adaptive,
                &sampling,
            )
            .unwrap();
        assert!(result.observed.success);
        let max_delta = audit.max_delta_wrms().expect("interior samples");
        assert!(
            max_delta <= DELTA_LIMIT,
            "lambda {lambda}, rtol {rtol}: max delta {max_delta:e}"
        );
        for sample in &audit.samples {
            assert!(sample.theta > 0.0 && sample.theta < 1.0);
            assert!(
                (sample.t - (sample.step_start + sample.theta * sample.step_size)).abs() <= 1.0e-12
            );
        }
    }
}

#[test]
fn a_corrupted_interpolant_is_caught_without_a_reference() {
    // A 1% error in the interpolated values is far above tolerance but
    // leaves the integration itself untouched.
    let (problem, y0, linear, adaptive, sampling) = setup(-50.0, 1.0e-6);
    let (result, audit) =
        integrate_sequential_matrix_free_adaptive_dense_with_interpolant_audit_and_evaluator(
            &problem,
            (0.0, 1.0),
            &y0,
            &linear,
            &adaptive,
            &sampling,
            |report, theta| {
                let value = rodas5p_dense_output(report, theta).unwrap();
                Ok(value.iter().map(|v| v * 1.01).collect())
            },
        )
        .unwrap();
    assert!(result.observed.success);
    let max_delta = audit.max_delta_wrms().unwrap();
    assert!(max_delta > DELTA_LIMIT, "max delta {max_delta:e}");
}

#[test]
fn the_fixed_inner_attribution_arm_runs_the_same_problem() {
    let (problem, y0, linear, adaptive, sampling) = setup(-50.0, 1.0e-6);
    let fixed = integrate_sequential_matrix_free_adaptive_dense_fixed_inner_observed(
        &problem,
        (0.0, 1.0),
        &y0,
        &linear,
        &adaptive,
        &sampling,
    )
    .unwrap();
    assert!(fixed.observed.success);
    assert_eq!(fixed.observed.t.len(), 101);
    // No stage of the attribution arm goes through the forcing rule.
    assert_eq!(fixed.observed.counters.forced_stage_solves, 0);
}
