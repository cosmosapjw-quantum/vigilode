//! Declared-structure derivative-free semilinear integration.

use crate::{
    AffineErrorNorm, CallbackError, ExplicitCallbackFailure, ExplicitCallbackWork,
    ExplicitErrorControl, ExplicitSolution, ExplicitSolveError, ExplicitStats, Options, Problem,
};
use rodas5p_core::CoreError;
use rodas5p_integrators::{
    OutputSchedule, integrate_affine_lawson_dopri5_observed,
    integrate_affine_lawson_dopri5_observed_with_norm, integrate_lawson_dopri5_observed,
    integrate_lawson_dopri5_observed_with_norm,
};
use serde::Serialize;
use std::cell::{Cell, RefCell};

/// Declared constant two-dimensional damping/rotation blocks and work counters.
pub use rodas5p_integrators::{DampedRotationBlock, DampedRotationFlow, QuotientWork};

/// Physical-coordinate integration and separate algebraic-flow work.
#[derive(Clone, Debug, Serialize)]
pub struct LawsonSolution {
    /// Endpoint observations, partial state and residual-callback telemetry.
    /// The `rhs_calls` here count evaluations of the declared residual N.
    pub integration: ExplicitSolution,
    /// Analytic block-flow coefficient and vector-action work.
    pub quotient_work: QuotientWork,
}

/// Derivative-free Lawson-DOPRI5 for a user-declared split `y'=Ay+N(t,y)`.
///
/// The supplied [`Problem`] must evaluate **N**, not the full right-hand side.
/// `flow` supplies A. No Jacobian, JVP, AD, finite differences or Newton solve
/// is used, even when the residual Problem happens to declare such providers.
/// Select this adapter when the constant damped-rotation block structure is
/// physically or algebraically known. No structure is inferred from samples.
#[derive(Clone, Copy, Debug, Default)]
pub struct LawsonDopri5;

impl LawsonDopri5 {
    /// Integrate the declared semilinear equation to its endpoint.
    pub fn solve(
        flow: &DampedRotationFlow,
        residual: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
    ) -> Result<LawsonSolution, ExplicitSolveError> {
        let times = if t_span.0 == t_span.1 {
            vec![t_span.0]
        } else {
            vec![t_span.0, t_span.1]
        };
        Self::solve_observed(flow, residual, t_span, y0, opts, &times)
    }

    /// Integrate with actual step landing on requested observation times.
    pub fn solve_observed(
        flow: &DampedRotationFlow,
        residual: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
    ) -> Result<LawsonSolution, ExplicitSolveError> {
        Self::solve_impl(flow, residual, t_span, y0, opts, times, None, None)
    }

    /// Use an affine physical-space embedded-error norm for the same split.
    pub fn solve_observed_with_norm(
        flow: &DampedRotationFlow,
        residual: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        norm: &AffineErrorNorm,
    ) -> Result<LawsonSolution, ExplicitSolveError> {
        Self::solve_impl(flow, residual, t_span, y0, opts, times, Some(norm), None)
    }

    /// Solve `y'=A*y+b+N(t,y)` with a declared constant b integrated exactly.
    /// `residual` must evaluate N only. No constancy is inferred from samples.
    #[allow(clippy::too_many_arguments)]
    pub fn solve_affine_observed(
        flow: &DampedRotationFlow,
        constant_forcing: &[f64],
        residual: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
    ) -> Result<LawsonSolution, ExplicitSolveError> {
        Self::solve_impl(
            flow,
            residual,
            t_span,
            y0,
            opts,
            times,
            None,
            Some(constant_forcing),
        )
    }

    /// The declared affine split with an existing physical-space error norm.
    #[allow(clippy::too_many_arguments)]
    pub fn solve_affine_observed_with_norm(
        flow: &DampedRotationFlow,
        constant_forcing: &[f64],
        residual: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        norm: &AffineErrorNorm,
    ) -> Result<LawsonSolution, ExplicitSolveError> {
        Self::solve_impl(
            flow,
            residual,
            t_span,
            y0,
            opts,
            times,
            Some(norm),
            Some(constant_forcing),
        )
    }

