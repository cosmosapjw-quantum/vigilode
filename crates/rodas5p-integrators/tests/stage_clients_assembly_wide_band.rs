//! The default first-use check of the coloured route on wide declared
//! patterns (loop node V31, owner decision OD-L1-27;
//! `src/stage_clients/assembly.rs`): `CheckColumns::All` where
//! `CheckColourings::total_groups` of the declared pattern is at least `n`,
//! `CheckColumns::MultiColouring` otherwise; `Sample(8)` on the dense route.
//!
//! (i) The rule: a wide declared pattern (`total_groups > n`) and the
//! equality case (`total_groups == n`) pick `All`; a narrow band, the boundary
//! `total_groups == n - 1` and a declared dense pattern (no further
//! colourings) keep `MultiColouring`; the dense route keeps `Sample(8)`.
//! `CheckColourings::total_groups_of` equals the `total_groups` of the built
//! colourings on every pattern tried, and the default reads nothing else.
//!
//! (ii) A client whose default became `All` runs it once, at the first
//! linearization, with `n` right-hand sides (one more with threads) where the
//! multi-colouring check would spend `total_groups`, and changes no bit of the
//! Jacobian.
//!
//! (iii) On every single-entry omission of a wide random pattern the default
//! decides as the explicit check the rule names for the declaration (the full
//! check on nearly all of them) and detects the omission; the explicit full
//! and multi-colouring checks detect it too.
//!
//! (iv) Every check stays selectable on a wide pattern: as the first-use check,
//! through the handle and through `verify`.

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::StageLinearSystem;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, CheckColourings, CheckColumns,
    CheckConstruction, ColumnRoute, LinearBackend, RhsPurity, SparsityPattern,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, Rhs, bits, from_rhs, nlkpp_band, nlkpp_dense, ring, ring_pattern};

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

