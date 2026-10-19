//! Generative collective complex cubic model and structure admission contracts (LP01).
//!
//! Autonomous N-mode complex cubic model:
//!   \dot{a}_j = (-kappa + i * width * \xi_j) * a_j + i * \chi * |\langle a \rangle|^2 * \langle a \rangle
//! with normalized positive weights w_j > 0, sum w_j = 1, detunings \xi_j \in [-1, 1],
//! independent complex initial amplitudes a_j(0) = z_j = u_j(0) + i v_j(0), and
//! collective mean \langle a \rangle = \sum_j w_j a_j.
//!
//! Real state vector layout:
//!   y \in R^{2N}, y[2j] = Re(a_j) = u_j, y[2j + 1] = Im(a_j) = v_j.

use std::sync::Arc;
use thiserror::Error;

use rodas5p_core::{CoreError, CoreResult};

use crate::problem::OdeProblem;

/// Errors arising from collective model admission or validation.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum CollectiveAdmissionError {
    #[error("mode count must be positive, got 0")]
    EmptyModes,
    #[error(
        "dimension mismatch: xi length {xi}, weights length {weights}, initial length {initial}"
    )]
    DimensionMismatch {
        xi: usize,
        weights: usize,
        initial: usize,
    },
    #[error("non-finite value encountered in collective model parameter: {0}")]
    NonFinite(String),
    #[error("detuning frequency xi[{index}] = {value} lies outside [-1, 1]")]
    DetuningOutOfRange { index: usize, value: f64 },
    #[error("weight w[{index}] = {value} is non-positive; strictly positive weights required")]
    NonPositiveWeight { index: usize, value: f64 },
    #[error("weights do not sum to 1.0 within tolerance: sum = {sum}, diff = {diff}")]
    UnnormalizedWeights { sum: f64, diff: f64 },
    #[error("initial weighted energy {energy} exceeds unit ball limit (1.0)")]
    EnergyExceedsUnitBall { energy: f64 },
    #[error("spectral width must be non-negative, got {0}")]
    NegativeWidth(f64),
    #[error("damping parameter kappa must be non-negative, got {0}")]
    NegativeDamping(f64),
    #[error("unsupported model: {0}")]
    UnsupportedModel(String),
}

/// An immutable, checked generative collective complex cubic model.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectiveModel {
    xi: Vec<f64>,
    weights: Vec<f64>,
    initial: Vec<(f64, f64)>,
    width: f64,
    chi: f64,
    carrier: f64,
    kappa: f64,
    initial_energy: f64,
}

/// Source-local admission accounting. Payload reads and the original ordinary
/// binary64 arithmetic are charged before validation can return an error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CollectiveAdmissionWork {
    pub payload_items: u64,
    pub arithmetic_ops: u64,
}

