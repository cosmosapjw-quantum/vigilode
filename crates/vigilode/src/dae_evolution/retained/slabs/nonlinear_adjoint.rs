//! True-flow continuous terminal sensitivity on the actual certified DAE run.
//! Whole-tube and subcell bounds are deliberately distinct from numerical
//! proposals along the interpolated primal path.
use super::*;
mod scalar;
use rodas5p_core::directed::{div_up, exp_interval, sub_up};
use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5Config, DopriStageKernel, OutputSchedule,
    integrate_dopri5_observed_with_kernel,
};
use std::cell::RefCell;
const Z: Iv = Iv { lo: 0., hi: 0. };
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NonlinearAdjointRoute {
    Whole,
    Sub4,
    Contracted16,
    ScalarExp16,
}
impl NonlinearAdjointRoute {
    fn subdivisions(self) -> usize {
        match self {
            Self::Whole => 1,
            Self::Sub4 => 4,
            _ => 16,
        }
    }
    fn passes(self) -> usize {
        if matches!(self, Self::Contracted16 | Self::ScalarExp16) {
            2
        } else {
            0
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct NonlinearAdjointWork {
    pub schur: de::SchurAdjointWork,
    pub slabs: usize,
    pub cells: usize,
    pub matrix_actions: usize,
    pub scalar_exponentials: usize,
    pub max_order: usize,
    pub max_cell_tail: f64,
    pub max_coefficient_width: f64,
    pub width_upper: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct NonlinearAdjointProposal {
    pub gradient: Vec<f64>,
    pub rhs_calls: u64,
    pub steps: usize,
    pub source_projections: usize,
}
pub struct NonlinearAdjointWorkspace {
    schur: de::SchurAdjointWorkspace,
}
impl NonlinearAdjointWorkspace {
    pub fn new() -> Self {
        Self {
            schur: de::SchurAdjointWorkspace::new(),
        }
    }
}
impl Default for NonlinearAdjointWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
fn intersection(a: Iv, b: Iv) -> CoreResult<Iv> {
    Iv::new(a.lo.max(b.lo), a.hi.min(b.hi))
}
fn pair(v: [f64; 2]) -> CoreResult<Iv> {
    Iv::new(v[0], v[1])
}
fn interval_exp(v: Iv) -> CoreResult<Iv> {
    Iv::new(exp_interval(v.lo)?.lo.max(0.), exp_interval(v.hi)?.hi)
}
fn physical_cell_box(
    src: &DaeSource,
    s: &CompactSlabView<'_>,
    theta: Iv,
    local: bool,
) -> CoreResult<(Iv, Iv, [Iv; 32])> {
    let nx = src.nx();
    let nz = src.nz();
    let n = nx + nz;
    let c = s
        .certificate
        .as_ref()
        .filter(|c| *c.global_qualified)
        .ok_or_else(|| bad("adjoint requires qualified slab"))?;
    let ex = c
        .whole_x_error
        .ok_or_else(|| bad("adjoint missing whole x error"))?;
    let ez = c
        .whole_z_error
        .ok_or_else(|| bad("adjoint missing whole z error"))?;
    if ex < 0.
        || ez < 0.
        || ex >= *c.tube_radius
        || c.x_range.len() != nx
        || c.branch_root_box.len() != nz
    {
        return Err(bad("adjoint primal tube contract"));
    }
    let h = Iv::point(s.times[1])?.sub(Iv::point(s.times[0])?)?;
    if h.lo <= 0. || theta.lo < 0. || theta.hi > 1. {
        return Err(bad("adjoint cell clock"));
    }
    let time = intersection(
        Iv::point(s.times[0])?.add(h.mul(theta)?)?,
        Iv::new(s.times[0], s.times[1])?,
    )?;
    let mut y = [Z; 32];
    for (i, r) in c.x_range.iter().chain(c.branch_root_box).enumerate() {
        y[i] = pair(*r)?;
    }
    if local {
        let row = |i: usize| -> &[[f64; 2]] {
            match &s.power {
                RetainedPowerRows::Owned(p) => &p[i],
                RetainedPowerRows::Flat(p) => &p[i],
            }
        };
        let count = match &s.power {
            RetainedPowerRows::Owned(p) => p.len(),
            RetainedPowerRows::Flat(p) => p.len(),
        };
        if count != n {
            return Err(bad("adjoint polynomial rows"));
        }
        for i in 0..n {
            let p = row(i);
            if p.is_empty() || p.len() > 5 {
                return Err(bad("adjoint polynomial degree"));
            }
            let v = p
                .iter()
                .rev()
                .try_fold(Z, |s, &c| s.mul(theta)?.add(pair(c)?))?;
            let rad = if i < nx { ex } else { ez };
            y[i] = intersection(y[i], v.add(Iv::new(-rad, rad)?)?)?;
        }
    }
    Ok((h, time, y))
}
fn bound_cell(
    src: &DaeSource,
    s: &CompactSlabView<'_>,
    theta: Iv,
    local: bool,
    passes: usize,
    weights: &[f64],
    w: &mut NonlinearAdjointWorkspace,
    a: &mut [Iv],
    goal: &mut [Iv],
    work: &mut NonlinearAdjointWork,
) -> CoreResult<Iv> {
    let n = src.nx() + src.nz();
    let c = s
        .certificate
        .as_ref()
        .ok_or_else(|| bad("adjoint certificate"))?;
    let (h, time, y) = physical_cell_box(src, s, theta, local)?;
    let refinement = if passes > 0 {
        de::GuardColumnRefinement::AdaptiveGaussSeidel2
    } else {
        de::GuardColumnRefinement::None
    };
    w.schur.enclose(
        src,
        time,
        &y[..n],
        c.preconditioner,
        weights,
        passes,
        refinement,
        a,
        goal,
        &mut work.schur,
    )?;
    for &v in a.iter() {
        work.max_coefficient_width = work.max_coefficient_width.max(sub_up(v.hi, v.lo)?);
    }
    Ok(h)
}
/// Time-ordered Peano-Baker enclosure. If every A(t) belongs to an
/// interval matrix, products of independent members enclose every
/// ordered integrand. The time simplex volume is h^k/k!, irrespective of
/// commutativity. The usual induced-norm tail encloses the omitted terms.
fn advance(
    a: &[Iv],
    n: usize,
    h: Iv,
    v: &mut [Iv],
    scalar: bool,
    w: &mut NonlinearAdjointWork,
) -> CoreResult<()> {
    if scalar {
        if n != 1 {
            return Err(bad("scalar exponential needs one differential state"));
        }
        v[0] = v[0].mul(interval_exp(h.mul(a[0])?)?)?;
        w.scalar_exponentials += 1;
        return Ok(());
    }
    let mut b = [Z; 1024];
    let mut m = 0_f64;
    for i in 0..n {
        let mut s = 0.;
        for j in 0..n {
            b[i * n + j] = h.mul(a[j * n + i])?;
            s = add_up(s, b[i * n + j].mag())?;
        }
        m = m.max(s);
    }
    // This is a bounded high-order enclosure, not an adaptive ODE step.
    // Refuse large arguments instead of silently loosening the tail.
    if m > 2. {
        return Err(bad("adjoint cell norm cap; subdivide"));
    }
    let size = v.iter().map(|x| x.mag()).fold(0., f64::max);
    let gain = exp_interval(m)?.hi;
    let mut tail = mul_up(size, gain)?;
    let mut order = 0;
    for k in 1..=25 {
        tail = div_up(mul_up(tail, m)?, k as f64)?;
        if k >= 3 && tail <= 1e-15 {
            order = k - 1;
            break;
        }
    }
    if order == 0 {
        return Err(bad("adjoint Taylor tail cap"));
    }
    let mut term = [Z; 32];
    term[..n].copy_from_slice(v);
    let mut sum = term;
    for k in 1..=order {
        let mut next = [Z; 32];
        for i in 0..n {
            let mut s = Z;
            for j in 0..n {
                if b[i * n + j] != Z {
                    s = s.add(b[i * n + j].mul(term[j])?)?;
                }
            }
            next[i] = s.div(Iv::point(k as f64)?)?;
            sum[i] = sum[i].add(next[i])?;
        }
        term = next;
        w.matrix_actions += 1;
    }
    for i in 0..n {
        v[i] = sum[i].add(Iv::new(-tail, tail)?)?;
    }
    w.max_order = w.max_order.max(order);
    w.max_cell_tail = w.max_cell_tail.max(tail);
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ScalarAdjointRoute {
    Repeated16,
    IntegralGuard16,
    Direct16,
    DirectBare16,
    Adaptive,
    FlowRatio,
    QuadraticSeed16,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ScalarAdjointWork {
    pub source_rows: usize,
    pub cells: usize,
    pub exponentials: usize,
    pub root_passes: usize,
    pub column_solves: usize,
    pub refinement_sweeps: usize,
    pub divisions: usize,
    pub split_evaluations: usize,
    pub cap_leaves: usize,
    pub quadratic_seed_uses: usize,
    pub width_upper: f64,
}
impl SlabReplayRun {
    fn adjoint_admission(&self, weights: &[f64]) -> CoreResult<&Engine> {
        let r = &self.inner.run;
        let e = &r.engine;
        if !r.global_qualified
            || e.warp.is_some()
            || e.conservation.is_some()
            || r.route == EvolutionRoute::Mass
            || !r.driver.is_success()
            // Legacy zero-origin, zero-incoming epochs omit this metadata
            // by design. Do not alter the existing serialized contract.
            || !r.epoch.map_or(r.driver.t.first()==Some(&0.),|a| {
                a.incoming_x_error == Some(0.)
                    && r.driver.t.first() == Some(&a.start)
                    && r.driver.t.last() == Some(&a.end)
            })
            || !r.slabs.first().and_then(|s|s.certificate.as_ref()).is_some_and(|c|c.global_qualified&&c.incoming==0.)
            || self.slabs_len() == 0
            || weights.len() != e.src.nx() + e.src.nz()
            || weights.iter().any(|w| !w.is_finite())
        {
            return Err(bad(
                "nonlinear adjoint needs full qualified zero-incoming reduced epoch and finite goal",
            ));
        }
        Ok(e)
    }
    /// Return an outward enclosure of the true continuous gradient.
    /// Output remains untouched on any refusal. Includes uncertainty from
    /// the original primal polynomial and its certified physical tube.
    pub fn nonlinear_adjoint_into(
        &self,
        weights: &[f64],
        route: NonlinearAdjointRoute,
        w: &mut NonlinearAdjointWorkspace,
        out: &mut [Iv],
    ) -> CoreResult<NonlinearAdjointWork> {
        let e = self.adjoint_admission(weights)?;
        let n = e.src.nx();
        let m = self.slabs_len();
        if out.len() != n || route == NonlinearAdjointRoute::ScalarExp16 && n != 1 {
            return Err(bad("adjoint output dimension/route"));
        }
        let mut work = NonlinearAdjointWork::default();
        let mut a = [Z; 1024];
        let mut goal = [Z; 32];
        let mut v = [Z; 32];
        let last = self.slab(m - 1)?;
        // Always use the point cell for the terminal covector, including
        // physical radius; do not use numerical projected z alone.
        bound_cell(
            &e.src,
            &last,
            Iv::point(1.)?,
            true,
            route.passes(),
            weights,
            w,
            &mut a[..n * n],
            &mut v[..n],
            &mut work,
        )?;
        let cells = route.subdivisions();
        for k in (0..m).rev() {
            let slab = self.slab(k)?;
            for j in (0..cells).rev() {
                let theta = Iv::new(j as f64 / cells as f64, (j + 1) as f64 / cells as f64)?;
                let h = bound_cell(
                    &e.src,
                    &slab,
                    theta,
                    route != NonlinearAdjointRoute::Whole,
                    route.passes(),
                    weights,
                    w,
                    &mut a[..n * n],
                    &mut goal[..n],
                    &mut work,
                )?;
                advance(
                    &a[..n * n],
                    n,
                    h.div(Iv::point(cells as f64)?)?,
                    &mut v[..n],
                    route == NonlinearAdjointRoute::ScalarExp16,
                    &mut work,
                )?;
                work.cells += 1;
            }
            work.slabs += 1;
        }
        for v in &v[..n] {
            work.width_upper = work.width_upper.max(sub_up(v.hi, v.lo)?);
        }
        out.copy_from_slice(&v[..n]);
        Ok(work)
    }
    /// Numerical proposal using the original projection/Jacobian on the
    /// native dense path. It is not the certificate. Detach work/audit so
    /// reading sensitivities cannot mutate the primal result's receipts.
    pub fn nonlinear_adjoint_dopri(
        &self,
        weights: &[f64],
        opts: &Options,
        kernel: DopriStageKernel,
    ) -> CoreResult<NonlinearAdjointProposal> {
        let e = self.adjoint_admission(weights)?;
        let n = e.src.nx();
        let mut e = e.clone();
        e.audit = None;
        e.work = Arc::new(Mutex::new(ProjectionWork::default()));
        let s = self.driver();
        let start = s.t[0];
        let end = *s.t.last().unwrap();
        let duration = Iv::point(end)?.sub(Iv::point(start)?)?;
        if duration.lo != duration.hi || duration.lo <= 0. {
            return Err(bad("adjoint reverse clock"));
        }
        let mut q = QueryWorkspace::new();
        let x = s.y.last().ok_or_else(|| bad("adjoint terminal state"))?;
        let report = e.query_projection(end, x, &e.seed, &mut q)?.owned();
        let mut goal = [0.; 32];
        for i in 0..n {
            goal[i] = weights[i];
            for j in 0..e.src.nz() {
                goal[i] = weights[n + j].mul_add(report.zx_columns[i][j], goal[i]);
            }
        }
        let scratch = RefCell::new(q);
        let rhs = |u: f64, v: &[f64], out: &mut [f64]| -> CoreResult<()> {
            let t = (end - u).clamp(start, end);
            let mut x = [0.; 32];
            s.at_into(t, &mut x[..n])
                .ok_or_else(|| bad("adjoint dense query"))?;
            let p = e.query_projection(t, &x[..n], &e.seed, &mut *scratch.borrow_mut())?;
            let a = match &p {
                QueryProjection::Scalar(v) => v.reduced_jacobian(),
                QueryProjection::Vector(v) => v.reduced_jacobian(),
            };
            for i in 0..n {
                out[i] = (0..n).fold(0., |s, j| a[j * n + i].mul_add(v[j], s));
            }
            Ok(())
        };
        let atol = match opts.atol {
            Atol::Scalar(v) => v,
            _ => return Err(bad("adjoint numerical scalar tolerance")),
        };
        let cfg = Dopri5Config {
            adaptive: AdaptiveStepConfig {
                atol,
                rtol: opts.rtol,
                initial_step: opts.h0,
                max_attempts: opts.max_steps,
                ..Default::default()
            },
            ..Default::default()
        };
        let span = (0., duration.lo);
        let output = OutputSchedule::new(vec![span.0, span.1])?;
        let r =
            integrate_dopri5_observed_with_kernel(&rhs, span, &goal[..n], &cfg, &output, kernel)?;
        if !r.observed.success {
            return Err(bad("adjoint numerical solve incomplete"));
        }
        let calls = e.work.lock().expect("adjoint work").calls;
        Ok(NonlinearAdjointProposal {
            gradient: r.final_state,
            rhs_calls: r.observed.counters.rhs_calls,
            steps: r.accepted_steps,
            source_projections: calls,
        })
    }
}
