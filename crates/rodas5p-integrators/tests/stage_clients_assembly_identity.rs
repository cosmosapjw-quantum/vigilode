//! Contracts of the finite-difference assembly client (loop node V08,
//! `src/stage_clients/assembly.rs`).
//!
//! (i) Every mode (dense or coloured route, sequential or threaded with
//! 1-4 workers, dense or band storage) assembles the Jacobian of the
//! reference finite differences bit for bit; the reference is a second
//! transcription of `crates/vigilode/src/fd.rs` in
//! `stage_clients_assembly_problems.rs`.
//!
//! (ii) Under the client driver every mode reproduces the protected driver
//! run with the reference finite differences as its Jacobian callback
//! (dense: `integrate_rodas5p_fast_observed_with_atol`; banded:
//! `integrate_rodas5p_fast_banded_observed_with_atol`, kernel `Indexed`):
//! step statistics, every counter (the right-hand sides of the assembly
//! taken out), every output, every linearization point and every
//! assembled Jacobian.
//!
//! (iii) Right-hand-side calls: `n + 1` or `colours + 1` per build, the
//! declared check calls on first use, counted by the client, by the run's
//! counters and by the right-hand side itself.
//!
//! (iv) A wrong pattern, an impure right-hand side and a refused
//! configuration fail closed with a typed error.

use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, CheckColumns, ColumnRoute, LinearBackend,
    RhsPurity, SparsityPattern, colour_columns, colouring_is_valid,
};
use rodas5p_integrators::{
    DriverStopReason, RODAS5P_FAST_CLIENT_DRIVER_ID, Rodas5pFastLu, StageLinearSystem,
};

#[path = "stage_clients_assembly_problems.rs"]
mod problems;
use problems::{
    Arm, ClientRun, ClientSpec, FdProblem, ProtectedRun, Rhs, RunConfig, bits, brusselator,
    final_state, from_rhs, impure, lower_coupled_chain, nlkpp_band, nlkpp_dense,
    reference_banded_fd, reference_dense_fd, ring, ring_pattern, run_client, run_protected_banded,
    run_protected_dense, upper_bidiagonal,
};

const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];
const WORKERS: [usize; 4] = [1, 2, 3, 4];
const DENSE_IN_PLACE: LinearBackend = LinearBackend::DenseLu(DenseLuMode::InPlace);

fn spec(route: ColumnRoute, arm: Arm, backend: LinearBackend) -> ClientSpec {
    ClientSpec {
        route,
        arm,
        backend,
        first_use_check: CheckColumns::Sample(AssemblyOptions::DEFAULT_CHECK_COLUMNS),
        fingerprints: true,
    }
}

/// The pattern a problem declares: its full band, or the ring pattern.
fn pattern_of(fd: &FdProblem) -> Option<SparsityPattern> {
    match fd.band {
        Some((lower, upper)) => Some(SparsityPattern::banded(fd.n, lower, upper).unwrap()),
        None if fd.id.starts_with("ring") => Some(ring_pattern(fd.n)),
        None => None,
    }
}

/// States at which Jacobians are compared: the start and two smooth
/// perturbations of it.
fn states(fd: &FdProblem) -> Vec<Vec<f64>> {
    let perturbed = |scale: f64, frequency: f64| -> Vec<f64> {
        fd.y0
            .iter()
            .enumerate()
            .map(|(i, y)| y * (1.0 + scale * (frequency * i as f64).sin()) + scale * 0.5)
            .collect()
    };
    vec![fd.y0.clone(), perturbed(0.05, 0.9), perturbed(0.4, 2.3)]
}

/// The Jacobian a client assembles at `(t, y)`, in its back end's layout.
fn assembled(fd: &FdProblem, spec: &ClientSpec, t: f64, y: &[f64]) -> Vec<f64> {
    let mut client = spec.build(fd.n);
    client
        .linearize(&fd.problem, t, y, &mut WorkCounters::default())
        .unwrap_or_else(|error| panic!("{}: {error}", fd.id));
    client.jacobian_values().unwrap()
}

fn assert_bits(label: &str, reference: &[f64], other: &[f64]) {
    assert_eq!(reference.len(), other.len(), "{label}: length");
    if let Some(index) =
        (0..reference.len()).find(|&k| reference[k].to_bits() != other[k].to_bits())
    {
        panic!(
            "{label}: entry {index} is {:e}, the reference {:e}",
            other[index], reference[index]
        );
    }
}