/// The options of `pattern` and `execution` without a named check.
fn default_options(pattern: &SparsityPattern, execution: Execution) -> AssemblyOptions {
    let options = AssemblyOptions::coloured(pattern.clone());
    match execution {
        Execution::Sequential => options,
        Execution::Threaded(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The random banded rows of the recording program of node V31
/// (`stage_clients_assembly_wide_band_record.rs`): the diagonal and every
/// `(i, j)` with `0 < |i - j| <= half_width` drawn with probability `density`.
fn random_band_rows(n: usize, half_width: usize, density: f64, seed: u64) -> Vec<Vec<usize>> {
    let uniform = |i: usize, j: usize| {
        let key = ((i as u64) << 32) | j as u64;
        (splitmix64(seed ^ splitmix64(key)) >> 11) as f64 / (1_u64 << 53) as f64
    };
    (0..n)
        .map(|i| {
            (i.saturating_sub(half_width)..=(i + half_width).min(n - 1))
                .filter(|&j| j == i || uniform(i, j) < density)
                .collect()
        })
        .collect()
}

/// The wide pattern of the review of node V29 and of the survey of node V31:
/// order 60, half-width 27, density 0.1, seed 1; `total_groups` 111.
fn wide_rows() -> Vec<Vec<usize>> {
    random_band_rows(60, 27, 0.1, 1)
}

fn pattern(rows: &[Vec<usize>]) -> SparsityPattern {
    SparsityPattern::from_rows(rows.len(), rows).unwrap()
}

fn band(n: usize, lower: usize, upper: usize) -> SparsityPattern {
    SparsityPattern::banded(n, lower, upper).unwrap()
}

/// A problem whose row `i` is the sum over the columns `rows[i]` of
/// `w_ij y_j (1 + y_j / 10)` (the right-hand side of the recording program).
fn generic_problem(id: &str, rows: &[Vec<usize>]) -> FdProblem {
    let n = rows.len();
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
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for (value, columns) in out.iter_mut().zip(&weights) {
                let mut acc = 0.0;
                for &(j, w) in columns {
                    acc += w * y[j] * (1.0 + 0.1 * y[j]);
                }
                *value = acc;
            }
            Ok(())
        },
    );
    let y0 = (0..n)
        .map(|j| 0.75 + 0.25 * (0.61 * j as f64 + 0.3).sin())
        .collect();
    from_rhs(id.into(), rhs, y0, (0.0, 1.0), None)
}

fn total_groups(pattern: &SparsityPattern) -> usize {
    CheckColourings::of(pattern).unwrap().total_groups()
}

/// What the rule gives for `pattern`.
fn rule(pattern: &SparsityPattern) -> CheckColumns {
    if total_groups(pattern) >= pattern.n() {
        CheckColumns::All
    } else {
        CheckColumns::MultiColouring
    }
}

/// The default of `pattern` through every constructor path.
fn assert_default(pattern: &SparsityPattern, expected: CheckColumns, label: &str) {
    let purity = RhsPurity::pure_and_reentrant();
    assert_eq!(
        CheckColourings::total_groups_of(pattern),
        total_groups(pattern),
        "{label}: total_groups_of"
    );
    assert_eq!(
        AssemblyOptions::default_first_use_check(&ColumnRoute::Coloured(pattern.clone())),
        expected,
        "{label}"
    );
    for options in [
        AssemblyOptions::coloured(pattern.clone()),
        AssemblyOptions::coloured(pattern.clone()).threaded(3, purity),
        AssemblyOptions::coloured(pattern.clone()).with_fingerprints(),
    ] {
        assert_eq!(options.first_use_check, expected, "{label}");
    }
}

#[test]
fn the_default_of_the_coloured_route_follows_total_groups() {
    // A wide declared pattern: more groups than columns, the full check.
    let wide = pattern(&wide_rows());
    let plan = CheckColourings::of(&wide).unwrap();
    assert_eq!(wide.bandwidth(), (27, 27));
    assert!(matches!(
        plan.construction(),
        CheckConstruction::Band { moduli, .. } if moduli == &[55, 56]
    ));
    assert_eq!(plan.total_groups(), 111);
    assert_default(&wide, CheckColumns::All, "wide random band, 111 > 60");

    // The equality case total_groups == n, both constructions: the full
    // check (the ruling of node V31: at equality it costs as much and is
    // exact).
    for (declared, total, band_construction, label) in [
        (ring_pattern(16), 16, true, "ring 16"),
        (ring_pattern(30), 30, true, "ring 30 (tie of the totals)"),
        (band(4, 1, 1), 4, false, "band (1, 1) of order 4"),
        (band(60, 15, 15), 60, false, "band (15, 15) of order 60"),
    ] {
        let plan = CheckColourings::of(&declared).unwrap();
        assert_eq!(
            (plan.total_groups(), declared.n()),
            (total, total),
            "{label}"
        );
        assert_eq!(
            matches!(plan.construction(), CheckConstruction::Band { .. }),
            band_construction,
            "{label}"
        );
        assert_default(&declared, CheckColumns::All, label);
    }

    // Fewer groups than columns: the multi-colouring check stays, at the
    // boundary total_groups == n - 1 too.
    for (declared, total, label) in [
        (ring_pattern(34), 33, "ring 34, total_groups n - 1"),
        (band(16, 2, 2), 11, "band (2, 2) of order 16"),
        (band(12, 1, 1), 7, "tridiagonal of order 12"),
        (band(4096, 24, 24), 150, "band (24, 24) of order 4096"),
        (band(131_072, 2, 2), 59, "band (2, 2) of order 131072"),
    ] {
        assert_eq!(total_groups(&declared), total, "{label}");
        assert_default(&declared, CheckColumns::MultiColouring, label);
    }

    // A declared dense pattern: its own colouring shifts every column alone,
    // so the multi-colouring check has no further colouring to evaluate and
    // stays the default at no cost.
    let dense = band(8, 7, 7);
    assert_eq!(total_groups(&dense), 0);
    assert_default(&dense, CheckColumns::MultiColouring, "declared dense");

    // The dense route keeps Sample(8).
    let purity = RhsPurity::pure_and_reentrant();
    assert_eq!(
        AssemblyOptions::default_first_use_check(&ColumnRoute::Dense),
        CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS)
    );
    assert_eq!(AssemblyOptions::DEFAULT_CHECK_COLUMNS, 8);
    for options in [
        AssemblyOptions::dense(),
        AssemblyOptions::dense().threaded(3, purity),
        AssemblyOptions::dense().with_fingerprints(),
    ] {
        assert_eq!(options.first_use_check, CheckColumns::Sample(8));
    }

    // total_groups_of equals the built colourings, and the default follows
    // the rule, on random banded patterns of both sides of the threshold.
    let (mut all, mut multi) = (0, 0);
    for n in [24, 60, 97] {
        for fraction in [0.05, 0.15, 0.25, 0.35, 0.45, 0.5, 0.75] {
            for density in [0.05, 0.1, 0.3, 1.0] {
                for seed in [1, 2] {
                    let width = ((fraction * n as f64).round() as usize).clamp(1, n - 1);
                    let declared = pattern(&random_band_rows(n, width, density, seed));
                    let label = format!("n {n}, half-width {width}, density {density}, {seed}");
                    let expected = rule(&declared);
                    assert_default(&declared, expected, &label);
                    if expected == CheckColumns::All {
                        all += 1;
                    } else {
                        multi += 1;
                    }
                }
            }
        }
    }
    println!(
        "STAGE_CLIENTS_ASSEMBLY_WIDE_BAND_JSON {}",
        serde_json::json!({
            "test": "the_default_of_the_coloured_route_follows_total_groups",
            "random_patterns_full_check": all,
            "random_patterns_multi_colouring": multi,
        })
    );
    assert!(all > 20 && multi > 20, "both sides tried: {all} / {multi}");
}

