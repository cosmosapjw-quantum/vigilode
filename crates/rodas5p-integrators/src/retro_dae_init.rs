//! Opt-in, source-owned semi-explicit polynomial index-1 initialization.
//! All admission concerns the declared fixed-(t,x) branch box. It does not
//! infer the index of an arbitrary singular mass matrix or prove evolution.
use crate::{
    ParallelExecution,
    retro_event_poly::{DeclaredPolynomial, RepresentedPolynomial, RootInterval, RootRoute},
    retro_foundation::{BatchMode, PreparedRhs},
    retro_portfolio::{Portfolio, Route, Selection},
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, LuSolveWorkspace, direct_solve,
    directed::{Interval, add_up, div_up, mul_up, sub_down},
};
use serde::Serialize;
type Iv = Interval;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("DAE initialization: {s}"))
}
fn p(v: f64) -> CoreResult<Iv> {
    Iv::point(v)
}
fn z() -> Iv {
    Iv { lo: 0., hi: 0. }
}
pub(crate) fn mid(v: Iv) -> f64 {
    (v.lo * 0.5 + v.hi * 0.5).clamp(v.lo, v.hi)
}
pub(crate) fn points(v: &[f64]) -> CoreResult<Vec<Iv>> {
    v.iter().map(|&x| p(x)).collect()
}
pub(crate) fn norm(v: &[Iv]) -> f64 {
    v.iter().map(Iv::mag).fold(0., f64::max)
}
fn distance(v: Iv, c: f64) -> CoreResult<f64> {
    Ok(v.sub(p(c)?)?.mag())
}
fn box_at(c: &[f64], r: f64) -> CoreResult<Vec<Iv>> {
    c.iter().map(|&x| p(x)?.add(Iv::new(-r, r)?)).collect()
}
pub(crate) fn subset(a: &[Iv], b: &[Iv]) -> bool {
    a.iter().zip(b).all(|(x, y)| y.lo <= x.lo && x.hi <= y.hi)
}
pub(crate) fn rows_norm(a: &[Vec<Iv>]) -> CoreResult<f64> {
    a.iter()
        .map(|r| r.iter().try_fold(0., |s, v| add_up(s, v.mag())))
        .try_fold(0.0_f64, |s, v| Ok(s.max(v?)))
}
pub(crate) fn matvec(a: &DenseMatrix, v: &[Iv]) -> CoreResult<Vec<Iv>> {
    (0..a.nrows())
        .map(|i| {
            a.row(i)
                .iter()
                .zip(v)
                .try_fold(z(), |s, (&c, &v)| s.add(v.scale(c)?))
        })
        .collect()
}
pub(crate) fn iv_matvec(a: &[Vec<Iv>], v: &[Iv]) -> CoreResult<Vec<Iv>> {
    a.iter()
        .map(|r| {
            r.iter()
                .zip(v)
                .try_fold(z(), |s, (&a, &b)| s.add(a.mul(b)?))
        })
        .collect()
}
pub(crate) fn defect(c: &DenseMatrix, j: &[Vec<Iv>]) -> CoreResult<Vec<Vec<Iv>>> {
    let n = c.nrows();
    (0..n)
        .map(|i| {
            (0..n)
                .map(|k| {
                    (0..n).try_fold(p(if i == k { 1. } else { 0. })?, |s, l| {
                        s.sub(j[l][k].scale(c[(i, l)])?)
                    })
                })
                .collect()
        })
        .collect()
}
#[derive(Clone)]
pub(crate) struct Eval {
    pub(crate) v: Vec<Iv>,
    pub(crate) j: Vec<Vec<Iv>>,
} // derivative columns: t, x, z
fn evaluate_legacy(polys: &[DeclaredPolynomial], t: Iv, y: &[Iv]) -> CoreResult<Eval> {
    let mut v = Vec::with_capacity(polys.len());
    let mut j = Vec::with_capacity(polys.len());
    for eq in polys {
        let mut sum = z();
        let mut row = vec![z(); 1 + y.len()];
        for term in eq.terms() {
            let vals = term
                .factors
                .iter()
                .map(|f| {
                    f.state
                        .iter()
                        .try_fold(p(f.constant)?.add(t.scale(f.time)?)?, |s, &(i, a)| {
                            s.add(y[i].scale(a)?)
                        })
                })
                .collect::<CoreResult<Vec<_>>>()?;
            let c = p(term.coefficient)?;
            sum = sum.add(vals.iter().try_fold(c, |s, &v| s.mul(v))?)?;
            for (i, f) in term.factors.iter().enumerate() {
                let other = vals
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i)
                    .try_fold(c, |s, (_, &v)| s.mul(v))?;
                row[0] = row[0].add(other.scale(f.time)?)?;
                for &(k, a) in &f.state {
                    row[k + 1] = row[k + 1].add(other.scale(a)?)?;
                }
            }
        }
        v.push(sum);
        j.push(row);
    }
    Ok(Eval { v, j })
}
/// Select the rounded output of the original declared source. All
/// derivatives are still evaluated, even when only values are requested,
/// preserving the original overflow/refusal contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEvalKind {
    Rhs,
    Jacobian,
    Time,
}

/// Caller-owned transactional staging for repeated point evaluations.
/// No heap allocation is needed to construct or reuse it.
pub struct SourceEvalWorkspace {
    staged: [f64; 32 * 32],
}
impl SourceEvalWorkspace {
    pub const fn new() -> Self {
        Self {
            staged: [0.; 32 * 32],
        }
    }
}
impl Default for SourceEvalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

// Exact same directed operations and iteration order as evaluate().
// The immutable validated DaeSource owns the dimension/factor caps.
// Do not skip a derivative because the requested output is RHS only.
pub(crate) fn evaluate_row_bounded(
    eq: &DeclaredPolynomial,
    t: Iv,
    y: &[Iv],
    row: &mut [Iv],
) -> CoreResult<Iv> {
    row.fill(z());
    let mut sum = z();
    let mut factors = [z(); 4];
    for term in eq.terms() {
        let vals = &mut factors[..term.factors.len()];
        for (v, f) in vals.iter_mut().zip(&term.factors) {
            *v = f
                .state
                .iter()
                .try_fold(p(f.constant)?.add(t.scale(f.time)?)?, |s, &(i, a)| {
                    s.add(y[i].scale(a)?)
                })?;
        }
        let c = p(term.coefficient)?;
        sum = sum.add(vals.iter().try_fold(c, |s, &v| s.mul(v))?)?;
        for (i, f) in term.factors.iter().enumerate() {
            let other = vals
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != i)
                .try_fold(c, |s, (_, &v)| s.mul(v))?;
            row[0] = row[0].add(other.scale(f.time)?)?;
            for &(k, a) in &f.state {
                row[k + 1] = row[k + 1].add(other.scale(a)?)?;
            }
        }
    }
    Ok(sum)
}
/// Owned interval result used by the original proof/projection paths.
pub(crate) fn evaluate(polys: &[DeclaredPolynomial], t: Iv, y: &[Iv]) -> CoreResult<Eval> {
    let mut v = Vec::with_capacity(polys.len());
    let mut j = Vec::with_capacity(polys.len());
    for eq in polys {
        let mut row = vec![z(); 1 + y.len()];
        v.push(evaluate_row_bounded(eq, t, y, &mut row)?);
        j.push(row);
    }
    Ok(Eval { v, j })
}

