//! Path sums and radius policies of causal stage majorants (research nodes
//! `research/thread_transfer_path_action_20261002` and
//! `research/thread_transfer_radius_proposal_20261002`, thread-transfer DAG
//! nodes P1-PATH-ACTION and P1-RADIUS-PROPOSAL). Research only.
//!
//! A doubling certificate bounds the stage error by `E = sum_{k<s} H^k a`
//! for a strictly lower nonnegative `s x s` block `H` (so `H^s = 0`) and a
//! nonnegative seed `a`. Three evaluations of the same finite sum:
//!
//! * [`upper_path_sum_matrix`]: `S = prod_l (I + H^(2^l))` by
//!   `S <- S + Q S; Q <- Q^2`, then `E = S a`. This is the order of
//!   `blocked_doubling_certificate_with_execution`, operation for operation;
//! * [`upper_path_sum_action`]: `e <- e + Q e; Q <- Q^2` on the seed itself,
//!   so no sum matrix is formed (`L` matrix-vector and `L - 1` matrix-matrix
//!   products for `L = ceil(log2 s)`);
//! * [`upper_causal_solve`]: `e_i = a_i + sum_{j<i} H_ij e_j`.
//!
//! In exact arithmetic all three are `sum_{j < 2^L} H^j a = sum_{j<s} H^j a`.
//! Every operation rounds upward and every input is nonnegative, so each is
//! an upper bound of that sum; they differ only in rounding order, and are
//! not bit-identical in general.
//!
//! A certificate's `H` depends on an a priori state radius. With a radius
//! box `D_(u,i)` (component `u`, stage `i`), row `i` of component `u`'s block
//! uses `D_(u,i)`, and the box closes when every state radius
//! `sum_{j<i} |alpha_ij| E_(u,j)` is at most `D_(u,i)`
//! ([`evaluate_radius_box`]). A common radius is the constant box. The
//! policies here only *propose* boxes: [`residual_seeded_common_radius`] from
//! the current residual (`D = factor * max B E(0)`, a necessary level for a
//! common radius since `H(D)` is monotone in `D`), [`causal_radius_box`] from
//! the causal recurrence. Validity is always decided by a full evaluation of
//! the closure inequality at the proposed box, and a failed proposal is a
//! failure of the proposal, not a proof that no radius exists.

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{add_up, mul_up},
};
use serde::{Deserialize, Serialize};

use crate::{CERTIFICATE_STRUCTURE_UNSUPPORTED, ParallelExecution, doubling_levels};

/// Counted work of a path-sum evaluation. `allocated_values` counts every
/// f64 slot allocated, `peak_live_values` the largest number alive at once
/// (both excluding the caller's `H` and seed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathSumWork {
    pub directed_operations: u64,
    pub matrix_products: u64,
    pub matrix_vector_products: u64,
    pub allocated_values: u64,
    pub peak_live_values: u64,
}

impl PathSumWork {
    fn absorb(&mut self, other: &Self) {
        self.directed_operations += other.directed_operations;
        self.matrix_products += other.matrix_products;
        self.matrix_vector_products += other.matrix_vector_products;
        self.allocated_values += other.allocated_values;
        self.peak_live_values = self.peak_live_values.max(other.peak_live_values);
    }
}

/// How the finite path sum is evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathEvaluation {
    /// [`upper_path_sum_matrix`].
    MatrixOrder,
    /// [`upper_path_sum_action`].
    ActionFirst,
}

type Block = Vec<Vec<f64>>;

/// A square, strictly lower, finite, nonnegative block and a finite
/// nonnegative seed of its size. Structural zeros are required to be exact
/// zeros: a tiny diagonal or upper entry is a different structure.
pub fn validate_strict_lower_block(h: &[Vec<f64>], seed: &[f64]) -> CoreResult<()> {
    let s = h.len();
    if s == 0 || seed.len() != s || h.iter().any(|row| row.len() != s) {
        return Err(CoreError::InvalidInput(format!(
            "{CERTIFICATE_STRUCTURE_UNSUPPORTED}: the path sum needs a nonempty square block and a seed of its size"
        )));
    }
    for (i, row) in h.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            if !(value.is_finite() && *value >= 0.0) {
                return Err(CoreError::InvalidInput(format!(
                    "CERTIFICATE_NOT_VALIDATED: block entry ({i}, {j}) = {value:e} is not finite and nonnegative"
                )));
            }
            if j >= i && *value != 0.0 {
                return Err(CoreError::InvalidInput(format!(
                    "{CERTIFICATE_STRUCTURE_UNSUPPORTED}: block entry ({i}, {j}) = {value:e} is on or above the diagonal"
                )));
            }
        }
    }
    if !seed.iter().all(|value| value.is_finite() && *value >= 0.0) {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: the seed is not finite and nonnegative".into(),
        ));
    }
    Ok(())
}

