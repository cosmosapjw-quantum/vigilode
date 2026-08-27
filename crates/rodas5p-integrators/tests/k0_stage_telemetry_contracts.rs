#[path = "../src/k0_stage_telemetry.rs"]
#[allow(dead_code)]
mod k0_stage_telemetry;

use k0_stage_telemetry::{
    K0_CLAIM_CLASS, K0_STAGE_RECEIPT_SCHEMA, K0ExecutionState, K0InitialGuessSource, K0KernelArm,
    K0ResidualSignCoverage, K0StageError, K0StageGeometry, K0StageIdentity, K0StageReceipt,
    K0StageWork, scaled_nonlinear_remainder,
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner, InitialGuess,
    LinearMethod, LinearOperator, LinearSolveReport, LinearSolverConfig, PreconditionerKind,
    WorkCounters, sha256_hex,
};
use rodas5p_integrators::{
    GmresKernelArm, OdeProblem, analyze_stage_rhs_novelty, manufactured_vector_problem,
    observe_sequential_matrix_free_step_k0, solver_configuration_token_k0,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, LgmresConfig, LgmresState, LinearSolveObservation,
    ObservedInitialGuessSource, solve_gcrodr, solve_gcrodr_observed, solve_gmres,
    solve_gmres_givens, solve_gmres_givens_observed, solve_gmres_observed, solve_lgmres,
    solve_lgmres_observed,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

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

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.17e} expected={expected:.17e} tolerance={tolerance:.3e}"
    );
}

#[test]
fn nonlinear_remainder_equation_map() {
    assert_eq!(
        scaled_nonlinear_remainder(&[5.0, -7.0], &[2.0, -3.0], &[3.0, -4.0], &[0.0, 0.0]).unwrap(),
        0.0
    );
    assert_close(
        scaled_nonlinear_remainder(&[4.0], &[1.0], &[1.0], &[1.0]).unwrap(),
        0.25,
        0.0,
    );
}

#[test]
fn scaled_nonlinear_remainder_exact_oracle_and_mutations() {
    let positive =
        scaled_nonlinear_remainder(&[6.0, -7.0], &[2.0, -3.0], &[3.0, -4.0], &[0.0, 0.0]).unwrap();
    assert_close(positive, 0.10846522890932808, 1.0e-15);

    let uncapped = scaled_nonlinear_remainder(&[1.0], &[-1.0], &[-1.0], &[-1.0]).unwrap();
    assert_eq!(uncapped, 4.0);
    assert!(uncapped > 1.0, "rho_i must not be capped at one");

    let denominator_witness =
        scaled_nonlinear_remainder(&[1.0, 0.0], &[0.0, 10.0], &[0.0, 0.0], &[0.0, 0.0]).unwrap();
    assert_close(denominator_witness, 101.0_f64.sqrt() / 10.0, 1.0e-15);
    let fi_only_mutant = 101.0_f64.sqrt();
    assert!((denominator_witness - fi_only_mutant).abs() > 1.0);

    let omitted_time_mutant = 3.0;
    assert_ne!(uncapped, omitted_time_mutant);

    let remainder = [1.0_f64, 0.0];
    let tolerance_weighted_wrms_mutant =
        ((remainder[0] / 2.0).powi(2) + (remainder[1] / 5.0).powi(2)).sqrt() / 2.0_f64.sqrt();
    assert!((positive - tolerance_weighted_wrms_mutant).abs() > 1.0e-2);
}

#[test]
fn scaled_nonlinear_remainder_is_jointly_scale_invariant() {
    let fi = [6.0, -7.0];
    let f0 = [2.0, -3.0];
    let j_delta = [3.0, -4.0];
    let t_delta = [0.0, 0.0];
    let baseline = scaled_nonlinear_remainder(&fi, &f0, &j_delta, &t_delta).unwrap();
    for factor in [2.0_f64.powi(40), 2.0_f64.powi(-40)] {
        let scaled = |values: &[f64]| {
            values
                .iter()
                .map(|value| factor * value)
                .collect::<Vec<_>>()
        };
        let actual = scaled_nonlinear_remainder(
            &scaled(&fi),
            &scaled(&f0),
            &scaled(&j_delta),
            &scaled(&t_delta),
        )
        .unwrap();
        assert_close(actual, baseline, 2.0e-15);
    }
}

