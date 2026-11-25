//! Bounded research primitives: original residuals and immutable source
//! ownership precede acceptance. No default route, arbitrary callbacks or
//! DAE index is silently changed.
use crate::{
    ParallelExecution,
    retro_portfolio::{Portfolio, Route, Selection},
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, LuSolveWorkspace, direct_solve,
    directed::{Interval, add_up, div_up, exp_interval, mul_up, sub_down},
    retro_dictionary::{Dictionary, RealMode},
};
use std::time::Instant;
type Iv = Interval;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro foundation: {s}"))
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn zero() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn infnorm(a: &[f64]) -> f64 {
    a.iter().fold(0.0_f64, |m, v| m.max(v.abs()))
}
fn valid_matrix(a: &DenseMatrix, n: usize) -> bool {
    a.nrows() == n && a.ncols() == n && a.as_slice().iter().all(|v| v.is_finite())
}
fn rownorm(a: &DenseMatrix) -> CoreResult<f64> {
    let mut norm = 0.0_f64;
    for i in 0..a.nrows() {
        norm = norm.max(a.row(i).iter().try_fold(0., |s, v| add_up(s, v.abs()))?);
    }
    Ok(norm)
}
fn mat_iv(a: &DenseMatrix, x: &[Iv]) -> CoreResult<Vec<Iv>> {
    (0..a.nrows())
        .map(|i| {
            a.row(i)
                .iter()
                .zip(x)
                .try_fold(zero(), |s, (&a, &b)| s.add(b.scale(a)?))
        })
        .collect()
}
fn enclosing_gap(a: &DenseMatrix) -> CoreResult<f64> {
    let n = a.nrows();
    if n == 0 || !valid_matrix(a, n) {
        return Err(bad("matrix shape"));
    }
    let mut gap = f64::INFINITY;
    for i in 0..n {
        let mut g = a[(i, i)].abs();
        for j in 0..n {
            if i != j {
                g = sub_down(g, a[(i, j)].abs())?;
            }
        }
        gap = gap.min(g);
    }
    if gap <= 0. || !gap.is_finite() {
        return Err(bad("strict row dominance not proved"));
    }
    Ok(gap)
}
fn residual(a: &DenseMatrix, b: &[f64], x: &[f64]) -> CoreResult<f64> {
    if b.len() != a.nrows() || x.len() != a.ncols() || b.iter().chain(x).any(|v| !v.is_finite()) {
        return Err(bad("residual shape/nonfinite"));
    }
    let mut r = 0.0_f64;
    for (i, &bi) in b.iter().enumerate() {
        let v = a
            .row(i)
            .iter()
            .zip(x)
            .try_fold(pt(bi)?, |s, (&a, &x)| s.sub(pt(a)?.scale(x)?))?;
        r = r.max(v.mag());
    }
    Ok(r)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchMode {
    FreshFactor,
    SerialReuse,
    FaerBatch,
    RayonChunks,
}
#[derive(Debug)]
pub struct BatchReport {
    pub values: Vec<Vec<f64>>,
    pub error_upper: f64,
    pub residual_upper: f64,
    pub setup_ns: u64,
}
/// Owns the exact represented matrix and its factors; no lookup by shape
/// is permitted. A changed source requires a new PreparedRhs owner.
pub struct PreparedRhs {
    a: DenseMatrix,
    lu: LuFactorization,
    scratch: LuSolveWorkspace,
    gap: f64,
    par: faer::Par,
    setup_ns: u64,
}
impl PreparedRhs {
    pub fn new(a: DenseMatrix) -> CoreResult<Self> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let start = Instant::now();
        let gap = enclosing_gap(&a)?;
        if a.nrows() > 2048 {
            return Err(bad("prepared dimension cap"));
        }
        let lu = LuFactorization::new(&a)?;
        let scratch = LuSolveWorkspace::new(a.nrows());
        Ok(Self {
            a,
            lu,
            scratch,
            gap,
            par: faer::get_global_parallelism(),
            setup_ns: start.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        })
    }
    pub fn matrix(&self) -> &DenseMatrix {
        &self.a
    }
    pub fn setup_ns(&self) -> u64 {
        self.setup_ns
    }
    pub fn matches(&self, a: &DenseMatrix) -> bool {
        self.a.nrows() == a.nrows()
            && self.a.ncols() == a.ncols()
            && self
                .a
                .as_slice()
                .iter()
                .zip(a.as_slice())
                .all(|(x, y)| x.to_bits() == y.to_bits())
    }
    pub fn solve(
        &mut self,
        rhs: &[Vec<f64>],
        mode: BatchMode,
        exec: &ParallelExecution,
    ) -> CoreResult<BatchReport> {
        if self.par != faer::get_global_parallelism() {
            return Err(bad("faer parallelism changed since preparation"));
        }
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = self.a.nrows();
        if rhs.is_empty()
            || rhs.len() > 4096
            || rhs
                .iter()
                .any(|v| v.len() != n || v.iter().any(|v| !v.is_finite()))
        {
            return Err(bad("batch shape"));
        }
        let values = match mode {
            BatchMode::FreshFactor => rhs
                .iter()
                .map(|b| direct_solve(&self.a, b))
                .collect::<CoreResult<Vec<_>>>()?,
            BatchMode::SerialReuse => {
                let mut x = rhs.to_vec();
                for v in &mut x {
                    self.lu
                        .solve_in_place_with_workspace(v, &mut self.scratch)?;
                }
                x
            }
            BatchMode::FaerBatch => self.lu.solve_rows(rhs)?,
            BatchMode::RayonChunks => {
                // Local Rayon workers each own scratch. Avoid nested faer
                // parallelism; global faer policy is never changed.
                if faer::get_global_parallelism() != faer::Par::Seq {
                    return Err(bad("Rayon requires sequential faer policy"));
                }
                let size = rhs.len().div_ceil(exec.threads()).max(1);
                let chunks = rhs.chunks(size).collect::<Vec<_>>();
                exec.map_ordered(&chunks, |c| {
                    let mut ws = LuSolveWorkspace::new(n);
                    let mut x = c.to_vec();
                    for v in &mut x {
                        self.lu.solve_in_place_with_workspace(v, &mut ws)?;
                    }
                    Ok(x)
                })?
                .into_iter()
                .flatten()
                .collect()
            }
        };
        // Every RHS is independently checked against the ORIGINAL matrix,
        // also parallel when the caller explicitly supplies a local pool.
        let pairs = rhs.iter().zip(&values).collect::<Vec<_>>();
        let rr = exec.map_ordered(&pairs, |(b, x)| {
            crate::affine_event_certificate::check_affine_event_arithmetic()?;
            residual(&self.a, b, x)
        })?;
        let r = rr.into_iter().fold(0., f64::max);
        let error_upper = div_up(r, self.gap)?;
        Ok(BatchReport {
            values,
            error_upper,
            residual_upper: r,
            setup_ns: self.setup_ns,
        })
    }
    pub fn adaptive(
        &mut self,
        p: &mut Portfolio,
        rhs: &[Vec<f64>],
        exec: &ParallelExecution,
        tol: f64,
    ) -> CoreResult<Selection<BatchReport>> {
        let routes = if rhs.len() >= 4 {
            [
                Route::PreparedFaerBatch,
                Route::PreparedSerial,
                Route::PreparedRayon,
            ]
        } else {
            [
                Route::PreparedSerial,
                Route::PreparedFaerBatch,
                Route::PreparedRayon,
            ]
        };
        p.choose(
            "prepared-rhs",
            &[
                self.a.nrows() as u64,
                rhs.len() as u64,
                self.gap.to_bits(),
                exec.threads() as u64,
            ],
            &routes,
            tol,
            |r| {
                let m = match r {
                    Route::PreparedSerial => BatchMode::SerialReuse,
                    Route::PreparedFaerBatch => BatchMode::FaerBatch,
                    _ => BatchMode::RayonChunks,
                };
                let v = self.solve(rhs, m, exec)?;
                let e = v.error_upper;
                Ok((v, e))
            },
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeMode {
    Sequential,
    OrderedScan,
}
#[derive(Debug)]
pub struct TimeReport {
    pub values: Vec<Vec<f64>>,
    pub node_upper: Vec<f64>,
    pub error_upper: f64,
}
/// Original aperiodic recurrence, not a periodic FFT surrogate:
/// y_i=G_i y_(i-1)+b_i. Arbitrary ordered noncommuting transitions.
pub struct Aperiodic {
    g: Vec<DenseMatrix>,
    gains: Vec<f64>,
    n: usize,
}
fn compose(
    left: &(DenseMatrix, Vec<f64>),
    g: &DenseMatrix,
    rhs: &[f64],
) -> CoreResult<(DenseMatrix, Vec<f64>)> {
    let a = g.matmul(&left.0)?;
    let mut b = g.matvec(&left.1)?;
    for (x, v) in b.iter_mut().zip(rhs) {
        *x += v;
    }
    if a.as_slice().iter().chain(&b).any(|v| !v.is_finite()) {
        return Err(bad("composition overflow"));
    }
    Ok((a, b))
}
impl Aperiodic {
    pub fn new(g: Vec<DenseMatrix>) -> CoreResult<Self> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = g.first().map_or(0, DenseMatrix::nrows);
        if n == 0 || n > 128 || g.len() > 4096 || g.iter().any(|x| !valid_matrix(x, n)) {
            return Err(bad("aperiodic shape"));
        }
        let gains = g.iter().map(rownorm).collect::<CoreResult<Vec<_>>>()?;
        Ok(Self { g, gains, n })
    }
    fn check(&self, y: &[f64], b: &[Vec<f64>]) -> CoreResult<()> {
        if y.len() != self.n
            || b.len() != self.g.len()
            || b.iter().any(|v| v.len() != self.n)
            || y.iter().chain(b.iter().flatten()).any(|v| !v.is_finite())
        {
            Err(bad("aperiodic RHS"))
        } else {
            Ok(())
        }
    }
    pub fn certify(&self, y: &[f64], b: &[Vec<f64>], x: Vec<Vec<f64>>) -> CoreResult<TimeReport> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        self.check(y, b)?;
        self.check(y, &x)?;
        let mut e = 0.;
        let mut prev = y;
        let mut node_upper = Vec::new();
        for (i, v) in x.iter().enumerate() {
            let mut r = 0.0_f64;
            for (j, &vj) in v.iter().enumerate() {
                let a = self.g[i]
                    .row(j)
                    .iter()
                    .zip(prev)
                    .try_fold(pt(b[i][j])?, |s, (&a, &x)| s.add(pt(a)?.scale(x)?))?;
                r = r.max(a.sub(pt(vj)?)?.mag());
            }
            e = add_up(mul_up(self.gains[i], e)?, r)?;
            node_upper.push(e);
            prev = v;
        }
        let error_upper = node_upper.iter().copied().fold(0., f64::max);
        Ok(TimeReport {
            values: x,
            node_upper,
            error_upper,
        })
    }
    pub fn solve(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        mode: TimeMode,
        exec: &ParallelExecution,
    ) -> CoreResult<TimeReport> {
        self.check(y, b)?;
        let x = match mode {
            TimeMode::Sequential => {
                let mut out = Vec::with_capacity(b.len());
                let mut prev = y.to_vec();
                for (g, b) in self.g.iter().zip(b) {
                    let mut v = g.matvec(&prev)?;
                    for (x, a) in v.iter_mut().zip(b) {
                        *x += a;
                    }
                    prev = v.clone();
                    out.push(v);
                }
                out
            }
            TimeMode::OrderedScan => {
                // Work-efficient blocked ordered scan. Parallelize local
                // products, compute block starts sequentially, then local
                // independent recurrences. No O(m log m) all-prefix copy.
                let chunk = b.len().div_ceil(exec.threads()).max(1);
                let ranges = (0..b.len())
                    .step_by(chunk)
                    .map(|i| (i, (i + chunk).min(b.len())))
                    .collect::<Vec<_>>();
                let blocks = exec.map_ordered(&ranges, |&(lo, hi)| {
                    let mut p = (DenseMatrix::identity(self.n), vec![0.; self.n]);
                    for (g, b) in self.g[lo..hi].iter().zip(&b[lo..hi]) {
                        p = compose(&p, g, b)?;
                    }
                    Ok(p)
                })?;
                let mut start = y.to_vec();
                let mut starts = Vec::new();
                for (g, b) in &blocks {
                    starts.push(start.clone());
                    let mut v = g.matvec(&start)?;
                    for (x, a) in v.iter_mut().zip(b) {
                        *x += a;
                    }
                    start = v;
                }
                let work = ranges.iter().zip(starts.iter()).collect::<Vec<_>>();
                exec.map_ordered(&work, |(range, start)| {
                    let (lo, hi) = **range;
                    let mut p = start.to_vec();
                    let mut v = Vec::new();
                    for (g, b) in self.g[lo..hi].iter().zip(&b[lo..hi]) {
                        let mut q = g.matvec(&p)?;
                        for (x, a) in q.iter_mut().zip(b) {
                            *x += a;
                        }
                        p = q.clone();
                        v.push(q);
                    }
                    Ok(v)
                })?
                .into_iter()
                .flatten()
                .collect()
            }
        };
        self.certify(y, b, x)
    }
    pub fn adaptive(
        &self,
        p: &mut Portfolio,
        y: &[f64],
        b: &[Vec<f64>],
        exec: &ParallelExecution,
        tol: f64,
    ) -> CoreResult<Selection<TimeReport>> {
        p.choose(
            "aperiodic",
            &[self.n as u64, self.g.len() as u64, exec.threads() as u64],
            &[Route::AperiodicSequential, Route::AperiodicScan],
            tol,
            |r| {
                let v = self.solve(
                    y,
                    b,
                    if r == Route::AperiodicSequential {
                        TimeMode::Sequential
                    } else {
                        TimeMode::OrderedScan
                    },
                    exec,
                )?;
                let e = v.error_upper;
                Ok((v, e))
            },
        )
    }
}
/// Table-backed polynomial-time action with full outward truncated word
/// evaluation. A_j are normalized-time generators on [0,1], not physical
/// time coefficients before scaling. Sum_j ||A_j||inf/(j+1)=M.
/// Noncommuting order is retained. Error is for the exact source linear IVP.
#[derive(Debug)]
pub struct PicardReport {
    pub values: Vec<f64>,
    pub error_upper: f64,
    pub tail_upper: f64,
    pub words: usize,
}
pub fn ordered_picard(a: &[DenseMatrix; 3], y: &[f64]) -> CoreResult<PicardReport> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let n = y.len();
    if n == 0 || n > 64 || y.iter().any(|v| !v.is_finite()) || a.iter().any(|m| !valid_matrix(m, n))
    {
        return Err(bad("Picard shape"));
    }
    let d = Dictionary::get()?;
    let m = a.iter().enumerate().try_fold(0., |s, (j, a)| {
        add_up(s, div_up(rownorm(a)?, (j + 1) as f64)?)
    })?;
    // Cap the majorant to avoid meaningless blowup or overflows.
    if m > 8. {
        return Err(bad("Picard majorant too large"));
    }
    let init = y.iter().map(|&x| pt(x)).collect::<CoreResult<Vec<_>>>()?;
    let mut total = init.clone();
    // Store suffix actions, indexed by base-three word of each length.
    // Each A_word y computed once rather than one matvec per prefix.
    let mut levels = vec![vec![init]];
    let mut k = 0;
    for len in 1..=5 {
        let prev = &levels[len - 1];
        let mut cur = Vec::with_capacity(3 * prev.len());
        for a in a {
            for tail in prev {
                cur.push(mat_iv(a, tail)?);
            }
        }
        for (v, word) in cur.iter().zip(&d.picard_words()[k..k + cur.len()]) {
            if word.word.len() != len {
                return Err(bad("Picard word order"));
            }
            for (x, &y) in total.iter_mut().zip(v) {
                *x = x.add(y.mul(word.coefficient.enclosure)?)?;
            }
        }
        k += cur.len();
        levels.push(cur);
    }
    let mut tail = infnorm(y);
    for j in 1..=6 {
        tail = div_up(mul_up(tail, m)?, j as f64)?;
    }
    tail = mul_up(tail, exp_interval(m)?.hi)?;
    let mut e = 0.0_f64;
    let mut values = Vec::new();
    for v in total {
        let x = 0.5 * v.lo + 0.5 * v.hi;
        let r = Iv::new(v.lo, v.hi)?.sub(pt(x)?)?.mag();
        e = e.max(add_up(r, tail)?);
        values.push(x);
    }
    Ok(PicardReport {
        values,
        error_upper: e,
        tail_upper: tail,
        words: k,
    })
}
/// Scalar/strictly triangular nilpotent Green carrier. The strict upper
/// triangular structure proves nilpotency, rather than testing a rounded
/// matrix power. Returns the ideal linear forcing response enclosure.
pub fn nilpotent_green(
    a: f64,
    e: &DenseMatrix,
    theta: f64,
    y: &[f64],
    forcing: &[Vec<f64>],
    mode: RealMode,
) -> CoreResult<Vec<Iv>> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let d = Dictionary::get()?;
    let basis = d.basis(forcing.len())?;
    let n = y.len();
    if n == 0
        || n > basis.damping_lift.len()
        || !valid_matrix(e, n)
        || !a.is_finite()
        || a < 0.
        || !theta.is_finite()
        || !(0. ..=1.).contains(&theta)
        || forcing.iter().any(|v| v.len() != n)
        || y.iter()
            .chain(forcing.iter().flatten())
            .any(|v| !v.is_finite())
    {
        return Err(bad("nilpotent carrier contract"));
    }
    for i in 0..n {
        for j in 0..=i {
            if e[(i, j)] != 0. {
                return Err(bad("strict upper triangular carrier required"));
            }
        }
    }
    let x = pt(a)?.scale(theta)?;
    let exp = d.phi(x, 0, mode)?;
    let mut value = vec![zero(); n];
    let mut v = y.iter().map(|&x| pt(x)).collect::<CoreResult<Vec<_>>>()?;
    let mut f = forcing
        .iter()
        .map(|x| x.iter().map(|&v| pt(v)).collect::<CoreResult<Vec<_>>>())
        .collect::<CoreResult<Vec<_>>>()?;
    let mut time = pt(1.)?;
    for l in 0..n {
        for (s, &v) in value.iter_mut().zip(&v) {
            *s = s.add(v.mul(time)?.mul(exp)?)?;
        }
        for (j, f) in f.iter().enumerate() {
            let w = basis.green_weight(d, j, a, theta, l, mode)?;
            for (s, &v) in value.iter_mut().zip(f) {
                *s = s.add(v.mul(w)?)?;
            }
        }
        if l + 1 < n {
            v = mat_iv(e, &v)?;
            for v in &mut f {
                *v = mat_iv(e, v)?;
            }
            time = time.scale(theta)?.div(pt((l + 1) as f64)?)?;
        }
    }
    Ok(value)
}

