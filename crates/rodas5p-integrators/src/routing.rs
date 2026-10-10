//! Routing by declared, validated problem structure (research node SP03,
//! `research/sp03_declared_structure_routing_20261010`).
//!
//! An opt-in entry point, [`integrate_rodas5p_routed_observed`], that trusts
//! only a structure the caller declared through
//! [`OdeProblem::with_declared_structure`] and, for a band, verified at
//! `(t0, y0)` before any integration step. In order:
//!
//! 1. a declared band goes to the banded fast driver
//!    ([`crate::integrate_rodas5p_fast_banded_observed`]);
//! 2. otherwise a `Dense` declaration with `n <= 8` goes to the dense fast
//!    driver ([`crate::integrate_rodas5p_fast_observed`], v2);
//! 3. otherwise a problem with an explicit Jacobian goes to the dense fast
//!    driver;
//! 4. otherwise an `Unstructured` problem with a JVP goes to the U-form
//!    matrix-free driver with the `Legacy` stage target
//!    ([`crate::integrate_rodas5p_mf_fast_observed_gmres_into`]).
//!
//! Band verification compares the band callback's `J v` with the problem's
//! own JVP, or with its dense Jacobian when only that is available, on
//! [`BAND_VERIFICATION_VECTORS`] seeded random vectors, and requires
//! `||J_band v - J_ref v||_2 <= 1e-12 ||J_ref v||_2`. Its work is charged to
//! the run's counters and recorded separately. A mismatch refuses the run
//! with [`StructureError::BandVerificationMismatch`]; the router never falls
//! back silently. The existing banded path is not a general sparse LU.
//!
//! Every existing entry point is unchanged: the router only calls them.

use rodas5p_core::{
    CoreError, InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use serde::Serialize;

use crate::{
    AdaptiveStepConfig, ObservedIntegrationResult, OdeProblem, OutputSchedule,
    problem::{ProblemStructure, StructureError},
    rodas5p_fast::{
        RODAS5P_FAST_BANDED_DRIVER_ID, RODAS5P_FAST_DRIVER_ID, Rodas5pFastBandedResult,
        Rodas5pFastResult, integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_observed,
    },
    rodas5p_matrix_free_fast::{
        RODAS5P_MF_FAST_DRIVER_ID, Rodas5pMfFastResult,
        integrate_rodas5p_mf_fast_observed_gmres_into,
    },
};

/// Seeded random vectors of the band verification.
pub const BAND_VERIFICATION_VECTORS: usize = 2;
/// Relative 2-norm mismatch the band verification accepts.
pub const BAND_VERIFICATION_TOLERANCE: f64 = 1.0e-12;
/// Seed of the band verification's vectors (SplitMix64).
pub const BAND_VERIFICATION_SEED: u64 = 0x5903_0000_5eed_0001;
/// A `Dense` declaration routes as "declared dense, small" up to this
/// dimension; above it, rule 3 (explicit Jacobian) applies.
pub const ROUTER_DENSE_SMALL_MAX: usize = 8;

/// What the band callback was compared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BandReference {
    /// The problem's own JVP callback.
    Jvp,
    /// The problem's dense Jacobian (no JVP available).
    DenseJacobian,
}

/// The record of one band verification and the work charged for it.
#[derive(Clone, Debug, Serialize)]
pub struct BandVerification {
    pub lower: usize,
    pub upper: usize,
    pub reference: BandReference,
    pub vectors: usize,
    pub seed: u64,
    pub tolerance: f64,
    /// `||J_band v - J_ref v||_2 / ||J_ref v||_2` per vector (0 when both
    /// products are zero).
    pub relative_mismatch: Vec<f64>,
    pub band_fills: u64,
    pub band_products: u64,
    pub jvp_products: u64,
    pub dense_jacobian_builds: u64,
    pub dense_products: u64,
    /// The counters charged to the run: the band fill and the dense
    /// Jacobian in `jacobian_builds`, band and dense products in
    /// `jacobian_matvecs`, JVP products in `jvp_calls` / `jvp_vectors`.
    pub charged: WorkCounters,
}

/// Why the router chose its driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutingReason {
    /// Rule 1: a declared (and verified) band.
    DeclaredBand,
    /// Rule 2: a `Dense` declaration with `n <= 8`.
    DeclaredDenseSmall,
    /// Rule 3: an explicit Jacobian.
    ExplicitJacobian,
    /// Rule 4: no explicit Jacobian, a JVP.
    UnstructuredJvp,
}

/// The driver the router called.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutedDriver {
    Banded,
    Dense,
    MatrixFree,
}