#[test]
fn every_mode_assembles_the_reference_jacobian_bit_for_bit() {
    let cases = [
        nlkpp_dense(24, false),
        nlkpp_band(40, 3, false),
        brusselator(10, false),
        ring(12, false, false),
        ring(11, false, false),
    ];
    let mut compared = 0;
    for fd in &cases {
        let n = fd.n;
        for (index, y) in states(fd).iter().enumerate() {
            let t = 0.25 * index as f64;
            let label = format!("{} state {index}", fd.id);
            let mut reference = DenseMatrix::zeros(n, n);
            reference_dense_fd(&fd.problem, t, y, &mut reference).unwrap();
            let reference = reference.as_slice();

            let dense_seq = assembled(
                fd,
                &spec(ColumnRoute::Dense, Arm::Seq, DENSE_IN_PLACE),
                t,
                y,
            );
            assert_bits(&format!("{label}: dense seq"), reference, &dense_seq);
            for workers in WORKERS {
                let threaded = assembled(
                    fd,
                    &spec(ColumnRoute::Dense, Arm::Threaded(workers), DENSE_IN_PLACE),
                    t,
                    y,
                );
                assert_bits(&format!("{label}: dense t{workers}"), reference, &threaded);
                compared += 1;
            }

            let Some(pattern) = pattern_of(fd) else {
                continue;
            };
            // Dense storage: the declared entries are the reference's and
            // every other entry is zero in both (the right-hand sides read
            // only what the pattern declares).
            let route = ColumnRoute::Coloured(pattern.clone());
            let coloured_seq = assembled(fd, &spec(route.clone(), Arm::Seq, DENSE_IN_PLACE), t, y);
            assert_bits(&format!("{label}: coloured seq"), reference, &coloured_seq);
            for workers in WORKERS {
                let threaded = assembled(
                    fd,
                    &spec(route.clone(), Arm::Threaded(workers), DENSE_IN_PLACE),
                    t,
                    y,
                );
                assert_bits(
                    &format!("{label}: coloured t{workers}"),
                    reference,
                    &threaded,
                );
                compared += 1;
            }

            let Some((lower, upper)) = fd.band else {
                continue;
            };
            // Band storage against the grouped reference.
            let mut band = vec![0.0; n * (lower + upper + 1)];
            reference_banded_fd(&fd.problem, t, y, &mut band, lower, upper).unwrap();
            for backend in [
                LinearBackend::BandedProtected { lower, upper },
                LinearBackend::BandedTight { lower, upper },
            ] {
                let seq = assembled(fd, &spec(route.clone(), Arm::Seq, backend), t, y);
                assert_bits(&format!("{label}: band seq {backend:?}"), &band, &seq);
                for workers in WORKERS {
                    let threaded = assembled(
                        fd,
                        &spec(route.clone(), Arm::Threaded(workers), backend),
                        t,
                        y,
                    );
                    assert_bits(
                        &format!("{label}: band t{workers} {backend:?}"),
                        &band,
                        &threaded,
                    );
                    compared += 1;
                }
            }
        }
    }
    assert!(compared >= 100, "threaded comparisons made: {compared}");
}

#[test]
fn a_dense_pattern_on_the_coloured_route_is_the_dense_route() {
    // Every column in a group of its own: n colours, the dense Jacobian.
    let fd = nlkpp_dense(20, false);
    let pattern = SparsityPattern::banded(fd.n, fd.n - 1, fd.n - 1).unwrap();
    let (_, colours) = colour_columns(&pattern);
    assert_eq!(colours, fd.n);
    let dense = assembled(
        &fd,
        &spec(ColumnRoute::Dense, Arm::Seq, DENSE_IN_PLACE),
        0.0,
        &fd.y0,
    );
    for arm in [Arm::Seq, Arm::Threaded(3)] {
        let coloured = assembled(
            &fd,
            &spec(ColumnRoute::Coloured(pattern.clone()), arm, DENSE_IN_PLACE),
            0.0,
            &fd.y0,
        );
        assert_bits(&format!("{arm:?}"), &dense, &coloured);
    }
}

#[test]
fn the_groups_are_curtis_powell_reid_first_fit() {
    // A full band: column j in group j mod (lower + upper + 1), the
    // grouping of fd.rs:597-598.
    for (n, lower, upper) in [(40, 2, 2), (40, 1, 3), (7, 3, 3), (5, 4, 4), (64, 0, 0)] {
        let pattern = SparsityPattern::banded(n, lower, upper).unwrap();
        let (colour, count) = colour_columns(&pattern);
        let width = lower + upper + 1;
        assert_eq!(count, width.min(n), "band ({lower}, {upper}) of order {n}");
        for (j, c) in colour.iter().enumerate() {
            assert_eq!(*c, j % width, "column {j} of band ({lower}, {upper})");
        }
        assert!(colouring_is_valid(&pattern, &colour));
        assert_eq!(pattern.bandwidth(), (lower, upper));
        assert_eq!(
            pattern.nnz(),
            (0..n).map(|i| pattern.row(i).len()).sum::<usize>()
        );
    }
    // The cyclic tridiagonal pattern: three groups when 3 divides n. For
    // n = 11 first fit in natural order ends with five (four would do: the
    // grouping is deterministic, not minimal).
    let (colour, count) = colour_columns(&ring_pattern(12));
    assert_eq!(count, 3);
    assert!(colouring_is_valid(&ring_pattern(12), &colour));
    let (colour, count) = colour_columns(&ring_pattern(11));
    assert_eq!(count, 5);
    assert_eq!(colour, [0, 1, 2, 0, 1, 2, 0, 1, 2, 3, 4]);
    assert!(colouring_is_valid(&ring_pattern(11), &colour));
    assert!(!colouring_is_valid(&ring_pattern(11), &[0; 11]));
    // An arrow: the full first row puts every column in its own group.
    let n = 9;
    let mut rows: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    rows[0] = (0..n).collect();
    let arrow = SparsityPattern::from_rows(n, &rows).unwrap();
    assert_eq!(colour_columns(&arrow).1, n);
    assert!(arrow.contains(0, 5) && !arrow.contains(5, 0));
    // Malformed patterns are refused.
    for rows in [
        vec![vec![1, 0], vec![1]],
        vec![vec![0, 2], vec![1]],
        vec![vec![0]],
    ] {
        assert!(matches!(
            SparsityPattern::from_rows(2, &rows),
            Err(AssemblyError::Configuration(_))
        ));
    }
    assert!(SparsityPattern::banded(4, 4, 0).is_err());
    assert!(SparsityPattern::banded(0, 0, 0).is_err());
}

