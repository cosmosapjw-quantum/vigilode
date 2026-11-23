//! Measurement program of loop node V08 (finite-difference Jacobian
//! assembly). One ignored test, driven by environment variables, printing
//! one JSON object per line behind a tag; the protocol is frozen in
//! `research/production_loop_20261116/jacobian_assembly/PREDECLARED.json`
//! and the logs are parsed by `build_results.py` of that directory.
//!
//! ```text
//! V08_MODE=identity   V08_CELL=<id>                 every arm and the protected driver, with fingerprints
//! V08_MODE=failclosed                               the negative controls
//! V08_MODE=timing     V08_CELL=<id> [V08_BACKEND=protected] [V08_ARMS=seq,t6]
//!                                                   right-hand-side cost, 2 warm-up + 7 timed rounds
//! V08_MODE=single     V08_CELL=<id> V08_ARM=<arm>   one solve (for perf stat)
//! cargo test --release -p rodas5p-integrators --test stage_clients_assembly_measure -- --ignored --nocapture
//! ```
//!
//! Cells are the problem ids of `PREDECLARED.json`; arms are `seq`, `t1`,
//! `t2`, `t4`, `t6` and, in timing mode, `prot` (the protected driver with
//! the reference finite differences as its Jacobian callback, a
//! diagnostic of the client's own overhead).

use std::hint::black_box;
use std::time::Instant;

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, AssemblyStageClient, CheckColumns, ColumnRoute, LinearBackend, SparsityPattern,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{
    Arm, ClientRun, ClientSpec, FdProblem, ProtectedRun, RunConfig, declared, final_fingerprint,
    final_state, impure, lower_coupled_chain, ring, run_client, run_protected_banded,
    run_protected_dense, upper_bidiagonal,
};

const WARM_UP_ROUNDS: usize = 2;
const TIMED_ROUNDS: usize = 7;
const RHS_BATCHES: usize = 7;
const RHS_BATCH_MIN_NS: u64 = 5_000_000;
const RHS_BATCH_MIN_CALLS: usize = 16;

fn hex(value: u64) -> String {
    format!("{value:016x}")
}

fn loadavg() -> serde_json::Value {
    let text = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let fields: Vec<f64> = text
        .split_whitespace()
        .take(3)
        .filter_map(|field| field.parse().ok())
        .collect();
    serde_json::json!({
        "load_1m": fields.first(),
        "load_5m": fields.get(1),
        "load_15m": fields.get(2),
        "raw": text.trim(),
    })
}

/// The route and the back end of a declared cell.
struct Cell {
    fd: FdProblem,
    route: ColumnRoute,
    backend: LinearBackend,
    backend_name: &'static str,
    /// The band of the protected banded driver; `None`: the dense driver.
    band: Option<(usize, usize)>,
}

fn cell(id: &str, counted: bool, protected_backend: bool) -> Cell {
    let fd = declared(id, counted).unwrap_or_else(|| panic!("unknown cell {id}"));
    let dense = matches!(id, "nlkpp-dense-n400" | "bruss1d-n128-dense");
    if dense {
        assert!(!protected_backend, "the dense cells have one back end");
        let mode =
            AssemblyStageClient::protected_dense_mode(&fd.problem, fd.span.0, &fd.y0).unwrap();
        return Cell {
            route: ColumnRoute::Dense,
            backend: LinearBackend::DenseLu(mode),
            backend_name: match mode {
                rodas5p_integrators::stage_clients::DenseLuMode::InPlace => "dense_lu(in-place)",
                rodas5p_integrators::stage_clients::DenseLuMode::Faer => "dense_lu(faer)",
            },
            band: None,
            fd,
        };
    }
    let (lower, upper) = fd.band.expect("a banded cell");
    let pattern = SparsityPattern::banded(fd.n, lower, upper).unwrap();
    let (backend, backend_name) = if protected_backend {
        (
            LinearBackend::BandedProtected { lower, upper },
            "banded_protected",
        )
    } else {
        (LinearBackend::BandedTight { lower, upper }, "banded_tight")
    };
    Cell {
        route: ColumnRoute::Coloured(pattern),
        backend,
        backend_name,
        band: Some((lower, upper)),
        fd,
    }
}