/// `left right` rounded upward, skipping zero terms, in the fixed order of
/// the blocked certificate (2 operations per nonzero term).
fn upper_block_product(left: &Block, right: &Block, work: &mut PathSumWork) -> CoreResult<Block> {
    let s = right.len();
    let mut out = vec![vec![0.0; s]; s];
    for (r, row) in left.iter().enumerate() {
        for c in 0..s {
            let mut total = 0.0;
            for (k, value) in row.iter().enumerate() {
                if *value != 0.0 && right[k][c] != 0.0 {
                    total = add_up(total, mul_up(*value, right[k][c])?)?;
                    work.directed_operations += 2;
                }
            }
            out[r][c] = total;
        }
    }
    work.matrix_products += 1;
    work.allocated_values += (s * s) as u64;
    Ok(out)
}

/// `Q e` rounded upward, skipping zero terms.
fn upper_block_apply(q: &Block, e: &[f64], work: &mut PathSumWork) -> CoreResult<Vec<f64>> {
    let mut out = vec![0.0; e.len()];
    for (r, row) in q.iter().enumerate() {
        let mut total = 0.0;
        for (k, value) in row.iter().enumerate() {
            if *value != 0.0 && e[k] != 0.0 {
                total = add_up(total, mul_up(*value, e[k])?)?;
                work.directed_operations += 2;
            }
        }
        out[r] = total;
    }
    work.matrix_vector_products += 1;
    work.allocated_values += e.len() as u64;
    Ok(out)
}

/// The blocked certificate's order: `S = I; Q = H; S <- S + Q S;
/// Q <- Q^2` over `L` levels, then `E_r = sum_k |S_rk| |a_k|`.
pub fn upper_path_sum_matrix(
    h: &[Vec<f64>],
    seed: &[f64],
    work: &mut PathSumWork,
) -> CoreResult<Vec<f64>> {
    validate_strict_lower_block(h, seed)?;
    let s = h.len();
    let levels = doubling_levels(s);
    let square = (s * s) as u64;
    let mut sum = (0..s)
        .map(|r| (0..s).map(|c| if r == c { 1.0 } else { 0.0 }).collect())
        .collect::<Block>();
    let mut power = h.to_vec();
    work.allocated_values += 2 * square;
    // sum, power, and the product being formed.
    work.peak_live_values = work.peak_live_values.max(3 * square);
    for level in 0..levels {
        let added = upper_block_product(&power, &sum, work)?;
        for (row, extra_row) in sum.iter_mut().zip(&added) {
            for (value, extra) in row.iter_mut().zip(extra_row) {
                *value = add_up(*value, *extra)?;
                work.directed_operations += 1;
            }
        }
        if level + 1 < levels {
            power = upper_block_product(&power, &power, work)?;
        }
    }
    let mut out = Vec::with_capacity(s);
    for row in &sum {
        let mut total = 0.0;
        for (x, y) in row.iter().zip(seed) {
            total = add_up(total, mul_up(x.abs(), y.abs())?)?;
        }
        out.push(total);
    }
    work.directed_operations += 2 * square;
    work.matrix_vector_products += 1;
    work.allocated_values += s as u64;
    Ok(out)
}

/// The action-first order: `e = a; Q = H; e <- e + Q e` (the whole right
/// side with the previous `e`), and `Q <- Q^2` between levels.
pub fn upper_path_sum_action(
    h: &[Vec<f64>],
    seed: &[f64],
    work: &mut PathSumWork,
) -> CoreResult<Vec<f64>> {
    validate_strict_lower_block(h, seed)?;
    let s = h.len();
    let levels = doubling_levels(s);
    let square = (s * s) as u64;
    let mut e = seed.to_vec();
    work.allocated_values += s as u64;
    if levels == 0 {
        work.peak_live_values = work.peak_live_values.max(s as u64);
        return Ok(e);
    }
    let mut q = h.to_vec();
    work.allocated_values += square;
    // e, Q e and Q, plus the square of Q while it is formed.
    let live_q = if levels > 1 { 2 * square } else { square };
    work.peak_live_values = work.peak_live_values.max(2 * s as u64 + live_q);
    for level in 0..levels {
        let applied = upper_block_apply(&q, &e, work)?;
        for (value, extra) in e.iter_mut().zip(&applied) {
            if *extra != 0.0 {
                *value = add_up(*value, *extra)?;
                work.directed_operations += 1;
            }
        }
        if level + 1 < levels {
            q = upper_block_product(&q, &q, work)?;
        }
    }
    Ok(e)
}

