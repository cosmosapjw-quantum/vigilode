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
    /// Scalar-only bounded prepared solve with original midpoint residual.
    BoundedPreparedScalar,
    /// Directed frozen affine coefficient proof, no rational string metadata.
    BoundedAffineScalar,
    /// Interval-only coefficient expansion competitor.
    BoundedAffineIntervalScalar,
    BoundedSturmScalar,
    BoundedSturmNewtonScalar,
    BoundedVector,
    BoundedVectorRefined,
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
    #[serde(skip_serializing_if = "init_count_zero")]
    pub root_map_rechecks: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub root_map_regrouped: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub root_map_expanded: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_prepared_batches: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_residual_terms: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub affine_dyadic_success: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_sturm_chains: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_sturm_iterations: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_sturm_newton: usize,
}
fn init_count_zero(n: &usize) -> bool {
    *n == 0
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
    #[serde(skip_serializing_if = "init_count_zero")]
    pub root_map_rechecks: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub root_map_regrouped: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub root_map_expanded: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_prepared_batches: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_residual_terms: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub affine_dyadic_success: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_sturm_chains: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_sturm_iterations: usize,
    #[serde(skip_serializing_if = "init_count_zero")]
    pub bounded_sturm_newton: usize,
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
        self.root_map_rechecks += w.legacy.root_map_rechecks;
        self.root_map_regrouped += w.legacy.root_map_regrouped;
        self.root_map_expanded += w.legacy.root_map_expanded;
        self.bounded_prepared_batches += w.legacy.bounded_prepared_batches;
        self.bounded_residual_terms += w.legacy.bounded_residual_terms;
        self.affine_dyadic_success += w.legacy.affine_dyadic_success;
        self.bounded_sturm_chains += w.legacy.bounded_sturm_chains;
        self.bounded_sturm_iterations += w.legacy.bounded_sturm_iterations;
        self.bounded_sturm_newton += w.legacy.bounded_sturm_newton;
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
        self.root_map_rechecks += w.root_map_rechecks;
        self.root_map_regrouped += w.root_map_regrouped;
        self.root_map_expanded += w.root_map_expanded;
        self.bounded_prepared_batches += w.bounded_prepared_batches;
        self.bounded_residual_terms += w.bounded_residual_terms;
        self.affine_dyadic_success += w.affine_dyadic_success;
        self.bounded_sturm_chains += w.bounded_sturm_chains;
        self.bounded_sturm_iterations += w.bounded_sturm_iterations;
        self.bounded_sturm_newton += w.bounded_sturm_newton;
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
    work: &mut InitWork,
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
    if subset(mapped, &b) {
        return Ok((b, q));
    }
    // Same affine interval image, but sum the small corrections before
    // adding the center. Recheck actual rounded endpoints, never infer
    // inclusion from a real-arithmetic radius inequality.
    let mut b = b;
    let mut radius = r;
    for retry in 0..=8 {
        if retry > 0 {
            radius = if retry == 1 {
                let mut spacing = 0_f64;
                for &v in center {
                    spacing = spacing
                        .max(p(v.next_up())?.sub(p(v)?)?.mag())
                        .max(p(v)?.sub(p(v.next_down())?)?.mag());
                }
                radius.max(spacing)
            } else {
                mul_up(radius, 2.)?
            };
            for (o, &v) in b.iter_mut().zip(center) {
                *o = p(v)?.add(Iv::new(-radius, radius)?)?;
            }
            if !subset(&b, branch) {
                break;
            }
            for (d, (&v, &c)) in delta.iter_mut().zip(b.iter().zip(center)) {
                *d = v.sub(p(c)?)?;
            }
            iv_matvec_into(&e, delta, ed)?;
        }
        work.root_map_rechecks += 1;
        for (i, x) in mapped.iter_mut().enumerate() {
            *x = p(center[i])?.add(ed[i].sub(cg[i])?)?;
        }
        if subset(mapped, &b) {
            if retry == 0 {
                work.root_map_regrouped += 1;
            } else {
                work.root_map_expanded += 1;
            }
            return Ok((b, q));
        }
    }
    Err(bad("local self-map inclusion not proved"))
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
        if matches!(
            route,
            InitRoute::BoundedVector | InitRoute::BoundedVectorRefined
        ) {
            return self
                .bounded_vector_recorded(
                    t,
                    x,
                    z0,
                    branch,
                    policy,
                    route == InitRoute::BoundedVectorRefined,
                    &mut VectorProjectionWorkspace::new(),
                    attempt,
                )
                .map(|r| r.to_owned_report());
        }
        if matches!(
            route,
            InitRoute::BoundedPreparedScalar
                | InitRoute::BoundedAffineScalar
                | InitRoute::BoundedAffineIntervalScalar
                | InitRoute::BoundedSturmScalar
                | InitRoute::BoundedSturmNewtonScalar
        ) {
            return self
                .scalar_reuse_recorded(t, x, z0, branch, policy, route, attempt)
                .map(|r| r.to_owned_report());
        }
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
            let (r, q) = verified_root_box(&c, &gc.v, &self.jac_z(&g), &center, &b, scratch, w)?;
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
    route: InitRoute,
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
    branch_defect: Option<f64>,
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
            route: self.route,
            mode: self.mode,
            t: self.t,
            x: self.x[..self.nx].to_vec(),
            original_z: vec![self.seed],
            z: vec![self.center],
            correction_upper: self.correction,
            branch: vec![self.branch],
            root_box: vec![self.root.as_pair()],
            branch_authority: if matches!(
                self.route,
                InitRoute::BoundedSturmScalar | InitRoute::BoundedSturmNewtonScalar
            ) {
                "bounded exact square-free Sturm count + certified root bracket"
            } else if matches!(
                self.route,
                InitRoute::BoundedAffineScalar | InitRoute::BoundedAffineIntervalScalar
            ) {
                "directed frozen affine root enclosure"
            } else {
                "whole-branch contraction + local self-map"
            }
            .to_string(),
            affine_root: None,
            original_residual_upper: self.original_residual,
            residual_upper: self.residual,
            state_error_upper: self.state_error,
            exact_center_consistency: self.residual == 0.,
            branch_defect_upper: self.branch_defect,
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
fn scalar_batch(
    a: f64,
    rhs: &mut [f64],
    route: InitRoute,
    attempt: &mut InitAttemptWork,
) -> CoreResult<()> {
    // Actual scalar linear batch, not an assertion that a general LU owner
    // was constructed. Preserve original batch and RHS accounting.
    attempt.legacy.linear_batches += 1;
    attempt.legacy.rhs_count += rhs.len();
    attempt.rhs_requested += rhs.len();
    let result = (|| {
        if !a.is_finite() {
            return Err(CoreError::NonFinite("matrix contains NaN/Inf".into()));
        }
        let prepared = route == InitRoute::BoundedPreparedScalar;
        if prepared {
            attempt.legacy.bounded_prepared_batches += 1;
            // Scalar specialization of strict row dominance and the
            // original represented-matrix residual. No faer owner claimed.
            crate::affine_event_certificate::check_affine_event_arithmetic()?;
            if a == 0. {
                return Err(bad("bounded scalar strict row dominance not proved"));
            }
        }
        let inv = a.recip();
        let mut residual = 0_f64;
        for b in rhs.iter_mut() {
            if !b.is_finite() {
                return Err(CoreError::NonFinite("LU RHS contains NaN/Inf".into()));
            }
            let original = *b;
            *b *= inv;
            if !b.is_finite() {
                return Err(CoreError::LinearSolve("LU produced NaN/Inf".into()));
            }
            if prepared {
                let r = p(original)?.sub(p(a)?.scale(*b)?)?.mag();
                attempt.legacy.bounded_residual_terms += 1;
                residual = residual.max(r);
            }
        }
        if prepared {
            let _error_upper = div_up(residual, a.abs())?;
        }
        Ok(())
    })();
    attempt.linear.record(result.is_ok());
    if result.is_ok() {
        attempt.rhs_returned += rhs.len();
    }
    result
}
// Exact frozen source polynomial enclosed coefficientwise in physical z.
// Higher coefficients must be exactly zero; no approximate affine admission.
fn frozen_affine_coefficients(eq: &DeclaredPolynomial, t: f64, x: &[f64]) -> CoreResult<[Iv; 5]> {
    let mut sum = [z(); 5];
    for term in eq.terms() {
        let mut poly = [z(); 5];
        poly[0] = p(term.coefficient)?;
        let mut degree = 0;
        for f in &term.factors {
            let mut c = p(f.constant)?.add(p(t)?.scale(f.time)?)?;
            let mut a = z();
            for &(i, w) in &f.state {
                if i == x.len() {
                    a = a.add(p(w)?)?;
                } else {
                    c = c.add(p(x[i])?.scale(w)?)?;
                }
            }
            let mut next = [z(); 5];
            for k in 0..=degree {
                next[k] = next[k].add(poly[k].mul(c)?)?;
                next[k + 1] = next[k + 1].add(poly[k].mul(a)?)?;
            }
            degree += 1;
            poly = next;
        }
        for (a, b) in sum.iter_mut().zip(poly) {
            *a = a.add(b)?;
        }
    }
    Ok(sum)
}
// Exact dyadic arithmetic with a bounded signed i128 mantissa. A failed
// capacity check is NOT evidence of non-affineness; the independently
// directed coefficient proof remains available. Never wrap an integer.
#[derive(Clone, Copy)]
struct RootDyadic {
    m: i128,
    e: i32,
}
impl RootDyadic {
    const ZERO: Self = Self { m: 0, e: 0 };
    fn new(m: i128, e: i32) -> Option<Self> {
        if m == 0 {
            return Some(Self::ZERO);
        }
        let k = m.trailing_zeros().min(126);
        Some(Self {
            m: m >> k,
            e: e.checked_add(k as i32)?,
        })
    }
    fn from_f64(v: f64) -> Option<Self> {
        if !v.is_finite() {
            return None;
        }
        let b = v.to_bits();
        let exp = ((b >> 52) & 2047) as i32;
        let m = ((b & ((1u64 << 52) - 1)) | if exp == 0 { 0 } else { 1u64 << 52 }) as i128;
        Self::new(
            if b >> 63 == 1 { -m } else { m },
            if exp == 0 { -1074 } else { exp - 1023 - 52 },
        )
    }
    fn add(self, b: Self) -> Option<Self> {
        if self.m == 0 {
            return Some(b);
        }
        if b.m == 0 {
            return Some(self);
        }
        let e = self.e.min(b.e);
        let scale = |v: Self| {
            let k = v.e.checked_sub(e)?;
            if k > 126 {
                return None;
            }
            v.m.checked_mul(1i128 << k)
        };
        Self::new(scale(self)?.checked_add(scale(b)?)?, e)
    }
    fn mul(self, b: Self) -> Option<Self> {
        Self::new(self.m.checked_mul(b.m)?, self.e.checked_add(b.e)?)
    }
    fn interval(self) -> Option<Iv> {
        // Conservative conversion range; exact integer comparison before
        // directed scaling, no unchecked saturated float-to-int decision.
        if self.m == 0 {
            return Some(z());
        }
        if !(-900..=900).contains(&self.e) || self.m.unsigned_abs() > (1u128 << 120) {
            return None;
        }
        let v = self.m as f64;
        let m = v as i128;
        let a = Iv::new(
            if m > self.m { v.next_down() } else { v },
            if m < self.m { v.next_up() } else { v },
        )
        .ok()?;
        a.scale(2f64.powi(self.e)).ok()
    }
}
fn frozen_dyadic(eq: &DeclaredPolynomial, t: f64, x: &[f64]) -> Option<[RootDyadic; 5]> {
    let q = RootDyadic::from_f64;
    let mut sum = [RootDyadic::ZERO; 5];
    for term in eq.terms() {
        let mut poly = [RootDyadic::ZERO; 5];
        poly[0] = q(term.coefficient)?;
        let mut degree = 0;
        for f in &term.factors {
            let mut c = q(f.constant)?.add(q(t)?.mul(q(f.time)?)?)?;
            let mut a = RootDyadic::ZERO;
            for &(i, w) in &f.state {
                if i == x.len() {
                    a = a.add(q(w)?)?;
                } else {
                    c = c.add(q(x[i])?.mul(q(w)?)?)?;
                }
            }
            let mut next = [RootDyadic::ZERO; 5];
            for k in 0..=degree {
                next[k] = next[k].add(poly[k].mul(c)?)?;
                next[k + 1] = next[k + 1].add(poly[k].mul(a)?)?;
            }
            degree += 1;
            poly = next;
        }
        for (v, w) in sum.iter_mut().zip(poly) {
            *v = v.add(w)?;
        }
    }
    Some(sum)
}
fn frozen_affine_dyadic(eq: &DeclaredPolynomial, t: f64, x: &[f64]) -> Option<[Iv; 5]> {
    let mut out = [z(); 5];
    for (v, d) in out.iter_mut().zip(frozen_dyadic(eq, t, x)?) {
        *v = d.interval()?;
    }
    Some(out)
}
// Primitive signed polynomial remainder sequence. Each elimination scales
// the dividend by |lc(B)|, never by a negative quantity. The negative
// remainder is therefore a positive multiple of the ordinary Sturm term.
// Checked bounded integer arithmetic; refusal on overflow or nonconstant gcd.
#[derive(Clone, Copy)]
struct SmallSturmPoly {
    a: [i128; 5],
    n: usize,
}
impl SmallSturmPoly {
    const ZERO: Self = Self { a: [0; 5], n: 0 };
    fn primitive(mut self) -> Option<Self> {
        while self.n > 0 && self.a[self.n - 1] == 0 {
            self.n -= 1;
        }
        let mut g = 0u128;
        for &v in &self.a[..self.n] {
            let mut b = v.unsigned_abs();
            while b != 0 {
                let r = g % b;
                g = b;
                b = r;
            }
        }
        if g > i128::MAX as u128 {
            return None;
        }
        if g > 1 {
            for v in &mut self.a[..self.n] {
                *v /= g as i128;
            }
        }
        Some(self)
    }
    fn from_dyadic(d: [RootDyadic; 5]) -> Option<Self> {
        let e = d.iter().filter(|v| v.m != 0).map(|v| v.e).min()?;
        let mut a = [0; 5];
        for (v, c) in a.iter_mut().zip(d) {
            if c.m == 0 {
                continue;
            }
            let k = c.e.checked_sub(e)?;
            if k > 126 {
                return None;
            }
            *v = c.m.checked_mul(1i128 << k)?;
        }
        Self { a, n: 5 }.primitive()
    }
    fn derivative(self) -> Option<Self> {
        let mut a = [0; 5];
        for i in 1..self.n {
            a[i - 1] = self.a[i].checked_mul(i as i128)?;
        }
        Self {
            a,
            n: self.n.saturating_sub(1),
        }
        .primitive()
    }
    fn negative_remainder(mut self, b: Self) -> Option<Self> {
        let lc = b.a[b.n.checked_sub(1)?];
        let abs = lc.checked_abs()?;
        while self.n >= b.n && self.n != 0 {
            let k = self.n - b.n;
            let r = self.a[self.n - 1].checked_mul(lc.signum())?;
            for i in 0..self.n {
                let t = if i >= k && i - k < b.n {
                    r.checked_mul(b.a[i - k])?
                } else {
                    0
                };
                self.a[i] = self.a[i].checked_mul(abs)?.checked_sub(t)?;
            }
            self = self.primitive()?;
        }
        for a in &mut self.a[..self.n] {
            *a = a.checked_neg()?;
        }
        self.primitive()
    }
    fn interval(self, x: Iv) -> CoreResult<Iv> {
        self.a[..self.n].iter().rev().try_fold(z(), |s, &v| {
            let a = RootDyadic::new(v, 0)
                .and_then(RootDyadic::interval)
                .ok_or_else(|| bad("bounded Sturm coefficient enclosure cap"))?;
            s.mul(x)?.add(a)
        })
    }
    fn sign(self, x: f64) -> CoreResult<Option<i8>> {
        if let Some(v) = (|| {
            let x = RootDyadic::from_f64(x)?;
            self.a[..self.n]
                .iter()
                .rev()
                .try_fold(RootDyadic::ZERO, |s, &v| {
                    s.mul(x)?.add(RootDyadic::new(v, 0)?)
                })
        })() {
            return Ok(Some(v.m.signum() as i8));
        }
        let r = self.interval(p(x)?)?;
        Ok(if r.lo > 0. {
            Some(1)
        } else if r.hi < 0. {
            Some(-1)
        } else if r.lo == 0. && r.hi == 0. {
            Some(0)
        } else {
            None
        })
    }
}
fn sturm_variation(chain: &[SmallSturmPoly], x: f64) -> CoreResult<usize> {
    let mut last = 0;
    let mut count = 0;
    for poly in chain {
        let s = poly
            .sign(x)?
            .ok_or_else(|| bad("bounded Sturm endpoint sign unknown"))?;
        if s != 0 {
            if last != 0 && last != s {
                count += 1;
            }
            last = s;
        }
    }
    Ok(count)
}
fn bounded_sturm_root(
    poly: SmallSturmPoly,
    b: Iv,
    newton: bool,
    w: &mut InitWork,
) -> CoreResult<Iv> {
    if poly.n < 2 {
        return Err(bad("bounded Sturm constant or zero polynomial"));
    }
    let mut chain = [SmallSturmPoly::ZERO; 5];
    chain[0] = poly;
    chain[1] = poly
        .derivative()
        .ok_or_else(|| bad("bounded Sturm derivative cap"))?;
    let mut n = 2;
    while chain[n - 1].n > 1 {
        let r = chain[n - 2]
            .negative_remainder(chain[n - 1])
            .ok_or_else(|| bad("bounded Sturm PRS cap"))?;
        if r.n == 0 {
            return Err(bad("bounded Sturm non-square-free polynomial"));
        }
        if n == chain.len() {
            return Err(bad("bounded Sturm chain cap"));
        }
        chain[n] = r;
        n += 1;
    }
    let lsign = poly
        .sign(b.lo)?
        .filter(|&s| s != 0)
        .ok_or_else(|| bad("bounded Sturm boundary sign"))?;
    let hsign = poly
        .sign(b.hi)?
        .filter(|&s| s != 0)
        .ok_or_else(|| bad("bounded Sturm boundary sign"))?;
    let vl = sturm_variation(&chain[..n], b.lo)?;
    let vh = sturm_variation(&chain[..n], b.hi)?;
    w.bounded_sturm_chains += 1;
    if vl.checked_sub(vh) != Some(1) || lsign == hsign {
        return Err(bad("bounded Sturm not exactly one simple interior root"));
    }
    let h = p(b.hi)?.sub(p(b.lo)?)?;
    let target = h.scale(2f64.powi(-49))?.lo;
    let mut bracket = b;
    // Use the original polynomial derivative, NOT the primitive scaled
    // derivative, for Newton. Its coefficient signs/values enclose exact p'.
    let mut derivative = [z(); 4];
    for i in 1..poly.n {
        derivative[i - 1] = RootDyadic::new(poly.a[i], 0)
            .and_then(RootDyadic::interval)
            .ok_or_else(|| bad("bounded Sturm derivative conversion"))?
            .scale(i as f64)?;
    }
    for _ in 0..128 {
        w.bounded_sturm_iterations += 1;
        if p(bracket.hi)?.sub(p(bracket.lo)?)?.hi <= target {
            return Ok(bracket);
        }
        let m = mid(bracket);
        if m <= bracket.lo || m >= bracket.hi {
            return Err(bad("bounded Sturm bracket precision"));
        }
        let sign = poly.sign(m)?;
        if sign == Some(0) {
            return p(m);
        }
        // Even when the exact sign is too expensive, interval Newton can
        // shrink a box already known to contain exactly one root.
        if newton || sign.is_none() {
            let j = derivative[..poly.n - 1]
                .iter()
                .rev()
                .try_fold(z(), |s, &a| s.mul(bracket)?.add(a))?;
            if !j.contains_zero() {
                let image = p(m)?.sub(poly.interval(p(m)?)?.div(j)?)?;
                let next = Iv::new(bracket.lo.max(image.lo), bracket.hi.min(image.hi))?;
                let nw = p(next.hi)?.sub(p(next.lo)?)?.hi;
                if nw <= target {
                    w.bounded_sturm_newton += 1;
                    return Ok(next);
                }
                if newton && nw < 0.5 * (bracket.hi - bracket.lo) {
                    if poly.sign(next.lo)? == Some(lsign) && poly.sign(next.hi)? == Some(hsign) {
                        bracket = next;
                        w.bounded_sturm_newton += 1;
                        continue;
                    }
                }
            }
        }
        match sign {
            Some(s) if s == lsign => bracket.lo = m,
            Some(s) if s == hsign => bracket.hi = m,
            _ => return Err(bad("bounded Sturm midpoint sign unknown")),
        }
    }
    Err(bad("bounded Sturm isolation cap"))
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
        let r = self.scalar_reuse_recorded(t, x, seed, branch, policy, InitRoute::Reuse, &mut w);
        (r, w)
    }
    /// Separate route provenance: scalar prepared original midpoint residual
    /// plus the same branch/derivative proof. Successful bounded core no heap.
    pub fn initialize_scalar_bounded_prepared_audited(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
    ) -> (CoreResult<ScalarReuseProjection>, InitAttemptWork) {
        let mut w = InitAttemptWork::default();
        let r = self.scalar_reuse_recorded(
            t,
            x,
            seed,
            branch,
            policy,
            InitRoute::BoundedPreparedScalar,
            &mut w,
        );
        (r, w)
    }
    /// Fixed-size affine source proof plus the original residual/derivative
    /// tests. Refuses unproved affine structure; no fallback to root count.
    pub fn initialize_scalar_bounded_affine_audited(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
    ) -> (CoreResult<ScalarReuseProjection>, InitAttemptWork) {
        let mut w = InitAttemptWork::default();
        let r = self.scalar_reuse_recorded(
            t,
            x,
            seed,
            branch,
            policy,
            InitRoute::BoundedAffineScalar,
            &mut w,
        );
        (r, w)
    }
    pub fn initialize_scalar_bounded_sturm_audited(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        newton: bool,
    ) -> (CoreResult<ScalarReuseProjection>, InitAttemptWork) {
        let mut w = InitAttemptWork::default();
        let r = self.scalar_reuse_recorded(
            t,
            x,
            seed,
            branch,
            policy,
            if newton {
                InitRoute::BoundedSturmNewtonScalar
            } else {
                InitRoute::BoundedSturmScalar
            },
            &mut w,
        );
        (r, w)
    }
    fn scalar_reuse_recorded(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        route: InitRoute,
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
        let mut affine_box = None;
        if matches!(
            route,
            InitRoute::BoundedAffineScalar
                | InitRoute::BoundedAffineIntervalScalar
                | InitRoute::BoundedSturmScalar
                | InitRoute::BoundedSturmNewtonScalar
        ) {
            for v in [b.lo, b.hi] {
                eval!(work, p(v)?);
                if work.values[nx].contains_zero() {
                    return Err(bad("scalar branch boundary root/unknown"));
                }
            }
            let q = if matches!(
                route,
                InitRoute::BoundedSturmScalar | InitRoute::BoundedSturmNewtonScalar
            ) {
                let poly = frozen_dyadic(&self.g[0], t, x)
                    .and_then(SmallSturmPoly::from_dyadic)
                    .ok_or_else(|| bad("bounded Sturm frozen integer cap"));
                w.frozen.record(poly.is_ok());
                let poly = poly?;
                let q = bounded_sturm_root(
                    poly,
                    b,
                    route == InitRoute::BoundedSturmNewtonScalar,
                    &mut w.legacy,
                );
                w.roots.record(q.is_ok());
                q?
            } else {
                let coeff = (|| {
                    let mut c = frozen_affine_coefficients(&self.g[0], t, x)?;
                    if route == InitRoute::BoundedAffineScalar {
                        if let Some(d) = frozen_affine_dyadic(&self.g[0], t, x) {
                            for (v, d) in c.iter_mut().zip(d) {
                                *v = Iv::new(v.lo.max(d.lo), v.hi.min(d.hi))?;
                            }
                            w.legacy.affine_dyadic_success += 1;
                        }
                    }
                    if c[2..].iter().any(|v| v.lo != 0. || v.hi != 0.) || c[1].contains_zero() {
                        return Err(bad("bounded frozen affine degree or slope not proved"));
                    }
                    Ok(c)
                })();
                w.frozen.record(coeff.is_ok());
                let coeff = coeff?;
                let q = (|| {
                    let root = (-coeff[0]).div(coeff[1])?;
                    if root.lo <= b.lo || root.hi >= b.hi {
                        return Err(bad("bounded affine interior root not proved"));
                    }
                    Ok(root)
                })();
                w.roots.record(q.is_ok());
                q?
            };
            w.legacy.root_calls += 1;
            if policy.mode == InitMode::ProjectAlgebraic {
                center = mid(q);
                eval!(work, p(center)?);
                let r = work.values[nx].mag();
                let d = mid(work.scalar_row(nx, nx)[zi]);
                if d != 0. {
                    let trial = center - mid(work.values[nx]) / d;
                    if q.contains(trial) {
                        eval!(work, p(trial)?);
                        if work.values[nx].mag() < r {
                            center = trial;
                            w.legacy.newton_steps += 1;
                        }
                    }
                }
            }
            affine_box = Some(q);
        } else if policy.mode == InitMode::ProjectAlgebraic {
            for _ in 0..policy.max_newton {
                eval!(work, p(center)?);
                let r = work.values[nx].mag();
                if r == 0. {
                    break;
                }
                let a = mid(work.scalar_row(nx, nx)[zi]);
                let mut d = [-mid(work.values[nx])];
                scalar_batch(a, &mut d, route, w)?;
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
        scalar_batch(a, &mut c, route, w)?;
        let c = c[0];
        let (root, branch_defect) = if let Some(root) = affine_box {
            (root, None)
        } else {
            eval!(work, b);
            let defect = scalar_defect(c, work.scalar_row(nx, nx)[zi])?;
            let branch_defect = add_up(0., defect.mag())?;
            let gap = sub_down(1., branch_defect)?;
            if gap <= 0. {
                return Err(bad("whole branch contraction not proved"));
            }
            let cg = z().add(point.values[nx].scale(c)?)?;
            let rad = mul_up(2., div_up(cg.mag(), gap)?)?;
            let mut root = p(center)?.add(Iv::new(-rad, rad)?)?;
            if root.lo < b.lo || root.hi > b.hi {
                return Err(bad("existence box leaves declared branch"));
            }
            let ed = z().add(defect.mul(root.sub(p(center)?)?)?)?;
            let mapped = p(center)?.sub(cg)?.add(ed)?;
            if mapped.lo < root.lo || mapped.hi > root.hi {
                let mut radius = rad;
                let mut admitted = false;
                for retry in 0..=8 {
                    if retry > 0 {
                        radius = if retry == 1 {
                            let spacing = p(center.next_up())?
                                .sub(p(center)?)?
                                .mag()
                                .max(p(center)?.sub(p(center.next_down())?)?.mag());
                            radius.max(spacing)
                        } else {
                            mul_up(radius, 2.)?
                        };
                        root = p(center)?.add(Iv::new(-radius, radius)?)?;
                        if root.lo < b.lo || root.hi > b.hi {
                            break;
                        }
                    }
                    let ed = z().add(defect.mul(root.sub(p(center)?)?)?)?;
                    w.legacy.root_map_rechecks += 1;
                    let image = p(center)?.add(ed.sub(cg)?)?;
                    if image.lo >= root.lo && image.hi <= root.hi {
                        if retry == 0 {
                            w.legacy.root_map_regrouped += 1;
                        } else {
                            w.legacy.root_map_expanded += 1;
                        }
                        admitted = true;
                        break;
                    }
                }
                if !admitted {
                    return Err(bad("local self-map inclusion not proved"));
                }
            }
            (root, Some(branch_defect))
        };
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
        scalar_batch(a, &mut sol[..nx + 2], route, w)?;
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
            route,
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

/// Bounded vector projection result. The fixed arrays are not a cached
/// matrix or source authority; owned diagnostics are made at the API edge.
pub struct BoundedVectorProjection {
    nx: usize,
    nz: usize,
    route: InitRoute,
    mode: InitMode,
    t: f64,
    x: [f64; 32],
    seed: [f64; 32],
    center: [f64; 32],
    branch: [Iv; 32],
    root: [Iv; 32],
    original_residual: f64,
    residual: f64,
    state_error: f64,
    correction: f64,
    branch_defect: f64,
    local_defect: f64,
    inverse_norm: f64,
    condition: f64,
    xdot: [f64; 32],
    sol: [f64; PROJ_COLUMNS],
    reduced: [f64; 1024],
    ft: [f64; 32],
    derivative_error: f64,
    tangent_residual: f64,
    work: InitWork,
}
impl BoundedVectorProjection {
    pub fn z(&self) -> &[f64] {
        &self.center[..self.nz]
    }
    pub fn xdot(&self) -> &[f64] {
        &self.xdot[..self.nx]
    }
    pub fn reduced_jacobian(&self) -> &[f64] {
        &self.reduced[..self.nx * self.nx]
    }
    pub fn reduced_time(&self) -> &[f64] {
        &self.ft[..self.nx]
    }
    pub fn work(&self) -> &InitWork {
        &self.work
    }
    pub fn to_owned_report(&self) -> InitReport {
        InitReport {
            route: self.route,
            mode: self.mode,
            t: self.t,
            x: self.x[..self.nx].to_vec(),
            original_z: self.seed[..self.nz].to_vec(),
            z: self.z().to_vec(),
            correction_upper: self.correction,
            branch: self.branch[..self.nz].iter().map(|v| v.as_pair()).collect(),
            root_box: self.root[..self.nz].iter().map(|v| v.as_pair()).collect(),
            branch_authority: "whole-branch contraction + local self-map".into(),
            affine_root: None,
            original_residual_upper: self.original_residual,
            residual_upper: self.residual,
            state_error_upper: self.state_error,
            exact_center_consistency: self.residual == 0.,
            branch_defect_upper: Some(self.branch_defect),
            local_defect_upper: self.local_defect,
            inverse_norm_upper: self.inverse_norm,
            condition_upper: self.condition,
            rank: self.nz,
            xdot: self.xdot().to_vec(),
            zdot: self.sol[(self.nx + 1) * self.nz..(self.nx + 2) * self.nz].to_vec(),
            zt: self.sol[..self.nz].to_vec(),
            zx_columns: self.sol[self.nz..(self.nx + 1) * self.nz]
                .chunks_exact(self.nz)
                .map(|v| v.to_vec())
                .collect(),
            reduced_jacobian: self
                .reduced_jacobian()
                .chunks_exact(self.nx)
                .map(|v| v.to_vec())
                .collect(),
            derivative_error_upper: self.derivative_error,
            tangent_residual_upper: self.tangent_residual,
            work: self.work.clone(),
            qualified: true,
        }
    }
}
pub struct VectorProjectionWorkspace {
    point: IntervalEvalWorkspace,
    eval: IntervalEvalWorkspace,
    inverse: crate::retro_dae_evolution::GuardInverseWorkspace,
    a: [f64; 1024],
    c: [f64; 1024],
    e: [Iv; 1024],
    projection: ProjectionWorkspace,
}
impl VectorProjectionWorkspace {
    pub const fn new() -> Self {
        Self {
            point: IntervalEvalWorkspace::new(),
            eval: IntervalEvalWorkspace::new(),
            inverse: crate::retro_dae_evolution::GuardInverseWorkspace::new(),
            a: [0.; 1024],
            c: [0.; 1024],
            e: [Iv { lo: 0., hi: 0. }; 1024],
            projection: ProjectionWorkspace::new(),
        }
    }
}
impl Default for VectorProjectionWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
fn vector_eval(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    a: &[Iv],
    w: &mut IntervalEvalWorkspace,
    receipt: &mut InitAttemptWork,
) -> CoreResult<()> {
    let mut y = [z(); 32];
    for (v, &x) in y[..x.len()].iter_mut().zip(x) {
        *v = p(x)?;
    }
    y[x.len()..x.len() + a.len()].copy_from_slice(a);
    let r = src.fill_interval_workspace(p(t)?, &y[..x.len() + a.len()], w);
    receipt.evaluations.record(r.is_ok());
    if r.is_ok() {
        receipt.legacy.evaluations += 1;
    }
    r
}
fn vector_row(w: &IntervalEvalWorkspace, n: usize, i: usize) -> &[Iv] {
    &w.jacobian[i * (n + 1)..(i + 1) * (n + 1)]
}
fn vector_inverse(
    w: &mut VectorProjectionWorkspace,
    nx: usize,
    nz: usize,
    receipt: &mut InitAttemptWork,
) -> CoreResult<()> {
    let n = nx + nz;
    for i in 0..nz {
        for j in 0..nz {
            w.a[i * nz + j] = mid(vector_row(&w.point, n, nx + i)[nx + 1 + j]);
        }
    }
    let r = w.inverse.invert(nz, &w.a[..nz * nz], &mut w.c[..nz * nz]);
    receipt.legacy.linear_batches += 1;
    receipt.legacy.rhs_count += nz;
    receipt.rhs_requested += nz;
    receipt.legacy.bounded_prepared_batches += 1;
    receipt.linear.record(r.is_ok());
    if r.is_ok() {
        receipt.rhs_returned += nz;
        let nnz = w.a[..nz * nz].iter().filter(|&&v| v != 0.).count();
        receipt.legacy.bounded_residual_terms +=
            nz * if nnz <= nz * nz / 2 { nnz } else { nz * nz };
    }
    r.map(|_| ())
}
// A numerical candidate, followed by the actual original interval residual.
// Refinement uses b-Ax in represented midpoint arithmetic and is optional.
fn vector_action(
    w: &VectorProjectionWorkspace,
    nz: usize,
    b: &[f64],
    refine: bool,
    out: &mut [f64],
    receipt: &mut InitAttemptWork,
) -> CoreResult<()> {
    receipt.legacy.linear_batches += 1;
    receipt.legacy.rhs_count += 1;
    receipt.rhs_requested += 1;
    let r = (|| {
        for i in 0..nz {
            out[i] = w.c[i * nz..(i + 1) * nz]
                .iter()
                .zip(b)
                .fold(0., |s, (&a, &v)| a.mul_add(v, s));
        }
        if refine {
            let mut residual = [0.; 32];
            for i in 0..nz {
                residual[i] = w.a[i * nz..(i + 1) * nz]
                    .iter()
                    .zip(&*out)
                    .fold(b[i], |s, (&a, &v)| (-a).mul_add(v, s));
            }
            for i in 0..nz {
                out[i] += w.c[i * nz..(i + 1) * nz]
                    .iter()
                    .zip(&residual[..nz])
                    .fold(0., |s, (&a, &v)| a.mul_add(v, s));
            }
            receipt.legacy.bounded_residual_terms += nz * nz;
        }
        if out.iter().any(|v| !v.is_finite()) {
            return Err(bad("bounded vector action nonfinite"));
        }
        Ok(())
    })();
    receipt.linear.record(r.is_ok());
    if r.is_ok() {
        receipt.rhs_returned += 1;
    }
    r
}
fn vector_defect(
    c: &[f64],
    j: &IntervalEvalWorkspace,
    nx: usize,
    nz: usize,
    e: &mut [Iv],
) -> CoreResult<f64> {
    let n = nx + nz;
    let mut q = 0_f64;
    for i in 0..nz {
        let mut sum = 0.;
        for k in 0..nz {
            let v = (0..nz).try_fold(p(if i == k { 1. } else { 0. })?, |s, l| {
                s.sub(vector_row(j, n, nx + l)[nx + 1 + k].scale(c[i * nz + l])?)
            })?;
            e[i * nz + k] = v;
            sum = add_up(sum, v.mag())?;
        }
        q = q.max(sum);
    }
    Ok(q)
}
impl DaeSource {
    /// Opt-in bounded inverse action. Strict dominance is a proposal-domain
    /// restriction, not a substitute for the full original-source proof.
    pub fn initialize_bounded_vector_audited(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        refine: bool,
        scratch: &mut VectorProjectionWorkspace,
    ) -> (CoreResult<BoundedVectorProjection>, InitAttemptWork) {
        let mut receipt = InitAttemptWork::default();
        let r =
            self.bounded_vector_recorded(t, x, seed, branch, policy, refine, scratch, &mut receipt);
        (r, receipt)
    }
    fn bounded_vector_recorded(
        &self,
        t: f64,
        x: &[f64],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        refine: bool,
        w: &mut VectorProjectionWorkspace,
        receipt: &mut InitAttemptWork,
    ) -> CoreResult<BoundedVectorProjection> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx = self.nx();
        let nz = self.nz();
        let n = nx + nz;
        if !t.is_finite()
            || x.len() != nx
            || seed.len() != nz
            || branch.len() != nz
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
        let mut b = [z(); 32];
        let mut center = [0.; 32];
        let mut zz = [z(); 32];
        for i in 0..nz {
            b[i] = Iv::new(branch[i][0], branch[i][1])?;
            if b[i].lo >= b[i].hi || !b[i].contains(seed[i]) {
                return Err(bad("invalid branch or initial guess outside"));
            }
            center[i] = seed[i];
            zz[i] = p(seed[i])?;
        }
        receipt.admitted = true;
        vector_eval(self, t, x, &zz[..nz], &mut w.point, receipt)?;
        let original_residual = norm(&w.point.values[nx..n]);
        if policy.mode == InitMode::Strict && original_residual > policy.residual_tol {
            return Err(bad("strict initial residual exceeds tolerance"));
        }
        if policy.mode == InitMode::ProjectAlgebraic {
            for _ in 0..policy.max_newton {
                for i in 0..nz {
                    zz[i] = p(center[i])?;
                }
                vector_eval(self, t, x, &zz[..nz], &mut w.point, receipt)?;
                let r = norm(&w.point.values[nx..n]);
                if r == 0. {
                    break;
                }
                vector_inverse(w, nx, nz, receipt)?;
                let mut rhs = [0.; 32];
                for i in 0..nz {
                    rhs[i] = -mid(w.point.values[nx + i]);
                }
                let mut d = [0.; 32];
                vector_action(w, nz, &rhs[..nz], refine, &mut d[..nz], receipt)?;
                let mut accepted = false;
                for j in 0..16 {
                    let mut trial = [0.; 32];
                    for i in 0..nz {
                        trial[i] = 2f64.powi(-j).mul_add(d[i], center[i]);
                        zz[i] = p(trial[i])?;
                    }
                    if !subset(&zz[..nz], &b[..nz]) {
                        continue;
                    }
                    vector_eval(self, t, x, &zz[..nz], &mut w.eval, receipt)?;
                    if norm(&w.eval.values[nx..n]) < r {
                        center = trial;
                        accepted = true;
                        receipt.legacy.newton_steps += 1;
                        break;
                    }
                }
                if !accepted {
                    break;
                }
            }
        }
        for i in 0..nz {
            zz[i] = p(center[i])?;
        }
        vector_eval(self, t, x, &zz[..nz], &mut w.point, receipt)?;
        let residual = norm(&w.point.values[nx..n]);
        if residual > policy.residual_tol {
            return Err(bad("original constraint residual exceeds tolerance"));
        }
        vector_inverse(w, nx, nz, receipt)?;
        vector_eval(self, t, x, &b[..nz], &mut w.eval, receipt)?;
        let branch_defect = vector_defect(&w.c, &w.eval, nx, nz, &mut w.e)?;
        let gap = sub_down(1., branch_defect)?;
        if gap <= 0. {
            return Err(bad("whole branch contraction not proved"));
        }
        let mut cg = [z(); 32];
        for i in 0..nz {
            cg[i] = w.c[i * nz..(i + 1) * nz]
                .iter()
                .zip(&w.point.values[nx..n])
                .try_fold(z(), |s, (&c, &v)| s.add(v.scale(c)?))?;
        }
        let mut radius = mul_up(2., div_up(norm(&cg[..nz]), gap)?)?;
        let mut root = [z(); 32];
        let mut proved = false;
        for retry in 0..=8 {
            if retry == 1 {
                for &v in &center[..nz] {
                    radius = radius
                        .max(p(v.next_up())?.sub(p(v)?)?.mag())
                        .max(p(v)?.sub(p(v.next_down())?)?.mag());
                }
            } else if retry > 1 {
                radius = mul_up(2., radius)?;
            }
            for i in 0..nz {
                root[i] = p(center[i])?.add(Iv::new(-radius, radius)?)?;
                w.projection.b[i] = root[i].sub(p(center[i])?)?;
            }
            if !subset(&root[..nz], &b[..nz]) {
                break;
            }
            for i in 0..nz {
                let ed = w.e[i * nz..(i + 1) * nz]
                    .iter()
                    .zip(&w.projection.b[..nz])
                    .try_fold(z(), |s, (&a, &v)| s.add(a.mul(v)?))?;
                w.projection.c[i] = ed;
                w.projection.d[i] = p(center[i])?.sub(cg[i])?.add(ed)?;
            }
            if retry == 0 && subset(&w.projection.d[..nz], &root[..nz]) {
                proved = true;
                break;
            }
            receipt.legacy.root_map_rechecks += 1;
            for i in 0..nz {
                w.projection.d[i] = p(center[i])?.add(w.projection.c[i].sub(cg[i])?)?;
            }
            if subset(&w.projection.d[..nz], &root[..nz]) {
                if retry == 0 {
                    receipt.legacy.root_map_regrouped += 1;
                } else {
                    receipt.legacy.root_map_expanded += 1;
                }
                proved = true;
                break;
            }
        }
        if !proved {
            return Err(bad("local self-map inclusion not proved"));
        }
        let mut state_error = 0_f64;
        let mut correction = 0_f64;
        for i in 0..nz {
            state_error = state_error.max(distance(root[i], center[i])?);
            correction = correction.max(p(center[i])?.sub(p(seed[i])?)?.mag());
        }
        if state_error > policy.state_tol {
            return Err(bad("certified root distance exceeds tolerance"));
        }
        vector_eval(self, t, x, &root[..nz], &mut w.eval, receipt)?;
        let local_defect = vector_defect(&w.c, &w.eval, nx, nz, &mut w.e)?;
        let gap = sub_down(1., local_defect)?;
        if gap <= 0. {
            return Err(bad("rank/regularity not proved at root"));
        }
        let mut cnorm = 0_f64;
        let mut jnorm = 0_f64;
        for i in 0..nz {
            cnorm = cnorm.max(
                w.c[i * nz..(i + 1) * nz]
                    .iter()
                    .try_fold(0., |s, v| add_up(s, v.abs()))?,
            );
            jnorm = jnorm.max(
                vector_row(&w.eval, n, nx + i)[nx + 1..]
                    .iter()
                    .try_fold(0., |s, v| add_up(s, v.mag()))?,
            );
        }
        let inverse_norm = div_up(cnorm, gap)?;
        let condition = mul_up(inverse_norm, jnorm)?;
        if condition > policy.condition_cap {
            return Err(bad("certified condition exceeds cap"));
        }
        let mut xdot = [0.; 32];
        for i in 0..nx {
            xdot[i] = mid(w.point.values[i]);
        }
        let mut sol = [0.; PROJ_COLUMNS];
        let mut derivative_error = 0_f64;
        for col in 0..nx + 2 {
            let mut br = [0.; 32];
            let mut rhs = [z(); 32];
            for i in 0..nz {
                let gc = vector_row(&w.point, n, nx + i);
                let gr = vector_row(&w.eval, n, nx + i);
                if col <= nx {
                    br[i] = -mid(gc[col]);
                    rhs[i] = -gr[col];
                } else {
                    br[i] = mid(gc[1..nx + 1]
                        .iter()
                        .zip(&xdot[..nx])
                        .try_fold(-gc[0], |s, (&a, &v)| s.sub(a.scale(v)?))?);
                    rhs[i] = gr[1..nx + 1]
                        .iter()
                        .zip(&w.eval.values[..nx])
                        .try_fold(-gr[0], |s, (&a, &v)| s.sub(a.mul(v)?))?;
                }
            }
            let v = &mut sol[col * nz..(col + 1) * nz];
            vector_action(w, nz, &br[..nz], refine, v, receipt)?;
            let mut res = [z(); 32];
            for i in 0..nz {
                let a = vector_row(&w.eval, n, nx + i)[nx + 1..]
                    .iter()
                    .zip(&*v)
                    .try_fold(z(), |s, (&a, &v)| s.add(a.scale(v)?))?;
                res[i] = rhs[i].sub(a)?;
            }
            let mut e = 0_f64;
            for i in 0..nz {
                let pc = w.c[i * nz..(i + 1) * nz]
                    .iter()
                    .zip(&res[..nz])
                    .try_fold(z(), |s, (&c, &v)| s.add(v.scale(c)?))?;
                e = e.max(pc.mag());
            }
            e = div_up(e, gap)?;
            derivative_error = derivative_error.max(e);
            for i in 0..nz {
                w.projection.enclosures[col * nz + i] = p(v[i])?.add(Iv::new(-e, e)?)?;
            }
        }
        for i in 0..nx {
            derivative_error = derivative_error.max(distance(w.eval.values[i], xdot[i])?);
        }
        let mut reduced = [0.; 1024];
        let mut ft = [0.; 32];
        for i in 0..nx {
            let fc = vector_row(&w.point, n, i);
            let fr = vector_row(&w.eval, n, i);
            ft[i] = fc[nx + 1..]
                .iter()
                .zip(&sol[..nz])
                .fold(mid(fc[0]), |s, (&a, &v)| mid(a).mul_add(v, s));
            if !ft[i].is_finite() {
                return Err(bad("bounded vector time nonfinite"));
            }
            for k in 0..nx {
                let col = (k + 1) * nz;
                let v = fc[nx + 1..]
                    .iter()
                    .zip(&sol[col..col + nz])
                    .try_fold(fc[k + 1], |s, (&a, &v)| s.add(a.scale(v)?))?;
                let exact = fr[nx + 1..]
                    .iter()
                    .zip(&w.projection.enclosures[col..col + nz])
                    .try_fold(fr[k + 1], |s, (&a, &v)| s.add(a.mul(v)?))?;
                reduced[i * nx + k] = mid(v);
                derivative_error = derivative_error.max(distance(exact, mid(v))?);
            }
        }
        let mut tangent_residual = 0_f64;
        for i in 0..nz {
            let gc = vector_row(&w.point, n, nx + i);
            let r = gc[1..nx + 1]
                .iter()
                .zip(&xdot[..nx])
                .chain(gc[nx + 1..].iter().zip(&sol[(nx + 1) * nz..(nx + 2) * nz]))
                .try_fold(gc[0], |s, (&a, &v)| s.add(a.scale(v)?))?;
            tangent_residual = tangent_residual.max(r.mag());
        }
        if derivative_error > policy.derivative_tol || tangent_residual > policy.derivative_tol {
            return Err(bad("derivative/Schur certificate exceeds tolerance"));
        }
        let mut xx = [0.; 32];
        xx[..nx].copy_from_slice(x);
        let mut ss = [0.; 32];
        ss[..nz].copy_from_slice(seed);
        Ok(BoundedVectorProjection {
            nx,
            nz,
            route: if refine {
                InitRoute::BoundedVectorRefined
            } else {
                InitRoute::BoundedVector
            },
            mode: policy.mode,
            t,
            x: xx,
            seed: ss,
            center,
            branch: b,
            root,
            original_residual,
            residual,
            state_error,
            correction,
            branch_defect,
            local_defect,
            inverse_norm,
            condition,
            xdot,
            sol,
            reduced,
            ft,
            derivative_error,
            tangent_residual,
            work: receipt.legacy.clone(),
        })
    }
}
