//! R7 original-source-only full-IVP native transactional q1/q2 versus protected RODAS5P.
//! GENERATED from the exact Git-locked WP3 original case definitions.
//! No solver core, controller, source parameter or tolerance modification.
use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSamplingPlan, OutputSchedule, TransactionalQ1Q2Config,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_dense_observed,
};
use serde_json::{Value, json};
use std::{hint::black_box, sync::Arc, time::Instant};

#[derive(Clone)]
struct OriginalSource {
    name: &'static str,
    k: Vec<f64>,
    g: Vec<f64>,
    c: Vec<Vec<f64>>,
    y0: Vec<f64>,
    steps: usize,
}
fn cases() -> Vec<OriginalSource> {
    let a = OriginalSource {
        name: "asymmetric_2",
        k: vec![4., 7.],
        g: vec![20., 40.],
        c: vec![vec![0., 12.], vec![8., 0.]],
        y0: vec![0.2, 0.12],
        steps: 64,
    };
    let mut ar = a.clone();
    ar.name = "asymmetric_2_refined";
    ar.steps = 128;
    let b = OriginalSource {
        name: "coupled_3_cubic",
        k: vec![6., 8., 10.],
        g: vec![60., 100., 30.],
        c: vec![vec![0., 30., 4.], vec![15., 0., 10.], vec![10., 8., 0.]],
        y0: vec![0.04, 0.06, 0.02],
        steps: 128,
    };
    let l = OriginalSource {
        name: "linear_exchange_control",
        k: vec![4., 4.],
        g: vec![0., 0.],
        c: vec![vec![0., 6.], vec![6., 0.]],
        y0: vec![0.1, 0.05],
        steps: 128,
    };
    vec![a, ar, b, l]
}
fn rhs(s: &OriginalSource, y: &[f64], out: &mut [f64]) {
    for i in 0..y.len() {
        let mut x = -s.k[i] * y[i] - s.g[i] * y[i] * y[i] * y[i];
        for j in 0..y.len() {
            if i != j {
                x += s.c[i][j] * (y[j] - y[i]);
            }
        }
        out[i] = x;
    }
}
fn jvp(s: &OriginalSource, y: &[f64], v: &[f64], out: &mut [f64]) {
    for i in 0..y.len() {
        let mut x = -(s.k[i] + 3. * s.g[i] * y[i] * y[i]) * v[i];
        for j in 0..y.len() {
            if i != j {
                x += s.c[i][j] * (v[j] - v[i]);
            }
        }
        out[i] = x;
    }
}
fn source_bits(s: &OriginalSource) -> Vec<u64> {
    let mut b = Vec::new();
    b.extend(s.k.iter().map(|x| x.to_bits()));
    b.extend(s.g.iter().map(|x| x.to_bits()));
    b.extend(s.y0.iter().map(|x| x.to_bits()));
    for row in &s.c {
        b.extend(row.iter().map(|x| x.to_bits()));
    }
    b
}
fn problem(s: &OriginalSource) -> Result<OdeProblem, Box<dyn std::error::Error>> {
    let s = Arc::new(s.clone());
    let sr = Arc::clone(&s);
    let sj = Arc::clone(&s);
    Ok(OdeProblem::new(
        "wp3-original-cooperative-cubic",
        s.k.len(),
        Arc::new(move |_t, y, out| {
            rhs(&sr, y, out);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_t, y, v, out| {
            jvp(&sj, y, v, out);
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )?)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // These are the original comparable WP3 tolerances and untouched formula.
    let rtols = [1e-4_f64, 1e-6, 1e-8];
    for s in cases() {
        let h = 0.1_f64 / s.steps as f64;
        assert_eq!(h * s.steps as f64, 0.1_f64);
        let labels = (0..=s.steps).map(|i| h * i as f64).collect::<Vec<_>>();
        let schedule = OutputSchedule::new(labels.clone())?;
        let ode = problem(&s)?;
        let linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            ..LinearSolverConfig::default()
        };
        let transactional = TransactionalQ1Q2Config::default();
        println!(
            "{}",
            json!({
                "kind":"R7_ORIGINAL_SOURCE_CASE",
                "name":s.name,
                "source_bits":source_bits(&s),
                "y0_bits":s.y0.iter().map(|x|x.to_bits()).collect::<Vec<_>>(),
                "h_bits":h.to_bits(),"steps":s.steps,
                "horizon_bits":0.1_f64.to_bits(),
                "original_wp3_git_blob":"5dc8df34d314c0d8562c6f548a574625cef442ff",
                "transactional_config":transactional,
                "linear_config":linear,
                "output_plan":"same-native-dense-grid-original-WP3",
            })
        );
        for rtol in rtols {
            let atol = rtol * 1e-2;
            let adaptive = AdaptiveStepConfig {
                rtol,
                atol,
                initial_step: h,
                max_step: 0.1,
                max_attempts: 100000,
                ..AdaptiveStepConfig::default()
            };
            for method in ["protected_sequential_RODAS5P", "native_transactional_q1_q2"] {
                let mut timings = Vec::new();
                let mut attempt_status = Vec::new();
                let mut first: Option<Value> = None;
                let mut first_bits: Option<Vec<Vec<u64>>> = None;
                let mut deterministic = true;
                for repetition in 0..3 {
                    let time = Instant::now();
                    let result = match method {
                        "protected_sequential_RODAS5P" => {
                            integrate_sequential_matrix_free_adaptive_dense_observed(
                                &ode,
                                (0.0, 0.1),
                                &s.y0,
                                &linear,
                                &adaptive,
                                &OutputSamplingPlan::dense(schedule.clone()),
                            )
                            .map(|r| (r.observed, json!({"protected_diagnostics":r.diagnostics})))
                            .map_err(|e| e.to_string())
                        }
                        "native_transactional_q1_q2" => {
                            integrate_transactional_q1_q2_adaptive_dense_observed(
                                &ode,
                                (0.0, 0.1),
                                &s.y0,
                                &transactional,
                                &adaptive,
                                &OutputSamplingPlan::dense(schedule.clone()),
                            )
                            .map(|r| {
                                (
                                    r.observed,
                                    json!({
                                        "native_transactional":r.transactional,
                                        "adaptive_diagnostics":r.diagnostics
                                    }),
                                )
                            })
                            .map_err(|e| e.to_string())
                        }
                        _ => unreachable!(),
                    };
                    let elapsed = time.elapsed().as_nanos();
                    timings.push(elapsed);
                    match result {
                        Ok((r, diag)) => {
                            let state_bits =
                                r.y.iter()
                                    .map(|row| row.iter().map(|x| x.to_bits()).collect::<Vec<_>>())
                                    .collect::<Vec<_>>();
                            let time_bits = r.t.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
                            let complete = r.success && r.t == labels && r.y.len() == labels.len();
                            attempt_status.push(json!({
                                "repetition":repetition,"kind":"OK",
                                "success":r.success,"complete_grid":complete,
                                "wall_ns":elapsed
                            }));
                            if let Some(ref prior) = first_bits {
                                if state_bits != *prior {
                                    deterministic = false;
                                }
                            } else {
                                first_bits = Some(state_bits.clone());
                            }
                            if first.is_none() {
                                first = Some(json!({
                                    "success":r.success,"complete_grid":complete,
                                    "message":r.message,
                                    "requested_grid_bits":labels.iter().map(|x|x.to_bits()).collect::<Vec<_>>(),
                                    "actual_grid_bits":time_bits,"state_bits":state_bits,
                                    "work_counters":r.counters,
                                    "internal_steps":r.internal_steps,
                                    "output_clipped_steps":r.output_clipped_steps,
                                    "diagnostics":diag
                                }));
                            }
                            black_box(r);
                        }
                        Err(message) => {
                            attempt_status.push(json!({
                                "repetition":repetition,"kind":"NATIVE_ERROR",
                                "error":message,"wall_ns":elapsed
                            }));
                            break;
                        }
                    }
                }
                println!(
                    "{}",
                    json!({
                        "kind":"R7_NATIVE_TRACE",
                        "source":s.name,"method":method,
                        "rtol_bits":rtol.to_bits(),"atol_bits":atol.to_bits(),
                        "repeat_wall_ns":timings,"repeat_status":attempt_status,
                        "repeats_bit_deterministic":deterministic,
                        "first":first,
                        "scientific_scope":"actual_original_native_full_IVP_not_matched_achieved_error_yet",
                        "production":"HOLD",
                    })
                );
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("{}", json!({"kind":"R7_FATAL","error":e.to_string()}));
        std::process::exit(1);
    }
}
