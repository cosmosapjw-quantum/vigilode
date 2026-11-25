//! Contracts of the dense LU reuse client (loop node V18,
//! `src/stage_clients/dense_lu_reuse.rs`).
//!
//! (i) Trajectory identity. Under the client driver the sequential client
//! reproduces the protected dense driver bit for bit: step statistics, every
//! counter, every output time and state and every accepted state. Checked on
//! the client-kit identity set (Robertson, HIRES, two banded Brusselators in
//! the in-place mode, the mean-field Brusselator of order 200 in the faer
//! mode, three tolerances each) and on the faer path at the orders 128, 512
//! and 2048 (mean-field Brusselator and skewnet; the order 2048 is an
//! ignored test because it takes minutes).
//!
//! (ii) Kernel identity. The combined factors, the row permutation and the
//! solves of the client equal those of `rodas5p_core::LuFactorization` (the
//! kernel of the protected faer path) on matrices that need row
//! interchanges.
//!
//! (iii) Errors. A singular `W` gives the typed error of the protected
//! path; misuse is refused.
//!
//! (iv) The threaded factorization is a partial-pivoting LU of the same
//! matrix, the same on every run with the same worker count. Whether it
//! equals the sequential factors bit for bit is printed, not asserted.

use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreError, DenseMatrix, LuFactorization, LuSolveWorkspace, WorkCounters};
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::dense_lu_reuse::{
    DENSE_LU_REUSE_MAX_FACTOR_WORKERS, DenseLuReuseStageClient,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, DriverPartial, OdeProblem, RODAS5P_FAST_CLIENT_DRIVER_ID,
    RODAS5P_FAST_DRIVER_ID, Rodas5pFastLu, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol, integrate_rodas5p_fast_observed_with_atol,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;
#[path = "stage_clients_dense_lu_reuse_problems.rs"]
mod problems;

use b1_problems::{Case, brusselator_problem, config, hires_case, robertson_case, schedule};
use problems::{
    Arm, ArmRun, Cell, SkewnetData, Trace, adaptive, fingerprint, max_abs_difference, meanfield,
    meanfield_problem, run_arm, skewnet, tracing_hook,
};

const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];
const STAGES: u64 = 8;

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

// ---------------------------------------------------------------------------
// (i) the client-kit identity set, with the configuration of the kit's test.
// ---------------------------------------------------------------------------

type Point = (u64, Vec<u64>);
type Points = Arc<Mutex<Vec<Point>>>;

/// `problem` with every callback forwarded and every in-place Jacobian
/// request recorded (`stage_clients_dense_identity.rs::traced`).
fn traced(problem: &OdeProblem) -> (OdeProblem, Points) {
    let trace: Points = Arc::new(Mutex::new(Vec::new()));
    let for_rhs = problem.clone();
    let for_jacobian = problem.clone();
    let for_into = problem.clone();
    let recorder = Arc::clone(&trace);
    let traced = OdeProblem::new(
        format!("{} (traced)", problem.name),
        problem.dimension,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            for_rhs.eval_rhs_into(t, y, out, &mut WorkCounters::default())
        }),
        None,
        Some(Arc::new(move |t: f64, y: &[f64]| {
            for_jacobian.dense_jacobian(t, y, &mut WorkCounters::default())
        })),
        None,
        None,
        problem.autonomous,
        None,
        None,
    )
    .unwrap()
    .with_jacobian_into(Arc::new(move |t: f64, y: &[f64], out: &mut DenseMatrix| {
        recorder.lock().unwrap().push((t.to_bits(), bits(y)));
        for_into.dense_jacobian_into(t, y, out, &mut WorkCounters::default())
    }));
    (traced, trace)
}

fn take(trace: &Points) -> Vec<Point> {
    std::mem::take(&mut *trace.lock().unwrap())
}

