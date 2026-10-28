//! Scoped standalone Rust 1.94.1 upper evaluator for BASS chemical6 signed lognorm.
//! Frozen source cc602e... / binding 8cb268...; not an owner-accepted rate provider.
//! This module does not modify the production dispatcher or the physical33 norm.
#![forbid(unsafe_code)]
use std::hint::black_box;

const N: usize = 6;
const NR: usize = 12;
const SOURCE: &str = "cc602e209e7deeb511ab2a1f3b2268d55206c43c";
const BINDING: &str = "8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16";
const CLOCK: &str = "x=ln(a/a_ref)";
const TH: f64 = f64::from_bits(0x3fefffffffffffff);

type Mat = [[Iv; N]; N];

#[derive(Clone, Copy, Debug)]
pub struct Iv {
    pub lo: f64,
    pub hi: f64,
}
impl Iv {
    pub const ZERO: Self = Self { lo: 0.0, hi: 0.0 };
    pub fn new(lo: f64, hi: f64) -> Result<Self, &'static str> {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err("invalid finite interval");
        }
        Ok(Self { lo, hi })
    }
    pub fn point(v: f64) -> Result<Self, &'static str> {
        Self::new(v, v)
    }
    pub fn add_iv(self, b: Self) -> Result<Self, &'static str> {
        Self::new((self.lo + b.lo).next_down(), (self.hi + b.hi).next_up())
    }
    pub fn neg_iv(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    pub fn mul_iv(self, b: Self) -> Result<Self, &'static str> {
        let x = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if x.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite interval product");
        }
        let lo = x.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::new(lo.next_down(), hi.next_up())
    }
    pub fn div_pos(self, b: Self) -> Result<Self, &'static str> {
        if b.lo <= 0.0 {
            return Err("nonpositive denominator");
        }
        let inv = Self::new((1.0 / b.hi).next_down(), (1.0 / b.lo).next_up())?;
        self.mul_iv(inv)
    }
}

/// Safe arithmetic probes for nearest-even and gradual underflow. Not a register read.
pub fn check_arithmetic() -> Result<(), &'static str> {
    if !cfg!(target_arch = "x86_64") {
        return Err("x86_64 arithmetic not verified");
    }
    let one = black_box(1.0_f64);
    let neg = black_box(-1.0_f64);
    let t = black_box(0.75 * f64::EPSILON);
    let half = black_box(0.5 * f64::EPSILON);
    let nearest = black_box(one + t).to_bits() == 1.0_f64.next_up().to_bits()
        && black_box(neg - t).to_bits() == (-1.0_f64).next_down().to_bits()
        && black_box(one + half).to_bits() == one.to_bits()
        && black_box(neg - half).to_bits() == neg.to_bits();
    let tiny = black_box(f64::from_bits(1));
    let normal = black_box(f64::MIN_POSITIVE);
    let gradual = black_box(tiny * one).to_bits() == 1
        && black_box(normal * black_box(0.5_f64)).to_bits() == 0x0008_0000_0000_0000;
    if nearest && gradual {
        Ok(())
    } else {
        Err("nondefault arithmetic")
    }
}

#[derive(Clone, Debug)]
pub struct SourceInput {
    pub source: String,
    pub binding: String,
    pub clock: String,
    pub unit: String,
    pub epoch: u64,
    /// Fixed source isotope weights wh,whe,wli, all positive finite binary64.
    pub isotope_weights: [f64; 3],
    /// Fixed weighted-infinity norm weights, strictly positive.
    pub norm_weights: [f64; N],
    /// Independent rate boxes, in original source order.
    pub rates: [Iv; NR],
}

#[derive(Clone, Debug)]
pub struct Bound {
    pub upper: f64,
    pub row: usize,
    pub vertex: usize,
    pub signs: u8,
    pub chosen_rate_endpoint: [f64; NR],
    pub source: String,
    pub epoch: u64,
}

fn vertices() -> [[f64; N]; 24] {
    let mut out = [[0.0; N]; 24];
    let mut index = 0;
    for h in [0.0, TH] {
        for he in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]] {
            for li in [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ] {
                out[index] = [h, he[0], he[1], li[0], li[1], li[2]];
                index += 1;
            }
        }
    }
    out
}

