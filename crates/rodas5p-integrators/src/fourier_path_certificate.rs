//! Native port of the Loop 10 Fourier-Volterra client (RVJ DAG node PP05,
//! research node `research/pp05_fourier_client_20261004`).
//!
//! Source: `inputs/loop10/code/fourier_volterra.py` of the Loop 11 ZIP
//! (`rhs`, `certificate`, `commit`, `build`, `physical`, `phase_candidate`,
//! `EP`) and loop 9 `rotate_interval`. Two coupled complex amplitudes
//! `(a, b)` on one step, normalized time `tau` in [0, 1]:
//! `a' = h i q c conj(a) b`, `b' = h i (q/2) c a^2`,
//! `c = g + lambda eps Re(phase a)`, `q = sigma 5/4` on the invariant leaf
//! `|a|^2 + 2|b|^2 = 9/16` (the archived scope). A path is
//! `sum c[k,j] tau^j e^{i k omega h tau}`.
//!
//! Candidates (binary64 coefficients) come from an untrusted Picard
//! predictor. Acceptance is only [`certificate`]: it recomputes the
//! `lambda = 1` right side by the noncyclic product in directed interval
//! arithmetic, builds the phase witness itself from `(omega, t, h)` (it takes
//! no caller phase) and charges any start mismatch. Two deviations from the
//! source are registered in the node: the charged start mismatch (binary64
//! cannot represent the source's exact start) and the internal phase witness
//! (finding A-PORT-03). This is not the existing quadratic
//! `Q2CertificateSource` and does not use it.

// `!(x <= bound)` is deliberate: a NaN must fail every acceptance test.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use std::collections::BTreeMap;

use rodas5p_core::directed::{Interval, add_up, div_up, mul_up, sqrt_up, sub_down, sub_up};
use rodas5p_core::{CoreError, CoreResult};

fn invalid(message: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("Fourier client: {message}"))
}

/// A binary64 complex point (candidate coefficient).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cplx {
    pub re: f64,
    pub im: f64,
}

impl Cplx {
    pub const ZERO: Cplx = Cplx { re: 0.0, im: 0.0 };
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    fn add(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }
    fn sub(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }
    fn mul(self, o: Self) -> Self {
        Self::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    fn scale(self, s: f64) -> Self {
        Self::new(self.re * s, self.im * s)
    }
    fn div(self, o: Self) -> Self {
        let d = o.re * o.re + o.im * o.im;
        Self::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    fn is_zero(self) -> bool {
        self.re == 0.0 && self.im == 0.0
    }
}

/// A complex interval: rectangle `re x i im`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CInterval {
    pub re: Interval,
    pub im: Interval,
}

impl CInterval {
    fn zero() -> Self {
        Self {
            re: Interval { lo: 0.0, hi: 0.0 },
            im: Interval { lo: 0.0, hi: 0.0 },
        }
    }
    fn point(z: Cplx) -> CoreResult<Self> {
        Ok(Self {
            re: Interval::point(z.re)?,
            im: Interval::point(z.im)?,
        })
    }
    fn real(x: Interval) -> Self {
        Self {
            re: x,
            im: Interval { lo: 0.0, hi: 0.0 },
        }
    }
    fn add(self, o: Self) -> CoreResult<Self> {
        Ok(Self {
            re: self.re.add(o.re)?,
            im: self.im.add(o.im)?,
        })
    }
    fn sub(self, o: Self) -> CoreResult<Self> {
        Ok(Self {
            re: self.re.sub(o.re)?,
            im: self.im.sub(o.im)?,
        })
    }
    fn mul(self, o: Self) -> CoreResult<Self> {
        Ok(Self {
            re: self.re.mul(o.re)?.sub(self.im.mul(o.im)?)?,
            im: self.re.mul(o.im)?.add(self.im.mul(o.re)?)?,
        })
    }
    fn mul_real(self, x: Interval) -> CoreResult<Self> {
        Ok(Self {
            re: self.re.mul(x)?,
            im: self.im.mul(x)?,
        })
    }
    /// `i x self` for a real interval `x`.
    fn mul_i_real(self, x: Interval) -> CoreResult<Self> {
        Ok(Self {
            re: -(self.im.mul(x)?),
            im: self.re.mul(x)?,
        })
    }
    fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }
    /// Upper bound of `|re| + |im|` over the rectangle.
    fn mag1(self) -> CoreResult<f64> {
        add_up(self.re.mag(), self.im.mag())
    }
    fn is_zero(self) -> bool {
        self.re.lo == 0.0 && self.re.hi == 0.0 && self.im.lo == 0.0 && self.im.hi == 0.0
    }
}

/// Candidate path: binary64 coefficients `c[k, j]`, harmonic `k`, degree `j`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FourierPath {
    coeffs: BTreeMap<(i32, u32), Cplx>,
}

