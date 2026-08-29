#![cfg(feature = "audit2-research")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::audit2_research::{
    AUDIT2_STRUCTURE_PROJECTION_TOLERANCE, Audit2ComparisonOutcome, Audit2CorrectionBackend,
    Audit2CorrectionComparison, Audit2CorrectionOutcome, Audit2FailurePhase, Audit2ResearchConfig,
    compare_audit2_research_corrections, run_audit2_research_correction,
};
use rodas5p_integrators::{
    OdeProblem, build_step_context, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, sequential_stages,
};

type Jvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

fn scalar_problem(
    name: &str,
    mass: Option<f64>,
    jacobian_value: f64,
    rhs_value: f64,
    jvp: Option<Jvp>,
) -> OdeProblem {
    let jacobian = Arc::new(move |_: f64, _: &[f64]| DenseMatrix::from_rows(&[&[jacobian_value]]));
    OdeProblem::new(
        name,
        1,
        Arc::new(move |_, _, out| {
            out[0] = rhs_value;
            Ok(())
        }),
        None,
        Some(jacobian),
        jvp,
        None,
        true,
        mass.map(|value| DenseMatrix::from_rows(&[&[value]]).unwrap()),
        None,
    )
    .unwrap()
}

fn exact_jvp(value: f64) -> Jvp {
    Arc::new(move |_, _, input, output| {
        output[0] = value * input[0];
        Ok(())
    })
}

fn perturbed_trial_stages<'a>(
    problem: &'a OdeProblem,
    y0: &[f64],
    h: f64,
    magnitude: f64,
) -> (rodas5p_integrators::StepContext<'a>, Vec<Vec<f64>>) {
    let mut setup = WorkCounters::default();
    let context = build_step_context(problem, 0.0, y0, h, &mut setup).unwrap();
    let mut stages = sequential_stages(&context, &LinearSolverConfig::default(), None, &mut setup)
        .unwrap()
        .stages;
    let n = problem.dimension;
    for (i, row) in stages.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value += magnitude * ((i * n + j + 1) as f64).sin();
        }
    }
    (context, stages)
}

fn completed_comparison(outcome: Audit2ComparisonOutcome) -> Audit2CorrectionComparison {
    match outcome {
        Audit2ComparisonOutcome::Completed(report) => *report,
        Audit2ComparisonOutcome::Failed(failure) => {
            panic!("shared comparison preparation failed: {failure:?}")
        }
    }
}

fn assert_condition_aware_agreement(report: &Audit2CorrectionComparison) {
    assert_eq!(report.independent_validation_apply_attempts, 2);
    assert_eq!(report.independent_validation_apply_completed, 2);
    assert_eq!(report.independent_validation_counters.diagnostic_matvecs, 2);
    let condition = report
        .target_condition_f
        .expect("finite full-target condition estimate required");
    assert!(condition.is_finite());
    let full_eta = report
        .full_target_backward_error
        .expect("finite oracle backward error required");
    let common_eta = report
        .common_w_backward_error
        .expect("finite common-W backward error required");
    assert!(full_eta <= 4096.0 * f64::EPSILON, "{full_eta:e}");
    assert!(common_eta <= 4096.0 * f64::EPSILON, "{common_eta:e}");
    if let Some(relative) = report.state_relative_difference {
        assert!(
            relative <= 8192.0 * f64::EPSILON * condition,
            "relative={relative:e}, condition={condition:e}"
        );
    }
}

