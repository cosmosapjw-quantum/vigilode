//! RHS-only Dormand–Prince 5(4) entry point for nonstiff ODEs.
//!
//! This facade shares the problem and tolerance inputs with RODAS5P, while
//! keeping a separate result schema that records the method actually run.
//! Neither a Jacobian provider nor a time-derivative provider is evaluated.

use std::cell::{Cell, RefCell};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, AffineErrorNorm, Dopri5Config, Dopri5StopReason, OutputSchedule,
    integrate_dopri5_dense_observed, integrate_dopri5_dense_observed_with_norm,
    integrate_dopri5_observed, integrate_dopri5_observed_with_norm,
};
use serde::Serialize;

use crate::{Atol, CallbackError, Options, Problem};

/// Schema of the RHS-only explicit integration record.
pub const EXPLICIT_STATS_SCHEMA: &str = "vigilode.explicit-stats.v1";

/// A request unsupported by the explicit endpoint-observation path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ExplicitUnsupported {
    /// A declared mass matrix requires a separate explicit mass-solve contract.
    #[error("DOPRI5 requires identity mass expressed by no mass matrix")]
    MassMatrix,
    /// Continuous interpolation is not implemented by this entry point.
    #[error("DOPRI5 dense output is not implemented; use explicit observation times")]
    DenseOutput,
    /// Event location requires a separately validated interpolation contract.
    #[error("DOPRI5 event location is not implemented")]
    Events,
}

/// Why an explicit solve was refused or its success check failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExplicitSolveError {
    /// Malformed input, rejected before entering user callbacks.
    #[error("invalid explicit-solver input: {0}")]
    InvalidInput(String),
    /// A requested feature is not implemented by this explicit path.
    #[error("unsupported explicit-solver request: {0}")]
    Unsupported(ExplicitUnsupported),
    /// The driver returned no partial integration record.
    #[error("explicit driver error: {0}")]
    Driver(String),
    /// An unsuccessful run checked with [`ExplicitSolution::ensure_success`].
    #[error("explicit integration failed: {message}")]
    Failed {
        /// Description of the stop.
        message: String,
        /// Accepted state, outputs and all work performed before the stop.
        partial: Box<ExplicitSolution>,
    },
}

/// Work observed at the user RHS boundary.
///
/// These fields overlap [`ExplicitStats::counters`]; they are diagnostic
/// decompositions and must not be added to that total again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ExplicitCallbackWork {
    /// Actual entries into the user's RHS, including errors.
    pub attempted: u64,
    /// Calls returning `Ok` with every output component finite.
    pub completed: u64,
    /// User-declared recoverable errors; the current attempt may retry.
    pub recoverable_errors: u64,
    /// User-declared fatal errors; integration stops immediately.
    pub fatal_errors: u64,
    /// `Ok` callbacks with an unwritten or nonfinite output component.
    pub malformed_outputs: u64,
}

/// A callback that stopped integration immediately.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ExplicitCallbackFailure {
    /// Stage time supplied to the callback.
    pub t: f64,
    /// Error text from the callback or finite-output guard.
    pub message: String,
    /// True for an `Ok` callback that left a nonfinite output component.
    pub malformed_output: bool,
}

/// Recorded inputs and actual work of an explicit solve.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ExplicitStats {
    /// Record schema version.
    pub schema: &'static str,
    /// Executed numerical method; this is independent of RODAS5P tiers.
    pub method: &'static str,
    /// Requested shared options. `tier` and `interior_error` do not route DOPRI5.
    pub options: Options,
    /// Actual adaptive controller configuration (reduced tolerances are retained
    /// here when an affine physical norm overrides local error acceptance).
    pub adaptive: AdaptiveStepConfig,
    /// `step-landing` or `quartic-dense-observations`.
    pub observation_mode: &'static str,
    /// Actual embedded-error geometry, distinct from requested shared options.
    pub error_control: ExplicitErrorControl,
    /// Total work, including rejected and failed attempts.
    pub counters: WorkCounters,
    /// Number of attempted time intervals.
    pub attempts: usize,
    /// Intervals accepted into the physical trajectory.
    pub accepted_steps: usize,
    /// Attempted intervals not accepted into the physical trajectory.
    pub rejected_steps: usize,
    /// Finite, successful RHS evaluations as counted by the kernel.
    pub rhs_completed: u64,
    /// Reuses of a derivative at an unchanged accepted base state.
    pub fsal_reuses: u64,
    /// User callback diagnostics, already represented in `counters`.
    pub callback_work: ExplicitCallbackWork,
    /// Accepted steps shortened to land on a requested observation.
    pub output_clipped_steps: usize,
}

