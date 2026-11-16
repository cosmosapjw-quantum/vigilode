//! Measurement program of loop L1 node V04_STAGE_COST_ANATOMY (`../PREDECLARED.json`).
//!
//! It runs the protected RODAS5P drivers ("plain" arm) and the same integrations with
//! result-neutral phase timers ("instrumented" arm: the V15 `TimedStageClient` around the dense
//! or banded reference client plus a clock around the right-hand side; for the matrix-free
//! driver clocks around the right-hand side and the JVP and its attempt observer), and prints
//! one JSON object per line. No number is interpreted here: `../build_results.py` does that.
//!
//! Subcommands: `list`, `selftest`, `pilot`, `identity`, `measure`, `single`.

mod instrument;
mod problems;

use std::io::Write as _;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::stage_clients::{
    DenseLuMode, DenseLuStageClient, PhaseTimes, TimedStageClient,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedStageLu, DriverPartial, JacobianIntoFn, OdeProblem,
    OutputSchedule, Rodas5pFastResult, Rodas5pMfFastResult, Rodas5pMfFastWorkspace, RoutingOptions,
    StageLinearSystem, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol, integrate_rodas5p_fast_observed_with_atol,
    integrate_rodas5p_mf_fast_observed_gmres_into, integrate_rodas5p_mf_fast_observed_traced,
};
use serde_json::{Value, json};

use instrument::{
    Counter, MarkedClient, MfClock, RhsClock, fd_band, fd_dense, fd_jvp, marked_jvp, marked_rhs,
    mf_timed_jvp, mf_timed_rhs, multiplied, timed_rhs,
};
use problems::{BandFillFn, JvpFn, ProblemDef, RhsFn};

// ---------------------------------------------------------------------------------------------
// The regime grid.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Driver {
    Dense,
    Banded,
    MatrixFree,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Jac {
    Analytic,
    Fd,
    Jvp,
    JvpFd,
}

impl Jac {
    fn id(self) -> &'static str {
        match self {
            Self::Analytic => "analytic",
            Self::Fd => "fd",
            Self::Jvp => "jvp",
            Self::JvpFd => "jvpfd",
        }
    }
}

#[derive(Clone, Debug)]
struct Cell {
    id: String,
    structure: &'static str,
    driver: Driver,
    family: &'static str,
    /// The size parameter of the family: cells (bruss1d), nodes (radnet), cells per axis
    /// (ac2d, ac3d, bruss2d), 8 (hires).
    size: usize,
    jac: Jac,
    mult: u32,
    exploratory: bool,
}

fn cell(
    structure: &'static str,
    driver: Driver,
    family: &'static str,
    size: usize,
    jac: Jac,
    mult: u32,
) -> Cell {
    let tag = match family {
        "ac2d" | "ac3d" | "bruss2d" => format!("g{size}"),
        "bruss1d" => format!("n{}", 2 * size),
        _ => format!("n{size}"),
    };
    Cell {
        id: format!("{structure}.{family}.{tag}.{}.x{mult}", jac.id()),
        structure,
        driver,
        family,
        size,
        jac,
        mult,
        exploratory: jac == Jac::JvpFd,
    }
}