fn assert_success_accounting(outcome: &Audit2CorrectionOutcome) {
    let success = outcome
        .completed()
        .expect("common-W correction must complete");
    let work = &success.work;
    assert_eq!(work.common_w_setup_attempts, 1);
    assert_eq!(work.common_w_setup_completed, 1);
    assert_eq!(work.factorization_attempts, 1);
    assert_eq!(work.factorization_completed, 1);
    assert_eq!(work.solve_attempts, 8);
    assert_eq!(work.solve_completed, 8);
    assert_eq!(work.correction_jvp_attempts, 14);
    assert_eq!(work.correction_jvp_completed, 14);
    assert_eq!(work.linear_diagnostic_apply_attempts, 1);
    assert_eq!(work.linear_diagnostic_apply_completed, 1);
    assert_eq!(work.diagnostic_shifted_apply_attempts, 8);
    assert_eq!(work.diagnostic_shifted_apply_completed, 8);
    assert_eq!(work.diagnostic_jvp_attempts, 14);
    assert_eq!(work.diagnostic_jvp_completed, 14);
    assert_eq!(work.nonlinear_residual_after_attempts, 1);
    assert_eq!(work.nonlinear_residual_after_completed, 1);
    assert_eq!(work.counters.direct_factorizations, 1);
    assert_eq!(work.counters.direct_solve_calls, 8);
    assert_eq!(work.counters.diagnostic_matvecs, 8);
    assert_eq!(work.counters.jvp_vectors, 52);
}

#[test]
fn full_target_is_default_and_common_w_requires_explicit_opt_in() {
    assert_eq!(
        Audit2ResearchConfig::default().backend,
        Audit2CorrectionBackend::FullTargetOracle
    );
    let (problem, y0) = manufactured_vector_problem(4, 50.0, 5.0, 0.1, 0.0).unwrap();
    let (context, stages) = perturbed_trial_stages(&problem, &y0, 0.01, 1e-5);
    let default =
        run_audit2_research_correction(&context, &stages, Audit2ResearchConfig::default());
    assert_eq!(
        default.completed().unwrap().backend,
        Audit2CorrectionBackend::FullTargetOracle
    );
    let explicit = run_audit2_research_correction(
        &context,
        &stages,
        Audit2ResearchConfig {
            backend: Audit2CorrectionBackend::CommonWBlockForward,
        },
    );
    assert_eq!(
        explicit.completed().unwrap().backend,
        Audit2CorrectionBackend::CommonWBlockForward
    );
}

#[test]
fn projected_target_matches_condition_aware_full_oracle() {
    for n in [4, 8, 16] {
        for h in [0.001, 0.01, 0.05, 0.1] {
            let (problem, y0) = manufactured_vector_problem(n, 50.0, 5.0, 0.1, 0.0).unwrap();
            let (context, stages) = perturbed_trial_stages(&problem, &y0, h, 1e-5);
            let report =
                completed_comparison(compare_audit2_research_corrections(&context, &stages));
            assert!(report.matching_trial_stage_states);
            assert!(report.projection.result_independent_fixed_rule);
            assert!(report.projection.projected_structure_bit_exact);
            assert_eq!(
                report.projection.tolerance,
                AUDIT2_STRUCTURE_PROJECTION_TOLERANCE
            );
            assert!(report.projection.max_alpha_forbidden_abs <= report.projection.tolerance);
            assert!(report.projection.max_gamma_upper_abs <= report.projection.tolerance);
            assert!(report.projection.max_gamma_diagonal_error_abs <= report.projection.tolerance);
            assert!(
                report.projection.projected_alpha_entries
                    + report.projection.projected_gamma_entries
                    > 0
            );
            assert_condition_aware_agreement(&report);
            assert_success_accounting(&report.common_w);
            println!(
                "AUDIT2_PROJECTED_CORRECTION {}",
                serde_json::json!({
                    "n": n,
                    "h": h,
                    "condition_f": report.target_condition_f,
                    "oracle_backward_error": report.full_target_backward_error,
                    "common_w_backward_error": report.common_w_backward_error,
                    "state_absolute_difference_l2": report.state_absolute_difference_l2,
                    "state_relative_difference": report.state_relative_difference,
                    "projection": report.projection,
                    "common_w_work": report.common_w.completed().unwrap().work,
                    "production_activation": false
                })
            );
        }
    }
}

