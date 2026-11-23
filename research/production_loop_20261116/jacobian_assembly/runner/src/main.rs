//! Standalone runner of loop node V08 (finite-difference Jacobian assembly).
//!
//! `facade-identity`: the sequential modes of the assembly client against
//! vigilode's own finite-difference code (`crates/vigilode/src/fd.rs`),
//! through `vigilode::Rodas5p::solve` with `Jacobian::FiniteDifference`.
//! The client is driven with the configuration the facade passes to the
//! driver (`solve.rs:227-233`: rtol, atol, `initial_step = h0`,
//! `max_attempts = max_steps`, every other field the default) and the
//! output schedule `[t0, t1]`.
//!
//! `facade-coloured`: the facade's coloured route, `Jacobian::sparse_jvp`
//! with `crates/vigilode/src/coloring.rs`, on the 1D Brusselator: colours,
//! JVP calls per Jacobian build, the recovered values against the analytic
//! dense Jacobian, and the solution against the analytic band Jacobian.
//!
//! Every result is one JSON object behind `V08_FACADE_JSON`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, AssemblyStageClient, ColumnRoute, LinearBackend, RhsPurity, SparsityPattern,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
};
use vigilode::{Atol, CallbackError, Jacobian, Options, Problem, Rodas5p, Solution};

#[path = "../../../../../crates/rodas5p-integrators/tests/stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, brusselator, nlkpp_dense};

const RTOL: f64 = 1.0e-6;
const ATOL: f64 = 1.0e-8;
const H0: f64 = 1.0e-6;
const MAX_STEPS: usize = 1_000_000;

fn hex(value: u64) -> String {
    format!("{value:016x}")
}

fn options(n: usize) -> Options {
    Options {
        rtol: RTOL,
        atol: Atol::Vector(vec![ATOL; n]),
        h0: H0,
        max_steps: MAX_STEPS,
        ..Options::default()
    }
}

/// The driver configuration of `vigilode::Rodas5p::solve` for [`options`].
fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        rtol: RTOL,
        atol: ATOL,
        initial_step: H0,
        max_attempts: MAX_STEPS,
        ..AdaptiveStepConfig::default()
    }
}

/// `fd` as a facade problem with the default finite-difference Jacobian.
fn facade_problem(fd: &FdProblem) -> Problem {
    let rhs = Arc::clone(&fd.rhs);
    Problem::new(fd.n, move |t: f64, y: &[f64], dy: &mut [f64]| {
        rhs(t, y, dy).map_err(|error| CallbackError::Fatal(error.to_string()))
    })
    .autonomous()
}

fn stats_field(solution: &Solution, section: &str, field: &str) -> serde_json::Value {
    let stats = serde_json::to_value(&solution.stats).unwrap();
    stats[section][field].clone()
}

fn differing_bits(a: &[f64], b: &[f64]) -> (usize, f64) {
    let count = a
        .iter()
        .zip(b)
        .filter(|(x, y)| x.to_bits() != y.to_bits())
        .count();
    let largest = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max);
    (count + a.len().abs_diff(b.len()), largest)
}

fn fingerprint(values: &[f64]) -> String {
    hex(rodas5p_integrators::stage_clients::assembly::fingerprint(
        rodas5p_integrators::stage_clients::assembly::FINGERPRINT_SEED,
        values,
    ))
}

