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
    // Propagate a previously enclosed center with an independent scalar l1
    // tail. Exact transpose semigroup is l1 contractive. The interval clock
    // encloses the exact represented difference, not a rounded duration.
    fn propagate_series(
        &mut self,
        duration: Iv,
        route: SpatialAdjointRoute,
        w: &mut SpatialAdjointWork,
    ) -> CoreResult<()> {
        if duration.lo < 0. || duration.hi < duration.lo {
            return Err(err("series clock"));
        }
        if duration.hi == 0. || self.q == 0. {
            return Ok(());
        }
        let n = self.state.len();
        let poisson = route != SpatialAdjointRoute::Taylor18;
        let mut cells = 1usize;
        let total = mul_up(self.q, duration.hi)?;
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
        let h = duration.div(Iv::point(cells as f64)?)?;
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
        w.cells += cells;
        w.max_order = w.max_order.max(order);
        Ok(())
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
                self.propagate_series(Iv::point(duration)?, route, &mut w)?;
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

/// Boundary covector construction for compact goal-directed replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialBoundaryDualPlan {
    /// Independent actions from the original terminal covector.
    Independent,
    /// Backward semigroup recurrence carrying a single l1 truncation bound.
    Chain,
    /// Near-stationary residual certificate when eligible, otherwise Chain.
    StationaryChain,
    /// Bounded individual-time cache with nearest later certified anchor.
    SubsetStationaryChain,
    /// Bounded individual-time cache; compute misses from terminal directly.
    SubsetStationaryIndependent,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub(super) struct BoundaryPreparation {
    pub actions: usize,
    pub cache_hit: bool,
    pub stationary: bool,
    pub stationary_residual: f64,
    pub reused: usize,
    pub built: usize,
    pub evicted: usize,
}
// Pairwise outward summation reduces the accumulation width without an
// allocating reduction tree. The inputs remain exact represented numbers.
fn pairwise_point_sum(v: &[f64]) -> CoreResult<Iv> {
    if v.len() <= 1 {
        return Iv::point(v.first().copied().unwrap_or(0.));
    }
    let (a, b) = v.split_at(v.len() / 2);
    pairwise_point_sum(a)?.add(pairwise_point_sum(b)?)
}
fn pairwise_shifted_dot(c: &[Iv], u: &[f64], s: f64) -> CoreResult<Iv> {
    if c.is_empty() {
        return Ok(Z);
    }
    if c.len() == 1 {
        return c[0].mul(Iv::point(u[0])?.sub(Iv::point(s)?)?);
    }
    let m = c.len() / 2;
    pairwise_shifted_dot(&c[..m], &u[..m], s)?.add(pairwise_shifted_dot(&c[m..], &u[m..], s)?)
}
/// Replacement policy for the bounded source-owned boundary pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum SpatialBoundaryPoolPolicy {
    /// Original FIFO, including terminal row storage.
    Fifo,
    /// Prefetch matching rows, implicit terminal, least recently used.
    Lru,
    /// Prefer rows with larger observed marginal sparse action cost.
    Cost,
    /// Cost policy with the initial boundary preferentially protected.
    PinnedCost,
}
// Bounded FIFO pool. Active boundary rows are copied out, so eviction never
// destroys authority of a prepared projection. Exact goal/route/terminal
// namespace is held by the owning SpatialBoundaryDual.
struct BoundaryPool {
    times: [Option<u64>; 32],
    works: [SpatialAdjointWork; 32],
    centers: Vec<Iv>,
    slots: usize,
    next: usize,
    terminal: Option<u64>,
    stationary: bool,
    residual: f64,
    policy: SpatialBoundaryPoolPolicy,
    ages: [u64; 32],
    costs: [usize; 32],
    clock: u64,
}
impl BoundaryPool {
    fn new(n: usize, slots: usize, policy: SpatialBoundaryPoolPolicy) -> CoreResult<Self> {
        if slots > 32 {
            return Err(err("boundary pool slots"));
        }
        Ok(Self {
            times: [None; 32],
            works: [SpatialAdjointWork::default(); 32],
            centers: vec![Z; n * slots],
            slots,
            next: 0,
            terminal: None,
            stationary: false,
            residual: 0.,
            policy,
            ages: [0; 32],
            costs: [0; 32],
            clock: 0,
        })
    }
    fn clear(&mut self) {
        self.times.fill(None);
        self.terminal = None;
        self.next = 0;
        self.ages.fill(0);
        self.costs.fill(0);
        self.clock = 0;
    }
    fn find(&self, t: f64) -> Option<usize> {
        self.times[..self.slots]
            .iter()
            .position(|v| *v == Some(t.to_bits()))
    }
    fn later(&self, t: f64, upper: f64) -> Option<(usize, f64)> {
        self.times[..self.slots]
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|b| (i, f64::from_bits(b))))
            .filter(|&(_, x)| x > t && x < upper)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }
    fn touch(&mut self, i: usize) {
        self.clock = self.clock.saturating_add(1);
        self.ages[i] = self.clock;
    }
    // Cost is measured incremental action work, not historical proof size.
    fn put_smart(&mut self, t: f64, c: &[Iv], w: SpatialAdjointWork, cost: usize) -> bool {
        let slot = self.times[..self.slots]
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                (0..self.slots)
                    .min_by_key(|&i| match self.policy {
                        SpatialBoundaryPoolPolicy::Lru => (0, 0, self.ages[i]),
                        SpatialBoundaryPoolPolicy::PinnedCost => (
                            usize::from(self.times[i] == Some(0f64.to_bits())),
                            self.costs[i],
                            self.ages[i],
                        ),
                        _ => (0, self.costs[i], self.ages[i]),
                    })
                    .unwrap()
            });
        let evicted = self.times[slot].is_some();
        self.times[slot] = Some(t.to_bits());
        self.works[slot] = w;
        self.costs[slot] = cost;
        let n = c.len();
        self.centers[slot * n..(slot + 1) * n].copy_from_slice(c);
        self.touch(slot);
        evicted
    }
    fn put(&mut self, t: f64, c: &[Iv], w: SpatialAdjointWork) -> bool {
        let slot = self.times[..self.slots]
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                let i = self.next;
                self.next = (i + 1) % self.slots;
                i
            });
        let evicted = self.times[slot].is_some();
        self.times[slot] = Some(t.to_bits());
        self.works[slot] = w;
        let n = c.len();
        self.centers[slot * n..(slot + 1) * n].copy_from_slice(c);
        evicted
    }
}
/// One source-owned propagator plus a flat table of boundary enclosures.
/// All cached covectors have the same exact terminal goal and time grid.
pub(super) struct SpatialBoundaryDual {
    inner: SpatialAdjointWorkspace,
    key: Vec<f64>,
    centers: Vec<Iv>,
    works: [SpatialAdjointWork; 9],
    times: [f64; 9],
    count: usize,
    route: SpatialBoundaryDualPlan,
    preparation: BoundaryPreparation,
    pool: BoundaryPool,
}
impl SpatialBoundaryDual {
    pub fn new(g: &DiffusionGrid) -> CoreResult<Self> {
        Self::with_pool(g, 0)
    }
    pub(super) fn with_pool(g: &DiffusionGrid, slots: usize) -> CoreResult<Self> {
        Self::with_pool_policy(g, slots, SpatialBoundaryPoolPolicy::Fifo)
    }
    pub(super) fn with_pool_policy(
        g: &DiffusionGrid,
        slots: usize,
        policy: SpatialBoundaryPoolPolicy,
    ) -> CoreResult<Self> {
        let n = g.dimension();
        Ok(Self {
            inner: SpatialAdjointWorkspace::new(g)?,
            key: vec![0.; n],
            centers: vec![Z; 9 * n],
            works: [SpatialAdjointWork::default(); 9],
            times: [0.; 9],
            count: 0,
            route: SpatialBoundaryDualPlan::Independent,
            preparation: BoundaryPreparation::default(),
            pool: BoundaryPool::new(n, slots, policy)?,
        })
    }
    pub(super) fn invalidate(&mut self) {
        self.count = 0;
        self.pool.clear();
    }
    pub fn prepare(
        &mut self,
        goal: &[f64],
        times: &[f64],
        route: SpatialBoundaryDualPlan,
    ) -> CoreResult<BoundaryPreparation> {
        let n = self.key.len();
        let m = times.len();
        if m < 2
            || m > 9
            || goal.len() != n
            || goal.iter().chain(times).any(|x| !x.is_finite())
            || times[0] < 0.
            || times.windows(2).any(|v| v[0] >= v[1])
        {
            return Err(err("boundary key"));
        }
        if matches!(
            route,
            SpatialBoundaryDualPlan::SubsetStationaryChain
                | SpatialBoundaryDualPlan::SubsetStationaryIndependent
        ) {
            return self.prepare_subset(goal, times, route);
        }
        self.pool.clear();
        if self.count == m
            && self.route == route
            && self.times[..m]
                .iter()
                .zip(times)
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && self
                .key
                .iter()
                .zip(goal)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        {
            return Ok(BoundaryPreparation {
                actions: 0,
                cache_hit: true,
                ..self.preparation
            });
        }
        self.count = 0;
        let tf = times[m - 1];
        let mut receipt = BoundaryPreparation::default();
        for (x, &c) in self.inner.state.iter_mut().zip(goal) {
            *x = Iv::point(c)?;
        }
        self.centers[(m - 1) * n..m * n].copy_from_slice(&self.inner.state);
        self.works[m - 1] = SpatialAdjointWork::default();
        if route == SpatialBoundaryDualPlan::StationaryChain {
            apply(
                &self.inner.rows,
                &self.inner.state,
                &mut self.inner.scratch,
                false,
            )?;
            receipt.actions += 1;
            receipt.stationary_residual = l1(&self.inner.scratch)?;
            // Selection threshold is heuristic, certificate below is exact
            // even for nonzero represented stationarity residual.
            receipt.stationary = receipt.stationary_residual <= 1e-10 * l1(&self.inner.state)?;
        }
        let mut w = SpatialAdjointWork::default();
        for k in (0..m - 1).rev() {
            if receipt.stationary {
                w = SpatialAdjointWork {
                    actions: 1,
                    l1_tail_upper: mul_up(
                        Iv::point(tf)?.sub(Iv::point(times[k])?)?.hi,
                        receipt.stationary_residual,
                    )?,
                    ..Default::default()
                };
            } else {
                let dt = if route == SpatialBoundaryDualPlan::Independent {
                    for (v, &c) in self.inner.state.iter_mut().zip(goal) {
                        *v = Iv::point(c)?;
                    }
                    w = SpatialAdjointWork::default();
                    Iv::point(tf)?.sub(Iv::point(times[k])?)?
                } else {
                    Iv::point(times[k + 1])?.sub(Iv::point(times[k])?)?
                };
                let old = w.actions;
                self.inner
                    .propagate_series(dt, SpatialAdjointRoute::PoissonWideDecay, &mut w)?;
                receipt.actions += w.actions - old;
            }
            self.centers[k * n..(k + 1) * n].copy_from_slice(&self.inner.state);
            let mut r = w;
            let tail = Iv::new(-w.l1_tail_upper, w.l1_tail_upper)?;
            for v in &self.inner.state {
                let a = v.add(tail)?;
                r.width_upper = r.width_upper.max(sub_up(a.hi, a.lo)?);
            }
            self.works[k] = r;
        }
        self.key.copy_from_slice(goal);
        self.times[..m].copy_from_slice(times);
        self.route = route;
        self.count = m;
        self.preparation = receipt;
        Ok(receipt)
    }
    fn prepare_subset(
        &mut self,
        goal: &[f64],
        times: &[f64],
        route: SpatialBoundaryDualPlan,
    ) -> CoreResult<BoundaryPreparation> {
        let n = self.key.len();
        let m = times.len();
        let tf = times[m - 1];
        if self.pool.slots == 0 {
            return Err(err("subset route requires pool"));
        }
        let same = self.pool.terminal == Some(tf.to_bits())
            && self.route == route
            && self
                .key
                .iter()
                .zip(goal)
                .all(|(a, b)| a.to_bits() == b.to_bits());
        self.count = 0;
        let mut receipt = BoundaryPreparation::default();
        if !same {
            self.pool.clear();
            for (v, &c) in self.inner.state.iter_mut().zip(goal) {
                *v = Iv::point(c)?;
            }
            apply(
                &self.inner.rows,
                &self.inner.state,
                &mut self.inner.scratch,
                false,
            )?;
            receipt.actions += 1;
            self.pool.residual = l1(&self.inner.scratch)?;
            self.pool.stationary = self.pool.residual <= 1e-10 * l1(&self.inner.state)?;
            self.key.copy_from_slice(goal);
            self.route = route;
            self.pool.terminal = Some(tf.to_bits());
        }
        receipt.stationary = self.pool.stationary;
        receipt.stationary_residual = self.pool.residual;
        if self.pool.policy != SpatialBoundaryPoolPolicy::Fifo {
            return self.prepare_smart(goal, times, route, receipt);
        }
        for k in (0..m).rev() {
            let t = times[k];
            if let Some(i) = self.pool.find(t) {
                self.centers[k * n..(k + 1) * n]
                    .copy_from_slice(&self.pool.centers[i * n..(i + 1) * n]);
                self.works[k] = self.pool.works[i];
                receipt.reused += 1;
                continue;
            }
            let mut w = SpatialAdjointWork::default();
            if k == m - 1
                || receipt.stationary
                || route == SpatialBoundaryDualPlan::SubsetStationaryIndependent
            {
                for (v, &c) in self.inner.state.iter_mut().zip(goal) {
                    *v = Iv::point(c)?;
                }
                if k < m - 1 {
                    let dt = Iv::point(tf)?.sub(Iv::point(t)?)?;
                    if receipt.stationary {
                        w.actions = 1;
                        w.l1_tail_upper = mul_up(dt.hi, receipt.stationary_residual)?;
                    } else {
                        self.inner.propagate_series(
                            dt,
                            SpatialAdjointRoute::PoissonWideDecay,
                            &mut w,
                        )?;
                        receipt.actions += w.actions;
                    }
                }
            } else {
                // Active next boundary and pool later boundaries are all valid
                // under this namespace. Select the closest strictly later one.
                let mut anchor = times[k + 1];
                if let Some((i, v)) = self.pool.later(t, anchor) {
                    anchor = v;
                    w = self.pool.works[i];
                    self.inner
                        .state
                        .copy_from_slice(&self.pool.centers[i * n..(i + 1) * n]);
                } else {
                    w = self.works[k + 1];
                    self.inner
                        .state
                        .copy_from_slice(&self.centers[(k + 1) * n..(k + 2) * n]);
                }
                let old = w.actions;
                self.inner.propagate_series(
                    Iv::point(anchor)?.sub(Iv::point(t)?)?,
                    SpatialAdjointRoute::PoissonWideDecay,
                    &mut w,
                )?;
                receipt.actions += w.actions - old;
            }
            let tail = Iv::new(-w.l1_tail_upper, w.l1_tail_upper)?;
            for v in &self.inner.state {
                let a = v.add(tail)?;
                w.width_upper = w.width_upper.max(sub_up(a.hi, a.lo)?);
            }
            self.centers[k * n..(k + 1) * n].copy_from_slice(&self.inner.state);
            self.works[k] = w;
            receipt.built += 1;
            receipt.evicted += usize::from(self.pool.put(t, &self.inner.state, w));
        }
        self.times[..m].copy_from_slice(times);
        self.count = m;
        receipt.cache_hit = receipt.built == 0 && receipt.actions == 0;
        self.preparation = receipt;
        Ok(receipt)
    }
    fn prepare_smart(
        &mut self,
        goal: &[f64],
        times: &[f64],
        route: SpatialBoundaryDualPlan,
        mut receipt: BoundaryPreparation,
    ) -> CoreResult<BoundaryPreparation> {
        let n = goal.len();
        let m = times.len();
        let tf = times[m - 1];
        let mut ready = [false; 9];
        // Terminal is exact input. Stationary certificates share the exact
        // goal center and cached residual, so neither needs a pool slot.
        for (k, &t) in times.iter().enumerate() {
            if k == m - 1 || receipt.stationary {
                for (v, &c) in self.centers[k * n..(k + 1) * n].iter_mut().zip(goal) {
                    *v = Iv::point(c)?;
                }
                let mut w = SpatialAdjointWork::default();
                if k < m - 1 {
                    w.actions = 1;
                    w.l1_tail_upper = mul_up(
                        Iv::point(tf)?.sub(Iv::point(t)?)?.hi,
                        receipt.stationary_residual,
                    )?;
                    let tail = Iv::new(-w.l1_tail_upper, w.l1_tail_upper)?;
                    for &v in &self.centers[k * n..(k + 1) * n] {
                        let a = v.add(tail)?;
                        w.width_upper = w.width_upper.max(sub_up(a.hi, a.lo)?);
                    }
                }
                self.works[k] = w;
                ready[k] = true;
            } else if let Some(i) = self.pool.find(t) {
                self.centers[k * n..(k + 1) * n]
                    .copy_from_slice(&self.pool.centers[i * n..(i + 1) * n]);
                self.works[k] = self.pool.works[i];
                self.pool.touch(i);
                ready[k] = true;
                receipt.reused += 1;
            }
        }
        for k in (0..m - 1).rev() {
            if ready[k] {
                continue;
            }
            let t = times[k];
            let mut w = SpatialAdjointWork::default();
            let mut anchor = tf;
            if route == SpatialBoundaryDualPlan::SubsetStationaryIndependent {
                for (v, &c) in self.inner.state.iter_mut().zip(goal) {
                    *v = Iv::point(c)?;
                }
            } else {
                anchor = times[k + 1];
                w = self.works[k + 1];
                if let Some((i, v)) = self.pool.later(t, anchor) {
                    anchor = v;
                    w = self.pool.works[i];
                    self.inner
                        .state
                        .copy_from_slice(&self.pool.centers[i * n..(i + 1) * n]);
                    self.pool.touch(i);
                } else {
                    self.inner
                        .state
                        .copy_from_slice(&self.centers[(k + 1) * n..(k + 2) * n]);
                }
            }
            let old = w.actions;
            self.inner.propagate_series(
                Iv::point(anchor)?.sub(Iv::point(t)?)?,
                SpatialAdjointRoute::PoissonWideDecay,
                &mut w,
            )?;
            let cost = w.actions - old;
            receipt.actions += cost;
            let tail = Iv::new(-w.l1_tail_upper, w.l1_tail_upper)?;
            for &v in &self.inner.state {
                let a = v.add(tail)?;
                w.width_upper = w.width_upper.max(sub_up(a.hi, a.lo)?);
            }
            self.centers[k * n..(k + 1) * n].copy_from_slice(&self.inner.state);
            self.works[k] = w;
            receipt.built += 1;
            receipt.evicted += usize::from(self.pool.put_smart(t, &self.inner.state, w, cost));
        }
        self.times[..m].copy_from_slice(times);
        self.count = m;
        receipt.cache_hit = receipt.built == 0 && receipt.actions == 0;
        self.preparation = receipt;
        Ok(receipt)
    }
    /// Copy the midpoint of a currently prepared directed center for a
    /// proposal-only monitor. This is not a certified covector enclosure.
    pub(super) fn copy_monitor_center(&self, k: usize, out: &mut [f64]) -> CoreResult<()> {
        let n = self.key.len();
        if k >= self.count || out.len() != n {
            return Err(err("monitor center"));
        }
        for (v, c) in out.iter_mut().zip(&self.centers[k * n..(k + 1) * n]) {
            *v = 0.5 * c.lo + 0.5 * c.hi;
        }
        Ok(())
    }
    /// Projection after removing the same scalar anchor at every boundary.
    /// L*1=0 gives a common constant which cancels from every signed defect.
    pub fn project(&self, k: usize, u: &[f64], anchor: f64) -> CoreResult<Iv> {
        let n = self.key.len();
        if k >= self.count
            || u.len() != n
            || !anchor.is_finite()
            || u.iter().any(|x| !x.is_finite())
        {
            return Err(err("boundary projection"));
        }
        let c = &self.centers[k * n..(k + 1) * n];
        let mut norm = 0_f64;
        for &x in u {
            norm = norm.max(Iv::point(x)?.sub(Iv::point(anchor)?)?.mag());
        }
        let radius = mul_up(self.works[k].l1_tail_upper, norm)?;
        pairwise_shifted_dot(c, u, anchor)?.add(Iv::new(-radius, radius)?)
    }
    pub fn receipt(&self, anchor: f64, start: Iv, end: Iv) -> CoreResult<SpatialAdjointWork> {
        if self.count < 2 {
            return Err(err("boundary receipt"));
        }
        let mut w = self.works[0];
        w.exact_goal = start
            .add(pairwise_point_sum(&self.key)?.scale(anchor)?)?
            .as_pair();
        w.signed_goal_error = start.sub(end)?.as_pair();
        Ok(w)
    }
}