/// The run of a client equals the protected run: statistics, outputs,
/// counters without the right-hand sides of the assembly, the
/// linearization points and the assembled Jacobians.
fn assert_same_run(label: &str, protected: &ProtectedRun, client: &ClientRun) {
    let (reference, run) = (&protected.result, &client.result);
    assert_eq!(run.lu, Rodas5pFastLu::Client, "{label}");
    assert_eq!(run.driver, RODAS5P_FAST_CLIENT_DRIVER_ID, "{label}");
    assert_eq!(run.observed.success, reference.observed.success, "{label}");
    assert_eq!(run.attempts, reference.attempts, "{label}: attempts");
    assert_eq!(
        run.accepted_steps, reference.accepted_steps,
        "{label}: accepted"
    );
    assert_eq!(
        run.rejected_steps, reference.rejected_steps,
        "{label}: rejected"
    );
    assert_eq!(
        run.jacobian_reuses, reference.jacobian_reuses,
        "{label}: reuses"
    );
    assert_eq!(
        run.observed.internal_steps, reference.observed.internal_steps,
        "{label}"
    );
    assert_eq!(
        bits(&run.observed.t),
        bits(&reference.observed.t),
        "{label}: output times"
    );
    assert_eq!(run.observed.y.len(), reference.observed.y.len(), "{label}");
    for (index, (a, b)) in run.observed.y.iter().zip(&reference.observed.y).enumerate() {
        assert_eq!(bits(a), bits(b), "{label}: output state {index}");
    }
    let (t, y) = final_state(run, &client.partial);
    let (reference_t, reference_y) = final_state(reference, &protected.partial);
    assert_eq!(
        t.to_bits(),
        reference_t.to_bits(),
        "{label}: last accepted time"
    );
    assert_eq!(bits(y), bits(reference_y), "{label}: last accepted state");
    assert_eq!(
        client.partial.as_ref().map(|partial| partial.reason),
        protected.partial.as_ref().map(|partial| partial.reason),
        "{label}: stop reason"
    );
    // The protected run's reference finite differences are charged to
    // scratch counters; the client's are charged to the run.
    let stats = &client.stats;
    let assembly = stats.assembly_rhs_calls + stats.verification_rhs_calls;
    assert_eq!(stats.failed_builds, 0, "{label}");
    let mut counters = run.observed.counters;
    counters.rhs_calls -= assembly;
    counters.rhs_evaluations -= assembly;
    assert_eq!(counters, reference.observed.counters, "{label}: counters");
    assert_eq!(stats.builds, protected.trace.builds, "{label}: builds");
    assert_eq!(
        stats.builds, reference.observed.counters.jacobian_builds,
        "{label}: builds charged by the driver"
    );
    assert_eq!(
        stats.state_fingerprint, protected.trace.state_fingerprint,
        "{label}: linearization points"
    );
    assert_eq!(
        stats.jacobian_fingerprint, protected.trace.jacobian_fingerprint,
        "{label}: assembled Jacobians"
    );
    assert_eq!(
        stats.assembly_rhs_calls,
        stats.builds * client.rhs_per_build as u64,
        "{label}: right-hand sides per build"
    );
}

/// The tolerances of `PREDECLARED.json` for the contract tests: `atol` is
/// `rtol` times the case scale of `b1_problems` (1 for the Brusselator) and
/// `1e-2 rtol` for the other problems.
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

