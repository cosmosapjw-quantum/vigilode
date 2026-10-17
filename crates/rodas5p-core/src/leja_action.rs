//! Newton-Leja phi actions with an estimate-only total (RVJ DAG node PP10,
//! `research/pp10_leja_candidate_20261004`). Opt-in research candidate: no
//! integrator, router or default dispatch calls it, and no result of it is
//! a certificate.
//!
//! The action is `F = sum_{k=0}^{4} phi_k(h A) w_k` for given vectors `w_k`
//! (`phi_0 = exp`), the same target as
//! [`crate::polynomial_action::joint_phi_action`], on a
//! [`SymmetricNonpositiveOperator`] whose enclosure is Gershgorin-verified.
//!
//! # Interval and points
//!
//! With `spec(A) in [-rho, -lambda]` the spectrum of `h A` lies in `[a, b]`,
//! `a = -(h rho)` rounded down and `b = -(h lambda)` rounded up. With
//! `c = (a + b) / 2` and `gamma = (b - a) / 4` the nodes are `x_j = c +
//! gamma xi_j`, where `xi_j` are real Leja points of `[-2, 2]` (capacity 1)
//! starting at `xi_0 = 2`. They are discrete Leja points: each `xi_j`
//! maximizes `prod_{i<j} |xi - xi_i|` (in logarithms) over the cosine grid
//! `2 cos(pi i / 2^15)`, `i = 0..=2^15`, first maximum on ties.
//!
//! # Divided differences
//!
//! The Newton form uses the scaled divided differences `d_{j,k} = gamma^j
//! phi_k[x_0, ..., x_j]` (the divided differences of `xi -> phi_k(c + gamma
//! xi)`), so that `p_k(h A) w = sum_j d_{j,k} r_j` with `r_0 = w` and
//! `r_{j+1} = ((h A - c I) / gamma - xi_j I) r_j`.
//!
//! They are computed as one matrix function (Opitz): for nodes `y_0 .. y_N`
//! and the lower bidiagonal `Z` with diagonal `y_i` and subdiagonal
//! `beta_i != 0`, `exp(Z)_{i,j} = (prod_{l=j}^{i-1} beta_l) exp[y_j, ...,
//! y_i]`. Since `phi_k(z) = exp[0, ..., 0, z]` (`k` zeros), `phi_k[x_0, ...,
//! x_j] = exp[0^k, x_0, ..., x_j]`, so with the nodes `(0, 0, 0, 0, x_0, ...,
//! x_m)`, `beta = (1, 1, 1, 1, gamma, gamma, ...)`, column `4 - k` of
//! `exp(Z)` holds `d_{j,k}` in row `4 + j`. The matrix is shifted by `s = c -
//! 2 gamma` (about `a`): `exp(Z) = e^s exp(Z - s I)`, and `Z - s I` has a
//! nonnegative diagonal (`gamma (xi_j + 2)` and `2 gamma - c`) and a positive
//! subdiagonal. Every Taylor term of `exp(Z - s I) e_col` is therefore a
//! nonnegative vector, `v_n = (Z - s I) v_{n-1} / n`, and the series has no
//! cancellation: each entry is accurate to a relative `O(n u)` for `n` terms
//! (no scaling and squaring, so tiny divided differences do not underflow
//! through a scaled intermediate). The terms are summed with Neumaier's
//! compensated summation, all in binary64; no extended precision. The series
//! stops once `n` exceeds the dimension and twice `max diag + max beta` and
//! every new term is below `2^-60` of its partial sum. This choice of the
//! standard matrix-function route (rather than the plain recurrence, which
//! loses all accuracy for close nodes) is documented here; it is not an
//! enclosure.
//!
//! # Stopping and status
//!
//! After the term `j` has been added (`j >= 1`), the estimate is
//! `sum_k |s_k| (|d_{j-1,k}| ||r_{j-1}|| + |d_{j,k}| ||r_j||)` (`s_k` the
//! same-vector scales, 1 otherwise), the size of the last two Newton terms
//! of the fused sum. The recurrence stops when the estimate is at most the
//! requested absolute tolerance or the degree reaches the cap
//! ([`LEJA_DEGREE_CAP`]); then [`LejaReport::converged`] is false. The total
//! error is **always** [`TotalErrorStatus::EstimateOnly`] with the reason
//! [`LEJA_TOTAL_NOT_CERTIFIED`]: the estimate is a heuristic, and neither
//! the truncation nor the rounding is bounded. [`LejaReport`] is a separate
//! type, so it cannot be passed to
//! [`crate::polynomial_action::JointPhiReport::admit_total_error`],
//! [`crate::polynomial_action::JointPhiReport::admit_laguerre_total`] or
//! [`crate::polynomial_action::route_joint_phi`], and its status would be
//! rejected by them.
//!
//! # Work
//!
//! Every step applies the operator once per block column through
//! [`LinearOperator::apply_rows`] and is charged like
//! [`crate::polynomial_action::joint_phi_action`]: one
//! `poly_block_products` and the block width in `poly_vector_products`. One
//! divided-difference table is one `poly_coefficient_setups` (also in the
//! scalar branch). No other counter changes.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Serialize;

use crate::{
    CoreError, CoreResult, DenseOperator, LinearOperator, WorkCounters,
    directed::{
        Interval, add_down, add_up, div_down, div_up, exp_interval, mul_down, mul_up, sub_down,
        sub_up,
    },
    polynomial_action::{
        EnclosureEvidence, JOINT_PHI_TERMS, JointPhiInput, SpectralEnclosure,
        SymmetricNonpositiveOperator, TotalErrorStatus,
    },
    safe_l2,
};

pub const LEJA_SCHEMA: &str = "vigilode-leja-phi-action-v1";
/// The reason of every Leja total: an estimate, never a certificate.
pub const LEJA_TOTAL_NOT_CERTIFIED: &str = "LEJA_TOTAL_NOT_CERTIFIED";
/// Prefix of every input or domain error of this module.
pub const LEJA_INPUT_UNSUPPORTED: &str = "LEJA_INPUT_UNSUPPORTED";
/// The largest Newton degree (operator products per block column).
pub const LEJA_DEGREE_CAP: usize = 128;
/// Largest `h rho` for which the shifted divided-difference series stays in
/// the binary64 range.
pub const LEJA_RANGE_LIMIT: f64 = 600.0;
/// Number of intervals of the cosine grid the discrete Leja points are
/// chosen from.
pub const LEJA_GRID_INTERVALS: usize = 1 << 15;
const MAX_TAYLOR_TERMS: usize = 100_000;
const TAYLOR_RELATIVE_TAIL: f64 = 1.0 / (1u64 << 60) as f64;
/// Confluent zero nodes in front of the Leja nodes (`phi_1 .. phi_4`).
const ZERO_NODES: usize = JOINT_PHI_TERMS - 1;

fn unsupported(reason: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("{LEJA_INPUT_UNSUPPORTED}: {reason}"))
}

/// The first [`LEJA_DEGREE_CAP`]` + 1` discrete Leja points of `[-2, 2]`.
pub fn leja_points() -> &'static [f64] {
    static POINTS: OnceLock<Vec<f64>> = OnceLock::new();
    POINTS.get_or_init(|| {
        let mut grid: Vec<f64> = (0..=LEJA_GRID_INTERVALS)
            .map(|i| 2.0 * (std::f64::consts::PI * i as f64 / LEJA_GRID_INTERVALS as f64).cos())
            .collect();
        grid[0] = 2.0;
        grid[LEJA_GRID_INTERVALS] = -2.0;
        let mut log_product = vec![0.0_f64; grid.len()];
        let mut taken = vec![false; grid.len()];
        let mut points = Vec::with_capacity(LEJA_DEGREE_CAP + 1);
        let mut next = 0;
        for _ in 0..=LEJA_DEGREE_CAP {
            let xi = grid[next];
            points.push(xi);
            taken[next] = true;
            let mut best = f64::NEG_INFINITY;
            for (i, g) in grid.iter().enumerate() {
                if taken[i] {
                    continue;
                }
                log_product[i] += (g - xi).abs().ln();
                if log_product[i] > best {
                    best = log_product[i];
                    next = i;
                }
            }
        }
        points
    })
}