fn cells() -> Vec<Cell> {
    use Driver::{Banded, Dense, MatrixFree};
    use Jac::{Analytic, Fd, Jvp, JvpFd};
    let mut all = Vec::new();
    // Dense direct, n in {8, 32, 128, 512, 2048}.
    all.push(cell("dense", Dense, "hires", 8, Analytic, 1));
    all.push(cell("dense", Dense, "hires", 8, Fd, 1));
    all.push(cell("dense", Dense, "bruss1d", 16, Analytic, 1));
    all.push(cell("dense", Dense, "bruss1d", 16, Fd, 1));
    all.push(cell("dense", Dense, "bruss1d", 16, Analytic, 10));
    all.push(cell("dense", Dense, "bruss1d", 16, Analytic, 100));
    for cells in [64, 256, 1024] {
        all.push(cell("dense", Dense, "bruss1d", cells, Analytic, 1));
        all.push(cell("dense", Dense, "bruss1d", cells, Fd, 1));
    }
    all.push(cell("dense", Dense, "radnet", 8, Analytic, 1));
    all.push(cell("dense", Dense, "radnet", 32, Analytic, 1));
    all.push(cell("dense", Dense, "radnet", 32, Fd, 1));
    for jac in [Analytic, Fd] {
        for mult in [1, 10, 100] {
            all.push(cell("dense", Dense, "radnet", 128, jac, mult));
        }
    }
    all.push(cell("dense", Dense, "radnet", 512, Analytic, 1));
    all.push(cell("dense", Dense, "radnet", 512, Fd, 1));
    all.push(cell("dense", Dense, "radnet", 512, Analytic, 10));
    all.push(cell("dense", Dense, "radnet", 512, Analytic, 100));
    all.push(cell("dense", Dense, "radnet", 2048, Analytic, 1));
    all.push(cell("dense", Dense, "radnet", 2048, Fd, 1));
    // Banded direct, 1D method of lines, n in {1e3, 1e4, 1e5}.
    for cells in [500, 5_000, 50_000] {
        all.push(cell("banded_1d", Banded, "bruss1d", cells, Analytic, 1));
        all.push(cell("banded_1d", Banded, "bruss1d", cells, Fd, 1));
    }
    for jac in [Analytic, Fd] {
        for mult in [10, 100] {
            all.push(cell("banded_1d", Banded, "bruss1d", 5_000, jac, mult));
        }
    }
    // 2D method of lines, banded direct.
    all.push(cell("banded_2d", Banded, "ac2d", 32, Analytic, 1));
    all.push(cell("banded_2d", Banded, "ac2d", 64, Analytic, 1));
    all.push(cell("banded_2d", Banded, "ac2d", 64, Fd, 1));
    all.push(cell("banded_2d", Banded, "ac2d", 64, Analytic, 10));
    all.push(cell("banded_2d", Banded, "ac2d", 64, Analytic, 100));
    all.push(cell("banded_2d", Banded, "ac2d", 128, Analytic, 1));
    all.push(cell("banded_2d", Banded, "bruss2d", 32, Analytic, 1));
    all.push(cell("banded_2d", Banded, "bruss2d", 64, Analytic, 1));
    all.push(cell("banded_2d", Banded, "bruss2d", 128, Analytic, 1));
    // 3D method of lines, banded direct.
    all.push(cell("banded_3d", Banded, "ac3d", 8, Analytic, 1));
    all.push(cell("banded_3d", Banded, "ac3d", 16, Analytic, 1));
    all.push(cell("banded_3d", Banded, "ac3d", 16, Fd, 1));
    // 2D method of lines, matrix-free.
    all.push(cell("matrix_free_2d", MatrixFree, "ac2d", 64, Jvp, 1));
    for mult in [1, 10, 100] {
        all.push(cell("matrix_free_2d", MatrixFree, "ac2d", 128, Jvp, mult));
    }
    all.push(cell("matrix_free_2d", MatrixFree, "ac2d", 256, Jvp, 1));
    all.push(cell("matrix_free_2d", MatrixFree, "bruss2d", 128, Jvp, 1));
    // 3D method of lines, matrix-free.
    all.push(cell("matrix_free_3d", MatrixFree, "ac3d", 16, Jvp, 1));
    for mult in [1, 10, 100] {
        all.push(cell("matrix_free_3d", MatrixFree, "ac3d", 32, Jvp, mult));
    }
    // Exploratory, counts only (never timed): the finite-difference JVP, one right-hand side
    // per product.
    all.push(cell("matrix_free_2d", MatrixFree, "ac2d", 128, JvpFd, 1));
    // The two cells with the longest single runs go last, so that the rest of the grid is
    // measured first (the table order is the measurement order).
    let long = [
        "dense.radnet.n2048.fd.x1",
        "banded_2d.bruss2d.g128.analytic.x1",
    ];
    let (mut first, last): (Vec<Cell>, Vec<Cell>) = all
        .into_iter()
        .partition(|c| !long.contains(&c.id.as_str()));
    first.extend(last);
    first
}

fn definition(cell: &Cell) -> ProblemDef {
    match cell.family {
        "hires" => problems::hires(),
        "bruss1d" => problems::bruss1d(cell.size),
        "radnet" => problems::radnet(cell.size),
        "ac2d" => problems::allen_cahn(2, cell.size),
        "ac3d" => problems::allen_cahn(3, cell.size),
        "bruss2d" => problems::bruss2d(cell.size),
        other => panic!("unknown family {other}"),
    }
}

fn find_cell(id: &str) -> Cell {
    cells()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| fail(&format!("unknown cell {id}")))
}

// ---------------------------------------------------------------------------------------------
// The arms of one cell.
// ---------------------------------------------------------------------------------------------

struct Arms {
    def: ProblemDef,
    driver: Driver,
    plain: OdeProblem,
    instrumented: OdeProblem,
    marked: OdeProblem,
    band: Option<BandedJacobian>,
    dense_mode: Option<DenseLuMode>,
    rhs_clock: Arc<RhsClock>,
    mf_clock: Arc<MfClock>,
    /// Right-hand-side evaluations inside a finite-difference Jacobian or JVP.
    fd_calls: Arc<Counter>,
    linear: LinearSolverConfig,
}

