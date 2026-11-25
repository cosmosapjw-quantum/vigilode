//! The default first-use check of the assembly client (loop node V29, owner
//! decision OD-L1-24; `src/stage_clients/assembly.rs`).
//!
//! (i) The default follows the route: the multi-colouring check on the
//! coloured route (dense LU and both band back ends, sequential and
//! threaded), `Sample(8)` on the dense route; every other check stays
//! selectable.
//!
//! (ii) The default check runs once, at the first linearization, with the
//! right-hand sides of its check (`CheckColourings::total_groups` on the
//! coloured route, 8 on the threaded dense route, one more with threads),
//! and changes no bit: the Jacobian and every output of a run equal those of
//! a client without a check, and the counters differ by the right-hand
//! sides of the check alone.
//!
//! (iii) On part of the corpus of node V26 (the declared controls, the
//! periodic ring declared tridiagonal, every single-entry omission of the
//! Brusselator pattern of order 128) the default decides as the explicit
//! multi-colouring check (coloured route) or `Sample(8)` (dense route) and
//! detects every wrong declaration, the 54 that `Sample(8)` passes included.
//!
//! (iv) The limits its documentation states (ledger L-0338, the constructions
//! of `research/production_loop_20261116/assembly_check/review_probe`): a row
//! whose undeclared couplings cancel in pairs (both constructions of the
//! further colourings) and an entry collapse pass the default, while the full
//! check, on the calling thread and on the pool, detects them.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, AssemblyStats, CheckColourings,
    CheckColumns, CheckConstruction, ColumnRoute, LinearBackend, RhsPurity, SparsityPattern,
};
use rodas5p_integrators::{
    OutputSchedule, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{
    FdProblem, Rhs, RunConfig, bits, brusselator, final_state, from_rhs, impure,
    lower_coupled_chain, nlkpp_band, nlkpp_dense, ring, ring_pattern, upper_bidiagonal,
};

const DENSE_IN_PLACE: LinearBackend = LinearBackend::DenseLu(DenseLuMode::InPlace);

/// Sequential, or threaded with this many workers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Execution {
    Sequential,
    Threaded(usize),
}

impl Execution {
    /// Base evaluations a check repeats on the calling thread.
    fn repeated_base(self) -> u64 {
        match self {
            Self::Sequential => 0,
            Self::Threaded(_) => 1,
        }
    }
}

