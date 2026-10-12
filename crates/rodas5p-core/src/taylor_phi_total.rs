//! A total-target certificate for the fused phi action (RVJ DAG node PP11,
//! `research/pp11_taylor_fused_total_20261004`). Research only and opt in:
//! no integrator calls this module.
//!
//! # Target
//!
//! `F = sum_{k=0}^{4} phi_k(h A) w_k` for the binary64 `A`, `h`, `w_k`
//! taken as exact reals. It is the top block of `exp(M) v` with
//! `M = [[hA, W], [0, J]]`, `W = [w_4, w_3, w_2, w_1]`, `J` the 4x4 upper
//! shift and `v = [w_0; e_4]` (Al-Mohy and Higham 2011, Theorem 2.1; the
//! layout of [`crate::taylor_phi::taylor_phi_action`]). That backend acts
//! on the stored matrix `M~`, whose top-left block is `fl(hA)`, and is
//! therefore EstimateOnly for `F`.
//!
//! # Bound
//!
//! For the caller's candidate `u` and the stepped certificate's own value
//! `y` of `exp(M~) v`, the triangle inequality gives
//!
//! `||u - F||_2 <= ||u - top(y)||_2 + ||y - exp(M~) v||_2
//!                + ||exp(M~) v - exp(M) v||_2`,
//!
//! where `||top(x)||_2 <= ||x||_2` lets the last two terms be bounded on the
//! whole augmented vector. The middle term is `E1`, the REV-02 directed
//! stepped certificate of `exp(M~) v` under the automatic step rule and
//! metric; the last term is the perturbation
//! `||Delta||_2 e^omega ||v||_2` (see [`certify_fused_phi_total`]).
//!
//! # Type separation
//!
//! [`FusedPhiCertificate`] is its own type with private fields and no
//! conversion from [`NonnormalExpCertificate`]: a certificate of `exp(M~) v`
//! (or of any `exp(A) v`) bounds a different target and cannot be passed
//! where a fused certificate is required.
//!
//! ```compile_fail
//! use rodas5p_core::nonnormal_certificate::NonnormalExpCertificate;
//! use rodas5p_core::taylor_phi_total::FusedPhiCertificate;
//! fn needs_fused(_: &FusedPhiCertificate) {}
//! fn misuse(exp_only: &NonnormalExpCertificate) {
//!     needs_fused(exp_only);
//! }
//! ```
//!
//! The same items compile when the fused type is passed:
//!
//! ```
//! use rodas5p_core::taylor_phi_total::{FusedPhiCertificate, certify_fused_phi_total};
//! fn needs_fused(c: &FusedPhiCertificate) -> f64 {
//!     c.bound()
//! }
//! let w = [vec![1.0], vec![0.0], vec![0.0], vec![0.0], vec![0.0]];
//! let cert = certify_fused_phi_total(&[vec![-1.0]], 0.5, &w, &[0.6]).unwrap();
//! assert!(needs_fused(&cert) >= 0.0);
//! ```
//!
//!
//! # Power-of-two normalization (research node PY04)
//!
//! [`certify_fused_phi_total_normalized`] uses `F(A, h, sigma w) = sigma
//! F(A, h, w)` for a power of two `sigma` (research node
//! `research/py04_fused_normalization_20261011`): it certifies the
//! normalized `v = fl(w / sigma)` with [`certify_fused_phi_total`] and
//! rescales the candidate and the bound outward, enclosing every rounding of
//! the normalization. Its result, [`NormalizedFusedPhiCertificate`], is a
//! separate type with private fields and no conversion from
//! [`FusedPhiCertificate`]:
//!
//! ```compile_fail
//! use rodas5p_core::taylor_phi_total::{FusedPhiCertificate, NormalizedFusedPhiCertificate};
//! fn needs_normalized(_: &NormalizedFusedPhiCertificate) {}
//! fn misuse(unnormalized: &FusedPhiCertificate) {
//!     needs_normalized(unnormalized);
//! }
//! ```
//!
//! ```compile_fail
//! use rodas5p_core::taylor_phi_total::{FusedPhiCertificate, NormalizedFusedPhiCertificate};
//! fn convert(unnormalized: FusedPhiCertificate) -> NormalizedFusedPhiCertificate {
//!     unnormalized.into()
//! }
//! ```
//!
//! The normalized type itself is accepted:
//!
//! ```
//! use rodas5p_core::taylor_phi_total::{
//!     NormalizedFusedPhiCertificate, certify_fused_phi_total_normalized,
//! };
//! fn needs_normalized(c: &NormalizedFusedPhiCertificate) -> f64 {
//!     c.bound()
//! }
//! let w = [vec![1.0e-310], vec![0.0], vec![0.0], vec![0.0], vec![0.0]];
//! let cert = certify_fused_phi_total_normalized(&[vec![-1.0]], 0.5, &w, None).unwrap();
//! assert!(needs_normalized(&cert) >= 0.0);
//! ```
//! [`NonnormalExpCertificate`]: crate::nonnormal_certificate::NonnormalExpCertificate

use serde::Serialize;

