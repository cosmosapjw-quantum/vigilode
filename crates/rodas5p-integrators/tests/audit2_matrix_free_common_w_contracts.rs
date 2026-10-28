#![cfg(feature = "audit2-research")]

use rodas5p_core::LinearSolverConfig;
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearOperator, LuFactorization, WorkCounters, inverse,
    safe_l2,
};
use rodas5p_integrators::audit2_research::{
    Audit2ComparisonOutcome, Audit2CorrectionBackend, Audit2CorrectionOutcome,
    Audit2ResearchConfig, compare_audit2_research_corrections, run_audit2_research_correction,
};
use rodas5p_integrators::{
    Audit2MatrixFreeBatchOutcome, Audit2MatrixFreeCommonWConfig, Audit2MatrixFreeCommonWSession,
    Audit2MatrixFreeCorrectionFailurePhase, Audit2MatrixFreeCorrectionOutcome, build_step_context,
    build_step_context_matrix_free, manufactured_vector_problem,
    run_audit2_matrix_free_common_w_correction, sequential_stages,
};

fn rhs_rows(dimension: usize) -> Vec<Vec<f64>> {
    (0..8)
        .map(|row| {
            (0..dimension)
                .map(|index| {
                    let x = (row + 1) as f64 * (index + 1) as f64;
                    (0.17 * x).sin() + 0.2 * (0.07 * x).cos()
                })
                .collect()
        })
        .collect()
}

fn completed(
    outcome: Audit2MatrixFreeBatchOutcome,
) -> rodas5p_integrators::Audit2MatrixFreeBatchSuccess {
    match outcome {
        Audit2MatrixFreeBatchOutcome::Completed(value) => *value,
        Audit2MatrixFreeBatchOutcome::Failed(failure) => {
            panic!("matrix-free batch unexpectedly failed: {failure:?}")
        }
    }
}

fn completed_matrix_free_correction(
    outcome: Audit2MatrixFreeCorrectionOutcome,
) -> rodas5p_integrators::Audit2MatrixFreeCorrectionSuccess {
    match outcome {
        Audit2MatrixFreeCorrectionOutcome::Completed(value) => *value,
        Audit2MatrixFreeCorrectionOutcome::Failed(failure) => {
            panic!("matrix-free correction unexpectedly failed: {failure:?}")
        }
    }
}

fn completed_explicit_correction(
    outcome: Audit2CorrectionOutcome,
) -> rodas5p_integrators::audit2_research::Audit2CorrectionSuccess {
    match outcome {
        Audit2CorrectionOutcome::Completed(value) => value,
        Audit2CorrectionOutcome::Failed(failure) => {
            panic!("explicit common-W correction unexpectedly failed: {failure:?}")
        }
    }
}