/// The step statistics, counters and outputs of `client` are those of the
/// protected run, bit for bit.
fn assert_same_result(
    label: &str,
    mode: DenseLuMode,
    protected: &(Rodas5pFastResult, Option<DriverPartial>),
    client: &(Rodas5pFastResult, Option<DriverPartial>),
) {
    let (reference, reference_partial) = protected;
    let (client, client_partial) = client;
    assert!(reference.observed.success, "{label}");
    assert!(client.observed.success, "{label}");
    assert_eq!(reference_partial, &None, "{label}");
    assert_eq!(client_partial, &None, "{label}");
    assert_eq!(reference.lu, mode.protected_lu(), "{label}");
    assert_eq!(reference.driver, RODAS5P_FAST_DRIVER_ID, "{label}");
    assert_eq!(client.lu, Rodas5pFastLu::Client, "{label}");
    assert_eq!(client.driver, RODAS5P_FAST_CLIENT_DRIVER_ID, "{label}");
    assert_eq!(client.attempts, reference.attempts, "{label}");
    assert_eq!(client.accepted_steps, reference.accepted_steps, "{label}");
    assert_eq!(client.rejected_steps, reference.rejected_steps, "{label}");
    assert_eq!(client.jacobian_reuses, reference.jacobian_reuses, "{label}");
    assert_eq!(
        client.observed.internal_steps, reference.observed.internal_steps,
        "{label}"
    );
    assert_eq!(
        client.observed.output_clipped_steps, reference.observed.output_clipped_steps,
        "{label}"
    );
    assert_eq!(
        client.observed.counters, reference.observed.counters,
        "{label}"
    );
    assert_eq!(
        bits(&client.observed.t),
        bits(&reference.observed.t),
        "{label}"
    );
    assert_eq!(
        client.observed.y.len(),
        reference.observed.y.len(),
        "{label}"
    );
    for (index, (a, b)) in client
        .observed
        .y
        .iter()
        .zip(&reference.observed.y)
        .enumerate()
    {
        assert_eq!(bits(a), bits(b), "{label}: output state {index}");
    }
}