/// The options of `route` and `execution` without a named check: the
/// default first-use check.
fn default_options(route: &ColumnRoute, execution: Execution) -> AssemblyOptions {
    let options = match route {
        ColumnRoute::Dense => AssemblyOptions::dense(),
        ColumnRoute::Coloured(pattern) => AssemblyOptions::coloured(pattern.clone()),
    };
    match execution {
        Execution::Sequential => options,
        Execution::Threaded(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    }
}

fn client(n: usize, backend: LinearBackend, options: AssemblyOptions) -> AssemblyStageClient {
    AssemblyStageClient::new(n, backend, options).unwrap()
}

/// The first linearization of a fresh client at `(t0, y0)`: `Ok` with the
/// statistics, or the typed error of the check.
fn first_use(
    fd: &FdProblem,
    backend: LinearBackend,
    options: AssemblyOptions,
) -> Result<AssemblyStats, AssemblyError> {
    let mut client = client(fd.n, backend, options);
    let handle = client.handle();
    match client.linearize(&fd.problem, fd.span.0, &fd.y0, &mut WorkCounters::default()) {
        Ok(()) => Ok(handle.stats()),
        Err(CoreError::InvalidInput(message)) => {
            let error = handle.last_error().expect("a typed error");
            assert_eq!(message, error.to_string());
            Err(error)
        }
        Err(other) => panic!("{}: not a check failure: {other}", fd.id),
    }
}

/// [`AssemblyStageClient::verify`] at `(t0, y0)` with `check` on a fresh
/// client without a first-use check.
fn explicit(
    fd: &FdProblem,
    route: &ColumnRoute,
    backend: LinearBackend,
    execution: Execution,
    check: CheckColumns,
) -> Result<usize, AssemblyError> {
    let options = default_options(route, execution).with_first_use_check(CheckColumns::None);
    client(fd.n, backend, options).verify(
        &fd.problem,
        fd.span.0,
        &fd.y0,
        check,
        &mut WorkCounters::default(),
    )
}

fn total_groups(route: &ColumnRoute) -> u64 {
    match route {
        ColumnRoute::Coloured(pattern) => {
            CheckColourings::of(pattern).unwrap().total_groups() as u64
        }
        ColumnRoute::Dense => unreachable!("the dense route has no further colourings"),
    }
}

#[test]
fn the_default_first_use_check_follows_the_route() {
    let purity = RhsPurity::pure_and_reentrant();
    let band = SparsityPattern::banded(16, 2, 2).unwrap();
    let coloured = ColumnRoute::Coloured(band.clone());
    assert_eq!(AssemblyOptions::DEFAULT_CHECK_COLUMNS, 8);
    assert_eq!(
        AssemblyOptions::default_first_use_check(&coloured),
        CheckColumns::MultiColouring
    );
    assert_eq!(
        AssemblyOptions::default_first_use_check(&ColumnRoute::Dense),
        CheckColumns::Sample(8)
    );
    // The constructors carry the default of their route; threads and
    // fingerprints keep it.
    for options in [
        AssemblyOptions::coloured(band.clone()),
        AssemblyOptions::coloured(band.clone()).threaded(3, purity),
        AssemblyOptions::coloured(band.clone()).with_fingerprints(),
        AssemblyOptions::coloured(ring_pattern(34)),
    ] {
        assert_eq!(options.first_use_check, CheckColumns::MultiColouring);
    }
    for options in [
        AssemblyOptions::dense(),
        AssemblyOptions::dense().threaded(3, purity),
        AssemblyOptions::dense().with_fingerprints(),
    ] {
        assert_eq!(options.first_use_check, CheckColumns::Sample(8));
    }
    // Every back end of the coloured route (dense LU and both band kernels)
    // takes the default, sequential and threaded; the dense route on the
    // dense LU.
    for backend in [
        DENSE_IN_PLACE,
        LinearBackend::BandedProtected { lower: 2, upper: 2 },
        LinearBackend::BandedTight { lower: 2, upper: 2 },
    ] {
        for execution in [Execution::Sequential, Execution::Threaded(2)] {
            AssemblyStageClient::new(16, backend, default_options(&coloured, execution))
                .unwrap_or_else(|error| panic!("{backend:?} {execution:?}: {error}"));
        }
    }
    for execution in [Execution::Sequential, Execution::Threaded(2)] {
        AssemblyStageClient::new(
            16,
            DENSE_IN_PLACE,
            default_options(&ColumnRoute::Dense, execution),
        )
        .unwrap();
    }
    // Every other check stays selectable: all of them on the coloured route,
    // all but the multi-colouring checks on the dense route.
    for check in [
        CheckColumns::None,
        CheckColumns::Sample(8),
        CheckColumns::All,
        CheckColumns::AllPooled,
        CheckColumns::MultiColouring,
        CheckColumns::MultiColouringPooled,
    ] {
        for execution in [Execution::Sequential, Execution::Threaded(2)] {
            let options = default_options(&coloured, execution).with_first_use_check(check);
            assert_eq!(options.first_use_check, check);
            let backend = LinearBackend::BandedTight { lower: 2, upper: 2 };
            assert!(AssemblyStageClient::new(16, backend, options).is_ok());
            let dense = default_options(&ColumnRoute::Dense, execution).with_first_use_check(check);
            let refused = matches!(
                AssemblyStageClient::new(16, DENSE_IN_PLACE, dense),
                Err(AssemblyError::Configuration(_))
            );
            assert_eq!(
                refused,
                matches!(
                    check,
                    CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled
                ),
                "{check:?}"
            );
        }
    }
}

/// The routes of the default per route: (name, problem, route, back end).
fn routes() -> Vec<(&'static str, FdProblem, ColumnRoute, LinearBackend)> {
    let band = |fd: &FdProblem| {
        let (lower, upper) = fd.band.unwrap();
        SparsityPattern::banded(fd.n, lower, upper).unwrap()
    };
    let bruss = brusselator(16, true);
    let bruss_band = band(&bruss);
    let wide = nlkpp_band(96, 4, true);
    let wide_band = band(&wide);
    vec![
        (
            "coloured, ring pattern, dense LU",
            ring(30, false, true),
            ColumnRoute::Coloured(ring_pattern(30)),
            DENSE_IN_PLACE,
        ),
        (
            "coloured, band (2, 2), dense LU",
            brusselator(16, true),
            ColumnRoute::Coloured(bruss_band.clone()),
            DENSE_IN_PLACE,
        ),
        (
            "banded, protected kernel",
            brusselator(16, true),
            ColumnRoute::Coloured(bruss_band),
            LinearBackend::BandedProtected { lower: 2, upper: 2 },
        ),
        (
            "banded, tight kernel",
            wide,
            ColumnRoute::Coloured(wide_band),
            LinearBackend::BandedTight { lower: 4, upper: 4 },
        ),
        (
            "dense",
            nlkpp_dense(24, true),
            ColumnRoute::Dense,
            DENSE_IN_PLACE,
        ),
        (
            "dense, Brusselator",
            bruss,
            ColumnRoute::Dense,
            DENSE_IN_PLACE,
        ),
    ]
}

#[test]
fn the_default_check_runs_once_at_the_first_linearization_on_every_route() {
    for (name, fd, route, backend) in routes() {
        for execution in [
            Execution::Sequential,
            Execution::Threaded(1),
            Execution::Threaded(3),
        ] {
            let label = format!("{name}, {execution:?}");
            fd.calls.store(0, Ordering::Relaxed);
            let mut checked = client(fd.n, backend, default_options(&route, execution));
            let handle = checked.handle();
            let mut unchecked = client(
                fd.n,
                backend,
                default_options(&route, execution).with_first_use_check(CheckColumns::None),
            );
            let mut counters = WorkCounters::default();
            let mut unchecked_counters = WorkCounters::default();
            let states = [
                fd.y0.clone(),
                fd.y0.iter().map(|y| y * 1.01 + 0.002).collect::<Vec<_>>(),
            ];
            for (step, y) in states.iter().cycle().take(3).enumerate() {
                let t = 0.125 * step as f64;
                checked
                    .linearize(&fd.problem, t, y, &mut counters)
                    .unwrap_or_else(|error| panic!("{label}: {error}"));
                unchecked
                    .linearize(&fd.problem, t, y, &mut unchecked_counters)
                    .unwrap();
                // The check changes no bit of the Jacobian.
                assert_eq!(
                    bits(&checked.jacobian_values().unwrap()),
                    bits(&unchecked.jacobian_values().unwrap()),
                    "{label}: Jacobian of linearization {step}"
                );
            }
            let stats = handle.stats();
            let rhs_per_build = checked.rhs_per_build() as u64;
            assert_eq!(stats.builds, 3, "{label}");
            assert_eq!(stats.assembly_rhs_calls, 3 * rhs_per_build, "{label}");
            let (verifications, columns, check_calls) = match (&route, execution) {
                (ColumnRoute::Coloured(_), _) => (
                    1,
                    fd.n as u64,
                    total_groups(&route) + execution.repeated_base(),
                ),
                (ColumnRoute::Dense, Execution::Sequential) => (0, 0, 0),
                (ColumnRoute::Dense, Execution::Threaded(_)) => (1, 8, 9),
            };
            assert_eq!(
                (
                    stats.verifications,
                    stats.verified_columns,
                    stats.verification_rhs_calls
                ),
                (verifications, columns, check_calls),
                "{label}"
            );
            let total = stats.assembly_rhs_calls + stats.verification_rhs_calls;
            assert_eq!(counters.rhs_calls, total, "{label}: counters");
            assert_eq!(
                unchecked_counters.rhs_calls,
                3 * rhs_per_build,
                "{label}: the client without a check"
            );
            assert_eq!(
                fd.calls.load(Ordering::Relaxed),
                total + unchecked_counters.rhs_calls,
                "{label}: callback"
            );
        }
    }
}

/// The tolerances of the contract tests of node V08: `atol` is `rtol` for
/// the Brusselator and `1e-2 rtol` otherwise.
fn config(fd: &FdProblem, rtol: f64) -> RunConfig {
    let scale = if fd.id.starts_with("bruss1d") {
        1.0
    } else {
        1.0e-2
    };
    RunConfig {
        rtol,
        atol: scale * rtol,
        max_attempts: 1_000_000,
    }
}

/// A whole run of the client driver with `options`; the result and the
/// client's statistics.
fn run(
    fd: &FdProblem,
    backend: LinearBackend,
    options: AssemblyOptions,
    config: &RunConfig,
) -> (
    Rodas5pFastResult,
    Option<rodas5p_integrators::DriverPartial>,
    AssemblyStats,
) {
    let client = client(fd.n, backend, options.with_fingerprints());
    let handle = client.handle();
    let boxed: Box<dyn StageLinearSystem> = Box::new(client);
    let output =
        OutputSchedule::new(vec![fd.span.0, 0.5 * (fd.span.0 + fd.span.1), fd.span.1]).unwrap();
    let (result, partial, _) = integrate_rodas5p_fast_client_observed_with_atol(
        &fd.problem,
        boxed,
        fd.span,
        &fd.y0,
        &config.adaptive(fd.span),
        &output,
        &vec![config.atol; fd.n],
    )
    .unwrap_or_else(|error| panic!("{}: {error}", fd.id));
    (result, partial, handle.stats())
}

#[test]
fn a_run_with_the_default_check_keeps_every_bit() {
    for (name, fd, route, backend) in routes() {
        for execution in [Execution::Sequential, Execution::Threaded(2)] {
            let label = format!("{name}, {execution:?}");
            let config = config(&fd, 1.0e-6);
            let (default, default_partial, default_stats) =
                run(&fd, backend, default_options(&route, execution), &config);
            let (none, none_partial, none_stats) = run(
                &fd,
                backend,
                default_options(&route, execution).with_first_use_check(CheckColumns::None),
                &config,
            );
            assert!(
                default.observed.success && default_partial.is_none(),
                "{label}"
            );
            assert!(default.accepted_steps > 10, "{label}");
            assert_eq!(
                (
                    default.attempts,
                    default.accepted_steps,
                    default.rejected_steps
                ),
                (none.attempts, none.accepted_steps, none.rejected_steps),
                "{label}: steps"
            );
            assert_eq!(bits(&default.observed.t), bits(&none.observed.t), "{label}");
            for (index, (a, b)) in default.observed.y.iter().zip(&none.observed.y).enumerate() {
                assert_eq!(bits(a), bits(b), "{label}: output {index}");
            }
            let (t, y) = final_state(&default, &default_partial);
            let (t_none, y_none) = final_state(&none, &none_partial);
            assert_eq!(
                (t.to_bits(), bits(y)),
                (t_none.to_bits(), bits(y_none)),
                "{label}: last state"
            );
            assert_eq!(
                (
                    default_stats.builds,
                    default_stats.assembly_rhs_calls,
                    default_stats.state_fingerprint,
                    default_stats.jacobian_fingerprint
                ),
                (
                    none_stats.builds,
                    none_stats.assembly_rhs_calls,
                    none_stats.state_fingerprint,
                    none_stats.jacobian_fingerprint
                ),
                "{label}: linearization points and Jacobians"
            );
            // The counters differ by the right-hand sides of the check alone.
            let check = default_stats.verification_rhs_calls;
            assert_eq!(none_stats.verification_rhs_calls, 0, "{label}");
            let mut counters = default.observed.counters;
            counters.rhs_calls -= check;
            counters.rhs_evaluations -= check;
            assert_eq!(counters, none.observed.counters, "{label}: counters");
        }
    }
}

/// The true pattern of the Brusselator of `cells` cells (unknowns
/// interleaved), as in the corpus of node V26.
fn brusselator_rows(cells: usize) -> Vec<Vec<usize>> {
    let n = 2 * cells;
    let mut rows = vec![Vec::new(); n];
    for i in 0..cells {
        let (u, v) = (2 * i, 2 * i + 1);
        if i > 0 {
            rows[u].push(u - 2);
            rows[v].push(v - 2);
        }
        rows[u].extend([u, v]);
        rows[v].extend([u, v]);
        if i + 1 < cells {
            rows[u].push(u + 2);
            rows[v].push(v + 2);
        }
    }
    for row in &mut rows {
        row.sort_unstable();
    }
    rows
}

#[test]
fn the_default_decides_as_the_multi_colouring_check_on_part_of_the_v26_corpus() {
    // (problem, declared route, back end, executions, wrong declaration)
    type Case = (
        String,
        Arc<FdProblem>,
        ColumnRoute,
        LinearBackend,
        Vec<Execution>,
        bool,
    );
    let mut cases: Vec<Case> = Vec::new();
    let both = vec![Execution::Sequential, Execution::Threaded(3)];
    let ring12 = Arc::new(ring(12, false, false));
    cases.push((
        "control 1: ring 12 declared tridiagonal".into(),
        ring12,
        ColumnRoute::Coloured(SparsityPattern::banded(12, 1, 1).unwrap()),
        DENSE_IN_PLACE,
        both.clone(),
        true,
    ));
    cases.push((
        "control 2: chain 24 declared upper bidiagonal".into(),
        Arc::new(lower_coupled_chain(24)),
        ColumnRoute::Coloured(upper_bidiagonal(24)),
        DENSE_IN_PLACE,
        both.clone(),
        true,
    ));
    let impure16 = Arc::new(impure(16));
    cases.push((
        "control 3: impure, tridiagonal".into(),
        Arc::clone(&impure16),
        ColumnRoute::Coloured(SparsityPattern::banded(16, 1, 1).unwrap()),
        DENSE_IN_PLACE,
        vec![Execution::Threaded(3)],
        true,
    ));
    cases.push((
        "control 3: impure, dense".into(),
        impure16,
        ColumnRoute::Dense,
        DENSE_IN_PLACE,
        vec![Execution::Threaded(3)],
        true,
    ));
    for n in [12, 13, 14, 15, 16, 32, 33, 34, 64, 100, 127, 128, 1000] {
        cases.push((
            format!("ring {n} declared tridiagonal"),
            Arc::new(ring(n, false, false)),
            ColumnRoute::Coloured(SparsityPattern::banded(n, 1, 1).unwrap()),
            DENSE_IN_PLACE,
            vec![Execution::Sequential],
            true,
        ));
    }
    let bruss = Arc::new(brusselator(64, false));
    let rows = brusselator_rows(64);
    for (i, row) in rows.iter().enumerate() {
        for &k in row {
            let mut wrong = rows.clone();
            wrong[i].retain(|&column| column != k);
            cases.push((
                format!("Brusselator 128 without ({i}, {k})"),
                Arc::clone(&bruss),
                ColumnRoute::Coloured(SparsityPattern::from_rows(128, &wrong).unwrap()),
                LinearBackend::BandedTight { lower: 2, upper: 2 },
                vec![Execution::Sequential],
                true,
            ));
        }
    }
    cases.push((
        "ring 30, true pattern".into(),
        Arc::new(ring(30, false, false)),
        ColumnRoute::Coloured(ring_pattern(30)),
        DENSE_IN_PLACE,
        both.clone(),
        false,
    ));
    cases.push((
        "Brusselator 128, true pattern".into(),
        bruss,
        ColumnRoute::Coloured(SparsityPattern::from_rows(128, &rows).unwrap()),
        LinearBackend::BandedTight { lower: 2, upper: 2 },
        both,
        false,
    ));
    assert_eq!(cases.len(), 4 + 13 + 508 + 2);

    let (mut decisions, mut default_misses, mut sample_misses) = (0, 0, 0);
    for (label, fd, route, backend, executions, wrong) in &cases {
        for &execution in executions {
            let label = format!("{label}, {execution:?}");
            let default = first_use(fd, *backend, default_options(route, execution));
            let reference = match route {
                ColumnRoute::Coloured(_) => CheckColumns::MultiColouring,
                ColumnRoute::Dense => CheckColumns::Sample(8),
            };
            let explicit_decision = explicit(fd, route, *backend, execution, reference);
            match (&default, &explicit_decision) {
                (Ok(stats), Ok(columns)) => {
                    assert_eq!(stats.verified_columns, *columns as u64, "{label}")
                }
                (Err(error), Err(expected)) => assert_eq!(error, expected, "{label}"),
                _ => panic!("{label}: default {default:?}, explicit {explicit_decision:?}"),
            }
            decisions += 1;
            if *wrong {
                if let Err(error) = &default {
                    assert!(error.is_contract_violation(), "{label}: {error:?}");
                } else {
                    default_misses += 1;
                }
                let sampled = explicit(fd, route, *backend, execution, CheckColumns::Sample(8));
                if sampled.is_ok() {
                    sample_misses += 1;
                }
            } else {
                assert!(default.is_ok(), "{label}: a correct pattern is refused");
            }
        }
    }
    assert_eq!(decisions, 2 + 2 + 1 + 1 + 13 + 508 + 2 + 2);
    assert_eq!(default_misses, 0, "wrong declarations the default passes");
    // The rings of order 34, 64, 100, 127, 128 and 1000 and 48 omissions of
    // order 128 pass Sample(8) (node V26).
    assert_eq!(sample_misses, 6 + 48, "wrong declarations Sample(8) passes");
}

/// A right-hand side whose rows read the declared columns of `rows` through
/// `w y_j (1 + y_j / 10)`, and `row` in addition `extra(y)`.
fn generic_rows(
    rows: &[Vec<usize>],
    row: usize,
    extra: impl Fn(&[f64]) -> f64 + Send + Sync + 'static,
) -> impl Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync + 'static {
    let weights: Vec<Vec<(usize, f64)>> = rows
        .iter()
        .enumerate()
        .map(|(i, columns)| {
            columns
                .iter()
                .map(|&j| (j, 0.5 + 0.37 * ((3 * i + 7 * j) % 11) as f64 / 11.0))
                .collect()
        })
        .collect();
    move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        for (i, (value, columns)) in out.iter_mut().zip(&weights).enumerate() {
            let mut acc = 0.0;
            for &(j, w) in columns {
                acc += w * y[j] * (1.0 + 0.1 * y[j]);
            }
            if i == row {
                acc += extra(y);
            }
            *value = acc;
        }
        Ok(())
    }
}

