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
use std::fmt::Write as _;
use std::sync::OnceLock;

use serde::Serialize;

use crate::leja_setup_work::{
    CountedStaticTable, LejaSetupWork, LejaSetupWorkSubtotal, SetupCoverage, StaticLejaTable,
    get_or_init_static,
};

use crate::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, LinearOperator, WorkCounters,
    directed::{
        Interval, add_down, add_up, div_down, div_up, exp_interval, mul_down, mul_up, sqrt_up,
        sub_down, sub_up,
    },
    polynomial_action::{
        EnclosureEvidence, JOINT_PHI_TERMS, JointPhiInput, SpectralEnclosure,
        SymmetricNonpositiveOperator, TotalErrorStatus,
    },
};

pub const LEJA_SCHEMA: &str = "vigilode-leja-phi-action-v1";
/// The reason of every Leja total: an estimate, never a certificate.
pub const LEJA_TOTAL_NOT_CERTIFIED: &str = "LEJA_TOTAL_NOT_CERTIFIED";
/// Total authority remains held until independent review accepts the interval
/// replay and its complete finite-domain/coefficient-tail premises.
pub const LEJA_ARITHMETIC_HOLD: &str = "LEJA_ARITHMETIC_HOLD: outward replay implementation awaits independent review of arithmetic-domain and coefficient-tail premises";
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

macro_rules! setup_kernel {
    ($work:ident, $operation:expr) => {{
        if let Some(counter) = $work.as_deref_mut() {
            counter.interval_kernel_invocation();
        }
        $operation
    }};
}

fn unsupported(reason: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("{LEJA_INPUT_UNSUPPORTED}: {reason}"))
}

/// The first [`LEJA_DEGREE_CAP`]` + 1` discrete Leja points of `[-2, 2]`.
pub fn leja_points() -> &'static [f64] {
    leja_points_with_setup(None)
}

/// Leja points with a separate cold-setup receipt accumulator.
pub fn leja_points_with_setup_work(work: &mut LejaSetupWork) -> &'static [f64] {
    leja_points_with_setup(Some(work))
}

fn leja_points_with_setup(work: Option<&mut LejaSetupWork>) -> &'static [f64] {
    static POINTS: OnceLock<CountedStaticTable<Vec<f64>>> = OnceLock::new();
    &get_or_init_static(&POINTS, StaticLejaTable::Points, work, build_leja_points)[..]
}

fn build_leja_points(mut work: Option<&mut LejaSetupWork>) -> Vec<f64> {
    const GRID_LEN: u64 = (LEJA_GRID_INTERVALS + 1) as u64;
    if let Some(work) = work.as_deref_mut() {
        // Four vector allocations: grid, log products, taken flags, output.
        work.allocation_request(GRID_LEN);
        work.allocation_request(GRID_LEN);
        work.allocation_request(GRID_LEN);
        work.allocation_request((LEJA_DEGREE_CAP + 1) as u64);
    }
    let mut grid: Vec<f64> = (0..=LEJA_GRID_INTERVALS)
        .map(|i| {
            if let Some(work) = work.as_deref_mut() {
                // Source-level scalar proxies: multiply, divide, cosine, multiply.
                work.scalar_proxy_units(4);
            }
            2.0 * (std::f64::consts::PI * i as f64 / LEJA_GRID_INTERVALS as f64).cos()
        })
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
            if let Some(work) = work.as_deref_mut() {
                // subtraction, absolute value, logarithm, accumulation, comparison
                work.scalar_proxy_units(5);
            }
            log_product[i] += (g - xi).abs().ln();
            if log_product[i] > best {
                best = log_product[i];
                next = i;
            }
        }
    }
    points
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
    leja_node_bounds_with_setup(None)
}

/// Node-polynomial prefix bounds with a separate cold-setup receipt.
pub fn leja_node_bounds_with_setup_work(work: &mut LejaSetupWork) -> &'static [f64] {
    leja_node_bounds_with_setup(Some(work))
}

fn leja_node_bounds_with_setup(work: Option<&mut LejaSetupWork>) -> &'static [f64] {
    static BOUNDS: OnceLock<CountedStaticTable<Vec<f64>>> = OnceLock::new();
    &get_or_init_static(&BOUNDS, StaticLejaTable::NodeBounds, work, |mut work| {
        let xi = leja_points_with_setup(work.as_deref_mut());
        if let Some(work) = work.as_deref_mut() {
            work.allocation_request((LEJA_DEGREE_CAP + 1) as u64);
        }
        let bounds: Vec<f64> = (0..=LEJA_DEGREE_CAP)
            .map(|m| leja_node_polynomial_bound_counted(m, xi, work.as_deref_mut()))
            .collect();
        bounds
    })[..]
}

/// Outward bounds for every contiguous Newton suffix: entry `[start][end]`
/// bounds `prod_{i=start}^{end-1} |x-xi_i|` on `[-2,2]`.
/// Prefix bounds cannot be substituted: the first two nodes have maximum 4,
/// while nodes 1 and 2 have product nearly 8 at x=2.
#[cfg(test)]
fn leja_suffix_bounds() -> &'static Vec<Vec<f64>> {
    leja_suffix_bounds_with_setup(None)
}

/// Contiguous Newton suffix bounds with a separate cold-setup receipt.
pub fn leja_suffix_bounds_with_setup_work(work: &mut LejaSetupWork) -> &'static Vec<Vec<f64>> {
    leja_suffix_bounds_with_setup(Some(work))
}

fn leja_suffix_bounds_with_setup(work: Option<&mut LejaSetupWork>) -> &'static Vec<Vec<f64>> {
    static BOUNDS: OnceLock<CountedStaticTable<Vec<Vec<f64>>>> = OnceLock::new();
    get_or_init_static(&BOUNDS, StaticLejaTable::SuffixBounds, work, |mut work| {
        let xi = leja_points_with_setup(work.as_deref_mut());
        let size = xi.len();
        if let Some(work) = work.as_deref_mut() {
            // One outer allocation and one row allocation for each start index.
            work.allocation_request(size as u64);
            for _ in 0..size {
                work.allocation_request((size + 1) as u64);
            }
        }
        let mut bounds: Vec<Vec<f64>> = vec![vec![0.0; size + 1]; size];
        for (start, row) in bounds.iter_mut().enumerate() {
            row[start] = 1.0;
            for part in 0..128 {
                if let Some(work) = work.as_deref_mut() {
                    work.scalar_proxy_units(3); // interval endpoints: one division and two adds
                    work.interval_kernel_invocation(); // Interval::point(1)
                }
                let lo = -2.0 + part as f64 / 32.0;
                let hi = lo + 1.0 / 32.0;
                let mut product = Interval::point(1.0).expect("finite exact constant");
                for end in start + 1..=size {
                    if let Some(work) = work.as_deref_mut() {
                        work.interval_kernel_invocation(); // sub_down
                    }
                    let factor_lo = sub_down(lo, xi[end - 1]).expect("finite Leja nodes");
                    if let Some(work) = work.as_deref_mut() {
                        work.interval_kernel_invocation(); // sub_up
                    }
                    let factor_hi = sub_up(hi, xi[end - 1]).expect("finite Leja nodes");
                    if let Some(work) = work.as_deref_mut() {
                        work.interval_kernel_invocation(); // interval construction
                    }
                    let factor = Interval::new(factor_lo, factor_hi).expect("ordered endpoints");
                    if let Some(work) = work.as_deref_mut() {
                        work.interval_kernel_invocation(); // interval multiplication
                    }
                    product = product.mul(factor).expect("bounded degree Leja product");
                    row[end] = row[end].max(product.mag());
                    if let Some(work) = work.as_deref_mut() {
                        work.scalar_proxy_units(3); // magnitude and maximum comparisons
                    }
                }
            }
        }
        bounds
    })
}

/// Scaled outward Euclidean norm for certificate arithmetic. Ordinary
/// `safe_l2` is a stable estimate, but can round below the exact norm.
fn certificate_l2_upper(x: &[f64], ops: &mut u64) -> CoreResult<f64> {
    if !x.iter().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite("Leja norm contains NaN/Inf".into()));
    }
    let scale = x.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Ok(0.0);
    }
    let mut sum = 0.0;
    for &value in x {
        *ops = ops.saturating_add(3);
        let normalized = div_up(value.abs(), scale)?;
        sum = add_up(sum, mul_up(normalized, normalized)?)?;
    }
    *ops = ops.saturating_add(2);
    mul_up(scale, sqrt_up(sum)?)
}

/// Enclose the exact matrix-vector product of the represented matrix and vector.
/// This is deliberately replayed independently of `DenseMatrix::matvec_into`:
/// its interval result lets the caller bound the actual local arithmetic defect,
/// including additive underflow and cancellation.
fn matvec_interval_into(
    matrix: &DenseMatrix,
    x: &[f64],
    lo: &mut [f64],
    hi: &mut [f64],
    ops: &mut u64,
) -> CoreResult<()> {
    if matrix.ncols() != x.len() || lo.len() != matrix.nrows() || hi.len() != matrix.nrows() {
        return Err(CoreError::Dimension(
            "Leja interval matvec shape mismatch".into(),
        ));
    }
    for i in 0..matrix.nrows() {
        let mut acc = Interval::point(0.0)?;
        for (&a, &xj) in matrix.row(i).iter().zip(x) {
            *ops = ops.saturating_add(16);
            acc = acc.add(Interval::point(a)?.mul(Interval::point(xj)?)?)?;
        }
        lo[i] = acc.lo;
        hi[i] = acc.hi;
    }
    Ok(())
}

