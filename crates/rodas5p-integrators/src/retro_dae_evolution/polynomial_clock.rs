//! Exact coefficient-proved common polynomial clock, no rounded ratio test.
use super::clock_tube::matches_terms;
use super::triangular_goal::{
    TriangularGoalRoute, TriangularGoalWork, clock_measure, clock_partition, initial_block, log1p,
    positive,
};
use super::*;
use rodas5p_core::directed::sub_up;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PolynomialClockRoute {
    Termwise,
    Factored,
    Intersection,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct PolynomialClockWork {
    pub blocks: usize,
    pub gradient_width: f64,
    pub goal: TriangularGoalWork,
    pub measure_width: f64,
}
#[derive(Clone, Copy)]
pub(super) struct ClockPolynomial {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub ratio: Iv,
}
// Decode represented finite reals; product of two 53-bit significands fits u128.
fn product_key(x: f64, y: f64) -> (bool, u128, i32) {
    fn unpack(x: f64) -> (u64, i32) {
        let u = x.to_bits();
        let e = ((u >> 52) & 2047) as i32;
        let m = u & ((1u64 << 52) - 1);
        if e == 0 {
            (m, -1074)
        } else {
            (m | (1u64 << 52), e - 1075)
        }
    }
    let (a, e) = unpack(x);
    let (b, f) = unpack(y);
    let v = a as u128 * b as u128;
    if v == 0 {
        return (false, 0, 0);
    }
    let z = v.trailing_zeros();
    (
        x.is_sign_negative() ^ y.is_sign_negative(),
        v >> z,
        e + f + z as i32,
    )
}
fn coeffs(eq: &DeclaredPolynomial, j: u8, order: usize) -> CoreResult<[f64; 3]> {
    let mut coeff = [0.; 3];
    let mut seen = [false; 3];
    let mut pattern = [[0u8; 4]; 3];
    for k in 0..3 {
        for i in k..k + order {
            pattern[k][i] = j;
        }
    }
    for term in eq.terms() {
        let len = term.factors.len();
        if len < order || len > order + 2 || seen[len - order] {
            return Err(bad("polynomial clock duplicate or degree"));
        }
        let k = len - order;
        seen[k] = true;
        coeff[k] = term.coefficient;
    }
    let mut want = [(0., &[][..]); 3];
    let mut n = 0;
    for k in 0..3 {
        if seen[k] {
            want[n] = (coeff[k], &pattern[k][..k + order]);
            n += 1;
        }
    }
    if !matches_terms(eq, &want[..n]) {
        return Err(bad("polynomial clock unsupported term"));
    }
    Ok(coeff)
}
impl ClockPolynomial {
    pub fn parse(src: &DaeSource, i: usize) -> CoreResult<Self> {
        let j = (src.nx() + i + 1) as u8;
        let a = coeffs(&src.f[i], j, 2)?.map(|v| -v);
        let b = coeffs(&src.f[i + 1], j, 1)?;
        if a.iter().any(|v| *v < 0.) {
            return Err(bad("polynomial clock negative rate coefficient"));
        }
        let pivot = (0..3)
            .filter(|&k| a[k] > 0.)
            .max_by(|&j, &k| a[j].total_cmp(&a[k]))
            .ok_or_else(|| bad("zero polynomial clock"))?;
        for k in 0..3 {
            if product_key(b[k], a[pivot]) != product_key(b[pivot], a[k]) {
                return Err(bad("polynomial clock exact proportionality"));
            }
        }
        Ok(Self {
            a,
            b,
            ratio: p(b[pivot])?.div(p(a[pivot])?)?,
        })
    }
    fn coupled() -> Self {
        Self {
            a: [0., 1., 0.],
            b: [0., 1., 0.],
            ratio: Iv { lo: 1., hi: 1. },
        }
    }
    fn measure(self, start: f64, end: f64, route: PolynomialClockRoute) -> CoreResult<Iv> {
        let termwise = || -> CoreResult<Iv> {
            let mut m = zero();
            for (k, a) in self.a.iter().enumerate() {
                if *a != 0. {
                    m = m.add(clock_measure(start, end, k)?.scale(*a)?)?;
                }
            }
            positive(m)
        };
        let factored = || -> CoreResult<Iv> {
            let t = p(end)?;
            let s = p(start)?;
            let delta = t.sub(s)?;
            let mut q = p(self.a[0])?;
            if self.a[1] != 0. {
                q = q.add(t.add(s)?.scale(0.5)?.scale(self.a[1])?)?;
            }
            if self.a[2] != 0. {
                q = q.add(
                    t.mul(t)?
                        .add(t.mul(s)?)?
                        .add(s.mul(s)?)?
                        .div(p(3.)?)?
                        .scale(self.a[2])?,
                )?;
            }
            positive(delta.mul(q)?)
        };
        match route {
            PolynomialClockRoute::Termwise => termwise(),
            PolynomialClockRoute::Factored => factored(),
            PolynomialClockRoute::Intersection => {
                let a = termwise()?;
                let b = factored()?;
                Iv::new(a.lo.max(b.lo), a.hi.min(b.hi))
            }
        }
    }
    pub(super) fn normalized(coeff: [f64; 3], start: f64, h: Iv) -> CoreResult<ScratchPoly> {
        let t = ScratchPoly::new(&[p(start)?, h])?;
        let mut out = ScratchPoly::new(&[p(coeff[0])?])?;
        if coeff[1] != 0. {
            out = out.add(t.scale(p(coeff[1])?)?.slice())?;
        }
        if coeff[2] != 0. {
            out = out.add(t.mul(t.slice())?.scale(p(coeff[2])?)?.slice())?;
        }
        Ok(out)
    }
}
/// Jointly enclose true-flow initial covector and signed terminal goal error.
/// All outputs are transactional; retained facade provides physical authority.
pub fn polynomial_clock_fused_into(
    src: &DaeSource,
    start: f64,
    end: f64,
    initial: &[f64],
    terminal: &[f64],
    weights: &[f64],
    route: PolynomialClockRoute,
    log: TriangularGoalRoute,
    gradient: &mut [Iv],
    goal: &mut Iv,
) -> CoreResult<PolynomialClockWork> {
    let coupled = clock_partition(src, initial, weights, start, end)?;
    let n = src.nx();
    if gradient.len() != n || terminal.len() != 2 * n || terminal.iter().any(|x| !x.is_finite()) {
        return Err(bad("polynomial clock output shape"));
    }
    let mut grad = [zero(); 16];
    let mut err = zero();
    let mut w = PolynomialClockWork::default();
    for i in (0..n).step_by(2) {
        let c = if coupled {
            ClockPolynomial::coupled()
        } else {
            ClockPolynomial::parse(src, i)?
        };
        let (p0, q0) = initial_block(initial, i, coupled)?;
        let m = c.measure(start, end, route)?;
        w.measure_width = w.measure_width.max(sub_up(m.hi, m.lo)?);
        let v = positive(p0.mul(m)?)?;
        let d = p(1.)?.add(v)?;
        let pe = p0.div(d)?;
        let qe = q0.add(log1p(v, log, &mut w.goal)?.mul(c.ratio)?)?;
        let ue = if coupled { pe.add(qe)? } else { pe };
        let wu = p(weights[i])?.add(p(weights[n + i])?)?;
        let wv = p(weights[i + 1])?.add(p(weights[n + i + 1])?)?;
        let wq = if coupled { wu.add(wv)? } else { wv };
        let lp = wu.div(d)?.div(d)?.add(wq.mul(c.ratio)?.mul(m)?.div(d)?)?;
        grad[i] = lp;
        grad[i + 1] = if coupled { wq.sub(lp)? } else { wq };
        for (j, x) in [(i, ue), (i + 1, qe), (n + i, ue), (n + i + 1, qe)] {
            err = err.add(x.sub(p(terminal[j])?)?.scale(weights[j])?)?;
        }
        w.blocks += 1;
    }
    for x in &grad[..n] {
        w.gradient_width = w.gradient_width.max(sub_up(x.hi, x.lo)?);
    }
    w.goal.blocks = w.blocks;
    w.goal.signed_error = err.as_pair();
    w.goal.width_upper = sub_up(err.hi, err.lo)?;
    gradient.copy_from_slice(&grad[..n]);
    *goal = err;
    Ok(w)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_products_cover_extremes() {
        assert_eq!(product_key(f64::MAX, 0.5), product_key(f64::MAX / 2., 1.));
        assert_eq!(
            product_key(f64::from_bits(1), 2.),
            product_key(f64::from_bits(2), 1.)
        );
        assert_ne!(product_key(f64::from_bits(1), 0.5), product_key(0., 1.));
        assert_ne!(product_key(0.75f64.next_up(), 0.5), product_key(0.375, 1.));
    }
}