/// One facade solve with finite differences against the client driver with
/// the sequential (and one threaded) assembly client.
fn facade_identity_case(
    case: &str,
    fd: &FdProblem,
    span: (f64, f64),
    band: Option<(usize, usize)>,
) {
    let n = fd.n;
    let problem = match band {
        Some((lower, upper)) => facade_problem(fd).banded(lower, upper),
        None => facade_problem(fd),
    };
    let solution = Rodas5p::solve(&problem, span, &fd.y0, &options(n)).unwrap();
    let facade_final = solution.y.last().unwrap().clone();
    let (route, backend, backend_name) = match band {
        Some((lower, upper)) => (
            ColumnRoute::Coloured(SparsityPattern::banded(n, lower, upper).unwrap()),
            LinearBackend::BandedProtected { lower, upper },
            "banded_protected".to_string(),
        ),
        None => {
            let mode =
                AssemblyStageClient::protected_dense_mode(&fd.problem, span.0, &fd.y0).unwrap();
            (
                ColumnRoute::Dense,
                LinearBackend::DenseLu(mode),
                format!("dense_lu({mode:?})"),
            )
        }
    };
    let output = OutputSchedule::new(vec![span.0, span.1]).unwrap();
    let atol = vec![ATOL; n];
    let mut arms = Vec::new();
    for workers in [None, Some(3)] {
        let base = match &route {
            ColumnRoute::Dense => AssemblyOptions::dense(),
            ColumnRoute::Coloured(pattern) => AssemblyOptions::coloured(pattern.clone()),
        };
        let assembly = match workers {
            None => base,
            Some(workers) => base.threaded(workers, RhsPurity::pure_and_reentrant()),
        };
        let client = AssemblyStageClient::new(n, backend, assembly).unwrap();
        let handle = client.handle();
        let boxed: Box<dyn StageLinearSystem> = Box::new(client);
        let (result, partial, _) = integrate_rodas5p_fast_client_observed_with_atol(
            &fd.problem,
            boxed,
            span,
            &fd.y0,
            &adaptive(),
            &output,
            &atol,
        )
        .unwrap();
        let client_final = result.observed.y.last().unwrap();
        let (different, largest) = differing_bits(&facade_final, client_final);
        let stats = handle.stats();
        arms.push(serde_json::json!({
            "arm": match workers { None => "seq".to_string(), Some(workers) => format!("t{workers}") },
            "success": result.observed.success && partial.is_none(),
            "attempts": result.attempts,
            "accepted_steps": result.accepted_steps,
            "rejected_steps": result.rejected_steps,
            "builds": stats.builds,
            "assembly_rhs_calls": stats.assembly_rhs_calls,
            "verification_rhs_calls": stats.verification_rhs_calls,
            "final_fingerprint": fingerprint(client_final),
            "final_components_with_different_bits": different,
            "final_largest_absolute_difference": largest,
            "same_steps_as_the_facade": result.attempts == solution.stats.attempts
                && result.accepted_steps == solution.stats.accepted_steps
                && result.rejected_steps == solution.stats.rejected_steps,
            "assembly_calls_equal_facade_fd_calls":
                stats.assembly_rhs_calls == solution.stats.finite_difference.rhs_calls,
            "bit_identical_to_the_facade": different == 0,
        }));
    }
    println!(
        "V08_FACADE_JSON {}",
        serde_json::json!({
            "measurement": "facade_identity",
            "case": case,
            "problem": fd.id,
            "n": n,
            "span": [span.0, span.1],
            "rtol": RTOL,
            "atol": ATOL,
            "facade": {
                "jacobian": "Jacobian::FiniteDifference",
                "band": band.map(|(lower, upper)| [lower, upper]),
                "entry_point": solution.stats.route.entry_point,
                "driver_id": solution.stats.route.driver_id,
                "status": format!("{:?}", solution.status),
                "attempts": solution.stats.attempts,
                "accepted_steps": solution.stats.accepted_steps,
                "rejected_steps": solution.stats.rejected_steps,
                "fd_jacobian_builds": solution.stats.finite_difference.jacobian_builds,
                "fd_rhs_calls": solution.stats.finite_difference.rhs_calls,
                "final_fingerprint": fingerprint(&facade_final),
            },
            "client_backend": backend_name,
            "client_arms": arms,
        })
    );
}

fn facade_identity() {
    facade_identity_case(
        "dense, nonlocal KPP n = 96",
        &nlkpp_dense(96, false),
        (0.0, 2.0),
        None,
    );
    facade_identity_case(
        "dense, Brusselator 32 cells",
        &brusselator(32, false),
        (0.0, 10.0),
        None,
    );
    facade_identity_case(
        "banded (2, 2), Brusselator 64 cells",
        &brusselator(64, false),
        (0.0, 10.0),
        Some((2, 2)),
    );
}

