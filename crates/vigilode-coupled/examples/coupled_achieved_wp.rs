//! Source-locked achieved-error work–precision experiment for VigilODE PR #156.
//!
//! Observes the original public coupled_inverse_be::Source::run without changing
//! the library's defaults or success rules. Every reported state is from native Rust.
//! Achieved error and rigorous source residual are checked separately in Python.
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;
use vigilode_coupled::coupled_inverse_be::{Config, Order, Source};

fn cases() -> Vec<(&'static str, Source, usize, f64, f64)> {
    vec![
        (
            "asymmetric_2",
            Source::new(
                vec![4.0, 7.0],
                vec![20.0, 40.0],
                vec![vec![0.0, 12.0], vec![8.0, 0.0]],
                vec![0.2, 0.12],
            )
            .expect("source A"),
            64,
            0.1,
            0.00890,
        ),
        (
            "asymmetric_2_refined",
            Source::new(
                vec![4.0, 7.0],
                vec![20.0, 40.0],
                vec![vec![0.0, 12.0], vec![8.0, 0.0]],
                vec![0.2, 0.12],
            )
            .expect("source A refined"),
            128,
            0.1,
            0.00445,
        ),
        (
            "coupled_3_cubic",
            Source::new(
                vec![6.0, 8.0, 10.0],
                vec![60.0, 100.0, 30.0],
                vec![
                    vec![0.0, 30.0, 4.0],
                    vec![15.0, 0.0, 10.0],
                    vec![10.0, 8.0, 0.0],
                ],
                vec![0.04, 0.06, 0.02],
            )
            .expect("source B"),
            128,
            0.1,
            0.00712,
        ),
        (
            "linear_exchange_control",
            Source::new(
                vec![4.0, 4.0],
                vec![0.0, 0.0],
                vec![vec![0.0, 6.0], vec![6.0, 0.0]],
                vec![0.1, 0.05],
            )
            .expect("linear negative control"),
            128,
            0.1,
            0.006,
        ),
    ]
}

fn config(steps: usize, horizon: f64, eps: f64, fast: bool) -> Config {
    Config {
        steps,
        horizon,
        physical_tolerance: eps,
        max_map_evaluations: 200_000,
        max_iterations_per_step: 90,
        order: if fast {
            Order::FastFirst
        } else {
            Order::BaselineFirst
        },
    }
}

fn timed(source: &Source, cfg: Config, repetitions: u64) -> (u128, u64, u64, u64, u64) {
    let mut map = 0_u64;
    let mut original = 0_u64;
    let mut inverse = 0_u64;
    let mut checksum = 0_u64;
    let start = Instant::now();
    for _ in 0..repetitions {
        let run = black_box(source.run(black_box(cfg)));
        assert!(
            run.completed,
            "timed source must complete: {:?}",
            run.failure
        );
        map += run.work.map_attempted;
        original += run.work.original_interval_components;
        inverse += run.work.inverse_interval_components;
        for x in &run.points.last().expect("nonempty trajectory").state {
            checksum = checksum.wrapping_add(x.to_bits());
        }
        black_box(&run);
    }
    (start.elapsed().as_nanos(), map, original, inverse, checksum)
}

fn main() {
    const ALPHAS: [f64; 5] = [0.0625, 0.25, 1.0, 4.0, 16.0];
    const REPS: u64 = 8;
    for (name, source, steps, horizon, original_eps) in cases() {
        let original_cfg = config(steps, horizon, original_eps, false);
        let legacy = source.run(original_cfg);
        assert!(
            legacy.completed,
            "parent alpha=1 must complete: {:?}",
            legacy.failure
        );
        let trunc = legacy
            .truncation_majorant
            .expect("source truncation majorant");
        assert!(trunc > 0.0 && trunc < original_eps);
        let slack = original_eps - trunc;
        println!(
            "VWP_COHORT={}",
            json!({
                "name":name,
                "source_bits":source.source_bits(),
                "steps":steps,
                "horizon_bits":horizon.to_bits(),
                "parent_eps_bits":original_eps.to_bits(),
                "truncation_bits":trunc.to_bits(),
                "alpha_bits":ALPHAS.map(f64::to_bits),
            })
        );
        for (alpha_id, &alpha) in ALPHAS.iter().enumerate() {
            let eps = if alpha == 1.0 {
                original_eps
            } else {
                trunc + alpha * slack
            };
            assert!(eps.is_finite() && eps > trunc);
            let mut completed = [false; 2];
            for fast in [false, true] {
                let cfg = config(steps, horizon, eps, fast);
                let result = source.run(cfg);
                let arm = usize::from(fast);
                completed[arm] = result.completed;
                if !result.completed {
                    println!(
                        "VWP_REJECTED={}",
                        json!({
                            "name":name,
                            "alpha_id":alpha_id,
                            "fast":fast,
                            "eps_bits":eps.to_bits(),
                            "failure":format!("{:?}", result.failure),
                            "attempted":result.work.map_attempted,
                        })
                    );
                    continue;
                }
                assert!(
                    result.points.iter().all(|p| p.physical_error_upper <= eps),
                    "original whole-IVP certificate must not exceed caller epsilon"
                );
                assert_eq!(result.truncation_majorant.expect("trunc"), trunc);
                println!(
                    "VWP_TRACE={}",
                    json!({
                        "name":name,
                        "alpha_id":alpha_id,
                        "alpha_bits":alpha.to_bits(),
                        "fast":fast,
                        "source_bits":source.source_bits(),
                        "n":source.dimension(),
                        "h_bits":(horizon / steps as f64).to_bits(),
                        "eps_bits":eps.to_bits(),
                        "parent_eps_bits":original_eps.to_bits(),
                        "truncation_majorant":trunc.to_bits(),
                        "stage_original_residual_target":result.original_stage_residual_target.expect("target").to_bits(),
                        "map_attempted":result.work.map_attempted,
                        "map_completed":result.work.map_completed,
                        "original_components":result.work.original_interval_components,
                        "inverse_components":result.work.inverse_interval_components,
                        "algebraic_candidates":result.work.algebraic_cubic_candidates,
                        "points":result.points.iter().map(|p| json!({
                            "time":p.time.to_bits(),
                            "state":p.state.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                            "rho":p.original_be_residual_upper.to_bits(),
                            "q":p.source_q_upper.to_bits(),
                            "E":p.physical_error_upper.to_bits(),
                            "callbacks":p.map_evaluations,
                        })).collect::<Vec<_>>(),
                    })
                );
            }
            // A benchmark is only meaningful if both arms completed this
            // original unaltered physical-tolerance contract.
            if !completed.iter().all(|&v| v) {
                println!("VWP_UNPAIRED name={name} alpha_id={alpha_id}");
                continue;
            }
            for fast in [false, true] {
                black_box(timed(&source, config(steps, horizon, eps, fast), 2));
            }
            for block in 0..5 {
                let order = if block % 2 == 0 {
                    [false, true, true, false]
                } else {
                    [true, false, false, true]
                };
                for fast in order {
                    let (ns, calls, original, inverse, checksum) =
                        timed(&source, config(steps, horizon, eps, fast), REPS);
                    println!(
                        "VWP_BENCH name={name} alpha_id={alpha_id} block={block} fast={fast} ns={ns} reps={REPS} map={calls} original={original} inverse={inverse} checksum={checksum}"
                    );
                }
            }
        }
    }
}
