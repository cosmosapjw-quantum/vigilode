use crate::{
    common::{
        apply_left, apply_left_with_raw, residual_threshold, selected_residual_norm,
        true_residual_into, validate_residual_scale, validate_system, validate_tolerances,
    },
    gmres::arnoldi_happy_breakdown_from_norm,
    kernels::{axpy, dot, linear_combination_into, normalize, two_pass_mgs},
    small::{generalized_eigen, least_squares},
    workspace::GcrodrWorkspace,
};
use faer::{Mat, c64};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, KrylovSystemIdentity, LinearOperator,
    LinearSolveReport, LuFactorization, Preconditioner, WorkCounters, exact_krylov_system_identity,
    safe_l2,
};
use serde::{Deserialize, Serialize};

type VectorBasis = Vec<Vec<f64>>;
type RecyclePair = (VectorBasis, VectorBasis);
type AugmentedRelation = (VectorBasis, VectorBasis, DenseMatrix);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GcrodrConfig {
    pub restart: usize,
    pub max_arnoldi: usize,
    pub recycle_dim: usize,
    pub rank_tol: f64,
    pub rtol: f64,
    pub atol: f64,
}
impl Default for GcrodrConfig {
    fn default() -> Self {
        Self {
            restart: 40,
            max_arnoldi: 200,
            recycle_dim: 8,
            rank_tol: 1e-12,
            rtol: 1e-11,
            atol: 1e-13,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GcrodrState {
    pub basis: Vec<Vec<f64>>,
    pub image: Vec<Vec<f64>>,
    pub operator_token: Option<u64>,
    pub system_identity: Option<KrylovSystemIdentity>,
    pub previous_solution: Option<Vec<f64>>,
    pub generation: u64,
}
impl GcrodrState {
    pub fn rank(&self) -> usize {
        self.basis.len()
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn verify_invariant(
        &self,
        op: &dyn LinearOperator,
        pc: &dyn Preconditioner,
        tol: f64,
    ) -> CoreResult<()> {
        if self.basis.len() != self.image.len() {
            return Err(CoreError::Dimension(
                "GCRO-DR state basis/image mismatch".into(),
            ));
        }
        let mut c = WorkCounters::default();
        for (u, img) in self.basis.iter().zip(&self.image) {
            let mut got = vec![0.0; u.len()];
            apply_left(op, pc, u, &mut got, &mut c, ApplyCategory::Refresh)?;
            let err = safe_l2(&got.iter().zip(img).map(|(a, b)| a - b).collect::<Vec<_>>());
            if err > tol * (1.0 + safe_l2(img)) {
                return Err(CoreError::LinearSolve(format!(
                    "BU=C invariant defect {err:.3e}"
                )));
            }
        }
        for i in 0..self.image.len() {
            for j in 0..self.image.len() {
                let got = self.image[i]
                    .iter()
                    .zip(&self.image[j])
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                let target = if i == j { 1.0 } else { 0.0 };
                if (got - target).abs() > tol {
                    return Err(CoreError::LinearSolve("C^T C invariant defect".into()));
                }
            }
        }
        Ok(())
    }
}

fn orthonormalize_pair(
    y: Vec<Vec<f64>>,
    by: Vec<Vec<f64>>,
    rank_tol: f64,
    absolute_rank_floor: f64,
    counters: &mut WorkCounters,
) -> CoreResult<RecyclePair> {
    if y.len() != by.len() {
        return Err(CoreError::Dimension("recycle pair mismatch".into()));
    }
    let columns = y.len();
    if columns == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let n = y[0].len();
    if y.iter().any(|v| v.len() != n) || by.iter().any(|v| v.len() != n) {
        return Err(CoreError::Dimension("ragged recycle pair".into()));
    }
    if !y
        .iter()
        .flatten()
        .chain(by.iter().flatten())
        .all(|v| v.is_finite())
    {
        return Err(CoreError::NonFinite(
            "non-finite recycle basis or image".into(),
        ));
    }

    // Match the stable normalization used by the Python reference: perform a
    // column-pivoted QR of BY, then transform Y with the same pivot and R^{-1}.
    // If B Y = BY, this constructs B U = C with orthonormal C without the
    // cancellation amplification of a one-pass Gram-Schmidt pair update.
    let by_matrix = Mat::from_fn(n, columns, |i, j| by[j][i]);
    let qr = by_matrix.col_piv_qr();
    let r = qr.thin_R();
    let diag_len = n.min(columns);
    let scale = if diag_len == 0 { 0.0 } else { r[(0, 0)].abs() };
    // A nominal rank threshold below sqrt(eps) can retain directions whose
    // normalization amplifies the finite Arnoldi-relation defect by more than
    // binary64 can safely certify.  The effective floor is therefore the
    // larger of the user threshold and sqrt(eps).
    let effective_rank_tol = rank_tol.max(f64::EPSILON.sqrt());
    let threshold = (effective_rank_tol * scale).max(absolute_rank_floor.max(0.0));
    let rank = if scale == 0.0 {
        0
    } else {
        (0..diag_len)
            .filter(|&i| r[(i, i)].abs() > threshold)
            .count()
    };
    counters.recycle_dropped_vectors += (columns - rank) as u64;
    if rank == 0 {
        return Ok((Vec::new(), Vec::new()));
    }

    let pivots = qr.P().arrays().0;
    let q = qr.compute_thin_Q();
    let mut r11 = DenseMatrix::zeros(rank, rank);
    for i in 0..rank {
        for j in 0..rank {
            r11[(i, j)] = r[(i, j)];
        }
    }
    let rt_factor = LuFactorization::new(&r11.transpose())?;
    let mut u = vec![vec![0.0; n]; rank];
    for row in 0..n {
        let yp_row: Vec<f64> = (0..rank).map(|j| y[pivots[j]][row]).collect();
        let solved = rt_factor.solve(&yp_row)?;
        for j in 0..rank {
            u[j][row] = solved[j];
        }
    }
    let c: Vec<Vec<f64>> = (0..rank)
        .map(|j| (0..n).map(|i| q[(i, j)]).collect())
        .collect();
    if !u
        .iter()
        .flatten()
        .chain(c.iter().flatten())
        .all(|v| v.is_finite())
    {
        return Err(CoreError::NonFinite(
            "recycle normalization produced NaN/Inf".into(),
        ));
    }
    Ok((u, c))
}

fn columns_dot(
    left: &[Vec<f64>],
    right: &[Vec<f64>],
    counters: &mut WorkCounters,
) -> CoreResult<DenseMatrix> {
    let mut out = DenseMatrix::zeros(left.len(), right.len());
    for i in 0..left.len() {
        for j in 0..right.len() {
            out[(i, j)] = dot(&left[i], &right[j], counters)?;
        }
    }
    Ok(out)
}
fn columns_mul(cols: &[Vec<f64>], p: &DenseMatrix) -> CoreResult<Vec<Vec<f64>>> {
    if cols.len() != p.nrows() {
        return Err(CoreError::Dimension(
            "column coefficient multiply mismatch".into(),
        ));
    }
    let n = cols.first().map_or(0, Vec::len);
    let mut out = vec![vec![0.0; n]; p.ncols()];
    for j in 0..p.ncols() {
        for q in 0..cols.len() {
            let a = p[(q, j)];
            for i in 0..n {
                out[j][i] += a * cols[q][i];
            }
        }
    }
    Ok(out)
}
fn mat_t_mul(a: &DenseMatrix, b: &DenseMatrix) -> CoreResult<DenseMatrix> {
    a.transpose().matmul(b)
}

fn real_harmonic_subspace(
    g: &DenseMatrix,
    wtv: &DenseMatrix,
    target: usize,
    counters: &mut WorkCounters,
) -> CoreResult<Option<DenseMatrix>> {
    if g.ncols() == 0 || target == 0 {
        return Ok(None);
    }
    let a = mat_t_mul(g, g)?;
    let b = g.transpose().matmul(wtv)?;
    counters.harmonic_ritz_solves += 1;
    let (values, vectors) = match generalized_eigen(&a, &b) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let mut order: Vec<usize> = (0..values.len())
        .filter(|&i| values[i].re.is_finite() && values[i].im.is_finite())
        .collect();
    order.sort_by(|&i, &j| values[i].norm().total_cmp(&values[j].norm()));
    let q = g.ncols();
    let mut raw: Vec<Vec<f64>> = Vec::new();
    let mut consumed = vec![false; values.len()];
    for &idx in &order {
        if consumed[idx] || raw.len() >= target {
            continue;
        }
        let val = values[idx];
        let z = &vectors[idx];
        let imag_norm = safe_l2(&z.iter().map(|v| v.im).collect::<Vec<_>>());
        let real_norm = safe_l2(&z.iter().map(|v| v.re).collect::<Vec<_>>());
        if val.im.abs() <= 1e-12 * (1.0 + val.norm()) && imag_norm <= 1e-12 * (1.0 + real_norm) {
            raw.push(z.iter().map(|v| v.re).collect());
            consumed[idx] = true;
        } else if raw.len() + 2 <= target {
            raw.push(z.iter().map(|v| v.re).collect());
            raw.push(z.iter().map(|v| v.im).collect());
            consumed[idx] = true;
            for &cand in &order {
                if cand != idx
                    && !consumed[cand]
                    && (values[cand] - c64::new(val.re, -val.im)).norm()
                        <= 1e-8 * (1.0 + val.norm())
                {
                    consumed[cand] = true;
                    break;
                }
            }
        }
    }
    if raw.is_empty() {
        return Ok(None);
    }
    let mut basis: Vec<Vec<f64>> = Vec::new();
    for mut v in raw {
        let _ = two_pass_mgs(&mut v, &basis, counters)?;
        if normalize(&mut v)? > 1e-12 {
            basis.push(v);
        }
        if basis.len() == target {
            break;
        }
    }
    if basis.is_empty() {
        return Ok(None);
    }
    let mut p = DenseMatrix::zeros(q, basis.len());
    for j in 0..basis.len() {
        for i in 0..q {
            p[(i, j)] = basis[j][i];
        }
    }
    Ok(Some(p))
}

fn update_recycle(
    vhat: &[Vec<f64>],
    what: &[Vec<f64>],
    g: &DenseMatrix,
    target: usize,
    rank_tol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<Option<RecyclePair>> {
    let wtv = columns_dot(what, vhat, counters)?;
    let Some(p) = real_harmonic_subspace(g, &wtv, target, counters)? else {
        return Ok(None);
    };
    let y = columns_mul(vhat, &p)?;
    let gp = g.matmul(&p)?;
    let by = columns_mul(what, &gp)?;
    // Harmonic Ritz combinations can cancel to an image that is numerically
    // null even when its own relative QR spectrum looks rank one.  Compare the
    // candidate image against the scale of the augmented Arnoldi relation G,
    // not against itself.  This remains scale invariant: if the whole operator
    // is multiplied by a constant, both G and BY scale by that constant.
    let relation_scale = (0..g.ncols())
        .map(|j| {
            (0..g.nrows())
                .map(|i| g[(i, j)] * g[(i, j)])
                .sum::<f64>()
                .sqrt()
        })
        .fold(0.0, f64::max);
    let absolute_rank_floor = 100.0 * f64::EPSILON * relation_scale;
    let (u, c) = orthonormalize_pair(y, by, rank_tol, absolute_rank_floor, counters)?;
    if u.is_empty() {
        Ok(None)
    } else {
        counters.recycle_updates += 1;
        counters.recycle_vectors_selected += u.len() as u64;
        Ok(Some((u, c)))
    }
}

fn augmented_relation(
    u: &[Vec<f64>],
    c: &[Vec<f64>],
    v: &[Vec<f64>],
    h: &DenseMatrix,
    bc: &DenseMatrix,
    p: usize,
) -> CoreResult<AugmentedRelation> {
    if u.is_empty() {
        return Ok((v[..p].to_vec(), v[..p + 1].to_vec(), prefix(h, p + 1, p)));
    }
    let k = u.len();
    let mut vhat = Vec::with_capacity(k + p);
    let mut ddiag = Vec::with_capacity(k);
    for col in u {
        let n = safe_l2(col);
        if n <= f64::MIN_POSITIVE {
            return Err(CoreError::LinearSolve("degenerate recycle column".into()));
        }
        ddiag.push(1.0 / n);
        vhat.push(col.iter().map(|x| x / n).collect());
    }
    vhat.extend_from_slice(&v[..p]);
    let mut what = c.to_vec();
    what.extend_from_slice(&v[..p + 1]);
    let mut g = DenseMatrix::zeros(k + p + 1, k + p);
    for i in 0..k {
        g[(i, i)] = ddiag[i];
    }
    for i in 0..k {
        for j in 0..p {
            g[(i, k + j)] = bc[(i, j)];
        }
    }
    for i in 0..p + 1 {
        for j in 0..p {
            g[(k + i, k + j)] = h[(i, j)];
        }
    }
    Ok((vhat, what, g))
}
fn prefix(a: &DenseMatrix, r: usize, c: usize) -> DenseMatrix {
    let mut out = DenseMatrix::zeros(r, c);
    for i in 0..r {
        for j in 0..c {
            out[(i, j)] = a[(i, j)];
        }
    }
    out
}

/// Diagnostics of one GCRO-DR cycle (research node
/// `research/rnext03_gcrodr_attribution_20261003`). Computed with uncounted
/// plain loops; tracing does not change the solve.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GcrodrCycleTrace {
    /// Recycle rank `k` at the start of the cycle.
    pub recycle_rank: usize,
    /// True residual norm at the start of the cycle, before the projection.
    pub residual_start: f64,
    /// `||C^T r|| / ||r||` of the preconditioned residual before projection.
    pub retained_fraction: f64,
    /// True residual norm after the recycle projection (start if `k = 0`).
    pub residual_after_projection: f64,
    pub arnoldi_columns: usize,
    /// `||C^T r - R y||`-type residual of the small least-squares problem.
    pub least_squares_residual: f64,
    /// True residual norm after the cycle's correction.
    pub residual_end: f64,
    /// `max |C^T C - I|`, `max |C^T V|`, `max |V^T V - I|` for the recycle
    /// images `C` and the Arnoldi basis `V` of this cycle.
    pub recycle_gram_defect: f64,
    pub recycle_arnoldi_coupling: f64,
    pub arnoldi_gram_defect: f64,
    /// Operator applications charged during the cycle (Krylov, diagnostic
    /// and refresh), from the counters.
    pub matvecs: u64,
    /// The stagnation reset dropped the recycle space after this cycle.
    pub reset: bool,
    /// The solve aborted inside this cycle (research node
    /// `research/int01_gcrodr_verified_reuse_20261003`): only `recycle_rank`,
    /// `residual_start` and `matvecs` (charged since the cycle began) are set.
    pub aborted: bool,
}

/// One check of a carried recycle pair before its reuse on the same system
/// (see [`GcrodrReusePolicy::verify_reuse`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GcrodrReuseCheck {
    pub rank: usize,
    /// `max_j ||M^-1 A u_j - c_j||_2`.
    pub defect: f64,
    /// The defect exceeded the tolerance and the pair was rebuilt from the
    /// fresh images.
    pub rebuilt: bool,
}

/// A traced solve: its cycles and the operator applications charged
/// outside them (initial residual, refresh, final check).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GcrodrTrace {
    pub cycles: Vec<GcrodrCycleTrace>,
    pub matvecs_total: u64,
    pub matvecs_outside_cycles: u64,
    pub reuse_checks: Vec<GcrodrReuseCheck>,
    /// `(charged products, recycle rank, residual)` at the start of the
    /// cycle in progress; moved into `cycles` as an aborted cycle if the
    /// solve stops inside it.
    open_cycle: Option<(u64, usize, f64)>,
}

/// How a carried recycle pair is treated (research nodes
/// `research/rnext03_gcrodr_attribution_20261003` and
/// `research/int01_gcrodr_verified_reuse_20261003`). The default is the
/// plain GCRO-DR solve.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GcrodrReusePolicy {
    /// Stagnation reset factor `q` in (0, 1); see [`solve_gcrodr_traced`].
    pub reset_factor: Option<f64>,
    /// Check `M^-1 A U = C` before reusing a carried pair on the same system:
    /// each `u_j` is applied once (charged as refresh products) and, if
    /// `max_j ||M^-1 A u_j - c_j||_2` exceeds this tolerance, the pair is
    /// rebuilt from those images as after an operator change.
    pub verify_reuse: Option<f64>,
}

fn charged_matvecs(c: &WorkCounters) -> u64 {
    c.linear_matvecs + c.diagnostic_matvecs + c.recycle_refresh_matvecs
}

fn max_gram_defect(left: &[Vec<f64>], right: &[Vec<f64>], identity: bool) -> f64 {
    let mut worst = 0.0_f64;
    for (i, a) in left.iter().enumerate() {
        for (j, b) in right.iter().enumerate() {
            let product: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
            let target = if identity && i == j { 1.0 } else { 0.0 };
            worst = worst.max((product - target).abs());
        }
    }
    worst
}

/// GCRO-DR with a per-cycle trace and an optional stagnation reset
/// (research node `research/rnext03_gcrodr_attribution_20261003`). With
/// `reset_factor = Some(q)`: after a cycle that started with recycle rank
/// `k > 0` and reduced the true residual by less than the factor `q`
/// (`residual_end > q residual_start`), the recycle space is dropped; the
/// solve continues with rank 0 and GCRO-DR rebuilds a recycle space from
/// its own later cycles, to which the rule applies again. Every operator
/// application stays charged to the same budget and counters; dropped
/// vectors are counted in `recycle_dropped_vectors`. With `None` the solve
/// is exactly [`solve_gcrodr_with_workspace_and_residual_scale`].
#[allow(clippy::too_many_arguments)]
pub fn solve_gcrodr_traced(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    residual_scale: Option<&[f64]>,
    workspace: &mut GcrodrWorkspace,
    reset_factor: Option<f64>,
    trace: &mut GcrodrTrace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gcrodr_with_policy(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        residual_scale,
        workspace,
        GcrodrReusePolicy {
            reset_factor,
            verify_reuse: None,
        },
        trace,
        counters,
    )
}

/// GCRO-DR under a [`GcrodrReusePolicy`], traced. With the default policy
/// it is exactly [`solve_gcrodr_with_workspace_and_residual_scale`].
#[allow(clippy::too_many_arguments)]
pub fn solve_gcrodr_with_policy(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    residual_scale: Option<&[f64]>,
    workspace: &mut GcrodrWorkspace,
    policy: GcrodrReusePolicy,
    trace: &mut GcrodrTrace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    if policy.reset_factor.is_some_and(|q| !(q > 0.0 && q < 1.0)) {
        return Err(CoreError::InvalidInput(
            "GCRO-DR reset factor must lie in (0, 1)".into(),
        ));
    }
    if policy
        .verify_reuse
        .is_some_and(|tol| !(tol.is_finite() && tol > 0.0))
    {
        return Err(CoreError::InvalidInput(
            "GCRO-DR reuse tolerance must be finite and positive".into(),
        ));
    }
    *trace = GcrodrTrace::default();
    let start = charged_matvecs(counters);
    let result = solve_gcrodr_inner(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        residual_scale,
        workspace,
        policy,
        Some(&mut *trace),
        counters,
    );
    let now = charged_matvecs(counters);
    if let Some((cycle_start, rank, residual_start)) = trace.open_cycle.take() {
        trace.cycles.push(GcrodrCycleTrace {
            recycle_rank: rank,
            residual_start,
            matvecs: now - cycle_start,
            aborted: true,
            ..GcrodrCycleTrace::default()
        });
    }
    trace.matvecs_total = now - start;
    trace.matvecs_outside_cycles =
        trace.matvecs_total - trace.cycles.iter().map(|c| c.matvecs).sum::<u64>();
    result
}

// State, workspace, and work ledger have deliberately distinct lifetimes and commit rules.
#[allow(clippy::too_many_arguments)]
pub fn solve_gcrodr_with_workspace_and_residual_scale(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    residual_scale: Option<&[f64]>,
    workspace: &mut GcrodrWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gcrodr_inner(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        residual_scale,
        workspace,
        GcrodrReusePolicy::default(),
        None,
        counters,
    )
}

#[allow(clippy::too_many_arguments)]
fn solve_gcrodr_inner(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    residual_scale: Option<&[f64]>,
    workspace: &mut GcrodrWorkspace,
    policy: GcrodrReusePolicy,
    mut trace: Option<&mut GcrodrTrace>,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    if config.restart < 2
        || config.max_arnoldi == 0
        || config.recycle_dim == 0
        || config.recycle_dim >= config.restart
    {
        return Err(CoreError::InvalidInput("invalid GCRO-DR dimensions".into()));
    }
    validate_tolerances("GCRO-DR", config.rtol, config.atol)?;
    if !(config.rank_tol > 0.0 && config.rank_tol < 1.0) {
        return Err(CoreError::InvalidInput(
            "GCRO-DR rank_tol must lie in (0, 1)".into(),
        ));
    }
    let n = validate_system(op, pc, rhs, x0)?;
    validate_residual_scale(residual_scale, n)?;
    let right_norm = selected_residual_norm(rhs, residual_scale)?;
    let threshold = residual_threshold("GCRO-DR", config.rtol, config.atol, right_norm)?;
    let before = *counters;
    let snapshot = state.clone();
    let system_identity = exact_krylov_system_identity(op, pc);
    workspace.common.prepare(n);
    let result = (|| {
        let mut local = state.clone();
        local
            .basis
            .truncate(config.recycle_dim.min(config.restart - 1));
        local.image.truncate(local.basis.len());
        local.basis.retain(|vector| vector.len() == n);
        if local.image.len() != local.basis.len() {
            local.image.clear();
        }
        let same_system =
            system_identity.is_some() && local.system_identity.as_ref() == system_identity.as_ref();
        if !same_system {
            // A warm start solves the system it came from. After a dimension,
            // operator, or preconditioner change it would seed a different
            // system (audit F-010), so it is dropped exactly as LGMRES does.
            local.previous_solution = None;
        }
        if !local.basis.is_empty() {
            if system_identity.is_some()
                && local.system_identity.as_ref() == system_identity.as_ref()
                && !local.image.is_empty()
            {
                let mut rebuilt = false;
                if let Some(tol) = policy.verify_reuse {
                    let mut images = Vec::with_capacity(local.basis.len());
                    let mut defect = 0.0_f64;
                    for (basis_vector, carried) in local.basis.iter().zip(&local.image) {
                        let mut image = vec![0.0; n];
                        apply_left_with_raw(
                            op,
                            pc,
                            basis_vector,
                            &mut image,
                            &mut workspace.common.scratch_b,
                            counters,
                            ApplyCategory::Refresh,
                        )?;
                        let difference = image
                            .iter()
                            .zip(carried)
                            .map(|(a, b)| a - b)
                            .collect::<Vec<_>>();
                        defect = defect.max(safe_l2(&difference));
                        images.push(image);
                    }
                    // NaN compares false: a non-finite defect rebuilds too.
                    rebuilt = !(defect <= tol);
                    if let Some(trace) = trace.as_deref_mut() {
                        trace.reuse_checks.push(GcrodrReuseCheck {
                            rank: local.basis.len(),
                            defect,
                            rebuilt,
                        });
                    }
                    if rebuilt {
                        let (basis, image) = orthonormalize_pair(
                            local.basis.clone(),
                            images,
                            config.rank_tol,
                            0.0,
                            counters,
                        )?;
                        local.basis = basis;
                        local.image = image;
                    }
                }
                if !rebuilt {
                    counters.recycle_same_operator_uses += 1;
                }
            } else {
                let mut images = Vec::with_capacity(local.basis.len());
                for basis_vector in &local.basis {
                    let mut image = vec![0.0; n];
                    apply_left_with_raw(
                        op,
                        pc,
                        basis_vector,
                        &mut image,
                        &mut workspace.common.scratch_b,
                        counters,
                        ApplyCategory::Refresh,
                    )?;
                    images.push(image);
                }
                let (basis, image) = orthonormalize_pair(
                    local.basis.clone(),
                    images,
                    config.rank_tol,
                    0.0,
                    counters,
                )?;
                local.basis = basis;
                local.image = image;
                if !local.basis.is_empty() {
                    counters.recycle_cross_operator_refreshes += 1;
                }
            }
        }

        if let Some(initial) = x0.or(local.previous_solution.as_deref()) {
            // `x0` is validated by `validate_system`; only a corrupt state can trip this.
            if initial.len() != n {
                return Err(CoreError::Dimension(format!(
                    "GCRO-DR warm start has length {} but the system has dimension {n}",
                    initial.len()
                )));
            }
            workspace.common.x.copy_from_slice(initial);
        }
        let mut total = 0usize;
        loop {
            if workspace.common.x.iter().all(|value| *value == 0.0) {
                workspace.common.residual.copy_from_slice(rhs);
            } else {
                true_residual_into(
                    op,
                    rhs,
                    &workspace.common.x,
                    &mut workspace.common.operator_output,
                    &mut workspace.common.residual,
                    counters,
                    ApplyCategory::Krylov,
                )?;
            }
            let mut residual_norm =
                selected_residual_norm(&workspace.common.residual, residual_scale)?;
            if residual_norm <= threshold {
                break;
            }
            if total >= config.max_arnoldi {
                return Err(CoreError::LinearSolve(
                    "GCRO-DR Arnoldi budget exhausted".into(),
                ));
            }
            let cycle_start_matvecs = charged_matvecs(counters);
            let residual_start = residual_norm;
            let start_rank = local.basis.len();
            if let Some(trace) = trace.as_deref_mut() {
                trace.open_cycle = Some((cycle_start_matvecs, start_rank, residual_start));
            }
            rodas5p_core::apply_preconditioner(
                pc,
                &workspace.common.residual,
                &mut workspace.common.preconditioned,
                counters,
            )?;
            let preconditioned_norm = safe_l2(&workspace.common.preconditioned);
            let mut retained_fraction = 0.0;
            if !local.basis.is_empty() {
                workspace.common.coefficients.resize(local.image.len(), 0.0);
                for (coefficient, image) in
                    workspace.common.coefficients.iter_mut().zip(&local.image)
                {
                    *coefficient = image
                        .iter()
                        .zip(&workspace.common.preconditioned)
                        .map(|(left, right)| left * right)
                        .sum();
                }
                linear_combination_into(
                    &local.basis,
                    &workspace.common.coefficients,
                    &mut workspace.common.scratch_a,
                )?;
                axpy(
                    1.0,
                    &workspace.common.scratch_a,
                    &mut workspace.common.x,
                    counters,
                )?;
                linear_combination_into(
                    &local.image,
                    &workspace.common.coefficients,
                    &mut workspace.common.scratch_a,
                )?;
                axpy(
                    -1.0,
                    &workspace.common.scratch_a,
                    &mut workspace.common.preconditioned,
                    counters,
                )?;
                counters.recycle_projection_calls += 1;
                if trace.is_some() {
                    retained_fraction = safe_l2(&workspace.common.coefficients)
                        / preconditioned_norm.max(f64::MIN_POSITIVE);
                }
                true_residual_into(
                    op,
                    rhs,
                    &workspace.common.x,
                    &mut workspace.common.operator_output,
                    &mut workspace.common.residual,
                    counters,
                    ApplyCategory::Diagnostic,
                )?;
                residual_norm = selected_residual_norm(&workspace.common.residual, residual_scale)?;
                if residual_norm <= threshold {
                    if let Some(trace) = trace.as_deref_mut() {
                        trace.open_cycle = None;
                        trace.cycles.push(GcrodrCycleTrace {
                            recycle_rank: start_rank,
                            residual_start,
                            retained_fraction,
                            residual_after_projection: residual_norm,
                            residual_end: residual_norm,
                            matvecs: charged_matvecs(counters) - cycle_start_matvecs,
                            ..GcrodrCycleTrace::default()
                        });
                    }
                    break;
                }
                rodas5p_core::apply_preconditioner(
                    pc,
                    &workspace.common.residual,
                    &mut workspace.common.preconditioned,
                    counters,
                )?;
            }
            let residual_after_projection = residual_norm;
            let beta = safe_l2(&workspace.common.preconditioned);
            if beta <= f64::MIN_POSITIVE {
                return Err(CoreError::LinearSolve("GCRO-DR residual breakdown".into()));
            }
            let recycle_rank = local.basis.len();
            let maximum_columns = (config.restart - recycle_rank)
                .max(1)
                .min(config.max_arnoldi - total)
                .min(n.max(1));
            let mut first_basis = workspace.common.preconditioned.clone();
            for value in &mut first_basis {
                *value /= beta;
            }
            let mut arnoldi_basis = vec![first_basis];
            let mut hessenberg = DenseMatrix::zeros(maximum_columns + 1, maximum_columns);
            let mut recycle_coupling = DenseMatrix::zeros(recycle_rank, maximum_columns);
            let mut actual_columns = 0usize;
            for column in 0..maximum_columns {
                let mut next = vec![0.0; n];
                apply_left_with_raw(
                    op,
                    pc,
                    &arnoldi_basis[column],
                    &mut next,
                    &mut workspace.common.scratch_b,
                    counters,
                    ApplyCategory::Krylov,
                )?;
                for index in 0..recycle_rank {
                    let coefficient = dot(&local.image[index], &next, counters)?;
                    recycle_coupling[(index, column)] = coefficient;
                    axpy(-coefficient, &local.image[index], &mut next, counters)?;
                }
                let h_column = two_pass_mgs(&mut next, &arnoldi_basis, counters)?;
                for (row, &coefficient) in h_column.iter().enumerate() {
                    hessenberg[(row, column)] = coefficient;
                }
                let next_norm = safe_l2(&next);
                hessenberg[(column + 1, column)] = next_norm;
                actual_columns = column + 1;
                total += 1;
                counters.linear_iterations += 1;
                let full_projection_norm = (0..recycle_rank)
                    .map(|index| recycle_coupling[(index, column)])
                    .chain(h_column.iter().copied())
                    .fold(0.0_f64, f64::hypot);
                let happy_breakdown =
                    arnoldi_happy_breakdown_from_norm(full_projection_norm, next_norm)?;
                if !happy_breakdown {
                    for value in &mut next {
                        *value /= next_norm;
                    }
                    arnoldi_basis.push(next);
                } else {
                    arnoldi_basis.push(vec![0.0; n]);
                    break;
                }
            }
            let (augmented_basis, augmented_image, relation) = augmented_relation(
                &local.basis,
                &local.image,
                &arnoldi_basis,
                &hessenberg,
                &recycle_coupling,
                actual_columns,
            )?;
            let right_small: Vec<f64> = augmented_image
                .iter()
                .map(|image| {
                    image
                        .iter()
                        .zip(&workspace.common.preconditioned)
                        .map(|(left, right)| left * right)
                        .sum()
                })
                .collect();
            let small_solution = least_squares(&relation, &right_small)?;
            let traced_geometry = trace.is_some().then(|| {
                let v = &arnoldi_basis[..actual_columns];
                let fitted: f64 = (0..relation.nrows())
                    .map(|row| {
                        let value: f64 = (0..relation.ncols())
                            .map(|column| relation[(row, column)] * small_solution[column])
                            .sum();
                        (right_small[row] - value).powi(2)
                    })
                    .sum();
                (
                    fitted.sqrt(),
                    max_gram_defect(&local.image, &local.image, true),
                    max_gram_defect(&local.image, v, false),
                    max_gram_defect(v, v, true),
                )
            });
            linear_combination_into(
                &augmented_basis,
                &small_solution,
                &mut workspace.common.scratch_a,
            )?;
            axpy(
                1.0,
                &workspace.common.scratch_a,
                &mut workspace.common.x,
                counters,
            )?;
            true_residual_into(
                op,
                rhs,
                &workspace.common.x,
                &mut workspace.common.operator_output,
                &mut workspace.common.residual,
                counters,
                ApplyCategory::Diagnostic,
            )?;
            residual_norm = selected_residual_norm(&workspace.common.residual, residual_scale)?;
            if let Some((basis, image)) = update_recycle(
                &augmented_basis,
                &augmented_image,
                &relation,
                config.recycle_dim,
                config.rank_tol,
                counters,
            )? {
                local.basis = basis;
                local.image = image;
            }
            let reset = policy.reset_factor.is_some_and(|q| {
                start_rank > 0 && residual_norm > threshold && residual_norm > q * residual_start
            });
            if reset {
                counters.recycle_dropped_vectors += local.basis.len() as u64;
                local.basis.clear();
                local.image.clear();
            }
            if let (Some(trace), Some((ls, gram, coupling, arnoldi_gram))) =
                (trace.as_deref_mut(), traced_geometry)
            {
                trace.open_cycle = None;
                trace.cycles.push(GcrodrCycleTrace {
                    recycle_rank: start_rank,
                    residual_start,
                    retained_fraction,
                    residual_after_projection,
                    arnoldi_columns: actual_columns,
                    least_squares_residual: ls,
                    residual_end: residual_norm,
                    recycle_gram_defect: gram,
                    recycle_arnoldi_coupling: coupling,
                    arnoldi_gram_defect: arnoldi_gram,
                    matvecs: charged_matvecs(counters) - cycle_start_matvecs,
                    reset,
                    aborted: false,
                });
            }
            if residual_norm <= threshold {
                break;
            }
        }
        true_residual_into(
            op,
            rhs,
            &workspace.common.x,
            &mut workspace.common.operator_output,
            &mut workspace.common.residual,
            counters,
            ApplyCategory::Diagnostic,
        )?;
        let residual_norm = selected_residual_norm(&workspace.common.residual, residual_scale)?;
        if !residual_norm.is_finite() || residual_norm > threshold {
            return Err(CoreError::LinearSolve(format!(
                "GCRO-DR true residual {residual_norm:.3e} exceeds {threshold:.3e}"
            )));
        }
        local.operator_token = Some(op.token());
        local.system_identity = system_identity;
        local.previous_solution = Some(workspace.common.x.clone());
        local.generation += 1;
        *state = local;
        counters.linear_solves += 1;
        let delta = counters.delta(before);
        Ok(LinearSolveReport {
            x: workspace.common.x.clone(),
            converged: true,
            info: 0,
            residual_norm,
            relative_residual: residual_norm / right_norm.max(f64::MIN_POSITIVE),
            iterations: total as u64,
            matvecs: delta.linear_matvecs,
            preconditioner_apps: delta.preconditioner_apps,
            method: "gcrodr".into(),
        })
    })();
    if result.is_err() {
        *state = snapshot;
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn solve_gcrodr_with_workspace(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    workspace: &mut GcrodrWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gcrodr_with_workspace_and_residual_scale(
        op, pc, rhs, x0, config, state, None, workspace, counters,
    )
}

pub fn solve_gcrodr(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gcrodr_with_workspace(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        &mut GcrodrWorkspace::default(),
        counters,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn solve_gcrodr_with_residual_scale(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GcrodrConfig,
    state: &mut GcrodrState,
    residual_scale: Option<&[f64]>,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gcrodr_with_workspace_and_residual_scale(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        residual_scale,
        &mut GcrodrWorkspace::default(),
        counters,
    )
}
