//! Work-precision benchmark of RODAS5P against the in-repository BDF and
//! Radau IIA integrators on four standard stiff problems (research node
//! `research/stiff_bdf_radau_benchmark_20261001`). SciPy's BDF and Radau run
//! the same equations from `tools/stiff_benchmark_scipy.py`, which also
//! checks that its right-hand sides match the samples exported here.
//!
//! Wall seconds are diagnostics on a shared host: the statistical authority
//! of timing is on hold (`rodas5p_fair_ab::timing_authority_registry`), so
//! nothing here is a speed promotion.

use std::{sync::Arc, time::Instant};

use anyhow::Result;
use rodas5p_core::{CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveStepConfig, BdfConfig, IntegrationMethod,
    NewtonTolerancePolicy, OdeProblem, OutputSchedule, RadauConfig,
    integrate_adaptive_observed_with_config, integrate_bdf_adaptive_observed,
    integrate_radau_adaptive_observed, robertson_problem, stiff_van_der_pol_problem,
};
use serde_json::{Value, json};

pub const SCHEMA: &str = "vigilode-stiff-bdf-radau-benchmark-v1";
pub const TOLERANCES: [f64; 7] = [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8, 1.0e-9];
pub const ARMS: [&str; 5] = [
    "rodas5p",
    "repo-bdf2",
    "repo-bdf2-tier-a",
    "repo-radau5",
    "repo-radau5-tier-a",
];
const INITIAL_STEP: f64 = 1.0e-6;
const MAX_ATTEMPTS: usize = 1_000_000;

pub struct BenchmarkProblem {
    pub id: &'static str,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    /// atol = rtol * atol_scale.
    pub atol_scale: f64,
}

fn hires_problem() -> CoreResult<(OdeProblem, Vec<f64>)> {
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
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
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
        Ok(j)
    });
    Ok((
        OdeProblem::new(
            "hires",
            8,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )?,
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    ))
}

/// The one-dimensional Brusselator of Hairer and Wanner (II, IV.1) with
/// `cells` interior points, interleaved as `u_1, v_1, u_2, v_2, ...`.
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
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
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
        Ok(j)
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
        )?,
        y0,
    ))
}

/// The four problems of the first benchmark (the default selection).
pub const DEFAULT_PROBLEMS: [&str; 4] = [
    "robertson",
    "hires",
    "van-der-pol-mu1000",
    "brusselator-1d-50",
];

/// Every problem the benchmark can run: the defaults and the 400-component
/// Brusselator of the native comparison.
pub fn benchmark_problems() -> CoreResult<Vec<BenchmarkProblem>> {
    let mut problems = default_problems()?;
    let (large, large_y0) = brusselator_problem(200)?;
    problems.push(BenchmarkProblem {
        id: "brusselator-1d-200",
        problem: large,
        y0: large_y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
    });
    Ok(problems)
}

fn default_problems() -> CoreResult<Vec<BenchmarkProblem>> {
    let (robertson, robertson_y0) = robertson_problem()?;
    let (hires, hires_y0) = hires_problem()?;
    let (vdp, vdp_y0) = stiff_van_der_pol_problem(1.0e3)?;
    let (brusselator, brusselator_y0) = brusselator_problem(50)?;
    Ok(vec![
        BenchmarkProblem {
            id: "robertson",
            problem: robertson,
            y0: robertson_y0,
            t_span: (0.0, 40.0),
            atol_scale: 1.0e-4,
        },
        BenchmarkProblem {
            id: "hires",
            problem: hires,
            y0: hires_y0,
            t_span: (0.0, 321.8122),
            atol_scale: 1.0e-4,
        },
        BenchmarkProblem {
            id: "van-der-pol-mu1000",
            problem: vdp,
            y0: vdp_y0,
            t_span: (0.0, 2000.0),
            atol_scale: 1.0,
        },
        BenchmarkProblem {
            id: "brusselator-1d-50",
            problem: brusselator,
            y0: brusselator_y0,
            t_span: (0.0, 10.0),
            atol_scale: 1.0,
        },
    ])
}

fn adaptive_config(problem: &BenchmarkProblem, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * problem.atol_scale,
        rtol,
        initial_step: INITIAL_STEP,
        min_step: 1.0e-14,
        max_step: problem.t_span.1 - problem.t_span.0,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

pub fn run_arm(
    arm: &str,
    problem: &BenchmarkProblem,
    rtol: f64,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    let adaptive = adaptive_config(problem, rtol);
    let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1])?;
    let (p, span, y0) = (&problem.problem, problem.t_span, problem.y0.as_slice());
    let tier_a = NewtonTolerancePolicy::ScaledToOuterTolerance;
    match arm {
        "rodas5p" => integrate_adaptive_observed_with_config(
            p,
            span,
            y0,
            IntegrationMethod::Sequential,
            Some(&LinearSolverConfig {
                method: LinearMethod::Direct,
                ..LinearSolverConfig::default()
            }),
            None,
            &adaptive,
            &output,
        ),
        "repo-bdf2" => {
            integrate_bdf_adaptive_observed(p, span, y0, &BdfConfig::default(), &adaptive, &output)
        }
        "repo-bdf2-tier-a" => integrate_bdf_adaptive_observed(
            p,
            span,
            y0,
            &BdfConfig {
                newton_tolerance: tier_a,
                ..BdfConfig::default()
            },
            &adaptive,
            &output,
        ),
        "repo-radau5" => integrate_radau_adaptive_observed(
            p,
            span,
            y0,
            &RadauConfig::default(),
            &adaptive,
            &output,
        ),
        "repo-radau5-tier-a" => integrate_radau_adaptive_observed(
            p,
            span,
            y0,
            &RadauConfig {
                newton_tolerance: tier_a,
                reuse_stage_lu_for_error_estimate: true,
                ..RadauConfig::default()
            },
            &adaptive,
            &output,
        ),
        other => Err(rodas5p_core::CoreError::InvalidInput(format!(
            "unknown benchmark arm {other}"
        ))),
    }
}

