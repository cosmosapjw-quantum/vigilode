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
//! Successful evaluations enclose the exact real value of the supplied binary64 inputs,
//! including the product \(z=\kappa h\). Intermediate overflow is refused. The large-z
//! backward recurrence can produce wide enclosures; callers must check the returned width.

use serde::{Deserialize, Serialize};

use crate::directed::{Interval, exp_interval, sub_up};
use crate::{CoreError, CoreResult};
use thiserror::Error;

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
        // Fields are public/serializable: constructors are not an admission boundary.
        let config =
            DampedMomentsConfig::new(config.kappa, config.horizon, config.max_j, config.max_k)?;
        let max_j = config.max_j;
        let max_k = config.max_k;
        let z = damping_product(config.kappa, config.horizon)?;

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
    let norm = compute_normalized_moment(damping_product(kappa, horizon)?, j, k)?;
    scale_by_h_power(norm, config.horizon, j + k + 1)
}

/// Work attempted by the checked forcing-moment consumer.
///
/// Counts are separate coarse-grained attempts, not arithmetic instruction
/// counts. They are charged before entering the corresponding fallible work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DampedForcingMomentWork {
    pub validation_attempts: u64,
    pub kernel_attempts: u64,
    pub width_checks: u64,
}

/// A damped forcing moment admitted by an outward absolute-width gate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DampedForcingMomentResult {
    /// Enclosure of (Q_{r,d}(a,h)=I_{d,r}(a,h)).
    pub interval: Interval,
    /// Outward upper bound on `interval.hi - interval.lo`.
    pub width_upper: f64,
}