/// Columns `0 ..= ZERO_NODES` of `exp(L)` for the lower bidiagonal `L` with
/// nonnegative `diagonal` and positive `subdiagonal`, by the vector Taylor
/// series with compensated summation (every term is nonnegative).
fn exp_bidiagonal_columns(diagonal: &[f64], subdiagonal: &[f64]) -> CoreResult<Vec<Vec<f64>>> {
    let size = diagonal.len();
    debug_assert_eq!(subdiagonal.len() + 1, size);
    if diagonal
        .iter()
        .chain(subdiagonal)
        .any(|x| !(x.is_finite() && *x >= 0.0))
    {
        return Err(unsupported(
            "divided-difference matrix needs a finite nonnegative diagonal and subdiagonal",
        ));
    }
    let max_entry = diagonal.iter().fold(0.0_f64, |m, x| m.max(*x))
        + subdiagonal.iter().fold(0.0_f64, |m, x| m.max(*x));
    let min_terms = (size as f64).max(2.0 * max_entry);
    let mut columns = Vec::with_capacity(ZERO_NODES + 1);
    for col in 0..=ZERO_NODES.min(size - 1) {
        let mut term = vec![0.0; size];
        term[col] = 1.0;
        let mut sum = term.clone();
        let mut compensation = vec![0.0; size];
        let mut converged = false;
        for n in 1..=MAX_TAYLOR_TERMS {
            let inverse = n as f64;
            for i in (col..size).rev() {
                let below = if i > col {
                    subdiagonal[i - 1] * term[i - 1]
                } else {
                    0.0
                };
                term[i] = (diagonal[i] * term[i] + below) / inverse;
            }
            let mut small = true;
            for i in col..size {
                // Neumaier: the error of `sum + term`, kept apart.
                let t = sum[i] + term[i];
                compensation[i] += if sum[i].abs() >= term[i].abs() {
                    (sum[i] - t) + term[i]
                } else {
                    (term[i] - t) + sum[i]
                };
                sum[i] = t;
                if term[i] > TAYLOR_RELATIVE_TAIL * sum[i] {
                    small = false;
                }
            }
            if !sum.iter().all(|x| x.is_finite()) {
                return Err(CoreError::NonFinite(
                    "Leja divided-difference series left the binary64 range".into(),
                ));
            }
            if small && n as f64 > min_terms {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(unsupported(
                "Leja divided-difference series did not converge",
            ));
        }
        columns.push(sum.iter().zip(&compensation).map(|(s, c)| s + c).collect());
    }
    Ok(columns)
}

/// Scaled Newton divided differences `d[j][k] = gamma^j phi_k[x_0..x_j]` for
/// the nodes `x_j = c + gamma xi_j`, `j = 0 .. xi.len()`. With `gamma = 0`
/// only `xi.len() == 1` is meaningful: `d[0][k] = phi_k(c)`.
pub fn leja_divided_differences(
    c: f64,
    gamma: f64,
    xi: &[f64],
) -> CoreResult<Vec<[f64; JOINT_PHI_TERMS]>> {
    if xi.is_empty() || !(c.is_finite() && gamma.is_finite() && c <= 0.0 && gamma >= 0.0) {
        return Err(unsupported(
            "divided differences need nodes, a finite center c <= 0 and a scale gamma >= 0",
        ));
    }
    if gamma == 0.0 && xi.len() > 1 {
        return Err(unsupported("a zero scale admits a single node"));
    }
    if xi.iter().any(|x| !(-2.0..=2.0).contains(x)) {
        return Err(unsupported("Leja points must lie in [-2, 2]"));
    }
    // Shift s = c - 2 gamma (about the left end a): Z - s I >= 0.
    let shift = c - 2.0 * gamma;
    if shift < -LEJA_RANGE_LIMIT - 1.0 {
        return Err(unsupported(format!(
            "interval left end {shift:e} beyond -{LEJA_RANGE_LIMIT}"
        )));
    }
    let zero_node = -shift;
    let mut diagonal = vec![zero_node; ZERO_NODES];
    diagonal.extend(xi.iter().map(|x| gamma * (x + 2.0)));
    let mut subdiagonal = vec![1.0; ZERO_NODES];
    subdiagonal.extend(std::iter::repeat_n(gamma, xi.len() - 1));
    let factor = shift.exp();
    let columns = exp_bidiagonal_columns(&diagonal, &subdiagonal)?;
    Ok((0..xi.len())
        .map(|j| std::array::from_fn(|k| factor * columns[ZERO_NODES - k][ZERO_NODES + j]))
        .collect())
}

/// A Newton-Leja phi action. The total error is always
/// [`TotalErrorStatus::EstimateOnly`] ([`Self::total_error`]); the type is
/// not a [`crate::polynomial_action::JointPhiReport`] and no certified API
/// accepts it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LejaReport {
    pub schema: String,
    /// `distinct` or `same-vector`.
    pub input: String,
    /// `newton`, `scalar` (`h = 0`, `A = 0` or `A = -lambda I`: one node,
    /// exact interpolation) or `zero-input`.
    pub branch: String,
    pub dimension: usize,
    /// Newton degree: operator products per block column.
    pub degree: usize,
    pub degree_cap: usize,
    /// Whether the estimate met the tolerance before the cap.
    pub converged: bool,
    /// `[a, b]` enclosing the spectrum of `h A` (outward rounded).
    pub interval: [f64; 2],
    pub center: f64,
    pub scale: f64,
    pub tolerance: f64,
    /// The last-two-terms estimate of the fused sum (0 in the scalar and
    /// zero-input branches); a heuristic, not a bound.
    pub estimate: f64,
    /// The same estimate per term `k`.
    pub column_estimates: Vec<f64>,
    /// `p_k(hA) w_k` for `k = 0..4`.
    pub columns: Vec<Vec<f64>>,
    pub fused: Vec<f64>,
    total_error: TotalErrorStatus,
    pub evidence: EnclosureEvidence,
    pub vector_products: u64,
    pub block_products: u64,
    pub coefficient_setups: u64,
}

impl LejaReport {
    /// Always [`TotalErrorStatus::EstimateOnly`] with reason
    /// [`LEJA_TOTAL_NOT_CERTIFIED`] and no bounded component.
    pub fn total_error(&self) -> &TotalErrorStatus {
        &self.total_error
    }
}

fn estimate_only() -> TotalErrorStatus {
    TotalErrorStatus::EstimateOnly {
        reason: format!(
            "{LEJA_TOTAL_NOT_CERTIFIED}: Newton-Leja last-two-terms estimate only; truncation and rounding are not bounded"
        ),
        bounded_components: 0.0,
    }
}

/// [`leja_phi_action_capped`] with the cap [`LEJA_DEGREE_CAP`].
pub fn leja_phi_action(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    tolerance: f64,
    work: &mut WorkCounters,
) -> CoreResult<LejaReport> {
    leja_phi_action_capped(op, h, input, tolerance, LEJA_DEGREE_CAP, work)
}

