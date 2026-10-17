//! Finite-window causal packet and native total certificate (LP02).
//!
//! Autonomous N-mode collective cubic model on [0, h]:
//!   \dot{a}_j = i * width * \xi_j * a_j + i * \chi * |\langle a \rangle|^2 * \langle a \rangle
//! with normalized positive weights w_j > 0, sum w_j = 1, frequencies \xi_j \in [-1, 1],
//! kappa = 0, and independent complex initial amplitudes a_j(0) = z_j.
//!
//! Provides moment prefix compilation, polynomial waveform Picard iteration,
//! exact causal factorial convolution via precomputed beta weights,
//! and original-model analytic error estimates for the mean and all modes.
//! Floating-point coefficient and reconstruction errors are not yet enclosed;
//! these estimates have diagnostic authority, not production certification.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use rodas5p_core::{CoreError, CoreResult};

use crate::collective_model::CollectiveModel;
use crate::packet_work::PacketWorkCounters;

/// Errors arising during causal packet compilation, candidate iteration, or certification.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum PacketError {
    #[error(
        "the polynomial packet implements kappa = 0; damping {0} requires the damped packet route"
    )]
    UnsupportedDamping(f64),
    #[error("non-finite value in packet {0}")]
    NonFinite(&'static str),
    #[error("horizon must be strictly positive, got {0}")]
    InvalidHorizon(f64),
    #[error("degree {degree} must be >= kernel order {order}")]
    DegreeLessThanOrder { degree: usize, order: usize },
    #[error("sweeps must be strictly positive, got 0")]
    ZeroSweeps,
    #[error("Picard candidate leaves proved complex-mean unit domain: bound {bound} > 1.0")]
    DomainExceeded { bound: f64 },
    #[error("Gronwall contract violated: h * L = {h_times_l} >= 1.0 (unsupported)")]
    GronwallConditionViolated { h_times_l: f64 },
    #[error("reconstruction index {index} out of bounds for mode count {modes}")]
    IndexOutOfBounds { index: usize, modes: usize },
    #[error("error budget exceeded: max mode bound {max_bound} > tolerance {tolerance}")]
    BudgetExceeded { max_bound: f64, tolerance: f64 },
}

impl From<PacketError> for CoreError {
    fn from(err: PacketError) -> Self {
        CoreError::InvalidInput(err.to_string())
    }
}

// ---------------------------------------------------------------------------
// Complex arithmetic helpers on (f64, f64)
// ---------------------------------------------------------------------------

#[inline(always)]
fn c_add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

#[inline(always)]
fn c_sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

#[inline(always)]
fn c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline(always)]
fn c_scale(a: (f64, f64), s: f64) -> (f64, f64) {
    (a.0 * s, a.1 * s)
}

#[inline(always)]
fn c_conj(a: (f64, f64)) -> (f64, f64) {
    (a.0, -a.1)
}

#[inline(always)]
fn c_upper(a: (f64, f64)) -> f64 {
    a.0.abs() + a.1.abs()
}

// ---------------------------------------------------------------------------
// Polynomial helpers in normalized time \tau = t / h \in [0, 1]
// ---------------------------------------------------------------------------

fn poly_add(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let len = a.len().max(b.len());
    let mut out = vec![(0.0, 0.0); len];
    for (i, &c) in a.iter().enumerate() {
        out[i] = c_add(out[i], c);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] = c_add(out[i], c);
    }
    out
}

fn poly_sub(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let len = a.len().max(b.len());
    let mut out = vec![(0.0, 0.0); len];
    for (i, &c) in a.iter().enumerate() {
        out[i] = c_add(out[i], c);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] = c_sub(out[i], c);
    }
    out
}

fn poly_mul(a: &[(f64, f64)], b: &[(f64, f64)], work: &mut PacketWorkCounters) -> Vec<(f64, f64)> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![(0.0, 0.0); a.len() + b.len() - 1];
    for (j, &xj) in a.iter().enumerate() {
        for (k, &yk) in b.iter().enumerate() {
            work.poly_multiply_terms += 1;
            out[j + k] = c_add(out[j + k], c_mul(xj, yk));
        }
    }
    out
}

fn poly_bound(p: &[(f64, f64)]) -> f64 {
    p.iter().map(|&c| c_upper(c)).sum()
}

fn poly_eval(p: &[(f64, f64)], x: f64, work: &mut PacketWorkCounters) -> (f64, f64) {
    let mut val = (0.0, 0.0);
    for &c in p.iter().rev() {
        work.output_horner_terms += 1;
        val = c_add(c_scale(val, x), c);
    }
    val
}

