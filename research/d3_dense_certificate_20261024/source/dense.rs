//! Immutable represented cubic/quintic curves for the fixed D2 Riccati model.
//!
//! The native directed arithmetic is reused unchanged. Its IEEE binary64,
//! round-to-nearest, gradual-underflow assumptions must hold on the executing
//! thread for construction, certification and every query. The runner owns
//! that environment check. No platform environment provider is defined here.
//!
//! Incoming radii refer to the same nonnegative exact Riccati solution at t0.
//! The runner starts from its given represented state with zero radius and
//! carries each certified endpoint radius. Positivity of that exact solution
//! supplies the contraction domain; an arbitrary signed initial uncertainty
//! set is outside this certificate contract.
#![forbid(unsafe_code)]

use crate::directed::{add_up, div_up, exp_interval, mul_up, sub_up, Interval};
use crate::CoreError;

const DIM: usize = 30;
const FULL: usize = 33;
const ZERO: Interval = Interval { lo: 0.0, hi: 0.0 };

#[derive(Debug)]
pub enum DenseError {
    Invalid(&'static str),
    Arithmetic(CoreError),
    /// A sufficient Bernstein-domain condition failed. An interior negative
    /// coefficient alone is not proof that the represented curve leaves D.
    DomainNotCertified { control: usize, row: usize, lower: f64 },
    QueryDomain(crate::domain::Violation),
    InvalidRadius { index: usize, value: f64 },
    Budget { row: usize, radius: f64, budget: f64 },
}

impl From<CoreError> for DenseError {
    fn from(value: CoreError) -> Self { Self::Arithmetic(value) }
}

impl std::fmt::Display for DenseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DenseError {}
pub type Result<T> = std::result::Result<T, DenseError>;

#[derive(Debug, Clone, Copy)]
pub struct Model { alpha: f64 }

impl Model {
    pub fn new(alpha: f64) -> Result<Self> {
        if alpha != 0.0 && alpha != 4.0 {
            return Err(DenseError::Invalid("supported alpha is 0 or 4"));
        }
        Ok(Self { alpha })
    }
    pub fn alpha(&self) -> f64 { self.alpha }
    fn rate(j: usize) -> f64 { [1.0, 20.0, 300.0][j % 3] }
    fn rhs(&self, j: usize, z: f64) -> f64 {
        -Self::rate(j) * z - self.alpha * z * z
    }
    fn jf(&self, j: usize, z: f64) -> f64 {
        (-Self::rate(j) - 2.0 * self.alpha * z) * self.rhs(j, z)
    }
}

#[derive(Debug, Clone)]
pub struct Curve {
    model: Model,
    degree: usize,
    t0: f64,
    t1: f64,
    delta: Interval,
    controls: [[f64; DIM]; 6],
}

impl Curve {
    /// Construct actual binary64 Bernstein controls. Certification applies to
    /// these stored controls, including their construction rounding.
    pub fn new(
        model: &Model, degree: usize, t0: f64, t1: f64,
        z0: &[f64; DIM], z1: &[f64; DIM],
    ) -> Result<Self> {
        let _ = duration(t0, t1)?;
        if degree != 3 && degree != 5 {
            return Err(DenseError::Invalid("supported degree is 3 or 5"));
        }
        if z0.iter().chain(z1).any(|x| !x.is_finite()) {
            return Err(DenseError::Invalid("non-finite endpoint"));
        }
        let h = t1 - t0;
        let mut controls = [[0.0; DIM]; 6];
        controls[0] = *z0;
        controls[degree] = *z1;
        for j in 0..DIM {
            let f0 = model.rhs(j, z0[j]);
            let f1 = model.rhs(j, z1[j]);
            if degree == 3 {
                controls[1][j] = z0[j] + h * f0 / 3.0;
                controls[2][j] = z1[j] - h * f1 / 3.0;
            } else {
                let jf0 = model.jf(j, z0[j]);
                let jf1 = model.jf(j, z1[j]);
                controls[1][j] = z0[j] + h * f0 / 5.0;
                controls[2][j] = z0[j] + 2.0 * h * f0 / 5.0 + h * h * jf0 / 20.0;
                controls[4][j] = z1[j] - h * f1 / 5.0;
                controls[3][j] = z1[j] - 2.0 * h * f1 / 5.0 + h * h * jf1 / 20.0;
            }
        }
        Self::from_controls(model, degree, t0, t1, controls)
    }

