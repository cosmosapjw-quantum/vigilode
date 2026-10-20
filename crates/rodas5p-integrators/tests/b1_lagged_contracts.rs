//! Contracts of the lagged-factorization stage-root RODAS5P driver
//! (production wave-1 node `b1-lagged`, plan §5 T1, T2, T9; §6 B1).
//!
//! * With a refresh forced at every attempt the stored LU is `W` itself, so
//!   every stage is the direct solve and the driver must reproduce
//!   `integrate_rodas5p_fast_observed` (asserted to 1e-12 relative).
//! * The preregistered measurement (`#[ignore]`d) prints the HIRES and
//!   Brusselator 1D (n = 100) counts and endpoint errors and the §6 B1 kill
//!   gate verdict; it never asserts the verdict.

use std::sync::Arc;
use std::time::Instant;

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, RODAS5P_LAGGED_DRIVER_ID, RODAS5P_LAGGED_THETA,
    RODAS5P_NONSTIFF_STAGE_SENSITIVITIES, Rodas5pLaggedOptions, integrate_rodas5p_fast_observed,
    integrate_rodas5p_lagged_observed, robertson_problem, rodas5p_lagged_stage_targets,
};

/// HIRES, copied from `rodas5p-cli/src/stiff_benchmark.rs::hires_problem`.
fn hires_problem() -> CoreResult<(OdeProblem, Vec<f64>)> {
    hires_problem_with_jvp(false)
}

/// HIRES, optionally with a JVP `J(y) v` (the explicit Jacobian times `v`).
fn hires_problem_with_jvp(with_jvp: bool) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        Ok(())
    });
    fn hires_fill(y: &[f64], j: &mut DenseMatrix) {
        j[(0, 0)] = -1.71;
        j[(0, 1)] = 0.43;
        j[(0, 2)] = 8.32;
        j[(1, 0)] = 1.71;
        j[(1, 1)] = -8.75;
        j[(2, 2)] = -10.03;
        j[(2, 3)] = 0.43;
        j[(2, 4)] = 0.035;
        j[(3, 1)] = 8.32;
        j[(3, 2)] = 1.71;
        j[(3, 3)] = -1.12;
        j[(4, 4)] = -1.745;
        j[(4, 5)] = 0.43;
        j[(4, 6)] = 0.43;
        j[(5, 3)] = 0.69;
        j[(5, 4)] = 1.71;
        j[(5, 5)] = -280.0 * y[7] - 0.43;
        j[(5, 6)] = 0.69;
        j[(5, 7)] = -280.0 * y[5];
        j[(6, 5)] = 280.0 * y[7];
        j[(6, 6)] = -1.81;
        j[(6, 7)] = 280.0 * y[5];
        j[(7, 5)] = -280.0 * y[7];
        j[(7, 6)] = 1.81;
        j[(7, 7)] = -280.0 * y[5];
    }
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
        hires_fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(|_t: f64, y: &[f64], j: &mut DenseMatrix| {
        hires_fill(y, j);
        Ok(())
    });
    Ok((
        OdeProblem::new(
            "hires",
            8,
            rhs,
            None,
            Some(jacobian),
            with_jvp.then(|| {
                Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
                    let mut j = DenseMatrix::zeros(8, 8);
                    hires_fill(y, &mut j);
                    out.copy_from_slice(&j.matvec(v)?);
                    Ok(())
                })
                    as Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>
            }),
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    ))
}

