//! VigilODE opt-in coupled dissipative cubic BE with source-owned inverse-Picard.
//!
//! Mathematical source: y_i'=-k_i*y_i-g_i*y_i^3+sum_j c_ij(y_j-y_i).
//! Only value evaluations and per-component algebraic cubic inverses are used.
//! No Jacobian, JVP, AD, Newton or replacement of the original RODAS5P solver.
//! Entire original BE stage residual and all returned physical errors use
//! source-matched outward Interval arithmetic. This is research-scoped BE order 1.

use rodas5p_core::directed::{Interval, add_down, add_up, div_up, mul_down, mul_up, sub_down};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Order {
    BaselineFirst,
    FastFirst,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FailureKind {
    Input,
    Arithmetic,
    Invariant,
    Contraction,
    Truncation,
    Domain,
    Budget,
    Iteration,
    Physical,
}
#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
}
fn fail(kind: FailureKind, message: impl Into<String>) -> Failure {
    Failure {
        kind,
        message: message.into(),
    }
}
fn arithmetic(e: impl std::fmt::Display) -> Failure {
    fail(FailureKind::Arithmetic, e.to_string())
}
fn ip(x: f64) -> Result<Interval, Failure> {
    Interval::point(x).map_err(arithmetic)
}
fn up(a: f64, b: f64) -> Result<f64, Failure> {
    add_up(a, b).map_err(arithmetic)
}
fn down(a: f64, b: f64) -> Result<f64, Failure> {
    sub_down(a, b).map_err(arithmetic)
}
fn mu(a: f64, b: f64) -> Result<f64, Failure> {
    mul_up(a, b).map_err(arithmetic)
}
fn du(a: f64, b: f64) -> Result<f64, Failure> {
    div_up(a, b).map_err(arithmetic)
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Config {
    pub steps: usize,
    pub horizon: f64,
    pub physical_tolerance: f64,
    pub max_map_evaluations: u64,
    pub max_iterations_per_step: usize,
    pub order: Order,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            steps: 128,
            horizon: 0.1,
            physical_tolerance: 0.00712,
            max_map_evaluations: 200_000,
            max_iterations_per_step: 90,
            order: Order::FastFirst,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Work {
    pub map_attempted: u64,
    pub map_completed: u64,
    pub original_interval_components: u64,
    pub inverse_interval_components: u64,
    pub algebraic_cubic_candidates: u64,
    pub picard_updates: u64,
    pub speculative_trials: u64,
    pub accepted_trials: u64,
    pub rejected_trials: u64,
    pub domain_skips: u64,
    pub source_steps: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Point {
    pub time: f64,
    pub state: Vec<f64>,
    pub original_be_residual_upper: f64,
    pub source_q_upper: f64,
    pub physical_error_upper: f64,
    pub map_evaluations: u64,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Attempt {
    pub completed: bool,
    pub points: Vec<Point>,
    pub work: Work,
    pub failure: Option<Failure>,
    pub truncation_majorant: Option<f64>,
    pub original_stage_residual_target: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct Source {
    k: Vec<f64>,
    g: Vec<f64>,
    c: Vec<Vec<f64>>,
    c_upper: Vec<f64>,
    y0: Vec<f64>,
    m0: f64,
    min_k: f64,
}
#[derive(Clone)]
struct Value {
    x: Vec<f64>,
    mapped: Vec<f64>,
    op_r: Vec<f64>,
    map_lower: f64,
    map_upper: f64,
    original_rho: f64,
}
struct Stage<'a> {
    src: &'a Source,
    prev: &'a [f64],
    h: f64,
    h_int: Interval,
    a: Vec<Interval>,
    b: Vec<Interval>,
    q: f64,
    residual_target: f64,
}
impl Source {
    pub fn new(k: Vec<f64>, g: Vec<f64>, c: Vec<Vec<f64>>, y0: Vec<f64>) -> Result<Self, Failure> {
        let n = k.len();
        if !(2..=16).contains(&n)
            || g.len() != n
            || c.len() != n
            || y0.len() != n
            || c.iter().any(|row| row.len() != n)
            || k.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || g.iter().any(|v| !v.is_finite() || *v < 0.0)
            || y0.iter().any(|v| !v.is_finite() || *v < 0.0)
            || c.iter().enumerate().any(|(i, row)| {
                row.iter()
                    .enumerate()
                    .any(|(j, v)| !v.is_finite() || *v < 0.0 || (i == j && *v != 0.0))
            })
        {
            return Err(fail(
                FailureKind::Input,
                "invalid dimension, positivity or frozen source",
            ));
        }
        let m0 = y0.iter().fold(0.0_f64, |a, &v| a.max(v));
        if m0 <= 0.0 {
            return Err(fail(
                FailureKind::Input,
                "positive source initial amplitude required",
            ));
        }
        let min_k = k.iter().copied().fold(f64::INFINITY, f64::min);
        let mut c_upper = Vec::with_capacity(n);
        for row in &c {
            let mut sum = 0.0;
            for &v in row {
                sum = up(sum, v)?;
            }
            c_upper.push(sum);
        }
        Ok(Self {
            k,
            g,
            c,
            c_upper,
            y0,
            m0,
            min_k,
        })
    }
    pub fn dimension(&self) -> usize {
        self.k.len()
    }
    pub fn source_bits(&self) -> Vec<u64> {
        let n = self.dimension();
        let mut out = Vec::with_capacity(3 * n + n * n);
        out.extend(self.k.iter().map(|v| v.to_bits()));
        out.extend(self.g.iter().map(|v| v.to_bits()));
        out.extend(self.y0.iter().map(|v| v.to_bits()));
        for row in &self.c {
            out.extend(row.iter().map(|v| v.to_bits()));
        }
        out
    }
    pub fn run(&self, cfg: Config) -> Attempt {
        let mut out = Attempt::default();
        match self.integrate(cfg, &mut out) {
            Ok(()) => out.completed = true,
            Err(e) => out.failure = Some(e),
        }
        out
    }
    fn integrate(&self, cfg: Config, out: &mut Attempt) -> Result<(), Failure> {
        if cfg.steps == 0
            || !cfg.steps.is_power_of_two()
            || cfg.steps > (1usize << 52)
            || !cfg.horizon.is_finite()
            || cfg.horizon <= 0.0
            || !cfg.physical_tolerance.is_finite()
            || cfg.physical_tolerance <= 0.0
            || cfg.max_map_evaluations == 0
            || cfg.max_iterations_per_step == 0
        {
            return Err(fail(
                FailureKind::Input,
                "invalid exactly indexed physical grid/budget",
            ));
        }
        let h = cfg.horizon / cfg.steps as f64;
        if !h.is_finite() || h <= 0.0 || h * cfg.steps as f64 != cfg.horizon {
            return Err(fail(
                FailureKind::Input,
                "unrepresentable step-clock endpoint",
            ));
        }
        let h_int = ip(h)?;
        let min_a = ip(1.0)?
            .add(h_int.mul(ip(self.min_k)?).map_err(arithmetic)?)
            .map_err(arithmetic)?;
        if min_a.lo <= 1.0 {
            return Err(fail(
                FailureKind::Arithmetic,
                "inverse dissipation factor rounded below unity",
            ));
        }
        let a_lower = min_a.lo;
        let n = self.dimension();
        let mut major = self.m0;
        let mut trunc = 0.0;
        let mut trunc_max = 0.0_f64;
        let mut defects = Vec::with_capacity(cfg.steps);
        let mut clocks = Vec::with_capacity(cfg.steps);
        let p0 = self.p_upper(self.m0)?;
        for i in 1..=cfg.steps {
            let l = self.l_upper(major)?;
            let p = self.p_upper(major)?;
            let defect = du(mu(mu(h, h)?, mu(l, p)?)?, 2.0)?;
            trunc = du(up(trunc, defect)?, a_lower)?;
            let label = h * i as f64;
            let exact = h_int.scale(i as f64).map_err(arithmetic)?;
            let dt = exact.sub(ip(label)?).map_err(arithmetic)?.mag();
            let clock = mu(p0, dt)?;
            trunc_max = trunc_max.max(up(trunc, clock)?);
            defects.push(defect);
            clocks.push(clock);
            major = du(major, a_lower)?;
        }
        out.truncation_majorant = Some(trunc_max);
        if trunc_max >= cfg.physical_tolerance {
            return Err(fail(
                FailureKind::Truncation,
                "analytical BE truncation upper exceeds physical target",
            ));
        }
        let remaining = down(cfg.physical_tolerance, trunc_max)?;
        let residual_target = mul_down(remaining, mul_down(h, self.min_k).map_err(arithmetic)?)
            .map_err(arithmetic)?;
        if residual_target <= 0.0 {
            return Err(fail(
                FailureKind::Truncation,
                "no positive original residual budget",
            ));
        }
        out.original_stage_residual_target = Some(residual_target);
        let mut state = self.y0.clone();
        let mut error = 0.0_f64;
        out.points.push(Point {
            time: 0.0,
            state: state.clone(),
            original_be_residual_upper: 0.0,
            source_q_upper: 0.0,
            physical_error_upper: 0.0,
            map_evaluations: 0,
        });
        for i in 0..cfg.steps {
            let before = out.work.map_attempted;
            let stage = Stage::new(self, &state, h, h_int, residual_target)?;
            let candidate = stage.solve(cfg, &mut out.work)?;
            error = du(up(up(error, defects[i])?, candidate.original_rho)?, a_lower)?;
            let error_at_label = up(error, clocks[i])?;
            if error_at_label > cfg.physical_tolerance {
                return Err(fail(
                    FailureKind::Physical,
                    "whole-IVP original-source error exceeds requested tolerance",
                ));
            }
            if candidate
                .x
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0 || *v > self.m0)
            {
                return Err(fail(
                    FailureKind::Domain,
                    "committed coupled state left certified box",
                ));
            }
            state = candidate.x;
            out.work.source_steps += 1;
            out.points.push(Point {
                time: h * (i + 1) as f64,
                state: state.clone(),
                original_be_residual_upper: candidate.original_rho,
                source_q_upper: stage.q,
                physical_error_upper: error_at_label,
                map_evaluations: out.work.map_attempted - before,
            });
        }
        debug_assert_eq!(out.points.len(), cfg.steps + 1);
        debug_assert_eq!(n, self.dimension());
        Ok(())
    }
    fn l_upper(&self, m: f64) -> Result<f64, Failure> {
        let m2 = mu(m, m)?;
        let mut l = 0.0_f64;
        for i in 0..self.dimension() {
            let v = up(
                up(self.k[i], mu(3.0, mu(self.g[i], m2)?)?)?,
                mu(2.0, self.c_upper[i])?,
            )?;
            l = l.max(v);
        }
        Ok(l)
    }
    fn p_upper(&self, m: f64) -> Result<f64, Failure> {
        let m2 = mu(m, m)?;
        let m3 = mu(m, m2)?;
        let mut p = 0.0_f64;
        for i in 0..self.dimension() {
            let v = up(
                up(mu(self.k[i], m)?, mu(self.g[i], m3)?)?,
                mu(mu(2.0, self.c_upper[i])?, m)?,
            )?;
            p = p.max(v);
        }
        Ok(p)
    }
}
impl<'a> Stage<'a> {
    fn new(
        src: &'a Source,
        prev: &'a [f64],
        h: f64,
        h_int: Interval,
        residual_target: f64,
    ) -> Result<Self, Failure> {
        let mut a = Vec::with_capacity(src.dimension());
        let mut b = Vec::with_capacity(src.dimension());
        let mut q = 0.0_f64;
        for i in 0..src.dimension() {
            let mut csum = ip(0.0)?;
            for &cij in &src.c[i] {
                csum = csum.add(ip(cij)?).map_err(arithmetic)?;
            }
            let ai = ip(1.0)?
                .add(
                    h_int
                        .mul(ip(src.k[i])?.add(csum).map_err(arithmetic)?)
                        .map_err(arithmetic)?,
                )
                .map_err(arithmetic)?;
            let bi = h_int.mul(ip(src.g[i])?).map_err(arithmetic)?;
            if ai.lo <= 1.0 {
                return Err(fail(
                    FailureKind::Arithmetic,
                    "source diagonal lower not above unity",
                ));
            }
            let qi = du(mu(h, csum.hi)?, ai.lo)?;
            q = q.max(qi);
            a.push(ai);
            b.push(bi);
        }
        if q >= 1.0 {
            return Err(fail(
                FailureKind::Contraction,
                "source-owned q rounded up to unity",
            ));
        }
        if prev.len() != src.dimension()
            || prev
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0 || *v > src.m0)
        {
            return Err(fail(
                FailureKind::Invariant,
                "previous state outside source-owned invariant box",
            ));
        }
        Ok(Self {
            src,
            prev,
            h,
            h_int,
            a,
            b,
            q,
            residual_target,
        })
    }
    fn evaluate(&self, x: Vec<f64>, cfg: Config, work: &mut Work) -> Result<Value, Failure> {
        let n = self.src.dimension();
        if x.len() != n
            || x.iter()
                .any(|v| !v.is_finite() || *v < 0.0 || *v > self.src.m0)
        {
            return Err(fail(
                FailureKind::Domain,
                "candidate outside source-owned positive box",
            ));
        }
        if work.map_attempted >= cfg.max_map_evaluations {
            return Err(fail(
                FailureKind::Budget,
                "map budget exhausted before source dispatch",
            ));
        }
        work.map_attempted += 1;
        let mut mapped = Vec::with_capacity(n);
        let mut op_r = Vec::with_capacity(n);
        let mut lower = 0.0_f64;
        let mut upper = 0.0_f64;
        let mut rho = 0.0_f64;
        for i in 0..n {
            // d_i = v_i + h sum_j c_ij*x_j; both operational candidate and
            // exact source-interval evaluation are computed independently.
            let mut d_float = self.prev[i];
            let mut d_int = ip(self.prev[i])?;
            let mut row_c = 0.0_f64;
            for j in 0..n {
                let cij = self.src.c[i][j];
                row_c += cij;
                d_float += self.h * cij * x[j];
                d_int = d_int
                    .add(
                        self.h_int
                            .mul(ip(cij)?.mul(ip(x[j])?).map_err(arithmetic)?)
                            .map_err(arithmetic)?,
                    )
                    .map_err(arithmetic)?;
            }
            let a_f = 1.0 + self.h * (self.src.k[i] + row_c);
            let b_f = self.h * self.src.g[i];
            if !a_f.is_finite()
                || a_f <= 0.0
                || !b_f.is_finite()
                || b_f < 0.0
                || !d_float.is_finite()
                || d_float < 0.0
            {
                return Err(fail(
                    FailureKind::Arithmetic,
                    "operational scalar inverse input invalid",
                ));
            }
            let w = if b_f == 0.0 || d_float == 0.0 {
                d_float / a_f
            } else {
                let beta = (b_f * d_float * d_float) / (a_f * a_f * a_f);
                if !beta.is_finite() || beta < 0.0 {
                    return Err(fail(FailureKind::Arithmetic, "inverse beta overflow"));
                }
                let z = 1.5 * (3.0 * beta).sqrt();
                let u = (z + z.hypot(1.0)).cbrt();
                (d_float / a_f) * (3.0 / (u * u + 1.0 + 1.0 / (u * u)))
            };
            if !w.is_finite() {
                return Err(fail(FailureKind::Arithmetic, "inverse candidate nonfinite"));
            }
            let w = w.clamp(0.0, self.src.m0);
            work.algebraic_cubic_candidates += u64::from(b_f > 0.0);
            let zi = ip(x[i])?;
            let wi = ip(w)?;
            let sigma = self.a[i]
                .mul(wi)
                .and_then(|v| v.add(self.b[i].mul(wi.mul(wi).and_then(|t| t.mul(wi))?)?))
                .and_then(|v| v.sub(d_int))
                .map_err(arithmetic)?;
            let delta = du(sigma.mag(), self.a[i].lo)?;
            let gap = Interval::exact_sum(w, -x[i]).map_err(arithmetic)?;
            let li = down(gap.mig(), delta)?.max(0.0);
            let ui = up(gap.mag(), delta)?;
            lower = lower.max(li);
            upper = upper.max(ui);
            let zi3 = zi.mul(zi).and_then(|v| v.mul(zi)).map_err(arithmetic)?;
            let orig = self.a[i]
                .mul(zi)
                .and_then(|v| v.add(self.b[i].mul(zi3)?))
                .and_then(|v| v.sub(d_int))
                .map_err(arithmetic)?;
            rho = rho.max(orig.mag());
            mapped.push(w);
            op_r.push(w - x[i]);
            work.original_interval_components += 1;
            work.inverse_interval_components += 1;
        }
        work.map_completed += 1;
        Ok(Value {
            x,
            mapped,
            op_r,
            map_lower: lower,
            map_upper: upper,
            original_rho: rho,
        })
    }
    fn solve(&self, cfg: Config, work: &mut Work) -> Result<Value, Failure> {
        let n = self.src.dimension();
        let mut current = self.evaluate(vec![0.0; n], cfg, work)?;
        let mut previous: Option<Value> = None;
        for _ in 0..cfg.max_iterations_per_step {
            if current.original_rho <= self.residual_target {
                return Ok(current);
            }
            let mut baseline: Option<Value> = None;
            let mut chosen: Option<Value> = None;
            if cfg.order == Order::BaselineFirst {
                work.picard_updates += 1;
                let p = self.evaluate(current.mapped.clone(), cfg, work)?;
                if p.original_rho <= self.residual_target {
                    chosen = Some(p)
                } else {
                    baseline = Some(p)
                };
            }
            if chosen.is_none() {
                if let Some(old) = &previous {
                    let scale = current
                        .op_r
                        .iter()
                        .chain(old.op_r.iter())
                        .fold(0.0_f64, |acc, &v| acc.max(v.abs()));
                    let (mut num, mut den) = (0.0_f64, 0.0_f64);
                    if scale > 0.0 {
                        for (&r, &r_old) in current.op_r.iter().zip(&old.op_r) {
                            let ri = r / scale;
                            let dr = ri - r_old / scale;
                            num += ri * dr;
                            den += dr * dr;
                        }
                    }
                    let gamma = num / den;
                    if gamma.is_finite() {
                        let trial: Vec<_> = current
                            .mapped
                            .iter()
                            .zip(&old.mapped)
                            .map(|(&v, &w)| v - gamma * (v - w))
                            .collect();
                        if trial
                            .iter()
                            .all(|v| v.is_finite() && *v >= 0.0 && *v <= self.src.m0)
                        {
                            if cfg.order == Order::BaselineFirst
                                || cfg.max_map_evaluations.saturating_sub(work.map_attempted) >= 2
                            {
                                work.speculative_trials += 1;
                                let candidate = self.evaluate(trial, cfg, work)?;
                                let better = match cfg.order {
                                    Order::BaselineFirst => {
                                        candidate.map_upper
                                            <= baseline
                                                .as_ref()
                                                .expect("previously evaluated baseline")
                                                .map_lower
                                    }
                                    Order::FastFirst => {
                                        candidate.map_upper
                                            <= mul_down(self.q, current.map_lower)
                                                .map_err(arithmetic)?
                                    }
                                };
                                if candidate.original_rho <= self.residual_target || better {
                                    work.accepted_trials += 1;
                                    chosen = Some(candidate);
                                } else {
                                    work.rejected_trials += 1;
                                }
                            }
                        } else {
                            work.domain_skips += 1;
                        }
                    }
                }
            }
            if chosen.is_none() {
                if let Some(b) = baseline {
                    chosen = Some(b);
                } else {
                    work.picard_updates += 1;
                    chosen = Some(self.evaluate(current.mapped.clone(), cfg, work)?);
                }
            }
            previous = Some(current);
            current = chosen.expect("every successful update must select a source value");
        }
        Err(fail(
            FailureKind::Iteration,
            "source-specific algebraic root cap",
        ))
    }
}