/// Outward replay of one Newton-Leja recurrence update. `product` encloses
/// the exact represented matrix action before multiplication by `h`; the
/// returned interval therefore includes additive underflow in both the
/// operator action and each recurrence operation.
fn recurrence_interval_next(
    h: f64,
    product: Interval,
    center: f64,
    old: f64,
    scale: f64,
    node: f64,
    ops: &mut u64,
) -> CoreResult<Interval> {
    *ops = ops.saturating_add(48);
    Interval::point(h)?
        .mul(product)?
        .sub(Interval::point(center)?.mul(Interval::point(old)?)?)?
        .div(Interval::point(scale)?)?
        .sub(Interval::point(node)?.mul(Interval::point(old)?)?)
}

#[inline]
fn interval_distance_upper(value: f64, interval: Interval) -> CoreResult<f64> {
    let to_lo = if value >= interval.lo {
        sub_up(value, interval.lo)?
    } else {
        sub_up(interval.lo, value)?
    };
    let to_hi = if value >= interval.hi {
        sub_up(value, interval.hi)?
    } else {
        sub_up(interval.hi, value)?
    };
    Ok(to_lo.max(to_hi))
}

fn normalized_spectral_support(a: f64, b: f64, center: f64, scale: f64) -> CoreResult<Interval> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(unsupported(
            "normalized spectral support requires a positive finite scale",
        ));
    }
    Interval::new(a, b)?
        .sub(Interval::point(center)?)?
        .div(Interval::point(scale)?)
}

fn enlarge_scale_for_normalized_support(
    a: f64,
    b: f64,
    center: f64,
    mut scale: f64,
    ops: &mut u64,
) -> CoreResult<(f64, Interval)> {
    for _ in 0..64 {
        *ops = ops.saturating_add(18);
        let support = normalized_spectral_support(a, b, center, scale)?;
        if support.lo >= -2.0 && support.hi <= 2.0 {
            return Ok((scale, support));
        }
        *ops = ops.saturating_add(12);
        let centered = Interval::new(a, b)?.sub(Interval::point(center)?)?;
        let required = div_up(centered.mag(), 2.0)?;
        let enlarged = scale.max(required).next_up();
        if !enlarged.is_finite() || enlarged <= scale {
            break;
        }
        scale = enlarged;
    }
    Err(unsupported(
        "outward normalized spectral support cannot be enclosed in [-2, 2]",
    ))
}

/// Update an ordinary coefficient sum and its exact outward replay in parallel.
/// The interval encloses the real sum of the represented coefficient and the
/// represented basis vector values; its distance from `sum` is the local
/// additive summation/product defect, including underflow.
fn accumulate_coefficient_term(
    sum: &mut [f64],
    sum_lo: &mut [f64],
    sum_hi: &mut [f64],
    coefficient: f64,
    basis: &[f64],
    ops: &mut u64,
) -> CoreResult<()> {
    if sum.len() != basis.len() || sum_lo.len() != sum.len() || sum_hi.len() != sum.len() {
        return Err(CoreError::Dimension(
            "Leja coefficient accumulation shape mismatch".into(),
        ));
    }
    *ops = ops.saturating_add(1);
    let coeff = Interval::point(coefficient)?;
    for i in 0..sum.len() {
        *ops = ops.saturating_add(16);
        let exact_term = coeff.mul(Interval::point(basis[i])?)?;
        let exact_sum = Interval::new(sum_lo[i], sum_hi[i])?.add(exact_term)?;
        sum[i] += coefficient * basis[i];
        if !sum[i].is_finite() {
            return Err(CoreError::NonFinite(
                "Leja coefficient accumulation produced NaN/Inf".into(),
            ));
        }
        sum_lo[i] = exact_sum.lo;
        sum_hi[i] = exact_sum.hi;
    }
    Ok(())
}

/// Replay column scaling and final fusion in interval arithmetic while writing
/// the same rounded output arrays returned by the action. The returned norm
/// encloses the complete computed-output minus exact represented-sum defect.
#[allow(clippy::too_many_arguments)]
fn replay_scaled_fusion(
    scales: &[f64; JOINT_PHI_TERMS],
    sums: &[Vec<f64>],
    sums_lo: &[Vec<f64>],
    sums_hi: &[Vec<f64>],
    columns: &mut [Vec<f64>],
    fused: &mut [f64],
    out_lo: &mut [f64],
    out_hi: &mut [f64],
    defects: &mut [f64],
    n: usize,
    ops: &mut u64,
) -> CoreResult<f64> {
    fused[..n].fill(0.0);
    out_lo[..n].fill(0.0);
    out_hi[..n].fill(0.0);
    for k in 0..JOINT_PHI_TERMS {
        let scale = scales[k];
        for i in 0..n {
            *ops = ops.saturating_add(16);
            let rounded_column = scale * sums[k][i];
            if !rounded_column.is_finite() {
                return Err(CoreError::NonFinite(
                    "Leja output scaling produced NaN/Inf".into(),
                ));
            }
            columns[k][i] = rounded_column;
            fused[i] += rounded_column;
            let exact_column =
                Interval::new(sums_lo[k][i], sums_hi[k][i])?.mul(Interval::point(scale)?)?;
            let exact_fused = Interval::new(out_lo[i], out_hi[i])?.add(exact_column)?;
            out_lo[i] = exact_fused.lo;
            out_hi[i] = exact_fused.hi;
        }
    }
    for i in 0..n {
        *ops = ops.saturating_add(4);
        if !fused[i].is_finite() {
            return Err(CoreError::NonFinite(
                "Leja output fusion produced NaN/Inf".into(),
            ));
        }
        defects[i] = interval_distance_upper(fused[i], Interval::new(out_lo[i], out_hi[i])?)?;
    }
    certificate_l2_upper(&defects[..n], ops)
}

/// Allocation-free directed coefficients `phi_k(c)` for the zero-width
/// scalar branch. This computes only the one-node table required by that
/// branch; it deliberately does not grant total-error authority.
#[cfg(test)]
fn scalar_phi_coefficients(c: f64) -> CoreResult<([f64; JOINT_PHI_TERMS], [f64; JOINT_PHI_TERMS])> {
    scalar_phi_coefficients_counted(c, None)
}

fn scalar_phi_coefficients_counted(
    c: f64,
    mut work: Option<&mut LejaSetupWork>,
) -> CoreResult<([f64; JOINT_PHI_TERMS], [f64; JOINT_PHI_TERMS])> {
    let zero = setup_kernel!(work, Interval::point(0.0))?;
    let mut intervals = [zero; JOINT_PHI_TERMS];
    if c == 0.0 {
        intervals = [
            setup_kernel!(work, Interval::point(1.0))?,
            setup_kernel!(work, Interval::point(1.0))?,
            setup_kernel!(work, Interval::point(0.5))?,
            {
                let lo = setup_kernel!(work, div_down(1.0, 6.0))?;
                let hi = setup_kernel!(work, div_up(1.0, 6.0))?;
                setup_kernel!(work, Interval::new(lo, hi))?
            },
            {
                let lo = setup_kernel!(work, div_down(1.0, 24.0))?;
                let hi = setup_kernel!(work, div_up(1.0, 24.0))?;
                setup_kernel!(work, Interval::new(lo, hi))?
            },
        ];
    } else {
        intervals[0] = setup_kernel!(work, exp_interval(c))?;
        if c.abs() <= 1.0 {
            let factorials = [1.0, 1.0, 2.0, 6.0, 24.0];
            let ci = setup_kernel!(work, Interval::point(c))?;
            for k in 1..JOINT_PHI_TERMS {
                let reciprocal_lo = setup_kernel!(work, div_down(1.0, factorials[k]))?;
                let reciprocal_hi = setup_kernel!(work, div_up(1.0, factorials[k]))?;
                let mut term = setup_kernel!(work, Interval::new(reciprocal_lo, reciprocal_hi))?;
                let mut sum = term;
                for order in 1..=64 {
                    term = setup_kernel!(work, term.mul(ci))?;
                    let divisor = setup_kernel!(work, Interval::point((order + k) as f64))?;
                    term = setup_kernel!(work, term.div(divisor))?;
                    sum = setup_kernel!(work, sum.add(term))?;
                }
                let next = setup_kernel!(work, term.mul(ci))?;
                let divisor = setup_kernel!(work, Interval::point((65 + k) as f64))?;
                let next = setup_kernel!(work, next.div(divisor))?;
                let ratio = setup_kernel!(work, div_up(c.abs(), (66 + k) as f64))?;
                let one_minus_ratio = setup_kernel!(work, sub_down(1.0, ratio))?;
                let tail = setup_kernel!(work, div_up(next.mag(), one_minus_ratio))?;
                let lo = setup_kernel!(work, sub_down(sum.lo, tail))?;
                let hi = setup_kernel!(work, add_up(sum.hi, tail))?;
                intervals[k] = setup_kernel!(work, Interval::new(lo, hi))?;
            }
        } else {
            let factorials = [1.0, 1.0, 2.0, 6.0];
            let ci = setup_kernel!(work, Interval::point(c))?;
            for k in 1..JOINT_PHI_TERMS {
                let f = factorials[k - 1];
                let reciprocal_lo = setup_kernel!(work, div_down(1.0, f))?;
                let reciprocal_hi = setup_kernel!(work, div_up(1.0, f))?;
                let reciprocal = setup_kernel!(work, Interval::new(reciprocal_lo, reciprocal_hi))?;
                let difference = setup_kernel!(work, intervals[k - 1].sub(reciprocal))?;
                intervals[k] = setup_kernel!(work, difference.div(ci))?;
            }
        }
    }
    let mut values = [0.0; JOINT_PHI_TERMS];
    let mut radii = [0.0; JOINT_PHI_TERMS];
    for k in 0..JOINT_PHI_TERMS {
        let iv = intervals[k];
        // For c <= 0, all phi_k(c) are nonnegative; this midpoint expression
        // avoids an overflowing endpoint sum.
        let mid = iv.lo * 0.5 + iv.hi * 0.5;
        values[k] = mid;
        let upper_radius = setup_kernel!(work, sub_up(iv.hi, mid))?;
        let lower_radius = setup_kernel!(work, sub_up(mid, iv.lo))?;
        radii[k] = upper_radius.max(lower_radius);
    }
    Ok((values, radii))
}

