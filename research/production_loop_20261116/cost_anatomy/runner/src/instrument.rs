//! Result-neutral wrappers of the V04 cost anatomy: the right-hand-side cost multiplier, the
//! finite-difference Jacobians, the wall clocks around the callbacks and the clock-free marker
//! functions that give callgrind one symbol per phase.
//!
//! "Result-neutral" means that a wrapper forwards the arguments and the result unchanged; the
//! runner checks it on every pair of runs (attempt counts, counters and the bits of the final
//! state against the protected driver on the unwrapped problem).

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rodas5p_core::{CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{JacobianIntoFn, OdeProblem, StageLinearSystem};

use crate::problems::{BandFillFn, JvpFn, RhsFn};

/// Evaluate the same right-hand side `k` times and keep the last result.
pub fn multiplied(rhs: RhsFn, k: u32) -> RhsFn {
    if k <= 1 {
        return rhs;
    }
    Arc::new(move |t, y, out| {
        for _ in 0..k {
            rhs(t, y, out)?;
            black_box(&mut *out);
        }
        Ok(())
    })
}

/// A single-writer counter (the runs are single-threaded; the atomics only make the closures
/// `Sync`).
#[derive(Default)]
pub struct Counter(AtomicU64);

impl Counter {
    #[inline]
    pub fn add(&self, value: u64) {
        self.0
            .store(self.0.load(Relaxed).wrapping_add(value), Relaxed);
    }
    pub fn get(&self) -> u64 {
        self.0.load(Relaxed)
    }
    pub fn reset(&self) {
        self.0.store(0, Relaxed);
    }
}

const SQRT_EPS: f64 = 1.490_116_119_384_765_6e-8;

/// The forward-difference increment of component `j`.
#[inline]
fn increment(yj: f64) -> f64 {
    SQRT_EPS * yj.abs().max(1.0)
}

/// A dense forward-difference Jacobian of `rhs`: `n + 1` evaluations, every entry written.
/// `calls` counts the evaluations (they do not pass through the driver's counters).
pub fn fd_dense(rhs: RhsFn, n: usize, calls: Arc<Counter>) -> JacobianIntoFn {
    let scratch = Mutex::new((vec![0.0; n], vec![0.0; n], vec![0.0; n]));
    Arc::new(move |t, y, out: &mut DenseMatrix| {
        let mut guard = scratch.lock().unwrap_or_else(|e| e.into_inner());
        let (yp, f0, f1) = &mut *guard;
        yp.copy_from_slice(y);
        rhs(t, y, f0)?;
        calls.add(1);
        let data = out.as_mut_slice();
        for j in 0..n {
            let saved = yp[j];
            yp[j] = saved + increment(saved);
            let inv = 1.0 / (yp[j] - saved);
            rhs(t, yp, f1)?;
            calls.add(1);
            yp[j] = saved;
            for i in 0..n {
                data[i * n + j] = (f1[i] - f0[i]) * inv;
            }
        }
        Ok(())
    })
}

/// A banded forward-difference Jacobian of `rhs` with the Curtis-Powell-Reid grouping of a band:
/// columns `j` and `j + lower + upper + 1` share an evaluation, so one Jacobian costs
/// `min(n, lower + upper + 1) + 1` evaluations.
pub fn fd_band(
    rhs: RhsFn,
    n: usize,
    lower: usize,
    upper: usize,
    calls: Arc<Counter>,
) -> BandFillFn {
    let scratch = Mutex::new((vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]));
    let jw = lower + upper + 1;
    Arc::new(move |t, y, band: &mut [f64]| {
        let mut guard = scratch.lock().unwrap_or_else(|e| e.into_inner());
        let (yp, f0, f1, inv) = &mut *guard;
        yp.copy_from_slice(y);
        rhs(t, y, f0)?;
        calls.add(1);
        for group in 0..jw.min(n) {
            for j in (group..n).step_by(jw) {
                yp[j] = y[j] + increment(y[j]);
                inv[j] = 1.0 / (yp[j] - y[j]);
            }
            rhs(t, yp, f1)?;
            calls.add(1);
            for j in (group..n).step_by(jw) {
                yp[j] = y[j];
                let first = j.saturating_sub(upper);
                let last = (j + lower).min(n - 1);
                for i in first..=last {
                    band[i * jw + (j + lower - i)] = (f1[i] - f0[i]) * inv[j];
                }
            }
        }
        Ok(())
    })
}

