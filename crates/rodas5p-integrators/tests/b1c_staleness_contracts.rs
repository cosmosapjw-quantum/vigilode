//! Contracts of the B1c-staleness rule of the lagged-factorization RODAS5P
//! driver (production wave-3 node `b1c-staleness`; plan
//! `docs/plan/IMPLEMENTATION_WAVE3.md` Task 2, §5 T1, T2, T9, §6 B1;
//! preregistration `research/b1c_staleness_20261021/PREREGISTRATION.md`).
//!
//! * (a) The staleness bound `q_pred` dominates `||I - P^{-1} W||_S`, formed
//!   explicitly by `n` LU solves, on Robertson and HIRES Jacobians.
//! * (b) For `n = 1` and `J = J_P = lambda` it is `|1 - h_P/h| / (1 - h_P gamma
//!   lambda)` within 1 ulp.
//! * (c) A stable non-normal Jacobian with a large weighted log norm (ruling
//!   FP1) is unbounded at every check, so `Bounded` refreshes every attempt
//!   and reproduces the fast driver.
//! * (d) `StalenessRule::Off` (the default) is the wave-2 driver bit for bit:
//!   Richardson pins of `b1b_gmres_contracts`, GMRES pins recorded at the
//!   base `75a0c6c` before any code change (ruling B2b).
//! * (e) A forced refresh with `Bounded` makes no staleness check (ruling
//!   B2c) and reproduces the fast driver.
//! * (f) Every factorization is charged to exactly one refresh reason.
//! * The preregistered kill gate measurement (`#[ignore]`d) writes
//!   `results.json` when `B1C_RESULTS_OUT` is set.

mod b1_problems;

use std::sync::Arc;
use std::time::Instant;

use b1_problems::*;
use rodas5p_core::{DenseMatrix, LuFactorization, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, RODAS5P_LAGGED_DRIVER_ID,
    RODAS5P_LAGGED_GMRES_BOUNDED_DRIVER_ID, RODAS5P_LAGGED_GMRES_DRIVER_ID,
    RODAS5P_LAGGED_RICHARDSON_BOUNDED_DRIVER_ID, RODAS5P_LAGGED_STALENESS_Q_MAX,
    Rodas5pLaggedOptions, Rodas5pLaggedResult, StageSolver, StalenessRule, StalenessWork,
    integrate_rodas5p_fast_observed, integrate_rodas5p_lagged_observed, rodas5p_staleness_bound,
};

const GAMMA: f64 = 0.21193756319429014;

fn options(stage_solver: StageSolver, staleness: StalenessRule) -> Rodas5pLaggedOptions {
    Rodas5pLaggedOptions {
        stage_solver,
        staleness,
        ..Rodas5pLaggedOptions::default()
    }
}

fn run(
    case: &Case,
    rtol: f64,
    points: usize,
    options: Rodas5pLaggedOptions,
) -> Rodas5pLaggedResult {
    integrate_rodas5p_lagged_observed(
        &case.problem,
        case.span,
        &case.y0,
        &config(case, rtol),
        &schedule(case.span, points),
        options,
    )
    .unwrap()
}

fn weights(y: &[f64], adaptive: &AdaptiveStepConfig) -> Vec<f64> {
    y.iter()
        .map(|v| adaptive.atol + adaptive.rtol * v.abs())
        .collect()
}

/// `||A||_S = max_i sum_j |A_ij| s_j / s_i`.
fn induced_norm(a: &DenseMatrix, s: &[f64]) -> f64 {
    let n = s.len();
    (0..n)
        .map(|i| (0..n).map(|j| a[(i, j)].abs() * s[j] / s[i]).sum::<f64>())
        .fold(0.0, f64::max)
}

/// `T = I - P^{-1} W`, column by column (n LU solves of `P`).
fn explicit_t(h: f64, h_p: f64, j: &DenseMatrix, j_p: &DenseMatrix) -> DenseMatrix {
    let n = j.nrows();
    let mut p = DenseMatrix::zeros(n, n);
    let mut w = DenseMatrix::zeros(n, n);
    for r in 0..n {
        for c in 0..n {
            p[(r, c)] = -j_p[(r, c)];
            w[(r, c)] = -j[(r, c)];
        }
        p[(r, r)] += 1.0 / (h_p * GAMMA);
        w[(r, r)] += 1.0 / (h * GAMMA);
    }
    let lu = LuFactorization::new(&p).unwrap();
    let mut t = DenseMatrix::identity(n);
    for c in 0..n {
        let column: Vec<f64> = (0..n).map(|r| w[(r, c)]).collect();
        let x = lu.solve(&column).unwrap();
        for r in 0..n {
            t[(r, c)] -= x[r];
        }
    }
    t
}