use crate::DenseMatrix;
use crate::directed::{
    Interval, PowerOfTwo, add_up, div_up, exp_interval, mul_down, mul_up, scale_pow2_staged,
    sqrt_up, sub_up,
};
use crate::nonnormal_certificate::{
    AutoMetric, NonnormalBoundStatus, NonnormalExpCertificate, certify_exp_action,
    certify_exp_action_stepped, osborne_metric, stepping_rule,
};
use crate::polynomial_action::JOINT_PHI_TERMS;
use crate::taylor_phi::taylor_phi_action;
use crate::{CoreError, CoreResult};

pub const FUSED_PHI_TOTAL_SCHEMA: &str = "vigilode-fused-phi-total-v1";

/// The degree of the automatic stepped certificate (the value
/// `certify_exp_action_auto` uses).
const STEPPED_DEGREE: usize = 20;

/// Whether the total bound is finite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FusedPhiStatus {
    Bounded,
    /// The stepped certificate gave no bound or a term overflowed: the
    /// certificate says nothing about the candidate.
    Unbounded,
}

/// The three terms of the total bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FusedPhiTerm {
    /// `||u - top(y)||_2`, rounded up.
    CandidateDistance,
    /// `E1`, the stepped certificate of `exp(M~) v`.
    SteppedExp,
    /// `||Delta||_2 e^omega ||v||_2`.
    Perturbation,
}

/// One metric of the automatic stepped certificate.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SteppedAttempt {
    metric: AutoMetric,
    steps: Option<usize>,
    bounded: bool,
    error_upper: f64,
    /// Why the attempt gave no bound, when it did not.
    note: Option<String>,
}

impl SteppedAttempt {
    pub fn metric(&self) -> AutoMetric {
        self.metric
    }
    pub fn steps(&self) -> Option<usize> {
        self.steps
    }
    pub fn bounded(&self) -> bool {
        self.bounded
    }
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// A certified upper bound on `||u - sum_k phi_k(hA) w_k||_2` for the exact
/// binary64 inputs. Built only by [`certify_fused_phi_total`]; serialized,
/// never deserialized.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FusedPhiCertificate {
    schema: &'static str,
    status: FusedPhiStatus,
    unbounded_reason: Option<String>,
    dimension: usize,
    h: f64,
    candidate: Vec<f64>,
    bound: f64,
    candidate_distance: f64,
    stepped_error: f64,
    perturbation: f64,
    dominant: FusedPhiTerm,
    delta_one_norm: f64,
    delta_inf_norm: f64,
    delta_two_norm: f64,
    omega: f64,
    v_two_norm: f64,
    chosen_metric: Option<AutoMetric>,
    stepped_candidate: Vec<f64>,
    attempts: Vec<SteppedAttempt>,
}

impl FusedPhiCertificate {
    pub fn schema(&self) -> &'static str {
        self.schema
    }
    pub fn status(&self) -> FusedPhiStatus {
        self.status
    }
    pub fn unbounded_reason(&self) -> Option<&str> {
        self.unbounded_reason.as_deref()
    }
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    pub fn h(&self) -> f64 {
        self.h
    }
    /// The caller's candidate `u` the bound is about.
    pub fn candidate(&self) -> &[f64] {
        &self.candidate
    }
    /// `||u - F||_2 <= bound` (infinite when [`FusedPhiStatus::Unbounded`]).
    pub fn bound(&self) -> f64 {
        self.bound
    }
    pub fn candidate_distance(&self) -> f64 {
        self.candidate_distance
    }
    /// `E1`.
    pub fn stepped_error(&self) -> f64 {
        self.stepped_error
    }
    pub fn perturbation(&self) -> f64 {
        self.perturbation
    }
    pub fn dominant(&self) -> FusedPhiTerm {
        self.dominant
    }
    pub fn delta_one_norm(&self) -> f64 {
        self.delta_one_norm
    }
    pub fn delta_inf_norm(&self) -> f64 {
        self.delta_inf_norm
    }
    /// Upper bound on `||hA - fl(hA)||_2`.
    pub fn delta_two_norm(&self) -> f64 {
        self.delta_two_norm
    }
    /// `omega >= max(mu(M), mu(M~), 0)` (infinite if it overflowed).
    pub fn omega(&self) -> f64 {
        self.omega
    }
    pub fn v_two_norm(&self) -> f64 {
        self.v_two_norm
    }
    /// The metric of the stepped certificate used, `None` when neither
    /// metric gave a bound.
    pub fn chosen_metric(&self) -> Option<AutoMetric> {
        self.chosen_metric
    }
    /// The stepped certificate's own value `y` of `exp(M~) v` (length
    /// `n + 4`).
    pub fn stepped_candidate(&self) -> &[f64] {
        &self.stepped_candidate
    }
    /// `top_n(y)`: an alternative candidate for `F`.
    pub fn stepped_top(&self) -> &[f64] {
        &self.stepped_candidate[..self.dimension]
    }
    /// Both metrics of the automatic certificate, identity first.
    pub fn attempts(&self) -> &[SteppedAttempt] {
        &self.attempts
    }
}

fn invalid(why: &str) -> CoreError {
    CoreError::InvalidInput(format!("fused phi certificate: {why}"))
}