/// A forward-difference JVP of `rhs` (the Jacobian-free Newton-Krylov product):
/// `J v ~ (f(y + e v) - f(y)) / e`, `e = sqrt(eps) (1 + |y|_2) / |v|_2`. `f(y)` is cached for the
/// last `y` (compared bit for bit), so a product costs one evaluation after the first.
pub fn fd_jvp(rhs: RhsFn, n: usize, calls: Arc<Counter>) -> JvpFn {
    struct Cache {
        y_bits: Vec<u64>,
        valid: bool,
        f0: Vec<f64>,
        yp: Vec<f64>,
        ynorm: f64,
    }
    let cache = Mutex::new(Cache {
        y_bits: vec![0; n],
        valid: false,
        f0: vec![0.0; n],
        yp: vec![0.0; n],
        ynorm: 0.0,
    });
    Arc::new(move |t, y, v, out| {
        let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        let cache = &mut *guard;
        let same = cache.valid && cache.y_bits.iter().zip(y).all(|(b, x)| *b == x.to_bits());
        if !same {
            cache.valid = false;
            rhs(t, y, &mut cache.f0)?;
            calls.add(1);
            for (b, x) in cache.y_bits.iter_mut().zip(y) {
                *b = x.to_bits();
            }
            cache.ynorm = y.iter().map(|x| x * x).sum::<f64>().sqrt();
            cache.valid = true;
        }
        let vnorm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if vnorm == 0.0 {
            out.fill(0.0);
            return Ok(());
        }
        let e = SQRT_EPS * (1.0 + cache.ynorm) / vnorm;
        for ((p, x), d) in cache.yp.iter_mut().zip(y).zip(v) {
            *p = x + e * d;
        }
        rhs(t, &cache.yp, out)?;
        calls.add(1);
        let inv = 1.0 / e;
        for (o, f) in out.iter_mut().zip(&cache.f0) {
            *o = (*o - f) * inv;
        }
        Ok(())
    })
}

// ---------------------------------------------------------------------------------------------
// Wall clocks.
// ---------------------------------------------------------------------------------------------

/// Accumulated wall time and calls of the right-hand side as the driver calls it (the
/// evaluations inside a finite-difference Jacobian are not routed through this clock).
#[derive(Default)]
pub struct RhsClock {
    pub ns: Counter,
    pub calls: Counter,
}

impl RhsClock {
    pub fn reset(&self) {
        self.ns.reset();
        self.calls.reset();
    }
}

#[inline]
fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

pub fn timed_rhs(rhs: RhsFn, clock: Arc<RhsClock>) -> RhsFn {
    Arc::new(move |t, y, out| {
        let start = Instant::now();
        let result = rhs(t, y, out);
        clock.ns.add(elapsed_ns(start));
        clock.calls.add(1);
        result
    })
}

/// The timeline of a matrix-free run, cut at the callback boundaries.
///
/// Every nanosecond between [`MfClock::start`] and [`MfClock::finish`] falls into exactly one
/// of: a right-hand-side call (`rhs_ns`), the stage-solve interval of stage `i`
/// (`solve_interval_ns[i]`: from the end of the right-hand-side call that precedes the solve,
/// or from the first JVP of an attempt that reuses `f(t, y)`, to the start of the next
/// right-hand-side call or the attempt observer), or the rest (`other_ns`: controller, output,
/// attempt set-up). The JVP time of stage `i` (`jvp_ns[i]`) lies inside its solve interval.
pub struct MfClock {
    base: Instant,
    mark: AtomicU64,
    /// 0: outside a solve interval; 1: inside the solve interval of `stage`.
    in_solve: AtomicUsize,
    stage: AtomicUsize,
    /// 1 when the next attempt evaluates `f(t, y)` (the first attempt and every attempt after
    /// an accepted one); 0 when it reuses the one of a rejected attempt.
    next_fresh: AtomicUsize,
    pub rhs_ns: Counter,
    pub rhs_calls: Counter,
    pub other_ns: Counter,
    pub solve_interval_ns: [Counter; 8],
    pub jvp_ns: [Counter; 8],
    pub jvp_calls: [Counter; 8],
    pub attempts_seen: Counter,
}

impl MfClock {
    pub fn new() -> Self {
        Self {
            base: Instant::now(),
            mark: AtomicU64::new(0),
            in_solve: AtomicUsize::new(0),
            stage: AtomicUsize::new(0),
            next_fresh: AtomicUsize::new(1),
            rhs_ns: Counter::default(),
            rhs_calls: Counter::default(),
            other_ns: Counter::default(),
            solve_interval_ns: Default::default(),
            jvp_ns: Default::default(),
            jvp_calls: Default::default(),
            attempts_seen: Counter::default(),
        }
    }

    #[inline]
    fn now(&self) -> u64 {
        elapsed_ns(self.base)
    }

    /// Close the open interval at `at` and book it.
    #[inline]
    fn close(&self, at: u64) {
        let span = at.saturating_sub(self.mark.load(Relaxed));
        if self.in_solve.load(Relaxed) == 1 {
            self.solve_interval_ns[self.stage.load(Relaxed).min(7)].add(span);
        } else {
            self.other_ns.add(span);
        }
        self.mark.store(at, Relaxed);
    }