fn build(cell: &Cell) -> CoreResult<Arms> {
    let def = definition(cell);
    let n = def.n;
    let rhs = multiplied(def.rhs.clone(), cell.mult);
    let fd_calls = Arc::new(Counter::default());
    let rhs_clock = Arc::new(RhsClock::default());
    let mf_clock = Arc::new(MfClock::new());
    let name = |arm: &str| format!("{} [{arm}]", cell.id);
    let (plain, instrumented, marked, band, dense_mode);
    match cell.driver {
        Driver::Dense => {
            let into: JacobianIntoFn = match cell.jac {
                Jac::Analytic => {
                    let fill = def.dense_fill.clone().expect("dense analytic Jacobian");
                    Arc::new(move |_t, y, out: &mut DenseMatrix| {
                        fill(y, out);
                        Ok(())
                    })
                }
                Jac::Fd => fd_dense(rhs.clone(), n, fd_calls.clone()),
                _ => fail("a dense cell takes an analytic or finite-difference Jacobian"),
            };
            let make = |rhs: RhsFn, arm: &str| -> CoreResult<OdeProblem> {
                let allocating = into.clone();
                Ok(OdeProblem::new(
                    name(arm),
                    n,
                    rhs,
                    None,
                    Some(Arc::new(move |t: f64, y: &[f64]| {
                        let mut matrix = DenseMatrix::zeros(n, n);
                        allocating(t, y, &mut matrix)?;
                        Ok(matrix)
                    })),
                    None,
                    None,
                    true,
                    None,
                    None,
                )?
                .with_jacobian_into(into.clone()))
            };
            plain = make(rhs.clone(), "plain")?;
            instrumented = make(timed_rhs(rhs.clone(), rhs_clock.clone()), "instrumented")?;
            marked = make(marked_rhs(rhs.clone()), "marked")?;
            band = None;
            // The LU the protected policy picks from the first Jacobian, evaluated once here,
            // outside every timed region.
            dense_mode = Some(DenseLuMode::protected_choice_at(
                &plain,
                def.t_span.0,
                &def.y0,
            )?);
        }
        Driver::Banded => {
            let declared = def.band.as_ref().expect("banded problem");
            let fill: BandFillFn = match cell.jac {
                Jac::Analytic => declared.fill.clone(),
                Jac::Fd => fd_band(
                    rhs.clone(),
                    n,
                    declared.lower,
                    declared.upper,
                    fd_calls.clone(),
                ),
                _ => fail("a banded cell takes an analytic or finite-difference Jacobian"),
            };
            band = Some(BandedJacobian {
                lower: declared.lower,
                upper: declared.upper,
                fill,
            });
            let jvp = def.jvp.clone().expect("banded problem with a JVP");
            let make = |rhs: RhsFn, arm: &str| {
                OdeProblem::new(
                    name(arm),
                    n,
                    rhs,
                    None,
                    None,
                    Some(jvp.clone()),
                    None,
                    true,
                    None,
                    None,
                )
            };
            plain = make(rhs.clone(), "plain")?;
            instrumented = make(timed_rhs(rhs.clone(), rhs_clock.clone()), "instrumented")?;
            marked = make(marked_rhs(rhs.clone()), "marked")?;
            dense_mode = None;
        }
        Driver::MatrixFree => {
            let jvp: JvpFn = match cell.jac {
                Jac::Jvp => def.jvp.clone().expect("matrix-free problem with a JVP"),
                Jac::JvpFd => fd_jvp(rhs.clone(), n, fd_calls.clone()),
                _ => fail("a matrix-free cell takes a JVP"),
            };
            let make = |rhs: RhsFn, jvp: JvpFn, arm: &str| {
                OdeProblem::new(
                    name(arm),
                    n,
                    rhs,
                    None,
                    None,
                    Some(jvp),
                    None,
                    true,
                    None,
                    None,
                )
            };
            plain = make(rhs.clone(), jvp.clone(), "plain")?;
            instrumented = make(
                mf_timed_rhs(rhs.clone(), mf_clock.clone()),
                mf_timed_jvp(jvp.clone(), mf_clock.clone()),
                "instrumented",
            )?;
            marked = make(marked_rhs(rhs.clone()), marked_jvp(jvp.clone()), "marked")?;
            band = None;
            dense_mode = None;
        }
    }
    fd_calls.reset();
    Ok(Arms {
        def,
        driver: cell.driver,
        plain,
        instrumented,
        marked,
        band,
        dense_mode,
        rhs_clock,
        mf_clock,
        fd_calls,
        // The linear configuration of the protected router's matrix-free route.
        linear: RoutingOptions::default().matrix_free,
    })
}

// ---------------------------------------------------------------------------------------------
// One run.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arm {
    Plain,
    Instrumented,
    Marked,
}

impl Arm {
    fn id(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Instrumented => "instrumented",
            Self::Marked => "marked",
        }
    }
}

struct RunOut {
    wall_ns: u64,
    attempts: usize,
    accepted: usize,
    rejected: usize,
    reuses: usize,
    success: bool,
    stop_reason: String,
    t_end: f64,
    y_end: Vec<f64>,
    counters: WorkCounters,
    lu: String,
    driver_id: String,
    fd_rhs_calls: u64,
    phases: Value,
}

fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn fnv64(words: impl Iterator<Item = u64>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for word in words {
        for byte in word.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn end_state(
    observed_t: &[f64],
    observed_y: &[Vec<f64>],
    success: bool,
    partial: &Option<DriverPartial>,
) -> (f64, Vec<f64>, String) {
    match (success, partial) {
        (true, _) => (
            *observed_t.last().expect("output"),
            observed_y.last().expect("output").clone(),
            "completed".into(),
        ),
        (false, Some(partial)) => (
            partial.t,
            partial.y.clone(),
            format!("{:?}", partial.reason),
        ),
        (false, None) => fail("a run that did not reach tf has no partial record"),
    }
}

fn from_fast(
    wall_ns: u64,
    result: Rodas5pFastResult,
    partial: Option<DriverPartial>,
    fd_rhs_calls: u64,
    phases: Value,
) -> RunOut {
    let (t_end, y_end, stop_reason) = end_state(
        &result.observed.t,
        &result.observed.y,
        result.observed.success,
        &partial,
    );
    RunOut {
        wall_ns,
        attempts: result.attempts,
        accepted: result.accepted_steps,
        rejected: result.rejected_steps,
        reuses: result.jacobian_reuses,
        success: result.observed.success,
        stop_reason,
        t_end,
        y_end,
        counters: result.observed.counters,
        lu: format!("{:?}", result.lu),
        driver_id: result.driver.into(),
        fd_rhs_calls,
        phases,
    }
}

fn from_mf(wall_ns: u64, result: Rodas5pMfFastResult, fd_rhs_calls: u64, phases: Value) -> RunOut {
    let (t_end, y_end, stop_reason) = end_state(
        &result.observed.t,
        &result.observed.y,
        result.observed.success,
        &result.partial,
    );
    RunOut {
        wall_ns,
        attempts: result.attempts,
        accepted: result.accepted_steps,
        rejected: result.rejected_steps,
        reuses: result.state_reuses,
        success: result.observed.success,
        stop_reason,
        t_end,
        y_end,
        counters: result.observed.counters,
        lu: "MatrixFree".into(),
        driver_id: result.driver.into(),
        fd_rhs_calls,
        phases,
    }
}

fn phase_times_json(times: PhaseTimes, clock: &RhsClock) -> Value {
    json!({
        "linearize_ns": times.linearize_ns,
        "factor_ns": times.factor_ns,
        "solve_ns": times.solve_ns,
        "rhs_ns": clock.ns.get(),
        "linearize_calls": times.linearize_calls,
        "factor_calls": times.factor_calls,
        "solve_calls": times.solve_calls,
        "rhs_calls": clock.calls.get(),
    })
}

fn inner_client(arms: &Arms) -> CoreResult<(Option<DenseLuStageClient>, Option<BandedStageLu>)> {
    let n = arms.def.n;
    Ok(match arms.driver {
        Driver::Dense => (
            Some(DenseLuStageClient::new(
                n,
                arms.dense_mode.expect("dense mode"),
            )?),
            None,
        ),
        Driver::Banded => (
            None,
            Some(BandedStageLu::new(n, arms.band.as_ref().expect("band"))?),
        ),
        Driver::MatrixFree => (None, None),
    })
}

/// One integration of `arm` with the attempt budget `budget`. Everything of the run, including
/// the construction and the release of the stage client, is inside the timed interval, as the
/// protected drivers build and drop their workspace inside the call.
#[unsafe(no_mangle)]
#[inline(never)]
fn v04_region(arms: &Arms, arm: Arm, budget: usize) -> CoreResult<RunOut> {
    let def = &arms.def;
    let n = def.n;
    let adaptive = AdaptiveStepConfig {
        atol: def.atol,
        rtol: def.rtol,
        initial_step: def.initial_step,
        min_step: 1.0e-14,
        max_step: def.t_span.1 - def.t_span.0,
        max_attempts: budget,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![def.t_span.0, def.t_span.1])?;
    let atol = vec![def.atol; n];
    arms.fd_calls.reset();
    arms.rhs_clock.reset();
    match (arms.driver, arm) {
        (Driver::Dense, Arm::Plain) => {
            let start = Instant::now();
            let (result, partial) = integrate_rodas5p_fast_observed_with_atol(
                &arms.plain,
                def.t_span,
                &def.y0,
                &adaptive,
                &output,
                &atol,
            )?;
            let wall = elapsed_ns(start);
            Ok(from_fast(
                wall,
                result,
                partial,
                arms.fd_calls.get(),
                Value::Null,
            ))
        }
        (Driver::Banded, Arm::Plain) => {
            let band = arms.band.as_ref().expect("band");
            let start = Instant::now();
            let run = integrate_rodas5p_fast_banded_observed_with_atol(
                &arms.plain,
                band,
                def.t_span,
                &def.y0,
                &adaptive,
                &output,
                &atol,
            )?;
            let wall = elapsed_ns(start);
            let work = json!({
                "banded_factor_operations": run.work.factor_operations,
                "banded_solve_operations": run.work.solve_operations,
                "banded_stored_slots": run.work.stored_slots,
            });
            Ok(from_fast(
                wall,
                run.fast,
                run.partial,
                arms.fd_calls.get(),
                work,
            ))
        }
        (Driver::Dense | Driver::Banded, Arm::Instrumented) => {
            let start = Instant::now();
            let (client, handle): (Box<dyn StageLinearSystem>, _) = match inner_client(arms)? {
                (Some(dense), _) => {
                    let timed = TimedStageClient::new(dense);
                    let handle = timed.handle();
                    (Box::new(timed), handle)
                }
                (_, Some(banded)) => {
                    let timed = TimedStageClient::new(banded);
                    let handle = timed.handle();
                    (Box::new(timed), handle)
                }
                _ => unreachable!("direct drivers have a client"),
            };
            let (result, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
                &arms.instrumented,
                client,
                def.t_span,
                &def.y0,
                &adaptive,
                &output,
                &atol,
            )?;
            drop(client);
            let wall = elapsed_ns(start);
            let phases = phase_times_json(handle.snapshot(), &arms.rhs_clock);
            Ok(from_fast(
                wall,
                result,
                partial,
                arms.fd_calls.get(),
                phases,
            ))
        }
        (Driver::Dense | Driver::Banded, Arm::Marked) => {
            let start = Instant::now();
            let inner: Box<dyn StageLinearSystem> = match inner_client(arms)? {
                (Some(dense), _) => Box::new(dense),
                (_, Some(banded)) => Box::new(banded),
                _ => unreachable!("direct drivers have a client"),
            };
            let (result, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
                &arms.marked,
                Box::new(MarkedClient { inner }),
                def.t_span,
                &def.y0,
                &adaptive,
                &output,
                &atol,
            )?;
            drop(client);
            let wall = elapsed_ns(start);
            Ok(from_fast(
                wall,
                result,
                partial,
                arms.fd_calls.get(),
                Value::Null,
            ))
        }
        (Driver::MatrixFree, Arm::Plain) => {
            let start = Instant::now();
            let result = integrate_rodas5p_mf_fast_observed_gmres_into(
                &arms.plain,
                def.t_span,
                &def.y0,
                &arms.linear,
                &adaptive,
                &output,
            )?;
            let wall = elapsed_ns(start);
            Ok(from_mf(wall, result, arms.fd_calls.get(), Value::Null))
        }
        (Driver::MatrixFree, Arm::Instrumented) => {
            let clock = &arms.mf_clock;
            let mut observer =
                |_t: f64, _y: &[f64], _h: f64, error: Option<f64>, _w: &Rodas5pMfFastWorkspace| {
                    clock.attempt_done(matches!(error, Some(e) if e <= 1.0));
                };
            let begin = clock.start();
            let result = integrate_rodas5p_mf_fast_observed_traced(
                &arms.instrumented,
                def.t_span,
                &def.y0,
                &arms.linear,
                &adaptive,
                &output,
                true,
                &mut observer,
            )?;
            let wall = clock.finish().saturating_sub(begin);
            let list = |counters: &[Counter; 8]| -> Vec<u64> {
                counters.iter().map(Counter::get).collect()
            };
            let phases = json!({
                "rhs_ns": clock.rhs_ns.get(),
                "rhs_calls": clock.rhs_calls.get(),
                "other_ns": clock.other_ns.get(),
                "solve_interval_ns": list(&clock.solve_interval_ns),
                "jvp_ns": list(&clock.jvp_ns),
                "jvp_calls": list(&clock.jvp_calls),
                "attempts_seen": clock.attempts_seen.get(),
            });
            Ok(from_mf(wall, result, arms.fd_calls.get(), phases))
        }
        (Driver::MatrixFree, Arm::Marked) => {
            let start = Instant::now();
            let result = integrate_rodas5p_mf_fast_observed_traced(
                &arms.marked,
                def.t_span,
                &def.y0,
                &arms.linear,
                &adaptive,
                &output,
                true,
                &mut |_, _, _, _, _| {},
            )?;
            let wall = elapsed_ns(start);
            Ok(from_mf(wall, result, arms.fd_calls.get(), Value::Null))
        }
    }
}

fn run_json(cell: &Cell, arm: Arm, out: &RunOut) -> Value {
    json!({
        "cell": cell.id,
        "arm": arm.id(),
        "wall_ns": out.wall_ns,
        "attempts": out.attempts,
        "accepted": out.accepted,
        "rejected": out.rejected,
        "reuses": out.reuses,
        "success": out.success,
        "stop_reason": out.stop_reason,
        "t_end": out.t_end,
        "t_end_bits": format!("{:016x}", out.t_end.to_bits()),
        "y_end_fnv64": format!("{:016x}", fnv64(out.y_end.iter().map(|v| v.to_bits()))),
        "y_end_finite": out.y_end.iter().all(|v| v.is_finite()),
        "lu": out.lu,
        "driver_id": out.driver_id,
        "fd_rhs_calls": out.fd_rhs_calls,
        "counters": serde_json::to_value(out.counters).expect("counters"),
        "phases": out.phases,
    })
}

/// Whether two runs are the same integration: the step statistics, every counter, the end time
/// and every bit of the end state.
fn identity_json(reference: &RunOut, other: &RunOut) -> Value {
    let checks = [
        ("attempts", reference.attempts == other.attempts),
        ("accepted", reference.accepted == other.accepted),
        ("rejected", reference.rejected == other.rejected),
        ("reuses", reference.reuses == other.reuses),
        ("success", reference.success == other.success),
        ("stop_reason", reference.stop_reason == other.stop_reason),
        (
            "t_end_bits",
            reference.t_end.to_bits() == other.t_end.to_bits(),
        ),
        (
            "y_end_bits",
            reference.y_end.len() == other.y_end.len()
                && reference
                    .y_end
                    .iter()
                    .zip(&other.y_end)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
        ),
        ("counters", reference.counters == other.counters),
        ("fd_rhs_calls", reference.fd_rhs_calls == other.fd_rhs_calls),
    ];
    let identical = checks.iter().all(|(_, ok)| *ok);
    let mut fields = serde_json::Map::new();
    for (name, ok) in checks {
        fields.insert(name.into(), Value::Bool(ok));
    }
    json!({"identical": identical, "fields": fields})
}

// ---------------------------------------------------------------------------------------------
// Host facts and output.
// ---------------------------------------------------------------------------------------------

fn fail(message: &str) -> ! {
    eprintln!("v04-cost-anatomy-runner: {message}");
    std::process::exit(2)
}

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}").expect("stdout");
    out.flush().expect("stdout");
}