    /// Research/test construction of an arbitrary immutable represented curve.
    /// It receives the same domain and residual checks as a generated curve.
    /// Driver endpoint/model binding remains a caller obligation.
    pub(crate) fn from_controls(
        model: &Model, degree: usize, t0: f64, t1: f64,
        mut controls: [[f64; DIM]; 6],
    ) -> Result<Self> {
        if degree != 3 && degree != 5 {
            return Err(DenseError::Invalid("supported degree is 3 or 5"));
        }
        let delta = duration(t0, t1)?;
        if controls.iter().flatten().any(|x| !x.is_finite()) {
            return Err(DenseError::Invalid("non-finite represented control"));
        }
        for k in degree + 1..6 { controls[k] = [0.0; DIM]; }
        for (k, z) in controls.iter().enumerate().take(degree + 1) {
            for (j, &value) in z.iter().enumerate() {
                if value < 0.0 {
                    return Err(DenseError::DomainNotCertified {
                        control: k, row: retained(j), lower: value,
                    });
                }
            }
            let full = lift_intervals(&z.map(|v| Interval { lo: v, hi: v }))?;
            for row in [23, 25, 28] {
                if full[row].lo < 0.0 {
                    return Err(DenseError::DomainNotCertified {
                        control: k, row, lower: full[row].lo,
                    });
                }
            }
        }
        Ok(Self { model: *model, degree, t0, t1, delta, controls })
    }

    pub fn controls(&self) -> [[f64; DIM]; 6] { self.controls }
    pub fn degree(&self) -> usize { self.degree }
    pub fn times(&self) -> (f64, f64) { (self.t0, self.t1) }

