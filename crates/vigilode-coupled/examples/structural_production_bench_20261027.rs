//! Same-binary PR91 / fused-direct / factorized-enclosure trajectory benchmark.
use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::ParallelExecution;
use serde_json::{Value, json};
use std::time::Instant;
use vigilode_coupled::structural_mean_field::MeanFieldAffine;

#[allow(dead_code)]
#[path = "support/structural_mean_field_pr91.rs"]
mod pr91;

fn fixture(n: usize) -> (Vec<f64>, Vec<f64>) {
    (
        (0..n).map(|i| 0.5 + 0.1 * (0.7 * i as f64).sin()).collect(),
        (0..n)
            .map(|i| 0.2 + 0.05 * (1.9 * i as f64).cos())
            .collect(),
    )
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
) -> (Vec<Vec<f64>>, Value) {
    let (mut y, b) = fixture(n);
    let h = 1.0 / steps as f64;
    let mut states = if retain { vec![y.clone()] } else { Vec::new() };
    let mut work = serde_json::Map::new();
    let mut largest_screen = 0.0_f64;
    let mut consume = |next: Vec<f64>, values: Value, screen: f64| {
        for (name, value) in values.as_object().unwrap() {
            let total =
                work.get(name).and_then(Value::as_u64).unwrap_or(0) + value.as_u64().unwrap();
            work.insert(name.clone(), json!(total));
        }
        y = next;
        largest_screen = largest_screen.max(screen);
        if retain {
            states.push(y.clone());
        }
    };
    // The closure receives state by value; borrow the last state separately in
    // each branch so diagnostics and the hot path use the identical solver.
    match arm {
        "pr91" => {
            let m = pr91::MeanFieldAffine::new(20.0, 19.0, b).unwrap();
            let p = m.plan(h).unwrap();
            let mut current = fixture(n).0;
            for _ in 0..steps {
                let c = p.step(&m, &current, 1e-9, e).unwrap();
                current = c.y_new.clone();
                consume(
                    c.y_new,
                    serde_json::to_value(c.work).unwrap(),
                    c.full_residual_linf,
                );
            }
        }
        "fused" => {
            let m = MeanFieldAffine::new(20.0, 19.0, b).unwrap();
            let p = m.plan(h).unwrap();
            let mut current = fixture(n).0;
            for _ in 0..steps {
                let c = p.step_charged(&m, &current, 1e-9, e).unwrap();
                current = c.y_new.clone();
                consume(
                    c.y_new,
                    serde_json::to_value(c.work).unwrap(),
                    c.full_residual_linf,
                );
            }
        }
        "enclosed" => {
            let m = MeanFieldAffine::new(20.0, 19.0, b).unwrap();
            let p = m.enclosure_plan(h).unwrap();
            let mut current = fixture(n).0;
            for _ in 0..steps {
                let c = p.step(&m, &current, 1e-9, e).unwrap();
                current = c.y_new.clone();
                consume(
                    c.y_new,
                    serde_json::to_value(c.work).unwrap(),
                    c.full_residual_upper_bound,
                );
            }
        }
        _ => panic!("unknown arm {arm}"),
    }
    if !retain {
        states.push(y);
    }
    (
        states,
        json!({"work":work,"maximum_screen_metric":largest_screen,"screen_kind":if arm=="enclosed" {"outward original-space upper bound"} else {"operational original-space f64 residual"}}),
    )
}
fn hot_trajectory(n: usize, steps: usize, arm: &str, e: &ParallelExecution) -> Vec<f64> {
    let (mut y, b) = fixture(n);
    let h = 1.0 / steps as f64;
    match arm {
        "pr91" => {
            let m = pr91::MeanFieldAffine::new(20.0, 19.0, b).unwrap();
            let p = m.plan(h).unwrap();
            for _ in 0..steps {
                y = p.step(&m, &y, 1e-9, e).unwrap().y_new;
            }
        }
        "fused" => {
            let m = MeanFieldAffine::new(20.0, 19.0, b).unwrap();
            let p = m.plan(h).unwrap();
            for _ in 0..steps {
                y = p.step_charged(&m, &y, 1e-9, e).unwrap().y_new;
            }
        }
        "enclosed" => {
            let m = MeanFieldAffine::new(20.0, 19.0, b).unwrap();
            let p = m.enclosure_plan(h).unwrap();
            for _ in 0..steps {
                y = p.step(&m, &y, 1e-9, e).unwrap().y_new;
            }
        }
        _ => panic!("unknown arm {arm}"),
    }
    y
}
fn oracle_fixtures() {
    let c = rodas5p_coefficients().unwrap();
    let mut cases = Vec::new();
    for n in [1, 3, 9] {
        for (d, k, h, scale) in [
            (0.0, 0.0, 0.125, 1.0),
            (2.0, 2.0, 0.125, 1.0),
            (20.0, 19.0, 1.0 / 64.0, 1.0),
            (2.0, 0.5, 0.125, 1e-300),
            (2.0, 0.3, 0.1, 1.0),
            (2.0, 0.5, 0.125, 1e100),
            (2.0, 0.5, 0.125, 1e-320),
        ] {
            let (mut y, mut b) = fixture(n);
            for (i, v) in y.iter_mut().enumerate() {
                *v *= if i % 2 == 0 { scale } else { -scale };
            }
            for (i, v) in b.iter_mut().enumerate() {
                *v *= if i % 3 == 0 { -scale } else { scale };
            }
            let m = MeanFieldAffine::new(d, k, b.clone()).unwrap();
            let p = m.enclosure_plan(h).unwrap();
            let result = p
                .step(&m, &y, 1e120, &ParallelExecution::sequential())
                .unwrap();
            cases.push(json!({"n":n,"d":d,"kappa":k,"h":h,"y":y,"forcing":b,"stages":result.stages_u,"bounds":result.stage_residual_upper_bounds,"work":result.work}));
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"gamma":c.gamma,"a":(0..8).map(|i|(0..8).map(|j|c.a[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),"c":(0..8).map(|i|(0..8).map(|j|c.c_matrix[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),"cases":cases})).unwrap());
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    let mode = a.get(1).map(String::as_str).unwrap_or("bench");
    if mode == "oracle" {
        oracle_fixtures();
        return;
    }
    let n: usize = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(2048);
    let steps: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(64);
    if mode == "profile" {
        let arm = &a[4];
        let iterations: usize = a[5].parse().unwrap();
        let threads: usize = a.get(6).map(|s| s.parse().unwrap()).unwrap_or(1);
        let e = ParallelExecution::rayon(threads).unwrap();
        let _ = rodas5p_coefficients().unwrap();
        let mut last = Vec::new();
        for _ in 0..iterations {
            last = std::hint::black_box(hot_trajectory(n, steps, arm, &e));
        }
        println!(
            "{}",
            json!({"mode":mode,"arm":arm,"dimension":n,"steps":steps,"iterations":iterations,"threads":threads,"last":last.first()})
        );
        return;
    }
    let blocks: usize = a.get(4).map(|s| s.parse().unwrap()).unwrap_or(5);
    let threads: usize = a.get(5).map(|s| s.parse().unwrap()).unwrap_or(1);
    let e = ParallelExecution::rayon(threads).unwrap();
    let (y0, b) = fixture(n);
    let base = trajectory(n, steps, "pr91", &e, true).0;
    let mut diagnostics = Vec::new();
    for arm in ["pr91", "fused", "enclosed"] {
        let (states, mut detail) = trajectory(n, steps, arm, &e, true);
        let mut error = 0.0_f64;
        let mut difference = 0.0_f64;
        let mut bits_equal = true;
        for (j, row) in states.iter().enumerate() {
            let r = exact(&y0, &b, j as f64 / steps as f64);
            for q in 0..n {
                error = error.max((row[q] - r[q]).abs());
                difference = difference.max((row[q] - base[j][q]).abs());
                bits_equal &= row[q].to_bits() == base[j][q].to_bits();
            }
        }
        assert!(error <= 1e-6, "physical cap {error}");
        assert!(difference <= 1e-11, "same-target cap {difference}");
        detail["arm"] = json!(arm);
        detail["maximum_common_grid_error"] = json!(error);
        detail["maximum_pr91_difference"] = json!(difference);
        detail["all_grid_bits_equal"] = json!(bits_equal);
        diagnostics.push(detail);
    }
    let mut timing = Vec::new();
    for candidate in ["fused", "enclosed"] {
        let start = Instant::now();
        std::hint::black_box(hot_trajectory(n, steps, "pr91", &e));
        let loops = (0.1 / start.elapsed().as_secs_f64())
            .ceil()
            .clamp(1.0, 1000.0) as usize;
        let mut samples = Vec::new();
        for block in 0..blocks {
            let order = if block % 2 == 0 {
                ["pr91", candidate, candidate, "pr91"]
            } else {
                [candidate, "pr91", "pr91", candidate]
            };
            for (position, arm) in order.into_iter().enumerate() {
                let start = Instant::now();
                for _ in 0..loops {
                    std::hint::black_box(hot_trajectory(n, steps, arm, &e));
                }
                samples.push(json!({"block":block,"position":position,"arm":arm,"seconds_per_trajectory":start.elapsed().as_secs_f64()/loops as f64}));
            }
        }
        timing.push(json!({"candidate":candidate,"loops_per_batch":loops,"samples":samples}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"dimension":n,"steps":steps,"threads":threads,"blocks":blocks,"accuracy_cap":1e-6,"same_target_cap":1e-11,"residual_cap":1e-9,"timing_scope":"complete trajectory, model+plan+all stages+full original-space verification+endpoint+allocations; pool, diagnostic serialization and observations outside timing","diagnostics":diagnostics,"timing":timing})).unwrap());
}
