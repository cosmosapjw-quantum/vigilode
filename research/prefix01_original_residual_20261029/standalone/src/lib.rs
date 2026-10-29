#![forbid(unsafe_code)]
//! Standalone VIGILODE-PREFIX01: Givens residual triggers work, but only an
//! outward original-equation residual can admit a candidate. No production use.
use std::hint::black_box;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}
impl Interval {
    pub fn point(x: f64) -> Result<Self, String> {
        if x.is_finite() {
            Ok(Self { lo: x, hi: x })
        } else {
            Err("nonfinite point".into())
        }
    }
    fn finite(lo: f64, hi: f64) -> Result<Self, String> {
        if lo.is_finite() && hi.is_finite() && lo <= hi {
            Ok(Self { lo, hi })
        } else {
            Err("nonfinite interval".into())
        }
    }
    pub fn add_out(self, r: Self) -> Result<Self, String> {
        Self::finite((self.lo + r.lo).next_down(), (self.hi + r.hi).next_up())
    }
    pub fn negated(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    pub fn sub_out(self, r: Self) -> Result<Self, String> {
        self.add_out(r.negated())
    }
    pub fn mul_out(self, r: Self) -> Result<Self, String> {
        let p = [
            self.lo * r.lo,
            self.lo * r.hi,
            self.hi * r.lo,
            self.hi * r.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Err("nonfinite product".into());
        }
        Self::finite(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    pub fn div_pos(self, r: Self) -> Result<Self, String> {
        if r.lo <= 0.0 || !r.hi.is_finite() {
            return Err("nonpositive divisor".into());
        }
        let p = [
            self.lo / r.lo,
            self.lo / r.hi,
            self.hi / r.lo,
            self.hi / r.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Err("nonfinite quotient".into());
        }
        Self::finite(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    pub fn abs_upper(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
}
pub fn check_arithmetic() -> Result<(), String> {
    if !cfg!(target_arch = "x86_64") {
        return Err("x86_64 only".into());
    }
    let one = black_box(1.0f64);
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
        Err("nearest-even/gradual-underflow required".into())
    } else {
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct BoundModel {
    pub n: usize,
    pub jacobian: Vec<f64>,
    pub weights: Vec<f64>,
    pub sigma: f64,
    pub model_id: String,
    pub source_id: String,
    pub clock_id: String,
    pub norm_id: String,
    pub epoch: u64,
}
impl BoundModel {
    pub fn validate(&self) -> Result<(), String> {
        if self.n == 0
            || self.jacobian.len() != self.n * self.n
            || self.weights.len() != self.n
            || !self.sigma.is_finite()
            || self.sigma < 0.0
            || self.weights.iter().any(|w| !w.is_finite() || *w <= 0.0)
            || self.jacobian.iter().any(|x| !x.is_finite())
            || [
                &self.model_id,
                &self.source_id,
                &self.clock_id,
                &self.norm_id,
            ]
            .iter()
            .any(|s| s.trim().is_empty())
        {
            Err("invalid point matrix, clock or binding".into())
        } else {
            Ok(())
        }
    }
    pub fn apply(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        if x.len() != self.n || out.len() != self.n {
            return Err("operator dimensions".into());
        }
        for i in 0..self.n {
            let mut s = 0.0;
            for (j, &xj) in x.iter().enumerate() {
                s = self.jacobian[i * self.n + j].mul_add(xj, s)
            }
            out[i] = x[i] - self.sigma * s;
            if !out[i].is_finite() {
                return Err("nonfinite operator action".into());
            }
        }
        Ok(())
    }
    pub fn mu_upper(&self) -> Result<f64, String> {
        self.validate()?;
        check_arithmetic()?;
        let mut maximum = f64::NEG_INFINITY;
        for i in 0..self.n {
            let mut row = Interval::point(self.jacobian[i * self.n + i])?;
            for j in 0..self.n {
                if i != j {
                    let x = Interval::point(self.jacobian[i * self.n + j].abs())?;
                    let y = Interval::point(self.weights[j])?;
                    let z = Interval::point(self.weights[i])?;
                    row = row.add_out(x.mul_out(y)?.div_pos(z)?)?;
                }
            }
            maximum = maximum.max(row.hi);
        }
        if maximum.is_finite() {
            Ok(maximum)
        } else {
            Err("nonfinite lognorm".into())
        }
    }
    pub fn inverse_bound(&self) -> Result<f64, String> {
        let m = self.mu_upper()?;
        let d = Interval::point(1.0)?
            .sub_out(Interval::point(self.sigma)?.mul_out(Interval::point(m)?)?)?;
        if d.lo <= 0.0 {
            return Err("sigma*mu>=1 or unproved margin".into());
        }
        Ok(Interval::point(1.0)?.div_pos(d)?.hi)
    }
    pub fn residual_upper(&self, rhs: &[f64], x: &[f64]) -> Result<f64, String> {
        self.validate()?;
        check_arithmetic()?;
        if rhs.len() != self.n || x.len() != self.n || rhs.iter().chain(x).any(|v| !v.is_finite()) {
            return Err("invalid residual inputs".into());
        }
        let mut max = 0.0f64;
        for i in 0..self.n {
            let mut jx = Interval::point(0.0)?;
            for (j, &xj) in x.iter().enumerate() {
                jx = jx.add_out(
                    Interval::point(self.jacobian[i * self.n + j])?
                        .mul_out(Interval::point(xj)?)?,
                )?;
            }
            let ri = Interval::point(rhs[i])?
                .sub_out(Interval::point(x[i])?)?
                .add_out(Interval::point(self.sigma)?.mul_out(jx)?)?;
            let wi = Interval::point(self.weights[i])?;
            max = max.max(Interval::point(ri.abs_upper())?.div_pos(wi)?.hi);
        }
        if max.is_finite() {
            Ok(max)
        } else {
            Err("nonfinite residual bound".into())
        }
    }
    pub fn certify(
        &self,
        rhs: &[f64],
        x: &[f64],
        tau: f64,
        stage_budget: f64,
    ) -> Result<Admission, String> {
        if !tau.is_finite() || tau < 0.0 || !stage_budget.is_finite() || stage_budget < 0.0 {
            return Err("invalid budgets".into());
        }
        let rho = self.residual_upper(rhs, x)?;
        let k = self.inverse_bound()?;
        let forward = Interval::point(k)?.mul_out(Interval::point(rho)?)?.hi;
        Ok(Admission {
            accepted: rho <= tau && forward <= stage_budget,
            residual_upper: rho,
            inverse_upper: k,
            forward_upper: forward,
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Admission {
    pub accepted: bool,
    pub residual_upper: f64,
    pub inverse_upper: f64,
    pub forward_upper: f64,
}
#[derive(Debug, Clone, Copy)]
pub enum TriggerMode {
    Prefix { projected_tau: f64, ratio: f64 },
    Always,
    EndOnly,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Work {
    pub arnoldi_columns: usize,
    pub operator_actions: usize,
    pub mgs_dots: usize,
    pub candidate_checks: usize,
    pub refused_checks: usize,
    pub backsolves: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct SolveOutcome {
    pub accepted: bool,
    pub work: Work,
    pub certificate: Option<Admission>,
    pub projected_last: f64,
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn norm(x: &[f64]) -> f64 {
    dot(x, x).sqrt()
}
fn backsolve(h: &[Vec<f64>], g: &[f64], k: usize) -> Result<Vec<f64>, String> {
    let mut y = vec![0.0; k];
    for i in (0..k).rev() {
        let mut rhs = g[i];
        for j in i + 1..k {
            rhs -= h[i][j] * y[j]
        }
        let d = h[i][i];
        if !d.is_finite() || d.abs() < f64::MIN_POSITIVE {
            return Err("singular prefix R".into());
        }
        y[i] = rhs / d;
        if !y[i].is_finite() {
            return Err("nonfinite prefix correction".into());
        }
    }
    Ok(y)
}
/// Single Arnoldi cycle with identity preconditioner. The *original* residual
/// gate is the only admission authority; the Givens projected value is a trigger.
/// All buffers, including caller output, are transactional on non-admission.
#[derive(Debug, Clone, Copy)]
pub struct SolveConfig {
    pub max_columns: usize,
    pub tau_inf: f64,
    pub stage_budget: f64,
    pub mode: TriggerMode,
}
pub fn solve_into(
    model: &BoundModel,
    rhs: &[f64],
    x0: &[f64],
    output: &mut [f64],
    cfg: SolveConfig,
) -> Result<SolveOutcome, String> {
    let SolveConfig {
        max_columns,
        tau_inf,
        stage_budget,
        mode,
    } = cfg;
    model.validate()?;
    check_arithmetic()?;
    let n = model.n;
    if rhs.len() != n
        || x0.len() != n
        || output.len() != n
        || max_columns == 0
        || max_columns > n
        || rhs.iter().chain(x0).any(|x| !x.is_finite())
    {
        return Err("invalid solve input or Arnoldi budget".into());
    }
    if let TriggerMode::Prefix {
        projected_tau,
        ratio,
    } = mode
        && (!projected_tau.is_finite() || projected_tau < 0.0 || !ratio.is_finite() || ratio <= 0.0)
    {
        return Err("invalid prefix trigger".into());
    }
    let mut work = Work::default();
    let mut ax0 = vec![0.0; n];
    model.apply(x0, &mut ax0)?;
    work.operator_actions += 1;
    let r0: Vec<f64> = rhs.iter().zip(&ax0).map(|(a, b)| a - b).collect();
    let beta = norm(&r0);
    if !beta.is_finite() {
        return Err("nonfinite initial residual".into());
    }
    let init = model.certify(rhs, x0, tau_inf, stage_budget)?;
    work.candidate_checks += 1;
    if init.accepted {
        output.copy_from_slice(x0);
        return Ok(SolveOutcome {
            accepted: true,
            work,
            certificate: Some(init),
            projected_last: beta,
        });
    }
    if beta <= 0.0 {
        return Ok(SolveOutcome {
            accepted: false,
            work,
            certificate: Some(init),
            projected_last: beta,
        });
    }
    let mut v = vec![vec![0.0; n]; max_columns + 1];
    for i in 0..n {
        v[0][i] = r0[i] / beta
    }
    let mut h = vec![vec![0.0; max_columns]; max_columns + 1];
    let mut cs = vec![0.0; max_columns];
    let mut sn = vec![0.0; max_columns];
    let mut g = vec![0.0; max_columns + 1];
    g[0] = beta;
    let mut last = beta;
    let mut last_cert = Some(init);
    for j in 0..max_columns {
        let mut w = vec![0.0; n];
        model.apply(&v[j], &mut w)?;
        work.operator_actions += 1;
        work.arnoldi_columns += 1;
        let original_norm = norm(&w);
        for _pass in 0..2 {
            for i in 0..=j {
                let hij = dot(&v[i], &w);
                work.mgs_dots += 1;
                h[i][j] += hij;
                for t in 0..n {
                    w[t] -= hij * v[i][t]
                }
            }
        }
        let hn = norm(&w);
        h[j + 1][j] = hn;
        let breakdown = hn <= 64.0 * f64::EPSILON * original_norm.max(f64::MIN_POSITIVE);
        if !breakdown {
            for i in 0..n {
                v[j + 1][i] = w[i] / hn
            }
        }
        for i in 0..j {
            let t = cs[i] * h[i][j] + sn[i] * h[i + 1][j];
            h[i + 1][j] = -sn[i] * h[i][j] + cs[i] * h[i + 1][j];
            h[i][j] = t
        }
        let d = h[j][j].hypot(h[j + 1][j]);
        if d == 0.0 || !d.is_finite() {
            return Err("zero or nonfinite Givens diagonal".into());
        }
        cs[j] = h[j][j] / d;
        sn[j] = h[j + 1][j] / d;
        h[j][j] = d;
        h[j + 1][j] = 0.0;
        g[j + 1] = -sn[j] * g[j];
        g[j] *= cs[j];
        last = g[j + 1].abs();
        let trigger = match mode {
            TriggerMode::Prefix {
                projected_tau,
                ratio,
            } => last <= projected_tau * ratio,
            TriggerMode::Always => true,
            TriggerMode::EndOnly => false,
        };
        if trigger || breakdown || j + 1 == max_columns {
            work.backsolves += 1;
            if let Ok(y) = backsolve(&h, &g, j + 1) {
                let mut candidate = x0.to_vec();
                for k in 0..=j {
                    for i in 0..n {
                        candidate[i] += y[k] * v[k][i]
                    }
                }
                if candidate.iter().all(|x| x.is_finite()) {
                    work.candidate_checks += 1;
                    let c = model.certify(rhs, &candidate, tau_inf, stage_budget)?;
                    last_cert = Some(c);
                    if c.accepted {
                        output.copy_from_slice(&candidate);
                        return Ok(SolveOutcome {
                            accepted: true,
                            work,
                            certificate: Some(c),
                            projected_last: last,
                        });
                    }
                    work.refused_checks += 1;
                } else {
                    work.refused_checks += 1
                }
            } else {
                work.refused_checks += 1
            }
        }
        if breakdown {
            break;
        }
    }
    Ok(SolveOutcome {
        accepted: false,
        work,
        certificate: last_cert,
        projected_last: last,
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSign {
    Positive,
    Negative,
    Unresolved,
}
/// Event at one algebraic stage state only; this is NOT a time-crossing proof.
pub fn event_sign(
    model: &BoundModel,
    x: &[f64],
    c: &[f64],
    threshold: f64,
    error_upper: f64,
) -> Result<EventSign, String> {
    model.validate()?;
    check_arithmetic()?;
    if x.len() != model.n
        || c.len() != model.n
        || !threshold.is_finite()
        || !error_upper.is_finite()
        || error_upper < 0.0
    {
        return Err("invalid event".into());
    }
    let mut val = Interval::point(-threshold)?;
    let mut dual = Interval::point(0.0)?;
    for i in 0..model.n {
        val = val.add_out(Interval::point(c[i])?.mul_out(Interval::point(x[i])?)?)?;
        dual = dual
            .add_out(Interval::point(c[i].abs())?.mul_out(Interval::point(model.weights[i])?)?)?;
    }
    let rad = dual.mul_out(Interval::point(error_upper)?)?;
    let range = val.add_out(Interval {
        lo: -rad.hi,
        hi: rad.hi,
    })?;
    Ok(if range.lo > 0.0 {
        EventSign::Positive
    } else if range.hi < 0.0 {
        EventSign::Negative
    } else {
        EventSign::Unresolved
    })
}