fn loadavg() -> Value {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let values: Vec<f64> = text
        .split_whitespace()
        .take(3)
        .filter_map(|v| v.parse().ok())
        .collect();
    json!(values)
}

fn cpus_allowed() -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn unix_time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn cell_json(cell: &Cell, def: &ProblemDef) -> Value {
    json!({
        "id": cell.id,
        "structure": cell.structure,
        "driver": format!("{:?}", cell.driver),
        "family": cell.family,
        "size_parameter": cell.size,
        "n": def.n,
        "jacobian_source": cell.jac.id(),
        "rhs_multiplier": cell.mult,
        "exploratory": cell.exploratory,
        "band_lower": def.band.as_ref().map(|b| b.lower),
        "band_upper": def.band.as_ref().map(|b| b.upper),
        "t_span": [def.t_span.0, def.t_span.1],
        "rtol": def.rtol,
        "atol": def.atol,
        "initial_step": def.initial_step,
        "problem": def.description,
    })
}

fn header(cell: &Cell, arms: &Arms, mode: &str, extra: Value) -> Value {
    json!({
        "type": "header",
        "mode": mode,
        "cell": cell_json(cell, &arms.def),
        "dense_lu_mode": arms.dense_mode.map(|m| format!("{m:?}")),
        "matrix_free_linear_config": (arms.driver == Driver::MatrixFree)
            .then(|| serde_json::to_value(&arms.linear).expect("linear config")),
        "protocol": extra,
        "cpus_allowed_list": cpus_allowed(),
        "loadavg_before": loadavg(),
        "unix_time": unix_time(),
    })
}

