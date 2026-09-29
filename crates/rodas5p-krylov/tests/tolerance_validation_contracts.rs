//! WU-2 (audit F-034, F-037): Krylov inputs that cannot define a stopping
//! threshold must fail closed before any operator work, and a degenerate
//! seeded solve must certify its true residual instead of assuming it.

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, ExactPreconditionerIdentity,
    IdentityPreconditioner, Preconditioner, WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, LgmresConfig, LgmresState, SeededGmresConfig,
    solve_gcrodr, solve_gmres, solve_gmres_givens, solve_lgmres, solve_seeded_gmres,
};

const BAD_TOLERANCES: [f64; 4] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0];

fn diagonal_system() -> (DenseOperator, IdentityPreconditioner, Vec<f64>) {
    let mut matrix = DenseMatrix::zeros(4, 4);
    for (i, value) in [2.0, 3.0, 5.0, 7.0].into_iter().enumerate() {
        matrix[(i, i)] = value;
    }
    (
        DenseOperator::new(matrix).unwrap(),
        IdentityPreconditioner::new(4),
        vec![1.0, -2.0, 0.5, 4.0],
    )
}

/// Every (rtol, atol) pair with exactly one invalid entry and one valid entry.
fn invalid_pairs() -> Vec<(f64, f64)> {
    let mut pairs = Vec::new();
    for bad in BAD_TOLERANCES {
        pairs.push((bad, 1.0e-12));
        pairs.push((1.0e-10, bad));
    }
    pairs
}

fn assert_invalid_input_without_work(
    kernel: &str,
    rtol: f64,
    atol: f64,
    result: CoreResult<rodas5p_core::LinearSolveReport>,
    work: &WorkCounters,
) {
    assert!(
        matches!(result, Err(CoreError::InvalidInput(_))),
        "{kernel} rtol={rtol:e} atol={atol:e}: expected InvalidInput, got {result:?}"
    );
    assert_eq!(
        *work,
        WorkCounters::default(),
        "{kernel} rtol={rtol:e} atol={atol:e}: rejected input must not charge work"
    );
}

#[test]
fn scalar_kernels_reject_non_finite_or_negative_tolerances_before_work() {
    let (op, pc, rhs) = diagonal_system();
    for (rtol, atol) in invalid_pairs() {
        let gmres = GmresConfig {
            restart: 4,
            max_arnoldi: 16,
            rtol,
            atol,
        };
        let mut work = WorkCounters::default();
        let result = solve_gmres(&op, &pc, &rhs, None, &gmres, &mut work);
        assert_invalid_input_without_work("gmres", rtol, atol, result, &work);

        let mut work = WorkCounters::default();
        let result = solve_gmres_givens(&op, &pc, &rhs, None, &gmres, &mut work);
        assert_invalid_input_without_work("gmres-givens", rtol, atol, result, &work);

        let lgmres = LgmresConfig {
            inner_m: 4,
            max_outer: 4,
            outer_k: 2,
            rtol,
            atol,
        };
        let mut state = LgmresState::default();
        let snapshot = state.clone();
        let mut work = WorkCounters::default();
        let result = solve_lgmres(&op, &pc, &rhs, None, &lgmres, &mut state, &mut work);
        assert_invalid_input_without_work("lgmres", rtol, atol, result, &work);
        assert_eq!(state, snapshot);

        let gcrodr = GcrodrConfig {
            restart: 4,
            max_arnoldi: 16,
            recycle_dim: 2,
            rank_tol: 1.0e-12,
            rtol,
            atol,
        };
        let mut state = GcrodrState::default();
        let snapshot = state.clone();
        let mut work = WorkCounters::default();
        let result = solve_gcrodr(&op, &pc, &rhs, None, &gcrodr, &mut state, &mut work);
        assert_invalid_input_without_work("gcrodr", rtol, atol, result, &work);
        assert_eq!(state, snapshot);
    }
}

#[test]
fn infinite_absolute_tolerance_does_not_certify_the_zero_vector() {
    // F-034: atol = +Inf made max(atol, rtol ||b||) infinite, so x = 0 was
    // returned as converged for a nonzero right-hand side.
    let (op, pc, rhs) = diagonal_system();
    let config = GmresConfig {
        restart: 4,
        max_arnoldi: 16,
        rtol: 1.0e-10,
        atol: f64::INFINITY,
    };
    let mut work = WorkCounters::default();
    let result = solve_gmres(&op, &pc, &rhs, None, &config, &mut work);
    assert!(
        !matches!(&result, Ok(report) if report.x.iter().all(|value| *value == 0.0)),
        "x = 0 certified for a nonzero RHS: {result:?}"
    );
    assert!(matches!(result, Err(CoreError::InvalidInput(_))));
}