fn kit_brusselator(cells: usize, id: &'static str) -> Case {
    let (problem, y0) = brusselator_problem(cells).unwrap();
    Case {
        id,
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

fn kit_meanfield() -> Case {
    let (problem, y0) = meanfield_problem(100, 0.1, None).unwrap();
    Case {
        id: "brusselator-1d-100-meanfield (n = 200, dense Jacobian)",
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

/// (i) for one kit case at the three tolerances. Returns the rejected
/// steps over the tolerances.
fn assert_reuse_reproduces_protected_on_kit_case(case: &Case, expected: DenseLuMode) -> usize {
    let n = case.problem.dimension;
    let mode = DenseLuMode::protected_choice_at(&case.problem, case.span.0, &case.y0).unwrap();
    assert_eq!(mode, expected, "{}: the protected policy's LU", case.id);
    let (problem, trace) = traced(&case.problem);
    let mut rejected = 0;
    for rtol in RTOLS {
        let label = format!("{} at rtol {rtol:e}, {mode:?}", case.id);
        let adaptive = config(case, rtol);
        let atol = vec![adaptive.atol; n];
        let output = schedule(case.span, 10);
        let protected = integrate_rodas5p_fast_observed_with_atol(
            &problem, case.span, &case.y0, &adaptive, &output, &atol,
        )
        .unwrap();
        let protected_trace = take(&trace);
        let client =
            DenseLuReuseStageClient::for_protected_run(&problem, case.span.0, &case.y0).unwrap();
        // The choice of the mode evaluated the Jacobian once.
        take(&trace);
        assert_eq!(client.mode(), mode, "{label}");
        assert_eq!(client.factor_workers(), 1, "{label}");
        let (result, partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
            &problem,
            Box::new(client),
            case.span,
            &case.y0,
            &adaptive,
            &output,
            &atol,
        )
        .unwrap();
        let client_trace = take(&trace);
        assert_same_result(&label, mode, &protected, &(result, partial));
        assert_eq!(
            protected_trace, client_trace,
            "{label}: accepted trajectory"
        );
        let reference = &protected.0;
        let counters = &reference.observed.counters;
        assert_eq!(protected_trace.len(), reference.accepted_steps, "{label}");
        assert_eq!(
            counters.direct_factorizations, reference.attempts as u64,
            "{label}"
        );
        assert_eq!(
            counters.direct_solve_calls,
            STAGES * reference.attempts as u64,
            "{label}"
        );
        println!(
            "DENSE_LU_REUSE_IDENTITY_JSON {}",
            serde_json::json!({
                "set": "client-kit",
                "case": case.id,
                "n": n,
                "rtol": rtol,
                "atol": adaptive.atol,
                "client_mode": format!("{mode:?}"),
                "protected_lu": format!("{:?}", reference.lu),
                "attempts": reference.attempts,
                "accepted_steps": reference.accepted_steps,
                "rejected_steps": reference.rejected_steps,
                "direct_factorizations": counters.direct_factorizations,
                "direct_solve_calls": counters.direct_solve_calls,
                "output_states_compared": reference.observed.y.len(),
                "accepted_states_compared": protected_trace.len(),
                "bit_identical": true,
            })
        );
        rejected += reference.rejected_steps;
    }
    rejected
}

#[test]
fn reuse_client_reproduces_protected_driver_on_the_client_kit_set() {
    let robertson = robertson_case();
    assert_eq!(robertson.problem.dimension, 3);
    assert_reuse_reproduces_protected_on_kit_case(&robertson, DenseLuMode::InPlace);
    let hires = hires_case();
    assert_eq!(hires.problem.dimension, 8);
    assert_reuse_reproduces_protected_on_kit_case(&hires, DenseLuMode::InPlace);
    let small = kit_brusselator(32, "brusselator-1d-32 (n = 64)");
    assert_eq!(small.problem.dimension, 64);
    assert_reuse_reproduces_protected_on_kit_case(&small, DenseLuMode::InPlace);
    let banded = kit_brusselator(100, "brusselator-1d-100 (n = 200)");
    assert_eq!(banded.problem.dimension, 200);
    let rejected = assert_reuse_reproduces_protected_on_kit_case(&banded, DenseLuMode::InPlace);
    assert!(rejected > 0, "the rejection path was not exercised");
    let meanfield = kit_meanfield();
    assert_eq!(meanfield.problem.dimension, 200);
    let rejected = assert_reuse_reproduces_protected_on_kit_case(&meanfield, DenseLuMode::Faer);
    assert!(rejected > 0, "the rejection path was not exercised");
}

// ---------------------------------------------------------------------------
// (i) the faer path at the orders 128, 512 and 2048.
// ---------------------------------------------------------------------------

/// What one arm of a cell did, for the comparison.
struct Observed {
    run: ArmRun,
    trace_states: usize,
    trace_hash: String,
}

fn observe(
    cell: &Cell,
    trace: &Arc<Mutex<Trace>>,
    arm: Arm,
    config: &AdaptiveStepConfig,
) -> Observed {
    *trace.lock().unwrap() = Trace::default();
    let run = run_arm(cell, &cell.problem, arm, DenseLuMode::Faer, config).unwrap();
    let trace = trace.lock().unwrap();
    Observed {
        run,
        trace_states: trace.states,
        trace_hash: trace.hash.hex(),
    }
}

/// The first quantity in which `other` differs from `reference`, or `None`.
fn first_difference(reference: &Observed, other: &Observed) -> Option<String> {
    let (a, b) = (&reference.run.result, &other.run.result);
    let differs = |name: &str, x: String, y: String| -> Option<String> {
        (x != y).then(|| format!("{name}: {x} vs {y}"))
    };
    differs("attempts", a.attempts.to_string(), b.attempts.to_string())
        .or_else(|| {
            differs(
                "accepted steps",
                a.accepted_steps.to_string(),
                b.accepted_steps.to_string(),
            )
        })
        .or_else(|| {
            differs(
                "rejected steps",
                a.rejected_steps.to_string(),
                b.rejected_steps.to_string(),
            )
        })
        .or_else(|| {
            differs(
                "Jacobian reuses",
                a.jacobian_reuses.to_string(),
                b.jacobian_reuses.to_string(),
            )
        })
        .or_else(|| {
            differs(
                "counters",
                format!("{:?}", a.observed.counters),
                format!("{:?}", b.observed.counters),
            )
        })
        .or_else(|| {
            differs(
                "success",
                a.observed.success.to_string(),
                b.observed.success.to_string(),
            )
        })
        .or_else(|| {
            differs(
                "accepted states",
                reference.trace_states.to_string(),
                other.trace_states.to_string(),
            )
        })
        .or_else(|| {
            differs(
                "accepted trajectory fingerprint",
                reference.trace_hash.clone(),
                other.trace_hash.clone(),
            )
        })
        .or_else(|| {
            let ((ta, ya), (tb, yb)) = (reference.run.end(), other.run.end());
            differs(
                "end state fingerprint",
                fingerprint(ta, ya),
                fingerprint(tb, yb),
            )
        })
        .or_else(|| {
            differs(
                "output times",
                format!("{:?}", bits(&a.observed.t)),
                format!("{:?}", bits(&b.observed.t)),
            )
        })
}

/// (i) for one faer-path cell: the kit client and the reuse client equal
/// the protected driver at every tolerance. `max_attempts` bounds each run.
fn assert_reuse_reproduces_protected_on_cell(
    cell: &Cell,
    trace: &Arc<Mutex<Trace>>,
    max_attempts: usize,
) {
    let mode = DenseLuMode::protected_choice_at(&cell.problem, cell.span.0, &cell.y0).unwrap();
    assert_eq!(
        mode,
        DenseLuMode::Faer,
        "{}: the protected policy's LU",
        cell.id
    );
    for rtol in RTOLS {
        let config = adaptive(cell, rtol, max_attempts);
        let label = format!("{} at rtol {rtol:e}", cell.id);
        let protected = observe(cell, trace, Arm::Protected, &config);
        assert_eq!(protected.run.result.lu, Rodas5pFastLu::Faer, "{label}");
        let counters = &protected.run.result.observed.counters;
        assert_eq!(
            counters.direct_factorizations, protected.run.result.attempts as u64,
            "{label}"
        );
        for arm in [Arm::Kit, Arm::Reuse] {
            let other = observe(cell, trace, arm, &config);
            let difference = first_difference(&protected, &other);
            assert_eq!(difference, None, "{label}, arm {}", arm.name());
            assert_eq!(other.run.result.lu, Rodas5pFastLu::Client, "{label}");
            let (t_end, y_end) = protected.run.end();
            println!(
                "DENSE_LU_REUSE_IDENTITY_JSON {}",
                serde_json::json!({
                    "set": "faer-path",
                    "case": cell.id,
                    "family": cell.family,
                    "n": cell.n,
                    "rtol": rtol,
                    "atol": config.atol,
                    "span": [cell.span.0, cell.span.1],
                    "max_attempts": max_attempts,
                    "arm": arm.name(),
                    "complete": protected.run.partial.is_none(),
                    "attempts": protected.run.result.attempts,
                    "accepted_steps": protected.run.result.accepted_steps,
                    "rejected_steps": protected.run.result.rejected_steps,
                    "direct_factorizations": counters.direct_factorizations,
                    "direct_solve_calls": counters.direct_solve_calls,
                    "accepted_states_compared": protected.trace_states,
                    "accepted_trajectory_fnv1a64": protected.trace_hash,
                    "end_time": t_end,
                    "end_state_fnv1a64": fingerprint(t_end, y_end),
                    "rows_moved_by_last_factorization": other.run.permuted_rows,
                    "bit_identical": true,
                })
            );
        }
    }
}

#[test]
fn reuse_client_reproduces_protected_driver_on_the_faer_path_n128_n512() {
    for n in [128, 512] {
        let (hook, trace) = tracing_hook();
        let cell = meanfield(n, 10.0, Some(hook));
        assert_reuse_reproduces_protected_on_cell(&cell, &trace, 1_000_000);
        let (hook, trace) = tracing_hook();
        let data = Arc::new(SkewnetData::new(n));
        let cell = skewnet(&data, 2.0, Some(hook));
        assert_reuse_reproduces_protected_on_cell(&cell, &trace, 1_000_000);
    }
}

/// The order 2048: complete runs of skewnet on `[0, 1]`, and the first 40
/// attempts of the mean-field Brusselator (a run that stops at the attempt
/// budget is compared at its last accepted state). Run with
/// `-- --ignored --nocapture`.
#[test]
#[ignore = "the order 2048 takes minutes: run with --ignored --nocapture"]
fn reuse_client_reproduces_protected_driver_on_the_faer_path_n2048() {
    let n = 2048;
    let (hook, trace) = tracing_hook();
    let data = Arc::new(SkewnetData::new(n));
    let cell = skewnet(&data, 1.0, Some(hook));
    assert_reuse_reproduces_protected_on_cell(&cell, &trace, 1_000_000);
    let (hook, trace) = tracing_hook();
    let cell = meanfield(n, 10.0, Some(hook));
    assert_reuse_reproduces_protected_on_cell(&cell, &trace, 40);
}

// ---------------------------------------------------------------------------
// (ii) kernel identity.
// ---------------------------------------------------------------------------

/// A problem of order `n` whose Jacobian is the constant matrix `jacobian`.
fn constant_jacobian_problem(jacobian: DenseMatrix) -> OdeProblem {
    let n = jacobian.nrows();
    OdeProblem::new(
        "constant Jacobian",
        n,
        Arc::new(|_t: f64, _y: &[f64], out: &mut [f64]| {
            out.fill(0.0);
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, _y: &[f64]| Ok(jacobian.clone()))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// A value in `[-1, 1)` from `(seed, i, j)`.
fn mixed(seed: u64, i: usize, j: usize) -> f64 {
    let mut z = (seed << 48 | (i as u64) << 24 | j as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z >> 11) as f64 * (2.0 / 9_007_199_254_740_992.0) - 1.0
}

/// A full Jacobian without diagonal dominance.
fn mixed_jacobian(n: usize, seed: u64) -> DenseMatrix {
    let mut jacobian = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            jacobian[(i, j)] = mixed(seed, i, j);
        }
    }
    jacobian
}

/// `W = I/(h gamma) - J` as the protected driver assembles it.
fn protected_w(jacobian: &DenseMatrix, inv_h_gamma: f64) -> DenseMatrix {
    let n = jacobian.nrows();
    let mut w = DenseMatrix::zeros(n, n);
    for (target, j) in w.as_mut_slice().iter_mut().zip(jacobian.as_slice()) {
        *target = -j;
    }
    for i in 0..n {
        w[(i, i)] += inv_h_gamma;
    }
    w
}

/// A reuse client in the faer mode that has linearized `jacobian` and
/// factored `W` for `inv_h_gamma`.
fn factored_client(
    jacobian: &DenseMatrix,
    inv_h_gamma: f64,
    workers: usize,
) -> DenseLuReuseStageClient {
    let n = jacobian.nrows();
    let problem = constant_jacobian_problem(jacobian.clone());
    let mut client = DenseLuReuseStageClient::new(n, DenseLuMode::Faer)
        .unwrap()
        .with_factor_workers(workers)
        .unwrap();
    let mut counters = WorkCounters::default();
    client
        .linearize(&problem, 0.0, &vec![0.0; n], &mut counters)
        .unwrap();
    client.factor(inv_h_gamma, &mut counters).unwrap();
    assert_eq!(counters.direct_factorizations, 1);
    client
}

fn right_hand_side(n: usize, seed: u64) -> Vec<f64> {
    (0..n).map(|i| 3.0 * mixed(seed, i, n + 7)).collect()
}

#[test]
fn factors_permutation_and_solves_equal_the_protected_kernel() {
    let mut moved_total = 0;
    for (index, n) in [1, 2, 3, 5, 16, 17, 33, 64, 65, 100, 129, 200, 257]
        .into_iter()
        .enumerate()
    {
        for inv_h_gamma in [0.0, 0.37, 41.5] {
            let jacobian = mixed_jacobian(n, index as u64 + 1);
            let w = protected_w(&jacobian, inv_h_gamma);
            // The protected kernel, as `LuFactorization::new` calls it.
            let reference = w.to_faer().partial_piv_lu();
            let mut client = factored_client(&jacobian, inv_h_gamma, 1);
            let label = format!("n = {n}, 1/(h gamma) = {inv_h_gamma}");
            let (forward, _) = reference.P().arrays();
            assert_eq!(client.row_permutation().unwrap(), forward, "{label}");
            moved_total += forward
                .iter()
                .enumerate()
                .filter(|(position, row)| position != *row)
                .count();
            for i in 0..n {
                for j in 0..n {
                    let expected = if i > j {
                        reference.L()[(i, j)]
                    } else {
                        reference.U()[(i, j)]
                    };
                    assert_eq!(
                        client.combined_factor(i, j).unwrap().to_bits(),
                        expected.to_bits(),
                        "{label}: factor entry ({i}, {j})"
                    );
                }
            }
            assert_eq!(client.combined_factor(n, 0), None, "{label}");
            // The solves, against the protected driver's solve call.
            let protected = LuFactorization::new(&w).unwrap();
            let mut workspace = LuSolveWorkspace::new(n);
            for seed in 0..3 {
                let mut expected = right_hand_side(n, 100 + seed);
                let mut solved = expected.clone();
                protected
                    .solve_in_place_with_workspace(&mut expected, &mut workspace)
                    .unwrap();
                let mut counters = WorkCounters::default();
                client.solve_in_place(&mut solved, &mut counters).unwrap();
                assert_eq!(counters.direct_solve_calls, 1, "{label}");
                assert_eq!(bits(&solved), bits(&expected), "{label}: solve {seed}");
            }
        }
    }
    assert!(
        moved_total > 1000,
        "the matrices did not exercise row interchanges: {moved_total} rows moved"
    );
}

// ---------------------------------------------------------------------------
// (iii) errors.
// ---------------------------------------------------------------------------

#[test]
fn singular_matrix_gives_the_typed_error_of_the_protected_path() {
    // Faer mode. W = 0 (J = I, 1/(h gamma) = 1) and a W with two equal
    // rows: the protected kernel factors without an error and its solve
    // reports the non-finite result.
    let n = 70;
    let mut equal_rows = mixed_jacobian(n, 9);
    for j in 0..n {
        equal_rows[(5, j)] = equal_rows[(40, j)];
    }
    for (label, jacobian, inv_h_gamma) in [
        ("zero matrix", DenseMatrix::identity(n), 1.0),
        ("two equal rows", equal_rows, 0.0),
    ] {
        let w = protected_w(&jacobian, inv_h_gamma);
        let protected = LuFactorization::new(&w).unwrap();
        let mut workspace = LuSolveWorkspace::new(n);
        let mut expected = right_hand_side(n, 3);
        let expected_outcome =
            protected.solve_in_place_with_workspace(&mut expected, &mut workspace);
        let mut client = factored_client(&jacobian, inv_h_gamma, 1);
        let mut solved = right_hand_side(n, 3);
        let mut counters = WorkCounters::default();
        let outcome = client.solve_in_place(&mut solved, &mut counters);
        assert_eq!(counters.direct_solve_calls, 1, "{label}");
        match (&expected_outcome, &outcome) {
            (Err(CoreError::LinearSolve(a)), Err(CoreError::LinearSolve(b))) => {
                assert_eq!(a, b, "{label}");
            }
            (Ok(()), Ok(())) => assert_eq!(bits(&solved), bits(&expected), "{label}"),
            other => panic!("{label}: protected and client disagree: {other:?}"),
        }
        if label == "zero matrix" {
            assert!(
                matches!(outcome, Err(CoreError::LinearSolve(_))),
                "{label}: {outcome:?}"
            );
        }
    }

    // A non-finite right-hand side is refused before the solve, with the
    // error of the protected kernel.
    let jacobian = mixed_jacobian(n, 4);
    let w = protected_w(&jacobian, 2.0);
    let protected = LuFactorization::new(&w).unwrap();
    let mut workspace = LuSolveWorkspace::new(n);
    let mut bad = right_hand_side(n, 5);
    bad[3] = f64::NAN;
    let expected = protected
        .solve_in_place_with_workspace(&mut bad.clone(), &mut workspace)
        .unwrap_err();
    let mut client = factored_client(&jacobian, 2.0, 1);
    let mut counters = WorkCounters::default();
    let got = client.solve_in_place(&mut bad, &mut counters).unwrap_err();
    assert!(matches!(got, CoreError::NonFinite(_)), "{got:?}");
    assert_eq!(format!("{got:?}"), format!("{expected:?}"));

    // In-place mode: the factorization itself reports the singular W and
    // leaves no factors, as in the client kit.
    let identity = constant_jacobian_problem(DenseMatrix::identity(2));
    let mut client = DenseLuReuseStageClient::new(2, DenseLuMode::InPlace).unwrap();
    let mut counters = WorkCounters::default();
    client
        .linearize(&identity, 0.0, &[1.0, 1.0], &mut counters)
        .unwrap();
    assert!(matches!(
        client.factor(1.0, &mut counters),
        Err(CoreError::LinearSolve(_))
    ));
    assert_eq!(counters.direct_factorizations, 1);
    assert!(matches!(
        client.solve_in_place(&mut [1.0, 1.0], &mut counters),
        Err(CoreError::LinearSolve(_))
    ));
}

#[test]
fn misuse_is_refused() {
    assert!(matches!(
        DenseLuReuseStageClient::new(0, DenseLuMode::Faer),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        DenseLuReuseStageClient::new(0, DenseLuMode::InPlace),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        DenseLuReuseStageClient::new(usize::MAX, DenseLuMode::Faer),
        Err(CoreError::InvalidInput(_))
    ));
    for mode in [DenseLuMode::InPlace, DenseLuMode::Faer] {
        let mut client = DenseLuReuseStageClient::new(3, mode).unwrap();
        assert_eq!(client.dimension(), 3);
        assert_eq!(client.mode(), mode);
        assert_eq!(client.factor_workers(), 1);
        assert_eq!(client.row_permutation(), None);
        assert_eq!(client.combined_factor(0, 0), None);
        let mut counters = WorkCounters::default();
        // No factors yet.
        assert!(matches!(
            client.solve_in_place(&mut [1.0, 2.0, 3.0], &mut counters),
            Err(CoreError::LinearSolve(_))
        ));
        // A wrong right-hand side length is refused before any work.
        let before = counters.direct_solve_calls;
        assert!(matches!(
            client.solve_in_place(&mut [1.0, 2.0], &mut counters),
            Err(CoreError::Dimension(_))
        ));
        assert_eq!(counters.direct_solve_calls, before);
        // The driver's charge of the Jacobian build is the only one kept.
        let case = robertson_case();
        counters.jacobian_builds += 1;
        client
            .linearize(&case.problem, 0.0, &case.y0, &mut counters)
            .unwrap();
        assert_eq!(counters.jacobian_builds, 1);
        assert_eq!(client.jacobian()[(0, 0)], -0.04);
        client.factor(1.0e3, &mut counters).unwrap();
        let mut x = [1.0, 2.0, 3.0];
        client.solve_in_place(&mut x, &mut counters).unwrap();
        assert!(x.iter().all(|v| v.is_finite()));
    }
    // Worker counts.
    let faer = || DenseLuReuseStageClient::new(80, DenseLuMode::Faer).unwrap();
    assert!(matches!(
        faer().with_factor_workers(0),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        faer().with_factor_workers(DENSE_LU_REUSE_MAX_FACTOR_WORKERS + 1),
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(faer().with_factor_workers(1).unwrap().factor_workers(), 1);
    assert_eq!(faer().with_factor_workers(3).unwrap().factor_workers(), 3);
    let in_place = || DenseLuReuseStageClient::new(3, DenseLuMode::InPlace).unwrap();
    assert!(matches!(
        in_place().with_factor_workers(2),
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(
        in_place().with_factor_workers(1).unwrap().factor_workers(),
        1
    );
}

// ---------------------------------------------------------------------------
// (iv) the threaded factorization.
// ---------------------------------------------------------------------------

#[test]
fn threaded_factorization_is_a_partial_pivoting_lu() {
    for (index, n) in [65, 130, 200, 257, 400, 512].into_iter().enumerate() {
        let jacobian = mixed_jacobian(n, 20 + index as u64);
        let inv_h_gamma = 0.25;
        let w = protected_w(&jacobian, inv_h_gamma);
        let sequential = factored_client(&jacobian, inv_h_gamma, 1);
        let w_norm = w.as_slice().iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        for workers in [2, 4, 6] {
            let label = format!("n = {n}, {workers} workers");
            let mut client = factored_client(&jacobian, inv_h_gamma, workers);
            assert_eq!(client.factor_workers(), workers, "{label}");
            let rows = client.row_permutation().unwrap().to_vec();
            let mut seen = vec![false; n];
            for row in &rows {
                assert!(!seen[*row], "{label}: row {row} twice");
                seen[*row] = true;
            }
            // |L| <= 1 and P W = L U to rounding.
            let factor = |i: usize, j: usize| client.combined_factor(i, j).unwrap();
            let mut worst = 0.0_f64;
            for i in 0..n {
                for j in 0..n {
                    if i > j {
                        assert!(factor(i, j).abs() <= 1.0, "{label}: multiplier ({i}, {j})");
                    }
                    let mut sum = 0.0;
                    for k in 0..=i.min(j) {
                        let l = if k == i { 1.0 } else { factor(i, k) };
                        sum += l * factor(k, j);
                    }
                    worst = worst.max((sum - w[(rows[i], j)]).abs());
                }
            }
            let bound = 64.0 * n as f64 * f64::EPSILON * w_norm;
            assert!(
                worst <= bound,
                "{label}: |P W - L U| = {worst:e} > {bound:e}"
            );
            // The same bits on a second factorization.
            let again = factored_client(&jacobian, inv_h_gamma, workers);
            assert_eq!(again.row_permutation().unwrap(), &rows[..], "{label}");
            let mut equals_sequential = sequential.row_permutation().unwrap() == &rows[..];
            for i in 0..n {
                for j in 0..n {
                    assert_eq!(
                        again.combined_factor(i, j).unwrap().to_bits(),
                        factor(i, j).to_bits(),
                        "{label}: repeat differs at ({i}, {j})"
                    );
                    equals_sequential &= sequential.combined_factor(i, j).unwrap().to_bits()
                        == factor(i, j).to_bits();
                }
            }
            // A solve agrees with the sequential one to rounding.
            let mut expected = right_hand_side(n, 31);
            let mut solved = expected.clone();
            let mut counters = WorkCounters::default();
            let mut reference = factored_client(&jacobian, inv_h_gamma, 1);
            reference
                .solve_in_place(&mut expected, &mut counters)
                .unwrap();
            client.solve_in_place(&mut solved, &mut counters).unwrap();
            let scale = expected.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
            let difference = max_abs_difference(&solved, &expected);
            assert!(
                difference <= 1.0e-9 * scale,
                "{label}: solve differs by {difference:e} (scale {scale:e})"
            );
            println!(
                "DENSE_LU_REUSE_THREADED_JSON {}",
                serde_json::json!({
                    "n": n,
                    "workers": workers,
                    "max_abs_pw_minus_lu": worst,
                    "bound": bound,
                    "factors_equal_sequential_bits": equals_sequential,
                    "solve_max_abs_difference_to_sequential": difference,
                    "solve_scale": scale,
                })
            );
        }
    }
}

#[test]
fn threaded_run_follows_the_sequential_run() {
    let data = Arc::new(SkewnetData::new(160));
    let cell = skewnet(&data, 1.0, None);
    let exact = cell.exact_end.clone().unwrap();
    for rtol in [1.0e-4, 1.0e-6] {
        let config = adaptive(&cell, rtol, 1_000_000);
        let run =
            |arm: Arm| run_arm(&cell, &cell.problem, arm, DenseLuMode::Faer, &config).unwrap();
        let sequential = run(Arm::Reuse);
        assert!(sequential.result.observed.success);
        let error = max_abs_difference(sequential.end().1, &exact);
        assert!(error <= 1.0e3 * rtol, "rtol {rtol:e}: error {error:e}");
        for workers in [2, 3] {
            let threaded = run(Arm::Threaded(workers));
            assert!(threaded.result.observed.success);
            let attempts = (
                sequential.result.attempts as f64,
                threaded.result.attempts as f64,
            );
            assert!(
                (attempts.0 - attempts.1).abs() <= 0.01 * attempts.0 + 1.0,
                "rtol {rtol:e}, {workers} workers: attempts {attempts:?}"
            );
            let difference = max_abs_difference(threaded.end().1, sequential.end().1);
            assert!(
                difference <= 1.0e-3 * rtol,
                "rtol {rtol:e}, {workers} workers: end states differ by {difference:e}"
            );
            println!(
                "DENSE_LU_REUSE_THREADED_RUN_JSON {}",
                serde_json::json!({
                    "case": cell.id,
                    "n": cell.n,
                    "rtol": rtol,
                    "workers": workers,
                    "attempts_sequential": sequential.result.attempts,
                    "attempts_threaded": threaded.result.attempts,
                    "error_sequential": error,
                    "end_state_max_abs_difference": difference,
                    "end_state_bit_identical":
                        bits(threaded.end().1) == bits(sequential.end().1),
                })
            );
        }
    }
}
