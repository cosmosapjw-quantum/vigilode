//! Contracts of the stage client kit (loop node V15,
//! `src/stage_clients/{dense_lu,timed}.rs`).
//!
//! (i) [`DenseLuStageClient`] under the client driver reproduces the
//! protected dense driver ([`integrate_rodas5p_fast_observed_with_atol`],
//! default options) bit for bit: the step statistics, every counter, every
//! output time and state, and every accepted time and state. The accepted
//! states are read where both drivers expose them: the Jacobian callback is
//! called once per accepted state (the start of each step), so a recording
//! callback sees the whole accepted trajectory of either driver; the client
//! driver's accept control gives a second, independent view of its own.
//!
//! The client's LU mode is the one the protected policy picks for the
//! problem. The policy offers faer only above order 64 with more than 10 %
//! nonzeros, which none of Robertson, HIRES and the 1D Brusselator reach
//! (the Brusselator's Jacobian is banded), so the faer mode is compared on
//! a Brusselator with a mean-field coupling whose Jacobian is half full.
//!
//! (ii) [`TimedStageClient`] is result-neutral and its call counts are the
//! driver's counters: one factorization per attempt, eight solves per
//! attempt, one linearization per Jacobian build.
//!
//! The ignored test prints diagnostic wall ratios; run it alone.

use std::hint::black_box;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::stage_clients::{
    DenseLuMode, DenseLuStageClient, PhaseTimes, TimedStageClient,
};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, DriverPartial, OdeProblem, OutputSchedule,
    RODAS5P_FAST_CLIENT_DRIVER_ID, RODAS5P_FAST_DRIVER_ID, RODAS5P_FAST_SMALL_LU_MAX,
    Rodas5pFastLu, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
    integrate_rodas5p_fast_observed_with_atol,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;
use b1_problems::{Case, brusselator_problem, config, hires_case, robertson_case, schedule};

const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];
/// RODAS5P stages: the stage solves of one attempt.
const STAGES: u64 = 8;

