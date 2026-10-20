//! [`Rodas5p::solve`]: validation, routing to a protected driver, and the
//! run record.

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, ObservedIntegrationResult, OdeProblem, OutputSchedule,
    RoutedDriver, RoutedRun, RoutingError, RoutingOptions, StructureDeclaration,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_routed_observed,
};

use crate::{
    error::SolveError,
    fd::{Guard, band_groups, banded_fd_fill, dense_fd_jacobian, dense_from_band},
    options::{Atol, Options, Tier},
    problem::{Jacobian, Problem},
    solution::{
        Callback, FD_INCREMENT_RULE, FailureReason, JacobianRecord, RouteRecord, STATS_SCHEMA,
        Solution, Stats, Status,
    },
};

type CoreRhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type CoreJacobian = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type CoreJvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

const ENTRY_ROUTED: &str = "integrate_rodas5p_routed_observed";
const ENTRY_BANDED: &str = "integrate_rodas5p_fast_banded_observed";

/// The RODAS5P solver: a stiffly accurate, L-stable Rosenbrock method of
/// order 5(4), run through the protected drivers of `rodas5p-integrators`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rodas5p;

/// How the facade will call the drivers, decided before any step.
enum Plan {
    /// Through the router, with an optional band declaration.
    Routed {
        declaration: Option<StructureDeclaration>,
    },
    /// The banded driver directly.
    Banded(BandedJacobian),
}

/// The driver outcome, normalized.
struct Run {
    observed: ObservedIntegrationResult,
    attempts: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    route: RouteRecord,
    matrix_free: bool,
}