// ---------------------------------------------------------------------------------------------
// Subcommands.
// ---------------------------------------------------------------------------------------------

struct Args {
    cell: Option<String>,
    arm: Option<String>,
    attempts: usize,
    warmups: usize,
    reps: usize,
}

fn parse_args(rest: &[String]) -> Args {
    let mut args = Args {
        cell: None,
        arm: None,
        attempts: 0,
        warmups: 2,
        reps: 7,
    };
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .unwrap_or_else(|| fail(&format!("{flag} needs a value")))
                .clone()
        };
        let number = |text: String| -> usize {
            text.parse()
                .unwrap_or_else(|_| fail(&format!("not a number: {text}")))
        };
        match flag.as_str() {
            "--cell" => args.cell = Some(value()),
            "--arm" => args.arm = Some(value()),
            "--attempts" => args.attempts = number(value()),
            "--warmups" => args.warmups = number(value()),
            "--reps" => args.reps = number(value()),
            other => fail(&format!("unknown argument {other}")),
        }
    }
    args
}

fn require_cell(args: &Args) -> Cell {
    let id = args
        .cell
        .as_deref()
        .unwrap_or_else(|| fail("--cell is required"));
    if args.attempts == 0 {
        fail("--attempts is required");
    }
    find_cell(id)
}

fn command_list() {
    let rows: Vec<Value> = cells()
        .iter()
        .map(|cell| cell_json(cell, &definition(cell)))
        .collect();
    emit(json!({"type": "cells", "cells": rows}));
}

/// One run of the protected driver with a small attempt budget: its wall time sizes the
/// attempt budget of the measurement (`../size_cells.py`). No phase is timed.
fn command_pilot(args: &Args) -> CoreResult<()> {
    let cell = require_cell(args);
    let arms = build(&cell)?;
    emit(header(
        &cell,
        &arms,
        "pilot",
        json!({"attempt_budget": args.attempts}),
    ));
    let out = v04_region(&arms, Arm::Plain, args.attempts)?;
    let mut row = run_json(&cell, Arm::Plain, &out);
    row["type"] = json!("pilot_run");
    emit(row);
    emit(
        json!({"type": "footer", "cell": cell.id, "loadavg_after": loadavg(), "unix_time": unix_time()}),
    );
    Ok(())
}

/// The three arms once each, compared bit for bit with the protected driver.
fn command_identity(args: &Args) -> CoreResult<()> {
    let cell = require_cell(args);
    let arms = build(&cell)?;
    emit(header(
        &cell,
        &arms,
        "identity",
        json!({"attempt_budget": args.attempts}),
    ));
    let plain = v04_region(&arms, Arm::Plain, args.attempts)?;
    let instrumented = v04_region(&arms, Arm::Instrumented, args.attempts)?;
    let marked = v04_region(&arms, Arm::Marked, args.attempts)?;
    for (arm, out) in [
        (Arm::Plain, &plain),
        (Arm::Instrumented, &instrumented),
        (Arm::Marked, &marked),
    ] {
        let mut row = run_json(&cell, arm, out);
        row["type"] = json!("identity_run");
        emit(row);
    }
    emit(json!({
        "type": "identity",
        "cell": cell.id,
        "attempt_budget": args.attempts,
        "instrumented_vs_plain": identity_json(&plain, &instrumented),
        "marked_vs_plain": identity_json(&plain, &marked),
    }));
    Ok(())
}