/// Shared borrowed admission preserves the constructor's check order, ordinary
/// summation order and thresholds. It allocates only for existing error strings.
fn validate_borrowed_model(
    xi: &[f64],
    weights: &[f64],
    initial: &[(f64, f64)],
    width: f64,
    chi: f64,
    carrier: f64,
    kappa: f64,
    work: &mut CollectiveAdmissionWork,
) -> Result<f64, CollectiveAdmissionError> {
    let n = xi.len();
    if n == 0 {
        return Err(CollectiveAdmissionError::EmptyModes);
    }
    if weights.len() != n || initial.len() != n {
        return Err(CollectiveAdmissionError::DimensionMismatch {
            xi: n,
            weights: weights.len(),
            initial: initial.len(),
        });
    }

    // Finiteness check for scalar parameters
    work.payload_items += 1;
    if !width.is_finite() {
        return Err(CollectiveAdmissionError::NonFinite("width".into()));
    }
    work.payload_items += 1;
    if !chi.is_finite() {
        return Err(CollectiveAdmissionError::NonFinite("chi".into()));
    }
    work.payload_items += 1;
    if !carrier.is_finite() {
        return Err(CollectiveAdmissionError::NonFinite("carrier".into()));
    }
    work.payload_items += 1;
    if !kappa.is_finite() {
        return Err(CollectiveAdmissionError::NonFinite("kappa".into()));
    }

    if width < 0.0 {
        return Err(CollectiveAdmissionError::NegativeWidth(width));
    }
    if kappa < 0.0 {
        return Err(CollectiveAdmissionError::NegativeDamping(kappa));
    }

    // Detuning and weight validation
    let mut weight_sum = 0.0;
    let mut initial_energy = 0.0;

    for (j, (&xj, &wj)) in xi.iter().zip(weights.iter()).enumerate() {
        work.payload_items += 1;
        if !xj.is_finite() {
            return Err(CollectiveAdmissionError::NonFinite(format!("xi[{j}]")));
        }
        work.payload_items += 1;
        if !wj.is_finite() {
            return Err(CollectiveAdmissionError::NonFinite(format!("weights[{j}]")));
        }
        work.payload_items += 2;
        let (u0, v0) = initial[j];
        if !u0.is_finite() || !v0.is_finite() {
            return Err(CollectiveAdmissionError::NonFinite(format!("initial[{j}]")));
        }

        if !(-1.0 - 1e-14..=1.0 + 1e-14).contains(&xj) {
            return Err(CollectiveAdmissionError::DetuningOutOfRange {
                index: j,
                value: xj,
            });
        }
        if wj <= 0.0 {
            return Err(CollectiveAdmissionError::NonPositiveWeight {
                index: j,
                value: wj,
            });
        }

        work.arithmetic_ops += 1;
        weight_sum += wj;
        work.arithmetic_ops += 5;
        initial_energy += wj * (u0 * u0 + v0 * v0);
    }

    work.arithmetic_ops += 2;
    let diff = (weight_sum - 1.0).abs();
    if diff > 1e-12 {
        return Err(CollectiveAdmissionError::UnnormalizedWeights {
            sum: weight_sum,
            diff,
        });
    }

    if initial_energy > 1.0 + 1e-12 {
        return Err(CollectiveAdmissionError::EnergyExceedsUnitBall {
            energy: initial_energy,
        });
    }

    Ok(initial_energy)
}

impl CollectiveModel {
    /// Constructs and validates a new [`CollectiveModel`].
    ///
    /// # Errors
    /// Returns a [`CollectiveAdmissionError`] if any input parameter fails
    /// domain, normalization, or unit-energy admission checks.
    pub fn new(
        xi: Vec<f64>,
        weights: Vec<f64>,
        initial: Vec<(f64, f64)>,
        width: f64,
        chi: f64,
        carrier: f64,
        kappa: f64,
    ) -> Result<Self, CollectiveAdmissionError> {
        let initial_energy = validate_borrowed_model(
            &xi,
            &weights,
            &initial,
            width,
            chi,
            carrier,
            kappa,
            &mut CollectiveAdmissionWork::default(),
        )?;

        Ok(Self {
            xi,
            weights,
            initial,
            width,
            chi,
            carrier,
            kappa,
            initial_energy,
        })
    }

    /// Apply exactly the constructor's admission checks to replacement centers.
    /// A successful borrowed validation allocates no model or vector and does
    /// not mutate this model. This is an admission check, not an ODE certificate.
    pub fn validate_candidate_centers(
        &self,
        centers: &[(f64, f64)],
    ) -> Result<f64, CollectiveAdmissionError> {
        self.validate_candidate_centers_with_work(centers, &mut CollectiveAdmissionWork::default())
    }

    pub(crate) fn validate_candidate_centers_with_work(
        &self,
        centers: &[(f64, f64)],
        work: &mut CollectiveAdmissionWork,
    ) -> Result<f64, CollectiveAdmissionError> {
        validate_borrowed_model(
            &self.xi,
            &self.weights,
            centers,
            self.width,
            self.chi,
            self.carrier,
            self.kappa,
            work,
        )
    }

    /// Number of complex modes N.
    #[inline]
    pub fn num_modes(&self) -> usize {
        self.xi.len()
    }

    /// Dimension of the real state vector (2N).
    #[inline]
    pub fn dimension(&self) -> usize {
        self.xi.len() * 2
    }

    /// Returns the detuning frequencies \xi_j.
    pub fn xi(&self) -> &[f64] {
        &self.xi
    }

    /// Returns the positive normalized weights w_j.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// Returns the initial complex amplitudes z_j = (u_j(0), v_j(0)).
    pub fn initial_amplitudes(&self) -> &[(f64, f64)] {
        &self.initial
    }