#[test]
fn staleness_bound_dominates_explicit_t_norm() {
    for case in [robertson_case(), hires_case()] {
        let adaptive = config(&case, 1.0e-6);
        let fast = integrate_rodas5p_fast_observed(
            &case.problem,
            case.span,
            &case.y0,
            &adaptive,
            &schedule(case.span, 8),
        )
        .unwrap();
        let (times, states) = (&fast.observed.t, &fast.observed.y);
        let mut counters = WorkCounters::default();
        let mut jac = |k: usize| {
            case.problem
                .dense_jacobian(times[k], &states[k], &mut counters)
                .unwrap()
        };
        let mut finite = 0;
        // Three current states, each with J_P from an earlier state.
        for (now, earlier) in [(1_usize, 0_usize), (4, 2), (8, 6)] {
            let (j, j_p) = (jac(now), jac(earlier));
            let s = weights(&states[now], &adaptive);
            for h in [1.0e-5, 1.0e-3, 1.0e-1] {
                for ratio in [0.5, 0.8, 1.0, 1.25, 2.0] {
                    let h_p = ratio * h;
                    let q = rodas5p_staleness_bound(h, h_p, GAMMA, &j, &j_p, &s);
                    assert!(!q.is_nan());
                    assert!(q >= 0.0);
                    if q.is_finite() {
                        finite += 1;
                        let t_norm = induced_norm(&explicit_t(h, h_p, &j, &j_p), &s);
                        assert!(
                            t_norm <= q * (1.0 + 1.0e-12),
                            "{}: state {now} (J_P {earlier}) h {h:e} h_P/h {ratio}: ||T||_S {t_norm:e} > q_pred {q:e}",
                            case.id
                        );
                    }
                }
            }
        }
        assert!(finite > 0, "{}: no finite bound exercised", case.id);
    }
}

#[test]
fn bound_is_exact_for_scalar_step_change() {
    let s = [1.0e-6];
    for lambda in [-1.0e6_f64, -1.0, 0.0] {
        let j = DenseMatrix::from_rows(&[&[lambda]]).unwrap();
        for h in [1.0e-6, 1.0e-2, 1.0] {
            for ratio in [0.5, 0.8, 1.0, 1.25, 2.0] {
                let h_p = ratio * h;
                let q = rodas5p_staleness_bound(h, h_p, GAMMA, &j, &j, &s);
                let expected = (1.0 - h_p / h).abs() / (1.0 - h_p * GAMMA * lambda);
                let ulps = (q.to_bits() as i64 - expected.to_bits() as i64).abs();
                assert!(
                    ulps <= 1,
                    "lambda {lambda:e} h {h:e} ratio {ratio}: q {q:e} vs {expected:e}"
                );
            }
        }
    }
    // Ruling B2d: a zero weight makes the bound +inf (never NaN).
    let j = DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -1.0]]).unwrap();
    let q = rodas5p_staleness_bound(1.0, 1.0, GAMMA, &j, &j, &[1.0, 0.0]);
    assert_eq!(q, f64::INFINITY);
    // h_P gamma mu_S = 2 >= 1: no bound.
    let grow = DenseMatrix::from_rows(&[&[1.0]]).unwrap();
    let q = rodas5p_staleness_bound(1.0, 2.0 / GAMMA, GAMMA, &grow, &grow, &[1.0]);
    assert_eq!(q, f64::INFINITY);
    assert_eq!(RODAS5P_LAGGED_STALENESS_Q_MAX, 0.5);
}