#[test]
fn rhs_novelty_angle_is_not_nested_space_overlap() {
    let retained = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
    let orthogonal = analyze_stage_rhs_novelty(&retained, &[0.0, 0.0, 1.0]).unwrap();
    assert_close(
        orthogonal.rhs_to_retained_angle_rad.unwrap(),
        std::f64::consts::FRAC_PI_2,
        1.0e-14,
    );

    let contained = analyze_stage_rhs_novelty(&retained, &[2.0, -3.0, 0.0]).unwrap();
    assert_close(contained.rhs_to_retained_angle_rad.unwrap(), 0.0, 1.0e-14);
}

#[test]
fn rhs_geometry_known_angles_and_independent_scale_invariance() {
    let retained = vec![vec![1.0, 0.0, 0.0]];
    let parallel = analyze_stage_rhs_novelty(&retained, &[-7.0, 0.0, 0.0]).unwrap();
    assert_close(parallel.rhs_to_retained_angle_rad.unwrap(), 0.0, 1.0e-14);

    let orthogonal = analyze_stage_rhs_novelty(&retained, &[0.0, 3.0, 0.0]).unwrap();
    assert_close(
        orthogonal.rhs_to_retained_angle_rad.unwrap(),
        std::f64::consts::FRAC_PI_2,
        1.0e-14,
    );

    let scaled_retained = vec![vec![-13.0, 0.0, 0.0]];
    let scaled = analyze_stage_rhs_novelty(&scaled_retained, &[0.0, -0.125, 0.0]).unwrap();
    assert_close(
        scaled.rhs_to_retained_angle_rad.unwrap(),
        orthogonal.rhs_to_retained_angle_rad.unwrap(),
        1.0e-14,
    );
    assert_eq!(
        scaled.rhs_singular_values.len(),
        orthogonal.rhs_singular_values.len()
    );
    for (actual, expected) in scaled
        .rhs_singular_values
        .iter()
        .zip(&orthogonal.rhs_singular_values)
    {
        assert_close(*actual, *expected, 1.0e-14);
    }

    let first = analyze_stage_rhs_novelty(&[], &[3.0, 4.0]).unwrap();
    assert_eq!(first.rhs_to_retained_angle_rad, None);
    assert_eq!(first.rhs_singular_values, vec![1.0]);
}

fn k0_linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 40,
        maxiter: 200,
        inner_m: 30,
        outer_k: 8,
        recycle_dim: 8,
        recycle_rank_tol: 1.0e-12,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
    }
}

#[test]
fn deterministic_solver_configuration_token_uses_only_canonical_values() {
    let config = k0_linear_config();
    let canonical = format!(
        "vigilode-k0-solver-config/v1|method=Gmres|rtol={:016x}|atol={:016x}|restart=40|maxiter=200|inner_m=30|outer_k=8|recycle_dim=8|recycle_rank_tol={:016x}|preconditioner=None|x0_strategy=Previous|kernel=legacy-restarted-gmres",
        config.rtol.to_bits(),
        config.atol.to_bits(),
        config.recycle_rank_tol.to_bits(),
    );
    let expected = sha256_hex(canonical.as_bytes());
    assert_eq!(
        solver_configuration_token_k0(&config, GmresKernelArm::LegacyRestartedGmres),
        expected
    );
    assert_eq!(
        solver_configuration_token_k0(&config.clone(), GmresKernelArm::LegacyRestartedGmres),
        expected
    );

    let mut changed = config;
    changed.preconditioner = PreconditionerKind::Jacobi;
    assert_ne!(
        solver_configuration_token_k0(&changed, GmresKernelArm::LegacyRestartedGmres),
        expected
    );
}

fn assert_step_parity(
    left: &rodas5p_integrators::StepResult,
    right: &rodas5p_integrators::StepResult,
) {
    assert_eq!(left.t_old.to_bits(), right.t_old.to_bits());
    assert_eq!(left.t_new.to_bits(), right.t_new.to_bits());
    assert_eq!(left.y_old, right.y_old);
    assert_eq!(left.y_new, right.y_new);
    assert_eq!(left.h.to_bits(), right.h.to_bits());
    assert_eq!(left.stages, right.stages);
    assert_eq!(left.error_vector, right.error_vector);
    assert_eq!(left.error_norm.to_bits(), right.error_norm.to_bits());
    assert_eq!(left.accepted, right.accepted);
    assert_eq!(left.method, right.method);
    assert_eq!(left.used_fallback, right.used_fallback);
    assert_eq!(left.counters, right.counters);
}