impl FourierPath {
    pub fn constant(z: Cplx) -> Self {
        let mut coeffs = BTreeMap::new();
        if !z.is_zero() {
            coeffs.insert((0, 0), z);
        }
        Self { coeffs }
    }
    pub fn from_coefficients(entries: impl IntoIterator<Item = ((i32, u32), Cplx)>) -> Self {
        let mut coeffs = BTreeMap::new();
        for (key, v) in entries {
            if !v.is_zero() {
                coeffs.insert(key, v);
            }
        }
        Self { coeffs }
    }
    pub fn coefficients(&self) -> &BTreeMap<(i32, u32), Cplx> {
        &self.coeffs
    }
    /// The value at `tau = 0`, rounded (the certificate encloses it).
    pub fn at_zero(&self) -> Cplx {
        self.coeffs
            .iter()
            .filter(|((_, j), _)| *j == 0)
            .fold(Cplx::ZERO, |s, (_, v)| s.add(*v))
    }
    fn add(&self, o: &Self) -> Self {
        let mut c = self.coeffs.clone();
        for (k, v) in &o.coeffs {
            let e = c.entry(*k).or_insert(Cplx::ZERO);
            *e = e.add(*v);
        }
        c.retain(|_, v| !v.is_zero());
        Self { coeffs: c }
    }
    fn mul(&self, o: &Self) -> Self {
        let mut c: BTreeMap<(i32, u32), Cplx> = BTreeMap::new();
        for ((k, j), x) in &self.coeffs {
            for ((l, n), y) in &o.coeffs {
                let e = c.entry((k + l, j + n)).or_insert(Cplx::ZERO);
                *e = e.add(x.mul(*y));
            }
        }
        c.retain(|_, v| !v.is_zero());
        Self { coeffs: c }
    }
    fn scale(&self, z: Cplx) -> Self {
        Self::from_coefficients(self.coeffs.iter().map(|(k, v)| (*k, v.mul(z))))
    }
    fn conj(&self) -> Self {
        Self::from_coefficients(self.coeffs.iter().map(|((k, j), v)| ((-k, *j), v.conj())))
    }
    /// Integral from 0 (the source's `primitive`, binary64).
    fn primitive(&self, a: f64) -> Self {
        let mut o: BTreeMap<(i32, u32), Cplx> = BTreeMap::new();
        let mut put = |key: (i32, u32), v: Cplx| {
            let e = o.entry(key).or_insert(Cplx::ZERO);
            *e = e.add(v);
        };
        for (&(k, j), &v) in &self.coeffs {
            if k == 0 || a == 0.0 {
                put((0, j + 1), v.scale(1.0 / (j + 1) as f64));
                continue;
            }
            let rate = Cplx::new(0.0, k as f64 * a);
            let mut b = v.div(rate);
            put((k, j), b);
            for n in (0..j).rev() {
                b = b.scale(-((n + 1) as f64)).div(rate);
                put((k, n), b);
            }
            put((0, 0), b.scale(-1.0));
        }
        let mut path = Self { coeffs: o };
        path.coeffs.retain(|_, v| !v.is_zero());
        path
    }
    fn truncate(&self, k_max: i32, p_max: u32) -> Self {
        Self::from_coefficients(
            self.coeffs
                .iter()
                .filter(|((k, j), _)| k.abs() <= k_max && *j <= p_max)
                .map(|(k, v)| (*k, *v)),
        )
    }
    fn enforce_start(&self, z: Cplx) -> Self {
        let shift = z.sub(self.at_zero());
        self.add(&Self::constant(shift))
    }
    fn to_interval(&self) -> CoreResult<IPath> {
        let mut c = BTreeMap::new();
        for (k, v) in &self.coeffs {
            c.insert(*k, CInterval::point(*v)?);
        }
        Ok(IPath { coeffs: c })
    }
}

/// Interval path used inside the certificate.
#[derive(Clone, Debug, Default)]
struct IPath {
    coeffs: BTreeMap<(i32, u32), CInterval>,
}

