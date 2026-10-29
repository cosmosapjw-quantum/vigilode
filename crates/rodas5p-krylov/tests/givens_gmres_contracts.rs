use rodas5p_core::{
    ClosureOperator, CoreError, DenseMatrix, DenseOperator, IdentityPreconditioner,
    JacobiPreconditioner, Preconditioner, WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GmresConfig, GmresGivensWorkspace, solve_gmres, solve_gmres_givens,
    solve_gmres_givens_with_workspace,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn nonnormal_shifted_operator(
    n: usize,
    upper_coupling: f64,
) -> ClosureOperator<impl Fn(&[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync> {
    ClosureOperator::new(n, move |x, y| {
        for i in 0..n {
            let diagonal = 1.0 + 0.02 * (i + 1) as f64;
            let upper = if i + 1 < n {
                -upper_coupling * x[i + 1]
            } else {
                0.0
            };
            let lower = if i > 0 { 0.03 * x[i - 1] } else { 0.0 };
            y[i] = diagonal * x[i] + upper + lower;
        }
        Ok(())
    })
}

fn rhs(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| ((i + 1) as f64 * 0.17).sin() + 0.2)
        .collect()
}

fn relative_defect(left: &[f64], right: &[f64]) -> f64 {
    let difference: Vec<f64> = left.iter().zip(right).map(|(a, b)| a - b).collect();
    safe_l2(&difference) / safe_l2(right).max(1.0)
}

struct AnisotropicPreconditioner {
    scales: [f64; 2],
}

struct DiagonalPreconditioner {
    scales: Vec<f64>,
}

impl Preconditioner for DiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.scales.len()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> rodas5p_core::CoreResult<()> {
        if x.len() != self.scales.len() || y.len() != self.scales.len() {
            return Err(CoreError::Dimension(
                "diagonal preconditioner shape mismatch".into(),
            ));
        }
        for ((output, input), scale) in y.iter_mut().zip(x).zip(&self.scales) {
            *output = scale * input;
        }
        Ok(())
    }
}

#[test]
fn nearly_dependent_nonidentity_system_keeps_uniform_scale_parity() {
    let solve = |alpha: f64| {
        let matrix =
            DenseMatrix::from_rows(&[&[alpha, 0.0], &[0.0, alpha * (1.0 + 1.0e-8)]]).unwrap();
        let operator = DenseOperator::new(matrix.clone()).unwrap();
        let preconditioner = IdentityPreconditioner::new(2);
        let exact = [1.0, -1.0];
        let rhs = matrix.matvec(&exact).unwrap();
        let config = GmresConfig {
            restart: 2,
            max_arnoldi: 4,
            rtol: 1.0e-12,
            atol: 0.0,
        };

        let mut legacy_work = WorkCounters::default();
        let legacy = solve_gmres(
            &operator,
            &preconditioner,
            &rhs,
            None,
            &config,
            &mut legacy_work,
        )
        .expect("legacy scale-parity solve");
        let mut givens_work = WorkCounters::default();
        let mut workspace = GmresGivensWorkspace::default();
        let givens = solve_gmres_givens_with_workspace(
            &operator,
            &preconditioner,
            &rhs,
            None,
            &config,
            &mut workspace,
            &mut givens_work,
        )
        .expect("Givens scale-parity solve");
        (legacy, givens, workspace, rhs, config, exact)
    };

    let normal = solve(1.0);
    let tiny = solve(1.0e-15);
    for (legacy, givens, _workspace, rhs, config, exact) in [&normal, &tiny] {
        let threshold = config.atol.max(config.rtol * safe_l2(rhs));
        assert!(legacy.residual_norm <= threshold);
        assert!(givens.residual_norm <= threshold);
        assert!(relative_defect(&legacy.x, exact) <= 2.0e-8);
        assert!(relative_defect(&givens.x, exact) <= 2.0e-8);
        assert_eq!(legacy.iterations, 2);
        assert_eq!(givens.iterations, 2);
    }
    assert_eq!(normal.0.iterations, tiny.0.iterations);
    assert_eq!(normal.1.iterations, tiny.1.iterations);
    assert_eq!(normal.2.statistics().restart_cycles, 1);
    assert_eq!(tiny.2.statistics().restart_cycles, 1);
}

#[test]
fn happy_breakdown_rejection_is_fail_closed_without_rebuilding_candidate() {
    let calls = AtomicUsize::new(0);
    let operator = rodas5p_core::ClosureOperator::new(2, move |x, y| {
        let multiplier = if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            1.0
        } else {
            2.0
        };
        for (output, input) in y.iter_mut().zip(x) {
            *output = multiplier * input;
        }
        Ok(())
    });
    let preconditioner = IdentityPreconditioner::new(2);
    let config = GmresConfig {
        restart: 2,
        max_arnoldi: 4,
        rtol: 1.0e-12,
        atol: 0.0,
    };
    let mut workspace = GmresGivensWorkspace::default();
    let mut work = WorkCounters::default();
    let result = solve_gmres_givens_with_workspace(
        &operator,
        &preconditioner,
        &[1.0, 0.0],
        None,
        &config,
        &mut workspace,
        &mut work,
    );

    assert!(matches!(result, Err(CoreError::LinearSolve(_))));
    let statistics = workspace.statistics();
    assert_eq!(statistics.restart_cycles, 1);
    assert_eq!(statistics.projected_residual_checks, 1);
    assert_eq!(statistics.rejected_projected_residual_checks, 1);
    assert_eq!(statistics.triangular_solves, 1);
    assert_eq!(work.linear_iterations, 1);
    assert_eq!(work.linear_solves, 0);
}