/// The wall protocol: `warmups + reps` repetitions, each one plain run and one instrumented run,
/// the order alternating from repetition to repetition.
fn command_measure(args: &Args) -> CoreResult<()> {
    let cell = require_cell(args);
    let arms = build(&cell)?;
    emit(header(
        &cell,
        &arms,
        "measure",
        json!({
            "attempt_budget": args.attempts,
            "warmups": args.warmups,
            "repetitions": args.reps,
            "arm_order": "plain first on even repetitions, instrumented first on odd repetitions",
        }),
    ));
    let started = Instant::now();
    for rep in 0..args.warmups + args.reps {
        let kind = if rep < args.warmups {
            "warmup"
        } else {
            "timed"
        };
        let order = if rep % 2 == 0 {
            [Arm::Plain, Arm::Instrumented]
        } else {
            [Arm::Instrumented, Arm::Plain]
        };
        let mut outs: Vec<(Arm, RunOut)> = Vec::with_capacity(2);
        for (position, arm) in order.into_iter().enumerate() {
            let out = v04_region(&arms, arm, args.attempts)?;
            let mut row = run_json(&cell, arm, &out);
            row["type"] = json!("run");
            row["rep"] = json!(rep);
            row["kind"] = json!(kind);
            row["position_in_pair"] = json!(position);
            emit(row);
            outs.push((arm, out));
        }
        let plain = &outs
            .iter()
            .find(|(arm, _)| *arm == Arm::Plain)
            .expect("plain")
            .1;
        let instrumented = &outs
            .iter()
            .find(|(arm, _)| *arm == Arm::Instrumented)
            .expect("instrumented")
            .1;
        let mut row = identity_json(plain, instrumented);
        row["type"] = json!("identity");
        row["cell"] = json!(cell.id);
        row["rep"] = json!(rep);
        row["kind"] = json!(kind);
        emit(row);
    }
    emit(json!({
        "type": "footer",
        "cell": cell.id,
        "loadavg_after": loadavg(),
        "elapsed_s": started.elapsed().as_secs_f64(),
        "unix_time": unix_time(),
    }));
    Ok(())
}