fn band_rows(n: usize, lower: usize, upper: usize) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| (i.saturating_sub(lower)..=(i + upper).min(n - 1)).collect())
        .collect()
}

fn ring_rows(n: usize) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| {
            let mut row = vec![(i + n - 1) % n, i, (i + 1) % n];
            row.sort_unstable();
            row.dedup();
            row
        })
        .collect()
}

/// The decisions of the default, `Sample(8)`, the full check and the pooled
/// full check on `fd` declared by `rows`.
fn limit_case(fd: &FdProblem, rows: &[Vec<usize>]) -> [Result<usize, AssemblyError>; 4] {
    let pattern = SparsityPattern::from_rows(fd.n, rows).unwrap();
    let route = ColumnRoute::Coloured(pattern);
    let default = first_use(
        fd,
        DENSE_IN_PLACE,
        default_options(&route, Execution::Sequential),
    )
    .map(|stats| stats.verified_columns as usize);
    let sequential = |check| explicit(fd, &route, DENSE_IN_PLACE, Execution::Sequential, check);
    [
        default,
        sequential(CheckColumns::Sample(8)),
        sequential(CheckColumns::All),
        explicit(
            fd,
            &route,
            DENSE_IN_PLACE,
            Execution::Threaded(3),
            CheckColumns::AllPooled,
        ),
    ]
}

