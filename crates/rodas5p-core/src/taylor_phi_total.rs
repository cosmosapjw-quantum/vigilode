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
//! [`NonnormalExpCertificate`]: crate::nonnormal_certificate::NonnormalExpCertificate

use serde::Serialize;

use crate::directed::{
    Interval, add_up, div_up, exp_interval, mul_down, mul_up, scale_pow2_is_exact,
    scale_pow2_nearest_counted, scale_pow2_up, sqrt_up, sub_up, sum_up,
};
use crate::matrix::DenseMatrix;
use crate::nonnormal_certificate::{
    AutoMetric, NonnormalBoundStatus, NonnormalExpCertificate, certify_exp_action,
    certify_exp_action_stepped, osborne_metric, stepping_rule,
};
use crate::polynomial_action::JOINT_PHI_TERMS;
use crate::taylor_phi::taylor_phi_action;
use crate::{CoreError, CoreResult};

pub const FUSED_PHI_TOTAL_SCHEMA: &str = "vigilode-fused-phi-total-v1";
pub const FUSED_PHI_RANGE_UNSUPPORTED: &str = "FUSED_PHI_RANGE_UNSUPPORTED";
pub const NORMALIZED_FUSED_PHI_SCHEMA: &str = "vigilode-normalized-fused-phi-total-v1";

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

/// A certified upper bound on `||candidate - sum_{k=0}^{4} phi_k(hA) w_k||_2`
/// produced by power-of-two normalization of the vectors `w_k` (RVJ DAG node
/// PY04, `research/py04_fused_normalization_20261011`).
///
/// # Type separation
///
/// [`NormalizedFusedPhiCertificate`] is a separate type with private fields and
/// no conversion from [`FusedPhiCertificate`]:
///
/// ```compile_fail
/// use rodas5p_core::taylor_phi_total::{FusedPhiCertificate, NormalizedFusedPhiCertificate};
/// fn needs_normalized(_: &NormalizedFusedPhiCertificate) {}
/// fn misuse(unnormalized: &FusedPhiCertificate) {
///     needs_normalized(unnormalized);
/// }
/// ```
///
/// And conversely:
///
/// ```compile_fail
/// use rodas5p_core::taylor_phi_total::{FusedPhiCertificate, NormalizedFusedPhiCertificate};
/// fn needs_unnormalized(_: &FusedPhiCertificate) {}
/// fn misuse(norm: &NormalizedFusedPhiCertificate) {
///     needs_unnormalized(norm);
/// }
/// ```
/// Speculative, candidate, fallback and verification work recorded across the
/// whole normalized certificate pipeline (RVJ DAG node PY04).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NormalizedFusedPhiWork {
    pub entries_scaled: usize,
    pub scaling_stages: usize,
    pub exactness_checks: usize,
    pub taylor_attempted: bool,
    pub taylor_accepted: bool,
    pub taylor_vector_products: u64,
    pub taylor_multiply_adds: u64,
    pub taylor_witness_entries: u64,
    pub fallback_probe_used: bool,
    pub fallback_probe_steps: usize,
    pub certificate_steps: usize,
    pub gershgorin_evaluations: usize,
    pub total_work_units: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NormalizedFusedPhiCertificate {
    schema: &'static str,
    status: FusedPhiStatus,
    dimension: usize,
    h: f64,
    exponent: i64,
    candidate: Vec<f64>,
    f_hat: Vec<f64>,
    bound: f64,
    f_hat_bound: f64,
    caller_distance: f64,
    normalized_bound: f64,
    rescaled_bound: f64,
    r_term: f64,
    l_term: f64,
    mu: Option<f64>,
    v: [Vec<f64>; JOINT_PHI_TERMS],
    z: Vec<f64>,
    stepped_error: f64,
    perturbation: f64,
    normalized_distance: f64,
    work: NormalizedFusedPhiWork,
    dominant: FusedPhiTerm,
    normalized_certificate: FusedPhiCertificate,
}