/// Tolerances and dimension actually used for embedded-error acceptance.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ExplicitErrorControl {
    /// `state-wrms` or `affine-physical-wrms`.
    pub mode: &'static str,
    pub dimension: usize,
    pub atol: Vec<f64>,
    pub rtol: f64,
}

/// An explicit solution at requested observation times and its last accepted state.
///
/// `success` means that the final time was reached with accepted embedded
/// error estimates. Global error depends on the problem and stability regime.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ExplicitSolution {
    /// Recorded observation times, including the initial state.
    pub t: Vec<f64>,
    /// States corresponding to `t`.
    pub y: Vec<Vec<f64>>,
    /// Whether the requested final time was reached.
    pub success: bool,
    /// Human-readable completion or stop message.
    pub message: String,
    /// Time of the last accepted physical state, even if not a scheduled output.
    pub final_time: f64,
    /// Last accepted physical state, preserved after an unsuccessful attempt.
    pub final_state: Vec<f64>,
    /// Typed driver stop, present for an unsuccessful integration.
    pub stop_reason: Option<Dopri5StopReason>,
    /// Detailed fatal or malformed callback failure, when applicable.
    pub callback_failure: Option<ExplicitCallbackFailure>,
    /// Method, tolerances and work record.
    pub stats: ExplicitStats,
}

impl ExplicitSolution {
    /// Return this successful solution, or an error that retains the partial run.
    pub fn ensure_success(self) -> Result<Self, ExplicitSolveError> {
        if self.success {
            Ok(self)
        } else {
            Err(ExplicitSolveError::Failed {
                message: self.message.clone(),
                partial: Box::new(self),
            })
        }
    }
}

/// Dormand–Prince 5(4), explicitly selected for nonstiff identity-mass ODEs.
///
/// RHS-only evaluation makes this method Jacobian-, matrix- and Newton-free.
/// The RHS may depend on time. During one solve it must denote one fixed
/// function: changing the model behind interior mutability invalidates FSAL.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dopri5;

impl Dopri5 {
    /// Integrate from `t_span.0` to `t_span.1`, returning endpoint observations.
    ///
    /// Scalar and componentwise absolute tolerances are supported. A mass
    /// matrix, dense-output request or event is refused before user code runs.
    /// Jacobian declarations and the `autonomous` marker do not affect the
    /// explicit stage evaluations; the actual RHS always receives stage time.
    /// `Options::tier` is a RODAS5P routing option, retained only as requested
    /// input here; `method` records the explicit method actually executed.
    ///
    /// User `Recoverable` errors reject the trial and permit a smaller step.
    /// User `Fatal` errors, or `Ok` with partial/nonfinite output, stop at the
    /// last accepted state. Check [`ExplicitSolution::ensure_success`].
    pub fn solve(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
    ) -> Result<ExplicitSolution, ExplicitSolveError> {
        let times = if t_span.0 == t_span.1 {
            vec![t_span.0]
        } else {
            vec![t_span.0, t_span.1]
        };
        Self::solve_observed(problem, t_span, y0, opts, &times)
    }

    /// Integrate with actual step landing at every requested observation time.
    ///
    /// `times` must be a valid increasing schedule within the time span. The
    /// shared output collector retains the initial and terminal physical state.
    /// No dense interpolation is evaluated between observations.
    pub fn solve_observed(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
    ) -> Result<ExplicitSolution, ExplicitSolveError> {
        Self::solve_impl(problem, t_span, y0, opts, times, false, None)
    }