fn parse_arm(name: &str) -> Arm {
    match name {
        "seq" => Arm::Seq,
        _ => Arm::Threaded(
            name.strip_prefix('t')
                .and_then(|workers| workers.parse().ok())
                .unwrap_or_else(|| panic!("unknown arm {name}")),
        ),
    }
}

fn spec(cell: &Cell, arm: Arm, fingerprints: bool) -> ClientSpec {
    ClientSpec {
        route: cell.route.clone(),
        arm,
        backend: cell.backend,
        first_use_check: CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS),
        fingerprints,
    }
}

fn client_record(cell: &Cell, arm: &str, run: &ClientRun) -> serde_json::Value {
    let (t_end, _) = final_state(&run.result, &run.partial);
    let counters = &run.result.observed.counters;
    serde_json::json!({
        "cell": cell.fd.id,
        "n": cell.fd.n,
        "route": if matches!(cell.route, ColumnRoute::Dense) { "dense" } else { "coloured" },
        "backend": cell.backend_name,
        "arm": arm,
        "wall_ns": run.wall_ns,
        "construct_ns": run.construct_ns,
        "linearize_ns": run.phases.linearize_ns,
        "factor_ns": run.phases.factor_ns,
        "solve_ns": run.phases.solve_ns,
        "linearize_calls": run.phases.linearize_calls,
        "factor_calls": run.phases.factor_calls,
        "solve_calls": run.phases.solve_calls,
        "success": run.result.observed.success,
        "stop_reason": run.partial.as_ref().map(|partial| format!("{:?}", partial.reason)),
        "attempts": run.result.attempts,
        "accepted_steps": run.result.accepted_steps,
        "rejected_steps": run.result.rejected_steps,
        "jacobian_reuses": run.result.jacobian_reuses,
        "t_end": t_end,
        "t_end_bits": hex(t_end.to_bits()),
        "final_fingerprint": hex(final_fingerprint(&run.result, &run.partial)),
        "rhs_calls": counters.rhs_calls,
        "jacobian_builds": counters.jacobian_builds,
        "direct_factorizations": counters.direct_factorizations,
        "direct_solve_calls": counters.direct_solve_calls,
        "colours": run.colours,
        "rhs_per_build": run.rhs_per_build,
        "builds": run.stats.builds,
        "failed_builds": run.stats.failed_builds,
        "assembly_rhs_calls": run.stats.assembly_rhs_calls,
        "verifications": run.stats.verifications,
        "verified_columns": run.stats.verified_columns,
        "verification_rhs_calls": run.stats.verification_rhs_calls,
        "state_fingerprint": hex(run.stats.state_fingerprint),
        "jacobian_fingerprint": hex(run.stats.jacobian_fingerprint),
    })
}

fn protected_record(cell: &Cell, run: &ProtectedRun) -> serde_json::Value {
    let (t_end, _) = final_state(&run.result, &run.partial);
    let counters = &run.result.observed.counters;
    serde_json::json!({
        "cell": cell.fd.id,
        "n": cell.fd.n,
        "route": if cell.band.is_some() { "banded reference finite differences" } else { "dense reference finite differences" },
        "backend": if cell.band.is_some() { "protected banded driver (kernel Indexed)" } else { "protected dense driver" },
        "driver": run.result.driver,
        "lu": format!("{:?}", run.result.lu),
        "arm": "prot",
        "wall_ns": run.wall_ns,
        "success": run.result.observed.success,
        "stop_reason": run.partial.as_ref().map(|partial| format!("{:?}", partial.reason)),
        "attempts": run.result.attempts,
        "accepted_steps": run.result.accepted_steps,
        "rejected_steps": run.result.rejected_steps,
        "jacobian_reuses": run.result.jacobian_reuses,
        "t_end": t_end,
        "t_end_bits": hex(t_end.to_bits()),
        "final_fingerprint": hex(final_fingerprint(&run.result, &run.partial)),
        "rhs_calls_without_jacobian": counters.rhs_calls,
        "jacobian_builds": counters.jacobian_builds,
        "direct_factorizations": counters.direct_factorizations,
        "direct_solve_calls": counters.direct_solve_calls,
        "builds": run.trace.builds,
        "state_fingerprint": hex(run.trace.state_fingerprint),
        "jacobian_fingerprint": hex(run.trace.jacobian_fingerprint),
    })
}

