//! [`Rodas5p::solve`]: validation, routing to a protected driver, and the
//! run record.

use std::sync::Arc;

use std::fmt::Display;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveDenseSegmentRun, AdaptiveFailureKind, AdaptiveStepConfig, BandedJacobian,
    DenseErrorControl, DriverPartial, DriverStopReason, FastInteriorControl, IntegrationMethod,
    InteriorSampling, JacobianIntoFn, ObservedIntegrationResult, OdeProblem, OutputSamplingPlan,
    OutputSchedule, RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID, RoutedDriver, RoutedRun, RoutingError,
    RoutingOptions, StructureDeclaration, integrate_adaptive_dense_observed_with_partial,
    integrate_adaptive_dense_observed_with_segments, integrate_rodas5p_fast_banded_dense_observed,
    integrate_rodas5p_fast_banded_dense_observed_with_atol, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_banded_observed_with_atol, integrate_rodas5p_fast_dense_observed,
    integrate_rodas5p_fast_dense_observed_with_atol, integrate_rodas5p_fast_observed_with_atol,
    integrate_rodas5p_routed_observed,
};

use crate::{
    error::{SolveError, Unsupported},
    fd::{Guard, band_groups, banded_fd_fill, dense_fd_jacobian, dense_from_band},
    options::{Atol, InteriorError, Options, Tier},
    problem::{Jacobian, Problem},
    solution::{
        Callback, DriverWork, Extension, FD_INCREMENT_RULE, FailureReason, JacobianRecord,
        KrylovStartRecord, RouteRecord, STATS_SCHEMA, Solution, Stats, Status, StopRecord,
    },
};

type CoreRhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type CoreJacobian = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type CoreJvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

const ENTRY_ROUTED: &str = "integrate_rodas5p_routed_observed";
const ENTRY_BANDED: &str = "integrate_rodas5p_fast_banded_observed";
/// The dense fast driver called directly: the router has no per-component
/// absolute tolerance, so a vector `atol` on an undeclared dense Jacobian
/// bypasses it (rule 3 is what the router would choose).
const ENTRY_DENSE_FAST_ATOL: &str = "integrate_rodas5p_fast_observed_with_atol";
/// The banded fast driver with a per-component absolute tolerance.
const ENTRY_BANDED_ATOL: &str = "integrate_rodas5p_fast_banded_observed_with_atol";
/// Dense output without a mass matrix (`WFilteredEnforce` or `Off`).
const ENTRY_FAST_DENSE: &str = "integrate_rodas5p_fast_dense_observed";
const ENTRY_FAST_DENSE_ATOL: &str = "integrate_rodas5p_fast_dense_observed_with_atol";
const ENTRY_FAST_BANDED_DENSE: &str = "integrate_rodas5p_fast_banded_dense_observed";
const ENTRY_FAST_BANDED_DENSE_ATOL: &str = "integrate_rodas5p_fast_banded_dense_observed_with_atol";
/// Dense output under a mass matrix or `InteriorError::Enforce`.
const ENTRY_SEQUENTIAL_DENSE: &str = "integrate_adaptive_dense_observed_with_segments";
/// The same driver without the segment sink (mass matrix, no dense output).
const ENTRY_SEQUENTIAL_NO_SEGMENTS: &str = "integrate_adaptive_dense_observed_with_partial";

/// The RODAS5P solver: a stiffly accurate, L-stable Rosenbrock method of
/// order 5(4), run through the protected drivers of `rodas5p-integrators`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rodas5p;

/// The validated absolute tolerance: one value, or one per component.
enum AtolSpec {
    Scalar(f64),
    Vector(Vec<f64>),
}

impl AtolSpec {
    /// The scalar `AdaptiveStepConfig::atol`. With a vector it is only
    /// validated (the drivers read the vector instead): the largest entry,
    /// so a uniform vector gives the record of the scalar run.
    fn adaptive_atol(&self) -> f64 {
        match self {
            Self::Scalar(a) => *a,
            Self::Vector(v) => v.iter().copied().fold(0.0, f64::max),
        }
    }
}