    fn solve_impl(
        flow: &DampedRotationFlow,
        residual: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
        times: &[f64],
        norm: Option<&AffineErrorNorm>,
        constant_forcing: Option<&[f64]>,
    ) -> Result<LawsonSolution, ExplicitSolveError> {
        let config = crate::explicit::validate(residual, t_span, y0, opts)?;
        if flow.dimension() != y0.len() {
            return Err(ExplicitSolveError::InvalidInput(
                "declared flow and residual dimensions differ".into(),
            ));
        }
        let schedule = OutputSchedule::new(times.to_vec())
            .map_err(|e| ExplicitSolveError::InvalidInput(e.to_string()))?;
        let work = Cell::new(ExplicitCallbackWork::default());
        let fatal = RefCell::new(None);
        let callback = |t: f64, y: &[f64], out: &mut [f64]| {
            let mut w = work.get();
            w.attempted = w.attempted.saturating_add(1);
            work.set(w);
            match (residual.rhs)(t, y, out) {
                Ok(()) => {
                    if let Some(i) = out.iter().position(|v| !v.is_finite()) {
                        let message =
                            format!("residual output component {i} is unwritten or nonfinite");
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
        let raw = match (norm, constant_forcing) {
            (None, None) => {
                integrate_lawson_dopri5_observed(flow, &callback, t_span, y0, &config, &schedule)
            }
            (Some(n), None) => integrate_lawson_dopri5_observed_with_norm(
                flow, &callback, t_span, y0, &config, &schedule, n,
            ),
            (None, Some(b)) => integrate_affine_lawson_dopri5_observed(
                flow, b, &callback, t_span, y0, &config, &schedule,
            ),
            (Some(n), Some(b)) => integrate_affine_lawson_dopri5_observed_with_norm(
                flow, b, &callback, t_span, y0, &config, &schedule, n,
            ),
        }
        .map_err(|e| match e {
            CoreError::InvalidInput(_) | CoreError::Dimension(_) => {
                ExplicitSolveError::InvalidInput(e.to_string())
            }
            _ => ExplicitSolveError::Driver(e.to_string()),
        })?;
        let mut r = raw.integration;
        if r.observed.t.last().copied() != Some(r.final_time) {
            r.observed.t.push(r.final_time);
            r.observed.y.push(r.final_state.clone());
        }
        let error_control = match norm {
            Some(n) => ExplicitErrorControl {
                mode: "affine-physical-wrms",
                dimension: n.physical_dimension(),
                atol: n.atol().to_vec(),
                rtol: n.rtol(),
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
        };
        Ok(LawsonSolution {
            integration: ExplicitSolution {
                t: r.observed.t,
                y: r.observed.y,
                success: r.observed.success,
                message: r.observed.message,
                final_time: r.final_time,
                final_state: r.final_state,
                stop_reason: r.stop_reason,
                callback_failure: fatal.into_inner(),
                stats: ExplicitStats {
                    schema: "vigilode.lawson-stats.v1",
                    method: if constant_forcing.is_some() {
                        "lawson-dopri5-declared-affine-rotation"
                    } else {
                        "lawson-dopri5-declared-damped-rotation"
                    },
                    options: opts.clone(),
                    adaptive: config.adaptive,
                    observation_mode: "step-landing",
                    error_control,
                    counters: r.observed.counters,
                    attempts: r.attempts,
                    accepted_steps: r.accepted_steps,
                    rejected_steps: r.rejected_steps,
                    rhs_completed: r.rhs_completed,
                    fsal_reuses: r.fsal_reuses,
                    callback_work: work.get(),
                    output_clipped_steps: r.observed.output_clipped_steps,
                },
            },
            quotient_work: raw.quotient_work,
        })
    }
}