impl IPath {
    fn constant(z: CInterval) -> Self {
        let mut coeffs = BTreeMap::new();
        coeffs.insert((0, 0), z);
        Self { coeffs }
    }
    fn add(&self, o: &Self) -> CoreResult<Self> {
        let mut c = self.coeffs.clone();
        for (k, v) in &o.coeffs {
            let e = c.entry(*k).or_insert(CInterval::zero());
            *e = e.add(*v)?;
        }
        Ok(Self { coeffs: c })
    }
    fn sub(&self, o: &Self) -> CoreResult<Self> {
        let mut c = self.coeffs.clone();
        for (k, v) in &o.coeffs {
            let e = c.entry(*k).or_insert(CInterval::zero());
            *e = e.sub(*v)?;
        }
        Ok(Self { coeffs: c })
    }
    /// The exact noncyclic product, enclosed.
    fn mul(&self, o: &Self) -> CoreResult<Self> {
        let mut c: BTreeMap<(i32, u32), CInterval> = BTreeMap::new();
        for ((k, j), x) in &self.coeffs {
            for ((l, n), y) in &o.coeffs {
                let e = c.entry((k + l, j + n)).or_insert(CInterval::zero());
                *e = e.add(x.mul(*y)?)?;
            }
        }
        Ok(Self { coeffs: c })
    }
    fn map(&self, f: impl Fn(CInterval) -> CoreResult<CInterval>) -> CoreResult<Self> {
        let mut c = BTreeMap::new();
        for (k, v) in &self.coeffs {
            c.insert(*k, f(*v)?);
        }
        Ok(Self { coeffs: c })
    }
    fn conj(&self) -> Self {
        Self {
            coeffs: self
                .coeffs
                .iter()
                .map(|((k, j), v)| ((-k, *j), v.conj()))
                .collect(),
        }
    }
    /// d/dtau of `sum c tau^j e^{i k a tau}`.
    fn derivative(&self, a: Interval) -> CoreResult<Self> {
        let mut c: BTreeMap<(i32, u32), CInterval> = BTreeMap::new();
        for (&(k, j), &v) in &self.coeffs {
            if j > 0 {
                let e = c.entry((k, j - 1)).or_insert(CInterval::zero());
                *e = e.add(v.mul_real(Interval::point(j as f64)?)?)?;
            }
            if k != 0 {
                let e = c.entry((k, j)).or_insert(CInterval::zero());
                *e = e.add(v.mul_i_real(Interval::point(k as f64)?.mul(a)?)?)?;
            }
        }
        Ok(Self { coeffs: c })
    }
    /// `sup_{tau in [0,1]} |path(tau)|_1`: per harmonic the complex
    /// Bernstein hull of the polynomial amplitude (degree not trimmed), then
    /// the triangle inequality over harmonics, all rounded up.
    fn sup_bound(&self) -> CoreResult<f64> {
        let mut by_k: BTreeMap<i32, Vec<(u32, CInterval)>> = BTreeMap::new();
        for (&(k, j), &v) in &self.coeffs {
            by_k.entry(k).or_default().push((j, v));
        }
        let mut out = 0.0;
        for terms in by_k.values() {
            let p = terms.iter().map(|(j, _)| *j).max().unwrap_or(0) as usize;
            let mut amp = vec![CInterval::zero(); p + 1];
            for (j, v) in terms {
                amp[*j as usize] = *v;
            }
            let mut best = 0.0_f64;
            for i in 0..=p {
                let mut b = CInterval::zero();
                for (j, a) in amp.iter().enumerate().take(i + 1) {
                    if a.is_zero() {
                        continue;
                    }
                    let ratio = binomial(i, j)?.div(binomial(p, j)?)?;
                    b = b.add(a.mul_real(ratio)?)?;
                }
                best = best.max(b.mag1()?);
            }
            out = add_up(out, best)?;
        }
        Ok(out)
    }
    /// Enclosure of the value at `tau = 1`.
    fn value_at_one(&self, a: f64) -> CoreResult<CInterval> {
        let mut by_k: BTreeMap<i32, CInterval> = BTreeMap::new();
        for (&(k, _), &v) in &self.coeffs {
            let e = by_k.entry(k).or_insert(CInterval::zero());
            *e = e.add(v)?;
        }
        let mut total = CInterval::zero();
        for (k, v) in by_k {
            let angle = Interval::point(k as f64)?.mul(Interval::point(a)?)?;
            let rot = rotation_interval(angle)?;
            total = total.add(v.mul(rot)?)?;
        }
        Ok(total)
    }
}

/// `C(n, k)` as an interval (exact up to 2^53).
fn binomial(n: usize, k: usize) -> CoreResult<Interval> {
    let mut v: u128 = 1;
    for i in 0..k {
        v = v * (n - i) as u128 / (i + 1) as u128;
    }
    if v <= (1u128 << 53) {
        Interval::point(v as f64)
    } else {
        let f = v as f64;
        Interval::new(f.next_down(), f.next_up())
    }
}

/// `e^{i x}` for every `x` in the interval: the ported `rotate_interval`
/// applied to both endpoints is not enough for a wide interval, so a wide
/// one is refused; a point or a few-ulp interval is reduced by halving to
/// `|r| <= 1/8`, the Taylor sums with the remainder `2 r^(n+1)/(n+1)!` are
/// enclosed, and interval doubling restores the angle.
fn rotation_interval(x: Interval) -> CoreResult<CInterval> {
    if x.lo == 0.0 && x.hi == 0.0 {
        return Ok(CInterval::real(Interval::point(1.0)?));
    }
    let (lo, hi) = (rotation_point(x.lo)?, rotation_point(x.hi)?);
    if x.lo == x.hi {
        return Ok(lo);
    }
    // e^{ix} on [lo, hi]: |e^{ix} - e^{i lo}| <= hi - lo, so the hull of the
    // lo enclosure widened by the width (rounded up) encloses it.
    let width = sub_up(x.hi, x.lo)?;
    if width > 1.0e-6 {
        return Err(invalid("rotation of a wide angle interval"));
    }
    let widen = |i: Interval| Interval::new(sub_down(i.lo, width)?, add_up(i.hi, width)?);
    let _ = hi;
    Ok(CInterval {
        re: widen(lo.re)?,
        im: widen(lo.im)?,
    })
}