#[test]
fn rejected_projected_triggers_back_off_diagnostics_but_stay_certified() {
    let matrix = DenseMatrix::from_rows(&[
        &[1.0, 0.0, 0.0, 0.0],
        &[0.0, 2.0, 0.0, 0.0],
        &[0.0, 0.0, 3.0, 0.0],
        &[0.0, 0.0, 0.0, 4.0],
    ])
    .unwrap();
    let operator = DenseOperator::new(matrix).unwrap();
    let preconditioner = DiagonalPreconditioner {
        scales: vec![1.0, 1.0e-6, 1.0e-6, 1.0e-6],
    };
    let rhs = [1.0, 1.0, 1.0, 1.0];
    let config = GmresConfig {
        restart: 4,
        max_arnoldi: 4,
        rtol: 1.0e-3,
        atol: 0.0,
    };
    let mut workspace = GmresGivensWorkspace::default();
    let mut work = WorkCounters::default();
    let report = solve_gmres_givens_with_workspace(
        &operator,
        &preconditioner,
        &rhs,
        None,
        &config,
        &mut workspace,
        &mut work,
    )
    .expect("backed-off but certified candidate");

    let threshold = config.atol.max(config.rtol * safe_l2(&rhs));
    let statistics = workspace.statistics();
    println!(
        "iterations={} projected_checks={} rejected_checks={} triangular_solves={} diagnostic_matvecs={}",
        report.iterations,
        statistics.projected_residual_checks,
        statistics.rejected_projected_residual_checks,
        statistics.triangular_solves,
        work.diagnostic_matvecs,
    );
    assert!(statistics.rejected_projected_residual_checks >= 2);
    assert!(statistics.projected_residual_checks < report.iterations);
    assert!(report.residual_norm <= threshold);
    assert_eq!(report.iterations, 4);
}

