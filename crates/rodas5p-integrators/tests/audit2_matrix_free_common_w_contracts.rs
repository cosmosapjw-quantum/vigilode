#![cfg(feature = "audit2-research")]

use std::sync::Arc;

use rodas5p_core::{CoreResult, DenseMatrix, LinearSolverConfig, WorkCounters, safe_l2};
use rodas5p_integrators::audit2_research::{
    Audit2CorrectionBackend, Audit2FailurePhase, Audit2MatrixFreeLinearConfig,
    Audit2ResearchConfig, compare_audit2_research_corrections,
    run_audit2_matrix_free_common_w_reuse, run_audit2_research_correction,
};
use rodas5p_integrators::{
    OdeProblem, build_step_context, build_step_context_matrix_free, manufactured_vector_problem,
    sequential_stages,
};

fn linear_config() -> Audit2MatrixFreeLinearConfig {
    Audit2MatrixFreeLinearConfig {
        restart: 24,
        max_arnoldi: 128,
        recycle_dim: 6,
        rank_tol: 1.0e-12,
        rtol: 1.0e-12,
        atol: 1.0e-14,
        max_completed_rows: None,
    }
}

fn linear_config_for_dimension(dimension: usize) -> Audit2MatrixFreeLinearConfig {
    let restart = 24.min(dimension.max(2));
    let recycle_dim = 6.min(restart.saturating_sub(1)).max(1);
    Audit2MatrixFreeLinearConfig {
        restart,
        max_arnoldi: 128.max(restart),
        recycle_dim,
        ..linear_config()
    }
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

fn flatten(rows: &[Vec<f64>]) -> Vec<f64> {
    rows.iter().flatten().copied().collect()
}

fn nonidentity_nonnormal_problem() -> (OdeProblem, Vec<f64>) {
    let n = 4;
    let jacobian = DenseMatrix::from_rows(&[
        &[-40.0, 2400.0, 0.0, 0.0],
        &[0.0, -80.0, 1200.0, 0.0],
        &[0.0, 0.0, -160.0, 600.0],
        &[0.0, 0.0, 0.0, -320.0],
    ])
    .unwrap();
    let mass = DenseMatrix::from_rows(&[
        &[1.0, 0.0, 0.0, 0.0],
        &[0.0, 1.5, 0.0, 0.0],
        &[0.0, 0.0, 0.9, 0.0],
        &[0.0, 0.0, 0.0, 2.0],
    ])
    .unwrap();
    let rhs_matrix = jacobian.clone();
    let rhs = Arc::new(move |_: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        let image = rhs_matrix.matvec(y)?;
        for (index, (target, value)) in out.iter_mut().zip(image).enumerate() {
            *target = value + 0.1 * (index + 1) as f64;
        }
        Ok(())
    });
    let matrix_for_jacobian = jacobian.clone();
    let jacobian_provider = Arc::new(move |_: f64, _: &[f64]| Ok(matrix_for_jacobian.clone()));
    let matrix_for_jvp = jacobian.clone();
    let jvp = Arc::new(
        move |_: f64, _: &[f64], input: &[f64], output: &mut [f64]| -> CoreResult<()> {
            let image = matrix_for_jvp.matvec(input)?;
            output.copy_from_slice(&image);
            Ok(())
        },
    );
    let problem = OdeProblem::new(
        "audit2-matrix-free-nonidentity-nonnormal",
        n,
        rhs,
        None,
        Some(jacobian_provider),
        Some(jvp),
        None,
        true,
        Some(mass),
        None,
    )
    .unwrap();
    (problem, vec![0.2, -0.1, 0.05, 0.3])
}

#[test]
fn matrix_free_common_w_requires_explicit_config_and_strict_matrix_free_context() {
    let (problem, y0) = manufactured_vector_problem(4, 50.0, 5.0, 0.1, 0.0).unwrap();
    let (explicit, stages) = perturbed_trial_stages(&problem, &y0, 0.01, 1.0e-5);
    let mut setup = WorkCounters::default();
    let matrix_free =
        build_step_context_matrix_free(&problem, 0.0, &y0, 0.01, &mut setup).unwrap();

    let missing = run_audit2_matrix_free_common_w_reuse(&matrix_free, &stages, None);
    let missing_failure = missing.failed().unwrap();
    assert_eq!(missing_failure.phase, Audit2FailurePhase::CommonWSetup);
    assert_eq!(missing_failure.work.common_w_setup_attempts, 0);

    let explicit_context =
        run_audit2_matrix_free_common_w_reuse(&explicit, &stages, Some(linear_config()));
    let explicit_failure = explicit_context.failed().unwrap();
    assert_eq!(explicit_failure.phase, Audit2FailurePhase::CommonWSetup);
    assert_eq!(explicit_failure.work.common_w_setup_attempts, 0);
}