/// Certify `||candidate - sum_{k=0}^{4} phi_k(hA) w_k||_2` for the exact
/// binary64 `a` (rows of an `n x n` matrix), `h` and `w`; see the module
/// documentation for the decomposition.
///
/// Refused (an error): an empty or non-square `a`, a `w_k` or candidate of
/// the wrong length, a non-finite entry, `h` negative or non-finite, and an
/// `fl(h a_ij)` that overflows. An overflow inside the certificate is not
/// an error: the result is [`FusedPhiStatus::Unbounded`].
pub fn certify_fused_phi_total(
    a: &[Vec<f64>],
    h: f64,
    w: &[Vec<f64>; JOINT_PHI_TERMS],
    candidate: &[f64],
) -> CoreResult<FusedPhiCertificate> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi certificate: A must be a nonempty n x n matrix".into(),
        ));
    }
    if w.iter().any(|wk| wk.len() != n) || candidate.len() != n {
        return Err(CoreError::Dimension(
            "fused phi certificate: every w_k and the candidate must have length n".into(),
        ));
    }
    if !(h.is_finite() && h >= 0.0) {
        return Err(invalid("h must be finite and nonnegative"));
    }
    if !a
        .iter()
        .flatten()
        .chain(w.iter().flatten())
        .chain(candidate)
        .all(|x| x.is_finite())
    {
        return Err(invalid("non-finite A, w_k or candidate"));
    }
    let p = JOINT_PHI_TERMS - 1;
    let size = n + p;
    // M~ in the layout of taylor_phi_action: fl(h a_ij), then the columns
    // w_4, w_3, w_2, w_1, and the shift J.
    let mut m = vec![vec![0.0; size]; size];
    // Entrywise enclosure of the exact h a_ij. Round to nearest returns one
    // of the two directed results, so [mul_down, mul_up] contains both h a_ij
    // and fl(h a_ij).
    let mut ha = vec![vec![Interval::point(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            let rounded = h * a[i][j];
            if !rounded.is_finite() {
                return Err(invalid("fl(h a_ij) overflows"));
            }
            m[i][j] = rounded;
            ha[i][j] = Interval::new(mul_down(h, a[i][j])?, mul_up(h, a[i][j])?)?;
        }
        for c in 0..p {
            m[i][n + c] = w[p - c][i];
        }
    }
    for c in 0..p - 1 {
        m[n + c][n + c + 1] = 1.0;
    }
    let mut v = vec![0.0; size];
    v[..n].copy_from_slice(&w[0]);
    v[size - 1] = 1.0;

    // |Delta_ij| = |h a_ij - fl(h a_ij)| <= mul_up - mul_down: both values
    // lie in that interval (rounded up, so the width is not understated).
    let mut col = vec![0.0_f64; n];
    let mut row = vec![0.0_f64; n];
    for i in 0..n {
        for j in 0..n {
            let width = sub_up(ha[i][j].hi, ha[i][j].lo)?;
            row[i] = add_up(row[i], width)?;
            col[j] = add_up(col[j], width)?;
        }
    }
    let delta_one = col.iter().copied().fold(0.0_f64, f64::max);
    let delta_inf = row.iter().copied().fold(0.0_f64, f64::max);
    // ||X||_2^2 <= ||X||_1 ||X||_inf (||X||_2^2 = rho(X^T X) <= ||X^T X||_inf
    // <= ||X^T||_inf ||X||_inf), rounded up.
    let delta_two = sqrt_up(mul_up(delta_one, delta_inf)?)?;

    // ||v||_2, rounded up; an overflow leaves it infinite.
    let v_two = v
        .iter()
        .try_fold(0.0, |acc, x| add_up(acc, mul_up(*x, *x)?))
        .and_then(sqrt_up)
        .unwrap_or(f64::INFINITY);

    // omega: the Gershgorin upper bound of the symmetric part evaluated on
    // the interval matrix whose top-left block is `ha`. Every point matrix in
    // it, M and M~ among them, has its Gershgorin bound below the interval
    // one, and lambda_max((X + X^T) / 2) = mu_2(X) is below the Gershgorin
    // bound of the symmetric part. An overflow leaves omega infinite.
    let omega = log_norm_upper(&m, &ha, n)
        .map(|mu| mu.max(0.0))
        .unwrap_or(f64::INFINITY);

    // Perturbation. exp(M) - exp(M~) = int_0^1 e^{(1-s) M} (M - M~) e^{s M~} ds
    // and ||e^{t X}||_2 <= e^{t mu_2(X)} for t >= 0, so with
    // omega >= max(mu(M), mu(M~), 0) the integrand is at most
    // e^omega ||M - M~||_2, and M - M~ is Delta padded with zeros. When the
    // enclosure of Delta is 0, M = M~ and the term is exactly 0 (no
    // exponential is evaluated).
    let perturbation = if delta_two == 0.0 {
        0.0
    } else if omega.is_finite() && v_two.is_finite() {
        exp_interval(omega)
            .and_then(|growth| mul_up(mul_up(delta_two, growth.hi)?, v_two))
            .unwrap_or(f64::INFINITY)
    } else {
        f64::INFINITY
    };

    // E1: the automatic directed stepped certificate of exp(M~) v.
    let attempts_full =
        [AutoMetric::Identity, AutoMetric::Osborne].map(|metric| stepped(&m, &v, metric));
    let chosen = {
        let identity = attempts_full[0].1.as_ref();
        let osborne = attempts_full[1].1.as_ref();
        // Both are valid bounds; the smaller one is kept, identity on ties
        // (the rule of certify_exp_action_auto).
        match (identity, osborne) {
            (Some(i), Some(o)) if o.error_upper < i.error_upper => Some((AutoMetric::Osborne, o)),
            (Some(i), _) => Some((AutoMetric::Identity, i)),
            (None, Some(o)) => Some((AutoMetric::Osborne, o)),
            (None, None) => None,
        }
    };
    let attempts = attempts_full.iter().map(|(a, _)| a.clone()).collect();
    let (chosen_metric, stepped_error, stepped_candidate) = match chosen {
        Some((metric, cert)) => (Some(metric), cert.error_upper, cert.candidate.clone()),
        None => (None, f64::INFINITY, vec![f64::NAN; size]),
    };

    // ||u - top(y)||_2 with each difference enclosed, rounded up.
    let candidate_distance = distance_upper(candidate, &stepped_candidate[..n]);

    let total = add_up(candidate_distance, stepped_error)
        .and_then(|s| add_up(s, perturbation))
        .unwrap_or(f64::INFINITY);
    let terms = [
        (FusedPhiTerm::CandidateDistance, candidate_distance),
        (FusedPhiTerm::SteppedExp, stepped_error),
        (FusedPhiTerm::Perturbation, perturbation),
    ];
    let dominant = terms
        .iter()
        .fold(terms[0], |best, t| if t.1 > best.1 { *t } else { best })
        .0;
    let unbounded_reason = if chosen_metric.is_none() {
        Some("the stepped certificate of exp(M~) v gave no bound under either metric".to_string())
    } else if !perturbation.is_finite() {
        Some("the perturbation term overflows".to_string())
    } else if !candidate_distance.is_finite() {
        Some("the candidate distance overflows".to_string())
    } else if !total.is_finite() {
        Some("the total overflows".to_string())
    } else {
        None
    };
    let status = if unbounded_reason.is_none() {
        FusedPhiStatus::Bounded
    } else {
        FusedPhiStatus::Unbounded
    };
    Ok(FusedPhiCertificate {
        schema: FUSED_PHI_TOTAL_SCHEMA,
        status,
        unbounded_reason,
        dimension: n,
        h,
        candidate: candidate.to_vec(),
        bound: if status == FusedPhiStatus::Bounded {
            total
        } else {
            f64::INFINITY
        },
        candidate_distance,
        stepped_error,
        perturbation,
        dominant,
        delta_one_norm: delta_one,
        delta_inf_norm: delta_inf,
        delta_two_norm: delta_two,
        omega,
        v_two_norm: v_two,
        chosen_metric,
        stepped_candidate,
        attempts,
    })
}

