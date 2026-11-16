//! Finite binary64 enclosures. Every primitive rounds its result out by one
//! representable number; transcendental functions use positive series and
//! explicit geometric remainder bounds, never an assumed libm error bound.

#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct IntervalError;

pub type Result<T> = std::result::Result<T, IntervalError>;

fn down(x: f64) -> f64 {
    if x == 0.0 {
        return -f64::from_bits(1);
    }
    if x > 0.0 {
        f64::from_bits(x.to_bits() - 1)
    } else {
        f64::from_bits(x.to_bits() + 1)
    }
}

fn up(x: f64) -> f64 {
    if x == 0.0 {
        return f64::from_bits(1);
    }
    if x > 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else {
        f64::from_bits(x.to_bits() - 1)
    }
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Result<Self> {
        if lo.is_finite() && hi.is_finite() && lo <= hi {
            Ok(Self { lo, hi })
        } else {
            Err(IntervalError)
        }
    }
    pub fn point(x: f64) -> Result<Self> {
        Self::new(x, x)
    }
    pub(crate) fn valid(&self) -> Result<()> {
        Self::new(self.lo, self.hi).map(|_| ())
    }
    fn rounded(lo: f64, hi: f64) -> Result<Self> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError);
        }
        Self::new(down(lo), up(hi))
    }
    pub fn add(&self, rhs: &Self) -> Result<Self> {
        self.valid()?;
        rhs.valid()?;
        Self::rounded(self.lo + rhs.lo, self.hi + rhs.hi)
    }
    pub fn sub(&self, rhs: &Self) -> Result<Self> {
        self.valid()?;
        rhs.valid()?;
        Self::rounded(self.lo - rhs.hi, self.hi - rhs.lo)
    }
    pub fn neg(&self) -> Result<Self> {
        self.valid()?;
        Self::new(-self.hi, -self.lo)
    }
    pub fn mul(&self, rhs: &Self) -> Result<Self> {
        self.valid()?;
        rhs.valid()?;
        let p = [
            self.lo * rhs.lo,
            self.lo * rhs.hi,
            self.hi * rhs.lo,
            self.hi * rhs.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Err(IntervalError);
        }
        Self::rounded(
            p.iter().copied().fold(f64::INFINITY, f64::min),
            p.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    pub fn div(&self, rhs: &Self) -> Result<Self> {
        self.valid()?;
        rhs.valid()?;
        if rhs.lo <= 0.0 && rhs.hi >= 0.0 {
            return Err(IntervalError);
        }
        let q = [
            self.lo / rhs.lo,
            self.lo / rhs.hi,
            self.hi / rhs.lo,
            self.hi / rhs.hi,
        ];
        if q.iter().any(|x| !x.is_finite()) {
            return Err(IntervalError);
        }
        Self::rounded(
            q.iter().copied().fold(f64::INFINITY, f64::min),
            q.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    pub fn exp(&self) -> Result<Self> {
        self.valid()?;
        Self::new(exp_scalar(self.lo)?.lo, exp_scalar(self.hi)?.hi)
    }
    pub fn ln(&self) -> Result<Self> {
        self.valid()?;
        if self.lo <= 0.0 {
            return Err(IntervalError);
        }
        Self::new(ln_scalar(self.lo)?.lo, ln_scalar(self.hi)?.hi)
    }
    pub fn powf(&self, exponent: f64) -> Result<Self> {
        self.valid()?;
        if self.lo <= 0.0 || !exponent.is_finite() {
            return Err(IntervalError);
        }
        if exponent == 0.0 {
            return Self::point(1.0);
        }
        if exponent == 1.0 {
            return Ok(*self);
        }
        self.ln()?.mul(&Self::point(exponent)?)?.exp()
    }
}

fn exp_scalar(x: f64) -> Result<Interval> {
    if x == 0.0 {
        return Interval::point(1.0);
    }
    if x < 0.0 {
        // Halve before inversion so exp(large negative x) can underflow
        // outward to [0, min_subnormal] without first overflowing exp(-x).
        let mut magnitude = -x;
        let mut squarings = 0;
        while magnitude > 700.0 {
            magnitude *= 0.5;
            squarings += 1;
        }
        let positive = exp_scalar(magnitude)?;
        let mut result = Interval::point(1.0)?.div(&positive)?;
        for _ in 0..squarings {
            result = result.mul(&result)?;
            result.lo = result.lo.max(0.0);
        }
        return Ok(result);
    }
    let mut y = x;
    let mut squarings = 0;
    while y > 0.5 {
        y *= 0.5;
        squarings += 1;
    }
    let iy = Interval::point(y)?;
    let mut term = Interval::point(1.0)?;
    let mut sum = term;
    // For 0 <= y <= 1/2, terms are positive and the omitted tail after
    // term 40 is <= 2 * term_41: each later ratio is at most 1/2.
    for n in 1..=40 {
        term = term.mul(&iy)?.div(&Interval::point(n as f64)?)?;
        sum = sum.add(&term)?;
    }
    let next = term.mul(&iy)?.div(&Interval::point(41.0)?)?;
    let tail = next.mul(&Interval::point(2.0)?)?;
    let mut result = Interval::new(sum.lo, sum.add(&tail)?.hi)?;
    for _ in 0..squarings {
        result = result.mul(&result)?;
    }
    Ok(result)
}

fn atanh_twice(z: Interval) -> Result<Interval> {
    z.valid()?;
    if z.lo < 0.0 || z.hi > 0.5 {
        return Err(IntervalError);
    }
    let z2 = z.mul(&z)?;
    let mut power = z;
    let mut sum = Interval::point(0.0)?;
    // 2 atanh(z) = 2 sum z^(2n+1)/(2n+1). For z <= 1/2,
    // the remaining sum is <= 2*next_term/(1-z^2) <= 4*next_term.
    for n in 0..40 {
        let term = power.div(&Interval::point((2 * n + 1) as f64)?)?;
        sum = sum.add(&term)?;
        power = power.mul(&z2)?;
    }
    let next = power.div(&Interval::point(81.0)?)?;
    let twice = sum.mul(&Interval::point(2.0)?)?;
    let tail = next.mul(&Interval::point(4.0)?)?;
    Interval::new(twice.lo, twice.add(&tail)?.hi)
}

fn ln_scalar(x: f64) -> Result<Interval> {
    if x == 1.0 {
        return Interval::point(0.0);
    }
    // Exact binary decomposition x = m*2^e, including subnormals. Scaling
    // a subnormal by 2^54 is exact and makes its exponent field nonzero.
    let (normal, correction) = if x < f64::MIN_POSITIVE {
        (x * (2.0_f64).powi(54), -54)
    } else {
        (x, 0)
    };
    let bits = normal.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i32 - 1023 + correction;
    let m = f64::from_bits((bits & ((1_u64 << 52) - 1)) | (1023_u64 << 52));
    let ln2 = atanh_twice(Interval::point(1.0)?.div(&Interval::point(3.0)?)?)?;
    let ln_m = if m == 1.0 {
        Interval::point(0.0)?
    } else {
        let z = Interval::point(m)?
            .sub(&Interval::point(1.0)?)?
            .div(&Interval::point(m)?.add(&Interval::point(1.0)?)?)?;
        atanh_twice(z)?
    };
    ln_m.add(&ln2.mul(&Interval::point(e as f64)?)?)
}