#[test]
fn every_mode_reproduces_the_protected_dense_driver() {
    let cases = [
        (nlkpp_dense(24, false), DenseLuMode::InPlace),
        // Above order 64 with a full Jacobian the protected policy picks faer.
        (nlkpp_dense(72, false), DenseLuMode::Faer),
        (brusselator(16, false), DenseLuMode::InPlace),
        (brusselator(40, false), DenseLuMode::InPlace),
        (ring(30, false, false), DenseLuMode::InPlace),
    ];
    let mut accepted = 0;
    for (fd, expected_mode) in &cases {
        let mode =
            AssemblyStageClient::protected_dense_mode(&fd.problem, fd.span.0, &fd.y0).unwrap();
        assert_eq!(mode, *expected_mode, "{}", fd.id);
        let backend = LinearBackend::DenseLu(mode);
        for rtol in RTOLS {
            let config = config(fd, rtol);
            let protected = run_protected_dense(fd, &config).unwrap();
            assert!(protected.result.observed.success, "{} at {rtol:e}", fd.id);
            assert_eq!(protected.result.lu, mode.protected_lu(), "{}", fd.id);
            assert!(protected.result.accepted_steps > 10, "{}", fd.id);
            accepted += protected.result.accepted_steps;
            let mut specs = vec![
                spec(ColumnRoute::Dense, Arm::Seq, backend),
                spec(ColumnRoute::Dense, Arm::Threaded(3), backend),
            ];
            if let Some(pattern) = pattern_of(fd) {
                let route = ColumnRoute::Coloured(pattern);
                specs.push(spec(route.clone(), Arm::Seq, backend));
                specs.push(spec(route, Arm::Threaded(3), backend));
            }
            for spec in &specs {
                let label = format!(
                    "{} at rtol {rtol:e}, {} route, {}",
                    fd.id,
                    if matches!(spec.route, ColumnRoute::Dense) {
                        "dense"
                    } else {
                        "coloured"
                    },
                    spec.arm.name()
                );
                let (_, run) = run_client(fd, spec, &config);
                let run = run.unwrap_or_else(|error| panic!("{label}: {error}"));
                assert_same_run(&label, &protected, &run);
            }
        }
    }
    assert!(accepted > 500, "accepted steps compared: {accepted}");
}

#[test]
fn the_coloured_route_reproduces_the_protected_banded_driver() {
    let cases = [brusselator(32, false), nlkpp_band(96, 4, false)];
    for fd in &cases {
        let (lower, upper) = fd.band.unwrap();
        let route = ColumnRoute::Coloured(SparsityPattern::banded(fd.n, lower, upper).unwrap());
        for rtol in RTOLS {
            let config = config(fd, rtol);
            let protected = run_protected_banded(fd, lower, upper, &config).unwrap();
            assert!(protected.result.observed.success, "{} at {rtol:e}", fd.id);
            assert_eq!(protected.result.lu, Rodas5pFastLu::Banded);
            assert!(protected.result.accepted_steps > 10, "{}", fd.id);
            for backend in [
                LinearBackend::BandedProtected { lower, upper },
                LinearBackend::BandedTight { lower, upper },
            ] {
                for arm in [Arm::Seq, Arm::Threaded(1), Arm::Threaded(3)] {
                    let label = format!("{} at rtol {rtol:e}, {backend:?}, {}", fd.id, arm.name());
                    let (_, run) = run_client(fd, &spec(route.clone(), arm, backend), &config);
                    let run = run.unwrap_or_else(|error| panic!("{label}: {error}"));
                    assert_eq!(run.colours, Some(lower + upper + 1), "{label}");
                    assert_same_run(&label, &protected, &run);
                }
            }
        }
    }
}

#[test]
fn a_run_cut_by_the_attempt_budget_is_reproduced_too() {
    // The run length rule of PREDECLARED.json: the same accepted prefix in
    // every arm and in the protected driver.
    let fd = brusselator(32, false);
    let (lower, upper) = fd.band.unwrap();
    let config = RunConfig {
        max_attempts: 48,
        ..config(&fd, 1.0e-6)
    };
    let protected = run_protected_banded(&fd, lower, upper, &config).unwrap();
    assert!(!protected.result.observed.success);
    assert_eq!(protected.result.attempts, 48);
    assert_eq!(
        protected.partial.as_ref().map(|partial| partial.reason),
        Some(DriverStopReason::AttemptBudget)
    );
    let route = ColumnRoute::Coloured(SparsityPattern::banded(fd.n, lower, upper).unwrap());
    for arm in [Arm::Seq, Arm::Threaded(2)] {
        let backend = LinearBackend::BandedTight { lower, upper };
        let (_, run) = run_client(&fd, &spec(route.clone(), arm, backend), &config);
        assert_same_run(
            &format!("budget, {}", arm.name()),
            &protected,
            &run.unwrap(),
        );
    }
}

