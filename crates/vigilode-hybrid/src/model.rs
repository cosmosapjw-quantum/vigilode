use super::{Result, finite};
use std::ops::{Add, Mul, Sub};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "native", derive(serde::Serialize))]
pub enum DerivativeMode {
    LocalAd,
    FullAd,
    Algebraic,
}
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "native", derive(serde::Serialize))]
pub struct Work {
    pub rhs_vectors: u64,
    pub jvp_vectors: u64,
    pub physical_vectors: u64,
    pub local_dual_cells: u64,
    pub full_dual_cells: u64,
    pub algebraic_cells: u64,
    pub packet_calls: u64,
    pub fft_calls: u64,
    pub curve_builds: u64,
    pub band_factorizations: u64,
    pub band_solves: u64,
    pub rank_resolvent_solves: u64,
    pub partial_ad_cells: u64,
    pub certificate_calls: u64,
    pub certificate_components: u64,
    pub output_queries: u64,
    pub candidate_calls: u64,
    pub candidate_failures: u64,
}
impl Work {
    pub fn accumulate(&mut self, w: &Self) {
        macro_rules! a{($($f:ident),*)=>{$(self.$f=self.$f.saturating_add(w.$f);)*}}
        a!(
            rhs_vectors,
            jvp_vectors,
            physical_vectors,
            local_dual_cells,
            full_dual_cells,
            algebraic_cells,
            packet_calls,
            fft_calls,
            curve_builds,
            band_factorizations,
            band_solves,
            rank_resolvent_solves,
            partial_ad_cells,
            certificate_calls,
            certificate_components,
            output_queries,
            candidate_calls,
            candidate_failures
        );
    }
}
/// Immutable generated reflecting-path model. d is the actual edge rate nu/dx^2.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "native", derive(serde::Serialize))]
pub struct Model {
    n: usize,
    d: f64,
    lambda: f64,
    alpha: f64,
    epsilon: f64,
}
impl Model {
    pub fn new(n: usize, d: f64, lambda: f64, alpha: f64, epsilon: f64) -> Result<Self> {
        if !(2..=65536).contains(&n)
            || ![d, lambda, alpha, epsilon]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0)
            || lambda < alpha
        {
            return Err("INVALID_GENERATED_MODEL".into());
        }
        if !(4.0 * d + lambda + 2.0 * alpha + 2.0 * epsilon).is_finite() {
            return Err("MODEL_RATE_OVERFLOW".into());
        }
        Ok(Self {
            n,
            d,
            lambda,
            alpha,
            epsilon,
        })
    }
    pub fn n(&self) -> usize {
        self.n
    }
    pub fn params(&self) -> [f64; 4] {
        [self.d, self.lambda, self.alpha, self.epsilon]
    }
    pub fn identity(&self) -> [u64; 5] {
        [
            self.n as u64,
            self.d.to_bits(),
            self.lambda.to_bits(),
            self.alpha.to_bits(),
            self.epsilon.to_bits(),
        ]
    }
    pub fn admits_packet(&self) -> bool {
        self.epsilon == 0.0 && self.n.is_power_of_two()
    }
    pub fn check(&self, y: &[f64]) -> Result<()> {
        if y.len() != self.n {
            return Err("STATE_SHAPE".into());
        }
        finite(y)
    }
    pub fn initial_box(&self, y: &[f64]) -> Result<()> {
        self.check(y)?;
        if y.iter().any(|x| *x < 0.0 || *x > 1.0) {
            Err("OUTSIDE_INVARIANT_BOX".into())
        } else {
            Ok(())
        }
    }
    pub fn mean(&self, y: &[f64]) -> Result<f64> {
        self.check(y)?;
        let m = y.iter().sum::<f64>() / self.n as f64;
        if m.is_finite() {
            Ok(m)
        } else {
            Err("MEAN_OVERFLOW".into())
        }
    }
    pub fn physical_into(&self, v: &[f64], out: &mut [f64], work: &mut Work) -> Result<()> {
        self.check(v)?;
        if out.len() != self.n {
            return Err("OUTPUT_SHAPE".into());
        }
        work.physical_vectors += 1;
        for i in 0..self.n {
            let mut a = -self.lambda * v[i];
            if i > 0 {
                a += self.d * (v[i - 1] - v[i]);
            }
            if i + 1 < self.n {
                a += self.d * (v[i + 1] - v[i]);
            }
            out[i] = a;
        }
        finite(out)
    }
    pub fn rhs_into(&self, y: &[f64], out: &mut [f64], work: &mut Work) -> Result<()> {
        self.check(y)?;
        if out.len() != self.n {
            return Err("OUTPUT_SHAPE".into());
        }
        work.rhs_vectors += 1;
        let m = self.mean(y)?;
        self.physical_into(y, out, work)?;
        for i in 0..self.n {
            out[i] += nonlinear(y[i], m, self.alpha, self.epsilon);
        }
        finite(out)
    }
    pub fn rhs(&self, y: &[f64]) -> Result<Vec<f64>> {
        let mut v = vec![0.0; self.n];
        self.rhs_into(y, &mut v, &mut Work::default())?;
        Ok(v)
    }
    pub fn jvp_into(
        &self,
        y: &[f64],
        v: &[f64],
        out: &mut [f64],
        mode: DerivativeMode,
        work: &mut Work,
    ) -> Result<()> {
        self.check(y)?;
        self.check(v)?;
        if out.len() != self.n {
            return Err("OUTPUT_SHAPE".into());
        }
        work.jvp_vectors += 1;
        let m = self.mean(y)?;
        let mv = self.mean(v)?;
        match mode {
            DerivativeMode::FullAd => {
                let yy: Vec<_> = y.iter().zip(v).map(|(&a, &b)| Dual(a, b)).collect();
                let mm = Dual(m, mv);
                work.full_dual_cells += self.n as u64;
                for i in 0..self.n {
                    let mut z = Dual::from(-self.lambda) * yy[i];
                    if i > 0 {
                        z = z + Dual::from(self.d) * (yy[i - 1] - yy[i]);
                    }
                    if i + 1 < self.n {
                        z = z + Dual::from(self.d) * (yy[i + 1] - yy[i]);
                    }
                    z = z + nonlinear(yy[i], mm, Dual::from(self.alpha), Dual::from(self.epsilon));
                    out[i] = z.1;
                }
            }
            DerivativeMode::LocalAd => {
                self.physical_into(v, out, work)?;
                work.local_dual_cells += self.n as u64;
                for i in 0..self.n {
                    out[i] += nonlinear(
                        Dual(y[i], v[i]),
                        Dual(m, mv),
                        Dual::from(self.alpha),
                        Dual::from(self.epsilon),
                    )
                    .1;
                }
            }
            DerivativeMode::Algebraic => {
                self.physical_into(v, out, work)?;
                work.algebraic_cells += self.n as u64;
                for i in 0..self.n {
                    out[i] += self.alpha * (mv * (1.0 - y[i]) - m * v[i])
                        - 2.0 * self.epsilon * y[i] * v[i];
                }
            }
        }
        finite(out)
    }
    pub fn jvp(&self, y: &[f64], v: &[f64]) -> Result<Vec<f64>> {
        let mut z = vec![0.0; self.n];
        self.jvp_into(y, v, &mut z, DerivativeMode::LocalAd, &mut Work::default())?;
        Ok(z)
    }
    /// J(y) f(y) for Hermite acceleration. Algebraic mode uses a known polynomial
    /// identity, but the action is still counted; only AD-oracle use disappears.
    pub fn acceleration_into(
        &self,
        y: &[f64],
        f: &[f64],
        out: &mut [f64],
        mode: DerivativeMode,
        work: &mut Work,
    ) -> Result<()> {
        self.jvp_into(y, f, out, mode, work)
    }
    /// Exact shift structure without a global state Jacobian. Local dual AD only
    /// differentiates the reaction graph with respect to q_i and the mean.
    pub fn shift_factor(&self, y: &[f64], shift: f64, work: &mut Work) -> Result<ShiftFactor> {
        self.initial_box(y)?;
        if !shift.is_finite() || shift <= 0.0 {
            return Err("INVALID_SHIFT".into());
        }
        let n = self.n;
        let mean = self.mean(y)?;
        let edge = -shift * self.d;
        let mut diag = vec![0.0; n];
        let mut u = vec![0.0; n];
        for i in 0..n {
            let local = nonlinear(
                Dual(y[i], 1.0),
                Dual(mean, 0.0),
                Dual(self.alpha, 0.0),
                Dual(self.epsilon, 0.0),
            )
            .1;
            let border = nonlinear(
                Dual(y[i], 0.0),
                Dual(mean, 1.0),
                Dual(self.alpha, 0.0),
                Dual(self.epsilon, 0.0),
            )
            .1;
            work.partial_ad_cells += 2;
            let neighbors = if i == 0 || i + 1 == n { 1.0 } else { 2.0 };
            diag[i] = 1.0 + shift * (self.lambda + neighbors * self.d - local);
            u[i] = shift * border;
        }
        finite(&diag)?;
        finite(&u)?;
        finite(&[edge])?;
        work.band_factorizations += 1;
        let mut multipliers = vec![0.0; n];
        for i in 1..n {
            if diag[i - 1] <= 0.0 {
                return Err("NONPOSITIVE_BAND_PIVOT".into());
            }
            multipliers[i] = edge / diag[i - 1];
            diag[i] -= multipliers[i] * edge;
        }
        if diag.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Err("INVALID_BAND_FACTOR".into());
        }
        let mut factor = ShiftFactor {
            diag,
            multipliers,
            edge,
            image: Vec::new(),
            denominator: 1.0,
        };
        let image = factor.triangular(&u, work)?;
        let denominator = 1.0 - image.iter().sum::<f64>() / n as f64;
        if !denominator.is_finite() || denominator <= 64.0 * f64::EPSILON {
            return Err("RANK_ONE_DENOMINATOR_UNRESOLVED".into());
        }
        factor.image = image;
        factor.denominator = denominator;
        Ok(factor)
    }
    pub fn packet(&self, y: &[f64], h: f64) -> Result<Vec<f64>> {
        self.packet_counted(y, h, &mut Work::default())
    }
    /// Exact-model collective/transverse construction in floating arithmetic.
    /// Candidate only: FFT/exp roundoff is checked through the actual curve defect.
    pub fn packet_counted(&self, y: &[f64], h: f64, work: &mut Work) -> Result<Vec<f64>> {
        self.initial_box(y)?;
        if !self.admits_packet() {
            return Err("EXACT_QUOTIENT_NOT_ADMITTED".into());
        }
        if !h.is_finite() || h < 0.0 {
            return Err("INVALID_PACKET_TIME".into());
        }
        if h == 0.0 {
            return Ok(y.to_vec());
        }
        work.packet_calls += 1;
        let mean = self.mean(y)?;
        let beta = self.alpha - self.lambda;
        let integral = if beta == 0.0 {
            h
        } else {
            (beta * h).exp_m1() / beta
        };
        let den = 1.0 + self.alpha * mean * integral;
        if !den.is_finite() || den <= 0.0 {
            return Err("PACKET_DENOMINATOR".into());
        }
        let next_mean = mean * (beta * h).exp() / den;
        let scalar = (-self.lambda * h).exp() / den;
        let n2 = self.n * 2;
        let mut spectrum = vec![(0.0, 0.0); n2];
        for i in 0..self.n {
            spectrum[i].0 = y[i] - mean;
            spectrum[n2 - 1 - i].0 = y[i] - mean;
        }
        fft(&mut spectrum, false);
        work.fft_calls += 1;
        for (k, z) in spectrum.iter_mut().enumerate() {
            let s = (std::f64::consts::PI * k as f64 / n2 as f64).sin();
            let factor = (-4.0 * self.d * s * s * h).exp() * scalar;
            z.0 *= factor;
            z.1 *= factor;
        }
        fft(&mut spectrum, true);
        work.fft_calls += 1;
        let result: Vec<_> = spectrum[..self.n].iter().map(|z| next_mean + z.0).collect();
        finite(&result)?;
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug)]
struct Dual(f64, f64);
impl From<f64> for Dual {
    fn from(v: f64) -> Self {
        Self(v, 0.0)
    }
}
impl Add for Dual {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1)
    }
}
impl Sub for Dual {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self(self.0 - b.0, self.1 - b.1)
    }
}
impl Mul for Dual {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self(self.0 * b.0, self.1 * b.0 + self.0 * b.1)
    }
}
fn nonlinear<T: Copy + From<f64> + Add<Output = T> + Sub<Output = T> + Mul<Output = T>>(
    q: T,
    m: T,
    alpha: T,
    epsilon: T,
) -> T {
    alpha * m * (T::from(1.0) - q) - epsilon * q * q
}
fn fft(v: &mut [(f64, f64)], inverse: bool) {
    let n = v.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            v.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = (if inverse { 2.0 } else { -2.0 }) * std::f64::consts::PI / len as f64;
        let wlen = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let mut w = (1.0, 0.0);
            for k in 0..len / 2 {
                let a = v[start + k];
                let b = v[start + k + len / 2];
                let z = (b.0 * w.0 - b.1 * w.1, b.0 * w.1 + b.1 * w.0);
                v[start + k] = (a.0 + z.0, a.1 + z.1);
                v[start + k + len / 2] = (a.0 - z.0, a.1 - z.1);
                w = (w.0 * wlen.0 - w.1 * wlen.1, w.0 * wlen.1 + w.1 * wlen.0);
            }
        }
        len *= 2;
    }
    if inverse {
        for z in v {
            z.0 /= n as f64;
            z.1 /= n as f64;
        }
    }
}
/// O(n) tridiagonal-plus-rank-one factor. Near-singular denominators refuse.
#[derive(Clone, Debug)]
pub struct ShiftFactor {
    diag: Vec<f64>,
    multipliers: Vec<f64>,
    edge: f64,
    image: Vec<f64>,
    denominator: f64,
}
impl ShiftFactor {
    fn triangular(&self, b: &[f64], w: &mut Work) -> Result<Vec<f64>> {
        let n = self.diag.len();
        if b.len() != n {
            return Err("BAND_RHS_SHAPE".into());
        }
        finite(b)?;
        w.band_solves += 1;
        let mut x = b.to_vec();
        for i in 1..n {
            x[i] -= self.multipliers[i] * x[i - 1];
        }
        x[n - 1] /= self.diag[n - 1];
        for i in (0..n - 1).rev() {
            x[i] = (x[i] - self.edge * x[i + 1]) / self.diag[i];
        }
        finite(&x)?;
        Ok(x)
    }
    pub fn solve(&self, b: &[f64], w: &mut Work) -> Result<Vec<f64>> {
        let mut x = self.triangular(b, w)?;
        w.rank_resolvent_solves += 1;
        let coefficient = x.iter().sum::<f64>() / x.len() as f64 / self.denominator;
        for (a, v) in x.iter_mut().zip(&self.image) {
            *a += coefficient * v;
        }
        finite(&x)?;
        Ok(x)
    }
}
