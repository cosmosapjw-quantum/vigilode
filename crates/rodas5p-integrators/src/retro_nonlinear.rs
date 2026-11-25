//! Bounded, source-owned whole-window one-stage implicit RK residual algebra.
//! A formal packet is a candidate, not an error certificate at xi=(1,...,1).
//! Original discrete and continuous residual bounds are independent.
use crate::affine_event_certificate::check_affine_event_arithmetic;
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, LuSolveWorkspace,
    directed::{Interval, add_up, div_up, mul_up, sqrt_up},
    sha256_hex,
};
type Iv = Interval;
const TABLE: &str = include_str!("retro_jet_table.txt");
pub const TABLE_SHA256: &str = "aca4c9cce0624f3fc8d8184e5eaeb2f56b85c26f5f63cfabfe605158305fd786";
const MAX_STEPS: usize = 256;
const MAX_N: usize = 32;
const MAX_TERMS: usize = 32;
const MAX_STORAGE: usize = 4_000_000;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro nonlinear: {s}"))
}
fn fail(s: &str) -> CoreError {
    CoreError::NonlinearSolve(format!("retro nonlinear: {s}"))
}
fn finite(v: &[f64]) -> CoreResult<()> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite("retro nonlinear".into()))
    }
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn zero() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm2(v: &[Iv]) -> CoreResult<f64> {
    sqrt_up(
        v.iter()
            .try_fold(0., |s, a| add_up(s, mul_up(a.mag(), a.mag())?))?,
    )
}
fn pow_iv(x: Iv, d: usize) -> CoreResult<Iv> {
    (0..d).try_fold(pt(1.)?, |a, _| a.mul(x))
}
fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1u32, |a, i| a * (n - i) as u32 / (i + 1) as u32) as f64
}

