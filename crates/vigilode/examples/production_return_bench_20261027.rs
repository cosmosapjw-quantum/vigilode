//! Exact-work ordinary/sentinel return comparison on nonlinear trajectories.
#[path = "support/production_return_cases.rs"]
mod cases;
use serde_json::json;
use std::{hint::black_box, time::Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let selected_case = args.get(1).map(String::as_str);
    let selected_policy = args.get(2).map(String::as_str);
    let repeats = args
        .get(3)
        .map_or(1, |s| s.parse::<usize>().expect("integer repeats"));
    assert!(repeats > 0);
    let mut rows = Vec::new();
    let mut all_pass = true;
    for case in cases::CASES {
        if selected_case.is_some_and(|s| s != "all" && s != case) {
            continue;
        }
        for policy in cases::POLICIES {
            if selected_policy.is_some_and(|s| {
                s != policy && !(s == "observed" && policy.starts_with("observed"))
            }) {
                continue;
            }
            let config = cases::config(policy);
            let clock = Instant::now();
            let mut last = None;
            for _ in 0..repeats {
                last = Some(black_box(cases::solve(case, &config, None)));
            }
            let elapsed = clock.elapsed().as_secs_f64();
            let (r, rhs, jvp) = last.unwrap();
            let error = cases::max_error(case, &r);
            let accounting = r.stats.accounting_complete
                && rhs == r.stats.rhs_calls
                && jvp == r.stats.jvp_vectors
                && r.stats.jacobian_builds == 0
                && r.stats.ft_calls == 0;
            let pass = r.success
                && r.final_time == 1.0
                && error <= cases::accuracy_cap(case)
                && accounting
                && (policy != "rhs-only" || jvp == 0);
            all_pass &= pass;
            rows.push(
                json!({"case":case,"policy":policy,"pass":pass,"success":r.success,
                "message":r.message,"max_error":error,"accuracy_cap":cases::accuracy_cap(case),
                "work":cases::work(&r),"rhs":rhs,"jvp":jvp,"attempts":r.stats.attempts,
                "switches":r.stats.switches,"accounting":accounting,"seconds":elapsed,
                "repeats":repeats,"config":config,"segments":r.stats.segments,
                "times":r.t,"states":r.y}),
            );
        }
    }
    assert!(!rows.is_empty(), "unknown case or policy");
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema_version":1,
        "dimension":cases::DIMENSION,"span":[0,1],"options":cases::options(),
        "all_pass":all_pass,"rows":rows}))
        .unwrap()
    );
    if !all_pass {
        std::process::exit(1);
    }
}