/// The 1D Brusselator, copied from
/// `rodas5p-cli/src/stiff_benchmark.rs::brusselator_problem`.
fn brusselator_problem(cells: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
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
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    });
    let n = 2 * cells;
    let fill = move |y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = 2.0 * u * v - 4.0 - 2.0 * c;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = -u * u - 2.0 * c;
            if i > 0 {
                j[(a, a - 2)] = c;
                j[(b, b - 2)] = c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = c;
                j[(b, b + 2)] = c;
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
            format!("brusselator-1d-{cells}"),
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

/// Name, problem, y0, span and atol scale (atol = rtol * scale), as in
/// `stiff_benchmark.rs::default_problems`.
struct Case {
    id: &'static str,
    problem: OdeProblem,
    y0: Vec<f64>,
    span: (f64, f64),
    atol_scale: f64,
}

fn robertson_case() -> Case {
    let (problem, y0) = robertson_problem().unwrap();
    Case {
        id: "robertson",
        problem,
        y0,
        span: (0.0, 40.0),
        atol_scale: 1.0e-4,
    }
}

fn hires_case() -> Case {
    let (problem, y0) = hires_problem().unwrap();
    Case {
        id: "hires",
        problem,
        y0,
        span: (0.0, 321.8122),
        atol_scale: 1.0e-4,
    }
}

fn brusselator_case() -> Case {
    let (problem, y0) = brusselator_problem(50).unwrap();
    Case {
        id: "brusselator-1d-50 (n = 100)",
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

/// `stiff_benchmark.rs::adaptive_config`.
fn config(case: &Case, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * case.atol_scale,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: case.span.1 - case.span.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

fn schedule(span: (f64, f64), points: usize) -> OutputSchedule {
    let times = (0..=points)
        .map(|k| {
            if k == points {
                span.1
            } else {
                span.0 + (span.1 - span.0) * k as f64 / points as f64
            }
        })
        .collect();
    OutputSchedule::new(times).unwrap()
}

#[test]
fn stage_targets_follow_the_preregistered_allocation() {
    let targets = rodas5p_lagged_stage_targets();
    assert_eq!(RODAS5P_LAGGED_THETA, 0.1);
    assert_eq!(
        RODAS5P_NONSTIFF_STAGE_SENSITIVITIES,
        [0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.02, 1.00]
    );
    for (rho, a) in targets.iter().zip(RODAS5P_NONSTIFF_STAGE_SENSITIVITIES) {
        assert_eq!(*rho, 0.1 / (8.0 * a.max(1.0)));
    }
}

#[test]
fn forced_refresh_reproduces_the_fast_driver_on_robertson_and_hires() {
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
            let lagged = integrate_rodas5p_lagged_observed(
                &case.problem,
                case.span,
                &case.y0,
                &adaptive,
                &output,
                Rodas5pLaggedOptions {
                    refresh_every_attempt: true,
                },
            )
            .unwrap();
            let (f, l) = (&fast.observed, &lagged.fast.observed);
            assert!(f.success && l.success, "{} rtol {rtol}", case.id);
            assert_eq!(f.t, l.t, "{} rtol {rtol}", case.id);
            assert_eq!(fast.accepted_steps, lagged.fast.accepted_steps);
            assert_eq!(fast.rejected_steps, lagged.fast.rejected_steps);
            assert_eq!(
                f.counters.direct_factorizations,
                l.counters.direct_factorizations
            );
            assert_eq!(f.counters.jacobian_builds, l.counters.jacobian_builds);
            // Every stage was the direct solve: no Richardson work at all.
            assert_eq!(lagged.work.richardson_iterations, 0);
            assert_eq!(lagged.work.operator_applications, 0);
            assert_eq!(
                lagged.work.preconditioner_refreshes,
                l.counters.direct_factorizations
            );
            for (a, b) in f.y.iter().flatten().zip(l.y.iter().flatten()) {
                let scale = a.abs().max(f64::MIN_POSITIVE);
                assert!(
                    (a - b).abs() <= 1.0e-12 * scale,
                    "{} rtol {rtol}: fast {a:e} vs lagged {b:e}",
                    case.id
                );
            }
            assert_eq!(lagged.fast.driver, RODAS5P_LAGGED_DRIVER_ID);
        }
    }
}

#[test]
fn lagged_run_charges_every_refresh_and_evaluates_the_jacobian_every_attempt() {
    for case in [robertson_case(), hires_case()] {
        let adaptive = config(&case, 1.0e-6);
        let output = schedule(case.span, 1);
        let lagged = integrate_rodas5p_lagged_observed(
            &case.problem,
            case.span,
            &case.y0,
            &adaptive,
            &output,
            Rodas5pLaggedOptions::default(),
        )
        .unwrap();
        let c = &lagged.fast.observed.counters;
        let w = &lagged.work;
        assert!(lagged.fast.observed.success, "{}", case.id);
        // A refresh is a factorization, and every factorization is a refresh.
        assert_eq!(w.preconditioner_refreshes, c.direct_factorizations);
        assert_eq!(
            w.preconditioner_refreshes,
            w.initial_refreshes + w.forced_refreshes + w.contraction_refreshes + w.budget_refreshes
        );
        assert_eq!(w.forced_refreshes, 0);
        assert!(w.initial_refreshes >= 1);
        // The exact RODAS5P target: one Jacobian per state, reused only by a
        // retry from the same state (T1).
        assert_eq!(
            c.jacobian_builds as usize,
            lagged.fast.attempts - lagged.fast.jacobian_reuses
        );
        // Every stage of every attempt is either direct or iterated.
        assert_eq!(
            (w.exact_stage_solves + w.lagged_stage_solves + w.refreshed_stage_solves) as usize,
            8 * lagged.fast.attempts
        );
        // Abandoned iterations stay counted.
        assert!(w.abandoned_iterations <= w.richardson_iterations);
        assert_eq!(w.operator_applications, w.richardson_iterations);
        if case.problem.has_jvp() {
            assert_eq!(w.jvp_applications, w.operator_applications);
        } else {
            assert_eq!(w.jvp_applications, 0);
        }
    }
}

struct Row {
    accepted: usize,
    rejected: usize,
    jacobians: u64,
    factorizations: u64,
    iterations: u64,
    abandoned: u64,
    operator_applications: u64,
    error_max_abs: f64,
    error_tol_units: f64,
    wall_ms: f64,
}

fn endpoint_errors(y: &[f64], reference: &[f64], adaptive: &AdaptiveStepConfig) -> (f64, f64) {
    let mut max_abs = 0.0_f64;
    let mut tol_units = 0.0_f64;
    for (a, r) in y.iter().zip(reference) {
        let e = (a - r).abs();
        max_abs = max_abs.max(e);
        tol_units = tol_units.max(e / (adaptive.atol + adaptive.rtol * r.abs()));
    }
    (max_abs, tol_units)
}

/// The preregistered B1 measurement (plan §6 B1; brief Task 3): rtol 1e-6,
/// reference = the fast driver at rtol 1e-11. Kill gate: factorizations
/// per accepted step > 0.4, or accepted steps > 1.05x fast, or endpoint
/// error (max over components, tolerance units) > 1.5x fast. Wall time is
/// diagnostic only (shared 2-core host).
#[test]
#[ignore = "measurement: run with --ignored --nocapture and record in docs/plan/nodes/b1-lagged.md"]
fn b1_lagged_measurement_hires_and_brusselator() {
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
        let start = Instant::now();
        let lagged = integrate_rodas5p_lagged_observed(
            &case.problem,
            case.span,
            &case.y0,
            &adaptive,
            &output,
            Rodas5pLaggedOptions::default(),
        )
        .unwrap();
        let lagged_ms = start.elapsed().as_secs_f64() * 1e3;
        assert!(fast.observed.success && lagged.fast.observed.success);
        let (fe_abs, fe_tol) =
            endpoint_errors(fast.observed.y.last().unwrap(), &reference_end, &adaptive);
        let (le_abs, le_tol) = endpoint_errors(
            lagged.fast.observed.y.last().unwrap(),
            &reference_end,
            &adaptive,
        );
        let fc = &fast.observed.counters;
        let lc = &lagged.fast.observed.counters;
        let rows = [
            (
                "fast",
                Row {
                    accepted: fast.accepted_steps,
                    rejected: fast.rejected_steps,
                    jacobians: fc.jacobian_builds,
                    factorizations: fc.direct_factorizations,
                    iterations: 0,
                    abandoned: 0,
                    operator_applications: 0,
                    error_max_abs: fe_abs,
                    error_tol_units: fe_tol,
                    wall_ms: fast_ms,
                },
            ),
            (
                "lagged",
                Row {
                    accepted: lagged.fast.accepted_steps,
                    rejected: lagged.fast.rejected_steps,
                    jacobians: lc.jacobian_builds,
                    factorizations: lc.direct_factorizations,
                    iterations: lagged.work.richardson_iterations,
                    abandoned: lagged.work.abandoned_iterations,
                    operator_applications: lagged.work.operator_applications,
                    error_max_abs: le_abs,
                    error_tol_units: le_tol,
                    wall_ms: lagged_ms,
                },
            ),
        ];
        println!("\n## {} (rtol 1e-6, atol {:e})", case.id, adaptive.atol);
        println!(
            "| arm | accepted | rejected | Jacobians | factorizations | fact/accepted | Richardson it. | abandoned it. | W·x (JVP/matvec) | endpoint max abs err | endpoint err (tol units) | wall ms (diagnostic) |"
        );
        println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
        for (arm, r) in &rows {
            println!(
                "| {arm} | {} | {} | {} | {} | {:.3} | {} | {} | {} | {:.3e} | {:.3e} | {:.2} |",
                r.accepted,
                r.rejected,
                r.jacobians,
                r.factorizations,
                r.factorizations as f64 / r.accepted as f64,
                r.iterations,
                r.abandoned,
                r.operator_applications,
                r.error_max_abs,
                r.error_tol_units,
                r.wall_ms
            );
        }
        let w = &lagged.work;
        println!(
            "refreshes: initial {}, contraction {}, budget {}; stages: exact {}, lagged {}, refreshed {}",
            w.initial_refreshes,
            w.contraction_refreshes,
            w.budget_refreshes,
            w.exact_stage_solves,
            w.lagged_stage_solves,
            w.refreshed_stage_solves
        );
        let l = &rows[1].1;
        let fact_ratio = l.factorizations as f64 / l.accepted as f64;
        let step_ratio = l.accepted as f64 / rows[0].1.accepted as f64;
        let error_ratio = l.error_tol_units / rows[0].1.error_tol_units;
        let fail_fact = fact_ratio > 0.4;
        let fail_steps = step_ratio > 1.05;
        let fail_error = error_ratio > 1.5;
        println!(
            "kill gate: fact/accepted {fact_ratio:.3} (> 0.4 fails: {fail_fact}), accepted ratio {step_ratio:.4} (> 1.05 fails: {fail_steps}), error ratio {error_ratio:.3} (> 1.5 fails: {fail_error}) => {}",
            if fail_fact || fail_steps || fail_error {
                "FAIL"
            } else {
                "PASS"
            }
        );
    }
}

/// Final review item 6 (T3 minor): the JVP branch of `apply_w` inside an
/// iterating stage. HIRES with a JVP and default options: after the first
/// step `P` is lagged (a new `J_0` per state, and `h` changes), so stages
/// iterate, and every product `W x` goes through the problem's JVP.
#[test]
fn lagged_jvp_branch_runs_inside_iterating_stages() {
    let (problem, y0) = hires_problem_with_jvp(true).unwrap();
    assert!(problem.has_jvp() && problem.has_explicit_jacobian());
    let case = Case {
        id: "hires+jvp",
        problem,
        y0,
        span: (0.0, 321.8122),
        atol_scale: 1.0e-4,
    };
    let adaptive = config(&case, 1.0e-6);
    let lagged = integrate_rodas5p_lagged_observed(
        &case.problem,
        case.span,
        &case.y0,
        &adaptive,
        &schedule(case.span, 1),
        Rodas5pLaggedOptions::default(),
    )
    .unwrap();
    let (c, w) = (&lagged.fast.observed.counters, &lagged.work);
    assert!(
        lagged.fast.observed.success,
        "{}",
        lagged.fast.observed.message
    );
    assert!(w.richardson_iterations > 0, "no stage iterated");
    assert!(
        w.lagged_stage_solves > 0,
        "no stage converged on a lagged P"
    );
    assert_eq!(w.jvp_applications, w.operator_applications);
    assert_eq!(w.operator_applications, w.richardson_iterations);
    assert!(c.jvp_calls >= w.jvp_applications);
    // The JVP route reproduces the explicit-matvec route's iteration counts
    // closely (same J_0): both converge with the same refresh pattern here.
    let (plain, _) = hires_problem().unwrap();
    let matvec = integrate_rodas5p_lagged_observed(
        &plain,
        case.span,
        &case.y0,
        &adaptive,
        &schedule(case.span, 1),
        Rodas5pLaggedOptions::default(),
    )
    .unwrap();
    assert_eq!(matvec.work.jvp_applications, 0);
    assert!(matvec.work.richardson_iterations > 0);
}

/// Final review item 11: the stage sensitivities are bound to the t-verify
/// output `research/production_theory_20261021/RESULTS.json`
/// (`claim1_T1_numbers.a_values_at_z0.abs_m`, mpmath on the coefficient
/// snapshot). Each constant must be no smaller than the verified value (a
/// smaller `a_i` would loosen `rho_i`) and equal to it rounded up to two
/// decimals.
#[test]
fn stage_sensitivities_are_the_verified_values_rounded_up() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/production_theory_20261021/RESULTS.json"
    );
    let text = std::fs::read_to_string(path).expect("t-verify RESULTS.json");
    let results: serde_json::Value = serde_json::from_str(&text).unwrap();
    let verified: Vec<f64> = results["claim1_T1_numbers"]["a_values_at_z0"]["abs_m"]
        .as_array()
        .expect("abs_m array")
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(verified.len(), RODAS5P_NONSTIFF_STAGE_SENSITIVITIES.len());
    assert_eq!(
        results["claim1_T1_numbers"]["a_values_at_z0"]["pass"],
        serde_json::Value::Bool(true)
    );
    for (i, (a, v)) in RODAS5P_NONSTIFF_STAGE_SENSITIVITIES
        .iter()
        .zip(&verified)
        .enumerate()
    {
        assert!(a >= v, "a_{} = {a} below the verified {v}", i + 1);
        let rounded_up = (v * 100.0).ceil() / 100.0;
        assert_eq!(*a, rounded_up, "a_{}: verified {v}", i + 1);
    }
}