/// How the facade will call the drivers, decided before any step.
enum Plan {
    /// Through the router, with an optional band declaration.
    Routed {
        declaration: Option<StructureDeclaration>,
    },
    /// The banded driver directly.
    Banded(BandedJacobian),
    /// The dense fast driver directly (a vector `atol` on an undeclared
    /// dense Jacobian).
    DenseFast,
    /// The fast dense driver with a continuous extension, dense storage
    /// (`None`) or banded: dense output without a mass matrix, under
    /// `WFilteredEnforce` or `Off`.
    FastDense { band: Option<BandedJacobian> },
    /// The sequential adaptive dense driver: a mass matrix, or dense output
    /// under `InteriorError::Enforce`.
    SequentialDense,
}

/// The driver outcome, normalized.
struct Run {
    observed: ObservedIntegrationResult,
    attempts: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    route: RouteRecord,
    matrix_free: bool,
    driver_work: DriverWork,
    /// The driver's partial record (`None` on success).
    partial: Option<DriverPartial>,
    /// Every accepted step's continuous extension (dense output).
    segments: Option<Extension>,
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
            atol: atol.adaptive_atol(),
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
        let sequential = sequential_route(problem, opts);
        let (ode, plan, jacobian_record) = build(problem, &guard, sequential)?;
        // Dense output without a mass matrix (and not `Enforce`) runs the
        // fast drivers with a continuous extension, the banded twin for a
        // band Jacobian or a finite-difference band. `validate` refused a
        // declared band on a dense Jacobian here.
        let plan = match plan {
            Plan::Routed { declaration: None } if opts.dense_output && !sequential => {
                Plan::FastDense { band: None }
            }
            Plan::Banded(band) if opts.dense_output && !sequential => {
                Plan::FastDense { band: Some(band) }
            }
            plan => plan,
        };
        // The router takes a scalar `atol` only: a vector on the undeclared
        // dense route calls the dense fast driver itself. `validate` refused
        // every other vector plan that would pass through the router.
        let plan = match (plan, &atol) {
            (Plan::Routed { declaration: None }, AtolSpec::Vector(_)) => Plan::DenseFast,
            (plan, _) => plan,
        };

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
                let partial = routed.partial.clone();
                let (driver_id, attempts, accepted, rejected, driver_work) = match &routed.run {
                    RoutedRun::Banded(r) => (
                        r.fast.driver,
                        r.fast.attempts,
                        r.fast.accepted_steps,
                        r.fast.rejected_steps,
                        DriverWork::Banded {
                            jacobian_reuses: r.fast.jacobian_reuses,
                            lu: (&r.fast.lu).into(),
                            banded: (&r.work).into(),
                        },
                    ),
                    RoutedRun::Dense(r) => (
                        r.driver,
                        r.attempts,
                        r.accepted_steps,
                        r.rejected_steps,
                        DriverWork::Dense {
                            jacobian_reuses: r.jacobian_reuses,
                            lu: (&r.lu).into(),
                        },
                    ),
                    RoutedRun::MatrixFree(r) => (
                        r.driver,
                        r.attempts,
                        r.accepted_steps,
                        r.rejected_steps,
                        DriverWork::MatrixFree {
                            state_reuses: r.state_reuses,
                            gcrodr_policy: r.gcrodr_policy,
                            krylov_workspace: r.krylov_workspace,
                            start_policy: r.start_policy,
                            recycle_scope: r.recycle_scope,
                            start_telemetry: r.start_telemetry.as_ref().map(|t| {
                                KrylovStartRecord {
                                    zero_starts: t.zero_starts,
                                    previous_stage_starts: t.previous_stage_starts,
                                    previous_step_starts: t.previous_step_starts,
                                    solver_carried_starts: t.solver_carried_starts,
                                    cross_system_basis_starts: t.cross_system_basis_starts,
                                }
                            }),
                        },
                    ),
                };
                Run {
                    observed: routed.run.observed().clone(),
                    attempts,
                    accepted_steps: accepted,
                    rejected_steps: rejected,
                    matrix_free: routed.routing.driver == RoutedDriver::MatrixFree,
                    driver_work,
                    route: RouteRecord {
                        entry_point: ENTRY_ROUTED,
                        driver_id,
                        routing: Some(routed.routing.into()),
                    },
                    partial,
                    segments: None,
                }
            }
            Plan::DenseFast => {
                let AtolSpec::Vector(atol) = &atol else {
                    unreachable!("the direct dense fast plan carries a vector atol")
                };
                let (r, partial) = integrate_rodas5p_fast_observed_with_atol(
                    &ode, t_span, y0, &adaptive, &schedule, atol,
                )
                .map_err(|e| driver_error(e, &guard))?;
                Run {
                    route: RouteRecord {
                        entry_point: ENTRY_DENSE_FAST_ATOL,
                        driver_id: r.driver,
                        routing: None,
                    },
                    observed: r.observed,
                    attempts: r.attempts,
                    accepted_steps: r.accepted_steps,
                    rejected_steps: r.rejected_steps,
                    matrix_free: false,
                    driver_work: DriverWork::Dense {
                        jacobian_reuses: r.jacobian_reuses,
                        lu: (&r.lu).into(),
                    },
                    partial,
                    segments: None,
                }
            }
            Plan::FastDense { band } => {
                let control = match opts.interior_error {
                    InteriorError::Off => FastInteriorControl::Off,
                    InteriorError::WFilteredEnforce => FastInteriorControl::WFilteredEnforce,
                    InteriorError::Enforce => {
                        unreachable!("Enforce routes to the sequential driver")
                    }
                };
                let (entry_point, result) = match (&band, &atol) {
                    (None, AtolSpec::Scalar(_)) => (
                        ENTRY_FAST_DENSE,
                        integrate_rodas5p_fast_dense_observed(&ode, t_span, y0, &adaptive, control),
                    ),
                    (None, AtolSpec::Vector(atol)) => (
                        ENTRY_FAST_DENSE_ATOL,
                        integrate_rodas5p_fast_dense_observed_with_atol(
                            &ode, t_span, y0, &adaptive, control, atol,
                        ),
                    ),
                    (Some(band), AtolSpec::Scalar(_)) => (
                        ENTRY_FAST_BANDED_DENSE,
                        integrate_rodas5p_fast_banded_dense_observed(
                            &ode, band, t_span, y0, &adaptive, control,
                        ),
                    ),
                    (Some(band), AtolSpec::Vector(atol)) => (
                        ENTRY_FAST_BANDED_DENSE_ATOL,
                        integrate_rodas5p_fast_banded_dense_observed_with_atol(
                            &ode, band, t_span, y0, &adaptive, control, atol,
                        ),
                    ),
                };
                let result = result.map_err(|e| driver_error(e, &guard))?;
                let fast = result.fast;
                Run {
                    route: RouteRecord {
                        entry_point,
                        driver_id: fast.driver,
                        routing: None,
                    },
                    attempts: fast.attempts,
                    accepted_steps: fast.accepted_steps,
                    rejected_steps: fast.rejected_steps,
                    matrix_free: false,
                    driver_work: DriverWork::FastDense {
                        banded: band.is_some(),
                        jacobian_reuses: fast.jacobian_reuses,
                        lu: (&fast.lu).into(),
                        band_work: result.banded.as_ref().map(Into::into),
                        interior: (&result.interior).into(),
                        segments: result.segments.len(),
                    },
                    observed: fast.observed,
                    partial: result.partial,
                    segments: Some(Extension::Fast(result.segments)),
                }
            }
            Plan::Banded(band) => {
                let (entry_point, result) = match &atol {
                    AtolSpec::Scalar(_) => (
                        ENTRY_BANDED,
                        integrate_rodas5p_fast_banded_observed(
                            &ode, &band, t_span, y0, &adaptive, &schedule,
                        ),
                    ),
                    AtolSpec::Vector(atol) => (
                        ENTRY_BANDED_ATOL,
                        integrate_rodas5p_fast_banded_observed_with_atol(
                            &ode, &band, t_span, y0, &adaptive, &schedule, atol,
                        ),
                    ),
                };
                let result = result.map_err(|e| driver_error(e, &guard))?;
                Run {
                    route: RouteRecord {
                        entry_point,
                        driver_id: result.fast.driver,
                        routing: None,
                    },
                    observed: result.fast.observed,
                    attempts: result.fast.attempts,
                    accepted_steps: result.fast.accepted_steps,
                    rejected_steps: result.fast.rejected_steps,
                    matrix_free: false,
                    driver_work: DriverWork::Banded {
                        jacobian_reuses: result.fast.jacobian_reuses,
                        lu: (&result.fast.lu).into(),
                        banded: (&result.work).into(),
                    },
                    partial: result.partial,
                    segments: None,
                }
            }
            Plan::SequentialDense => {
                // Dense output samples the interior estimate on every
                // step, since `Solution::at` may evaluate any of them;
                // without it only the endpoint is returned and controlled.
                let (control, interior_sampling) = if opts.dense_output {
                    let control = match opts.interior_error {
                        InteriorError::WFilteredEnforce => DenseErrorControl::WFilteredEnforce,
                        InteriorError::Enforce => DenseErrorControl::Enforce,
                        InteriorError::Off => DenseErrorControl::Off,
                    };
                    (control, InteriorSampling::EveryStep)
                } else {
                    (DenseErrorControl::Off, InteriorSampling::OutputTimes)
                };
                // `None` is the sequential step's default: the direct LU.
                let linear_solver = LinearSolverConfig::default();
                // Without dense output no continuous extension is returned,
                // so the run does not record one (10 n f64 per accepted
                // step); the run is otherwise bit for bit the same.
                let plan_out = OutputSamplingPlan::dense(schedule.clone());
                let run = if opts.dense_output {
                    integrate_adaptive_dense_observed_with_segments(
                        &ode,
                        t_span,
                        y0,
                        IntegrationMethod::Sequential,
                        None,
                        None,
                        &adaptive,
                        &plan_out,
                        control,
                        interior_sampling,
                    )
                    .map_err(|e| driver_error(e, &guard))?
                } else {
                    let (result, report, partial) = integrate_adaptive_dense_observed_with_partial(
                        &ode,
                        t_span,
                        y0,
                        IntegrationMethod::Sequential,
                        None,
                        None,
                        &adaptive,
                        &plan_out,
                        control,
                        interior_sampling,
                    )
                    .map_err(|e| driver_error(e, &guard))?;
                    AdaptiveDenseSegmentRun {
                        result,
                        report,
                        segments: Vec::new(),
                        partial,
                    }
                };
                let diagnostics = &run.result.diagnostics;
                Run {
                    attempts: diagnostics.attempts,
                    accepted_steps: diagnostics.accepted_macro_steps,
                    rejected_steps: diagnostics.rejected_macro_steps,
                    route: RouteRecord {
                        entry_point: if opts.dense_output {
                            ENTRY_SEQUENTIAL_DENSE
                        } else {
                            ENTRY_SEQUENTIAL_NO_SEGMENTS
                        },
                        driver_id: RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID,
                        routing: None,
                    },
                    matrix_free: false,
                    driver_work: DriverWork::SequentialDense {
                        control: (&control).into(),
                        interior_sampling: (&interior_sampling).into(),
                        linear_solver: (&linear_solver).into(),
                        samples: run.report.samples.len(),
                        interior_rejections: run
                            .report
                            .samples
                            .iter()
                            .filter(|sample| !sample.accepted)
                            .count(),
                        unavailable: run.report.unavailable,
                        estimate_failures: run.report.estimate_failures,
                        max_accepted_estimate: run.report.max_accepted_estimate(),
                        estimate_counters: (&run.report.counters).into(),
                        segments: run.segments.len(),
                    },
                    observed: run.result.observed,
                    partial: run.partial,
                    segments: opts
                        .dense_output
                        .then_some(Extension::Sequential(run.segments)),
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
            None => Status::Failed(failure_reason(
                run.partial.as_ref(),
                run.attempts,
                &adaptive,
            )),
        };

        // The returned states: every accepted step under dense output;
        // otherwise the driver's samples, followed after a failure by the
        // last accepted state.
        let (t, y) = match &run.segments {
            Some(Extension::Fast(segments)) => (
                segments.times().to_vec(),
                (0..=segments.len())
                    .map(|k| segments.state(k).to_vec())
                    .collect(),
            ),
            Some(Extension::Sequential(segments)) => {
                let mut t = Vec::with_capacity(segments.len() + 1);
                let mut y = Vec::with_capacity(segments.len() + 1);
                t.push(t_span.0);
                y.push(y0.to_vec());
                for segment in segments {
                    t.push(segment.t_new);
                    y.push(segment.y_new.clone());
                }
                (t, y)
            }
            None => {
                let (mut t, mut y) = (run.observed.t.clone(), run.observed.y.clone());
                if let Some(partial) = &run.partial
                    && t.last().is_none_or(|&last| last < partial.t)
                {
                    t.push(partial.t);
                    y.push(partial.y.clone());
                }
                (t, y)
            }
        };
        let stop = run.partial.as_ref().map(|partial| StopRecord {
            t: partial.t,
            reason: (&partial.reason).into(),
            last_failure: partial.last_failure.as_ref().map(Into::into),
        });

        let stats = Stats {
            schema: STATS_SCHEMA,
            options: opts.clone(),
            matrix_free_linear: run
                .matrix_free
                .then(|| (&RoutingOptions::default().matrix_free).into()),
            adaptive: (&adaptive).into(),
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
            counters: (&counters).into(),
            driver_counters: (&driver_counters).into(),
            driver_work: run.driver_work,
            finite_difference: fd,
            callbacks: guard.callback_work(),
            stop,
            driver_message: run.observed.message,
            terminal_event: None,
            event_evaluations: 0,
        };
        let mut solution = Solution {
            t,
            y,
            stats,
            status,
            events: Vec::new(),
            extension: run.segments,
        };
        let (evaluations, terminal) = crate::events::locate(&mut solution, &problem.events);
        solution.stats.event_evaluations = evaluations;
        solution.stats.terminal_event = terminal;
        Ok(solution)
    }
}