/// `sum_k phi_k(h A) w_k` by the Newton-Leja recurrence with the estimate
/// tolerance `tolerance` (absolute, fused 2-norm) and a degree cap in `1
/// ..= LEJA_DEGREE_CAP`. Inputs are checked before any work is charged.
pub fn leja_phi_action_capped(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    tolerance: f64,
    degree_cap: usize,
    work: &mut WorkCounters,
) -> CoreResult<LejaReport> {
    let n = op.dimension();
    if !(h.is_finite() && h >= 0.0) {
        return Err(unsupported(format!(
            "step h = {h:e} must be finite and >= 0"
        )));
    }
    if !(tolerance.is_finite() && tolerance > 0.0) {
        return Err(unsupported(format!(
            "tolerance {tolerance:e} must be finite and positive"
        )));
    }
    if !(1..=LEJA_DEGREE_CAP).contains(&degree_cap) {
        return Err(unsupported(format!(
            "degree cap {degree_cap} outside 1..={LEJA_DEGREE_CAP}"
        )));
    }
    let SpectralEnclosure {
        lambda,
        rho,
        evidence,
    } = op.enclosure().clone();
    if !matches!(evidence, EnclosureEvidence::Gershgorin) {
        return Err(unsupported(
            "the spectral enclosure is not Gershgorin-verified",
        ));
    }
    let (block, scales, input_name): (Vec<Vec<f64>>, [f64; JOINT_PHI_TERMS], &str) = match input {
        JointPhiInput::Distinct(vectors) => {
            if vectors.iter().any(|w| w.len() != n) {
                return Err(unsupported("every w_k must have the operator dimension"));
            }
            (vectors.to_vec(), [1.0; JOINT_PHI_TERMS], "distinct")
        }
        JointPhiInput::SameVector { vector, scales } => {
            if vector.len() != n {
                return Err(unsupported("vector must have the operator dimension"));
            }
            (vec![vector.to_vec()], scales, "same-vector")
        }
    };
    if !block.iter().flatten().chain(&scales).all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite("Leja input contains NaN/Inf".into()));
    }
    let a = -mul_up(h, rho)?;
    let b = -mul_down(h, lambda)?;
    if -a > LEJA_RANGE_LIMIT {
        return Err(unsupported(format!(
            "h rho = {:e} above {LEJA_RANGE_LIMIT}",
            -a
        )));
    }
    let width = block.len();
    let column_of = |k: usize| if width == 1 { 0 } else { k };
    let zero_input = (0..JOINT_PHI_TERMS)
        .all(|k| scales[k] == 0.0 || block[column_of(k)].iter().all(|x| *x == 0.0));
    let scalar = h == 0.0 || rho == 0.0 || lambda == rho;

    let mut sums = vec![vec![0.0; n]; JOINT_PHI_TERMS];
    let mut degree = 0;
    let mut converged = true;
    let mut estimate = 0.0;
    let mut column_estimates = vec![0.0; JOINT_PHI_TERMS];
    let mut vector_products = 0;
    let mut block_products = 0;
    let mut coefficient_setups = 0;
    let (center, scale, branch);
    if zero_input {
        center = 0.5 * (a + b);
        scale = 0.25 * (b - a);
        branch = "zero-input";
    } else if scalar {
        // One node -h lambda: p_k is the constant phi_k(-h lambda), exact
        // on A = -lambda I.
        center = if h == 0.0 || rho == 0.0 { 0.0 } else { b };
        scale = 0.0;
        branch = "scalar";
        let d = leja_divided_differences(center, 0.0, &[2.0])?;
        coefficient_setups += 1;
        for (k, sum) in sums.iter_mut().enumerate() {
            for (out, w) in sum.iter_mut().zip(&block[column_of(k)]) {
                *out = d[0][k] * w;
            }
        }
    } else {
        center = 0.5 * (a + b);
        scale = 0.25 * (b - a);
        branch = "newton";
        let xi = &leja_points()[..=degree_cap];
        let d = leja_divided_differences(center, scale, xi)?;
        coefficient_setups += 1;
        let dense = DenseOperator::new(op.matrix().clone())?;
        let mut current = block.clone();
        let mut product = vec![vec![0.0; n]; width];
        let accumulate = |j: usize, vectors: &[Vec<f64>], sums: &mut [Vec<f64>]| {
            let norms: Vec<f64> = vectors.iter().map(|v| crate::safe_l2(v)).collect();
            let mut terms = [0.0; JOINT_PHI_TERMS];
            for (k, sum) in sums.iter_mut().enumerate() {
                let column = column_of(k);
                let coefficient = d[j][k];
                for (out, x) in sum.iter_mut().zip(&vectors[column]) {
                    *out += coefficient * x;
                }
                terms[k] = scales[k].abs() * coefficient.abs() * norms[column];
            }
            terms
        };
        let mut previous_terms = accumulate(0, &current, &mut sums);
        converged = false;
        for j in 1..=degree_cap {
            dense.apply_rows(&current, &mut product)?;
            block_products += 1;
            vector_products += width as u64;
            let node = xi[j - 1];
            for (r, y) in current.iter_mut().zip(&product) {
                for (ri, yi) in r.iter_mut().zip(y) {
                    *ri = (h * yi - center * *ri) / scale - node * *ri;
                }
            }
            if !current.iter().flatten().all(|x| x.is_finite()) {
                work.poly_block_products += block_products;
                work.poly_vector_products += vector_products;
                work.poly_coefficient_setups += coefficient_setups;
                return Err(CoreError::NonFinite(
                    "Newton-Leja recurrence produced NaN/Inf".into(),
                ));
            }
            let terms = accumulate(j, &current, &mut sums);
            degree = j;
            for k in 0..JOINT_PHI_TERMS {
                column_estimates[k] = previous_terms[k] + terms[k];
            }
            estimate = column_estimates.iter().sum();
            if estimate <= tolerance {
                converged = true;
                break;
            }
            previous_terms = terms;
        }
    }
    work.poly_block_products += block_products;
    work.poly_vector_products += vector_products;
    work.poly_coefficient_setups += coefficient_setups;

    let columns: Vec<Vec<f64>> = sums
        .into_iter()
        .enumerate()
        .map(|(k, sum)| sum.into_iter().map(|x| scales[k] * x).collect())
        .collect();
    let fused: Vec<f64> = (0..n)
        .map(|i| columns.iter().fold(0.0, |acc, column| acc + column[i]))
        .collect();
    if !fused.iter().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite(
            "Newton-Leja action produced NaN/Inf".into(),
        ));
    }
    Ok(LejaReport {
        schema: LEJA_SCHEMA.into(),
        input: input_name.into(),
        branch: branch.into(),
        dimension: n,
        degree,
        degree_cap,
        converged,
        interval: [a, b],
        center,
        scale,
        tolerance,
        estimate,
        column_estimates,
        columns,
        fused,
        total_error: estimate_only(),
        evidence,
        vector_products,
        block_products,
        coefficient_setups,
    })
}

// -----------------------------------------------------------------------------
// Certified Leja Phi Action (RVJ DAG node PY05)
// -----------------------------------------------------------------------------

pub const LEJA_CERTIFIED_SCHEMA: &str = "vigilode-leja-certified-v1";

/// The precomputed node-polynomial bounds `M_{m+1} >= max_{xi in [-2, 2]} prod_{j=0}^m |xi - xi_j|`
/// for all degrees `m = 0 ..= LEJA_DEGREE_CAP`.
pub fn leja_node_bounds() -> &'static [f64] {
    static BOUNDS: OnceLock<Vec<f64>> = OnceLock::new();
    BOUNDS.get_or_init(|| {
        let xi = leja_points();
        (0..=LEJA_DEGREE_CAP)
            .map(|m| leja_node_polynomial_bound(m, xi))
            .collect()
    })
}

