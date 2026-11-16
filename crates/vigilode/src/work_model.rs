//! WorkModel v2: Instruction Reference (Ir) and polynomial linear-algebra overhead model.

use serde::{Deserialize, Serialize};

/// Fixed-point polynomial denominator for WorkModel v2 weights ($2^{16} = 65536$).
pub const WORK_MODEL_DENOMINATOR: u64 = 65536;

/// Fixed-point polynomial weight in system dimension `n`: $P(n) = a_0 + a_1 n + a_2 n^2 + a_3 n^3$.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolynomialWeight {
    /// Constant term $a_0$.
    pub a0: u64,
    /// Linear term coefficient $a_1$.
    pub a1: u64,
    /// Quadratic term coefficient $a_2$.
    pub a2: u64,
    /// Cubic term coefficient $a_3$.
    pub a3: u64,
}

impl PolynomialWeight {
    /// Construct a new polynomial weight.
    pub const fn new(a0: u64, a1: u64, a2: u64, a3: u64) -> Self {
        Self { a0, a1, a2, a3 }
    }

    /// Evaluate the polynomial in dimension `n` using checked `u128` arithmetic.
    pub fn eval(&self, n: usize) -> Option<u128> {
        let n = n as u128;
        let mut acc = self.a3 as u128;
        acc = acc.checked_mul(n)?.checked_add(self.a2 as u128)?;
        acc = acc.checked_mul(n)?.checked_add(self.a1 as u128)?;
        acc = acc.checked_mul(n)?.checked_add(self.a0 as u128)?;
        Some(acc)
    }
}

/// Right-hand side evaluation cost tier in Instruction Reference (Ir) units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RhsCost {
    /// Cheap right-hand side (~50 Ir).
    #[default]
    Cheap,
    /// Moderate right-hand side (~500 Ir).
    Moderate,
    /// Expensive right-hand side (~5000 Ir).
    Expensive,
    /// User-declared instruction reference cost per evaluation.
    DeclaredIr(u64),
}

impl RhsCost {
    /// Nominal instruction reference cost per right-hand side evaluation.
    pub const fn ir_cost(self) -> u64 {
        match self {
            Self::Cheap => 50,
            Self::Moderate => 500,
            Self::Expensive => 5000,
            Self::DeclaredIr(ir) => ir,
        }
    }
}

/// Work model version used for switching and cost qualification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkModelVersion {
    /// Work model v1: count-based proxy (legacy default).
    #[default]
    V1,
    /// Work model v2: Instruction Reference (Ir) polynomial overhead model.
    V2,
}

/// Polynomial weights for the 9 linear algebra and solver events in WorkModel v2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkModelV2Weights {
    /// Linear solve calls.
    pub linear_solves: PolynomialWeight,
    /// Krylov iterations.
    pub linear_iterations: PolynomialWeight,
    /// Orthogonalization inner products (dot products).
    pub orthogonalization_inner_products: PolynomialWeight,
    /// Orthogonalization vector updates (axpy).
    pub orthogonalization_vector_updates: PolynomialWeight,
    /// Direct dense LU factorizations.
    pub direct_factorizations: PolynomialWeight,
    /// Direct forward/back solve calls.
    pub direct_solve_calls: PolynomialWeight,
    /// Krylov vectors generated in phi method.
    pub phi_krylov_vectors: PolynomialWeight,
    /// Projected matrix exponential evaluations in phi method.
    pub phi_projected_exponentials: PolynomialWeight,
    /// Preconditioner applications.
    pub preconditioner_apps: PolynomialWeight,
}

/// Frozen fixed-point polynomial weights over denominator 65536.
pub const FROZEN_WEIGHTS: WorkModelV2Weights = WorkModelV2Weights {
    linear_solves: PolynomialWeight::new(6553600, 65536, 0, 0),
    linear_iterations: PolynomialWeight::new(655360, 131072, 0, 0),
    orthogonalization_inner_products: PolynomialWeight::new(0, 131072, 0, 0),
    orthogonalization_vector_updates: PolynomialWeight::new(0, 131072, 0, 0),
    direct_factorizations: PolynomialWeight::new(0, 0, 0, 43691),
    direct_solve_calls: PolynomialWeight::new(0, 0, 131072, 0),
    phi_krylov_vectors: PolynomialWeight::new(0, 131072, 0, 0),
    phi_projected_exponentials: PolynomialWeight::new(6553600, 0, 0, 0),
    preconditioner_apps: PolynomialWeight::new(0, 131072, 0, 0),
};

