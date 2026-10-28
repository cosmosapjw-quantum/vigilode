use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner, LinearOperator,
    LinearSolveReport, Preconditioner, WorkCounters, wrms,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, GmresGivensWorkspace, LgmresConfig, LgmresState,
    solve_gcrodr, solve_gcrodr_with_residual_scale, solve_gmres, solve_gmres_givens,
    solve_gmres_givens_with_residual_scale, solve_gmres_givens_with_workspace_and_residual_scale,
    solve_gmres_with_residual_scale, solve_lgmres, solve_lgmres_with_residual_scale,
};

fn independently_certified_wrms(
    operator: &DenseOperator,
    rhs: &[f64],
    x: &[f64],
    scale: &[f64],
) -> f64 {
    let mut applied = vec![0.0; rhs.len()];
    operator.apply(x, &mut applied).unwrap();
    let residual = rhs
        .iter()
        .zip(applied)
        .map(|(right, left)| right - left)
        .collect::<Vec<_>>();
    wrms(&residual, scale).unwrap()
}

fn assert_report_bits_equal(left: &LinearSolveReport, right: &LinearSolveReport) {
    assert_eq!(left.x.len(), right.x.len());
    for (left, right) in left.x.iter().zip(&right.x) {
        assert_eq!(left.to_bits(), right.to_bits());
    }
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

struct UnevenPreconditioner {
    factors: Vec<f64>,
}

impl Preconditioner for UnevenPreconditioner {
    fn dimension(&self) -> usize {
        self.factors.len()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        for ((out, value), factor) in y.iter_mut().zip(x).zip(&self.factors) {
            *out = value * factor;
        }
        Ok(())
    }
}

#[test]
fn wrms_certification_prevents_large_components_from_hiding_small_component_residuals() {
    // Defect caught: an unweighted threshold based on ||b||_2 accepts x0 because
    // the 1e12 component makes a unit residual in component two look negligible.
    let operator = DenseOperator::new(DenseMatrix::identity(2)).unwrap();
    let preconditioner = IdentityPreconditioner::new(2);
    let rhs = [1.0e12, 1.0];
    let x0 = [1.0e12, 0.0];
    let scale = [1.0e12, 1.0];
    let gmres = GmresConfig {
        restart: 2,
        max_arnoldi: 4,
        rtol: 1.0e-6,
        atol: 0.0,
    };

    let mut work = WorkCounters::default();
    let report = solve_gmres_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        Some(&x0),
        &gmres,
        Some(&scale),
        &mut work,
    )
    .unwrap();
    assert!(report.iterations > 0);
    assert!((report.x[1] - 1.0).abs() < 1.0e-12);
    assert!(independently_certified_wrms(&operator, &rhs, &report.x, &scale) <= 1.0e-6);

    let mut work = WorkCounters::default();
    let report = solve_gmres_givens_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        Some(&x0),
        &gmres,
        Some(&scale),
        &mut work,
    )
    .unwrap();
    assert!(report.iterations > 0);
    assert!((report.x[1] - 1.0).abs() < 1.0e-12);
    assert!(independently_certified_wrms(&operator, &rhs, &report.x, &scale) <= 1.0e-6);

    let mut state = LgmresState::default();
    let mut work = WorkCounters::default();
    let report = solve_lgmres_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        Some(&x0),
        &LgmresConfig {
            inner_m: 2,
            max_outer: 4,
            outer_k: 1,
            rtol: 1.0e-6,
            atol: 0.0,
        },
        &mut state,
        Some(&scale),
        &mut work,
    )
    .unwrap();
    assert!(report.iterations > 0);
    assert!((report.x[1] - 1.0).abs() < 1.0e-12);
    assert!(independently_certified_wrms(&operator, &rhs, &report.x, &scale) <= 1.0e-6);

    let mut state = GcrodrState::default();
    let mut work = WorkCounters::default();
    let report = solve_gcrodr_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        Some(&x0),
        &GcrodrConfig {
            restart: 2,
            max_arnoldi: 4,
            recycle_dim: 1,
            rank_tol: 1.0e-12,
            rtol: 1.0e-6,
            atol: 0.0,
        },
        &mut state,
        Some(&scale),
        &mut work,
    )
    .unwrap();
    assert!(report.iterations > 0);
    assert!((report.x[1] - 1.0).abs() < 1.0e-12);
    assert!(independently_certified_wrms(&operator, &rhs, &report.x, &scale) <= 1.0e-6);
}