fn direct_oracle(matrix: &DenseMatrix, rhs: &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> {
    let factor = LuFactorization::new(matrix)?;
    rhs.iter().map(|row| factor.solve(row)).collect()
}

#[test]
fn actual_shifted_operator_reuses_one_matrix_free_session_across_two_batches() -> CoreResult<()> {
    // Fixed before outcome inspection: moderately stiff, strongly nonnormal,
    // eight-RHS common-W-shaped batches and the existing GMRES defaults below.
    let dimension = 48;
    let (problem, y0) = manufactured_vector_problem(dimension, 1.0e4, 3.0, 0.9, 0.0)?;
    let mut explicit_build = WorkCounters::default();
    let explicit = build_step_context(&problem, 0.0, &y0, 1.0e-4, &mut explicit_build)?;
    let matrix = explicit
        .shifted
        .explicit()
        .expect("separate small explicit oracle must expose W")
        .clone();

    let mut matrix_free_build = WorkCounters::default();
    let matrix_free =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-4, &mut matrix_free_build)?;
    assert!(matrix_free.shifted.explicit().is_none());
    assert_eq!(matrix_free_build.jacobian_builds, 0);

    let config = Audit2MatrixFreeCommonWConfig {
        restart: 24,
        max_arnoldi: 192,
        rtol: 1.0e-11,
        atol: 1.0e-13,
    };
    let rhs = rhs_rows(dimension);
    let oracle = direct_oracle(&matrix, &rhs)?;
    let mut session = Audit2MatrixFreeCommonWSession::new(&matrix_free, config)?;
    let first = completed(session.solve_rows(&rhs));

    assert_eq!(first.session.setup_attempts, 1);
    assert_eq!(first.session.setup_completed, 1);
    assert_eq!(first.session.identity_preconditioner_setups, 1);
    assert_eq!(first.session.workspace_initializations, 1);
    assert_eq!(first.session.batch_attempts, 1);
    assert_eq!(first.session.batch_completed, 1);
    assert_eq!(first.session.solve_attempts, 8);
    assert_eq!(first.session.solve_completed, 8);
    assert_eq!(first.solutions.len(), 8);
    assert_eq!(first.solve_reports.len(), 8);
    assert_eq!(first.session.counters.direct_factorizations, 0);
    assert_eq!(first.session.counters.direct_solve_calls, 0);
    assert_eq!(first.session.workspace_capacity_growth_after_first, 0);
    assert!(
        first
            .session
            .workspace_capacity_after_first_solve
            .unwrap_or(0)
            > 0
    );

    let matrix_norm = safe_l2(matrix.as_slice());
    let matrix_inverse = inverse(&matrix)?;
    let condition_f = matrix_norm * safe_l2(matrix_inverse.as_slice());
    let mut max_backward_error: f64 = 0.0;
    let mut max_relative_difference: f64 = 0.0;
    for ((candidate, exact), b) in first.solutions.iter().zip(&oracle).zip(&rhs) {
        let image = matrix.matvec(candidate)?;
        let residual = safe_l2(
            &image
                .iter()
                .zip(b)
                .map(|(left, right)| left - right)
                .collect::<Vec<_>>(),
        );
        let eta = residual / (matrix_norm * safe_l2(candidate) + safe_l2(b));
        let relative = safe_l2(
            &candidate
                .iter()
                .zip(exact)
                .map(|(left, right)| left - right)
                .collect::<Vec<_>>(),
        ) / safe_l2(exact).max(f64::MIN_POSITIVE);
        assert!(eta.is_finite());
        max_backward_error = max_backward_error.max(eta);
        max_relative_difference = max_relative_difference.max(relative);
        assert!(
            relative <= 32.0 * condition_f * eta.max(f64::EPSILON),
            "relative={relative:e}, eta={eta:e}, condition_f={condition_f:e}"
        );
    }

    let scaled_rhs: Vec<Vec<f64>> = rhs
        .iter()
        .map(|row| row.iter().map(|value| -0.5 * value).collect())
        .collect();
    let second = completed(session.solve_rows(&scaled_rhs));
    assert_eq!(second.session.setup_attempts, 1);
    assert_eq!(second.session.setup_completed, 1);
    assert_eq!(second.session.identity_preconditioner_setups, 1);
    assert_eq!(second.session.workspace_initializations, 1);
    assert_eq!(second.session.batch_attempts, 2);
    assert_eq!(second.session.batch_completed, 2);
    assert_eq!(second.session.solve_attempts, 16);
    assert_eq!(second.session.solve_completed, 16);
    assert_eq!(second.session.workspace_capacity_growth_after_first, 0);
    assert_eq!(
        first.session.workspace_capacity_after_first_solve,
        second.session.workspace_capacity_after_first_solve
    );
    assert_eq!(first.session.operator_token, second.session.operator_token);
    assert_eq!(second.session.counters.direct_factorizations, 0);
    assert_eq!(second.session.counters.direct_solve_calls, 0);
    assert_eq!(second.session.counters.linear_solves, 16);
    assert_eq!(second.session.counters.block_linear_solves, 2);
    assert_eq!(
        second.session.counters.jvp_calls,
        second
            .session
            .counters
            .linear_matvecs
            .saturating_add(second.session.counters.diagnostic_matvecs)
    );
    assert_eq!(
        second.session.counters.jvp_vectors,
        second.session.counters.jvp_calls
    );
    assert!(second.session.counters.jvp_calls > 0);
    assert_eq!(second.session.counters.recycle_projection_calls, 0);
    assert_eq!(second.session.counters.recycle_same_operator_uses, 0);
    println!(
        "AUDIT2_MATRIX_FREE_COMMON_W {}",
        serde_json::json!({
            "dimension": dimension,
            "rhs_per_batch": rhs.len(),
            "batches": second.session.batch_completed,
            "setup_attempts": second.session.setup_attempts,
            "setup_completed": second.session.setup_completed,
            "workspace_initializations": second.session.workspace_initializations,
            "identity_preconditioner_setups": second.session.identity_preconditioner_setups,
            "solve_attempts": second.session.solve_attempts,
            "solve_completed": second.session.solve_completed,
            "workspace_capacity_after_first_solve": second.session.workspace_capacity_after_first_solve,
            "workspace_capacity_current": second.session.workspace_capacity_current,
            "workspace_capacity_growth_after_first": second.session.workspace_capacity_growth_after_first,
            "condition_f": condition_f,
            "max_normalized_backward_error": max_backward_error,
            "max_direct_relative_difference": max_relative_difference,
            "counters": second.session.counters,
            "explicit_w_used_by_session": false,
            "krylov_basis_reuse_claimed": false,
            "production_activation": false
        })
    );
    Ok(())
}