/// Computes an outward interval upper bound on `max_{xi in [-2, 2]} prod_{j=0}^m |xi - xi_j|`
/// by evaluating over uniform subintervals of `[-2, 2]` using directed interval arithmetic.
///
/// Guaranteed zero heap allocations on the evaluation path.
pub fn leja_node_polynomial_bound(m: usize, xi: &[f64]) -> f64 {
    if m >= xi.len() {
        return f64::INFINITY;
    }
    const SUBDIVISIONS: usize = 128;
    const H_SUB: f64 = 4.0 / (SUBDIVISIONS as f64);
    let mut max_val = 0.0_f64;

    for p in 0..SUBDIVISIONS {
        let lo = -2.0 + (p as f64) * H_SUB;
        let hi = -2.0 + ((p + 1) as f64) * H_SUB;
        let int_p = match Interval::new(lo, hi) {
            Ok(int) => int,
            Err(_) => return f64::INFINITY,
        };

        let mut acc = match Interval::point(1.0) {
            Ok(p) => p,
            Err(_) => return f64::INFINITY,
        };
        for &node in &xi[..=m] {
            let diff_lo = match sub_down(int_p.lo, node) {
                Ok(v) => v,
                Err(_) => return f64::INFINITY,
            };
            let diff_hi = match sub_up(int_p.hi, node) {
                Ok(v) => v,
                Err(_) => return f64::INFINITY,
            };
            let diff = match Interval::new(diff_lo, diff_hi) {
                Ok(int) => int,
                Err(_) => return f64::INFINITY,
            };
            acc = match acc.mul(diff) {
                Ok(res) => res,
                Err(_) => return f64::INFINITY,
            };
        }
        let mag = acc.mag();
        if !mag.is_finite() {
            return f64::INFINITY;
        }
        if mag > max_val {
            max_val = mag;
        }
    }
    max_val
}

/// Upper bound on a sum of nonnegative terms, slice-based and zero-allocation.
#[inline(always)]
fn sum_up_slice(values: &[f64]) -> CoreResult<f64> {
    values.iter().copied().try_fold(0.0, add_up)
}

/// Certified Taylor series for lower bidiagonal matrix exponential columns,
/// returning lower and upper bounds componentwise: `(columns_lo, columns_hi)`.
#[allow(clippy::type_complexity, clippy::needless_range_loop)]
fn exp_bidiagonal_columns_certified(
    diagonal_lo: &[f64],
    diagonal_hi: &[f64],
    subdiagonal_lo: &[f64],
    subdiagonal_hi: &[f64],
) -> CoreResult<(Vec<Vec<f64>>, Vec<Vec<f64>>)> {
    let size = diagonal_lo.len();
    if diagonal_hi.len() != size
        || subdiagonal_lo.len() + 1 != size
        || subdiagonal_hi.len() + 1 != size
    {
        return Err(CoreError::Dimension(
            "bidiagonal matrix component length mismatch".into(),
        ));
    }
    if diagonal_lo
        .iter()
        .chain(diagonal_hi)
        .chain(subdiagonal_lo)
        .chain(subdiagonal_hi)
        .any(|x| !(x.is_finite() && *x >= 0.0))
    {
        return Err(unsupported(
            "divided-difference matrix needs a finite nonnegative diagonal and subdiagonal",
        ));
    }
    let mut max_entry = diagonal_hi[0];
    for i in 1..size {
        let row_sum = add_up(diagonal_hi[i], subdiagonal_hi[i - 1])?;
        if row_sum > max_entry {
            max_entry = row_sum;
        }
    }
    let min_terms = (size as f64).max(mul_up(2.0, max_entry)?);
    let mut columns_lo = Vec::with_capacity(ZERO_NODES + 1);
    let mut columns_hi = Vec::with_capacity(ZERO_NODES + 1);

    for col in 0..=ZERO_NODES.min(size - 1) {
        let mut term_lo = vec![0.0; size];
        let mut term_hi = vec![0.0; size];
        term_lo[col] = 1.0;
        term_hi[col] = 1.0;
        let mut sum_lo = term_lo.clone();
        let mut sum_hi = term_hi.clone();
        let mut converged = false;

        for n in 1..=MAX_TAYLOR_TERMS {
            let inv_lo = div_down(1.0, n as f64)?;
            let inv_hi = div_up(1.0, n as f64)?;
            for i in (col..size).rev() {
                let below_lo = if i > col {
                    mul_down(subdiagonal_lo[i - 1], term_lo[i - 1])?
                } else {
                    0.0
                };
                let below_hi = if i > col {
                    mul_up(subdiagonal_hi[i - 1], term_hi[i - 1])?
                } else {
                    0.0
                };
                term_lo[i] = mul_down(
                    add_down(mul_down(diagonal_lo[i], term_lo[i])?, below_lo)?,
                    inv_lo,
                )?;
                term_hi[i] = mul_up(
                    add_up(mul_up(diagonal_hi[i], term_hi[i])?, below_hi)?,
                    inv_hi,
                )?;
            }
            let mut small = true;
            for i in col..size {
                sum_lo[i] = add_down(sum_lo[i], term_lo[i])?;
                sum_hi[i] = add_up(sum_hi[i], term_hi[i])?;
                if term_hi[i] > mul_up(TAYLOR_RELATIVE_TAIL, sum_hi[i])? {
                    small = false;
                }
            }
            if !sum_hi.iter().all(|x| x.is_finite()) {
                return Err(CoreError::NonFinite(
                    "Leja divided-difference series left the binary64 range".into(),
                ));
            }
            if small && n as f64 > min_terms {
                let ratio = div_up(max_entry, (n + 1) as f64)?;
                if ratio < 1.0 {
                    let geom = div_up(ratio, sub_down(1.0, ratio)?)?;
                    let t_inf = term_hi[col..size].iter().copied().fold(0.0_f64, f64::max);
                    let tail = mul_up(t_inf, geom)?;
                    for i in col..size {
                        sum_hi[i] = add_up(sum_hi[i], tail)?;
                    }
                    converged = true;
                    break;
                }
            }
        }
        if !converged {
            return Err(unsupported(
                "Leja divided-difference series did not converge",
            ));
        }
        columns_lo.push(sum_lo);
        columns_hi.push(sum_hi);
    }
    Ok((columns_lo, columns_hi))
}

/// Certified scaled Newton divided differences `(d, d_rad)`:
/// `d[j][k] = gamma^j phi_k[x_0..x_j]` with outward radius `d_rad[j][k]`
/// enclosing the exact divided difference: `|d_exact - d| <= d_rad`.
#[allow(clippy::type_complexity)]
pub fn leja_divided_differences_certified(
    c: f64,
    gamma: f64,
    xi: &[f64],
) -> CoreResult<(Vec<[f64; JOINT_PHI_TERMS]>, Vec<[f64; JOINT_PHI_TERMS]>)> {
    if xi.is_empty() || !(c.is_finite() && gamma.is_finite() && c <= 0.0 && gamma >= 0.0) {
        return Err(unsupported(
            "divided differences need nodes, a finite center c <= 0 and a scale gamma >= 0",
        ));
    }
    if gamma == 0.0 && xi.len() > 1 {
        return Err(unsupported("a zero scale admits a single node"));
    }
    if xi.iter().any(|x| !(-2.0..=2.0).contains(x)) {
        return Err(unsupported("Leja points must lie in [-2, 2]"));
    }
    let shift_lo = sub_down(c, mul_up(2.0, gamma)?)?;
    let shift_hi = sub_up(c, mul_down(2.0, gamma)?)?;
    if shift_lo < -LEJA_RANGE_LIMIT - 1.0 {
        return Err(unsupported(format!(
            "interval left end {shift_lo:e} beyond -{LEJA_RANGE_LIMIT}"
        )));
    }
    let exp_s_lo = exp_interval(shift_lo)?.lo;
    let mut exp_s_hi = exp_interval(shift_hi)?.hi;
    if exp_s_hi <= 0.0 {
        exp_s_hi = 0.0_f64.next_up();
    }
    let exp_s = Interval::new(exp_s_lo, exp_s_hi)?;

    let zero_node_lo = (-shift_hi).max(0.0);
    let zero_node_hi = (-shift_lo).max(0.0);
    let mut diagonal_lo = vec![zero_node_lo; ZERO_NODES];
    let mut diagonal_hi = vec![zero_node_hi; ZERO_NODES];
    for &x in xi {
        let x2_lo = add_down(x, 2.0)?;
        let x2_hi = add_up(x, 2.0)?;
        diagonal_lo.push(mul_down(gamma, x2_lo)?.max(0.0));
        diagonal_hi.push(mul_up(gamma, x2_hi)?);
    }
    let mut subdiagonal_lo = vec![1.0; ZERO_NODES];
    subdiagonal_lo.extend(std::iter::repeat_n(gamma, xi.len() - 1));
    let mut subdiagonal_hi = vec![1.0; ZERO_NODES];
    subdiagonal_hi.extend(std::iter::repeat_n(gamma, xi.len() - 1));

    let (cols_lo, cols_hi) = exp_bidiagonal_columns_certified(
        &diagonal_lo,
        &diagonal_hi,
        &subdiagonal_lo,
        &subdiagonal_hi,
    )?;

    let mut d = Vec::with_capacity(xi.len());
    let mut d_rad = Vec::with_capacity(xi.len());

    for j in 0..xi.len() {
        let mut mid_row = [0.0; JOINT_PHI_TERMS];
        let mut rad_row = [0.0; JOINT_PHI_TERMS];
        for k in 0..JOINT_PHI_TERMS {
            let col = ZERO_NODES - k;
            let row = ZERO_NODES + j;
            let lo_entry = cols_lo[col][row];
            let hi_entry = cols_hi[col][row];
            let val_lo = mul_down(lo_entry, exp_s.lo)?.max(0.0);
            let val_hi = mul_up(hi_entry, exp_s.hi)?;
            let mid = 0.5 * (val_lo + val_hi);
            let dist_hi = sub_up(val_hi, mid)?;
            let dist_lo = sub_up(mid, val_lo)?;
            let rad = dist_hi.max(dist_lo).max(0.0);
            mid_row[k] = mid;
            rad_row[k] = rad;
        }
        d.push(mid_row);
        d_rad.push(rad_row);
    }
    Ok((d, d_rad))
}