#[test]
fn right_hand_side_calls_are_colours_plus_one_per_build() {
    let tolerance = 1.0e-6;
    // (problem, route, colours or n, declared check calls: sequential, threaded)
    let coloured = brusselator(32, true);
    let (lower, upper) = coloured.band.unwrap();
    let pattern = SparsityPattern::banded(coloured.n, lower, upper).unwrap();
    let coloured_config = config(&coloured, tolerance);
    let protected = run_protected_banded(&coloured, lower, upper, &coloured_config).unwrap();
    let stage_calls = protected.result.observed.counters.rhs_calls;
    for (arm, check_calls) in [(Arm::Seq, 8), (Arm::Threaded(2), 9), (Arm::Threaded(4), 9)] {
        coloured.calls.store(0, Ordering::Relaxed);
        let spec = spec(
            ColumnRoute::Coloured(pattern.clone()),
            arm,
            LinearBackend::BandedTight { lower, upper },
        );
        let (_, run) = run_client(&coloured, &spec, &coloured_config);
        let run = run.unwrap();
        let stats = run.stats;
        let label = format!("coloured {}", arm.name());
        assert_eq!(run.colours, Some(5), "{label}");
        assert_eq!(run.rhs_per_build, 6, "{label}");
        assert!(stats.builds > 10, "{label}");
        assert_eq!(stats.assembly_rhs_calls, stats.builds * 6, "{label}");
        assert_eq!(stats.verifications, 1, "{label}");
        assert_eq!(stats.verified_columns, 8, "{label}");
        assert_eq!(stats.verification_rhs_calls, check_calls, "{label}");
        let total = stage_calls + stats.assembly_rhs_calls + stats.verification_rhs_calls;
        assert_eq!(
            run.result.observed.counters.rhs_calls, total,
            "{label}: counters"
        );
        assert_eq!(
            coloured.calls.load(Ordering::Relaxed),
            total,
            "{label}: callback"
        );
    }

    let dense = nlkpp_dense(24, true);
    let dense_config = config(&dense, tolerance);
    let protected = run_protected_dense(&dense, &dense_config).unwrap();
    let stage_calls = protected.result.observed.counters.rhs_calls;
    for (arm, checks, check_calls) in [(Arm::Seq, 0, 0), (Arm::Threaded(3), 1, 9)] {
        dense.calls.store(0, Ordering::Relaxed);
        let (_, run) = run_client(
            &dense,
            &spec(ColumnRoute::Dense, arm, DENSE_IN_PLACE),
            &dense_config,
        );
        let run = run.unwrap();
        let stats = run.stats;
        let label = format!("dense {}", arm.name());
        assert_eq!(run.colours, None, "{label}");
        assert_eq!(stats.assembly_rhs_calls, stats.builds * 25, "{label}");
        assert_eq!(stats.verifications, checks, "{label}");
        assert_eq!(stats.verification_rhs_calls, check_calls, "{label}");
        let total = stage_calls + stats.assembly_rhs_calls + stats.verification_rhs_calls;
        assert_eq!(
            run.result.observed.counters.rhs_calls, total,
            "{label}: counters"
        );
        assert_eq!(
            dense.calls.load(Ordering::Relaxed),
            total,
            "{label}: callback"
        );
    }
}

#[test]
fn checks_can_be_switched_off_and_requested() {
    let fd = brusselator(12, true);
    let (lower, upper) = fd.band.unwrap();
    let route = ColumnRoute::Coloured(SparsityPattern::banded(fd.n, lower, upper).unwrap());
    let backend = LinearBackend::BandedTight { lower, upper };
    let mut counters = WorkCounters::default();

    // No first-use check: every build is colours + 1 calls.
    let mut client = ClientSpec {
        first_use_check: CheckColumns::None,
        ..spec(route.clone(), Arm::Seq, backend)
    }
    .build(fd.n);
    let handle = client.handle();
    for _ in 0..3 {
        client
            .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
            .unwrap();
    }
    assert_eq!(fd.calls.load(Ordering::Relaxed), 18);
    assert_eq!(handle.stats().verifications, 0);
    // A requested full check runs at the next linearization, once.
    handle.request_check(CheckColumns::All);
    client
        .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
        .unwrap();
    client
        .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
        .unwrap();
    let stats = handle.stats();
    assert_eq!(stats.builds, 5);
    assert_eq!(stats.assembly_rhs_calls, 30);
    assert_eq!(
        (stats.verifications, stats.verified_columns),
        (1, fd.n as u64)
    );
    assert_eq!(stats.verification_rhs_calls, fd.n as u64);
    assert_eq!(counters.rhs_calls, 30 + fd.n as u64);
    assert_eq!(fd.calls.load(Ordering::Relaxed), 30 + fd.n as u64);
    // The direct check returns the number of columns it evaluated.
    assert_eq!(
        client.verify(
            &fd.problem,
            0.0,
            &fd.y0,
            CheckColumns::Sample(5),
            &mut counters
        ),
        Ok(5)
    );
    assert_eq!(
        client.verify(
            &fd.problem,
            0.0,
            &fd.y0,
            CheckColumns::Sample(1000),
            &mut counters
        ),
        Ok(fd.n)
    );

    // The first-use check (spec() names Sample(8)) runs once, at the first linearization.
    let mut client = spec(route, Arm::Seq, backend).build(fd.n);
    let handle = client.handle();
    for _ in 0..3 {
        client
            .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
            .unwrap();
    }
    assert_eq!(
        (
            handle.stats().verifications,
            handle.stats().verified_columns
        ),
        (1, 8)
    );

    // The sequential dense route checks nothing unless asked.
    let mut client = spec(ColumnRoute::Dense, Arm::Seq, DENSE_IN_PLACE).build(fd.n);
    let handle = client.handle();
    client
        .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
        .unwrap();
    assert_eq!(handle.stats().verifications, 0);
    assert_eq!(
        client.verify(&fd.problem, 0.0, &fd.y0, CheckColumns::All, &mut counters),
        Ok(fd.n)
    );
}

/// The run must end with `InvalidInput`, nothing built, and a contract
/// error in the handle.
fn assert_fails_closed(label: &str, fd: &FdProblem, spec: &ClientSpec) -> AssemblyError {
    let (handle, run) = run_client(fd, spec, &config(fd, 1.0e-6));
    match run {
        Err(CoreError::InvalidInput(message)) => {
            let error = handle
                .last_error()
                .unwrap_or_else(|| panic!("{label}: no typed error"));
            assert!(error.is_contract_violation(), "{label}: {error:?}");
            assert_eq!(message, error.to_string(), "{label}");
            let stats = handle.stats();
            assert_eq!((stats.builds, stats.failed_builds), (0, 1), "{label}");
            error
        }
        Err(other) => panic!("{label}: wrong error kind: {other}"),
        Ok(run) => panic!(
            "{label}: the run went on ({} accepted steps)",
            run.result.accepted_steps
        ),
    }
}