/// A declaration the router could not honour as declared, and what it did
/// instead. Band verification failures are never a fallback: they refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum RoutingFallback {
    /// A `Dense` declaration above [`ROUTER_DENSE_SMALL_MAX`]: rule 3 (the
    /// same dense driver, routed by its explicit Jacobian).
    DenseDeclaredAboveSmallMax { dimension: usize, small_max: usize },
}

/// The routing decision of one run.
#[derive(Clone, Debug, Serialize)]
pub struct RoutingRecord {
    pub declared: ProblemStructure,
    pub dimension: usize,
    pub reason: RoutingReason,
    pub driver: RoutedDriver,
    pub driver_id: &'static str,
    pub fallback: Option<RoutingFallback>,
    pub verification: Option<BandVerification>,
}

/// The run of the chosen driver, as that driver returns it, except that the
/// verification charge is added to its counters.
#[derive(Clone, Debug)]
pub enum RoutedRun {
    Banded(Rodas5pFastBandedResult),
    Dense(Rodas5pFastResult),
    MatrixFree(Rodas5pMfFastResult),
}

impl RoutedRun {
    pub fn observed(&self) -> &ObservedIntegrationResult {
        match self {
            Self::Banded(run) => &run.fast.observed,
            Self::Dense(run) => &run.observed,
            Self::MatrixFree(run) => &run.observed,
        }
    }

    fn observed_mut(&mut self) -> &mut ObservedIntegrationResult {
        match self {
            Self::Banded(run) => &mut run.fast.observed,
            Self::Dense(run) => &mut run.observed,
            Self::MatrixFree(run) => &mut run.observed,
        }
    }

    pub fn attempts(&self) -> usize {
        match self {
            Self::Banded(run) => run.fast.attempts,
            Self::Dense(run) => run.attempts,
            Self::MatrixFree(run) => run.attempts,
        }
    }

    pub fn accepted_steps(&self) -> usize {
        match self {
            Self::Banded(run) => run.fast.accepted_steps,
            Self::Dense(run) => run.accepted_steps,
            Self::MatrixFree(run) => run.accepted_steps,
        }
    }