/// Clients of the default options whose default is the full check: (name,
/// problem, declared pattern, back end).
fn full_default_clients() -> Vec<(&'static str, FdProblem, SparsityPattern, LinearBackend)> {
    let rows = wide_rows();
    vec![
        (
            "wide random band, dense LU",
            generic_problem("wide-n60", &rows),
            pattern(&rows),
            DENSE_IN_PLACE,
        ),
        (
            "ring 30, dense LU",
            ring(30, false, false),
            ring_pattern(30),
            DENSE_IN_PLACE,
        ),
        (
            "band (15, 15) of order 60, tight kernel",
            nlkpp_band(60, 15, false),
            band(60, 15, 15),
            LinearBackend::BandedTight {
                lower: 15,
                upper: 15,
            },
        ),
        (
            "band (15, 15) of order 60, protected kernel",
            nlkpp_band(60, 15, false),
            band(60, 15, 15),
            LinearBackend::BandedProtected {
                lower: 15,
                upper: 15,
            },
        ),
    ]
}

#[test]
fn a_client_whose_default_became_the_full_check_runs_it_once() {
    for (name, fd, declared, backend) in full_default_clients() {
        let n = fd.n as u64;
        let groups = total_groups(&declared) as u64;
        assert!(groups >= n, "{name}");
        for execution in [
            Execution::Sequential,
            Execution::Threaded(1),
            Execution::Threaded(3),
        ] {
            let label = format!("{name}, {execution:?}");
            let options = default_options(&declared, execution);
            assert_eq!(options.first_use_check, CheckColumns::All, "{label}");
            let mut checked = AssemblyStageClient::new(fd.n, backend, options.clone()).unwrap();
            let handle = checked.handle();
            let mut unchecked = AssemblyStageClient::new(
                fd.n,
                backend,
                options.with_first_use_check(CheckColumns::None),
            )
            .unwrap();
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
            // One check, at the first linearization: every column alone, n
            // right-hand sides where the multi-colouring check spends
            // total_groups >= n.
            assert_eq!(
                (
                    stats.verifications,
                    stats.verified_columns,
                    stats.verification_rhs_calls
                ),
                (1, n, n + execution.repeated_base()),
                "{label}"
            );
            assert_eq!(
                counters.rhs_calls,
                stats.assembly_rhs_calls + stats.verification_rhs_calls,
                "{label}: counters"
            );
            assert_eq!(unchecked_counters.rhs_calls, 3 * rhs_per_build, "{label}");
            // The explicit multi-colouring check on the same configuration
            // spends total_groups right-hand sides.
            let mut reference = AssemblyStageClient::new(
                fd.n,
                backend,
                default_options(&declared, execution).with_first_use_check(CheckColumns::None),
            )
            .unwrap();
            assert_eq!(
                reference.verify(
                    &fd.problem,
                    fd.span.0,
                    &fd.y0,
                    CheckColumns::MultiColouring,
                    &mut WorkCounters::default(),
                ),
                Ok(fd.n),
                "{label}"
            );
            assert_eq!(
                reference.handle().stats().verification_rhs_calls,
                groups + execution.repeated_base(),
                "{label}"
            );
        }
    }
}

