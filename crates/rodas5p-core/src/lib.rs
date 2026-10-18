#![forbid(unsafe_code)]

mod binary_scaling;
mod coefficients;
pub mod complex_shift_jet;
pub mod damped_moments;
pub mod directed;
mod error;
mod hash;
pub mod laguerre_adjoint;
pub mod leja_action;
pub mod leja_setup_work;
mod matrix;
mod matrix_functions;
pub mod nonnormal_certificate;
mod norms;
mod operator;
pub mod polynomial_action;
pub mod residual_gain;
pub mod shared_shift_jet;
pub mod shared_shift_policy;
mod solver_types;
pub mod sparse_direct;
pub mod taylor_phi;
pub mod taylor_phi_total;
pub mod transform_bound;
mod work;

pub use binary_scaling::{
    binary_power, binary_scale, binary_split, times_power, weight_phi_vectors,
};
pub use coefficients::{
    CoefficientPrecisionAvailability, RODAS5P_COEFFICIENT_SNAPSHOT_SCHEMA_VERSION,
    RODAS5P_COEFFICIENT_SNAPSHOT_SHA256, Rodas5pCoefficientProvenance, Rodas5pCoefficients,
    load_rodas5p_coefficients, rodas5p_coefficients,
};
pub use damped_moments::{
    DampedMomentTable, DampedMomentsConfig, compute_damped_moment, compute_normalized_damped_moment,
};
pub use directed::{
    PowerOfTwo, scale_pow2_down, scale_pow2_is_exact, scale_pow2_nearest,
    scale_pow2_nearest_counted, scale_pow2_up,
};
pub use error::{CoreError, CoreResult};
pub use hash::{sha256_f64_slice_bytes, sha256_hex};
pub use leja_action::{
    CertifiedLejaCoeffCache, CertifiedLejaReport, CertifiedLejaWorkspace, LEJA_CERTIFIED_SCHEMA,
    LEJA_DEGREE_CAP, LEJA_RANGE_LIMIT, LEJA_SCHEMA, LEJA_TOTAL_NOT_CERTIFIED, LejaReport,
    leja_divided_differences, leja_divided_differences_certified, leja_node_bounds,
    leja_node_bounds_with_setup_work, leja_node_polynomial_bound, leja_phi_action,
    leja_phi_action_capped, leja_phi_action_certified, leja_phi_action_certified_capped,
    leja_phi_action_certified_with_cache, leja_phi_action_certified_with_cache_and_workspace,
    leja_phi_action_certified_with_setup_work, leja_points, leja_points_with_setup_work,
    leja_suffix_bounds_with_setup_work,
};
pub use leja_setup_work::{
    CoefficientCacheObservation, LEJA_SETUP_RECEIPT_SCHEMA_V1, LejaSetupReceiptV1, LejaSetupWork,
    LejaSetupWorkSubtotal, OnceLockObservation, SetupCoverage,
};
pub use matrix::{DenseMatrix, LuFactorization, direct_solve, inverse};
pub use matrix_functions::{
    DensePhiCombinationReport, dense_fused_phi_action, dense_fused_phi_action_report,
    dense_phi_action, dense_phi_combination, dense_phi_combination_report, matrix_exp_pade13,
};
pub use norms::{error_scale, safe_l2, wrms};
pub use operator::{
    ApplyCategory, ClosureOperator, DenseOperator, DirectPreconditioner, ExactDenseMatrixIdentity,
    ExactOperatorIdentity, ExactPreconditionerIdentity, IdentityPreconditioner,
    JacobiPreconditioner, KrylovSystemIdentity, LinearOperator, OperatorApplicationWork,
    Preconditioner, PreconditionerSide, ShiftedOperator, apply_counted, apply_jvp_counted,
    apply_preconditioner, apply_rows_counted, exact_krylov_system_identity,
};
pub use polynomial_action::{
    ActionPlan, BasisPlanBranch, EnclosureIdentity, LAGUERRE_COMPONENTS_UNSUPPORTED,
    LaguerreAdjointColumnExport, LaguerreAssemblyExport, LaguerreColumnTermExport,
    LaguerreComponentExport, LaguerreExportIdentity, LocalBasisPlan, RouteCost, RoutePolicy,
    RouterCaches, SingleRoutedPhi, count_chebyshev_series_terms, count_laguerre_series_terms,
    execute_joint_phi, execute_joint_phi_with_cost, joint_phi_action_laguerre_components,
    plan_joint_phi, plan_joint_phi_local, route_joint_phi_single, scale_bound_up,
};
pub use residual_gain::{
    AssemblyUncertainty, DeclaredStructure, EstimateSource, EstimatedCorrection, FrozenMetric,
    GainEvidence, LinearTargetId, OperatorKind, ResidualGainWitness, VerifiedCorrection,
    WitnessRejection, dense_lu_comparator, projected_nu_estimate,
};
pub use solver_types::{
    InitialGuess, LinearMethod, LinearSolveReport, LinearSolverConfig, PreconditionerKind,
};
pub use sparse_direct::SparseDirectSolver;
pub use taylor_phi_total::{
    FUSED_PHI_RANGE_UNSUPPORTED, NORMALIZED_FUSED_PHI_SCHEMA, NormalizedFusedPhiCertificate,
    NormalizedFusedPhiWork, certify_fused_phi_total_normalized,
};
pub use work::WorkCounters;