/// Optional coefficient cache for certified Leja runs.
///
/// Separate from `CoefficientCache` and `LaguerreEnvelopeCache`.
#[derive(Clone, Debug, Default)]
pub struct CertifiedLejaCoeffCache {
    entries: HashMap<(u64, u64, usize), CachedCertifiedLeja>,
}

#[derive(Clone, Debug)]
struct CachedCertifiedLeja {
    d: Vec<[f64; JOINT_PHI_TERMS]>,
    d_rad: Vec<[f64; JOINT_PHI_TERMS]>,
}

impl CertifiedLejaCoeffCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[allow(clippy::type_complexity)]
    pub fn get_certified(
        &self,
        center: f64,
        scale: f64,
        degree_cap: usize,
    ) -> Option<(&[[f64; JOINT_PHI_TERMS]], &[[f64; JOINT_PHI_TERMS]])> {
        let key = (center.to_bits(), scale.to_bits(), degree_cap);
        self.entries
            .get(&key)
            .map(|entry| (entry.d.as_slice(), entry.d_rad.as_slice()))
    }

    pub fn insert_certified(
        &mut self,
        center: f64,
        scale: f64,
        degree_cap: usize,
        d: Vec<[f64; JOINT_PHI_TERMS]>,
        d_rad: Vec<[f64; JOINT_PHI_TERMS]>,
    ) {
        let key = (center.to_bits(), scale.to_bits(), degree_cap);
        self.entries.insert(key, CachedCertifiedLeja { d, d_rad });
    }
}

/// A certified Newton-Leja phi action report carrying a total error bound.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CertifiedLejaReport {
    pub schema: String,
    pub input: String,
    pub branch: String,
    pub dimension: usize,
    pub degree: usize,
    pub degree_cap: usize,
    pub converged: bool,
    pub interval: [f64; 2],
    pub center: f64,
    pub scale: f64,
    pub total_budget: f64,
    pub total_bound: f64,
    pub truncation_bound: f64,
    pub divided_difference_bound: f64,
    pub recurrence_bound: f64,
    pub summation_bound: f64,
    pub columns: Vec<Vec<f64>>,
    pub fused: Vec<f64>,
    total_error: TotalErrorStatus,
    pub evidence: EnclosureEvidence,
    pub vector_products: u64,
    pub block_products: u64,
    pub coefficient_setups: u64,
    pub certification_ops: u64,
}

impl CertifiedLejaReport {
    pub fn total_error(&self) -> &TotalErrorStatus {
        &self.total_error
    }
    pub fn total_bound(&self) -> f64 {
        self.total_bound
    }
    pub fn truncation_bound(&self) -> f64 {
        self.truncation_bound
    }
    pub fn divided_difference_bound(&self) -> f64 {
        self.divided_difference_bound
    }
    pub fn recurrence_bound(&self) -> f64 {
        self.recurrence_bound
    }
    pub fn summation_bound(&self) -> f64 {
        self.summation_bound
    }
}

/// Pre-allocated workspace for certified Leja phi action.
///
/// Ensures zero heap allocations in the degree/recurrence inner loops.
#[derive(Clone, Debug)]
pub struct CertifiedLejaWorkspace {
    pub current: Vec<Vec<f64>>,
    pub product: Vec<Vec<f64>>,
    pub sums: Vec<Vec<f64>>,
    pub columns: Vec<Vec<f64>>,
    pub fused: Vec<f64>,
    pub r_norms: [[f64; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP + 1],
    pub residuals: [[f64; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP],
    pub report: Option<CertifiedLejaReport>,
}

impl Default for CertifiedLejaWorkspace {
    fn default() -> Self {
        Self {
            current: Vec::new(),
            product: Vec::new(),
            sums: Vec::new(),
            columns: Vec::new(),
            fused: Vec::new(),
            r_norms: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP + 1],
            residuals: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP],
            report: None,
        }
    }
}

impl CertifiedLejaWorkspace {
    pub fn new(dimension: usize, width: usize) -> Self {
        let columns = vec![vec![0.0; dimension]; JOINT_PHI_TERMS];
        let fused = vec![0.0; dimension];
        let report = CertifiedLejaReport {
            schema: String::with_capacity(64),
            input: String::with_capacity(32),
            branch: String::with_capacity(32),
            dimension,
            degree: 0,
            degree_cap: LEJA_DEGREE_CAP,
            converged: false,
            interval: [0.0, 0.0],
            center: 0.0,
            scale: 0.0,
            total_budget: 0.0,
            total_bound: 0.0,
            truncation_bound: 0.0,
            divided_difference_bound: 0.0,
            recurrence_bound: 0.0,
            summation_bound: 0.0,
            columns: vec![vec![0.0; dimension]; JOINT_PHI_TERMS],
            fused: vec![0.0; dimension],
            total_error: TotalErrorStatus::Certified { bound: 0.0 },
            evidence: EnclosureEvidence::Gershgorin,
            vector_products: 0,
            block_products: 0,
            coefficient_setups: 0,
            certification_ops: 0,
        };
        Self {
            current: vec![vec![0.0; dimension]; width],
            product: vec![vec![0.0; dimension]; width],
            sums: vec![vec![0.0; dimension]; JOINT_PHI_TERMS],
            columns,
            fused,
            r_norms: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP + 1],
            residuals: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP],
            report: Some(report),
        }
    }

    pub fn ensure_capacity(&mut self, dimension: usize, width: usize) {
        if self.current.len() < width {
            self.current.resize_with(width, Vec::new);
        }
        for v in self.current.iter_mut().take(width) {
            if v.len() < dimension {
                v.resize(dimension, 0.0);
            }
        }
        if self.product.len() < width {
            self.product.resize_with(width, Vec::new);
        }
        for v in self.product.iter_mut().take(width) {
            if v.len() < dimension {
                v.resize(dimension, 0.0);
            }
        }
        if self.sums.len() < JOINT_PHI_TERMS {
            self.sums.resize_with(JOINT_PHI_TERMS, Vec::new);
        }
        for v in self.sums.iter_mut() {
            if v.len() < dimension {
                v.resize(dimension, 0.0);
            }
        }
        if self.columns.len() < JOINT_PHI_TERMS {
            self.columns.resize_with(JOINT_PHI_TERMS, Vec::new);
        }
        for v in self.columns.iter_mut() {
            if v.len() < dimension {
                v.resize(dimension, 0.0);
            }
        }
        if self.fused.len() < dimension {
            self.fused.resize(dimension, 0.0);
        }
    }
}

