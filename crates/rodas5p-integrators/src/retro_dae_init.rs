//! Opt-in, source-owned semi-explicit polynomial index-1 initialization.
//! All admission concerns the declared fixed-(t,x) branch box. It does not
//! infer the index of an arbitrary singular mass matrix or prove evolution.
use crate::{
    ParallelExecution,
    retro_event_poly::{DeclaredPolynomial, RepresentedPolynomial, RootRoute},
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
pub(crate) fn evaluate(polys: &[DeclaredPolynomial], t: Iv, y: &[Iv]) -> CoreResult<Eval> {
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
fn evaluate_row_bounded(
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
        let mut y = points(x)?;
        y.extend_from_slice(a);
        Ok((evaluate(&self.f, p(t)?, &y)?, evaluate(&self.g, p(t)?, &y)?))
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
        Ok(norm(&evaluate(&self.g, p(t)?, &points(y)?)?.v))
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
        let a = evaluate(&self.f, p(t)?, &y)?;
        let b = evaluate(&self.g, p(t)?, &y)?;
        Ok(a.v.iter().chain(&b.v).copied().map(mid).collect())
    }
    pub fn jacobian_values(&self, t: f64, y: &[f64]) -> CoreResult<Vec<f64>> {
        if y.len() != self.nx() + self.nz() {
            return Err(bad("state shape"));
        }
        let y = points(y)?;
        let a = evaluate(&self.f, p(t)?, &y)?;
        let b = evaluate(&self.g, p(t)?, &y)?;
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
        let a = evaluate(&self.f, p(t)?, &y)?;
        let b = evaluate(&self.g, p(t)?, &y)?;
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
        InitRoute::Direct | InitRoute::ScalarRoot => {
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
fn verified_root_box(
    c: &DenseMatrix,
    g: &[Iv],
    j: &[Vec<Iv>],
    center: &[f64],
    branch: &[Iv],
) -> CoreResult<(Vec<Iv>, f64)> {
    let e = defect(c, j)?;
    let q = rows_norm(&e)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("whole branch contraction not proved"));
    }
    let cg = matvec(c, g)?;
    let eta = norm(&cg);
    // For rounded box endpoints use explicit componentwise self-map
    // inclusion, not a floating-point assertion of eta/(1-q).
    let r = mul_up(2., div_up(eta, gap)?)?;
    let b = box_at(center, r)?;
    if !subset(&b, branch) {
        return Err(bad("existence box leaves declared branch"));
    }
    let delta = b
        .iter()
        .zip(center)
        .map(|(&v, &c)| v.sub(p(c)?))
        .collect::<CoreResult<Vec<_>>>()?;
    let ed = iv_matvec(&e, &delta)?;
    let mapped = center
        .iter()
        .zip(cg)
        .zip(ed)
        .map(|((&v, g), d)| p(v)?.sub(g)?.add(d))
        .collect::<CoreResult<Vec<_>>>()?;
    if !subset(&mapped, &b) {
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
        if route == InitRoute::ScalarRoot && nz != 1 {
            return Err(bad("scalar root route requires nz=1"));
        }
        let exec = if route == InitRoute::PreparedRayon {
            ParallelExecution::rayon(4)?
        } else {
            ParallelExecution::sequential()
        };
        let mut w = InitWork::default();
        let (_, g0) = self.eval(t, x, &points(z0)?)?;
        w.evaluations += 1;
        let original_residual_upper = norm(&g0.v);
        if policy.mode == InitMode::Strict && original_residual_upper > policy.residual_tol {
            return Err(bad("strict initial residual exceeds tolerance"));
        }
        let mut center = z0.to_vec();
        let mut scalar_box = None;
        if route == InitRoute::ScalarRoot {
            for v in [b[0].lo, b[0].hi] {
                let (_, g) = self.eval(t, x, &[p(v)?])?;
                w.evaluations += 1;
                if g.v[0].contains_zero() {
                    return Err(bad("scalar branch boundary root/unknown"));
                }
            }
            let poly = RepresentedPolynomial::frozen_scalar(&self.g[0], t, x, branch[0])?;
            let roots = poly.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-48))?;
            w.root_calls += 1;
            w.dictionary_uses += roots.work.dictionary_uses;
            w.conditioned_horner_uses += roots.work.conditioned_horner_uses;
            if roots.roots.len() != 1 || roots.roots[0].multiplicity != 1 {
                return Err(bad("scalar branch not exactly one simple root"));
            }
            let [l, h] = roots.roots[0].time;
            let q = Iv::new(l, h)?;
            if !subset(&[q], &b) {
                return Err(bad("scalar root box leaves branch"));
            }
            if policy.mode == InitMode::ProjectAlgebraic {
                center[0] = mid(q);
                // Polish the numerical center, not the exact isolation
                // certificate. Only admit an improving point in the box.
                let (_, v) = self.eval(t, x, &[p(center[0])?])?;
                w.evaluations += 1;
                let d = mid(v.j[0][1 + nx]);
                if d != 0. {
                    let trial = center[0] - mid(v.v[0]) / d;
                    if q.contains(trial) {
                        let (_, g) = self.eval(t, x, &[p(trial)?])?;
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
                let (_, g) = self.eval(t, x, &points(&center)?)?;
                w.evaluations += 1;
                let r = norm(&g.v);
                if r == 0. {
                    break;
                }
                let a = center_matrix(&self.jac_z(&g))?;
                let rhs = g.v.iter().copied().map(|v| -mid(v)).collect::<Vec<_>>();
                let d = batch(&a, &[rhs], route, &exec, &mut w)?.remove(0);
                let mut accepted = false;
                for j in 0..16 {
                    let s = 2f64.powi(-j);
                    let trial = center
                        .iter()
                        .zip(&d)
                        .map(|(&v, &d)| s.mul_add(d, v))
                        .collect::<Vec<_>>();
                    if !subset(&points(&trial)?, &b) {
                        continue;
                    }
                    let (_, v) = self.eval(t, x, &points(&trial)?)?;
                    w.evaluations += 1;
                    if norm(&v.v) < r {
                        center = trial;
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
        let (fc, gc) = self.eval(t, x, &points(&center)?)?;
        w.evaluations += 1;
        let residual_upper = norm(&gc.v);
        if residual_upper > policy.residual_tol {
            return Err(bad("original constraint residual exceeds tolerance"));
        }
        let a = center_matrix(&self.jac_z(&gc))?;
        let c = preconditioner(&a, route, &exec, &mut w)?;
        let (root, branch_defect_upper, authority) = if let Some(r) = scalar_box {
            (r, None, "exact scalar root count".to_string())
        } else {
            let (_, g) = self.eval(t, x, &b)?;
            w.evaluations += 1;
            let (r, q) = verified_root_box(&c, &gc.v, &self.jac_z(&g), &center, &b)?;
            (
                r,
                Some(q),
                "whole-branch contraction + local self-map".to_string(),
            )
        };
        let state_error_upper = root
            .iter()
            .zip(&center)
            .map(|(&v, &c)| distance(v, c))
            .collect::<CoreResult<Vec<_>>>()?
            .into_iter()
            .fold(0., f64::max);
        if state_error_upper > policy.state_tol {
            return Err(bad("certified root distance exceeds tolerance"));
        }
        let (fr, gr) = self.eval(t, x, &root)?;
        w.evaluations += 1;
        let j = self.jac_z(&gr);
        let q = rows_norm(&defect(&c, &j)?)?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("rank/regularity not proved at root"));
        }
        let ci = (0..nz)
            .map(|i| points(c.row(i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let inverse_norm_upper = div_up(rows_norm(&ci)?, gap)?;
        let condition_upper = mul_up(inverse_norm_upper, rows_norm(&j)?)?;
        if condition_upper > policy.condition_cap {
            return Err(bad("certified condition exceeds cap"));
        }
        // actual root RHS columns: z_t, each z_x column, and z_dot.
        let mut rhs = Vec::with_capacity(nx + 2);
        rhs.push(gr.j.iter().map(|r| -r[0]).collect::<Vec<_>>());
        for i in 0..nx {
            rhs.push(gr.j.iter().map(|r| -r[i + 1]).collect());
        }
        rhs.push(
            gr.j.iter()
                .map(|r| {
                    r[1..1 + nx]
                        .iter()
                        .zip(&fr.v)
                        .try_fold(-r[0], |s, (&g, &f)| s.sub(g.mul(f)?))
                })
                .collect::<CoreResult<Vec<_>>>()?,
        );
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
        let sol = batch(&a, &br, route, &exec, &mut w)?;
        let mut err = Vec::new();
        let mut enc = Vec::new();
        for (b, v) in rhs.iter().zip(&sol) {
            let av = iv_matvec(&j, &points(v)?)?;
            let residual = b
                .iter()
                .zip(av)
                .map(|(&b, a)| b.sub(a))
                .collect::<CoreResult<Vec<_>>>()?;
            let e = div_up(norm(&matvec(&c, &residual)?), gap)?;
            err.push(e);
            enc.push(box_at(v, e)?);
        }
        let mut derivative_error_upper = err.iter().copied().fold(0., f64::max);
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
                    .zip(&enc[1 + k])
                    .try_fold(fr.j[i][k + 1], |s, (&fz, &v)| s.add(fz.mul(v)?))?;
                let m = mid(v);
                *entry = m;
                derivative_error_upper = derivative_error_upper.max(distance(exact, m)?);
            }
        }
        let zdot = sol.last().unwrap().clone();
        let tangent_residual_upper = norm(
            &gc.j
                .iter()
                .map(|r| {
                    r[1..1 + nx]
                        .iter()
                        .zip(&xdot)
                        .chain(r[1 + nx..].iter().zip(&zdot))
                        .try_fold(r[0], |s, (&a, &v)| s.add(a.scale(v)?))
                })
                .collect::<CoreResult<Vec<_>>>()?,
        );
        if derivative_error_upper > policy.derivative_tol
            || tangent_residual_upper > policy.derivative_tol
        {
            return Err(bad("derivative/Schur certificate exceeds tolerance"));
        }
        let correction_upper = norm(
            &center
                .iter()
                .zip(z0)
                .map(|(&a, &b)| p(a)?.sub(p(b)?))
                .collect::<CoreResult<Vec<_>>>()?,
        );
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
            work: w,
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