    /// Zero everything and start the timeline; returns the start time.
    pub fn start(&self) -> u64 {
        self.rhs_ns.reset();
        self.rhs_calls.reset();
        self.other_ns.reset();
        self.attempts_seen.reset();
        for i in 0..8 {
            self.solve_interval_ns[i].reset();
            self.jvp_ns[i].reset();
            self.jvp_calls[i].reset();
        }
        self.in_solve.store(0, Relaxed);
        self.stage.store(0, Relaxed);
        self.next_fresh.store(1, Relaxed);
        let at = self.now();
        self.mark.store(at, Relaxed);
        at
    }

    /// End the timeline; returns the end time.
    pub fn finish(&self) -> u64 {
        let at = self.now();
        self.close(at);
        self.in_solve.store(0, Relaxed);
        at
    }

    /// The attempt observer: the attempt has returned; `accepted` says whether the driver
    /// accepts it (the next attempt then starts from a new state).
    pub fn attempt_done(&self, accepted: bool) {
        let at = self.now();
        self.close(at);
        self.in_solve.store(0, Relaxed);
        self.next_fresh.store(usize::from(accepted), Relaxed);
        self.attempts_seen.add(1);
    }
}

pub fn mf_timed_rhs(rhs: RhsFn, clock: Arc<MfClock>) -> RhsFn {
    Arc::new(move |t, y, out| {
        let start = clock.now();
        // The first right-hand side of an attempt from a new state is f(t, y) of stage 1 (index
        // 0); every later one opens the next stage. An attempt that reuses f(t, y) and whose
        // first solve needed no JVP starts here at index 1.
        let was_in_solve = clock.in_solve.load(Relaxed) == 1;
        clock.close(start);
        if was_in_solve {
            clock.stage.store(clock.stage.load(Relaxed) + 1, Relaxed);
        } else if clock.next_fresh.load(Relaxed) == 1 {
            clock.stage.store(0, Relaxed);
        } else {
            clock.stage.store(1, Relaxed);
        }
        let result = rhs(t, y, out);
        let end = clock.now();
        clock.rhs_ns.add(end.saturating_sub(start));
        clock.rhs_calls.add(1);
        clock.mark.store(end, Relaxed);
        clock.in_solve.store(1, Relaxed);
        result
    })
}

pub fn mf_timed_jvp(jvp: JvpFn, clock: Arc<MfClock>) -> JvpFn {
    Arc::new(move |t, y, v, out| {
        let start = clock.now();
        if clock.in_solve.load(Relaxed) == 0 {
            // An attempt that reuses f(t, y): its first event is the first JVP of stage 1.
            clock.close(start);
            clock.stage.store(0, Relaxed);
            clock.in_solve.store(1, Relaxed);
        }
        let result = jvp(t, y, v, out);
        let end = clock.now();
        let stage = clock.stage.load(Relaxed).min(7);
        clock.jvp_ns[stage].add(end.saturating_sub(start));
        clock.jvp_calls[stage].add(1);
        result
    })
}

// ---------------------------------------------------------------------------------------------
// Clock-free markers: one unmangled, never-inlined symbol per phase, so that callgrind's
// inclusive cost of the symbol is the instruction count of the phase.
// ---------------------------------------------------------------------------------------------

type RhsDyn = dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync;
type JvpDyn = dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync;

#[unsafe(no_mangle)]
#[inline(never)]
pub fn v04_phase_rhs(rhs: &RhsDyn, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
    rhs(t, y, out)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn v04_phase_jvp(
    jvp: &JvpDyn,
    t: f64,
    y: &[f64],
    v: &[f64],
    out: &mut [f64],
) -> CoreResult<()> {
    jvp(t, y, v, out)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn v04_phase_linearize(
    client: &mut dyn StageLinearSystem,
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    client.linearize(problem, t, y, counters)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn v04_phase_factor(
    client: &mut dyn StageLinearSystem,
    inv_h_gamma: f64,
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    client.factor(inv_h_gamma, counters)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn v04_phase_solve(
    client: &mut dyn StageLinearSystem,
    b: &mut [f64],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    client.solve_in_place(b, counters)
}

pub fn marked_rhs(rhs: RhsFn) -> RhsFn {
    Arc::new(move |t, y, out| v04_phase_rhs(&*rhs, t, y, out))
}

pub fn marked_jvp(jvp: JvpFn) -> JvpFn {
    Arc::new(move |t, y, v, out| v04_phase_jvp(&*jvp, t, y, v, out))
}

/// Forwards every call of a client through the marker symbols; no clock, no counter.
pub struct MarkedClient {
    pub inner: Box<dyn StageLinearSystem>,
}

impl StageLinearSystem for MarkedClient {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        v04_phase_linearize(self.inner.as_mut(), problem, t, y, counters)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        v04_phase_factor(self.inner.as_mut(), inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        v04_phase_solve(self.inner.as_mut(), b, counters)
    }
}
