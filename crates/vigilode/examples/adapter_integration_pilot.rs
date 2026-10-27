//! In-process balanced comparisons of selected policies and relevant direct controls.
//! No telemetry serialization occurs inside timed batches.
#[path = "local_bottleneck_support/models.rs"]
mod models;
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode::{
    Atol, AutomaticOde, AutomaticOdePolicy, DerivativeFallback, Dopri5, ExplicitSolution, Jacobian,
    Options, Problem, Rodas5p, Solution, SwitchingConfig, SwitchingDriverStats, SwitchingEvidence,
    SwitchingMethod, SwitchingSolution, SwitchingSolver,
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
        // Stiff relaxation to a smooth path, then a nonstiff accuracy-limited
        // oscillation. The exact solution is y=g(t) for every lambda(t), so
        // the implicit history is cheap while the current explicit route wins.
        "stale-return" => {
            fn lam(t: f64) -> (f64, f64) {
                let q = (200.0 * (t - 0.5)).tanh();
                (1.0 + 4999.5 * (1.0 - q), -4999.5 * 200.0 * (1.0 - q * q))
            }
            fn g(t: f64) -> (f64, f64, f64) {
                let (a, w) = (1e-2, 150.0);
                let q = (200.0 * (t - 0.5)).tanh();
                let (s, s1, s2) = (
                    0.5 * (1.0 + q),
                    100.0 * (1.0 - q * q),
                    -40000.0 * q * (1.0 - q * q),
                );
                let (sn, cs) = (w * t).sin_cos();
                (
                    t.cos() + a * s * sn,
                    -t.sin() + a * (s1 * sn + s * w * cs),
                    -t.cos() + a * (s2 * sn + 2.0 * s1 * w * cs - s * w * w * sn),
                )
            }
            let (g1, _, _) = g(1.0);
            (
                Problem::new(1, |t, y, o| {
                    let (l, _) = lam(t);
                    let (g0, g1, _) = g(t);
                    o[0] = -l * (y[0] - g0) + g1;
                    Ok(())
                })
                .jacobian(Jacobian::dense(|t, _, j| {
                    j[0] = -lam(t).0;
                    Ok(())
                }))
                .time_derivative(|t, y, o| {
                    let (l, lp) = lam(t);
                    let (g0, g1, g2) = g(t);
                    o[0] = -lp * (y[0] - g0) + l * g1 + g2;
                    Ok(())
                }),
                vec![g(0.0).0],
                1.0,
                vec![g1],
                1e-5,
                1e-14,
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

// Reuse the published nonlinear fixtures, including the directional dual
// arithmetic. Dense and matrix-free case5 controls differ only in provider.
fn catalog_problem(m: &models::Model, dense: bool) -> Problem {
    let mr = m.clone();
    let mut p = Problem::new(m.n, move |t, y, out| {
        mr.apply(t, y, None, false, out, false);
        Ok(())
    });
    let mj = m.clone();
    if dense {
        p = p.jacobian(Jacobian::dense(move |t, y, a| {
            // These tiny fixed fixtures have n<=6. Avoid provider allocation
            // differences while preserving exactly the same dual derivatives.
            let mut v = [0.0; 6];
            let mut col = [0.0; 6];
            for j in 0..mj.n {
                v[j] = 1.0;
                mj.apply(t, y, Some(&v[..mj.n]), false, &mut col[..mj.n], false);
                for i in 0..mj.n {
                    a[i * mj.n + j] = col[i];
                }
                v[j] = 0.0;
            }
            Ok(())
        }));
    } else {
        p = p.jacobian(Jacobian::jvp(move |t, y, v, out| {
            mj.apply(t, y, Some(v), false, out, false);
            Ok(())
        }));
    }
    if m.autonomous() {
        p.autonomous()
    } else {
        let mt = m.clone();
        p.time_derivative(move |t, y, out| {
            mt.apply(t, y, None, true, out, false);
            Ok(())
        })
    }
}

struct Fixture {
    problem: Problem,
    dense_problem: Option<Problem>,
    y0: Vec<f64>,
    end: f64,
    reference: Vec<f64>,
    cap: f64,
    uncertainty: f64,
    options: Options,
    report_only: bool,
}

fn fixture(case: &str) -> Fixture {
    let gyro = case.starts_with("gyro-");
    let (mut problem, mut y0, mut end, mut reference, mut cap, mut uncertainty) =
        if gyro { model("decay") } else { model(case) };
    let mut dense_problem = None;
    let mut options = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-7,
        max_steps: 100_000,
        ..Default::default()
    };
    if case.starts_with("transition-") || case == "relaxation" || case == "stale-return" {
        options.h0 = 1e-4;
    }
    if case == "case5" || gyro {
        let m = models::Model::new(if gyro { 10 } else { 5 }, false).unwrap();
        problem = catalog_problem(&m, false);
        dense_problem = Some(catalog_problem(&m, true));
        y0 = m.initial();
        end = m.end;
        cap = 1e-4;
        options.h0 = 1e-4;
        options.rtol = 1e-6;
        options.atol = Atol::Scalar(1e-9);
        if gyro {
            reference = m.exact(end).unwrap();
            uncertainty = 1e-13;
        }
        if case == "gyro-tight" {
            options.rtol = 1e-9;
            options.atol = Atol::Scalar(1e-12);
        }
    }
    Fixture {
        problem,
        dense_problem,
        y0,
        end,
        reference,
        cap,
        uncertainty,
        options,
        report_only: case == "gyro-default",
    }
}

// Preserve raw results until timing stops; constructing JSON here would
// charge different amounts of serialization to the compared solvers.
// Preserve the benchmark's existing inline result ownership; boxing would add
// allocation work to the measured implicit arm merely to satisfy a size lint.
#[allow(clippy::large_enum_variant)]
enum Raw {
    Switching(SwitchingSolution),
    Explicit(ExplicitSolution),
    Implicit(Solution),
}
fn solve(f: &Fixture, arm: &str) -> Result<Raw, String> {
    let span = (0.0, f.end);
    match arm {
        "evidence"
        | "adaptive"
        | "adaptive-unseeded"
        | "adaptive-no-startup-retry"
        | "adaptive-sentinel"
        | "adaptive-probe-unbudgeted" => {
            let mut config = if arm.starts_with("adaptive") {
                SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
            } else {
                SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
            };
            if arm == "adaptive-unseeded" {
                config.reuse_entry_pilot_step = false;
            }
            if arm == "adaptive-no-startup-retry" {
                config.retry_unresolved_entry_once = false;
            }
            if arm == "adaptive-sentinel" {
                config.return_sentinel_interval = 1;
            }
            // Diagnostic control only: isolates the historical-rate screen.
            if arm == "adaptive-probe-unbudgeted" {
                config.probe_attempt_budget = usize::MAX;
            }
            SwitchingSolver::solve(&f.problem, span, &f.y0, &f.options, &config)
                .map(Raw::Switching)
                .map_err(|e| e.to_string())
        }
        "dopri" => Dopri5::solve(&f.problem, span, &f.y0, &f.options)
            .map(Raw::Explicit)
            .map_err(|e| e.to_string()),
        "rodas" | "rodas-dense" => {
            let p = if arm == "rodas-dense" {
                f.dense_problem.as_ref().unwrap_or(&f.problem)
            } else {
                &f.problem
            };
            Rodas5p::solve(p, span, &f.y0, &f.options)
                .map(Raw::Implicit)
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("unknown arm {arm}")),
    }
}

fn summary(raw: &Raw, f: &Fixture) -> Value {
    let (success, time, state, attempts, rhs, stats) = match raw {
        Raw::Switching(r) => (
            r.success,
            r.final_time,
            r.final_state.as_slice(),
            r.stats.attempts,
            r.stats.rhs_calls,
            json!(r.stats),
        ),
        Raw::Explicit(r) => (
            r.success,
            r.final_time,
            r.final_state.as_slice(),
            r.stats.attempts,
            r.stats.counters.rhs_calls,
            json!(r.stats),
        ),
        Raw::Implicit(r) => (
            r.is_success(),
            r.t.last().copied().unwrap_or(0.0),
            r.final_state().unwrap_or(&[]),
            r.stats.attempts,
            r.stats.counters.rhs_calls,
            json!(r.stats),
        ),
    };
    let valid = state.len() == f.reference.len()
        && !state.is_empty()
        && state.iter().chain(&f.reference).all(|v| v.is_finite());
    let error = state
        .iter()
        .zip(&f.reference)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    let mut dispositions = json!({});
    if let Raw::Switching(r) = raw {
        // Uncommitted is a physical-state fact, not a synonym for wasted work.
        for s in &r.stats.segments {
            let kind = if s.is_probe && s.method == SwitchingMethod::Rodas5p {
                if s.decision.starts_with("same-window implicit comparator") {
                    if s.committed {
                        "productive_implicit_comparator"
                    } else if s.success {
                        "uncommitted_return_comparator"
                    } else {
                        "incomplete_return_comparator"
                    }
                } else if s.evidence == SwitchingEvidence::ImplicitCheaperOnPrefix && !s.committed {
                    "winning_entry_pilot_uncommitted"
                } else if s.committed {
                    "productive_implicit_comparator"
                } else {
                    "losing_or_incomplete_entry_pilot"
                }
            } else if s.is_probe && s.method == SwitchingMethod::Dopri5 {
                if s.committed {
                    "committed_explicit_return"
                } else {
                    "uncommitted_explicit_return"
                }
            } else if s.committed {
                "committed_integration"
            } else {
                "other_uncommitted"
            };
            let d = dispositions
                .as_object_mut()
                .unwrap()
                .entry(kind)
                .or_insert_with(|| json!({"segments":0,"attempts":0,"rhs":0,"work_units":0.0}));
            d["segments"] = json!(d["segments"].as_u64().unwrap() + 1);
            d["attempts"] = json!(d["attempts"].as_u64().unwrap() + s.attempts as u64);
            d["rhs"] = json!(d["rhs"].as_u64().unwrap() + s.rhs_calls);
            d["work_units"] = json!(d["work_units"].as_f64().unwrap() + s.work_units);
        }
    }
    json!({"success":success,"reached_time":time,"state":state,"attempts":attempts,"rhs":rhs,
        "max_component_absolute_error":error,"accuracy_pass":valid && success && time==f.end
          && error+f.uncertainty<=f.cap,"dispositions":dispositions,"stats":stats})
}

fn batch(f: &Fixture, arm: &str, repetitions: usize) -> Result<(f64, Raw), String> {
    let start = Instant::now();
    let mut last = None;
    for _ in 0..repetitions {
        // Replacing last also charges destruction of the preceding result.
        last = Some(black_box(solve(black_box(f), black_box(arm))?));
    }
    let seconds = start.elapsed().as_secs_f64();
    Ok((seconds, last.unwrap()))
}

fn calibrate(f: &Fixture, arm: &str) -> Result<(usize, f64), String> {
    let mut n = 1;
    let mut elapsed_total = 0.0;
    loop {
        let (elapsed, raw) = batch(f, arm, n)?;
        black_box(raw);
        elapsed_total += elapsed;
        if elapsed >= 0.100 || n >= 1_000_000 {
            return Ok((n, elapsed_total));
        }
        n = (n * 2).min(1_000_000);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let case = args.get(1).map_or("rotation", String::as_str);
    if case == "rhs-only-availability" {
        return availability();
    }
    let a = args.get(2).map_or("evidence", String::as_str);
    let b = args.get(3).map_or("adaptive", String::as_str);
    let blocks: usize = args
        .get(4)
        .map_or("5", String::as_str)
        .parse()
        .map_err(|e| format!("{e}"))?;
    if !(1..=20).contains(&blocks) {
        return Err("blocks must be 1..20".into());
    }
    let f = fixture(case);
    let before_a = summary(&solve(&f, a)?, &f);
    let before_b = summary(&solve(&f, b)?, &f);
    let (na, ca) = calibrate(&f, a)?;
    let (nb, cb) = calibrate(&f, b)?;
    let mut timings = Vec::new();
    let mut deterministic = true;
    let mut timed_accuracy_pass = true;
    let whole = Instant::now();
    for block in 0..blocks {
        let order = if block % 2 == 0 {
            [a, b, b, a]
        } else {
            [b, a, a, b]
        };
        for (slot, arm) in order.iter().enumerate() {
            if whole.elapsed().as_secs_f64() > 55.0 {
                return Err("cell exceeded 55 second timing budget".into());
            }
            let n = if *arm == a { na } else { nb };
            let (seconds, raw) = batch(&f, arm, n)?;
            // Extraction, comparison, and serialization are after the clock.
            let observed = summary(&raw, &f);
            deterministic &= observed
                == if *arm == a {
                    before_a.clone()
                } else {
                    before_b.clone()
                };
            timed_accuracy_pass &= observed["accuracy_pass"].as_bool().unwrap();
            timings.push(json!({"block":block,"slot":slot,"arm":arm,"repetitions":n,
                "seconds":seconds,"seconds_per_solve":seconds/n as f64}));
        }
    }
    println!(
        "{}",
        json!({"schema":"vigilode.adapter-integration-pair.v1","case":case,
        "arms":[a,b],"blocks":blocks,"order":"ABBA/BAAB alternating",
        "target_batch_seconds":0.100,"timings":timings,"deterministic":deterministic,
        "timed_last_solve_accuracy_pass":timed_accuracy_pass,"initial_records":{(a):before_a,(b):before_b},
        "calibration_seconds":{(a):ca,(b):cb},"options":f.options,"reference":f.reference,
        "reference_uncertainty":f.uncertainty,"accuracy_cap":f.cap,"report_only":f.report_only,
        "comparison_scope":"same full-state endpoint error cap; attained errors reported; no tolerance retuning",
        "output_contract_difference":"switching retains eight uniform landing windows plus accepted batch endpoints; direct controls request only span endpoints; selection-overhead comparison includes that documented landing obligation",
        "timing_scope":"solver setup, allocations, all callbacks, rejected attempts, pilots, transitions and stats construction; excludes fixed fixture creation and JSON extraction; last result destruction is outside each batch",
        "validation_scope":"each arm before timing plus last solve of every batch; repeated solve counts and state deterministic on sampled outputs",
        "source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").ok()})
    );
    Ok(())
}

fn availability() -> Result<(), String> {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    });
    let o = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        ..Default::default()
    };
    let policy = AutomaticOdePolicy {
        derivatives: DerivativeFallback::ProvidedOnly,
        ..Default::default()
    };
    let automatic =
        AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &o, &policy).map_err(|e| e.to_string())?;
    let times: Vec<f64> = (0..=8).map(|i| i as f64 / 8.0).collect();
    let direct =
        Dopri5::solve_observed(&p, (0.0, 1.0), &[1.0], &o, &times).map_err(|e| e.to_string())?;
    let native_counters = automatic
        .stats
        .segments
        .first()
        .and_then(|s| match &s.driver {
            SwitchingDriverStats::Dopri5(stats) => Some(json!(stats.counters)),
            _ => None,
        });
    let exact_match = automatic.stats.segments.len() == 1
        && native_counters == Some(json!(direct.stats.counters))
        && automatic.stats.attempts == direct.stats.attempts
        && automatic.stats.accepted_steps == direct.stats.accepted_steps
        && automatic.stats.rejected_steps == direct.stats.rejected_steps
        && automatic.final_state == direct.final_state
        && automatic.y == direct.y
        && automatic.t == direct.t;
    let error = (automatic.final_state[0] - (-1.0_f64).exp()).abs();
    let passed = automatic.success
        && direct.success
        && exact_match
        && error <= 1e-5
        && automatic.config.fallback == DerivativeFallback::Forbidden
        && automatic
            .message
            .contains("implicit challenger unavailable");
    println!(
        "{}",
        json!({"schema":"vigilode.adapter-availability.v1","passed":passed,
        "fixture":"RHS-only scalar decay; optional ProvidedOnly permission; replay permission false",
        "full_native_work_and_state_match":exact_match,"max_component_absolute_error":error,
        "accuracy_cap":1e-5,"automatic":automatic,"direct_observed":direct,
        "timing_authority":"none; previously rejected requests do not define a performance baseline"})
    );
    if passed {
        Ok(())
    } else {
        Err("availability/observed parity failed".into())
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("adapter_integration_pilot: {e}");
        std::process::exit(2);
    }
}