/// Compute linear algebra and solver overhead in Instruction Reference (Ir) units.
pub fn compute_overhead_ir(
    counters: &rodas5p_core::WorkCounters,
    dimension: usize,
) -> Option<u128> {
    let mut numerator: u128 = 0;

    let items = [
        (counters.linear_solves, &FROZEN_WEIGHTS.linear_solves),
        (
            counters.linear_iterations,
            &FROZEN_WEIGHTS.linear_iterations,
        ),
        (
            counters.orthogonalization_inner_products,
            &FROZEN_WEIGHTS.orthogonalization_inner_products,
        ),
        (
            counters.orthogonalization_vector_updates,
            &FROZEN_WEIGHTS.orthogonalization_vector_updates,
        ),
        (
            counters.direct_factorizations,
            &FROZEN_WEIGHTS.direct_factorizations,
        ),
        (
            counters.direct_solve_calls,
            &FROZEN_WEIGHTS.direct_solve_calls,
        ),
        (
            counters.phi_krylov_vectors,
            &FROZEN_WEIGHTS.phi_krylov_vectors,
        ),
        (
            counters.phi_projected_exponentials,
            &FROZEN_WEIGHTS.phi_projected_exponentials,
        ),
        (
            counters.preconditioner_apps,
            &FROZEN_WEIGHTS.preconditioner_apps,
        ),
    ];

    for (count, weight) in items {
        if count > 0 {
            let w = weight.eval(dimension)?;
            let contrib = (count as u128).checked_mul(w)?;
            numerator = numerator.checked_add(contrib)?;
        }
    }

    Some(numerator / (WORK_MODEL_DENOMINATOR as u128))
}

/// Total integer work under WorkModel v2, in Instruction Reference (Ir) units.
pub fn integer_work_v2_from_counters(
    counters: &rodas5p_core::WorkCounters,
    dimension: usize,
    rhs_cost: RhsCost,
) -> Option<u128> {
    let rhs_unit_ir = rhs_cost.ir_cost() as u128;
    let jacobian_rhs_units = (counters.jacobian_builds as u128).checked_mul(dimension as u128)?;
    let mut callback_units = jacobian_rhs_units;
    callback_units = callback_units.checked_add(counters.rhs_calls as u128)?;
    callback_units = callback_units.checked_add(counters.jvp_vectors as u128)?;
    callback_units = callback_units.checked_add(counters.ft_calls as u128)?;
    let callback_ir = callback_units.checked_mul(rhs_unit_ir)?;

    let overhead_ir = compute_overhead_ir(counters, dimension)?;
    callback_ir.checked_add(overhead_ir)
}

/// Total integer work under WorkModel v2 for a switching segment.
pub fn integer_work_v2(
    segment: &crate::switching::SwitchingSegment,
    dimension: usize,
    rhs_cost: RhsCost,
) -> Option<u128> {
    let mut counters = match &segment.driver {
        crate::switching::SwitchingDriverStats::Dopri5(stats) => stats.counters,
        crate::switching::SwitchingDriverStats::Rodas5p(stats) => {
            rodas5p_core::WorkCounters::from(&stats.counters)
        }
    };
    counters.rhs_calls = counters.rhs_calls.max(segment.rhs_calls);
    counters.jacobian_builds = counters.jacobian_builds.max(segment.jacobian_builds);
    counters.jvp_vectors = counters.jvp_vectors.max(segment.jvp_vectors);
    counters.ft_calls = counters.ft_calls.max(segment.ft_calls);
    counters.linear_solves = counters.linear_solves.max(segment.linear_solves);

    integer_work_v2_from_counters(&counters, dimension, rhs_cost)
}
