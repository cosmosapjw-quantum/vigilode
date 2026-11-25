//! Unit tests of the multi-colouring check and of the pooled checks of the
//! assembly client (loop node V26, `src/stage_clients/assembly/multi.rs`
//! and `assembly/engine.rs`; frozen design in
//! `research/production_loop_20261116/assembly_check/PREDECLARED.json`).
//!
//! (i) The further colourings are the frozen constructions: valid, with
//! injective colour tuples, and the construction with the smaller total of
//! groups is chosen.
//!
//! (ii) On every band of order `n <= 40` (every `lower, upper < n`) and on
//! two seeded random sparse patterns, every single-entry omission of the
//! true pattern: the multi-colouring check (`MultiColouring`) detects
//! exactly the omissions the full check (`All`) detects, or the arm that
//! misses is rounding-limited: at every state it evaluated with the omitted
//! column shifted, the bits of the omitted entry's row are the same with
//! and without that shift (evaluated directly).
//!
//! (iii) The pooled checks decide as the checks on the calling thread.
//!
//! (iv) True patterns pass every check, a check does not change the
//! assembled Jacobian, and the multi-colouring check refuses the dense
//! route.

use std::sync::Arc;

use rodas5p_core::{CoreResult, WorkCounters};
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, CheckColourings, CheckColumns,
    CheckConstruction, LinearBackend, RhsPurity, SparsityPattern, colour_columns,
    colouring_is_valid,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{FdProblem, Rhs, brusselator, from_rhs, impure, ring, ring_pattern};

const DENSE_IN_PLACE: LinearBackend = LinearBackend::DenseLu(DenseLuMode::InPlace);

/// The forward-difference increment of `crates/vigilode/src/fd.rs:443-455`.
fn increment(yj: f64) -> f64 {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    if forward.is_finite() {
        forward
    } else {
        yj - raw
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn unit(&mut self) -> f64 {
        (self.next() % (1_u64 << 40)) as f64 / (1_u64 << 40) as f64
    }

    fn below(&mut self, k: usize) -> usize {
        (self.next() % k as u64) as usize
    }
}

fn band_rows(n: usize, lower: usize, upper: usize) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| (i.saturating_sub(lower)..=(i + upper).min(n - 1)).collect())
        .collect()
}

/// `dy_i/dt = sum_j a_ij y_j (1 + y_j / 10)` over the declared columns of
/// row `i` (coefficients of either sign, magnitude 0.5 to 1.5): every
/// declared entry is a true dependency, and nothing else is read.
fn rows_problem(id: String, rows: &[Vec<usize>], rng: &mut Lcg) -> FdProblem {
    let n = rows.len();
    let coefficients: Vec<Vec<(usize, f64)>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&j| {
                    let magnitude = 0.5 + rng.unit();
                    (
                        j,
                        if rng.below(2) == 0 {
                            magnitude
                        } else {
                            -magnitude
                        },
                    )
                })
                .collect()
        })
        .collect();
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for (value, row) in out.iter_mut().zip(&coefficients) {
                let mut acc = 0.0;
                for &(j, a) in row {
                    acc += a * y[j] * (1.0 + 0.1 * y[j]);
                }
                *value = acc;
            }
            Ok(())
        },
    );
    let y0 = (0..n).map(|_| 0.5 + rng.unit()).collect();
    from_rhs(id, rhs, y0, (0.0, 1.0), None)
}