impl Preconditioner for AnisotropicPreconditioner {
    fn dimension(&self) -> usize {
        2
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> rodas5p_core::CoreResult<()> {
        if x.len() != 2 || y.len() != 2 {
            return Err(CoreError::Dimension(
                "anisotropic preconditioner shape mismatch".into(),
            ));
        }
        y[0] = self.scales[0] * x[0];
        y[1] = self.scales[1] * x[1];
        Ok(())
    }
}

#[test]
fn candidate_stops_inside_restart_and_preserves_true_residual_authority() {
    let n = 48;
    let operator = nonnormal_shifted_operator(n, 0.18);
    let preconditioner = IdentityPreconditioner::new(n);
    let right_hand_side = rhs(n);
    let config = GmresConfig {
        restart: 32,
        max_arnoldi: 64,
        rtol: 1.0e-6,
        atol: 1.0e-12,
    };

    let mut legacy_work = WorkCounters::default();
    let legacy = solve_gmres(
        &operator,
        &preconditioner,
        &right_hand_side,
        None,
        &config,
        &mut legacy_work,
    )
    .expect("legacy GMRES");

    let mut candidate_work = WorkCounters::default();
    let mut workspace = GmresGivensWorkspace::default();
    let candidate = solve_gmres_givens_with_workspace(
        &operator,
        &preconditioner,
        &right_hand_side,
        None,
        &config,
        &mut workspace,
        &mut candidate_work,
    )
    .expect("Givens GMRES candidate");

    println!(
        "legacy_iterations={} candidate_iterations={} legacy_linear_matvecs={} candidate_linear_matvecs={} candidate_diagnostic_matvecs={}",
        legacy.iterations,
        candidate.iterations,
        legacy_work.linear_matvecs,
        candidate_work.linear_matvecs,
        candidate_work.diagnostic_matvecs,
    );

    let threshold = config.atol.max(config.rtol * safe_l2(&right_hand_side));
    assert!(candidate.iterations < config.restart as u64);
    assert!(candidate.iterations < legacy.iterations);
    assert!(candidate.residual_norm <= threshold);
    assert!(candidate_work.linear_matvecs < legacy_work.linear_matvecs);
    assert!(candidate_work.diagnostic_matvecs >= 2);
    assert_eq!(workspace.statistics().rejected_projected_residual_checks, 0);
    assert!(relative_defect(&candidate.x, &legacy.x) <= 3.0e-6);
}

#[test]
fn candidate_resets_incremental_state_across_restart_cycles() {
    let n = 48;
    let operator = nonnormal_shifted_operator(n, 0.5);
    let preconditioner = IdentityPreconditioner::new(n);
    let right_hand_side = rhs(n);
    let config = GmresConfig {
        restart: 4,
        max_arnoldi: 96,
        rtol: 1.0e-9,
        atol: 1.0e-12,
    };

    let mut workspace = GmresGivensWorkspace::default();
    let mut work = WorkCounters::default();
    let report = solve_gmres_givens_with_workspace(
        &operator,
        &preconditioner,
        &right_hand_side,
        None,
        &config,
        &mut workspace,
        &mut work,
    )
    .expect("restarted Givens GMRES");

    let threshold = config.atol.max(config.rtol * safe_l2(&right_hand_side));
    assert!(workspace.statistics().restart_cycles > 1);
    assert!(report.iterations > config.restart as u64);
    assert!(report.residual_norm <= threshold);
    assert_eq!(report.iterations, work.linear_iterations);
}

#[test]
fn projected_residual_trigger_cannot_bypass_true_residual_certification() {
    let matrix = DenseMatrix::from_rows(&[&[1.0, 0.0], &[0.0, 2.0]]).unwrap();
    let operator = DenseOperator::new(matrix).unwrap();
    let preconditioner = AnisotropicPreconditioner {
        scales: [1.0, 1.0e-6],
    };
    let right_hand_side = [1.0, 1.0];
    let config = GmresConfig {
        restart: 2,
        max_arnoldi: 4,
        rtol: 1.0e-3,
        atol: 0.0,
    };
    let mut workspace = GmresGivensWorkspace::default();
    let mut work = WorkCounters::default();
    let report = solve_gmres_givens_with_workspace(
        &operator,
        &preconditioner,
        &right_hand_side,
        None,
        &config,
        &mut workspace,
        &mut work,
    )
    .expect("true-residual-certified candidate");

    let threshold = config.atol.max(config.rtol * safe_l2(&right_hand_side));
    assert!(workspace.statistics().projected_residual_checks >= 2);
    assert!(workspace.statistics().rejected_projected_residual_checks >= 1);
    assert!(report.residual_norm <= threshold);
    assert_eq!(report.iterations, 2);
    assert!(work.diagnostic_matvecs >= 3);
}

#[test]
fn candidate_preserves_the_jacobi_preconditioner_contract() {
    let matrix =
        DenseMatrix::from_rows(&[&[10.0, 2.0, 0.0], &[1.0, 7.0, 1.0], &[0.0, 1.0, 5.0]]).unwrap();
    let exact = vec![1.0, 2.0, -1.0];
    let right_hand_side = matrix.matvec(&exact).unwrap();
    let operator = DenseOperator::new(matrix.clone()).unwrap();
    let preconditioner = JacobiPreconditioner::from_matrix(&matrix).unwrap();
    let config = GmresConfig {
        restart: 3,
        max_arnoldi: 20,
        rtol: 1.0e-12,
        atol: 1.0e-14,
    };
    let mut work = WorkCounters::default();
    let report = solve_gmres_givens(
        &operator,
        &preconditioner,
        &right_hand_side,
        None,
        &config,
        &mut work,
    )
    .expect("preconditioned Givens GMRES");

    assert!(relative_defect(&report.x, &exact) <= 1.0e-10);
    assert!(report.residual_norm <= 1.0e-11);
    assert!(work.preconditioner_apps > 0);
    assert!(work.diagnostic_matvecs >= 1);
}

#[test]
fn candidate_rejects_non_finite_tolerances_before_operator_work() {
    let operator = DenseOperator::new(DenseMatrix::identity(2)).unwrap();
    let preconditioner = IdentityPreconditioner::new(2);
    let mut work = WorkCounters::default();
    let result = solve_gmres_givens(
        &operator,
        &preconditioner,
        &[1.0, -1.0],
        None,
        &GmresConfig {
            restart: 2,
            max_arnoldi: 4,
            rtol: f64::NAN,
            atol: 0.0,
        },
        &mut work,
    );
    assert!(result.is_err());
    assert_eq!(work.linear_matvecs, 0);
    assert_eq!(work.diagnostic_matvecs, 0);
}