impl NormalizedFusedPhiCertificate {
    pub fn schema(&self) -> &'static str {
        self.schema
    }
    pub fn status(&self) -> FusedPhiStatus {
        self.status
    }
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    pub fn h(&self) -> f64 {
        self.h
    }
    pub fn exponent(&self) -> i64 {
        self.exponent
    }
    pub fn candidate(&self) -> &[f64] {
        &self.candidate
    }
    pub fn f_hat(&self) -> &[f64] {
        &self.f_hat
    }
    pub fn bound(&self) -> f64 {
        self.bound
    }
    pub fn f_hat_bound(&self) -> f64 {
        self.f_hat_bound
    }
    pub fn caller_distance(&self) -> f64 {
        self.caller_distance
    }
    pub fn normalized_bound(&self) -> f64 {
        self.normalized_bound
    }
    pub fn rescaled_bound(&self) -> f64 {
        self.rescaled_bound
    }
    pub fn r_term(&self) -> f64 {
        self.r_term
    }
    pub fn l_term(&self) -> f64 {
        self.l_term
    }
    pub fn mu(&self) -> Option<f64> {
        self.mu
    }
    pub fn v(&self) -> &[Vec<f64>; JOINT_PHI_TERMS] {
        &self.v
    }
    pub fn z(&self) -> &[f64] {
        &self.z
    }
    pub fn stepped_error(&self) -> f64 {
        self.stepped_error
    }
    pub fn perturbation(&self) -> f64 {
        self.perturbation
    }
    pub fn normalized_distance(&self) -> f64 {
        self.normalized_distance
    }
    pub fn work(&self) -> &NormalizedFusedPhiWork {
        &self.work
    }
    pub fn work_entries_scaled(&self) -> usize {
        self.work.entries_scaled
    }
    pub fn work_stages(&self) -> usize {
        self.work.scaling_stages
    }
    pub fn work_total(&self) -> u64 {
        self.work.total_work_units
    }
    pub fn fallback_probe_used(&self) -> bool {
        self.work.fallback_probe_used
    }
    pub fn taylor_accepted(&self) -> bool {
        self.work.taylor_accepted
    }
    pub fn dominant(&self) -> FusedPhiTerm {
        self.dominant
    }
    pub fn normalized_certificate(&self) -> &FusedPhiCertificate {
        &self.normalized_certificate
    }
}