/// The first linearization of a fresh client with `options`: the typed
/// error of the check, or `Ok` with the columns checked.
fn first_use(
    fd: &FdProblem,
    backend: LinearBackend,
    options: AssemblyOptions,
) -> Result<usize, AssemblyError> {
    let mut client = AssemblyStageClient::new(fd.n, backend, options).unwrap();
    let handle = client.handle();
    match client.linearize(&fd.problem, fd.span.0, &fd.y0, &mut WorkCounters::default()) {
        Ok(()) => Ok(handle.stats().verified_columns as usize),
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
    declared: &SparsityPattern,
    execution: Execution,
    check: CheckColumns,
) -> Result<usize, AssemblyError> {
    let options = default_options(declared, execution).with_first_use_check(CheckColumns::None);
    AssemblyStageClient::new(fd.n, DENSE_IN_PLACE, options)
        .unwrap()
        .verify(
            &fd.problem,
            fd.span.0,
            &fd.y0,
            check,
            &mut WorkCounters::default(),
        )
}

#[test]
fn the_full_default_detects_every_omission_of_a_wide_pattern() {
    let rows = wide_rows();
    let fd = generic_problem("wide-n60", &rows);
    let truth = pattern(&rows);
    assert_eq!(
        first_use(
            &fd,
            DENSE_IN_PLACE,
            default_options(&truth, Execution::Sequential)
        ),
        Ok(60),
        "the true pattern passes"
    );
    let (mut omissions, mut full_default, mut reached_the_check) = (0, 0, 0);
    for (i, row) in rows.iter().enumerate() {
        for &k in row.iter().filter(|&&k| k != i) {
            let mut wrong = rows.clone();
            wrong[i].retain(|&column| column != k);
            let declared = pattern(&wrong);
            // One omission can narrow the band or change the client's
            // colouring: the default follows the rule of the declaration.
            let expected = rule(&declared);
            for execution in [Execution::Sequential, Execution::Threaded(3)] {
                let label = format!("without ({i}, {k}), {execution:?}");
                let options = default_options(&declared, execution);
                assert_eq!(options.first_use_check, expected, "{label}");
                let default = first_use(&fd, DENSE_IN_PLACE, options);
                let same = explicit(&fd, &declared, execution, expected);
                assert_eq!(default, same, "{label}: the default is {expected:?}");
                let error = default.expect_err(&label);
                assert!(error.is_contract_violation(), "{label}: {error:?}");
                for other in [CheckColumns::All, CheckColumns::MultiColouring] {
                    assert!(
                        explicit(&fd, &declared, execution, other)
                            .is_err_and(|error| error.is_contract_violation()),
                        "{label}: {other:?} detects it too"
                    );
                }
                if execution == Execution::Sequential {
                    omissions += 1;
                    if expected == CheckColumns::All {
                        full_default += 1;
                        if !matches!(error, AssemblyError::UncoveredRowChanged { .. }) {
                            reached_the_check += 1;
                        }
                    }
                }
            }
        }
    }
    let off_diagonal: usize = rows.iter().map(|row| row.len() - 1).sum();
    println!(
        "STAGE_CLIENTS_ASSEMBLY_WIDE_BAND_JSON {}",
        serde_json::json!({
            "test": "the_full_default_detects_every_omission_of_a_wide_pattern",
            "omissions": omissions,
            "full_check_default": full_default,
            "full_check_default_found_by_the_check_not_the_build": reached_the_check,
        })
    );
    assert_eq!(omissions, off_diagonal);
    // The full check is the default of (nearly) every omission, and some
    // omissions pass the build and are found by the full check itself.
    assert!(
        full_default * 10 >= omissions * 9,
        "{full_default} of {omissions}"
    );
    assert!(
        reached_the_check > 10,
        "{reached_the_check} of {full_default}"
    );
}

#[test]
fn every_check_stays_selectable_on_a_wide_pattern() {
    let rows = wide_rows();
    let fd = generic_problem("wide-n60", &rows);
    let declared = pattern(&rows);
    let n = fd.n as u64;
    let groups = total_groups(&declared) as u64;
    for execution in [Execution::Sequential, Execution::Threaded(2)] {
        let base = execution.repeated_base();
        for (check, columns, calls) in [
            (CheckColumns::None, 0, 0),
            (CheckColumns::Sample(8), 8, 8 + base),
            (CheckColumns::All, n, n + base),
            (CheckColumns::AllPooled, n, n + base),
            (CheckColumns::MultiColouring, n, groups + base),
            (CheckColumns::MultiColouringPooled, n, groups + base),
        ] {
            let label = format!("{check:?}, {execution:?}");
            // As the first-use check.
            let options = default_options(&declared, execution).with_first_use_check(check);
            assert_eq!(options.first_use_check, check, "{label}");
            let mut client = AssemblyStageClient::new(fd.n, DENSE_IN_PLACE, options).unwrap();
            let handle = client.handle();
            client
                .linearize(&fd.problem, fd.span.0, &fd.y0, &mut WorkCounters::default())
                .unwrap();
            let stats = handle.stats();
            let verifications = u64::from(check != CheckColumns::None);
            assert_eq!(
                (
                    stats.verifications,
                    stats.verified_columns,
                    stats.verification_rhs_calls
                ),
                (verifications, columns, calls),
                "{label}: first use"
            );
            // Through the handle, at a later linearization of a client of
            // the default options.
            let mut client = AssemblyStageClient::new(
                fd.n,
                DENSE_IN_PLACE,
                default_options(&declared, execution),
            )
            .unwrap();
            let handle = client.handle();
            client
                .linearize(&fd.problem, fd.span.0, &fd.y0, &mut WorkCounters::default())
                .unwrap();
            let before = handle.stats();
            handle.request_check(check);
            client
                .linearize(&fd.problem, 0.5, &fd.y0, &mut WorkCounters::default())
                .unwrap();
            let after = handle.stats();
            assert_eq!(
                (
                    after.verifications - before.verifications,
                    after.verified_columns - before.verified_columns,
                    after.verification_rhs_calls - before.verification_rhs_calls
                ),
                (verifications, columns, calls),
                "{label}: requested"
            );
            // Through verify.
            assert_eq!(
                explicit(&fd, &declared, execution, check),
                Ok(columns as usize),
                "{label}: verify"
            );
        }
    }
    // The multi-colouring checks stay refused on the dense route.
    for check in [
        CheckColumns::MultiColouring,
        CheckColumns::MultiColouringPooled,
    ] {
        assert!(matches!(
            AssemblyStageClient::new(
                24,
                DENSE_IN_PLACE,
                AssemblyOptions::dense().with_first_use_check(check)
            ),
            Err(AssemblyError::Configuration(_))
        ));
    }
    // The threaded dense route still runs Sample(8) at its first use.
    let dense = nlkpp_dense(24, false);
    let mut client = AssemblyStageClient::new(
        24,
        DENSE_IN_PLACE,
        AssemblyOptions::dense().threaded(2, RhsPurity::pure_and_reentrant()),
    )
    .unwrap();
    client
        .linearize(
            &dense.problem,
            dense.span.0,
            &dense.y0,
            &mut WorkCounters::default(),
        )
        .unwrap();
    let stats = client.handle().stats();
    assert_eq!(
        (
            stats.verifications,
            stats.verified_columns,
            stats.verification_rhs_calls
        ),
        (1, 8, 9)
    );
}
