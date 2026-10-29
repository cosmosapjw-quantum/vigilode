//! Native, derivative-free pilot on the published exponential-chirp equation.
//! Measures full physical DOPRI at requested physical WRMS atol and chart DOPRI
//! at a stricter physical-Linf sufficient local budget. Endpoint errors and
//! the common endpoint cap are reported separately; achieved errors differ.
use serde_json::json;
use std::time::Instant;
use vigilode::{Atol, CallbackError, Dopri5, ExponentialChirpChart, Options, Problem};

fn main() {
    let gamma: f64 = 0.8;
    let beta: f64 = 1.2;
    let end: f64 = 8.0;
    let chart = ExponentialChirpChart::new(1.0, beta, gamma, (0.0, end)).unwrap();
    let mut rows = Vec::new();
    for epsilon in [0.0, 0.3] {
        for physical_atol in [1e-6, 1e-8] {
            for mode in ["physical-dopri5", "declared-chart-dopri5"] {
                let transformed = mode == "declared-chart-dopri5";
                let problem = Problem::new(2, move |t, y, out| {
                    if transformed {
                        let physical = chart
                            .to_physical(t, [y[0], y[1]])
                            .map_err(CallbackError::Fatal)?;
                        let radius2 = y[0] * y[0] + y[1] * y[1];
                        let factor = -epsilon * radius2;
                        let value = chart
                            .rotating_rhs(
                                t,
                                [y[0], y[1]],
                                [factor * physical[0], factor * physical[1]],
                            )
                            .map_err(CallbackError::Fatal)?;
                        out.copy_from_slice(&value);
                    } else {
                        let omega = chart.frequency(t).map_err(CallbackError::Fatal)?;
                        let p = (y[1] + gamma * y[0]) / omega;
                        let factor = -epsilon * (y[0] * y[0] + p * p);
                        out[0] = y[1] + factor * y[0];
                        out[1] = -(2.0 * gamma - beta) * y[1]
                            - (omega * omega + gamma * gamma - gamma * beta) * y[0]
                            + factor * y[1];
                    }
                    Ok(())
                });
                let atol = if transformed {
                    chart.conservative_local_atol(physical_atol).unwrap()
                } else {
                    physical_atol
                };
                let options = Options {
                    rtol: 0.0,
                    atol: Atol::Scalar(atol),
                    h0: 1e-3,
                    max_steps: 200_000,
                    ..Default::default()
                };
                let initial = if transformed {
                    vec![1.0, 0.0]
                } else {
                    vec![1.0, -gamma]
                };
                // One warmup followed by five separately timed solves.
                let mut times = Vec::new();
                let mut last = None;
                for k in 0..6 {
                    let start = Instant::now();
                    let solution = Dopri5::solve(&problem, (0.0, end), &initial, &options).unwrap();
                    let elapsed = start.elapsed().as_secs_f64();
                    if k > 0 {
                        times.push(elapsed);
                    }
                    last = Some(solution);
                }
                let solution = last.unwrap();
                let t = solution.final_time;
                let result = if transformed {
                    chart
                        .to_physical(t, [solution.final_state[0], solution.final_state[1]])
                        .unwrap()
                } else {
                    [solution.final_state[0], solution.final_state[1]]
                };
                let radius = (-gamma * t).exp()
                    / (1.0 + (epsilon / gamma) * (1.0 - (-2.0 * gamma * t).exp())).sqrt();
                let phi = chart.phase(t).unwrap();
                let omega = chart.frequency(t).unwrap();
                let exact = [
                    radius * phi.cos(),
                    radius * (-gamma * phi.cos() - omega * phi.sin()),
                ];
                let actual_error = (result[0] - exact[0])
                    .abs()
                    .max((result[1] - exact[1]).abs());
                times.sort_by(f64::total_cmp);
                rows.push(json!({"case":if epsilon==0.0{"catalog-15-exponential-chirp"}else{"equivariant-cubic-chirp"},
                    "method":mode,"physical_local_atol":physical_atol,"transformed_atol":atol,"rtol":0.0,
                    "success":solution.success,"final_time":t,"physical_final_state":result,
                    "analytic_binary64_reference":exact,"actual_endpoint_linf_error":actual_error,
                    "endpoint_cap_1e_minus_4_pass":solution.success&&actual_error<=1e-4,
                    "median_seconds":times[2],"timed_samples_seconds":times,
                    "accepted":solution.stats.accepted_steps,"rejected":solution.stats.rejected_steps,
                    "rhs_calls":solution.stats.counters.rhs_calls,"jacobian_builds":solution.stats.counters.jacobian_builds,
                    "jvp_calls":solution.stats.counters.jvp_calls,"linear_solves":solution.stats.counters.linear_solves,
                    "nonlinear_solves":solution.stats.counters.nonlinear_solves,
                    "scope":"declared family; transformed physical error majorant; actual errors differ; no generic winner"}));
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"schema":"vigilode.oscillatory-chart-pilot.v1","rows":rows})
        )
        .unwrap()
    );
}
