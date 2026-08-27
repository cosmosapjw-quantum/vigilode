#[path = "../src/k0_stage_telemetry.rs"]
mod k0_stage_telemetry;

use k0_stage_telemetry::{
    K0_CLAIM_CLASS, K0_STAGE_RECEIPT_SCHEMA, K0ExecutionState, K0InitialGuessSource, K0KernelArm,
    K0ResidualSignCoverage, K0StageError, K0StageGeometry, K0StageIdentity, K0StageReceipt,
    K0StageWork,
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner, LinearOperator,
    LinearSolveReport, WorkCounters,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, LgmresConfig, LgmresState, LinearSolveObservation,
    ObservedInitialGuessSource, solve_gcrodr, solve_gcrodr_observed, solve_gmres,
    solve_gmres_givens, solve_gmres_givens_observed, solve_gmres_observed, solve_lgmres,
    solve_lgmres_observed,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn identity() -> K0StageIdentity {
    K0StageIdentity {
        case_id: "hires-legacy-fixed".into(),
        family: "Hires".into(),
        kernel_arm: K0KernelArm::LegacyRestartedGmres,
        attempt_id: 1,
        stage_index: 3,
    }
}

fn complete_receipt() -> K0StageReceipt {
    K0StageReceipt {
        schema: K0_STAGE_RECEIPT_SCHEMA.into(),
        identity: identity(),
        claim_class: K0_CLAIM_CLASS.into(),
        execution_state: K0ExecutionState::Complete,
        solver_method: Some("legacy-restarted-gmres".into()),
        initial_guess_source: Some(K0InitialGuessSource::Zero),
        initial_true_residual_norm: Some(4.0),
        final_true_residual_norm: Some(1.0e-12),
        work: Some(K0StageWork::new(5, 2, 1).expect("checked work")),
        geometry: Some(K0StageGeometry {
            scaled_nonlinear_remainder: Some(0.25),
            rhs_jacobian_angle_radians: Some(0.5),
            singular_values: vec![2.0, 0.5],
        }),
        error: None,
    }
}

#[test]
fn schema_accepts_exact_contract_and_rejects_wrong_schema_claim_or_identity() {
    let receipt = complete_receipt();
    assert_eq!(receipt.validate(), Ok(()));
    assert_eq!(
        receipt.identity.kernel_arm.as_str(),
        "legacy-restarted-gmres"
    );
    assert_eq!(
        K0KernelArm::IncrementalGivensCandidate.as_str(),
        "incremental-givens-candidate"
    );

    let mut wrong_schema = receipt.clone();
    wrong_schema.schema = "vigilode-k0-stage-receipt/v0".into();
    assert!(wrong_schema.validate().is_err());

    let mut wrong_claim = receipt.clone();
    wrong_claim.claim_class = "AUTHORITATIVE".into();
    assert!(wrong_claim.validate().is_err());

    let mut missing_family = receipt;
    missing_family.identity.family.clear();
    assert!(missing_family.validate().is_err());
}

#[test]
fn accounting_checked_total_accepts_exact_and_rejects_diagnostic_omission() {
    assert_eq!(
        K0StageWork::new(11, 7, 3).unwrap().operator_applies_total,
        18
    );

    let omitted_diagnostic = K0StageWork {
        linear_matvecs: 11,
        diagnostic_matvecs: 7,
        operator_applies_total: 11,
        telemetry_jvp_overhead: 3,
    };
    assert!(omitted_diagnostic.validate().is_err());
}

#[test]
fn accounting_counter_overflow_is_rejected() {
    assert!(K0StageWork::new(u64::MAX, 1, 0).is_err());
}

#[test]
fn execution_state_error_preserves_identity_work_and_error() {
    let mut receipt = complete_receipt();
    receipt.execution_state = K0ExecutionState::Error;
    receipt.error = Some(K0StageError {
        kind: "LINEAR_SOLVE_FAILURE".into(),
        message: "true residual remained above tolerance".into(),
    });
    assert_eq!(receipt.validate(), Ok(()));
    assert_eq!(receipt.identity(), &identity());
    assert!(receipt.work.is_some());
}

#[test]
fn execution_state_error_without_work_is_rejected() {
    let mut receipt = complete_receipt();
    receipt.execution_state = K0ExecutionState::Error;
    receipt.error = Some(K0StageError {
        kind: "LINEAR_SOLVE_FAILURE".into(),
        message: "failed before completion".into(),
    });
    receipt.work = None;
    assert!(receipt.validate().is_err());
}

#[test]
fn schema_rejects_nan_and_infinite_norms_geometry() {
    let mut nan_norm = complete_receipt();
    nan_norm.final_true_residual_norm = Some(f64::NAN);
    assert!(nan_norm.validate().is_err());

    let mut infinite_angle = complete_receipt();
    infinite_angle
        .geometry
        .as_mut()
        .unwrap()
        .rhs_jacobian_angle_radians = Some(f64::INFINITY);
    assert!(infinite_angle.validate().is_err());

    let mut negative_singular_value = complete_receipt();
    negative_singular_value
        .geometry
        .as_mut()
        .unwrap()
        .singular_values = vec![1.0, -0.1];
    assert!(negative_singular_value.validate().is_err());
}

#[test]
fn residual_sign_norm_telemetry_is_explicitly_non_authoritative() {
    assert_eq!(
        K0StageReceipt::residual_sign_coverage(),
        K0ResidualSignCoverage::NotCoveredByNormTelemetry
    );
}

fn solver_system() -> (DenseOperator, Vec<f64>) {
    let matrix =
        DenseMatrix::from_rows(&[&[4.0, 1.0, 0.0], &[2.0, 3.0, 1.0], &[0.0, -1.0, 2.0]]).unwrap();
    let rhs = matrix.matvec(&[1.0, -2.0, 0.5]).unwrap();
    (DenseOperator::new(matrix).unwrap(), rhs)
}

fn gmres_config() -> GmresConfig {
    GmresConfig {
        restart: 3,
        max_arnoldi: 30,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    }
}

fn lgmres_config() -> LgmresConfig {
    LgmresConfig {
        inner_m: 2,
        max_outer: 10,
        outer_k: 2,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    }
}

fn gcrodr_config() -> GcrodrConfig {
    GcrodrConfig {
        restart: 3,
        max_arnoldi: 30,
        recycle_dim: 1,
        rank_tol: 1.0e-12,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    }
}

fn assert_report_parity(left: &LinearSolveReport, right: &LinearSolveReport) {
    assert_eq!(left.x, right.x);
    assert_eq!(left.converged, right.converged);
    assert_eq!(left.info, right.info);
    assert_eq!(left.residual_norm.to_bits(), right.residual_norm.to_bits());
    assert_eq!(
        left.relative_residual.to_bits(),
        right.relative_residual.to_bits()
    );
    assert_eq!(left.iterations, right.iterations);
    assert_eq!(left.matvecs, right.matvecs);
    assert_eq!(left.preconditioner_apps, right.preconditioner_apps);
    assert_eq!(left.method, right.method);
}

#[test]
fn solver_observation_production_path_is_observationally_identical() {
    let (operator, rhs) = solver_system();
    let preconditioner = IdentityPreconditioner::new(3);

    for givens in [false, true] {
        let mut plain_work = WorkCounters::default();
        let plain = if givens {
            solve_gmres_givens(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gmres_config(),
                &mut plain_work,
            )
        } else {
            solve_gmres(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gmres_config(),
                &mut plain_work,
            )
        }
        .unwrap();
        let mut observed_work = WorkCounters::default();
        let mut observation = LinearSolveObservation::default();
        let observed = if givens {
            solve_gmres_givens_observed(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gmres_config(),
                &mut observed_work,
                &mut observation,
            )
        } else {
            solve_gmres_observed(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gmres_config(),
                &mut observed_work,
                &mut observation,
            )
        }
        .unwrap();
        assert_report_parity(&plain, &observed);
        assert_eq!(plain_work, observed_work);
        assert_eq!(
            observation.initial_guess_source,
            Some(ObservedInitialGuessSource::Zero)
        );
        assert_eq!(
            observation.final_true_residual_norm,
            Some(observed.residual_norm)
        );
    }

    let mut plain_lgmres_state = LgmresState::default();
    let mut observed_lgmres_state = LgmresState::default();
    let mut plain_work = WorkCounters::default();
    let plain = solve_lgmres(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres_config(),
        &mut plain_lgmres_state,
        &mut plain_work,
    )
    .unwrap();
    let mut observed_work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    let observed = solve_lgmres_observed(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres_config(),
        &mut observed_lgmres_state,
        &mut observed_work,
        &mut observation,
    )
    .unwrap();
    assert_report_parity(&plain, &observed);
    assert_eq!(plain_work, observed_work);
    assert_eq!(plain_lgmres_state, observed_lgmres_state);

    let mut plain_gcrodr_state = GcrodrState::default();
    let mut observed_gcrodr_state = GcrodrState::default();
    let mut plain_work = WorkCounters::default();
    let plain = solve_gcrodr(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config(),
        &mut plain_gcrodr_state,
        &mut plain_work,
    )
    .unwrap();
    let mut observed_work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    let observed = solve_gcrodr_observed(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config(),
        &mut observed_gcrodr_state,
        &mut observed_work,
        &mut observation,
    )
    .unwrap();
    assert_report_parity(&plain, &observed);
    assert_eq!(plain_work, observed_work);
    assert_eq!(plain_gcrodr_state, observed_gcrodr_state);
}

#[test]
fn solver_observation_initial_residual_observation_adds_no_hidden_apply() {
    let (operator, rhs) = solver_system();
    let preconditioner = IdentityPreconditioner::new(3);
    let x0 = [0.25, -0.5, 0.125];

    for givens in [false, true] {
        let mut plain_work = WorkCounters::default();
        if givens {
            solve_gmres_givens(
                &operator,
                &preconditioner,
                &rhs,
                Some(&x0),
                &gmres_config(),
                &mut plain_work,
            )
        } else {
            solve_gmres(
                &operator,
                &preconditioner,
                &rhs,
                Some(&x0),
                &gmres_config(),
                &mut plain_work,
            )
        }
        .unwrap();

        let mut observed_work = WorkCounters::default();
        let mut observation = LinearSolveObservation::default();
        if givens {
            solve_gmres_givens_observed(
                &operator,
                &preconditioner,
                &rhs,
                Some(&x0),
                &gmres_config(),
                &mut observed_work,
                &mut observation,
            )
        } else {
            solve_gmres_observed(
                &operator,
                &preconditioner,
                &rhs,
                Some(&x0),
                &gmres_config(),
                &mut observed_work,
                &mut observation,
            )
        }
        .unwrap();
        assert_eq!(plain_work, observed_work);
        assert_eq!(
            observation.initial_guess_source,
            Some(ObservedInitialGuessSource::ExternalPrevious)
        );
        assert!(observation.initial_true_residual_norm.unwrap() > 0.0);
    }
}

#[test]
fn solver_observation_initial_guess_source_identity_matrix() {
    let (operator, rhs) = solver_system();
    let preconditioner = IdentityPreconditioner::new(3);

    let mut lgmres_state = LgmresState {
        operator_token: Some(operator.token()),
        previous_solution: Some(vec![0.25, -0.5, 0.125]),
        ..LgmresState::default()
    };
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    solve_lgmres_observed(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres_config(),
        &mut lgmres_state,
        &mut work,
        &mut observation,
    )
    .unwrap();
    assert_eq!(
        observation.initial_guess_source,
        Some(ObservedInitialGuessSource::InternalLgmresPrevious)
    );

    let mut gcrodr_state = GcrodrState {
        operator_token: Some(operator.token()),
        previous_solution: Some(vec![0.25, -0.5, 0.125]),
        ..GcrodrState::default()
    };
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    solve_gcrodr_observed(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config(),
        &mut gcrodr_state,
        &mut work,
        &mut observation,
    )
    .unwrap();
    assert_eq!(
        observation.initial_guess_source,
        Some(ObservedInitialGuessSource::InternalGcrodrPrevious)
    );

    let mut recycle_state = GcrodrState::default();
    let mut seed_work = WorkCounters::default();
    solve_gcrodr(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config(),
        &mut recycle_state,
        &mut seed_work,
    )
    .unwrap();
    assert!(recycle_state.rank() > 0);
    recycle_state.previous_solution = None;
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    solve_gcrodr_observed(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr_config(),
        &mut recycle_state,
        &mut work,
        &mut observation,
    )
    .unwrap();
    assert_eq!(
        observation.initial_guess_source,
        Some(ObservedInitialGuessSource::RecycleProjection)
    );
    assert!(work.recycle_projection_calls > 0);
}

#[test]
fn solver_observation_recycle_rollback_is_unchanged() {
    let (operator, rhs) = solver_system();
    let preconditioner = IdentityPreconditioner::new(3);
    let bad_rhs = [f64::NAN, rhs[1], rhs[2]];

    let mut lgmres_state = LgmresState {
        directions: vec![vec![1.0, 0.0, 0.0]],
        images: vec![None],
        operator_token: Some(operator.token()),
        previous_solution: Some(vec![0.1, 0.2, 0.3]),
        generation: 9,
    };
    let lgmres_snapshot = lgmres_state.clone();
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    assert!(
        solve_lgmres_observed(
            &operator,
            &preconditioner,
            &bad_rhs,
            None,
            &lgmres_config(),
            &mut lgmres_state,
            &mut work,
            &mut observation,
        )
        .is_err()
    );
    assert_eq!(lgmres_state, lgmres_snapshot);

    let mut gcrodr_state = GcrodrState {
        basis: vec![vec![1.0, 0.0, 0.0]],
        image: vec![vec![4.0, 2.0, 0.0]],
        operator_token: Some(operator.token()),
        previous_solution: Some(vec![0.1, 0.2, 0.3]),
        generation: 9,
    };
    let gcrodr_snapshot = gcrodr_state.clone();
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    assert!(
        solve_gcrodr_observed(
            &operator,
            &preconditioner,
            &bad_rhs,
            None,
            &gcrodr_config(),
            &mut gcrodr_state,
            &mut work,
            &mut observation,
        )
        .is_err()
    );
    assert_eq!(gcrodr_state, gcrodr_snapshot);
}

struct FailAfterRecycleProjectionOperator {
    matrix: DenseMatrix,
    calls: AtomicUsize,
    fail_on_call: usize,
    token: u64,
}

impl LinearOperator for FailAfterRecycleProjectionOperator {
    fn dimension(&self) -> usize {
        self.matrix.nrows()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call == self.fail_on_call {
            return Err(CoreError::LinearSolve(
                "injected failure after recycle projection".into(),
            ));
        }
        self.matrix.matvec_into(x, y)
    }

    fn token(&self) -> u64 {
        self.token
    }
}

#[test]
fn solver_observation_failure_after_recycle_projection_rolls_back() {
    let matrix =
        DenseMatrix::from_rows(&[&[4.0, 1.0, 0.0], &[2.0, 3.0, 1.0], &[0.0, -1.0, 2.0]]).unwrap();
    let rhs = matrix.matvec(&[1.0, -2.0, 0.5]).unwrap();
    let operator = FailAfterRecycleProjectionOperator {
        matrix,
        calls: AtomicUsize::new(0),
        fail_on_call: 2,
        token: 919,
    };
    let preconditioner = IdentityPreconditioner::new(3);
    let mut state = GcrodrState {
        basis: vec![vec![1.0, 0.0, 0.0]],
        image: vec![vec![4.0, 2.0, 0.0]],
        operator_token: Some(operator.token()),
        previous_solution: None,
        generation: 3,
    };
    let snapshot = state.clone();
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    assert!(
        solve_gcrodr_observed(
            &operator,
            &preconditioner,
            &rhs,
            None,
            &gcrodr_config(),
            &mut state,
            &mut work,
            &mut observation,
        )
        .is_err()
    );
    assert_eq!(
        observation.initial_guess_source,
        Some(ObservedInitialGuessSource::RecycleProjection)
    );
    assert_eq!(work.recycle_projection_calls, 1);
    assert_eq!(state, snapshot);
}

#[test]
fn solver_observation_true_residual_remains_authority() {
    let (operator, rhs) = solver_system();
    let preconditioner = IdentityPreconditioner::new(3);
    let mut work = WorkCounters::default();
    let mut observation = LinearSolveObservation::default();
    let report = solve_gmres_givens_observed(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres_config(),
        &mut work,
        &mut observation,
    )
    .unwrap();
    let threshold = gmres_config()
        .atol
        .max(gmres_config().rtol * rodas5p_core::safe_l2(&rhs));
    assert_eq!(
        observation.final_true_residual_norm,
        Some(report.residual_norm)
    );
    assert!(observation.final_true_residual_norm.unwrap() <= threshold);
    assert!(work.diagnostic_matvecs >= 1);
}
