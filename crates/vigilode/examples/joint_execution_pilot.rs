//! Actual ODE ensemble dispatch with identity changes and observed-cost drift.
//! Timing is diagnostic; no performance assertion or implicit calibration.
use rodas5p_core::CoreResult;
use rodas5p_integrators::ParallelExecution;
use serde_json::json;
use std::{hint::black_box, time::Instant};
use vigilode::execution_policy::*;
use vigilode::selection_evidence::SelectionIdentity;
use vigilode::{Atol, Dopri5, Options, Problem};

#[derive(Clone, Copy)]
struct Job {
    rate: f64,
    horizon: f64,
}
fn solve(job: &Job) -> CoreResult<(f64, f64)> {
    let rate = job.rate;
    let p = Problem::new(1, move |_, y, out| {
        out[0] = rate * y[0] * (1.0 - y[0]);
        Ok(())
    })
    .autonomous();
    let o = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        h0: 1e-4,
        ..Default::default()
    };
    let s = Dopri5::solve(&p, (0.0, job.horizon), &[0.1], &o).unwrap();
    assert!(s.success);
    let value = s.final_state[0];
    let exact = 1.0 / (1.0 + 9.0 * (-job.rate * job.horizon).exp());
    let error = (value - exact).abs();
    assert!(error < 1e-7);
    Ok((value, error))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workers = std::env::args()
        .nth(1)
        .unwrap_or("2".into())
        .parse::<usize>()?;
    let batch = 64;
    let mut identity = ExecutionIdentity {
        workload: WorkloadKey {
            model_epoch: 1,
            method_tag: 5,
            dimension: 1,
            batch_len: batch,
            layer: ExecutionLayer::Ensemble,
            environment_epoch: 1,
        },
        comparison: SelectionIdentity {
            model_epoch: 1,
            provider_epoch: 1,
            physical_norm_epoch: 1,
            tolerance_epoch: 1,
            window_epoch: 1,
            ..Default::default()
        },
    };
    let jobs = |horizon| {
        (0..batch)
            .map(|i| Job {
                rate: 0.5 + 1.5 * i as f64 / batch as f64,
                horizon,
            })
            .collect::<Vec<_>>()
    };
    let mut executor = AdaptiveBatchExecutor::new(workers, ExecutionPolicyConfig::monitored())?;
    let serial = ParallelExecution::sequential();
    let direct_pool_started = Instant::now();
    let parallel = ParallelExecution::rayon(workers)?;
    let direct_parallel_startup_seconds = direct_pool_started.elapsed().as_secs_f64();
    for (phase, horizon) in [
        ("calibrated-short", 0.05),
        ("unannounced-cost-drift", 32.0),
        ("explicit-epoch-refresh", 32.0),
    ] {
        let data = jobs(horizon);
        if phase == "explicit-epoch-refresh" {
            identity.comparison.window_epoch += 1;
            identity.comparison.provider_epoch += 1;
        }
        let calibration_seconds = if phase != "unannounced-cost-drift" {
            executor.calibrate_replay_safe_for(identity, &data, solve, |a, b| a == b)?
        } else {
            0.0
        };
        let mut rows = Vec::new();
        let mut reference: Option<Vec<(f64, f64)>> = None;
        for repeat in 0..5 {
            for order in 0..3 {
                let arm = (repeat + order) % 3;
                let start = Instant::now();
                let (output, decision) = match arm {
                    0 => (serial.map_ordered(&data, solve)?, None),
                    1 => (parallel.map_ordered(&data, solve)?, None),
                    _ => {
                        let (o, d) =
                            executor.map_ordered_for(identity, 5 - repeat, false, &data, solve)?;
                        (o, Some(d))
                    }
                };
                let elapsed = start.elapsed().as_secs_f64();
                let output_parity = reference.as_ref().is_none_or(|r| r == &output);
                assert!(output_parity);
                if reference.is_none() {
                    reference = Some(output.clone());
                }
                let error = output.iter().map(|x| x.1).fold(0.0, f64::max);
                black_box(&output);
                rows.push(
                    json!({"repeat":repeat,"arm":(["serial","parallel","adaptive"][arm]),
                    "inclusive_seconds":elapsed,"decision":decision,"output_parity":output_parity,
                    "maximum_endpoint_error":error,"physical_error_cap":1e-7,
                    "output_checksum":output.iter().map(|x|x.0).sum::<f64>()}),
                );
            }
        }
        println!(
            "{}",
            json!({"schema":"vigilode.joint-execution-pilot.v1","phase":phase,
            "identity":identity,"workers":workers,"physical_span":[0.0,horizon],
            "deliberately_stale_window_epoch":phase=="unannounced-cost-drift",
            "calibration_seconds":calibration_seconds,"startup_seconds_cumulative":executor.startup_seconds,
            "direct_parallel_startup_seconds_cumulative":direct_parallel_startup_seconds,
            "warmup_seconds_last_calibration":executor.warmup_seconds,
            "calibration_samples_last":executor.calibration_samples,"telemetry_cumulative":executor.selector.telemetry(),
            "rows":rows,"scope":"Actual logistic ODE ensembles; same-horizon alternatives within each phase; empirical timing; stale-window phase is a detector stress case, never certified eligible reuse"})
        );
    }
    Ok(())
}
