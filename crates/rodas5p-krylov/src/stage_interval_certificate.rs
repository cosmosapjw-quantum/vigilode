//! Conditional interval enclosure for one shifted linear stage A=I-sigma*J.
//! A source owner MUST establish that physical J and b belong to the supplied
//! intervals for the SAME physical source/clock/norm/epoch. This code only
//! certifies all represented real systems contained in those input intervals.
//! No source ownership, physical33 membership or matrix-free JVP error is inferred.
#![forbid(unsafe_code)]

/// Verify nearest-even binary64 with gradual underflow before outward bounds.
/// The caller must also maintain this FP environment for the entire call.
pub fn check_arithmetic() -> Result<(), String> {
    use std::hint::black_box;
    if !cfg!(target_arch = "x86_64") {
        return Err("unsupported float environment".into());
    }
    let one = black_box(1.0_f64);
    let eps = black_box(f64::EPSILON);
    let t = black_box(0.75 * eps);
    let h = black_box(0.5 * eps);
    if black_box(one + t).to_bits() != one.next_up().to_bits()
        || black_box(-one - t).to_bits() != (-one).next_down().to_bits()
        || black_box(one + h).to_bits() != one.to_bits()
        || black_box(-one - h).to_bits() != (-one).to_bits()
        || black_box(f64::from_bits(1) * one).to_bits() != 1
        || black_box(f64::MIN_POSITIVE * black_box(0.5)).to_bits() != 0x0008_0000_0000_0000
    {
        Err("binary64 nearest-even and gradual-underflow required".into())
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}
impl Interval {
    pub fn new(lo: f64, hi: f64) -> Result<Self, String> {
        if lo.is_finite() && hi.is_finite() && lo <= hi {
            Ok(Self { lo, hi })
        } else {
            Err("invalid interval endpoints".into())
        }
    }
    pub fn point(x: f64) -> Result<Self, String> {
        Self::new(x, x)
    }
    fn result(lo: f64, hi: f64) -> Result<Self, String> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err("nonfinite interval arithmetic".into());
        }
        Self::new(lo.next_down(), hi.next_up())
    }
    pub fn add_out(self, b: Self) -> Result<Self, String> {
        Self::result(self.lo + b.lo, self.hi + b.hi)
    }
    pub fn negated(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    pub fn sub_out(self, b: Self) -> Result<Self, String> {
        self.add_out(b.negated())
    }
    pub fn mul_out(self, b: Self) -> Result<Self, String> {
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if !v.iter().all(|x| x.is_finite()) {
            return Err("nonfinite interval product".into());
        }
        Self::result(
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    pub fn div_positive(self, b: Self) -> Result<Self, String> {
        if b.lo <= 0.0 || !b.hi.is_finite() {
            return Err("nonpositive interval divisor".into());
        }
        let v = [
            self.lo / b.lo,
            self.lo / b.hi,
            self.hi / b.lo,
            self.hi / b.hi,
        ];
        if !v.iter().all(|x| x.is_finite()) {
            return Err("nonfinite interval quotient".into());
        }
        Self::result(
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    pub fn abs_up(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceStamp {
    pub model: String,
    pub source: String,
    pub clock: String,
    pub norm: String,
    pub epoch: u64,
}
impl SourceStamp {
    fn is_valid(&self) -> bool {
        [&self.model, &self.source, &self.clock, &self.norm]
            .iter()
            .all(|s| !s.trim().is_empty())
    }
}

/// The (mathematical) enclosed J and b for a single frozen stage.
/// Construction validates representation, NOT the membership of a physical source.
#[derive(Clone, Debug)]
pub struct StageIntervalBox {
    n: usize,
    jacobian: Vec<Interval>,
    rhs: Vec<Interval>,
    weight: Vec<f64>,
    physical_weight: Vec<f64>,
    sigma: f64,
    stamp: SourceStamp,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionalStageVerdict {
    pub admitted_within_given_box: bool,
    pub original_residual_upper: f64,
    pub weighted_inverse_upper: f64,
    pub norm_transport_upper: f64,
    pub physical_error_upper: f64,
    pub signed_lognorm_upper: f64,
}
impl StageIntervalBox {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        n: usize,
        jacobian: Vec<Interval>,
        rhs: Vec<Interval>,
        weight: Vec<f64>,
        physical_weight: Vec<f64>,
        sigma: f64,
        stamp: SourceStamp,
    ) -> Result<Self, String> {
        if n == 0
            || jacobian.len() != n.checked_mul(n).ok_or("dimension overflow")?
            || rhs.len() != n
            || weight.len() != n
            || physical_weight.len() != n
            || !sigma.is_finite()
            || sigma < 0.0
            || !stamp.is_valid()
        {
            return Err("invalid source-box shape, shift or stamp".into());
        }
        if weight
            .iter()
            .chain(&physical_weight)
            .any(|x| !x.is_finite() || *x <= 0.0)
        {
            return Err("invalid positive weights".into());
        }
        for v in jacobian.iter().chain(&rhs) {
            Interval::new(v.lo, v.hi)?;
        }
        Ok(Self {
            n,
            jacobian,
            rhs,
            weight,
            physical_weight,
            sigma,
            stamp,
        })
    }
    pub fn signed_lognorm_upper(&self) -> Result<f64, String> {
        let mut upper = f64::NEG_INFINITY;
        for i in 0..self.n {
            let mut row = Interval::point(self.jacobian[i * self.n + i].hi)?;
            for j in 0..self.n {
                if j != i {
                    let term = Interval::point(self.jacobian[i * self.n + j].abs_up())?
                        .mul_out(Interval::point(self.weight[j])?)?
                        .div_positive(Interval::point(self.weight[i])?)?;
                    row = row.add_out(term)?;
                }
            }
            upper = upper.max(row.hi);
        }
        if !upper.is_finite() {
            return Err("nonfinite signed lognorm".into());
        }
        Ok(upper)
    }
    fn weighted_residual_upper(&self, x: &[f64]) -> Result<f64, String> {
        if x.len() != self.n || x.iter().any(|a| !a.is_finite()) {
            return Err("invalid candidate".into());
        }
        let mut result = 0.0_f64;
        for i in 0..self.n {
            let mut jx = Interval::point(0.0)?;
            for (j, &xj) in x.iter().enumerate() {
                jx = jx.add_out(self.jacobian[i * self.n + j].mul_out(Interval::point(xj)?)?)?;
            }
            let r = self.rhs[i]
                .sub_out(Interval::point(x[i])?)?
                .add_out(Interval::point(self.sigma)?.mul_out(jx)?)?;
            let ub = Interval::point(r.abs_up())?
                .div_positive(Interval::point(self.weight[i])?)?
                .hi;
            result = result.max(ub);
        }
        Ok(result)
    }
    pub fn certify(
        &self,
        x: &[f64],
        original_tolerance: f64,
        stage_budget: f64,
    ) -> Result<ConditionalStageVerdict, String> {
        if !original_tolerance.is_finite()
            || original_tolerance < 0.0
            || !stage_budget.is_finite()
            || stage_budget < 0.0
        {
            return Err("nonfinite or negative stage budgets".into());
        }
        check_arithmetic()?;
        let mu = self.signed_lognorm_upper()?;
        let margin = Interval::point(1.0)?
            .sub_out(Interval::point(self.sigma)?.mul_out(Interval::point(mu)?)?)?;
        if margin.lo <= 0.0 {
            return Err("unproved inverse margin".into());
        }
        let inv = Interval::point(1.0)?.div_positive(margin)?.hi;
        let rho = self.weighted_residual_upper(x)?;
        let mut c = 0.0_f64;
        for i in 0..self.n {
            c = c.max(
                Interval::point(self.weight[i])?
                    .div_positive(Interval::point(self.physical_weight[i])?)?
                    .hi,
            );
        }
        let forward = Interval::point(rho)?
            .mul_out(Interval::point(inv)?)?
            .mul_out(Interval::point(c)?)?
            .hi;
        Ok(ConditionalStageVerdict {
            admitted_within_given_box: rho <= original_tolerance && forward <= stage_budget,
            original_residual_upper: rho,
            weighted_inverse_upper: inv,
            norm_transport_upper: c,
            physical_error_upper: forward,
            signed_lognorm_upper: mu,
        })
    }
    /// Caller must provide live source owner read. Equality of tokens is NOT
    /// evidence that the source itself belongs to the interval box.
    /// Caller must serialize source mutation with output commit.
    pub fn commit_if_admitted(
        &self,
        expected: &SourceStamp,
        mut live: impl FnMut() -> SourceStamp,
        candidate: &[f64],
        output: &mut [f64],
        original_tolerance: f64,
        stage_budget: f64,
    ) -> Result<ConditionalStageVerdict, String> {
        if output.len() != self.n || candidate.len() != self.n {
            return Err("bad output/candidate shape".into());
        }
        if expected != &self.stamp || live() != *expected {
            return Err("stale source identity before certificate".into());
        }
        let verdict = self.certify(candidate, original_tolerance, stage_budget)?;
        if live() != *expected {
            return Err("stale source identity before output commit".into());
        }
        if verdict.admitted_within_given_box {
            output.copy_from_slice(candidate);
        }
        Ok(verdict)
    }
}