fn basis_matrix(a: usize, z: &[f64; N], e: &[f64; N]) -> Result<Mat, &'static str> {
    let mut p = [[Iv::ZERO; N]; N];
    let mut q = [[Iv::ZERO; N]; N];
    // Exactly the source P and Q transition blocks, one unit rate at a time.
    let entries: &[(usize, usize, f64)] = match a {
        0 => &[(0, 0, -1.0)],
        1 => &[(1, 1, -1.0), (1, 2, -1.0)],
        2 => &[(1, 1, -1.0), (2, 1, 1.0)],
        3 => &[(3, 3, -1.0), (3, 4, -1.0), (3, 5, -1.0)],
        4 => &[(3, 3, -1.0), (4, 3, 1.0)],
        5 => &[(4, 4, -1.0), (5, 4, 1.0)],
        6 => &[(0, 0, -1.0)],
        7 => &[(1, 1, -1.0)],
        8 => &[(1, 2, 1.0), (2, 2, -1.0)],
        9 => &[(3, 3, -1.0)],
        10 => &[(3, 4, 1.0), (4, 4, -1.0)],
        11 => &[(4, 5, 1.0), (5, 5, -1.0)],
        _ => return Err("rate index"),
    };
    for &(i, j, c) in entries {
        if a < 6 {
            p[i][j] = Iv::point(c)?;
        } else {
            q[i][j] = Iv::point(c)?;
        }
    }
    if a < 6 {
        return Ok(p);
    }
    let mut electron = Iv::ZERO;
    for k in 0..N {
        electron = electron.add_iv(Iv::point(e[k])?.mul_iv(Iv::point(z[k])?)?)?;
    }
    let mut qz = [Iv::ZERO; N];
    for i in 0..N {
        for k in 0..N {
            qz[i] = qz[i].add_iv(q[i][k].mul_iv(Iv::point(z[k])?)?)?;
        }
    }
    let mut jac = [[Iv::ZERO; N]; N];
    for i in 0..N {
        for j in 0..N {
            jac[i][j] = electron
                .mul_iv(q[i][j])?
                .add_iv(qz[i].mul_iv(Iv::point(e[j])?)?)?;
        }
    }
    Ok(jac)
}

pub fn certify_mu(input: &SourceInput) -> Result<Bound, &'static str> {
    check_arithmetic()?;
    if input.source != SOURCE
        || input.binding != BINDING
        || input.clock != CLOCK
        || input.unit.trim().is_empty()
    {
        return Err("source/binding/clock/unit mismatch");
    }
    let [wh, whe, wli] = input.isotope_weights;
    if [wh, whe, wli].iter().any(|v| !v.is_finite() || *v <= 0.0)
        || input
            .norm_weights
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        || input
            .rates
            .iter()
            .any(|v| !v.lo.is_finite() || !v.hi.is_finite() || v.lo < 0.0 || v.hi < v.lo)
    {
        return Err("invalid isotope, norm or rate input");
    }
    let e = [wh, whe, 2.0 * whe, wli, 2.0 * wli, 3.0 * wli];
    if e.iter().any(|v| !v.is_finite()) {
        return Err("electron weights overflow");
    }
    let mut ratios = [[Iv::ZERO; N]; N];
    for (i, row) in ratios.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = Iv::point(input.norm_weights[j])?.div_pos(Iv::point(input.norm_weights[i])?)?;
        }
    }
    let mut best = Bound {
        upper: f64::NEG_INFINITY,
        row: 0,
        vertex: 0,
        signs: 0,
        chosen_rate_endpoint: [0.0; NR],
        source: input.source.clone(),
        epoch: input.epoch,
    };
    for (vi, z) in vertices().iter().enumerate() {
        let mut basis = [[[Iv::ZERO; N]; N]; NR];
        for (a, item) in basis.iter_mut().enumerate() {
            *item = basis_matrix(a, z, &e)?;
        }
        for row in 0..N {
            for mask in 0..32_u8 {
                let mut value = 0.0_f64;
                let mut chosen = [0.0; NR];
                for a in 0..NR {
                    let mut coeff = basis[a][row][row];
                    let mut bit = 0;
                    for col in 0..N {
                        if col == row {
                            continue;
                        }
                        let term = basis[a][row][col].mul_iv(ratios[row][col])?;
                        coeff = coeff.add_iv(if (mask >> bit) & 1 == 1 {
                            term
                        } else {
                            term.neg_iv()
                        })?;
                        bit += 1;
                    }
                    // Each exact signed row is affine in this rate. The maximum
                    // over its interval is at one of its two endpoints.
                    let lo = coeff.mul_iv(Iv::point(input.rates[a].lo)?)?;
                    let hi = coeff.mul_iv(Iv::point(input.rates[a].hi)?)?;
                    let (term, rate) = if hi.hi >= lo.hi {
                        (hi.hi, input.rates[a].hi)
                    } else {
                        (lo.hi, input.rates[a].lo)
                    };
                    value = (value + term).next_up();
                    if !value.is_finite() {
                        return Err("nonfinite lognorm sum");
                    }
                    chosen[a] = rate;
                }
                if value > best.upper {
                    best.upper = value;
                    best.row = row;
                    best.vertex = vi;
                    best.signs = mask;
                    best.chosen_rate_endpoint = chosen;
                }
            }
        }
    }
    if !best.upper.is_finite() {
        return Err("no finite bound");
    }
    Ok(best)
}