    /// Certify the original ODE defect, propagated component errors and the
    /// absolute full33 curve budget. Query rounding is admitted per query.
    pub fn certify(&self, incoming: &[f64; DIM], budget: f64) -> Result<Certified> {
        if !budget.is_finite() || budget <= 0.0 {
            return Err(DenseError::Invalid("curve budget must be finite and positive"));
        }
        for (index, &value) in incoming.iter().enumerate() {
            if !value.is_finite() || value < 0.0 {
                return Err(DenseError::InvalidRadius { index, value });
            }
        }
        let n = self.degree;
        let m = 2 * n;
        let mut defect_lo = [[0.0; 11]; DIM];
        let mut defect_hi = [[0.0; 11]; DIM];
        let mut rho = [0.0_f64; DIM];
        let mut endpoint = [0.0; DIM];
        let mut uniform = [0.0; DIM];
        let mut gain_lo = [0.0; DIM];
        let mut gain_hi = [0.0; DIM];
        let mut response_upper = [0.0; DIM];

        // Exact binomial integers are at most O(10^3) here; only their division
        // is rounded, and that division is enclosed by the original Interval.
        let mut wd = [[ZERO; 6]; 11];
        let mut wp = [[ZERO; 6]; 11];
        let mut w2 = [[ZERO; 6]; 11];
        for k in 0..=m {
            for i in 0..n {
                if k >= i && k - i <= m - (n - 1) {
                    wd[k][i] = ratio(
                        binomial(n - 1, i) * binomial(m - (n - 1), k - i),
                        binomial(m, k),
                    )?;
                }
            }
            for i in 0..=n {
                if k >= i && k - i <= m - n {
                    wp[k][i] = ratio(binomial(n, i) * binomial(m - n, k - i), binomial(m, k))?;
                }
                if k >= i && k - i <= n {
                    w2[k][i] = ratio(binomial(n, i) * binomial(n, k - i), binomial(m, k))?;
                }
            }
        }

        let mut gains = [ZERO; 3];
        let mut responses = [0.0; 3];
        for (i, lambda) in [1.0, 20.0, 300.0].into_iter().enumerate() {
            let exponent = Interval::point(-lambda)?.mul(self.delta)?;
            let lo = exp_interval(exponent.lo)?.lo.max(0.0);
            let hi = exp_interval(exponent.hi)?.hi.min(1.0);
            gains[i] = Interval::new(lo, hi)?;
            let numerator = sub_up(1.0, lo)?;
            responses[i] = div_up(numerator, lambda)?.min(self.delta.hi);
            if !(responses[i].is_finite() && responses[i] >= 0.0) {
                return Err(DenseError::Invalid("invalid contraction response enclosure"));
            }
        }

        for j in 0..DIM {
            let mut b = [ZERO; 6];
            let mut derivative = [ZERO; 6];
            for i in 0..=n { b[i] = Interval::point(self.controls[i][j])?; }
            for i in 0..n {
                derivative[i] = b[i + 1].sub(b[i])?.scale(n as f64)?.div(self.delta)?;
            }
            for k in 0..=m {
                let mut d = ZERO;
                let mut p = ZERO;
                let mut p2 = ZERO;
                for i in 0..n {
                    if wd[k][i].hi != 0.0 { d = d.add(wd[k][i].mul(derivative[i])?)?; }
                }
                for i in 0..=n {
                    if wp[k][i].hi != 0.0 { p = p.add(wp[k][i].mul(b[i])?)?; }
                    if self.model.alpha != 0.0 && w2[k][i].hi != 0.0 {
                        p2 = p2.add(w2[k][i].mul(b[i].mul(b[k - i])?)?)?;
                    }
                }
                let residual = d.add(p.scale(Model::rate(j))?)?.add(p2.scale(self.model.alpha)?)?;
                defect_lo[j][k] = residual.lo;
                defect_hi[j][k] = residual.hi;
                rho[j] = rho[j].max(residual.mag());
            }
            gain_lo[j] = gains[j % 3].lo;
            gain_hi[j] = gains[j % 3].hi;
            response_upper[j] = responses[j % 3];
            let carry = mul_up(gain_hi[j], incoming[j])?;
            let forcing = mul_up(response_upper[j], rho[j])?;
            endpoint[j] = add_up(carry, forcing)?;
            uniform[j] = incoming[j].max(endpoint[j]);
        }
        let physical = lift_radii(&uniform)?;
        check_budget(&physical, budget)?;
        let diagnostics = Diagnostics {
            alpha: self.model.alpha, degree: n, t0: self.t0, t1: self.t1,
            controls: self.controls, delta: self.delta.as_pair(), incoming: *incoming,
            rho, endpoint, uniform, defect_lo, defect_hi, gain_lo, gain_hi, response_upper,
        };
        Ok(Certified { curve: self.clone(), diagnostics, budget })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Diagnostics {
    pub alpha: f64,
    pub degree: usize,
    pub t0: f64,
    pub t1: f64,
    pub controls: [[f64; DIM]; 6],
    pub delta: [f64; 2],
    pub incoming: [f64; DIM],
    pub rho: [f64; DIM],
    pub endpoint: [f64; DIM],
    pub uniform: [f64; DIM],
    pub defect_lo: [[f64; 11]; DIM],
    pub defect_hi: [[f64; 11]; DIM],
    pub gain_lo: [f64; DIM],
    pub gain_hi: [f64; DIM],
    pub response_upper: [f64; DIM],
}

#[derive(Debug, Clone)]
pub struct Certified { curve: Curve, diagnostics: Diagnostics, budget: f64 }

#[derive(Debug, Clone, Copy)]
pub struct Query {
    pub center: [f64; FULL],
    pub radius: [f64; FULL],
    pub rounding: [f64; FULL],
}

impl Certified {
    pub fn diagnostics(&self) -> Diagnostics { self.diagnostics }
    pub fn endpoint_radius(&self) -> [f64; DIM] { self.diagnostics.endpoint }
    pub fn uniform_radius(&self) -> [f64; DIM] { self.diagnostics.uniform }
    pub fn budget(&self) -> f64 { self.budget }

    /// A future query is checked independently. Failure changes no certificate
    /// or committed trajectory state.
    pub fn query(&self, t: f64) -> Result<Query> {
        let c = &self.curve;
        if !t.is_finite() || t < c.t0 || t > c.t1 {
            return Err(DenseError::Invalid("query outside finite represented segment"));
        }
        let (theta, theta_center) = if t == c.t0 {
            (Interval::point(0.0)?, 0.0)
        } else if t == c.t1 {
            (Interval::point(1.0)?, 1.0)
        } else {
            let quotient = Interval::point(t)?.sub(Interval::point(c.t0)?)?.div(c.delta)?;
            // Exact t is already checked in [t0,t1], so intersecting with
            // [0,1] tightens a coordinate enclosure without changing a state.
            let theta = Interval::new(quotient.lo.max(0.0), quotient.hi.min(1.0))?;
            let center = (t - c.t0) / (c.t1 - c.t0);
            if !center.is_finite() || !(0.0..=1.0).contains(&center) {
                return Err(DenseError::Invalid("invalid represented query coordinate"));
            }
            (theta, center)
        };
        let mut values = c.controls;
        let mut boxes = [[ZERO; DIM]; 6];
        for i in 0..=c.degree {
            for j in 0..DIM { boxes[i][j] = Interval::point(values[i][j])?; }
        }
        let one_minus = Interval::point(1.0)?.sub(theta)?;
        for level in 1..=c.degree {
            for i in 0..=c.degree - level {
                for j in 0..DIM {
                    values[i][j] = (1.0 - theta_center) * values[i][j] + theta_center * values[i + 1][j];
                    boxes[i][j] = one_minus.mul(boxes[i][j])?.add(theta.mul(boxes[i + 1][j])?)?;
                }
            }
        }
        // Preserve endpoint bits, including signed zeros, for endpoint queries.
        let compact = if t == c.t0 { c.controls[0] }
            else if t == c.t1 { c.controls[c.degree] } else { values[0] };
        let center = lift_center(&compact);
        crate::domain::check(&compact, &center).map_err(DenseError::QueryDomain)?;
        let exact_curve = lift_intervals(&boxes[0])?;
        let physical_radius = lift_radii(&self.diagnostics.uniform)?;
        let mut rounding = [0.0; FULL];
        let mut radius = [0.0; FULL];
        for row in 0..FULL {
            let left = sub_up(center[row], exact_curve[row].lo)?;
            let right = sub_up(exact_curve[row].hi, center[row])?;
            rounding[row] = left.max(right).max(0.0);
            radius[row] = add_up(physical_radius[row], rounding[row])?;
        }
        check_budget(&radius, self.budget)?;
        Ok(Query { center, radius, rounding })
    }
}

fn duration(t0: f64, t1: f64) -> Result<Interval> {
    if !(t0.is_finite() && t1.is_finite() && t0 < t1) {
        return Err(DenseError::Invalid("finite increasing segment times required"));
    }
    let delta = Interval::point(t1)?.sub(Interval::point(t0)?)?;
    if delta.lo <= 0.0 {
        return Err(DenseError::Invalid("positive represented duration not enclosed"));
    }
    Ok(delta)
}

fn binomial(n: usize, k: usize) -> u64 {
    if k > n { return 0; }
    let k = k.min(n - k);
    let mut value = 1_u64;
    for i in 1..=k { value = value * (n + 1 - i) as u64 / i as u64; }
    value
}

fn ratio(numerator: u64, denominator: u64) -> Result<Interval> {
    Ok(Interval::point(numerator as f64)?.div(Interval::point(denominator as f64)?)?)
}

fn retained(j: usize) -> usize {
    match j { 0..=22 => j, 23 => 24, 24..=25 => j + 2, 26..=28 => j + 3, _ => 32 }
}

fn lift_center(z: &[f64; DIM]) -> [f64; FULL] {
    let mut y = [0.0; FULL];
    for j in 0..DIM { y[retained(j)] = z[j]; }
    y[23] = 1.0 - z[23];
    y[25] = 0.5 - z[24] - z[25];
    y[28] = 0.2 - z[26] - z[27] - z[28];
    y
}

fn lift_intervals(z: &[Interval; DIM]) -> Result<[Interval; FULL]> {
    let mut y = [ZERO; FULL];
    for j in 0..DIM { y[retained(j)] = z[j]; }
    y[23] = Interval::point(1.0)?.sub(z[23])?;
    y[25] = Interval::point(0.5)?.sub(z[24])?.sub(z[25])?;
    y[28] = Interval::point(0.2)?.sub(z[26])?.sub(z[27])?.sub(z[28])?;
    Ok(y)
}

fn lift_radii(r: &[f64; DIM]) -> Result<[f64; FULL]> {
    let mut y = [0.0; FULL];
    for j in 0..DIM { y[retained(j)] = r[j]; }
    y[23] = r[23];
    y[25] = add_up(r[24], r[25])?;
    y[28] = add_up(add_up(r[26], r[27])?, r[28])?;
    Ok(y)
}

fn check_budget(radii: &[f64; FULL], budget: f64) -> Result<()> {
    for (row, &radius) in radii.iter().enumerate() {
        if !radius.is_finite() || radius < 0.0 {
            return Err(DenseError::InvalidRadius { index: row, value: radius });
        }
        if radius > budget { return Err(DenseError::Budget { row, radius, budget }); }
    }
    Ok(())
}