fn update_leja_total_status(
    status: &mut TotalErrorStatus,
    converged: bool,
    degree_cap: usize,
    total_budget: f64,
    total_bound: f64,
) {
    if !matches!(status, TotalErrorStatus::EstimateOnly { .. }) {
        *status = TotalErrorStatus::EstimateOnly {
            reason: String::with_capacity(256),
            bounded_components: total_bound,
        };
    }
    if let TotalErrorStatus::EstimateOnly {
        reason,
        bounded_components,
    } = status
    {
        reason.clear();
        if !converged {
            write!(reason, "{LEJA_TOTAL_NOT_CERTIFIED}: degree cap {degree_cap} reached without meeting budget {total_budget:e}; ")
                .expect("writing to String cannot fail");
        }
        reason.push_str(LEJA_ARITHMETIC_HOLD);
        *bounded_components = total_bound;
    }
}

/// Computes an outward interval upper bound on `max_{xi in [-2, 2]} prod_{j=0}^m |xi - xi_j|`
/// by evaluating over uniform subintervals of `[-2, 2]` using directed interval arithmetic.
///
/// Guaranteed zero heap allocations on the evaluation path.
pub fn leja_node_polynomial_bound(m: usize, xi: &[f64]) -> f64 {
    leja_node_polynomial_bound_counted(m, xi, None)
}