#[test]
fn overflowing_residual_threshold_is_rejected() {
    // Finite tolerances whose product with ||b|| overflows define no threshold.
    let (op, pc, rhs) = diagonal_system();
    let rhs: Vec<f64> = rhs.iter().map(|value| value * 1.0e10).collect();
    let gmres = GmresConfig {
        restart: 4,
        max_arnoldi: 16,
        rtol: 1.0e300,
        atol: 0.0,
    };
    let mut work = WorkCounters::default();
    let result = solve_gmres(&op, &pc, &rhs, None, &gmres, &mut work);
    assert!(
        matches!(result, Err(CoreError::InvalidInput(_))),
        "{result:?}"
    );

    let lgmres = LgmresConfig {
        inner_m: 4,
        max_outer: 4,
        outer_k: 2,
        rtol: 1.0e300,
        atol: 0.0,
    };
    let mut state = LgmresState::default();
    let result = solve_lgmres(&op, &pc, &rhs, None, &lgmres, &mut state, &mut work);
    assert!(
        matches!(result, Err(CoreError::InvalidInput(_))),
        "{result:?}"
    );

    let gcrodr = GcrodrConfig {
        restart: 4,
        max_arnoldi: 16,
        recycle_dim: 2,
        rank_tol: 1.0e-12,
        rtol: 1.0e300,
        atol: 0.0,
    };
    let mut state = GcrodrState::default();
    let result = solve_gcrodr(&op, &pc, &rhs, None, &gcrodr, &mut state, &mut work);
    assert!(
        matches!(result, Err(CoreError::InvalidInput(_))),
        "{result:?}"
    );
}

#[test]
fn zero_absolute_tolerance_with_finite_relative_tolerance_still_converges() {
    // Regression guard for the E-05 configuration (atol = 0).
    let (op, pc, rhs) = diagonal_system();
    let config = GmresConfig {
        restart: 4,
        max_arnoldi: 16,
        rtol: 1.0e-10,
        atol: 0.0,
    };
    let mut work = WorkCounters::default();
    let report = solve_gmres(&op, &pc, &rhs, None, &config, &mut work).unwrap();
    assert!(report.converged);
}

#[test]
fn gcrodr_rejects_a_rank_tolerance_outside_the_unit_interval() {
    let (op, pc, rhs) = diagonal_system();
    for rank_tol in [f64::NAN, 0.0, 1.0, -1.0e-12, f64::INFINITY] {
        let config = GcrodrConfig {
            restart: 4,
            max_arnoldi: 16,
            recycle_dim: 2,
            rank_tol,
            rtol: 1.0e-10,
            atol: 0.0,
        };
        let mut state = GcrodrState::default();
        let mut work = WorkCounters::default();
        let result = solve_gcrodr(&op, &pc, &rhs, None, &config, &mut state, &mut work);
        assert_invalid_input_without_work("gcrodr rank_tol", rank_tol, 0.0, result, &work);
    }
}

/// A valid but singular preconditioner that maps every vector to zero.
struct ZeroingPreconditioner(usize);

impl Preconditioner for ZeroingPreconditioner {
    fn dimension(&self) -> usize {
        self.0
    }
    fn apply(&self, _x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y.fill(0.0);
        Ok(())
    }
    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        None
    }
}

#[test]
fn seeded_gmres_rejects_a_preconditioner_that_annihilates_a_nonzero_rhs() {
    // F-037: the degenerate-seed branch returned converged = true with zero
    // residual norms, although x = 0 leaves the true residual equal to b.
    let (op, _, _) = diagonal_system();
    let pc = ZeroingPreconditioner(4);
    let rhs = vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 2.0, 0.0, 0.0]];
    let mut work = WorkCounters::default();
    let result = solve_seeded_gmres(&op, &pc, &rhs, &SeededGmresConfig::default(), &mut work);
    assert!(
        matches!(result, Err(CoreError::LinearSolve(_))),
        "{result:?}"
    );
    assert_eq!(work.linear_solves, 0);
    assert_eq!(work.block_linear_solves, 0);
}

#[test]
fn seeded_gmres_certifies_a_zero_rhs_block_with_true_residuals() {
    let (op, _, _) = diagonal_system();
    let pc = ZeroingPreconditioner(4);
    let rhs = vec![vec![0.0; 4], vec![0.0; 4]];
    let mut work = WorkCounters::default();
    let report =
        solve_seeded_gmres(&op, &pc, &rhs, &SeededGmresConfig::default(), &mut work).unwrap();
    assert!(report.converged);
    assert!(report.residual_norms.iter().all(|norm| *norm == 0.0));
    assert!(report.solutions.iter().flatten().all(|value| *value == 0.0));
}

#[test]
fn seeded_gmres_degenerate_seed_reports_true_norms_under_a_loose_atol() {
    let (op, _, _) = diagonal_system();
    let pc = ZeroingPreconditioner(4);
    let rhs = vec![vec![1.0e-3, 0.0, 0.0, 0.0], vec![0.0, 2.0e-3, 0.0, 0.0]];
    let config = SeededGmresConfig {
        atol: 1.0,
        ..SeededGmresConfig::default()
    };
    let mut work = WorkCounters::default();
    let report = solve_seeded_gmres(&op, &pc, &rhs, &config, &mut work).unwrap();
    assert!(report.converged);
    for (norm, row) in report.residual_norms.iter().zip(&rhs) {
        assert_eq!(norm.to_bits(), safe_l2(row).to_bits());
    }
    assert_eq!(report.maximum_residual_norm.to_bits(), 2.0e-3_f64.to_bits());
}