/// Transactional interval output staging. Derivative columns are t,x,z;
/// only the active prefix is used, and every active entry is overwritten.
pub struct IntervalEvalWorkspace {
    values: [Iv; 32],
    jacobian: [Iv; 32 * 33],
}
impl IntervalEvalWorkspace {
    pub const fn new() -> Self {
        const ZERO: Iv = Iv { lo: 0., hi: 0. };
        Self {
            values: [ZERO; 32],
            jacobian: [ZERO; 32 * 33],
        }
    }
}
impl Default for IntervalEvalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
/// Immutable polynomial source. f and g share state ordering [x,z].
#[derive(Clone, Debug, Serialize)]
pub struct DaeSource {
    pub(crate) f: Vec<DeclaredPolynomial>,
    pub(crate) g: Vec<DeclaredPolynomial>,
}
impl DaeSource {
    pub fn new(f: Vec<DeclaredPolynomial>, g: Vec<DeclaredPolynomial>) -> CoreResult<Self> {
        if f.is_empty() || g.is_empty() || f.len() + g.len() > 32 {
            return Err(bad("declared partition size"));
        }
        for e in f.iter().chain(&g) {
            e.validate(f.len() + g.len())?;
        }
        Ok(Self { f, g })
    }
    pub fn nx(&self) -> usize {
        self.f.len()
    }
    pub fn nz(&self) -> usize {
        self.g.len()
    }
    fn eval(&self, t: f64, x: &[f64], a: &[Iv]) -> CoreResult<(Eval, Eval)> {
        let n = self.nx() + self.nz();
        if x.len() != self.nx() || a.len() != self.nz() {
            return Err(bad("interval source partition shape"));
        }
        let mut y = [z(); 32];
        for (v, &x) in y[..self.nx()].iter_mut().zip(x) {
            *v = p(x)?;
        }
        y[self.nx()..n].copy_from_slice(a);
        Ok((
            evaluate(&self.f, p(t)?, &y[..n])?,
            evaluate(&self.g, p(t)?, &y[..n])?,
        ))
    }
    fn eval_point(&self, t: f64, x: &[f64], a: &[f64]) -> CoreResult<(Eval, Eval)> {
        if a.len() != self.nz() {
            return Err(bad("source algebraic shape"));
        }
        let mut b = [z(); 32];
        for (v, &x) in b[..a.len()].iter_mut().zip(a) {
            *v = p(x)?;
        }
        self.eval(t, x, &b[..a.len()])
    }
    fn jac_z(&self, g: &Eval) -> Vec<Vec<Iv>> {
        g.j.iter().map(|r| r[1 + self.nx()..].to_vec()).collect()
    }
    /// Ordinary rounded source evaluation for the existing mass driver.
    /// Its result is not by itself an interval certificate.
    pub fn constraint_upper(&self, t: f64, y: &[f64]) -> CoreResult<f64> {
        if y.len() != self.nx() + self.nz() {
            return Err(bad("state shape"));
        }
        let t = p(t)?;
        let n = y.len();
        let mut ys = [z(); 32];
        for (v, &x) in ys[..n].iter_mut().zip(y) {
            *v = p(x)?;
        }
        let mut row = [z(); 33];
        let mut upper = 0_f64;
        for eq in &self.g {
            upper = upper.max(evaluate_row_bounded(eq, t, &ys[..n], &mut row[..=n])?.mag());
        }
        Ok(upper)
    }
    fn interval_inputs(&self, t: Iv, y: &[Iv]) -> CoreResult<()> {
        if y.len() != self.nx() + self.nz() {
            return Err(bad("interval source state shape"));
        }
        Iv::new(t.lo, t.hi)?;
        for v in y {
            Iv::new(v.lo, v.hi)?;
        }
        Ok(())
    }
    /// Owned original-source interval evaluation. Values ordered f,g;
    /// derivative rows have columns t,x,z, using the same directed kernel
    /// as proof and projection. Not an existence certificate on its own.
    pub fn interval_values(&self, t: Iv, y: &[Iv]) -> CoreResult<(Vec<Iv>, Vec<Vec<Iv>>)> {
        self.interval_inputs(t, y)?;
        let n = y.len();
        let mut values = Vec::with_capacity(n);
        let mut jac = Vec::with_capacity(n);
        for eq in self.f.iter().chain(&self.g) {
            let mut row = vec![z(); n + 1];
            values.push(evaluate_row_bounded(eq, t, y, &mut row)?);
            jac.push(row);
        }
        Ok((values, jac))
    }
    /// Full value and row-major derivative enclosure, committed only on
    /// success. Caller workspace has no cached physical/source authority.
    /// Successful calls allocate no heap; error diagnostics may allocate.
    pub fn interval_into(
        &self,
        t: Iv,
        y: &[Iv],
        values: &mut [Iv],
        jacobian: &mut [Iv],
        work: &mut IntervalEvalWorkspace,
    ) -> CoreResult<()> {
        self.interval_inputs(t, y)?;
        let n = y.len();
        let stride = n + 1;
        if values.len() != n || jacobian.len() != n * stride {
            return Err(bad("interval source output shape"));
        }
        self.fill_interval_workspace(t, y, work)?;
        values.copy_from_slice(&work.values[..n]);
        jacobian.copy_from_slice(&work.jacobian[..n * stride]);
        Ok(())
    }
    // Shared staging kernel; no cached source authority. All rows overwritten.
    fn fill_interval_workspace(
        &self,
        t: Iv,
        y: &[Iv],
        work: &mut IntervalEvalWorkspace,
    ) -> CoreResult<()> {
        self.interval_inputs(t, y)?;
        let stride = y.len() + 1;
        for (i, eq) in self.f.iter().chain(&self.g).enumerate() {
            work.values[i] =
                evaluate_row_bounded(eq, t, y, &mut work.jacobian[i * stride..(i + 1) * stride])?;
        }
        Ok(())
    }
    /// Rounded original-source output, written atomically only after all
    /// equations and derivatives succeed. Output order matches the Vec API;
    /// Jacobian is row-major in [x,z] order. Successful bounded evaluation
    /// requires no heap allocations; diagnostics on error may allocate.
    fn point_into(
        &self,
        t: f64,
        y: &[f64],
        kind: SourceEvalKind,
        out: &mut [f64],
        staged: &mut [f64],
    ) -> CoreResult<()> {
        let n = self.nx() + self.nz();
        if y.len() != n {
            return Err(bad("state shape"));
        }
        let len = if kind == SourceEvalKind::Jacobian {
            n * n
        } else {
            n
        };
        if out.len() != len || staged.len() < len {
            return Err(bad("source output shape"));
        }
        let mut points = [z(); 32];
        for (v, &x) in points[..n].iter_mut().zip(y) {
            *v = p(x)?;
        }
        let t = p(t)?;
        let mut row = [z(); 33];
        for (i, eq) in self.f.iter().chain(&self.g).enumerate() {
            let v = evaluate_row_bounded(eq, t, &points[..n], &mut row[..=n])?;
            match kind {
                SourceEvalKind::Rhs => staged[i] = mid(v),
                SourceEvalKind::Time => staged[i] = mid(row[0]),
                SourceEvalKind::Jacobian => {
                    for (d, &v) in staged[i * n..(i + 1) * n].iter_mut().zip(&row[1..=n]) {
                        *d = mid(v);
                    }
                }
            }
        }
        out.copy_from_slice(&staged[..len]);
        Ok(())
    }
    /// Reuse caller-owned staging for any rounded original-source output.
    pub fn evaluate_into(
        &self,
        t: f64,
        y: &[f64],
        kind: SourceEvalKind,
        out: &mut [f64],
        work: &mut SourceEvalWorkspace,
    ) -> CoreResult<()> {
        self.point_into(t, y, kind, out, &mut work.staged)
    }
    /// Stack-staged RHS (including the algebraic residual rows).
    pub fn rhs_into(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.point_into(t, y, SourceEvalKind::Rhs, out, &mut [0.; 32])
    }
    /// Stack-staged row-major full source Jacobian.
    pub fn jacobian_into(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.point_into(t, y, SourceEvalKind::Jacobian, out, &mut [0.; 32 * 32])
    }
    /// Stack-staged explicit source time derivative.
    pub fn time_into(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.point_into(t, y, SourceEvalKind::Time, out, &mut [0.; 32])
    }
    pub fn rhs_values(&self, t: f64, y: &[f64]) -> CoreResult<Vec<f64>> {
        if y.len() != self.nx() + self.nz() {
            return Err(bad("state shape"));
        }
        let y = points(y)?;
        let a = evaluate_legacy(&self.f, p(t)?, &y)?;
        let b = evaluate_legacy(&self.g, p(t)?, &y)?;
        Ok(a.v.iter().chain(&b.v).copied().map(mid).collect())
    }
    pub fn jacobian_values(&self, t: f64, y: &[f64]) -> CoreResult<Vec<f64>> {
        if y.len() != self.nx() + self.nz() {
            return Err(bad("state shape"));
        }
        let y = points(y)?;
        let a = evaluate_legacy(&self.f, p(t)?, &y)?;
        let b = evaluate_legacy(&self.g, p(t)?, &y)?;
        Ok(a.j
            .iter()
            .chain(&b.j)
            .flat_map(|r| r[1..].iter().copied().map(mid))
            .collect())
    }
    pub fn time_values(&self, t: f64, y: &[f64]) -> CoreResult<Vec<f64>> {
        if y.len() != self.nx() + self.nz() {
            return Err(bad("state shape"));
        }
        let y = points(y)?;
        let a = evaluate_legacy(&self.f, p(t)?, &y)?;
        let b = evaluate_legacy(&self.g, p(t)?, &y)?;
        Ok(a.j.iter().chain(&b.j).map(|r| mid(r[0])).collect())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum InitMode {
    Strict,
    ProjectAlgebraic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum InitRoute {
    Direct,
    Reuse,
    FaerBatch,
    PreparedSerial,
    PreparedBatch,
    PreparedRayon,
    ScalarRoot,
    /// Explicit opt-in: exact frozen degree-one root, no automatic fallback.
    AffineExact,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct InitPolicy {
    pub mode: InitMode,
    pub state_tol: f64,
    pub residual_tol: f64,
    pub derivative_tol: f64,
    pub condition_cap: f64,
    pub max_newton: usize,
}
impl Default for InitPolicy {
    fn default() -> Self {
        Self {
            mode: InitMode::Strict,
            state_tol: 1e-9,
            residual_tol: 1e-9,
            derivative_tol: 1e-7,
            condition_cap: 1e10,
            max_newton: 32,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct InitWork {
    pub evaluations: usize,
    pub newton_steps: usize,
    pub linear_batches: usize,
    pub rhs_count: usize,
    pub prepared_owners: usize,
    pub prepared_support_batches: usize,
    pub prepared_residual_terms: usize,
    pub faer_batches: usize,
    pub rayon_batches: usize,
    pub root_calls: usize,
    pub dictionary_uses: usize,
    pub conditioned_horner_uses: usize,
}
/// Boundary calls, not arithmetic inside failed kernels. A returned
/// call may still be physically unqualified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AttemptCount {
    pub started: usize,
    pub returned: usize,
}
impl AttemptCount {
    pub fn record(&mut self, success: bool) {
        self.started += 1;
        self.returned += usize::from(success);
    }
    pub fn merge(&mut self, b: Self) {
        self.started += b.started;
        self.returned += b.returned;
    }
}
/// Retained on Err. Linear RHS requests are not completed solves.
#[derive(Clone, Debug, Default, Serialize)]
pub struct InitAttemptWork {
    pub admitted: bool,
    pub evaluations: AttemptCount,
    pub frozen: AttemptCount,
    pub roots: AttemptCount,
    pub linear: AttemptCount,
    pub rhs_requested: usize,
    pub rhs_returned: usize,
    pub legacy: InitWork,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ProjectionAttemptWork {
    pub calls: AttemptCount,
    pub admitted: usize,
    pub evaluations: AttemptCount,
    pub frozen: AttemptCount,
    pub roots: AttemptCount,
    pub linear: AttemptCount,
    pub rhs_requested: usize,
    pub rhs_returned: usize,
    pub successful_rhs_requested: usize,
    pub refused_rhs_requested: usize,
}
impl ProjectionAttemptWork {
    pub fn record(&mut self, w: &InitAttemptWork, success: bool) {
        self.calls.record(success);
        self.admitted += usize::from(w.admitted);
        self.evaluations.merge(w.evaluations);
        self.frozen.merge(w.frozen);
        self.roots.merge(w.roots);
        self.linear.merge(w.linear);
        self.rhs_requested += w.rhs_requested;
        self.rhs_returned += w.rhs_returned;
        if success {
            self.successful_rhs_requested += w.rhs_requested;
        } else {
            self.refused_rhs_requested += w.rhs_requested;
        }
    }
    pub fn merge(&mut self, w: &Self) {
        self.calls.merge(w.calls);
        self.admitted += w.admitted;
        self.evaluations.merge(w.evaluations);
        self.frozen.merge(w.frozen);
        self.roots.merge(w.roots);
        self.linear.merge(w.linear);
        self.rhs_requested += w.rhs_requested;
        self.rhs_returned += w.rhs_returned;
        self.successful_rhs_requested += w.successful_rhs_requested;
        self.refused_rhs_requested += w.refused_rhs_requested;
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct InitReport {
    pub route: InitRoute,
    pub mode: InitMode,
    pub t: f64,
    pub x: Vec<f64>,
    pub original_z: Vec<f64>,
    pub z: Vec<f64>,
    pub correction_upper: f64,
    pub branch: Vec<[f64; 2]>,
    pub root_box: Vec<[f64; 2]>,
    pub branch_authority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affine_root: Option<RootInterval>,
    pub original_residual_upper: f64,
    pub residual_upper: f64,
    pub state_error_upper: f64,
    pub exact_center_consistency: bool,
    pub branch_defect_upper: Option<f64>,
    pub local_defect_upper: f64,
    pub inverse_norm_upper: f64,
    pub condition_upper: f64,
    pub rank: usize,
    pub xdot: Vec<f64>,
    pub zdot: Vec<f64>,
    pub zt: Vec<f64>,
    pub zx_columns: Vec<Vec<f64>>,
    pub reduced_jacobian: Vec<Vec<f64>>,
    pub derivative_error_upper: f64,
    pub tangent_residual_upper: f64,
    pub work: InitWork,
    pub qualified: bool,
}
pub(crate) fn center_matrix(j: &[Vec<Iv>]) -> CoreResult<DenseMatrix> {
    DenseMatrix::new(
        j.len(),
        j.len(),
        j.iter().flat_map(|r| r.iter().copied().map(mid)).collect(),
    )
}
fn batch(
    a: &DenseMatrix,
    rhs: &[Vec<f64>],
    route: InitRoute,
    exec: &ParallelExecution,
    w: &mut InitWork,
) -> CoreResult<Vec<Vec<f64>>> {
    w.linear_batches += 1;
    w.rhs_count += rhs.len();
    match route {
        InitRoute::Direct | InitRoute::ScalarRoot | InitRoute::AffineExact => {
            rhs.iter().map(|b| direct_solve(a, b)).collect()
        }
        InitRoute::Reuse => {
            let lu = LuFactorization::new(a)?;
            let mut scratch = LuSolveWorkspace::new(a.nrows());
            let mut x = rhs.to_vec();
            for r in &mut x {
                lu.solve_in_place_with_workspace(r, &mut scratch)?;
            }
            Ok(x)
        }
        InitRoute::FaerBatch => {
            w.faer_batches += 1;
            LuFactorization::new(a)?.solve_rows(rhs)
        }
        _ => {
            let mut owner = PreparedRhs::new(a.clone())?;
            w.prepared_owners += 1;
            let mode = match route {
                InitRoute::PreparedSerial => BatchMode::SerialReuse,
                InitRoute::PreparedBatch => {
                    w.faer_batches += 1;
                    BatchMode::FaerBatch
                }
                _ => {
                    w.rayon_batches += 1;
                    BatchMode::RayonChunks
                }
            };
            let r = owner.solve(rhs, mode, exec)?;
            if r.residual_mode == crate::retro_foundation::ResidualMode::ExactSupport {
                w.prepared_support_batches += 1;
            }
            w.prepared_residual_terms += r.residual_terms;
            Ok(r.values)
        }
    }
}
pub(crate) fn preconditioner(
    a: &DenseMatrix,
    route: InitRoute,
    exec: &ParallelExecution,
    w: &mut InitWork,
) -> CoreResult<DenseMatrix> {
    let n = a.nrows();
    let rhs = (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1. } else { 0. }).collect())
        .collect::<Vec<_>>();
    DenseMatrix::from_vec_rows(batch(a, &rhs, route, exec, w)?).map(|r| r.transpose())
}
// Only scratch, never an authoritative source or matrix owner. All active
// entries are written before use, including after failed initialization.
// nx+nz <=32, hence (nx+2)*nz <= 289 (max at nx=15,nz=17).
const PROJ_DIM: usize = 32;
const PROJ_COLUMNS: usize = 289;
pub struct ProjectionWorkspace {
    a: [Iv; PROJ_DIM],
    b: [Iv; PROJ_DIM],
    c: [Iv; PROJ_DIM],
    d: [Iv; PROJ_DIM],
    rhs: [Iv; PROJ_COLUMNS],
    enclosures: [Iv; PROJ_COLUMNS],
}
impl ProjectionWorkspace {
    pub const fn new() -> Self {
        const ZERO: Iv = Iv { lo: 0., hi: 0. };
        Self {
            a: [ZERO; PROJ_DIM],
            b: [ZERO; PROJ_DIM],
            c: [ZERO; PROJ_DIM],
            d: [ZERO; PROJ_DIM],
            rhs: [ZERO; PROJ_COLUMNS],
            enclosures: [ZERO; PROJ_COLUMNS],
        }
    }
}
impl Default for ProjectionWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
fn matvec_into(a: &DenseMatrix, v: &[Iv], out: &mut [Iv]) -> CoreResult<()> {
    if a.ncols() != v.len() || a.nrows() != out.len() {
        return Err(bad("projection matvec shape"));
    }
    for (i, x) in out.iter_mut().enumerate() {
        *x = a
            .row(i)
            .iter()
            .zip(v)
            .try_fold(z(), |s, (&c, &v)| s.add(v.scale(c)?))?;
    }
    Ok(())
}
fn iv_matvec_into(a: &[Vec<Iv>], v: &[Iv], out: &mut [Iv]) -> CoreResult<()> {
    if a.len() != out.len() || a.iter().any(|r| r.len() != v.len()) {
        return Err(bad("projection interval matvec shape"));
    }
    for (r, x) in a.iter().zip(out) {
        *x = r
            .iter()
            .zip(v)
            .try_fold(z(), |s, (&a, &b)| s.add(a.mul(b)?))?;
    }
    Ok(())
}
fn verified_root_box(
    c: &DenseMatrix,
    g: &[Iv],
    j: &[Vec<Iv>],
    center: &[f64],
    branch: &[Iv],
    scratch: &mut ProjectionWorkspace,
) -> CoreResult<(Vec<Iv>, f64)> {
    let n = center.len();
    let e = defect(c, j)?;
    let q = rows_norm(&e)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("whole branch contraction not proved"));
    }
    let cg = &mut scratch.a[..n];
    matvec_into(c, g, cg)?;
    let eta = norm(cg);
    // For rounded box endpoints use explicit componentwise self-map
    // inclusion, not a floating-point assertion of eta/(1-q).
    let r = mul_up(2., div_up(eta, gap)?)?;
    let b = box_at(center, r)?;
    if !subset(&b, branch) {
        return Err(bad("existence box leaves declared branch"));
    }
    let delta = &mut scratch.b[..n];
    for (d, (&v, &c)) in delta.iter_mut().zip(b.iter().zip(center)) {
        *d = v.sub(p(c)?)?;
    }
    let ed = &mut scratch.c[..n];
    iv_matvec_into(&e, delta, ed)?;
    let mapped = &mut scratch.d[..n];
    for (i, x) in mapped.iter_mut().enumerate() {
        *x = p(center[i])?.sub(cg[i])?.add(ed[i])?;
    }
    if !subset(mapped, &b) {
        return Err(bad("local self-map inclusion not proved"));
    }
    Ok((b, q))
}
impl DaeSource {
    /// Explicit policy and branch. All returned centers are numerical; exact
    /// equality is separately reported when interval residual is zero.
    pub fn initialize(
        &self,
        t: f64,
        x: &[f64],
        z0: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        route: InitRoute,
    ) -> CoreResult<InitReport> {
        self.initialize_with_workspace(
            t,
            x,
            z0,
            branch,
            policy,
            route,
            &mut ProjectionWorkspace::new(),
        )
    }
    /// Same original source proof and owned result; caller supplies reusable
    /// temporary interval columns. No stale factor/source authority is retained.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_with_workspace(
        &self,
        t: f64,
        x: &[f64],
        z0: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        route: InitRoute,
        scratch: &mut ProjectionWorkspace,
    ) -> CoreResult<InitReport> {
        self.initialize_recorded(
            t,
            x,
            z0,
            branch,
            policy,
            route,
            scratch,
            &mut InitAttemptWork::default(),
        )
    }
    /// Original initialization plus boundary-work evidence on success or refusal.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_audited(
        &self,
        t: f64,
        x: &[f64],
        z0: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        route: InitRoute,
    ) -> (CoreResult<InitReport>, InitAttemptWork) {
        let mut a = InitAttemptWork::default();
        let r = self.initialize_recorded(
            t,
            x,
            z0,
            branch,
            policy,
            route,
            &mut ProjectionWorkspace::new(),
            &mut a,
        );
        (r, a)
    }
    #[allow(clippy::too_many_arguments)]
    fn initialize_recorded(
        &self,
        t: f64,
        x: &[f64],
        z0: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        route: InitRoute,
        scratch: &mut ProjectionWorkspace,
        attempt: &mut InitAttemptWork,
    ) -> CoreResult<InitReport> {
        macro_rules! observed {
            ($field:ident,$e:expr) => {{
                let r = (|| $e)();
                attempt.$field.record(r.is_ok());
                r
            }};
        }
        macro_rules! linear {
            ($n:expr,$e:expr) => {{
                let n = $n;
                attempt.rhs_requested += n;
                let r = (|| $e)();
                attempt.linear.record(r.is_ok());
                if r.is_ok() {
                    attempt.rhs_returned += n;
                }
                r
            }};
        }
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx = self.nx();
        let nz = self.nz();
        if !t.is_finite()
            || x.len() != nx
            || z0.len() != nz
            || branch.len() != nz
            || x.iter().chain(z0).any(|v| !v.is_finite())
            || [
                policy.state_tol,
                policy.residual_tol,
                policy.derivative_tol,
                policy.condition_cap,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
            || !(1..=64).contains(&policy.max_newton)
        {
            return Err(bad("policy/input shape"));
        }
        let b = branch
            .iter()
            .map(|&[l, h]| Iv::new(l, h))
            .collect::<CoreResult<Vec<_>>>()?;
        if b.iter()
            .zip(z0)
            .any(|(v, &c)| v.lo >= v.hi || !v.contains(c))
        {
            return Err(bad("invalid branch or initial guess outside"));
        }
        if matches!(route, InitRoute::ScalarRoot | InitRoute::AffineExact) && nz != 1 {
            return Err(bad("scalar root route requires nz=1"));
        }
        attempt.admitted = true;
        let exec = if route == InitRoute::PreparedRayon {
            ParallelExecution::rayon(4)?
        } else {
            ParallelExecution::sequential()
        };
        let w = &mut attempt.legacy;
        let (_, g0) = observed!(evaluations, self.eval_point(t, x, z0))?;
        w.evaluations += 1;
        let original_residual_upper = norm(&g0.v);
        if policy.mode == InitMode::Strict && original_residual_upper > policy.residual_tol {
            return Err(bad("strict initial residual exceeds tolerance"));
        }
        let mut center = z0.to_vec();
        let mut scalar_box = None;
        let mut affine_root = None;
        if matches!(route, InitRoute::ScalarRoot | InitRoute::AffineExact) {
            for v in [b[0].lo, b[0].hi] {
                let (_, g) = observed!(evaluations, self.eval(t, x, &[p(v)?]))?;
                w.evaluations += 1;
                if g.v[0].contains_zero() {
                    return Err(bad("scalar branch boundary root/unknown"));
                }
            }
            let poly = observed!(
                frozen,
                RepresentedPolynomial::frozen_scalar(&self.g[0], t, x, branch[0])
            )?;
            let [l, h] = if route == InitRoute::AffineExact {
                let r = observed!(roots, poly.isolate_affine_exact(2f64.powi(-48)))?
                    .ok_or_else(|| bad("affine branch not exactly one interior root"))?;
                w.root_calls += 1;
                let time = r.time;
                affine_root = Some(r);
                time
            } else {
                let roots = observed!(
                    roots,
                    poly.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-48))
                )?;
                w.root_calls += 1;
                w.dictionary_uses += roots.work.dictionary_uses;
                w.conditioned_horner_uses += roots.work.conditioned_horner_uses;
                if roots.roots.len() != 1 || roots.roots[0].multiplicity != 1 {
                    return Err(bad("scalar branch not exactly one simple root"));
                }
                roots.roots[0].time
            };
            let q = Iv::new(l, h)?;
            if !subset(&[q], &b) {
                return Err(bad("scalar root box leaves branch"));
            }
            if policy.mode == InitMode::ProjectAlgebraic {
                center[0] = mid(q);
                // Polish the numerical center, not the exact isolation
                // certificate. Only admit an improving point in the box.
                let (_, v) = observed!(evaluations, self.eval(t, x, &[p(center[0])?]))?;
                w.evaluations += 1;
                let d = mid(v.j[0][1 + nx]);
                if d != 0. {
                    let trial = center[0] - mid(v.v[0]) / d;
                    if q.contains(trial) {
                        let (_, g) = observed!(evaluations, self.eval(t, x, &[p(trial)?]))?;
                        w.evaluations += 1;
                        if norm(&g.v) < norm(&v.v) {
                            center[0] = trial;
                            w.newton_steps += 1;
                        }
                    }
                }
            }
            scalar_box = Some(vec![q]);
        } else if policy.mode == InitMode::ProjectAlgebraic {
            // Newton is a proposal mechanism. Neither a small rounded
            // correction nor a successful factorization authorizes a root.
            for _ in 0..policy.max_newton {
                let (_, g) = observed!(evaluations, self.eval_point(t, x, &center))?;
                w.evaluations += 1;
                let r = norm(&g.v);
                if r == 0. {
                    break;
                }
                let a = center_matrix(&self.jac_z(&g))?;
                let rhs = g.v.iter().copied().map(|v| -mid(v)).collect::<Vec<_>>();
                let d = linear!(1, batch(&a, &[rhs], route, &exec, w))?.remove(0);
                let mut accepted = false;
                for j in 0..16 {
                    let s = 2f64.powi(-j);
                    let mut trial = [0.; 32];
                    let mut trial_box = [z(); 32];
                    for (i, (&v, &d)) in center.iter().zip(&d).enumerate() {
                        trial[i] = s.mul_add(d, v);
                        trial_box[i] = p(trial[i])?;
                    }
                    if !subset(&trial_box[..nz], &b) {
                        continue;
                    }
                    let (_, v) = observed!(evaluations, self.eval(t, x, &trial_box[..nz]))?;
                    w.evaluations += 1;
                    if norm(&v.v) < r {
                        center.copy_from_slice(&trial[..nz]);
                        accepted = true;
                        w.newton_steps += 1;
                        break;
                    }
                }
                if !accepted {
                    break;
                }
            }
        }
        let (fc, gc) = observed!(evaluations, self.eval_point(t, x, &center))?;
        w.evaluations += 1;
        let residual_upper = norm(&gc.v);
        if residual_upper > policy.residual_tol {
            return Err(bad("original constraint residual exceeds tolerance"));
        }
        let a = center_matrix(&self.jac_z(&gc))?;
        let c = linear!(nz, preconditioner(&a, route, &exec, w))?;
        let (root, branch_defect_upper, authority) = if let Some(r) = scalar_box {
            let label = if affine_root.is_some() {
                "exact affine scalar root"
            } else {
                "exact scalar root count"
            };
            (r, None, label.to_string())
        } else {
            let (_, g) = observed!(evaluations, self.eval(t, x, &b))?;
            w.evaluations += 1;
            let (r, q) = verified_root_box(&c, &gc.v, &self.jac_z(&g), &center, &b, scratch)?;
            (
                r,
                Some(q),
                "whole-branch contraction + local self-map".to_string(),
            )
        };
        let state_error_upper = root.iter().zip(&center).try_fold(0_f64, |s, (&v, &c)| {
            Ok::<f64, CoreError>(s.max(distance(v, c)?))
        })?;
        if state_error_upper > policy.state_tol {
            return Err(bad("certified root distance exceeds tolerance"));
        }
        let (fr, gr) = observed!(evaluations, self.eval(t, x, &root))?;
        w.evaluations += 1;
        let j = self.jac_z(&gr);
        let q = rows_norm(&defect(&c, &j)?)?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("rank/regularity not proved at root"));
        }
        let mut cnorm = 0_f64;
        for i in 0..nz {
            let v = c
                .row(i)
                .iter()
                .try_fold(0_f64, |s, &v| add_up(s, p(v)?.mag()))?;
            cnorm = cnorm.max(v);
        }
        let inverse_norm_upper = div_up(cnorm, gap)?;
        let condition_upper = mul_up(inverse_norm_upper, rows_norm(&j)?)?;
        if condition_upper > policy.condition_cap {
            return Err(bad("certified condition exceeds cap"));
        }
        // actual root RHS columns: z_t, each z_x column, and z_dot.
        let ncols = nx + 2;
        let len = ncols * nz;
        let rhs = &mut scratch.rhs[..len];
        for col in 0..=nx {
            for (i, r) in gr.j.iter().enumerate() {
                rhs[col * nz + i] = -r[col];
            }
        }
        for (i, r) in gr.j.iter().enumerate() {
            rhs[(nx + 1) * nz + i] = r[1..1 + nx]
                .iter()
                .zip(&fr.v)
                .try_fold(-r[0], |s, (&g, &f)| s.sub(g.mul(f)?))?;
        }
        // Centers are evaluated from the same immutable original source.
        let xdot = fc.v.iter().copied().map(mid).collect::<Vec<_>>();
        let mut br = Vec::with_capacity(nx + 2);
        br.push(gc.j.iter().map(|r| -mid(r[0])).collect::<Vec<_>>());
        for i in 0..nx {
            br.push(gc.j.iter().map(|r| -mid(r[i + 1])).collect());
        }
        br.push(
            gc.j.iter()
                .map(|r| {
                    r[1..1 + nx]
                        .iter()
                        .zip(&xdot)
                        .try_fold(-r[0], |s, (&g, &f)| s.sub(g.scale(f)?))
                        .map(mid)
                })
                .collect::<CoreResult<Vec<_>>>()?,
        );
        let sol = linear!(br.len(), batch(&a, &br, route, &exec, w))?;
        let enc = &mut scratch.enclosures[..len];
        let points = &mut scratch.a[..nz];
        let av = &mut scratch.b[..nz];
        let residual = &mut scratch.c[..nz];
        let pc = &mut scratch.d[..nz];
        let mut derivative_error_upper = 0_f64;
        for (col, (b, v)) in rhs.chunks_exact(nz).zip(&sol).enumerate() {
            for (a, &x) in points.iter_mut().zip(v) {
                *a = p(x)?;
            }
            iv_matvec_into(&j, points, av)?;
            for (r, (&b, &a)) in residual.iter_mut().zip(b.iter().zip(av.iter())) {
                *r = b.sub(a)?;
            }
            matvec_into(&c, residual, pc)?;
            let e = div_up(norm(pc), gap)?;
            derivative_error_upper = derivative_error_upper.max(e);
            for (a, &x) in enc[col * nz..(col + 1) * nz].iter_mut().zip(v) {
                *a = p(x)?.add(Iv::new(-e, e)?)?;
            }
        }
        for (&v, &f) in xdot.iter().zip(&fr.v) {
            derivative_error_upper = derivative_error_upper.max(distance(f, v)?);
        }
        let mut reduced_jacobian = vec![vec![0.; nx]; nx];
        for (i, row) in reduced_jacobian.iter_mut().enumerate() {
            for (k, entry) in row.iter_mut().enumerate() {
                let v = fc.j[i][1 + nx..]
                    .iter()
                    .zip(&sol[1 + k])
                    .try_fold(fc.j[i][k + 1], |s, (&fz, &v)| s.add(fz.scale(v)?))?;
                let exact = fr.j[i][1 + nx..]
                    .iter()
                    .zip(&enc[(1 + k) * nz..(2 + k) * nz])
                    .try_fold(fr.j[i][k + 1], |s, (&fz, &v)| s.add(fz.mul(v)?))?;
                let m = mid(v);
                *entry = m;
                derivative_error_upper = derivative_error_upper.max(distance(exact, m)?);
            }
        }
        let zdot = sol.last().unwrap().clone();
        let tangent_residual_upper = gc.j.iter().try_fold(0_f64, |s, r| {
            let v = r[1..1 + nx]
                .iter()
                .zip(&xdot)
                .chain(r[1 + nx..].iter().zip(&zdot))
                .try_fold(r[0], |s, (&a, &v)| s.add(a.scale(v)?))?;
            Ok::<f64, CoreError>(s.max(v.mag()))
        })?;
        if derivative_error_upper > policy.derivative_tol
            || tangent_residual_upper > policy.derivative_tol
        {
            return Err(bad("derivative/Schur certificate exceeds tolerance"));
        }
        let correction_upper = center.iter().zip(z0).try_fold(0_f64, |s, (&a, &b)| {
            Ok::<f64, CoreError>(s.max(p(a)?.sub(p(b)?)?.mag()))
        })?;
        Ok(InitReport {
            route,
            mode: policy.mode,
            t,
            x: x.to_vec(),
            original_z: z0.to_vec(),
            z: center,
            correction_upper,
            branch: branch.to_vec(),
            root_box: root.iter().map(|v| v.as_pair()).collect(),
            branch_authority: authority,
            affine_root,
            original_residual_upper,
            residual_upper,
            state_error_upper,
            exact_center_consistency: residual_upper == 0.,
            branch_defect_upper,
            local_defect_upper: q,
            inverse_norm_upper,
            condition_upper,
            rank: nz,
            xdot,
            zdot,
            zt: sol[0].clone(),
            zx_columns: sol[1..1 + nx].to_vec(),
            reduced_jacobian,
            derivative_error_upper,
            tangent_residual_upper,
            work: w.clone(),
            qualified: true,
        })
    }
    /// RETRO cost history is advisory only: every candidate re-certifies
    /// the same source and branch. State and derivative budgets remain hard.
    pub fn adaptive(
        &self,
        t: f64,
        x: &[f64],
        z: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        portfolio: &mut Portfolio,
    ) -> CoreResult<Selection<InitReport>> {
        let mut routes = vec![
            Route::DaeReuse,
            Route::DaeDirect,
            Route::DaeBatch,
            Route::DaePreparedSerial,
            Route::DaePreparedBatch,
            Route::DaePreparedRayon,
        ];
        if self.nz() == 1 {
            routes.insert(0, Route::DaeScalarRoot);
        }
        portfolio.choose(
            "dae-initialization",
            &[
                self.nx() as u64,
                self.nz() as u64,
                policy.mode as u64,
                policy.derivative_tol.to_bits(),
                policy.residual_tol.to_bits(),
                policy.condition_cap.to_bits(),
            ],
            &routes,
            policy.state_tol,
            |r| {
                let route = match r {
                    Route::DaeDirect => InitRoute::Direct,
                    Route::DaeReuse => InitRoute::Reuse,
                    Route::DaeBatch => InitRoute::FaerBatch,
                    Route::DaePreparedSerial => InitRoute::PreparedSerial,
                    Route::DaePreparedBatch => InitRoute::PreparedBatch,
                    Route::DaePreparedRayon => InitRoute::PreparedRayon,
                    Route::DaeScalarRoot => InitRoute::ScalarRoot,
                    _ => return Err(bad("route")),
                };
                let v = self.initialize(t, x, z, branch, policy, route)?;
                let e = v.state_error_upper;
                Ok((v, e))
            },
        )
    }
}

