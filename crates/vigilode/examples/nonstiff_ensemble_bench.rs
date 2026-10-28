//! One complete independent-trajectory ensemble measurement per process.
//! The campaign runner supplies warmups, repetitions and execution ordering.

use std::time::Instant;

use rodas5p_core::WorkCounters;
use vigilode::{Atol, Dopri5Ensemble, Dopri5EnsembleJob, Options, Problem};

fn arg(args: &[String], name: &str, default: &str) -> String {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .unwrap_or_else(|| default.into())
}

fn initial_value(component: usize) -> f64 {
    0.5 + 0.5 * (component % 17) as f64 / 16.0
}

fn rate(member: usize, component: usize) -> f64 {
    0.5 + 0.35 * (component % 13) as f64 / 12.0 + member as f64 * 0.01
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let workers: usize = arg(&args, "--threads", "1").parse().unwrap();
    let n: usize = arg(&args, "--dimension", "4096").parse().unwrap();
    let members: usize = arg(&args, "--members", "16").parse().unwrap();
    let rtol: f64 = arg(&args, "--rtol", "1e-6").parse().unwrap();
    let source = arg(&args, "--source-commit", "UNRECORDED");
    assert!(n > 0 && members > 0);
    let end = 4.0;
    // A reused executor has no per-batch pool construction cost. Record that
    // separately so users can reconstruct first-use cost as well.
    let pool_start = Instant::now();
    let pool = Dopri5Ensemble::new(workers).unwrap();
    let pool_setup_seconds = pool_start.elapsed().as_secs_f64();
    let start = Instant::now();
    let problems: Vec<_> = (0..members)
        .map(|member| {
            Problem::new(n, move |_t, y, out| {
                for (i, (value, derivative)) in y.iter().zip(out.iter_mut()).enumerate() {
                    *derivative = -rate(member, i) * value;
                }
                Ok(())
            })
            .autonomous()
        })
        .collect();
    let initial: Vec<Vec<_>> = (0..members)
        .map(|_| (0..n).map(initial_value).collect())
        .collect();
    let options = Options {
        rtol,
        atol: Atol::Scalar(rtol * 1e-3),
        h0: 1e-3,
        ..Options::default()
    };
    let jobs: Vec<_> = (0..members)
        .map(|member| Dopri5EnsembleJob {
            problem: &problems[member],
            t_span: (0.0, end),
            y0: &initial[member],
            options: &options,
        })
        .collect();
    let results = pool.solve(&jobs).unwrap();
    let elapsed_seconds = start.elapsed().as_secs_f64();
    let mut counters = WorkCounters::default();
    let mut attempts = 0;
    let mut accepted_steps = 0;
    let mut rejected_steps = 0;
    let mut fsal_reuses = 0;
    let mut rhs_completed = 0;
    let mut success = true;
    let mut max_abs_error = 0.0_f64;
    let mut max_normalized_error = 0.0_f64;
    let mut member_records = Vec::new();
    let mut states = Vec::new();
    let mut reference = Vec::new();
    let mut times = Vec::new();
    for (member, result) in results.into_iter().enumerate() {
        match result {
            Ok(solution) => {
                let shape_ok = solution.t.len() == solution.y.len()
                    && solution
                        .y
                        .iter()
                        .all(|y| y.len() == n && y.iter().all(|v| v.is_finite()));
                let member_success = solution.success
                    && shape_ok
                    && solution.final_time == end
                    && solution.t.last() == Some(&end);
                success &= member_success;
                let exact: Vec<Vec<_>> = solution
                    .t
                    .iter()
                    .map(|&t| {
                        (0..n)
                            .map(|i| initial_value(i) * (-rate(member, i) * t).exp())
                            .collect()
                    })
                    .collect();
                if shape_ok {
                    for (state, exact_state) in solution.y.iter().zip(&exact) {
                        for (i, (&value, &reference_value)) in
                            state.iter().zip(exact_state).enumerate()
                        {
                            let error = (value - reference_value).abs();
                            max_abs_error = max_abs_error.max(error);
                            max_normalized_error = max_normalized_error.max(
                                error
                                    / 1.0_f64
                                        .max(initial_value(i).abs())
                                        .max(reference_value.abs()),
                            );
                        }
                    }
                }
                counters.accumulate(solution.stats.counters);
                attempts += solution.stats.attempts;
                accepted_steps += solution.stats.accepted_steps;
                rejected_steps += solution.stats.rejected_steps;
                fsal_reuses += solution.stats.fsal_reuses;
                rhs_completed += solution.stats.rhs_completed;
                member_records.push(serde_json::json!({"index":member,"success":member_success,
                    "message":solution.message,"work":solution.stats,"final_time":solution.final_time,
                    "final_state":if member_success {None}else{Some(solution.final_state)}}));
                times.push(solution.t);
                states.push(solution.y);
                reference.push(exact);
            }
            Err(error) => {
                success = false;
                member_records.push(
                    serde_json::json!({"index":member,"success":false,"message":error.to_string()}),
                );
                times.push(Vec::new());
                states.push(Vec::new());
                reference.push(Vec::new());
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "schema":"vigilode.nonstiff-benchmark-run.v1","source_commit":source,
            "case":"ensemble-decay","dimension":n,"members":members,"threads":workers,
            "mode":"dopri5-ensemble","rtol":rtol,"atol":rtol*1e-3,"end":end,
            "success":success,"elapsed_seconds":elapsed_seconds,"pool_setup_seconds":pool_setup_seconds,
            "max_abs_error":if success {Some(max_abs_error)}else{None},
            "max_normalized_error":if success {Some(max_normalized_error)}else{None},
            "requested_times":[0.0,end],"times":times,"states":states,"reference":reference,
            "work":{"attempts":attempts,"accepted_steps":accepted_steps,"rejected_steps":rejected_steps,
                "fsal_reuses":fsal_reuses,"rhs_completed":rhs_completed,"counters":counters},
            "member_records":member_records,
            "error_normalization":"max_member,i,t |e|/max(1,|y_initial|,|y_exact|); sampled endpoints",
            "timing_scope":"problem/initial/workspace construction and all independent integrations; excludes local pool setup, reference and JSON"
        })
    );
}