fn rotation_point(angle: f64) -> CoreResult<CInterval> {
    if !angle.is_finite() {
        return Err(invalid("nonfinite angle"));
    }
    if angle == 0.0 {
        return Ok(CInterval::real(Interval::point(1.0)?));
    }
    // Reduction modulo 2 pi with a Cody-Waite split checked at 60 digits:
    // 2 pi = A + B + d, A with 26 trailing zero bits (k A exact for
    // |k| < 2^23), 0 < d < ulp(B). The source halves the angle and doubles
    // back, which in binary64 interval arithmetic widens by about 2.8 per
    // doubling; the reduction keeps the enclosure near one ulp.
    let two_pi_a = f64::from_bits(0x4019_21FB_5400_0000);
    let two_pi_b = Interval::new(
        f64::from_bits(0x3E31_0B46_11A6_2633),
        f64::from_bits(0x3E31_0B46_11A6_2633).next_up(),
    )?;
    let k = (angle / (two_pi_a + two_pi_b.lo)).round();
    if k.abs() >= (1u64 << 23) as f64 {
        return Err(invalid("angle outside the reduction range"));
    }
    let r = Interval::point(angle)?
        .sub(Interval::point(k * two_pi_a)?)?
        .sub(Interval::point(k)?.mul(two_pi_b)?)?;
    let mut c = Interval::point(1.0)?;
    let mut sn = Interval::point(0.0)?;
    let mut power = Interval::point(1.0)?;
    let mut n = 1;
    let rmag = r.mag();
    let tail = loop {
        power = power.mul(r)?.div(Interval::point(n as f64)?)?;
        match n % 4 {
            0 => c = c.add(power)?,
            1 => sn = sn.add(power)?,
            2 => c = c.sub(power)?,
            _ => sn = sn.sub(power)?,
        }
        // Each remaining tail is at most 2 |r|^(n+1)/(n+1)! once
        // |r| / (n+2) <= 1/2 (geometric majorant).
        let t = div_up(mul_up(2.0, mul_up(power.mag(), rmag)?)?, (n + 1) as f64)?;
        if (rmag / (n + 2) as f64 <= 0.5 && t < 1.0e-40) || n >= 120 {
            if rmag / (n + 2) as f64 > 0.5 {
                return Err(invalid("rotation series did not converge"));
            }
            break t;
        }
        n += 1;
    };
    let unit = Interval::new(-1.0, 1.0)?;
    let clamp = |i: Interval| Interval::new(i.lo.max(unit.lo), i.hi.min(unit.hi));
    let cc = clamp(Interval::new(sub_down(c.lo, tail)?, add_up(c.hi, tail)?)?)?;
    let ss = clamp(Interval::new(sub_down(sn.lo, tail)?, add_up(sn.hi, tail)?)?)?;
    Ok(CInterval { re: cc, im: ss })
}

/// Model parameters; `q = sigma 5/4` (invariant leaf).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FourierModel {
    pub omega: f64,
    pub g: f64,
    pub eps: f64,
    pub sigma: i32,
    pub epoch: u64,
}

impl FourierModel {
    pub fn new(omega: f64, sigma: i32) -> CoreResult<Self> {
        if !omega.is_finite() || !(sigma == 1 || sigma == -1) {
            return Err(invalid("finite omega and sigma = +-1 required"));
        }
        Ok(Self {
            omega,
            g: 0.125,
            eps: 0.125,
            sigma,
            epoch: 0,
        })
    }
    pub fn q(&self) -> f64 {
        self.sigma as f64 * 1.25
    }
    /// `5/4 (2|g| + 3|eps|)`, rounded up.
    fn growth_rate(&self) -> CoreResult<f64> {
        mul_up(
            1.25,
            add_up(mul_up(2.0, self.g.abs())?, mul_up(3.0, self.eps.abs())?)?,
        )
    }
}

/// A committed state: time, amplitudes, error bound, generation.
#[derive(Clone, Debug, PartialEq)]
pub struct FourierState {
    t: f64,
    a: [Cplx; 2],
    error: f64,
    generation: u64,
}