/// The stepped certificate under one metric with the REV-02 step rule. Any
/// failure (an overflow inside the certificate, an unusable balancing) is
/// recorded and gives no bound; it never gives a smaller one.
fn stepped(
    m: &[Vec<f64>],
    v: &[f64],
    metric: AutoMetric,
) -> (SteppedAttempt, Option<NonnormalExpCertificate>) {
    let failed = |steps, note: String| {
        (
            SteppedAttempt {
                metric,
                steps,
                bounded: false,
                error_upper: f64::INFINITY,
                note: Some(note),
            },
            None,
        )
    };
    let d = match metric {
        AutoMetric::Identity => None,
        AutoMetric::Osborne => match osborne_metric(m) {
            Ok(d) => Some(d),
            Err(e) => return failed(None, format!("osborne metric: {e}")),
        },
    };
    let probe = match certify_exp_action(m, v, 0.0, d.as_deref(), STEPPED_DEGREE, None) {
        Ok(p) => p,
        Err(e) => return failed(None, format!("numerical range: {e}")),
    };
    let steps = stepping_rule(probe.numerical_range, 1.0);
    match certify_exp_action_stepped(m, v, 1.0, d.as_deref(), STEPPED_DEGREE, steps) {
        Ok(cert)
            if cert.status == NonnormalBoundStatus::Bounded
                && cert.error_upper.is_finite()
                && cert.candidate.iter().all(|x| x.is_finite()) =>
        {
            (
                SteppedAttempt {
                    metric,
                    steps: Some(steps),
                    bounded: true,
                    error_upper: cert.error_upper,
                    note: None,
                },
                Some(cert),
            )
        }
        Ok(_) => failed(Some(steps), "stepped certificate unbounded".into()),
        Err(e) => failed(Some(steps), format!("stepped certificate: {e}")),
    }
}

/// `max_i (H_ii + sum_{j != i} |H_ij|)` for `H = (X + X^T) / 2`, `X` the
/// augmented matrix with the top-left `n x n` block replaced by `ha`,
/// evaluated outward.
#[allow(clippy::needless_range_loop)] // (i, j) and (j, i) are both read
fn log_norm_upper(m: &[Vec<f64>], ha: &[Vec<Interval>], n: usize) -> CoreResult<f64> {
    let size = m.len();
    let entry = |i: usize, j: usize| -> CoreResult<Interval> {
        if i < n && j < n {
            Ok(ha[i][j])
        } else {
            Interval::point(m[i][j])
        }
    };
    let half = Interval::point(0.5)?;
    let mut upper = f64::NEG_INFINITY;
    for i in 0..size {
        let mut off = 0.0;
        for j in 0..size {
            if j != i {
                off = add_up(off, entry(i, j)?.add(entry(j, i)?)?.mul(half)?.mag())?;
            }
        }
        upper = upper.max(add_up(entry(i, i)?.hi, off)?);
    }
    Ok(upper)
}

