//! Diffsol 0.16.1 comparator of the component-triage campaign
//! (`research/component_triage_20261019`, contract `HARNESS_CONTRACT.md`).
//!
//! Subcommands (exit 0 whenever a row/document was printed, 2 on a usage error):
//!
//! * `list`: the problems and methods of this harness (`vigilode-diffsol-triage-list-v1`).
//! * `run --problem P --method {bdf,esdirk34,tr-bdf2} --rtol R --grid {endpoint|uniform101}
//!   --campaign ID [--repetitions K]`: one `vigilode-triage-row-v1` row of arm
//!   `ext.diffsol-<method>`. Scalar `atol = rtol * atol_scale`, output times of the grid
//!   (`t0 + (tf - t0) i / 100`, both ends exact), solution by `OdeSolverMethod::solve_dense`.
//!   Nothing is timed. Raw upstream statistics go to `extra_work`.
//! * `parity --input PARITY.json`: reads a `rodas5p triage-parity` document and prints the
//!   same layout (`vigilode-triage-parity-v1`) evaluated by the Diffsol-side operators.

mod problems;

use std::cell::Cell;
use std::rc::Rc;

use diffsol::error::OdeSolverError;
use diffsol::ode_solver::OdeSolverStatistics;
use diffsol::{
    ConstantOp, DenseMatrix, DiffsolError, FaerContext, FaerLU, FaerMat, FaerVec, LaError,
    MatrixCommon, NonLinearOp, NonLinearOpJacobian, OdeBuilder, OdeEquations, OdeSolverMethod,
    OdeSolverStopReason, Op, Vector, VectorHost,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use problems::Model;

const ROW_SCHEMA: &str = "vigilode-triage-row-v1";
const PARITY_SCHEMA: &str = "vigilode-triage-parity-v1";
const LIST_SCHEMA: &str = "vigilode-diffsol-triage-list-v1";
const COUNTER_SEMANTICS: &str = "external-upstream-raw";
const DIFFSOL_VERSION: &str = "0.16.1";
const UNIFORM_POINTS: usize = 101;

type M = FaerMat<f64>;
type V = FaerVec<f64>;

// ------------------------------------------------------------------ methods

#[derive(Clone, Copy, PartialEq, Eq)]
enum Method {
    Bdf,
    Esdirk34,
    TrBdf2,
}

impl Method {
    const ALL: [Method; 3] = [Method::Bdf, Method::Esdirk34, Method::TrBdf2];

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == text)
    }
    fn id(self) -> &'static str {
        match self {
            Self::Bdf => "bdf",
            Self::Esdirk34 => "esdirk34",
            Self::TrBdf2 => "tr-bdf2",
        }
    }
    fn arm(self) -> String {
        format!("ext.diffsol-{}", self.id())
    }
    fn constructor(self) -> &'static str {
        match self {
            Self::Bdf => {
                "OdeSolverProblem::bdf::<LS>() (variable-order BDF, NewtonNonlinearSolver<NoLineSearch>)"
            }
            Self::Esdirk34 => "OdeSolverProblem::esdirk34::<LS>() (Sdirk with Tableau::esdirk34)",
            Self::TrBdf2 => "OdeSolverProblem::tr_bdf2::<LS>() (Sdirk with Tableau::tr_bdf2)",
        }
    }
}

// ------------------------------------------------------------------ grid

#[derive(Clone, Copy, PartialEq, Eq)]
enum Grid {
    Endpoint,
    Uniform101,
}

impl Grid {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "endpoint" => Some(Self::Endpoint),
            "uniform101" => Some(Self::Uniform101),
            _ => None,
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Endpoint => "endpoint",
            Self::Uniform101 => "uniform101",
        }
    }
    /// The contract grid (`triage.rs::Grid::times`): `[t0, tf]`, or
    /// `t0 + (tf - t0) i / 100` with both ends exact.
    fn times(self, (t0, tf): (f64, f64)) -> Vec<f64> {
        match self {
            Self::Endpoint => vec![t0, tf],
            Self::Uniform101 => {
                let last = (UNIFORM_POINTS - 1) as f64;
                let mut times: Vec<f64> = (0..UNIFORM_POINTS)
                    .map(|i| t0 + (tf - t0) * i as f64 / last)
                    .collect();
                times[0] = t0;
                times[UNIFORM_POINTS - 1] = tf;
                times
            }
        }
    }
}