/// `e_i = a_i + sum_{j<i} H_ij e_j`, rounded upward.
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn upper_causal_solve(
    h: &[Vec<f64>],
    seed: &[f64],
    work: &mut PathSumWork,
) -> CoreResult<Vec<f64>> {
    validate_strict_lower_block(h, seed)?;
    let s = h.len();
    let mut e = seed.to_vec();
    work.allocated_values += s as u64;
    work.peak_live_values = work.peak_live_values.max(s as u64);
    for i in 0..s {
        let mut total = e[i];
        for j in 0..i {
            if h[i][j] != 0.0 && e[j] != 0.0 {
                total = add_up(total, mul_up(h[i][j], e[j])?)?;
                work.directed_operations += 2;
            }
        }
        e[i] = total;
    }
    Ok(e)
}

/// One of the three evaluations.
pub fn upper_path_sum(
    h: &[Vec<f64>],
    seed: &[f64],
    mode: PathEvaluation,
    work: &mut PathSumWork,
) -> CoreResult<Vec<f64>> {
    match mode {
        PathEvaluation::MatrixOrder => upper_path_sum_matrix(h, seed, work),
        PathEvaluation::ActionFirst => upper_path_sum_action(h, seed, work),
    }
}

/// The entries of a causal majorant with component blocks: for each
/// component `u` a strictly lower `s x s` block `H_u(D)` whose row `i`
/// depends on that row's state radius, a seed `a_u`, and the absolute
/// state map `|alpha|` shared by all components.
pub trait MajorantEntries: Sync {
    fn stages(&self) -> usize;
    fn components(&self) -> usize;
    /// `a_(u,i) >= 0`.
    fn seed(&self, component: usize, stage: usize) -> f64;
    /// `|alpha_ij|` for `j < i`.
    fn alpha_abs(&self, stage: usize, column: usize) -> f64;
    /// `H_u(D)_ij` for `j < i`, rounded upward, at the row's radius `D`.
    fn coupling(
        &self,
        component: usize,
        stage: usize,
        column: usize,
        radius: f64,
    ) -> CoreResult<f64>;
    /// Directed operations of one [`MajorantEntries::coupling`] call.
    fn coupling_operations(&self) -> u64;
}

/// State radii `D_(u,i)`: `radii[u][i]` for component `u` and stage `i`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadiusBox {
    pub radii: Vec<Vec<f64>>,
}

impl RadiusBox {
    /// The constant box of a common radius.
    pub fn common(components: usize, stages: usize, radius: f64) -> CoreResult<Self> {
        let radii = Self {
            radii: vec![vec![radius; stages]; components],
        };
        radii.validate(components, stages)?;
        Ok(radii)
    }

    pub fn validate(&self, components: usize, stages: usize) -> CoreResult<()> {
        if self.radii.len() != components || self.radii.iter().any(|row| row.len() != stages) {
            return Err(CoreError::InvalidInput(format!(
                "CERTIFICATE_NOT_VALIDATED: a radius box must be {components} x {stages}"
            )));
        }
        if !self
            .radii
            .iter()
            .flatten()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(CoreError::InvalidInput(
                "CERTIFICATE_NOT_VALIDATED: every state radius must be finite and >= 0".into(),
            ));
        }
        Ok(())
    }

    pub fn max(&self) -> f64 {
        self.radii.iter().flatten().copied().fold(0.0, f64::max)
    }
}

/// The path sum at one radius box.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxEvaluation {
    pub mode: PathEvaluation,
    /// `bound[i][u] >= E_(u,i)`, the layout of a certificate's stage bound.
    pub bound: Vec<Vec<f64>>,
    /// `state_radius[u][i] = sum_{j<i} |alpha_ij| E_(u,j)`, rounded upward.
    pub state_radius: Vec<Vec<f64>>,
    pub max_state_radius: f64,
    /// Every state radius is at most its box entry.
    pub closes: bool,
    /// Path-sum work plus the operations of forming the blocks.
    pub work: PathSumWork,
    /// Nonzero block entries.
    pub nonzeros: u64,
}

