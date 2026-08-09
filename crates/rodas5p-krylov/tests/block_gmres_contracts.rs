use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{CoreResult, IdentityPreconditioner, LinearOperator, WorkCounters};
use rodas5p_krylov::{BlockGmresConfig, solve_block_gmres};

struct DiagonalBatchOperator {
    diagonal: Vec<f64>,
    block_calls: AtomicUsize,
    scalar_calls: AtomicUsize,
}

impl DiagonalBatchOperator {
    fn new(diagonal: Vec<f64>) -> Self {
        Self {
            diagonal,
            block_calls: AtomicUsize::new(0),
            scalar_calls: AtomicUsize::new(0),
        }
    }
}

impl LinearOperator for DiagonalBatchOperator {
    fn dimension(&self) -> usize {
        self.diagonal.len()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.scalar_calls.fetch_add(1, Ordering::Relaxed);
        for ((out, &value), &diagonal) in y.iter_mut().zip(x).zip(&self.diagonal) {
            *out = diagonal * value;
        }
        Ok(())
    }

    fn apply_rows(&self, inputs: &[Vec<f64>], outputs: &mut [Vec<f64>]) -> CoreResult<()> {
        self.block_calls.fetch_add(1, Ordering::Relaxed);
        for (input, output) in inputs.iter().zip(outputs) {
            for ((out, &value), &diagonal) in output.iter_mut().zip(input).zip(&self.diagonal) {
                *out = diagonal * value;
            }
        }
        Ok(())
    }

    fn token(&self) -> u64 {
        0xB10C
    }
}

#[test]
fn block_gmres_solves_multiple_rhs_through_block_operator_calls() {
    let operator = DiagonalBatchOperator::new(vec![2.0, 3.0, 5.0, 7.0]);
    let pc = IdentityPreconditioner::new(4);
    let rhs = vec![
        vec![2.0, 3.0, 5.0, 7.0],
        vec![4.0, -6.0, 10.0, -14.0],
        vec![1.0, 0.0, 5.0, 0.0],
    ];
    let mut counters = WorkCounters::default();
    let report = solve_block_gmres(
        &operator,
        &pc,
        &rhs,
        &BlockGmresConfig {
            max_basis: 16,
            rtol: 1.0e-12,
            atol: 1.0e-14,
            rank_tolerance: 1.0e-13,
        },
        &mut counters,
    )
    .unwrap();

    assert!(report.converged);
    assert!(report.maximum_relative_residual < 1.0e-12);
    assert_eq!(report.solutions.len(), rhs.len());
    let expected = [
        vec![1.0, 1.0, 1.0, 1.0],
        vec![2.0, -2.0, 2.0, -2.0],
        vec![0.5, 0.0, 1.0, 0.0],
    ];
    for (actual, target) in report.solutions.iter().zip(expected) {
        assert!(
            actual
                .iter()
                .zip(target)
                .all(|(value, expected)| (value - expected).abs() < 1.0e-12)
        );
    }
    assert!(operator.block_calls.load(Ordering::Relaxed) > 0);
    assert!(counters.block_matvecs > 0);
    assert!(counters.linear_matvecs >= counters.block_matvecs);
}

#[test]
fn block_gmres_deflates_linearly_dependent_rhs() {
    let operator = DiagonalBatchOperator::new(vec![1.5, 2.0, 2.5]);
    let pc = IdentityPreconditioner::new(3);
    let rhs = vec![
        vec![1.0, 2.0, 3.0],
        vec![2.0, 4.0, 6.0],
        vec![-1.0, -2.0, -3.0],
    ];
    let mut counters = WorkCounters::default();
    let report = solve_block_gmres(
        &operator,
        &pc,
        &rhs,
        &BlockGmresConfig::default(),
        &mut counters,
    )
    .unwrap();

    assert!(report.converged);
    assert_eq!(report.initial_block_rank, 1);
    assert!(report.maximum_relative_residual < 1.0e-10);
}

#[test]
fn block_gmres_rejects_ragged_rhs_without_counting_a_solve() {
    let operator = DiagonalBatchOperator::new(vec![1.0, 2.0]);
    let pc = IdentityPreconditioner::new(2);
    let rhs = vec![vec![1.0, 2.0], vec![3.0]];
    let mut counters = WorkCounters::default();
    let error = solve_block_gmres(
        &operator,
        &pc,
        &rhs,
        &BlockGmresConfig::default(),
        &mut counters,
    )
    .unwrap_err();
    assert!(format!("{error}").contains("RHS"));
    assert_eq!(counters.linear_solves, 0);
}

#[test]
fn seeded_shared_basis_refines_all_rhs_to_the_same_true_residual_contract() {
    use rodas5p_krylov::{SeededGmresConfig, solve_seeded_gmres};

    let operator = DiagonalBatchOperator::new(vec![1.0, 2.0, 4.0, 8.0, 16.0]);
    let pc = IdentityPreconditioner::new(5);
    let rhs = vec![
        vec![1.0, 1.0, 1.0, 1.0, 1.0],
        vec![0.0, 2.0, 0.0, 4.0, 0.0],
        vec![3.0, -1.0, 2.0, 0.5, -4.0],
    ];
    let mut counters = WorkCounters::default();
    let report = solve_seeded_gmres(
        &operator,
        &pc,
        &rhs,
        &SeededGmresConfig {
            shared_basis: 3,
            restart: 5,
            max_arnoldi: 20,
            rtol: 1.0e-12,
            atol: 1.0e-14,
            rank_tolerance: 1.0e-13,
        },
        &mut counters,
    )
    .unwrap();

    assert!(report.converged);
    assert_eq!(report.initial_block_rank, 1);
    assert_eq!(report.method, "seeded-shared-gmres");
    assert!(report.maximum_relative_residual < 1.0e-12);
    assert!(counters.linear_solves >= rhs.len() as u64);
}

#[test]
fn exhausted_block_gmres_retains_attempted_work_counters() {
    let operator = DiagonalBatchOperator::new(vec![1.0, 2.0, 4.0, 8.0]);
    let pc = IdentityPreconditioner::new(4);
    let rhs = vec![vec![1.0, 1.0, 1.0, 1.0]];
    let mut counters = WorkCounters::default();
    let error = solve_block_gmres(
        &operator,
        &pc,
        &rhs,
        &BlockGmresConfig {
            max_basis: 1,
            rtol: 1.0e-14,
            atol: 1.0e-16,
            rank_tolerance: 1.0e-13,
        },
        &mut counters,
    )
    .unwrap_err();

    assert!(format!("{error}").contains("exhausted"));
    assert!(counters.linear_matvecs > 0);
    assert!(counters.diagnostic_matvecs > 0);
    assert!(counters.block_matvecs > 0);
    assert!(operator.block_calls.load(Ordering::Relaxed) > 0);
}

#[test]
fn block_gmres_does_not_false_converge_on_small_nonzero_rhs() {
    let operator = DiagonalBatchOperator::new(vec![1.0, 2.0]);
    let pc = IdentityPreconditioner::new(2);
    let rhs = vec![vec![1.0e-14, 0.0]];
    let mut counters = WorkCounters::default();
    let report = solve_block_gmres(
        &operator,
        &pc,
        &rhs,
        &BlockGmresConfig {
            max_basis: 4,
            rtol: 1.0e-12,
            atol: 0.0,
            rank_tolerance: 1.0e-12,
        },
        &mut counters,
    )
    .unwrap();

    assert!(report.maximum_relative_residual <= 1.0e-12);
    assert!((report.solutions[0][0] - 1.0e-14).abs() <= 1.0e-28);
}