// ------------------------------------------------------------------ counting LU

thread_local! {
    static LU_FACTORIZATIONS: Cell<u64> = const { Cell::new(0) };
    static LU_SOLVES: Cell<u64> = const { Cell::new(0) };
}

/// `FaerLU<f64>` with a count of its factorizations (`set_linearisation`, one faer
/// `FullPivLu` each) and back-substitutions. Pure delegation: the numbers are the
/// upstream ones; the counts cross-check `number_of_linear_solver_setups`.
#[derive(Default)]
struct CountingLu(FaerLU<f64>);

impl diffsol_la::LinearSolver<M> for CountingLu {
    fn set_linearisation<C: diffsol_la::LinearOp<T = f64, V = V, M = M, C = FaerContext>>(
        &mut self,
        op: &C,
    ) {
        LU_FACTORIZATIONS.with(|c| c.set(c.get() + 1));
        diffsol_la::LinearSolver::set_linearisation(&mut self.0, op);
    }

    fn set_sparsity<C: diffsol_la::LinearOp<T = f64, V = V, M = M, C = FaerContext>>(
        &mut self,
        op: &C,
    ) {
        diffsol_la::LinearSolver::set_sparsity(&mut self.0, op);
    }

    fn solve_in_place(&self, x: &mut V) -> Result<(), LaError> {
        LU_SOLVES.with(|c| c.set(c.get() + 1));
        diffsol_la::LinearSolver::solve_in_place(&self.0, x)
    }
}

fn reset_lu_counters() {
    LU_FACTORIZATIONS.with(|c| c.set(0));
    LU_SOLVES.with(|c| c.set(0));
}

// ------------------------------------------------------------------ helpers

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hex_vec(values: &[f64]) -> Vec<String> {
    values.iter().map(|x| hex(*x)).collect()
}

fn unhex(value: &Value) -> Result<f64, String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("expected a hex string, got {value}"))?;
    if text.len() != 16
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!(
            "not a 16-digit lowercase binary64 hex string: {text:?}"
        ));
    }
    u64::from_str_radix(text, 16)
        .map(f64::from_bits)
        .map_err(|e| e.to_string())
}

fn unhex_vec(value: &Value) -> Result<Vec<f64>, String> {
    value
        .as_array()
        .ok_or_else(|| "expected a list of hex strings".to_string())?
        .iter()
        .map(unhex)
        .collect()
}

fn binary_sha256() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    let bytes = std::fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(&bytes)))
}

fn source_json() -> Value {
    let commit = std::env::var("VIGILODE_SOURCE_COMMIT")
        .ok()
        .filter(|s| !s.is_empty());
    let clean = match std::env::var("VIGILODE_TREE_CLEAN").ok().as_deref() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    };
    json!({"commit": commit, "tree_clean": clean})
}

fn usage_exit(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!(
        "usage: diffsol-triage list\n       diffsol-triage run --problem P --method {{bdf,esdirk34,tr-bdf2}} --rtol R \
         --grid {{endpoint|uniform101}} --campaign ID [--repetitions K]\n       diffsol-triage parity --input PARITY.json"
    );
    std::process::exit(2);
}

fn options(args: &[String]) -> Vec<(String, String)> {
    if !args.len().is_multiple_of(2) {
        usage_exit("options come in --key value pairs");
    }
    args.chunks_exact(2)
        .map(|pair| {
            if !pair[0].starts_with("--") {
                usage_exit(&format!("unexpected argument {:?}", pair[0]));
            }
            (pair[0].clone(), pair[1].clone())
        })
        .collect()
}