/// The typed reason of a run that stopped early without a fatal callback
/// error: the driver's stop reason and the kind of its last rejection.
fn failure_reason(
    partial: Option<&DriverPartial>,
    attempts: usize,
    adaptive: &AdaptiveStepConfig,
) -> FailureReason {
    let min_step = adaptive.min_step;
    let budget = FailureReason::MaxStepsReached {
        max_steps: adaptive.max_attempts,
    };
    match partial {
        Some(partial) if partial.reason == DriverStopReason::AttemptBudget => budget,
        Some(partial) => match partial.last_failure {
            Some(AdaptiveFailureKind::LinearSolve) => FailureReason::LinearSolveFailed { min_step },
            Some(AdaptiveFailureKind::NonFinite) => FailureReason::NonFinite { min_step },
            Some(AdaptiveFailureKind::NonlinearSolve) => {
                FailureReason::NonlinearSolveFailed { min_step }
            }
            Some(AdaptiveFailureKind::LocalError) | None => {
                FailureReason::StepSizeTooSmall { min_step }
            }
        },
        // Every routed driver reports a partial record on failure; this
        // keeps a typed answer if one does not.
        None if attempts >= adaptive.max_attempts => budget,
        None => FailureReason::StepSizeTooSmall { min_step },
    }
}