    /// Sample a quartic continuous extension of accepted DOPRI5 steps.
    ///
    /// Interior observations do not shorten integration steps. The polynomial
    /// has a local O(h^5) interpolation error under smoothness assumptions;
    /// the embedded endpoint estimator does not independently bound that error.
    /// This API returns requested samples, not a retained interpolant for later
    /// queries. Keep `Options::dense_output` false; that flag requests the
    /// retained-output contract and remains unsupported here.
    pub fn solve_dense_observed(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
    ) -> Result<ExplicitSolution, ExplicitSolveError> {
        Self::solve_impl(problem, t_span, y0, opts, times, true, None)
    }

    /// Control the embedded error in the declared affine physical state.
    ///
    /// The norm's physical-space atol/rtol override the shared state's local
    /// error tolerances; both requested and executed settings are recorded.
    /// Returned states remain in the original reduced coordinates.
    pub fn solve_observed_with_norm(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        norm: &AffineErrorNorm,
    ) -> Result<ExplicitSolution, ExplicitSolveError> {
        Self::solve_impl(problem, t_span, y0, opts, times, false, Some(norm))
    }

    /// Combine dense observations with affine physical embedded-error control.
    /// The physical norm controls endpoints; interpolated observations have no
    /// separate local-error acceptance test. No Jacobian or linear solve occurs.
    pub fn solve_dense_observed_with_norm(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        norm: &AffineErrorNorm,
    ) -> Result<ExplicitSolution, ExplicitSolveError> {
        Self::solve_impl(problem, t_span, y0, opts, times, true, Some(norm))
    }

    fn solve_impl(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        dense: bool,
        norm: Option<&AffineErrorNorm>,
    ) -> Result<ExplicitSolution, ExplicitSolveError> {
        let config = validate(problem, t_span, y0, opts)?;
        let schedule = OutputSchedule::new(times.to_vec())
            .map_err(|e| ExplicitSolveError::InvalidInput(e.to_string()))?;
        let work = Cell::new(ExplicitCallbackWork::default());
        let fatal = RefCell::new(None);
        let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
            let mut w = work.get();
            w.attempted = w.attempted.saturating_add(1);
            work.set(w);
            // The kernel poisons every reused output immediately before this
            // callback. Do not repeat the same n stores at this boundary.
            // Keep this finite scan for detailed public failure telemetry.
            match (problem.rhs)(t, y, out) {
                Ok(()) => {
                    if let Some(i) = out.iter().position(|v| !v.is_finite()) {
                        let message = format!("RHS output component {i} is unwritten or nonfinite");
                        let mut w = work.get();
                        w.malformed_outputs = w.malformed_outputs.saturating_add(1);
                        work.set(w);
                        *fatal.borrow_mut() = Some(ExplicitCallbackFailure {
                            t,
                            message: message.clone(),
                            malformed_output: true,
                        });
                        Err(CoreError::InvalidInput(message))
                    } else {
                        let mut w = work.get();
                        w.completed = w.completed.saturating_add(1);
                        work.set(w);
                        Ok(())
                    }
                }
                Err(CallbackError::Recoverable(message)) => {
                    let mut w = work.get();
                    w.recoverable_errors = w.recoverable_errors.saturating_add(1);
                    work.set(w);
                    Err(CoreError::NonFinite(message))
                }
                Err(CallbackError::Fatal(message)) => {
                    let mut w = work.get();
                    w.fatal_errors = w.fatal_errors.saturating_add(1);
                    work.set(w);
                    *fatal.borrow_mut() = Some(ExplicitCallbackFailure {
                        t,
                        message: message.clone(),
                        malformed_output: false,
                    });
                    Err(CoreError::InvalidInput(message))
                }
            }
        };
        let raw_result = match (dense, norm) {
            (false, None) => integrate_dopri5_observed(&rhs, t_span, y0, &config, &schedule),
            (true, None) => integrate_dopri5_dense_observed(&rhs, t_span, y0, &config, &schedule),
            (false, Some(norm)) => {
                integrate_dopri5_observed_with_norm(&rhs, t_span, y0, &config, &schedule, norm)
            }
            (true, Some(norm)) => integrate_dopri5_dense_observed_with_norm(
                &rhs, t_span, y0, &config, &schedule, norm,
            ),
        };
        let mut result = raw_result.map_err(|e| match e {
            CoreError::InvalidInput(_) | CoreError::Dimension(_) => {
                ExplicitSolveError::InvalidInput(e.to_string())
            }
            _ => ExplicitSolveError::Driver(e.to_string()),
        })?;
        // A failed attempt may follow accepted steps between observations.
        // Keep that committed endpoint in the public partial solution too.
        if result.observed.t.last().copied() != Some(result.final_time) {
            result.observed.t.push(result.final_time);
            result.observed.y.push(result.final_state.clone());
        }
        Ok(ExplicitSolution {
            t: result.observed.t,
            y: result.observed.y,
            success: result.observed.success,
            message: result.observed.message,
            final_time: result.final_time,
            final_state: result.final_state,
            stop_reason: result.stop_reason,
            callback_failure: fatal.into_inner(),
            stats: ExplicitStats {
                schema: EXPLICIT_STATS_SCHEMA,
                method: "dopri5-5(4)-fsal",
                options: opts.clone(),
                error_control: match norm {
                    Some(norm) => ExplicitErrorControl {
                        mode: "affine-physical-wrms",
                        dimension: norm.physical_dimension(),
                        atol: norm.atol().to_vec(),
                        rtol: norm.rtol(),
                    },
                    None => ExplicitErrorControl {
                        mode: "state-wrms",
                        dimension: y0.len(),
                        atol: config
                            .atol
                            .clone()
                            .unwrap_or_else(|| vec![config.adaptive.atol]),
                        rtol: config.adaptive.rtol,
                    },
                },
                adaptive: config.adaptive,
                observation_mode: if dense {
                    "quartic-dense-observations"
                } else {
                    "step-landing"
                },
                counters: result.observed.counters,
                attempts: result.attempts,
                accepted_steps: result.accepted_steps,
                rejected_steps: result.rejected_steps,
                rhs_completed: result.rhs_completed,
                fsal_reuses: result.fsal_reuses,
                callback_work: work.get(),
                output_clipped_steps: result.observed.output_clipped_steps,
            },
        })
    }
}