/// `y' = J y` with `J = [[-1, K], [0, -1]]`, `K = 1e8`: eigenvalues `-1`,
/// weighted log norm `mu_S ~ K s_1 / s_0`. The state stays below `atol /
/// rtol`, so the weights stay within a factor 1.4 of `atol` and
/// `h gamma mu_S >= 1` for every `h >= 1e-7` (ruling FP1).
fn nonnormal_case() -> Case {
    const K: f64 = 1.0e8;
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -y[0] + K * y[1];
        out[1] = -y[1];
        Ok(())
    });
    let jacobian =
        Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::from_rows(&[&[-1.0, K], &[0.0, -1.0]]));
    let problem = OdeProblem::new(
        "nonnormal-K1e8",
        2,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    Case {
        id: "nonnormal-K1e8",
        problem,
        y0: vec![0.0, 1.0e-8],
        span: (0.0, 20.0),
        // atol = rtol: weights ~ atol.
        atol_scale: 1.0,
    }
}

#[test]
fn unbounded_log_norm_refreshes_every_attempt() {
    let case = nonnormal_case();
    for solver in [StageSolver::Gmres, StageSolver::Richardson] {
        for rtol in [1.0e-4, 1.0e-6] {
            let adaptive = config(&case, rtol);
            let output = schedule(case.span, 8);
            let fast = integrate_rodas5p_fast_observed(
                &case.problem,
                case.span,
                &case.y0,
                &adaptive,
                &output,
            )
            .unwrap();
            let lagged = run(&case, rtol, 8, options(solver, StalenessRule::Bounded));
            let (f, l) = (&fast.observed, &lagged.fast.observed);
            let st = &lagged.staleness;
            assert!(f.success && l.success);
            assert!(st.checks > 0, "{solver:?} rtol {rtol}: no check");
            assert_eq!(st.checks as usize, lagged.fast.attempts - 1);
            assert_eq!(st.unbounded_checks, st.checks, "{solver:?} rtol {rtol}");
            assert_eq!(st.staleness_refreshes, st.checks);
            assert_eq!(lagged.work.initial_refreshes, 1);
            assert_eq!(lagged.work.lagged_stage_solves, 0);
            assert_eq!(f.t, l.t);
            assert_eq!(fast.accepted_steps, lagged.fast.accepted_steps);
            assert_eq!(fast.rejected_steps, lagged.fast.rejected_steps);
            assert_eq!(
                f.counters.direct_factorizations,
                l.counters.direct_factorizations
            );
            for (a, b) in f.y.iter().flatten().zip(l.y.iter().flatten()) {
                let scale = a.abs().max(f64::MIN_POSITIVE);
                assert!(
                    (a - b).abs() <= 1.0e-12 * scale,
                    "{solver:?} rtol {rtol}: fast {a:e} vs lagged {b:e}"
                );
            }
        }
    }
}