/// Certifies `||candidate - sum_{k=0}^4 phi_k(hA) w_k||_2` using common
/// power-of-two normalization of the vectors `w_k`.
///
/// Any non-finite candidate or bound (`> f64::MAX` or `is_infinite()` / `is_nan()`)
/// returns `Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))`.
#[allow(clippy::needless_range_loop)]
pub fn certify_fused_phi_total_normalized(
    a: &[Vec<f64>],
    h: f64,
    w: &[Vec<f64>; JOINT_PHI_TERMS],
    caller_candidate: Option<&[f64]>,
) -> CoreResult<NormalizedFusedPhiCertificate> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "normalized fused phi: A must be a nonempty n x n matrix".into(),
        ));
    }
    if w.iter().any(|wk| wk.len() != n) {
        return Err(CoreError::Dimension(
            "normalized fused phi: every w_k must have length n".into(),
        ));
    }
    if caller_candidate.is_some_and(|cand| cand.len() != n) {
        return Err(CoreError::Dimension(
            "normalized fused phi: caller candidate must have length n".into(),
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

    // 1. Normalize: find binary exponent e of the largest |w_{k,i}|
    let mut largest = 0.0_f64;
    for wk in w {
        for &x in wk {
            let ax = x.abs();
            if ax > largest {
                largest = ax;
            }
        }
    }

    let e = if largest > 0.0 {
        let bits = largest.to_bits();
        let exp_bits = ((bits >> 52) & 0x7ff) as i64;
        if exp_bits > 0 {
            exp_bits - 1023
        } else {
            let mantissa = bits & 0x000f_ffff_ffff_ffff;
            let msb = 63 - mantissa.leading_zeros() as i64;
            -1074 + msb
        }
    } else {
        0
    };

    let mut v: [Vec<f64>; JOINT_PHI_TERMS] = std::array::from_fn(|_| Vec::with_capacity(n));
    let mut norm_stages = 0_u64;
    for (k, wk) in w.iter().enumerate() {
        for &x in wk {
            let (scaled_val, st) = scale_pow2_nearest_counted(x, -e);
            v[k].push(scaled_val);
            norm_stages = norm_stages.saturating_add(st);
        }
    }
    let norm_entries_scaled = 5 * n;

    // 2. Certify on v:
    let rows: Vec<&[f64]> = a.iter().map(Vec::as_slice).collect();
    let dense = DenseMatrix::from_rows(&rows)?;
    const TAYLOR_BUDGET: f64 = 1e-13;
    let taylor = taylor_phi_action(&dense, h, &v, TAYLOR_BUDGET);

    let taylor_attempted = true;
    let mut taylor_accepted = false;
    let mut taylor_vp = 0_u64;
    let mut taylor_fma = 0_u64;
    let mut taylor_witness = (n * n) as u64;
    let mut fallback_probe_used = false;
    let mut fallback_probe_steps = 0_usize;

    let z = match taylor {
        Ok(report) => {
            taylor_accepted = true;
            taylor_vp = report.work.augmented_vector_products;
            taylor_fma = report.work.multiply_adds;
            taylor_witness = report.work.witness_entries;
            report.fused
        }
        Err(_) => {
            fallback_probe_used = true;
            let probe = certify_fused_phi_total(a, h, &v, &vec![0.0; n])
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            for att in probe.attempts() {
                fallback_probe_steps =
                    fallback_probe_steps.saturating_add(att.steps().unwrap_or(0));
            }
            let top = probe.stepped_top();
            if !top.iter().all(|x| x.is_finite()) {
                return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
            }
            top.to_vec()
        }
    };

    let norm_cert = certify_fused_phi_total(a, h, &v, &z)
        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
    if norm_cert.status() != FusedPhiStatus::Bounded {
        return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
    }
    let mut certificate_steps = 0_usize;
    for att in norm_cert.attempts() {
        certificate_steps = certificate_steps.saturating_add(att.steps().unwrap_or(0));
    }

    let norm_bound = norm_cert.bound();
    let stepped_error = norm_cert.stepped_error();
    let perturbation = norm_cert.perturbation();
    let normalized_distance = norm_cert.candidate_distance();

    // 3. Rescale candidate:
    let mut f_hat = Vec::with_capacity(n);
    let mut rescale_stages = 0_u64;
    for &zi in &z {
        let (fhi, st) = scale_pow2_nearest_counted(zi, e);
        if !fhi.is_finite() {
            return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
        }
        f_hat.push(fhi);
        rescale_stages = rescale_stages.saturating_add(st);
    }
    let total_entries_scaled = norm_entries_scaled.saturating_add(n);
    let total_scaling_stages = (norm_stages.saturating_add(rescale_stages)) as usize;

    // 4. Bound:
    let rescaled_bound = match scale_pow2_up(norm_bound, e) {
        Ok(b) if b.is_finite() && b <= f64::MAX => b,
        _ => return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into())),
    };

    // R: ||fl(sigma z) - sigma z||_2
    let mut exactness_checks = 0_usize;
    let mut r_exact = true;
    for &zi in &z {
        exactness_checks = exactness_checks.saturating_add(1);
        if !scale_pow2_is_exact(zi, e) {
            r_exact = false;
        }
    }
    let r_term = if r_exact {
        0.0
    } else {
        mul_up(
            sqrt_up(n as f64)
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?,
            f64::from_bits(1),
        )
        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?
    };

    // L: sigma * sum_k (e^{max(mu, 0)} / k!) ||w_k / sigma - v_k||_2
    let mut w_exact = true;
    let mut inexact_per_k = [0_usize; 5];
    for (k, wk) in w.iter().enumerate() {
        for &x in wk {
            exactness_checks = exactness_checks.saturating_add(1);
            if !scale_pow2_is_exact(x, -e) {
                w_exact = false;
                inexact_per_k[k] = inexact_per_k[k].saturating_add(1);
            }
        }
    }

    let (l_term, mu, gershgorin_evaluations) = if w_exact {
        (0.0, None, 0_usize)
    } else {
        // Compute mu on hA:
        let mut ha = vec![
            vec![
                Interval::point(0.0).map_err(|_| CoreError::InvalidInput(
                    FUSED_PHI_RANGE_UNSUPPORTED.into()
                ))?;
                n
            ];
            n
        ];
        for i in 0..n {
            for j in 0..n {
                ha[i][j] = Interval::new(
                    mul_down(h, a[i][j])
                        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?,
                    mul_up(h, a[i][j])
                        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?,
                )
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            }
        }
        let half = Interval::point(0.5)
            .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
        let mut mu_val = f64::NEG_INFINITY;
        for i in 0..n {
            let mut off = 0.0;
            for j in 0..n {
                if j != i {
                    let sym_ij = ha[i][j]
                        .add(ha[j][i])
                        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?
                        .mul(half)
                        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
                    off = add_up(off, sym_ij.mag())
                        .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
                }
            }
            mu_val =
                mu_val
                    .max(add_up(ha[i][i].hi, off).map_err(|_| {
                        CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into())
                    })?);
        }
        let mu_max = mu_val.max(0.0);
        let growth = exp_interval(mu_max)
            .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?
            .hi;

        let mut terms = Vec::with_capacity(5);
        for (k, &c_k) in inexact_per_k.iter().enumerate() {
            if c_k == 0 {
                continue;
            }
            let norm_diff = mul_up(
                sqrt_up(c_k as f64)
                    .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?,
                f64::from_bits(1),
            )
            .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            let scaled_diff = scale_pow2_up(norm_diff, e)
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            if !scaled_diff.is_finite() || scaled_diff > f64::MAX {
                return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
            }
            let k_fact = match k {
                0 | 1 => 1.0,
                2 => 2.0,
                3 => 6.0,
                4 => 24.0,
                _ => 1.0,
            };
            let coeff = div_up(growth, k_fact)
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            let term = mul_up(coeff, scaled_diff)
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            terms.push(term);
        }
        let l_val = sum_up(terms)
            .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
        if !l_val.is_finite() || l_val > f64::MAX {
            return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
        }
        (l_val, Some(mu_val), n * n)
    };

    let f_hat_bound = add_up(
        add_up(rescaled_bound, r_term)
            .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?,
        l_term,
    )
    .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
    if !f_hat_bound.is_finite() || f_hat_bound > f64::MAX {
        return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
    }

    // 5. Caller candidate:
    let (candidate, caller_distance, bound) = match caller_candidate {
        Some(u) => {
            let dist = distance_upper(u, &f_hat);
            let b = add_up(dist, f_hat_bound)
                .map_err(|_| CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()))?;
            if !b.is_finite() || b > f64::MAX {
                return Err(CoreError::InvalidInput(FUSED_PHI_RANGE_UNSUPPORTED.into()));
            }
            (u.to_vec(), dist, b)
        }
        None => (f_hat.clone(), 0.0, f_hat_bound),
    };

    let dominant = if caller_distance > f_hat_bound {
        FusedPhiTerm::CandidateDistance
    } else {
        norm_cert.dominant()
    };

    let total_work_units = (total_scaling_stages as u64)
        .saturating_add(exactness_checks as u64)
        .saturating_add(taylor_vp)
        .saturating_add(taylor_witness)
        .saturating_add((fallback_probe_steps as u64).saturating_mul(20))
        .saturating_add((certificate_steps as u64).saturating_mul(20))
        .saturating_add(gershgorin_evaluations as u64);

    let work = NormalizedFusedPhiWork {
        entries_scaled: total_entries_scaled,
        scaling_stages: total_scaling_stages,
        exactness_checks,
        taylor_attempted,
        taylor_accepted,
        taylor_vector_products: taylor_vp,
        taylor_multiply_adds: taylor_fma,
        taylor_witness_entries: taylor_witness,
        fallback_probe_used,
        fallback_probe_steps,
        certificate_steps,
        gershgorin_evaluations,
        total_work_units,
    };

    Ok(NormalizedFusedPhiCertificate {
        schema: NORMALIZED_FUSED_PHI_SCHEMA,
        status: FusedPhiStatus::Bounded,
        dimension: n,
        h,
        exponent: e,
        candidate,
        f_hat,
        bound,
        f_hat_bound,
        caller_distance,
        normalized_bound: norm_bound,
        rescaled_bound,
        r_term,
        l_term,
        mu,
        v,
        z,
        stepped_error,
        perturbation,
        normalized_distance,
        work,
        dominant,
        normalized_certificate: norm_cert,
    })
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
