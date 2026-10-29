use super::{DerivativeMode, Model, Result, Work, finite};
/// Outward binary64 arithmetic: IEEE round-to-nearest, gradual underflow,
/// no fast-math/FTZ. Overflow refuses; public malformed operands are checked.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "native", derive(serde::Serialize))]
pub struct Iv {
    pub lo: f64,
    pub hi: f64,
}
impl Iv {
    pub fn new(lo: f64, hi: f64) -> Result<Self> {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            Err("INVALID_INTERVAL".into())
        } else {
            Ok(Self { lo, hi })
        }
    }
    pub fn point(x: f64) -> Result<Self> {
        Self::new(x, x)
    }
    pub fn zero() -> Self {
        Self { lo: 0.0, hi: 0.0 }
    }
    fn valid(self) -> Result<()> {
        Self::new(self.lo, self.hi).map(|_| ())
    }
    fn is_zero(self) -> bool {
        self.lo == 0.0 && self.hi == 0.0
    }
    pub fn add(self, b: Self) -> Result<Self> {
        self.valid()?;
        b.valid()?;
        if b.is_zero() {
            return Ok(self);
        }
        if self.is_zero() {
            return Ok(b);
        }
        Self::new((self.lo + b.lo).next_down(), (self.hi + b.hi).next_up())
    }
    pub fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    pub fn sub(self, b: Self) -> Result<Self> {
        self.valid()?;
        b.valid()?;
        if self.lo == self.hi && b.lo == b.hi && self.lo == b.lo {
            return Ok(Self::zero());
        }
        self.add(b.neg())
    }
    pub fn mul(self, b: Self) -> Result<Self> {
        self.valid()?;
        b.valid()?;
        if self.is_zero() || b.is_zero() {
            return Ok(Self::zero());
        }
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        finite(&v)?;
        Self::new(
            v.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            v.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    pub fn div(self, b: Self) -> Result<Self> {
        self.valid()?;
        b.valid()?;
        if b.lo <= 0.0 && b.hi >= 0.0 {
            return Err("INTERVAL_DIVISION_BY_ZERO".into());
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        self.mul(Self::new((1.0 / b.hi).next_down(), (1.0 / b.lo).next_up())?)
    }
    pub fn scale(self, a: f64) -> Result<Self> {
        self.mul(Self::point(a)?)
    }
    pub fn mag(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
    pub fn contains(self, x: f64) -> bool {
        x >= self.lo && x <= self.hi
    }
}
fn binomial(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut c = 1u64;
    for j in 0..k {
        c = c * (n - j) as u64 / (j + 1) as u64;
    }
    c
}
fn fraction(n: u64, d: u64) -> Result<Iv> {
    if n == 0 {
        return Ok(Iv::zero());
    }
    Iv::point(n as f64)?.div(Iv::point(d as f64)?)
}
fn elevate(p: &[Iv], degree: usize) -> Result<Vec<Iv>> {
    let n = p.len() - 1;
    if degree < n {
        return Err("DEGREE_REDUCTION".into());
    }
    if degree == n {
        return Ok(p.to_vec());
    }
    let mut b = vec![Iv::zero(); degree + 1];
    for (k, out) in b.iter_mut().enumerate() {
        for j in k.saturating_sub(degree - n)..=k.min(n) {
            let w = fraction(
                binomial(n, j) * binomial(degree - n, k - j),
                binomial(degree, k),
            )?;
            *out = out.add(w.mul(p[j])?)?;
        }
    }
    Ok(b)
}
fn product(a: &[Iv], b: &[Iv]) -> Result<Vec<Iv>> {
    let n = a.len() - 1;
    let m = b.len() - 1;
    let mut c = vec![Iv::zero(); n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            let w = fraction(binomial(n, i) * binomial(m, j), binomial(n + m, i + j))?;
            c[i + j] = c[i + j].add(w.mul(a[i].mul(b[j])?)?)?;
        }
    }
    Ok(c)
}
fn exp_negative(x: Iv) -> Result<Iv> {
    if x.lo < 0.0 {
        return Err("NEGATIVE_DECAY_INTERVAL".into());
    }
    if x.is_zero() {
        return Iv::point(1.0);
    }
    let mut y = x;
    let mut scale = 0;
    while y.hi > 0.5 {
        y = y.scale(0.5)?;
        scale += 1;
        if scale > 1024 {
            return Err("EXP_RANGE".into());
        }
    }
    let mut term = Iv::point(1.0)?;
    let mut sum = term;
    for k in 1..=18 {
        term = term.mul(y.neg())?.div(Iv::point(k as f64)?)?;
        sum = sum.add(term)?;
    }
    let next = term.mul(y)?.div(Iv::point(19.0)?)?.mag();
    sum = sum.add(Iv::new(-next, next)?)?;
    sum = Iv::new(sum.lo.max(0.0), sum.hi.min(1.0))?;
    for _ in 0..scale {
        sum = sum.mul(sum)?;
        sum = Iv::new(sum.lo.max(0.0), sum.hi.min(1.0))?;
    }
    Ok(sum)
}
fn flow_radius(initial: f64, defect: f64, decay: f64, dt: Iv) -> Result<f64> {
    if !initial.is_finite() || initial < 0.0 || !defect.is_finite() || defect < 0.0 || dt.lo < 0.0 {
        return Err("INVALID_ERROR_RADIUS".into());
    }
    if decay == 0.0 {
        return Ok(Iv::point(initial)?.add(dt.scale(defect)?)?.hi);
    }
    let e = exp_negative(dt.scale(decay)?)?;
    let one = Iv::point(1.0)?;
    let v = e.scale(initial)?.add(
        one.sub(e)?
            .mul(Iv::point(defect)?.div(Iv::point(decay)?)?)?,
    )?;
    Ok(v.hi.max(0.0))
}
pub fn quintic_controls(
    y0: f64,
    y1: f64,
    f0: f64,
    f1: f64,
    a0: f64,
    a1: f64,
    h: f64,
) -> Result<[f64; 6]> {
    finite(&[y0, y1, f0, f1, a0, a1, h])?;
    if h <= 0.0 {
        return Err("INVALID_CURVE_STEP".into());
    }
    let b1 = y0 + h * f0 / 5.0;
    let b4 = y1 - h * f1 / 5.0;
    let b = [
        y0,
        b1,
        2.0 * b1 - y0 + h * h * a0 / 20.0,
        2.0 * b4 - y1 + h * h * a1 / 20.0,
        b4,
        y1,
    ];
    finite(&b)?;
    Ok(b)
}
/// Actual represented polynomial; private controls and model identity.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "native", derive(serde::Serialize))]
pub struct Curve {
    owner: [u64; 5],
    t0: f64,
    t1: f64,
    controls: Vec<Vec<f64>>,
}
impl Curve {
    pub fn from_controls(model: &Model, t0: f64, t1: f64, controls: Vec<Vec<f64>>) -> Result<Self> {
        if !t0.is_finite() || !t1.is_finite() || t1 <= t0 || !(t1 - t0).is_normal() {
            return Err("CURVE_CLOCK".into());
        }
        if controls.len() != model.n() || controls.is_empty() {
            return Err("CURVE_SHAPE".into());
        }
        let p = controls[0].len();
        if !(2..=6).contains(&p) || controls.iter().any(|b| b.len() != p) {
            return Err("CURVE_DEGREE".into());
        }
        for b in &controls {
            finite(b)?;
        }
        Ok(Self {
            owner: model.identity(),
            t0,
            t1,
            controls,
        })
    }
    pub fn hermite(
        model: &Model,
        t0: f64,
        t1: f64,
        y0: &[f64],
        y1: &[f64],
        quintic: bool,
        mode: DerivativeMode,
        work: &mut Work,
    ) -> Result<Self> {
        model.check(y0)?;
        model.check(y1)?;
        work.curve_builds += 1;
        let n = model.n();
        let h = t1 - t0;
        let mut f0 = vec![0.0; n];
        let mut f1 = vec![0.0; n];
        model.rhs_into(y0, &mut f0, work)?;
        model.rhs_into(y1, &mut f1, work)?;
        let mut rows = Vec::with_capacity(n);
        if quintic {
            let mut a0 = vec![0.0; n];
            let mut a1 = vec![0.0; n];
            model.acceleration_into(y0, &f0, &mut a0, mode, work)?;
            model.acceleration_into(y1, &f1, &mut a1, mode, work)?;
            for i in 0..n {
                rows.push(quintic_controls(y0[i], y1[i], f0[i], f1[i], a0[i], a1[i], h)?.to_vec());
            }
        } else {
            for i in 0..n {
                rows.push(vec![
                    y0[i],
                    y0[i] + h * f0[i] / 3.0,
                    y1[i] - h * f1[i] / 3.0,
                    y1[i],
                ]);
            }
        }
        Self::from_controls(model, t0, t1, rows)
    }
    pub fn start(&self) -> f64 {
        self.t0
    }
    pub fn end(&self) -> f64 {
        self.t1
    }
    pub fn controls(&self) -> &[Vec<f64>] {
        &self.controls
    }
    pub fn endpoint(&self) -> Vec<f64> {
        self.controls.iter().map(|b| *b.last().unwrap()).collect()
    }
    pub fn initial(&self) -> Vec<f64> {
        self.controls.iter().map(|b| b[0]).collect()
    }
    fn duration(&self) -> Result<Iv> {
        let h = Iv::point(self.t1)?.sub(Iv::point(self.t0)?)?;
        if h.lo <= 0.0 {
            Err("UNRESOLVED_POSITIVE_CLOCK".into())
        } else {
            Ok(h)
        }
    }
    fn at(&self, t: f64) -> Result<(Vec<f64>, f64)> {
        if !t.is_finite() || t < self.t0 || t > self.t1 {
            return Err("OUTPUT_OUTSIDE_CURVE".into());
        }
        if t == self.t0 {
            return Ok((self.initial(), 0.0));
        }
        if t == self.t1 {
            return Ok((self.endpoint(), 0.0));
        }
        let theta = ((t - self.t0) / (self.t1 - self.t0)).clamp(0.0, 1.0);
        let th = Iv::point(t)?
            .sub(Iv::point(self.t0)?)?
            .div(self.duration()?)?;
        let th = Iv::new(th.lo.max(0.0), th.hi.min(1.0))?;
        let om = Iv::point(1.0)?.sub(th)?;
        let mut values = Vec::with_capacity(self.controls.len());
        let mut displacement = 0.0_f64;
        for b in &self.controls {
            let mut p = b.clone();
            let mut r: Vec<Iv> = b.iter().map(|&v| Iv::point(v)).collect::<Result<_>>()?;
            for degree in (1..b.len()).rev() {
                for k in 0..degree {
                    p[k] = (1.0 - theta) * p[k] + theta * p[k + 1];
                    r[k] = om.mul(r[k])?.add(th.mul(r[k + 1])?)?;
                }
            }
            finite(&[p[0]])?;
            let diff = r[0].sub(Iv::point(p[0])?)?.mag();
            displacement = displacement.max(diff);
            values.push(p[0]);
        }
        Ok((values, displacement))
    }
}
/// Generated-model continuous error proof. Initial radius and true initial
/// invariant-box membership are caller premises; integrate checks both.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "native", derive(serde::Serialize))]
pub struct Segment {
    curve: Curve,
    initial_radius: f64,
    defect_upper: f64,
    decay_lower: f64,
    endpoint_radius: f64,
    window_radius: f64,
    residual_coefficients: Vec<Vec<Iv>>,
}
impl Segment {
    pub fn verify(
        model: &Model,
        curve: Curve,
        initial_radius: f64,
        work: &mut Work,
    ) -> Result<Self> {
        work.certificate_calls += 1;
        if model.identity() != curve.owner {
            return Err("CERTIFICATE_MODEL_MISMATCH".into());
        }
        if !initial_radius.is_finite() || initial_radius < 0.0 {
            return Err("INVALID_INITIAL_RADIUS".into());
        }
        if curve
            .controls
            .iter()
            .flatten()
            .any(|x| *x < 0.0 || *x > 1.0)
        {
            return Err("CURVE_INVARIANT_BOX_UNPROVEN".into());
        }
        let p = curve.controls[0].len() - 1;
        let total = 2 * p;
        let n = model.n();
        let dt = curve.duration()?;
        let [d, lambda, alpha, epsilon] = model.params();
        let mut mean = vec![Iv::zero(); p + 1];
        for b in &curve.controls {
            for k in 0..=p {
                mean[k] = mean[k].add(Iv::point(b[k])?)?;
            }
        }
        for z in &mut mean {
            *z = z.div(Iv::point(n as f64)?)?;
        }
        let elevated_mean = elevate(&mean, total)?;
        let mut allres = Vec::with_capacity(n);
        let mut defect = 0.0_f64;
        for i in 0..n {
            work.certificate_components += 1;
            let b: Vec<_> = curve.controls[i]
                .iter()
                .map(|&x| Iv::point(x))
                .collect::<Result<_>>()?;
            let derivative: Vec<_> = (0..p)
                .map(|k| b[k + 1].sub(b[k])?.scale(p as f64)?.div(dt))
                .collect::<Result<_>>()?;
            let derivative = elevate(&derivative, total)?;
            let mut linear = vec![Iv::zero(); p + 1];
            for k in 0..=p {
                let mut v = b[k].scale(-lambda)?;
                if i > 0 {
                    v = v.add(Iv::point(curve.controls[i - 1][k])?.sub(b[k])?.scale(d)?)?;
                }
                if i + 1 < n {
                    v = v.add(Iv::point(curve.controls[i + 1][k])?.sub(b[k])?.scale(d)?)?;
                }
                linear[k] = v;
            }
            let linear = elevate(&linear, total)?;
            let mq = product(&mean, &b)?;
            let qq = product(&b, &b)?;
            let mut residual = vec![Iv::zero(); total + 1];
            for k in 0..=total {
                let rhs = linear[k]
                    .add(elevated_mean[k].sub(mq[k])?.scale(alpha)?)?
                    .sub(qq[k].scale(epsilon)?)?;
                residual[k] = derivative[k].sub(rhs)?;
                defect = defect.max(residual[k].mag());
            }
            allres.push(residual);
        }
        let decay = Iv::point(lambda)?.sub(Iv::point(alpha)?)?.lo.max(0.0);
        let endpoint = flow_radius(initial_radius, defect, decay, dt)?;
        let window = initial_radius.max(endpoint);
        Ok(Self {
            curve,
            initial_radius,
            defect_upper: defect,
            decay_lower: decay,
            endpoint_radius: endpoint,
            window_radius: window,
            residual_coefficients: allres,
        })
    }
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn defect(&self) -> f64 {
        self.defect_upper
    }
    pub fn endpoint_radius(&self) -> f64 {
        self.endpoint_radius
    }
    pub fn window_radius(&self) -> f64 {
        self.window_radius
    }
    pub fn evaluate(&self, t: f64, work: &mut Work) -> Result<(Vec<f64>, f64)> {
        work.output_queries += 1;
        let (v, round) = self.curve.at(t)?;
        let flow = if t == self.curve.start() {
            self.initial_radius
        } else if t == self.curve.end() {
            self.endpoint_radius
        } else {
            flow_radius(
                self.initial_radius,
                self.defect_upper,
                self.decay_lower,
                Iv::point(t)?.sub(Iv::point(self.curve.start())?)?,
            )?
        };
        let radius = Iv::point(flow)?.add(Iv::point(round)?)?.hi;
        Ok((v, radius))
    }
}
