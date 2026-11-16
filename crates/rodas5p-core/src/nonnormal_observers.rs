//! Nonnormal and nonautonomous stability observers (Node P4 `P4_NONNORMAL_OBSERVERS`).
//!
//! Implements four independent observer families for nonnormal dynamics,
//! transient amplification, and time-dependent metric bounds:
//!
//! 1. [`FixedHLogNormObserver`]:
//!    Given a positive-definite metric $H$ (identity, diagonal, or general fixed SPD matrix),
//!    computes the weighted logarithmic norm:
//!    $$\mu_H(J) = \lambda_{\max}\left( \frac{1}{2} H^{-1/2} (J^T H + H J) H^{-1/2} \right).$$
//!    For $H = I$, recovers the Euclidean numerical abscissa $\mu_2(J) = \lambda_{\max}((J + J^T)/2)$.
//!
//! 2. [`MovingHMetricObserver`]:
//!    Implements CTSH candidate CTSH-VIG-METRICFLOW-004 for time-dependent metric $H(t)$
//!    and its derivative $\dot{H}(t)$:
//!    $$M_H(t) = \frac{1}{2} H(t)^{-1/2} \left[ \dot{H}(t) + J(t)^T H(t) + H(t) J(t) \right] H(t)^{-1/2},$$
//!    $$\mu_H(t) = \lambda_{\max}(M_H(t)).$$
//!    Tracks the integrated growth bound $\exp\left(\int_0^t \mu_H(s) ds\right)$.
//!    Supports metric-switch jump factors: when switching from $H_-$ to $H_+$,
//!    bounds the norm jump by $\gamma_{\mathrm{jump}} = \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}$
//!    and multiplies the cumulative growth bound by $\gamma_{\mathrm{jump}}$.
//!
//! 3. [`QrSackerSellObserver`]:
//!    Continuous QR decomposition framework (Steyer & Van Vleck 2018) for $Y'(t) = J(t) Y(t)$,
//!    $Y(t) = Q(t) R(t)$.
//!    Tracks diagonal growth rates $\alpha_i(t) = (Q(t)^T J(t) Q(t))_{ii}$, computes max local
//!    rate $\alpha_{\max}(t) = \max_i \alpha_i(t)$, and accumulates $\exp\left(\int_0^t \alpha_{\max}(s) ds\right)$.
//!
//! 4. [`ResolventNumericalAbscissaObserver`]:
//!    Computes $\mu_2(J) = \lambda_{\max}((J + J^T)/2)$ and spectral abscissa $\alpha(J) = \max_i \mathrm{Re}(\lambda_i)$
//! 4. [`ResolventNumericalAbscissaObserver`]:
//!    Computes $\mu_2(J) = \lambda_{\max}((J + J^T)/2)$ and spectral abscissa $\alpha(J) = \max_i \mathrm{Re}(\lambda_i)$
//!    (Higham & Trefethen 1993).
//!    Detects transient nonnormal amplification when $\alpha(J) < 0$ but $\mu_2(J) > 0$.
//!    Computes resolvent lower/upper estimates (Kreiss matrix bound).
//!
//! 5. [`ValidatedSlabEnclosureObserver`]:
//!    Rigorous continuous-time full-slab enclosure observer (Finding OBS-001 remediation).
//!    Requires full-slab enclosures for $J(t) \in \mathbf{J}$, $H(t) \in \mathbf{H}$, and $\dot{H}(t) \in \dot{\mathbf{H}}$,
//!    verifies uniform positive definiteness $\lambda_{\min}(H) \ge \underline{\lambda} > 0$ on the entire slab,
//!    encloses the continuous supremum $\overline{\mu}_{\mathrm{slab}} \ge \sup_{s \in [t_n, t_{n+1}]} \mu_H(s)$,
//!    and computes cumulative growth bounds with outward interval exponential $\exp([\int_0^t \mu_H(s) ds])$
//!    via Cody-Waite reduction and directed rounding (`crate::directed::exp_interval`).
//!    Only this verified full-slab mode reports [`GrowthBoundStatus::Certified`].
//!    Sampled modes (1-3) report [`GrowthBoundStatus::SampledDiagnostic`] because finite pointwise samples
//!    do NOT bound the continuous-time supremum over an entire time slab.

use serde::{Deserialize, Serialize};

use crate::{
    CoreError, DenseMatrix,
    directed::{
        Interval, add_down, add_up, div_up, exp_interval, mul_down, mul_up, sqrt_down, sqrt_up,
        sub_down,
    },
};

/// Certification status for a growth bound or rate estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthBoundStatus {
    /// Outward or analytically exact mathematical bound with guaranteed containment
    /// over the full continuous time slab [t_n, t_{n+1}], enclosing all operator variations,
    /// quadrature remainders, metric jumps, and floating-point roundoff.
    Certified,
    /// Sampled, Ritz, empirical, or uncertified point estimate.
    EstimateOnly,
    /// Sampled diagnostic evaluating pointwise rates along a discrete trajectory
    /// (e.g. discrete samples of $\mu_H(t_k)$ or $\alpha_{\max}(t_k)$); does NOT bound
    /// the continuous-time supremum over the entire time slab $[t_n, t_{n+1}]$.
    SampledDiagnostic,
}

impl GrowthBoundStatus {
    /// Returns true if this status represents a mathematically certified bound.
    #[inline]
    pub fn is_certified(self) -> bool {
        matches!(self, Self::Certified)
    }

    /// Returns true if this status represents an uncertified estimate or sampled diagnostic.
    #[inline]
    pub fn is_sampled_or_estimate(self) -> bool {
        matches!(self, Self::EstimateOnly | Self::SampledDiagnostic)
    }
}

/// Work counters tracking all observer operations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverWorkCounters {
    pub setup_work: usize,
    pub rhs_calls: usize,
    pub qr_steps: usize,
    pub eigen_solves: usize,
    pub metric_updates: usize,
    pub jump_factor_computations: usize,
    pub slab_enclosures: usize,
    pub hot_loop_allocations: usize,
}

