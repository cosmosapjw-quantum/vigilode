//! Paired-policy diagnostic. Invoke one case/policy/repetition per process.
//! Timing includes the complete solver wrapper and every discarded pilot.
use serde_json::{Value, json};
use std::time::Instant;
use vigilode::{
    Atol, DerivativeFallback, Dopri5, Jacobian, Options, Problem, Rodas5p, SwitchingConfig,
    SwitchingMethod, SwitchingSolver,
};

fn model(name: &str) -> (Problem, Vec<f64>, f64, Vec<f64>, f64, f64) {
    match name {
        "decay" => (
            Problem::new(1, |_, y, o| {
                o[0] = -y[0];
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, _, j| {
                j[0] = -1.0;
                Ok(())
            }))
            .autonomous(),
            vec![1.0],
            1.0,
            vec![(-1.0_f64).exp()],
            1e-5,
            1e-14,
        ),
        "rotation" => (
            Problem::new(2, |_, y, o| {
                o[0] = -100.0 * y[1];
                o[1] = 100.0 * y[0];
                Ok(())
            })
            .jacobian(Jacobian::jvp(|_, _, v, o| {
                o[0] = -100.0 * v[1];
                o[1] = 100.0 * v[0];
                Ok(())
            }))
            .autonomous(),
            vec![1.0, 0.0],
            1.0,
            vec![100.0_f64.cos(), 100.0_f64.sin()],
            1e-5,
            1e-13,
        ),
        "stiff-forced" => (
            Problem::new(1, |t, y, o| {
                o[0] = -1e4 * (y[0] - t.cos()) - t.sin();
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, _, j| {
                j[0] = -1e4;
                Ok(())
            }))
            .time_derivative(|t, _, o| {
                o[0] = -1e4 * t.sin() - t.cos();
                Ok(())
            }),
            vec![1.0],
            1.0,
            vec![1.0_f64.cos()],
            1e-5,
            1e-14,
        ),
        "relaxation" => (
            Problem::new(1, |t, y, o| {
                let l = 1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh());
                o[0] = -l * (y[0] - t.cos()) - t.sin();
                Ok(())
            })
            .jacobian(Jacobian::dense(|t, _, j| {
                j[0] = -(1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh()));
                Ok(())
            }))
            .time_derivative(|t, y, o| {
                let q = (40.0 * (t - 0.4)).tanh();
                let l = 1.0 + 4999.5 * (1.0 - q);
                let lp = -199980.0 * (1.0 - q * q);
                o[0] = -lp * (y[0] - t.cos()) - l * t.sin() - t.cos();
                Ok(())
            }),
            vec![1.0],
            1.0,
            vec![1.0_f64.cos()],
            1e-5,
            1e-14,
        ),
        "case5" => {
            let reference:Value=serde_json::from_str(include_str!(
                "../../../research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json")).unwrap();
            (
                Problem::new(2, |t, y, o| {
                    o[1] = -y[1] / (1.0 + t) + 0.2 * y[0];
                    o[0] = -(y[0] - y[1].cos()) / 1e-6 - y[1].sin() * o[1];
                    Ok(())
                })
                .jacobian(Jacobian::dense(|t, y, j| {
                    let f = -y[1] / (1.0 + t) + 0.2 * y[0];
                    j[0] = -1e6 - 0.2 * y[1].sin();
                    j[1] = -1e6 * y[1].sin() - y[1].cos() * f + y[1].sin() / (1.0 + t);
                    j[2] = 0.2;
                    j[3] = -1.0 / (1.0 + t);
                    Ok(())
                }))
                .time_derivative(|t, y, o| {
                    o[1] = y[1] / (1.0 + t).powi(2);
                    o[0] = -y[1].sin() * o[1];
                    Ok(())
                }),
                vec![0.0, 1.0],
                2.0,
                reference["reference_state"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect(),
                1e-4,
                1e-12,
            )
        }
        _ if name.starts_with("transition-") => {
            let tau: f64 = name
                .strip_prefix("transition-")
                .unwrap()
                .parse::<f64>()
                .unwrap()
                / 100.0;
            (
                Problem::new(2, move |t, y, o| {
                    let l = 5e4 * (1.0 + (60.0 * (t - tau)).tanh());
                    let (s, c) = (100.0 * t).sin_cos();
                    o[0] = -100.0 * y[1] - l * (y[0] - c);
                    o[1] = 100.0 * y[0] - l * (y[1] - s);
                    Ok(())
                })
                .jacobian(Jacobian::jvp(move |t, _, v, o| {
                    let l = 5e4 * (1.0 + (60.0 * (t - tau)).tanh());
                    o[0] = -100.0 * v[1] - l * v[0];
                    o[1] = 100.0 * v[0] - l * v[1];
                    Ok(())
                }))
                .time_derivative(move |t, y, o| {
                    let q = (60.0 * (t - tau)).tanh();
                    let l = 5e4 * (1.0 + q);
                    let lp = 3e6 * (1.0 - q * q);
                    let (s, c) = (100.0 * t).sin_cos();
                    o[0] = -lp * (y[0] - c) - 100.0 * l * s;
                    o[1] = -lp * (y[1] - s) + 100.0 * l * c;
                    Ok(())
                }),
                vec![1.0, 0.0],
                1.0,
                vec![100.0_f64.cos(), 100.0_f64.sin()],
                1e-5,
                1e-13,
            )
        }
        _ => panic!("unknown case {name}"),
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let case = args.get(1).map_or("rotation", String::as_str);
    let policy = args.get(2).map_or("adaptive", String::as_str);
    let repetition: usize = args.get(3).map_or("1", String::as_str).parse().unwrap();
    let (p, y0, end, reference, cap, uncertainty) = model(case);
    let o = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 50000,
        ..Default::default()
    };
    let start = Instant::now();
    let (success, reached, y, attempts, rhs, extra, elapsed) = match policy {
        "adaptive" | "evidence" => {
            let c = if policy == "adaptive" {
                SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
            } else {
                SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
            };
            let r = SwitchingSolver::solve(&p, (0.0, end), &y0, &o, &c).unwrap();
            let elapsed = start.elapsed().as_secs_f64();
            let pilot_count = r
                .stats
                .segments
                .iter()
                .filter(|s| s.is_probe && s.method == SwitchingMethod::Rodas5p)
                .count();
            let discarded_rhs: u64 = r
                .stats
                .segments
                .iter()
                .filter(|s| s.is_probe && !s.committed)
                .map(|s| s.rhs_calls)
                .sum();
            let implicit_discarded_rhs: u64 = r
                .stats
                .segments
                .iter()
                .filter(|s| s.is_probe && !s.committed && s.method == SwitchingMethod::Rodas5p)
                .map(|s| s.rhs_calls)
                .sum();
            let explicit_return_rhs: u64 = r
                .stats
                .segments
                .iter()
                .filter(|s| s.is_probe && s.method == SwitchingMethod::Dopri5)
                .map(|s| s.rhs_calls)
                .sum();
            let implicit_time: f64 = r
                .stats
                .segments
                .iter()
                .filter(|s| s.committed && s.method == SwitchingMethod::Rodas5p)
                .map(|s| s.reached_time - s.start_time)
                .sum();
            (
                r.success,
                r.final_time,
                r.final_state.clone(),
                r.stats.attempts,
                r.stats.rhs_calls,
                json!({"pilot_count":pilot_count,"discarded_pilot_rhs":discarded_rhs,
                "implicit_discarded_rhs":implicit_discarded_rhs,"explicit_return_rhs":explicit_return_rhs,
                "switches":r.stats.switches,
                "implicit_committed_time":implicit_time,"accounting_complete":r.stats.accounting_complete,
                "segments":r.stats.segments.iter().map(|s|json!({"method":s.method,"start":s.start_time,
                    "target":s.target_time,"reached":s.reached_time,"probe":s.is_probe,"committed":s.committed,
                    "success":s.success,"attempts":s.attempts,"rhs":s.rhs_calls,"work":s.work_units,
                    "initial_h":s.initial_step,"next_h":s.proposed_next_step,"schedule":s.entry_probe_reason,
                    "evidence":s.evidence,"ratio":s.comparator_work_ratio})).collect::<Vec<_>>() }),
                elapsed,
            )
        }
        "dopri" => {
            let r = Dopri5::solve(&p, (0.0, end), &y0, &o).unwrap();
            let elapsed = start.elapsed().as_secs_f64();
            (
                r.success,
                r.final_time,
                r.final_state,
                r.stats.attempts,
                r.stats.counters.rhs_calls,
                json!({"pilot_count":0,"discarded_pilot_rhs":0,"switches":0,"implicit_committed_time":0.0}),
                elapsed,
            )
        }
        "rodas" => {
            let r = Rodas5p::solve(&p, (0.0, end), &y0, &o).unwrap();
            let elapsed = start.elapsed().as_secs_f64();
            (
                r.is_success(),
                *r.t.last().unwrap(),
                r.final_state().unwrap().to_vec(),
                r.stats.attempts,
                r.stats.counters.rhs_calls,
                json!({"pilot_count":0,"discarded_pilot_rhs":0,"switches":0,"implicit_committed_time":end}),
                elapsed,
            )
        }
        _ => panic!("unknown policy"),
    };
    let valid_state = y.len() == reference.len()
        && !reference.is_empty()
        && y.iter().chain(&reference).all(|v| v.is_finite());
    let error = y
        .iter()
        .zip(&reference)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    println!(
        "{}",
        json!({"schema":"vigilode.adaptive-entry-pilot.v1","case":case,"policy":policy,
        "repetition":repetition,"warmup":repetition==0,"success":success,"reached_time":reached,"target":end,
        "inclusive_seconds":elapsed,"attempts":attempts,"rhs":rhs,"physical_state":y,
        "physical_endpoint_max_abs_error":error,"reference":reference,"reference_empirical_uncertainty":uncertainty,
        "accuracy_cap":cap,"finite_matching_state":valid_state,
        "accuracy_pass":valid_state&&success&&reached==end&&error.is_finite()&&error+uncertainty<=cap,
        "detail":extra,"timing_scope":"solver wrapper including setup, all pilots and transitions; excludes fixed model construction and JSON output",
        "comparison_scope":"whole trajectory at common physical endpoint cap; achieved errors can differ; not per-decision false-positive frequency"})
    );
}
