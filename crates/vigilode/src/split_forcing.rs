//! Precompute the state-independent part of a declared additive RHS on RODAS5P stage clocks.
//! The user callbacks must be pure for a fixed model. A model/parameter change requires
//! another prepared problem. Resources, traces or clocks with side effects are unsuitable.
//! The state-dependent stages and all linear solves retain their causal order.

use std::sync::Arc;

use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::ParallelExecution;

use crate::{CallbackError, Problem, SolveError, problem::RhsFn};

type TimeForcing = Arc<dyn Fn(f64, &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;
type ForcingRow = (u64, Result<Vec<f64>, CallbackError>);

/// Split RHS `N(t, y) + g(t)` and a reusable, bounded pool for its time-only callback.
pub struct SplitForcing {
    dimension: usize,
    state: RhsFn,
    forcing: TimeForcing,
    workers: ParallelExecution,
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
        })
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
                let mut values = vec![0.0; self.dimension];
                let result = (self.forcing)(clock, &mut values).map(|()| values);
                Ok((clock.to_bits(), result))
            })
            .map_err(|err| SolveError::InvalidInput(err.to_string()))?;
        Ok(self.build_problem(rows))
    }

    fn build_problem(&self, rows: Vec<ForcingRow>) -> Problem {
        let state = Arc::clone(&self.state);
        let forcing = Arc::clone(&self.forcing);
        Problem::new(self.dimension, move |clock, y, out| {
            state(clock, y, out)?;
            let temporary;
            let g = if let Some((_, row)) = rows.iter().find(|r| r.0 == clock.to_bits()) {
                row.as_ref().map_err(Clone::clone)?.as_slice()
            } else {
                temporary = {
                    let mut values = vec![0.0; out.len()];
                    forcing(clock, &mut values)?;
                    values
                };
                &temporary
            };
            for (value, addition) in out.iter_mut().zip(g) {
                *value += addition;
            }
            Ok(())
        })
    }
}