fn option<'a>(opts: &'a [(String, String)], key: &str) -> Option<&'a str> {
    opts.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn require<'a>(opts: &'a [(String, String)], key: &str) -> &'a str {
    option(opts, key).unwrap_or_else(|| usage_exit(&format!("missing {key}")))
}

fn check_known(opts: &[(String, String)], known: &[&str]) {
    for (key, _) in opts {
        if !known.contains(&key.as_str()) {
            usage_exit(&format!("unknown option {key}"));
        }
    }
}

// ------------------------------------------------------------------ problem construction

/// Callback invocations seen by the harness closures (a cross-check of the
/// upstream `OpStatistics` of the right-hand side).
#[derive(Default)]
struct Callbacks {
    rhs: Cell<u64>,
    jvp: Cell<u64>,
}

/// Builds `OdeBuilder::<FaerMat<f64>>` for `model` and evaluates `$body` with
/// the built problem bound to `$problem` (the closure types are unnameable, so
/// the problem never leaves this expansion).
macro_rules! with_problem {
    ($model:expr, $rtol:expr, $atol:expr, $callbacks:expr, |$problem:ident| $body:expr) => {{
        let model: Rc<Model> = $model;
        let callbacks: Rc<Callbacks> = $callbacks;
        let (m_rhs, m_jvp, m_init) = (model.clone(), model.clone(), model.clone());
        let (c_rhs, c_jvp) = (callbacks.clone(), callbacks.clone());
        let n = model.dimension();
        let built = OdeBuilder::<M>::new()
            .t0(model.t_span.0)
            .rtol($rtol)
            .atol([$atol])
            .rhs_implicit(
                move |x: &V, _p: &V, t: f64, y: &mut V| {
                    c_rhs.rhs.set(c_rhs.rhs.get() + 1);
                    (m_rhs.rhs)(t, x.as_slice(), y.as_mut_slice());
                },
                move |x: &V, _p: &V, t: f64, v: &V, y: &mut V| {
                    c_jvp.jvp.set(c_jvp.jvp.get() + 1);
                    (m_jvp.jvp)(t, x.as_slice(), v.as_slice(), y.as_mut_slice());
                },
            )
            .init(
                move |_p: &V, _t: f64, y: &mut V| y.as_mut_slice().copy_from_slice(&m_init.y0),
                n,
            )
            .build();
        match built {
            Err(error) => Err(error),
            Ok($problem) => Ok($body),
        }
    }};
}

// ------------------------------------------------------------------ run

/// What one integration produced.
struct Integration {
    status: &'static str,
    error: Option<String>,
    error_variant: Option<String>,
    stop_reason: Option<String>,
    output_times: Vec<f64>,
    states: Vec<Vec<f64>>,
    solver_statistics: Option<OdeSolverStatistics>,
    rhs_statistics: Option<Value>,
    solver_options: Option<Value>,
    rhs_closure_calls: u64,
    jvp_closure_calls: u64,
    lu_factorizations: u64,
    lu_solves: u64,
}

fn classify(error: &DiffsolError) -> &'static str {
    match error {
        DiffsolError::OdeSolverError(
            OdeSolverError::TooManyNonlinearSolverFailures { .. }
            | OdeSolverError::TooManyErrorTestFailures { .. }
            | OdeSolverError::StepSizeTooSmall { .. },
        ) => "solver_failure",
        _ => "error",
    }
}

fn variant(error: &DiffsolError) -> String {
    let debug = format!("{error:?}");
    debug.split(['{', ' ']).next().unwrap_or("").to_string()
}

/// `solve_dense` on a freshly constructed solver; the statistics are read from
/// the solver after the call (also when it failed).
macro_rules! drive {
    ($constructed:expr, $times:expr) => {{
        match $constructed {
            Err(error) => (Err(error), None),
            Ok(mut solver) => {
                let result = solver.solve_dense($times);
                let statistics: OdeSolverStatistics = solver.get_statistics().clone();
                (result, Some(statistics))
            }
        }
    }};
}

