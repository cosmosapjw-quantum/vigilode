//! Certified terminal covectors of the represented semidiscrete diffusion
//! generator. Exact real rates, not exactness of floating RHS evaluation.
//! No claim about the continuum spatial discretization error.
use super::*;
use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval as Iv, add_up, div_up, mul_up, sub_down, sub_up},
    polynomial_action::exp_neg_interval_enclosure,
};
use serde::Serialize;
const Z: Iv = Iv { lo: 0., hi: 0. };
fn err(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("spatial adjoint: {s}"))
}
fn l1(v: &[Iv]) -> CoreResult<f64> {
    v.iter().try_fold(0., |a, x| add_up(a, x.mag()))
}
fn dot(v: &[Iv], u: &[f64]) -> CoreResult<Iv> {
    v.iter()
        .zip(u)
        .try_fold(Z, |s, (&v, &u)| s.add(v.scale(u)?))
}
/// Original row as positive directed edges. Diagonal is the negative exact
/// sum of non-self rates. Duplicated periodic neighbors are intentional.
impl DiffusionGrid {
    /// Return all six represented neighbor edges; ignore self edges.
    pub fn represented_row(&self, i: usize) -> Option<[(usize, f64); 6]> {
        if i >= self.dimension {
            return None;
        }
        let w = self.width;
        let x = i % w;
        let r = i / w;
        let west = r * w + (x + w - 1) % w;
        let east = r * w + (x + 1) % w;
        Some(match &*self.geometry {
            Geometry::Cartesian { ny, nz, rates, .. } => {
                let y = r % ny;
                let z = r / ny;
                let ym = z * ny + (y + ny - 1) % ny;
                let yp = z * ny + (y + 1) % ny;
                let zm = (z + nz - 1) % nz * ny + y;
                let zp = (z + 1) % nz * ny + y;
                [
                    (west, rates[0]),
                    (east, rates[0]),
                    (ym * w + x, rates[1]),
                    (yp * w + x, rates[1]),
                    (zm * w + x, rates[2]),
                    (zp * w + x, rates[2]),
                ]
            }
            Geometry::Sphere(rings) => {
                let a = &rings[r];
                [
                    (west, a.azimuth),
                    (east, a.azimuth),
                    (r.saturating_sub(1) * w + x, a.north),
                    ((r + 1).min(rings.len() - 1) * w + x, a.south),
                    (i, 0.),
                    (i, 0.),
                ]
            }
        })
    }
    /// Euclidean transpose of the represented generator. The sphere is
    /// not assumed exactly area-self-adjoint after coefficient rounding.
    pub fn apply_transpose(&self, v: &[f64], out: &mut [f64]) -> Result<(), CallbackError> {
        if v.len() != self.dimension || out.len() != self.dimension {
            return Err(CallbackError::Fatal("diffusion transpose shape".into()));
        }
        if matches!(&*self.geometry, Geometry::Cartesian { .. }) {
            return self.apply(v, out, SpatialRows::Serial);
        }
        out.fill(0.);
        for (i, &x) in v.iter().enumerate() {
            for (j, r) in self.represented_row(i).unwrap() {
                if i != j && r != 0. {
                    let p = r * x;
                    out[i] -= p;
                    out[j] += p;
                }
            }
        }
        Ok(())
    }
    /// Opt-in row-contiguous evaluation of the represented Euclidean transpose.
    /// At a spherical cell, incoming meridional rates belong to the adjacent
    /// rings, not the local ring. No exact area-symmetry assumption is made.
    pub fn apply_transpose_gather(&self, v: &[f64], out: &mut [f64]) -> Result<(), CallbackError> {
        if v.len() != self.dimension || out.len() != self.dimension {
            return Err(CallbackError::Fatal("diffusion transpose shape".into()));
        }
        let Geometry::Sphere(rings) = &*self.geometry else {
            return self.apply(v, out, SpatialRows::Serial);
        };
        let w = self.width;
        for (r, o) in out.chunks_mut(w).enumerate() {
            let ring = &rings[r];
            let cur = &v[r * w..(r + 1) * w];
            let rn = r.saturating_sub(1);
            let rs = (r + 1).min(rings.len() - 1);
            let vn = &v[rn * w..(rn + 1) * w];
            let vs = &v[rs * w..(rs + 1) * w];
            let incoming_n = if r > 0 { rings[rn].south } else { 0. };
            let incoming_s = if r + 1 < rings.len() {
                rings[rs].north
            } else {
                0.
            };
            for (x, dst) in o.iter_mut().enumerate() {
                let c = cur[x];
                let xm = if x == 0 { w - 1 } else { x - 1 };
                let xp = if x + 1 == w { 0 } else { x + 1 };
                let lon = ring.azimuth * ((cur[xm] - c) + (cur[xp] - c));
                *dst = lon
                    + (incoming_n * vn[x] - ring.north * c)
                    + (incoming_s * vs[x] - ring.south * c);
            }
        }
        Ok(())
    }
    /// Numerical adjoint problem with the alternative contiguous transpose
    /// callback. Use the original-source certificate for endpoint authority.
    pub fn adjoint_problem_gather(&self) -> Problem {
        let r = self.clone();
        let j = self.clone();
        Problem::new(self.dimension, move |_, u, o| {
            r.apply_transpose_gather(u, o)
        })
        .jacobian(Jacobian::jvp(move |_, _, v, o| {
            j.apply_transpose_gather(v, o)
        }))
        .autonomous()
    }
    /// Reverse-clock continuous adjoint ODE of the semidiscrete problem.
    pub fn adjoint_problem(&self) -> Problem {
        let r = self.clone();
        let j = self.clone();
        Problem::new(self.dimension, move |_, u, o| r.apply_transpose(u, o))
            .jacobian(Jacobian::jvp(move |_, _, v, o| j.apply_transpose(v, o)))
            .autonomous()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// Explicit, non-promoted semidiscrete adjoint enclosure routes.
pub enum SpatialAdjointRoute {
    /// Direct degree-18 Taylor action.
    Taylor18,
    /// Degree-18 uniformization.
    Poisson18,
    /// Uniformization with a directed Poisson tail order.
    PoissonAdaptive,
    /// Wider Poisson subcells (experimental).
    PoissonWide,
    /// Wider Poisson subcells with decay-weighted tail (experimental).
    PoissonWideDecay,
    /// Approximate eigenmode proposal with original transpose residual.
    ModeResidual,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
/// Resource and directed goal-error receipt for one successful action.
pub struct SpatialAdjointWork {
    /// Number of time cells (zero for a mode identity).
    pub cells: usize,
    /// Sparse represented transpose or transition actions.
    pub actions: usize,
    /// Maximum series order.
    pub max_order: usize,
    /// Accumulated l1 truncation or mode-residual error.
    pub l1_tail_upper: f64,
    /// Nonpositive rate proposal for the mode route.
    pub mode_rate: f64,
    /// Maximum component enclosure width.
    pub width_upper: f64,
    /// Outward interval of the exact semidiscrete terminal goal.
    pub exact_goal: [f64; 2],
    /// Exact goal minus the represented numerical terminal goal.
    pub signed_goal_error: [f64; 2],
}
#[derive(Clone)]
struct Row {
    edges: [(usize, f64); 6],
    exit: Iv,
    p: [Iv; 6],
    stay: Iv,
}
/// Source-bound reusable sparse rates and caller-owned work storage.
pub struct SpatialAdjointWorkspace {
    grid: DiffusionGrid,
    rows: Vec<Row>,
    q: f64,
    state: Vec<Iv>,
    term: Vec<Iv>,
    sum: Vec<Iv>,
    scratch: Vec<Iv>,
}
fn apply(rows: &[Row], v: &[Iv], out: &mut [Iv], poisson: bool) -> CoreResult<()> {
    out.fill(Z);
    for (i, r) in rows.iter().enumerate() {
        out[i] = out[i].add(if poisson {
            r.stay.mul(v[i])?
        } else {
            (-r.exit).mul(v[i])?
        })?;
        for (k, &(j, a)) in r.edges.iter().enumerate() {
            if i != j && a != 0. {
                out[j] = out[j].add(if poisson {
                    r.p[k].mul(v[i])?
                } else {
                    v[i].scale(a)?
                })?;
            }
        }
    }
    Ok(())
}
fn tail_after(m: f64, p: usize) -> CoreResult<f64> {
    let mut t = 1.;
    for k in 1..=p + 1 {
        t = div_up(mul_up(t, m)?, k as f64)?;
    }
    div_up(t, sub_down(1., div_up(m, (p + 2) as f64)?)?)
}
impl SpatialAdjointWorkspace {
    /// Prepare the represented source and reusable work buffers.
    pub fn new(grid: &DiffusionGrid) -> CoreResult<Self> {
        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = grid.dimension();
        if n > 16384 {
            return Err(err("workspace dimension cap"));
        }
        let mut rows = Vec::with_capacity(n);
        let mut q = 0_f64;
        for i in 0..n {
            let edges = grid.represented_row(i).ok_or_else(|| err("row missing"))?;
            let exit = edges
                .iter()
                .filter(|&&(j, _)| j != i)
                .try_fold(Z, |s, &(_, a)| s.add(Iv::point(a)?))?;
            if exit.lo < 0. {
                return Err(err("negative exit rate"));
            }
            q = q.max(exit.hi);
            rows.push(Row {
                edges,
                exit,
                p: [Z; 6],
                stay: Z,
            });
        }
        if q > 0. {
            for (i, r) in rows.iter_mut().enumerate() {
                for (k, &(j, a)) in r.edges.iter().enumerate() {
                    if j != i && a != 0. {
                        r.p[k] = Iv::point(a)?.div(Iv::point(q)?)?;
                    }
                }
                let s = Iv::point(1.)?.sub(r.exit.div(Iv::point(q)?)?)?;
                // Exact row sum and q>=exit imply mathematical stay in [0,1].
                r.stay = Iv::new(s.lo.max(0.), s.hi.min(1.))?;
            }
        }
        Ok(Self {
            grid: grid.clone(),
            rows,
            q,
            state: vec![Z; n],
            term: vec![Z; n],
            sum: vec![Z; n],
            scratch: vec![Z; n],
        })
    }
    /// Bound the true semidiscrete terminal goal and the gradient with respect
    /// to the represented initial vector. Endpoint is the actual numerical
    /// result supplied by the caller; report encloses exact minus numerical
    /// goal, not a bound on its full state or continuum discretization.
    /// Successful calls reuse all buffers, and refusal does not alter out.
    pub fn certify_into(
        &mut self,
        goal: &[f64],
        initial: &[f64],
        endpoint: &[f64],
        duration: f64,
        route: SpatialAdjointRoute,
        out: &mut [Iv],
    ) -> CoreResult<SpatialAdjointWork> {
        let n = self.grid.dimension();
        if goal.len() != n
            || initial.len() != n
            || endpoint.len() != n
            || out.len() != n
            || !duration.is_finite()
            || duration < 0.
            || goal
                .iter()
                .chain(initial)
                .chain(endpoint)
                .any(|x| !x.is_finite())
        {
            return Err(err("shape or nonfinite goal/clock"));
        }
        let mut w = SpatialAdjointWork::default();
        for (o, &c) in self.state.iter_mut().zip(goal) {
            *o = Iv::point(c)?;
        }
        if duration > 0. && self.q > 0. {
            if route == SpatialAdjointRoute::ModeResidual {
                apply(&self.rows, &self.state, &mut self.scratch, false)?;
                let denom = goal.iter().map(|c| c * c).sum::<f64>();
                let num = goal
                    .iter()
                    .zip(&self.scratch)
                    .map(|(c, v)| c * (v.lo * 0.5 + v.hi * 0.5))
                    .sum::<f64>();
                let mu = if denom > 0. && denom.is_finite() && (num / denom).is_finite() {
                    (num / denom).min(0.)
                } else {
                    0.
                };
                w.mode_rate = mu;
                w.actions = 1;
                let mut residual = 0.;
                for (&c, &v) in goal.iter().zip(&self.scratch) {
                    residual = add_up(residual, v.sub(Iv::point(c)?.scale(mu)?)?.mag())?;
                }
                w.l1_tail_upper = mul_up(duration, residual)?;
                let gain = exp_neg_interval_enclosure(Iv::point(duration)?.scale(-mu)?)?;
                for (o, &c) in self.state.iter_mut().zip(goal) {
                    *o = gain.scale(c)?;
                }
            } else {
                let poisson = route != SpatialAdjointRoute::Taylor18;
                let mut cells = 1usize;
                let total = mul_up(self.q, duration)?;
                let wide = matches!(
                    route,
                    SpatialAdjointRoute::PoissonWide | SpatialAdjointRoute::PoissonWideDecay
                );
                let cap = if wide {
                    4.
                } else if poisson {
                    1.
                } else {
                    0.5
                };
                while div_up(total, cells as f64)? > cap {
                    cells *= 2;
                    if cells > 4096 {
                        return Err(err("subcell cap"));
                    }
                }
                let h = Iv::point(duration)?.div(Iv::point(cells as f64)?)?;
                let a = h.scale(self.q)?;
                let m = if poisson { a.hi } else { mul_up(2., a.hi)? };
                if m > if wide {
                    4.000000000000004
                } else {
                    1.000000000000001
                } {
                    return Err(err("series norm cap"));
                }
                let decay = if poisson {
                    exp_neg_interval_enclosure(a)?
                } else {
                    Iv::point(1.)?
                };
                let mut order = 18;
                let tail = |k: usize| -> CoreResult<f64> {
                    let t = tail_after(m, k)?;
                    if route == SpatialAdjointRoute::PoissonWideDecay {
                        mul_up(t, decay.hi)
                    } else {
                        Ok(t)
                    }
                };
                if route == SpatialAdjointRoute::PoissonAdaptive || wide {
                    let target = rodas5p_core::directed::div_down(1e-15, cells as f64)?;
                    let mut bounds = if wide { 4..=48 } else { 0..=24 };
                    order = bounds
                        .find(|&k| tail(k).is_ok_and(|t| t <= target))
                        .ok_or_else(|| err("Poisson tail order cap"))?;
                }
                let local = tail(order)?;
                for _ in 0..cells {
                    // Carry one l1 radius: exact semigroup is l1 contractive, so the
                    // previous truncation radius never needs componentwise inflation.
                    w.l1_tail_upper = add_up(w.l1_tail_upper, mul_up(local, l1(&self.state)?)?)?;
                    self.term.copy_from_slice(&self.state);
                    self.sum.copy_from_slice(&self.state);
                    for k in 1..=order {
                        apply(&self.rows, &self.term, &mut self.scratch, poisson)?;
                        let coef = if poisson { a } else { h }.div(Iv::point(k as f64)?)?;
                        for i in 0..n {
                            self.term[i] = self.scratch[i].mul(coef)?;
                            self.sum[i] = self.sum[i].add(self.term[i])?;
                        }
                        w.actions += 1;
                    }
                    for i in 0..n {
                        self.state[i] = self.sum[i].mul(decay)?;
                    }
                }
                w.cells = cells;
                w.max_order = order;
            }
        }
        let radius = mul_up(
            w.l1_tail_upper,
            initial.iter().map(|x| x.abs()).fold(0., f64::max),
        )?;
        let exact = dot(&self.state, initial)?.add(Iv::new(-radius, radius)?)?;
        let numeric = goal
            .iter()
            .zip(endpoint)
            .try_fold(Z, |s, (&c, &u)| s.add(Iv::point(c)?.scale(u)?))?;
        w.exact_goal = exact.as_pair();
        w.signed_goal_error = exact.sub(numeric)?.as_pair();
        let tail = Iv::new(-w.l1_tail_upper, w.l1_tail_upper)?;
        for i in 0..n {
            self.scratch[i] = self.state[i].add(tail)?;
            w.width_upper = w
                .width_upper
                .max(sub_up(self.scratch[i].hi, self.scratch[i].lo)?);
        }
        out.copy_from_slice(&self.scratch);
        Ok(w)
    }
}

/// Reuse one source-bound adjoint action for multiple physical terminal goals
/// with identical covector and duration, and distinct initial/endpoint vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialBatchPlan {
    /// Independently repeat each certified action.
    Repeated,
    /// Reuse the full center and a single global l1 tail.
    SharedL1,
    /// Dot the componentwise inflated covector with each initial state.
    SharedComponent,
    /// Subtract the represented constant mode before applying the tail.
    SharedCentered,
    /// Intersect constant-mode and original single-tail goal certificates.
    SharedCenteredIntersect,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
/// Actual aggregate batch work, rather than the per-pair inherited receipt.
pub struct SpatialBatchWork {
    /// Number of initial/endpoint pairs.
    pub pairs: usize,
    /// Actual sparse actions across the batch.
    pub actions: usize,
    /// Largest signed goal interval width.
    pub max_goal_width: f64,
    /// Largest initial covector component width.
    pub max_gradient_width: f64,
}
/// Signed budget gate, distinct from the existence of a valid enclosure.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SpatialBatchBudgetWork {
    /// Actual batch matrix work (including a refused budget).
    pub work: SpatialBatchWork,
    /// Count of pairs whose signed error magnitude is within the budget.
    pub accepted_pairs: usize,
    /// All pairs met the original signed absolute error budget.
    pub accepted: bool,
    /// Maximum signed error magnitude across pairs.
    pub max_signed_error_upper: f64,
}
/// Source-owned reusable storage for a bounded batch.
pub struct SpatialAdjointBatchWorkspace {
    inner: SpatialAdjointWorkspace,
    grad: Vec<Iv>,
    receipts: Vec<SpatialAdjointWork>,
}
impl SpatialAdjointBatchWorkspace {
    /// Allocate once for the given represented grid and maximum batch size.
    pub fn new(grid: &DiffusionGrid, capacity: usize) -> CoreResult<Self> {
        if capacity == 0 || capacity > 4096 {
            return Err(err("batch capacity"));
        }
        Ok(Self {
            inner: SpatialAdjointWorkspace::new(grid)?,
            grad: vec![Z; grid.dimension()],
            receipts: vec![SpatialAdjointWork::default(); capacity],
        })
    }
    /// Flat pair-major inputs. Each supplied endpoint belongs to the
    /// respective initial condition; no claim about its primal accuracy.
    /// A refusal leaves both caller-owned output slices unchanged.
    pub fn certify_batch_into(
        &mut self,
        goal: &[f64],
        initial: &[f64],
        endpoint: &[f64],
        duration: f64,
        route: SpatialAdjointRoute,
        plan: SpatialBatchPlan,
        gradient: &mut [Iv],
        out: &mut [SpatialAdjointWork],
    ) -> CoreResult<SpatialBatchWork> {
        let work = self.compute_batch(
            goal,
            initial,
            endpoint,
            duration,
            route,
            plan,
            gradient.len(),
            out.len(),
        )?;
        gradient.copy_from_slice(&self.grad);
        out.copy_from_slice(&self.receipts[..out.len()]);
        Ok(work)
    }
    /// Accept only when every signed terminal goal error fits the given
    /// absolute budget. Caller outputs remain unchanged on rejection/error.
    pub fn certify_batch_budgeted_into(
        &mut self,
        goal: &[f64],
        initial: &[f64],
        endpoint: &[f64],
        duration: f64,
        route: SpatialAdjointRoute,
        plan: SpatialBatchPlan,
        budget: f64,
        gradient: &mut [Iv],
        out: &mut [SpatialAdjointWork],
    ) -> CoreResult<SpatialBatchBudgetWork> {
        if !budget.is_finite() || budget <= 0. {
            return Err(err("batch signed budget"));
        }
        let work = self.compute_batch(
            goal,
            initial,
            endpoint,
            duration,
            route,
            plan,
            gradient.len(),
            out.len(),
        )?;
        let mut r = SpatialBatchBudgetWork {
            work,
            ..Default::default()
        };
        for v in &self.receipts[..out.len()] {
            let mag = v.signed_goal_error[0]
                .abs()
                .max(v.signed_goal_error[1].abs());
            r.max_signed_error_upper = r.max_signed_error_upper.max(mag);
            r.accepted_pairs += usize::from(mag <= budget);
        }
        r.accepted = r.accepted_pairs == out.len();
        if r.accepted {
            gradient.copy_from_slice(&self.grad);
            out.copy_from_slice(&self.receipts[..out.len()]);
        }
        Ok(r)
    }
    fn compute_batch(
        &mut self,
        goal: &[f64],
        initial: &[f64],
        endpoint: &[f64],
        duration: f64,
        route: SpatialAdjointRoute,
        plan: SpatialBatchPlan,
        gradient_len: usize,
        k: usize,
    ) -> CoreResult<SpatialBatchWork> {
        let n = self.inner.grid.dimension();
        if k == 0
            || k > self.receipts.len()
            || n.checked_mul(k) != Some(initial.len())
            || initial.len() != endpoint.len()
            || goal.len() != n
            || gradient_len != n
            || !duration.is_finite()
            || duration < 0.
            || goal
                .iter()
                .chain(initial)
                .chain(endpoint)
                .any(|x| !x.is_finite())
        {
            return Err(err("batch adjoint shape/clock"));
        }
        let mut work = SpatialBatchWork {
            pairs: k,
            ..Default::default()
        };
        let first = self.inner.certify_into(
            goal,
            &initial[..n],
            &endpoint[..n],
            duration,
            route,
            &mut self.grad,
        )?;
        work.actions = first.actions;
        self.receipts[0] = first;
        for i in 0..k {
            let u = &initial[i * n..(i + 1) * n];
            let v = &endpoint[i * n..(i + 1) * n];
            let rec = if plan == SpatialBatchPlan::Repeated && i > 0 {
                let r = self
                    .inner
                    .certify_into(goal, u, v, duration, route, &mut self.grad)?;
                work.actions += r.actions;
                r
            } else if plan == SpatialBatchPlan::SharedL1 && i == 0
                || plan == SpatialBatchPlan::Repeated
            {
                first
            } else {
                let mut r = first;
                let plain = || -> CoreResult<Iv> {
                    let radius = mul_up(
                        r.l1_tail_upper,
                        u.iter().map(|x| x.abs()).fold(0., f64::max),
                    )?;
                    dot(&self.inner.state, u)?.add(Iv::new(-radius, radius)?)
                };
                let exact = match plan {
                    SpatialBatchPlan::SharedComponent => dot(&self.grad, u)?,
                    SpatialBatchPlan::SharedCentered
                    | SpatialBatchPlan::SharedCenteredIntersect => {
                        // For the exact represented generator L*1=0.
                        // Choose a represented shift; enclose every subtraction.
                        let lo = u.iter().copied().fold(f64::INFINITY, f64::min);
                        let hi = u.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        let shift = 0.5 * lo + 0.5 * hi;
                        let constant = goal
                            .iter()
                            .try_fold(Z, |s, &c| s.add(Iv::point(c)?))?
                            .scale(shift)?;
                        let mut norm = 0_f64;
                        let mut value = constant;
                        for (&v, &c) in u.iter().zip(&self.inner.state) {
                            let centered = Iv::point(v)?.sub(Iv::point(shift)?)?;
                            norm = norm.max(centered.mag());
                            value = value.add(c.mul(centered)?)?;
                        }
                        let rad = mul_up(r.l1_tail_upper, norm)?;
                        let centered = value.add(Iv::new(-rad, rad)?)?;
                        if plan == SpatialBatchPlan::SharedCenteredIntersect {
                            let p = plain()?;
                            Iv::new(p.lo.max(centered.lo), p.hi.min(centered.hi))?
                        } else {
                            centered
                        }
                    }
                    _ => plain()?,
                };
                let numeric = goal
                    .iter()
                    .zip(v)
                    .try_fold(Z, |s, (&c, &x)| s.add(Iv::point(c)?.scale(x)?))?;
                r.exact_goal = exact.as_pair();
                r.signed_goal_error = exact.sub(numeric)?.as_pair();
                r
            };
            work.max_goal_width = work
                .max_goal_width
                .max(sub_up(rec.signed_goal_error[1], rec.signed_goal_error[0])?);
            work.max_gradient_width = work.max_gradient_width.max(rec.width_upper);
            self.receipts[i] = rec;
        }
        Ok(work)
    }
}

// Source-owned, exact-key reuse of a terminal covector. Its interval center
// and single l1 tail remain private; only directed goal projections escape.
pub(super) struct SpatialDualCache {
    inner: SpatialAdjointWorkspace,
    key: Vec<f64>,
    zero: Vec<f64>,
    gradient: Vec<Iv>,
    duration: Option<u64>,
    work: SpatialAdjointWork,
}
impl SpatialDualCache {
    pub(super) fn new(grid: &DiffusionGrid) -> CoreResult<Self> {
        let n = grid.dimension();
        Ok(Self {
            inner: SpatialAdjointWorkspace::new(grid)?,
            key: vec![0.; n],
            zero: vec![0.; n],
            gradient: vec![Z; n],
            duration: None,
            work: SpatialAdjointWork::default(),
        })
    }
    pub(super) fn prepare(&mut self, goal: &[f64], duration: f64) -> CoreResult<(usize, bool)> {
        if goal.len() != self.key.len()
            || goal.iter().any(|v| !v.is_finite())
            || !duration.is_finite()
            || duration < 0.
        {
            return Err(err("dual cache key"));
        }
        if self.duration == Some(duration.to_bits())
            && self
                .key
                .iter()
                .zip(goal)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        {
            return Ok((0, true));
        }
        self.duration = None;
        let work = self.inner.certify_into(
            goal,
            &self.zero,
            &self.zero,
            duration,
            SpatialAdjointRoute::PoissonWideDecay,
            &mut self.gradient,
        )?;
        self.key.copy_from_slice(goal);
        self.work = work;
        self.duration = Some(duration.to_bits());
        Ok((work.actions, false))
    }
    pub(super) fn project(&self, u: &[f64]) -> CoreResult<SpatialAdjointWork> {
        if self.duration.is_none() || u.len() != self.key.len() || u.iter().any(|v| !v.is_finite())
        {
            return Err(err("unprepared dual projection"));
        }
        let mut r = self.work;
        let lo = u.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = u.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let s = 0.5 * lo + 0.5 * hi;
        let constant = self
            .key
            .iter()
            .try_fold(Z, |a, &c| a.add(Iv::point(c)?))?
            .scale(s)?;
        let mut value = constant;
        let mut norm = 0_f64;
        for (&v, &c) in u.iter().zip(&self.inner.state) {
            let d = Iv::point(v)?.sub(Iv::point(s)?)?;
            norm = norm.max(d.mag());
            value = value.add(c.mul(d)?)?;
        }
        let radius = mul_up(r.l1_tail_upper, norm)?;
        let centered = value.add(Iv::new(-radius, radius)?)?;
        let radius = mul_up(
            r.l1_tail_upper,
            u.iter().map(|x| x.abs()).fold(0., f64::max),
        )?;
        let plain = dot(&self.inner.state, u)?.add(Iv::new(-radius, radius)?)?;
        let exact = Iv::new(plain.lo.max(centered.lo), plain.hi.min(centered.hi))?;
        r.exact_goal = exact.as_pair();
        r.signed_goal_error = exact
            .sub(
                self.key
                    .iter()
                    .zip(u)
                    .try_fold(Z, |a, (&c, &x)| a.add(Iv::point(c)?.scale(x)?))?,
            )?
            .as_pair();
        Ok(r)
    }
}