/// The random problem of the V08 review probe: random rows, terms `y`,
/// `y^2`, `sin y`, `tanh(y / 2)`, and a start with zeros, negative zeros,
/// subnormal-scale, large and ordinary values.
fn random_problem(n: usize, density: f64, rng: &mut Lcg) -> (FdProblem, Vec<Vec<usize>>) {
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in rows.iter_mut().enumerate() {
        for j in 0..n {
            if rng.unit() < density || (i == j && rng.unit() < 0.7) {
                row.push(j);
            }
        }
    }
    let terms: Vec<Vec<(usize, f64, u8)>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&j| (j, 2.0 * rng.unit() - 1.0, rng.below(4) as u8))
                .collect()
        })
        .collect();
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for (i, (value, row)) in out.iter_mut().zip(&terms).enumerate() {
                let mut acc = 0.25 * (i as f64).sin();
                for &(j, a, kind) in row {
                    let v = y[j];
                    acc += a * match kind {
                        0 => v,
                        1 => v * v,
                        2 => v.sin(),
                        _ => (0.5 * v).tanh(),
                    };
                }
                *value = acc;
            }
            Ok(())
        },
    );
    let y0 = (0..n)
        .map(|_| {
            let r = rng.unit();
            if r < 0.05 {
                0.0
            } else if r < 0.10 {
                -0.0
            } else if r < 0.15 {
                1.0e-300 * rng.unit()
            } else if r < 0.20 {
                1.0e6 * (rng.unit() - 0.5)
            } else if r < 0.25 {
                -1.0
            } else {
                4.0 * (rng.unit() - 0.5)
            }
        })
        .collect();
    (
        from_rhs(format!("random-n{n}"), rhs, y0, (0.0, 1.0), None),
        rows,
    )
}