    /// Writes the initial state vector into `y0 \in R^{2N}`.
    pub fn initial_state(&self, y0: &mut [f64]) -> CoreResult<()> {
        let dim = self.dimension();
        if y0.len() != dim {
            return Err(CoreError::Dimension(format!(
                "initial_state: buffer length {} != expected dimension {dim}",
                y0.len()
            )));
        }
        for (j, &(u, v)) in self.initial.iter().enumerate() {
            y0[2 * j] = u;
            y0[2 * j + 1] = v;
        }
        Ok(())
    }

    /// Spectral width parameter.
    #[inline]
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Cubic coupling parameter \chi.
    #[inline]
    pub fn chi(&self) -> f64 {
        self.chi
    }

    /// Carrier frequency parameter.
    #[inline]
    pub fn carrier(&self) -> f64 {
        self.carrier
    }

    /// Linear damping parameter \kappa.
    #[inline]
    pub fn kappa(&self) -> f64 {
        self.kappa
    }

    /// Weighted energy of the initial state E(0) = \sum_j w_j |z_j|^2.
    #[inline]
    pub fn initial_energy(&self) -> f64 {
        self.initial_energy
    }

    /// Computes the weighted energy E(y) = \sum_j w_j (u_j^2 + v_j^2).
    ///
    /// Hot-loop zero allocation.
    pub fn energy(&self, y: &[f64]) -> f64 {
        let n = self.num_modes();
        let mut e = 0.0;
        for j in 0..n {
            let u = y[2 * j];
            let v = y[2 * j + 1];
            e += self.weights[j] * (u * u + v * v);
        }
        e
    }

    /// Computes the complex collective mean \langle a \rangle = (\bar{u}, \bar{v}) = \sum_j w_j (u_j, v_j).
    ///
    /// Hot-loop zero allocation.
    pub fn mean(&self, y: &[f64]) -> (f64, f64) {
        let n = self.num_modes();
        let mut u_bar = 0.0;
        let mut v_bar = 0.0;
        for j in 0..n {
            let w = self.weights[j];
            u_bar += w * y[2 * j];
            v_bar += w * y[2 * j + 1];
        }
        (u_bar, v_bar)
    }

