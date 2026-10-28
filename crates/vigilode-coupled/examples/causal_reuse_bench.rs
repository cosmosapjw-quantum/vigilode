//! Same-binary work, physical accuracy, and bounded timing of causal reuse.
use rodas5p_core::{LinearMethod, LinearSolverConfig, PreconditionerKind, rodas5p_coefficients};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, ParallelExecution};
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
use vigilode_coupled::{
    Attempt, CausalSchedule, Config, RootErrorPolicy, coupled_step_causal,
    integrate_causal_observed,
};

type ReuseSnapshot = (Vec<Vec<f64>>, Vec<(u64, u64, bool)>);

fn model(n: usize, eps: f64) -> OdeProblem {
    OdeProblem::new(
        "causal-reuse-riccati",
        n,
        Arc::new(move |_, y, f| {
            for i in 0..n {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i] + eps * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, f| {
            for i in 0..n {
                f[i] = ([-1.0, -20.0, -100.0][i % 3] + 2.0 * eps * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
fn run(p: &OdeProblem, y: &[f64], h: f64, e: &ParallelExecution, s: CausalSchedule) -> Attempt {
    coupled_step_causal(p, 0.0, y, h, &Config::default(), e, s)
}
fn bits_equal(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
}
fn target_oracle(n: usize, eps: f64, h: f64) -> Vec<f64> {
    let c = rodas5p_coefficients().unwrap();
    (0..n)
        .map(|k| {
            let a = [-1.0, -20.0, -100.0][k % 3];
            let mut u = [0.0; 8];
            for i in 0..8 {
                let mut state = 0.1;
                let mut mix = 0.0;
                for (j, &previous_stage) in u.iter().take(i).enumerate() {
                    state += c.a[(i, j)] * previous_stage;
                    mix += c.c_matrix[(i, j)] * previous_stage;
                }
                u[i] = (h * c.gamma * (a * state + eps * state * state) + c.gamma * mix)
                    / (1.0 - h * c.gamma * (a + 0.2 * eps));
            }
            0.1 + (0..8).map(|i| c.b_code[i] * u[i]).sum::<f64>()
        })
        .collect()
}
fn grid() -> Value {
    let mut rows = Vec::new();
    for n in [6, 96, 6144] {
        for eps in [0.0, 1.0, 10.0] {
            for h in [0.01, 0.1] {
                for threads in [1, 4] {
                    let p = model(n, eps);
                    let y = vec![0.1; n];
                    let e = if threads == 1 {
                        ParallelExecution::sequential()
                    } else {
                        ParallelExecution::rayon(threads).unwrap()
                    };
                    let reference = target_oracle(n, eps, h);
                    let old = run(&p, &y, h, &e, CausalSchedule::BatchedPicard);
                    let new = run(&p, &y, h, &e, CausalSchedule::BatchedPicardReuse);
                    let same = old
                        .candidate
                        .as_ref()
                        .zip(new.candidate.as_ref())
                        .map(|(a, b)| {
                            bits_equal(&a.y_new, &b.y_new)
                                && bits_equal(&a.error_vector, &b.error_vector)
                                && a.stages_u
                                    .iter()
                                    .zip(&b.stages_u)
                                    .all(|(a, b)| bits_equal(a, b))
                        });
                    let forward = run(&p, &y, h, &e, CausalSchedule::Forward);
                    for (name, r) in [("legacy", old), ("reuse", new), ("forward", forward)] {
                        let error = r.candidate.as_ref().map(|c| {
                            c.y_new
                                .iter()
                                .zip(&reference)
                                .map(|(a, b)| (a - b).abs())
                                .fold(0.0, f64::max)
                        });
                        rows.push(json!({"dimension":n,"epsilon":eps,"h":h,"threads":threads,"schedule":name,
                "success":r.candidate.is_some(),"failure":r.failure,"same_bits_legacy_reuse":same,
                "target_max_abs_error":error,"target_cap":2e-10,"qualified":error.is_some_and(|e|e<=2e-10),
                "embedded_wrms":r.candidate.as_ref().map(|c|c.embedded_wrms),
                "full_residual_wrms":r.candidate.as_ref().map(|c|c.full_residual_wrms),"work":r.work}));
                    }
                }
            }
        }
    }
    json!(rows)
}
fn trajectories() -> Value {
    let mut rows = Vec::new();
    for n in [6, 96] {
        let p = model(n, 0.5);
        let y = vec![0.1; n];
        let adaptive = AdaptiveStepConfig {
            atol: 1e-11,
            rtol: 1e-7,
            initial_step: 0.05,
            max_step: 0.05,
            ..Default::default()
        };
        let schedule =
            OutputSchedule::new((0..=32).map(|i| 0.2 * i as f64 / 32.0).collect()).unwrap();
        let linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            preconditioner: PreconditionerKind::None,
            atol: 1e-14,
            rtol: 1e-12,
            ..Default::default()
        };
        let mut previous: Option<ReuseSnapshot> = None;
        for (name, mode) in [
            ("legacy", CausalSchedule::BatchedPicard),
            ("reuse", CausalSchedule::BatchedPicardReuse),
        ] {
            let result = integrate_causal_observed(
                &p,
                (0.0, 0.2),
                &y,
                &adaptive,
                &schedule,
                &Config::default(),
                &ParallelExecution::sequential(),
                &linear,
                &RootErrorPolicy::Operational,
                mode,
            )
            .unwrap();
            let max_error = result
                .observed
                .t
                .iter()
                .zip(&result.observed.y)
                .flat_map(|(&t, y)| {
                    y.iter().enumerate().map(move |(i, &yi)| {
                        let a = [-1.0, -20.0, -100.0][i % 3];
                        let e = (a * t).exp();
                        (yi - 0.1 * e / (1.0 + (0.05 / a) * (1.0 - e))).abs()
                    })
                })
                .fold(0.0, f64::max);
            let signature: Vec<_> = result
                .attempts
                .iter()
                .map(|a| (a.t.to_bits(), a.h.to_bits(), a.accepted))
                .collect();
            let same = previous.as_ref().map(|(ys, sig)| {
                ys.len() == result.observed.y.len()
                    && ys
                        .iter()
                        .zip(&result.observed.y)
                        .all(|(a, b)| bits_equal(a, b))
                    && sig == &signature
            });
            let rhs: u64 = result
                .attempts
                .iter()
                .map(|a| a.candidate_work.rhs_attempted + a.fallback_callback_work.rhs_attempted)
                .sum();
            let jvp: u64 = result
                .attempts
                .iter()
                .map(|a| a.candidate_work.jvp_attempted + a.fallback_callback_work.jvp_attempted)
                .sum();
            let solves: u64 = result
                .attempts
                .iter()
                .map(|a| a.candidate_work.small_block_solves)
                .sum();
            let hits: u64 = result
                .attempts
                .iter()
                .map(|a| a.candidate_work.causal_rhs_reused)
                .sum();
            let blocks: u64 = result
                .attempts
                .iter()
                .map(|a| a.candidate_work.causal_block_solves_reused)
                .sum();
            rows.push(json!({"dimension":n,"schedule":name,"success":result.observed.success,"observations":result.observed.t.len(),
                "attempts":result.attempts.len(),"accepted":result.attempts.iter().filter(|a|a.accepted).count(),
                "fallbacks":result.attempts.iter().filter(|a|a.fallback_attempted).count(),"max_physical_error":max_error,"physical_cap":3e-9,
                "same_observed_and_attempt_bits":same,"rhs_attempted":rhs,"jvp_attempted":jvp,"block_solves":solves,
                "rhs_reused":hits,"blocks_reused":blocks,"qualified":result.observed.success&&max_error<=3e-9}));
            previous = Some((result.observed.y, signature));
        }
    }
    json!(rows)
}
fn timings() -> Value {
    let mut rows = Vec::new();
    for n in [96, 6144] {
        for threads in [1, 2, 4] {
            let p = model(n, 10.0);
            let y = vec![0.1; n];
            let e = if threads == 1 {
                ParallelExecution::sequential()
            } else {
                ParallelExecution::rayon(threads).unwrap()
            };
            let a = run(&p, &y, 0.1, &e, CausalSchedule::BatchedPicard);
            let b = run(&p, &y, 0.1, &e, CausalSchedule::BatchedPicardReuse);
            assert!(bits_equal(
                &a.candidate.as_ref().unwrap().y_new,
                &b.candidate.as_ref().unwrap().y_new
            ));
            let mut blocks = Vec::new();
            for block in 0..5 {
                let order = if block % 2 == 0 {
                    [false, true, true, false]
                } else {
                    [true, false, false, true]
                };
                let mut samples = Vec::new();
                let mut old = 0.0;
                let mut new = 0.0;
                for reuse in order {
                    let start = Instant::now();
                    for _ in 0..5 {
                        std::hint::black_box(run(
                            &p,
                            &y,
                            0.1,
                            &e,
                            if reuse {
                                CausalSchedule::BatchedPicardReuse
                            } else {
                                CausalSchedule::BatchedPicard
                            },
                        ));
                    }
                    let sec = start.elapsed().as_secs_f64() / 5.0;
                    if reuse {
                        new += sec;
                    } else {
                        old += sec;
                    }
                    samples.push(json!({"schedule":if reuse {"reuse"}else{"legacy"},"seconds_per_step":sec,"repetitions":5}));
                }
                blocks.push(
                    json!({"block":block,"baseline_over_candidate":old/new,"segments":samples}),
                );
            }
            rows.push(json!({"dimension":n,"epsilon":10.0,"h":0.1,"threads":threads,"blocks":blocks,"legacy_work":a.work,"reuse_work":b.work}));
        }
    }
    json!(rows)
}
fn main() {
    let timing = std::env::args().nth(1).as_deref() == Some("timing");
    let result = if timing {
        json!({"schema":"vigilode.causal-reuse-timing.v1","rows":timings()})
    } else {
        json!({"schema":"vigilode.causal-reuse-evidence.v1","grid":grid(),"trajectories":trajectories()})
    };
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