/// Forms `H_u` at `radii` (row `i` of component `u` at `radii[u][i]`),
/// evaluates `E_u` in `mode` per component on `execution`, and checks
/// closure. Deterministic for any worker count: components are independent
/// and each is evaluated in a fixed order.
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn evaluate_radius_box<E: MajorantEntries>(
    entries: &E,
    radii: &RadiusBox,
    mode: PathEvaluation,
    execution: &ParallelExecution,
) -> CoreResult<BoxEvaluation> {
    let (s, n) = (entries.stages(), entries.components());
    if s == 0 || n == 0 {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: a majorant needs stages and components".into(),
        ));
    }
    radii.validate(n, s)?;
    let components = (0..n).collect::<Vec<_>>();
    type ComponentResult = (Vec<f64>, PathSumWork, u64);
    let per_component =
        execution.map_ordered(&components, |&u| -> CoreResult<ComponentResult> {
            let mut work = PathSumWork::default();
            let mut nonzeros = 0_u64;
            let mut block = vec![vec![0.0; s]; s];
            work.allocated_values += (s * s) as u64;
            for i in 0..s {
                for j in 0..i {
                    block[i][j] = entries.coupling(u, i, j, radii.radii[u][i])?;
                    work.directed_operations += entries.coupling_operations();
                    nonzeros += u64::from(block[i][j] != 0.0);
                }
            }
            let seed = (0..s).map(|i| entries.seed(u, i)).collect::<Vec<_>>();
            let column = upper_path_sum(&block, &seed, mode, &mut work)?;
            Ok((column, work, nonzeros))
        })?;
    let mut bound = vec![vec![0.0; n]; s];
    let mut work = PathSumWork::default();
    let mut nonzeros = 0;
    for (u, (column, component_work, component_nonzeros)) in per_component.into_iter().enumerate() {
        for i in 0..s {
            bound[i][u] = column[i];
        }
        work.absorb(&component_work);
        nonzeros += component_nonzeros;
    }
    let mut state_radius = vec![vec![0.0; s]; n];
    let mut max_state_radius = 0.0_f64;
    let mut closes = true;
    for i in 0..s {
        for u in 0..n {
            let mut total = 0.0;
            for j in 0..i {
                total = add_up(total, mul_up(entries.alpha_abs(i, j), bound[j][u].abs())?)?;
            }
            state_radius[u][i] = total;
            max_state_radius = max_state_radius.max(total);
            closes &= total <= radii.radii[u][i];
        }
    }
    Ok(BoxEvaluation {
        mode,
        bound,
        state_radius,
        max_state_radius,
        closes,
        work,
        nonzeros,
    })
}

/// Outcome of a radius proposal: every evaluation is kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadiusProposal {
    pub policy: String,
    /// The proposed box, if one was formed.
    pub radii: Option<RadiusBox>,
    pub evaluations: Vec<BoxEvaluation>,
    /// The final evaluation at the proposed box closed.
    pub closes: bool,
    pub reason: Option<String>,
}

/// Residual-seeded common radius: one evaluation at `D = 0` gives
/// `B = max_(u,i) sum_j |alpha_ij| E_(u,j)(0)`, which any closing common
/// radius must reach (`H(D)` is monotone in `D`); the proposal is
/// `D = factor * B` rounded upward, then evaluated in full. `B = 0` closes at
/// `D = 0` with the first evaluation.
pub fn residual_seeded_common_radius<E: MajorantEntries>(
    entries: &E,
    factor: f64,
    mode: PathEvaluation,
    execution: &ParallelExecution,
) -> CoreResult<RadiusProposal> {
    if !(factor.is_finite() && factor > 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "CERTIFICATE_NOT_VALIDATED: the radius factor {factor:e} must be finite and positive"
        )));
    }
    let (s, n) = (entries.stages(), entries.components());
    let policy = format!("residual-seeded-common/factor={factor}");
    let zero = RadiusBox::common(n, s, 0.0)?;
    let preflight = evaluate_radius_box(entries, &zero, mode, execution)?;
    if preflight.closes {
        return Ok(RadiusProposal {
            policy,
            radii: Some(zero),
            evaluations: vec![preflight],
            closes: true,
            reason: None,
        });
    }
    let radius = mul_up(factor, preflight.max_state_radius)?;
    let proposal = RadiusBox::common(n, s, radius)?;
    let check = evaluate_radius_box(entries, &proposal, mode, execution)?;
    let closes = check.closes;
    Ok(RadiusProposal {
        policy,
        radii: Some(proposal),
        evaluations: vec![preflight, check],
        closes,
        reason: (!closes).then(|| "RADIUS_PROPOSAL_DID_NOT_CLOSE".into()),
    })
}