fn run_protected(cell: &Cell, config: &RunConfig) -> ProtectedRun {
    match cell.band {
        Some((lower, upper)) => run_protected_banded(&cell.fd, lower, upper, config).unwrap(),
        None => run_protected_dense(&cell.fd, config).unwrap(),
    }
}

/// Every arm with fingerprints, and the protected driver.
fn identity(id: &str) {
    let config = RunConfig::declared();
    for protected_backend in [false, true] {
        if protected_backend && id != "nlkpp-band-n4096-r24" {
            continue;
        }
        let cell = cell(id, true, protected_backend);
        cell.fd.calls.store(0, std::sync::atomic::Ordering::Relaxed);
        let protected = run_protected(&cell, &config);
        println!("V08_IDENTITY_JSON {}", protected_record(&cell, &protected));
        for arm in ["seq", "t1", "t2", "t4", "t6"] {
            cell.fd.calls.store(0, std::sync::atomic::Ordering::Relaxed);
            let (handle, run) = run_client(&cell.fd, &spec(&cell, parse_arm(arm), true), &config);
            match run {
                Ok(run) => {
                    let mut record = client_record(&cell, arm, &run);
                    record["rhs_calls_counted_by_the_callback"] =
                        serde_json::json!(cell.fd.calls.load(std::sync::atomic::Ordering::Relaxed));
                    println!("V08_IDENTITY_JSON {record}");
                }
                Err(error) => println!(
                    "V08_IDENTITY_JSON {}",
                    serde_json::json!({
                        "cell": cell.fd.id,
                        "backend": cell.backend_name,
                        "arm": arm,
                        "error": error.to_string(),
                        "typed_error": handle.last_error().map(|error| format!("{error:?}")),
                    })
                ),
            }
        }
    }
}

