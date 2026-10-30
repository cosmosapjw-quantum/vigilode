//! Research-only certified damped oscillatory convolution moments for the
//! N2/N3H represented two-rate source, Rust 1.99.1, RN + gradual underflow.
//! NO arbitrary ODE source binding, no adaptive acceptance, no global error,
//! no native RODAS8 production claim.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct I {
    pub lo: f64,
    pub hi: f64,
}
fn dn(v: f64) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v.next_down())
    } else {
        Err("nonfinite directed operation".into())
    }
}
fn up(v: f64) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v.next_up())
    } else {
        Err("nonfinite directed operation".into())
    }
}
#[allow(clippy::should_implement_trait)]
impl I {
    pub fn point(x: f64) -> Result<Self, String> {
        if !x.is_finite() {
            Err("nonfinite source".into())
        } else {
            Ok(Self { lo: x, hi: x })
        }
    }
    pub fn new(lo: f64, hi: f64) -> Result<Self, String> {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            Err("invalid enclosure".into())
        } else {
            Ok(Self { lo, hi })
        }
    }
    pub fn add(self, b: Self) -> Result<Self, String> {
        Self::new(dn(self.lo + b.lo)?, up(self.hi + b.hi)?)
    }
    pub fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    pub fn sub(self, b: Self) -> Result<Self, String> {
        self.add(b.neg())
    }
    pub fn mul(self, b: Self) -> Result<Self, String> {
        let p = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Err("multiplication overflow".into());
        }
        Self::new(
            dn(p.iter().copied().fold(f64::INFINITY, f64::min))?,
            up(p.iter().copied().fold(f64::NEG_INFINITY, f64::max))?,
        )
    }
    pub fn div_positive(self, b: Self) -> Result<Self, String> {
        if b.lo <= 0. || !b.hi.is_finite() {
            return Err("nonpositive divisor".into());
        }
        let p = [
            self.lo / b.lo,
            self.lo / b.hi,
            self.hi / b.lo,
            self.hi / b.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Err("division overflow".into());
        }
        Self::new(
            dn(p.iter().copied().fold(f64::INFINITY, f64::min))?,
            up(p.iter().copied().fold(f64::NEG_INFINITY, f64::max))?,
        )
    }
    pub fn clamp_nonnegative(self) -> Self {
        Self {
            lo: self.lo.max(0.),
            hi: self.hi.max(0.),
        }
    }
    pub fn mag(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
    pub fn pad(self, r: Self) -> Result<Self, String> {
        if r.lo < 0. {
            return Err("negative padding".into());
        }
        self.add(Self::new(-r.hi, r.hi)?)
    }
    pub fn zero() -> Self {
        Self { lo: 0., hi: 0. }
    }
    pub fn one() -> Self {
        Self { lo: 1., hi: 1. }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Z {
    pub re: I,
    pub im: I,
}
#[allow(clippy::should_implement_trait)]
impl Z {
    pub fn zero() -> Self {
        Self {
            re: I::zero(),
            im: I::zero(),
        }
    }
    pub fn one() -> Self {
        Self {
            re: I::one(),
            im: I::zero(),
        }
    }
    pub fn add(self, b: Self) -> Result<Self, String> {
        Ok(Self {
            re: self.re.add(b.re)?,
            im: self.im.add(b.im)?,
        })
    }
    pub fn mul(self, b: Self) -> Result<Self, String> {
        Ok(Self {
            re: self.re.mul(b.re)?.sub(self.im.mul(b.im)?)?,
            im: self.re.mul(b.im)?.add(self.im.mul(b.re)?)?,
        })
    }
    pub fn div_positive(self, b: I) -> Result<Self, String> {
        Ok(Self {
            re: self.re.div_positive(b)?,
            im: self.im.div_positive(b)?,
        })
    }
    pub fn pad(self, r: I) -> Result<Self, String> {
        Ok(Self {
            re: self.re.pad(r)?,
            im: self.im.pad(r)?,
        })
    }
}
fn i(x: f64) -> Result<I, String> {
    I::point(x)
}
fn pow(base: I, n: usize) -> Result<I, String> {
    let mut x = I::one();
    for _ in 0..n {
        x = x.mul(base)?;
    }
    Ok(x)
}
fn fact(n: usize) -> Result<I, String> {
    let mut x = I::one();
    for k in 1..=n {
        x = x.mul(i(k as f64)?)?;
    }
    Ok(x)
}

/// Rigorous enclosed exp(-x) for exactly represented nonnegative x<=1000.
/// Uses positive exponential series, analytic geometric tail and range reduction.
pub fn expneg(x: f64) -> Result<I, String> {
    if !x.is_finite() || !(0.0..=1000.).contains(&x) {
        return Err("expneg outside certified range".into());
    }
    if x == 0. {
        return i(1.);
    }
    let mut y = x;
    let mut m = 0;
    while y > 0.25 {
        y *= 0.5;
        m += 1;
    }
    let yi = i(y)?;
    let mut term = I::one();
    let mut sum = I::one();
    for k in 1..=80 {
        term = term.mul(yi)?.div_positive(i(k as f64)?)?;
        sum = sum.add(term)?;
    }
    let next = term.mul(yi)?.div_positive(i(81.)?)?;
    let denom = I::one().sub(yi.div_positive(i(82.)?)?)?;
    let upper = sum.add(next.div_positive(denom)?)?;
    if sum.lo <= 0. {
        return Err("failed positive exponential lower".into());
    }
    // true exp(y) in [sum.lo,upper.hi], reciprocal reverses inequalities
    let mut ans = I::new(dn(1. / upper.hi)?, up(1. / sum.lo)?)?;
    for _ in 0..m {
        ans = ans.mul(ans)?.clamp_nonnegative();
    }
    Ok(ans)
}
/// Interval enclosure for exp(-a*h), where the product of exactly represented
/// binary64 a,h is checked via fma; otherwise an interval argument is required.
fn expneg_product(a: f64, h: f64) -> Result<I, String> {
    let v = a * h;
    if !v.is_finite() || a.mul_add(h, -v) != 0. {
        return Err("nonexact represented exponential exponent".into());
    }
    expneg(v)
}

/// Finite Taylor for I_r(c)=int_0^h s^r exp(±c*s)ds, when c*h<=60.
fn integral_series(r: usize, c: f64, h: f64, positive: bool) -> Result<I, String> {
    if !c.is_finite() || !h.is_finite() || c < 0. || h <= 0. || c * h > 60. {
        return Err("integral series outside scope".into());
    }
    let mut term = pow(i(h)?, r + 1)?.div_positive(i((r + 1) as f64)?)?;
    let mut sum = term;
    for k in 1..=220 {
        term = term
            .mul(i(c)?)?
            .mul(i(h)?)?
            .mul(i((r + k) as f64)?)?
            .div_positive(i(k as f64)?)?
            .div_positive(i((r + k + 1) as f64)?)?;
        sum = if positive || k % 2 == 0 {
            sum.add(term)?
        } else {
            sum.sub(term)?
        };
    }
    let mut next = term
        .mul(i(c)?)?
        .mul(i(h)?)?
        .mul(i((r + 221) as f64)?)?
        .div_positive(i(221.)?)?
        .div_positive(i((r + 222) as f64)?)?;
    next = next.div_positive(I::one().sub(i(c * h)?.div_positive(i(222.)?)?)?)?;
    let lower = if positive {
        sum.lo
    } else {
        dn(sum.lo - next.hi)?
    };
    I::new(lower, up(sum.hi + next.hi)?).map(|x| x.clamp_nonnegative())
}

/// Nonnegative real J_r=integral_0^h exp[-a(h-s)-b*s]*s^r ds.
/// Source parameters are exactly represented binary64, a/b nonnegative.
pub fn j_list(a: f64, b: f64, h: f64, top: usize) -> Result<Vec<I>, String> {
    if !(a.is_finite()
        && b.is_finite()
        && h.is_finite()
        && a >= 0.
        && b >= 0.
        && h > 0.
        && h <= 0.25
        && top <= 40)
    {
        return Err("invalid N4 source kernel parameters".into());
    }
    // Ensure subtraction source identity is exact for the represented rates.
    let d = (a - b).abs();
    let signed = a - b;
    let bb = signed - a;
    let diff_error = (a - (signed - bb)) + (-b - bb);
    if diff_error != 0. {
        return Err("represented rate difference not exact".into());
    }
    let mut out = Vec::with_capacity(top + 1);
    let d_h = d * h;
    if !d_h.is_finite() || d.mul_add(h, -d_h) != 0. {
        return Err("rate-difference times h is not exactly represented".into());
    }
    if b >= a {
        let factor = expneg_product(a, h)?;
        if d == 0. {
            for r in 0..=top {
                out.push(factor.mul(pow(i(h)?, r + 1)?.div_positive(i((r + 1) as f64)?)?)?);
            }
            return Ok(out);
        }
        if d * h <= 2. * (top as f64 + 2.) {
            for r in 0..=top {
                out.push(factor.mul(integral_series(r, d, h, false)?)?)
            }
            return Ok(out);
        }
        let ed = expneg_product(d, h)?;
        let mut prev = I::one().sub(ed)?.div_positive(i(d)?)?.clamp_nonnegative();
        out.push(factor.mul(prev)?);
        for r in 1..=top {
            prev = prev
                .mul(i(r as f64)?)?
                .sub(pow(i(h)?, r)?.mul(ed)?)?
                .div_positive(i(d)?)?
                .clamp_nonnegative();
            out.push(factor.mul(prev)?);
        }
        return Ok(out);
    }
    if d * h <= 2. * (top as f64 + 2.) {
        let ea = expneg_product(a, h)?;
        for r in 0..=top {
            out.push(ea.mul(integral_series(r, d, h, true)?)?);
        }
        return Ok(out);
    }
    let ea = expneg_product(a, h)?;
    let eb = expneg_product(b, h)?;
    let mut prev = eb.sub(ea)?.div_positive(i(d)?)?.clamp_nonnegative();
    out.push(prev);
    for r in 1..=top {
        prev = eb
            .mul(pow(i(h)?, r)?)?
            .sub(prev.mul(i(r as f64)?)?)?
            .div_positive(i(d)?)?
            .clamp_nonnegative();
        out.push(prev)
    }
    Ok(out)
}

/// Enclosure of exact-real kernel M_l. Input theta is the source's represented
/// frequency, NOT a freely caller-swappable different OdeProblem.
pub fn source_moment(a: f64, b: f64, theta: f64, h: f64, degree: usize) -> Result<Z, String> {
    if !theta.is_finite() || degree > 9 || !h.is_finite() || h <= 0.0 {
        return Err("uncertified oscillatory source".into());
    }
    let phase_product = i(theta.abs())?.mul(i(h)?)?;
    if phase_product.hi > 0.25 {
        return Err("oscillatory phase outside proven source box".into());
    }
    let p = 18;
    if theta == 0. {
        let j = j_list(a, b, h, degree)?;
        return Ok(Z {
            re: j[degree],
            im: I::zero(),
        });
    }
    let j = j_list(a, b, h, degree + p + 1)?;
    let mut inner = Z::zero();
    let mut coef = Z::one();
    let mut phase = Z::zero();
    let mut phase_coef = Z::one();
    let ith = Z {
        re: I::zero(),
        im: i(theta)?.mul(i(h)?)?,
    };
    let itheta = Z {
        re: I::zero(),
        im: i(-theta)?,
    };
    for q in 0..=p {
        inner = inner.add(coef.mul(Z {
            re: j[degree + q],
            im: I::zero(),
        })?)?;
        phase = phase.add(phase_coef)?;
        coef = coef.mul(itheta)?.div_positive(i((q + 1) as f64)?)?;
        phase_coef = phase_coef.mul(ith)?.div_positive(i((q + 1) as f64)?)?;
    }
    let tail = pow(i(theta.abs())?, p + 1)?
        .div_positive(fact(p + 1)?)?
        .mul(j[degree + p + 1])?;
    let phase_tail = pow(i(theta.abs())?.mul(i(h)?)?, p + 1)?.div_positive(fact(p + 1)?)?;
    phase.pad(phase_tail)?.mul(inner.pad(tail)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_zero_oscillation_symmetry() {
        for (a, b) in [(0.25, 0.75), (1000., 0.75), (0.25, 3000.), (1000., 1000.5)] {
            let z = source_moment(a, b, 0., 0.25, 0).unwrap();
            assert!(z.re.lo >= 0. && z.im.lo <= 0. && z.im.hi >= 0.);
        }
    }
    #[test]
    fn domain_refusal() {
        assert!(source_moment(-1., 1., 0., 0.25, 0).is_err());
        assert!(source_moment(1., 2., 2., 0.25, 0).is_err());
        assert!(source_moment(1., 2., 0., 0.5, 0).is_err());
    }
    #[test]
    fn subnormal_directed() {
        let tiny = f64::from_bits(1);
        assert!(
            I::point(tiny)
                .unwrap()
                .mul(I::point(tiny).unwrap())
                .unwrap()
                .hi
                >= tiny
        );
    }
    #[test]
    fn exp_decay_enclosed() {
        let e = expneg(0.25).unwrap();
        assert!(e.lo < 0.25f64.exp().recip() && 0.25f64.exp().recip() < e.hi);
    }
}

#[allow(dead_code)]
#[path = "n3h_source_bounded_copy.rs"]
pub mod n3h_source_bounded_copy;