/// `||u - y||_2` with each `u_i - y_i` enclosed, rounded up; infinite when
/// `y` is not finite or the sum overflows.
fn distance_upper(u: &[f64], y: &[f64]) -> f64 {
    let sum = u.iter().zip(y).try_fold(0.0, |acc, (ui, yi)| {
        let gap = Interval::point(*ui)?.sub(Interval::point(*yi)?)?.mag();
        add_up(acc, mul_up(gap, gap)?)
    });
    sum.and_then(sqrt_up).unwrap_or(f64::INFINITY)
}

// ------------------------------------------------------------------------
// Power-of-two normalization (research node PY04).

pub const FUSED_PHI_NORMALIZED_SCHEMA: &str = "vigilode-fused-phi-normalized-v1";

/// Typed refusal of [`certify_fused_phi_total_normalized`]: the rescaled
/// candidate, or a bound, lies above `f64::MAX`.
pub const FUSED_PHI_RANGE_UNSUPPORTED: &str = "FUSED_PHI_RANGE_UNSUPPORTED";

/// Truncation budget of the normalized Taylor candidate (absolute, 2-norm,
/// on the normalized data, whose largest entry lies in `[1, 2)`): the PP11
/// export's value.
pub const NORMALIZED_TAYLOR_BUDGET: f64 = 1e-13;

/// Where the normalized candidate `z` comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedCandidateSource {
    /// `taylor_phi_action` on `(A, h, v)`.
    Taylor,
    /// Taylor rejected the domain: the stepped certificate's own top block.
    SteppedTop,
    /// Taylor rejected and the stepped top was not finite: zeros.
    Zeros,
}

/// The three terms of `bound(F_hat)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedFusedTerm {
    /// `up(sigma E)`.
    ScaledCertificate,
    /// `R = ||fl(sigma z) - sigma z||_2`, enclosed.
    RescaleRounding,
    /// `L`, the rounding of `w / sigma` propagated through `phi_k(hA)`.
    NormalizationRounding,
}

/// Work of the normalization: binary64 entries scaled by a power of two and
/// the staged multiplications they took (counted, not timed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NormalizationWork {
    /// `w_k -> v_k`: `5 n` entries.
    pub input_entries: u64,
    pub input_stages: u64,
    /// `z -> F_hat`: `n` entries.
    pub candidate_entries: u64,
    pub candidate_stages: u64,
    /// Scalings inside the bound (`sigma E`, `R`, `L`, the caller distance).
    pub bound_scalings: u64,
    pub bound_stages: u64,
}

impl NormalizationWork {
    pub fn entries_scaled(&self) -> u64 {
        self.input_entries + self.candidate_entries + self.bound_scalings
    }
    pub fn stages(&self) -> u64 {
        self.input_stages + self.candidate_stages + self.bound_stages
    }
}

/// A certified upper bound on `||F_hat - sum_k phi_k(hA) w_k||_2` (and, when
/// a caller candidate `u` is given, on `||u - F||_2`) for the exact binary64
/// inputs, from the certificate of the normalized data. Built only by
/// [`certify_fused_phi_total_normalized`]; serialized, never deserialized;
/// no conversion from [`FusedPhiCertificate`] exists.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NormalizedFusedPhiCertificate {
    schema: &'static str,
    status: FusedPhiStatus,
    unbounded_reason: Option<String>,
    dimension: usize,
    h: f64,
    exponent: i64,
    normalized_vectors: [Vec<f64>; JOINT_PHI_TERMS],
    normalization_inexact: [usize; JOINT_PHI_TERMS],
    normalized_candidate: Vec<f64>,
    normalized_source: NormalizedCandidateSource,
    normalized_taylor_rejection: Option<String>,
    normalized: FusedPhiCertificate,
    candidate: Vec<f64>,
    rescale_inexact: usize,
    scaled_certificate: f64,
    rescale_rounding: f64,
    normalization_rounding: f64,
    log_norm_upper: f64,
    bound: f64,
    dominant: NormalizedFusedTerm,
    caller_candidate: Option<Vec<f64>>,
    caller_distance: Option<f64>,
    caller_bound: Option<f64>,
    work: NormalizationWork,
}