/// The negative controls of the fail-closed discriminator.
fn fail_closed() {
    let config = RunConfig {
        rtol: 1.0e-6,
        atol: 1.0e-8,
        max_attempts: 1_000_000,
    };
    let dense = LinearBackend::DenseLu(rodas5p_integrators::stage_clients::DenseLuMode::InPlace);
    let default_check = CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS);
    let ring12 = ring(12, false, false);
    let chain = lower_coupled_chain(24);
    let impure16 = impure(16);
    let tridiagonal = SparsityPattern::banded(12, 1, 1).unwrap();
    // (control, problem, route, arm, first-use check, must fail)
    let controls: Vec<(&str, &FdProblem, ColumnRoute, Arm, CheckColumns, bool)> = vec![
        (
            "1 omitted coupling reaches a row its colour does not cover",
            &ring12,
            ColumnRoute::Coloured(tridiagonal.clone()),
            Arm::Seq,
            default_check,
            true,
        ),
        (
            "1 omitted coupling reaches a row its colour does not cover",
            &ring12,
            ColumnRoute::Coloured(tridiagonal),
            Arm::Threaded(3),
            default_check,
            true,
        ),
        (
            "2 omitted coupling aliased into a declared entry",
            &chain,
            ColumnRoute::Coloured(upper_bidiagonal(24)),
            Arm::Seq,
            default_check,
            true,
        ),
        (
            "2 omitted coupling aliased into a declared entry",
            &chain,
            ColumnRoute::Coloured(upper_bidiagonal(24)),
            Arm::Threaded(3),
            default_check,
            true,
        ),
        (
            "3 right-hand side that is not a function of its input",
            &impure16,
            ColumnRoute::Dense,
            Arm::Threaded(3),
            default_check,
            true,
        ),
        (
            "3 right-hand side that is not a function of its input",
            &impure16,
            ColumnRoute::Coloured(SparsityPattern::banded(16, 1, 1).unwrap()),
            Arm::Threaded(3),
            default_check,
            true,
        ),
        (
            "4 full check requested with the aliased pattern",
            &chain,
            ColumnRoute::Coloured(upper_bidiagonal(24)),
            Arm::Seq,
            CheckColumns::All,
            true,
        ),
        (
            "counter-example: the aliased pattern without check columns is not noticed",
            &chain,
            ColumnRoute::Coloured(upper_bidiagonal(24)),
            Arm::Seq,
            CheckColumns::None,
            false,
        ),
        (
            "positive control: the true ring pattern with a full check",
            &ring12,
            ColumnRoute::Coloured(problems::ring_pattern(12)),
            Arm::Threaded(3),
            CheckColumns::All,
            false,
        ),
    ];
    for (control, fd, route, arm, check, must_fail) in controls {
        let spec = ClientSpec {
            route,
            arm,
            backend: dense,
            first_use_check: check,
            fingerprints: false,
        };
        let (handle, run) = run_client(fd, &spec, &config);
        let stats = handle.stats();
        let typed = handle.last_error();
        let (outcome, error_kind, message, accepted) = match &run {
            Ok(run) => ("ran", None, None, Some(run.result.accepted_steps)),
            Err(error) => {
                let kind = match error {
                    CoreError::InvalidInput(_) => "InvalidInput",
                    CoreError::NonFinite(_) => "NonFinite",
                    CoreError::Dimension(_) => "Dimension",
                    CoreError::FatalCallback(_) => "FatalCallback",
                    CoreError::LinearSolve(_) => "LinearSolve",
                    CoreError::NonlinearSolve(_) => "NonlinearSolve",
                    CoreError::Coefficients(_) => "Coefficients",
                };
                ("error", Some(kind), Some(error.to_string()), None)
            }
        };
        // The direct check on a fresh client of the same configuration.
        let mut direct = ClientSpec {
            first_use_check: CheckColumns::None,
            ..spec.clone()
        }
        .build(fd.n);
        let direct = direct.verify(
            &fd.problem,
            fd.span.0,
            &fd.y0,
            CheckColumns::All,
            &mut WorkCounters::default(),
        );
        println!(
            "V08_FAILCLOSED_JSON {}",
            serde_json::json!({
                "control": control,
                "problem": fd.id,
                "arm": arm.name(),
                "first_use_check": format!("{check:?}"),
                "must_fail": must_fail,
                "outcome": outcome,
                "driver_error_kind": error_kind,
                "driver_error": message,
                "typed_error": typed.as_ref().map(|error| format!("{error:?}")),
                "typed_error_is_contract_violation": typed.as_ref().map(|error| error.is_contract_violation()),
                "accepted_steps": accepted,
                "completed_builds": stats.builds,
                "failed_builds": stats.failed_builds,
                "direct_full_check": match &direct {
                    Ok(columns) => format!("Ok({columns})"),
                    Err(error) => format!("{error:?}"),
                },
                "direct_full_check_failed": direct.is_err(),
            })
        );
    }
}

/// Wall time per right-hand side at `y0`: one warm-up batch, then
/// [`RHS_BATCHES`] batches of at least [`RHS_BATCH_MIN_NS`].
fn rhs_cost(fd: &FdProblem) -> serde_json::Value {
    let mut out = vec![0.0; fd.n];
    let mut counters = WorkCounters::default();
    let mut batch = |calls: usize| -> u64 {
        let start = Instant::now();
        for _ in 0..calls {
            fd.problem
                .eval_rhs_into(fd.span.0, black_box(&fd.y0), &mut out, &mut counters)
                .unwrap();
            black_box(&out);
        }
        u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    };
    let probe = batch(RHS_BATCH_MIN_CALLS).max(1);
    let per_call = probe as f64 / RHS_BATCH_MIN_CALLS as f64;
    let calls = ((RHS_BATCH_MIN_NS as f64 / per_call).ceil() as usize).max(RHS_BATCH_MIN_CALLS);
    batch(calls);
    let per_call_ns: Vec<f64> = (0..RHS_BATCHES)
        .map(|_| batch(calls) as f64 / calls as f64)
        .collect();
    serde_json::json!({
        "cell": fd.id,
        "n": fd.n,
        "calls_per_batch": calls,
        "batches": RHS_BATCHES,
        "per_call_ns": per_call_ns,
    })
}