#[test]
fn a_wrong_pattern_fails_closed() {
    // (1) An omitted coupling that reaches a row its colour does not cover:
    // caught by the first build. The ring declared as a tridiagonal band:
    // column 0 (colour 0) reaches row 11, whose declared columns 10 and 11
    // have colours 1 and 2.
    let ring12 = ring(12, false, false);
    let tridiagonal = SparsityPattern::banded(12, 1, 1).unwrap();
    for arm in [Arm::Seq, Arm::Threaded(3)] {
        let error = assert_fails_closed(
            &format!("uncovered row, {}", arm.name()),
            &ring12,
            &ClientSpec {
                first_use_check: CheckColumns::None,
                ..spec(
                    ColumnRoute::Coloured(tridiagonal.clone()),
                    arm,
                    DENSE_IN_PLACE,
                )
            },
        );
        assert_eq!(
            error,
            AssemblyError::UncoveredRowChanged { row: 11, colour: 0 }
        );
    }

    // (2) An omitted coupling that is aliased into a declared entry of the
    // same colour: every build's own check passes, the first-use check
    // columns catch it.
    let chain = lower_coupled_chain(24);
    let route = ColumnRoute::Coloured(upper_bidiagonal(24));
    for arm in [Arm::Seq, Arm::Threaded(3)] {
        let error = assert_fails_closed(
            &format!("aliased entry, {}", arm.name()),
            &chain,
            &spec(route.clone(), arm, DENSE_IN_PLACE),
        );
        assert!(
            matches!(
                error,
                AssemblyError::UndeclaredEntry { .. } | AssemblyError::AliasedEntry { .. }
            ),
            "{error:?}"
        );
        // Without the check columns the wrong Jacobian would go through:
        // the per-build check alone cannot see an aliased entry.
        let mut unchecked = ClientSpec {
            first_use_check: CheckColumns::None,
            ..spec(route.clone(), arm, DENSE_IN_PLACE)
        }
        .build(chain.n);
        unchecked
            .linearize(&chain.problem, 0.0, &chain.y0, &mut WorkCounters::default())
            .unwrap();
        // (4) A check on request finds it: through the handle at the next
        // linearization, and directly.
        let handle = unchecked.handle();
        handle.request_check(CheckColumns::All);
        let refused =
            unchecked.linearize(&chain.problem, 0.0, &chain.y0, &mut WorkCounters::default());
        assert!(
            matches!(refused, Err(CoreError::InvalidInput(_))),
            "{refused:?}"
        );
        assert_eq!(
            handle.last_error(),
            Some(AssemblyError::UndeclaredEntry { row: 1, column: 0 })
        );
        assert_eq!(unchecked.jacobian_values(), None);
        // One check column in the middle: the declared entry (11, 12)
        // carries the contribution of column 10.
        let direct = unchecked.verify(
            &chain.problem,
            0.0,
            &chain.y0,
            CheckColumns::Sample(1),
            &mut WorkCounters::default(),
        );
        assert!(
            matches!(
                direct,
                Err(AssemblyError::AliasedEntry {
                    row: 11,
                    column: 12,
                    ..
                })
            ),
            "{direct:?}"
        );
    }
    // The same request before a run ends the run before any step.
    let unchecked = ClientSpec {
        first_use_check: CheckColumns::None,
        ..spec(route.clone(), Arm::Seq, DENSE_IN_PLACE)
    };
    let (_, run) = run_client(&chain, &unchecked, &config(&chain, 1.0e-6));
    assert!(
        run.is_ok(),
        "an aliased entry is invisible without check columns"
    );
    let checked = ClientSpec {
        first_use_check: CheckColumns::All,
        ..unchecked
    };
    assert_fails_closed("full check before the run", &chain, &checked);

    // The correct patterns pass the full check.
    let mut correct = spec(
        ColumnRoute::Coloured(ring_pattern(12)),
        Arm::Threaded(2),
        DENSE_IN_PLACE,
    )
    .build(12);
    assert_eq!(
        correct.verify(
            &ring12.problem,
            0.0,
            &ring12.y0,
            CheckColumns::All,
            &mut WorkCounters::default()
        ),
        Ok(12)
    );
}