// Fixed-width scalar Reuse projection. Each call recomputes all source
// evaluations and certificates; no cached state, factor, or authority.
// The one-row faer 0.24.4 triangular base case is b * recip(a).
// Reuse the existing transactional interval evaluator workspace instead of
// keeping a separate scalar-only Jacobian implementation.
impl IntervalEvalWorkspace {
    fn scalar_row(&self, i: usize, nx: usize) -> &[Iv] {
        let stride = nx + 2;
        &self.jacobian[i * stride..(i + 1) * stride]
    }
    fn scalar_eval(&mut self, src: &DaeSource, t: f64, x: &[f64], a: Iv) -> CoreResult<()> {
        let nx = src.nx();
        let mut y = [z(); 32];
        for (v, &x) in y[..nx].iter_mut().zip(x) {
            *v = p(x)?;
        }
        y[nx] = a;
        src.fill_interval_workspace(p(t)?, &y[..nx + 1], self)
    }
}
/// Fixed-size, fully checked scalar Reuse projection result. It is not a
/// substitute for a slab certificate. Materialize the owning report only
/// when a caller actually needs persistent evidence.
pub struct ScalarReuseProjection {
    nx: usize,
    mode: InitMode,
    t: f64,
    x: [f64; 31],
    seed: f64,
    center: f64,
    branch: [f64; 2],
    root: Iv,
    correction: f64,
    original_residual: f64,
    residual: f64,
    state_error: f64,
    branch_defect: f64,
    local_defect: f64,
    inverse_norm: f64,
    condition: f64,
    xdot: [f64; 31],
    sol: [f64; 33],
    reduced: [f64; 31 * 31],
    derivative_error: f64,
    tangent_residual: f64,
    work: InitWork,
}
impl ScalarReuseProjection {
    pub fn z(&self) -> f64 {
        self.center
    }
    pub fn zt(&self) -> f64 {
        self.sol[0]
    }
    pub fn xdot(&self) -> &[f64] {
        &self.xdot[..self.nx]
    }
    pub fn reduced_jacobian(&self) -> &[f64] {
        &self.reduced[..self.nx * self.nx]
    }
    pub fn work(&self) -> &InitWork {
        &self.work
    }
    pub fn to_owned_report(&self) -> InitReport {
        InitReport {
            route: InitRoute::Reuse,
            mode: self.mode,
            t: self.t,
            x: self.x[..self.nx].to_vec(),
            original_z: vec![self.seed],
            z: vec![self.center],
            correction_upper: self.correction,
            branch: vec![self.branch],
            root_box: vec![self.root.as_pair()],
            branch_authority: "whole-branch contraction + local self-map".to_string(),
            affine_root: None,
            original_residual_upper: self.original_residual,
            residual_upper: self.residual,
            state_error_upper: self.state_error,
            exact_center_consistency: self.residual == 0.,
            branch_defect_upper: Some(self.branch_defect),
            local_defect_upper: self.local_defect,
            inverse_norm_upper: self.inverse_norm,
            condition_upper: self.condition,
            rank: 1,
            xdot: self.xdot().to_vec(),
            zdot: vec![self.sol[self.nx + 1]],
            zt: vec![self.sol[0]],
            zx_columns: self.sol[1..=self.nx].iter().map(|&x| vec![x]).collect(),
            reduced_jacobian: self
                .reduced_jacobian()
                .chunks_exact(self.nx)
                .map(|r| r.to_vec())
                .collect(),
            derivative_error_upper: self.derivative_error,
            tangent_residual_upper: self.tangent_residual,
            work: self.work.clone(),
            qualified: true,
        }
    }
}
fn scalar_batch(a: f64, rhs: &mut [f64], attempt: &mut InitAttemptWork) -> CoreResult<()> {
    // Actual scalar linear batch, not an assertion that a general LU owner
    // was constructed. Preserve original batch and RHS accounting.
    attempt.legacy.linear_batches += 1;
    attempt.legacy.rhs_count += rhs.len();
    attempt.rhs_requested += rhs.len();
    let result = (|| {
        if !a.is_finite() {
            return Err(CoreError::NonFinite("matrix contains NaN/Inf".into()));
        }
        let inv = a.recip();
        for b in rhs.iter_mut() {
            if !b.is_finite() {
                return Err(CoreError::NonFinite("LU RHS contains NaN/Inf".into()));
            }
            *b *= inv;
            if !b.is_finite() {
                return Err(CoreError::LinearSolve("LU produced NaN/Inf".into()));
            }
        }
        Ok(())
    })();
    attempt.linear.record(result.is_ok());
    if result.is_ok() {
        attempt.rhs_returned += rhs.len();
    }
    result
}
fn scalar_defect(c: f64, j: Iv) -> CoreResult<Iv> {
    p(1.)?.sub(j.scale(c)?)
}
impl DaeSource {
    /// Same scalar Reuse admission and interval proof as initialize(),
    /// with fixed-size results. Successful path makes no heap requests.
    /// Other initialization routes and vector blocks retain their own API.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_scalar_reuse_audited(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
    ) -> (CoreResult<ScalarReuseProjection>, InitAttemptWork) {
        let mut w = InitAttemptWork::default();
        let r = self.scalar_reuse_recorded(t, x, seed, branch, policy, &mut w);
        (r, w)
    }
    fn scalar_reuse_recorded(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        w: &mut InitAttemptWork,
    ) -> CoreResult<ScalarReuseProjection> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx = self.nx();
        if self.nz() != 1 {
            return Err(bad("scalar Reuse requires nz=1"));
        }
        if !t.is_finite()
            || x.len() != nx
            || seed.len() != 1
            || branch.len() != 1
            || x.iter().chain(seed).any(|v| !v.is_finite())
            || [
                policy.state_tol,
                policy.residual_tol,
                policy.derivative_tol,
                policy.condition_cap,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
            || !(1..=64).contains(&policy.max_newton)
        {
            return Err(bad("policy/input shape"));
        }
        let b = Iv::new(branch[0][0], branch[0][1])?;
        if b.lo >= b.hi || !b.contains(seed[0]) {
            return Err(bad("invalid branch or initial guess outside"));
        }
        w.admitted = true;
        let zi = nx + 1;
        let mut work = IntervalEvalWorkspace::new();
        let mut point = IntervalEvalWorkspace::new();
        macro_rules! eval {
            ($a:expr,$v:expr) => {{
                let r = (|| $a.scalar_eval(self, t, x, $v))();
                w.evaluations.record(r.is_ok());
                r?;
                w.legacy.evaluations += 1;
            }};
        }
        eval!(work, p(seed[0])?);
        let original_residual = work.values[nx].mag();
        if policy.mode == InitMode::Strict && original_residual > policy.residual_tol {
            return Err(bad("strict initial residual exceeds tolerance"));
        }
        let mut center = seed[0];
        if policy.mode == InitMode::ProjectAlgebraic {
            for _ in 0..policy.max_newton {
                eval!(work, p(center)?);
                let r = work.values[nx].mag();
                if r == 0. {
                    break;
                }
                let a = mid(work.scalar_row(nx, nx)[zi]);
                let mut d = [-mid(work.values[nx])];
                scalar_batch(a, &mut d, w)?;
                let mut accepted = false;
                for j in 0..16 {
                    let trial = 2f64.powi(-j).mul_add(d[0], center);
                    let q = p(trial)?;
                    if !b.contains(trial) {
                        continue;
                    }
                    eval!(work, q);
                    if work.values[nx].mag() < r {
                        center = trial;
                        accepted = true;
                        w.legacy.newton_steps += 1;
                        break;
                    }
                }
                if !accepted {
                    break;
                }
            }
        }
        eval!(point, p(center)?);
        let residual = point.values[nx].mag();
        if residual > policy.residual_tol {
            return Err(bad("original constraint residual exceeds tolerance"));
        }
        let a = mid(point.scalar_row(nx, nx)[zi]);
        let mut c = [1.];
        scalar_batch(a, &mut c, w)?;
        let c = c[0];
        eval!(work, b);
        let defect = scalar_defect(c, work.scalar_row(nx, nx)[zi])?;
        let branch_defect = add_up(0., defect.mag())?;
        let gap = sub_down(1., branch_defect)?;
        if gap <= 0. {
            return Err(bad("whole branch contraction not proved"));
        }
        let cg = z().add(point.values[nx].scale(c)?)?;
        let rad = mul_up(2., div_up(cg.mag(), gap)?)?;
        let root = p(center)?.add(Iv::new(-rad, rad)?)?;
        if root.lo < b.lo || root.hi > b.hi {
            return Err(bad("existence box leaves declared branch"));
        }
        let delta = root.sub(p(center)?)?;
        let ed = z().add(defect.mul(delta)?)?;
        let mapped = p(center)?.sub(cg)?.add(ed)?;
        if mapped.lo < root.lo || mapped.hi > root.hi {
            return Err(bad("local self-map inclusion not proved"));
        }
        let state_error = distance(root, center)?;
        if state_error > policy.state_tol {
            return Err(bad("certified root distance exceeds tolerance"));
        }
        eval!(work, root);
        let j = work.scalar_row(nx, nx)[zi];
        let local_defect = add_up(0., scalar_defect(c, j)?.mag())?;
        let gap = sub_down(1., local_defect)?;
        if gap <= 0. {
            return Err(bad("rank/regularity not proved at root"));
        }
        let inverse_norm = div_up(add_up(0., p(c)?.mag())?, gap)?;
        let condition = mul_up(inverse_norm, add_up(0., j.mag())?)?;
        if condition > policy.condition_cap {
            return Err(bad("certified condition exceeds cap"));
        }
        let gc = point.scalar_row(nx, nx);
        let gr = work.scalar_row(nx, nx);
        let mut rhs = [z(); 33];
        for k in 0..=nx {
            rhs[k] = -gr[k];
        }
        rhs[nx + 1] = gr[1..=nx]
            .iter()
            .zip(&work.values[..nx])
            .try_fold(-gr[0], |s, (&g, &f)| s.sub(g.mul(f)?))?;
        let mut xdot = [0.; 31];
        for (v, &f) in xdot[..nx].iter_mut().zip(&point.values[..nx]) {
            *v = mid(f);
        }
        let mut sol = [0.; 33];
        for k in 0..=nx {
            sol[k] = -mid(gc[k]);
        }
        sol[nx + 1] = mid(gc[1..=nx]
            .iter()
            .zip(&xdot[..nx])
            .try_fold(-gc[0], |s, (&g, &f)| s.sub(g.scale(f)?))?);
        scalar_batch(a, &mut sol[..nx + 2], w)?;
        let mut enc = [z(); 33];
        let mut derivative_error = 0_f64;
        for k in 0..nx + 2 {
            let av = z().add(j.mul(p(sol[k])?)?)?;
            let residual = rhs[k].sub(av)?;
            let pc = z().add(residual.scale(c)?)?;
            let e = div_up(pc.mag(), gap)?;
            derivative_error = derivative_error.max(e);
            enc[k] = p(sol[k])?.add(Iv::new(-e, e)?)?;
        }
        for (&v, &f) in xdot[..nx].iter().zip(&work.values[..nx]) {
            derivative_error = derivative_error.max(distance(f, v)?);
        }
        let mut reduced = [0.; 31 * 31];
        for i in 0..nx {
            for k in 0..nx {
                let v = point.scalar_row(i, nx)[k + 1]
                    .add(point.scalar_row(i, nx)[zi].scale(sol[k + 1])?)?;
                let exact = work.scalar_row(i, nx)[k + 1]
                    .add(work.scalar_row(i, nx)[zi].mul(enc[k + 1])?)?;
                let m = mid(v);
                reduced[i * nx + k] = m;
                derivative_error = derivative_error.max(distance(exact, m)?);
            }
        }
        let tangent_residual = gc[1..=nx]
            .iter()
            .zip(&xdot[..nx])
            .chain(gc[zi..=zi].iter().zip(&sol[nx + 1..nx + 2]))
            .try_fold(gc[0], |s, (&a, &v)| s.add(a.scale(v)?))?
            .mag();
        if derivative_error > policy.derivative_tol || tangent_residual > policy.derivative_tol {
            return Err(bad("derivative/Schur certificate exceeds tolerance"));
        }
        let correction = p(center)?.sub(p(seed[0])?)?.mag();
        let mut xx = [0.; 31];
        xx[..nx].copy_from_slice(x);
        Ok(ScalarReuseProjection {
            nx,
            mode: policy.mode,
            t,
            x: xx,
            seed: seed[0],
            center,
            branch: branch[0],
            root,
            correction,
            original_residual,
            residual,
            state_error,
            branch_defect,
            local_defect,
            inverse_norm,
            condition,
            xdot,
            sol,
            reduced,
            derivative_error,
            tangent_residual,
            work: w.legacy.clone(),
        })
    }
}
