//! Research-only all-at-once temporal recurrence with the original initial
//! condition. Source-owned real binary64 affine maps repeat an ordered
//! period. No continuous-IVP, event or arbitrary RHS claim.
//!
//! For the period transfer T and m=2^L periods, H(z)=sum_{j<m} z^j T^j
//! =prod_{ell<L}(I+z^(2^ell) T^(2^ell)). The cyclic and negacyclic
//! products modulo z^m +/- alpha reconstruct the low band exactly.
//! H is the *return-factor-absorbed* closure inverse numerator: it
//! requires neither I-alpha*T^m nor any modal I-z*T to be invertible.
//! Candidate rounding (including FFT, powers and period aggregation) is
//! not trusted. Directed residuals of the original ungrouped recurrence
//! and interval ordered prefixes bound the discrete trajectory error.
use crate::{
    ParallelExecution, affine_event_certificate::check_affine_event_arithmetic,
    fourier_path_candidate::warp::Plan as FftPlan, fourier_path_certificate::Cplx,
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix,
    directed::{Interval, add_down, add_up, div_up, mul_down, mul_up, sub_down},
};
use std::f64::consts::PI;

const MAX_CYCLES: usize = 4096;
const MAX_PERIOD: usize = 16;
const MAX_N: usize = 64;
const MAX_STATE_VALUES: usize = 1 << 20;
type Vectors = Vec<Vec<f64>>;
type Iv = Interval;

fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro temporal: {s}"))
}
fn finite(v: &[f64]) -> CoreResult<()> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite(
            "retro temporal candidate or input".into(),
        ))
    }
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn norm(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |m, x| m.max(x.abs()))
}
fn cadd(a: Cplx, b: Cplx) -> Cplx {
    Cplx::new(a.re + b.re, a.im + b.im)
}
fn cmul(a: Cplx, b: Cplx) -> Cplx {
    Cplx::new(a.re * b.re - a.im * b.im, a.re * b.im + a.im * b.re)
}
fn conj(a: Cplx) -> Cplx {
    Cplx::new(a.re, -a.im)
}
fn cscale(a: Cplx, r: f64) -> Cplx {
    Cplx::new(a.re * r, a.im * r)
}
fn cfinite(v: &[Cplx]) -> CoreResult<()> {
    if v.iter().all(|z| z.re.is_finite() && z.im.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite("retro temporal FFT".into()))
    }
}
// Universal rational phase angle, no accumulated phase recurrence. Exact
// cardinal values avoid spurious imaginary channels at self partners.
fn phase(num: i64, den: usize) -> Cplx {
    let d = den as i64;
    let k = num.rem_euclid(2 * d);
    if k == 0 {
        Cplx::new(1., 0.)
    } else if k == d {
        Cplx::new(-1., 0.)
    } else if 2 * k == d {
        Cplx::new(0., 1.)
    } else if 2 * k == 3 * d {
        Cplx::new(0., -1.)
    } else {
        let (s, c) = (PI * k as f64 / d as f64).sin_cos();
        Cplx::new(c, s)
    }
}
struct Sector {
    len: usize,
    fft: FftPlan,
    twist: Vec<Cplx>,
    untwist: Vec<Cplx>,
    reps: Vec<usize>,
    partner: Vec<usize>,
    factors: Vec<Vec<Cplx>>,
}
impl Sector {
    fn new(len: usize, levels: usize, alpha: f64, negative: bool) -> Self {
        let rho = alpha.powf(1. / len as f64);
        let shift = usize::from(negative);
        let mut twist = Vec::with_capacity(len);
        let mut untwist = Vec::with_capacity(len);
        for j in 0..len {
            let a = rho.powi(j as i32);
            let z = phase((shift * j) as i64, len);
            twist.push(cscale(z, a));
            untwist.push(cscale(conj(z), 1. / a));
        }
        let partner: Vec<_> = (0..len).map(|k| (len + shift - k) % len).collect();
        let reps: Vec<_> = (0..len).filter(|&k| k <= partner[k]).collect();
        let factors = (0..len)
            .map(|k| {
                (0..levels)
                    .map(|j| {
                        let d = 1usize << j;
                        cscale(
                            phase((shift as i64 - 2 * k as i64) * d as i64, len),
                            rho.powi(d as i32),
                        )
                    })
                    .collect()
            })
            .collect();
        Self {
            len,
            fft: FftPlan::new(len),
            twist,
            untwist,
            reps,
            partner,
            factors,
        }
    }
}
/// Problem-independent temporal coefficient dictionary. The table contains
/// the dyadic factor phases, involution representatives, twist/untwist and
/// the existing PP06 radix-2 plan. Prepare once and share across operators.
/// It is not an offline persisted table or an accuracy certificate.
pub struct TemporalTable {
    cycles: usize,
    levels: usize,
    plus: Sector,
    minus: Sector,
    padded: Sector,
}
impl TemporalTable {
    pub fn new(cycles: usize, alpha: f64) -> CoreResult<Self> {
        if !(2..=MAX_CYCLES).contains(&cycles)
            || !cycles.is_power_of_two()
            || !alpha.is_finite()
            || !(1. / 4096. ..=1.).contains(&alpha)
        {
            return Err(bad(
                "cycles must be a power of two in [2,4096], alpha in [2^-12,1]",
            ));
        }
        let levels = cycles.trailing_zeros() as usize;
        Ok(Self {
            cycles,
            levels,
            plus: Sector::new(cycles, levels, alpha, false),
            minus: Sector::new(cycles, levels, alpha, true),
            padded: Sector::new(2 * cycles, levels, 1., false),
        })
    }
    pub fn cycles(&self) -> usize {
        self.cycles
    }
    pub fn dyadic_levels(&self) -> usize {
        self.levels
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Mode {
    Sequential,
    Scan,
    Padded,
    Crt,
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Work {
    /// Candidate real dense matrix-vector actions (complex counts as two).
    pub matvecs: usize,
    pub forward_ffts: usize,
    pub inverse_ffts: usize,
    pub fft_points: usize,
    pub modal_representatives: usize,
    pub scan_chunks: usize,
    /// Original recurrence residual rows certified.
    pub residual_rows: usize,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Certificate {
    pub accepted: bool,
    pub residual_upper: f64,
    pub error_upper: f64,
    pub endpoint_upper: f64,
    pub node_error_upper: Vec<f64>,
    pub endpoint_component_upper: Vec<f64>,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub values: Vectors,
    pub error_upper: f64,
    pub accepted: bool,
    pub certificate: Certificate,
    pub work: Work,
}

fn valid_pattern(pattern: &[DenseMatrix]) -> CoreResult<usize> {
    if pattern.is_empty() || pattern.len() > MAX_PERIOD {
        return Err(bad("period size"));
    }
    let n = pattern[0].nrows();
    if n == 0 || n > MAX_N || pattern.iter().any(|g| g.nrows() != n || g.ncols() != n) {
        return Err(bad("matrix shape or dimension"));
    }
    for g in pattern {
        finite(g.as_slice())?;
    }
    Ok(n)
}
fn valid_data(n: usize, period: usize, cycles: usize, y: &[f64], b: &[Vec<f64>]) -> CoreResult<()> {
    if y.len() != n
        || b.len() != period * cycles
        || b.len().checked_mul(n).is_none_or(|v| v > MAX_STATE_VALUES)
        || b.iter().any(|x| x.len() != n)
    {
        return Err(bad("initial/RHS shape or state cap"));
    }
    finite(y)?;
    for x in b {
        finite(x)?;
    }
    Ok(())
}
fn step(g: &DenseMatrix, y: &[f64], b: &[f64], out: &mut [f64]) -> CoreResult<()> {
    g.matvec_into(y, out)?;
    for (x, &v) in out.iter_mut().zip(b) {
        *x += v;
    }
    finite(out)
}
fn sweep(pattern: &[DenseMatrix], y: &[f64], b: &[Vec<f64>]) -> CoreResult<Vectors> {
    let n = y.len();
    let mut out = Vec::with_capacity(b.len());
    for (i, rhs) in b.iter().enumerate() {
        let mut x = vec![0.; n];
        step(
            &pattern[i % pattern.len()],
            out.last().map_or(y, Vec::as_slice),
            rhs,
            &mut x,
        )?;
        out.push(x);
    }
    Ok(out)
}
fn end_only(pattern: &[DenseMatrix], y: &[f64], b: &[Vec<f64>]) -> CoreResult<Vec<f64>> {
    let mut x = y.to_vec();
    let mut next = vec![0.; y.len()];
    for (i, rhs) in b.iter().enumerate() {
        step(&pattern[i % pattern.len()], &x, rhs, &mut next)?;
        std::mem::swap(&mut x, &mut next);
    }
    Ok(x)
}

/// Independent proof data for the exact represented recurrence. Ordered
/// prefix products P_j=G_j ... G_0 are enclosed directly, never assembled
/// from rounded candidate T or T^k. This preserves signed cancellation
/// across a period, unlike multiplying only absolute step matrices.
struct Geometry<'a> {
    pattern: &'a [DenseMatrix],
    n: usize,
    prefix_abs: Vec<DenseMatrix>,
    row_sums: Vec<Vec<f64>>,
    rounding: RowRounding,
}
/// Same exact-binary FMA model as warp_cubic: relative u=2^-53 and
/// conservative additive eta=MIN_POSITIVE. The rounded row is finite.
#[derive(Clone, Copy)]
struct RowRounding {
    gamma: f64,
    floor: f64,
    additive: f64,
    denom: f64,
}
impl RowRounding {
    fn new(n: usize) -> CoreResult<Self> {
        let k = (n + 1) as f64;
        let ku = k * (f64::EPSILON * 0.5);
        let denom = sub_down(1., ku)?;
        let floor = mul_up(k, f64::MIN_POSITIVE)?;
        Ok(Self {
            gamma: div_up(ku, denom)?,
            floor,
            additive: div_up(floor, denom)?,
            denom,
        })
    }
    fn residual(
        self,
        g: &[f64],
        prev: &[f64],
        b: f64,
        x: f64,
        row_sum: f64,
        prev_norm: f64,
    ) -> CoreResult<f64> {
        let mut r = b - x;
        for (&a, &z) in g.iter().zip(prev) {
            r = a.mul_add(z, r);
        }
        let mag = add_up(add_up(b.abs(), x.abs())?, mul_up(row_sum, prev_norm)?)?;
        add_up(r.abs(), add_up(mul_up(self.gamma, mag)?, self.additive)?)
    }
    // Nonnegative represented terms: computed s >= (1-ku)*true - k*eta.
    // One conservative inflation per row, not one per multiplication.
    fn positive(self, g: &[f64], v: &[f64], start: f64) -> CoreResult<f64> {
        let mut s = start;
        for (&a, &e) in g.iter().zip(v) {
            s = a.abs().mul_add(e, s);
        }
        div_up(add_up(s, self.floor)?, self.denom)
    }
}
impl<'a> Geometry<'a> {
    fn new(pattern: &'a [DenseMatrix]) -> CoreResult<Self> {
        check_affine_event_arithmetic()?;
        let n = valid_pattern(pattern)?;
        let zero = pt(0.)?;
        let mut prev = vec![zero; n * n];
        for i in 0..n {
            prev[i * n + i] = pt(1.)?;
        }
        let mut prefix_abs = Vec::with_capacity(pattern.len());
        for (j, g) in pattern.iter().enumerate() {
            let mut next = vec![zero; n * n];
            if j == 0 {
                for (x, &a) in next.iter_mut().zip(g.as_slice()) {
                    *x = pt(a)?;
                }
            } else {
                for i in 0..n {
                    for k in 0..n {
                        let mut s = zero;
                        for l in 0..n {
                            s = s.add(prev[l * n + k].scale(g[(i, l)])?)?;
                        }
                        next[i * n + k] = s;
                    }
                }
            }
            prefix_abs.push(DenseMatrix::new(n, n, next.iter().map(Iv::mag).collect())?);
            prev = next;
        }
        let row_sums = pattern
            .iter()
            .map(|g| {
                (0..n)
                    .map(|i| g.row(i).iter().try_fold(0., |s, a| add_up(s, a.abs())))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        Ok(Self {
            pattern,
            n,
            prefix_abs,
            row_sums,
            rounding: RowRounding::new(n)?,
        })
    }
    fn verify(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        x: &[Vec<f64>],
        tol: f64,
        directed: bool,
    ) -> CoreResult<Certificate> {
        check_affine_event_arithmetic()?;
        let n = self.n;
        let p = self.pattern.len();
        if !tol.is_finite()
            || tol <= 0.
            || x.len() != b.len()
            || x.is_empty()
            || x.iter().any(|v| v.len() != n)
        {
            return Err(bad("certificate inputs"));
        }
        let mut carry = vec![0.; n];
        let mut local = vec![0.; n];
        let mut bound = vec![0.; n];
        let mut max_res = 0.0_f64;
        let mut max_err = 0.0_f64;
        let mut nodes = Vec::with_capacity(x.len());
        for i in 0..x.len() {
            finite(&x[i])?;
            let j = i % p;
            let g = &self.pattern[j];
            if j == 0 {
                local.fill(0.);
            }
            let prev = if i == 0 { y } else { &x[i - 1] };
            let mut next = vec![0.; n];
            let prev_norm = norm(prev);
            for a in 0..n {
                // The exact represented residual, not the residual of a
                // rounded group transfer, nor a cyclic-system residual.
                let r = if directed {
                    let mut lo = add_down(b[i][a], -x[i][a])?;
                    let mut hi = add_up(b[i][a], -x[i][a])?;
                    for (&ga, &z) in g.row(a).iter().zip(prev) {
                        lo = add_down(lo, mul_down(ga, z)?)?;
                        hi = add_up(hi, mul_up(ga, z)?)?;
                    }
                    lo.abs().max(hi.abs())
                } else {
                    self.rounding.residual(
                        g.row(a),
                        prev,
                        b[i][a],
                        x[i][a],
                        self.row_sums[j][a],
                        prev_norm,
                    )?
                };
                max_res = max_res.max(r);
                let mut l = r;
                if j != 0 {
                    if directed {
                        for (&ga, &e) in g.row(a).iter().zip(&local) {
                            l = add_up(l, mul_up(ga.abs(), e)?)?;
                        }
                    } else {
                        l = self.rounding.positive(g.row(a), &local, l)?;
                    }
                }
                next[a] = l;
                let total = if directed {
                    let mut total = l;
                    for (&ga, &e) in self.prefix_abs[j].row(a).iter().zip(&carry) {
                        total = add_up(total, mul_up(ga, e)?)?;
                    }
                    total
                } else {
                    self.rounding
                        .positive(self.prefix_abs[j].row(a), &carry, l)?
                };
                bound[a] = total;
            }
            local = next;
            let u = norm(&bound);
            nodes.push(u);
            max_err = max_err.max(u);
            if j + 1 == p {
                carry.copy_from_slice(&bound);
            }
        }
        Ok(Certificate {
            accepted: max_err <= tol,
            residual_upper: max_res,
            error_upper: max_err,
            endpoint_upper: *nodes.last().unwrap(),
            node_error_upper: nodes,
            endpoint_component_upper: bound,
        })
    }
}

pub struct Prepared<'a> {
    table: &'a TemporalTable,
    geo: Geometry<'a>,
    transfer: DenseMatrix,
    powers: Vec<DenseMatrix>,
}
impl<'a> Prepared<'a> {
    pub fn new(table: &'a TemporalTable, pattern: &'a [DenseMatrix]) -> CoreResult<Self> {
        let geo = Geometry::new(pattern)?;
        if table.cycles * pattern.len() * geo.n > MAX_STATE_VALUES {
            return Err(bad("state cap"));
        }
        let mut transfer = pattern[0].clone();
        for g in &pattern[1..] {
            transfer = g.matmul(&transfer)?;
            finite(transfer.as_slice())?;
        }
        let mut powers = vec![transfer.clone()];
        // Include T^m for a single-chunk scan and for its prepared bound;
        // CRT needs only T^(m/2). Nonfinite power fails closed.
        for _ in 0..table.levels {
            let last = powers.last().unwrap();
            let next = last.matmul(last)?;
            finite(next.as_slice())?;
            powers.push(next);
        }
        Ok(Self {
            table,
            geo,
            transfer,
            powers,
        })
    }
    pub fn transfer(&self) -> &DenseMatrix {
        &self.transfer
    }
    pub fn prepared_matrix_products(&self) -> usize {
        self.geo.pattern.len() - 1 + self.table.levels
    }
    pub fn certify(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        x: &[Vec<f64>],
        tol: f64,
    ) -> CoreResult<Certificate> {
        valid_data(self.geo.n, self.geo.pattern.len(), self.table.cycles, y, b)?;
        self.geo.verify(y, b, x, tol, false)
    }
    /// Slower term-by-term directed reference for auditing the row-FMA
    /// certificate. Both use the same original recurrence and prefix bounds.
    pub fn certify_directed(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        x: &[Vec<f64>],
        tol: f64,
    ) -> CoreResult<Certificate> {
        valid_data(self.geo.n, self.geo.pattern.len(), self.table.cycles, y, b)?;
        self.geo.verify(y, b, x, tol, true)
    }
    /// Candidate alone for diagnostic cost separation. Do not treat this as
    /// accepted; use certify() or solve() before any downstream commitment.
    pub fn candidate(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        mode: Mode,
        exec: &ParallelExecution,
    ) -> CoreResult<(Vectors, Work)> {
        let n = self.geo.n;
        let p = self.geo.pattern.len();
        let m = self.table.cycles;
        valid_data(n, p, m, y, b)?;
        let mut w = Work::default();
        let x = match mode {
            Mode::Sequential => {
                w.matvecs = b.len();
                sweep(self.geo.pattern, y, b)?
            }
            Mode::Scan => self.scan(y, b, exec, &mut w)?,
            Mode::Padded | Mode::Crt => self.spectral(y, b, mode, exec, &mut w)?,
        };
        for v in &x {
            finite(v)?;
        }
        Ok((x, w))
    }
    pub fn solve(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        mode: Mode,
        exec: &ParallelExecution,
        tol: f64,
    ) -> CoreResult<Report> {
        let (values, mut work) = self.candidate(y, b, mode, exec)?;
        let certificate = self.certify(y, b, &values, tol)?;
        work.residual_rows = b.len() * self.geo.n;
        Ok(Report {
            error_upper: certificate.error_upper,
            accepted: certificate.accepted,
            values,
            certificate,
            work,
        })
    }
    fn scan(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        exec: &ParallelExecution,
        w: &mut Work,
    ) -> CoreResult<Vectors> {
        let m = self.table.cycles;
        let p = self.geo.pattern.len();
        let chunks = exec.threads().min(m).next_power_of_two();
        let cycles = m / chunks;
        let size = cycles * p;
        w.scan_chunks = chunks;
        if chunks == 1 {
            w.matvecs = b.len();
            return sweep(self.geo.pattern, y, b);
        }
        let indices: Vec<_> = (0..chunks).collect();
        let zero = vec![0.; self.geo.n];
        let q = exec.map_ordered(&indices, |&i| {
            end_only(self.geo.pattern, &zero, &b[i * size..(i + 1) * size])
        })?;
        let a = &self.powers[cycles.trailing_zeros() as usize];
        let mut starts = Vec::with_capacity(chunks);
        let mut cur = y.to_vec();
        for rhs in &q[..chunks - 1] {
            starts.push(cur.clone());
            let mut x = vec![0.; cur.len()];
            step(a, &cur, rhs, &mut x)?;
            cur = x;
        }
        starts.push(cur);
        let parts = exec.map_ordered(&indices, |&i| {
            sweep(self.geo.pattern, &starts[i], &b[i * size..(i + 1) * size])
        })?;
        w.matvecs = 2 * b.len() + chunks - 1;
        Ok(parts.into_iter().flatten().collect())
    }
    fn spectral(
        &self,
        y: &[f64],
        b: &[Vec<f64>],
        mode: Mode,
        exec: &ParallelExecution,
        w: &mut Work,
    ) -> CoreResult<Vectors> {
        let m = self.table.cycles;
        let n = self.geo.n;
        let p = self.geo.pattern.len();
        let zero = vec![0.; n];
        let indices: Vec<_> = (0..m).collect();
        let q = if p == 1 {
            let mut q = b.to_vec();
            let mut first = vec![0.; n];
            step(&self.geo.pattern[0], y, &b[0], &mut first)?;
            q[0] = first;
            w.matvecs += 1;
            q
        } else {
            w.matvecs += b.len();
            exec.map_ordered(&indices, |&i| {
                end_only(
                    self.geo.pattern,
                    if i == 0 { y } else { &zero },
                    &b[i * p..(i + 1) * p],
                )
            })?
        };
        let ends = if mode == Mode::Padded {
            self.sector(&self.table.padded, &q, exec, w)?
        } else {
            let plus = self.sector(&self.table.plus, &q, exec, w)?;
            let minus = self.sector(&self.table.minus, &q, exec, w)?;
            plus.into_iter()
                .zip(minus)
                .map(|(a, b)| a.iter().zip(b).map(|(&x, y)| 0.5 * x + 0.5 * y).collect())
                .collect()
        };
        if p == 1 {
            return Ok(ends);
        }
        // Restore every original time node, not just period endpoints.
        // The final node of each period is the spectral candidate, not a
        // hidden replacement by the exact causal recurrence.
        let parts = exec.map_ordered(&indices, |&i| {
            let mut part = sweep(
                self.geo.pattern,
                if i == 0 { y } else { &ends[i - 1] },
                &b[i * p..i * p + p - 1],
            )?;
            part.push(ends[i].clone());
            Ok(part)
        })?;
        w.matvecs += m * (p - 1);
        Ok(parts.into_iter().flatten().collect())
    }
    fn sector(
        &self,
        s: &Sector,
        b: &[Vec<f64>],
        exec: &ParallelExecution,
        w: &mut Work,
    ) -> CoreResult<Vectors> {
        let n = self.geo.n;
        let m = self.table.cycles;
        let len = s.len;
        // FFT across time, separately for each state component. Same
        // immutable plans and phase table reused for every RHS packet.
        let components: Vec<_> = (0..n).collect();
        let transformed = exec.map_ordered(&components, |&i| {
            let mut v = vec![Cplx::ZERO; len];
            for (j, rhs) in b.iter().enumerate() {
                v[j] = cscale(s.twist[j], rhs[i]);
            }
            s.fft.apply(&mut v, false);
            cfinite(&v)?;
            Ok(v)
        })?;
        let modes = exec.map_ordered(&s.reps, |&k| {
            let mut x: Vec<_> = transformed.iter().map(|v| v[k]).collect();
            if k == s.partner[k] {
                for v in &mut x {
                    v.im = 0.;
                }
            }
            let mut tmp = vec![Cplx::ZERO; n];
            for (g, &z) in self.powers.iter().zip(&s.factors[k]) {
                for (i, dst) in tmp.iter_mut().enumerate() {
                    let mut gx = Cplx::ZERO;
                    for (&a, &v) in g.row(i).iter().zip(&x) {
                        gx = cadd(gx, cscale(v, a));
                    }
                    *dst = cadd(x[i], cmul(z, gx));
                }
                std::mem::swap(&mut x, &mut tmp);
            }
            cfinite(&x)?;
            Ok(x)
        })?;
        let mut data = vec![vec![Cplx::ZERO; len]; n];
        for (&k, x) in s.reps.iter().zip(&modes) {
            let l = s.partner[k];
            for (i, &z) in x.iter().enumerate() {
                let z = if k == l { Cplx::new(z.re, 0.) } else { z };
                data[i][k] = z;
                data[i][l] = conj(z);
            }
        }
        let restored = exec.map_ordered(&data, |row| {
            let mut v = row.clone();
            s.fft.apply(&mut v, true);
            cfinite(&v)?;
            Ok(v[..m]
                .iter()
                .zip(&s.untwist)
                .map(|(&z, &a)| cmul(z, a).re)
                .collect::<Vec<_>>())
        })?;
        w.forward_ffts += n;
        w.inverse_ffts += n;
        w.fft_points += 2 * n * len;
        w.modal_representatives += s.reps.len();
        w.matvecs += 2 * self.table.levels * s.reps.len();
        Ok((0..m)
            .map(|j| restored.iter().map(|x| x[j]).collect())
            .collect())
    }
}

/// Fresh sequential comparator without constructing an FFT dictionary or
/// rounded transfer powers. Identical original-equation certificate.
pub fn solve_sequential(
    pattern: &[DenseMatrix],
    y: &[f64],
    b: &[Vec<f64>],
    tol: f64,
) -> CoreResult<Report> {
    let geo = Geometry::new(pattern)?;
    if b.is_empty()
        || !b.len().is_multiple_of(pattern.len())
        || b.len() / pattern.len() > MAX_CYCLES
    {
        return Err(bad("invalid sequential span"));
    }
    valid_data(geo.n, pattern.len(), b.len() / pattern.len(), y, b)?;
    let values = sweep(pattern, y, b)?;
    let certificate = geo.verify(y, b, &values, tol, false)?;
    Ok(Report {
        error_upper: certificate.error_upper,
        accepted: certificate.accepted,
        values,
        certificate,
        work: Work {
            matvecs: b.len(),
            residual_rows: b.len() * geo.n,
            ..Work::default()
        },
    })
}