impl Rodas5p {
    /// Integrate `problem` from `t_span.0` to `t_span.1` starting at `y0`.
    ///
    /// Returns `Err` only when nothing could be integrated (malformed or
    /// unsupported input, a refused structure declaration, or a driver error
    /// that left no partial result). A run that starts and stops early is
    /// `Ok` with [`Status::Failed`]; see [`Solution::ensure_success`].
    pub fn solve(
        problem: &Problem,
        t_span: (f64, f64),
        y0: &[f64],
        opts: &Options,
    ) -> Result<Solution, SolveError> {
        let atol = validate(problem, t_span, y0, opts)?;
        let adaptive = AdaptiveStepConfig {
            rtol: opts.rtol,
            atol,
            initial_step: opts.h0,
            max_attempts: opts.max_steps,
            ..AdaptiveStepConfig::default()
        };
        adaptive
            .validate()
            .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
        let times = if t_span.1 > t_span.0 {
            vec![t_span.0, t_span.1]
        } else {
            vec![t_span.0]
        };
        let schedule =
            OutputSchedule::new(times).map_err(|e| SolveError::InvalidInput(e.to_string()))?;

        let guard = Arc::new(Guard::default());
        let (ode, plan, jacobian_record) = build(problem, &guard)?;

        let run = match plan {
            Plan::Routed { declaration } => {
                let ode = match declaration {
                    Some(declaration) => ode
                        .with_declared_structure(declaration)
                        .map_err(|e| SolveError::InvalidInput(e.to_string()))?,
                    None => ode,
                };
                let routed =
                    integrate_rodas5p_routed_observed(&ode, t_span, y0, &adaptive, &schedule)
                        .map_err(|e| match e {
                            RoutingError::Structure(s) => {
                                SolveError::StructureRefused(s.to_string())
                            }
                            RoutingError::Integration(c) => driver_error(c, &guard),
                        })?;
                let (driver_id, attempts, accepted, rejected) = match &routed.run {
                    RoutedRun::Banded(r) => (
                        r.fast.driver,
                        r.fast.attempts,
                        r.fast.accepted_steps,
                        r.fast.rejected_steps,
                    ),
                    RoutedRun::Dense(r) => {
                        (r.driver, r.attempts, r.accepted_steps, r.rejected_steps)
                    }
                    RoutedRun::MatrixFree(r) => {
                        (r.driver, r.attempts, r.accepted_steps, r.rejected_steps)
                    }
                };
                Run {
                    observed: routed.run.observed().clone(),
                    attempts,
                    accepted_steps: accepted,
                    rejected_steps: rejected,
                    matrix_free: routed.routing.driver == RoutedDriver::MatrixFree,
                    route: RouteRecord {
                        entry_point: ENTRY_ROUTED,
                        driver_id,
                        routing: Some(routed.routing),
                    },
                }
            }
            Plan::Banded(band) => {
                let result = integrate_rodas5p_fast_banded_observed(
                    &ode, &band, t_span, y0, &adaptive, &schedule,
                )
                .map_err(|e| driver_error(e, &guard))?;
                Run {
                    route: RouteRecord {
                        entry_point: ENTRY_BANDED,
                        driver_id: result.fast.driver,
                        routing: None,
                    },
                    observed: result.fast.observed,
                    attempts: result.fast.attempts,
                    accepted_steps: result.fast.accepted_steps,
                    rejected_steps: result.fast.rejected_steps,
                    matrix_free: false,
                }
            }
        };

        let fd = guard.fd_work();
        let driver_counters = run.observed.counters;
        let mut counters: WorkCounters = driver_counters;
        counters.rhs_calls += fd.rhs_calls;
        counters.rhs_evaluations += fd.rhs_calls;

        let status = match guard.fatal() {
            Some(fatal) => Status::Failed(FailureReason::CallbackFailed {
                callback: fatal.callback,
                t: fatal.t,
                message: fatal.message,
            }),
            None if run.observed.success => Status::Success,
            None if run.attempts >= adaptive.max_attempts => {
                Status::Failed(FailureReason::MaxStepsReached {
                    max_steps: adaptive.max_attempts,
                })
            }
            None => Status::Failed(FailureReason::StepSizeTooSmall {
                min_step: adaptive.min_step,
            }),
        };

        let stats = Stats {
            schema: STATS_SCHEMA,
            options: opts.clone(),
            matrix_free_linear: run
                .matrix_free
                .then(|| RoutingOptions::default().matrix_free),
            adaptive,
            t_span,
            y0: y0.to_vec(),
            dimension: problem.dimension,
            autonomous: problem.autonomous,
            time_derivative: problem.time_derivative.is_some(),
            jacobian: jacobian_record,
            tier_requested: opts.tier,
            // No candidate driver is attached in this release.
            tier_executed: Tier::Protected,
            route: run.route,
            attempts: run.attempts,
            accepted_steps: run.accepted_steps,
            rejected_steps: run.rejected_steps,
            internal_steps: run.observed.internal_steps,
            output_clipped_steps: run.observed.output_clipped_steps,
            counters,
            driver_counters,
            finite_difference: fd,
            callbacks: guard.callback_work(),
            driver_message: run.observed.message,
        };
        Ok(Solution {
            t: run.observed.t,
            y: run.observed.y,
            stats,
            status,
        })
    }
}

fn driver_error(error: CoreError, guard: &Guard) -> SolveError {
    match guard.fatal() {
        Some(fatal) => SolveError::Driver(format!(
            "{error}; first fatal {:?} callback error at t = {:e}: {}",
            fatal.callback, fatal.t, fatal.message
        )),
        None => SolveError::Driver(error.to_string()),
    }
}