#[test]
fn nonsingular_nonidentity_mass_and_strong_nonnormality_are_supported() {
    let (problem, y0, mass, linear) =
        manufactured_mass_nonlinear_problem(1_000.0, 50.0, 20.0, 0.0).unwrap();
    let determinant = mass[(0, 0)] * mass[(1, 1)] - mass[(0, 1)] * mass[(1, 0)];
    assert!(determinant.abs() > 1.0);
    assert_ne!(mass, DenseMatrix::identity(2));
    assert!(linear[(0, 1)].abs() > 100.0 * linear[(1, 0)].abs());
    let (context, stages) = perturbed_trial_stages(&problem, &y0, 1e-4, 1e-7);
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    assert_condition_aware_agreement(&report);
    assert_success_accounting(&report.common_w);
    assert_eq!(
        report
            .common_w
            .completed()
            .unwrap()
            .work
            .counters
            .mass_matvecs,
        16
    );
    println!(
        "AUDIT2_MASS_NONNORMAL {}",
        serde_json::json!({
            "mass_determinant": determinant,
            "nonnormal_off_diagonal_ratio": linear[(0, 1)].abs()/linear[(1, 0)].abs(),
            "condition_f": report.target_condition_f,
            "common_w_backward_error": report.common_w_backward_error,
            "state_relative_difference": report.state_relative_difference,
            "nonlinear_residual_before": report.common_w.completed().unwrap().initial_residual_l2,
            "nonlinear_residual_after": report.common_w.completed().unwrap().nonlinear_residual_after_l2
        })
    );
}

#[test]
fn zero_rhs_uses_finite_absolute_criteria_without_zero_over_zero() {
    let problem = scalar_problem("audit2-zero", None, 0.0, 0.0, Some(exact_jvp(0.0)));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    let full = report.full_target.completed().unwrap();
    let common = report.common_w.completed().unwrap();
    assert_eq!(full.initial_residual_l2, 0.0);
    assert_eq!(common.correction_l2, 0.0);
    assert_eq!(common.linear_residual_l2, 0.0);
    assert_eq!(common.nonlinear_residual_after_l2, 0.0);
    assert_eq!(report.full_target_backward_error, Some(0.0));
    assert_eq!(report.common_w_backward_error, Some(0.0));
    assert_eq!(report.state_absolute_difference_l2, Some(0.0));
    assert_eq!(report.state_relative_difference, None);
    assert_success_accounting(&report.common_w);
}

#[test]
fn missing_jvp_fails_only_the_opt_in_arm_before_setup() {
    let problem = scalar_problem("audit2-missing-jvp", None, -1.0, 1.0, None);
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    assert!(report.full_target.completed().is_some());
    let failure = report.common_w.failed().expect("missing JVP must be typed");
    assert_eq!(failure.phase, Audit2FailurePhase::JvpAccess);
    assert_eq!(failure.work.common_w_setup_attempts, 0);
    assert_eq!(failure.work.factorization_attempts, 0);
    assert_eq!(failure.work.solve_attempts, 0);
}

