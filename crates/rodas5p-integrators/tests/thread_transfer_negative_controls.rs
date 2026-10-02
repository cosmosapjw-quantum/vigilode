//! Method-labelled negative controls (research node
//! `research/thread_transfer_negative_controls_20261002`, thread-transfer DAG
//! node P1-NEGATIVE-CONTROLS): the RVJ5 counterexamples of the prior thread,
//! run with the repository's own methods. Each row carries its method; no
//! outcome is transferred from one method to another, and no stage
//! certificate or embedded proxy is treated as an ODE error bound.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use std::sync::Arc;

use common::write_output;
use rodas5p_core::{CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    ExponentialKrylovConfig, OdeProblem, ParallelExecution, exprb43_step, pexprb54s4_step,
    sequential_step,
};
use serde_json::{Value, json};

const METHODS: [&str; 3] = ["rodas5p", "exprb43", "pexprb54s4"];

/// `x' = x^2, y' = (-kappa + 2x) y + x^2`; slow manifold `y = x^2 / kappa`.
fn semilinear(kappa: f64) -> OdeProblem {
    OdeProblem::new(
        format!("semilinear-kappa{kappa:e}"),
        2,
        Arc::new(move |_t: f64, z: &[f64], out: &mut [f64]| {
            let (x, y) = (z[0], z[1]);
            out[0] = x * x;
            out[1] = (-kappa + 2.0 * x) * y + x * x;
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, z: &[f64]| {
            let (x, y) = (z[0], z[1]);
            DenseMatrix::new(
                2,
                2,
                vec![2.0 * x, 0.0, 2.0 * y + 2.0 * x, -kappa + 2.0 * x],
            )
        })),
        Some(Arc::new(
            move |_t: f64, z: &[f64], v: &[f64], out: &mut [f64]| {
                let (x, y) = (z[0], z[1]);
                out[0] = 2.0 * x * v[0];
                out[1] = (2.0 * y + 2.0 * x) * v[0] + (-kappa + 2.0 * x) * v[1];
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// `y' = lambda (y - t^5) + 5 t^4`, exact solution `t^5` from `y(0) = 0`.
fn quintic_pr(lambda: f64) -> OdeProblem {
    OdeProblem::new(
        format!("quintic-pr-lambda{lambda:e}"),
        1,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = lambda * (y[0] - t.powi(5)) + 5.0 * t.powi(4);
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, _y: &[f64]| {
            DenseMatrix::new(1, 1, vec![lambda])
        })),
        Some(Arc::new(
            move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
                out[0] = lambda * v[0];
                Ok(())
            },
        )),
        Some(Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
            out[0] = -5.0 * lambda * t.powi(4) + 20.0 * t.powi(3);
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap()
}

/// One step of `method`: the new state and the method's own embedded error
/// vector (RODAS5P: `error_vector`; exponential: `y_new - y_embedded`).
/// Nonautonomous problems go to the exponential methods time-augmented.
fn step(
    method: &str,
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
) -> CoreResult<(Vec<f64>, Vec<f64>)> {
    match method {
        "rodas5p" => {
            let direct = LinearSolverConfig {
                method: LinearMethod::Direct,
                ..LinearSolverConfig::default()
            };
            let mut counters = WorkCounters::default();
            let r = sequential_step(
                problem,
                t,
                y,
                h,
                &direct,
                None,
                1.0,
                1.0,
                true,
                &mut counters,
            )?;
            Ok((r.y_new, r.error_vector))
        }
        _ => {
            let (augmented, state) = if problem.autonomous {
                (problem.clone(), y.to_vec())
            } else {
                let mut state = y.to_vec();
                state.push(t);
                (problem.time_augmented_clone()?, state)
            };
            let config = ExponentialKrylovConfig::default();
            let report = if method == "exprb43" {
                exprb43_step(&augmented, t, &state, h, config)?
            } else {
                pexprb54s4_step(
                    &augmented,
                    t,
                    &state,
                    h,
                    config,
                    &ParallelExecution::sequential(),
                )?
            };
            let n = y.len();
            let embedded = report
                .y_embedded
                .clone()
                .unwrap_or_else(|| report.y_new.clone());
            let error = report.y_new[..n]
                .iter()
                .zip(&embedded[..n])
                .map(|(a, b)| a - b)
                .collect();
            Ok((report.y_new[..n].to_vec(), error))
        }
    }
}

fn record<T: serde::Serialize>(outcome: CoreResult<T>) -> Value {
    match outcome {
        Ok(value) => json!(value),
        Err(error) => json!({"error": error.to_string()}),
    }
}

#[test]
fn negative_controls_by_method() {
    let mut rows = Vec::new();
    let mut completed = true;
    let mut verdicts = serde_json::Map::new();
    for method in METHODS {
        // Two-step semilinear no-go, kappa = h^-6.
        let mut two_step = Vec::new();
        let mut ratios = Vec::new();
        for n in [16_u32, 32, 64, 128, 256] {
            let h = 1.0 / n as f64;
            let kappa = (n as f64).powi(6);
            let problem = semilinear(kappa);
            let outcome = (|| -> CoreResult<Value> {
                let (first, _) = step(method, &problem, 0.0, &[1.0, 1.0 / kappa], h)?;
                let (second, _) = step(method, &problem, h, &first, h)?;
                let x_exact = 1.0 / (1.0 - 2.0 * h);
                let y_exact = x_exact * x_exact / kappa;
                let ratio = (second[1] - y_exact) / h.powi(3);
                ratios.push((n, ratio));
                Ok(json!({
                    "N": n, "kappa": kappa,
                    "y_error_over_h3": ratio,
                    "x_error_over_h5": (second[0] - x_exact) / h.powi(5),
                    "manifold_defect_times_kappa": (second[1] - second[0] * second[0] / kappa) * kappa,
                    "finite": second.iter().all(|v| v.is_finite()),
                }))
            })();
            completed &= outcome.as_ref().is_ok_and(|v| v["finite"] == json!(true));
            two_step.push(record(outcome));
        }
        // Fixed kappa = 8: classical order.
        let mut fixed = Vec::new();
        let mut errors = Vec::new();
        for n in [8_u32, 16, 32, 64] {
            let h = 1.0 / (4.0 * n as f64);
            let kappa = 8.0;
            let problem = semilinear(kappa);
            let outcome = (|| -> CoreResult<f64> {
                let mut z = vec![1.0, 1.0 / kappa];
                for k in 0..n {
                    z = step(method, &problem, k as f64 * h, &z, h)?.0;
                }
                let x = 4.0 / 3.0;
                Ok(((z[0] - x).powi(2) + (z[1] - x * x / kappa).powi(2)).sqrt())
            })();
            if let Ok(error) = outcome {
                errors.push(error);
            }
            completed &= outcome.is_ok();
            fixed.push(json!({"N": n, "h": h, "error": record(outcome)}));
        }
        let rates = errors
            .windows(2)
            .map(|w| (w[0] / w[1]).log2())
            .collect::<Vec<_>>();
        // Quintic PR embedded effectivity, one step from (0, 0).
        let mut quintic = Vec::new();
        let mut last_effectivity = f64::NAN;
        let h: f64 = 0.1;
        for z in [-10.0, -100.0, -1.0e4, -1.0e6] {
            let problem = quintic_pr(z / h);
            let outcome = (|| -> CoreResult<Value> {
                let (y1, error) = step(method, &problem, 0.0, &[0.0], h)?;
                let true_error = (y1[0] - h.powi(5)).abs();
                let estimate = error[0].abs();
                let effectivity = estimate / true_error;
                last_effectivity = effectivity;
                Ok(
                    json!({"z": z, "true_local_error": true_error, "embedded_estimate": estimate,
                    "effectivity": effectivity, "finite": y1[0].is_finite() && estimate.is_finite()}),
                )
            })();
            completed &= outcome.as_ref().is_ok_and(|v| v["finite"] == json!(true));
            quintic.push(record(outcome));
        }
        let near = |n: u32| {
            ratios
                .iter()
                .find(|(m, _)| *m == n)
                .is_some_and(|(_, r)| (r - 4.0 / 3.0).abs() <= 0.1)
        };
        let shares_no_go = near(128) && near(256);
        let shares_blindness = last_effectivity < 1.0e-5;
        verdicts.insert(
            method.into(),
            json!({"shares_rvj5_two_step_no_go": shares_no_go, "shares_rvj5_embedded_blindness": shares_blindness}),
        );
        println!(
            "{method}: two-step y/h^3 {:?}; fixed-kappa rates {rates:?}; effectivity at z=-1e6 {last_effectivity:e}",
            ratios
        );
        rows.push(json!({
            "method": method,
            "two_step_semilinear": two_step,
            "fixed_kappa_8": {"data": fixed, "observed_rates": rates},
            "quintic_pr": quintic,
        }));
    }
    let result = json!({
        "schema": "vigilode-thread-transfer-negative-controls-native-v1",
        "arithmetic": "binary64; fixed steps h = 1/N (N a power of two) or h = 0.1",
        "gate_native": {"runs_complete": completed},
        "per_method": verdicts,
        "rows": rows,
        "claim_ceiling": "specific-method, specific-problem regression results; nothing is transferred between methods",
    });
    write_output("THREAD_TRANSFER_NEGATIVE_CONTROLS_OUTPUT", &result);
    println!(
        "{}",
        json!({"gate_native": result["gate_native"], "per_method": result["per_method"]})
    );
}