#[test]
fn none_scale_is_bit_identical_to_each_legacy_solver_entry_point() {
    // Defect caught: adding WRMS control to the legacy function body can silently
    // change old reports, work accounting, or committed recycle state.
    let matrix = DenseMatrix::from_rows(&[&[4.0, 1.0], &[-0.5, 2.0]]).unwrap();
    let operator = DenseOperator::new(matrix.clone()).unwrap();
    let preconditioner = IdentityPreconditioner::new(2);
    let rhs = matrix.matvec(&[0.75, -1.25]).unwrap();

    let gmres = GmresConfig {
        restart: 2,
        max_arnoldi: 8,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    };
    let mut legacy_work = WorkCounters::default();
    let legacy = solve_gmres(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres,
        &mut legacy_work,
    )
    .unwrap();
    let mut sibling_work = WorkCounters::default();
    let sibling = solve_gmres_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres,
        None,
        &mut sibling_work,
    )
    .unwrap();
    assert_report_bits_equal(&legacy, &sibling);
    assert_eq!(legacy_work, sibling_work);

    let mut legacy_work = WorkCounters::default();
    let legacy = solve_gmres_givens(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres,
        &mut legacy_work,
    )
    .unwrap();
    let mut sibling_work = WorkCounters::default();
    let mut workspace = GmresGivensWorkspace::default();
    let sibling = solve_gmres_givens_with_workspace_and_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gmres,
        &mut workspace,
        None,
        &mut sibling_work,
    )
    .unwrap();
    assert_report_bits_equal(&legacy, &sibling);
    assert_eq!(legacy_work, sibling_work);

    let lgmres = LgmresConfig {
        inner_m: 2,
        max_outer: 4,
        outer_k: 1,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    };
    let mut legacy_state = LgmresState::default();
    let mut legacy_work = WorkCounters::default();
    let legacy = solve_lgmres(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres,
        &mut legacy_state,
        &mut legacy_work,
    )
    .unwrap();
    let mut sibling_state = LgmresState::default();
    let mut sibling_work = WorkCounters::default();
    let sibling = solve_lgmres_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &lgmres,
        &mut sibling_state,
        None,
        &mut sibling_work,
    )
    .unwrap();
    assert_report_bits_equal(&legacy, &sibling);
    assert_eq!(legacy_work, sibling_work);
    assert_eq!(legacy_state, sibling_state);

    let gcrodr = GcrodrConfig {
        restart: 2,
        max_arnoldi: 8,
        recycle_dim: 1,
        rank_tol: 1.0e-12,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    };
    let mut legacy_state = GcrodrState::default();
    let mut legacy_work = WorkCounters::default();
    let legacy = solve_gcrodr(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr,
        &mut legacy_state,
        &mut legacy_work,
    )
    .unwrap();
    let mut sibling_state = GcrodrState::default();
    let mut sibling_work = WorkCounters::default();
    let sibling = solve_gcrodr_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &gcrodr,
        &mut sibling_state,
        None,
        &mut sibling_work,
    )
    .unwrap();
    assert_report_bits_equal(&legacy, &sibling);
    assert_eq!(legacy_work, sibling_work);
    assert_eq!(legacy_state, sibling_state);
}

#[test]
fn invalid_wrms_scales_fail_before_work_or_recycle_state_changes() {
    // Defect caught: late scale validation can consume matvecs or partially commit
    // an LGMRES/GCRO-DR recycle generation before returning an input error.
    let operator = DenseOperator::new(DenseMatrix::identity(2)).unwrap();
    let preconditioner = IdentityPreconditioner::new(2);
    let rhs = [1.0, 2.0];
    let gmres = GmresConfig::default();
    let lgmres = LgmresConfig::default();
    let gcrodr = GcrodrConfig {
        restart: 4,
        recycle_dim: 1,
        ..GcrodrConfig::default()
    };
    let invalid_scales = [vec![1.0], vec![1.0, 0.0], vec![1.0, f64::NAN]];

    for scale in &invalid_scales {
        let mut work = WorkCounters {
            rhs_calls: 7,
            ..WorkCounters::default()
        };
        let before = work;
        assert!(
            solve_gmres_with_residual_scale(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gmres,
                Some(scale),
                &mut work,
            )
            .is_err()
        );
        assert_eq!(work, before);

        let mut workspace = GmresGivensWorkspace::default();
        let mut work = before;
        assert!(
            solve_gmres_givens_with_workspace_and_residual_scale(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gmres,
                &mut workspace,
                Some(scale),
                &mut work,
            )
            .is_err()
        );
        assert_eq!(work, before);

        let mut state = LgmresState {
            previous_solution: Some(vec![0.25, -0.5]),
            generation: 3,
            ..LgmresState::default()
        };
        let state_before = state.clone();
        let mut work = before;
        assert!(
            solve_lgmres_with_residual_scale(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &lgmres,
                &mut state,
                Some(scale),
                &mut work,
            )
            .is_err()
        );
        assert_eq!(work, before);
        assert_eq!(state, state_before);

        let mut state = GcrodrState {
            previous_solution: Some(vec![0.25, -0.5]),
            generation: 3,
            ..GcrodrState::default()
        };
        let state_before = state.clone();
        let mut work = before;
        assert!(
            solve_gcrodr_with_residual_scale(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &gcrodr,
                &mut state,
                Some(scale),
                &mut work,
            )
            .is_err()
        );
        assert_eq!(work, before);
        assert_eq!(state, state_before);
    }
}

#[test]
fn reported_norm_is_raw_wrms_even_with_a_strongly_uneven_preconditioner() {
    // Defect caught: certifying ||M^-1 r|| instead of the raw residual can report
    // a value six orders of magnitude smaller than the outer-scaled defect.
    let operator = DenseOperator::new(DenseMatrix::identity(2)).unwrap();
    let preconditioner = UnevenPreconditioner {
        factors: vec![1.0e-6, 1.0e6],
    };
    let rhs = [1.0, 1.0];
    let scale = [1.0, 1.0];
    let config = GmresConfig {
        restart: 1,
        max_arnoldi: 4,
        rtol: 0.8,
        atol: 0.0,
    };
    let mut work = WorkCounters::default();
    let report = solve_gmres_with_residual_scale(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &config,
        Some(&scale),
        &mut work,
    )
    .unwrap();

    let mut applied = vec![0.0; 2];
    operator.apply(&report.x, &mut applied).unwrap();
    let raw = rhs
        .iter()
        .zip(applied)
        .map(|(right, left)| right - left)
        .collect::<Vec<_>>();
    let raw_wrms = wrms(&raw, &scale).unwrap();
    let mut preconditioned = vec![0.0; 2];
    preconditioner.apply(&raw, &mut preconditioned).unwrap();
    let preconditioned_wrms = wrms(&preconditioned, &scale).unwrap();
    assert_eq!(report.residual_norm.to_bits(), raw_wrms.to_bits());
    assert!(raw_wrms <= 0.8);
    assert!(raw_wrms > 1.0e5 * preconditioned_wrms);
    assert!(work.preconditioner_apps > 0);
}