fn integrate(model: Rc<Model>, method: Method, rtol: f64, atol: f64, times: &[f64]) -> Integration {
    reset_lu_counters();
    let callbacks = Rc::new(Callbacks::default());
    let n = model.dimension();
    type Outcome = (
        Result<(M, OdeSolverStopReason<f64>), DiffsolError>,
        Option<OdeSolverStatistics>,
        Option<Value>,
        Value,
    );
    let outcome: Result<Outcome, DiffsolError> =
        with_problem!(model.clone(), rtol, atol, callbacks.clone(), |problem| {
            let (result, statistics) = match method {
                Method::Bdf => drive!(problem.bdf::<CountingLu>(), times),
                Method::Esdirk34 => drive!(problem.esdirk34::<CountingLu>(), times),
                Method::TrBdf2 => drive!(problem.tr_bdf2::<CountingLu>(), times),
            };
            let rhs_statistics = serde_json::to_value(problem.eqn.rhs().statistics()).ok();
            let o = &problem.ode_options;
            let options = json!({
                "max_nonlinear_solver_iterations": o.max_nonlinear_solver_iterations,
                "max_error_test_failures": o.max_error_test_failures,
                "max_nonlinear_solver_failures": o.max_nonlinear_solver_failures,
                "nonlinear_solver_tolerance": o.nonlinear_solver_tolerance,
                "min_timestep": o.min_timestep,
                "max_timestep_growth": o.max_timestep_growth,
                "min_timestep_growth": o.min_timestep_growth,
                "max_timestep_shrink": o.max_timestep_shrink,
                "min_timestep_shrink": o.min_timestep_shrink,
                "update_jacobian_after_steps": o.update_jacobian_after_steps,
                "update_rhs_jacobian_after_steps": o.update_rhs_jacobian_after_steps,
                "threshold_to_update_jacobian": o.threshold_to_update_jacobian,
                "threshold_to_update_rhs_jacobian": o.threshold_to_update_rhs_jacobian,
                "pi_control_proportional": o.pi_control_proportional,
                "pi_control_integral": o.pi_control_integral,
                "builder_h0": problem.h0,
                "builder_use_coloring": false,
            });
            (result, statistics, rhs_statistics, options)
        });
    let mut out = Integration {
        status: "error",
        error: None,
        error_variant: None,
        stop_reason: None,
        output_times: Vec::new(),
        states: Vec::new(),
        solver_statistics: None,
        rhs_statistics: None,
        solver_options: None,
        rhs_closure_calls: 0,
        jvp_closure_calls: 0,
        lu_factorizations: 0,
        lu_solves: 0,
    };
    match outcome {
        Err(error) => {
            out.status = "refused";
            out.error = Some(format!("OdeBuilder::build: {error}"));
            out.error_variant = Some(variant(&error));
        }
        Ok((result, statistics, rhs_statistics, options)) => {
            out.solver_statistics = statistics;
            out.rhs_statistics = rhs_statistics;
            out.solver_options = Some(options);
            match result {
                Err(error) => {
                    out.status = if out.solver_statistics.is_some() {
                        classify(&error)
                    } else {
                        "error"
                    };
                    let stage = if out.solver_statistics.is_some() {
                        "solve_dense"
                    } else {
                        "solver construction"
                    };
                    out.error = Some(format!("{stage}: {error}"));
                    out.error_variant = Some(variant(&error));
                }
                Ok((matrix, stop_reason)) => {
                    out.stop_reason = Some(format!("{stop_reason:?}"));
                    if matrix.ncols() != times.len() || matrix.nrows() != n {
                        out.status = "error";
                        out.error = Some(format!(
                            "solve_dense returned a {} x {} matrix for {} states and {} output times",
                            matrix.nrows(),
                            matrix.ncols(),
                            n,
                            times.len()
                        ));
                    } else if !matches!(stop_reason, OdeSolverStopReason::TstopReached) {
                        out.status = "error";
                        out.error = Some(format!("solve_dense stopped with {stop_reason:?}"));
                    } else {
                        out.status = "ok";
                        out.output_times = times.to_vec();
                        out.states = (0..times.len())
                            .map(|col| (0..n).map(|row| matrix.get_index(row, col)).collect())
                            .collect();
                    }
                }
            }
        }
    }
    out.rhs_closure_calls = callbacks.rhs.get();
    out.jvp_closure_calls = callbacks.jvp.get();
    out.lu_factorizations = LU_FACTORIZATIONS.with(|c| c.get());
    out.lu_solves = LU_SOLVES.with(|c| c.get());
    out
}

