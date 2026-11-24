//! Precompute the state-independent part of a declared additive RHS on RODAS5P stage clocks.
//! The user callbacks must be pure for a fixed model. A model/parameter change requires
//! another prepared problem. Resources, traces or clocks with side effects are unsuitable.
//! The state-dependent stages and all linear solves retain their causal order.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::ParallelExecution;

use crate::{CallbackError, Problem, SolveError, problem::RhsFn};

type TimeForcing = Arc<dyn Fn(f64, &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;
type ForcingRow = (u64, Result<Vec<f64>, CallbackError>);

/// Cumulative work across returned problems from one `SplitForcing` instance.
/// Sample after a run (or subtract two quiescent snapshots). Rejected and speculative
/// work is included. The fields are sampled independently while workers are running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForcingWork {
    /// Distinct rows evaluated in advance.
    pub prepared: u64,
    /// Composite RHS requests for a row already prepared (including a stored error).
    pub hits: u64,
    /// On-demand forcing evaluations, including every request in a direct problem.
    pub misses: u64,
    /// Actual forcing evaluations reporting an error or an invalid output vector.
    pub failed_evaluations: u64,
}

#[derive(Default)]
struct Counts {
    prepared: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    failed: AtomicU64,
}

fn evaluate(
    g: &TimeForcing,
    clock: f64,
    n: usize,
    counts: &Counts,
) -> Result<Vec<f64>, CallbackError> {
    let mut values = vec![f64::NAN; n];
    let row = g(clock, &mut values).and_then(|()| {
        if values.iter().all(|v| v.is_finite()) {
            Ok(values)
        } else {
            Err(CallbackError::Fatal(
                "split forcing did not produce a complete finite vector".into(),
            ))
        }
    });
    if row.is_err() {
        counts.failed.fetch_add(1, Ordering::Relaxed);
    }
    row
}

/// Split RHS `N(t, y) + g(t)` and a reusable, bounded pool for its time-only callback.
pub struct SplitForcing {
    dimension: usize,
    state: RhsFn,
    forcing: TimeForcing,
    workers: ParallelExecution,
    counts: Arc<Counts>,
}

impl SplitForcing {
    /// Both callbacks must fill the whole output and may report ordinary facade callback errors.
    /// `threads = 1` enables serial deduplication. Multiple workers evaluate distinct times.
    pub fn new<N, G>(
        dimension: usize,
        threads: usize,
        state: N,
        forcing: G,
    ) -> Result<Self, SolveError>
    where
        N: Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
        G: Fn(f64, &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
    {
        if dimension == 0 {
            return Err(SolveError::InvalidInput(
                "split forcing requires positive dimension".into(),
            ));
        }
        let workers = ParallelExecution::rayon(threads)
            .map_err(|err| SolveError::InvalidInput(err.to_string()))?;
        Ok(Self {
            dimension,
            state: Arc::new(state),
            forcing: Arc::new(forcing),
            workers,
            counts: Arc::new(Counts::default()),
        })
    }

    /// Read the counters accumulated by this instance and its returned problems.
    pub fn work(&self) -> ForcingWork {
        ForcingWork {
            prepared: self.counts.prepared.load(Ordering::Relaxed),
            hits: self.counts.hits.load(Ordering::Relaxed),
            misses: self.counts.misses.load(Ordering::Relaxed),
            failed_evaluations: self.counts.failed.load(Ordering::Relaxed),
        }
    }

    /// Ordinary composite RHS with on-demand forcing, suitable as the serial reference.
    pub fn direct_problem(&self) -> Problem {
        self.build_problem(Vec::new())
    }

    /// Prepare the clock grid for a proposed attempt from `t` with positive step `h`.
    /// The returned problem can be used directly with the normal facade. A rejected trial,
    /// clipped step, time derivative probe, or subsequent step with a different clock calls
    /// `g` directly on a miss. The cache compares exact floating-point timestamp bits.
    ///
    /// Supply the *total* explicit time derivative if using `Problem::time_derivative`.
    /// The default finite-difference estimator sees the full composite RHS. An error from
    /// a speculative forcing evaluation is delivered only if the solver requests that time.
    /// Each returned problem owns one immutable table of at most eight vectors.
    pub fn prepare_rodas5p_trial(&self, t: f64, h: f64) -> Result<Problem, SolveError> {
        if !(t.is_finite() && h.is_finite() && h > 0.0 && (t + h).is_finite() && t + h > t) {
            return Err(SolveError::InvalidInput(
                "invalid split-forcing trial clock".into(),
            ));
        }
        let coeff =
            rodas5p_coefficients().map_err(|err| SolveError::InvalidInput(err.to_string()))?;
        // Stage zero uses the base t directly in the native driver (important for signed zero).
        let mut times = vec![t];
        for &c in coeff.c.iter().skip(1) {
            let clock = t + c * h;
            if !times.iter().any(|x| x.to_bits() == clock.to_bits()) {
                times.push(clock);
            }
        }
        let rows = self
            .workers
            .map_ordered(&times, |&clock| {
                self.counts.prepared.fetch_add(1, Ordering::Relaxed);
                let result = evaluate(&self.forcing, clock, self.dimension, &self.counts);
                Ok((clock.to_bits(), result))
            })
            .map_err(|err| SolveError::InvalidInput(err.to_string()))?;
        Ok(self.build_problem(rows))
    }

    fn build_problem(&self, rows: Vec<ForcingRow>) -> Problem {
        let state = Arc::clone(&self.state);
        let forcing = Arc::clone(&self.forcing);
        let counts = Arc::clone(&self.counts);
        let n = self.dimension;
        Problem::new(n, move |clock, y, out| {
            if out.len() != n || y.len() != n {
                return Err(CallbackError::Fatal("split RHS dimension mismatch".into()));
            }
            out.fill(f64::NAN);
            state(clock, y, out)?;
            if out.iter().any(|v| !v.is_finite()) {
                return Err(CallbackError::Fatal(
                    "split state callback did not produce a complete finite vector".into(),
                ));
            }
            let temporary;
            let g = if let Some((_, row)) = rows.iter().find(|r| r.0 == clock.to_bits()) {
                counts.hits.fetch_add(1, Ordering::Relaxed);
                row.as_ref().map_err(Clone::clone)?.as_slice()
            } else {
                counts.misses.fetch_add(1, Ordering::Relaxed);
                temporary = evaluate(&forcing, clock, n, &counts)?;
                &temporary
            };
            for (value, addition) in out.iter_mut().zip(g) {
                *value += addition;
                if !value.is_finite() {
                    return Err(CallbackError::Fatal("split RHS sum is nonfinite".into()));
                }
            }
            Ok(())
        })
    }
}