/// Evaluates F(p) = i * chi * |p|^2 * p in polynomial algebra.
fn force(p: &[(f64, f64)], chi: f64, work: &mut PacketWorkCounters) -> Vec<(f64, f64)> {
    work.nonlinear_evaluations += 1;
    let p_conj: Vec<(f64, f64)> = p.iter().map(|&c| c_conj(c)).collect();
    let p2 = poly_mul(p, &p_conj, work);
    let p3 = poly_mul(&p2, p, work);

    // Multiply by i * chi: (u + i v) * (i * chi) = -chi * v + i * chi * u
    p3.into_iter().map(|(u, v)| (-chi * v, chi * u)).collect()
}

// ---------------------------------------------------------------------------
// Precomputed Causal Beta Weights (LC1/LC2 contracts)
// ---------------------------------------------------------------------------

/// Exact causal beta weights table B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}.
#[derive(Clone, Debug, PartialEq)]
pub struct CausalWeights {
    kernel_degree: usize,
    forcing_degree: usize,
    rows: Vec<Vec<f64>>,
}

impl CausalWeights {
    /// Compiles a table of beta weights for orders up to `kernel_degree` and `forcing_degree`.
    pub fn compile(kernel_degree: usize, forcing_degree: usize) -> Self {
        let mut rows = Vec::with_capacity(kernel_degree + 1);
        for j in 0..=kernel_degree {
            let mut row = Vec::with_capacity(forcing_degree + 1);
            let first = 1.0 / ((j + 1) as f64);
            row.push(first);
            let mut current = first;
            for k in 1..=forcing_degree {
                current *= (k as f64) / ((j + k + 1) as f64);
                row.push(current);
            }
            rows.push(row);
        }
        Self {
            kernel_degree,
            forcing_degree,
            rows,
        }
    }

    /// Returns the beta weight B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}.
    #[inline]
    pub fn get(&self, j: usize, k: usize) -> f64 {
        if j < self.rows.len() && k < self.rows[j].len() {
            self.rows[j][k]
        } else {
            let mut val = 1.0 / ((j + 1) as f64);
            for m in 1..=k {
                val *= (m as f64) / ((j + m + 1) as f64);
            }
            val
        }
    }
}

