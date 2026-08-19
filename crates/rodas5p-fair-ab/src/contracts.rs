use std::collections::BTreeMap;

use rodas5p_core::{DenseMatrix, WorkCounters, direct_solve, safe_l2, sha256_hex};
use serde::{Deserialize, Serialize};

use crate::{FairError, FairResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolverKind {
    Gmres,
    Lgmres,
    Gcrodr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreconditionerKind {
    None,
    Jacobi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecycleLifetime {
    Off,
    Stage,
    Persistent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SequenceKind {
    Fixed,
    SlowDrift,
    Abrupt,
    Rotating,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolveStatus {
    Converged,
    Failed,
    BudgetExhausted,
    NumericalFailure,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FairSolveConfig {
    pub solver: SolverKind,
    pub rtol: f64,
    pub atol: f64,
    pub restart: usize,
    pub recycle_dim: usize,
    pub hard_operator_budget: u64,
    pub preconditioner: PreconditionerKind,
    pub use_previous_oracle_guess: bool,
}

impl FairSolveConfig {
    pub fn validate(&self) -> FairResult<()> {
        if self.rtol < 0.0 || self.atol < 0.0 {
            return Err(FairError::Invalid(
                "linear tolerances must be nonnegative".into(),
            ));
        }
        if self.restart < 2 {
            return Err(FairError::Invalid("restart must be at least two".into()));
        }
        if self.recycle_dim == 0 || self.recycle_dim >= self.restart {
            return Err(FairError::Invalid(
                "recycle_dim must lie in [1,restart)".into(),
            ));
        }
        if self.hard_operator_budget == 0 {
            return Err(FairError::Invalid(
                "hard_operator_budget must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkLedger {
    pub operator_krylov: u64,
    pub operator_refresh: u64,
    pub operator_diagnostic: u64,
    pub preconditioner_setup: u64,
    pub preconditioner_apply: u64,
    pub recycle_updates: u64,
    pub recycle_resets: u64,
    pub budget_exhaustions: u64,
    pub linear_iterations: u64,
    pub orthogonalization_inner_products: u64,
    pub orthogonalization_vector_updates: u64,
    pub harmonic_ritz_solves: u64,
}

impl WorkLedger {
    pub fn operator_total(self) -> u64 {
        self.operator_krylov + self.operator_refresh + self.operator_diagnostic
    }

    pub fn add_assign(&mut self, other: Self) {
        self.operator_krylov += other.operator_krylov;
        self.operator_refresh += other.operator_refresh;
        self.operator_diagnostic += other.operator_diagnostic;
        self.preconditioner_setup += other.preconditioner_setup;
        self.preconditioner_apply += other.preconditioner_apply;
        self.recycle_updates += other.recycle_updates;
        self.recycle_resets += other.recycle_resets;
        self.budget_exhaustions += other.budget_exhaustions;
        self.linear_iterations += other.linear_iterations;
        self.orthogonalization_inner_products += other.orthogonalization_inner_products;
        self.orthogonalization_vector_updates += other.orthogonalization_vector_updates;
        self.harmonic_ritz_solves += other.harmonic_ritz_solves;
    }

    pub fn from_counters(counters: WorkCounters) -> Self {
        Self {
            operator_krylov: counters.linear_matvecs,
            operator_refresh: counters.recycle_refresh_matvecs,
            operator_diagnostic: counters.diagnostic_matvecs,
            preconditioner_setup: 0,
            preconditioner_apply: counters.preconditioner_apps,
            recycle_updates: counters.recycle_updates,
            recycle_resets: 0,
            budget_exhaustions: 0,
            linear_iterations: counters.linear_iterations,
            orthogonalization_inner_products: counters.orthogonalization_inner_products,
            orthogonalization_vector_updates: counters.orthogonalization_vector_updates,
            harmonic_ritz_solves: counters.harmonic_ritz_solves,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimingLedger {
    pub setup_seconds: f64,
    pub solve_seconds: f64,
    pub residual_seconds: f64,
    pub total_seconds: f64,
}

impl TimingLedger {
    pub fn add_assign(&mut self, other: Self) {
        self.setup_seconds += other.setup_seconds;
        self.solve_seconds += other.solve_seconds;
        self.residual_seconds += other.residual_seconds;
        self.total_seconds += other.total_seconds;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResidualCertificate {
    pub passed: bool,
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub threshold: f64,
}

#[derive(Clone, Debug)]
pub struct LinearSystemCase {
    pub matrix: DenseMatrix,
    pub rhs: Vec<f64>,
    pub oracle_solution: Vec<f64>,
    pub operator_id: String,
    pub system_id: String,
    pub step_index: usize,
    pub stage_index: usize,
    pub metadata: BTreeMap<String, String>,
}

fn append_u64(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&(value as u64).to_le_bytes());
}

pub(crate) fn matrix_digest(matrix: &DenseMatrix) -> String {
    let mut bytes = b"dense-f64-le".to_vec();
    append_u64(&mut bytes, matrix.nrows());
    append_u64(&mut bytes, matrix.ncols());
    for value in matrix.as_slice() {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    sha256_hex(&bytes)
}

pub(crate) fn system_digest(
    operator_id: &str,
    rhs: &[f64],
    step_index: usize,
    stage_index: usize,
) -> String {
    let mut bytes = operator_id.as_bytes().to_vec();
    for value in rhs {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    append_u64(&mut bytes, step_index);
    append_u64(&mut bytes, stage_index);
    sha256_hex(&bytes)
}

impl LinearSystemCase {
    pub fn from_matrix(
        matrix: DenseMatrix,
        rhs: Vec<f64>,
        step_index: usize,
        stage_index: usize,
    ) -> FairResult<Self> {
        if matrix.nrows() != matrix.ncols() || rhs.len() != matrix.nrows() {
            return Err(FairError::Invalid("linear system shape mismatch".into()));
        }
        if !rhs.iter().all(|value| value.is_finite()) {
            return Err(FairError::Invalid("linear system RHS is non-finite".into()));
        }
        let oracle_solution = direct_solve(&matrix, &rhs)?;
        let operator_id = matrix_digest(&matrix);
        let system_id = system_digest(&operator_id, &rhs, step_index, stage_index);
        Ok(Self {
            matrix,
            rhs,
            oracle_solution,
            operator_id,
            system_id,
            step_index,
            stage_index,
            metadata: BTreeMap::new(),
        })
    }

    pub fn dimension(&self) -> usize {
        self.rhs.len()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FairSolveResult {
    pub solver: SolverKind,
    pub status: SolveStatus,
    pub solution: Vec<f64>,
    pub certificate: ResidualCertificate,
    pub ledger: WorkLedger,
    pub timing: TimingLedger,
    pub system_id: String,
    pub operator_id: String,
    pub relative_solution_error: f64,
    pub iterations: u64,
    pub message: String,
}

pub(crate) fn relative_solution_error(solution: &[f64], oracle: &[f64]) -> f64 {
    let difference_norm = solution
        .iter()
        .zip(oracle)
        .fold(0.0_f64, |norm, (left, right)| norm.hypot(left - right));
    difference_norm / safe_l2(oracle).max(f64::MIN_POSITIVE)
}