/// Every refusal that does not need the drivers; returns the scalar `atol`.
fn validate(
    problem: &Problem,
    (t0, t1): (f64, f64),
    y0: &[f64],
    opts: &Options,
) -> Result<f64, SolveError> {
    let invalid = |m: String| Err(SolveError::InvalidInput(m));
    let unsupported = |m: &str| Err(SolveError::Unsupported(m.into()));
    let n = problem.dimension;
    if n == 0 {
        return invalid("the problem dimension must be positive".into());
    }
    if y0.len() != n {
        return invalid(format!(
            "y0 has length {}, the problem dimension is {n}",
            y0.len()
        ));
    }
    if !y0.iter().all(|v| v.is_finite()) {
        return invalid("y0 contains NaN/Inf".into());
    }
    if !(t0.is_finite() && t1.is_finite()) || t1 < t0 {
        return invalid(format!(
            "the span ({t0:e}, {t1:e}) must be finite with t1 >= t0"
        ));
    }
    if !(opts.rtol.is_finite() && opts.rtol >= 0.0) {
        return invalid("rtol must be finite and nonnegative".into());
    }
    if !(opts.h0.is_finite() && opts.h0 > 0.0) {
        return invalid("h0 must be finite and positive".into());
    }
    if opts.max_steps == 0 {
        return invalid("max_steps must be positive".into());
    }
    let atol = match &opts.atol {
        Atol::Scalar(a) => *a,
        Atol::Vector(v) => {
            if v.len() != n {
                return invalid(format!(
                    "atol has length {}, the problem dimension is {n}",
                    v.len()
                ));
            }
            return unsupported(
                "a vector atol: no protected driver of this release takes a per-component \
                 absolute tolerance, and the facade does not substitute a scalar",
            );
        }
    };
    if let Some(m) = &problem.mass {
        if m.len() != n * n {
            return invalid(format!(
                "the mass matrix has {} entries, expected {}",
                m.len(),
                n * n
            ));
        }
        return unsupported(
            "a mass matrix: the protected fast drivers of this release integrate M = I only",
        );
    }
    if opts.dense_output {
        return unsupported(
            "dense_output: no protected driver of this release returns per-step states \
             without changing its step sequence",
        );
    }
    if problem.autonomous && problem.time_derivative.is_some() {
        return invalid("an autonomous problem cannot have a time derivative".into());
    }
    let band_ok = |l: usize, u: usize| l < n && u < n;
    if let Some((l, u)) = problem.band
        && !band_ok(l, u)
    {
        return invalid(format!(
            "the declared band ({l}, {u}) lies outside the {n} x {n} matrix"
        ));
    }
    match &problem.jacobian {
        Jacobian::Banded { lower, upper, .. } => {
            if !band_ok(*lower, *upper) {
                return invalid(format!(
                    "the Jacobian band ({lower}, {upper}) lies outside the {n} x {n} matrix"
                ));
            }
            if let Some(declared) = problem.band
                && declared != (*lower, *upper)
            {
                return invalid(format!(
                    "the declared band {declared:?} differs from the Jacobian band ({lower}, {upper})"
                ));
            }
        }
        Jacobian::Jvp(_) if problem.band.is_some() => {
            return unsupported("a band declaration on a JVP-only problem");
        }
        _ => {}
    }
    Ok(atol)
}