fn validate(
    problem: &Problem,
    (t0, t1): (f64, f64),
    y0: &[f64],
    opts: &Options,
) -> Result<Dopri5Config, ExplicitSolveError> {
    let invalid = |s: &str| ExplicitSolveError::InvalidInput(s.into());
    if problem.dimension == 0 || y0.len() != problem.dimension {
        return Err(invalid(
            "positive problem dimension and matching y0 are required",
        ));
    }
    if !y0.iter().all(|v| v.is_finite()) {
        return Err(invalid("y0 must be finite"));
    }
    if !(t0.is_finite() && t1.is_finite() && t1 >= t0) {
        return Err(invalid("finite forward span t1 >= t0 is required"));
    }
    if problem.mass.is_some() {
        return Err(ExplicitSolveError::Unsupported(
            ExplicitUnsupported::MassMatrix,
        ));
    }
    if opts.dense_output {
        return Err(ExplicitSolveError::Unsupported(
            ExplicitUnsupported::DenseOutput,
        ));
    }
    if !problem.events.is_empty() {
        return Err(ExplicitSolveError::Unsupported(ExplicitUnsupported::Events));
    }
    let (scalar_atol, atol) = match &opts.atol {
        Atol::Scalar(a) => (*a, None),
        Atol::Vector(v) => {
            if v.len() != problem.dimension || !v.iter().all(|a| a.is_finite() && *a >= 0.0) {
                return Err(invalid(
                    "atol vector must match dimension and be finite/nonnegative",
                ));
            }
            (v.iter().copied().fold(0.0, f64::max), Some(v.clone()))
        }
    };
    let adaptive = AdaptiveStepConfig {
        atol: scalar_atol,
        rtol: opts.rtol,
        initial_step: opts.h0,
        max_attempts: opts.max_steps,
        ..AdaptiveStepConfig::default()
    };
    adaptive
        .validate()
        .map_err(|e| ExplicitSolveError::InvalidInput(e.to_string()))?;
    Ok(Dopri5Config {
        adaptive,
        atol,
        fsal: true,
        recover_nonfinite_rhs: true,
    })
}