/// The Brusselator's analytic Jacobian entries at `y`: `(row, column, value)`.
fn brusselator_entries(cells: usize, y: &[f64]) -> Vec<(usize, usize, f64)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let mut entries = Vec::new();
    for i in 0..cells {
        let (u, v) = (y[2 * i], y[2 * i + 1]);
        let (a, b) = (2 * i, 2 * i + 1);
        if i > 0 {
            entries.push((a, a - 2, c));
        }
        entries.push((a, a, 2.0 * u * v - 4.0 - 2.0 * c));
        entries.push((a, b, u * u));
        if i + 1 < cells {
            entries.push((a, a + 2, c));
        }
        if i > 0 {
            entries.push((b, b - 2, c));
        }
        entries.push((b, a, 3.0 - 2.0 * u * v));
        entries.push((b, b, -u * u - 2.0 * c));
        if i + 1 < cells {
            entries.push((b, b + 2, c));
        }
    }
    entries
}

/// `out = J(y) v` of the Brusselator, written entry by entry.
fn brusselator_jvp(cells: usize, y: &[f64], v: &[f64], out: &mut [f64]) {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    for i in 0..cells {
        let (u, w) = (y[2 * i], y[2 * i + 1]);
        let (a, b) = (2 * i, 2 * i + 1);
        let mut du = (2.0 * u * w - 4.0 - 2.0 * c) * v[a] + u * u * v[b];
        let mut dw = (3.0 - 2.0 * u * w) * v[a] + (-u * u - 2.0 * c) * v[b];
        if i > 0 {
            du += c * v[a - 2];
            dw += c * v[b - 2];
        }
        if i + 1 < cells {
            du += c * v[a + 2];
            dw += c * v[b + 2];
        }
        out[a] = du;
        out[b] = dw;
    }
}

/// The values `coloring::recover_values` returns for the Jacobian at `y`,
/// compared entry by entry with the analytic Jacobian: `(entries, entries
/// with different bits, largest absolute difference)`.
fn recovered_against_analytic(
    cells: usize,
    pattern: &vigilode::coloring::SparsityPattern,
    y: &[f64],
) -> (usize, usize, f64) {
    let n = 2 * cells;
    let (colors, count) = vigilode::coloring::column_coloring(pattern);
    let mut compressed = vec![vec![0.0; n]; count];
    let mut seed = vec![0.0; n];
    for (k, out) in compressed.iter_mut().enumerate() {
        vigilode::coloring::seed_vector(&colors, k, &mut seed);
        brusselator_jvp(cells, y, &seed, out);
    }
    let mut values = vec![0.0; pattern.nnz()];
    vigilode::coloring::recover_values(pattern, &colors, &compressed, &mut values);
    // The analytic entries in row-major pattern order.
    let mut analytic = brusselator_entries(cells, y);
    analytic.sort_by_key(|&(row, column, _)| (row, column));
    assert_eq!(analytic.len(), values.len());
    let reference: Vec<f64> = analytic.iter().map(|&(_, _, value)| value).collect();
    let (different, largest) = differing_bits(&values, &reference);
    (values.len(), different, largest)
}