fn driver_error(error: impl Display, guard: &Guard) -> SolveError {
    match guard.fatal() {
        Some(fatal) => SolveError::Driver(format!(
            "{error}; first fatal {:?} callback error at t = {:e}: {}",
            fatal.callback, fatal.t, fatal.message
        )),
        None => SolveError::Driver(error.to_string()),
    }
}

/// Every refusal that does not need the drivers; returns the validated
/// absolute tolerance.
fn validate(
    problem: &Problem,
    (t0, t1): (f64, f64),
    y0: &[f64],
    opts: &Options,
) -> Result<AtolSpec, SolveError> {
    let invalid = |m: String| Err(SolveError::InvalidInput(m));
    let unsupported = |u: Unsupported| Err(SolveError::Unsupported(u));
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
        Atol::Scalar(a) => AtolSpec::Scalar(*a),
        Atol::Vector(v) => {
            if v.len() != n {
                return invalid(format!(
                    "atol has length {}, the problem dimension is {n}",
                    v.len()
                ));
            }
            if !v.iter().all(|a| a.is_finite() && *a >= 0.0) {
                return invalid("every atol entry must be finite and nonnegative".into());
            }
            AtolSpec::Vector(v.clone())
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
        if !m.iter().all(|v| v.is_finite()) {
            return invalid("the mass matrix contains NaN/Inf".into());
        }
    }
    if let Some(reason) = crate::capabilities::primary_unsupported(problem, opts) {
        return unsupported(reason);
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
    if let Jacobian::Banded { lower, upper, .. } = &problem.jacobian {
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
    if let Some(reason) = crate::capabilities::final_unsupported(problem) {
        return unsupported(reason);
    }
    Ok(atol)
}

/// Whether a run takes the sequential dense driver: a mass matrix, or dense
/// output under `InteriorError::Enforce`.
fn sequential_route(problem: &Problem, opts: &Options) -> bool {
    problem.mass.is_some() || (opts.dense_output && opts.interior_error == InteriorError::Enforce)
}

/// The driver-level problem, the plan and the Jacobian record.
fn build(
    problem: &Problem,
    guard: &Arc<Guard>,
    sequential: bool,
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
    // The in-place form of a user dense Jacobian for the fast drivers, which
    // keep one matrix across steps (no n x n allocation per build).
    let mut jacobian_into: Option<JacobianIntoFn> = None;
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
            jacobian_into = Some({
                let (user, guard) = (user.clone(), guard.clone());
                Arc::new(move |t, y, m: &mut DenseMatrix| {
                    // The driver hands back the previous matrix; the facade
                    // documents a zeroed buffer before every call (B3d).
                    m.as_mut_slice().fill(0.0);
                    guard.call(Callback::Jacobian, t, || user(t, y, m.as_mut_slice()))?;
                    finite(m.as_slice(), "dense Jacobian")
                })
            });
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

    // The sequential dense driver reads the Jacobian through the dense
    // provider set above (a band is assembled into it, a finite-difference
    // band keeps its grouped columns).
    let plan = if sequential {
        Plan::SequentialDense
    } else {
        plan
    };
    let mass = problem
        .mass
        .as_ref()
        .map(|m| DenseMatrix::new(n, n, m.clone()))
        .transpose()
        .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
    let ode = OdeProblem::new(
        "vigilode-facade",
        n,
        rhs,
        None,
        jacobian,
        jvp,
        partial_t,
        problem.autonomous,
        mass,
        None,
    )
    .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
    let ode = match jacobian_into {
        Some(fill) => ode.with_jacobian_into(fill),
        None => ode,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn partial(
        reason: DriverStopReason,
        last_failure: Option<AdaptiveFailureKind>,
    ) -> DriverPartial {
        DriverPartial {
            t: 0.25,
            y: vec![1.0],
            reason,
            last_failure,
        }
    }

    #[test]
    fn failure_reason_follows_the_driver_stop_and_last_rejection() {
        let adaptive = AdaptiveStepConfig::default();
        let min_step = adaptive.min_step;
        let budget = FailureReason::MaxStepsReached {
            max_steps: adaptive.max_attempts,
        };
        let reason =
            |p: Option<DriverPartial>, attempts| failure_reason(p.as_ref(), attempts, &adaptive);
        use AdaptiveFailureKind as K;
        use DriverStopReason as R;
        assert_eq!(
            reason(Some(partial(R::AttemptBudget, Some(K::LinearSolve))), 0),
            budget
        );
        assert_eq!(
            reason(Some(partial(R::MinimumStep, Some(K::LinearSolve))), 0),
            FailureReason::LinearSolveFailed { min_step }
        );
        assert_eq!(
            reason(Some(partial(R::MinimumStep, Some(K::NonFinite))), 0),
            FailureReason::NonFinite { min_step }
        );
        assert_eq!(
            reason(Some(partial(R::MinimumStep, Some(K::NonlinearSolve))), 0),
            FailureReason::NonlinearSolveFailed { min_step }
        );
        for last in [Some(K::LocalError), None] {
            assert_eq!(
                reason(Some(partial(R::MinimumStep, last)), 0),
                FailureReason::StepSizeTooSmall { min_step }
            );
        }
        assert_eq!(reason(None, adaptive.max_attempts), budget);
        assert_eq!(
            reason(None, 1),
            FailureReason::StepSizeTooSmall { min_step }
        );
    }
}