fn timing(id: &str) {
    let protected_backend = std::env::var("V08_BACKEND").is_ok_and(|name| name == "protected");
    let arms: Vec<String> = std::env::var("V08_ARMS")
        .unwrap_or_else(|_| "seq,t1,t2,t4,t6,prot".into())
        .split(',')
        .map(str::to_string)
        .collect();
    let config = RunConfig::declared();
    let cell = cell(id, false, protected_backend);
    println!(
        "V08_LOAD_JSON {}",
        serde_json::json!({"cell": id, "backend": cell.backend_name, "when": "before", "loadavg": loadavg()})
    );
    let mut cost = rhs_cost(&cell.fd);
    cost["backend"] = serde_json::json!(cell.backend_name);
    println!("V08_RHS_COST_JSON {cost}");
    for round in 0..WARM_UP_ROUNDS + TIMED_ROUNDS {
        for position in 0..arms.len() {
            let arm = &arms[(round + position) % arms.len()];
            let mut record = if arm == "prot" {
                protected_record(&cell, &run_protected(&cell, &config))
            } else {
                let (_, run) = run_client(&cell.fd, &spec(&cell, parse_arm(arm), false), &config);
                client_record(&cell, arm, &run.unwrap())
            };
            record["round"] = serde_json::json!(round);
            record["position_in_round"] = serde_json::json!(position);
            record["warm_up"] = serde_json::json!(round < WARM_UP_ROUNDS);
            record["backend_cell"] = serde_json::json!(cell.backend_name);
            println!("V08_RUN_JSON {record}");
        }
    }
    println!(
        "V08_LOAD_JSON {}",
        serde_json::json!({"cell": id, "backend": cell.backend_name, "when": "after", "loadavg": loadavg()})
    );
}

fn single(id: &str) {
    let arm = std::env::var("V08_ARM").expect("V08_ARM");
    let protected_backend = std::env::var("V08_BACKEND").is_ok_and(|name| name == "protected");
    let config = RunConfig::declared();
    let cell = cell(id, false, protected_backend);
    let mut record = if arm == "prot" {
        protected_record(&cell, &run_protected(&cell, &config))
    } else {
        let (_, run) = run_client(&cell.fd, &spec(&cell, parse_arm(&arm), false), &config);
        client_record(&cell, &arm, &run.unwrap())
    };
    record["backend_cell"] = serde_json::json!(cell.backend_name);
    println!("V08_SINGLE_JSON {record}");
}

#[test]
#[ignore = "measurement program of loop node V08; run it alone with V08_MODE set"]
fn v08_measure() {
    let mode = std::env::var("V08_MODE").unwrap_or_default();
    let cell = std::env::var("V08_CELL").unwrap_or_default();
    println!(
        "V08_META_JSON {}",
        serde_json::json!({
            "mode": mode,
            "cell": cell,
            "backend_env": std::env::var("V08_BACKEND").ok(),
            "arms_env": std::env::var("V08_ARMS").ok(),
            "arm_env": std::env::var("V08_ARM").ok(),
            "warm_up_rounds": WARM_UP_ROUNDS,
            "timed_rounds": TIMED_ROUNDS,
            "max_attempts": RunConfig::declared().max_attempts,
            "rtol": RunConfig::declared().rtol,
            "atol": RunConfig::declared().atol,
            "cpus_allowed_list": std::fs::read_to_string("/proc/self/status")
                .ok()
                .and_then(|status| {
                    status
                        .lines()
                        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
                        .map(|list| list.trim().to_string())
                }),
        })
    );
    match mode.as_str() {
        "identity" => identity(&cell),
        "failclosed" => fail_closed(),
        "timing" => timing(&cell),
        "single" => single(&cell),
        _ => println!(
            "V08_USAGE set V08_MODE (identity, failclosed, timing, single); see the file header"
        ),
    }
}