fn stat_u64(stats: &Value, key: &str) -> Option<u64> {
    stats.get(key).and_then(Value::as_u64)
}

/// The `extra_work` object: raw upstream counters, the derived `nfev`/`njev`/`nlu`
/// and the mapping that derives them.
fn extra_work(
    run: &Integration,
    method: Method,
    atol: f64,
) -> (Value, Option<u64>, Option<u64>, Option<u64>) {
    let solver = run
        .solver_statistics
        .as_ref()
        .and_then(|s| serde_json::to_value(s).ok());
    let rhs = run.rhs_statistics.clone();
    let reason_no_solver = "the solver was not constructed (no OdeSolverStatistics exist)";
    let reason_no_problem = "the problem was not built (no OpStatistics exist)";
    let (nfev, nfev_reason) = match rhs.as_ref().and_then(|r| stat_u64(r, "number_of_calls")) {
        Some(v) => (Some(v), None),
        None => (None, Some(reason_no_problem)),
    };
    let (njev, njev_reason) = match rhs
        .as_ref()
        .and_then(|r| stat_u64(r, "number_of_matrix_evals"))
    {
        Some(v) => (Some(v), None),
        None => (None, Some(reason_no_problem)),
    };
    let (nlu, nlu_reason) = match solver
        .as_ref()
        .and_then(|s| stat_u64(s, "number_of_linear_solver_setups"))
    {
        Some(v) => (Some(v), None),
        None => (None, Some(reason_no_solver)),
    };
    let steps = solver.as_ref().and_then(|s| stat_u64(s, "number_of_steps"));
    let etf = solver
        .as_ref()
        .and_then(|s| stat_u64(s, "number_of_error_test_failures"));
    let nlsf = solver
        .as_ref()
        .and_then(|s| stat_u64(s, "number_of_nonlinear_solver_fails"));
    let rejected = match (etf, nlsf) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    };
    let attempts = match (steps, rejected) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    };
    let jac_muls = rhs.as_ref().and_then(|r| stat_u64(r, "number_of_jac_muls"));
    let checks = json!({
        "nfev_equals_rhs_closure_calls": nfev.map(|v| v == run.rhs_closure_calls),
        "jac_muls_equal_jvp_closure_calls": jac_muls.map(|v| v == run.jvp_closure_calls),
        "nlu_equals_lu_factorizations": nlu.map(|v| v == run.lu_factorizations),
    });
    let value = json!({
        "upstream": format!("diffsol {DIFFSOL_VERSION}"),
        "method": method.id(),
        "constructor": method.constructor(),
        "linear_solver": "FaerLU<f64> (faer FullPivLu on the dense FaerMat), wrapped by a delegating counter",
        "solve_api": "OdeSolverMethod::solve_dense(t_eval = the grid): one column per requested time, interpolated by the solver's own dense output; t_eval[last] is the stop time (set_stop_time)",
        "stop_reason": run.stop_reason,
        "error_variant": run.error_variant,
        "solver_statistics": solver,
        "solver_statistics_reason": if run.solver_statistics.is_none() { Some(reason_no_solver) } else { None },
        "rhs_op_statistics": rhs,
        "rhs_op_statistics_reason": if run.rhs_statistics.is_none() { Some(reason_no_problem) } else { None },
        "nfev": nfev,
        "nfev_reason": nfev_reason,
        "njev": njev,
        "njev_reason": njev_reason,
        "nlu": nlu,
        "nlu_reason": nlu_reason,
        "derived_mapping": {
            "nfev": "rhs_op_statistics.number_of_calls: every evaluation of f through the Diffsol rhs operator (NonLinearOp::call_inplace), including the two of state initialisation (dy0 and the starting-step heuristic)",
            "njev": "rhs_op_statistics.number_of_matrix_evals: full Jacobian evaluations of f (CVODE nje); each is assembled by Diffsol from `dimension` analytic J v calls with unit vectors (use_coloring = false, dense faer matrix), counted in rhs_op_statistics.number_of_jac_muls",
            "nlu": "solver_statistics.number_of_linear_solver_setups: Jacobian/LU setups (CVODE nsetups) including the initial one; each setup forms I - c J (reusing the cached J unless it was marked stale) and runs one FullPivLu factorization",
            "accepted_steps": "solver_statistics.number_of_steps (incremented only after an accepted step: bdf.rs step(), runge_kutta.rs step_accepted)",
            "rejected_steps": "solver_statistics.number_of_error_test_failures + solver_statistics.number_of_nonlinear_solver_fails (each is one failed step attempt that is retried; both counters are cumulative over the integration)",
            "attempts": "accepted_steps + rejected_steps",
        },
        "harness_counters": {
            "rhs_closure_calls": run.rhs_closure_calls,
            "jvp_closure_calls": run.jvp_closure_calls,
            "lu_factorizations": run.lu_factorizations,
            "lu_solves": run.lu_solves,
        },
        "mapping_check": checks,
        "solver_options": run.solver_options,
        "tolerance": {
            "rtol_and_scalar_atol": "OdeBuilder::rtol(rtol).atol([atol]): the one atol is broadcast to every component (build_atol)",
            "atol": atol,
            "error_norm": "Diffsol WRMS: sqrt(mean_i (e_i / (atol + rtol |y_i|))^2) <= 1",
        },
        "h0_policy": "solver_default",
        "h0_policy_detail": "Diffsol chooses the first step itself (StateRefMut::set_step_size, Hairer-Norsett-Wanner I II.4.2 heuristic); OdeBuilder::h0 only fixes its sign and is left at the default 1.0",
        "max_step": "none (Diffsol has no maximum step option)",
        "timing": "none recorded",
    });
    (value, attempts, steps, rejected)
}

