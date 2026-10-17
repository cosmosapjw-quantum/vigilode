//! Common damping entire-moment kernel with rigorous outward interval enclosure.
//!
//! Research node LP03 of `research/promotion_loop12_20261011/PORTING_DAG.json`.
//! Implements outward entire moments for the damped Volterra convolution kernel:
//!
//! \[
//! I_{j, k}(h, \kappa) = \int_0^h e^{-\kappa(h - s)} (h - s)^j s^k \, ds
//! \]
//!
//! In normalized time \(u = s / h \in [0, 1]\), with \(z = \kappa h \ge 0\):
//!
//! \[
//! I_{j, k}(h, \kappa) = h^{j + k + 1} J_{j, k}(z), \quad J_{j, k}(z) = \int_0^1 e^{-z(1 - u)} (1 - u)^j u^k \, du
//! \]
//!
//! At \(z = 0\) (\(\kappa \to 0\)), this continuously reduces to:
//! \[
//! J_{j, k}(0) = B(j + 1, k + 1) = \frac{j! k!}{(j + k + 1)!}
//! \]
//!
//! For all \(z \ge 0\), rigorous outward bounds using [`Interval`] arithmetic enclose the exact value,
//! completely avoiding subtractive cancellation and numerical instability.

use serde::{Deserialize, Serialize};

use crate::directed::Interval;
use crate::{CoreError, CoreResult};

/// Maximum moment order supported by the table.
pub const MAX_MOMENT_ORDER: usize = 64;

/// Configuration for the damped moment table.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DampedMomentsConfig {
    pub kappa: f64,
    pub horizon: f64,
    pub max_j: usize,
    pub max_k: usize,
}

impl DampedMomentsConfig {
    pub fn new(kappa: f64, horizon: f64, max_j: usize, max_k: usize) -> CoreResult<Self> {
        if !kappa.is_finite() || kappa < 0.0 {
            return Err(CoreError::InvalidInput(format!(
                "damped_moments: kappa must be non-negative finite, got {kappa}"
            )));
        }
        if !horizon.is_finite() || horizon <= 0.0 {
            return Err(CoreError::InvalidInput(format!(
                "damped_moments: horizon must be positive finite, got {horizon}"
            )));
        }
        if max_j > MAX_MOMENT_ORDER || max_k > MAX_MOMENT_ORDER {
            return Err(CoreError::InvalidInput(format!(
                "damped_moments: max order exceeds limit {MAX_MOMENT_ORDER} (j={max_j}, k={max_k})"
            )));
        }
        Ok(Self {
            kappa,
            horizon,
            max_j,
            max_k,
        })
    }
}

/// Precomputed table of damped moment enclosures.
#[derive(Clone, Debug, PartialEq)]
pub struct DampedMomentTable {
    config: DampedMomentsConfig,
    /// Row-major table of normalized moments J_{j, k}(z) \in [0, 1].
    normalized: Vec<Interval>,
}

impl DampedMomentTable {
    /// Compiles a damped moment table up to orders `(max_j, max_k)`.
    pub fn compile(config: DampedMomentsConfig) -> CoreResult<Self> {
        let max_j = config.max_j;
        let max_k = config.max_k;
        let z = config.kappa * config.horizon;

        let num_elements = (max_j + 1) * (max_k + 1);
        let mut normalized = Vec::with_capacity(num_elements);

        for j in 0..=max_j {
            for k in 0..=max_k {
                let interval = compute_normalized_moment(z, j, k)?;
                normalized.push(interval);
            }
        }

        Ok(Self { config, normalized })
    }

    #[inline]
    fn index(&self, j: usize, k: usize) -> CoreResult<usize> {
        if j > self.config.max_j || k > self.config.max_k {
            return Err(CoreError::InvalidInput(format!(
                "damped_moments: index out of bounds ({j}, {k}) > ({}, {})",
                self.config.max_j, self.config.max_k
            )));
        }
        Ok(j * (self.config.max_k + 1) + k)
    }

    /// Returns the normalized moment enclosure \(J_{j, k}(\kappa h) = \int_0^1 e^{-\kappa h(1 - u)} (1 - u)^j u^k du\).
    pub fn get_normalized(&self, j: usize, k: usize) -> CoreResult<Interval> {
        let idx = self.index(j, k)?;
        Ok(self.normalized[idx])
    }

    /// Returns the unscaled physical moment enclosure \(I_{j, k}(h, \kappa) = h^{j + k + 1} J_{j, k}(\kappa h)\).
    pub fn get_unscaled(&self, j: usize, k: usize) -> CoreResult<Interval> {
        let norm = self.get_normalized(j, k)?;
        scale_by_h_power(norm, self.config.horizon, j + k + 1)
    }

    pub fn kappa(&self) -> f64 {
        self.config.kappa
    }

    pub fn horizon(&self) -> f64 {
        self.config.horizon
    }

    pub fn max_j(&self) -> usize {
        self.config.max_j
    }

    pub fn max_k(&self) -> usize {
        self.config.max_k
    }

    pub fn config(&self) -> &DampedMomentsConfig {
        &self.config
    }
}

/// Standalone function to compute the unscaled moment \(I_{j, k}(h, \kappa)\).
pub fn compute_damped_moment(kappa: f64, horizon: f64, j: usize, k: usize) -> CoreResult<Interval> {
    let config = DampedMomentsConfig::new(kappa, horizon, j, k)?;
    let norm = compute_normalized_moment(kappa * horizon, j, k)?;
    scale_by_h_power(norm, config.horizon, j + k + 1)
}