/// Computes the causal Volterra convolution \int_0^\tau K(\tau - \sigma) F(\sigma) d\sigma.
fn convolve(
    kernel: &[(f64, f64)],
    forcing: &[(f64, f64)],
    table: &CausalWeights,
    work: &mut PacketWorkCounters,
) -> Vec<(f64, f64)> {
    let mut out = vec![(0.0, 0.0); kernel.len() + forcing.len()];
    for (j, &a) in kernel.iter().enumerate() {
        for (k, &b) in forcing.iter().enumerate() {
            work.volterra_terms += 1;
            let w = table.get(j, k);
            let term = c_scale(c_mul(a, b), w);
            out[j + k + 1] = c_add(out[j + k + 1], term);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Packet Structures and Compilation
// ---------------------------------------------------------------------------

/// A compiled finite-window packet at horizon h and Taylor order p.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPacket {
    pub model: CollectiveModel,
    pub horizon: f64,
    pub order: usize,
    pub mu: Vec<f64>,
    pub nu: Vec<(f64, f64)>,
    pub free: Vec<(f64, f64)>,
    pub kernel: Vec<(f64, f64)>,
}

/// Compiles moment prefix and basis polynomials into a [`CompiledPacket`].
pub fn compile_packet(
    model: &CollectiveModel,
    horizon: f64,
    order: usize,
    work: &mut PacketWorkCounters,
) -> Result<CompiledPacket, PacketError> {
    if model.kappa() != 0.0 {
        return Err(PacketError::UnsupportedDamping(model.kappa()));
    }
    if horizon <= 0.0 || !horizon.is_finite() {
        return Err(PacketError::InvalidHorizon(horizon));
    }

    let mut mu = Vec::with_capacity(order + 1);
    let mut nu = Vec::with_capacity(order + 1);
    for k in 0..=order {
        mu.push(model.spectral_moment(k));
        nu.push(model.weighted_moment(k));
        work.moment_terms += (model.num_modes() as u64) * 2;
    }

    let mut free = Vec::with_capacity(order + 1);
    let mut kernel = Vec::with_capacity(order + 1);

    // b_0 = 1.0, b_k = b_{k-1} * (i * width * h) / k
    let mut b = (1.0, 0.0);
    free.push(c_mul(b, nu[0]));
    kernel.push(c_scale(b, mu[0]));

    let width = model.width();
    for k in 1..=order {
        // Multiply by (i * width * h) / k:
        // (u + i v) * (i * factor) = -factor * v + i * factor * u
        let factor = (width * horizon) / (k as f64);
        b = (-factor * b.1, factor * b.0);

        free.push(c_mul(b, nu[k]));
        kernel.push(c_scale(b, mu[k]));
    }

    Ok(CompiledPacket {
        model: model.clone(),
        horizon,
        order,
        mu,
        nu,
        free,
        kernel,
    })
}

// ---------------------------------------------------------------------------
// Candidate Picard Iteration
// ---------------------------------------------------------------------------

/// A generated Picard polynomial waveform candidate for the complex mean.
#[derive(Clone, Debug, PartialEq)]
pub struct PacketCandidate {
    pub mean: Vec<(f64, f64)>,
    pub degree: usize,
    pub sweeps: usize,
}

/// Generates a Picard candidate polynomial for the complex mean in sweeps.
pub fn generate_candidate(
    packet: &CompiledPacket,
    degree: usize,
    sweeps: usize,
    table: &CausalWeights,
    work: &mut PacketWorkCounters,
) -> Result<PacketCandidate, PacketError> {
    if degree < packet.order {
        return Err(PacketError::DegreeLessThanOrder {
            degree,
            order: packet.order,
        });
    }
    if sweeps == 0 {
        return Err(PacketError::ZeroSweeps);
    }

    let mut p = packet.free.clone();
    p.resize(degree + 1, (0.0, 0.0));

    let h = packet.horizon;
    let chi = packet.model.chi();

    for _ in 0..sweeps {
        let f = force(&p, chi, work);
        let conv_res = convolve(&packet.kernel, &f, table, work);
        let scaled_conv: Vec<(f64, f64)> = conv_res.into_iter().map(|c| c_scale(c, h)).collect();
        let mut next = poly_add(&packet.free, &scaled_conv);
        next.truncate(degree + 1);
        if next.len() < degree + 1 {
            next.resize(degree + 1, (0.0, 0.0));
        }

        // Exact original initial mean enforcement at t=0
        next[0] = packet.nu[0];
        p = next;
    }

    Ok(PacketCandidate {
        mean: p,
        degree,
        sweeps,
    })
}

// ---------------------------------------------------------------------------
// Native Total Certificate
// ---------------------------------------------------------------------------

/// Analytic error estimate for the candidate on [0, h].
///
/// `roundoff_enclosed` is false until the whole coefficient, residual, norm,
/// tail and reconstruction calculation uses verified outward arithmetic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacketCertificate {
    pub roundoff_enclosed: bool,
    pub mean_bound: f64,
    pub mode_bounds: Vec<f64>,
    pub volterra_defect: f64,
    pub kernel_model: f64,
    pub tail: f64,
    pub candidate_domain_bound: f64,
    pub lipschitz_l: f64,
    pub gain_g: f64,
    pub horizon: f64,
    pub order: usize,
}

fn factorial(n: usize) -> f64 {
    let mut f = 1.0;
    for i in 2..=n {
        f *= i as f64;
    }
    f
}

/// Computes the continuous-model tail and defect estimate in ordinary binary64.
/// The result explicitly reports `roundoff_enclosed = false`.
pub fn compute_certificate(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    table: &CausalWeights,
    work: &mut PacketWorkCounters,
) -> Result<PacketCertificate, PacketError> {
    if packet.model.kappa() != 0.0 {
        return Err(PacketError::UnsupportedDamping(packet.model.kappa()));
    }
    if !packet.horizon.is_finite() || packet.horizon <= 0.0 {
        return Err(PacketError::InvalidHorizon(packet.horizon));
    }
    if cand
        .mean
        .iter()
        .chain(packet.free.iter())
        .chain(packet.kernel.iter())
        .any(|&(re, im)| !re.is_finite() || !im.is_finite())
    {
        return Err(PacketError::NonFinite("coefficient"));
    }
    let h = packet.horizon;
    let bound_mean = poly_bound(&cand.mean);
    if !bound_mean.is_finite() {
        return Err(PacketError::NonFinite("candidate domain bound"));
    }
    if bound_mean > 1.0 {
        return Err(PacketError::DomainExceeded { bound: bound_mean });
    }

    let chi = packet.model.chi();
    let l = 3.0 * chi.abs();
    let g = chi.abs();

    let hl = h * l;
    if hl >= 1.0 - 1e-14 {
        return Err(PacketError::GronwallConditionViolated { h_times_l: hl });
    }

    let f = force(&cand.mean, chi, work);
    let conv_res = convolve(&packet.kernel, &f, table, work);
    let scaled_conv: Vec<(f64, f64)> = conv_res.into_iter().map(|c| c_scale(c, h)).collect();
    let expected = poly_add(&packet.free, &scaled_conv);
    let residual = poly_sub(&cand.mean, &expected);
    let defect = poly_bound(&residual);

    // Kernel tail bound tau = (width * h)^{p+1} / (p+1)!
    let width = packet.model.width();
    let p = packet.order;
    let tau = (width * h).powi((p + 1) as i32) / factorial(p + 1);

    // Initial amplitude upper bound sum w_j |z_j|
    let mut a0 = 0.0;
    for (&w, &z) in packet
        .model
        .weights()
        .iter()
        .zip(packet.model.initial_amplitudes().iter())
    {
        a0 += w * c_upper(z);
    }

    let kernel_model = tau * (a0 + (h * g) / ((p + 2) as f64));
    let mean_bound = (defect + kernel_model) / (1.0 - hl);

    let mut mode_bounds = Vec::with_capacity(packet.model.num_modes());
    for &z in packet.model.initial_amplitudes() {
        let emode = tau * (c_upper(z) + (h * g) / ((p + 2) as f64)) + hl * mean_bound;
        mode_bounds.push(emode);
    }

    if [defect, tau, kernel_model, mean_bound, l, g]
        .iter()
        .any(|x| !x.is_finite())
        || mode_bounds.iter().any(|x| !x.is_finite())
    {
        return Err(PacketError::NonFinite("error estimate"));
    }

    Ok(PacketCertificate {
        roundoff_enclosed: false,
        mean_bound,
        mode_bounds,
        volterra_defect: defect,
        kernel_model,
        tail: tau,
        candidate_domain_bound: bound_mean,
        lipschitz_l: l,
        gain_g: g,
        horizon: h,
        order: p,
    })
}

// ---------------------------------------------------------------------------
// Microscopic Mode Reconstruction in O(p N)
// ---------------------------------------------------------------------------

/// Computes reconstruction polynomials beta(\xi) and gamma(\xi).
#[allow(clippy::type_complexity)]
pub fn reconstruction_polynomials(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    work: &mut PacketWorkCounters,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let p = packet.order;
    let h = packet.horizon;
    let width = packet.model.width();
    let chi = packet.model.chi();

    let f = force(&cand.mean, chi, work);

    let mut beta = Vec::with_capacity(p + 1);
    let mut gamma = Vec::with_capacity(p + 1);

    let mut b = (1.0, 0.0);
    for j in 0..=p {
        if j > 0 {
            let factor = (width * h) / (j as f64);
            b = (-factor * b.1, factor * b.0);
        }
        beta.push(b);

        let mut acc = (0.0, 0.0);
        let mut w = 1.0 / ((j + 1) as f64);
        for (k, &fk) in f.iter().enumerate() {
            if k > 0 {
                w *= (k as f64) / ((j + k + 1) as f64);
            }
            acc = c_add(acc, c_scale(fk, w));
        }
        gamma.push(c_scale(c_mul(b, acc), h));
    }

    (beta, gamma)
}

/// Reconstructs all N microscopic mode amplitudes a_j(h) at the horizon end.
pub fn reconstruct_modes(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    work: &mut PacketWorkCounters,
) -> Vec<(f64, f64)> {
    let (beta, gamma) = reconstruction_polynomials(cand, packet, work);
    let mut out = Vec::with_capacity(packet.model.num_modes());

    for (&xi, &z) in packet
        .model
        .xi()
        .iter()
        .zip(packet.model.initial_amplitudes().iter())
    {
        let b_val = poly_eval(&beta, xi, work);
        let g_val = poly_eval(&gamma, xi, work);
        out.push(c_add(c_mul(b_val, z), g_val));
    }

    out
}

/// Evaluates a single mode a_j(tau) at normalized time tau = t/h \in [0, 1].
pub fn evaluate_mode_at(
    cand: &PacketCandidate,
    packet: &CompiledPacket,
    mode_idx: usize,
    tau: f64,
    work: &mut PacketWorkCounters,
) -> CoreResult<(f64, f64)> {
    if mode_idx >= packet.model.num_modes() {
        return Err(CoreError::InvalidInput(format!(
            "evaluate_mode_at: mode_idx {mode_idx} >= num_modes {}",
            packet.model.num_modes()
        )));
    }
    if !(0.0..=1.0).contains(&tau) {
        return Err(CoreError::InvalidInput(format!(
            "evaluate_mode_at: tau {tau} out of range [0, 1]"
        )));
    }
    if tau == 0.0 {
        return Ok(packet.model.initial_amplitudes()[mode_idx]);
    }

    let h = packet.horizon * tau;
    let sub_packet = compile_packet(&packet.model, h, packet.order, work)?;
    // For the shorter normalized coordinate s, the old waveform is P(tau*s).
    // Restrict coefficients before applying the nonlinear force and convolution.
    let mut restricted = cand.clone();
    let mut scale = 1.0;
    for coefficient in &mut restricted.mean {
        *coefficient = c_scale(*coefficient, scale);
        scale *= tau;
    }
    let modes = reconstruct_modes(&restricted, &sub_packet, work);
    Ok(modes[mode_idx])
}