fn cmd_run(args: &[String]) {
    let opts = options(args);
    check_known(
        &opts,
        &[
            "--problem",
            "--method",
            "--rtol",
            "--grid",
            "--campaign",
            "--repetitions",
        ],
    );
    let problem_id = require(&opts, "--problem");
    let method = Method::parse(require(&opts, "--method"))
        .unwrap_or_else(|| usage_exit("--method must be one of bdf, esdirk34, tr-bdf2"));
    let rtol: f64 = require(&opts, "--rtol")
        .parse()
        .unwrap_or_else(|_| usage_exit("--rtol is not a number"));
    if !(rtol.is_finite() && rtol > 0.0) {
        usage_exit("--rtol must be finite and positive");
    }
    let grid = Grid::parse(require(&opts, "--grid"))
        .unwrap_or_else(|| usage_exit("--grid must be endpoint or uniform101"));
    let campaign = require(&opts, "--campaign");
    let repetitions: usize = option(&opts, "--repetitions")
        .unwrap_or("1")
        .parse()
        .unwrap_or_else(|_| usage_exit("--repetitions is not a count"));
    if repetitions == 0 {
        usage_exit("--repetitions must be at least 1");
    }
    let model = Rc::new(
        problems::model(problem_id)
            .unwrap_or_else(|| usage_exit(&format!("unknown problem {problem_id:?}"))),
    );
    let atol = rtol * model.atol_scale;
    let times = grid.times(model.t_span);
    let runs: Vec<Integration> = (0..repetitions)
        .map(|_| integrate(model.clone(), method, rtol, atol, &times))
        .collect();
    let first = &runs[0];
    let fingerprint = |r: &Integration| {
        (
            r.status,
            r.states
                .iter()
                .map(|s| s.iter().map(|x| x.to_bits()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            r.solver_statistics
                .as_ref()
                .and_then(|s| serde_json::to_string(s).ok()),
            r.rhs_statistics.as_ref().map(Value::to_string),
        )
    };
    let deterministic = runs.iter().all(|r| fingerprint(r) == fingerprint(first));
    let (extra, attempts, accepted, rejected) = extra_work(first, method, atol);
    let row = json!({
        "schema": ROW_SCHEMA,
        "campaign": campaign,
        "arm": method.arm(),
        "components": [],
        "problem": model.id,
        "tier": model.tier,
        "dimension": model.dimension(),
        "rtol": rtol,
        "atol": atol,
        "grid": grid.id(),
        "h0_scale": 1.0,
        "h0": null,
        "status": first.status,
        "error": first.error,
        "success": first.status == "ok",
        "attempts": attempts,
        "accepted_steps": accepted,
        "rejected_steps": rejected,
        "output_times": first.output_times,
        "states_hex": first.states.iter().map(|s| hex_vec(s)).collect::<Vec<_>>(),
        "counters": null,
        "extra_work": extra,
        "counter_semantics": COUNTER_SEMANTICS,
        "deterministic": deterministic,
        "repetitions": repetitions,
        "binary": {"sha256": binary_sha256(), "profile": "external"},
        "source": source_json(),
    });
    println!("{row}");
}

// ------------------------------------------------------------------ parity

fn cmd_parity(args: &[String]) {
    let opts = options(args);
    check_known(&opts, &["--input"]);
    let path = require(&opts, "--input");
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| usage_exit(&format!("cannot read {path}: {e}")));
    let doc: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| usage_exit(&format!("{path}: not JSON: {e}")));
    if doc.get("schema").and_then(Value::as_str) != Some(PARITY_SCHEMA) {
        usage_exit(&format!("{path}: schema is not {PARITY_SCHEMA}"));
    }
    let problem_id = doc
        .get("problem")
        .and_then(Value::as_str)
        .unwrap_or_else(|| usage_exit("parity document has no problem"));
    let model = Rc::new(
        problems::model(problem_id)
            .unwrap_or_else(|| usage_exit(&format!("unknown problem {problem_id:?}"))),
    );
    let n = model.dimension();
    if doc.get("dimension").and_then(Value::as_u64) != Some(n as u64) {
        usage_exit(&format!(
            "parity document dimension {:?} != Diffsol model dimension {n}",
            doc.get("dimension")
        ));
    }
    let direction = unhex_vec(&doc["jvp_direction_hex"])
        .unwrap_or_else(|e| usage_exit(&format!("jvp_direction_hex: {e}")));
    if direction.len() != n {
        usage_exit("jvp_direction_hex has the wrong length");
    }
    let samples_in = doc
        .get("samples")
        .and_then(Value::as_array)
        .unwrap_or_else(|| usage_exit("parity document has no samples"));
    let callbacks = Rc::new(Callbacks::default());
    // Any tolerance builds the same operators; rtol/atol are not used by f, J or J v.
    let built = with_problem!(
        model.clone(),
        1.0e-6,
        1.0e-6,
        callbacks.clone(),
        |problem| {
            let rhs = problem.eqn.rhs();
            let v = V::from_vec(direction.clone(), FaerContext::default());
            let init = problem.eqn.init().call(problem.t0);
            let mut samples = Vec::new();
            for (index, sample) in samples_in.iter().enumerate() {
                let t = unhex(&sample["t_hex"])
                    .unwrap_or_else(|e| usage_exit(&format!("sample {index}: t_hex: {e}")));
                let y_values = unhex_vec(&sample["y_hex"])
                    .unwrap_or_else(|e| usage_exit(&format!("sample {index}: y_hex: {e}")));
                if y_values.len() != n {
                    usage_exit(&format!(
                        "sample {index}: state length {} != {n}",
                        y_values.len()
                    ));
                }
                let y = V::from_vec(y_values.clone(), FaerContext::default());
                let mut f = V::zeros(n, FaerContext::default());
                rhs.call_inplace(&y, t, &mut f);
                let jac = rhs.jacobian(&y, t);
                let rows: Vec<Vec<String>> = (0..n)
                    .map(|i| (0..n).map(|k| hex(jac.get_index(i, k))).collect())
                    .collect();
                let mut jv = V::zeros(n, FaerContext::default());
                rhs.jac_mul_inplace(&y, t, &v, &mut jv);
                samples.push(json!({
                    "t_hex": hex(t),
                    "y_hex": hex_vec(&y_values),
                    "f_hex": hex_vec(f.as_slice()),
                    "f_error": null,
                    "jacobian": {"format": "dense", "rows_hex": rows},
                    "jvp_hex": hex_vec(jv.as_slice()),
                }));
            }
            (samples, init.as_slice().to_vec())
        }
    );
    let (samples, y0) = built.unwrap_or_else(|e: DiffsolError| {
        eprintln!("error: OdeBuilder::build failed: {e}");
        std::process::exit(1);
    });
    let out = json!({
        "schema": PARITY_SCHEMA,
        "problem": model.id,
        "dimension": n,
        "autonomous": model.autonomous,
        "jacobian": model.rust_jacobian,
        "jvp_direction_hex": hex_vec(&direction),
        "mass_hex": null,
        "samples": samples,
        "diffsol": {
            "producer": "tools/benchmarks/diffsol/triage diffsol-triage parity",
            "version": DIFFSOL_VERSION,
            "input_sha256": format!("{:x}", Sha256::digest(&bytes)),
            "binary_sha256": binary_sha256(),
            "y0_hex": hex_vec(&y0),
            "t_span": [model.t_span.0, model.t_span.1],
            "t_span_hex": [hex(model.t_span.0), hex(model.t_span.1)],
            "atol_scale": model.atol_scale,
            "atol_scale_hex": hex(model.atol_scale),
            "f": "NonLinearOp::call_inplace of the built rhs operator",
            "jacobian": "NonLinearOpJacobian::jacobian of the built rhs operator: dense assembly from `dimension` jac_mul calls with unit vectors (use_coloring = false), the matrix the solvers factorize as I - c J",
            "jvp": "NonLinearOpJacobian::jac_mul_inplace with v = jvp_direction_hex",
            "jvp_origin": model.jvp_origin,
            "source": model.source,
        },
    });
    println!("{out}");
}