/// One arm once (for callgrind and `perf stat`); the arm `none` builds the problem and runs
/// nothing, which gives the set-up cost to subtract.
fn command_single(args: &Args) -> CoreResult<()> {
    let cell = require_cell(args);
    let arm_name = args
        .arm
        .as_deref()
        .unwrap_or_else(|| fail("--arm is required"));
    let arms = build(&cell)?;
    emit(header(
        &cell,
        &arms,
        "single",
        json!({"attempt_budget": args.attempts, "arm": arm_name}),
    ));
    let arm = match arm_name {
        "plain" => Some(Arm::Plain),
        "instrumented" => Some(Arm::Instrumented),
        "marked" => Some(Arm::Marked),
        "none" => None,
        other => fail(&format!("unknown arm {other}")),
    };
    if let Some(arm) = arm {
        let out = v04_region(&arms, arm, args.attempts)?;
        let mut row = run_json(&cell, arm, &out);
        row["type"] = json!("single_run");
        emit(row);
    }
    emit(
        json!({"type": "footer", "cell": cell.id, "loadavg_after": loadavg(), "unix_time": unix_time()}),
    );
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Self-test of the problem definitions and wrappers (no integration).
// ---------------------------------------------------------------------------------------------

fn max_relative_difference(a: &[f64], b: &[f64]) -> f64 {
    let scale = a
        .iter()
        .chain(b)
        .fold(0.0_f64, |acc, v| acc.max(v.abs()))
        .max(f64::MIN_POSITIVE);
    a.iter()
        .zip(b)
        .fold(0.0_f64, |acc, (x, y)| acc.max((x - y).abs()))
        / scale
}

fn band_to_dense(n: usize, lower: usize, upper: usize, band: &[f64]) -> Vec<f64> {
    let jw = lower + upper + 1;
    let mut dense = vec![0.0; n * n];
    for i in 0..n {
        for j in i.saturating_sub(lower)..=(i + upper).min(n - 1) {
            dense[i * n + j] = band[i * jw + (j + lower - i)];
        }
    }
    dense
}

fn command_selftest() -> CoreResult<()> {
    let mut failures = 0;
    let mut check = |name: String, value: f64, limit: f64| {
        let ok = value <= limit;
        if !ok {
            failures += 1;
        }
        emit(json!({"type": "selftest", "check": name, "value": value, "limit": limit, "ok": ok}));
    };
    let cases: Vec<ProblemDef> = vec![
        problems::hires(),
        problems::bruss1d(6),
        problems::radnet(9),
        problems::allen_cahn(2, 5),
        problems::allen_cahn(3, 4),
        problems::bruss2d(4),
    ];
    for def in &cases {
        let n = def.n;
        // A state away from the initial one, so that no term vanishes by symmetry.
        let y: Vec<f64> = def
            .y0
            .iter()
            .enumerate()
            .map(|(i, v)| v + 0.01 * ((i * 7 + 3) % 11) as f64)
            .collect();
        let t0 = def.t_span.0;
        let calls = Arc::new(Counter::default());
        let fd = fd_dense(def.rhs.clone(), n, calls.clone());
        let mut fd_matrix = DenseMatrix::zeros(n, n);
        fd(t0, &y, &mut fd_matrix)?;
        check(
            format!("{}: fd_dense costs n + 1 evaluations", def.family),
            (calls.get() as f64 - (n + 1) as f64).abs(),
            0.0,
        );
        if let Some(fill) = &def.dense_fill {
            let mut analytic = DenseMatrix::zeros(n, n);
            fill(&y, &mut analytic);
            check(
                format!(
                    "{}: dense analytic Jacobian vs forward differences",
                    def.family
                ),
                max_relative_difference(analytic.as_slice(), fd_matrix.as_slice()),
                1.0e-5,
            );
            if let Some(_jvp) = &def.jvp {
                let from_jvp = problems::dense_from_jvp(def, &y)?;
                check(
                    format!("{}: dense analytic Jacobian vs JVP columns", def.family),
                    max_relative_difference(analytic.as_slice(), from_jvp.as_slice()),
                    1.0e-13,
                );
            }
        }
        if let Some(band) = &def.band {
            let jw = band.lower + band.upper + 1;
            let mut analytic = vec![0.0; n * jw];
            (band.fill)(t0, &y, &mut analytic)?;
            let dense = band_to_dense(n, band.lower, band.upper, &analytic);
            let from_jvp = problems::dense_from_jvp(def, &y)?;
            check(
                format!(
                    "{}: band Jacobian vs JVP columns (whole matrix)",
                    def.family
                ),
                max_relative_difference(&dense, from_jvp.as_slice()),
                1.0e-13,
            );
            let calls = Arc::new(Counter::default());
            let fd = fd_band(def.rhs.clone(), n, band.lower, band.upper, calls.clone());
            let mut fd_band_values = vec![0.0; n * jw];
            fd(t0, &y, &mut fd_band_values)?;
            check(
                format!(
                    "{}: fd_band costs min(n, l + u + 1) + 1 evaluations",
                    def.family
                ),
                (calls.get() as f64 - (jw.min(n) + 1) as f64).abs(),
                0.0,
            );
            check(
                format!(
                    "{}: band analytic Jacobian vs grouped forward differences",
                    def.family
                ),
                max_relative_difference(&analytic, &fd_band_values),
                1.0e-5,
            );
        }
        if let Some(jvp) = &def.jvp {
            let v: Vec<f64> = (0..n).map(|i| ((i * 5 + 1) % 7) as f64 - 3.0).collect();
            let (mut exact, mut approx) = (vec![0.0; n], vec![0.0; n]);
            jvp(t0, &y, &v, &mut exact)?;
            let calls = Arc::new(Counter::default());
            let fd = fd_jvp(def.rhs.clone(), n, calls.clone());
            fd(t0, &y, &v, &mut approx)?;
            fd(t0, &y, &v, &mut approx)?;
            check(
                format!("{}: fd_jvp vs analytic JVP", def.family),
                max_relative_difference(&exact, &approx),
                1.0e-5,
            );
            check(
                format!(
                    "{}: fd_jvp costs one evaluation after the first product",
                    def.family
                ),
                (calls.get() as f64 - 3.0).abs(),
                0.0,
            );
        }
        // The multiplier and the clocks forward the right-hand side bit for bit.
        let (mut direct, mut wrapped) = (vec![0.0; n], vec![0.0; n]);
        (def.rhs)(t0, &y, &mut direct)?;
        let clock = Arc::new(RhsClock::default());
        let chain = timed_rhs(marked_rhs(multiplied(def.rhs.clone(), 10)), clock.clone());
        chain(t0, &y, &mut wrapped)?;
        let same = direct
            .iter()
            .zip(&wrapped)
            .all(|(a, b)| a.to_bits() == b.to_bits());
        check(
            format!(
                "{}: multiplier x10, marker and clock keep the right-hand side bits",
                def.family
            ),
            if same && clock.calls.get() == 1 {
                0.0
            } else {
                1.0
            },
            0.0,
        );
    }
    emit(json!({"type": "selftest_summary", "failures": failures}));
    if failures == 0 {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "{failures} self-test checks failed"
        )))
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let Some(command) = argv.get(1) else {
        fail(
            "usage: v04-cost-anatomy-runner <list|selftest|pilot|identity|measure|single> [--cell ID] [--arm ARM] [--attempts N] [--warmups N] [--reps N]",
        );
    };
    let args = parse_args(&argv[2..]);
    let outcome = match command.as_str() {
        "list" => {
            command_list();
            Ok(())
        }
        "selftest" => command_selftest(),
        "pilot" => command_pilot(&args),
        "identity" => command_identity(&args),
        "measure" => command_measure(&args),
        "single" => command_single(&args),
        other => fail(&format!("unknown command {other}")),
    };
    if let Err(error) = outcome {
        emit(json!({"type": "error", "message": error.to_string()}));
        fail(&error.to_string());
    }
}