/// (id, accepted, rejected, factorizations, iterations, abandoned
/// iterations, fold of the endpoint bits, first and last endpoint component
/// bits) at rtol 1e-6, one output interval.
type Pin = (&'static str, usize, usize, u64, u64, u64, u64, u64, u64);

/// `b1b_gmres_contracts::RICHARDSON_BASELINE` (recorded at `0879a7e`).
const RICHARDSON_BASELINE: [Pin; 3] = [
    (
        "robertson",
        43,
        2,
        43,
        308,
        216,
        0xf470f0b1f6d5b609,
        0x3fe6e80d9df56862,
        0x3fd22fbe3d3364ef,
    ),
    (
        "hires",
        205,
        8,
        134,
        4434,
        770,
        0xb42427b86aeb21ca,
        0x3f482781614ba3c0,
        0x3f6758e2aa95f8f9,
    ),
    (
        "brusselator-1d-50 (n = 100)",
        82,
        10,
        70,
        1673,
        386,
        0x49a7c068cc1f9b46,
        0x3fee602eaf5419de,
        0x400885aee1f362df,
    ),
];

/// The default GMRES arm, printed at the wave-3 base `75a0c6c` (unmodified
/// wave-2 code) before any b1c change (ruling B2b); iterations are GMRES
/// Arnoldi columns.
const GMRES_BASELINE: [Pin; 3] = [
    (
        "robertson",
        40,
        11,
        6,
        674,
        5,
        0xf470f53c10665e87,
        0x3fe6e80d8b6a52a1,
        0x3fd22fbe624a0d61,
    ),
    (
        "hires",
        205,
        10,
        15,
        4868,
        20,
        0x35ea603f90c50ec9,
        0x3f4827812f7ee3bb,
        0x3f6758e3ae7584c0,
    ),
    (
        "brusselator-1d-50 (n = 100)",
        980,
        859,
        9,
        11401,
        43,
        0x1d688e575c5d823d,
        0x3fee60348067f15c,
        0x400885b110ff6124,
    ),
];

#[test]
fn bounded_off_is_wave2_bitwise() {
    assert_eq!(StalenessRule::default(), StalenessRule::Off);
    assert_eq!(
        Rodas5pLaggedOptions::default().staleness,
        StalenessRule::Off
    );
    for (solver, pins, driver) in [
        (
            StageSolver::Richardson,
            RICHARDSON_BASELINE,
            RODAS5P_LAGGED_DRIVER_ID,
        ),
        (
            StageSolver::Gmres,
            GMRES_BASELINE,
            RODAS5P_LAGGED_GMRES_DRIVER_ID,
        ),
    ] {
        for (case, pin) in [robertson_case(), hires_case(), brusselator_case()]
            .iter()
            .zip(pins)
        {
            assert_eq!(case.id, pin.0);
            let r = run(case, 1.0e-6, 1, options(solver, StalenessRule::Off));
            assert_eq!(r.fast.driver, driver);
            let end = r.fast.observed.y.last().unwrap();
            let fold = end.iter().fold(0u64, |h, v| h.rotate_left(7) ^ v.to_bits());
            assert_eq!(
                (
                    r.fast.accepted_steps,
                    r.fast.rejected_steps,
                    r.fast.observed.counters.direct_factorizations,
                    r.work.richardson_iterations + r.work.gmres_iterations,
                    r.work.abandoned_iterations,
                    fold,
                    end[0].to_bits(),
                    end[end.len() - 1].to_bits(),
                ),
                (pin.1, pin.2, pin.3, pin.4, pin.5, pin.6, pin.7, pin.8),
                "{solver:?} {}",
                case.id
            );
            assert_eq!(r.staleness, StalenessWork::default());
        }
    }
}

#[test]
fn forced_refresh_bounded_equals_fast() {
    for case in [robertson_case(), hires_case()] {
        for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
            let adaptive = config(&case, rtol);
            let output = schedule(case.span, 8);
            let fast = integrate_rodas5p_fast_observed(
                &case.problem,
                case.span,
                &case.y0,
                &adaptive,
                &output,
            )
            .unwrap();
            for (solver, driver) in [
                (
                    StageSolver::Richardson,
                    RODAS5P_LAGGED_RICHARDSON_BOUNDED_DRIVER_ID,
                ),
                (StageSolver::Gmres, RODAS5P_LAGGED_GMRES_BOUNDED_DRIVER_ID),
            ] {
                let lagged = run(
                    &case,
                    rtol,
                    8,
                    Rodas5pLaggedOptions {
                        refresh_every_attempt: true,
                        stage_solver: solver,
                        staleness: StalenessRule::Bounded,
                    },
                );
                let (f, l) = (&fast.observed, &lagged.fast.observed);
                assert!(f.success && l.success, "{} rtol {rtol}", case.id);
                assert_eq!(lagged.fast.driver, driver);
                // Ruling B2c: no check when a forced refresh is due.
                assert_eq!(lagged.staleness, StalenessWork::default());
                assert_eq!(f.t, l.t, "{} rtol {rtol}", case.id);
                assert_eq!(fast.accepted_steps, lagged.fast.accepted_steps);
                assert_eq!(fast.rejected_steps, lagged.fast.rejected_steps);
                assert_eq!(
                    f.counters.direct_factorizations,
                    l.counters.direct_factorizations
                );
                assert_eq!(f.counters.jacobian_builds, l.counters.jacobian_builds);
                for (a, b) in f.y.iter().flatten().zip(l.y.iter().flatten()) {
                    let scale = a.abs().max(f64::MIN_POSITIVE);
                    assert!(
                        (a - b).abs() <= 1.0e-12 * scale,
                        "{} rtol {rtol} {solver:?}: fast {a:e} vs lagged {b:e}",
                        case.id
                    );
                }
            }
        }
    }
}

