//! Exact simultaneous session-median interval (re-audit R4 of 2026-10-01,
//! R4-STAT-DEV-04), ported from
//! `research/adversarial_reaudit_20261001_r4/statistics/exact_session_interval.py`.
//!
//! Estimand ([`SESSION_CELL_MEDIAN_ESTIMAND`]): over a fixed declared set of
//! `C` cases, the median of each case's population median `m_c` of the
//! session-cell median log speedup `Z_sc`. Premises: the session vectors
//! `(Z_s1, .., Z_sC)` are independent and identically distributed (cases
//! may depend on each other), every session measures every case, the number
//! of sessions is fixed in advance, and no session is dropped because of
//! its outcome.
//!
//! Per case, `I_c = [Z_(k)c, Z_(S-k+1)c]` misses `m_c` with probability at
//! most `q(S, k) = 2 * 2^-S * sum_{j<k} binom(S, j)` (exact for a
//! continuous law, conservative with atoms). The largest `k` with `q(S, k)
//! <= alpha / C` makes all `C` intervals hold together with probability at
//! least `1 - C q` by the union bound, whatever the dependence between
//! cases, and since the median is monotone in each coordinate the interval
//! `[median_c Z_(k)c, median_c Z_(S-k+1)c]` covers the target with that
//! probability. When no `k >= 1` qualifies the interval is unbounded.
//! Binomial tails are exact integers; no sampling, no bootstrap.
//!
//! The decision is Promote iff `lower > ln(required)`, Block iff `upper <
//! ln(required)`, otherwise Inconclusive (ties are inconclusive).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{FairError, FairResult, PairedTimingDecision, SESSION_CELL_MEDIAN_ESTIMAND};

/// Unsigned big integer, little-endian base 2^32: enough for exact tails.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Natural(Vec<u32>);