/// One monomial coefficient of a matrix polynomial, excluding the constant.
#[derive(Clone, Debug)]
pub struct PencilTerm {
    pub powers: Vec<usize>,
    pub operator: DenseMatrix,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuotientMode {
    DirectBlock,
    Folded,
}
#[derive(Debug)]
pub struct QuotientReport {
    pub values: Vec<Vec<f64>>,
    pub residual_upper: f64,
    pub error_upper: f64,
    pub iterations: usize,
}
struct ReducedTerm {
    weights: Vec<Iv>,
    operator: DenseMatrix,
}
/// Tensor quotient by independent monic polynomials; axis zero is fastest.
/// A conservative original-operator inverse bound is mandatory. General
/// nonseparable ideals and arbitrary invertible centers are not admitted.
pub struct TensorPencil {
    n: usize,
    channels: usize,
    center: DenseMatrix,
    terms: Vec<ReducedTerm>,
    lu: LuFactorization,
    scratch: LuSolveWorkspace,
    gap: f64,
    par: faer::Par,
}
impl TensorPencil {
    pub fn new(center: DenseMatrix, moduli: &[Vec<f64>], terms: &[PencilTerm]) -> CoreResult<Self> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = center.nrows();
        if n == 0
            || n > 64
            || moduli.is_empty()
            || moduli.len() > 3
            || terms.len() > 16
            || moduli
                .iter()
                .any(|p| p.is_empty() || p.len() > 8 || p.iter().any(|v| !v.is_finite()))
        {
            return Err(bad("tensor quotient dimensions"));
        }
        let channels = moduli.iter().map(Vec::len).product::<usize>();
        if channels > 64 || channels * n > 512 {
            return Err(bad("tensor quotient cap"));
        }
        let mut gap = enclosing_gap(&center)?;
        let mut reduced = Vec::new();
        for term in terms {
            if term.powers.len() != moduli.len()
                || term.powers.iter().all(|&p| p == 0)
                || term.powers.iter().any(|&p| p > 8)
                || !valid_matrix(&term.operator, n)
            {
                return Err(bad("tensor term shape"));
            }
            let mut w = vec![zero(); channels * channels];
            for col in 0..channels {
                let mut v = vec![zero(); channels];
                v[col] = pt(1.)?;
                let mut stride = 1;
                for (axis, p) in moduli.iter().enumerate() {
                    for _ in 0..term.powers[axis] {
                        let mut out = vec![zero(); channels];
                        for (k, &z) in v.iter().enumerate() {
                            let digit = (k / stride) % p.len();
                            if digit + 1 < p.len() {
                                out[k + stride] = out[k + stride].add(z)?;
                            } else {
                                let base = k - digit * stride;
                                for (j, &c) in p.iter().enumerate() {
                                    let i = base + j * stride;
                                    out[i] = out[i].add(z.scale(-c)?)?;
                                }
                            }
                        }
                        v = out;
                    }
                    stride *= p.len();
                }
                for (row, z) in v.into_iter().enumerate() {
                    w[row * channels + col] = z;
                }
            }
            let mut bound = 0.0_f64;
            for row in w.chunks(channels) {
                bound = bound.max(row.iter().try_fold(0., |s, v| add_up(s, v.mag()))?);
            }
            gap = sub_down(gap, mul_up(bound, rownorm(&term.operator)?)?)?;
            reduced.push(ReducedTerm {
                weights: w,
                operator: term.operator.clone(),
            });
        }
        if gap <= 0. || !gap.is_finite() {
            return Err(bad("quotient inverse gain not proved"));
        }
        let lu = LuFactorization::new(&center)?;
        let scratch = LuSolveWorkspace::new(n);
        Ok(Self {
            n,
            channels,
            center,
            terms: reduced,
            lu,
            scratch,
            gap,
            par: faer::get_global_parallelism(),
        })
    }
    pub fn channels(&self) -> usize {
        self.channels
    }
    fn check(&self, b: &[Vec<f64>]) -> CoreResult<()> {
        if b.len() != self.channels
            || b.iter()
                .any(|r| r.len() != self.n || r.iter().any(|v| !v.is_finite()))
        {
            Err(bad("quotient coefficient shape"))
        } else {
            Ok(())
        }
    }
    /// Independent outward residual using the represented moduli and all terms.
    pub fn certify(
        &self,
        b: &[Vec<f64>],
        x: Vec<Vec<f64>>,
        iterations: usize,
    ) -> CoreResult<QuotientReport> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        self.check(b)?;
        self.check(&x)?;
        let xi = x
            .iter()
            .map(|v| v.iter().map(|&a| pt(a)).collect::<CoreResult<Vec<_>>>())
            .collect::<CoreResult<Vec<_>>>()?;
        let mut r = xi
            .iter()
            .zip(b)
            .map(|(v, b)| {
                mat_iv(&self.center, v)?
                    .into_iter()
                    .zip(b)
                    .map(|(a, &b)| pt(b)?.sub(a))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        for t in &self.terms {
            let dx = xi
                .iter()
                .map(|v| mat_iv(&t.operator, v))
                .collect::<CoreResult<Vec<_>>>()?;
            for (row, r) in r.iter_mut().enumerate() {
                for (col, d) in dx.iter().enumerate() {
                    let w = t.weights[row * self.channels + col];
                    if w.mag() != 0. {
                        for (r, &z) in r.iter_mut().zip(d) {
                            *r = r.sub(w.mul(z)?)?;
                        }
                    }
                }
            }
        }
        let residual_upper = r.iter().flatten().fold(0.0_f64, |s, v| s.max(v.mag()));
        Ok(QuotientReport {
            values: x,
            residual_upper,
            error_upper: div_up(residual_upper, self.gap)?,
            iterations,
        })
    }
    pub fn solve(
        &mut self,
        b: &[Vec<f64>],
        mode: QuotientMode,
        tol: f64,
        max_iters: usize,
    ) -> CoreResult<QuotientReport> {
        self.check(b)?;
        if !tol.is_finite()
            || tol <= 0.
            || max_iters == 0
            || max_iters > 128
            || self.par != faer::get_global_parallelism()
        {
            return Err(bad("quotient solve policy"));
        }
        let mid = |v: Iv| 0.5 * v.lo + 0.5 * v.hi;
        if mode == QuotientMode::DirectBlock {
            let dim = self.n * self.channels;
            let mut a = DenseMatrix::zeros(dim, dim);
            for i in 0..self.channels {
                for r in 0..self.n {
                    for c in 0..self.n {
                        a[(i * self.n + r, i * self.n + c)] = self.center[(r, c)];
                    }
                }
            }
            for t in &self.terms {
                for i in 0..self.channels {
                    for j in 0..self.channels {
                        let w = mid(t.weights[i * self.channels + j]);
                        if w != 0. {
                            for r in 0..self.n {
                                for c in 0..self.n {
                                    a[(i * self.n + r, j * self.n + c)] += w * t.operator[(r, c)];
                                }
                            }
                        }
                    }
                }
            }
            let flat = b.iter().flatten().copied().collect::<Vec<_>>();
            let x = direct_solve(&a, &flat)?;
            return self.certify(b, x.chunks(self.n).map(<[f64]>::to_vec).collect(), 1);
        }
        let mut x = vec![vec![0.; self.n]; self.channels];
        for k in 1..=max_iters {
            let mut next = b.to_vec();
            for t in &self.terms {
                let dx = x
                    .iter()
                    .map(|v| t.operator.matvec(v))
                    .collect::<CoreResult<Vec<_>>>()?;
                for (row, v) in next.iter_mut().enumerate() {
                    for (col, d) in dx.iter().enumerate() {
                        let w = mid(t.weights[row * self.channels + col]);
                        if w != 0. {
                            for (v, &d) in v.iter_mut().zip(d) {
                                *v -= w * d;
                            }
                        }
                    }
                }
            }
            for v in &mut next {
                self.lu
                    .solve_in_place_with_workspace(v, &mut self.scratch)?;
            }
            let report = self.certify(b, next, k)?;
            if report.error_upper <= tol || k == max_iters {
                return Ok(report);
            }
            x = report.values;
        }
        unreachable!()
    }
    pub fn adaptive(
        &mut self,
        p: &mut Portfolio,
        b: &[Vec<f64>],
        tol: f64,
        max_iters: usize,
    ) -> CoreResult<Selection<QuotientReport>> {
        p.choose(
            "tensor-quotient",
            &[
                self.n as u64,
                self.channels as u64,
                self.terms.len() as u64,
                self.gap.to_bits(),
                max_iters as u64,
            ],
            &[Route::QuotientFolded, Route::QuotientDirect],
            tol,
            |r| {
                let v = self.solve(
                    b,
                    if r == Route::QuotientFolded {
                        QuotientMode::Folded
                    } else {
                        QuotientMode::DirectBlock
                    },
                    tol,
                    max_iters,
                )?;
                let e = v.error_upper;
                Ok((v, e))
            },
        )
    }
}
/// Full table evaluation into caller-owned output slots. Independent points
/// may use a bounded Rayon pool; the coefficient lane layout stays unchanged.
pub fn phi_batch_into(x: &[f64], out: &mut [[Iv; 9]], exec: &ParallelExecution) -> CoreResult<()> {
    dictionary_batch_into(x, out, exec, |d, x| d.phi_all_fast(x))
}
pub fn moment_batch_into(
    x: &[f64],
    out: &mut [[Iv; 36]],
    exec: &ParallelExecution,
) -> CoreResult<()> {
    dictionary_batch_into(x, out, exec, |d, x| d.moments_all_fast(x))
}
fn dictionary_batch_into<const N: usize>(
    x: &[f64],
    out: &mut [[Iv; N]],
    exec: &ParallelExecution,
    eval: impl Fn(&Dictionary, f64) -> CoreResult<[Iv; N]> + Send + Sync,
) -> CoreResult<()> {
    if x.len() != out.len() {
        return Err(bad("dictionary batch shape"));
    }
    if x.is_empty() {
        return Ok(());
    }
    let d = Dictionary::get()?;
    let size = x.len().div_ceil(exec.threads().saturating_mul(4)).max(1);
    let inputs = x.chunks(size).collect::<Vec<_>>();
    let mut outputs = out.chunks_mut(size).collect::<Vec<_>>();
    exec.try_for_each_ordered_mut(&inputs, &mut outputs, |xs, ys| {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        for (&x, y) in xs.iter().zip(ys.iter_mut()) {
            *y = eval(d, x)?;
        }
        Ok(())
    })
}
