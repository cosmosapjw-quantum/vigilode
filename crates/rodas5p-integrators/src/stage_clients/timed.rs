//! Phase-timing wrapper client (loop node V15).
//!
//! [`TimedStageClient`] forwards every [`StageLinearSystem`] call to an
//! inner client and accumulates the wall time and the number of calls of
//! the three phases (linearize, factor, solve) in a [`PhaseTimes`]. It
//! changes no argument, result or counter, so a run with the wrapper is the
//! run of the inner client, bit for bit.
//!
//! The client driver takes its client as `Box<dyn StageLinearSystem>` and
//! hands it back type-erased, so the totals are also published through a
//! [`PhaseTimesHandle`] taken before the run. The wrapper is the only
//! writer; publishing is six relaxed atomic stores per call. No call
//! allocates: the shared cell is allocated once in
//! [`TimedStageClient::new`].
//!
//! Wall time on a shared host is diagnostic; the call counts are exact.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use rodas5p_core::{CoreResult, WorkCounters};

use crate::{OdeProblem, StageLinearSystem};

/// Accumulated wall time (nanoseconds) and calls of the three phases of a
/// [`StageLinearSystem`]. A call that returns an error is counted and timed
/// like any other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhaseTimes {
    pub linearize_ns: u64,
    pub factor_ns: u64,
    pub solve_ns: u64,
    pub linearize_calls: u64,
    pub factor_calls: u64,
    pub solve_calls: u64,
}

impl PhaseTimes {
    /// The time of the three phases together.
    pub fn total_ns(&self) -> u64 {
        self.linearize_ns
            .saturating_add(self.factor_ns)
            .saturating_add(self.solve_ns)
    }
}

/// The published copy of a wrapper's [`PhaseTimes`].
#[derive(Debug, Default)]
struct SharedPhaseTimes {
    linearize_ns: AtomicU64,
    factor_ns: AtomicU64,
    solve_ns: AtomicU64,
    linearize_calls: AtomicU64,
    factor_calls: AtomicU64,
    solve_calls: AtomicU64,
}

impl SharedPhaseTimes {
    fn publish(&self, times: &PhaseTimes) {
        self.linearize_ns
            .store(times.linearize_ns, Ordering::Relaxed);
        self.factor_ns.store(times.factor_ns, Ordering::Relaxed);
        self.solve_ns.store(times.solve_ns, Ordering::Relaxed);
        self.linearize_calls
            .store(times.linearize_calls, Ordering::Relaxed);
        self.factor_calls
            .store(times.factor_calls, Ordering::Relaxed);
        self.solve_calls.store(times.solve_calls, Ordering::Relaxed);
    }

    fn snapshot(&self) -> PhaseTimes {
        PhaseTimes {
            linearize_ns: self.linearize_ns.load(Ordering::Relaxed),
            factor_ns: self.factor_ns.load(Ordering::Relaxed),
            solve_ns: self.solve_ns.load(Ordering::Relaxed),
            linearize_calls: self.linearize_calls.load(Ordering::Relaxed),
            factor_calls: self.factor_calls.load(Ordering::Relaxed),
            solve_calls: self.solve_calls.load(Ordering::Relaxed),
        }
    }
}

/// A reader of a [`TimedStageClient`]'s totals that outlives the wrapper's
/// move into the driver. Read it after the run: a snapshot taken while a
/// call is in flight on another thread may mix two calls' fields.
#[derive(Clone, Debug)]
pub struct PhaseTimesHandle {
    shared: Arc<SharedPhaseTimes>,
}

impl PhaseTimesHandle {
    /// The totals published so far.
    pub fn snapshot(&self) -> PhaseTimes {
        self.shared.snapshot()
    }
}

/// A [`StageLinearSystem`] that times the calls of an inner client.
pub struct TimedStageClient<C> {
    inner: C,
    times: PhaseTimes,
    shared: Arc<SharedPhaseTimes>,
}

impl<C: StageLinearSystem> TimedStageClient<C> {
    /// Wrap `inner` with zeroed totals.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            times: PhaseTimes::default(),
            shared: Arc::new(SharedPhaseTimes::default()),
        }
    }

    /// A reader of the totals for after the wrapper has been boxed and
    /// moved into the driver.
    pub fn handle(&self) -> PhaseTimesHandle {
        PhaseTimesHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    /// The totals so far.
    pub fn times(&self) -> PhaseTimes {
        self.times
    }

    /// Zero the totals (the handles see the reset).
    pub fn reset(&mut self) {
        self.times = PhaseTimes::default();
        self.shared.publish(&self.times);
    }

    /// The wrapped client.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// The wrapped client, mutably.
    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// The wrapped client and the totals.
    pub fn into_parts(self) -> (C, PhaseTimes) {
        (self.inner, self.times)
    }
}

/// Nanoseconds since `start`, saturating.
#[inline]
fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

impl<C: StageLinearSystem> StageLinearSystem for TimedStageClient<C> {
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
        let start = Instant::now();
        let result = self.inner.linearize(problem, t, y, counters);
        self.times.linearize_ns = self.times.linearize_ns.saturating_add(elapsed_ns(start));
        self.times.linearize_calls += 1;
        self.shared.publish(&self.times);
        result
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let start = Instant::now();
        let result = self.inner.factor(inv_h_gamma, counters);
        self.times.factor_ns = self.times.factor_ns.saturating_add(elapsed_ns(start));
        self.times.factor_calls += 1;
        self.shared.publish(&self.times);
        result
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        let start = Instant::now();
        let result = self.inner.solve_in_place(b, counters);
        self.times.solve_ns = self.times.solve_ns.saturating_add(elapsed_ns(start));
        self.times.solve_calls += 1;
        self.shared.publish(&self.times);
        result
    }
}
