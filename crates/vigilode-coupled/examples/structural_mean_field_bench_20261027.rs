//! Same-binary complete trajectory comparison with common original-space gate.
use rodas5p_integrators::ParallelExecution;
use serde_json::json;
use std::time::Instant;
use vigilode_coupled::structural_mean_field::{MeanFieldAffine, StructuralWork};

fn setup(n: usize) -> (MeanFieldAffine, Vec<f64>, Vec<f64>) {
    let y: Vec<_> = (0..n).map(|i| 0.5 + 0.1 * (i as f64 * 0.7).sin()).collect();
    let b: Vec<_> = (0..n)
        .map(|i| 0.2 + 0.05 * (i as f64 * 1.9).cos())
        .collect();
    (MeanFieldAffine::new(20.0, 19.0, b.clone()).unwrap(), y, b)
}
fn exact(y: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    let my = y.iter().sum::<f64>() / y.len() as f64;
    let mb = b.iter().sum::<f64>() / b.len() as f64;
    let collective = (-t).exp() * my - (-t).exp_m1() * mb;
    y.iter()
        .zip(b)
        .map(|(a, b)| {
            collective + (-20.0 * t).exp() * (a - my) - (-20.0 * t).exp_m1() / 20.0 * (b - mb)
        })
        .collect()
}
fn trajectory(
    n: usize,
    steps: usize,
    arm: &str,
    e: &ParallelExecution,
    retain: bool,
) -> (Vec<Vec<f64>>, StructuralWork, u64) {
    let (m, mut y, _) = setup(n);
    let h = 1.0 / steps as f64;
    let plan = if arm == "quotient-reused" {
        Some(m.plan(h).unwrap())
    } else {
        None
    };
    let mut setups = if plan.is_some() { 16 } else { 0 };
    let mut states = Vec::new();
    let mut work = StructuralWork::default();
    if retain {
        states.push(y.clone());
    }
    for _ in 0..steps {
        let c = match arm {
            "sequential" => m.sequential_step(&y, h, 1e-9, e).unwrap(),
            "quotient-fresh" => {
                let p = m.plan(h).unwrap();
                setups += 16;
                p.step(&m, &y, 1e-9, e).unwrap()
            }
            "quotient-reused" => plan.as_ref().unwrap().step(&m, &y, 1e-9, e).unwrap(),
            _ => panic!("unknown arm"),
        };
        work.rhs_evaluations += c.work.rhs_evaluations;
        work.physical_linear_actions += c.work.physical_linear_actions;
        work.shifted_resolvent_actions += c.work.shifted_resolvent_actions;
        work.full_state_causal_accumulations += c.work.full_state_causal_accumulations;
        work.lifted_components += c.work.lifted_components;
        work.original_space_residual_components += c.work.original_space_residual_components;
        y = c.y_new;
        if retain {
            states.push(y.clone());
        }
    }
    if !retain {
        states.push(y);
    }
    (states, work, setups)
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let n: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(2048);
    let steps: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(64);
    let blocks: usize = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(5);
    let threads: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(1);
    let e = ParallelExecution::rayon(threads).unwrap();
    let arms = ["sequential", "quotient-fresh", "quotient-reused"];
    let (_, y0, b) = setup(n);
    let base = trajectory(n, steps, arms[0], &e, true).0;
    let mut diagnostics = Vec::new();
    for arm in arms {
        let (states, work, setups) = trajectory(n, steps, arm, &e, true);
        let mut exact_error = 0.0_f64;
        let mut target_error = 0.0_f64;
        for (j, state) in states.iter().enumerate() {
            let reference = exact(&y0, &b, j as f64 / steps as f64);
            for q in 0..n {
                exact_error = exact_error.max((state[q] - reference[q]).abs());
                target_error = target_error.max((state[q] - base[j][q]).abs());
            }
        }
        assert!(
            exact_error <= 1e-6,
            "frozen common-grid exact error cap {exact_error}"
        );
        assert!(
            target_error <= 1e-11,
            "same-target tolerance {target_error}"
        );
        diagnostics.push(json!({"arm":arm,"maximum_common_grid_error":exact_error,"maximum_baseline_difference":target_error,"work":work,"scalar_setup_solves":setups}));
    }
    let mut timing = Vec::new();
    for candidate in &arms[1..] {
        let now = Instant::now();
        std::hint::black_box(trajectory(n, steps, "sequential", &e, false));
        let loops = (0.1 / now.elapsed().as_secs_f64())
            .ceil()
            .clamp(1.0, 1000.0) as usize;
        let mut samples = Vec::new();
        for block in 0..blocks {
            let order = if block % 2 == 0 {
                ["sequential", candidate, candidate, "sequential"]
            } else {
                [candidate, "sequential", "sequential", candidate]
            };
            for (position, arm) in order.iter().enumerate() {
                let start = Instant::now();
                for _ in 0..loops {
                    std::hint::black_box(trajectory(n, steps, arm, &e, false));
                }
                samples.push(json!({"block":block,"position":position,"arm":arm,"seconds_per_trajectory":start.elapsed().as_secs_f64()/loops as f64}));
            }
        }
        timing.push(json!({"candidate":candidate,"loops_per_batch":loops,"samples":samples}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"dimension":n,"steps":steps,"threads":threads,"d":20.0,"kappa":19.0,"h":1.0/steps as f64,"accuracy_cap":1e-6,"root_residual_linf_cap":1e-9,"timing_scope":"complete trajectory including model allocation, plan construction, eight-stage full-space screen and endpoint; pool and retained observations outside timing","diagnostics":diagnostics,"timing":timing})).unwrap());
}