/// The driver-level problem, the plan and the Jacobian record.
fn build(
    problem: &Problem,
    guard: &Arc<Guard>,
) -> Result<(OdeProblem, Plan, JacobianRecord), SolveError> {
    let n = problem.dimension;
    let rhs: CoreRhs = {
        let (user, guard) = (problem.rhs.clone(), guard.clone());
        Arc::new(move |t, y, out| guard.call(Callback::Rhs, t, || user(t, y, out)))
    };
    let partial_t: Option<CoreRhs> = problem.time_derivative.clone().map(|user| {
        let guard = guard.clone();
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            guard.call(Callback::TimeDerivative, t, || user(t, y, out))
        }) as CoreRhs
    });

    let mut jacobian: Option<CoreJacobian> = None;
    let mut jvp: Option<CoreJvp> = None;
    let (plan, record) = match (&problem.jacobian, problem.band) {
        (Jacobian::Dense(user), band) => {
            let dense: CoreJacobian = {
                let (user, guard) = (user.clone(), guard.clone());
                Arc::new(move |t, y| {
                    let mut m = DenseMatrix::zeros(n, n);
                    guard.call(Callback::Jacobian, t, || user(t, y, m.as_mut_slice()))?;
                    finite(m.as_slice(), "dense Jacobian")?;
                    Ok(m)
                })
            };
            jacobian = Some(dense.clone());
            match band {
                None => (Plan::Routed { declaration: None }, JacobianRecord::Dense),
                Some((lower, upper)) => {
                    let width = lower + upper + 1;
                    let fill = Arc::new(
                        move |t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
                            let m = dense(t, y)?;
                            for i in 0..n {
                                for j in i.saturating_sub(lower)..=(i + upper).min(n - 1) {
                                    out[i * width + (j + lower - i)] = m[(i, j)];
                                }
                            }
                            Ok(())
                        },
                    );
                    (
                        Plan::Routed {
                            declaration: Some(StructureDeclaration::banded(n, lower, upper, fill)),
                        },
                        JacobianRecord::DenseDeclaredBand { lower, upper },
                    )
                }
            }
        }
        (Jacobian::Jvp(user), _) => {
            let (user, guard) = (user.clone(), guard.clone());
            jvp = Some(Arc::new(move |t, y, v, out| {
                guard.call(Callback::Jvp, t, || user(t, y, v, out))
            }));
            (Plan::Routed { declaration: None }, JacobianRecord::Jvp)
        }
        (Jacobian::FiniteDifference, None) => {
            jacobian = Some(Arc::new(dense_fd_jacobian(
                guard.clone(),
                problem.rhs.clone(),
                n,
            )));
            (
                Plan::Routed { declaration: None },
                JacobianRecord::FiniteDifferenceDense {
                    increment: FD_INCREMENT_RULE,
                },
            )
        }
        (Jacobian::FiniteDifference, Some((lower, upper))) => {
            let fill = Arc::new(banded_fd_fill(
                guard.clone(),
                problem.rhs.clone(),
                n,
                lower,
                upper,
            ));
            jacobian = Some(band_as_dense(fill.clone(), n, lower, upper));
            (
                Plan::Banded(BandedJacobian { lower, upper, fill }),
                JacobianRecord::FiniteDifferenceBanded {
                    lower,
                    upper,
                    groups: band_groups(n, lower, upper),
                    increment: FD_INCREMENT_RULE,
                },
            )
        }
        (Jacobian::Banded { lower, upper, fill }, _) => {
            let (lower, upper) = (*lower, *upper);
            let fill: CoreRhs = {
                let (user, guard) = (fill.clone(), guard.clone());
                Arc::new(move |t, y, out| {
                    guard.call(Callback::Jacobian, t, || user(t, y, out))?;
                    finite(out, "band Jacobian")
                })
            };
            jacobian = Some(band_as_dense(fill.clone(), n, lower, upper));
            (
                Plan::Banded(BandedJacobian { lower, upper, fill }),
                JacobianRecord::Banded { lower, upper },
            )
        }
    };

    let ode = OdeProblem::new(
        "vigilode-facade",
        n,
        rhs,
        None,
        jacobian,
        jvp,
        partial_t,
        problem.autonomous,
        None,
        None,
    )
    .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
    Ok((ode, plan, record))
}

/// A dense Jacobian assembled from a band fill. `OdeProblem` requires a
/// Jacobian provider; the banded driver itself reads the band.
fn band_as_dense(fill: CoreRhs, n: usize, lower: usize, upper: usize) -> CoreJacobian {
    Arc::new(move |t, y| {
        let mut values = vec![0.0; n * (lower + upper + 1)];
        fill(t, y, &mut values)?;
        Ok(dense_from_band(&values, n, lower, upper))
    })
}

fn finite(values: &[f64], what: &str) -> CoreResult<()> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite(format!("{what} produced NaN/Inf")))
    }
}