#[test]
fn actual_block_forward_correction_matches_existing_explicit_common_w_reference() -> CoreResult<()>
{
    // Fixed before outcome inspection: one n=16,h=0.01 manufactured vector
    // coordinate, the inherited structural projection, and the derived
    // condition-aware forward-error bound below.
    let dimension = 16;
    let h = 0.01;
    let (problem, y0) = manufactured_vector_problem(dimension, 50.0, 5.0, 0.1, 0.0)?;
    let mut setup = WorkCounters::default();
    let explicit = build_step_context(&problem, 0.0, &y0, h, &mut setup)?;
    let mut stages =
        sequential_stages(&explicit, &LinearSolverConfig::default(), None, &mut setup)?.stages;
    for (stage, row) in stages.iter_mut().enumerate() {
        for (component, value) in row.iter_mut().enumerate() {
            *value += 1.0e-5 * ((stage * dimension + component + 1) as f64).sin();
        }
    }
    let explicit_reference = completed_explicit_correction(run_audit2_research_correction(
        &explicit,
        &stages,
        Audit2ResearchConfig {
            backend: Audit2CorrectionBackend::CommonWBlockForward,
        },
    ));
    let condition = match compare_audit2_research_corrections(&explicit, &stages) {
        Audit2ComparisonOutcome::Completed(value) => value
            .target_condition_f
            .expect("explicit target condition required"),
        Audit2ComparisonOutcome::Failed(value) => {
            panic!("explicit comparison failed: {value:?}")
        }
    };

    let matrix_free =
        build_step_context_matrix_free(&problem, 0.0, &y0, h, &mut WorkCounters::default())?;
    let config = Audit2MatrixFreeCommonWConfig::default();
    let candidate = completed_matrix_free_correction(run_audit2_matrix_free_common_w_correction(
        &matrix_free,
        &stages,
        config,
    ));

    assert!(candidate.projection.projected_structure_bit_exact);
    assert_eq!(candidate.solve_reports.len(), explicit.coeffs.stages());
    assert!(
        candidate
            .solve_reports
            .iter()
            .all(|report| report.converged)
    );
    let session = candidate.work.session.as_ref().unwrap();
    assert_eq!(session.setup_attempts, 1);
    assert_eq!(session.setup_completed, 1);
    assert_eq!(session.batch_attempts, 1);
    assert_eq!(session.batch_completed, 1);
    assert_eq!(session.solve_attempts, explicit.coeffs.stages() as u64);
    assert_eq!(session.solve_completed, explicit.coeffs.stages() as u64);
    assert_eq!(session.workspace_capacity_growth_after_first, 0);
    assert_eq!(session.counters.direct_factorizations, 0);
    assert_eq!(session.counters.direct_solve_calls, 0);
    assert_eq!(
        session.counters.jvp_calls,
        session
            .counters
            .linear_matvecs
            .saturating_add(session.counters.diagnostic_matvecs)
    );
    assert_eq!(session.counters.jvp_vectors, session.counters.jvp_calls);
    assert!(session.counters.jvp_calls > 0);
    assert_eq!(candidate.work.correction_jvp_attempts, 14);
    assert_eq!(candidate.work.correction_jvp_completed, 14);
    assert_eq!(candidate.work.diagnostic_shifted_apply_attempts, 8);
    assert_eq!(candidate.work.diagnostic_shifted_apply_completed, 8);
    assert_eq!(candidate.work.diagnostic_jvp_attempts, 14);
    assert_eq!(candidate.work.diagnostic_jvp_completed, 14);

    let candidate_flat: Vec<f64> = candidate.correction.iter().flatten().copied().collect();
    let reference_flat: Vec<f64> = explicit_reference
        .correction
        .iter()
        .flatten()
        .copied()
        .collect();
    let relative = safe_l2(
        &candidate_flat
            .iter()
            .zip(&reference_flat)
            .map(|(left, right)| left - right)
            .collect::<Vec<_>>(),
    ) / safe_l2(&reference_flat).max(f64::MIN_POSITIVE);
    let bound = 64.0 * condition * config.rtol.max(f64::EPSILON);
    assert!(
        relative <= bound,
        "relative={relative:e}, bound={bound:e}, condition={condition:e}"
    );
    let residual_bound = 64.0
        * ((explicit.coeffs.stages() as f64).sqrt() * config.atol
            + config.rtol * candidate.initial_residual_l2);
    assert!(
        candidate.linear_residual_l2 <= residual_bound,
        "linear residual={:e}, bound={residual_bound:e}",
        candidate.linear_residual_l2
    );
    println!(
        "AUDIT2_MATRIX_FREE_CORRECTION {}",
        serde_json::json!({
            "dimension": dimension,
            "h": h,
            "stages": explicit.coeffs.stages(),
            "condition_f": condition,
            "relative_to_explicit_common_w": relative,
            "condition_aware_bound": bound,
            "initial_residual_l2": candidate.initial_residual_l2,
            "linear_residual_l2": candidate.linear_residual_l2,
            "linear_residual_bound": residual_bound,
            "projection": &candidate.projection,
            "work": candidate.work,
            "explicit_w_used_by_candidate": false,
            "production_activation": false
        })
    );
    Ok(())
}

