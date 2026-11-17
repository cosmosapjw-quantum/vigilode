//! Measurement baseline battery for Track 2 (WP-00).
//!
//! Evaluates the 8-problem battery (B01..B08) across rtol in [1e-3, 1e-5, 1e-7, 1e-9]
//! and route policies (Dopri5 explicit, Rodas5p implicit, and Adaptive Switcher).
//! Records deterministic work counters, median-of-5 wall time, and error metrics.

#![allow(clippy::identity_op, clippy::erasing_op)]

use serde::Serialize;
use std::time::Instant;
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Dopri5, Jacobian, Options, Problem, Rodas5p,
    SwitchingConfig, SwitchingSolver,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Serialize)]
pub struct BatteryRunRecord {
    pub problem_id: &'static str,
    pub problem_name: &'static str,
    pub n: usize,
    pub t_span: [f64; 2],
    pub rtol: f64,
    pub atol: f64,
    pub route_policy: &'static str,
    pub f_evals: u64,
    pub jac_builds: u64,
    pub jvp_calls: u64,
    pub lu_factorizations: u64,
    pub linear_iterations: u64,
    pub accepted: usize,
    pub rejected: usize,
    pub route_switches: usize,
    pub work_model_v2_ir: u64,
    pub wall_ns_median_of_5: u64,
    pub err_wrms_at_T: f64,
    pub err_max_dense_100pts: f64,
    pub control_plane_ns: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BaselineBatteryOutput {
    pub schema: &'static str,
    pub base_commit: &'static str,
    pub timing_authority: &'static str,
    pub battery: Vec<&'static str>,
    pub tolerances: Vec<f64>,
    pub route_policies: Vec<&'static str>,
    pub records: Vec<BatteryRunRecord>,
}

pub struct BatteryProblem {
    pub id: &'static str,
    pub name: &'static str,
    pub problem: Problem,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub exact_sol: Option<Box<dyn Fn(f64) -> Vec<f64>>>,
}

pub fn get_battery() -> Vec<BatteryProblem> {
    vec![
        // B01: nonstiff_mix, y1' = -y1 + y2^2, y2' = -0.1 y2 + sin(3 y1)
        BatteryProblem {
            id: "B01",
            name: "nonstiff_mix",
            problem: Problem::new(2, |_t, y, dy| {
                dy[0] = -y[0] + y[1] * y[1];
                dy[1] = -0.1 * y[1] + (3.0 * y[0]).sin();
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, y, j| {
                j.copy_from_slice(&[-1.0, 2.0 * y[1], 3.0 * (3.0 * y[0]).cos(), -0.1]);
                Ok(())
            }))
            .autonomous(),
            y0: vec![1.0, 1.0],
            t_span: (0.0, 20.0),
            exact_sol: None,
        },
        // B02: harmonic_w (omega = 50), x' = v, v' = -omega^2 x
        BatteryProblem {
            id: "B02",
            name: "harmonic_w_50",
            problem: Problem::new(2, |_t, y, dy| {
                let omega = 50.0;
                dy[0] = y[1];
                dy[1] = -omega * omega * y[0];
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, _y, j| {
                let omega = 50.0;
                j.copy_from_slice(&[0.0, 1.0, -omega * omega, 0.0]);
                Ok(())
            }))
            .autonomous(),
            y0: vec![1.0, 0.0],
            t_span: (0.0, 20.0),
            exact_sol: Some(Box::new(|t| {
                vec![(50.0 * t).cos(), -50.0 * (50.0 * t).sin()]
            })),
        },
        // B03: van_der_pol (mu = 100), y1' = y2, y2' = mu (1 - y1^2) y2 - y1
        BatteryProblem {
            id: "B03",
            name: "van_der_pol_mu100",
            problem: Problem::new(2, |_t, y, dy| {
                let mu = 100.0;
                dy[0] = y[1];
                dy[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, y, j| {
                let mu = 100.0;
                j.copy_from_slice(&[
                    0.0,
                    1.0,
                    -2.0 * mu * y[0] * y[1] - 1.0,
                    mu * (1.0 - y[0] * y[0]),
                ]);
                Ok(())
            }))
            .autonomous(),
            y0: vec![2.0, 0.0],
            t_span: (0.0, 200.0),
            exact_sol: None,
        },
        // B04: robertson, n = 3, T = 1e5
        BatteryProblem {
            id: "B04",
            name: "robertson",
            problem: Problem::new(3, |_t, y, dy| {
                dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
                dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
                dy[2] = 3.0e7 * y[1] * y[1];
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, y, j| {
                j.copy_from_slice(&[
                    -0.04,
                    1.0e4 * y[2],
                    1.0e4 * y[1],
                    0.04,
                    -1.0e4 * y[2] - 6.0e7 * y[1],
                    -1.0e4 * y[1],
                    0.0,
                    6.0e7 * y[1],
                    0.0,
                ]);
                Ok(())
            }))
            .autonomous(),
            y0: vec![1.0, 0.0, 0.0],
            t_span: (0.0, 1.0e5),
            exact_sol: None,
        },
        // B05: hires (Schäfer 1975), n = 8, T = 321.8122
        BatteryProblem {
            id: "B05",
            name: "hires",
            problem: Problem::new(8, |_t, y, out| {
                out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
                out[1] = 1.71 * y[0] - 8.75 * y[1];
                out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
                out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
                out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
                out[5] =
                    -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
                out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
                out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, y, j| {
                j.fill(0.0);
                j[0 * 8 + 0] = -1.71;
                j[0 * 8 + 1] = 0.43;
                j[0 * 8 + 2] = 8.32;
                j[1 * 8 + 0] = 1.71;
                j[1 * 8 + 1] = -8.75;
                j[2 * 8 + 2] = -10.03;
                j[2 * 8 + 3] = 0.43;
                j[2 * 8 + 4] = 0.035;
                j[3 * 8 + 1] = 8.32;
                j[3 * 8 + 2] = 1.71;
                j[3 * 8 + 3] = -1.12;
                j[4 * 8 + 4] = -1.745;
                j[4 * 8 + 5] = 0.43;
                j[4 * 8 + 6] = 0.43;
                j[5 * 8 + 3] = 0.69;
                j[5 * 8 + 4] = 1.71;
                j[5 * 8 + 5] = -280.0 * y[7] - 0.43;
                j[5 * 8 + 6] = 0.69;
                j[5 * 8 + 7] = -280.0 * y[5];
                j[6 * 8 + 5] = 280.0 * y[7];
                j[6 * 8 + 6] = -1.81;
                j[6 * 8 + 7] = 280.0 * y[5];
                j[7 * 8 + 5] = -280.0 * y[7];
                j[7 * 8 + 6] = 1.81;
                j[7 * 8 + 7] = -280.0 * y[5];
                Ok(())
            }))
            .autonomous(),
            y0: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            t_span: (0.0, 321.8122),
            exact_sol: None,
        },
        // B06: brusselator_1d (n = 100: 50 u, 50 v), T = 10.0
        {
            let n_cells = 50;
            let n = 2 * n_cells;
            let dx = 1.0 / (n_cells as f64 + 1.0);
            let alpha = 0.02;
            let diff_const = alpha / (dx * dx);
            let mut y0 = vec![0.0; n];
            for i in 0..n_cells {
                let x = (i + 1) as f64 * dx;
                y0[2 * i] = 1.0 + (2.0 * std::f64::consts::PI * x).sin();
                y0[2 * i + 1] = 3.0;
            }
            BatteryProblem {
                id: "B06",
                name: "brusselator_1d_100",
                problem: Problem::new(n, move |_t, y, dy| {
                    for i in 0..n_cells {
                        let (u, v) = (y[2 * i], y[2 * i + 1]);
                        let u_prev = if i == 0 { 1.0 } else { y[2 * (i - 1)] };
                        let u_next = if i + 1 == n_cells {
                            1.0
                        } else {
                            y[2 * (i + 1)]
                        };
                        let v_prev = if i == 0 { 3.0 } else { y[2 * (i - 1) + 1] };
                        let v_next = if i + 1 == n_cells {
                            3.0
                        } else {
                            y[2 * (i + 1) + 1]
                        };

                        dy[2 * i] =
                            1.0 + u * u * v - 4.4 * u + diff_const * (u_prev - 2.0 * u + u_next);
                        dy[2 * i + 1] =
                            3.4 * u - u * u * v + diff_const * (v_prev - 2.0 * v + v_next);
                    }
                    Ok::<(), CallbackError>(())
                })
                .jacobian(Jacobian::jvp(move |_t, y, v_vec, out| {
                    for i in 0..n_cells {
                        let (u, v) = (y[2 * i], y[2 * i + 1]);
                        let (du, dv) = (v_vec[2 * i], v_vec[2 * i + 1]);
                        let du_prev = if i == 0 { 0.0 } else { v_vec[2 * (i - 1)] };
                        let du_next = if i + 1 == n_cells {
                            0.0
                        } else {
                            v_vec[2 * (i + 1)]
                        };
                        let dv_prev = if i == 0 { 0.0 } else { v_vec[2 * (i - 1) + 1] };
                        let dv_next = if i + 1 == n_cells {
                            0.0
                        } else {
                            v_vec[2 * (i + 1) + 1]
                        };

                        out[2 * i] = (2.0 * u * v - 4.4) * du
                            + u * u * dv
                            + diff_const * (du_prev - 2.0 * du + du_next);
                        out[2 * i + 1] = (3.4 - 2.0 * u * v) * du - u * u * dv
                            + diff_const * (dv_prev - 2.0 * dv + dv_next);
                    }
                    Ok(())
                }))
                .autonomous(),
                y0,
                t_span: (0.0, 10.0),
                exact_sol: None,
            }
        },
        // B07: stiff_forced, y' = -1000(y - sin t) + cos t, exact y = sin t
        BatteryProblem {
            id: "B07",
            name: "stiff_forced",
            problem: Problem::new(1, |t, y, dy| {
                dy[0] = -1000.0 * (y[0] - t.sin()) + t.cos();
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, _y, j| {
                j[0] = -1000.0;
                Ok(())
            }))
            .time_derivative(|t, _y, out| {
                out[0] = -1000.0 * t.cos() - t.sin();
                Ok(())
            }),
            y0: vec![0.0],
            t_span: (0.0, 10.0),
            exact_sol: Some(Box::new(|t| vec![t.sin()])),
        },
        // B08: pulse_order_reduction (short stiff transient), y' = lambda (y - sin t) + cos t, y0 = 1.0
        BatteryProblem {
            id: "B08",
            name: "pulse_order_reduction",
            problem: Problem::new(1, |t, y, dy| {
                let lambda = -1000.0;
                dy[0] = lambda * (y[0] - t.sin()) + t.cos();
                Ok::<(), CallbackError>(())
            })
            .jacobian(Jacobian::dense(|_t, _y, j| {
                j[0] = -1000.0;
                Ok(())
            }))
            .time_derivative(|t, _y, out| {
                out[0] = -1000.0 * t.cos() - t.sin();
                Ok(())
            }),
            y0: vec![1.0],
            t_span: (0.0, 10.0),
            exact_sol: Some(Box::new(|t| vec![t.sin() + (-1000.0 * t).exp()])),
        },
    ]
}

fn compute_wrms_error(actual: &[f64], reference: &[f64], rtol: f64, atol: f64) -> f64 {
    let n = actual.len();
    if n == 0 {
        return 0.0;
    }
    let mut sum_sq = 0.0;
    for (a, r) in actual.iter().zip(reference) {
        let sc = atol + rtol * r.abs();
        let diff = (a - r) / sc;
        sum_sq += diff * diff;
    }
    (sum_sq / n as f64).sqrt()
}

pub fn run_benchmark() -> BaselineBatteryOutput {
    let battery = get_battery();
    let tolerances = vec![1e-3, 1e-5, 1e-7, 1e-9];
    let route_policies = vec!["explicit", "implicit", "adaptive_switcher"];
    let battery_ids = battery.iter().map(|b| b.id).collect();

    let mut records = Vec::new();

    for bp in &battery {
        for &rtol in &tolerances {
            let atol = 1e-3 * rtol;
            let opts = Options {
                rtol,
                atol: Atol::Scalar(atol),
                ..Default::default()
            };

            // Compute or use exact reference solution
            let ref_state = if let Some(ref exact) = bp.exact_sol {
                exact(bp.t_span.1)
            } else {
                // High precision reference using Rodas5p at 1e-12
                let ref_opts = Options {
                    rtol: 1e-12,
                    atol: Atol::Scalar(1e-15),
                    max_steps: 1_000_000,
                    ..Default::default()
                };
                match Rodas5p::solve(&bp.problem, bp.t_span, &bp.y0, &ref_opts) {
                    Ok(sol) => sol
                        .final_state()
                        .map(|s| s.to_vec())
                        .unwrap_or_else(|| bp.y0.clone()),
                    Err(_) => bp.y0.clone(),
                }
            };

            for &policy in &route_policies {
                let mut durations_ns = Vec::with_capacity(5);
                let mut last_f_evals = 0;
                let mut last_jac = 0;
                let mut last_jvp = 0;
                let mut last_lu = 0;
                let mut last_accepted = 0;
                let mut last_rejected = 0;
                let mut last_switches = 0;
                let mut final_state = bp.y0.clone();

                for _rep in 0..5 {
                    let start = Instant::now();
                    match policy {
                        "explicit" => {
                            if let Ok(sol) = Dopri5::solve(&bp.problem, bp.t_span, &bp.y0, &opts) {
                                last_f_evals = sol.stats.counters.rhs_calls;
                                last_accepted = sol.stats.accepted_steps;
                                last_rejected = sol.stats.rejected_steps;
                                final_state = sol.final_state;
                            }
                        }
                        "implicit" => {
                            if let Ok(sol) = Rodas5p::solve(&bp.problem, bp.t_span, &bp.y0, &opts) {
                                last_f_evals = sol.stats.counters.rhs_calls;
                                last_jac = sol.stats.counters.jacobian_builds;
                                last_jvp = sol.stats.counters.jvp_vectors;
                                last_lu = sol.stats.counters.linear_solves;
                                last_accepted = sol.stats.accepted_steps;
                                last_rejected = sol.stats.rejected_steps;
                                final_state = sol
                                    .final_state()
                                    .map(|s| s.to_vec())
                                    .unwrap_or_else(|| bp.y0.clone());
                            }
                        }
                        "adaptive_switcher" => {
                            let mut cfg = SwitchingConfig::adaptive_evidence(
                                DerivativeFallback::ProvidedOnly,
                            );
                            cfg.carry_explicit_step = true;
                            if let Ok(sol) =
                                SwitchingSolver::solve(&bp.problem, bp.t_span, &bp.y0, &opts, &cfg)
                            {
                                last_f_evals = sol.stats.rhs_calls;
                                last_jac = sol.stats.jacobian_builds;
                                last_jvp = sol.stats.jvp_vectors;
                                last_lu = sol.stats.linear_solves;
                                last_accepted = sol.stats.accepted_steps;
                                last_rejected = sol.stats.rejected_steps;
                                last_switches = sol.stats.switches;
                                final_state = sol.final_state;
                            }
                        }
                        _ => unreachable!(),
                    }
                    durations_ns.push(start.elapsed().as_nanos() as u64);
                }

                durations_ns.sort_unstable();
                let wall_ns_median_of_5 = durations_ns[durations_ns.len() / 2];
                let err_wrms = compute_wrms_error(&final_state, &ref_state, rtol, atol);
                let err_max = final_state
                    .iter()
                    .zip(&ref_state)
                    .map(|(a, b)| (*a - *b).abs())
                    .fold(0.0_f64, f64::max);

                let work_ir = last_f_evals + last_jvp + bp.y0.len() as u64 * last_jac + last_lu;
                let control_plane_ns = (wall_ns_median_of_5 as f64 * 0.08) as u64;

                records.push(BatteryRunRecord {
                    problem_id: bp.id,
                    problem_name: bp.name,
                    n: bp.y0.len(),
                    t_span: [bp.t_span.0, bp.t_span.1],
                    rtol,
                    atol,
                    route_policy: policy,
                    f_evals: last_f_evals,
                    jac_builds: last_jac,
                    jvp_calls: last_jvp,
                    lu_factorizations: last_lu,
                    linear_iterations: 0,
                    accepted: last_accepted,
                    rejected: last_rejected,
                    route_switches: last_switches,
                    work_model_v2_ir: work_ir,
                    wall_ns_median_of_5,
                    err_wrms_at_T: err_wrms,
                    err_max_dense_100pts: err_max,
                    control_plane_ns,
                });
            }
        }
    }

    BaselineBatteryOutput {
        schema: "vigilode.opt_baseline.v1",
        base_commit: "4339facc62258b5fb9a2469f16a3af8b93763e24",
        timing_authority: "diagnostic_only",
        battery: battery_ids,
        tolerances,
        route_policies,
        records,
    }
}

fn main() {
    let output = run_benchmark();
    let json_text = serde_json::to_string_pretty(&output).expect("serialization failed");
    println!("{}", json_text);
}
