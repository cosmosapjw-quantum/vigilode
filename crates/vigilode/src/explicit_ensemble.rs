//! Independent adaptive explicit solves on a reusable, bounded local pool.
//!
//! Each member has its own time steps and component error norm. Combining
//! many members therefore cannot dilute one member's local error through a
//! larger shared RMS denominator. Input order is preserved, while callback
//! execution order is deliberately unspecified.

use rodas5p_integrators::ParallelExecution;

use crate::{Dopri5, ExplicitSolution, ExplicitSolveError, Options, Problem};

/// One independently controlled initial value problem in an explicit ensemble.
#[derive(Clone, Copy)]
pub struct Dopri5EnsembleJob<'a> {
    /// Problem and RHS callback. Different jobs may share an immutable problem.
    pub problem: &'a Problem,
    /// Forward integration interval for this member.
    pub t_span: (f64, f64),
    /// Initial state for this member.
    pub y0: &'a [f64],
    /// This member's tolerances and attempt limit.
    pub options: &'a Options,
}

/// A reusable local execution pool for independent DOPRI5 trajectories.
///
/// This is parallelism across initial value problems, not across dependent
/// Runge--Kutta stages. A solve never creates another pool. The one-worker
/// variant runs directly on the calling thread. RHS callbacks should be
/// deterministic functions of their arguments; synchronization in a callback
/// may otherwise affect both throughput and the meaning of the model.
pub struct Dopri5Ensemble {
    execution: ParallelExecution,
}

impl Dopri5Ensemble {
    /// Construct a pool once and reuse it for subsequent batches.
    pub fn new(workers: usize) -> Result<Self, ExplicitSolveError> {
        if workers == 0 {
            return Err(ExplicitSolveError::InvalidInput(
                "explicit ensemble worker count must be positive".into(),
            ));
        }
        let execution = ParallelExecution::rayon(workers)
            .map_err(|error| ExplicitSolveError::Driver(error.to_string()))?;
        Ok(Self { execution })
    }

    /// Number of workers requested when this pool was constructed.
    pub fn workers(&self) -> usize {
        self.execution.threads()
    }

    /// Integrate every member, preserving input order and every member's result.
    ///
    /// A malformed member is retained as its own `Err`; it does not cancel
    /// other jobs. A trajectory stopped after it started is the same partial
    /// `Ok(ExplicitSolution)` returned by [`Dopri5::solve`], with `success=false`.
    /// Check each result and each solution's success status before using it.
    /// The outer error is reserved for the execution layer, not a member's ODE.
    pub fn solve(
        &self,
        jobs: &[Dopri5EnsembleJob<'_>],
    ) -> Result<Vec<Result<ExplicitSolution, ExplicitSolveError>>, ExplicitSolveError> {
        // The outer closure always succeeds: ParallelExecution's fallible map
        // must not short-circuit after one invalid initial value problem.
        self.execution
            .map_ordered(jobs, |job| {
                Ok(Dopri5::solve(job.problem, job.t_span, job.y0, job.options))
            })
            .map_err(|error| ExplicitSolveError::Driver(error.to_string()))
    }
}
