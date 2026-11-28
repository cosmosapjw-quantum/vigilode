//! Source-owned verified sampling of an already represented clock trajectory.
//! No numerical interpolation or rounded request parameter is a proof of
//! the physical radius: the source's exact flow is enclosed at the actual time.
use super::polynomial_clock::ClockPolynomial;
use super::triangular_goal::{clock_partition, initial_block, log1p, positive};
use super::*;
use crate::output::{DenseOutputPreparationWork, OutputCollector, OutputSchedule};
use std::cell::Cell;
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ClockDenseWork {
    pub slabs: usize,
    pub requests: usize,
    pub center_components: u64,
    pub radius_components: u64,
    pub max_radius: f64,
    pub log_terms: usize,
    pub square_roots: usize,
}
/// Fixed schedule and exact source authority; approximate centers supplied by
/// the caller. A failed slab preparation leaves the committed prefix intact.
pub struct ClockDenseSampler<'a> {
    src: &'a DaeSource,
    schedule: OutputSchedule,
    collector: OutputCollector,
    n: usize,
    coupled: bool,
    start: f64,
    end: f64,
    last: f64,
    next: usize,
    initial_x: [f64; 16],
    initial_center: [f64; 32],
    initial_radius: [f64; 32],
    initial: [Iv; 16],
    clocks: [ClockPolynomial; 8],
    measure: PolynomialClockRoute,
    log: TriangularGoalRoute,
    work: ClockDenseWork,
}
fn exact_state(
    n: usize,
    coupled: bool,
    initial: &[Iv; 16],
    clocks: &[ClockPolynomial; 8],
    start: f64,
    time: f64,
    measure: PolynomialClockRoute,
    log: TriangularGoalRoute,
    work: &mut TriangularGoalWork,
) -> CoreResult<[Iv; 32]> {
    let mut out = [zero(); 32];
    for i in (0..n).step_by(2) {
        let c = clocks[i / 2];
        let p0 = initial[i];
        let q0 = initial[i + 1];
        let m = c.measure(start, time, measure)?;
        let v = positive(p0.mul(m)?)?;
        let pe = p0.div(p(1.)?.add(v)?)?;
        let qe = q0.add(log1p(v, log, work)?.mul(c.ratio)?)?;
        let ue = if coupled { pe.add(qe)? } else { pe };
        out[i] = ue;
        out[i + 1] = qe;
        out[n + i] = ue;
        out[n + i + 1] = qe;
    }
    Ok(out)
}
impl<'a> ClockDenseSampler<'a> {
    pub fn new(
        src: &'a DaeSource,
        span: (f64, f64),
        x: &[f64],
        center: &[f64],
        schedule: &OutputSchedule,
        measure: PolynomialClockRoute,
        log: TriangularGoalRoute,
    ) -> CoreResult<Self> {
        let n = src.nx();
        if n > 16 || center.len() != 2 * n || center.iter().any(|v| !v.is_finite()) {
            return Err(bad("clock dense initial center shape"));
        }
        let coupled = clock_partition(src, x, &[0.; 32][..2 * n], span.0, span.1)?;
        let mut initial = [zero(); 16];
        let mut clocks = [ClockPolynomial::coupled(); 8];
        for i in (0..n).step_by(2) {
            clocks[i / 2] = if coupled {
                ClockPolynomial::coupled()
            } else {
                ClockPolynomial::parse(src, i)?
            };
            let (pp, q) = initial_block(x, i, coupled)?;
            initial[i] = pp;
            initial[i + 1] = q;
        }
        let mut initial_x = [0.; 16];
        initial_x[..n].copy_from_slice(x);
        let mut initial_center = [0.; 32];
        initial_center[..2 * n].copy_from_slice(center);
        let mut initial_radius = [0.; 32];
        for i in 0..2 * n {
            initial_radius[i] = p(x[i % n])?.sub(p(center[i])?)?.mag();
        }
        let collector = OutputCollector::new(schedule, span, center)?
            .with_verified_radii(&initial_radius[..2 * n])?;
        Ok(Self {
            src,
            schedule: schedule.clone(),
            collector,
            n,
            coupled,
            start: span.0,
            end: span.1,
            last: span.0,
            next: 1,
            initial_x,
            initial_center,
            initial_radius,
            initial,
            clocks,
            measure,
            log,
            work: ClockDenseWork {
                max_radius: initial_radius[..2 * n].iter().copied().fold(0., f64::max),
                ..Default::default()
            },
        })
    }
    pub fn matches(&self, src: &DaeSource, span: (f64, f64), x: &[f64], center: &[f64]) -> bool {
        std::ptr::eq(src, self.src)
            && span == (self.start, self.end)
            && x == &self.initial_x[..self.n]
            && center == &self.initial_center[..2 * self.n]
    }
    pub fn reset(&mut self) -> CoreResult<()> {
        self.collector.reset_verified_prefix(
            &self.initial_center[..2 * self.n],
            &self.initial_radius[..2 * self.n],
        )?;
        self.last = self.start;
        self.next = 1;
        self.work = ClockDenseWork::default();
        self.work.max_radius = self.initial_radius[..2 * self.n]
            .iter()
            .copied()
            .fold(0., f64::max);
        Ok(())
    }
    pub fn view(&self) -> (&[f64], &[Vec<f64>], &[Vec<f64>]) {
        let (t, y) = self.collector.reusable_view();
        (t, y, self.collector.committed_output_radii())
    }
    pub fn complete(&self) -> bool {
        self.last == self.end && self.collector.is_complete()
    }
    pub fn work(&self) -> ClockDenseWork {
        self.work
    }
    /// Slab ownership (t_old,t_new]; endpoint uses actual represented center.
    /// Interior centers use the collector's rounded theta, but the radius
    /// encloses exact physical state at the original binary64 request time.
    pub fn advance(
        &mut self,
        times: [f64; 2],
        power: &[[Iv; 5]],
        endpoint: &[f64],
    ) -> CoreResult<()> {
        let n = self.n;
        let dim = 2 * n;
        if times[0] != self.last
            || !times[1].is_finite()
            || times[1] <= times[0]
            || times[1] > self.end
            || power.len() != dim
            || endpoint.len() != dim
            || power
                .iter()
                .flatten()
                .any(|v| !v.lo.is_finite() || !v.hi.is_finite() || v.lo > v.hi)
            || endpoint.iter().any(|v| !v.is_finite())
        {
            return Err(bad("clock dense slab shape/clock"));
        }
        let centers = Cell::new([0.; 32]);
        let mut next = self.next;
        let schedule = self.schedule.times();
        let mut log_work = TriangularGoalWork::default();
        let mut max_radius = self.work.max_radius;
        let mut receipt = DenseOutputPreparationWork::default();
        self.collector.prepare_dense_interval_with_radii(
            times[0],
            times[1],
            endpoint,
            |theta, out| {
                let mut c = [0.; 32];
                for i in 0..dim {
                    let v = eval(&power[i], p(theta)?)?;
                    c[i] = 0.5 * v.lo + 0.5 * v.hi;
                }
                out.copy_from_slice(&c[..dim]);
                centers.set(c);
                Ok(())
            },
            |_, out| {
                let time = *schedule
                    .get(next)
                    .ok_or_else(|| bad("clock dense due time"))?;
                let exact = exact_state(
                    n,
                    self.coupled,
                    &self.initial,
                    &self.clocks,
                    self.start,
                    time,
                    self.measure,
                    self.log,
                    &mut log_work,
                )?;
                let c = centers.get();
                let center = if time == times[1] {
                    endpoint
                } else {
                    &c[..dim]
                };
                for i in 0..dim {
                    out[i] = exact[i].sub(p(center[i])?)?.mag();
                    max_radius = max_radius.max(out[i]);
                }
                next += 1;
                Ok(())
            },
            &mut receipt,
        )?;
        self.collector.commit_prepared_dense_interval();
        self.next = next;
        self.last = times[1];
        self.work.slabs += 1;
        self.work.requests += receipt.requests_attempted as usize;
        self.work.center_components += receipt.center_components;
        self.work.radius_components += receipt.radius_components;
        self.work.max_radius = max_radius;
        self.work.log_terms += log_work.series_terms;
        self.work.square_roots += log_work.square_roots;
        Ok(())
    }
}