#[test]
fn correction_entry_rejects_explicit_w_before_projection_or_preparation_work() -> CoreResult<()> {
    let (problem, y0) = manufactured_vector_problem(8, 100.0, 1.0, 0.2, 0.0)?;
    let mut setup = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 1.0e-3, &mut setup)?;
    let stages =
        sequential_stages(&context, &LinearSolverConfig::default(), None, &mut setup)?.stages;
    let failure = match run_audit2_matrix_free_common_w_correction(
        &context,
        &stages,
        Audit2MatrixFreeCommonWConfig::default(),
    ) {
        Audit2MatrixFreeCorrectionOutcome::Completed(_) => {
            panic!("matrix-free correction must reject an explicit W")
        }
        Audit2MatrixFreeCorrectionOutcome::Failed(value) => *value,
    };
    assert_eq!(
        failure.phase,
        Audit2MatrixFreeCorrectionFailurePhase::InputValidation
    );
    assert!(failure.projection.is_none());
    assert!(failure.projected_residual.is_none());
    assert!(failure.partial_correction.is_empty());
    assert!(failure.partial_reports.is_empty());
    assert_eq!(failure.work.preparation_counters, WorkCounters::default());
    assert!(failure.work.session.is_none());
    Ok(())
}

#[test]
fn explicit_w_is_rejected_before_any_session_setup_is_admitted() -> CoreResult<()> {
    let (problem, y0) = manufactured_vector_problem(8, 100.0, 1.0, 0.2, 0.0)?;
    let context = build_step_context(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let failure =
        Audit2MatrixFreeCommonWSession::new(&context, Audit2MatrixFreeCommonWConfig::default())
            .expect_err("explicit W must fail closed");
    assert!(matches!(failure.error, CoreError::InvalidInput(_)));
    assert_eq!(failure.session.setup_attempts, 1);
    assert_eq!(failure.session.setup_completed, 0);
    assert_eq!(failure.session.identity_preconditioner_setups, 0);
    assert_eq!(failure.session.workspace_initializations, 0);
    Ok(())
}

#[test]
fn malformed_late_rhs_retains_completed_rows_and_spent_work() -> CoreResult<()> {
    let (problem, y0) = manufactured_vector_problem(12, 1.0e3, 2.0, 0.5, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 5.0e-4, &mut WorkCounters::default())?;
    let mut session =
        Audit2MatrixFreeCommonWSession::new(&context, Audit2MatrixFreeCommonWConfig::default())?;
    let mut rhs = rhs_rows(12);
    rhs.truncate(3);
    rhs[1][4] = f64::NAN;
    let failure = match session.solve_rows(&rhs) {
        Audit2MatrixFreeBatchOutcome::Completed(_) => panic!("NaN row must fail"),
        Audit2MatrixFreeBatchOutcome::Failed(value) => *value,
    };
    assert_eq!(failure.failed_row, Some(1));
    assert_eq!(failure.partial_solutions.len(), 1);
    assert_eq!(failure.partial_reports.len(), 1);
    assert_eq!(failure.session.batch_attempts, 1);
    assert_eq!(failure.session.batch_completed, 0);
    assert_eq!(failure.session.solve_attempts, 1);
    assert_eq!(failure.session.solve_completed, 1);
    assert!(failure.session.counters.linear_matvecs > 0);
    assert_eq!(
        failure.session.counters.jvp_calls,
        failure
            .session
            .counters
            .linear_matvecs
            .saturating_add(failure.session.counters.diagnostic_matvecs)
    );
    assert_eq!(
        failure.session.counters.jvp_vectors,
        failure.session.counters.jvp_calls
    );
    assert!(failure.session.workspace_capacity_current > 0);
    println!(
        "AUDIT2_MATRIX_FREE_FAILURE {}",
        serde_json::json!({
            "phase": failure.phase,
            "failed_row": failure.failed_row,
            "partial_solutions": failure.partial_solutions.len(),
            "partial_reports": failure.partial_reports.len(),
            "session": failure.session
        })
    );
    Ok(())
}

#[test]
fn invalid_gmres_config_is_rejected_before_setup_completion() -> CoreResult<()> {
    let (problem, y0) = manufactured_vector_problem(8, 100.0, 1.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let invalid_config = Audit2MatrixFreeCommonWConfig {
        restart: 0,
        ..Audit2MatrixFreeCommonWConfig::default()
    };
    let failure = Audit2MatrixFreeCommonWSession::new(&context, invalid_config)
        .expect_err("zero restart must fail");
    assert!(matches!(failure.error, CoreError::InvalidInput(_)));
    assert_eq!(failure.session.setup_attempts, 1);
    assert_eq!(failure.session.setup_completed, 0);
    assert_eq!(failure.session.identity_preconditioner_setups, 0);
    assert_eq!(failure.session.workspace_initializations, 0);

    let trial_stages = vec![vec![0.0; 8]; 8];
    let correction_failure =
        match run_audit2_matrix_free_common_w_correction(&context, &trial_stages, invalid_config) {
            Audit2MatrixFreeCorrectionOutcome::Completed(_) => {
                panic!("invalid GMRES config must fail before correction solves")
            }
            Audit2MatrixFreeCorrectionOutcome::Failed(value) => *value,
        };
    assert_eq!(
        correction_failure.phase,
        Audit2MatrixFreeCorrectionFailurePhase::Solve
    );
    let session = correction_failure
        .work
        .session
        .expect("attempted correction setup must retain its session snapshot");
    assert_eq!(session.setup_attempts, 1);
    assert_eq!(session.setup_completed, 0);
    assert_eq!(session.solve_attempts, 0);
    assert_eq!(session.solve_completed, 0);
    Ok(())
}