fn client(
    n: usize,
    pattern: &SparsityPattern,
    workers: Option<usize>,
) -> Result<AssemblyStageClient, AssemblyError> {
    let options =
        AssemblyOptions::coloured(pattern.clone()).with_first_use_check(CheckColumns::None);
    let options = match workers {
        None => options,
        Some(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    };
    AssemblyStageClient::new(n, DENSE_IN_PLACE, options)
}

/// One build and one check at `y0` on a fresh client.
fn decide(
    fd: &FdProblem,
    pattern: &SparsityPattern,
    workers: Option<usize>,
    check: CheckColumns,
) -> Result<usize, AssemblyError> {
    client(fd.n, pattern, workers)?.verify(
        &fd.problem,
        fd.span.0,
        &fd.y0,
        check,
        &mut WorkCounters::default(),
    )
}

fn detected(outcome: &Result<usize, AssemblyError>) -> bool {
    outcome
        .as_ref()
        .is_err_and(AssemblyError::is_contract_violation)
}

/// Whether the bits of row `i` are the same with and without the shift of
/// column `k` at every state of `states` (each the set of shifted columns,
/// `k` among them).
fn rounding_limited(fd: &FdProblem, states: &[Vec<usize>], i: usize, k: usize) -> bool {
    let mut out = vec![0.0; fd.n];
    let mut row_at = |state: &[f64]| {
        out.fill(0.0);
        fd.problem
            .eval_rhs_into(fd.span.0, state, &mut out, &mut WorkCounters::default())
            .unwrap();
        out[i]
    };
    states.iter().all(|shifted| {
        let mut state = fd.y0.clone();
        for &j in shifted {
            state[j] = increment(fd.y0[j]);
        }
        let with = row_at(&state);
        state[k] = fd.y0[k];
        with.to_bits() == row_at(&state).to_bits()
    })
}

/// The states a check evaluated with column `k` shifted: the group of `k`
/// in the client's colouring, then column `k` alone (full check) or the
/// group of `k` in every further colouring (multi-colouring check).
fn states_of(pattern: &SparsityPattern, k: usize, multi: bool) -> Vec<Vec<usize>> {
    let (colour, _) = colour_columns(pattern);
    let mut states = vec![
        (0..pattern.n())
            .filter(|&j| colour[j] == colour[k])
            .collect::<Vec<_>>(),
    ];
    if multi {
        let plan = CheckColourings::of(pattern).unwrap();
        for r in 0..plan.colourings() {
            states.push(plan.columns(r, plan.colour(r, k)));
        }
    } else {
        states.push(vec![k]);
    }
    states
}

#[derive(Debug, Default)]
struct Tally {
    omissions: u64,
    /// Detected by the build that precedes every check (the uncovered-row
    /// test of the client's colouring), so by both checks.
    at_build: u64,
    both: u64,
    neither: u64,
    full_only: u64,
    multi_only: u64,
    /// Misses of the full check that are rounding-limited for it.
    full_misses_limited: u64,
    failures: Vec<String>,
}

impl Tally {
    fn merge(&mut self, other: Tally) {
        self.omissions += other.omissions;
        self.at_build += other.at_build;
        self.both += other.both;
        self.neither += other.neither;
        self.full_only += other.full_only;
        self.multi_only += other.multi_only;
        self.full_misses_limited += other.full_misses_limited;
        self.failures.extend(other.failures);
    }
}

/// Every single-entry omission of `rows`, the true pattern of `fd`.
fn omissions(fd: &FdProblem, rows: &[Vec<usize>], tally: &mut Tally) {
    let n = fd.n;
    for i in 0..n {
        for &k in &rows[i] {
            let mut wrong = rows.to_vec();
            wrong[i].retain(|&column| column != k);
            let pattern = SparsityPattern::from_rows(n, &wrong).unwrap();
            tally.omissions += 1;
            let label = format!("{} omitting ({i}, {k})", fd.id);
            // Every check runs after the build and only if the build passes
            // (AssemblyStageClient::assemble): an omission the build detects
            // is detected by both checks with the build's error.
            match decide(fd, &pattern, None, CheckColumns::None) {
                Err(error) if error.is_contract_violation() => {
                    tally.at_build += 1;
                    tally.both += 1;
                    continue;
                }
                Err(error) => {
                    tally
                        .failures
                        .push(format!("{label}: the build failed: {error:?}"));
                    continue;
                }
                Ok(_) => {}
            }
            let full = decide(fd, &pattern, None, CheckColumns::All);
            let multi = decide(fd, &pattern, None, CheckColumns::MultiColouring);
            let full_limited = || rounding_limited(fd, &states_of(&pattern, k, false), i, k);
            let multi_limited = || rounding_limited(fd, &states_of(&pattern, k, true), i, k);
            match (detected(&full), detected(&multi)) {
                (true, true) => tally.both += 1,
                (false, false) => {
                    tally.neither += 1;
                    if full_limited() {
                        tally.full_misses_limited += 1;
                    }
                }
                (true, false) => {
                    tally.full_only += 1;
                    if !multi_limited() {
                        tally.failures.push(format!(
                            "{label}: the full check detects ({full:?}), the multi-colouring check \
                             passes and is not rounding-limited"
                        ));
                    }
                }
                (false, true) => {
                    tally.multi_only += 1;
                    if full_limited() {
                        tally.full_misses_limited += 1;
                    } else {
                        tally.failures.push(format!(
                            "{label}: the multi-colouring check detects ({multi:?}), the full \
                             check passes and is not rounding-limited"
                        ));
                    }
                }
            }
            for outcome in [&full, &multi] {
                if let Err(error) = outcome
                    && !error.is_contract_violation()
                {
                    tally
                        .failures
                        .push(format!("{label}: not a contract violation: {error:?}"));
                }
            }
        }
    }
}

fn report(label: &str, tally: &Tally) {
    println!(
        "V26_MC_UNIT_JSON {}",
        serde_json::json!({
            "test": label,
            "single_entry_omissions": tally.omissions,
            "detected_at_the_build": tally.at_build,
            "detected_by_both": tally.both,
            "detected_by_neither": tally.neither,
            "full_check_only": tally.full_only,
            "multi_colouring_only": tally.multi_only,
            "full_check_misses_rounding_limited": tally.full_misses_limited,
            "failures": tally.failures.len(),
        })
    );
    assert!(
        tally.failures.is_empty(),
        "{label}: {} failures, first: {:?}",
        tally.failures.len(),
        tally.failures.iter().take(5).collect::<Vec<_>>()
    );
}

#[test]
fn the_constructions_are_the_frozen_ones() {
    // Full bands of the cost cells (PREDECLARED.json mc_construction.cost_note).
    let plan = CheckColourings::of(&SparsityPattern::banded(131_072, 2, 2).unwrap()).unwrap();
    assert_eq!(
        plan.construction(),
        &CheckConstruction::Band {
            lower: 2,
            upper: 2,
            moduli: vec![5, 6, 7, 11, 13, 17]
        }
    );
    assert_eq!(plan.total_groups(), 59);
    let plan = CheckColourings::of(&SparsityPattern::banded(4096, 24, 24).unwrap()).unwrap();
    assert_eq!(
        plan.construction(),
        &CheckConstruction::Band {
            lower: 24,
            upper: 24,
            moduli: vec![49, 50, 51]
        }
    );
    assert_eq!(plan.total_groups(), 150);
    // The periodic ring: its corners make the band as wide as the matrix, so
    // the refinement of the client's colouring is cheaper.
    let truth = ring_pattern(1000);
    let plan = CheckColourings::of(&truth).unwrap();
    assert_eq!(plan.total_groups_band(), 1000);
    assert!(matches!(
        plan.construction(),
        CheckConstruction::Refinement { .. }
    ));
    assert!(plan.total_groups() < plan.total_groups_band());
    // Order 1, a diagonal and a dense pattern.
    let plan = CheckColourings::of(&SparsityPattern::banded(1, 0, 0).unwrap()).unwrap();
    assert_eq!(plan.colourings(), 0);
    let diagonal = SparsityPattern::banded(9, 0, 0).unwrap();
    let plan = CheckColourings::of(&diagonal).unwrap();
    assert!(plan.total_groups() >= 1);
    let dense = SparsityPattern::banded(7, 6, 6).unwrap();
    let plan = CheckColourings::of(&dense).unwrap();
    assert_eq!(
        plan.colourings(),
        0,
        "every column has its own colour already"
    );
    // Validity, injectivity and the choice on many patterns.
    let mut rng = Lcg(0x0026_1124_0001);
    let mut patterns = vec![truth, diagonal, dense];
    for n in [1, 2, 3, 5, 8, 13, 40, 100] {
        for (lower, upper) in [(0, 0), (1, 0), (0, 2), (1, 1), (3, 2), (n - 1, 0)] {
            if lower < n && upper < n {
                patterns.push(SparsityPattern::banded(n, lower, upper).unwrap());
            }
        }
        for density in [0.02, 0.1, 0.4] {
            let (_, rows) = random_problem(n, density, &mut rng);
            patterns.push(SparsityPattern::from_rows(n, &rows).unwrap());
        }
    }
    for pattern in &patterns {
        let plan = CheckColourings::of(pattern).unwrap();
        let n = pattern.n();
        let (c1, _) = colour_columns(pattern);
        let mut tuples: Vec<Vec<usize>> = (0..n)
            .map(|j| {
                std::iter::once(c1[j])
                    .chain((0..plan.colourings()).map(|r| plan.colour(r, j)))
                    .collect()
            })
            .collect();
        for r in 0..plan.colourings() {
            let colours: Vec<usize> = (0..n).map(|j| plan.colour(r, j)).collect();
            assert!(
                colouring_is_valid(pattern, &colours),
                "colouring {r} of order {n}"
            );
            let mut seen = vec![false; n];
            for g in 0..plan.groups(r) {
                let columns = plan.columns(r, g);
                assert!(!columns.is_empty(), "group {g} of colouring {r} is empty");
                for j in columns {
                    assert_eq!(plan.colour(r, j), g);
                    assert!(!seen[j]);
                    seen[j] = true;
                }
            }
            assert!(seen.iter().all(|&s| s), "colouring {r} misses a column");
        }
        tuples.sort();
        tuples.dedup();
        assert_eq!(tuples.len(), n, "colour tuples are not injective");
        assert_eq!(
            plan.total_groups(),
            plan.total_groups_band().min(plan.total_groups_refinement())
        );
    }
}

#[test]
fn the_multi_colouring_check_detects_what_the_full_check_detects_on_every_band() {
    let shapes: Vec<(usize, usize, usize)> = (1..=40)
        .flat_map(|n| (0..n).flat_map(move |lower| (0..n).map(move |upper| (n, lower, upper))))
        .collect();
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, 6);
    let mut tally = Tally::default();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|thread| {
                let shapes = &shapes;
                scope.spawn(move || {
                    let mut tally = Tally::default();
                    for &(n, lower, upper) in shapes.iter().skip(thread).step_by(threads) {
                        let rows = band_rows(n, lower, upper);
                        let mut rng = Lcg(((n * 64 + lower) * 64 + upper) as u64);
                        let fd =
                            rows_problem(format!("band-n{n}-{lower}-{upper}"), &rows, &mut rng);
                        omissions(&fd, &rows, &mut tally);
                    }
                    tally
                })
            })
            .collect();
        for handle in handles {
            tally.merge(handle.join().unwrap());
        }
    });
    report("every band of order 1 to 40", &tally);
}