#[test]
fn an_impure_right_hand_side_fails_closed() {
    let fd = impure(16);
    // Threaded dense route: the repeated base evaluation differs.
    let error = assert_fails_closed(
        "impure, dense threaded",
        &fd,
        &spec(ColumnRoute::Dense, Arm::Threaded(3), DENSE_IN_PLACE),
    );
    assert!(
        matches!(error, AssemblyError::NotReproducible { .. }),
        "{error:?}"
    );
    // Coloured route, either execution: rows no column of a colour covers
    // change from call to call.
    let diagonal = SparsityPattern::banded(16, 0, 0).unwrap();
    for arm in [Arm::Seq, Arm::Threaded(3)] {
        assert_fails_closed(
            &format!("impure, coloured {}", arm.name()),
            &fd,
            &spec(ColumnRoute::Coloured(diagonal.clone()), arm, DENSE_IN_PLACE),
        );
    }
    let tridiagonal = SparsityPattern::banded(16, 1, 1).unwrap();
    assert_fails_closed(
        "impure, coloured tridiagonal",
        &fd,
        &spec(
            ColumnRoute::Coloured(tridiagonal),
            Arm::Threaded(2),
            DENSE_IN_PLACE,
        ),
    );
    // The sequential dense route has no second evaluation to compare with;
    // a requested check finds the difference.
    let mut client = spec(ColumnRoute::Dense, Arm::Seq, DENSE_IN_PLACE).build(16);
    let checked = client.verify(
        &fd.problem,
        0.0,
        &fd.y0,
        CheckColumns::Sample(2),
        &mut WorkCounters::default(),
    );
    assert!(
        matches!(checked, Err(AssemblyError::NotReproducible { .. })),
        "{checked:?}"
    );
}

/// A right-hand side that fails when one of `bad` is shifted away from its
/// start value, and always when `base_fails`.
fn failing(n: usize, bad: &'static [usize], base_fails: bool, overflow: bool) -> FdProblem {
    let y0: Vec<f64> = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect();
    let start = y0.clone();
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            if base_fails {
                return Err(CoreError::NonFinite("the base evaluation fails".into()));
            }
            for i in 0..n {
                out[i] = -y[i];
            }
            for &k in bad {
                if y[k] != start[k] {
                    if overflow {
                        // The difference to the base value overflows.
                        out[0] = f64::MAX;
                    } else {
                        return Err(CoreError::NonFinite(format!("column {k} fails")));
                    }
                } else if overflow {
                    out[0] = -f64::MAX;
                }
            }
            Ok(())
        },
    );
    from_rhs(format!("failing-n{n}"), rhs, y0, (0.0, 1.0), None)
}

#[test]
fn failures_are_reported_as_the_sequential_build_meets_them() {
    let n = 14;
    let dense = |arm| ClientSpec {
        first_use_check: CheckColumns::None,
        ..spec(ColumnRoute::Dense, arm, DENSE_IN_PLACE)
    };
    let linearize = |fd: &FdProblem, arm: Arm| {
        let mut client = dense(arm).build(n);
        let mut counters = WorkCounters::default();
        let outcome = client
            .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
            .map_err(|error| format!("{error:?}"));
        (
            outcome,
            counters.rhs_calls,
            client.handle().stats(),
            client.jacobian_values(),
        )
    };
    // Columns 9 and 5 fail: the sequential build stops at column 5.
    let fd = failing(n, &[9, 5], false, false);
    let (sequential, calls, stats, values) = linearize(&fd, Arm::Seq);
    assert_eq!(
        sequential,
        Err(r#"NonFinite("column 5 fails")"#.to_string())
    );
    assert_eq!(calls, 7, "base and columns 0 ..= 5");
    assert_eq!(
        (
            stats.builds,
            stats.failed_builds,
            stats.failed_build_rhs_calls
        ),
        (0, 1, 7)
    );
    assert_eq!(values, None);
    for workers in WORKERS {
        let (threaded, calls, stats, values) = linearize(&fd, Arm::Threaded(workers));
        assert_eq!(threaded, sequential, "t{workers}");
        // Every right-hand side of the build was entered.
        assert_eq!(calls, n as u64 + 1, "t{workers}");
        assert_eq!(stats.failed_build_rhs_calls, n as u64 + 1, "t{workers}");
        assert_eq!(values, None);
    }
    // The base evaluation fails.
    let fd = failing(n, &[], true, false);
    let (sequential, calls, ..) = linearize(&fd, Arm::Seq);
    assert_eq!(
        sequential,
        Err(r#"NonFinite("the base evaluation fails")"#.to_string())
    );
    assert_eq!(calls, 1);
    assert_eq!(linearize(&fd, Arm::Threaded(3)).0, sequential);
    // An entry that overflows: the same NonFinite in both.
    let fd = failing(n, &[3], false, true);
    let (sequential, ..) = linearize(&fd, Arm::Seq);
    assert_eq!(
        sequential,
        Err(r#"NonFinite("finite-difference Jacobian entry overflowed")"#.to_string())
    );
    assert_eq!(linearize(&fd, Arm::Threaded(3)).0, sequential);
    // A rejectable failure does not poison the client: the next build at a
    // state that works succeeds.
    let fd = failing(n, &[5], false, false);
    let mut client = dense(Arm::Threaded(2)).build(n);
    let mut counters = WorkCounters::default();
    assert!(
        client
            .linearize(&fd.problem, 0.0, &fd.y0, &mut counters)
            .is_err()
    );
    let mut moved = fd.y0.clone();
    moved[5] += 1.0;
    // Column 5 now differs from the start in every evaluation: all fail.
    assert!(
        client
            .linearize(&fd.problem, 0.0, &moved, &mut counters)
            .is_err()
    );
    let healthy = nlkpp_dense(n, false);
    client
        .linearize(&healthy.problem, 0.0, &healthy.y0, &mut counters)
        .unwrap();
    assert!(client.jacobian_values().is_some());
}

#[test]
fn threaded_right_hand_sides_run_on_the_clients_own_pool() {
    let n = 40;
    let names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&names);
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            let name = std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_string();
            seen.lock().unwrap().push(name);
            for i in 0..n {
                out[i] = -y[i] * y[i];
            }
            Ok(())
        },
    );
    let fd = from_rhs("thread-names".into(), rhs, vec![0.5; n], (0.0, 1.0), None);
    let caller = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_string();
    for workers in [1, 3] {
        names.lock().unwrap().clear();
        let mut client = ClientSpec {
            first_use_check: CheckColumns::None,
            ..spec(ColumnRoute::Dense, Arm::Threaded(workers), DENSE_IN_PLACE)
        }
        .build(n);
        assert_eq!(client.workers(), Some(workers));
        for _ in 0..4 {
            client
                .linearize(&fd.problem, 0.0, &fd.y0, &mut WorkCounters::default())
                .unwrap();
        }
        let recorded = std::mem::take(&mut *names.lock().unwrap());
        assert_eq!(recorded.len(), 4 * (n + 1));
        let distinct: BTreeSet<&String> = recorded.iter().collect();
        assert!(distinct.len() <= workers, "{distinct:?}");
        assert!(
            distinct
                .iter()
                .all(|name| name.starts_with("rodas5p-fd-assembly-")),
            "{distinct:?}"
        );
        // The check columns run on the calling thread.
        client
            .verify(
                &fd.problem,
                0.0,
                &fd.y0,
                CheckColumns::Sample(3),
                &mut WorkCounters::default(),
            )
            .unwrap();
        let recorded = std::mem::take(&mut *names.lock().unwrap());
        assert_eq!(recorded.len(), (n + 1) + 1 + 3);
        assert_eq!(recorded.iter().filter(|name| **name == caller).count(), 4);
    }
    // Sequential execution creates no pool and stays on the calling thread.
    names.lock().unwrap().clear();
    let mut client = spec(ColumnRoute::Dense, Arm::Seq, DENSE_IN_PLACE).build(n);
    assert_eq!(client.workers(), None);
    client
        .linearize(&fd.problem, 0.0, &fd.y0, &mut WorkCounters::default())
        .unwrap();
    assert!(names.lock().unwrap().iter().all(|name| *name == caller));
}