fn counters_json(c: &WorkCounters) -> Value {
    json!({
        "rhs_evaluations": c.rhs_evaluations,
        "jacobian_builds": c.jacobian_builds,
        "direct_factorizations": c.direct_factorizations,
        "linear_solves": c.linear_solves,
        "direct_solve_calls": c.direct_solve_calls,
        "nonlinear_iterations": c.nonlinear_iterations,
    })
}

/// Deterministic sample states for the right-hand-side parity check.
fn parity_states(y0: &[f64]) -> Vec<Vec<f64>> {
    let perturbed = y0
        .iter()
        .enumerate()
        .map(|(i, &y)| y * (1.0 + 0.1 * ((i + 1) as f64).sin()) + 0.01 * ((i + 1) as f64).cos())
        .collect();
    vec![y0.to_vec(), perturbed]
}

fn parity_samples(problem: &BenchmarkProblem) -> Result<Value> {
    let p = &problem.problem;
    let mut samples = Vec::new();
    for state in parity_states(&problem.y0) {
        let mut counters = WorkCounters::default();
        let f = p.eval_rhs(problem.t_span.0, &state, &mut counters)?;
        let jac = p.dense_jacobian(problem.t_span.0, &state, &mut counters)?;
        let rows = (0..p.dimension)
            .map(|i| (0..p.dimension).map(|j| jac[(i, j)]).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        samples.push(json!({ "t": problem.t_span.0, "y": state, "f": f, "jacobian": rows }));
    }
    Ok(json!(samples))
}

/// Every arm, problem and tolerance: `warmups` untimed runs, then
/// `repetitions` timed runs, which must all end in the same state.
pub fn stiff_benchmark(
    repetitions: usize,
    warmups: usize,
    problem_ids: &[String],
    arms: &[String],
) -> Result<Value> {
    anyhow::ensure!(repetitions >= 1, "at least one timed repetition");
    for arm in arms {
        anyhow::ensure!(ARMS.contains(&arm.as_str()), "unknown arm {arm}");
    }
    let mut problems = Vec::new();
    for problem in benchmark_problems()? {
        if problem_ids.iter().any(|id| id == problem.id) {
            problems.push(problem);
        }
    }
    anyhow::ensure!(
        problems.len() == problem_ids.len(),
        "unknown problem in {problem_ids:?}"
    );
    let mut rows = Vec::new();
    let mut parity = serde_json::Map::new();
    for problem in &problems {
        parity.insert(problem.id.into(), parity_samples(problem)?);
        for arm in arms.iter().map(String::as_str) {
            for &rtol in &TOLERANCES {
                for _ in 0..warmups {
                    let _ = run_arm(arm, problem, rtol);
                }
                let mut walls = Vec::with_capacity(repetitions);
                let mut first: Option<CoreResult<AdaptiveObservedIntegrationResult>> = None;
                let mut deterministic = true;
                for _ in 0..repetitions {
                    let started = Instant::now();
                    let result = run_arm(arm, problem, rtol);
                    walls.push(started.elapsed().as_secs_f64());
                    match (&first, &result) {
                        (None, _) => {}
                        (Some(Ok(a)), Ok(b)) => {
                            deterministic &= a.observed.y == b.observed.y
                                && a.observed.counters == b.observed.counters;
                        }
                        (Some(Err(_)), Err(_)) => {}
                        _ => deterministic = false,
                    }
                    if first.is_none() {
                        first = Some(result);
                    }
                }
                let mut sorted = walls.clone();
                sorted.sort_by(f64::total_cmp);
                let median = sorted[sorted.len() / 2];
                let row = match first.expect("one repetition") {
                    Ok(result) if result.observed.success => {
                        let d = &result.diagnostics;
                        json!({
                            "problem": problem.id, "arm": arm, "rtol": rtol,
                            "atol": rtol * problem.atol_scale,
                            "status": "completed",
                            "final_state": result.observed.y.last(),
                            "accepted_steps": d.accepted_macro_steps,
                            "rejected_steps": d.rejected_macro_steps,
                            "counters": counters_json(&result.observed.counters),
                            "wall_seconds": walls, "wall_median": median,
                            "deterministic": deterministic,
                        })
                    }
                    Ok(result) => json!({
                        "problem": problem.id, "arm": arm, "rtol": rtol,
                        "atol": rtol * problem.atol_scale,
                        "status": "failed", "message": result.observed.message,
                        "counters": counters_json(&result.observed.counters),
                        "wall_seconds": walls, "wall_median": median,
                        "deterministic": deterministic,
                    }),
                    Err(error) => json!({
                        "problem": problem.id, "arm": arm, "rtol": rtol,
                        "atol": rtol * problem.atol_scale,
                        "status": "failed", "message": error.to_string(),
                        "wall_seconds": walls, "wall_median": median,
                        "deterministic": deterministic,
                    }),
                };
                eprintln!(
                    "{} {} rtol={rtol:e}: {} median {:.4e} s",
                    problem.id, arm, row["status"], median
                );
                rows.push(row);
            }
        }
    }
    Ok(json!({
        "schema": SCHEMA,
        "timing_status": "DIAGNOSTIC: wall seconds on a shared host; statistical timing authority is on hold",
        "repetitions": repetitions,
        "warmups": warmups,
        "initial_step": INITIAL_STEP,
        "max_attempts": MAX_ATTEMPTS,
        "problems": problems.iter().map(|p| json!({
            "id": p.id, "dimension": p.problem.dimension, "t_span": [p.t_span.0, p.t_span.1],
            "y0": p.y0, "atol_scale": p.atol_scale,
        })).collect::<Vec<_>>(),
        "parity_samples": parity,
        "lu_microbench": lu_microbench(&[50, 200], 51)?,
        "rows": rows,
    }))
}

/// Median seconds of one dense LU factorization (faer partial pivoting, as
/// used by the RODAS5P arm) of two `2 cells` square matrices: the Brusselator
/// iteration matrix `I - 0.05 J(y0)` ("banded") and the full matrix
/// `1/(1 + |i - j|) + n delta_ij`, the same pair the native driver times.
fn lu_microbench(cells: &[usize], repetitions: usize) -> Result<Value> {
    let mut out = serde_json::Map::new();
    for &c in cells {
        let (problem, y0) = brusselator_problem(c)?;
        let n = problem.dimension;
        let mut counters = WorkCounters::default();
        let jac = problem.dense_jacobian(0.0, &y0, &mut counters)?;
        let mut banded = DenseMatrix::zeros(n, n);
        let mut full = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let delta = f64::from(u8::from(i == j));
                banded[(i, j)] = delta - 0.05 * jac[(i, j)];
                full[(i, j)] = 1.0 / (1.0 + i.abs_diff(j) as f64) + n as f64 * delta;
            }
        }
        let mut entry = serde_json::Map::new();
        for (label, matrix) in [("banded", &banded), ("full", &full)] {
            let mut walls = Vec::with_capacity(repetitions);
            for _ in 0..repetitions {
                let started = Instant::now();
                let lu = rodas5p_core::LuFactorization::new(matrix)?;
                walls.push(started.elapsed().as_secs_f64());
                std::hint::black_box(&lu);
            }
            walls.sort_by(f64::total_cmp);
            entry.insert(
                label.into(),
                json!({ "faer-partial-pivot-lu": walls[walls.len() / 2] }),
            );
        }
        out.insert(n.to_string(), Value::Object(entry));
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_jacobians_match_finite_differences() {
        for problem in benchmark_problems().unwrap() {
            if problem.problem.dimension > 100 {
                continue;
            }
            let p = &problem.problem;
            let n = p.dimension;
            for state in parity_states(&problem.y0) {
                let mut counters = WorkCounters::default();
                let jac = p.dense_jacobian(0.0, &state, &mut counters).unwrap();
                // Central differences are exact for the quadratic terms
                // (Robertson at y2 = 0 has a zero Jacobian entry whose
                // one-sided difference is -3e7 h).
                for j in 0..n {
                    let h = 1.0e-5 * state[j].abs().max(1.0);
                    let (mut up, mut down) = (state.clone(), state.clone());
                    up[j] += h;
                    down[j] -= h;
                    let fu = p.eval_rhs(0.0, &up, &mut counters).unwrap();
                    let fdn = p.eval_rhs(0.0, &down, &mut counters).unwrap();
                    for i in 0..n {
                        let fd = (fu[i] - fdn[i]) / (2.0 * h);
                        let scale = jac[(i, j)].abs().max(1.0);
                        assert!(
                            (fd - jac[(i, j)]).abs() <= 1.0e-6 * scale,
                            "{} J[{i},{j}] = {} vs {fd}",
                            problem.id,
                            jac[(i, j)]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_arm_completes_a_loose_hires_run() {
        let problems = benchmark_problems().unwrap();
        let hires = problems.iter().find(|p| p.id == "hires").unwrap();
        for arm in ARMS {
            let result = run_arm(arm, hires, 1.0e-3).unwrap();
            assert!(
                result.observed.success,
                "{arm}: {}",
                result.observed.message
            );
        }
    }
}
