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

use std::sync::OnceLock;

use serde::Serialize;

use crate::{
    CoreError, CoreResult, DenseOperator, LinearOperator, WorkCounters,
    directed::{mul_down, mul_up},
    polynomial_action::{
        EnclosureEvidence, JOINT_PHI_TERMS, JointPhiInput, SpectralEnclosure,
        SymmetricNonpositiveOperator, TotalErrorStatus,
    },
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