    pub fn rejected_steps(&self) -> usize {
        match self {
            Self::Banded(run) => run.fast.rejected_steps,
            Self::Dense(run) => run.rejected_steps,
            Self::MatrixFree(run) => run.rejected_steps,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RoutedResult {
    pub run: RoutedRun,
    pub routing: RoutingRecord,
}

impl RoutedResult {
    /// The run's counters without the verification charge: the counters of
    /// the target driver called directly.
    pub fn driver_counters(&self) -> WorkCounters {
        let counters = self.run.observed().counters;
        match &self.routing.verification {
            Some(v) => counters.delta(v.charged),
            None => counters,
        }
    }
}

/// A refused or failed routed run.
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    /// The declaration was refused (before any integration step).
    #[error(transparent)]
    Structure(#[from] StructureError),
    /// The chosen driver refused its input or failed.
    #[error(transparent)]
    Integration(#[from] CoreError),
}

/// Options of the router: the linear configuration of the matrix-free route.
#[derive(Clone, Debug)]
pub struct RoutingOptions {
    pub matrix_free: LinearSolverConfig,
}

impl Default for RoutingOptions {
    /// The production stage target of the U-form driver (SPD07, ALG01's
    /// base): GMRES, restart 40, linear rtol 1e-10, atol 1e-14, no
    /// preconditioner, zero start, Krylov budget 200.
    fn default() -> Self {
        Self {
            matrix_free: LinearSolverConfig {
                method: LinearMethod::Gmres,
                rtol: 1.0e-10,
                atol: 1.0e-14,
                preconditioner: PreconditionerKind::None,
                x0_strategy: InitialGuess::Zero,
                maxiter: 200,
                ..LinearSolverConfig::default()
            },
        }
    }
}

/// The routing decision for `problem`, without verification or integration.
pub fn route_problem(
    problem: &OdeProblem,
) -> Result<(RoutingReason, RoutedDriver, Option<RoutingFallback>), RoutingError> {
    let n = problem.dimension;
    match problem.declared_structure() {
        ProblemStructure::Banded { .. } => {
            return Ok((RoutingReason::DeclaredBand, RoutedDriver::Banded, None));
        }
        ProblemStructure::Dense if n <= ROUTER_DENSE_SMALL_MAX => {
            return Ok((RoutingReason::DeclaredDenseSmall, RoutedDriver::Dense, None));
        }
        _ => {}
    }
    let fallback = (problem.declared_structure() == ProblemStructure::Dense).then_some(
        RoutingFallback::DenseDeclaredAboveSmallMax {
            dimension: n,
            small_max: ROUTER_DENSE_SMALL_MAX,
        },
    );
    if problem.has_explicit_jacobian_callback() {
        return Ok((
            RoutingReason::ExplicitJacobian,
            RoutedDriver::Dense,
            fallback,
        ));
    }
    if problem.declared_structure() == ProblemStructure::Unstructured && problem.has_jvp() {
        return Ok((
            RoutingReason::UnstructuredJvp,
            RoutedDriver::MatrixFree,
            None,
        ));
    }
    Err(RoutingError::Integration(CoreError::InvalidInput(
        "no route: the problem has neither an explicit Jacobian nor a JVP".into(),
    )))
}

/// SplitMix64 stream mapped to `[-1, 1)`.
fn seeded_vector(n: usize, state: &mut u64) -> Vec<f64> {
    (0..n)
        .map(|_| {
            *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = *state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            ((z >> 11) as f64) * (1.0 / (1u64 << 53) as f64) * 2.0 - 1.0
        })
        .collect()
}

fn norm2(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Verify the declared band of `problem` at `(t0, y0)` (rule 1's
/// precondition). Errors with a typed [`StructureError`] when no band is
/// declared, a callback fails or produces a non-finite value, or the
/// mismatch exceeds [`BAND_VERIFICATION_TOLERANCE`].
pub fn verify_declared_band(
    problem: &OdeProblem,
    t0: f64,
    y0: &[f64],
) -> Result<BandVerification, StructureError> {
    let failed = |what: String| StructureError::BandVerificationFailed(what);
    let band = problem
        .declared_band()
        .ok_or_else(|| failed("no band is declared".into()))?;
    let n = problem.dimension;
    if y0.len() != n {
        return Err(failed(format!(
            "initial state has length {}, problem dimension {n}",
            y0.len()
        )));
    }
    let (l, u) = (band.lower, band.upper);
    let width = l + u + 1;
    let mut charged = WorkCounters::default();
    let mut values = vec![0.0; n * width];
    charged.jacobian_builds += 1;
    (band.fill)(t0, y0, &mut values).map_err(|e| failed(format!("band callback: {e}")))?;
    if !values.iter().all(|v| v.is_finite()) {
        return Err(failed("band callback produced NaN/Inf".into()));
    }
    let jvp = problem.jvp_callback().cloned();
    let reference = if jvp.is_some() {
        BandReference::Jvp
    } else {
        BandReference::DenseJacobian
    };
    let dense = match reference {
        BandReference::DenseJacobian => Some(
            problem
                .dense_jacobian(t0, y0, &mut charged)
                .map_err(|e| failed(format!("dense Jacobian: {e}")))?,
        ),
        BandReference::Jvp => None,
    };
    let mut record = BandVerification {
        lower: l,
        upper: u,
        reference,
        vectors: BAND_VERIFICATION_VECTORS,
        seed: BAND_VERIFICATION_SEED,
        tolerance: BAND_VERIFICATION_TOLERANCE,
        relative_mismatch: Vec::with_capacity(BAND_VERIFICATION_VECTORS),
        band_fills: 1,
        band_products: 0,
        jvp_products: 0,
        dense_jacobian_builds: u64::from(dense.is_some()),
        dense_products: 0,
        charged: WorkCounters::default(),
    };
    let mut state = BAND_VERIFICATION_SEED;
    let mut banded = vec![0.0; n];
    let mut exact = vec![0.0; n];
    for k in 0..BAND_VERIFICATION_VECTORS {
        let v = seeded_vector(n, &mut state);
        // Band product, columns in ascending order.
        for (i, out) in banded.iter_mut().enumerate() {
            let first = i.saturating_sub(l);
            let last = (i + u).min(n - 1);
            let mut sum = 0.0;
            for j in first..=last {
                sum += values[i * width + (j + l - i)] * v[j];
            }
            *out = sum;
        }
        record.band_products += 1;
        charged.jacobian_matvecs += 1;
        match (&jvp, &dense) {
            (Some(jvp), _) => {
                exact.fill(0.0);
                jvp(t0, y0, &v, &mut exact).map_err(|e| failed(format!("JVP: {e}")))?;
                record.jvp_products += 1;
                charged.jvp_calls += 1;
                charged.jvp_vectors += 1;
            }
            (None, Some(matrix)) => {
                for (out, row) in exact.iter_mut().zip(matrix.as_slice().chunks_exact(n)) {
                    *out = row.iter().zip(&v).map(|(a, x)| a * x).sum();
                }
                record.dense_products += 1;
                charged.jacobian_matvecs += 1;
            }
            (None, None) => unreachable!("a reference is always chosen"),
        }
        if !banded.iter().chain(&exact).all(|x| x.is_finite()) {
            return Err(failed(format!("non-finite product for vector {k}")));
        }
        let diff = banded
            .iter()
            .zip(&exact)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        let reference_norm = norm2(&exact);
        if diff > BAND_VERIFICATION_TOLERANCE * reference_norm {
            return Err(StructureError::BandVerificationMismatch {
                vector: k,
                mismatch: diff,
                reference_norm,
                tolerance: BAND_VERIFICATION_TOLERANCE,
            });
        }
        record.relative_mismatch.push(if diff == 0.0 {
            0.0
        } else {
            diff / reference_norm
        });
    }
    record.charged = charged;
    Ok(record)
}

/// Adaptive RODAS5P through the driver chosen by the problem's declared and
/// validated structure (research node SP03); see the module documentation.
/// The result records the routing reason, any fallback and the band
/// verification, whose work is charged to the run's counters.
pub fn integrate_rodas5p_routed_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Result<RoutedResult, RoutingError> {
    integrate_rodas5p_routed_observed_with_options(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        &RoutingOptions::default(),
    )
}

/// [`integrate_rodas5p_routed_observed`] with explicit [`RoutingOptions`].
pub fn integrate_rodas5p_routed_observed_with_options(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: &RoutingOptions,
) -> Result<RoutedResult, RoutingError> {
    let (reason, driver, fallback) = route_problem(problem)?;
    let mut routing = RoutingRecord {
        declared: problem.declared_structure(),
        dimension: problem.dimension,
        reason,
        driver,
        driver_id: match driver {
            RoutedDriver::Banded => RODAS5P_FAST_BANDED_DRIVER_ID,
            RoutedDriver::Dense => RODAS5P_FAST_DRIVER_ID,
            RoutedDriver::MatrixFree => RODAS5P_MF_FAST_DRIVER_ID,
        },
        fallback,
        verification: None,
    };
    let mut run = match driver {
        RoutedDriver::Banded => {
            let band = problem
                .declared_band()
                .expect("rule 1 is chosen only for a declared band");
            // Before any integration step: a mismatch refuses the run.
            routing.verification = Some(verify_declared_band(problem, t_span.0, y0)?);
            RoutedRun::Banded(integrate_rodas5p_fast_banded_observed(
                problem, &band, t_span, y0, adaptive, output,
            )?)
        }
        RoutedDriver::Dense => RoutedRun::Dense(integrate_rodas5p_fast_observed(
            problem, t_span, y0, adaptive, output,
        )?),
        RoutedDriver::MatrixFree => {
            RoutedRun::MatrixFree(integrate_rodas5p_mf_fast_observed_gmres_into(
                problem,
                t_span,
                y0,
                &options.matrix_free,
                adaptive,
                output,
            )?)
        }
    };
    if let Some(verification) = &routing.verification {
        run.observed_mut().counters.accumulate(verification.charged);
    }
    Ok(RoutedResult { run, routing })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rodas5p_core::{CoreResult, DenseMatrix};

    use super::*;
    use crate::problem::StructureDeclaration;

    /// Tridiagonal `J = tridiag(1, -2 - y_i, 1)` (n components) with an
    /// explicit Jacobian and, optionally, a JVP.
    fn tridiagonal(n: usize, with_jvp: bool) -> OdeProblem {
        let rhs = Arc::new(
            move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
                for i in 0..n {
                    let left = if i > 0 { y[i - 1] } else { 0.0 };
                    let right = if i + 1 < n { y[i + 1] } else { 0.0 };
                    out[i] = left - 2.0 * y[i] - 0.5 * y[i] * y[i] + right;
                }
                Ok(())
            },
        );
        let jacobian = Arc::new(move |_t: f64, y: &[f64]| -> CoreResult<DenseMatrix> {
            let mut j = DenseMatrix::zeros(n, n);
            for i in 0..n {
                j[(i, i)] = -2.0 - y[i];
                if i > 0 {
                    j[(i, i - 1)] = 1.0;
                }
                if i + 1 < n {
                    j[(i, i + 1)] = 1.0;
                }
            }
            Ok(j)
        });
        let jvp = with_jvp.then(|| {
            Arc::new(
                move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| -> CoreResult<()> {
                    for i in 0..n {
                        let left = if i > 0 { v[i - 1] } else { 0.0 };
                        let right = if i + 1 < n { v[i + 1] } else { 0.0 };
                        out[i] = left + (-2.0 - y[i]) * v[i] + right;
                    }
                    Ok(())
                },
            ) as crate::problem::JvpFn
        });
        OdeProblem::new(
            "tridiagonal",
            n,
            rhs,
            None,
            Some(jacobian),
            jvp,
            None,
            true,
            None,
            None,
        )
        .unwrap()
    }

    fn band_fill(n: usize, lower: usize) -> crate::BandedJacobianFn {
        Arc::new(move |_t: f64, y: &[f64], band: &mut [f64]| {
            let width = lower + 2;
            for i in 0..n {
                band[i * width + lower] = -2.0 - y[i];
                if i > 0 && lower >= 1 {
                    band[i * width + lower - 1] = 1.0;
                }
                if i + 1 < n {
                    band[i * width + lower + 1] = 1.0;
                }
            }
            Ok(())
        })
    }

    #[test]
    fn seeded_vectors_are_deterministic_and_in_range() {
        let (mut a, mut b) = (BAND_VERIFICATION_SEED, BAND_VERIFICATION_SEED);
        let (x, y) = (seeded_vector(64, &mut a), seeded_vector(64, &mut b));
        assert_eq!(x, y);
        assert!(x.iter().all(|v| (-1.0..1.0).contains(v)));
        assert_ne!(seeded_vector(64, &mut a), x);
    }

    #[test]
    fn verification_uses_the_jvp_when_present_and_charges_it() {
        let n = 12;
        let y0 = vec![0.25; n];
        for with_jvp in [true, false] {
            let p = tridiagonal(n, with_jvp)
                .with_declared_structure(StructureDeclaration::banded(n, 1, 1, band_fill(n, 1)))
                .unwrap();
            let v = verify_declared_band(&p, 0.0, &y0).unwrap();
            assert_eq!(v.band_products, 2);
            assert_eq!(v.charged.jacobian_matvecs, if with_jvp { 2 } else { 4 });
            if with_jvp {
                assert_eq!(v.reference, BandReference::Jvp);
                assert_eq!((v.charged.jvp_calls, v.jvp_products), (2, 2));
                assert_eq!(v.charged.jacobian_builds, 1);
            } else {
                assert_eq!(v.reference, BandReference::DenseJacobian);
                assert_eq!((v.dense_products, v.dense_jacobian_builds), (2, 1));
                assert_eq!(v.charged.jacobian_builds, 2);
                assert_eq!(v.charged.jvp_calls, 0);
            }
            assert!(v.relative_mismatch.iter().all(|m| *m <= 1e-15));
        }
    }

    #[test]
    fn a_band_narrower_than_the_jacobian_is_rejected() {
        let n = 12;
        // lower = 0: the subdiagonal ones are dropped.
        let p = tridiagonal(n, false)
            .with_declared_structure(StructureDeclaration::banded(n, 0, 1, band_fill(n, 0)))
            .unwrap();
        assert!(matches!(
            verify_declared_band(&p, 0.0, &vec![0.25; n]),
            Err(StructureError::BandVerificationMismatch { .. })
        ));
    }

    #[test]
    fn routing_rules_in_order() {
        let n = 12;
        let banded = tridiagonal(n, true)
            .with_declared_structure(StructureDeclaration::banded(n, 1, 1, band_fill(n, 1)))
            .unwrap();
        assert_eq!(
            route_problem(&banded).unwrap(),
            (RoutingReason::DeclaredBand, RoutedDriver::Banded, None)
        );
        let small = tridiagonal(8, true)
            .with_declared_structure(StructureDeclaration::dense(8))
            .unwrap();
        assert_eq!(
            route_problem(&small).unwrap(),
            (RoutingReason::DeclaredDenseSmall, RoutedDriver::Dense, None)
        );
        let large = tridiagonal(n, true)
            .with_declared_structure(StructureDeclaration::dense(n))
            .unwrap();
        assert_eq!(
            route_problem(&large).unwrap(),
            (
                RoutingReason::ExplicitJacobian,
                RoutedDriver::Dense,
                Some(RoutingFallback::DenseDeclaredAboveSmallMax {
                    dimension: n,
                    small_max: 8
                })
            )
        );
        let plain = tridiagonal(n, true);
        assert_eq!(
            route_problem(&plain).unwrap(),
            (RoutingReason::ExplicitJacobian, RoutedDriver::Dense, None)
        );
        let jvp_only = plain.jvp_only_clone().unwrap();
        assert_eq!(
            route_problem(&jvp_only).unwrap(),
            (
                RoutingReason::UnstructuredJvp,
                RoutedDriver::MatrixFree,
                None
            )
        );
        // The strict matrix-free clone drops a declaration.
        assert_eq!(
            banded.jvp_only_clone().unwrap().declared_structure(),
            ProblemStructure::Unstructured
        );
    }
}