#[test]
fn configurations_that_cannot_work_are_refused() {
    let refused = |backend, options: AssemblyOptions, n: usize| {
        matches!(
            AssemblyStageClient::new(n, backend, options),
            Err(AssemblyError::Configuration(_))
        )
    };
    let band = LinearBackend::BandedTight { lower: 1, upper: 1 };
    // Order zero; the dense route on a band; a pattern of another order; a
    // pattern outside the band; a band outside the matrix; no workers.
    assert!(refused(DENSE_IN_PLACE, AssemblyOptions::dense(), 0));
    assert!(refused(band, AssemblyOptions::dense(), 8));
    let pattern = |n, lower, upper| SparsityPattern::banded(n, lower, upper).unwrap();
    assert!(refused(
        DENSE_IN_PLACE,
        AssemblyOptions::coloured(pattern(9, 1, 1)),
        8
    ));
    assert!(refused(
        band,
        AssemblyOptions::coloured(pattern(8, 2, 1)),
        8
    ));
    assert!(refused(band, AssemblyOptions::coloured(ring_pattern(8)), 8));
    assert!(refused(
        LinearBackend::BandedProtected { lower: 8, upper: 0 },
        AssemblyOptions::coloured(pattern(8, 1, 1)),
        8
    ));
    assert!(refused(
        DENSE_IN_PLACE,
        AssemblyOptions::dense().threaded(0, RhsPurity::pure_and_reentrant()),
        8
    ));
    // A pattern inside a wider band is accepted.
    let wide = LinearBackend::BandedProtected { lower: 3, upper: 2 };
    assert!(AssemblyStageClient::new(8, wide, AssemblyOptions::coloured(pattern(8, 1, 1))).is_ok());

    // A problem or a state of another dimension is refused at the call.
    let fd = nlkpp_dense(8, false);
    let other = nlkpp_dense(9, false);
    let mut client = spec(ColumnRoute::Dense, Arm::Seq, DENSE_IN_PLACE).build(8);
    let mut counters = WorkCounters::default();
    assert!(matches!(
        client.linearize(&other.problem, 0.0, &other.y0, &mut counters),
        Err(CoreError::Dimension(_))
    ));
    assert!(matches!(
        client.linearize(&fd.problem, 0.0, &other.y0, &mut counters),
        Err(CoreError::Dimension(_))
    ));
    assert!(matches!(
        client.verify(
            &other.problem,
            0.0,
            &other.y0,
            CheckColumns::All,
            &mut counters
        ),
        Err(AssemblyError::Configuration(_))
    ));
    assert_eq!(counters.rhs_calls, 0);
    // A solve before any factorization is the back end's error.
    assert!(matches!(
        client.solve_in_place(&mut [0.0; 8], &mut counters),
        Err(CoreError::LinearSolve(_))
    ));
}