impl NormalizedFusedPhiCertificate {
    pub fn schema(&self) -> &'static str {
        self.schema
    }
    /// `Unbounded` exactly when the certificate of the normalized data is.
    pub fn status(&self) -> FusedPhiStatus {
        self.status
    }
    pub fn unbounded_reason(&self) -> Option<&str> {
        self.unbounded_reason.as_deref()
    }
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    pub fn h(&self) -> f64 {
        self.h
    }
    /// `sigma = 2^e`, `e` the binary exponent of the largest `|w_k,i|`
    /// (0 when every `w_k` is zero).
    pub fn sigma(&self) -> PowerOfTwo {
        PowerOfTwo::new(self.exponent)
    }
    pub fn exponent(&self) -> i64 {
        self.exponent
    }
    /// `v_k = fl(w_k / sigma)` (round to nearest).
    pub fn normalized_vectors(&self) -> &[Vec<f64>; JOINT_PHI_TERMS] {
        &self.normalized_vectors
    }
    /// Per `k`, the number of entries of `w_k / sigma` that rounded.
    pub fn normalization_inexact(&self) -> [usize; JOINT_PHI_TERMS] {
        self.normalization_inexact
    }
    /// `z`, the candidate for `F(A, h, v)`.
    pub fn normalized_candidate(&self) -> &[f64] {
        &self.normalized_candidate
    }
    pub fn normalized_source(&self) -> NormalizedCandidateSource {
        self.normalized_source
    }
    pub fn normalized_taylor_rejection(&self) -> Option<&str> {
        self.normalized_taylor_rejection.as_deref()
    }
    /// The certificate of `z` for `F(A, h, v)`; its bound is `E`.
    pub fn normalized_certificate(&self) -> &FusedPhiCertificate {
        &self.normalized
    }
    /// `F_hat = fl(sigma z)` (staged, round to nearest).
    pub fn candidate(&self) -> &[f64] {
        &self.candidate
    }
    /// Entries of `sigma z` that rounded.
    pub fn rescale_inexact(&self) -> usize {
        self.rescale_inexact
    }
    /// `up(sigma E)` (infinite when `Unbounded`).
    pub fn scaled_certificate(&self) -> f64 {
        self.scaled_certificate
    }
    /// `R` (0 when every entry of `sigma z` is exact).
    pub fn rescale_rounding(&self) -> f64 {
        self.rescale_rounding
    }
    /// `L` (0 when every `w_k / sigma` is exact).
    pub fn normalization_rounding(&self) -> f64 {
        self.normalization_rounding
    }
    /// `mu >= mu_2(hA)` from the interval Gershgorin bound of the symmetric
    /// part of the exact `hA` (infinite if it overflowed).
    pub fn log_norm_upper(&self) -> f64 {
        self.log_norm_upper
    }
    /// `||F_hat - F||_2 <= bound` (infinite when `Unbounded`).
    pub fn bound(&self) -> f64 {
        self.bound
    }
    pub fn dominant(&self) -> NormalizedFusedTerm {
        self.dominant
    }
    pub fn caller_candidate(&self) -> Option<&[f64]> {
        self.caller_candidate.as_deref()
    }
    /// `||u - F_hat||_2`, rounded up.
    pub fn caller_distance(&self) -> Option<f64> {
        self.caller_distance
    }
    /// `||u - F||_2 <= ||u - F_hat||_2 + bound(F_hat)`.
    pub fn caller_bound(&self) -> Option<f64> {
        self.caller_bound
    }
    pub fn work(&self) -> NormalizationWork {
        self.work
    }
}

fn range_refusal(why: &str) -> CoreError {
    CoreError::InvalidInput(format!(
        "{FUSED_PHI_RANGE_UNSUPPORTED}: fused phi normalized certificate: {why}"
    ))
}

/// `sqrt(count) 2^exponent`, rounded up (0 for no entries).
fn quantum_norm_up(count: usize, exponent: i64, work: &mut NormalizationWork) -> CoreResult<f64> {
    if count == 0 {
        return Ok(0.0);
    }
    scaled_up(sqrt_up(count as f64)?, exponent, work)
}

/// `up(x 2^e)`, counted in `work.bound_*`.
fn scaled_up(x: f64, e: i64, work: &mut NormalizationWork) -> CoreResult<f64> {
    let s = scale_pow2_staged(x, e)?;
    work.bound_scalings += 1;
    work.bound_stages += u64::from(s.stages);
    s.up()
}

/// `||x||_2` for nonnegative magnitudes, rounded up, with the squares taken
/// after scaling by the power of two of the largest entry (no squares
/// underflow to the outward floor or overflow). An overflowing result is an
/// error.
fn scaled_norm_up(mags: &[f64], work: &mut NormalizationWork) -> CoreResult<f64> {
    let largest = mags.iter().copied().fold(0.0_f64, f64::max);
    let Some(g) = PowerOfTwo::floor_of(largest) else {
        return Ok(0.0);
    };
    let mut sum = 0.0;
    for &m in mags {
        let s = scaled_up(m, -g.exponent, work)?;
        sum = add_up(sum, mul_up(s, s)?)?;
    }
    scaled_up(sqrt_up(sum)?, g.exponent, work)
}