// ------------------------------------------------------------------ list

fn cmd_list(args: &[String]) {
    if !args.is_empty() {
        usage_exit("list takes no options");
    }
    let problems: Vec<Value> = problems::PROBLEM_IDS
        .iter()
        .map(|id| {
            let m = problems::model(id).expect("registered problem");
            json!({
                "id": m.id,
                "tier": m.tier,
                "dimension": m.dimension(),
                "t_span": [m.t_span.0, m.t_span.1],
                "atol_scale": m.atol_scale,
                "autonomous": m.autonomous,
                "breakpoints": [],
                "jacobian": m.rust_jacobian,
                "jvp_origin": m.jvp_origin,
                "y0_hex": hex_vec(&m.y0),
                "source": m.source,
            })
        })
        .collect();
    let methods: Vec<Value> = Method::ALL
        .iter()
        .map(|m| json!({"id": m.id(), "arm": m.arm(), "constructor": m.constructor()}))
        .collect();
    println!(
        "{}",
        json!({
            "schema": LIST_SCHEMA,
            "diffsol": DIFFSOL_VERSION,
            "linear_solver": "FaerLU<f64>",
            "methods": methods,
            "problems": problems,
            "binary_sha256": binary_sha256(),
        })
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("parity") => cmd_parity(&args[1..]),
        Some("list") => cmd_list(&args[1..]),
        Some("--help") | Some("-h") | Some("help") => {
            println!(
                "diffsol-triage (Diffsol {DIFFSOL_VERSION}): list | run --problem P --method {{bdf,esdirk34,tr-bdf2}} \
                 --rtol R --grid {{endpoint|uniform101}} --campaign ID [--repetitions K] | parity --input PARITY.json"
            );
        }
        _ => usage_exit("expected a subcommand: list, run or parity"),
    }
}
