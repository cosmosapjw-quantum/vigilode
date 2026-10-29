//! Short selection experiment: actual independent nonlinear ODE solves.
//! Calibration and holdout measurements are distinct; no timing assertions.
use rodas5p_core::CoreResult;
use rodas5p_integrators::ParallelExecution;
use serde_json::json;
use std::{hint::black_box, time::Instant};
use vigilode::execution_policy::*;
use vigilode::{Atol, Dopri5, Options, Problem};

fn logistic(rate: f64) -> CoreResult<f64> {
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
    let s = Dopri5::solve(&p, (0.0, 4.0), &[0.1], &o).unwrap();
    assert!(s.success);
    let result = s.final_state[0];
    let exact = 1.0 / (1.0 + 9.0 * (-rate * 4.0).exp());
    assert!((result - exact).abs() < 1e-7);
    Ok(result)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workers = std::env::args()
        .nth(1)
        .unwrap_or("2".into())
        .parse::<usize>()?;
    let repetitions = 20;
    for batch in [1, 64, 256] {
        let items: Vec<f64> = (0..batch)
            .map(|i| 0.5 + 1.5 * i as f64 / batch as f64)
            .collect();
        let key = WorkloadKey {
            model_epoch: 1,
            method_tag: 5,
            dimension: 1,
            batch_len: batch,
            layer: ExecutionLayer::Ensemble,
            environment_epoch: 1,
        };
        let mut adaptive = AdaptiveBatchExecutor::new(workers, Default::default())?;
        let calibration =
            adaptive.calibrate_replay_safe(key, &items, |r| logistic(*r), |a, b| a == b)?;
        let serial = ParallelExecution::sequential();
        let parallel = ParallelExecution::rayon(workers)?;
        let mut rows = Vec::new();
        for repeat in 0..repetitions {
            // Cyclic order controls simple order drift; held-out calls follow calibration.
            for arm_index in 0..3 {
                let arm = (arm_index + repeat) % 3;
                let start = Instant::now();
                let (out, decision) = match arm {
                    0 => (serial.map_ordered(&items, |r| logistic(*r))?, None),
                    1 => (parallel.map_ordered(&items, |r| logistic(*r))?, None),
                    _ => {
                        let (x, d) = adaptive.map_ordered(
                            key,
                            repetitions - repeat,
                            false,
                            &items,
                            |r| logistic(*r),
                        )?;
                        (x, Some(d))
                    }
                };
                let elapsed = start.elapsed().as_secs_f64();
                black_box(&out);
                rows.push(json!({"repeat":repeat,"arm":(["serial","parallel","adaptive"][arm]),
                    "seconds":elapsed,"decision":decision,"output_checksum":out.iter().sum::<f64>()}));
            }
        }
        println!(
            "{}",
            json!({"workload":key,"workers":workers,"repetitions":repetitions,
            "calibration_seconds":calibration,"startup_seconds":adaptive.startup_seconds,
            "warmup_seconds":adaptive.warmup_seconds,"calibration_samples":adaptive.calibration_samples,
            "telemetry":adaptive.selector.telemetry(),"rows":rows,
            "accuracy":"Every member logistic analytic endpoint abs error < 1e-7; paired output bit identity",
            "scope":"Opt-in ensemble dispatch of actual Dopri5 solves; one host, no general timing claim"})
        );
    }
    Ok(())
}