impl ObserverWorkCounters {
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------
// 2x2 Linear Algebra Helpers (Zero-Allocation, In-Place Stack Arrays)
// ---------------------------------------------------------------------------

#[inline]
pub fn is_finite_2x2(m: &[[f64; 2]; 2]) -> bool {
    m[0][0].is_finite() && m[0][1].is_finite() && m[1][0].is_finite() && m[1][1].is_finite()
}

#[inline]
pub fn mat_transpose_2x2(a: &[[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [[a[0][0], a[1][0]], [a[0][1], a[1][1]]]
}

#[inline]
pub fn mat_mul_2x2(a: &[[f64; 2]; 2], b: &[[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ],
    ]
}

#[inline]
pub fn is_spd_2x2(h: &[[f64; 2]; 2]) -> Result<(), CoreError> {
    if !is_finite_2x2(h) {
        return Err(CoreError::NonFinite(
            "metric H contains non-finite entries".into(),
        ));
    }
    let sym_diff = (h[0][1] - h[1][0]).abs();
    let scale = (h[0][0].abs() + h[1][1].abs() + 1.0) * 1e-12;
    if sym_diff > scale {
        return Err(CoreError::InvalidInput("metric H must be symmetric".into()));
    }
    if h[0][0] <= 0.0 || h[1][1] <= 0.0 {
        return Err(CoreError::InvalidInput(
            "metric H diagonal entries must be strictly positive".into(),
        ));
    }
    let det = h[0][0] * h[1][1] - h[0][1] * h[1][0];
    if det <= 0.0 {
        return Err(CoreError::InvalidInput(
            "metric H determinant must be strictly positive".into(),
        ));
    }
    Ok(())
}

/// Compute $H^{-1/2}$ for a symmetric positive-definite 2x2 matrix $H$.
///
/// Uses the exact closed form:
/// $d = \sqrt{\det(H)}, t = \mathrm{tr}(H)$,
/// $H^{-1/2} = \frac{1}{d \sqrt{t + 2d}} \begin{pmatrix} h_{11} + d & -h_{01} \\ -h_{10} & h_{00} + d \end{pmatrix}$.
#[inline]
pub fn h_inv_sqrt_2x2(h: &[[f64; 2]; 2]) -> Result<[[f64; 2]; 2], CoreError> {
    is_spd_2x2(h)?;
    let d = (h[0][0] * h[1][1] - h[0][1] * h[1][0]).sqrt();
    let t = h[0][0] + h[1][1];
    let denom = d * (t + 2.0 * d).sqrt();
    if denom <= 0.0 || !denom.is_finite() {
        return Err(CoreError::InvalidInput(
            "metric H is singular or ill-conditioned".into(),
        ));
    }
    Ok([
        [(h[1][1] + d) / denom, -h[0][1] / denom],
        [-h[1][0] / denom, (h[0][0] + d) / denom],
    ])
}

/// Largest eigenvalue of a symmetric 2x2 matrix $M$.
#[inline]
pub fn lambda_max_sym_2x2(m: &[[f64; 2]; 2]) -> f64 {
    let diff = m[0][0] - m[1][1];
    let off = 0.5 * (m[0][1] + m[1][0]);
    let disc = (diff * diff + 4.0 * off * off).sqrt();
    0.5 * (m[0][0] + m[1][1] + disc)
}

/// Norm jump factor $\gamma_{\mathrm{jump}} = \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}$.
#[inline]
pub fn jump_factor_2x2(h_minus: &[[f64; 2]; 2], h_plus: &[[f64; 2]; 2]) -> Result<f64, CoreError> {
    let w = h_inv_sqrt_2x2(h_minus)?;
    is_spd_2x2(h_plus)?;
    let p = mat_mul_2x2(h_plus, &w);
    let g = mat_mul_2x2(&w, &p);
    let g_sym = [
        [g[0][0], 0.5 * (g[0][1] + g[1][0])],
        [0.5 * (g[0][1] + g[1][0]), g[1][1]],
    ];
    let lam = lambda_max_sym_2x2(&g_sym);
    if lam <= 0.0 || !lam.is_finite() {
        return Err(CoreError::InvalidInput(
            "invalid non-positive metric jump eigenvalue".into(),
        ));
    }
    Ok(lam.sqrt())
}

// ---------------------------------------------------------------------------
// 1. FixedHLogNormObserver
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum FixedHMetric {
    Identity {
        dim: usize,
    },
    Diagonal {
        diag: Vec<f64>,
    },
    Dense2x2 {
        h: [[f64; 2]; 2],
        h_inv_sqrt: [[f64; 2]; 2],
    },
    Dense {
        dim: usize,
        h: DenseMatrix,
        h_inv_sqrt: DenseMatrix,
    },
}

/// Computes weighted logarithmic norm $\mu_H(J)$ for a fixed SPD metric $H$.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedHLogNormObserver {
    metric: FixedHMetric,
    dim: usize,
    pub work_counters: ObserverWorkCounters,
}

impl FixedHLogNormObserver {
    /// Construct with identity metric $H = I_n$ (recovering Euclidean $\mu_2(J)$).
    pub fn identity(dim: usize) -> Self {
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Self {
            metric: FixedHMetric::Identity { dim },
            dim,
            work_counters: counters,
        }
    }

    /// Construct with diagonal metric $H = \mathrm{diag}(d)$.
    pub fn diagonal(diag: &[f64]) -> Result<Self, CoreError> {
        let dim = diag.len();
        if dim == 0 {
            return Err(CoreError::Dimension("diagonal cannot be empty".into()));
        }
        for (i, &d) in diag.iter().enumerate() {
            if !d.is_finite() || d <= 0.0 {
                return Err(CoreError::InvalidInput(format!(
                    "diagonal entry {i} must be positive and finite, got {d}"
                )));
            }
        }
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Ok(Self {
            metric: FixedHMetric::Diagonal {
                diag: diag.to_vec(),
            },
            dim,
            work_counters: counters,
        })
    }

    /// Construct with fixed 2x2 SPD matrix $H$.
    pub fn from_2x2(h: [[f64; 2]; 2]) -> Result<Self, CoreError> {
        let h_inv_sqrt = h_inv_sqrt_2x2(&h)?;
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Ok(Self {
            metric: FixedHMetric::Dense2x2 { h, h_inv_sqrt },
            dim: 2,
            work_counters: counters,
        })
    }

    /// Dimension of the observer.
    #[inline]
    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Certification status of the bound.
    ///
    /// Evaluates pointwise logarithmic norm $\mu_H(J)$ at discrete sample points.
    /// Pointwise samples do NOT bound continuous-time evolution over a time slab
    /// in a time-dependent or trajectory context.
    /// Advertises [`GrowthBoundStatus::SampledDiagnostic`].
    #[inline]
    pub fn status(&self) -> GrowthBoundStatus {
        GrowthBoundStatus::SampledDiagnostic
    }

    /// Directly computes Euclidean numerical abscissa $\mu_2(J) = \lambda_{\max}((J + J^T)/2)$.
    #[inline]
    pub fn compute_mu_euclidean_2x2(j: &[[f64; 2]; 2]) -> f64 {
        let sym = [
            [j[0][0], 0.5 * (j[0][1] + j[1][0])],
            [0.5 * (j[0][1] + j[1][0]), j[1][1]],
        ];
        lambda_max_sym_2x2(&sym)
    }

    /// Computes $\mu_H(J)$ for a 2x2 Jacobian matrix $J$ (Zero-Allocation Hot Loop).
    pub fn compute_mu_2x2(&mut self, j: &[[f64; 2]; 2]) -> Result<f64, CoreError> {
        if !is_finite_2x2(j) {
            return Err(CoreError::NonFinite(
                "Jacobian contains non-finite entries".into(),
            ));
        }
        self.work_counters.eigen_solves += 1;
        match &self.metric {
            FixedHMetric::Identity { .. } => Ok(Self::compute_mu_euclidean_2x2(j)),
            FixedHMetric::Dense2x2 { h, h_inv_sqrt } => {
                let hj = mat_mul_2x2(h, j);
                let k = [
                    [2.0 * hj[0][0], hj[0][1] + hj[1][0]],
                    [hj[1][0] + hj[0][1], 2.0 * hj[1][1]],
                ];
                let p = mat_mul_2x2(&k, h_inv_sqrt);
                let wkw = mat_mul_2x2(h_inv_sqrt, &p);
                let m = [
                    [0.5 * wkw[0][0], 0.25 * (wkw[0][1] + wkw[1][0])],
                    [0.25 * (wkw[0][1] + wkw[1][0]), 0.5 * wkw[1][1]],
                ];
                Ok(lambda_max_sym_2x2(&m))
            }
            FixedHMetric::Diagonal { diag } => {
                if diag.len() != 2 {
                    return Err(CoreError::Dimension("diagonal dimension mismatch".into()));
                }
                let (d0, d1) = (diag[0], diag[1]);
                let s00 = j[0][0];
                let s11 = j[1][1];
                let ratio01 = (d1 / d0).sqrt();
                let ratio10 = (d0 / d1).sqrt();
                let s01 = 0.5 * (ratio01 * j[1][0] + ratio10 * j[0][1]);
                let m = [[s00, s01], [s01, s11]];
                Ok(lambda_max_sym_2x2(&m))
            }
            FixedHMetric::Dense { .. } => Err(CoreError::InvalidInput(
                "use compute_mu_dense for general dense matrix".into(),
            )),
        }
    }

    /// Computes $\mu_H(J)$ for a DenseMatrix.
    pub fn compute_mu(&mut self, j: &DenseMatrix) -> Result<f64, CoreError> {
        if j.nrows() != self.dim || j.ncols() != self.dim {
            return Err(CoreError::Dimension(format!(
                "Jacobian shape {}x{} does not match observer dimension {}",
                j.nrows(),
                j.ncols(),
                self.dim
            )));
        }
        if self.dim == 2 {
            let j_arr = [[j[(0, 0)], j[(0, 1)]], [j[(1, 0)], j[(1, 1)]]];
            self.compute_mu_2x2(&j_arr)
        } else {
            match &self.metric {
                FixedHMetric::Identity { .. } => {
                    self.work_counters.eigen_solves += 1;
                    // For diagonal or general, compute symmetric part
                    let mut s = DenseMatrix::zeros(self.dim, self.dim);
                    for i in 0..self.dim {
                        for k in 0..self.dim {
                            s[(i, k)] = 0.5 * (j[(i, k)] + j[(k, i)]);
                        }
                    }
                    // Power iteration / Lanczos or Gershgorin upper estimate
                    let mut max_diag = f64::NEG_INFINITY;
                    for i in 0..self.dim {
                        let mut row_sum = 0.0;
                        for k in 0..self.dim {
                            if i != k {
                                row_sum += s[(i, k)].abs();
                            }
                        }
                        let upper = s[(i, i)] + row_sum;
                        if upper > max_diag {
                            max_diag = upper;
                        }
                    }
                    Ok(max_diag)
                }
                FixedHMetric::Diagonal { diag } => {
                    self.work_counters.eigen_solves += 1;
                    let mut max_diag = f64::NEG_INFINITY;
                    for i in 0..self.dim {
                        let mut row_sum = 0.0;
                        for k in 0..self.dim {
                            if i != k {
                                let term = 0.5
                                    * ((diag[k] / diag[i]).sqrt() * j[(k, i)]
                                        + (diag[i] / diag[k]).sqrt() * j[(i, k)]);
                                row_sum += term.abs();
                            }
                        }
                        let upper = j[(i, i)] + row_sum;
                        if upper > max_diag {
                            max_diag = upper;
                        }
                    }
                    Ok(max_diag)
                }
                _ => Err(CoreError::InvalidInput(
                    "unsupported dense metric dimension".into(),
                )),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. MovingHMetricObserver (CTSH-VIG-METRICFLOW-004)
// ---------------------------------------------------------------------------

/// Implements candidate CTSH-VIG-METRICFLOW-004 for time-dependent metric $H(t)$.
///
/// Computes:
/// $$M_H(t) = \frac{1}{2} H(t)^{-1/2} \left[ \dot{H}(t) + J(t)^T H(t) + H(t) J(t) \right] H(t)^{-1/2},$$
/// $$\mu_H(t) = \lambda_{\max}(M_H(t)).$$
///
/// Accumulates the growth bound:
/// $$\exp\left( \int_0^t \mu_H(s) ds \right).$$
///
/// On metric switches from $H_-$ to $H_+$, accumulates jump factor:
/// $$\gamma_{\mathrm{jump}} = \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}.$$
#[derive(Clone, Debug, PartialEq)]
pub struct MovingHMetricObserver {
    dim: usize,
    current_h_2x2: [[f64; 2]; 2],
    integrated_exponent: f64,
    cumulative_jump_factor: f64,
    cumulative_growth_bound: f64,
    current_mu: f64,
    pub work_counters: ObserverWorkCounters,
}

impl MovingHMetricObserver {
    /// Construct a new moving metric observer with initial 2x2 SPD metric $H(0)$.
    pub fn new_2x2(initial_h: [[f64; 2]; 2]) -> Result<Self, CoreError> {
        is_spd_2x2(&initial_h)?;
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Ok(Self {
            dim: 2,
            current_h_2x2: initial_h,
            integrated_exponent: 0.0,
            cumulative_jump_factor: 1.0,
            cumulative_growth_bound: 1.0,
            current_mu: 0.0,
            work_counters: counters,
        })
    }

    /// Construct with identity metric $H(0) = I_2$.
    pub fn identity_2x2() -> Self {
        let initial_h = [[1.0, 0.0], [0.0, 1.0]];
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Self {
            dim: 2,
            current_h_2x2: initial_h,
            integrated_exponent: 0.0,
            cumulative_jump_factor: 1.0,
            cumulative_growth_bound: 1.0,
            current_mu: 0.0,
            work_counters: counters,
        }
    }

    /// Certification status of the bound.
    ///
    /// Discrete pointwise integration $\sum \mu_H(t_k) \Delta t$ with ordinary f64 arithmetic
    /// evaluates sampled growth rates. It does NOT bound the continuous-time supremum over
    /// the full slab $[t_n, t_{n+1}]$ (e.g. rapidly oscillating $\dot{H}$ or interior peaks
    /// between samples are missed). Advertises [`GrowthBoundStatus::SampledDiagnostic`].
    #[inline]
    pub fn status(&self) -> GrowthBoundStatus {
        GrowthBoundStatus::SampledDiagnostic
    }

    /// Computes instantaneous growth rate $\mu_H(t) = \lambda_{\max}(M_H(t))$ for 2x2.
    pub fn compute_mu_h_2x2(
        &mut self,
        j: &[[f64; 2]; 2],
        h: &[[f64; 2]; 2],
        hdot: &[[f64; 2]; 2],
    ) -> Result<f64, CoreError> {
        if !is_finite_2x2(j) || !is_finite_2x2(h) || !is_finite_2x2(hdot) {
            return Err(CoreError::NonFinite(
                "non-finite matrix passed to compute_mu_h_2x2".into(),
            ));
        }
        let w = h_inv_sqrt_2x2(h)?;
        let hj = mat_mul_2x2(h, j);
        let k = [
            [
                hdot[0][0] + 2.0 * hj[0][0],
                hdot[0][1] + hj[0][1] + hj[1][0],
            ],
            [
                hdot[1][0] + hj[1][0] + hj[0][1],
                hdot[1][1] + 2.0 * hj[1][1],
            ],
        ];
        let p = mat_mul_2x2(&k, &w);
        let wkw = mat_mul_2x2(&w, &p);
        let m_h = [
            [0.5 * wkw[0][0], 0.25 * (wkw[0][1] + wkw[1][0])],
            [0.25 * (wkw[0][1] + wkw[1][0]), 0.5 * wkw[1][1]],
        ];
        let mu_h = lambda_max_sym_2x2(&m_h);
        self.current_h_2x2 = *h;
        self.current_mu = mu_h;
        self.work_counters.eigen_solves += 1;
        self.work_counters.metric_updates += 1;
        Ok(mu_h)
    }

    /// Integrate one step $\Delta t$ with time-dependent metric $H(t)$ and $\dot{H}(t)$.
    ///
    /// Updates integrated growth bound and returns instantaneous rate $\mu_H(t)$.
    pub fn step_2x2(
        &mut self,
        dt: f64,
        j: &[[f64; 2]; 2],
        h: &[[f64; 2]; 2],
        hdot: &[[f64; 2]; 2],
    ) -> Result<f64, CoreError> {
        if !(dt > 0.0 && dt.is_finite()) {
            return Err(CoreError::InvalidInput(
                "step size dt must be positive and finite".into(),
            ));
        }
        let mu_h = self.compute_mu_h_2x2(j, h, hdot)?;
        self.integrated_exponent += mu_h * dt;
        self.cumulative_growth_bound = self.cumulative_jump_factor * self.integrated_exponent.exp();
        Ok(mu_h)
    }

    /// Switch metric discontinuously from current $H_-$ to new $H_+$.
    ///
    /// Bounding norm jump by $\gamma_{\mathrm{jump}} = \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}$,
    /// and multiplying cumulative growth bound by $\gamma_{\mathrm{jump}}$.
    pub fn switch_metric_2x2(&mut self, h_new: [[f64; 2]; 2]) -> Result<f64, CoreError> {
        let gamma_jump = jump_factor_2x2(&self.current_h_2x2, &h_new)?;
        self.cumulative_jump_factor *= gamma_jump;
        self.cumulative_growth_bound *= gamma_jump;
        self.current_h_2x2 = h_new;
        self.work_counters.jump_factor_computations += 1;
        self.work_counters.metric_updates += 1;
        Ok(gamma_jump)
    }

    /// Direct helper to compute metric jump factor between two 2x2 SPD matrices.
    #[inline]
    pub fn compute_jump_factor_2x2(
        h_minus: [[f64; 2]; 2],
        h_plus: [[f64; 2]; 2],
    ) -> Result<f64, CoreError> {
        jump_factor_2x2(&h_minus, &h_plus)
    }

    #[inline]
    pub fn cumulative_growth_bound(&self) -> f64 {
        self.cumulative_growth_bound
    }

    #[inline]
    pub fn integrated_exponent(&self) -> f64 {
        self.integrated_exponent
    }

    #[inline]
    pub fn current_mu(&self) -> f64 {
        self.current_mu
    }

    #[inline]
    pub fn cumulative_jump_factor(&self) -> f64 {
        self.cumulative_jump_factor
    }

    #[inline]
    pub fn current_h(&self) -> [[f64; 2]; 2] {
        self.current_h_2x2
    }
}

// ---------------------------------------------------------------------------
// 3. QrSackerSellObserver (Steyer & Van Vleck 2018)
// ---------------------------------------------------------------------------

/// Continuous QR decomposition framework (Steyer & Van Vleck 2018).
///
/// Tracks continuous evolution $Y'(t) = J(t) Y(t)$ with $Y(t) = Q(t) R(t)$.
/// Diagonal rates: $\alpha_i(t) = (Q(t)^T J(t) Q(t))_{ii}$.
/// Maximum rate: $\alpha_{\max}(t) = \max_i \alpha_i(t)$.
/// Cumulative bound: $\exp\left(\int_0^t \alpha_{\max}(s) ds\right)$.
#[derive(Clone, Debug, PartialEq)]
pub struct QrSackerSellObserver {
    dim: usize,
    q_2x2: [[f64; 2]; 2],
    current_alpha: [f64; 2],
    current_alpha_max: f64,
    integrated_exponent: f64,
    cumulative_growth_bound: f64,
    pub work_counters: ObserverWorkCounters,
}

impl QrSackerSellObserver {
    /// Construct with identity initial orthogonal basis $Q(0) = I_2$.
    pub fn new_2x2() -> Self {
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Self {
            dim: 2,
            q_2x2: [[1.0, 0.0], [0.0, 1.0]],
            current_alpha: [0.0, 0.0],
            current_alpha_max: 0.0,
            integrated_exponent: 0.0,
            cumulative_growth_bound: 1.0,
            work_counters: counters,
        }
    }

    /// Construct with explicit initial orthogonal matrix $Q(0)$.
    pub fn with_initial_q_2x2(q: [[f64; 2]; 2]) -> Result<Self, CoreError> {
        if !is_finite_2x2(&q) {
            return Err(CoreError::NonFinite(
                "initial Q contains non-finite entries".into(),
            ));
        }
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Ok(Self {
            dim: 2,
            q_2x2: q,
            current_alpha: [0.0, 0.0],
            current_alpha_max: 0.0,
            integrated_exponent: 0.0,
            cumulative_growth_bound: 1.0,
            work_counters: counters,
        })
    }

    /// Certification status of the bound.
    ///
    /// Continuous QR with discrete time-stepping $(I + \Delta t J) Q$ and Gram-Schmidt
    /// orthonormalization without outward error enclosures and orthogonality defect tracking
    /// evaluates sampled local rates $\alpha_{\max}(t_k)$ along a trajectory.
    /// It does NOT certify continuous-time growth over the full slab $[t_n, t_{n+1}]$.
    /// Advertises [`GrowthBoundStatus::SampledDiagnostic`].
    #[inline]
    pub fn status(&self) -> GrowthBoundStatus {
        GrowthBoundStatus::SampledDiagnostic
    }

    /// Compute orthogonality defect of the current basis matrix $Q$:
    /// $\|Q^T Q - I\|_{\max} = \max_{i,j} |(Q^T Q)_{ij} - \delta_{ij}|$.
    #[inline]
    pub fn orthogonality_defect(&self) -> f64 {
        let qt = mat_transpose_2x2(&self.q_2x2);
        let q_prod = mat_mul_2x2(&qt, &self.q_2x2);
        let d00 = (q_prod[0][0] - 1.0).abs();
        let d01 = q_prod[0][1].abs();
        let d10 = q_prod[1][0].abs();
        let d11 = (q_prod[1][1] - 1.0).abs();
        d00.max(d01).max(d10).max(d11)
    }

    /// Computes diagonal growth rates $\alpha_i(t) = (Q(t)^T J(t) Q(t))_{ii}$ and $\alpha_{\max}$.
    pub fn compute_rates_2x2(&mut self, j: &[[f64; 2]; 2]) -> ([f64; 2], f64) {
        let qt = mat_transpose_2x2(&self.q_2x2);
        let jq = mat_mul_2x2(j, &self.q_2x2);
        let b = mat_mul_2x2(&qt, &jq);
        let a1 = b[0][0];
        let a2 = b[1][1];
        let a_max = a1.max(a2);
        self.current_alpha = [a1, a2];
        self.current_alpha_max = a_max;
        ([a1, a2], a_max)
    }

    /// Advance one step $\Delta t$, updating $Q(t + \Delta t)$ via QR factorization of $(I + \Delta t J) Q$.
    pub fn step_2x2(&mut self, dt: f64, j: &[[f64; 2]; 2]) -> Result<f64, CoreError> {
        if !(dt > 0.0 && dt.is_finite()) {
            return Err(CoreError::InvalidInput(
                "dt must be positive and finite".into(),
            ));
        }
        if !is_finite_2x2(j) {
            return Err(CoreError::NonFinite(
                "Jacobian contains non-finite entries".into(),
            ));
        }
        let (_, a_max) = self.compute_rates_2x2(j);
        self.integrated_exponent += a_max * dt;
        self.cumulative_growth_bound = self.integrated_exponent.exp();

        // Advance Q(t) via QR of Z = (I + dt J) Q
        let id_plus_dt_j = [
            [1.0 + dt * j[0][0], dt * j[0][1]],
            [dt * j[1][0], 1.0 + dt * j[1][1]],
        ];
        let z = mat_mul_2x2(&id_plus_dt_j, &self.q_2x2);

        // Gram-Schmidt orthonormalization
        let r00 = (z[0][0] * z[0][0] + z[1][0] * z[1][0]).sqrt();
        if r00 <= 1e-15 || !r00.is_finite() {
            return Err(CoreError::LinearSolve(
                "continuous QR column 0 collapsed".into(),
            ));
        }
        let q00 = z[0][0] / r00;
        let q10 = z[1][0] / r00;

        let r01 = q00 * z[0][1] + q10 * z[1][1];
        let w0 = z[0][1] - r01 * q00;
        let w1 = z[1][1] - r01 * q10;
        let r11 = (w0 * w0 + w1 * w1).sqrt();
        if r11 <= 1e-15 || !r11.is_finite() {
            return Err(CoreError::LinearSolve(
                "continuous QR column 1 collapsed".into(),
            ));
        }
        let q01 = w0 / r11;
        let q11 = w1 / r11;

        self.q_2x2 = [[q00, q01], [q10, q11]];
        self.work_counters.qr_steps += 1;

        Ok(a_max)
    }

    #[inline]
    pub fn current_q_2x2(&self) -> [[f64; 2]; 2] {
        self.q_2x2
    }

    #[inline]
    pub fn current_alpha(&self) -> [f64; 2] {
        self.current_alpha
    }

    #[inline]
    pub fn current_alpha_max(&self) -> f64 {
        self.current_alpha_max
    }

    #[inline]
    pub fn integrated_exponent(&self) -> f64 {
        self.integrated_exponent
    }

    #[inline]
    pub fn cumulative_growth_bound(&self) -> f64 {
        self.cumulative_growth_bound
    }
}

// ---------------------------------------------------------------------------
// 4. ResolventNumericalAbscissaObserver (Higham & Trefethen 1993)
// ---------------------------------------------------------------------------

/// Diagnostic report containing spectral, numerical abscissa, and resolvent bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolventAnalysis {
    pub spectral_abscissa: f64,
    pub numerical_abscissa: f64,
    pub has_transient_amplification: bool,
    pub kreiss_lower_bound: f64,
    pub kreiss_status: GrowthBoundStatus,
    pub resolvent_upper_estimate: f64,
    pub upper_status: GrowthBoundStatus,
}

/// Resolvent and numerical abscissa observer (Higham & Trefethen 1993).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolventNumericalAbscissaObserver {
    pub work_counters: ObserverWorkCounters,
}

impl ResolventNumericalAbscissaObserver {
    pub fn new() -> Self {
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Self {
            work_counters: counters,
        }
    }

    /// Spectral abscissa $\alpha(J) = \max_i \mathrm{Re}(\lambda_i)$.
    #[inline]
    pub fn spectral_abscissa_2x2(j: &[[f64; 2]; 2]) -> f64 {
        let tr = j[0][0] + j[1][1];
        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        let disc = tr * tr - 4.0 * det;
        if disc >= 0.0 {
            0.5 * (tr + disc.sqrt())
        } else {
            0.5 * tr
        }
    }

    /// Euclidean numerical abscissa $\mu_2(J) = \lambda_{\max}((J + J^T)/2)$.
    #[inline]
    pub fn numerical_abscissa_2x2(j: &[[f64; 2]; 2]) -> f64 {
        FixedHLogNormObserver::compute_mu_euclidean_2x2(j)
    }

    /// Computes resolvent 2-norm $\|(s I - J)^{-1}\|_2$ for real $s > \alpha(J)$.
    pub fn resolvent_norm_2x2(&self, j: &[[f64; 2]; 2], s: f64) -> Result<f64, CoreError> {
        let a = [[s - j[0][0], -j[0][1]], [-j[1][0], s - j[1][1]]];
        let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
        if det.abs() <= 1e-15 || !det.is_finite() {
            return Err(CoreError::LinearSolve(
                "resolvent matrix is singular at evaluation point".into(),
            ));
        }
        let inv_a = [
            [a[1][1] / det, -a[0][1] / det],
            [-a[1][0] / det, a[0][0] / det],
        ];
        let m = mat_mul_2x2(&inv_a, &mat_transpose_2x2(&inv_a));
        let m_sym = [
            [m[0][0], 0.5 * (m[0][1] + m[1][0])],
            [0.5 * (m[0][1] + m[1][0]), m[1][1]],
        ];
        let lam_max = lambda_max_sym_2x2(&m_sym);
        Ok(lam_max.max(0.0).sqrt())
    }

    /// Sampled Kreiss matrix theorem lower bound $\sup_{s > 0} s \|(s I - J)^{-1}\|_2$.
    ///
    /// Note: because this uses sampled evaluation, its status is [`GrowthBoundStatus::EstimateOnly`].
    pub fn kreiss_lower_bound_2x2(&mut self, j: &[[f64; 2]; 2]) -> Result<f64, CoreError> {
        let mut max_k = 1.0_f64; // At t=0, ||exp(t J)||_2 = 1.0
        let alpha = Self::spectral_abscissa_2x2(j);
        let s_min = (alpha + 0.01).max(0.01);

        // Bounded loop over 100 geometrically spaced points in [s_min, 100.0]
        for step in 0..100 {
            let ratio = step as f64 / 99.0;
            let s = s_min * (100.0 / s_min).powf(ratio);
            if let Ok(norm) = self.resolvent_norm_2x2(j, s) {
                let k = s * norm;
                if k > max_k {
                    max_k = k;
                }
            }
        }
        self.work_counters.eigen_solves += 1;
        Ok(max_k)
    }

    /// Comprehensive analysis of 2x2 matrix $J$.
    pub fn analyze_2x2(&mut self, j: &[[f64; 2]; 2]) -> Result<ResolventAnalysis, CoreError> {
        if !is_finite_2x2(j) {
            return Err(CoreError::NonFinite(
                "Jacobian contains non-finite entries".into(),
            ));
        }
        let alpha = Self::spectral_abscissa_2x2(j);
        let mu2 = Self::numerical_abscissa_2x2(j);
        self.work_counters.eigen_solves += 2;

        let has_transient = alpha < 0.0 && mu2 > 0.0;
        let kreiss_lower = self.kreiss_lower_bound_2x2(j)?;

        // Upper estimate from Crouzeix numerical range bound
        let resolvent_upper = if mu2 <= 0.0 {
            1.0
        } else {
            let decay_ratio = if alpha < 0.0 {
                (mu2 / (-alpha)).max(0.0)
            } else {
                mu2
            };
            (1.0 + std::f64::consts::SQRT_2) * (1.0 + decay_ratio)
        };

        Ok(ResolventAnalysis {
            spectral_abscissa: alpha,
            numerical_abscissa: mu2,
            has_transient_amplification: has_transient,
            kreiss_lower_bound: kreiss_lower,
            kreiss_status: GrowthBoundStatus::EstimateOnly,
            resolvent_upper_estimate: resolvent_upper,
            upper_status: GrowthBoundStatus::EstimateOnly,
        })
    }
}

// ---------------------------------------------------------------------------
// 5. ValidatedSlabEnclosureObserver (Finding OBS-001 Remediation)
// ---------------------------------------------------------------------------

/// Outward interval square $X^2 = \{ x^2 : x \in X \}$.
/// Guaranteed non-negative lower bound $\ge 0$.
#[inline]
pub fn interval_sqr(x: Interval) -> Result<Interval, CoreError> {
    if !x.lo.is_finite() || !x.hi.is_finite() {
        return Err(CoreError::NonFinite(
            "interval square of non-finite bounds".into(),
        ));
    }
    let max_abs = x.lo.abs().max(x.hi.abs());
    let hi = mul_up(max_abs, max_abs)?;
    let lo = if x.contains_zero() {
        0.0
    } else {
        let min_abs = x.lo.abs().min(x.hi.abs());
        mul_down(min_abs, min_abs)?
    };
    Interval::new(lo, hi)
}

/// Outward interval square root $\sqrt{[\underline{x}, \overline{x}]}$.
#[inline]
pub fn interval_sqrt(x: Interval) -> Result<Interval, CoreError> {
    if x.hi < 0.0 {
        return Err(CoreError::InvalidInput(
            "interval square root of strictly negative interval".into(),
        ));
    }
    let lo = if x.lo <= 0.0 { 0.0 } else { sqrt_down(x.lo)? };
    let hi = sqrt_up(x.hi.max(0.0))?;
    Interval::new(lo, hi)
}

/// 2x2 matrix multiplication in outward interval arithmetic.
#[inline]
pub fn interval_mat_mul_2x2(
    a: &[[Interval; 2]; 2],
    b: &[[Interval; 2]; 2],
) -> Result<[[Interval; 2]; 2], CoreError> {
    let c00 = a[0][0].mul(b[0][0])?.add(a[0][1].mul(b[1][0])?)?;
    let c01 = a[0][0].mul(b[0][1])?.add(a[0][1].mul(b[1][1])?)?;
    let c10 = a[1][0].mul(b[0][0])?.add(a[1][1].mul(b[1][0])?)?;
    let c11 = a[1][0].mul(b[0][1])?.add(a[1][1].mul(b[1][1])?)?;
    Ok([[c00, c01], [c10, c11]])
}

/// Convert standard 2x2 f64 matrix to degenerate (exact) point intervals.
#[inline]
pub fn point_interval_matrix_2x2(a: &[[f64; 2]; 2]) -> Result<[[Interval; 2]; 2], CoreError> {
    if !is_finite_2x2(a) {
        return Err(CoreError::NonFinite(
            "cannot create interval matrix from non-finite f64 matrix".into(),
        ));
    }
    let m00 = Interval::point(a[0][0])?;
    let m01 = Interval::point(a[0][1])?;
    let m10 = Interval::point(a[1][0])?;
    let m11 = Interval::point(a[1][1])?;
    Ok([[m00, m01], [m10, m11]])
}

/// Verify that interval matrix H is symmetric and uniformly strictly positive-definite on the entire slab.
/// Returns outward eigenvalue bounds `(lambda_min_lo, lambda_max_hi)` with `lambda_min_lo > 0`.
pub fn verify_spd_interval_2x2(h: &[[Interval; 2]; 2]) -> Result<(f64, f64), CoreError> {
    for row in h {
        for entry in row {
            if !entry.lo.is_finite() || !entry.hi.is_finite() {
                return Err(CoreError::NonFinite(
                    "metric H contains non-finite interval bounds".into(),
                ));
            }
        }
    }
    // Symmetry: off-diagonals must overlap
    let overlap_lo = h[0][1].lo.max(h[1][0].lo);
    let overlap_hi = h[0][1].hi.min(h[1][0].hi);
    if overlap_lo > overlap_hi {
        return Err(CoreError::InvalidInput(
            "metric H off-diagonal intervals do not overlap (not symmetric)".into(),
        ));
    }
    let h01_sym = Interval::new(overlap_lo, overlap_hi)?;

    // Diagonal positivity
    if h[0][0].lo <= 0.0 || h[1][1].lo <= 0.0 {
        return Err(CoreError::InvalidInput(
            "metric H diagonal lower bounds must be strictly positive".into(),
        ));
    }

    // Determinant lower bound: h00 * h11 - h01^2
    let h00_h11 = h[0][0].mul(h[1][1])?;
    let h01_sq = interval_sqr(h01_sym)?;
    let det = h00_h11.sub(h01_sq)?;
    if det.lo <= 0.0 {
        return Err(CoreError::InvalidInput(
            "metric H determinant is not strictly positive on slab".into(),
        ));
    }

    // Eigenvalues: 1/2 [ (h00 + h11) +/- sqrt( (h00 - h11)^2 + 4 h01^2 ) ]
    let tr = h[0][0].add(h[1][1])?;
    let diff = h[0][0].sub(h[1][1])?;
    let diff_sq = interval_sqr(diff)?;
    let four = Interval::point(4.0)?;
    let four_off = four.mul(h01_sq)?;
    let disc = diff_sq.add(four_off)?;
    let sqrt_disc = interval_sqrt(disc)?;

    let lambda_min_lo = 0.5 * sub_down(tr.lo, sqrt_disc.hi)?;
    if lambda_min_lo <= 0.0 {
        return Err(CoreError::InvalidInput(
            "metric H minimum eigenvalue lower bound is not strictly positive".into(),
        ));
    }
    let lambda_max_hi = 0.5 * add_up(tr.hi, sqrt_disc.hi)?;
    Ok((lambda_min_lo, lambda_max_hi))
}

/// Outward upper bound on lambda_max(K) for symmetric 2x2 interval matrix K.
pub fn lambda_max_sym_interval_2x2(k: &[[Interval; 2]; 2]) -> Result<f64, CoreError> {
    let tr = k[0][0].add(k[1][1])?;
    let diff = k[0][0].sub(k[1][1])?;
    let diff_sq = interval_sqr(diff)?;
    let off_sym = Interval::new(k[0][1].lo.min(k[1][0].lo), k[0][1].hi.max(k[1][0].hi))?;
    let four_off = Interval::point(4.0)?.mul(interval_sqr(off_sym)?)?;
    let disc = diff_sq.add(four_off)?;
    let sqrt_disc = interval_sqrt(disc)?;
    let lam_hi = 0.5 * add_up(tr.hi, sqrt_disc.hi)?;
    Ok(lam_hi)
}

/// Computes $H^{-1/2}$ for an interval 2x2 SPD matrix $H$.
pub fn h_inv_sqrt_interval_2x2(h: &[[Interval; 2]; 2]) -> Result<[[Interval; 2]; 2], CoreError> {
    let (lambda_min, _) = verify_spd_interval_2x2(h)?;
    if lambda_min <= 0.0 {
        return Err(CoreError::InvalidInput("metric H is not SPD".into()));
    }
    let overlap_lo = h[0][1].lo.max(h[1][0].lo);
    let overlap_hi = h[0][1].hi.min(h[1][0].hi);
    let h01_sym = Interval::new(overlap_lo, overlap_hi)?;
    let h01_sq = interval_sqr(h01_sym)?;
    let det = h[0][0].mul(h[1][1])?.sub(h01_sq)?;
    let d = interval_sqrt(det)?;
    let tr = h[0][0].add(h[1][1])?;
    let two_d = d.scale(2.0)?;
    let tr_plus_2d = tr.add(two_d)?;
    let sqrt_t_2d = interval_sqrt(tr_plus_2d)?;
    let denom = d.mul(sqrt_t_2d)?;
    if denom.lo <= 0.0 {
        return Err(CoreError::InvalidInput(
            "metric H square root inverse denominator not strictly positive".into(),
        ));
    }
    let w00 = h[1][1].add(d)?.div(denom)?;
    let neg_h01 = -h01_sym;
    let w01 = neg_h01.div(denom)?;
    let w10 = w01;
    let w11 = h[0][0].add(d)?.div(denom)?;
    Ok([[w00, w01], [w10, w11]])
}

/// Outward upper bound on the continuous supremum $\sup_{s \in [t_n, t_{n+1}]} \mu_H(s)$
/// for time-dependent or enclosed interval operators $J, H, \dot{H}$.
pub fn enclose_slab_mu_2x2(
    j_slab: &[[Interval; 2]; 2],
    h_slab: &[[Interval; 2]; 2],
    hdot_slab: &[[Interval; 2]; 2],
) -> Result<f64, CoreError> {
    let (lambda_min_h, lambda_max_h) = verify_spd_interval_2x2(h_slab)?;

    // Compute H * J
    let hj = interval_mat_mul_2x2(h_slab, j_slab)?;

    // S = H J + (H J)^T
    let s00 = hj[0][0].scale(2.0)?;
    let s01 = hj[0][1].add(hj[1][0])?;
    let _s10 = s01;
    let s11 = hj[1][1].scale(2.0)?;

    // K = S + Hdot
    let k00 = s00.add(hdot_slab[0][0])?;
    let hdot_off_sym = Interval::new(
        0.5 * add_down(hdot_slab[0][1].lo, hdot_slab[1][0].lo)?,
        0.5 * add_up(hdot_slab[0][1].hi, hdot_slab[1][0].hi)?,
    )?;
    let k01 = s01.add(hdot_off_sym)?;
    let k10 = k01;
    let k11 = s11.add(hdot_slab[1][1])?;

    let k = [[k00, k01], [k10, k11]];
    let lam_k_max = lambda_max_sym_interval_2x2(&k)?;

    // Rayleigh quotient bound on M_H = 1/2 H^{-1/2} K H^{-1/2}:
    let rayleigh_upper = if lam_k_max >= 0.0 {
        0.5 * div_up(lam_k_max, lambda_min_h)?
    } else {
        0.5 * div_up(lam_k_max, lambda_max_h)?
    };

    // If H^{-1/2} can be directly inverted, compute M_H = 1/2 W K W
    let direct_upper = (|| -> Result<f64, CoreError> {
        let w = h_inv_sqrt_interval_2x2(h_slab)?;
        let kw = interval_mat_mul_2x2(&k, &w)?;
        let wkw = interval_mat_mul_2x2(&w, &kw)?;
        let m_h = [
            [wkw[0][0].scale(0.5)?, wkw[0][1].scale(0.5)?],
            [wkw[1][0].scale(0.5)?, wkw[1][1].scale(0.5)?],
        ];
        let m_h_max = lambda_max_sym_interval_2x2(&m_h)?;
        Ok(rayleigh_upper.min(m_h_max))
    })();

    if let Ok(val) = direct_upper {
        return Ok(val);
    }

    Ok(rayleigh_upper)
}

/// Outward jump factor upper bound $\gamma_{\mathrm{jump}} = \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}$.
pub fn enclose_jump_factor_2x2(
    h_minus: &[[Interval; 2]; 2],
    h_plus: &[[Interval; 2]; 2],
) -> Result<f64, CoreError> {
    let (lambda_min_minus, _) = verify_spd_interval_2x2(h_minus)?;
    let (_, lambda_max_plus) = verify_spd_interval_2x2(h_plus)?;

    let ratio_hi = div_up(lambda_max_plus, lambda_min_minus)?;
    let rayleigh_jump_hi = sqrt_up(ratio_hi)?;

    // If direct inversion of H_- is available:
    let direct_jump = (|| -> Result<f64, CoreError> {
        let w_minus = h_inv_sqrt_interval_2x2(h_minus)?;
        let h_plus_w = interval_mat_mul_2x2(h_plus, &w_minus)?;
        let g = interval_mat_mul_2x2(&w_minus, &h_plus_w)?;
        let lam_g_max = lambda_max_sym_interval_2x2(&g)?;
        if lam_g_max > 0.0 {
            sqrt_up(lam_g_max)
        } else {
            Ok(rayleigh_jump_hi)
        }
    })();

    if let Ok(direct_jump_hi) = direct_jump {
        return Ok(rayleigh_jump_hi.min(direct_jump_hi));
    }

    Ok(rayleigh_jump_hi)
}

/// Verified full time-slab moving-metric enclosure observer.
///
/// In contrast to sampled observers that evaluate $\mu_H(t_k)$ or $\alpha_{\max}(t_k)$
/// at discrete points (which do NOT bound the continuous supremum over $[t_n, t_{n+1}]$),
/// this observer requires:
/// 1. Continuous slab enclosure for $J(t) \in \mathbf{J}$, $H(t) \in \mathbf{H}$, and $\dot{H}(t) \in \dot{\mathbf{H}}$.
/// 2. Guaranteed uniform positive definiteness lower bound $\underline{\lambda}_H > 0$ on the entire slab.
/// 3. Outward eigenvalue upper bound $\overline{\mu}_{\mathrm{slab}} \ge \sup_{s \in [t_n, t_{n+1}]} \mu_{H(s)}(J(s), \dot{H}(s))$.
/// 4. Outward directed interval exponential $\exp([\int_0^t \mu_H(s) ds])$ with Cody-Waite reduction and roundoff enclosure.
/// 5. Outward enclosure of metric switch jump factors $\gamma_{\mathrm{jump}} = \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}$.
///
/// Only when all conditions are outwardly enclosed and verified does this observer
/// advertise [`GrowthBoundStatus::Certified`].
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedSlabEnclosureObserver {
    dim: usize,
    current_h_2x2: [[Interval; 2]; 2],
    cumulative_exponent_hi: f64,
    cumulative_jump_factor_hi: f64,
    cumulative_growth_bound_hi: f64,
    last_slab_mu_hi: f64,
    pub work_counters: ObserverWorkCounters,
}

impl ValidatedSlabEnclosureObserver {
    /// Construct with identity metric $H(0) = I_2$.
    pub fn identity_2x2() -> Self {
        let one = Interval { lo: 1.0, hi: 1.0 };
        let zero = Interval { lo: 0.0, hi: 0.0 };
        let initial_h = [[one, zero], [zero, one]];
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Self {
            dim: 2,
            current_h_2x2: initial_h,
            cumulative_exponent_hi: 0.0,
            cumulative_jump_factor_hi: 1.0,
            cumulative_growth_bound_hi: 1.0,
            last_slab_mu_hi: 0.0,
            work_counters: counters,
        }
    }

    /// Construct with constant f64 SPD matrix $H(0)$.
    pub fn from_constant_h_2x2(initial_h: [[f64; 2]; 2]) -> Result<Self, CoreError> {
        let h_int = point_interval_matrix_2x2(&initial_h)?;
        Self::new_2x2(h_int)
    }

    /// Construct with interval SPD matrix $H(0)$.
    pub fn new_2x2(initial_h: [[Interval; 2]; 2]) -> Result<Self, CoreError> {
        verify_spd_interval_2x2(&initial_h)?;
        let counters = ObserverWorkCounters {
            setup_work: 1,
            ..Default::default()
        };
        Ok(Self {
            dim: 2,
            current_h_2x2: initial_h,
            cumulative_exponent_hi: 0.0,
            cumulative_jump_factor_hi: 1.0,
            cumulative_growth_bound_hi: 1.0,
            last_slab_mu_hi: 0.0,
            work_counters: counters,
        })
    }

    /// Certification status of the bound.
    ///
    /// Evaluates verified continuous full time-slab enclosures with outward interval
    /// arithmetic, eigenvalue upper bounds, and Cody-Waite exponential enclosures.
    /// Advertises [`GrowthBoundStatus::Certified`].
    #[inline]
    pub fn status(&self) -> GrowthBoundStatus {
        GrowthBoundStatus::Certified
    }

    /// Dimension of the observer.
    #[inline]
    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Integrate one time slab $[t_n, t_{n+1}]$ of duration $\Delta t > 0$,
    /// given full-slab enclosures for $J(t)$, $H(t)$, and $\dot{H}(t)$.
    ///
    /// Computes outward continuous supremum $\overline{\mu}_{\mathrm{slab}} \ge \sup_{s \in [t_n, t_{n+1}]} \mu_H(s)$,
    /// accumulates the exponent with outward addition, and updates the certified cumulative
    /// growth bound via outward interval exponential `exp_interval`.
    pub fn step_slab_2x2(
        &mut self,
        dt: f64,
        j_slab: &[[Interval; 2]; 2],
        h_slab: &[[Interval; 2]; 2],
        hdot_slab: &[[Interval; 2]; 2],
    ) -> Result<f64, CoreError> {
        if !(dt > 0.0 && dt.is_finite()) {
            return Err(CoreError::InvalidInput(
                "step size dt must be positive and finite".into(),
            ));
        }

        let mu_upper = enclose_slab_mu_2x2(j_slab, h_slab, hdot_slab)?;
        let slab_exponent_hi = mul_up(mu_upper, dt)?;
        self.cumulative_exponent_hi = add_up(self.cumulative_exponent_hi, slab_exponent_hi)?;

        // Outward exponential enclosure using directed Cody-Waite interval exp
        let exp_int = exp_interval(self.cumulative_exponent_hi)?;
        self.cumulative_growth_bound_hi = mul_up(self.cumulative_jump_factor_hi, exp_int.hi)?;

        self.current_h_2x2 = *h_slab;
        self.last_slab_mu_hi = mu_upper;
        self.work_counters.slab_enclosures += 1;
        self.work_counters.eigen_solves += 1;
        self.work_counters.metric_updates += 1;

        Ok(mu_upper)
    }

    /// Convenience helper: integrate one time slab for constant Jacobian $J$ and constant metric $H$.
    pub fn step_constant_j_2x2(&mut self, dt: f64, j: &[[f64; 2]; 2]) -> Result<f64, CoreError> {
        let j_int = point_interval_matrix_2x2(j)?;
        let zero = Interval { lo: 0.0, hi: 0.0 };
        let hdot_zero = [[zero, zero], [zero, zero]];
        let current_h = self.current_h_2x2;
        self.step_slab_2x2(dt, &j_int, &current_h, &hdot_zero)
    }

    /// Discontinuous metric switch from current $H_-$ to new $H_+$.
    ///
    /// Outwardly encloses jump factor $\gamma_{\mathrm{jump}} \ge \sqrt{\lambda_{\max}(H_-^{-1/2} H_+ H_-^{-1/2})}$,
    /// and multiplies the cumulative growth bound with outward directed rounding.
    pub fn switch_metric_2x2(&mut self, h_new: [[Interval; 2]; 2]) -> Result<f64, CoreError> {
        let gamma_jump_hi = enclose_jump_factor_2x2(&self.current_h_2x2, &h_new)?;
        self.cumulative_jump_factor_hi = mul_up(self.cumulative_jump_factor_hi, gamma_jump_hi)?;
        self.cumulative_growth_bound_hi = mul_up(self.cumulative_growth_bound_hi, gamma_jump_hi)?;
        self.current_h_2x2 = h_new;
        self.work_counters.jump_factor_computations += 1;
        self.work_counters.metric_updates += 1;
        Ok(gamma_jump_hi)
    }

    /// Discontinuous metric switch for constant f64 SPD matrix $H_+$.
    pub fn switch_metric_constant_2x2(&mut self, h_new: [[f64; 2]; 2]) -> Result<f64, CoreError> {
        let h_int = point_interval_matrix_2x2(&h_new)?;
        self.switch_metric_2x2(h_int)
    }

    #[inline]
    pub fn cumulative_growth_bound(&self) -> f64 {
        self.cumulative_growth_bound_hi
    }

    #[inline]
    pub fn cumulative_exponent(&self) -> f64 {
        self.cumulative_exponent_hi
    }

    #[inline]
    pub fn cumulative_jump_factor(&self) -> f64 {
        self.cumulative_jump_factor_hi
    }

    #[inline]
    pub fn last_slab_mu(&self) -> f64 {
        self.last_slab_mu_hi
    }

    #[inline]
    pub fn current_h(&self) -> [[Interval; 2]; 2] {
        self.current_h_2x2
    }
}