/// `max_i (hi(hA_ii) + sum_{j != i} |(hA_ij + hA_ji) / 2|)` over the interval
/// enclosure of the exact `hA`, evaluated outward.
#[allow(clippy::needless_range_loop)] // (i, j) and (j, i) are both read
fn block_log_norm_upper(a: &[Vec<f64>], h: f64) -> CoreResult<f64> {
    let n = a.len();
    let mut ha = vec![vec![Interval::point(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            ha[i][j] = Interval::new(mul_down(h, a[i][j])?, mul_up(h, a[i][j])?)?;
        }
    }
    let half = Interval::point(0.5)?;
    let mut upper = f64::NEG_INFINITY;
    for i in 0..n {
        let mut off = 0.0;
        for j in 0..n {
            if j != i {
                off = add_up(off, ha[i][j].add(ha[j][i])?.mul(half)?.mag())?;
            }
        }
        upper = upper.max(add_up(ha[i][i].hi, off)?);
    }
    Ok(upper)
}

/// Certify `sum_{k=0}^{4} phi_k(hA) w_k` through the power-of-two
/// normalization `w = sigma (v + d)` (research node PY04).
///
/// 1. `sigma = 2^e`, `e` the binary exponent of the largest `|w_k,i|` over
///    all five `w_k` (one `sigma` for all), and `v_k = fl(w_k / sigma)` by
///    staged scaling.
/// 2. `z = taylor_phi_action(A, h, v)` with [`NORMALIZED_TAYLOR_BUDGET`]; if
///    Taylor rejects, the stepped certificate's top block (zeros if it is not
///    finite). `E` is the bound of `certify_fused_phi_total(A, h, v, z)`.
/// 3. `F_hat = fl(sigma z)`, staged.
/// 4. `bound(F_hat) = up(sigma E) + R + L`:
///    - `R >= ||F_hat - sigma z||_2`: 0 when every entry is exact, else
///      `sqrt(r) 2^-1075` rounded up, `r` the inexact entries (a power-of-two
///      scaling rounds only into the subnormal range, by at most half of
///      `2^-1074`);
///    - `L = sigma sum_k e^{max(mu, 0)} / k! sqrt(c_k) 2^-1075`, rounded up,
///      `c_k` the inexact entries of `v_k` (so `||w_k / sigma - v_k||_2 <=
///      sqrt(c_k) 2^-1075`), exactly 0 when every `v_k` is exact; it bounds
///      `sigma ||sum_k phi_k(hA) (w_k / sigma - v_k)||_2` because
///      `||phi_k(hA)||_2 <= e^{max(mu, 0)} / k!` for `mu >= mu_2(hA)`.
///
///    Then `||F_hat - F||_2 <= R + sigma ||z - F(A, h, v)||_2 + L`.
/// 5. For a caller candidate `u`: `||u - F_hat||_2` (rounded up, scaled) plus
///    `bound(F_hat)`.
///
/// Refused as invalid input: what [`certify_fused_phi_total`] refuses, and a
/// caller candidate of the wrong length or not finite. Refused with
/// [`FUSED_PHI_RANGE_UNSUPPORTED`]: a rescaled candidate entry or a bound
/// above `f64::MAX` (never a `Bounded` certificate with a non-finite value).
/// When the certificate of the normalized data is `Unbounded`, so is the
/// result (bound infinite).
pub fn certify_fused_phi_total_normalized(
    a: &[Vec<f64>],
    h: f64,
    w: &[Vec<f64>; JOINT_PHI_TERMS],
    caller_candidate: Option<&[f64]>,
) -> CoreResult<NormalizedFusedPhiCertificate> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi normalized certificate: A must be a nonempty n x n matrix".into(),
        ));
    }
    if w.iter().any(|wk| wk.len() != n) || caller_candidate.is_some_and(|u| u.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi normalized certificate: every w_k and the caller candidate must have length n"
                .into(),
        ));
    }
    if !(h.is_finite() && h >= 0.0) {
        return Err(invalid("h must be finite and nonnegative"));
    }
    if !a
        .iter()
        .flatten()
        .chain(w.iter().flatten())
        .chain(caller_candidate.into_iter().flatten())
        .all(|x| x.is_finite())
    {
        return Err(invalid("non-finite A, w_k or caller candidate"));
    }
    let mut work = NormalizationWork::default();

    // 1. Normalize by one power of two.
    let largest = w.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
    let sigma = PowerOfTwo::floor_of(largest).unwrap_or(PowerOfTwo::ONE);
    let inverse = sigma.inverse();
    let mut normalization_inexact = [0_usize; JOINT_PHI_TERMS];
    let mut v: [Vec<f64>; JOINT_PHI_TERMS] = std::array::from_fn(|_| Vec::with_capacity(n));
    for k in 0..JOINT_PHI_TERMS {
        for &x in &w[k] {
            let s = scale_pow2_staged(x, inverse.exponent)?;
            work.input_entries += 1;
            work.input_stages += u64::from(s.stages);
            if s.exact_vs_nearest != std::cmp::Ordering::Equal {
                normalization_inexact[k] += 1;
            }
            v[k].push(s.nearest);
        }
    }

    // 2. Candidate and certificate on the normalized data.
    let rows: Vec<&[f64]> = a.iter().map(Vec::as_slice).collect();
    let dense = DenseMatrix::from_rows(&rows)?;
    let (z, source, rejection) = match taylor_phi_action(&dense, h, &v, NORMALIZED_TAYLOR_BUDGET) {
        Ok(report) => (report.fused, NormalizedCandidateSource::Taylor, None),
        Err(error) => {
            let probe = certify_fused_phi_total(a, h, &v, &vec![0.0; n])?;
            let top = probe.stepped_top().to_vec();
            if top.iter().all(|x| x.is_finite()) {
                (
                    top,
                    NormalizedCandidateSource::SteppedTop,
                    Some(error.to_string()),
                )
            } else {
                (
                    vec![0.0; n],
                    NormalizedCandidateSource::Zeros,
                    Some(error.to_string()),
                )
            }
        }
    };
    let normalized = certify_fused_phi_total(a, h, &v, &z)?;

    // 3. Rescale the candidate.
    let mut candidate = Vec::with_capacity(n);
    let mut rescale_inexact = 0;
    let mut overflow = false;
    for &x in &z {
        let s = scale_pow2_staged(x, sigma.exponent)?;
        work.candidate_entries += 1;
        work.candidate_stages += u64::from(s.stages);
        overflow |= s.overflow;
        if s.exact_vs_nearest != std::cmp::Ordering::Equal {
            rescale_inexact += 1;
        }
        candidate.push(s.nearest);
    }
    let log_norm = block_log_norm_upper(a, h).unwrap_or(f64::INFINITY);
    let build = |status,
                 unbounded_reason,
                 scaled_certificate,
                 rescale_rounding,
                 normalization_rounding,
                 bound,
                 dominant,
                 caller_distance,
                 caller_bound,
                 work| NormalizedFusedPhiCertificate {
        schema: FUSED_PHI_NORMALIZED_SCHEMA,
        status,
        unbounded_reason,
        dimension: n,
        h,
        exponent: sigma.exponent,
        normalized_vectors: v.clone(),
        normalization_inexact,
        normalized_candidate: z.clone(),
        normalized_source: source,
        normalized_taylor_rejection: rejection.clone(),
        normalized: normalized.clone(),
        candidate: candidate.clone(),
        rescale_inexact,
        scaled_certificate,
        rescale_rounding,
        normalization_rounding,
        log_norm_upper: log_norm,
        bound,
        dominant,
        caller_candidate: caller_candidate.map(<[f64]>::to_vec),
        caller_distance,
        caller_bound,
        work,
    };
    if normalized.status() == FusedPhiStatus::Unbounded {
        let reason = format!(
            "the certificate of the normalized data is unbounded: {}",
            normalized
                .unbounded_reason()
                .unwrap_or("no reason recorded")
        );
        return Ok(build(
            FusedPhiStatus::Unbounded,
            Some(reason),
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            NormalizedFusedTerm::ScaledCertificate,
            None,
            None,
            work,
        ));
    }
    if overflow {
        return Err(range_refusal(
            "a rescaled candidate entry is above f64::MAX",
        ));
    }

    // 4. The bound.
    let refuse = |what: &'static str| move |_: CoreError| range_refusal(what);
    let scaled_certificate = scaled_up(normalized.bound(), sigma.exponent, &mut work)
        .map_err(refuse("sigma E is above f64::MAX"))?;
    let rescale_rounding = quantum_norm_up(rescale_inexact, -1075, &mut work)
        .map_err(refuse("R is above f64::MAX"))?;
    let normalization_rounding = if normalization_inexact.iter().all(|&c| c == 0) {
        0.0
    } else {
        let growth = if log_norm.is_finite() {
            exp_interval(log_norm.max(0.0))
                .map_err(refuse("e^max(mu, 0) is above f64::MAX"))?
                .hi
        } else {
            return Err(range_refusal("mu is above f64::MAX"));
        };
        let factorials = [1.0, 1.0, 2.0, 6.0, 24.0];
        let mut sum = 0.0;
        for k in 0..JOINT_PHI_TERMS {
            if normalization_inexact[k] > 0 {
                let term = mul_up(
                    div_up(growth, factorials[k])?,
                    sqrt_up(normalization_inexact[k] as f64)?,
                )?;
                sum = add_up(sum, term)?;
            }
        }
        scaled_up(sum, sigma.exponent.saturating_sub(1075), &mut work)
            .map_err(refuse("L is above f64::MAX"))?
    };
    let bound = add_up(scaled_certificate, rescale_rounding)
        .and_then(|s| add_up(s, normalization_rounding))
        .map_err(refuse("the bound is above f64::MAX"))?;
    let terms = [
        (NormalizedFusedTerm::ScaledCertificate, scaled_certificate),
        (NormalizedFusedTerm::RescaleRounding, rescale_rounding),
        (
            NormalizedFusedTerm::NormalizationRounding,
            normalization_rounding,
        ),
    ];
    let dominant = terms
        .iter()
        .fold(terms[0], |best, t| if t.1 > best.1 { *t } else { best })
        .0;

    // 5. The caller candidate.
    let (caller_distance, caller_bound) = match caller_candidate {
        None => (None, None),
        Some(u) => {
            let mut gaps = Vec::with_capacity(n);
            for (ui, fi) in u.iter().zip(&candidate) {
                let gap = Interval::point(*ui)?
                    .sub(Interval::point(*fi)?)
                    .map_err(refuse("u - F_hat is above f64::MAX"))?;
                gaps.push(gap.mag());
            }
            let distance = scaled_norm_up(&gaps, &mut work)
                .map_err(refuse("||u - F_hat|| is above f64::MAX"))?;
            let total =
                add_up(distance, bound).map_err(refuse("the caller bound is above f64::MAX"))?;
            (Some(distance), Some(total))
        }
    };
    Ok(build(
        FusedPhiStatus::Bounded,
        None,
        scaled_certificate,
        rescale_rounding,
        normalization_rounding,
        bound,
        dominant,
        caller_distance,
        caller_bound,
        work,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_case_matches_the_closed_form() {
        // n = 1, A = -1, h = 0.5 (power of two): F = e^{-1/2} w_0.
        let w = [vec![1.0], vec![0.0], vec![0.0], vec![0.0], vec![0.0]];
        let exact = (-0.5_f64).exp();
        let cert = certify_fused_phi_total(&[vec![-1.0]], 0.5, &w, &[exact]).unwrap();
        assert_eq!(cert.status(), FusedPhiStatus::Bounded);
        assert_eq!(cert.perturbation(), 0.0);
        assert!(cert.bound() < 1e-14, "{}", cert.bound());
        assert!(cert.bound() >= cert.stepped_error());
    }
}