fn assert_staleness_ledger(id: &str, r: &Rodas5pLaggedResult) {
    let (w, st, c) = (&r.work, &r.staleness, &r.fast.observed.counters);
    assert!(r.fast.observed.success, "{id}");
    assert_eq!(
        w.preconditioner_refreshes,
        w.initial_refreshes
            + w.forced_refreshes
            + w.contraction_refreshes
            + w.budget_refreshes
            + st.staleness_refreshes,
        "{id}"
    );
    assert_eq!(w.preconditioner_refreshes, c.direct_factorizations, "{id}");
    assert_eq!(w.forced_refreshes, 0);
    assert!(w.initial_refreshes >= 1);
    assert!(st.checks > 0, "{id}: no staleness check");
    assert!(st.staleness_refreshes <= st.checks);
    assert!(st.unbounded_checks <= st.staleness_refreshes);
    assert!(st.q_pred_max.is_finite() && st.q_pred_max >= 0.0);
    // At most one check per attempt.
    assert!(st.checks as usize <= r.fast.attempts);
    assert_eq!(
        (w.exact_stage_solves + w.lagged_stage_solves + w.refreshed_stage_solves) as usize,
        8 * r.fast.attempts
    );
    assert_eq!(w.preconditioner_applications, c.direct_solve_calls, "{id}");
}