/// [`leja_phi_action_certified_capped`] with the cap [`LEJA_DEGREE_CAP`].
pub fn leja_phi_action_certified(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    work: &mut WorkCounters,
) -> CoreResult<CertifiedLejaReport> {
    leja_phi_action_certified_with_cache(op, h, input, total_budget, LEJA_DEGREE_CAP, None, work)
}

/// Certified Leja phi action with a degree cap in `1 ..= LEJA_DEGREE_CAP`.
pub fn leja_phi_action_certified_capped(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    degree_cap: usize,
    work: &mut WorkCounters,
) -> CoreResult<CertifiedLejaReport> {
    leja_phi_action_certified_with_cache(op, h, input, total_budget, degree_cap, None, work)
}

/// Certified Leja phi action with an optional coefficient cache.
pub fn leja_phi_action_certified_with_cache(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    degree_cap: usize,
    cache: Option<&mut CertifiedLejaCoeffCache>,
    work: &mut WorkCounters,
) -> CoreResult<CertifiedLejaReport> {
    let width = match input {
        JointPhiInput::Distinct(_) => JOINT_PHI_TERMS,
        JointPhiInput::SameVector { .. } => 1,
    };
    let mut ws = CertifiedLejaWorkspace::new(op.dimension(), width);
    let rep_ref = leja_phi_action_certified_with_cache_and_workspace(
        op,
        h,
        input,
        total_budget,
        degree_cap,
        cache,
        work,
        &mut ws,
    )?;
    Ok(rep_ref.clone())
}