fn leja_node_polynomial_bound_counted(
    m: usize,
    xi: &[f64],
    mut work: Option<&mut LejaSetupWork>,
) -> f64 {
    if m >= xi.len() {
        return f64::INFINITY;
    }
    const SUBDIVISIONS: usize = 128;
    const H_SUB: f64 = 4.0 / (SUBDIVISIONS as f64);
    let mut max_val = 0.0_f64;

    for p in 0..SUBDIVISIONS {
        if let Some(work) = work.as_deref_mut() {
            work.scalar_proxy_units(7); // endpoint arithmetic and max/magnitude comparisons
            work.interval_kernel_invocation(); // Interval::point(1)
        }
        let lo = -2.0 + (p as f64) * H_SUB;
        let hi = -2.0 + ((p + 1) as f64) * H_SUB;
        if let Some(work) = work.as_deref_mut() {
            // Count the interval construction separately from the initial
            // point interval and the factor interval below.
            work.interval_kernel_invocation();
        }
        let int_p = match Interval::new(lo, hi) {
            Ok(int) => int,
            Err(_) => return f64::INFINITY,
        };

        let mut acc = match Interval::point(1.0) {
            Ok(p) => p,
            Err(_) => return f64::INFINITY,
        };
        for &node in &xi[..=m] {
            if let Some(work) = work.as_deref_mut() {
                work.interval_kernel_invocation(); // sub_down
            }
            let diff_lo = match sub_down(int_p.lo, node) {
                Ok(v) => v,
                Err(_) => return f64::INFINITY,
            };
            if let Some(work) = work.as_deref_mut() {
                work.interval_kernel_invocation(); // sub_up
            }
            let diff_hi = match sub_up(int_p.hi, node) {
                Ok(v) => v,
                Err(_) => return f64::INFINITY,
            };
            if let Some(work) = work.as_deref_mut() {
                work.interval_kernel_invocation(); // interval construction
            }
            let diff = match Interval::new(diff_lo, diff_hi) {
                Ok(int) => int,
                Err(_) => return f64::INFINITY,
            };
            if let Some(work) = work.as_deref_mut() {
                work.interval_kernel_invocation(); // interval multiplication
            }
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
    mut work: Option<&mut LejaSetupWork>,
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
        let row_sum = setup_kernel!(work, add_up(diagonal_hi[i], subdiagonal_hi[i - 1]))?;
        if row_sum > max_entry {
            max_entry = row_sum;
        }
    }
    let min_terms = (size as f64).max(setup_kernel!(work, mul_up(2.0, max_entry))?);
    if let Some(work) = work.as_deref_mut() {
        work.allocation_request((ZERO_NODES + 1) as u64);
        work.allocation_request((ZERO_NODES + 1) as u64);
    }
    let mut columns_lo = Vec::with_capacity(ZERO_NODES + 1);
    let mut columns_hi = Vec::with_capacity(ZERO_NODES + 1);

    for col in 0..=ZERO_NODES.min(size - 1) {
        if let Some(work) = work.as_deref_mut() {
            for _ in 0..4 {
                work.allocation_request(size as u64);
            }
        }
        let mut term_lo = vec![0.0; size];
        let mut term_hi = vec![0.0; size];
        term_lo[col] = 1.0;
        term_hi[col] = 1.0;
        let mut sum_lo = term_lo.clone();
        let mut sum_hi = term_hi.clone();
        let mut converged = false;

        for n in 1..=MAX_TAYLOR_TERMS {
            let inv_lo = setup_kernel!(work, div_down(1.0, n as f64))?;
            let inv_hi = setup_kernel!(work, div_up(1.0, n as f64))?;
            for i in (col..size).rev() {
                let below_lo = if i > col {
                    setup_kernel!(work, mul_down(subdiagonal_lo[i - 1], term_lo[i - 1]))?
                } else {
                    0.0
                };
                let below_hi = if i > col {
                    setup_kernel!(work, mul_up(subdiagonal_hi[i - 1], term_hi[i - 1]))?
                } else {
                    0.0
                };
                let diagonal_term_lo = setup_kernel!(work, mul_down(diagonal_lo[i], term_lo[i]))?;
                let numerator_lo = setup_kernel!(work, add_down(diagonal_term_lo, below_lo))?;
                term_lo[i] = setup_kernel!(work, mul_down(numerator_lo, inv_lo))?;
                let diagonal_term_hi = setup_kernel!(work, mul_up(diagonal_hi[i], term_hi[i]))?;
                let numerator_hi = setup_kernel!(work, add_up(diagonal_term_hi, below_hi))?;
                term_hi[i] = setup_kernel!(work, mul_up(numerator_hi, inv_hi))?;
            }
            let mut small = true;
            for i in col..size {
                sum_lo[i] = setup_kernel!(work, add_down(sum_lo[i], term_lo[i]))?;
                sum_hi[i] = setup_kernel!(work, add_up(sum_hi[i], term_hi[i]))?;
                if term_hi[i] > setup_kernel!(work, mul_up(TAYLOR_RELATIVE_TAIL, sum_hi[i]))? {
                    small = false;
                }
            }
            if !sum_hi.iter().all(|x| x.is_finite()) {
                return Err(CoreError::NonFinite(
                    "Leja divided-difference series left the binary64 range".into(),
                ));
            }
            if small && n as f64 > min_terms {
                let ratio = setup_kernel!(work, div_up(max_entry, (n + 1) as f64))?;
                if ratio < 1.0 {
                    let one_minus_ratio = setup_kernel!(work, sub_down(1.0, ratio))?;
                    let geom = setup_kernel!(work, div_up(ratio, one_minus_ratio))?;
                    let t_inf = term_hi[col..size].iter().copied().fold(0.0_f64, f64::max);
                    let tail = setup_kernel!(work, mul_up(t_inf, geom))?;
                    for i in col..size {
                        sum_hi[i] = setup_kernel!(work, add_up(sum_hi[i], tail))?;
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
    leja_divided_differences_certified_counted(c, gamma, xi, None)
}

#[allow(clippy::type_complexity)]
fn leja_divided_differences_certified_counted(
    c: f64,
    gamma: f64,
    xi: &[f64],
    mut work: Option<&mut LejaSetupWork>,
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
    let two_gamma_hi = setup_kernel!(work, mul_up(2.0, gamma))?;
    let shift_lo = setup_kernel!(work, sub_down(c, two_gamma_hi))?;
    let two_gamma_lo = setup_kernel!(work, mul_down(2.0, gamma))?;
    let shift_hi = setup_kernel!(work, sub_up(c, two_gamma_lo))?;
    if shift_lo < -LEJA_RANGE_LIMIT - 1.0 {
        return Err(unsupported(format!(
            "interval left end {shift_lo:e} beyond -{LEJA_RANGE_LIMIT}"
        )));
    }
    let exp_s_lo = setup_kernel!(work, exp_interval(shift_lo))?.lo;
    let mut exp_s_hi = setup_kernel!(work, exp_interval(shift_hi))?.hi;
    if exp_s_hi <= 0.0 {
        exp_s_hi = 0.0_f64.next_up();
    }
    let exp_s = setup_kernel!(work, Interval::new(exp_s_lo, exp_s_hi))?;

    let zero_node_lo = (-shift_hi).max(0.0);
    let zero_node_hi = (-shift_lo).max(0.0);
    let size = ZERO_NODES + xi.len();
    if let Some(work) = work.as_deref_mut() {
        // Two diagonal and two subdiagonal allocations happen before the
        // exponential. Result vectors are charged only after it succeeds.
        for items in [
            size as u64,
            size as u64,
            (size - 1) as u64,
            (size - 1) as u64,
        ] {
            work.allocation_request(items);
        }
    }
    let mut diagonal_lo = Vec::with_capacity(size);
    diagonal_lo.resize(ZERO_NODES, zero_node_lo);
    let mut diagonal_hi = Vec::with_capacity(size);
    diagonal_hi.resize(ZERO_NODES, zero_node_hi);
    for &x in xi {
        let x2_lo = setup_kernel!(work, add_down(x, 2.0))?;
        let x2_hi = setup_kernel!(work, add_up(x, 2.0))?;
        diagonal_lo.push(setup_kernel!(work, mul_down(gamma, x2_lo))?.max(0.0));
        diagonal_hi.push(setup_kernel!(work, mul_up(gamma, x2_hi))?);
    }
    let mut subdiagonal_lo = Vec::with_capacity(size - 1);
    subdiagonal_lo.resize(ZERO_NODES, 1.0);
    subdiagonal_lo.extend(std::iter::repeat_n(gamma, xi.len() - 1));
    let mut subdiagonal_hi = Vec::with_capacity(size - 1);
    subdiagonal_hi.resize(ZERO_NODES, 1.0);
    subdiagonal_hi.extend(std::iter::repeat_n(gamma, xi.len() - 1));

    let (cols_lo, cols_hi) = exp_bidiagonal_columns_certified(
        &diagonal_lo,
        &diagonal_hi,
        &subdiagonal_lo,
        &subdiagonal_hi,
        work.as_deref_mut(),
    )?;

    if let Some(work) = work.as_deref_mut() {
        work.allocation_request(xi.len() as u64);
        work.allocation_request(xi.len() as u64);
    }
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
            let val_lo = setup_kernel!(work, mul_down(lo_entry, exp_s.lo))?.max(0.0);
            let val_hi = setup_kernel!(work, mul_up(hi_entry, exp_s.hi))?;
            let mid = 0.5 * (val_lo + val_hi);
            let dist_hi = setup_kernel!(work, sub_up(val_hi, mid))?;
            let dist_lo = setup_kernel!(work, sub_up(mid, val_lo))?;
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
    setup_receipt: Option<LejaSetupWorkSubtotal>,
    setup_coverage: SetupCoverage,
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

    /// Compute a trusted coefficient entry with this module's outward engine.
    /// Caller-supplied coefficient/radius arrays cannot enter the cache.
    ///
    /// ```compile_fail
    /// use rodas5p_core::leja_action::CertifiedLejaCoeffCache;
    /// let mut cache = CertifiedLejaCoeffCache::new();
    /// cache.insert_certified(-1.5, 0.25, 16, vec![[0.0; 5]; 17], vec![[0.0; 5]; 17]);
    /// ```
    pub fn precompute_certified(
        &mut self,
        center: f64,
        scale: f64,
        degree_cap: usize,
    ) -> CoreResult<()> {
        self.precompute_certified_inner(center, scale, degree_cap, None, None)
            .map(|_| ())
    }

    /// Trusted coefficient-table preparation with a separate immutable setup
    /// receipt. Failed calls retain the work attempted up to the refusal.
    pub fn precompute_certified_with_setup_work(
        &mut self,
        center: f64,
        scale: f64,
        degree_cap: usize,
        work: &mut LejaSetupWork,
    ) -> CoreResult<()> {
        self.precompute_certified_inner(center, scale, degree_cap, Some(work), None)
            .map(|_| ())
    }

    fn precompute_certified_inner(
        &mut self,
        center: f64,
        scale: f64,
        degree_cap: usize,
        mut work: Option<&mut LejaSetupWork>,
        coefficient_setup_attempts: Option<&mut u64>,
    ) -> CoreResult<bool> {
        if let Some(work) = work.as_deref_mut() {
            work.cache_api_attempt();
        }
        if !(1..=LEJA_DEGREE_CAP).contains(&degree_cap) {
            return Err(unsupported(
                "certified cache degree outside supported range",
            ));
        }
        let key = (center.to_bits(), scale.to_bits(), degree_cap);
        let cached_entry = self.entries.get(&key);
        let cached = cached_entry.is_some();
        if let Some(work) = work.as_deref_mut() {
            work.cache_lookup(cached);
            if let Some(entry) = cached_entry {
                work.record_coefficient_cache_entry(
                    [key.0, key.1, key.2 as u64],
                    false,
                    entry.setup_receipt,
                    Some(entry.setup_coverage),
                );
            }
        }
        if !cached {
            let setup_start = work.as_deref().map(LejaSetupWork::current_subtotal);
            if let Some(attempts) = coefficient_setup_attempts {
                *attempts = attempts.saturating_add(1);
            }
            if let Some(work) = work.as_deref_mut() {
                work.coefficient_table_attempt();
            }
            let xi = leja_points_with_setup(work.as_deref_mut());
            let (d, d_rad) = leja_divided_differences_certified_counted(
                center,
                scale,
                &xi[..=degree_cap],
                work.as_deref_mut(),
            )?;
            if let Some(work) = work.as_deref_mut() {
                work.coefficient_table_complete();
                work.cache_insert_attempt();
                if self.entries.len() == self.entries.capacity() {
                    work.allocation_request(1);
                }
            }
            self.entries
                .try_reserve(1)
                .map_err(|err| unsupported(format!("certified cache allocation refused: {err}")))?;
            if let Some(work) = work.as_deref_mut() {
                work.cache_insert_complete();
            }
            let setup_receipt = match (work.as_deref(), setup_start) {
                (Some(work), Some(start)) => work.delta_since(start),
                _ => None,
            };
            let setup_coverage = work
                .as_deref()
                .map(LejaSetupWork::current_coverage)
                .unwrap_or(SetupCoverage::PartialUnattributed);
            if let Some(work) = work {
                work.record_coefficient_cache_entry(
                    [key.0, key.1, key.2 as u64],
                    true,
                    setup_receipt,
                    Some(setup_coverage),
                );
            }
            self.insert_certified_with_setup_receipt(
                center,
                scale,
                degree_cap,
                d,
                d_rad,
                setup_receipt,
                setup_coverage,
            );
            return Ok(true);
        }
        Ok(false)
    }

    fn insert_certified(
        &mut self,
        center: f64,
        scale: f64,
        degree_cap: usize,
        d: Vec<[f64; JOINT_PHI_TERMS]>,
        d_rad: Vec<[f64; JOINT_PHI_TERMS]>,
    ) {
        let key = (center.to_bits(), scale.to_bits(), degree_cap);
        self.entries.insert(
            key,
            CachedCertifiedLeja {
                d,
                d_rad,
                setup_receipt: None,
                setup_coverage: SetupCoverage::PartialUnattributed,
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_certified_with_setup_receipt(
        &mut self,
        center: f64,
        scale: f64,
        degree_cap: usize,
        d: Vec<[f64; JOINT_PHI_TERMS]>,
        d_rad: Vec<[f64; JOINT_PHI_TERMS]>,
        setup_receipt: Option<LejaSetupWorkSubtotal>,
        setup_coverage: SetupCoverage,
    ) {
        let key = (center.to_bits(), scale.to_bits(), degree_cap);
        self.entries.insert(
            key,
            CachedCertifiedLeja {
                d,
                d_rad,
                setup_receipt,
                setup_coverage,
            },
        );
    }
}

/// Newton-Leja diagnostic report retaining the historical API name.
/// `total_error()` is EstimateOnly while the total arithmetic proof is on hold;
/// convergence means the computed component sum met the budget.
///
/// The report is immutable through the public API so its payload cannot become
/// detached from a future total-error authority status. Serialization retains
/// the historical field names and values.
///
/// ```compile_fail
/// use rodas5p_core::leja_action::CertifiedLejaReport;
/// fn mutate_payload(report: &mut CertifiedLejaReport) {
///     report.total_bound = 0.0;
///     report.fused[0] = 0.0;
/// }
/// ```
///
/// ```compile_fail
/// use rodas5p_core::leja_action::CertifiedLejaReport;
/// fn mutate_status(report: &mut CertifiedLejaReport) {
///     report.total_error = unimplemented!();
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CertifiedLejaReport {
    schema: String,
    input: String,
    branch: String,
    dimension: usize,
    degree: usize,
    degree_cap: usize,
    converged: bool,
    interval: [f64; 2],
    center: f64,
    scale: f64,
    total_budget: f64,
    total_bound: f64,
    truncation_bound: f64,
    divided_difference_bound: f64,
    recurrence_bound: f64,
    summation_bound: f64,
    columns: Vec<Vec<f64>>,
    fused: Vec<f64>,
    total_error: TotalErrorStatus,
    evidence: EnclosureEvidence,
    vector_products: u64,
    block_products: u64,
    coefficient_setups: u64,
    certification_ops: u64,
}

impl CertifiedLejaReport {
    pub fn schema(&self) -> &str {
        &self.schema
    }
    pub fn input(&self) -> &str {
        &self.input
    }
    pub fn branch(&self) -> &str {
        &self.branch
    }
    pub const fn dimension(&self) -> usize {
        self.dimension
    }
    pub const fn degree(&self) -> usize {
        self.degree
    }
    pub const fn degree_cap(&self) -> usize {
        self.degree_cap
    }
    pub const fn converged(&self) -> bool {
        self.converged
    }
    pub const fn interval(&self) -> [f64; 2] {
        self.interval
    }
    pub const fn center(&self) -> f64 {
        self.center
    }
    pub const fn scale(&self) -> f64 {
        self.scale
    }
    pub const fn total_budget(&self) -> f64 {
        self.total_budget
    }
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
    pub fn columns(&self) -> &[Vec<f64>] {
        &self.columns
    }
    pub fn fused(&self) -> &[f64] {
        &self.fused
    }
    pub fn evidence(&self) -> &EnclosureEvidence {
        &self.evidence
    }
    pub const fn vector_products(&self) -> u64 {
        self.vector_products
    }
    pub const fn block_products(&self) -> u64 {
        self.block_products
    }
    pub const fn coefficient_setups(&self) -> u64 {
        self.coefficient_setups
    }
    pub const fn certification_ops(&self) -> u64 {
        self.certification_ops
    }
}

/// Pre-allocated workspace for certified Leja phi action.
///
/// Ensures zero heap allocations in the degree/recurrence inner loops.
/// Its last successful report is exposed through an immutable getter only.
///
/// ```compile_fail
/// use rodas5p_core::leja_action::CertifiedLejaWorkspace;
/// fn replace_report(workspace: &mut CertifiedLejaWorkspace) {
///     workspace.report = None;
/// }
/// ```
#[derive(Clone, Debug)]
pub struct CertifiedLejaWorkspace {
    pub current: Vec<Vec<f64>>,
    pub product: Vec<Vec<f64>>,
    product_lo: Vec<Vec<f64>>,
    product_hi: Vec<Vec<f64>>,
    pub sums: Vec<Vec<f64>>,
    sums_lo: Vec<Vec<f64>>,
    sums_hi: Vec<Vec<f64>>,
    pub columns: Vec<Vec<f64>>,
    pub fused: Vec<f64>,
    output_lo: Vec<f64>,
    output_hi: Vec<f64>,
    local_defect: Vec<f64>,
    pub r_norms: [[f64; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP + 1],
    pub residuals: [[f64; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP],
    report: Option<CertifiedLejaReport>,
    last_certification_ops: u64,
}

impl Default for CertifiedLejaWorkspace {
    fn default() -> Self {
        Self {
            current: Vec::new(),
            product: Vec::new(),
            product_lo: Vec::new(),
            product_hi: Vec::new(),
            sums: Vec::new(),
            sums_lo: Vec::new(),
            sums_hi: Vec::new(),
            columns: Vec::new(),
            fused: Vec::new(),
            output_lo: Vec::new(),
            output_hi: Vec::new(),
            local_defect: Vec::new(),
            r_norms: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP + 1],
            residuals: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP],
            report: None,
            last_certification_ops: 0,
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
            total_error: TotalErrorStatus::EstimateOnly {
                reason: LEJA_ARITHMETIC_HOLD.into(),
                bounded_components: 0.0,
            },
            evidence: EnclosureEvidence::Gershgorin,
            vector_products: 0,
            block_products: 0,
            coefficient_setups: 0,
            certification_ops: 0,
        };
        Self {
            current: vec![vec![0.0; dimension]; width],
            product: vec![vec![0.0; dimension]; width],
            product_lo: vec![vec![0.0; dimension]; width],
            product_hi: vec![vec![0.0; dimension]; width],
            sums: vec![vec![0.0; dimension]; JOINT_PHI_TERMS],
            sums_lo: vec![vec![0.0; dimension]; JOINT_PHI_TERMS],
            sums_hi: vec![vec![0.0; dimension]; JOINT_PHI_TERMS],
            columns,
            fused,
            output_lo: vec![0.0; dimension],
            output_hi: vec![0.0; dimension],
            local_defect: vec![0.0; dimension],
            r_norms: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP + 1],
            residuals: [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP],
            report: Some(report),
            last_certification_ops: 0,
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
        for buffers in [&mut self.product_lo, &mut self.product_hi] {
            if buffers.len() < width {
                buffers.resize_with(width, Vec::new);
            }
            for v in buffers.iter_mut().take(width) {
                if v.len() < dimension {
                    v.resize(dimension, 0.0);
                }
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
        for buffers in [&mut self.sums_lo, &mut self.sums_hi] {
            if buffers.len() < JOINT_PHI_TERMS {
                buffers.resize_with(JOINT_PHI_TERMS, Vec::new);
            }
            for v in buffers.iter_mut() {
                if v.len() < dimension {
                    v.resize(dimension, 0.0);
                }
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
        for buffer in [
            &mut self.output_lo,
            &mut self.output_hi,
            &mut self.local_defect,
        ] {
            if buffer.len() < dimension {
                buffer.resize(dimension, 0.0);
            }
        }
    }

    /// Arithmetic-accounting proxy for the most recent action attempt,
    /// including attempts refused after certificate work has begun.
    pub fn last_certification_ops(&self) -> u64 {
        self.last_certification_ops
    }

    /// Returns the last successfully committed report without mutable access.
    pub fn report(&self) -> Option<&CertifiedLejaReport> {
        self.report.as_ref()
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

/// Leja candidate with component bounds and cap in `1 ..= LEJA_DEGREE_CAP`.
/// The returned total remains EstimateOnly under [`LEJA_ARITHMETIC_HOLD`].
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

/// Leja candidate with an internally verified coefficient cache.
/// The returned total remains EstimateOnly under [`LEJA_ARITHMETIC_HOLD`].
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

/// Leja candidate with preallocated workspace and optional coefficient cache.
/// The returned total remains EstimateOnly under [`LEJA_ARITHMETIC_HOLD`].
///
/// The preallocated route can be allocation-free after warm-up when dimensions,
/// degree cap, cache state, and report-string capacity are unchanged and the
/// call succeeds. A changed convergence reason can grow the report `String`;
/// higher-level wrappers allocate a workspace and clone the report.
#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
pub fn leja_phi_action_certified_with_cache_and_workspace<'a>(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    degree_cap: usize,
    cache: Option<&mut CertifiedLejaCoeffCache>,
    work: &mut WorkCounters,
    workspace: &'a mut CertifiedLejaWorkspace,
) -> CoreResult<&'a CertifiedLejaReport> {
    leja_phi_action_certified_with_cache_and_workspace_inner(
        op,
        h,
        input,
        total_budget,
        degree_cap,
        cache,
        work,
        workspace,
        None,
    )
}

/// Same preallocated candidate path while recording cold table and coefficient
/// setup in a separate v1 receipt accumulator. Historical `WorkCounters` and
/// report serialization remain unchanged.
#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
pub fn leja_phi_action_certified_with_setup_work<'a>(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    degree_cap: usize,
    cache: Option<&mut CertifiedLejaCoeffCache>,
    work: &mut WorkCounters,
    workspace: &'a mut CertifiedLejaWorkspace,
    setup_work: &mut LejaSetupWork,
) -> CoreResult<&'a CertifiedLejaReport> {
    leja_phi_action_certified_with_cache_and_workspace_inner(
        op,
        h,
        input,
        total_budget,
        degree_cap,
        cache,
        work,
        workspace,
        Some(setup_work),
    )
}

#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
fn leja_phi_action_certified_with_cache_and_workspace_inner<'a>(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    degree_cap: usize,
    mut cache: Option<&mut CertifiedLejaCoeffCache>,
    work: &mut WorkCounters,
    workspace: &'a mut CertifiedLejaWorkspace,
    mut setup_work: Option<&mut LejaSetupWork>,
) -> CoreResult<&'a CertifiedLejaReport> {
    workspace.last_certification_ops = 0;
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
    for sum in workspace.sums_lo[..JOINT_PHI_TERMS].iter_mut() {
        sum[..n].fill(0.0);
    }
    for sum in workspace.sums_hi[..JOINT_PHI_TERMS].iter_mut() {
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
            if let Some(setup) = setup_work.as_deref_mut() {
                setup.coefficient_table_attempt();
            }
            let (d, d_rad) = scalar_phi_coefficients_counted(center, setup_work.as_deref_mut())?;
            if let Some(setup) = setup_work.as_deref_mut() {
                setup.coefficient_table_complete();
            }
            for (k, sum) in workspace.sums[..JOINT_PHI_TERMS].iter_mut().enumerate() {
                let col = column_of(k);
                let w = &workspace.current[col][..n];
                accumulate_coefficient_term(
                    &mut sum[..n],
                    &mut workspace.sums_lo[k][..n],
                    &mut workspace.sums_hi[k][..n],
                    d[k],
                    w,
                    &mut certification_ops,
                )?;
            }
            let mut diff_terms = [0.0; JOINT_PHI_TERMS];
            let interval_width = sub_up(b, a)?;
            let support_hi = add_up(center, mul_up(2.0, scale)?)?;
            let derivative_exp = exp_interval(verified_interval[1].max(support_hi).max(0.0))?.hi;
            certification_ops = certification_ops.saturating_add(1);

            for k in 0..JOINT_PHI_TERMS {
                let col = column_of(k);
                let w_norm =
                    certificate_l2_upper(&workspace.current[col][..n], &mut certification_ops)?;
                let d_err = mul_up(d_rad[k], w_norm)?;
                let fact = match k {
                    0 => 1.0,
                    1 => 2.0,
                    2 => 6.0,
                    3 => 24.0,
                    _ => 120.0,
                };
                let width_err = mul_up(
                    derivative_exp,
                    mul_up(div_up(interval_width, fact)?, w_norm)?,
                )?;
                let total_d_err = add_up(d_err, width_err)?;
                diff_terms[k] = mul_up(scales[k].abs(), total_d_err)?;
                certification_ops = certification_ops.saturating_add(7);
            }
            divided_difference_bound = sum_up_slice(&diff_terms)?;
            summation_bound = replay_scaled_fusion(
                &scales,
                &workspace.sums,
                &workspace.sums_lo,
                &workspace.sums_hi,
                &mut workspace.columns,
                &mut workspace.fused,
                &mut workspace.output_lo,
                &mut workspace.output_hi,
                &mut workspace.local_defect,
                n,
                &mut certification_ops,
            )?;
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
            let (normalized_scale, _normalized_support) =
                enlarge_scale_for_normalized_support(a, b, center, scale, &mut certification_ops)?;
            if normalized_scale != scale {
                scale = normalized_scale;
                verified_interval = [
                    sub_down(center, mul_up(2.0, scale)?)?,
                    add_up(center, mul_up(2.0, scale)?)?,
                ];
            }
            let support_hi = add_up(center, mul_up(2.0, scale)?)?;
            verified_interval[1] = verified_interval[1].max(support_hi);
            let derivative_exp = exp_interval(verified_interval[1].max(0.0))?.hi;
            branch = "newton";
            let xi = &leja_points_with_setup(setup_work.as_deref_mut())[..=degree_cap];
            let node_bounds = leja_node_bounds_with_setup(setup_work.as_deref_mut());
            let suffix_bounds = leja_suffix_bounds_with_setup(setup_work.as_deref_mut());
            let mut tail_bounds = [[0.0; JOINT_PHI_TERMS]; LEJA_DEGREE_CAP];

            let (d_calc, dr_calc);
            let (d, d_rad): (&[[f64; JOINT_PHI_TERMS]], &[[f64; JOINT_PHI_TERMS]]) =
                if let Some(ref mut c) = cache {
                    if let Some(setup) = setup_work.as_deref_mut() {
                        c.precompute_certified_inner(
                            center,
                            scale,
                            degree_cap,
                            Some(setup),
                            Some(&mut local_coefficient_setups),
                        )?;
                        setup.cache_lookup(true); // coefficient-slice retrieval below
                    } else if c.get_certified(center, scale, degree_cap).is_none() {
                        local_coefficient_setups += 1;
                        let (d_val, dr_val) =
                            leja_divided_differences_certified(center, scale, xi)?;
                        c.insert_certified(center, scale, degree_cap, d_val, dr_val);
                    }
                    c.get_certified(center, scale, degree_cap).unwrap()
                } else {
                    local_coefficient_setups += 1;
                    if let Some(setup) = setup_work.as_deref_mut() {
                        setup.coefficient_table_attempt();
                    }
                    let res = leja_divided_differences_certified_counted(
                        center,
                        scale,
                        xi,
                        setup_work.as_deref_mut(),
                    )?;
                    if let Some(setup) = setup_work.as_deref_mut() {
                        setup.coefficient_table_complete();
                    }
                    d_calc = res.0;
                    dr_calc = res.1;
                    (&d_calc, &dr_calc)
                };

            for col in 0..width {
                workspace.r_norms[0][col] =
                    certificate_l2_upper(&workspace.current[col][..n], &mut certification_ops)?;
            }

            for (k, sum) in workspace.sums[..JOINT_PHI_TERMS].iter_mut().enumerate() {
                let col = column_of(k);
                let coeff = d[0][k];
                let cur = &workspace.current[col][..n];
                accumulate_coefficient_term(
                    &mut sum[..n],
                    &mut workspace.sums_lo[k][..n],
                    &mut workspace.sums_hi[k][..n],
                    coeff,
                    cur,
                    &mut certification_ops,
                )?;
            }

            for j in 1..=degree_cap {
                local_block_products += 1;
                for col in 0..width {
                    local_vector_products += 1;
                    op.matrix().matvec_into(
                        &workspace.current[col][..n],
                        &mut workspace.product[col][..n],
                    )?;
                    matvec_interval_into(
                        op.matrix(),
                        &workspace.current[col][..n],
                        &mut workspace.product_lo[col][..n],
                        &mut workspace.product_hi[col][..n],
                        &mut certification_ops,
                    )?;
                }

                let node = xi[j - 1];
                for col in 0..width {
                    let cur = &mut workspace.current[col][..n];
                    let prod = &workspace.product[col][..n];
                    for i in 0..n {
                        let old = cur[i];
                        cur[i] = (h * prod[i] - center * old) / scale - node * old;
                        let exact_next = recurrence_interval_next(
                            h,
                            Interval::new(
                                workspace.product_lo[col][i],
                                workspace.product_hi[col][i],
                            )?,
                            center,
                            old,
                            scale,
                            node,
                            &mut certification_ops,
                        )?;
                        workspace.local_defect[i] = interval_distance_upper(cur[i], exact_next)?;
                    }
                    workspace.residuals[j - 1][col] =
                        certificate_l2_upper(&workspace.local_defect[..n], &mut certification_ops)?;
                }

                for col in 0..width {
                    if !workspace.current[col][..n].iter().all(|x| x.is_finite()) {
                        return Err(CoreError::NonFinite(
                            "Newton-Leja recurrence produced NaN/Inf".into(),
                        ));
                    }
                }

                for col in 0..width {
                    let r_norm =
                        certificate_l2_upper(&workspace.current[col][..n], &mut certification_ops)?;
                    workspace.r_norms[j][col] = r_norm;
                }

                for (k, sum) in workspace.sums[..JOINT_PHI_TERMS].iter_mut().enumerate() {
                    let col = column_of(k);
                    let coeff = d[j][k];
                    let cur = &workspace.current[col][..n];
                    accumulate_coefficient_term(
                        &mut sum[..n],
                        &mut workspace.sums_lo[k][..n],
                        &mut workspace.sums_hi[k][..n],
                        coeff,
                        cur,
                        &mut certification_ops,
                    )?;
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
                        mul_up(
                            derivative_exp,
                            mul_up(workspace.r_norms[0][col], mul_up(m_val, coeff_k)?)?,
                        )?,
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
                // Bound each correctly indexed suffix over [-2, 2]. Incremental
                // accumulation costs O(m^2) coefficient updates through degree m,
                // rather than recomputing all earlier terms in O(m^3).
                let mut t3_terms = [0.0_f64; JOINT_PHI_TERMS];
                for k in 0..JOINT_PHI_TERMS {
                    let col = column_of(k);
                    let mut col_sum = 0.0_f64;
                    for step_i in 0..j {
                        tail_bounds[step_i][k] = add_up(
                            tail_bounds[step_i][k],
                            mul_up(
                                add_up(d[j][k].abs(), d_rad[j][k])?,
                                suffix_bounds[step_i + 1][j],
                            )?,
                        )?;
                        let t_poly = tail_bounds[step_i][k];
                        certification_ops = certification_ops.saturating_add(3);
                        col_sum =
                            add_up(col_sum, mul_up(t_poly, workspace.residuals[step_i][col])?)?;
                        certification_ops = certification_ops.saturating_add(2);
                    }
                    t3_terms[k] = mul_up(scales[k].abs(), col_sum)?;
                    certification_ops = certification_ops.saturating_add(1);
                }
                let cur_t3 = sum_up_slice(&t3_terms)?;
                certification_ops = certification_ops.saturating_add(4);

                // Outward replay covers coefficient products/sums, column
                // scaling and final fusion without a relative-only model.
                let cur_t4 = replay_scaled_fusion(
                    &scales,
                    &workspace.sums,
                    &workspace.sums_lo,
                    &workspace.sums_hi,
                    &mut workspace.columns,
                    &mut workspace.fused,
                    &mut workspace.output_lo,
                    &mut workspace.output_hi,
                    &mut workspace.local_defect,
                    n,
                    &mut certification_ops,
                )?;

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

    workspace.last_certification_ops = certification_ops;
    outcome?;

    let output_result = (|| -> CoreResult<()> {
        if zero_input {
            summation_bound = replay_scaled_fusion(
                &scales,
                &workspace.sums,
                &workspace.sums_lo,
                &workspace.sums_hi,
                &mut workspace.columns,
                &mut workspace.fused,
                &mut workspace.output_lo,
                &mut workspace.output_hi,
                &mut workspace.local_defect,
                n,
                &mut certification_ops,
            )?;
            total_bound = summation_bound;
        }
        if !workspace.fused[..n].iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite(
                "Newton-Leja action produced NaN/Inf".into(),
            ));
        }
        Ok(())
    })();
    workspace.last_certification_ops = certification_ops;
    output_result?;

    if let Some(ref mut rep) = workspace.report {
        if rep.columns.len() < JOINT_PHI_TERMS {
            rep.columns.resize_with(JOINT_PHI_TERMS, Vec::new);
        }
        rep.columns.truncate(JOINT_PHI_TERMS);
        for column in &mut rep.columns {
            column.resize(n, 0.0);
        }
        rep.fused.resize(n, 0.0);
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
        update_leja_total_status(
            &mut rep.total_error,
            converged,
            degree_cap,
            total_budget,
            total_bound,
        );
        rep.evidence = evidence;
        rep.vector_products = local_vector_products;
        rep.block_products = local_block_products;
        rep.coefficient_setups = local_coefficient_setups;
        rep.certification_ops = certification_ops;
    } else {
        let mut total_error = TotalErrorStatus::EstimateOnly {
            reason: String::with_capacity(256),
            bounded_components: total_bound,
        };
        update_leja_total_status(
            &mut total_error,
            converged,
            degree_cap,
            total_budget,
            total_bound,
        );
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
    fn zero_center_scalar_phi_radii_enclose_nonbinary_factorials() {
        let (values, radii) = scalar_phi_coefficients(0.0).unwrap();
        for radius in radii.iter().take(3) {
            assert_eq!(*radius, 0.0);
        }
        for (k, lower_floor, upper_ceil) in [
            (3, 0x3fc5_5555_5555_5555, 0x3fc5_5555_5555_5556),
            (4, 0x3fa5_5555_5555_5555, 0x3fa5_5555_5555_5556),
        ] {
            assert!(radii[k] > 0.0, "phi_{k} needs a nonzero radius");
            let lower = sub_down(values[k], radii[k]).unwrap();
            let upper = add_up(values[k], radii[k]).unwrap();
            assert!(lower.to_bits() <= lower_floor);
            assert!(upper.to_bits() >= upper_ceil);
            // Exact rational ordering check: the chosen floor/ceil are the
            // adjacent binary64 values around 1/6 and 1/24, respectively.
        }
    }

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
    fn counted_node_prefix_includes_interval_domain_construction() {
        let xi = [0.0];
        let mut setup = LejaSetupWork::new();
        let bound = leja_node_polynomial_bound_counted(0, &xi, Some(&mut setup));
        assert!(bound.is_finite() && bound >= 2.0);
        // Per subdivision: one point, one domain Interval::new, then
        // sub_down/sub_up, factor Interval::new, and interval multiplication.
        assert_eq!(
            setup.snapshot().subtotal().interval_kernel_invocations(),
            6 * 128
        );
    }

    #[test]
    fn actual_leja_points_initializer_has_cold_and_warm_receipts() {
        let cell: OnceLock<CountedStaticTable<Vec<f64>>> = OnceLock::new();
        let mut cold_work = LejaSetupWork::new();
        let cold_points = get_or_init_static(
            &cell,
            StaticLejaTable::Points,
            Some(&mut cold_work),
            build_leja_points,
        );
        assert_eq!(cold_points.len(), LEJA_DEGREE_CAP + 1);
        let cold = cold_work.snapshot();
        assert_eq!(
            cold.leja_points_observation(),
            crate::leja_setup_work::OnceLockObservation::InitializedByThisCall
        );
        assert_eq!(cold.subtotal().allocation_requests(), 4);
        assert!(cold.subtotal().scalar_proxy_units() > 0);

        let mut warm_work = LejaSetupWork::new();
        let warm_points = get_or_init_static(
            &cell,
            StaticLejaTable::Points,
            Some(&mut warm_work),
            build_leja_points,
        );
        assert_eq!(warm_points, cold_points);
        let warm = warm_work.snapshot();
        assert_eq!(
            warm.leja_points_observation(),
            crate::leja_setup_work::OnceLockObservation::WarmCountedInitializer
        );
        assert_eq!(warm.subtotal().allocation_requests(), 0);
        assert_eq!(warm.subtotal().scalar_proxy_units(), 0);
        assert_eq!(warm.leja_points_history(), Some(cold.subtotal()));
    }

    #[test]
    fn failed_counted_cache_miss_retains_historical_coefficient_setup_attempt() {
        let mut cache = CertifiedLejaCoeffCache::new();
        let mut setup = LejaSetupWork::new();
        let mut coefficient_setups = 0;
        let result = cache.precompute_certified_inner(
            -1.0e10,
            0.0,
            8,
            Some(&mut setup),
            Some(&mut coefficient_setups),
        );
        assert!(result.is_err());
        assert_eq!(coefficient_setups, 1);
        let receipt = setup.snapshot();
        assert_eq!(receipt.subtotal().coefficient_tables_attempted(), 1);
        assert_eq!(receipt.subtotal().coefficient_tables_completed(), 0);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn failed_exponential_does_not_charge_unattempted_result_vectors() {
        let mut setup = LejaSetupWork::new();
        let result =
            leja_divided_differences_certified_counted(0.0, 300.0, &[2.0], Some(&mut setup));
        assert!(result.is_err());
        // Four input diagonals/subdiagonals, two exponential output vectors,
        // and four work vectors for the first column, where overflow occurs.
        // Final d/d_rad vectors are never attempted.
        assert_eq!(setup.snapshot().subtotal().allocation_requests(), 10);
        assert_eq!(setup.snapshot().subtotal().allocation_items_requested(), 48);
        assert_eq!(
            setup.snapshot().subtotal().coefficient_tables_attempted(),
            0
        );
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

#[cfg(test)]
mod production_audit_suffix {
    use super::*;
    use crate::DenseMatrix;

    #[test]
    fn suffix_bound_covers_the_actual_suffix_not_a_prefix() {
        let nodes = leja_points();
        let actual = (2.0 - nodes[1]) * (2.0 - nodes[2]);
        assert!(actual > leja_node_bounds()[1]);
        assert!(leja_suffix_bounds()[1][3] >= actual);
        for start in 0..8 {
            for end in start..16 {
                for index in 0..=32 {
                    let x = -2.0 + index as f64 / 8.0;
                    let actual: f64 = nodes[start..end].iter().map(|node| x - node).product();
                    assert!(actual.abs() <= leja_suffix_bounds()[start][end]);
                }
            }
        }
    }

    #[test]
    fn trusted_prewarm_matches_cold_and_encloses_diagonal_exponential() {
        let op = SymmetricNonpositiveOperator::gershgorin(
            DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -2.0]]).unwrap(),
        )
        .unwrap();
        let vector = [1.0, 1.0];
        let input = JointPhiInput::SameVector {
            vector: &vector,
            scales: [1.0, 0.0, 0.0, 0.0, 0.0],
        };
        let mut work = WorkCounters::default();
        let cold = leja_phi_action_certified_capped(&op, 1.0, input, 1e-6, 16, &mut work).unwrap();
        let mut cache = CertifiedLejaCoeffCache::new();
        cache
            .precompute_certified(cold.center, cold.scale, 16)
            .unwrap();
        let warm = leja_phi_action_certified_with_cache(
            &op,
            1.0,
            input,
            1e-6,
            16,
            Some(&mut cache),
            &mut work,
        )
        .unwrap();
        assert_eq!(warm.fused, cold.fused);
        assert_eq!(warm.total_bound(), cold.total_bound());
        assert!(
            matches!(warm.total_error(), TotalErrorStatus::EstimateOnly { reason, .. } if reason.contains(LEJA_ARITHMETIC_HOLD))
        );
        let err = crate::safe_l2(&[
            warm.fused[0] - (-1.0_f64).exp(),
            warm.fused[1] - (-2.0_f64).exp(),
        ]);
        assert!(err < warm.total_bound());
        assert!(cache.precompute_certified(f64::NAN, 0.25, 16).is_err());
        assert!(cache.precompute_certified(-1.5, 0.25, 129).is_err());
    }

    #[test]
    fn certificate_norm_is_outward_at_normal_and_subnormal_scales() {
        let mut ops = 0;
        assert!(certificate_l2_upper(&[3.0, 4.0], &mut ops).unwrap() >= 5.0);
        let tiny = f64::from_bits(1);
        assert!(certificate_l2_upper(&[tiny, tiny], &mut ops).unwrap() >= 2.0 * tiny);
        assert!(certificate_l2_upper(&[f64::NAN], &mut ops).is_err());
        assert!(ops > 0);
    }

    #[test]
    fn interval_matvec_replay_covers_underflow_and_cancellation() {
        let tiny = f64::from_bits(1);
        let matrix = DenseMatrix::from_rows(&[&[tiny]]).unwrap();
        let x = [0.5];
        let actual = matrix.matvec(&x).unwrap();
        assert_eq!(actual[0], 0.0);
        let mut lo = [0.0];
        let mut hi = [0.0];
        let mut ops = 0;
        matvec_interval_into(&matrix, &x, &mut lo, &mut hi, &mut ops).unwrap();
        assert!(lo[0] <= 0.0 && hi[0] >= 0.0);
        assert!(lo[0] < 0.0 && hi[0] > 0.0);
        assert!(
            interval_distance_upper(actual[0], Interval::new(lo[0], hi[0]).unwrap()).unwrap() > 0.0
        );

        let matrix = DenseMatrix::from_rows(&[&[1.0, 1.0, 1.0]]).unwrap();
        let x = [1.0e16, 1.0, -1.0e16];
        let actual = matrix.matvec(&x).unwrap();
        assert_eq!(actual[0], 0.0);
        matvec_interval_into(&matrix, &x, &mut lo, &mut hi, &mut ops).unwrap();
        assert!(Interval::new(lo[0], hi[0]).unwrap().contains(1.0));
        assert!(ops > 0);
    }

    #[test]
    fn recurrence_interval_replay_covers_subnormal_additive_underflow() {
        let tiny = f64::from_bits(1);
        let rounded = (tiny * 1.0 - 0.0) / 2.0 - 0.0;
        assert_eq!(rounded, 0.0);

        let mut ops = 0;
        let exact_next = recurrence_interval_next(
            tiny,
            Interval::point(1.0).unwrap(),
            0.0,
            1.0,
            2.0,
            0.0,
            &mut ops,
        )
        .unwrap();
        let additive_defect = interval_distance_upper(rounded, exact_next).unwrap();
        let old_relative_only_bound = 8.0 * f64::EPSILON * rounded.abs();
        assert!(old_relative_only_bound < additive_defect);
        assert!(exact_next.contains(0.0));
        assert!(exact_next.lo <= 0.0 && exact_next.hi >= tiny);
        assert!(ops > 0);
    }

    #[test]
    fn coefficient_sum_and_output_replay_cover_additive_underflow() {
        let mut sum = [0.0];
        let mut sum_lo = [0.0];
        let mut sum_hi = [0.0];
        let mut ops = 0;
        for value in [1.0e16, 1.0, -1.0e16] {
            accumulate_coefficient_term(
                &mut sum,
                &mut sum_lo,
                &mut sum_hi,
                1.0,
                &[value],
                &mut ops,
            )
            .unwrap();
        }
        assert_eq!(sum[0], 0.0);
        let exact_sum = Interval::new(sum_lo[0], sum_hi[0]).unwrap();
        assert!(exact_sum.contains(1.0));
        assert!(interval_distance_upper(sum[0], exact_sum).unwrap() >= 1.0);

        let tiny = f64::from_bits(1);
        let scales = [0.5, 0.0, 0.0, 0.0, 0.0];
        let sums = vec![vec![tiny]; JOINT_PHI_TERMS];
        let sums_lo = sums.clone();
        let sums_hi = sums.clone();
        let mut columns = vec![vec![0.0]; JOINT_PHI_TERMS];
        let mut fused = [0.0];
        let mut out_lo = [0.0];
        let mut out_hi = [0.0];
        let mut defects = [0.0];
        let bound = replay_scaled_fusion(
            &scales,
            &sums,
            &sums_lo,
            &sums_hi,
            &mut columns,
            &mut fused,
            &mut out_lo,
            &mut out_hi,
            &mut defects,
            1,
            &mut ops,
        )
        .unwrap();
        assert_eq!(fused[0], 0.0);
        assert!(bound > 0.0);
        assert!(Interval::new(out_lo[0], out_hi[0]).unwrap().contains(0.0));
        assert!(ops > 0);
    }

    #[test]
    fn interval_replay_refuses_nonfinite_action_and_checks_normalized_boundary() {
        let matrix = DenseMatrix::from_rows(&[&[f64::MAX]]).unwrap();
        let mut lo = [0.0];
        let mut hi = [0.0];
        let mut ops = 0;
        assert!(matvec_interval_into(&matrix, &[2.0], &mut lo, &mut hi, &mut ops).is_err());
        assert!(
            ops > 0,
            "refused interval products must retain attempted work"
        );

        let a = -7.769_021_559_909_74e-98;
        let b = -7.769_021_559_909_737e-98;
        let center = 0.5 * (a + b);
        let initial_scale = div_up(sub_up(b, a).unwrap(), 4.0).unwrap();
        let initial_support = normalized_spectral_support(a, b, center, initial_scale).unwrap();
        assert!(initial_support.lo < -2.0 || initial_support.hi > 2.0);
        let (scale, support) =
            enlarge_scale_for_normalized_support(a, b, center, initial_scale, &mut ops).unwrap();
        assert!(scale > initial_scale);
        assert!(support.lo >= -2.0 && support.hi <= 2.0);
        assert!(support.lo <= (a - center) / scale);
        assert!(support.hi >= (b - center) / scale);
    }

    #[test]
    fn positive_outward_support_increases_t1_exponential_factor() {
        let tiny = f64::from_bits(1);
        let matrix = DenseMatrix::from_rows(&[&[-tiny, 0.0], &[0.0, -2.0 * tiny]]).unwrap();
        let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
        let input = JointPhiInput::SameVector {
            vector: &[1.0, -0.5],
            scales: [1.0, 0.0, 0.0, 0.0, 0.0],
        };
        let mut work = WorkCounters::default();
        let report = leja_phi_action_certified_capped(&op, 0.5, input, 1.0, 8, &mut work).unwrap();
        assert_eq!(report.branch, "newton");
        assert!(report.interval[1] > 0.0, "support={:?}", report.interval);
        let b = -mul_down(0.5, op.enclosure().lambda).unwrap();
        assert!(
            report.interval[1] > b,
            "expanded support upper {} must exceed spectral b {b}",
            report.interval[1]
        );
        let factor = exp_interval(report.interval[1].max(0.0)).unwrap().hi;
        assert!(factor > 1.0);
        assert!(matches!(
            report.total_error(),
            TotalErrorStatus::EstimateOnly { .. }
        ));
    }

    #[test]
    fn refused_output_preserves_certificate_work_accounting() {
        let matrix = DenseMatrix::from_rows(&[&[-1.0]]).unwrap();
        let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
        let mut cache = CertifiedLejaCoeffCache::new();
        let mut workspace = CertifiedLejaWorkspace::new(1, 1);
        let mut work = WorkCounters::default();
        let good_vector = [0.25];
        let good_input = JointPhiInput::SameVector {
            vector: &good_vector,
            scales: [1.0, 0.0, 0.0, 0.0, 0.0],
        };
        leja_phi_action_certified_with_cache_and_workspace(
            &op,
            0.0,
            good_input,
            1.0,
            8,
            Some(&mut cache),
            &mut work,
            &mut workspace,
        )
        .unwrap();
        let committed = workspace.report.as_ref().unwrap().clone();

        let vector = [1.0e308];
        let input = JointPhiInput::SameVector {
            vector: &vector,
            scales: [1.0e308, 0.0, 0.0, 0.0, 0.0],
        };
        let result = leja_phi_action_certified_with_cache_and_workspace(
            &op,
            0.0,
            input,
            1.0,
            8,
            Some(&mut cache),
            &mut work,
            &mut workspace,
        );
        assert!(result.is_err());
        drop(result);
        assert!(workspace.last_certification_ops() > 0);
        assert_eq!(workspace.report.as_ref().unwrap(), &committed);
    }

    #[test]
    fn interval_matvec_overflow_refusal_preserves_workspace_work() {
        let matrix = DenseMatrix::from_rows(&[&[-100.0, 0.0], &[0.0, -90.0]]).unwrap();
        let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
        let vector = [1.0e308, 0.0];
        let input = JointPhiInput::SameVector {
            vector: &vector,
            scales: [1.0, 0.0, 0.0, 0.0, 0.0],
        };
        let mut cache = CertifiedLejaCoeffCache::new();
        let mut workspace = CertifiedLejaWorkspace::new(2, 2);
        let mut work = WorkCounters::default();
        let good_vector = [0.25, -0.5];
        leja_phi_action_certified_with_cache_and_workspace(
            &op,
            0.0,
            JointPhiInput::SameVector {
                vector: &good_vector,
                scales: [1.0, 0.0, 0.0, 0.0, 0.0],
            },
            1.0,
            8,
            Some(&mut cache),
            &mut work,
            &mut workspace,
        )
        .unwrap();
        let committed = workspace.report.as_ref().unwrap().clone();
        let result = leja_phi_action_certified_with_cache_and_workspace(
            &op,
            1.0,
            input,
            1.0,
            8,
            Some(&mut cache),
            &mut work,
            &mut workspace,
        );
        assert!(result.is_err());
        drop(result);
        assert!(workspace.last_certification_ops() > 0);
        assert_eq!(workspace.report.as_ref().unwrap(), &committed);
        assert!(work.poly_block_products > 0);
        assert!(work.poly_vector_products > 0);
    }
}