fn run_case(case_id: &str, problem: &OdeProblem, y0: &[f64], h: f64) -> serde_json::Value {
    let (explicit_context, stages) = perturbed_trial_stages(problem, y0, h, 1.0e-5);
    let explicit = run_audit2_research_correction(
        &explicit_context,
        &stages,
        Audit2ResearchConfig {
            backend: Audit2CorrectionBackend::CommonWBlockForward,
        },
    );
    let explicit_success = explicit.completed().expect("explicit common-W arm must complete");
    let comparison = match compare_audit2_research_corrections(&explicit_context, &stages) {
        rodas5p_integrators::audit2_research::Audit2ComparisonOutcome::Completed(report) => report,
        failure => panic!("explicit comparison failed: {failure:?}"),
    };
    let condition = comparison
        .target_condition_f
        .expect("finite explicit target condition estimate required");

    let mut setup = WorkCounters::default();
    let matrix_free =
        build_step_context_matrix_free(problem, 0.0, y0, h, &mut setup).unwrap();
    assert!(matrix_free.shifted.explicit().is_none());
    let candidate = run_audit2_matrix_free_common_w_reuse(
        &matrix_free,
        &stages,
        Some(linear_config_for_dimension(problem.dimension)),
    );
    let success = candidate.completed().expect("matrix-free arm must complete");
    assert_eq!(success.correction.len(), 8);
    assert_eq!(success.solve_reports.len(), 8);
    assert_eq!(success.work.common_w_setup_attempts, 1);
    assert_eq!(success.work.common_w_setup_completed, 1);
    assert_eq!(success.work.solve_attempts, 8);
    assert_eq!(success.work.solve_completed, 8);
    assert_eq!(success.work.factorization_attempts, 0);
    assert_eq!(success.work.factorization_completed, 0);
    assert_eq!(success.work.counters.direct_factorizations, 0);
    assert_eq!(success.work.counters.direct_solve_calls, 0);
    assert_eq!(success.work.counters.recycle_cross_operator_refreshes, 0);
    assert!(success.work.counters.jvp_vectors > 0);
    assert!(success.solve_reports.iter().all(|report| report.converged));
    assert!(
        success.solve_reports.iter().skip(1).any(|report| report.recycle_reused),
        "at least one later row must consume same-operator recycle data"
    );
    assert!(success.work.counters.recycle_same_operator_uses > 0);

    let reference = flatten(&explicit_success.correction);
    let observed = flatten(&success.correction);
    let difference = safe_l2(
        &observed
            .iter()
            .zip(&reference)
            .map(|(candidate, oracle)| candidate - oracle)
            .collect::<Vec<_>>(),
    );
    let reference_norm = safe_l2(&reference);
    let relative = if reference_norm == 0.0 {
        difference
    } else {
        difference / reference_norm
    };
    let fixed_bound = 8192.0 * f64::EPSILON * condition.max(1.0);
    assert!(
        relative <= fixed_bound,
        "case={case_id}, relative={relative:e}, fixed_bound={fixed_bound:e}, condition={condition:e}"
    );
    let normalized_linear_residual =
        success.linear_residual_l2 / success.initial_residual_l2.max(f64::MIN_POSITIVE);
    assert!(
        normalized_linear_residual <= 1.0e-9,
        "case={case_id}, normalized residual={normalized_linear_residual:e}"
    );

    serde_json::json!({
        "case_id": case_id,
        "dimension": problem.dimension,
        "h": h,
        "condition_f": condition,
        "relative_correction_difference": relative,
        "fixed_condition_aware_bound": fixed_bound,
        "normalized_linear_residual": normalized_linear_residual,
        "recycle_same_operator_uses": success.work.counters.recycle_same_operator_uses,
        "recycle_cross_operator_refreshes": success.work.counters.recycle_cross_operator_refreshes,
        "recycle_updates": success.work.counters.recycle_updates,
        "linear_iterations": success.work.counters.linear_iterations,
        "linear_matvecs": success.work.counters.linear_matvecs,
        "jvp_vectors": success.work.counters.jvp_vectors,
        "direct_factorizations": success.work.counters.direct_factorizations,
        "direct_solve_calls": success.work.counters.direct_solve_calls
    })
}

#[test]
fn matrix_free_common_w_reuses_gcrodr_state_and_matches_explicit_arm_on_13_cases() {
    let mut rows = Vec::new();
    for n in [4, 8, 16] {
        for h in [0.001, 0.01, 0.05, 0.1] {
            let (problem, y0) = manufactured_vector_problem(n, 50.0, 5.0, 0.1, 0.0).unwrap();
            rows.push(run_case(&format!("identity-n{n}-h{h}"), &problem, &y0, h));
        }
    }
    let (problem, y0) = nonidentity_nonnormal_problem();
    rows.push(run_case("nonidentity-mass-strong-nonnormal", &problem, &y0, 0.01));
    assert_eq!(rows.len(), 13);
    println!(
        "AUDIT2_MATRIX_FREE_COMMON_W_ROWS {}",
        serde_json::to_string(&rows).unwrap()
    );
}

#[test]
fn matrix_free_common_w_bounded_stop_retains_completed_rows_reports_and_work() {
    let (problem, y0) = manufactured_vector_problem(8, 50.0, 5.0, 0.1, 0.0).unwrap();
    let (_, stages) = perturbed_trial_stages(&problem, &y0, 0.01, 1.0e-5);
    let mut setup = WorkCounters::default();
    let matrix_free =
        build_step_context_matrix_free(&problem, 0.0, &y0, 0.01, &mut setup).unwrap();
    let mut config = linear_config();
    config.max_completed_rows = Some(3);
    let outcome =
        run_audit2_matrix_free_common_w_reuse(&matrix_free, &stages, Some(config));
    let failure = outcome.failed().expect("bounded stop must fail closed");
    assert_eq!(failure.phase, Audit2FailurePhase::Solve);
    assert_eq!(failure.partial_correction.len(), 3);
    assert_eq!(failure.partial_solve_reports.len(), 3);
    assert_eq!(failure.work.common_w_setup_attempts, 1);
    assert_eq!(failure.work.common_w_setup_completed, 1);
    assert_eq!(failure.work.solve_attempts, 3);
    assert_eq!(failure.work.solve_completed, 3);
    assert!(failure.work.counters.jvp_vectors > 0);
    assert_eq!(failure.work.counters.direct_factorizations, 0);
    assert_eq!(failure.work.counters.direct_solve_calls, 0);
}