#[test]
fn stage_accounting_closes_and_telemetry_overhead_is_named_without_behavior_change() {
    let (problem, y0) = manufactured_vector_problem(4, 100.0, 10.0, 0.2, 0.0).unwrap();
    let config = k0_linear_config();
    let mut plain_work = WorkCounters::default();
    let plain = rodas5p_integrators::sequential_matrix_free_step_with_gmres_kernel(
        &problem,
        0.0,
        &y0,
        1.0e-4,
        &config,
        GmresKernelArm::LegacyRestartedGmres,
        None,
        1.0e-12,
        1.0e-10,
        true,
        &mut plain_work,
    )
    .unwrap();

    let mut observed_work = WorkCounters::default();
    let observed = observe_sequential_matrix_free_step_k0(
        "manufactured-k0-step",
        "manufactured-vector",
        &problem,
        0.0,
        &y0,
        1.0e-4,
        &config,
        GmresKernelArm::LegacyRestartedGmres,
        None,
        1.0e-12,
        1.0e-10,
        true,
        &mut observed_work,
    );
    assert_eq!(observed.error, None);
    assert_step_parity(&plain, observed.step.as_ref().unwrap());
    assert_eq!(plain_work, observed_work);
    assert_eq!(observed.stage_receipts.len(), 8);
    rodas5p_integrators::validate_stage_receipt_sequence(&observed.stage_receipts).unwrap();
    assert_eq!(observed.solver_configuration_tokens.len(), 1);
    for receipt in &observed.stage_receipts {
        let work = receipt.work.as_ref().unwrap();
        assert_eq!(
            work.operator_applies_total,
            work.linear_matvecs
                .checked_add(work.diagnostic_matvecs)
                .unwrap()
        );
    }
    assert_eq!(
        observed
            .stage_receipts
            .iter()
            .map(|receipt| receipt.work.as_ref().unwrap().telemetry_jvp_overhead)
            .sum::<u64>(),
        7
    );
}

fn stage_three_failure_problem() -> (OdeProblem, Vec<f64>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let rhs_calls = calls.clone();
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        if rhs_calls.fetch_add(1, Ordering::SeqCst) == 3 {
            return Err(CoreError::InvalidInput(
                "injected stage-3 RHS failure".into(),
            ));
        }
        out[0] = -y[0];
        Ok(())
    });
    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = -v[0];
        Ok(())
    });
    (
        OdeProblem::new(
            "k0-stage-three-failure",
            1,
            rhs,
            None,
            None,
            Some(jvp),
            None,
            true,
            None,
            None,
        )
        .unwrap(),
        vec![1.0],
    )
}

#[test]
fn stage_identity_is_exhaustive_and_injected_stage_three_error_is_preserved() {
    let (problem, y0) = stage_three_failure_problem();
    let mut work = WorkCounters::default();
    let outcome = observe_sequential_matrix_free_step_k0(
        "injected-stage-failure",
        "manufactured-failure",
        &problem,
        0.0,
        &y0,
        1.0e-3,
        &k0_linear_config(),
        GmresKernelArm::LegacyRestartedGmres,
        None,
        1.0e-12,
        1.0e-10,
        true,
        &mut work,
    );
    assert!(outcome.step.is_none());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap()
            .contains("injected stage-3")
    );
    assert_eq!(outcome.stage_receipts.len(), 4);
    assert_eq!(
        outcome
            .stage_receipts
            .iter()
            .map(|receipt| receipt.identity.stage_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert!(outcome.stage_receipts[..3].iter().all(|receipt| {
        receipt.execution_state == rodas5p_integrators::K0ExecutionState::Complete
    }));
    assert_eq!(
        outcome.stage_receipts[3].execution_state,
        rodas5p_integrators::K0ExecutionState::Error
    );
    assert!(outcome.stage_receipts[3].work.is_some());
    assert!(outcome.stage_receipts[3].error.is_some());
    rodas5p_integrators::validate_stage_receipt_sequence(&outcome.stage_receipts).unwrap();

    let mut duplicate = outcome.stage_receipts.clone();
    duplicate.push(duplicate[2].clone());
    assert!(rodas5p_integrators::validate_stage_receipt_sequence(&duplicate).is_err());

    let mut missing = outcome.stage_receipts;
    missing.remove(1);
    assert!(rodas5p_integrators::validate_stage_receipt_sequence(&missing).is_err());
}