fn brusselator_case(cells: usize, id: &'static str) -> Case {
    let (problem, y0) = brusselator_problem(cells).unwrap();
    Case {
        id,
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

/// The 1D Brusselator of `b1_problems` with a mean-field coupling
/// `kappa (mean(u) - u_i)` on `u` and the same on `v`. The `u-u` and `v-v`
/// blocks of its Jacobian are full (half of all entries), so the protected
/// policy picks faer above order 64.
fn meanfield_brusselator_problem(cells: usize, kappa: f64) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let mean = kappa / cells as f64;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (mut ubar, mut vbar) = (0.0, 0.0);
        for i in 0..cells {
            ubar += y[2 * i];
            vbar += y[2 * i + 1];
        }
        ubar /= cells as f64;
        vbar /= cells as f64;
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur) + kappa * (ubar - u);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr) + kappa * (vbar - v);
        }
        Ok(())
    });
    let n = 2 * cells;
    // Writes every entry of the fixed pattern: both full blocks and the
    // local u-v couplings.
    let fill = move |y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            for k in 0..cells {
                j[(2 * i, 2 * k)] = mean;
                j[(2 * i + 1, 2 * k + 1)] = mean;
            }
        }
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = mean + 2.0 * u * v - 4.0 - 2.0 * c - kappa;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = mean - u * u - 2.0 * c - kappa;
            if i > 0 {
                j[(a, a - 2)] = mean + c;
                j[(b, b - 2)] = mean + c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = mean + c;
                j[(b, b + 2)] = mean + c;
            }
        }
    };
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(move |_t: f64, y: &[f64], j: &mut DenseMatrix| {
        fill(y, j);
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    Ok((
        OdeProblem::new(
            format!("brusselator-1d-{cells}-meanfield"),
            n,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        y0,
    ))
}

fn meanfield_case() -> Case {
    let (problem, y0) = meanfield_brusselator_problem(100, 0.1).unwrap();
    Case {
        id: "brusselator-1d-100-meanfield (n = 200, dense Jacobian)",
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

/// One linearization point: the bits of `t` and of `y`.
type Point = (u64, Vec<u64>);
type Trace = Arc<Mutex<Vec<Point>>>;

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// `problem` with every callback forwarded unchanged and every in-place
/// Jacobian request recorded in the returned trace. Both drivers build the
/// Jacobian once per accepted state (and once more after an attempt that
/// failed with an error), so the trace is the accepted trajectory.
fn traced(problem: &OdeProblem) -> (OdeProblem, Trace) {
    let trace: Trace = Arc::new(Mutex::new(Vec::new()));
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

fn take(trace: &Trace) -> Vec<Point> {
    std::mem::take(&mut *trace.lock().unwrap())
}

/// FNV-1a (64 bit) of the points' bits: a fingerprint of a trajectory.
fn fingerprint(points: &[Point]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut eat = |word: u64| {
        for byte in word.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for (t, y) in points {
        eat(*t);
        y.iter().copied().for_each(&mut eat);
    }
    format!("{hash:016x}")
}

/// The first place two trajectories differ, as text; `None` when equal.
fn first_difference(reference: &[Point], other: &[Point]) -> Option<String> {
    for (step, (a, b)) in reference.iter().zip(other).enumerate() {
        if a.0 != b.0 {
            return Some(format!(
                "accepted state {step}: t {:e} vs {:e}",
                f64::from_bits(a.0),
                f64::from_bits(b.0)
            ));
        }
        if let Some(i) = (0..a.1.len().max(b.1.len())).find(|&i| a.1.get(i) != b.1.get(i)) {
            return Some(format!(
                "accepted state {step} (t = {:e}), component {i}: {:?} vs {:?}",
                f64::from_bits(a.0),
                a.1.get(i).map(|x| f64::from_bits(*x)),
                b.1.get(i).map(|x| f64::from_bits(*x))
            ));
        }
    }
    (reference.len() != other.len())
        .then(|| format!("{} accepted states vs {}", reference.len(), other.len()))
}

fn assert_same_trajectory(label: &str, what: &str, reference: &[Point], other: &[Point]) {
    if let Some(difference) = first_difference(reference, other) {
        panic!("{label}: {what} diverges at {difference}");
    }
}

/// The step statistics, counters and output of `client` are those of the
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

struct Setup {
    adaptive: AdaptiveStepConfig,
    output: OutputSchedule,
    atol: Vec<f64>,
}

fn setup(case: &Case, rtol: f64) -> Setup {
    let adaptive = config(case, rtol);
    Setup {
        atol: vec![adaptive.atol; case.problem.dimension],
        output: schedule(case.span, 10),
        adaptive,
    }
}

fn run_protected(
    problem: &OdeProblem,
    case: &Case,
    setup: &Setup,
) -> (Rodas5pFastResult, Option<DriverPartial>) {
    integrate_rodas5p_fast_observed_with_atol(
        problem,
        case.span,
        &case.y0,
        &setup.adaptive,
        &setup.output,
        &setup.atol,
    )
    .unwrap()
}

fn run_client(
    problem: &OdeProblem,
    case: &Case,
    setup: &Setup,
    client: Box<dyn StageLinearSystem>,
) -> (Rodas5pFastResult, Option<DriverPartial>) {
    let (result, partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
        problem,
        client,
        case.span,
        &case.y0,
        &setup.adaptive,
        &setup.output,
        &setup.atol,
    )
    .unwrap();
    (result, partial)
}

/// Totals over the tolerances of one case, for the coverage assertions.
#[derive(Default)]
struct Coverage {
    rejected: usize,
    reuses: usize,
}

/// (i) for one case at every tolerance, with the LU mode `expected`.
fn assert_client_reproduces_protected(case: &Case, expected: DenseLuMode) -> Coverage {
    let n = case.problem.dimension;
    let mode = DenseLuMode::protected_choice_at(&case.problem, case.span.0, &case.y0).unwrap();
    assert_eq!(mode, expected, "{}: the protected policy's LU", case.id);
    let (problem, trace) = traced(&case.problem);
    let mut coverage = Coverage::default();
    for rtol in RTOLS {
        let label = format!("{} at rtol {rtol:e}, {mode:?}", case.id);
        let setup = setup(case, rtol);

        let protected = run_protected(&problem, case, &setup);
        let protected_trace = take(&trace);

        let client = run_client(
            &problem,
            case,
            &setup,
            Box::new(DenseLuStageClient::new(n, mode).unwrap()),
        );
        let client_trace = take(&trace);

        // The same client run under an accept control that always
        // continues: every accepted `(t, y)` as the driver reports it.
        let mut views: Vec<Point> = Vec::new();
        let mut control = |view: &AcceptedStepView<'_>| {
            views.push((view.t.to_bits(), bits(view.y)));
            AcceptControl::Continue
        };
        let (controlled, controlled_partial, _client) =
            integrate_rodas5p_fast_client_observed_with_atol_and_control(
                &problem,
                Box::new(DenseLuStageClient::new(n, mode).unwrap()),
                case.span,
                &case.y0,
                &setup.adaptive,
                &setup.output,
                &setup.atol,
                &mut control,
            )
            .unwrap();
        let controlled = (controlled, controlled_partial);
        let controlled_trace = take(&trace);

        assert_same_result(&label, mode, &protected, &client);
        assert_same_result(&label, mode, &protected, &controlled);
        assert_same_trajectory(
            &label,
            "the client's accepted trajectory",
            &protected_trace,
            &client_trace,
        );
        assert_same_trajectory(
            &label,
            "the controlled client's accepted trajectory",
            &protected_trace,
            &controlled_trace,
        );

        // No attempt failed with an error, so every accepted state was
        // linearized exactly once: the trace is t_0 .. t_{N-1}.
        let reference = &protected.0;
        let counters = &reference.observed.counters;
        assert_eq!(
            counters.linear_solve_failures
                + counters.nonlinear_solve_failures
                + counters.nonfinite_step_failures,
            0,
            "{label}"
        );
        let accepted = reference.accepted_steps;
        assert_eq!(protected_trace.len(), accepted, "{label}");
        assert_eq!(counters.jacobian_builds, accepted as u64, "{label}");
        assert_eq!(
            protected_trace[0],
            (case.span.0.to_bits(), bits(&case.y0)),
            "{label}"
        );
        // The accept control's view of step k is the state step k + 1
        // starts from in the protected run; the last one is the output at
        // the end of the span.
        assert_eq!(views.len(), accepted, "{label}");
        assert_same_trajectory(
            &label,
            "the accept control's view",
            &protected_trace[1..],
            &views[..accepted - 1],
        );
        let last_output = (
            reference.observed.t.last().unwrap().to_bits(),
            bits(reference.observed.y.last().unwrap()),
        );
        assert_eq!(last_output.0, case.span.1.to_bits(), "{label}");
        assert_eq!(views[accepted - 1], last_output, "{label}");

        // One factorization per attempt, eight solves per attempt.
        assert_eq!(
            counters.direct_factorizations, reference.attempts as u64,
            "{label}"
        );
        assert_eq!(
            counters.linear_solves,
            STAGES * reference.attempts as u64,
            "{label}"
        );
        assert_eq!(
            counters.direct_solve_calls, counters.linear_solves,
            "{label}"
        );

        // The whole accepted trajectory t_0 .. t_N.
        let mut trajectory = protected_trace;
        trajectory.push(last_output);
        println!(
            "STAGE_CLIENTS_IDENTITY_JSON {}",
            serde_json::json!({
                "case": case.id,
                "n": n,
                "rtol": rtol,
                "atol": setup.adaptive.atol,
                "client_mode": format!("{mode:?}"),
                "protected_lu": format!("{:?}", reference.lu),
                "attempts": reference.attempts,
                "accepted_steps": accepted,
                "rejected_steps": reference.rejected_steps,
                "jacobian_reuses": reference.jacobian_reuses,
                "jacobian_builds": counters.jacobian_builds,
                "rhs_calls": counters.rhs_calls,
                "direct_factorizations": counters.direct_factorizations,
                "direct_solve_calls": counters.direct_solve_calls,
                "linear_solves": counters.linear_solves,
                "output_states_compared": reference.observed.y.len(),
                "accepted_states_compared": trajectory.len(),
                "accepted_trajectory_fnv1a64": fingerprint(&trajectory),
                "bit_identical": true,
            })
        );
        coverage.rejected += reference.rejected_steps;
        coverage.reuses += reference.jacobian_reuses;
    }
    coverage
}

#[test]
fn dense_client_reproduces_protected_driver_robertson() {
    let case = robertson_case();
    assert_eq!(case.problem.dimension, 3);
    assert_client_reproduces_protected(&case, DenseLuMode::InPlace);
}

#[test]
fn dense_client_reproduces_protected_driver_hires() {
    let case = hires_case();
    assert_eq!(case.problem.dimension, 8);
    assert_client_reproduces_protected(&case, DenseLuMode::InPlace);
}

#[test]
fn dense_client_reproduces_protected_driver_brusselator_n32() {
    let case = brusselator_case(32, "brusselator-1d-32 (n = 64)");
    assert_eq!(case.problem.dimension, 64);
    assert_client_reproduces_protected(&case, DenseLuMode::InPlace);
}

#[test]
fn dense_client_reproduces_protected_driver_brusselator_n100() {
    let case = brusselator_case(100, "brusselator-1d-100 (n = 200)");
    assert_eq!(case.problem.dimension, 200);
    let coverage = assert_client_reproduces_protected(&case, DenseLuMode::InPlace);
    // The rejection path and the Jacobian reuse after it were exercised.
    assert!(coverage.rejected > 0 && coverage.reuses > 0);
}

#[test]
fn dense_client_reproduces_protected_driver_meanfield_brusselator_n100_faer() {
    let case = meanfield_case();
    assert_eq!(case.problem.dimension, 200);
    let coverage = assert_client_reproduces_protected(&case, DenseLuMode::Faer);
    assert!(coverage.rejected > 0 && coverage.reuses > 0);
}

/// (ii) for every case: the wrapped client's run is the protected run and
/// the wrapper's call counts are the driver's counters.
#[test]
fn timed_wrapper_is_result_neutral_and_counts_match_driver() {
    let cases = [
        (robertson_case(), DenseLuMode::InPlace),
        (hires_case(), DenseLuMode::InPlace),
        (
            brusselator_case(32, "brusselator-1d-32 (n = 64)"),
            DenseLuMode::InPlace,
        ),
        (
            brusselator_case(100, "brusselator-1d-100 (n = 200)"),
            DenseLuMode::InPlace,
        ),
        (meanfield_case(), DenseLuMode::Faer),
    ];
    for (case, mode) in &cases {
        let n = case.problem.dimension;
        for rtol in RTOLS {
            let label = format!("{} at rtol {rtol:e}, timed {mode:?}", case.id);
            let setup = setup(case, rtol);
            let protected = run_protected(&case.problem, case, &setup);
            let timed = TimedStageClient::new(DenseLuStageClient::new(n, *mode).unwrap());
            let handle = timed.handle();
            assert_eq!(handle.snapshot(), PhaseTimes::default(), "{label}");
            let client = run_client(&case.problem, case, &setup, Box::new(timed));
            assert_same_result(&label, *mode, &protected, &client);

            let times = handle.snapshot();
            let result = &client.0;
            let counters = &result.observed.counters;
            assert_eq!(
                counters.linear_solve_failures
                    + counters.nonlinear_solve_failures
                    + counters.nonfinite_step_failures,
                0,
                "{label}"
            );
            assert_eq!(times.factor_calls, result.attempts as u64, "{label}");
            assert_eq!(
                times.factor_calls, counters.direct_factorizations,
                "{label}"
            );
            assert_eq!(
                times.solve_calls,
                STAGES * result.attempts as u64,
                "{label}"
            );
            assert_eq!(times.solve_calls, counters.linear_solves, "{label}");
            assert_eq!(times.solve_calls, counters.direct_solve_calls, "{label}");
            assert_eq!(times.linearize_calls, counters.jacobian_builds, "{label}");
            assert_eq!(
                times.linearize_calls, result.accepted_steps as u64,
                "{label}"
            );
            assert!(times.total_ns() > 0, "{label}: {times:?}");
            println!(
                "STAGE_CLIENTS_TIMED_JSON {}",
                serde_json::json!({
                    "case": case.id,
                    "n": n,
                    "rtol": rtol,
                    "client_mode": format!("{mode:?}"),
                    "attempts": result.attempts,
                    "accepted_steps": result.accepted_steps,
                    "linearize_calls": times.linearize_calls,
                    "factor_calls": times.factor_calls,
                    "solve_calls": times.solve_calls,
                    "result_neutral": true,
                })
            );
        }
    }
}

/// The mode the protected policy does not pick is a valid solver of the
/// same systems, not a bit-identical one: on the three named problems the
/// faer client completes and ends within a few tolerance units of the
/// protected (in-place) run.
#[test]
fn faer_mode_off_policy_is_a_valid_solver_of_the_named_problems() {
    let cases = [
        robertson_case(),
        hires_case(),
        brusselator_case(32, "brusselator-1d-32 (n = 64)"),
        brusselator_case(100, "brusselator-1d-100 (n = 200)"),
    ];
    for case in &cases {
        let n = case.problem.dimension;
        for rtol in RTOLS {
            let label = format!("{} at rtol {rtol:e}, off-policy faer", case.id);
            let setup = setup(case, rtol);
            let (protected, _) = run_protected(&case.problem, case, &setup);
            assert_eq!(protected.lu, Rodas5pFastLu::InPlaceZeroSkipping, "{label}");
            let (client, partial) = run_client(
                &case.problem,
                case,
                &setup,
                Box::new(DenseLuStageClient::new(n, DenseLuMode::Faer).unwrap()),
            );
            assert!(client.observed.success && partial.is_none(), "{label}");
            let end = client.observed.y.last().unwrap();
            let reference = protected.observed.y.last().unwrap();
            let tolerance_units = end
                .iter()
                .zip(reference)
                .map(|(a, r)| (a - r).abs() / (setup.adaptive.atol + rtol * r.abs()))
                .fold(0.0_f64, f64::max);
            assert!(tolerance_units <= 10.0, "{label}: {tolerance_units}");
            println!(
                "STAGE_CLIENTS_OFF_POLICY_JSON {}",
                serde_json::json!({
                    "case": case.id,
                    "n": n,
                    "rtol": rtol,
                    "client_mode": "Faer",
                    "protected_lu": format!("{:?}", protected.lu),
                    "attempts_client": client.attempts,
                    "attempts_protected": protected.attempts,
                    "endpoint_difference_tolerance_units": tolerance_units,
                    "endpoint_bit_identical": bits(end) == bits(reference),
                })
            );
        }
    }
}

/// The protected policy's boundaries and the client's refusals.
#[test]
fn protected_choice_follows_the_policy_and_misuse_is_refused() {
    let small = RODAS5P_FAST_SMALL_LU_MAX;
    let full = |n: usize| DenseMatrix::new(n, n, vec![1.0; n * n]).unwrap();
    assert_eq!(
        DenseLuMode::protected_choice(&full(small)),
        DenseLuMode::InPlace
    );
    assert_eq!(
        DenseLuMode::protected_choice(&full(small + 1)),
        DenseLuMode::Faer
    );
    // Above the small order the density decides: exactly 10 % stays in
    // place, one entry more goes to faer.
    let n = 100;
    let mut sparse = DenseMatrix::zeros(n, n);
    sparse.as_mut_slice()[..n * n / 10].fill(1.0);
    assert_eq!(DenseLuMode::protected_choice(&sparse), DenseLuMode::InPlace);
    sparse.as_mut_slice()[n * n / 10] = 1.0;
    assert_eq!(DenseLuMode::protected_choice(&sparse), DenseLuMode::Faer);
    assert_eq!(
        DenseLuMode::InPlace.protected_lu(),
        Rodas5pFastLu::InPlaceZeroSkipping
    );
    assert_eq!(DenseLuMode::Faer.protected_lu(), Rodas5pFastLu::Faer);

    assert!(matches!(
        DenseLuStageClient::new(0, DenseLuMode::InPlace),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        DenseLuStageClient::new(usize::MAX, DenseLuMode::Faer),
        Err(CoreError::InvalidInput(_))
    ));
    let case = robertson_case();
    assert_eq!(
        DenseLuStageClient::for_protected_run(&case.problem, case.span.0, &case.y0)
            .unwrap()
            .mode(),
        DenseLuMode::InPlace
    );
    for mode in [DenseLuMode::InPlace, DenseLuMode::Faer] {
        let mut client = DenseLuStageClient::new(3, mode).unwrap();
        assert_eq!(client.dimension(), 3);
        assert_eq!(client.mode(), mode);
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
        // The driver's charge of the build is the only one that stays.
        counters.jacobian_builds += 1;
        client
            .linearize(&case.problem, 0.0, &case.y0, &mut counters)
            .unwrap();
        assert_eq!(counters.jacobian_builds, 1);
        assert_eq!(client.jacobian()[(0, 0)], -0.04);
        client.factor(1.0e3, &mut counters).unwrap();
        assert_eq!(counters.direct_factorizations, 1);
        let mut x = [1.0, 2.0, 3.0];
        client.solve_in_place(&mut x, &mut counters).unwrap();
        assert!(x.iter().all(|v| v.is_finite()));
    }
    // A singular W is a retryable error of the in-place factorization and
    // leaves no factors: J = I (a problem with the identity Jacobian) and
    // 1/(h gamma) = 1 give W = 0.
    let identity = OdeProblem::new(
        "identity Jacobian",
        2,
        Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
            out.copy_from_slice(y);
            Ok(())
        }),
        None,
        Some(Arc::new(|_t: f64, _y: &[f64]| Ok(DenseMatrix::identity(2)))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut client = DenseLuStageClient::new(2, DenseLuMode::InPlace).unwrap();
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

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}

fn minimum(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Minor page faults of this process so far (`/proc/self/stat`, field 10);
/// `None` where that file does not exist or does not parse.
fn minor_page_faults() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // The command name (field 2) is parenthesized and may contain spaces.
    let after_name = &stat[stat.rfind(')')? + 1..];
    after_name.split_whitespace().nth(7)?.parse().ok()
}

/// Diagnostic wall time of the client driver against the protected driver:
/// seven interleaved repetitions per case, the arm order rotating with the
/// repetition. Wall time on a shared host is diagnostic only; nothing is
/// asserted about it. Run alone:
/// `-- --ignored --exact diagnostic_wall_ratio_client_over_protected --nocapture --test-threads=1`.
#[test]
#[ignore = "diagnostic wall time: run alone with --ignored --nocapture --test-threads=1"]
fn diagnostic_wall_ratio_client_over_protected() {
    const REPETITIONS: usize = 7;
    const RUNS_PER_REPETITION: usize = 5;
    let rtol = 1.0e-6;
    let cases = [
        (
            brusselator_case(32, "brusselator-1d-32 (n = 64)"),
            DenseLuMode::InPlace,
        ),
        (
            brusselator_case(100, "brusselator-1d-100 (n = 200)"),
            DenseLuMode::InPlace,
        ),
        (meanfield_case(), DenseLuMode::Faer),
    ];
    for (case, mode) in &cases {
        let n = case.problem.dimension;
        let setup = setup(case, rtol);
        let protected_arm = || run_protected(&case.problem, case, &setup).0;
        let client_arm = || {
            let client = Box::new(DenseLuStageClient::new(n, *mode).unwrap());
            run_client(&case.problem, case, &setup, client).0
        };
        let timed_arm = || {
            let timed = TimedStageClient::new(DenseLuStageClient::new(n, *mode).unwrap());
            let handle = timed.handle();
            let result = run_client(&case.problem, case, &setup, Box::new(timed)).0;
            (result, handle.snapshot())
        };
        // Warm-up, and the identity of the three arms.
        let reference = protected_arm();
        let end = bits(reference.observed.y.last().unwrap());
        assert_eq!(bits(client_arm().observed.y.last().unwrap()), end);
        assert_eq!(bits(timed_arm().0.observed.y.last().unwrap()), end);

        // Wall time per run and minor page faults per run of one arm.
        let time = |arm: &dyn Fn()| {
            let faults = minor_page_faults();
            let start = Instant::now();
            for _ in 0..RUNS_PER_REPETITION {
                arm();
            }
            let ns = start.elapsed().as_nanos() as f64 / RUNS_PER_REPETITION as f64;
            let faults = match (faults, minor_page_faults()) {
                (Some(before), Some(after)) => {
                    Some((after - before) as f64 / RUNS_PER_REPETITION as f64)
                }
                _ => None,
            };
            (ns, faults)
        };
        let (mut protected_ns, mut client_ns, mut timed_ns) = (Vec::new(), Vec::new(), Vec::new());
        let (mut protected_faults, mut client_faults) = (Vec::new(), Vec::new());
        let mut phases = PhaseTimes::default();
        let mut timed_run_ns = 0.0;
        for repetition in 0..REPETITIONS {
            for slot in 0..3 {
                match (repetition + slot) % 3 {
                    0 => {
                        let (ns, faults) = time(&|| {
                            black_box(protected_arm());
                        });
                        protected_ns.push(ns);
                        protected_faults.push(faults);
                    }
                    1 => {
                        let (ns, faults) = time(&|| {
                            black_box(client_arm());
                        });
                        client_ns.push(ns);
                        client_faults.push(faults);
                    }
                    _ => {
                        let (ns, _) = time(&|| {
                            black_box(timed_arm());
                        });
                        timed_ns.push(ns);
                        // One more timed run for the phase split, with its
                        // own wall time.
                        let start = Instant::now();
                        let (_, times) = timed_arm();
                        timed_run_ns = start.elapsed().as_nanos() as f64;
                        phases = times;
                    }
                }
            }
        }
        let ratios = |arm: &[f64]| -> Vec<f64> {
            arm.iter().zip(&protected_ns).map(|(a, p)| a / p).collect()
        };
        let client_ratios = ratios(&client_ns);
        let timed_ratios = ratios(&timed_ns);
        println!(
            "STAGE_CLIENTS_WALL_JSON {}",
            serde_json::json!({
                "diagnostic_only": true,
                "case": case.id,
                "n": n,
                "rtol": rtol,
                "client_mode": format!("{mode:?}"),
                "repetitions": REPETITIONS,
                "runs_per_repetition": RUNS_PER_REPETITION,
                "interleaved": true,
                "attempts": reference.attempts,
                "accepted_steps": reference.accepted_steps,
                "protected_ns_per_run": protected_ns,
                "client_ns_per_run": client_ns,
                "timed_client_ns_per_run": timed_ns,
                "protected_median_ns": median(&protected_ns),
                "client_median_ns": median(&client_ns),
                "timed_client_median_ns": median(&timed_ns),
                "protected_min_ns": minimum(&protected_ns),
                "client_min_ns": minimum(&client_ns),
                "timed_client_min_ns": minimum(&timed_ns),
                "client_over_protected_ratio_of_minima":
                    minimum(&client_ns) / minimum(&protected_ns),
                "timed_over_protected_ratio_of_minima":
                    minimum(&timed_ns) / minimum(&protected_ns),
                "protected_minor_page_faults_per_run": protected_faults,
                "client_minor_page_faults_per_run": client_faults,
                "client_over_protected_ratio_of_medians":
                    median(&client_ns) / median(&protected_ns),
                "client_over_protected_median_of_ratios": median(&client_ratios),
                "client_over_protected_ratios": client_ratios,
                "timed_over_protected_ratio_of_medians":
                    median(&timed_ns) / median(&protected_ns),
                "timed_over_protected_median_of_ratios": median(&timed_ratios),
                "timed_over_protected_ratios": timed_ratios,
                "last_timed_run": {
                    "wall_ns": timed_run_ns,
                    "linearize_ns": phases.linearize_ns,
                    "factor_ns": phases.factor_ns,
                    "solve_ns": phases.solve_ns,
                    "linearize_calls": phases.linearize_calls,
                    "factor_calls": phases.factor_calls,
                    "solve_calls": phases.solve_calls,
                    "phases_over_wall": phases.total_ns() as f64 / timed_run_ns,
                },
            })
        );
    }
}

/// One arm of the comparison, repeated, for an external instruction count
/// (`perf stat -e instructions:u` on this test binary): unlike wall time,
/// user-space instructions do not depend on the load of the host. The
/// environment selects the arm: `STAGE_CLIENTS_ARM` (`protected`, `client`
/// or `timed`), `STAGE_CLIENTS_CASE` (`brusselator-n32`, `brusselator-n100`
/// or `meanfield-n100`) and `STAGE_CLIENTS_RUNS` (0 measures the set-up
/// alone, to be subtracted). Without `STAGE_CLIENTS_ARM` it does nothing.
#[test]
#[ignore = "diagnostic single arm for perf stat: set STAGE_CLIENTS_ARM, _CASE and _RUNS"]
fn diagnostic_single_arm_for_instruction_count() {
    let Ok(arm) = std::env::var("STAGE_CLIENTS_ARM") else {
        println!("STAGE_CLIENTS_ARM is not set: nothing to run");
        return;
    };
    let case_name = std::env::var("STAGE_CLIENTS_CASE").expect("STAGE_CLIENTS_CASE");
    let runs: usize = std::env::var("STAGE_CLIENTS_RUNS")
        .expect("STAGE_CLIENTS_RUNS")
        .parse()
        .expect("STAGE_CLIENTS_RUNS is a count");
    let (case, mode) = match case_name.as_str() {
        "brusselator-n32" => (
            brusselator_case(32, "brusselator-1d-32 (n = 64)"),
            DenseLuMode::InPlace,
        ),
        "brusselator-n100" => (
            brusselator_case(100, "brusselator-1d-100 (n = 200)"),
            DenseLuMode::InPlace,
        ),
        "meanfield-n100" => (meanfield_case(), DenseLuMode::Faer),
        other => panic!("unknown STAGE_CLIENTS_CASE {other}"),
    };
    let n = case.problem.dimension;
    let rtol = 1.0e-6;
    let setup = setup(&case, rtol);
    let (mut attempts, mut accepted) = (0, 0);
    for _ in 0..runs {
        let result = match arm.as_str() {
            "protected" => run_protected(&case.problem, &case, &setup).0,
            "client" => {
                let client = Box::new(DenseLuStageClient::new(n, mode).unwrap());
                run_client(&case.problem, &case, &setup, client).0
            }
            "timed" => {
                let timed = TimedStageClient::new(DenseLuStageClient::new(n, mode).unwrap());
                run_client(&case.problem, &case, &setup, Box::new(timed)).0
            }
            other => panic!("unknown STAGE_CLIENTS_ARM {other}"),
        };
        assert!(result.observed.success);
        attempts = result.attempts;
        accepted = result.accepted_steps;
        black_box(result);
    }
    println!(
        "STAGE_CLIENTS_ARM_JSON {}",
        serde_json::json!({
            "arm": arm,
            "case": case.id,
            "case_key": case_name,
            "n": n,
            "rtol": rtol,
            "client_mode": format!("{mode:?}"),
            "runs": runs,
            "attempts_per_run": attempts,
            "accepted_steps_per_run": accepted,
        })
    );
}