/// Typed refusal from the forcing-moment consumer boundary.
#[derive(Debug, Error)]
pub enum DampedForcingMomentRefusal {
    #[error("invalid forcing-moment input: {0}")]
    InvalidInput(&'static str),
    #[error("damped-moment kernel refused: {0}")]
    Kernel(#[from] CoreError),
    #[error("outward width arithmetic refused: {0}")]
    WidthArithmetic(CoreError),
    #[error("forcing-moment interval width {width_upper:e} exceeds limit {max_width:e}")]
    WidthExceeded { width_upper: f64, max_width: f64 },
}

/// Computes the forcing moment (Q_{r,d}(a,h)=I_{d,r}(a,h)), then admits it
/// only if its outward absolute interval width is at most `max_width`.
///
/// This is a checked scalar-kernel consumer only. It does not admit a damped
/// packet solver or assert a packet/output convention. Attempt counters are
/// retained on all typed refusal paths.
pub fn compute_damped_forcing_moment(
    a: f64,
    h: f64,
    d: usize,
    r: usize,
    max_width: f64,
    work: &mut DampedForcingMomentWork,
) -> Result<DampedForcingMomentResult, DampedForcingMomentRefusal> {
    work.validation_attempts = work.validation_attempts.saturating_add(1);
    if !a.is_finite() || a < 0.0 {
        return Err(DampedForcingMomentRefusal::InvalidInput(
            "damping rate must be nonnegative and finite",
        ));
    }
    if !h.is_finite() || h <= 0.0 {
        return Err(DampedForcingMomentRefusal::InvalidInput(
            "horizon must be positive and finite",
        ));
    }
    if d > MAX_MOMENT_ORDER || r > MAX_MOMENT_ORDER {
        return Err(DampedForcingMomentRefusal::InvalidInput(
            "moment order exceeds supported maximum",
        ));
    }
    if !max_width.is_finite() || max_width < 0.0 {
        return Err(DampedForcingMomentRefusal::InvalidInput(
            "absolute width limit must be nonnegative and finite",
        ));
    }

    work.kernel_attempts = work.kernel_attempts.saturating_add(1);
    let interval = compute_damped_moment(a, h, d, r)?;

    work.width_checks = work.width_checks.saturating_add(1);
    let width_upper =
        sub_up(interval.hi, interval.lo).map_err(DampedForcingMomentRefusal::WidthArithmetic)?;
    if width_upper > max_width {
        return Err(DampedForcingMomentRefusal::WidthExceeded {
            width_upper,
            max_width,
        });
    }

    Ok(DampedForcingMomentResult {
        interval,
        width_upper,
    })
}

/// Standalone function to compute the normalized moment \(J_{j, k}(\kappa h)\).
pub fn compute_normalized_damped_moment(
    kappa: f64,
    horizon: f64,
    j: usize,
    k: usize,
) -> CoreResult<Interval> {
    let _config = DampedMomentsConfig::new(kappa, horizon, j, k)?;
    compute_normalized_moment(damping_product(kappa, horizon)?, j, k)
}

// ---------------------------------------------------------------------------
// Internal Algorithms with Certified Outward Interval Enclosure
// ---------------------------------------------------------------------------

fn beta_interval(j: usize, k: usize) -> CoreResult<Interval> {
    let mut val = Interval::point(1.0)?.div(Interval::point((j + 1) as f64)?)?;
    for step in 1..=k {
        let num = Interval::new(step as f64, step as f64)?;
        let den = Interval::new((j + step + 1) as f64, (j + step + 1) as f64)?;
        val = val.mul(num)?.div(den)?;
    }
    Ok(val)
}

fn damping_product(kappa: f64, horizon: f64) -> CoreResult<Interval> {
    let z = Interval::point(kappa)?.mul(Interval::point(horizon)?)?;
    // The directed underflow fallback may extend below zero; positivity is exact.
    Interval::new(z.lo.max(0.0), z.hi)
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
fn compute_normalized_moment(z: Interval, j: usize, k: usize) -> CoreResult<Interval> {
    if z.lo < 0.0 {
        return Err(CoreError::InvalidInput(format!(
            "damped_moments: z must be non-negative finite, got {z:?}"
        )));
    }

    let b_jk = beta_interval(j, k)?;

    // Branch 1: Exactly zero damping
    if z.hi == 0.0 {
        return Ok(b_jk);
    }

    // Branch 2: Small z <= 1.0 (Taylor series expansion with certified remainder)
    if z.hi <= 1.0 {
        return compute_small_z_taylor(z, j, k, b_jk);
    }

    // Branch 3: Large z > 1.0 (Backward recurrence on incomplete exponential moments)
    compute_large_z_backward(z, j, k, b_jk)
}

/// Taylor expansion: e^{-z(1 - u)} = sum_{n=0}^{N-1} (-z)^n / n! (1 - u)^n + R_N
fn compute_small_z_taylor(z: Interval, j: usize, k: usize, b_jk: Interval) -> CoreResult<Interval> {
    let n_terms = 28;
    let mut sum = Interval::point(0.0)?;

    let z_int = z;
    let mut z_pow = Interval::point(1.0)?;
    let mut n_fact = Interval::point(1.0)?;
    let mut b = b_jk;

    for n in 0..n_terms {
        let term = z_pow.div(n_fact)?.mul(b)?;

        if n % 2 == 0 {
            sum = sum.add(term)?;
        } else {
            sum = sum.sub(term)?;
        }

        let next_n = (n + 1) as f64;
        z_pow = z_pow.mul(z_int)?;
        n_fact = n_fact.mul(Interval::point(next_n)?)?;
        // B(j+n+2,k+1) = B(j+n+1,k+1) * (j+n+1)/(j+k+n+2).
        // Reuse the prefix in O(k + n_terms), rather than rebuilding O(k) each term.
        b = b
            .mul(Interval::point((j + n + 1) as f64)?)?
            .div(Interval::point((j + k + n + 2) as f64)?)?;
    }

    // Certified outward remainder: |R_N| <= (z^N / N!) * B(j + N + 1, k + 1)
    let rem_bound = z_pow.div(n_fact)?.mul(b)?;

    let enclosed = sum.add(Interval::new(-rem_bound.hi, rem_bound.hi)?)?;
    let lo = enclosed.lo.max(0.0);
    let hi = enclosed.hi.min(b_jk.hi);

    Interval::new(lo, hi)
}

/// Backward recurrence for E_m(z) = \int_0^1 e^{-z v} v^m dv, v = 1 - u
/// z E_m = m E_{m-1} - e^{-z}  =>  E_{m-1} = (z E_m + e^{-z}) / m
fn compute_large_z_backward(
    z: Interval,
    j: usize,
    k: usize,
    b_jk: Interval,
) -> CoreResult<Interval> {
    let top = j + k + 45;
    // Monotonicity encloses exp(-z) over the entire rounded-product interval.
    let exp_int = Interval::new(exp_interval(-z.hi)?.lo, exp_interval(-z.lo)?.hi)?;
    let z_int = z;

    // At top: 0 <= E_top <= 1 / (top + 1)
    let top_bound = Interval::point(1.0)?.div(Interval::point((top + 1) as f64)?)?;
    let mut e_m = Interval::new(0.0, top_bound.hi)?;

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
    let mut binom = Interval::point(1.0)?;

    for l in 0..=k {
        if l > 0 {
            binom = binom
                .mul(Interval::point((k - l + 1) as f64)?)?
                .div(Interval::point(l as f64)?)?;
        }
        let term = binom.mul(e_table[j + l])?;

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

// A03-SCALAR-RATE-01: generated normalized rate and checked interval consumer.
// This API has no packet/trajectory authority. Legacy entry points above retain
// their existing numerical and allocation semantics.
pub const SCALAR_RATE_PRIMITIVE_CAP: u64 = 10_000_000;
pub const SCALAR_RATE_SCRATCH_LEN: usize = 129;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalarRateWork {
    pub validation_attempts: u64,
    pub endpoint_attempts: u64,
    pub width_checks: u64,
    pub fallback_attempts: u64,
    pub cells_attempted: u64,
    pub query_refusals: u64,
    pub interval_primitive_attempts: u64,
    pub coefficient_items: u64,
    pub output_items: u64,
    pub scratch_bytes: u64,
    /// Allocator requests made by this adapter (zero; caller scratch is external).
    /// Empirical allocator measurements are still required for VG-1 admission.
    pub allocation_requests: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRateRefusal {
    InvalidInput { field: &'static str },
    BindingMismatch { field: &'static str },
    ScratchTooSmall { required: usize, provided: usize },
    UnsupportedMomentOrder { required: usize, supported: usize },
    ArithmeticRefusal { stage: &'static str },
    MomentWidthExceeded { width: f64, limit: f64 },
    MomentWorkBudgetExceeded { used: u64, cap: u64 },
}

type ScalarResult<T> = Result<T, ScalarRateRefusal>;

/// Immutable provenance for the exact binary64 inputs defining 2(q+1) kappa h.
/// No public interval/certificate setter or deserializer is provided.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedDampingRate {
    kappa_bits: u64,
    horizon_bits: u64,
    q: usize,
    rate: Interval,
}

impl NormalizedDampingRate {
    pub fn from_inputs(
        kappa: f64,
        h: f64,
        q: usize,
        work: &mut ScalarRateWork,
    ) -> ScalarResult<Self> {
        work.validation_attempts = work.validation_attempts.saturating_add(1);
        let result = (|| {
            if !kappa.is_finite() || kappa < 0.0 {
                return Err(ScalarRateRefusal::InvalidInput { field: "kappa" });
            }
            if !h.is_finite() || h <= 0.0 {
                return Err(ScalarRateRefusal::InvalidInput { field: "horizon" });
            }
            // Exact integer-to-binary64 admission, including the factor two.
            let n = q
                .checked_add(1)
                .and_then(|v| v.checked_mul(2))
                .ok_or(ScalarRateRefusal::InvalidInput { field: "q" })?;
            if n > (1u64 << 53) as usize {
                return Err(ScalarRateRefusal::InvalidInput { field: "q" });
            }
            let mut op = ScalarOps { work };
            let kh = op.mul(sp(kappa), sp(h))?;
            let rate = nonnegative(op.mul(kh, sp(n as f64))?);
            Ok(Self {
                kappa_bits: kappa.to_bits(),
                horizon_bits: h.to_bits(),
                q,
                rate,
            })
        })();
        if result.is_err() {
            work.query_refusals = work.query_refusals.saturating_add(1);
        }
        result
    }
    pub fn interval(&self) -> Interval {
        self.rate
    }
    pub fn input_bits(&self) -> (u64, u64, usize) {
        (self.kappa_bits, self.horizon_bits, self.q)
    }

    /// Rebinding is checked against exact input bits before any query work.
    pub fn query(
        &self,
        kappa: f64,
        h: f64,
        q: usize,
        time: Interval,
        r: usize,
        d: usize,
        max_width: f64,
        scratch: &mut [Interval],
        work: &mut ScalarRateWork,
    ) -> ScalarResult<DampedForcingMomentResult> {
        if self.input_bits() != (kappa.to_bits(), h.to_bits(), q) {
            work.validation_attempts = work.validation_attempts.saturating_add(1);
            work.query_refusals = work.query_refusals.saturating_add(1);
            return Err(ScalarRateRefusal::BindingMismatch {
                field: "normalized_rate_inputs",
            });
        }
        checked_interval_forcing_moment(self.rate, time, r, d, max_width, scratch, work)
    }
}

fn sp(x: f64) -> Interval {
    Interval { lo: x, hi: x }
}
fn nonnegative(x: Interval) -> Interval {
    Interval {
        lo: x.lo.max(0.0),
        hi: x.hi.max(0.0),
    }
}

struct ScalarOps<'a> {
    work: &'a mut ScalarRateWork,
}
impl ScalarOps<'_> {
    fn charge(&mut self) -> ScalarResult<()> {
        let used = self.work.interval_primitive_attempts.checked_add(1).ok_or(
            ScalarRateRefusal::MomentWorkBudgetExceeded {
                used: u64::MAX,
                cap: SCALAR_RATE_PRIMITIVE_CAP,
            },
        )?;
        self.work.interval_primitive_attempts = used;
        if used > SCALAR_RATE_PRIMITIVE_CAP {
            return Err(ScalarRateRefusal::MomentWorkBudgetExceeded {
                used,
                cap: SCALAR_RATE_PRIMITIVE_CAP,
            });
        }
        Ok(())
    }
    fn add(&mut self, a: Interval, b: Interval) -> ScalarResult<Interval> {
        self.charge()?;
        a.add(b).map_err(|_| ScalarRateRefusal::ArithmeticRefusal {
            stage: "interval_add",
        })
    }
    fn sub(&mut self, a: Interval, b: Interval) -> ScalarResult<Interval> {
        self.charge()?;
        a.sub(b).map_err(|_| ScalarRateRefusal::ArithmeticRefusal {
            stage: "interval_sub",
        })
    }
    fn mul(&mut self, a: Interval, b: Interval) -> ScalarResult<Interval> {
        self.charge()?;
        a.mul(b).map_err(|_| ScalarRateRefusal::ArithmeticRefusal {
            stage: "interval_mul",
        })
    }
    fn div(&mut self, a: Interval, b: Interval) -> ScalarResult<Interval> {
        self.charge()?;
        a.div(b).map_err(|_| ScalarRateRefusal::ArithmeticRefusal {
            stage: "interval_div",
        })
    }
    fn exp(&mut self, x: f64) -> ScalarResult<Interval> {
        self.charge()?;
        exp_interval(x).map_err(|_| ScalarRateRefusal::ArithmeticRefusal {
            stage: "endpoint_exp",
        })
    }
    fn pow(&mut self, x: Interval, n: usize) -> ScalarResult<Interval> {
        let mut y = sp(1.0);
        for _ in 0..n {
            y = self.mul(y, x)?;
        }
        Ok(y)
    }
    fn beta(&mut self, j: usize, k: usize) -> ScalarResult<Interval> {
        let mut b = self.div(sp(1.0), sp((j + 1) as f64))?;
        for i in 1..=k {
            let p = self.mul(b, sp(i as f64))?;
            b = self.div(p, sp((j + i + 1) as f64))?;
        }
        Ok(b)
    }
    fn width(&mut self, x: Interval) -> ScalarResult<f64> {
        self.work.width_checks = self.work.width_checks.saturating_add(1);
        self.charge()?;
        sub_up(x.hi, x.lo).map_err(|_| ScalarRateRefusal::ArithmeticRefusal { stage: "width" })
    }
    fn endpoint(
        &mut self,
        a: f64,
        t: f64,
        r: usize,
        d: usize,
        scratch: &mut [Interval],
    ) -> ScalarResult<Interval> {
        self.work.endpoint_attempts = self.work.endpoint_attempts.saturating_add(1);
        if t == 0.0 {
            return Ok(sp(0.0));
        }
        let z = nonnegative(self.mul(sp(a), sp(t))?);
        let beta = self.beta(d, r)?;
        let normalized = if z.hi == 0.0 {
            beta
        } else if z.hi <= 1.0 {
            let mut sum = sp(0.0);
            let mut power = sp(1.0);
            let mut fact = sp(1.0);
            let mut b = beta;
            for n in 0..28 {
                let divided = self.div(power, fact)?;
                let term = self.mul(divided, b)?;
                sum = if n % 2 == 0 {
                    self.add(sum, term)?
                } else {
                    self.sub(sum, term)?
                };
                power = self.mul(power, z)?;
                fact = self.mul(fact, sp((n + 1) as f64))?;
                let bn = self.mul(b, sp((d + n + 1) as f64))?;
                b = self.div(bn, sp((d + r + n + 2) as f64))?;
            }
            let divided = self.div(power, fact)?;
            let rem = self.mul(divided, b)?;
            let enclosed = self.add(
                sum,
                Interval {
                    lo: -rem.hi,
                    hi: rem.hi,
                },
            )?;
            Interval {
                lo: enclosed.lo.max(0.0),
                hi: enclosed.hi.min(beta.hi),
            }
        } else {
            let elo = self.exp(-z.hi)?;
            let ehi = self.exp(-z.lo)?;
            let exponential = Interval {
                lo: elo.lo,
                hi: ehi.hi,
            };
            let top = d + r + 45;
            let bound = self.div(sp(1.0), sp((top + 1) as f64))?;
            let mut e = Interval {
                lo: 0.0,
                hi: bound.hi,
            };
            for m in (1..=top).rev() {
                let product = self.mul(z, e)?;
                let numerator = self.add(product, exponential)?;
                e = self.div(numerator, sp(m as f64))?;
                // Exact integral identity E_(m-1)<=1/m prevents useless
                // recurrence overgrowth; this intersection adds no hypothesis.
                let upper = self.div(sp(1.0), sp(m as f64))?;
                e = Interval {
                    lo: e.lo.max(0.0),
                    hi: e.hi.min(upper.hi),
                };
                if m - 1 <= d + r {
                    scratch[m - 1] = e;
                }
            }
            let mut sum = sp(0.0);
            let mut binomial = sp(1.0);
            for l in 0..=r {
                if l > 0 {
                    let product = self.mul(binomial, sp((r - l + 1) as f64))?;
                    binomial = self.div(product, sp(l as f64))?;
                }
                let term = self.mul(binomial, scratch[d + l])?;
                sum = if l % 2 == 0 {
                    self.add(sum, term)?
                } else {
                    self.sub(sum, term)?
                };
            }
            Interval {
                lo: sum.lo.max(0.0),
                hi: sum.hi.min(beta.hi),
            }
        };
        if normalized.lo > normalized.hi {
            return Err(ScalarRateRefusal::ArithmeticRefusal {
                stage: "kernel_intersection",
            });
        }
        let scale = self.pow(sp(t), r + d + 1)?;
        Ok(nonnegative(self.mul(normalized, scale)?))
    }
    fn cell(
        &mut self,
        a: f64,
        t: f64,
        l: f64,
        u: f64,
        r: usize,
        d: usize,
    ) -> ScalarResult<Interval> {
        self.work.cells_attempted = self.work.cells_attempted.saturating_add(1);
        let delta = nonnegative(self.sub(sp(u), sp(l))?);
        let tu = nonnegative(self.sub(sp(t), sp(u))?);
        let mut mass = sp(0.0);
        let mut bi = sp(1.0);
        for i in 0..=r {
            if i > 0 {
                let p = self.mul(bi, sp((r - i + 1) as f64))?;
                bi = self.div(p, sp(i as f64))?;
            }
            let mut bj = sp(1.0);
            for j in 0..=d {
                if j > 0 {
                    let p = self.mul(bj, sp((d - j + 1) as f64))?;
                    bj = self.div(p, sp(j as f64))?;
                }
                let mut term = self.mul(bi, bj)?;
                let p = self.pow(tu, r - i)?;
                term = self.mul(term, p)?;
                let p = self.pow(sp(l), d - j)?;
                term = self.mul(term, p)?;
                let p = self.pow(delta, i + j + 1)?;
                term = self.mul(term, p)?;
                let b = self.beta(i, j)?;
                term = self.mul(term, b)?;
                mass = self.add(mass, term)?;
                self.work.coefficient_items = self.work.coefficient_items.saturating_add(1);
            }
        }
        let au = nonnegative(self.mul(sp(a), sp(u))?);
        let al = nonnegative(self.mul(sp(a), sp(l))?);
        let lower = self.exp(-au.hi)?;
        let upper = self.exp(-al.lo)?;
        Ok(nonnegative(self.mul(
            mass,
            Interval {
                lo: lower.lo.max(0.0),
                hi: upper.hi,
            },
        )?))
    }
}

/// Lower-level arithmetic API. The caller owns the inclusion meaning of the
/// supplied rate/time intervals; this function does not certify their origin.
/// The generated API above binds the rate to exact original input bits.
pub fn checked_interval_forcing_moment(
    rate: Interval,
    time: Interval,
    r: usize,
    d: usize,
    max_width: f64,
    scratch: &mut [Interval],
    work: &mut ScalarRateWork,
) -> ScalarResult<DampedForcingMomentResult> {
    work.validation_attempts = work.validation_attempts.saturating_add(1);
    let result = (|| {
        for (field, x) in [("rate", rate), ("time", time)] {
            if !x.lo.is_finite() || !x.hi.is_finite() || x.lo < 0.0 || x.lo > x.hi {
                return Err(ScalarRateRefusal::InvalidInput { field });
            }
        }
        if !max_width.is_finite() || max_width < 0.0 {
            return Err(ScalarRateRefusal::InvalidInput { field: "max_width" });
        }
        if r > 64 || d > 64 {
            return Err(ScalarRateRefusal::UnsupportedMomentOrder {
                required: r.max(d),
                supported: 64,
            });
        }
        if scratch.len() < SCALAR_RATE_SCRATCH_LEN {
            return Err(ScalarRateRefusal::ScratchTooSmall {
                required: SCALAR_RATE_SCRATCH_LEN,
                provided: scratch.len(),
            });
        }
        work.scratch_bytes = (SCALAR_RATE_SCRATCH_LEN * std::mem::size_of::<Interval>()) as u64;
        let mut op = ScalarOps { work };
        let lower = op.endpoint(rate.hi, time.lo, r, d, scratch)?;
        let upper = op.endpoint(rate.lo, time.hi, r, d, scratch)?;
        let mut interval = Interval {
            lo: lower.lo,
            hi: upper.hi,
        };
        let mut width = op.width(interval)?;
        if width > max_width {
            // Only a width failure of the large-rate recurrence enables cells.
            let z = nonnegative(op.mul(rate, time)?);
            if z.hi <= 1.0 {
                return Err(ScalarRateRefusal::MomentWidthExceeded {
                    width,
                    limit: max_width,
                });
            }
            op.work.fallback_attempts = op.work.fallback_attempts.saturating_add(1);
            let cell_start = op.work.cells_attempted;
            // Uniform dyadic refinement: levels 0..11 evaluate 8190 endpoint
            // cells in total, at most 2048 leaves per endpoint. Depth/leaves
            // remain below the frozen maxima 20/4096.
            for depth in 0..=20 {
                let leaves = 1usize << depth;
                if leaves > 4096 || op.work.cells_attempted - cell_start + 2 * leaves as u64 > 8191
                {
                    break;
                }
                let mut lo_sum = sp(0.0);
                let mut hi_sum = sp(0.0);
                for index in 0..leaves {
                    // Exact binary dyadic coordinates, outward physical endpoints.
                    // Multiplication can round: use lower/upper coordinates to
                    // enclose each cell's mass rather than leave mesh gaps.
                    let f0 = sp(index as f64 / leaves as f64);
                    let f1 = sp((index + 1) as f64 / leaves as f64);
                    let ll = op.mul(sp(time.lo), f0)?;
                    let lu = op.mul(sp(time.lo), f1)?;
                    let hl = op.mul(sp(time.hi), f0)?;
                    let hu = op.mul(sp(time.hi), f1)?;
                    // Dyadic scaling by <=2^20 is exact for normal fixtures.
                    // General underflow/nonexact coordinates are refused rather
                    // than overlapping or gapping an allegedly exact partition.
                    if ll.lo != ll.hi || lu.lo != lu.hi || hl.lo != hl.hi || hu.lo != hu.hi {
                        return Err(ScalarRateRefusal::ArithmeticRefusal {
                            stage: "nonexact_dyadic_partition",
                        });
                    }
                    let low = op.cell(rate.hi, time.lo, ll.lo, lu.hi, r, d)?;
                    let high = op.cell(rate.lo, time.hi, hl.lo, hu.hi, r, d)?;
                    lo_sum = op.add(lo_sum, low)?;
                    hi_sum = op.add(hi_sum, high)?;
                }
                let cells = Interval {
                    lo: lo_sum.lo,
                    hi: hi_sum.hi,
                };
                interval = Interval {
                    lo: interval.lo.max(cells.lo),
                    hi: interval.hi.min(cells.hi),
                };
                if interval.lo > interval.hi {
                    return Err(ScalarRateRefusal::ArithmeticRefusal {
                        stage: "fallback_intersection",
                    });
                }
                width = op.width(interval)?;
                if width <= max_width {
                    break;
                }
            }
        }
        if width > max_width {
            return Err(ScalarRateRefusal::MomentWidthExceeded {
                width,
                limit: max_width,
            });
        }
        op.work.output_items = op.work.output_items.saturating_add(1);
        Ok(DampedForcingMomentResult {
            interval,
            width_upper: width,
        })
    })();
    if result.is_err() {
        work.query_refusals = work.query_refusals.saturating_add(1);
    }
    result
}