/// Conservative rational comparison from Dini, not an optimal stiff bound.
/// For mu>0 requires h*mu<1; refuses otherwise. All arithmetic is outward.
pub fn rational_whole_error(
    mu_upper: f64,
    h: f64,
    defect: f64,
    e0: f64,
) -> Result<f64, &'static str> {
    check_arithmetic()?;
    if !mu_upper.is_finite()
        || !h.is_finite()
        || h <= 0.0
        || !defect.is_finite()
        || defect < 0.0
        || !e0.is_finite()
        || e0 < 0.0
    {
        return Err("invalid error-budget input");
    }
    let m = mu_upper.max(0.0);
    let mh = Iv::point(m)?.mul_iv(Iv::point(h)?)?;
    if mh.hi >= 1.0 {
        return Err("m*h >= 1; rational bound unresolved");
    }
    let denom = Iv::point(1.0)?.add_iv(mh.neg_iv())?;
    if denom.lo <= 0.0 {
        return Err("nonpositive comparison denominator");
    }
    let numer = Iv::point(e0)?.add_iv(Iv::point(h)?.mul_iv(Iv::point(defect)?)?)?;
    Ok(numer.div_pos(denom)?.hi)
}

pub fn electron_event_dual_upper(
    isotope_weights: [f64; 3],
    norm_weights: [f64; N],
) -> Result<f64, &'static str> {
    let [wh, whe, wli] = isotope_weights;
    let e = [wh, whe, 2.0 * whe, wli, 2.0 * wli, 3.0 * wli];
    let mut upper = 0.0_f64;
    for i in 0..N {
        if !e[i].is_finite()
            || e[i] <= 0.0
            || !norm_weights[i].is_finite()
            || norm_weights[i] <= 0.0
        {
            return Err("invalid event dual input");
        }
        upper = (upper + Iv::point(e[i])?.mul_iv(Iv::point(norm_weights[i])?)?.hi).next_up();
    }
    Ok(upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SourceInput {
        SourceInput {
            source: SOURCE.to_string(),
            binding: BINDING.to_string(),
            clock: CLOCK.to_string(),
            unit: "dimensionless-fraction-per-log-a".to_string(),
            epoch: 42,
            isotope_weights: [0.125; 3],
            norm_weights: [1.0; 6],
            rates: std::array::from_fn(|a| {
                if a < 6 {
                    Iv {
                        lo: 1.0 / 16.0,
                        hi: 1.0 / 8.0,
                    }
                } else {
                    Iv {
                        lo: 1.0 / 32.0,
                        hi: 1.0 / 16.0,
                    }
                }
            }),
        }
    }
    #[test]
    fn exact_fixture_enclosed() {
        let b = certify_mu(&fixture()).unwrap();
        let exact = 378302368699121663.0 / 2305843009213693952.0;
        assert!(b.upper >= exact);
        assert!(b.upper < exact + 1e-10, "{}", b.upper);
        assert_eq!(b.row, 4);
        assert_eq!(b.vertex, 23);
        assert_eq!(b.source, SOURCE);
        assert_eq!(b.epoch, 42);
    }
    #[test]
    fn zero_rates_are_nonnegative_upper() {
        let mut f = fixture();
        for r in &mut f.rates {
            *r = Iv::ZERO;
        }
        let b = certify_mu(&f).unwrap();
        assert!(b.upper >= 0.0 && b.upper < 1e-12);
    }
    #[test]
    fn wrong_source_clock_unit_refused() {
        let mut f = fixture();
        f.source = "other".into();
        assert!(certify_mu(&f).is_err());
        f = fixture();
        f.clock = "proper-time".into();
        assert!(certify_mu(&f).is_err());
        f = fixture();
        f.unit = " ".into();
        assert!(certify_mu(&f).is_err());
    }
    #[test]
    fn invalid_rate_norm_isotope_refused() {
        let mut f = fixture();
        f.rates[3].lo = -1.0;
        assert!(certify_mu(&f).is_err());
        f = fixture();
        f.norm_weights[1] = 0.0;
        assert!(certify_mu(&f).is_err());
        f = fixture();
        f.isotope_weights[0] = f64::NAN;
        assert!(certify_mu(&f).is_err());
        f = fixture();
        f.rates[0].hi = f64::INFINITY;
        assert!(certify_mu(&f).is_err());
    }
    #[test]
    fn weighted_norm_and_source_feedback_change_bound() {
        let mut f = fixture();
        let a = certify_mu(&f).unwrap().upper;
        f.norm_weights = [1.0, 1.25, 1.5, 1.0, 1.25, 1.5];
        let b = certify_mu(&f).unwrap().upper;
        assert!(a != b);
        f = fixture();
        f.isotope_weights = [0.25; 3];
        assert_ne!(certify_mu(&f).unwrap().upper, a);
    }
    #[test]
    fn rational_dini_gate_and_dual() {
        let mu = certify_mu(&fixture()).unwrap().upper;
        let r = rational_whole_error(mu, 1.0 / 32.0, 1.0 / 4096.0, 1.0 / 65536.0).unwrap();
        assert!(r >= 1.0 / 65536.0 + 1.0 / (32.0 * 4096.0));
        assert!(rational_whole_error(2.0, 0.5, 0.0, 0.0).is_err());
        assert!(electron_event_dual_upper([0.125; 3], [1.0; 6]).unwrap() >= 1.25);
    }
    #[test]
    fn arithmetic_probe() {
        assert!(check_arithmetic().is_ok());
    }
}