/// Immutable projection of the exact offline multiplication table for
/// R[x,y,z]/(x,y,z)^7. Coefficient zero is the unit channel. The pairs list
/// omits the unit; it is used for powers of positive-filtration increments.
#[derive(Clone, Debug)]
pub struct JetTable {
    axes: usize,
    degree: usize,
    exponents: Vec<[usize; 3]>,
    pairs: Vec<Vec<(usize, usize)>>,
}
impl JetTable {
    pub fn new(axes: usize, degree: usize) -> CoreResult<Self> {
        Self::from_bytes(TABLE, axes, degree)
    }
    pub fn from_bytes(text: &str, axes: usize, degree: usize) -> CoreResult<Self> {
        if !(1..=3).contains(&axes)
            || !(1..=6).contains(&degree)
            || text.len() > 8192
            || sha256_hex(text.as_bytes()) != TABLE_SHA256
        {
            return Err(bad("table identity or projection"));
        }
        let rows: Vec<Vec<usize>> = text
            .lines()
            .map(|l| {
                l.split_whitespace()
                    .map(|x| x.parse::<usize>().map_err(|_| bad("table integer")))
                    .collect()
            })
            .collect::<CoreResult<_>>()?;
        let mut map = vec![None; rows.len()];
        let mut exponents = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            if r.len() < 3 || !(r.len() - 3).is_multiple_of(2) {
                return Err(bad("table row"));
            }
            if r[..3].iter().sum::<usize>() <= degree && r[axes..3].iter().all(|&x| x == 0) {
                map[i] = Some(exponents.len());
                exponents.push([r[0], r[1], r[2]]);
            }
        }
        let mut pairs = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            if map[i].is_some() {
                let p = r[3..]
                    .chunks_exact(2)
                    .map(|c| {
                        Ok((
                            map.get(c[0])
                                .copied()
                                .flatten()
                                .ok_or_else(|| bad("table pair"))?,
                            map.get(c[1])
                                .copied()
                                .flatten()
                                .ok_or_else(|| bad("table pair"))?,
                        ))
                    })
                    .collect::<CoreResult<Vec<_>>>()?;
                pairs.push(p);
            }
        }
        Ok(Self {
            axes,
            degree,
            exponents,
            pairs,
        })
    }
    pub fn axes(&self) -> usize {
        self.axes
    }
    pub fn degree(&self) -> usize {
        self.degree
    }
    pub fn channels(&self) -> usize {
        self.exponents.len()
    }
    pub fn exponents(&self) -> &[[usize; 3]] {
        &self.exponents
    }
    pub fn multiply(&self, a: &[f64], b: &[f64]) -> CoreResult<Vec<f64>> {
        let n = self.channels();
        if a.len() != n || b.len() != n {
            return Err(bad("polynomial shape"));
        }
        finite(a)?;
        finite(b)?;
        let mut c = vec![0.; n];
        c[0] = a[0] * b[0];
        for (i, v) in c.iter_mut().enumerate().skip(1) {
            *v = a[0] * b[i]
                + a[i] * b[0]
                + self.pairs[i].iter().map(|&(j, k)| a[j] * b[k]).sum::<f64>();
        }
        finite(&c)?;
        Ok(c)
    }
}
#[derive(Clone, Debug)]
pub struct Term {
    pub beta: f64,
    pub v: Vec<f64>,
    pub degree: usize,
}
#[derive(Clone, Debug)]
pub struct Model {
    a: DenseMatrix,
    b: Vec<f64>,
    terms: Vec<Term>,
}
impl Model {
    pub fn new(a: DenseMatrix, b: Vec<f64>, terms: Vec<Term>) -> CoreResult<Self> {
        check_affine_event_arithmetic()?;
        let n = a.nrows();
        if n == 0 || n > MAX_N || a.ncols() != n || b.len() != n || terms.len() > MAX_TERMS {
            return Err(bad("model shape"));
        }
        finite(a.as_slice())?;
        finite(&b)?;
        for t in &terms {
            if t.v.len() != n
                || !t.beta.is_finite()
                || t.beta < 0.
                || ![3, 5, 7].contains(&t.degree)
            {
                return Err(bad("monotone term contract"));
            }
            finite(&t.v)?;
        }
        for i in 0..n {
            let mut r = a[(i, i)];
            for j in 0..n {
                if i != j {
                    r = add_up(r, pt(a[(i, j)])?.add(pt(a[(j, i)])?)?.scale(0.5)?.mag())?;
                }
            }
            if r > 0. {
                return Err(bad("symmetric part not certified nonpositive"));
            }
        }
        Ok(Self { a, b, terms })
    }
    pub fn dimension(&self) -> usize {
        self.b.len()
    }
    pub fn rhs(&self, y: &[f64]) -> CoreResult<Vec<f64>> {
        let n = self.dimension();
        if y.len() != n {
            return Err(bad("state shape"));
        }
        finite(y)?;
        let mut f = self.a.matvec(y)?;
        for (a, &b) in f.iter_mut().zip(&self.b) {
            *a += b;
        }
        for t in &self.terms {
            let c = t.beta * dot(&t.v, y).powi(t.degree as i32);
            for (a, &v) in f.iter_mut().zip(&t.v) {
                *a -= c * v;
            }
        }
        finite(&f)?;
        Ok(f)
    }
    pub fn jacobian(&self, y: &[f64]) -> CoreResult<DenseMatrix> {
        let n = self.dimension();
        if y.len() != n {
            return Err(bad("Jacobian state shape"));
        }
        finite(y)?;
        let mut j = self.a.clone();
        for t in &self.terms {
            let c = t.beta * t.degree as f64 * dot(&t.v, y).powi((t.degree - 1) as i32);
            for i in 0..n {
                for k in 0..n {
                    j[(i, k)] -= c * t.v[i] * t.v[k];
                }
            }
        }
        finite(j.as_slice())?;
        Ok(j)
    }
    fn rhs_iv(&self, y: &[Iv]) -> CoreResult<Vec<Iv>> {
        let n = self.dimension();
        let mut f = Vec::with_capacity(n);
        for i in 0..n {
            let mut s = pt(self.b[i])?;
            for (&a, &x) in self.a.row(i).iter().zip(y) {
                s = s.add(x.scale(a)?)?;
            }
            f.push(s);
        }
        for t in &self.terms {
            let s =
                t.v.iter()
                    .zip(y)
                    .try_fold(zero(), |a, (&v, &x)| a.add(x.scale(v)?))?;
            let c = pow_iv(s, t.degree)?.scale(t.beta)?;
            for (a, &v) in f.iter_mut().zip(&t.v) {
                *a = a.sub(c.scale(v)?)?;
            }
        }
        Ok(f)
    }
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Work {
    pub source_evals: usize,
    pub jacobians: usize,
    pub factorizations: usize,
    pub linear_solves: usize,
    pub coefficient_pairs: usize,
    pub recenterings: usize,
    pub corrections: usize,
    pub certified_steps: usize,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Certificate {
    pub accepted: bool,
    pub residual_upper: f64,
    pub error_upper: f64,
    pub node_upper: Vec<f64>,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct ContinuousCertificate {
    /// Ideal cubic Hermite path, original physical IVP, not the discrete one-stage RK solution.
    pub error_upper: f64,
    pub node_upper: Vec<f64>,
    pub local_defect_integrals: Vec<f64>,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub values: Vec<f64>,
    pub certificate: Certificate,
    pub work: Work,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Mode {
    Sequential,
    Chord,
    Jet,
}

pub struct Window<'a> {
    model: &'a Model,
    y0: Vec<f64>,
    h: f64,
    steps: usize,
    theta: f64,
}
impl<'a> Window<'a> {
    /// Implicit midpoint default for this research client only.
    pub fn new(model: &'a Model, y0: Vec<f64>, h: f64, steps: usize) -> CoreResult<Self> {
        Self::with_theta(model, y0, h, steps, 0.5)
    }
    /// One-stage RK with A=theta, b=1; not the nonlinear endpoint theta
    /// method. theta=1 is backward Euler, theta=.5 is midpoint.
    pub fn with_theta(
        model: &'a Model,
        y0: Vec<f64>,
        h: f64,
        steps: usize,
        theta: f64,
    ) -> CoreResult<Self> {
        if !theta.is_finite() || !(0.5..=1.).contains(&theta) {
            return Err(bad("theta outside dissipative contract"));
        }
        if y0.len() != model.dimension()
            || !h.is_finite()
            || h <= 0.
            || !(1..=MAX_STEPS).contains(&steps)
            || !(h * steps as f64).is_finite()
        {
            return Err(bad("window domain"));
        }
        finite(&y0)?;
        Ok(Self {
            model,
            y0,
            h,
            steps,
            theta,
        })
    }
    pub fn dimension(&self) -> usize {
        self.model.dimension()
    }
    pub fn steps(&self) -> usize {
        self.steps
    }
    pub fn h(&self) -> f64 {
        self.h
    }
    fn validate(&self, x: &[f64]) -> CoreResult<()> {
        if x.len() != self.dimension() * self.steps {
            return Err(bad("trajectory shape"));
        }
        finite(x)
    }
    fn ends<'b>(&'b self, x: &'b [f64], i: usize) -> (&'b [f64], &'b [f64]) {
        let n = self.dimension();
        (
            if i == 0 {
                &self.y0
            } else {
                &x[(i - 1) * n..i * n]
            },
            &x[i * n..(i + 1) * n],
        )
    }
    fn stage(&self, a: &[f64], b: &[f64]) -> Vec<f64> {
        a.iter()
            .zip(b)
            .map(|(&a, &b)| (1. - self.theta) * a + self.theta * b)
            .collect()
    }
    pub fn theta(&self) -> f64 {
        self.theta
    }
    fn local_residual(&self, a: &[f64], b: &[f64], w: &mut Work) -> CoreResult<Vec<f64>> {
        let mid = self.stage(a, b);
        w.source_evals += 1;
        let f = self.model.rhs(&mid)?;
        let r: Vec<_> = a
            .iter()
            .zip(b)
            .zip(f)
            .map(|((&a, &b), f)| b - a - self.h * f)
            .collect();
        finite(&r)?;
        Ok(r)
    }
    fn residual(&self, x: &[f64], w: &mut Work) -> CoreResult<Vec<f64>> {
        self.validate(x)?;
        let mut r = Vec::with_capacity(x.len());
        for i in 0..self.steps {
            let (a, b) = self.ends(x, i);
            r.extend(self.local_residual(a, b, w)?);
        }
        Ok(r)
    }
    fn residual_iv(&self, a: &[f64], b: &[f64]) -> CoreResult<Vec<Iv>> {
        let mid = a
            .iter()
            .zip(b)
            .map(|(&a, &b)| {
                if self.theta == 0.5 {
                    pt(a)?.add(pt(b)?)?.scale(0.5)
                } else {
                    pt(a)?
                        .scale(1. - self.theta)?
                        .add(pt(b)?.scale(self.theta)?)
                }
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let f = self.model.rhs_iv(&mid)?;
        a.iter()
            .zip(b)
            .zip(f)
            .map(|((&a, &b), f)| pt(b)?.sub(pt(a)?)?.sub(f.scale(self.h)?))
            .collect()
    }
    pub fn certify(&self, x: &[f64], tol: f64) -> CoreResult<Certificate> {
        check_affine_event_arithmetic()?;
        self.validate(x)?;
        if !tol.is_finite() || tol <= 0. {
            return Err(bad("certificate tolerance"));
        }
        let mut e = 0.;
        let mut rmax = 0.0_f64;
        let mut nodes = Vec::with_capacity(self.steps);
        for i in 0..self.steps {
            let (a, b) = self.ends(x, i);
            let r = norm2(&self.residual_iv(a, b)?)?;
            rmax = rmax.max(r);
            e = add_up(e, r)?;
            nodes.push(e);
        }
        Ok(Certificate {
            accepted: e <= tol,
            residual_upper: rmax,
            error_upper: e,
            node_upper: nodes,
        })
    }
    /// Linear-only one-stage RK forecast over the full window. The initial state
    /// stays fixed; the forecast is never accepted as a boundary condition.
    pub fn forecast(&self, w: &mut Work) -> CoreResult<Vec<f64>> {
        let n = self.dimension();
        let (d, e) = blocks(&self.model.a, self.h, self.theta)?;
        let lu = LuFactorization::new(&d)?;
        w.factorizations += 1;
        let mut scratch = LuSolveWorkspace::new(n);
        let mut x = self.y0.clone();
        let mut out = Vec::with_capacity(n * self.steps);
        for _ in 0..self.steps {
            let mut r = e.matvec(&x)?;
            for (a, &b) in r.iter_mut().zip(&self.model.b) {
                *a += self.h * b;
            }
            lu.solve_in_place_with_workspace(&mut r, &mut scratch)?;
            w.linear_solves += 1;
            out.extend(&r);
            x = r;
        }
        Ok(out)
    }
    pub fn lift(&self, table: &JetTable, center: &[f64], w: &mut Work) -> CoreResult<Packet> {
        let frozen = Frozen::new(self, center, w)?;
        frozen.lift(table, w)
    }
    /// Independently evaluates the exact residual homotopy target, including
    /// the original constant residual at the center. Diagnostic only.
    pub fn homotopy_residual(
        &self,
        center: &[f64],
        x: &[f64],
        parameters: &[f64],
    ) -> CoreResult<f64> {
        if !(1..=3).contains(&parameters.len()) {
            return Err(bad("homotopy parameter count"));
        }
        finite(parameters)?;
        let a = self.residual(center, &mut Work::default())?;
        let b = self.residual(x, &mut Work::default())?;
        Ok(a.iter()
            .zip(b)
            .enumerate()
            .fold(0.0_f64, |r, (i, (&a, b))| {
                r.max((b + (parameters[(i / self.dimension()) % parameters.len()] - 1.) * a).abs())
            }))
    }
    pub fn solve(
        &self,
        table: &JetTable,
        mode: Mode,
        tol: f64,
        initial: Option<&[f64]>,
    ) -> CoreResult<Report> {
        if !tol.is_finite() || tol <= 0. {
            return Err(bad("solve tolerance"));
        }
        let mut w = Work::default();
        let mut x = if mode == Mode::Sequential {
            self.sequential(tol, &mut w)?
        } else if let Some(v) = initial {
            self.validate(v)?;
            v.to_vec()
        } else {
            self.forecast(&mut w)?
        };
        let mut cert = self.certify(&x, tol)?;
        w.certified_steps += self.steps;
        if !cert.accepted && mode == Mode::Chord {
            let mut frozen = Frozen::new(self, &x, &mut w)?;
            for _ in 0..32 {
                let r = self
                    .residual(&x, &mut w)?
                    .into_iter()
                    .map(|v| -v)
                    .collect::<Vec<_>>();
                let dx = frozen.linear(&r, &mut w)?;
                for (a, b) in x.iter_mut().zip(dx) {
                    *a += b;
                }
                w.corrections += 1;
                cert = self.certify(&x, tol)?;
                w.certified_steps += self.steps;
                if cert.accepted {
                    break;
                }
            }
        } else if !cert.accepted && mode == Mode::Jet {
            for _ in 0..4 {
                let packet = self.lift(table, &x, &mut w)?;
                let next = packet.evaluate(&vec![1.; table.axes])?;
                w.recenterings += 1;
                let c = self.certify(&next, tol)?;
                w.certified_steps += self.steps;
                // Fail closed on worsening residual/error bound.
                if c.error_upper >= cert.error_upper && !c.accepted {
                    break;
                }
                x = next;
                cert = c;
                if cert.accepted {
                    break;
                }
            }
        }
        Ok(Report {
            values: x,
            certificate: cert,
            work: w,
        })
    }
    fn sequential(&self, tol: f64, w: &mut Work) -> CoreResult<Vec<f64>> {
        let n = self.dimension();
        let mut scratch = LuSolveWorkspace::new(n);
        let mut old = self.y0.clone();
        let mut all = Vec::with_capacity(n * self.steps);
        for _ in 0..self.steps {
            let mut x = old.clone();
            let mut done = false;
            for _ in 0..32 {
                let r = self.local_residual(&old, &x, w)?;
                if dot(&r, &r).sqrt() <= tol / (4. * self.steps as f64) {
                    done = true;
                    break;
                }
                let mid = self.stage(&old, &x);
                let j = self.model.jacobian(&mid)?;
                w.jacobians += 1;
                let (d, _) = blocks(&j, self.h, self.theta)?;
                let lu = LuFactorization::new(&d)?;
                w.factorizations += 1;
                let mut dx = r.into_iter().map(|a| -a).collect::<Vec<_>>();
                lu.solve_in_place_with_workspace(&mut dx, &mut scratch)?;
                w.linear_solves += 1;
                for (a, b) in x.iter_mut().zip(dx) {
                    *a += b;
                }
                finite(&x)?;
            }
            if !done {
                return Err(fail("sequential Newton iteration cap"));
            }
            all.extend(&x);
            old = x;
        }
        Ok(all)
    }
    /// Ideal cubic Hermite reconstruction through represented nodes, using
    /// exact mathematical f at those nodes. The integral of the original
    /// normalized continuous defect is bounded from polynomial coefficients.
    pub fn continuous_certificate(&self, x: &[f64]) -> CoreResult<ContinuousCertificate> {
        check_affine_event_arithmetic()?;
        self.validate(x)?;
        let n = self.dimension();
        let mut prev_f = self.model.rhs_iv(
            &self
                .y0
                .iter()
                .map(|&x| pt(x))
                .collect::<CoreResult<Vec<_>>>()?,
        )?;
        let mut e = 0.;
        let mut nodes = Vec::new();
        let mut locals = Vec::new();
        for i in 0..self.steps {
            let (a, b) = self.ends(x, i);
            let next_f = self
                .model
                .rhs_iv(&b.iter().map(|&x| pt(x)).collect::<CoreResult<Vec<_>>>()?)?;
            let mut u = vec![[zero(); 4]; n];
            for j in 0..n {
                let delta = pt(b[j])?.sub(pt(a[j])?)?;
                let p = prev_f[j].scale(self.h)?;
                let q = next_f[j].scale(self.h)?;
                u[j] = [
                    pt(a[j])?,
                    p,
                    delta.scale(3.)?.sub(p.scale(2.)?.add(q)?)?,
                    delta.scale(-2.)?.add(p)?.add(q)?,
                ];
            }
            let mut r = vec![vec![zero(); n]; 22];
            for j in 0..n {
                r[0][j] = pt(self.model.b[j])?.scale(-self.h)?;
                for k in 0..3 {
                    r[k][j] = r[k][j].add(u[j][k + 1].scale((k + 1) as f64)?)?;
                }
                for (k, row) in r.iter_mut().enumerate().take(4) {
                    for (&a, v) in self.model.a.row(j).iter().zip(&u) {
                        row[j] = row[j].sub(v[k].scale(a)?.scale(self.h)?)?;
                    }
                }
            }
            for t in &self.model.terms {
                let mut s = [zero(); 4];
                for (&v, p) in t.v.iter().zip(&u) {
                    for (k, a) in s.iter_mut().enumerate() {
                        *a = a.add(p[k].scale(v)?)?;
                    }
                }
                let mut power = vec![pt(1.)?];
                for _ in 0..t.degree {
                    power = poly_mul(&power, &s)?;
                }
                for (k, p) in power.into_iter().enumerate() {
                    let p = p.scale(t.beta)?.scale(self.h)?;
                    for (a, &v) in r[k].iter_mut().zip(&t.v) {
                        *a = a.add(p.scale(v)?)?;
                    }
                }
            }
            let mut local = 0.;
            for (k, row) in r.iter().enumerate() {
                local = add_up(local, div_up(norm2(row)?, (k + 1) as f64)?)?;
            }
            locals.push(local);
            e = add_up(e, local)?;
            nodes.push(e);
            prev_f = next_f;
        }
        Ok(ContinuousCertificate {
            error_upper: e,
            node_upper: nodes,
            local_defect_integrals: locals,
        })
    }
}
fn poly_mul(a: &[Iv], b: &[Iv]) -> CoreResult<Vec<Iv>> {
    let mut p = vec![zero(); a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            p[i + j] = p[i + j].add(x.mul(y)?)?;
        }
    }
    Ok(p)
}
fn blocks(j: &DenseMatrix, h: f64, theta: f64) -> CoreResult<(DenseMatrix, DenseMatrix)> {
    let n = j.nrows();
    let d = DenseMatrix::identity(n).combine(j, -theta * h)?;
    let e = DenseMatrix::identity(n).combine(j, (1. - theta) * h)?;
    finite(d.as_slice())?;
    finite(e.as_slice())?;
    Ok((d, e))
}
struct Frozen<'a, 'b> {
    window: &'a Window<'b>,
    center: Vec<f64>,
    residual: Vec<f64>,
    factors: Vec<LuFactorization>,
    e: Vec<DenseMatrix>,
    base: Vec<f64>,
    scratch: LuSolveWorkspace,
}
impl<'a, 'b> Frozen<'a, 'b> {
    fn new(win: &'a Window<'b>, x: &[f64], w: &mut Work) -> CoreResult<Self> {
        let n = win.dimension();
        let mut factors = Vec::new();
        let mut e = Vec::new();
        let mut base = Vec::new();
        let residual = win.residual(x, w)?;
        for i in 0..win.steps {
            let (a, b) = win.ends(x, i);
            let mid = win.stage(a, b);
            let j = win.model.jacobian(&mid)?;
            w.jacobians += 1;
            let (d, ei) = blocks(&j, win.h, win.theta)?;
            factors.push(LuFactorization::new(&d)?);
            w.factorizations += 1;
            e.push(ei);
            base.extend(win.model.terms.iter().map(|t| dot(&t.v, &mid)));
        }
        finite(&base)?;
        Ok(Self {
            window: win,
            center: x.to_vec(),
            residual,
            factors,
            e,
            base,
            scratch: LuSolveWorkspace::new(n),
        })
    }
    fn linear(&mut self, rhs: &[f64], w: &mut Work) -> CoreResult<Vec<f64>> {
        let n = self.window.dimension();
        let mut x = vec![0.; rhs.len()];
        let mut prev = vec![0.; n];
        for i in 0..self.window.steps {
            let mut v = self.e[i].matvec(&prev)?;
            for (a, &b) in v.iter_mut().zip(&rhs[i * n..(i + 1) * n]) {
                *a += b;
            }
            self.factors[i].solve_in_place_with_workspace(&mut v, &mut self.scratch)?;
            w.linear_solves += 1;
            x[i * n..(i + 1) * n].copy_from_slice(&v);
            prev = v;
        }
        Ok(x)
    }
    fn lift(mut self, table: &JetTable, w: &mut Work) -> CoreResult<Packet> {
        let win = self.window;
        let n = win.dimension();
        let m = win.steps;
        let terms = &win.model.terms;
        let nt = terms.len();
        let nc = table.channels();
        let count = m * nt * 8 * nc;
        if count > MAX_STORAGE {
            return Err(bad("jet workspace cap"));
        }
        let mut powers = vec![0.; count];
        let mut coefs = vec![vec![0.; m * n]; nc];
        coefs[0].clone_from(&self.center);
        let mut multipliers = vec![[0.; 8]; m * nt];
        for (i, base) in self.base.iter().enumerate() {
            let t = &terms[i % nt];
            for (k, value) in multipliers[i]
                .iter_mut()
                .enumerate()
                .take(t.degree + 1)
                .skip(2)
            {
                *value = win.h * t.beta * choose(t.degree, k) * base.powi((t.degree - k) as i32);
            }
        }
        let index = |i: usize, t: usize, k: usize, a: usize| (((i * nt + t) * 8 + k) * nc) + a;
        for (a, &ex) in table.exponents.iter().enumerate().skip(1) {
            let degree = ex.iter().sum::<usize>();
            let axis = ex.iter().position(|&v| v != 0).unwrap();
            let mut rhs = vec![0.; m * n];
            for i in 0..m {
                if degree == 1 && i % table.axes == axis {
                    for (r, &v) in rhs[i * n..(i + 1) * n]
                        .iter_mut()
                        .zip(&self.residual[i * n..(i + 1) * n])
                    {
                        *r = -v;
                    }
                }
                for (t, term) in terms.iter().enumerate() {
                    let mut nonlinear = 0.;
                    for k in 2..=term.degree.min(degree) {
                        let mut p = 0.;
                        for &(b, c) in &table.pairs[a] {
                            p += powers[index(i, t, 1, b)] * powers[index(i, t, k - 1, c)];
                        }
                        powers[index(i, t, k, a)] = p;
                        w.coefficient_pairs += table.pairs[a].len();
                        nonlinear += multipliers[i * nt + t][k] * p;
                    }
                    for (r, &v) in rhs[i * n..(i + 1) * n].iter_mut().zip(&term.v) {
                        *r -= nonlinear * v;
                    }
                }
            }
            let x = self.linear(&rhs, w)?;
            for i in 0..m {
                for (t, term) in terms.iter().enumerate() {
                    let mut s = 0.;
                    for (j, &v) in term.v.iter().enumerate() {
                        s += v
                            * (win.theta * x[i * n + j]
                                + if i == 0 {
                                    0.
                                } else {
                                    (1. - win.theta) * x[(i - 1) * n + j]
                                });
                    }
                    powers[index(i, t, 1, a)] = s;
                }
            }
            coefs[a] = x;
        }
        for x in &coefs {
            finite(x)?;
        }
        Ok(Packet {
            axes: table.axes,
            exponents: table.exponents.clone(),
            coefficients: coefs,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Packet {
    axes: usize,
    exponents: Vec<[usize; 3]>,
    coefficients: Vec<Vec<f64>>,
}
impl Packet {
    pub fn coefficients(&self) -> &[Vec<f64>] {
        &self.coefficients
    }
    pub fn exponents(&self) -> &[[usize; 3]] {
        &self.exponents
    }
    pub fn evaluate(&self, xi: &[f64]) -> CoreResult<Vec<f64>> {
        if xi.len() != self.axes {
            return Err(bad("packet parameter shape"));
        }
        finite(xi)?;
        let mut x = self.coefficients[0].clone();
        for (ex, c) in self.exponents.iter().zip(&self.coefficients).skip(1) {
            let s = xi
                .iter()
                .zip(ex)
                .map(|(&x, &k)| x.powi(k as i32))
                .product::<f64>();
            for (a, &b) in x.iter_mut().zip(c) {
                *a += s * b;
            }
        }
        finite(&x)?;
        Ok(x)
    }
}