#[test]
fn staleness_ledger_identity() {
    for case in [robertson_case(), hires_case(), brusselator_case()] {
        for (solver, driver) in [
            (
                StageSolver::Richardson,
                RODAS5P_LAGGED_RICHARDSON_BOUNDED_DRIVER_ID,
            ),
            (StageSolver::Gmres, RODAS5P_LAGGED_GMRES_BOUNDED_DRIVER_ID),
        ] {
            let r = run(&case, 1.0e-6, 1, options(solver, StalenessRule::Bounded));
            assert_eq!(r.fast.driver, driver);
            assert_staleness_ledger(&format!("{} {solver:?}", case.id), &r);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn arm_json(
    arm: &str,
    accepted: usize,
    rejected: usize,
    attempts: usize,
    counters: &WorkCounters,
    lagged: Option<&Rodas5pLaggedResult>,
    errors: (f64, f64),
    wall_ms: f64,
) -> serde_json::Value {
    serde_json::json!({
        "arm": arm,
        "accepted_steps": accepted,
        "rejected_steps": rejected,
        "attempts": attempts,
        "counters": counters,
        "factorizations_per_accepted_step": counters.direct_factorizations as f64 / accepted as f64,
        "lagged_work": lagged.map(|l| l.work),
        "staleness_work": lagged.map(|l| l.staleness),
        "driver": lagged.map(|l| l.fast.driver),
        "endpoint_error_max_abs": errors.0,
        "endpoint_error_tol_units": errors.1,
        "wall_ms_diagnostic": wall_ms,
    })
}

/// The preregistered B1c kill gate
/// (`research/b1c_staleness_20261021/PREREGISTRATION.md`): HIRES and
/// Brusselator 1D n = 100, rtol 1e-6, reference = the fast driver at rtol
/// 1e-11, arm `Gmres + Bounded`. Per problem all of: factorizations per
/// accepted step <= 0.4, accepted steps <= 1.05x fast, endpoint error
/// (tolerance units) <= 1.5x fast. PASS iff both problems pass.
/// `Richardson + Bounded` is context only. Wall time is diagnostic.
#[test]
#[ignore = "measurement: run with --ignored --nocapture, B1C_RESULTS_OUT=<path> writes results.json"]
fn b1c_kill_gate_measurement() {
    let mut problems = Vec::new();
    let mut all_pass = true;
    for case in [hires_case(), brusselator_case()] {
        let output = schedule(case.span, 1);
        let reference = integrate_rodas5p_fast_observed(
            &case.problem,
            case.span,
            &case.y0,
            &config(&case, 1.0e-11),
            &output,
        )
        .unwrap();
        assert!(reference.observed.success);
        let reference_end = reference.observed.y.last().unwrap().clone();
        let adaptive = config(&case, 1.0e-6);
        let start = Instant::now();
        let fast =
            integrate_rodas5p_fast_observed(&case.problem, case.span, &case.y0, &adaptive, &output)
                .unwrap();
        let fast_ms = start.elapsed().as_secs_f64() * 1e3;
        assert!(fast.observed.success);
        let fast_errors =
            endpoint_errors(fast.observed.y.last().unwrap(), &reference_end, &adaptive);
        let mut arms = vec![arm_json(
            "fast",
            fast.accepted_steps,
            fast.rejected_steps,
            fast.attempts,
            &fast.observed.counters,
            None,
            fast_errors,
            fast_ms,
        )];
        let mut gate = serde_json::Value::Null;
        for (arm, solver) in [
            ("lagged-gmres-bounded (gated)", StageSolver::Gmres),
            (
                "lagged-richardson-bounded (context only)",
                StageSolver::Richardson,
            ),
        ] {
            let start = Instant::now();
            let lagged = run(&case, 1.0e-6, 1, options(solver, StalenessRule::Bounded));
            let ms = start.elapsed().as_secs_f64() * 1e3;
            assert!(lagged.fast.observed.success);
            let errors = endpoint_errors(
                lagged.fast.observed.y.last().unwrap(),
                &reference_end,
                &adaptive,
            );
            let c = &lagged.fast.observed.counters;
            let fact_ratio = c.direct_factorizations as f64 / lagged.fast.accepted_steps as f64;
            let step_ratio = lagged.fast.accepted_steps as f64 / fast.accepted_steps as f64;
            let error_ratio = errors.1 / fast_errors.1;
            let pass_fact = fact_ratio <= 0.4;
            let pass_steps = step_ratio <= 1.05;
            let pass_error = error_ratio <= 1.5;
            let pass = pass_fact && pass_steps && pass_error;
            println!(
                "{} {arm}: accepted {} (fast {}), rejected {}, factorizations {}, fact/acc {fact_ratio:.4}, step ratio {step_ratio:.4}, error tol units {:.4e} (fast {:.4e}), error ratio {error_ratio:.4}, staleness {:?}, work {:?}, wall {ms:.1} ms => {}",
                case.id,
                lagged.fast.accepted_steps,
                fast.accepted_steps,
                lagged.fast.rejected_steps,
                c.direct_factorizations,
                errors.1,
                fast_errors.1,
                lagged.staleness,
                lagged.work,
                if pass { "PASS" } else { "FAIL" }
            );
            let criteria = serde_json::json!({
                "factorizations_per_accepted_step": fact_ratio,
                "factorizations_per_accepted_step_max": 0.4,
                "factorizations_pass": pass_fact,
                "accepted_step_ratio": step_ratio,
                "accepted_step_ratio_max": 1.05,
                "steps_pass": pass_steps,
                "endpoint_error_ratio": error_ratio,
                "endpoint_error_ratio_max": 1.5,
                "error_pass": pass_error,
                "verdict": if pass { "PASS" } else { "FAIL" },
            });
            if solver == StageSolver::Gmres {
                all_pass &= pass;
                gate = criteria.clone();
            }
            let mut row = arm_json(
                arm,
                lagged.fast.accepted_steps,
                lagged.fast.rejected_steps,
                lagged.fast.attempts,
                c,
                Some(&lagged),
                errors,
                ms,
            );
            row["criteria"] = criteria;
            arms.push(row);
        }
        problems.push(serde_json::json!({
            "problem": case.id,
            "dimension": case.problem.dimension,
            "span": [case.span.0, case.span.1],
            "rtol": adaptive.rtol,
            "atol": adaptive.atol,
            "reference": "integrate_rodas5p_fast_observed at rtol 1e-11",
            "reference_accepted_steps": reference.accepted_steps,
            "arms": arms,
            "gate": gate,
        }));
    }
    let verdict = if all_pass { "PASS" } else { "FAIL" };
    println!("B1c kill gate (Gmres + Bounded, HIRES and Brusselator n = 100): {verdict}");
    let results = serde_json::json!({
        "node": "b1c-staleness",
        "test": "b1c_staleness_contracts::b1c_kill_gate_measurement",
        "gamma": GAMMA,
        "q_max": RODAS5P_LAGGED_STALENESS_Q_MAX,
        "gated_arm": "StageSolver::Gmres + StalenessRule::Bounded",
        "problems": problems,
        "verdict": verdict,
    });
    if let Ok(path) = std::env::var("B1C_RESULTS_OUT") {
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&results).unwrap() + "\n",
        )
        .unwrap();
    }
}