#[test]
fn the_multi_colouring_check_detects_what_the_full_check_detects_on_random_patterns() {
    let mut tally = Tally::default();
    for (seed, n, density) in [
        (0x0026_1124_00a1_u64, 48, 0.08),
        (0x0026_1124_00b2, 120, 0.03),
    ] {
        let mut rng = Lcg(seed);
        let (fd, rows) = random_problem(n, density, &mut rng);
        omissions(&fd, &rows, &mut tally);
    }
    report("two seeded random sparse patterns", &tally);
}

#[test]
fn pooled_checks_decide_as_checks_on_the_calling_thread() {
    let mut cases: Vec<(FdProblem, SparsityPattern)> = Vec::new();
    // Every single-entry omission of the Brusselator of 16 cells.
    let fd = brusselator(16, false);
    let truth = pattern_of_brusselator(16);
    for i in 0..fd.n {
        for &k in truth[i].iter() {
            let mut wrong = truth.clone();
            wrong[i].retain(|&column| column != k);
            cases.push((
                brusselator(16, false),
                SparsityPattern::from_rows(fd.n, &wrong).unwrap(),
            ));
        }
    }
    cases.push((
        brusselator(16, false),
        SparsityPattern::from_rows(fd.n, &truth).unwrap(),
    ));
    for n in [12, 34, 64] {
        cases.push((
            ring(n, false, false),
            SparsityPattern::banded(n, 1, 1).unwrap(),
        ));
        cases.push((ring(n, false, false), ring_pattern(n)));
    }
    let mut rng = Lcg(0x0026_1124_00c3);
    let (fd, rows) = random_problem(40, 0.08, &mut rng);
    let pattern = SparsityPattern::from_rows(40, &rows).unwrap();
    cases.push((fd, pattern));
    let mut compared = 0;
    for (fd, pattern) in &cases {
        for (sequential, pooled) in [
            (CheckColumns::All, CheckColumns::AllPooled),
            (
                CheckColumns::MultiColouring,
                CheckColumns::MultiColouringPooled,
            ),
        ] {
            let reference = format!("{:?}", decide(fd, pattern, None, sequential));
            for workers in [1, 2, 3, 6] {
                let on_thread = format!("{:?}", decide(fd, pattern, Some(workers), sequential));
                let on_pool = format!("{:?}", decide(fd, pattern, Some(workers), pooled));
                assert_eq!(on_thread, reference, "{} {sequential:?} t{workers}", fd.id);
                assert_eq!(on_pool, reference, "{} {pooled:?} t{workers}", fd.id);
                compared += 2;
            }
            // Without a pool the pooled options run on the calling thread.
            assert_eq!(
                format!("{:?}", decide(fd, pattern, None, pooled)),
                reference,
                "{} {pooled:?} sequential client",
                fd.id
            );
        }
    }
    // An impure right-hand side: the threaded client's base evaluation
    // catches it first, with and without the pool.
    let fd = impure(16);
    let pattern = SparsityPattern::banded(16, 1, 1).unwrap();
    for check in [
        CheckColumns::All,
        CheckColumns::AllPooled,
        CheckColumns::MultiColouring,
        CheckColumns::MultiColouringPooled,
    ] {
        assert!(
            detected(&decide(&fd, &pattern, Some(3), check)),
            "{check:?}"
        );
    }
    assert!(compared > 0);
}