/// Causal radius box: per component, in stage order,
/// `D_(u,i) = (1 + inflation) sum_{j<i} |alpha_ij| E_(u,j)` and
/// `E_(u,i) = a_(u,i) + sum_{j<i} H_u(D_(u,i))_ij E_(u,j)`, all rounded
/// upward; each `E` is formed at the already inflated radii, so the full
/// evaluation sees the same quantities up to its rounding order. The
/// construction is only a proposal: the box is then evaluated in full in
/// `mode`, and only that evaluation decides closure.
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn causal_radius_box<E: MajorantEntries>(
    entries: &E,
    inflation: f64,
    mode: PathEvaluation,
    execution: &ParallelExecution,
) -> CoreResult<RadiusProposal> {
    if !(inflation.is_finite() && inflation >= 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "CERTIFICATE_NOT_VALIDATED: the radius inflation {inflation:e} must be finite and >= 0"
        )));
    }
    let (s, n) = (entries.stages(), entries.components());
    let scale = add_up(1.0, inflation)?;
    let mut radii = vec![vec![0.0; s]; n];
    for (u, row) in radii.iter_mut().enumerate() {
        let mut e = vec![0.0; s];
        for i in 0..s {
            let mut state = 0.0;
            for j in 0..i {
                state = add_up(state, mul_up(entries.alpha_abs(i, j), e[j])?)?;
            }
            row[i] = mul_up(scale, state)?;
            let mut total = entries.seed(u, i);
            for j in 0..i {
                let coupling = entries.coupling(u, i, j, row[i])?;
                if coupling != 0.0 && e[j] != 0.0 {
                    total = add_up(total, mul_up(coupling, e[j])?)?;
                }
            }
            e[i] = total;
        }
    }
    let proposal = RadiusBox { radii };
    let check = evaluate_radius_box(entries, &proposal, mode, execution)?;
    let closes = check.closes;
    Ok(RadiusProposal {
        policy: format!("causal-box/inflation={inflation:e}"),
        radii: Some(proposal),
        evaluations: vec![check],
        closes,
        reason: (!closes).then(|| "RADIUS_PROPOSAL_DID_NOT_CLOSE".into()),
    })
}

/// A test family with `H(D)_ij = H0_ij + D_i H1_ij` (one component), rounded
/// upward. With `H0 = 0`, `H1` and `|alpha|` the subdiagonal ones and seed
/// `1`, it is the majorant of `k_0 = 1, k_i = 1 + k_(i-1)^2` at the zero
/// candidate, for which no common radius closes (stage 2 needs
/// `1 + D <= D`) while the box `(0, 1, 2)` does.
#[derive(Clone, Debug, PartialEq)]
pub struct AffineMajorant {
    pub h0: Vec<Vec<f64>>,
    pub h1: Vec<Vec<f64>>,
    pub alpha_abs: Vec<Vec<f64>>,
    pub seed: Vec<f64>,
}

impl AffineMajorant {
    pub fn new(
        h0: Vec<Vec<f64>>,
        h1: Vec<Vec<f64>>,
        alpha_abs: Vec<Vec<f64>>,
        seed: Vec<f64>,
    ) -> CoreResult<Self> {
        validate_strict_lower_block(&h0, &seed)?;
        validate_strict_lower_block(&h1, &seed)?;
        validate_strict_lower_block(&alpha_abs, &seed)?;
        Ok(Self {
            h0,
            h1,
            alpha_abs,
            seed,
        })
    }
}

impl MajorantEntries for AffineMajorant {
    fn stages(&self) -> usize {
        self.seed.len()
    }
    fn components(&self) -> usize {
        1
    }
    fn seed(&self, _component: usize, stage: usize) -> f64 {
        self.seed[stage]
    }
    fn alpha_abs(&self, stage: usize, column: usize) -> f64 {
        self.alpha_abs[stage][column]
    }
    fn coupling(
        &self,
        _component: usize,
        stage: usize,
        column: usize,
        radius: f64,
    ) -> CoreResult<f64> {
        add_up(
            self.h0[stage][column],
            mul_up(radius, self.h1[stage][column])?,
        )
    }
    fn coupling_operations(&self) -> u64 {
        2
    }
}