#[test]
fn failed_jvp_preserves_attempt_and_partial_progress() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let observed = attempts.clone();
    let failing: Jvp = Arc::new(move |_, _, _, _| {
        observed.fetch_add(1, Ordering::SeqCst);
        Err(CoreError::LinearSolve("injected JVP failure".into()))
    });
    let problem = scalar_problem("audit2-failed-jvp", None, 0.0, 1.0, Some(failing));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    assert!(report.full_target.completed().is_some());
    let failure = report
        .common_w
        .failed()
        .expect("JVP failure must be retained");
    assert_eq!(failure.phase, Audit2FailurePhase::CorrectionJvp);
    assert_eq!(failure.partial_correction.len(), 1);
    assert_eq!(failure.work.solve_attempts, 1);
    assert_eq!(failure.work.solve_completed, 1);
    assert_eq!(failure.work.correction_jvp_attempts, 1);
    assert_eq!(failure.work.correction_jvp_completed, 0);
    assert_eq!(failure.work.counters.jvp_calls, 1);
    assert_eq!(failure.work.counters.jvp_vectors, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[test]
fn singular_and_overflowing_solves_are_typed_and_counted() {
    for (name, mass) in [("singular", 0.0), ("overflow", 1e-320)] {
        let problem = scalar_problem(name, Some(mass), 0.0, 1.0, Some(exact_jvp(0.0)));
        let context =
            build_step_context(&problem, 0.0, &[0.0], 1.0, &mut WorkCounters::default()).unwrap();
        let stages = vec![vec![0.0]; context.coeffs.stages()];
        let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
        let failure = report
            .common_w
            .failed()
            .expect("unrepresentable solve must fail");
        assert_eq!(failure.phase, Audit2FailurePhase::Solve, "{name}");
        assert_eq!(failure.work.common_w_setup_attempts, 1, "{name}");
        assert_eq!(failure.work.common_w_setup_completed, 1, "{name}");
        assert_eq!(failure.work.factorization_attempts, 1, "{name}");
        assert_eq!(failure.work.factorization_completed, 1, "{name}");
        assert_eq!(failure.work.solve_attempts, 1, "{name}");
        assert_eq!(failure.work.solve_completed, 0, "{name}");
        assert!(failure.partial_correction.is_empty());
        assert!(
            failure.message.contains("LU"),
            "{name}: {}",
            failure.message
        );
    }
}

#[test]
fn overflow_after_a_completed_zero_row_retains_partial_solve_progress() {
    let zero = DenseMatrix::zeros(1, 1);
    let jacobian = zero.clone();
    let problem = OdeProblem::new(
        "audit2-late-overflow",
        1,
        Arc::new(|time, _, out| {
            out[0] = if time == 0.0 { 0.0 } else { f64::MAX };
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| Ok(jacobian.clone()))),
        Some(exact_jvp(0.0)),
        Some(Arc::new(|_, _, out| {
            out[0] = 0.0;
            Ok(())
        })),
        false,
        Some(DenseMatrix::from_rows(&[&[1e-200]]).unwrap()),
        None,
    )
    .unwrap();
    let context =
        build_step_context(&problem, 0.0, &[0.0], 1.0, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    let failure = report.common_w.failed().expect("later row must overflow");
    assert_eq!(failure.phase, Audit2FailurePhase::Solve);
    assert_eq!(failure.partial_correction, vec![vec![0.0]]);
    assert_eq!(failure.work.solve_attempts, 2);
    assert_eq!(failure.work.solve_completed, 1);
    assert_eq!(failure.work.correction_jvp_attempts, 2);
    assert_eq!(failure.work.correction_jvp_completed, 2);
}

#[test]
fn nonfinite_trial_input_fails_before_all_algorithmic_work() {
    let problem = scalar_problem("audit2-nonfinite", None, 0.0, 0.0, Some(exact_jvp(0.0)));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let mut stages = vec![vec![0.0]; context.coeffs.stages()];
    stages[0][0] = f64::NAN;
    let failure = match compare_audit2_research_corrections(&context, &stages) {
        Audit2ComparisonOutcome::Failed(failure) => failure,
        Audit2ComparisonOutcome::Completed(_) => panic!("NaN input was accepted"),
    };
    assert_eq!(failure.phase, Audit2FailurePhase::InputValidation);
    assert_eq!(failure.preparation_counters, WorkCounters::default());
}

#[test]
fn inconsistent_jvp_is_exposed_as_a_domain_counterexample_not_a_pass() {
    let problem = scalar_problem(
        "audit2-inconsistent-jvp",
        None,
        0.0,
        1.0,
        Some(exact_jvp(100.0)),
    );
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    let full = report.full_target.completed().unwrap();
    let common = report.common_w.completed().unwrap();
    assert!(
        report.state_absolute_difference_l2.unwrap() > 1e-6,
        "inconsistent JVP must not look oracle-equivalent"
    );
    assert!(
        report.common_w_backward_error.unwrap() > 1e-8,
        "independent full-target residual must expose the mismatch"
    );
    assert!(common.nonlinear_residual_after_l2 > full.nonlinear_residual_after_l2 + 1e-8);
    assert!(common.correction_l2.is_finite());
}

#[test]
fn malformed_shape_is_a_typed_preparation_failure() {
    let problem = scalar_problem("audit2-shape", None, 0.0, 0.0, Some(exact_jvp(0.0)));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let failure = run_audit2_research_correction(
        &context,
        &[],
        Audit2ResearchConfig {
            backend: Audit2CorrectionBackend::CommonWBlockForward,
        },
    )
    .failed()
    .cloned()
    .expect("shape failure must be retained");
    assert_eq!(failure.phase, Audit2FailurePhase::InputValidation);
    assert!(failure.projection.is_none());
    assert_eq!(failure.work, Default::default());
}