fn pattern_of_brusselator(cells: usize) -> Vec<Vec<usize>> {
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
fn true_patterns_pass_and_the_jacobian_does_not_change() {
    let cases: Vec<(FdProblem, SparsityPattern)> = vec![
        (
            brusselator(64, false),
            SparsityPattern::from_rows(128, &pattern_of_brusselator(64)).unwrap(),
        ),
        (
            brusselator(64, false),
            SparsityPattern::banded(128, 2, 2).unwrap(),
        ),
        (ring(100, false, false), ring_pattern(100)),
    ];
    for (fd, pattern) in &cases {
        let mut reference = client(fd.n, pattern, None).unwrap();
        reference
            .verify(
                &fd.problem,
                0.0,
                &fd.y0,
                CheckColumns::None,
                &mut WorkCounters::default(),
            )
            .unwrap();
        let reference = reference.jacobian_values().unwrap();
        for workers in [None, Some(4)] {
            for check in [
                CheckColumns::Sample(8),
                CheckColumns::All,
                CheckColumns::AllPooled,
                CheckColumns::MultiColouring,
                CheckColumns::MultiColouringPooled,
            ] {
                let mut checked = client(fd.n, pattern, workers).unwrap();
                let mut counters = WorkCounters::default();
                let columns = checked
                    .verify(&fd.problem, 0.0, &fd.y0, check, &mut counters)
                    .unwrap_or_else(|error| panic!("{} {check:?} {workers:?}: {error:?}", fd.id));
                let values = checked.jacobian_values().unwrap();
                assert!(
                    values
                        .iter()
                        .zip(&reference)
                        .all(|(a, b)| a.to_bits() == b.to_bits()),
                    "{} {check:?} {workers:?}: the check changed the Jacobian",
                    fd.id
                );
                let stats = checked.handle().stats();
                let expected = match check {
                    CheckColumns::Sample(k) => k,
                    CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled => {
                        CheckColourings::of(pattern).unwrap().total_groups()
                    }
                    _ => fd.n,
                } as u64
                    + u64::from(workers.is_some());
                assert_eq!(
                    stats.verification_rhs_calls, expected,
                    "{} {check:?}",
                    fd.id
                );
                let columns_expected = match check {
                    CheckColumns::Sample(k) => k,
                    _ => fd.n,
                };
                assert_eq!(columns, columns_expected);
                assert_eq!(
                    counters.rhs_calls,
                    stats.assembly_rhs_calls + stats.verification_rhs_calls
                );
            }
        }
    }
}

#[test]
fn the_multi_colouring_check_refuses_the_dense_route() {
    let fd = brusselator(8, false);
    for check in [
        CheckColumns::MultiColouring,
        CheckColumns::MultiColouringPooled,
    ] {
        let refused = AssemblyStageClient::new(
            fd.n,
            DENSE_IN_PLACE,
            AssemblyOptions::dense().with_first_use_check(check),
        );
        assert!(
            matches!(refused, Err(AssemblyError::Configuration(_))),
            "{check:?}"
        );
        let mut dense = AssemblyStageClient::new(
            fd.n,
            DENSE_IN_PLACE,
            AssemblyOptions::dense().with_first_use_check(CheckColumns::None),
        )
        .unwrap();
        let mut counters = WorkCounters::default();
        assert!(matches!(
            dense.verify(&fd.problem, 0.0, &fd.y0, check, &mut counters),
            Err(AssemblyError::Configuration(_))
        ));
    }
    // The pooled full check on the dense route decides as the full check.
    let fd = impure(12);
    let options = AssemblyOptions::dense().with_first_use_check(CheckColumns::None);
    for check in [CheckColumns::All, CheckColumns::AllPooled] {
        let mut dense = AssemblyStageClient::new(fd.n, DENSE_IN_PLACE, options.clone()).unwrap();
        let outcome = dense.verify(
            &fd.problem,
            0.0,
            &fd.y0,
            check,
            &mut WorkCounters::default(),
        );
        assert!(
            matches!(outcome, Err(AssemblyError::NotReproducible { .. })),
            "{check:?}"
        );
    }
}