impl Natural {
    fn from_u64(value: u64) -> Self {
        let mut n = Self(vec![value as u32, (value >> 32) as u32]);
        n.trim();
        n
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn mul_small(&self, factor: u64) -> Self {
        let mut out = Vec::with_capacity(self.0.len() + 2);
        let mut carry = 0_u128;
        for limb in &self.0 {
            let product = u128::from(*limb) * u128::from(factor) + carry;
            out.push(product as u32);
            carry = product >> 32;
        }
        while carry > 0 {
            out.push(carry as u32);
            carry >>= 32;
        }
        let mut n = Self(out);
        n.trim();
        n
    }

    fn add(&self, other: &Self) -> Self {
        let mut out = Vec::with_capacity(self.0.len().max(other.0.len()) + 1);
        let mut carry = 0_u64;
        for i in 0..self.0.len().max(other.0.len()) {
            let sum = u64::from(*self.0.get(i).unwrap_or(&0))
                + u64::from(*other.0.get(i).unwrap_or(&0))
                + carry;
            out.push(sum as u32);
            carry = sum >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        let mut n = Self(out);
        n.trim();
        n
    }

    fn pow2(exponent: usize) -> Self {
        let mut limbs = vec![0_u32; exponent / 32 + 1];
        limbs[exponent / 32] = 1 << (exponent % 32);
        Self(limbs)
    }

    /// `(quotient, remainder)` of division by a small divisor.
    fn div_small(&self, divisor: u32) -> (Self, u32) {
        let mut out = vec![0_u32; self.0.len()];
        let mut remainder = 0_u64;
        for i in (0..self.0.len()).rev() {
            let current = (remainder << 32) | u64::from(self.0[i]);
            out[i] = (current / u64::from(divisor)) as u32;
            remainder = current % u64::from(divisor);
        }
        let mut n = Self(out);
        n.trim();
        (n, remainder as u32)
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn to_decimal(&self) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let mut digits = Vec::new();
        let mut value = self.clone();
        while !value.is_zero() {
            let (quotient, remainder) = value.div_small(10);
            digits.push(char::from(b'0' + remainder as u8));
            value = quotient;
        }
        digits.iter().rev().collect()
    }
}

impl PartialOrd for Natural {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Natural {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// An exact nonnegative rational `numerator / denominator` as decimal
/// strings, reduced by powers of two (the denominators are powers of two).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactRatio {
    pub numerator: String,
    pub denominator: String,
}

impl ExactRatio {
    fn reduced(mut numerator: Natural, mut denominator: Natural) -> Self {
        while !numerator.is_zero()
            && numerator.0[0].is_multiple_of(2)
            && denominator
                .0
                .first()
                .is_some_and(|limb| limb.is_multiple_of(2))
        {
            numerator = numerator.div_small(2).0;
            denominator = denominator.div_small(2).0;
        }
        if numerator.is_zero() {
            denominator = Natural::from_u64(1);
        }
        Self {
            numerator: numerator.to_decimal(),
            denominator: denominator.to_decimal(),
        }
    }

    /// The value as binary64 for reports, to about 1e-16 relative for any
    /// size (the decimal strings may exceed the binary64 range on their own).
    pub fn as_f64(&self) -> f64 {
        let scaled = |digits: &str| -> (f64, i32) {
            let keep = digits.len().min(17);
            let mantissa = digits[..keep].parse::<f64>().unwrap_or(f64::NAN);
            (mantissa, (digits.len() - keep) as i32)
        };
        let (n, ne) = scaled(&self.numerator);
        let (d, de) = scaled(&self.denominator);
        n / d * 10.0_f64.powi(ne - de)
    }

    /// `"n/d"`, or `"n"` when `d = 1`.
    pub fn display(&self) -> String {
        if self.denominator == "1" {
            self.numerator.clone()
        } else {
            format!("{}/{}", self.numerator, self.denominator)
        }
    }
}

/// The order statistic of a design.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMedianDesign {
    pub sessions: usize,
    pub cases: usize,
    pub alpha: ExactRatio,
    /// `None`: no finite interval meets `alpha / C`.
    pub k: Option<usize>,
    /// `q(S, k)`, 0 when unbounded.
    pub per_case_failure: ExactRatio,
    /// `1 - C q(S, k)`, 1 when unbounded.
    pub simultaneous_coverage_lower: ExactRatio,
}

/// Largest supported session count (the integers stay small but the
/// design is a fixed-sample one).
pub const SESSION_MEDIAN_MAX_SESSIONS: usize = 4096;

/// The design for `sessions` sessions, `cases` cases and `alpha =
/// alpha_numerator / alpha_denominator`, with exact integer tails.
pub fn session_median_design(
    sessions: usize,
    cases: usize,
    alpha_numerator: u64,
    alpha_denominator: u64,
) -> FairResult<SessionMedianDesign> {
    if sessions == 0
        || sessions > SESSION_MEDIAN_MAX_SESSIONS
        || cases == 0
        || alpha_denominator == 0
        || alpha_numerator == 0
        || alpha_numerator >= alpha_denominator
    {
        return Err(FairError::Invalid(format!(
            "session-median design needs 1..={SESSION_MEDIAN_MAX_SESSIONS} sessions, at least one case and 0 < alpha < 1"
        )));
    }
    // binom(S, j) for j = 0..=S/2 + 1, exactly.
    let mut binomials = vec![Natural::from_u64(1)];
    for j in 1..=sessions.div_ceil(2) {
        let previous = binomials.last().expect("nonempty");
        let (quotient, remainder) = previous
            .mul_small((sessions + 1 - j) as u64)
            .div_small(j as u32);
        debug_assert_eq!(remainder, 0);
        binomials.push(quotient);
    }
    // q(S, k) <= alpha / C  <=>  2 C den sum_{j<k} binom <= num 2^S.
    let rhs = Natural::pow2(sessions).mul_small(alpha_numerator);
    let scale = 2 * cases as u64;
    let mut chosen = None;
    let mut sum = Natural(Vec::new());
    for k in 1..=sessions.div_ceil(2) {
        sum = sum.add(&binomials[k - 1]);
        let lhs = sum.mul_small(scale).mul_small(alpha_denominator);
        if lhs <= rhs {
            chosen = Some((k, sum.clone()));
        } else {
            break;
        }
    }
    let alpha = ExactRatio::reduced(
        Natural::from_u64(alpha_numerator),
        Natural::from_u64(alpha_denominator),
    );
    let denominator = Natural::pow2(sessions);
    Ok(match chosen {
        None => SessionMedianDesign {
            sessions,
            cases,
            alpha,
            k: None,
            per_case_failure: ExactRatio::reduced(Natural(Vec::new()), Natural::from_u64(1)),
            simultaneous_coverage_lower: ExactRatio::reduced(
                Natural::from_u64(1),
                Natural::from_u64(1),
            ),
        },
        Some((k, sum)) => {
            let twice = sum.mul_small(2);
            let total = twice.mul_small(cases as u64);
            // 1 - C q = (2^S - 2 C sum) / 2^S, nonnegative by the choice.
            let coverage = subtract(&denominator, &total);
            SessionMedianDesign {
                sessions,
                cases,
                alpha,
                k: Some(k),
                per_case_failure: ExactRatio::reduced(twice, denominator.clone()),
                simultaneous_coverage_lower: ExactRatio::reduced(coverage, denominator),
            }
        }
    })
}

fn subtract(a: &Natural, b: &Natural) -> Natural {
    let mut out = Vec::with_capacity(a.0.len());
    let mut borrow = 0_i64;
    for i in 0..a.0.len() {
        let mut value = i64::from(a.0[i]) - i64::from(*b.0.get(i).unwrap_or(&0)) - borrow;
        borrow = 0;
        if value < 0 {
            value += 1 << 32;
            borrow = 1;
        }
        out.push(value as u32);
    }
    let mut n = Natural(out);
    n.trim();
    n
}

/// One session's complete cell medians.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionCells {
    pub session: u32,
    /// Case id to the session-cell median log speedup.
    pub cells: BTreeMap<String, f64>,
    /// A session that failed is never an input.
    #[serde(default)]
    pub failed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionIntervalStatus {
    Finite,
    Unbounded,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMedianInterval {
    pub estimand: String,
    pub design: SessionMedianDesign,
    pub status: SessionIntervalStatus,
    /// Log-speedup bounds; `None` when unbounded.
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub decision: PairedTimingDecision,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        // A cell without pairs has no median; NaN is rejected as an input.
        return f64::NAN;
    }
    if n % 2 == 1 {
        values[n / 2]
    } else {
        0.5 * values[n / 2 - 1] + 0.5 * values[n / 2]
    }
}

/// The interval for a fixed design: `rows` must be exactly
/// `planned_sessions` distinct, unfailed sessions, each with exactly the
/// `declared_cases` (unique) and finite cells.
pub fn exact_session_median_interval(
    rows: &[SessionCells],
    declared_cases: &[String],
    planned_sessions: usize,
    alpha_numerator: u64,
    alpha_denominator: u64,
    required_speedup: f64,
) -> FairResult<SessionMedianInterval> {
    let reject = |why: String| Err(FairError::Invalid(format!("SESSION_MEDIAN_INPUT: {why}")));
    if rows.is_empty() || declared_cases.is_empty() {
        return reject("empty design".into());
    }
    if rows.len() != planned_sessions {
        return reject(format!(
            "{} sessions observed, {planned_sessions} planned: the stopping rule is fixed",
            rows.len()
        ));
    }
    if !(required_speedup.is_finite() && required_speedup > 0.0) {
        return reject("required speedup must be finite and positive".into());
    }
    let cases = declared_cases.iter().collect::<BTreeSet<_>>();
    if cases.len() != declared_cases.len() {
        return reject("duplicate case".into());
    }
    let mut sessions = BTreeSet::new();
    for row in rows {
        if !sessions.insert(row.session) {
            return reject(format!("duplicate session {}", row.session));
        }
        if row.failed {
            return reject(format!("failed session {}", row.session));
        }
        if row.cells.keys().collect::<BTreeSet<_>>() != cases {
            return reject(format!("missing or extra case in session {}", row.session));
        }
        if !row.cells.values().all(|value| value.is_finite()) {
            return reject(format!("nonfinite cell in session {}", row.session));
        }
    }
    let design = session_median_design(
        rows.len(),
        declared_cases.len(),
        alpha_numerator,
        alpha_denominator,
    )?;
    let Some(k) = design.k else {
        return Ok(SessionMedianInterval {
            estimand: SESSION_CELL_MEDIAN_ESTIMAND.into(),
            design,
            status: SessionIntervalStatus::Unbounded,
            lower: None,
            upper: None,
            decision: PairedTimingDecision::Inconclusive,
        });
    };
    let mut lows = Vec::with_capacity(declared_cases.len());
    let mut highs = Vec::with_capacity(declared_cases.len());
    for case in declared_cases {
        let mut z = rows.iter().map(|row| row.cells[case]).collect::<Vec<_>>();
        z.sort_by(f64::total_cmp);
        lows.push(z[k - 1]);
        highs.push(z[z.len() - k]);
    }
    let (lower, upper) = (median(&mut lows), median(&mut highs));
    let threshold = required_speedup.ln();
    let decision = if lower > threshold {
        PairedTimingDecision::Promote
    } else if upper < threshold {
        PairedTimingDecision::Block
    } else {
        PairedTimingDecision::Inconclusive
    };
    Ok(SessionMedianInterval {
        estimand: SESSION_CELL_MEDIAN_ESTIMAND.into(),
        design,
        status: SessionIntervalStatus::Finite,
        lower: Some(lower),
        upper: Some(upper),
        decision,
    })
}

/// The session-cell medians of a receipt's records: one row per session,
/// the median of each case's pair log speedups in that session.
pub fn session_cells_from_records(records: &[crate::SessionRecord]) -> Vec<SessionCells> {
    records
        .iter()
        .map(|record| SessionCells {
            session: record.provenance.session,
            cells: record
                .cases
                .iter()
                .map(|case| (case.case_id.clone(), median(&mut case.log_speedups())))
                .collect(),
            failed: !record.failures.is_empty(),
        })
        .collect()
}