/// Certified Leja phi action with preallocated workspace and optional coefficient cache.
///
/// Guaranteed zero heap allocations on warmed calls where workspace and cache entries are pre-warmed.
#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
pub fn leja_phi_action_certified_with_cache_and_workspace<'a>(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    degree_cap: usize,
    mut cache: Option<&mut CertifiedLejaCoeffCache>,
    work: &mut WorkCounters,
    workspace: &'a mut CertifiedLejaWorkspace,
) -> CoreResult<&'a CertifiedLejaReport> {
    let n = op.dimension();
    if !(h.is_finite() && h >= 0.0) {
        return Err(unsupported(format!(
            "step h = {h:e} must be finite and >= 0"
        )));
    }
    if !(total_budget.is_finite() && total_budget > 0.0) {
        return Err(unsupported(format!(
            "total budget {total_budget:e} must be finite and positive"
        )));
    }
    if !(1..=LEJA_DEGREE_CAP).contains(&degree_cap) {
        return Err(unsupported(format!(
            "degree cap {degree_cap} outside 1..={LEJA_DEGREE_CAP}"
        )));
    }
    let enclosure = op.enclosure();
    let lambda = enclosure.lambda;
    let rho = enclosure.rho;
    if !matches!(enclosure.evidence, EnclosureEvidence::Gershgorin) {
        return Err(unsupported(
            "the spectral enclosure is not Gershgorin-verified",
        ));
    }
    let evidence = EnclosureEvidence::Gershgorin;

    let width = match input {
        JointPhiInput::Distinct(_) => JOINT_PHI_TERMS,
        JointPhiInput::SameVector { .. } => 1,
    };
    workspace.ensure_capacity(n, width);

    let (scales, input_name): ([f64; JOINT_PHI_TERMS], &'static str) = match input {
        JointPhiInput::Distinct(vectors) => {
            if vectors.iter().any(|w| w.len() != n) {
                return Err(unsupported("every w_k must have the operator dimension"));
            }
            for (dst, src) in workspace.current[..width].iter_mut().zip(vectors.iter()) {
                dst[..n].copy_from_slice(&src[..n]);
            }
            ([1.0; JOINT_PHI_TERMS], "distinct")
        }
        JointPhiInput::SameVector { vector, scales } => {
            if vector.len() != n {
                return Err(unsupported("vector must have the operator dimension"));
            }
            workspace.current[0][..n].copy_from_slice(&vector[..n]);
            (scales, "same-vector")
        }
    };

    for col in 0..width {
        if !workspace.current[col][..n].iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite("Leja input contains NaN/Inf".into()));
        }
    }
    if !scales.iter().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite("Leja scales contain NaN/Inf".into()));
    }

    let a = -mul_up(h, rho)?;
    let b = -mul_down(h, lambda)?;
    if -a > LEJA_RANGE_LIMIT {
        return Err(unsupported(format!(
            "h rho = {:e} above {LEJA_RANGE_LIMIT}",
            -a
        )));
    }
    let column_of = |k: usize| if width == 1 { 0 } else { k };
    let zero_input = (0..JOINT_PHI_TERMS).all(|k| {
        scales[k] == 0.0
            || workspace.current[column_of(k)][..n]
                .iter()
                .all(|x| *x == 0.0)
    });
    let scalar = h == 0.0 || rho == 0.0 || lambda == rho;

    for sum in workspace.sums[..JOINT_PHI_TERMS].iter_mut() {
        sum[..n].fill(0.0);
    }

    let mut degree = 0;
    let mut converged = false;
    let mut total_bound = 0.0;
    let mut truncation_bound = 0.0;
    let mut divided_difference_bound = 0.0;
    let mut recurrence_bound = 0.0;
    let mut summation_bound = 0.0;
    let mut certification_ops = 0_u64;

    let mut local_block_products = 0_u64;
    let mut local_vector_products = 0_u64;
    let mut local_coefficient_setups = 0_u64;

    let mut center = 0.0_f64;
    let mut scale = 0.0_f64;
    let mut branch = "zero-input";
    let mut verified_interval = [a, b];

    let mut run_action = || -> CoreResult<()> {
        if zero_input {
            center = 0.5 * (a + b);
            scale = 0.25 * (b - a);
            branch = "zero-input";
            verified_interval = [a, b];
            converged = true;
        } else if scalar {
            center = if h == 0.0 || rho == 0.0 { 0.0 } else { b };
            scale = 0.0;
            branch = "scalar";
            verified_interval = [a, b];
            local_coefficient_setups += 1;
            let (d, d_rad) = leja_divided_differences_certified(center, 0.0, &[2.0])?;
            for (k, sum) in workspace.sums[..JOINT_PHI_TERMS].iter_mut().enumerate() {
                let col = column_of(k);
                let w = &workspace.current[col][..n];
                for (out, wi) in sum[..n].iter_mut().zip(w) {
                    *out = d[0][k] * *wi;
                }
            }
            let mut diff_terms = [0.0; JOINT_PHI_TERMS];
            let mut sum_terms = [0.0; JOINT_PHI_TERMS];
            let mut p_norms = [0.0; JOINT_PHI_TERMS];
            let interval_width = sub_up(b, a)?;
            certification_ops = certification_ops.saturating_add(1);

            for k in 0..JOINT_PHI_TERMS {
                let col = column_of(k);
                let w_norm = safe_l2(&workspace.current[col][..n]);
                p_norms[k] = safe_l2(&workspace.sums[k][..n]);
                let d_err = mul_up(d_rad[0][k], w_norm)?;
                let fact = match k {
                    0 => 1.0,
                    1 => 2.0,
                    2 => 6.0,
                    3 => 24.0,
                    _ => 120.0,
                };
                let width_err = mul_up(div_up(interval_width, fact)?, w_norm)?;
                let total_d_err = add_up(d_err, width_err)?;
                diff_terms[k] = mul_up(scales[k].abs(), total_d_err)?;
                sum_terms[k] = mul_up(scales[k].abs(), mul_up(f64::EPSILON, p_norms[k])?)?;
                certification_ops = certification_ops.saturating_add(7);
            }
            divided_difference_bound = sum_up_slice(&diff_terms)?;
            let mut fused_terms = [0.0; JOINT_PHI_TERMS];
            for k in 0..JOINT_PHI_TERMS {
                fused_terms[k] = mul_up(scales[k].abs(), p_norms[k])?;
            }
            let fused_err = mul_up(4.0 * f64::EPSILON, sum_up_slice(&fused_terms)?)?;
            summation_bound = add_up(sum_up_slice(&sum_terms)?, fused_err)?;
            total_bound = add_up(divided_difference_bound, summation_bound)?;
            certification_ops = certification_ops.saturating_add(16);
            converged = total_bound <= total_budget;
        } else {
            center = 0.5 * (a + b);
            let mut sc = div_up(sub_up(b, a)?, 4.0)?;
            if sc > 0.0 {
                while sub_down(center, mul_up(2.0, sc)?)? > a
                    || add_up(center, mul_up(2.0, sc)?)? < b
                {
                    sc = sc.next_up();
                }
            }
            scale = sc;
            verified_interval = [
                sub_down(center, mul_up(2.0, scale)?)?,
                add_up(center, mul_up(2.0, scale)?)?,
            ];
            branch = "newton";
            let xi = &leja_points()[..=degree_cap];
            let node_bounds = leja_node_bounds();

            let (d_calc, dr_calc);
            let (d, d_rad): (&[[f64; JOINT_PHI_TERMS]], &[[f64; JOINT_PHI_TERMS]]) =
                if let Some(ref mut c) = cache {
                    if c.get_certified(center, scale, degree_cap).is_none() {
                        local_coefficient_setups += 1;
                        let (d_val, dr_val) =
                            leja_divided_differences_certified(center, scale, xi)?;
                        c.insert_certified(center, scale, degree_cap, d_val, dr_val);
                    }
                    c.get_certified(center, scale, degree_cap).unwrap()
                } else {
                    local_coefficient_setups += 1;
                    let res = leja_divided_differences_certified(center, scale, xi)?;
                    d_calc = res.0;
                    dr_calc = res.1;
                    (&d_calc, &dr_calc)
                };

            for col in 0..width {
                workspace.r_norms[0][col] = safe_l2(&workspace.current[col][..n]);
            }

            for (k, sum) in workspace.sums[..JOINT_PHI_TERMS].iter_mut().enumerate() {
                let col = column_of(k);
                let coeff = d[0][k];
                let cur = &workspace.current[col][..n];
                for (out, x) in sum[..n].iter_mut().zip(cur) {
                    *out += coeff * *x;
                }
            }

            let u = f64::EPSILON;
            let nu = mul_up(n as f64, u)?;
            let denom = sub_down(1.0, nu)?;
            if denom <= 0.0 {
                return Err(CoreError::InvalidInput(format!(
                    "dimension n = {n} exceeds precision threshold for rounding error bound (n*u >= 1)"
                )));
            }
            let gamma_n = div_up(nu, denom)?;
            certification_ops = certification_ops.saturating_add(3);

            for j in 1..=degree_cap {
                for col in 0..width {
                    op.matrix().matvec_into(
                        &workspace.current[col][..n],
                        &mut workspace.product[col][..n],
                    )?;
                }
                local_block_products += 1;
                local_vector_products += width as u64;

                let node = xi[j - 1];
                for col in 0..width {
                    let cur = &mut workspace.current[col][..n];
                    let prod = &workspace.product[col][..n];
                    for (ri, yi) in cur.iter_mut().zip(prod) {
                        *ri = (h * *yi - center * *ri) / scale - node * *ri;
                    }
                }

                for col in 0..width {
                    if !workspace.current[col][..n].iter().all(|x| x.is_finite()) {
                        return Err(CoreError::NonFinite(
                            "Newton-Leja recurrence produced NaN/Inf".into(),
                        ));
                    }
                }

                for col in 0..width {
                    let r_norm = safe_l2(&workspace.current[col][..n]);
                    workspace.r_norms[j][col] = r_norm;
                    let prev_norm = workspace.r_norms[j - 1][col];
                    let term_mv =
                        mul_up(div_up(h, scale)?, mul_up(mul_up(gamma_n, rho)?, prev_norm)?)?;
                    let term_scalar = mul_up(6.0 * u, r_norm)?;
                    let factor_pre = add_up(
                        div_up(add_up(mul_up(h, rho)?, center.abs())?, scale)?,
                        node.abs(),
                    )?;
                    let term_pre = mul_up(4.0 * u, mul_up(factor_pre, prev_norm)?)?;
                    workspace.residuals[j - 1][col] =
                        add_up(term_mv, add_up(term_scalar, term_pre)?)?;
                    certification_ops = certification_ops.saturating_add(13);
                }

                for (k, sum) in workspace.sums[..JOINT_PHI_TERMS].iter_mut().enumerate() {
                    let col = column_of(k);
                    let coeff = d[j][k];
                    let cur = &workspace.current[col][..n];
                    for (out, x) in sum[..n].iter_mut().zip(cur) {
                        *out += coeff * *x;
                    }
                }

                // Certified bounds evaluation at degree j:
                // Truncation bound: node polynomial bound M_{j+1} over [-2, 2] times scaling.
                let m_val = node_bounds[j];
                let mut t1_terms = [0.0_f64; JOINT_PHI_TERMS];
                for k in 0..JOINT_PHI_TERMS {
                    let col = column_of(k);
                    let mut coeff_k = 1.0_f64;
                    for p in 1..=(j + 1 + k) {
                        let numer = if p <= j + 1 { scale } else { 1.0 };
                        coeff_k = div_up(mul_up(coeff_k, numer)?, p as f64)?;
                        certification_ops = certification_ops.saturating_add(2);
                    }
                    t1_terms[k] = mul_up(
                        scales[k].abs(),
                        mul_up(workspace.r_norms[0][col], mul_up(m_val, coeff_k)?)?,
                    )?;
                    certification_ops = certification_ops.saturating_add(3);
                }
                let cur_t1 = sum_up_slice(&t1_terms)?;
                certification_ops = certification_ops.saturating_add(4);

                // Divided difference radius bound:
                let mut t2_terms = [0.0_f64; JOINT_PHI_TERMS];
                for k in 0..JOINT_PHI_TERMS {
                    let col = column_of(k);
                    let mut col_sum = 0.0_f64;
                    for step_i in 0..=j {
                        col_sum = add_up(
                            col_sum,
                            mul_up(d_rad[step_i][k], workspace.r_norms[step_i][col])?,
                        )?;
                        certification_ops = certification_ops.saturating_add(2);
                    }
                    t2_terms[k] = mul_up(scales[k].abs(), col_sum)?;
                    certification_ops = certification_ops.saturating_add(1);
                }
                let cur_t2 = sum_up_slice(&t2_terms)?;
                certification_ops = certification_ops.saturating_add(4);

                // Recurrence propagation bound:
                // Forward error recursion: residual at step_i propagates through (hA - cI)/scale - xi*I,
                // bounded by subpolynomial bound node_bounds[deg_diff - 1] over [-2, 2].
                let mut t3_terms = [0.0_f64; JOINT_PHI_TERMS];
                for k in 0..JOINT_PHI_TERMS {
                    let col = column_of(k);
                    let mut col_sum = 0.0_f64;
                    for step_i in 0..j {
                        let mut t_poly = 0.0_f64;
                        for l in (step_i + 1)..=j {
                            let deg_diff = l - step_i - 1;
                            let m_sub = if deg_diff == 0 {
                                1.0
                            } else {
                                node_bounds[deg_diff - 1]
                            };
                            t_poly = add_up(t_poly, mul_up(d[l][k].abs(), m_sub)?)?;
                            certification_ops = certification_ops.saturating_add(2);
                        }
                        col_sum =
                            add_up(col_sum, mul_up(t_poly, workspace.residuals[step_i][col])?)?;
                        certification_ops = certification_ops.saturating_add(2);
                    }
                    t3_terms[k] = mul_up(scales[k].abs(), col_sum)?;
                    certification_ops = certification_ops.saturating_add(1);
                }
                let cur_t3 = sum_up_slice(&t3_terms)?;
                certification_ops = certification_ops.saturating_add(4);

                let ju = mul_up((j + 1) as f64, u)?;
                let denom_j = sub_down(1.0, ju)?;
                if denom_j <= 0.0 {
                    return Err(CoreError::InvalidInput(format!(
                        "degree j = {j} exceeds precision threshold (j*u >= 1)"
                    )));
                }
                let factor_running = div_up(ju, denom_j)?;
                certification_ops = certification_ops.saturating_add(3);

                // Summation and fused rounding bounds:
                let mut p_norms = [0.0; JOINT_PHI_TERMS];
                let mut t4_terms = [0.0_f64; JOINT_PHI_TERMS];
                for k in 0..JOINT_PHI_TERMS {
                    p_norms[k] = safe_l2(&workspace.sums[k][..n]);
                    let col = column_of(k);
                    let mut sum_r = 0.0_f64;
                    for step_i in 0..=j {
                        sum_r = add_up(
                            sum_r,
                            mul_up(d[step_i][k].abs(), workspace.r_norms[step_i][col])?,
                        )?;
                        certification_ops = certification_ops.saturating_add(2);
                    }
                    let term_running = mul_up(factor_running, sum_r)?;
                    let term_scale = mul_up(u, mul_up(scales[k].abs(), p_norms[k])?)?;
                    t4_terms[k] = add_up(mul_up(scales[k].abs(), term_running)?, term_scale)?;
                    certification_ops = certification_ops.saturating_add(5);
                }
                let mut fused_terms = [0.0; JOINT_PHI_TERMS];
                for k in 0..JOINT_PHI_TERMS {
                    fused_terms[k] = mul_up(scales[k].abs(), p_norms[k])?;
                }
                let fused_err = mul_up(4.0 * u, sum_up_slice(&fused_terms)?)?;
                let cur_t4 = add_up(sum_up_slice(&t4_terms)?, fused_err)?;
                certification_ops = certification_ops.saturating_add(11);

                let cur_total = add_up(cur_t1, add_up(cur_t2, add_up(cur_t3, cur_t4)?)?)?;
                certification_ops = certification_ops.saturating_add(3);

                degree = j;
                truncation_bound = cur_t1;
                divided_difference_bound = cur_t2;
                recurrence_bound = cur_t3;
                summation_bound = cur_t4;
                total_bound = cur_total;

                if cur_total <= total_budget {
                    converged = true;
                    break;
                }
            }
        }
        Ok(())
    };

    let outcome = run_action();

    // UNCONDITIONALLY flush work counters on both Ok and Err paths:
    work.poly_block_products += local_block_products;
    work.poly_vector_products += local_vector_products;
    work.poly_coefficient_setups += local_coefficient_setups;

    outcome?;

    for k in 0..JOINT_PHI_TERMS {
        let scale_k = scales[k];
        let sum_k = &workspace.sums[k][..n];
        let col_k = &mut workspace.columns[k][..n];
        for (dst, src) in col_k.iter_mut().zip(sum_k) {
            *dst = scale_k * *src;
        }
    }
    let fused_slice = &mut workspace.fused[..n];
    fused_slice.fill(0.0);
    for k in 0..JOINT_PHI_TERMS {
        let col_k = &workspace.columns[k][..n];
        for (f, c) in fused_slice.iter_mut().zip(col_k) {
            *f += *c;
        }
    }
    if !fused_slice.iter().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite(
            "Newton-Leja action produced NaN/Inf".into(),
        ));
    }

    let total_error = if converged {
        TotalErrorStatus::Certified { bound: total_bound }
    } else {
        TotalErrorStatus::EstimateOnly {
            reason: format!(
                "{LEJA_TOTAL_NOT_CERTIFIED}: degree cap {degree_cap} reached without meeting budget {total_budget:e}"
            ),
            bounded_components: total_bound,
        }
    };

    if let Some(ref mut rep) = workspace.report {
        rep.schema.clear();
        rep.schema.push_str(LEJA_CERTIFIED_SCHEMA);
        rep.input.clear();
        rep.input.push_str(input_name);
        rep.branch.clear();
        rep.branch.push_str(branch);
        rep.dimension = n;
        rep.degree = degree;
        rep.degree_cap = degree_cap;
        rep.converged = converged;
        rep.interval = verified_interval;
        rep.center = center;
        rep.scale = scale;
        rep.total_budget = total_budget;
        rep.total_bound = total_bound;
        rep.truncation_bound = truncation_bound;
        rep.divided_difference_bound = divided_difference_bound;
        rep.recurrence_bound = recurrence_bound;
        rep.summation_bound = summation_bound;
        for k in 0..JOINT_PHI_TERMS {
            rep.columns[k][..n].copy_from_slice(&workspace.columns[k][..n]);
        }
        rep.fused[..n].copy_from_slice(&workspace.fused[..n]);
        rep.total_error = total_error;
        rep.evidence = evidence;
        rep.vector_products = local_vector_products;
        rep.block_products = local_block_products;
        rep.coefficient_setups = local_coefficient_setups;
        rep.certification_ops = certification_ops;
    } else {
        workspace.report = Some(CertifiedLejaReport {
            schema: LEJA_CERTIFIED_SCHEMA.into(),
            input: input_name.into(),
            branch: branch.into(),
            dimension: n,
            degree,
            degree_cap,
            converged,
            interval: verified_interval,
            center,
            scale,
            total_budget,
            total_bound,
            truncation_bound,
            divided_difference_bound,
            recurrence_bound,
            summation_bound,
            columns: (0..JOINT_PHI_TERMS)
                .map(|k| workspace.columns[k][..n].to_vec())
                .collect(),
            fused: workspace.fused[..n].to_vec(),
            total_error,
            evidence,
            vector_products: local_vector_products,
            block_products: local_block_products,
            coefficient_setups: local_coefficient_setups,
            certification_ops,
        });
    }

    Ok(workspace.report.as_ref().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leja_points_start_at_the_ends_and_stay_distinct() {
        let xi = leja_points();
        assert_eq!(xi.len(), LEJA_DEGREE_CAP + 1);
        assert_eq!(xi[0], 2.0);
        assert_eq!(xi[1], -2.0);
        assert!(xi[2].abs() < 1e-3, "{}", xi[2]);
        let mut sorted = xi.to_vec();
        sorted.sort_by(f64::total_cmp);
        assert!(sorted.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn divided_differences_match_closed_forms() {
        // Two nodes: gamma exp[x0, x1] = gamma (e^x1 - e^x0) / (x1 - x0).
        let (c, gamma) = (-3.0, 0.75);
        let xi = [2.0, -2.0];
        let d = leja_divided_differences(c, gamma, &xi).unwrap();
        let (x0, x1) = (c + gamma * 2.0, c - gamma * 2.0);
        assert!((d[0][0] - x0.exp()).abs() <= 4.0 * f64::EPSILON * x0.exp());
        let expected = gamma * (x1.exp() - x0.exp()) / (x1 - x0);
        assert!(
            (d[1][0] - expected).abs() <= 1e-14 * expected.abs(),
            "{d:?}"
        );
        // phi_1(x0) = (e^x0 - 1) / x0.
        let phi1 = x0.exp_m1() / x0;
        assert!((d[0][1] - phi1).abs() <= 1e-14 * phi1, "{d:?}");
        // Single node at 0: 1/k!.
        let d = leja_divided_differences(0.0, 0.0, &[2.0]).unwrap();
        for (k, f) in [1.0, 1.0, 2.0, 6.0, 24.0].iter().enumerate() {
            assert!((d[0][k] - 1.0 / f).abs() <= 4.0 * f64::EPSILON / f);
        }
    }
}