    /// Evaluates the real-valued ODE vector field in-place with zero heap allocations.
    ///
    /// \dot{u}_j = -kappa * u_j - width * \xi_j * v_j - \chi * (\bar{u}^2 + \bar{v}^2) * \bar{v}
    /// \dot{v}_j = -kappa * v_j + width * \xi_j * u_j + \chi * (\bar{u}^2 + \bar{v}^2) * \bar{u}
    #[inline]
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) -> CoreResult<()> {
        let dim = self.dimension();
        if y.len() != dim || dy.len() != dim {
            return Err(CoreError::Dimension(format!(
                "rhs dimension mismatch: y={}, dy={}, expected {dim}",
                y.len(),
                dy.len()
            )));
        }

        let n = self.num_modes();
        let (u_bar, v_bar) = self.mean(y);
        let m2 = u_bar * u_bar + v_bar * v_bar;
        let chi = self.chi;
        let width = self.width;
        let kappa = self.kappa;

        let nl_u = -chi * m2 * v_bar;
        let nl_v = chi * m2 * u_bar;

        for j in 0..n {
            let u = y[2 * j];
            let v = y[2 * j + 1];
            let xi = self.xi[j];

            dy[2 * j] = -kappa * u - width * xi * v + nl_u;
            dy[2 * j + 1] = -kappa * v + width * xi * u + nl_v;
        }

        Ok(())
    }

    /// Evaluates the analytic Jacobian-vector product J(y) * v in-place with zero heap allocations.
    ///
    /// Operates in O(N) operations via the low-rank collective feedback differential.
    #[inline]
    pub fn jvp(&self, _t: f64, y: &[f64], v: &[f64], jv: &mut [f64]) -> CoreResult<()> {
        let dim = self.dimension();
        if y.len() != dim || v.len() != dim || jv.len() != dim {
            return Err(CoreError::Dimension(format!(
                "jvp dimension mismatch: y={}, v={}, jv={}, expected {dim}",
                y.len(),
                v.len(),
                jv.len()
            )));
        }

        let n = self.num_modes();
        let (u_bar, v_bar) = self.mean(y);
        let (du_bar, dv_bar) = self.mean(v);

        let m2 = u_bar * u_bar + v_bar * v_bar;
        let d_m2 = 2.0 * (u_bar * du_bar + v_bar * dv_bar);

        let chi = self.chi;
        let width = self.width;
        let kappa = self.kappa;

        // Differential of nonlinear terms:
        // d(-chi * m2 * v_bar) = -chi * (d_m2 * v_bar + m2 * dv_bar)
        // d( chi * m2 * u_bar) =  chi * (d_m2 * u_bar + m2 * du_bar)
        let d_nl_u = -chi * (d_m2 * v_bar + m2 * dv_bar);
        let d_nl_v = chi * (d_m2 * u_bar + m2 * du_bar);

        for j in 0..n {
            let du = v[2 * j];
            let dv = v[2 * j + 1];
            let xi = self.xi[j];

            jv[2 * j] = -kappa * du - width * xi * dv + d_nl_u;
            jv[2 * j + 1] = -kappa * dv + width * xi * du + d_nl_v;
        }

        Ok(())
    }

    /// Computes the k-th real spectral moment \mu_k = \sum_j w_j \xi_j^k.
    pub fn spectral_moment(&self, k: usize) -> f64 {
        let mut mu = 0.0;
        for (&w, &xi) in self.weights.iter().zip(self.xi.iter()) {
            mu += w * xi.powi(k as i32);
        }
        mu
    }

    /// Computes the k-th complex weighted initial moment \nu_k = \sum_j w_j \xi_j^k z_j.
    pub fn weighted_moment(&self, k: usize) -> (f64, f64) {
        let mut re = 0.0;
        let mut im = 0.0;
        for ((&w, &xi), &(u, v)) in self
            .weights
            .iter()
            .zip(self.xi.iter())
            .zip(self.initial.iter())
        {
            let factor = w * xi.powi(k as i32);
            re += factor * u;
            im += factor * v;
        }
        (re, im)
    }

    /// Creates an [`OdeProblem`] adapter backed by this collective model.
    ///
    /// The generated problem has exact in-place `rhs` and `jvp` closures,
    /// identity mass, and autonomous dynamics.
    pub fn to_ode_problem(&self) -> CoreResult<OdeProblem> {
        let model = Arc::new(self.clone());
        let m1 = Arc::clone(&model);
        let m2 = Arc::clone(&model);

        let rhs = Arc::new(move |t: f64, y: &[f64], dy: &mut [f64]| -> CoreResult<()> {
            m1.rhs(t, y, dy)
        });

        let jvp = Arc::new(
            move |t: f64, y: &[f64], v: &[f64], jv: &mut [f64]| -> CoreResult<()> {
                m2.jvp(t, y, v, jv)
            },
        );

        OdeProblem::new(
            format!("collective-cubic-{}", self.num_modes()),
            self.dimension(),
            rhs,
            None,
            None,
            Some(jvp),
            None,
            true,
            None,
            None,
        )
    }
}

/// Verifies whether an [`OdeProblem`] carries a valid declared collective structure.
///
/// Refuses generic black-box ODE problems with typed [`CollectiveAdmissionError::UnsupportedModel`].
pub fn verify_collective_admission(
    problem: &OdeProblem,
    model: Option<&CollectiveModel>,
) -> Result<(), CollectiveAdmissionError> {
    match model {
        Some(m) => {
            if m.dimension() != problem.dimension {
                return Err(CollectiveAdmissionError::DimensionMismatch {
                    xi: m.num_modes(),
                    weights: m.num_modes(),
                    initial: m.num_modes(),
                });
            }
            if problem.mass_matrix.is_some() {
                return Err(CollectiveAdmissionError::UnsupportedModel(
                    "collective packet requires identity mass matrix".into(),
                ));
            }
            Ok(())
        }
        None => Err(CollectiveAdmissionError::UnsupportedModel(
            "arbitrary generic ODE callbacks cannot be admitted into collective packet pipeline without CollectiveModel binding".into(),
        )),
    }
}