fn facade_coloured() {
    let cells = 64;
    let n = 2 * cells;
    let fd = brusselator(cells, false);
    let span = (0.0, 10.0);
    // The true sparsity: the entries of the analytic Jacobian.
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (row, column, _) in brusselator_entries(cells, &fd.y0) {
        rows[row].push(column);
    }
    for row in &mut rows {
        row.sort_unstable();
    }
    let pattern = vigilode::coloring::SparsityPattern::from_rows(n, &rows).unwrap();
    let (colors, colours) = vigilode::coloring::column_coloring(&pattern);
    let valid = vigilode::coloring::coloring_is_valid(&pattern, &colors);

    // The facade solve with the coloured JVP route; the JVP counts its calls.
    let jvp_calls = Arc::new(AtomicU64::new(0));
    let counter = Arc::clone(&jvp_calls);
    let sparse = facade_problem(&fd).jacobian(Jacobian::sparse_jvp(
        pattern.clone(),
        move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            counter.fetch_add(1, Ordering::Relaxed);
            brusselator_jvp(cells, y, v, out);
            Ok(())
        },
    ));
    let coloured = Rodas5p::solve(&sparse, span, &fd.y0, &options(n)).unwrap();
    let jvp_total = jvp_calls.load(Ordering::Relaxed);
    let builds = stats_field(&coloured, "driver_counters", "jacobian_builds");

    // The same solve with the analytic band Jacobian.
    let banded = facade_problem(&fd).jacobian(Jacobian::banded(
        2,
        2,
        move |_t: f64, y: &[f64], band: &mut [f64]| {
            for (row, column, value) in brusselator_entries(cells, y) {
                band[row * 5 + (column + 2 - row)] = value;
            }
            Ok(())
        },
    ));
    let analytic = Rodas5p::solve(&banded, span, &fd.y0, &options(n)).unwrap();
    // And with the facade's grouped finite differences of the same band.
    let finite =
        Rodas5p::solve(&facade_problem(&fd).banded(2, 2), span, &fd.y0, &options(n)).unwrap();

    let coloured_final = coloured.y.last().unwrap();
    let (different, largest) = differing_bits(coloured_final, analytic.y.last().unwrap());
    let (fd_different, fd_largest) = differing_bits(coloured_final, finite.y.last().unwrap());
    let at_start = recovered_against_analytic(cells, &pattern, &fd.y0);
    let at_end = recovered_against_analytic(cells, &pattern, coloured_final);
    println!(
        "V08_FACADE_JSON {}",
        serde_json::json!({
            "measurement": "facade_coloured_route",
            "code_under_test": "vigilode Jacobian::sparse_jvp with crates/vigilode/src/coloring.rs",
            "problem": "1D Brusselator, 64 cells (n = 128), analytic JVP; not a holdout configuration",
            "n": n,
            "span": [span.0, span.1],
            "rtol": RTOL,
            "atol": ATOL,
            "pattern": {
                "nnz": pattern.nnz(),
                "bandwidth": [pattern.bandwidth().0, pattern.bandwidth().1],
                "is_banded": pattern.is_banded(),
                "colours": colours,
                "colouring_is_valid": valid,
                "groups_of_the_full_band_for_comparison": 5,
            },
            "solve_with_sparse_jvp": {
                "entry_point": coloured.stats.route.entry_point,
                "driver_id": coloured.stats.route.driver_id,
                "status": format!("{:?}", coloured.status),
                "attempts": coloured.stats.attempts,
                "accepted_steps": coloured.stats.accepted_steps,
                "rejected_steps": coloured.stats.rejected_steps,
                "jacobian_builds_charged_by_the_driver": builds,
                "jvp_calls_counted_by_the_callback": jvp_total,
                "jvp_calls_per_jacobian_build": builds.as_u64().map(|builds| jvp_total as f64 / builds as f64),
                "jvp_calls_in_stats_counters": stats_field(&coloured, "counters", "jvp_calls"),
                "jvp_calls_in_stats_driver_counters": stats_field(&coloured, "driver_counters", "jvp_calls"),
                "rhs_calls_in_stats_counters": stats_field(&coloured, "counters", "rhs_calls"),
            },
            "recovered_values_against_the_analytic_jacobian": {
                "at_y0": {"entries": at_start.0, "entries_with_different_bits": at_start.1, "largest_absolute_difference": at_start.2},
                "at_the_final_state": {"entries": at_end.0, "entries_with_different_bits": at_end.1, "largest_absolute_difference": at_end.2},
            },
            "final_state_against_the_analytic_band_jacobian": {
                "entry_point": analytic.stats.route.entry_point,
                "attempts": analytic.stats.attempts,
                "accepted_steps": analytic.stats.accepted_steps,
                "components_with_different_bits": different,
                "largest_absolute_difference": largest,
            },
            "final_state_against_the_facade_grouped_finite_differences": {
                "entry_point": finite.stats.route.entry_point,
                "attempts": finite.stats.attempts,
                "accepted_steps": finite.stats.accepted_steps,
                "fd_jacobian_builds": finite.stats.finite_difference.jacobian_builds,
                "fd_rhs_calls": finite.stats.finite_difference.rhs_calls,
                "fd_rhs_calls_per_build": finite.stats.finite_difference.rhs_calls as f64
                    / finite.stats.finite_difference.jacobian_builds as f64,
                "components_with_different_bits": fd_different,
                "largest_absolute_difference": fd_largest,
            },
        })
    );
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    match mode.as_str() {
        "facade-identity" => facade_identity(),
        "facade-coloured" => facade_coloured(),
        _ => {
            eprintln!("usage: v08-assembly-runner facade-identity | facade-coloured");
            std::process::exit(2);
        }
    }
}