/// Standalone function to compute the normalized moment \(J_{j, k}(\kappa h)\).
pub fn compute_normalized_damped_moment(
    kappa: f64,
    horizon: f64,
    j: usize,
    k: usize,
) -> CoreResult<Interval> {
    let _config = DampedMomentsConfig::new(kappa, horizon, j, k)?;
    compute_normalized_moment(kappa * horizon, j, k)
}

// ---------------------------------------------------------------------------
// Internal Algorithms with Certified Outward Interval Enclosure
// ---------------------------------------------------------------------------

fn beta_interval(j: usize, k: usize) -> CoreResult<Interval> {
    let mut val = Interval::new(1.0 / ((j + 1) as f64), 1.0 / ((j + 1) as f64))?;
    for step in 1..=k {
        let num = Interval::new(step as f64, step as f64)?;
        let den = Interval::new((j + step + 1) as f64, (j + step + 1) as f64)?;
        val = val.mul(num)?.div(den)?;
    }
    Ok(val)
}

fn scale_by_h_power(interval: Interval, h: f64, power: usize) -> CoreResult<Interval> {
    let mut factor = Interval::point(1.0)?;
    let h_int = Interval::point(h)?;
    for _ in 0..power {
        factor = factor.mul(h_int)?;
    }
    interval.mul(factor)
}

/// Computes J_{j, k}(z) = \int_0^1 e^{-z(1 - u)} (1 - u)^j u^k du
fn compute_normalized_moment(z: f64, j: usize, k: usize) -> CoreResult<Interval> {
    if !z.is_finite() || z < 0.0 {
        return Err(CoreError::InvalidInput(format!(
            "damped_moments: z must be non-negative finite, got {z}"
        )));
    }

    let b_jk = beta_interval(j, k)?;

    // Branch 1: Exactly zero damping
    if z == 0.0 {
        return Ok(b_jk);
    }

    // Branch 2: Small z <= 1.0 (Taylor series expansion with certified remainder)
    if z <= 1.0 {
        return compute_small_z_taylor(z, j, k, b_jk);
    }

    // Branch 3: Large z > 1.0 (Backward recurrence on incomplete exponential moments)
    compute_large_z_backward(z, j, k, b_jk)
}

/// Taylor expansion: e^{-z(1 - u)} = sum_{n=0}^{N-1} (-z)^n / n! (1 - u)^n + R_N
fn compute_small_z_taylor(z: f64, j: usize, k: usize, b_jk: Interval) -> CoreResult<Interval> {
    let n_terms = 28;
    let mut sum = Interval::point(0.0)?;

    let z_int = Interval::point(z)?;
    let mut z_pow = Interval::point(1.0)?;
    let mut n_fact = Interval::point(1.0)?;

    for n in 0..n_terms {
        let b = beta_interval(j + n, k)?;
        let term = z_pow.div(n_fact)?.mul(b)?;

        if n % 2 == 0 {
            sum = sum.add(term)?;
        } else {
            sum = sum.sub(term)?;
        }

        let next_n = (n + 1) as f64;
        z_pow = z_pow.mul(z_int)?;
        n_fact = n_fact.mul(Interval::point(next_n)?)?;
    }

    // Certified outward remainder: |R_N| <= (z^N / N!) * B(j + N + 1, k + 1)
    let rem_b = beta_interval(j + n_terms, k)?;
    let rem_bound = z_pow.div(n_fact)?.mul(rem_b)?;

    let lo = (sum.lo - rem_bound.hi).max(0.0);
    let hi = (sum.hi + rem_bound.hi).min(b_jk.hi);

    Interval::new(lo, hi)
}

/// Backward recurrence for E_m(z) = \int_0^1 e^{-z v} v^m dv, v = 1 - u
/// z E_m = m E_{m-1} - e^{-z}  =>  E_{m-1} = (z E_m + e^{-z}) / m
fn compute_large_z_backward(z: f64, j: usize, k: usize, b_jk: Interval) -> CoreResult<Interval> {
    let top = j + k + 45;
    let exp_neg_z = (-z).exp();

    // Outward enclosure of exp(-z)
    let exp_int = Interval::new(
        (exp_neg_z * (1.0 - 1e-15)).max(0.0),
        exp_neg_z * (1.0 + 1e-15),
    )?;
    let z_int = Interval::point(z)?;

    // At top: 0 <= E_top <= 1 / (top + 1)
    let mut e_m = Interval::new(0.0, 1.0 / ((top + 1) as f64))?;

    // Store E_0 .. E_{j + k}
    let mut e_table = vec![Interval::point(0.0)?; j + k + 1];

    for m in (1..=top).rev() {
        if m - 1 <= j + k {
            let m_int = Interval::point(m as f64)?;
            let num = z_int.mul(e_m)?.add(exp_int)?;
            e_m = num.div(m_int)?;
            e_table[m - 1] = e_m;
        } else {
            let m_int = Interval::point(m as f64)?;
            let num = z_int.mul(e_m)?.add(exp_int)?;
            e_m = num.div(m_int)?;
        }
    }

    // Binomial expansion of (1 - v)^k:
    // J_{j, k} = sum_{l=0}^k (-1)^l binom(k, l) E_{j + l}(z)
    let mut res = Interval::point(0.0)?;
    let mut binom = 1.0;

    for l in 0..=k {
        if l > 0 {
            binom *= ((k - l + 1) as f64) / (l as f64);
        }
        let term = Interval::point(binom)?.mul(e_table[j + l])?;

        if l % 2 == 0 {
            res = res.add(term)?;
        } else {
            res = res.sub(term)?;
        }
    }

    let lo = res.lo.max(0.0);
    let hi = res.hi.min(b_jk.hi);
    Interval::new(lo, hi)
}
