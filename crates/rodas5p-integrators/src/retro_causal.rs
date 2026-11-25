//! Source-owned scalar affine research controller. Forecasts allocate work,
//! not future boundary values. The native causal RODAS5P solver is unchanged.
//! Exact represented piecewise-constant equations and absolute budgets only.
//! Endpoint and ideal dense-path certificates are distinct; returned queries
//! include evaluation error. Nonterminal event brackets do not stop the run.
use crate::{
    AdaptiveStepConfig, FastDenseSegments, FastInteriorControl, OdeProblem,
    affine_event_certificate::{
        AFFINE_EVENT_CLOCK_ID, AffineEventBinding, AffineEventSpec, AffineScalarModel,
        check_affine_event_arithmetic,
    },
    integrate_rodas5p_fast_dense_observed,
    retro_green::RetroTable,
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, WorkCounters,
    directed::{Interval, add_up, div_down, mul_up},
};
use std::sync::Arc;

type Iv = Interval;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro causal: {s}"))
}
fn failed(s: &str) -> CoreError {
    CoreError::NonlinearSolve(format!("retro causal: {s}"))
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn width(start: f64, end: f64) -> CoreResult<Iv> {
    pt(end)?.sub(pt(start)?)
}
fn nonnegative(x: Iv) -> CoreResult<Iv> {
    Iv::new(x.lo.max(0.), x.hi)
}
pub(crate) fn gain(table: &RetroTable, rate: f64, h: Iv) -> CoreResult<f64> {
    if rate == 0. {
        return Ok(1.);
    }
    let x = nonnegative(h.scale(rate)?)?;
    Ok(table.phi(x, 0)?.hi.min(1.))
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct SlabSpec {
    pub end: f64,
    pub rate: f64,
    pub forcing: f64,
    pub forecast: f64,
    pub radius: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum OutputKind {
    Endpoint,
    Dense,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Target {
    pub slab: usize,
    pub kind: OutputKind,
    pub tolerance: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Mode {
    Forward,
    Uniform,
    Retro,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Plan {
    pub budgets: Vec<f64>,
    pub gain_upper: Vec<f64>,
    pub forecast_fallback: bool,
    pub uniform_fallback: bool,
    pub planned_target_upper: Vec<f64>,
}
fn row(g: &[f64], t: Target) -> CoreResult<Vec<f64>> {
    let mut w = vec![0.; g.len()];
    w[t.slab] = 1.;
    let mut v = if t.kind == OutputKind::Dense {
        1.
    } else {
        g[t.slab]
    };
    for i in (0..t.slab).rev() {
        w[i] = v;
        v = mul_up(v, g[i])?.min(1.);
    }
    Ok(w)
}
fn dot(w: &[f64], b: &[f64]) -> CoreResult<f64> {
    w.iter()
        .zip(b)
        .try_fold(0., |s, (&x, &y)| add_up(s, mul_up(x, y)?))
}
fn predicted_cost(b: &[f64]) -> f64 {
    b.iter().map(|x| x.powf(-0.25)).sum()
}
/// The forecast is a caller-declared deterministic interval, checked
/// against this source-owned model before using it. For an arbitrary RHS
/// this check cannot be inferred from a trajectory sample.
pub fn plan(
    table: &RetroTable,
    slabs: &[SlabSpec],
    targets: &[Target],
    mode: Mode,
    local_cap: f64,
) -> CoreResult<Plan> {
    check_affine_event_arithmetic()?;
    let n = slabs.len();
    if n == 0
        || n > 256
        || targets.is_empty()
        || targets.len() > 512
        || !local_cap.is_finite()
        || local_cap <= 0.
        || targets
            .iter()
            .any(|t| t.slab >= n || !t.tolerance.is_finite() || t.tolerance <= 0.)
        || !targets
            .iter()
            .any(|t| t.slab == n - 1 && t.kind == OutputKind::Endpoint)
    {
        return Err(bad(
            "invalid dimensions, tolerance, cap or missing final endpoint",
        ));
    }
    let mut gs = Vec::with_capacity(n);
    let mut start = 0.;
    let mut fallback = false;
    for s in slabs {
        if ![s.end, s.rate, s.forcing, s.forecast, s.radius]
            .iter()
            .all(|x| x.is_finite())
            || s.end <= start
            || s.rate <= 0.
            || s.forecast < 0.
            || s.radius < 0.
        {
            return Err(bad("slab source or forecast outside domain"));
        }
        let h = width(start, s.end)?;
        if h.lo <= 0. {
            return Err(bad("nonpositive exact clock width"));
        }
        // Require the represented rate to be inside the exact declared box.
        // If an interval comparison is inconclusive, use the safe unity arm.
        let lower = pt(s.forecast)?.sub(pt(s.radius)?)?;
        let upper = pt(s.forecast)?.add(pt(s.radius)?)?;
        if s.rate < lower.hi || s.rate > upper.lo {
            fallback = true;
        }
        if nonnegative(h.scale(s.rate)?)?.hi > 16384. {
            return Err(bad("source decay argument exceeds table domain"));
        }
        let lo = lower.lo.max(0.);
        gs.push(if mode == Mode::Forward || fallback {
            1.
        } else {
            gain(table, lo, h)?
        });
        start = s.end;
    }
    if mode == Mode::Forward || fallback {
        gs.fill(1.);
    }
    let rows = targets
        .iter()
        .map(|&t| row(&gs, t))
        .collect::<CoreResult<Vec<_>>>()?;
    // 2% reserve for directed accumulation and downstream roundoff. It is
    // not an empirical error estimate, and final constraints are rechecked.
    let mut uniform = local_cap;
    let mut candidate = vec![local_cap; n];
    for (t, w) in targets.iter().zip(&rows) {
        let cap = div_down(
            rodas5p_core::directed::mul_down(0.98, t.tolerance)?,
            dot(w, &vec![1.; n])?,
        )?;
        uniform = uniform.min(cap);
        let sum: f64 = w[..=t.slab].iter().map(|x| x.powf(0.2)).sum();
        // p=4 cost proxy, single-target optimum for uncapped equal cost.
        // Min across target proposals is feasible, not generally optimal.
        for i in 0..=t.slab {
            if w[i] > 0. {
                let b = (0.98 * t.tolerance / sum) * w[i].powf(-0.8);
                if b.is_nan() {
                    return Err(bad("allocation proposal not finite"));
                }
                candidate[i] = candidate[i].min(b);
            }
        }
    }
    let uniform_vec = vec![uniform; n];
    let feasible = |b: &[f64]| -> CoreResult<bool> {
        if b.iter().any(|x| !x.is_finite() || *x <= 0.) {
            return Ok(false);
        }
        for (w, t) in rows.iter().zip(targets) {
            if dot(w, b)? > t.tolerance {
                return Ok(false);
            }
        }
        Ok(true)
    };
    if !feasible(&uniform_vec)? {
        return Err(bad("uniform budget underflow or verification failure"));
    }
    let uniform_fallback = mode == Mode::Retro
        && (!feasible(&candidate)? || predicted_cost(&candidate) > predicted_cost(&uniform_vec));
    let budgets = if mode == Mode::Retro && !uniform_fallback {
        candidate
    } else {
        uniform_vec
    };
    let planned_target_upper = rows
        .iter()
        .map(|w| dot(w, &budgets))
        .collect::<CoreResult<_>>()?;
    Ok(Plan {
        budgets,
        gain_upper: gs,
        forecast_fallback: fallback,
        uniform_fallback,
        planned_target_upper,
    })
}

pub(crate) fn flow(table: &RetroTable, s: SlabSpec, start: f64, t: f64, y: f64) -> CoreResult<Iv> {
    if t == start {
        return pt(y);
    }
    let h = width(start, t)?;
    let x = nonnegative(h.scale(s.rate)?)?;
    let a = table.phi(x, 0)?;
    let b = h.mul(table.phi(x, 1)?)?.scale(s.forcing)?;
    a.scale(y)?.add(b)
}
fn endpoint_defect(
    table: &RetroTable,
    s: SlabSpec,
    start: f64,
    y0: f64,
    y: f64,
) -> CoreResult<f64> {
    Ok(pt(y)?.sub(flow(table, s, start, s.end, y0)?)?.mag())
}
fn problem(s: SlabSpec) -> CoreResult<OdeProblem> {
    OdeProblem::new(
        "retro-source-owned-affine",
        1,
        Arc::new(move |_, y, out| {
            out[0] = (-s.rate).mul_add(y[0], s.forcing);
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| DenseMatrix::new(1, 1, vec![-s.rate]))),
        None,
        None,
        true,
        None,
        None,
    )
}
fn event_spec(s: SlabSpec, i: usize) -> CoreResult<AffineEventSpec> {
    AffineEventSpec::new(
        AffineEventBinding::new(
            "retro-affine",
            "source-owned-constant",
            AFFINE_EVENT_CLOCK_ID,
            "caller-state-time",
            "unused-defect-threshold",
            i as u64,
        )?,
        AffineScalarModel::new(pt(s.rate)?, pt(s.forcing)?)?,
        0.,
    )
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Work {
    pub slab_trials: usize,
    pub slab_rejections: usize,
    pub endpoint_checks: usize,
    pub dense_checks: usize,
    pub native: WorkCounters,
}
#[derive(Clone, Debug)]
pub struct SlabRun {
    spec: SlabSpec,
    start: f64,
    initial: f64,
    endpoint: f64,
    incoming: f64,
    local_endpoint: f64,
    local_dense: Option<f64>,
    endpoint_upper: f64,
    dense_upper: Option<f64>,
    atol: f64,
    segments: FastDenseSegments,
}
impl SlabRun {
    pub fn start(&self) -> f64 {
        self.start
    }
    pub fn end(&self) -> f64 {
        self.spec.end
    }
    pub fn endpoint(&self) -> f64 {
        self.endpoint
    }
    pub fn incoming_upper(&self) -> f64 {
        self.incoming
    }
    pub fn local_endpoint_upper(&self) -> f64 {
        self.local_endpoint
    }
    pub fn local_dense_upper(&self) -> Option<f64> {
        self.local_dense
    }
    pub fn endpoint_upper(&self) -> f64 {
        self.endpoint_upper
    }
    /// Bound on the exact-real represented dense polynomial, not every
    /// rounded query. Use query() for a rounding-inclusive value.
    pub fn dense_upper(&self) -> Option<f64> {
        self.dense_upper
    }
    pub fn accepted_atol(&self) -> f64 {
        self.atol
    }
    pub fn native_steps(&self) -> usize {
        self.segments.len()
    }
    /// A posteriori error of the actual native dense value at this represented
    /// time, using the original source flow and incoming radius. This is
    /// source-specific; it does not certify arbitrary callbacks.
    pub fn query(&self, table: &RetroTable, t: f64) -> CoreResult<(f64, f64)> {
        check_affine_event_arithmetic()?;
        if !t.is_finite() || t < self.start || t > self.spec.end {
            return Err(bad("query outside slab"));
        }
        let mut x = [0.];
        self.segments.evaluate_into(t, &mut x)?;
        let local = pt(x[0])?
            .sub(flow(table, self.spec, self.start, t, self.initial)?)?
            .mag();
        let inherited = mul_up(
            gain(table, self.spec.rate, width(self.start, t)?)?,
            self.incoming,
        )?;
        Ok((x[0], add_up(local, inherited)?))
    }
}
#[derive(Clone, Debug)]
pub struct Run {
    pub plan: Plan,
    pub work: Work,
    pub slabs: Vec<SlabRun>,
}
/// All requested targets are independently rechecked before returning a
/// successful result. The caller cannot inject a modified allocation plan.
pub fn integrate(
    table: &RetroTable,
    slabs: &[SlabSpec],
    targets: &[Target],
    mode: Mode,
    local_cap: f64,
    y0: f64,
) -> CoreResult<Run> {
    if !y0.is_finite() {
        return Err(bad("initial state"));
    }
    let plan = plan(table, slabs, targets, mode, local_cap)?;
    let mut work = Work::default();
    let mut runs = Vec::with_capacity(slabs.len());
    let (mut start, mut initial, mut incoming) = (0., y0, 0.);
    for (i, &s) in slabs.iter().enumerate() {
        let p = problem(s)?;
        let dense = targets
            .iter()
            .any(|t| t.slab == i && t.kind == OutputKind::Dense);
        let sp = event_spec(s, i)?;
        let budget = plan.budgets[i];
        let mut atol = budget;
        let mut accepted = None;
        let h = s.end - start;
        for _ in 0..14 {
            if !atol.is_finite() || atol <= 0. {
                return Err(failed("retry tolerance underflow"));
            }
            let min_step = 1e-14_f64.min(h * 1e-8);
            if min_step <= 0. {
                return Err(bad("step resolution"));
            }
            let adapt = AdaptiveStepConfig {
                atol,
                rtol: 0.,
                initial_step: h.min(1. / s.rate).max(min_step),
                max_step: h,
                min_step,
                max_attempts: 100000,
                ..AdaptiveStepConfig::default()
            };
            work.slab_trials += 1;
            let r = integrate_rodas5p_fast_dense_observed(
                &p,
                (start, s.end),
                &[initial],
                &adapt,
                FastInteriorControl::Off,
            )?;
            work.native.accumulate(r.fast.observed.counters);
            if !r.fast.observed.success || r.segments.times().last() != Some(&s.end) {
                return Err(failed("native driver failed; no complete result"));
            }
            let end = r.segments.state(r.segments.len())[0];
            work.endpoint_checks += 1;
            let local = endpoint_defect(table, s, start, initial, end)?;
            let mut local_dense = None;
            if dense && local <= budget {
                let mut e = 0.0_f64;
                let mut largest = 0.0_f64;
                for k in 0..r.segments.len() {
                    work.dense_checks += 1;
                    let cert = r.segments.certify_affine_scalar_segment(k, &sp, e)?;
                    largest = largest.max(cert.whole_slab_error_upper());
                    e = cert.endpoint_error_upper();
                }
                local_dense = Some(largest.max(local));
            }
            if local <= budget && (!dense || local_dense.is_some_and(|e| e <= budget)) {
                accepted = Some((r.segments, end, local, local_dense, atol));
                break;
            }
            work.slab_rejections += 1;
            atol *= 0.25;
        }
        let (segments, end, local, local_dense, atol) =
            accepted.ok_or_else(|| failed("local certificate retry cap"))?;
        let endpoint_upper = add_up(mul_up(plan.gain_upper[i], incoming)?, local)?;
        let dense_upper = local_dense.map(|d| add_up(incoming, d)).transpose()?;
        for t in targets.iter().filter(|t| t.slab == i) {
            let e = if t.kind == OutputKind::Endpoint {
                endpoint_upper
            } else {
                dense_upper.ok_or_else(|| failed("missing dense proof"))?
            };
            if e > t.tolerance {
                return Err(failed("requested global target exceeded"));
            }
        }
        runs.push(SlabRun {
            spec: s,
            start,
            initial,
            endpoint: end,
            incoming,
            local_endpoint: local,
            local_dense,
            endpoint_upper,
            dense_upper,
            atol,
            segments,
        });
        start = s.end;
        initial = end;
        incoming = endpoint_upper;
    }
    Ok(Run {
        plan,
        work,
        slabs: runs,
    })
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub enum EventBracket {
    NoCrossing,
    Unresolved,
    Unique { lo: f64, hi: f64, qualified: bool },
}
fn sign(table: &RetroTable, s: &SlabRun, t: f64, threshold: f64) -> CoreResult<i8> {
    let (v, e) = s.query(table, t)?;
    let d = pt(v)?.add(Iv::new(-e, e)?)?.sub(pt(threshold)?)?;
    Ok(if d.lo > 0. {
        1
    } else if d.hi < 0. {
        -1
    } else {
        0
    })
}
/// Constant scalar affine flow is strictly monotone unless at equilibrium.
/// Opposite certified signs imply one unique crossing inside that slab.
/// A wide or unresolved bracket is not silently promoted. No terminal stop.
pub fn bracket_event(
    table: &RetroTable,
    s: &SlabRun,
    threshold: f64,
    time_tol: f64,
) -> CoreResult<EventBracket> {
    if !threshold.is_finite() || !time_tol.is_finite() || time_tol <= 0. {
        return Err(bad("event input"));
    }
    let (mut lo, mut hi) = (s.start, s.spec.end);
    let left = sign(table, s, lo, threshold)?;
    let right = sign(table, s, hi, threshold)?;
    if left == 0 || right == 0 {
        return Ok(EventBracket::Unresolved);
    }
    if left == right {
        return Ok(EventBracket::NoCrossing);
    }
    for _ in 0..80 {
        if width(lo, hi)?.hi <= time_tol {
            break;
        }
        let old = (lo, hi);
        // If the central query is inconclusive, try the two interior
        // quarter points. Retain only provably signed endpoints.
        let mid = lo + 0.5 * (hi - lo);
        for x in [mid, lo + 0.25 * (hi - lo), lo + 0.75 * (hi - lo)] {
            if !(lo < x && x < hi) {
                continue;
            }
            let v = sign(table, s, x, threshold)?;
            if v == left {
                lo = x;
            } else if v == right {
                hi = x;
            }
        }
        if (lo, hi) == old {
            break;
        }
    }
    Ok(EventBracket::Unique {
        lo,
        hi,
        qualified: width(lo, hi)?.hi <= time_tol,
    })
}