impl FourierState {
    /// The archived initial state `(1/4, 1/2)` on the leaf `9/16`.
    pub fn initial() -> Self {
        Self {
            t: 0.0,
            a: [Cplx::new(0.25, 0.0), Cplx::new(0.5, 0.0)],
            error: 0.0,
            generation: 0,
        }
    }
    pub fn t(&self) -> f64 {
        self.t
    }
    pub fn amplitudes(&self) -> [Cplx; 2] {
        self.a
    }
    pub fn error(&self) -> f64 {
        self.error
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// The certificate's numbers; no public constructor besides
/// [`certificate`], no deserialization.
#[derive(Clone, Debug, PartialEq)]
pub struct FourierCertificate {
    accepted: bool,
    reason: Option<String>,
    sup_bound: f64,
    differential_defect: f64,
    phase_defect: f64,
    residual: f64,
    start_mismatch: f64,
    growth: f64,
    endpoint: [Cplx; 2],
    endpoint_rounding: f64,
    error: f64,
    closure_defect: f64,
    phase_mode: &'static str,
    binding: TrialBinding,
}

impl FourierCertificate {
    pub fn accepted(&self) -> bool {
        self.accepted
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn error(&self) -> f64 {
        self.error
    }
    pub fn growth(&self) -> f64 {
        self.growth
    }
    pub fn residual(&self) -> f64 {
        self.residual
    }
    pub fn differential_defect(&self) -> f64 {
        self.differential_defect
    }
    pub fn phase_defect(&self) -> f64 {
        self.phase_defect
    }
    pub fn start_mismatch(&self) -> f64 {
        self.start_mismatch
    }
    pub fn sup_bound(&self) -> f64 {
        self.sup_bound
    }
    pub fn endpoint(&self) -> [Cplx; 2] {
        self.endpoint
    }
    pub fn endpoint_rounding(&self) -> f64 {
        self.endpoint_rounding
    }
    pub fn closure_defect(&self) -> f64 {
        self.closure_defect
    }
    pub fn phase_mode(&self) -> &'static str {
        self.phase_mode
    }
}

/// Exact copies of what a certificate was computed for.
#[derive(Clone, Debug, PartialEq)]
struct TrialBinding {
    state: FourierState,
    model: FourierModel,
    h: f64,
    path: [FourierPath; 2],
}

/// The phase witness `P(tau)` and `phase_error >= sup |P - e^{i omega (t + h
/// tau)}|_1`, built from the rotation enclosure of `omega t`: a degree-20
/// confluent Taylor polynomial of `e^{i omega h tau}` when `|omega h| <= 1`,
/// otherwise the single harmonic `(1, 0)`.
fn phase_witness(model: &FourierModel, t: f64, h: f64) -> CoreResult<(IPath, f64, &'static str)> {
    let angle = Interval::point(model.omega)?.mul(Interval::point(t)?)?;
    let rot = rotation_interval(angle)?;
    // Midpoint and its outward distance to the enclosure, 1-norm.
    let mid = Cplx::new(
        0.5 * rot.re.lo + 0.5 * rot.re.hi,
        0.5 * rot.im.lo + 0.5 * rot.im.hi,
    );
    let dist =
        |m: f64, i: Interval| -> CoreResult<f64> { Ok(sub_up(m, i.lo)?.max(sub_up(i.hi, m)?)) };
    let e0 = add_up(dist(mid.re, rot.re)?, dist(mid.im, rot.im)?)?;
    let a = Interval::point(model.omega)?.mul(Interval::point(h)?)?;
    let midi = CInterval::point(mid)?;
    if a.mag() <= 1.0 {
        let degree = 20;
        let mut coeffs = BTreeMap::new();
        let mut power = CInterval::real(Interval::point(1.0)?);
        coeffs.insert((0, 0), midi);
        for j in 1..=degree {
            power = power
                .mul_i_real(a)?
                .mul_real(Interval::point(1.0)?.div(Interval::point(j as f64)?)?)?;
            coeffs.insert((0, j), midi.mul(power)?);
        }
        // |a|^(d+1)/(d+1)! tail of e^{i a tau} on [0, 1].
        let mut tail = 1.0_f64;
        for j in 1..=degree + 1 {
            tail = mul_up(tail, div_up(a.mag(), j as f64)?)?;
        }
        let error = add_up(e0, mul_up(CInterval::point(mid)?.mag1()?, tail)?)?;
        Ok((IPath { coeffs }, error, "CONFLUENT_TAYLOR_PHASE"))
    } else {
        Ok((
            IPath::constant(midi).shift_harmonic(1),
            e0,
            "FOURIER_EXP_PHASE",
        ))
    }
}

impl IPath {
    fn shift_harmonic(self, k: i32) -> Self {
        Self {
            coeffs: self
                .coeffs
                .into_iter()
                .map(|((kk, j), v)| ((kk + k, j), v))
                .collect(),
        }
    }
}

/// The lambda = 1 right side times `h`, enclosed (noncyclic products).
fn rhs_h(path: &[IPath; 2], model: &FourierModel, phase: &IPath, h: f64) -> CoreResult<[IPath; 2]> {
    let (a, b) = (&path[0], &path[1]);
    let pa = phase.mul(a)?;
    let half = Interval::point(0.5)?;
    let realmod = pa.add(&pa.conj())?.map(|v| v.mul_real(half))?;
    let c = IPath::constant(CInterval::real(Interval::point(model.g)?))
        .add(&realmod.map(|v| v.mul_real(Interval::point(model.eps)?))?)?;
    let hq = Interval::point(h)?.mul(Interval::point(model.q())?)?;
    let fa = c.mul(&a.conj())?.mul(b)?.map(|v| v.mul_i_real(hq))?;
    let fb = c.mul(a)?.mul(a)?.map(|v| v.mul_i_real(hq.mul(half)?))?;
    Ok([fa, fb])
}

/// The original-target certificate of one step (source `certificate`).
pub fn certificate(
    state: &FourierState,
    h: f64,
    model: &FourierModel,
    path: &[FourierPath; 2],
) -> CoreResult<FourierCertificate> {
    if !(h.is_finite() && h > 0.0) {
        return Err(invalid("positive finite step required"));
    }
    let growth_rate = model.growth_rate()?;
    let hg = mul_up(h, growth_rate)?;
    if hg >= 1.0 {
        return Err(invalid("positive contractive slab required (h growth < 1)"));
    }
    let ipath = [path[0].to_interval()?, path[1].to_interval()?];
    // Start mismatch, charged (registered deviation 1).
    let mut start_mismatch = 0.0_f64;
    for (p, z) in ipath.iter().zip(&state.a) {
        let mut s = CInterval::zero();
        for (&(_, j), &v) in &p.coeffs {
            if j == 0 {
                s = s.add(v)?;
            }
        }
        let d = s.sub(CInterval::point(*z)?)?;
        start_mismatch = start_mismatch.max(d.mag1()?);
    }
    let binding = TrialBinding {
        state: state.clone(),
        model: *model,
        h,
        path: path.clone(),
    };
    let b = ipath[0].sup_bound()?.max(ipath[1].sup_bound()?);
    let (phase, phase_error, phase_mode) = phase_witness(model, state.t, h)?;
    let reject = |reason: &str, b: f64| FourierCertificate {
        accepted: false,
        reason: Some(reason.into()),
        sup_bound: b,
        differential_defect: f64::INFINITY,
        phase_defect: f64::INFINITY,
        residual: f64::INFINITY,
        start_mismatch,
        growth: f64::INFINITY,
        endpoint: [Cplx::ZERO; 2],
        endpoint_rounding: f64::INFINITY,
        error: f64::INFINITY,
        closure_defect: f64::INFINITY,
        phase_mode,
        binding: binding.clone(),
    };
    if !(b <= 1.0) {
        return Ok(reject("CANDIDATE_DOMAIN", b));
    }
    let a = Interval::point(model.omega)?.mul(Interval::point(h)?)?;
    let ff = rhs_h(&ipath, model, &phase, h)?;
    let mut d_alg = 0.0_f64;
    for (p, f) in ipath.iter().zip(&ff) {
        d_alg = d_alg.max(p.derivative(a)?.sub(f)?.sup_bound()?);
    }
    // h |q eps| B^3 phase_error.
    let d_phase = mul_up(
        mul_up(h, mul_up(model.q().abs(), model.eps.abs())?)?,
        mul_up(mul_up(mul_up(b, b)?, b)?, phase_error)?,
    )?;
    let residual = add_up(d_alg, d_phase)?;
    let growth = div_up(1.0, sub_down(1.0, hg)?)?;
    let mut endpoint = [Cplx::ZERO; 2];
    let mut rnd = 0.0_f64;
    for (j, p) in ipath.iter().enumerate() {
        let v = p.value_at_one(a.hi)?;
        let m = Cplx::new(0.5 * v.re.lo + 0.5 * v.re.hi, 0.5 * v.im.lo + 0.5 * v.im.hi);
        let dist =
            |m: f64, i: Interval| -> CoreResult<f64> { Ok(sub_up(m, i.lo)?.max(sub_up(i.hi, m)?)) };
        rnd = rnd.max(add_up(dist(m.re, v.re)?, dist(m.im, v.im)?)?);
        endpoint[j] = m;
    }
    if a.lo != a.hi {
        return Err(invalid("omega h is not exact"));
    }
    let start = add_up(state.error, start_mismatch)?;
    let error = add_up(mul_up(growth, add_up(start, residual)?)?, rnd)?;
    let closure = ipath[0]
        .mul(&ipath[0].conj())?
        .add(
            &ipath[1]
                .mul(&ipath[1].conj())?
                .map(|v| v.mul_real(Interval::point(2.0)?))?,
        )?
        .sub(&IPath::constant(CInterval::real(Interval::point(0.5625)?)))?;
    let closure_defect = closure.sup_bound()?;
    Ok(FourierCertificate {
        accepted: error.is_finite(),
        reason: None,
        sup_bound: b,
        differential_defect: d_alg,
        phase_defect: d_phase,
        residual,
        start_mismatch,
        growth,
        endpoint,
        endpoint_rounding: rnd,
        error,
        closure_defect,
        phase_mode,
        binding,
    })
}

/// A built trial: the path and its certificate.
#[derive(Clone, Debug)]
pub struct FourierTrial {
    pub path: [FourierPath; 2],
    pub certificate: FourierCertificate,
    pub h: f64,
    pub k_max: i32,
    pub p_max: u32,
    pub sweeps: usize,
    /// Predictor right-side evaluations (convolution calls).
    pub rhs_calls: usize,
}

/// Commit an accepted trial within `cap` (source `commit`): refuses an
/// unaccepted or over-budget certificate and any stale or mixed binding.
pub fn commit(
    trial: &FourierTrial,
    state: &FourierState,
    model: &FourierModel,
    cap: f64,
) -> CoreResult<FourierState> {
    let c = &trial.certificate;
    if !c.accepted || !(c.error <= cap) {
        return Err(invalid("uncertified or budget-failed candidate"));
    }
    if c.binding.state != *state
        || c.binding.model != *model
        || c.binding.h != trial.h
        || c.binding.path != trial.path
    {
        return Err(invalid("stale or mixed trial"));
    }
    let t = state.t + trial.h;
    if t <= state.t {
        return Err(invalid("time does not advance"));
    }
    Ok(FourierState {
        t,
        a: c.endpoint,
        error: c.error,
        generation: state.generation + 1,
    })
}

/// Which right side the untrusted predictor uses: the client's coupled
/// one, or a negative control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredictorRhs {
    Coupled,
    /// Coupling dropped: `b` and `a` frozen at the start in the other
    /// equation, no `eps` term (independent scalars).
    IndependentScalar,
    /// The phase rotated by 0.1 rad (a wrong phase witness).
    WrongPhase,
    /// Harmonics outside `|k| <= K` wrapped cyclically instead of dropped.
    CyclicAlias,
}

/// The binary64 phase polynomial of the predictor (untrusted).
fn predictor_phase(model: &FourierModel, t: f64, h: f64, rotate: f64) -> FourierPath {
    let (s, c) = (model.omega * t + rotate).sin_cos();
    let mid = Cplx::new(c, s);
    let a = model.omega * h;
    if a.abs() <= 1.0 {
        let mut entries = vec![((0, 0), mid)];
        let mut power = Cplx::new(1.0, 0.0);
        for j in 1..=20u32 {
            power = power.mul(Cplx::new(0.0, a)).scale(1.0 / j as f64);
            entries.push(((0, j), mid.mul(power)));
        }
        FourierPath::from_coefficients(entries)
    } else {
        FourierPath::from_coefficients([((1, 0), mid)])
    }
}

/// Binary64 right side times `h` (direct noncyclic product) for the
/// predictor.
pub fn predictor_rhs_h(
    path: &[FourierPath; 2],
    model: &FourierModel,
    phase: &FourierPath,
    h: f64,
    lambda: f64,
) -> [FourierPath; 2] {
    let (a, b) = (&path[0], &path[1]);
    let pa = phase.mul(a);
    let realmod = pa.add(&pa.conj()).scale(Cplx::new(0.5, 0.0));
    let c = FourierPath::constant(Cplx::new(model.g, 0.0))
        .add(&realmod.scale(Cplx::new(lambda * model.eps, 0.0)));
    let fa = c.mul(&a.conj()).mul(b).scale(Cplx::new(0.0, h * model.q()));
    let fb = c.mul(a).mul(a).scale(Cplx::new(0.0, h * model.q() / 2.0));
    [fa, fb]
}

/// A predictor right side: the direct product, or an injected one (PP06
/// plugs in the FFT product here).
pub type RhsFn<'a> = &'a mut dyn FnMut(
    &[FourierPath; 2],
    &FourierModel,
    &FourierPath,
    f64,
) -> CoreResult<[FourierPath; 2]>;

fn wrap(path: &FourierPath, k_max: i32) -> FourierPath {
    let period = 2 * k_max + 1;
    let mut o: BTreeMap<(i32, u32), Cplx> = BTreeMap::new();
    for (&(k, j), &v) in &path.coeffs {
        let kk = (k + k_max).rem_euclid(period) - k_max;
        let e = o.entry((kk, j)).or_insert(Cplx::ZERO);
        *e = e.add(v);
    }
    FourierPath::from_coefficients(o)
}

/// Untrusted Picard predictor (source `build`, `homotopy = (1,)`) and the
/// certificate of its result.
#[allow(clippy::too_many_arguments)]
pub fn build_trial(
    state: &FourierState,
    h: f64,
    model: &FourierModel,
    k_max: i32,
    p_max: u32,
    sweeps: usize,
    kind: PredictorRhs,
    rhs: Option<RhsFn<'_>>,
) -> CoreResult<FourierTrial> {
    if !(h > 0.0) || k_max < 0 || p_max < 1 || sweeps < 1 {
        return Err(invalid("bad step or resource contract"));
    }
    let rotate = if kind == PredictorRhs::WrongPhase {
        0.1
    } else {
        0.0
    };
    let phase = predictor_phase(model, state.t, h, rotate);
    let a = model.omega * h;
    let mut path = [
        FourierPath::constant(state.a[0]),
        FourierPath::constant(state.a[1]),
    ];
    let mut rhs = rhs;
    let mut calls = 0;
    for _ in 0..sweeps {
        let ff = match (&mut rhs, kind) {
            (Some(f), PredictorRhs::Coupled) => f(&path, model, &phase, h)?,
            (_, PredictorRhs::IndependentScalar) => {
                let a0 = FourierPath::constant(state.a[0]);
                let b0 = FourierPath::constant(state.a[1]);
                let g = Cplx::new(model.g, 0.0);
                [
                    a0.conj()
                        .mul(&b0)
                        .scale(g.mul(Cplx::new(0.0, h * model.q()))),
                    a0.mul(&a0)
                        .scale(g.mul(Cplx::new(0.0, h * model.q() / 2.0))),
                ]
            }
            _ => predictor_rhs_h(&path, model, &phase, h, 1.0),
        };
        calls += 1;
        let mut next = [FourierPath::default(), FourierPath::default()];
        for j in 0..2 {
            let raw = FourierPath::constant(state.a[j]).add(&ff[j].primitive(a));
            let shaped = if kind == PredictorRhs::CyclicAlias {
                wrap(&raw, k_max).truncate(k_max, p_max)
            } else {
                raw.truncate(k_max, p_max)
            };
            next[j] = shaped.enforce_start(state.a[j]);
        }
        path = next;
    }
    let certificate = certificate(state, h, model, &path)?;
    Ok(FourierTrial {
        path,
        certificate,
        h,
        k_max,
        p_max,
        sweeps,
        rhs_calls: calls,
    })
}

/// `physical`: the state rotated to physical coordinates and the
/// H-norm error `sqrt(r_a^2 + 2 r_b^2)`, rounded up.
pub fn physical_error(state: &FourierState, model: &FourierModel) -> CoreResult<([Cplx; 2], f64)> {
    let mut values = [Cplx::ZERO; 2];
    let mut rads = [0.0_f64; 2];
    for (j, z) in state.a.iter().enumerate() {
        let angle = Interval::point((j + 1) as f64)?
            .mul(Interval::point(model.omega)?)?
            .mul(Interval::point(state.t)?)?;
        let v = CInterval::point(*z)?.mul(rotation_interval(angle)?)?;
        let m = Cplx::new(0.5 * v.re.lo + 0.5 * v.re.hi, 0.5 * v.im.lo + 0.5 * v.im.hi);
        let r = add_up(
            sub_up(m.re, v.re.lo)?.max(sub_up(v.re.hi, m.re)?),
            sub_up(m.im, v.im.lo)?.max(sub_up(v.im.hi, m.im)?),
        )?;
        values[j] = m;
        rads[j] = add_up(state.error, r)?;
    }
    let h = sqrt_up(add_up(
        mul_up(rads[0], rads[0])?,
        mul_up(2.0, mul_up(rads[1], rads[1])?)?,
    )?)?;
    Ok((values, h))
}

/// One record of a driver run.
#[derive(Clone, Debug)]
pub struct StepRecord {
    pub start: FourierState,
    pub trial: FourierTrial,
    pub state: FourierState,
    pub physical_error: f64,
}

/// Driver counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DriverCounts {
    pub candidate_builds: usize,
    pub candidate_rejections: usize,
    pub rhs_calls: usize,
    pub step_halvings: usize,
}

