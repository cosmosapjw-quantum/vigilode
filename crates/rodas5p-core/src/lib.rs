#![forbid(unsafe_code)]

mod coefficients;
mod error;
mod hash;
mod matrix;
mod matrix_functions;
mod norms;
mod operator;
mod solver_types;
mod work;

pub use coefficients::{
    CoefficientPrecisionAvailability, RODAS5P_COEFFICIENT_SNAPSHOT_SCHEMA_VERSION,
    Rodas5pCoefficientProvenance, Rodas5pCoefficients, load_rodas5p_coefficients,
};
pub use error::{CoreError, CoreResult};
pub use hash::sha256_hex;
pub use matrix::{DenseMatrix, LuFactorization, direct_solve, inverse};
pub use matrix_functions::{dense_fused_phi_action, dense_phi_action, matrix_exp_pade13};
pub use norms::{error_scale, safe_l2, wrms};
pub use operator::{
    ApplyCategory, ClosureOperator, DenseOperator, DirectPreconditioner, ExactDenseMatrixIdentity,
    ExactOperatorIdentity, ExactPreconditionerIdentity, IdentityPreconditioner,
    JacobiPreconditioner, KrylovSystemIdentity, LinearOperator, OperatorApplicationWork,
    Preconditioner, ShiftedOperator, apply_counted, apply_jvp_counted, apply_preconditioner,
    apply_rows_counted, exact_krylov_system_identity,
};
pub use solver_types::{
    InitialGuess, LinearMethod, LinearSolveReport, LinearSolverConfig, PreconditionerKind,
};
pub use work::WorkCounters;