#[test]
fn the_limits_stated_for_the_default_are_real() {
    // Pairwise cancelling undeclared couplings (review probe `cancel`),
    // construction B: tridiagonal declaration of order 60, row 30 also reads
    // (y0 - y45) + (y1 - y16) with the four values equal at the checked
    // state; {0, 45} and {1, 16} share their groups mod 3 and mod 5, {0, 16}
    // and {45, 1} mod 4.
    let n = 60;
    let rows = band_rows(n, 1, 1);
    let plan = CheckColourings::of(&SparsityPattern::from_rows(n, &rows).unwrap()).unwrap();
    assert!(matches!(
        plan.construction(),
        CheckConstruction::Band { moduli, .. } if moduli == &[3, 4, 5]
    ));
    let rhs: Rhs = Arc::new(generic_rows(&rows, 30, |y| (y[0] - y[45]) + (y[1] - y[16])));
    let mut y0: Vec<f64> = (0..n).map(|i| 0.5 + 0.013 * i as f64).collect();
    for j in [0, 45, 1, 16] {
        y0[j] = 0.7;
    }
    let fd = from_rhs("cancel-B".into(), rhs, y0, (0.0, 1.0), None);
    let [default, sample, full, pooled] = limit_case(&fd, &rows);
    assert_eq!(default, Ok(n), "the default passes the cancelling row");
    assert_eq!(sample, Ok(8), "Sample(8) passes it too");
    let expected = Err(AssemblyError::UndeclaredEntry { row: 30, column: 0 });
    assert_eq!(full, expected, "the full check detects it");
    assert_eq!(pooled, expected, "the pooled full check detects it");

    // The same with construction G: the true ring of order 84 (classes of
    // 28 columns, primes 2, 3, 5), row 40 also reads (y0 - y30) + (y3 - y63).
    let n = 84;
    let rows = ring_rows(n);
    let plan = CheckColourings::of(&SparsityPattern::from_rows(n, &rows).unwrap()).unwrap();
    assert!(matches!(
        plan.construction(),
        CheckConstruction::Refinement { primes } if primes == &[2, 3, 5]
    ));
    let rhs: Rhs = Arc::new(generic_rows(&rows, 40, |y| (y[0] - y[30]) + (y[3] - y[63])));
    let mut y0: Vec<f64> = (0..n).map(|i| 0.5 + 0.011 * i as f64).collect();
    for j in [0, 30, 3, 63] {
        y0[j] = 0.7;
    }
    let fd = from_rhs("cancel-G".into(), rhs, y0, (0.0, 1.0), None);
    let [default, sample, full, pooled] = limit_case(&fd, &rows);
    assert_eq!(default, Ok(n), "the default passes the cancelling row");
    assert_eq!(sample, Ok(8), "Sample(8) passes it too");
    let expected = Err(AssemblyError::UndeclaredEntry { row: 40, column: 0 });
    assert_eq!(full, expected, "the full check detects it");
    assert_eq!(pooled, expected, "the pooled full check detects it");

    // Entry collapse (review probe `collapse`, its first hit, trial 10):
    // tridiagonal declaration of order 60, row 20 is
    // c0 (y20 - s20) + c1 (y21 - s21) + 0.75 (y19 - s19) + d (y41 - s41) with
    // d about one ulp of the row; the row's bits change with y41, but every
    // entry the default compares rounds to the same number.
    let n = 60;
    let rows = band_rows(n, 1, 1);
    let (c0, c1, d) = (
        1.014_684_066_151_021_5,
        1.003_831_640_050_302_7,
        2.852_333_095_691_641e-16,
    );
    let mut s: Vec<f64> = (0..n).map(|i| 0.6 + 0.017 * i as f64).collect();
    s[20] = 1.608_181_549_847_968_4;
    s[21] = 1.613_004_690_001_616_8;
    s[19] = 1.3;
    s[41] = 1.0;
    let start = s.clone();
    let row20 = move |y: &[f64]| {
        let mut acc = c0 * (y[20] - start[20]);
        acc += c1 * (y[21] - start[21]);
        acc += 0.75 * (y[19] - start[19]);
        acc += d * (y[41] - start[41]);
        acc
    };
    let generic = generic_rows(&rows, usize::MAX, |_| 0.0);
    let rhs: Rhs = Arc::new(
        move |t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            generic(t, y, out)?;
            out[20] = row20(y);
            Ok(())
        },
    );
    let fd = from_rhs("collapse".into(), rhs, s, (0.0, 1.0), None);
    let [default, sample, full, pooled] = limit_case(&fd, &rows);
    assert_eq!(default, Ok(n), "the default passes the collapsed entry");
    let expected = Err(AssemblyError::UndeclaredEntry {
        row: 20,
        column: 41,
    });
    assert_eq!(sample, expected, "column 41 is one of the 8 samples");
    assert_eq!(full, expected, "the full check detects it");
    assert_eq!(pooled, expected, "the pooled full check detects it");
}