/// The source's step driver (`run_case`, DIRECT policy): step cap 1/8,
/// candidates `(2,4,2)` then `(3,6,4)`, local budget `tol h / (8 T G)` and
/// `error <= tol/4`, halving down to `2^-20`, physical check `<= tol`.
pub fn run_driver(
    model: &FourierModel,
    t_end: f64,
    tol: f64,
    cap: f64,
    mut rhs: Option<RhsFn<'_>>,
) -> CoreResult<(Vec<StepRecord>, DriverCounts)> {
    let mut state = FourierState::initial();
    let growth_rate = model.growth_rate()?;
    let global = div_up(1.0, sub_down(1.0, mul_up(t_end, growth_rate)?)?)?;
    let mut records = Vec::new();
    let mut counts = DriverCounts::default();
    while state.t < t_end {
        let mut h = cap.min(t_end - state.t);
        let accepted = loop {
            let mut found = None;
            for (k, p, sweeps) in [(2, 4, 2), (3, 6, 4)] {
                let trial = build_trial(
                    &state,
                    h,
                    model,
                    k,
                    p,
                    sweeps,
                    PredictorRhs::Coupled,
                    rhs.as_mut().map(|f| &mut **f as RhsFn<'_>),
                )?;
                counts.candidate_builds += 1;
                counts.rhs_calls += trial.rhs_calls;
                let c = &trial.certificate;
                if c.accepted {
                    let local = sub_up(c.error, mul_up(c.growth, state.error)?.min(c.error))?;
                    let local_budget = tol * h / (8.0 * t_end * global);
                    if local <= local_budget && c.error <= tol / 4.0 {
                        found = Some(trial);
                        break;
                    }
                }
                counts.candidate_rejections += 1;
            }
            if let Some(t) = found {
                break t;
            }
            h *= 0.5;
            counts.step_halvings += 1;
            if h < 2.0_f64.powi(-20) {
                return Err(invalid("STEP_RESOURCE_CAP"));
            }
        };
        let next = commit(&accepted, &state, model, tol / 4.0)?;
        let (_, physical) = physical_error(&next, model)?;
        if !(physical <= tol) {
            return Err(invalid("physical budget not closed"));
        }
        records.push(StepRecord {
            start: state.clone(),
            trial: accepted,
            state: next.clone(),
            physical_error: physical,
        });
        state = next;
        if records.len() > 4096 {
            return Err(invalid("resource cap: number of steps"));
        }
    }
    Ok((records, counts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_encloses_library_values() {
        for x in [0.1, 1.0, 5.0, -40.0, 1250.0, 5000.0, 3750.0, -1.0e5] {
            let r = rotation_point(x).unwrap();
            assert!(
                r.re.lo <= x.cos() + 1e-15 && x.cos() - 1e-15 <= r.re.hi,
                "{x}"
            );
            assert!(
                r.im.lo <= x.sin() + 1e-15 && x.sin() - 1e-15 <= r.im.hi,
                "{x}"
            );
            assert!(r.re.hi - r.re.lo < 1e-12, "{x}");
        }
    }

    #[test]
    fn primitive_derivative_roundtrip() {
        let p = FourierPath::from_coefficients([
            ((0, 0), Cplx::new(1.0, 0.5)),
            ((1, 2), Cplx::new(-0.25, 0.75)),
        ]);
        let q = p.primitive(3.0);
        let d = q
            .to_interval()
            .unwrap()
            .derivative(Interval::point(3.0).unwrap())
            .unwrap();
        for ((k, j), v) in &p.coeffs {
            let e = d.coeffs[&(*k, *j)];
            assert!((e.re.lo - v.re).abs() < 1e-14 && (e.im.lo - v.im).abs() < 1e-14);
        }
    }
}
