//! U11 pre-registered holdout runner (see ../PREREGISTRATION.json). Prints one JSON line per
//! (problem, tolerance, arm) with the final state, work and paired-interleaved wall times.

use std::time::Instant;

use serde_json::json;
use vigilode::{Atol, CapabilitySet, Options, PeerOde, PeerSolveOptions, Rodas5p};

mod models;
use models::{Spec, problem, specs};

fn run_arm(s: &Spec, arm: &str, tol: f64) -> (serde_json::Value, f64) {
    let p = problem(s);
    let clock = Instant::now();
    let v = match arm {
        "rodas5p" => {
            let opts = Options {
                rtol: tol,
                atol: Atol::Scalar(tol * 1e-2),
                ..Options::default()
            };
            match Rodas5p::solve(&p, (0.0, s.t_end), &s.y0, &opts) {
                Ok(sol) => json!({"success": sol.is_success(), "y_end": sol.final_state(),
                                  "accepted": sol.stats.accepted_steps, "rejected": sol.stats.rejected_steps,
                                  "counters": sol.stats.counters, "callbacks": sol.stats.callbacks}),
                Err(e) => json!({"success": false, "error": e.to_string()}),
            }
        }
        _ => {
            let threads = if arm == "ppsw1_t6" { 6 } else { 1 };
            let allowed = CapabilitySet {
                explicit_jacobian: true,
                dense_matrix: true,
                newton: true,
                ..CapabilitySet::NONE
            };
            let o = PeerSolveOptions {
                rtol: tol,
                atol: tol * 1e-2,
                h0_rule_h_max: s.t_end / 100.0,
                threads,
                ..PeerSolveOptions::default()
            };
            match PeerOde::solve(&p, (0.0, s.t_end), &s.y0, &o, allowed) {
                Ok(sol) => json!({"success": sol.success, "failure": sol.failure,
                                  "y_end": sol.y.last(), "h0": sol.h0,
                                  "accepted": sol.step_work.accepted_steps,
                                  "rejected": sol.step_work.rejected_steps,
                                  "startup": sol.startup_work, "steps": sol.step_work}),
                Err(e) => json!({"success": false, "error": e.to_string()}),
            }
        }
    };
    (v, clock.elapsed().as_secs_f64())
}

fn main() {
    let arms = ["rodas5p", "ppsw1_t1", "ppsw1_t6"];
    for s in specs() {
        for tol in [1e-4, 1e-5, 1e-6, 1e-7, 1e-8] {
            let mut times: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
            let mut last = vec![serde_json::Value::Null; arms.len()];
            for rep in 0..9 {
                for k in 0..arms.len() {
                    let a = (k + rep) % arms.len();
                    let (v, secs) = run_arm(&s, arms[a], tol);
                    if rep >= 2 {
                        times[a].push(secs);
                    }
                    last[a] = v;
                }
            }
            for (a, arm) in arms.iter().enumerate() {
                let mut t = times[a].clone();
                t.sort_by(f64::total_cmp);
                println!(
                    "{}",
                    json!({"problem": s.name, "tol": tol, "arm": arm, "result": last[a],
                           "wall_median_s": t[t.len() / 2], "wall_all_s": times[a],
                           "warmups": 2, "repetitions": t.len()})
                );
            }
            eprintln!("{} tol {tol} done", s.name);
        }
    }
}