/// Only a strict sign of the electron-fraction event, never a crossing count.
/// `represented_event` must enclose the exact real e^T p on the same source clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventSign {
    Positive,
    Negative,
    Unresolved,
}

pub fn electron_event_sign(
    represented_event: Iv,
    threshold: Iv,
    dual_upper: f64,
    state_error_upper: f64,
) -> Result<EventSign, &'static str> {
    check_arithmetic()?;
    if !dual_upper.is_finite()
        || dual_upper < 0.0
        || !state_error_upper.is_finite()
        || state_error_upper < 0.0
    {
        return Err("invalid event error budget");
    }
    let margin = Iv::point(dual_upper)?.mul_iv(Iv::point(state_error_upper)?)?;
    let centered = represented_event.add_iv(threshold.neg_iv())?;
    let bound = centered.add_iv(Iv::new(-margin.hi, margin.hi)?)?;
    Ok(if bound.lo > 0.0 {
        EventSign::Positive
    } else if bound.hi < 0.0 {
        EventSign::Negative
    } else {
        EventSign::Unresolved
    })
}

#[cfg(test)]
mod event_tests {
    use super::*;
    #[test]
    fn positive_negative_and_grazing_are_distinguished() {
        let dual = electron_event_dual_upper([0.125; 3], [1.0; 6]).unwrap();
        let r = 1.0 / 1024.0;
        assert_eq!(
            electron_event_sign(Iv::point(0.5).unwrap(), Iv::point(0.25).unwrap(), dual, r)
                .unwrap(),
            EventSign::Positive
        );
        assert_eq!(
            electron_event_sign(Iv::point(0.25).unwrap(), Iv::point(0.5).unwrap(), dual, r)
                .unwrap(),
            EventSign::Negative
        );
        assert_eq!(
            electron_event_sign(Iv::point(0.5).unwrap(), Iv::point(0.5).unwrap(), dual, r).unwrap(),
            EventSign::Unresolved
        );
        assert!(
            electron_event_sign(
                Iv::point(0.5).unwrap(),
                Iv::point(0.25).unwrap(),
                dual,
                f64::NAN
            )
            .is_err()
        );
    }
}