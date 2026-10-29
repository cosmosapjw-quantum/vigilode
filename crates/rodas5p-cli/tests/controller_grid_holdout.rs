//! Research node `research/ct01_controller_grid_holdout_20261010` (CT01):
//! the predictive controller (`PredictiveCapped2`, gated; `PredictiveCapped`,
//! reported) against the production `Integral` controller on a fresh interior
//! output grid and a fresh fixed corpus of initial steps.
//!
//! Cells, as ALG05: the dense fast driver
//! (`integrate_rodas5p_fast_observed_with_telemetry`, which is
//! `integrate_rodas5p_fast_observed` plus the opt-in observer; see
//! `ct01_controller_telemetry_parity.rs`) on van-der-pol-mu1000, hires,
//! robertson and brusselator-1d-50 from the stiff benchmark's
//! `benchmark_problems()`, with atol = rtol * atol_scale, min step 1e-14,
//! max step = span, 1,000,000 attempts, everything else the default except
//! the initial step (the seed). ALG05's quarter-decade ladders.
//!
//! Fresh output grid: t_k = t0 + k (tf - t0) / 40, k = 1..40 (the last point
//! is tf). Gated seeds h0 in {2e-6, 7e-6, 2e-5, 7e-5, 2e-4, 7e-4}; the anchor
//! seed 3e-6 (ALG05's) is reported only. 92 rungs x 7 seeds x 3 arms = 1932
//! runs. The seeds are a fixed deterministic corpus, not random samples.
//!
//! Error: the grid error is the maximum over the 40 grid points of
//! `max_i |y_i - r_i| / max(|r_i|, 1e-10)` against a reference trajectory at
//! the same points; the endpoint error is the same metric at tf only. The
//! reference of a problem is arm `I` on the grid with h0 = 1e-6 at rtol 1e-13
//! (atol = rtol * atol_scale); its check is the same at rtol 1e-12.
//!
//! `export_runs` writes the references and then every run (states at every
//! grid point as hex bits, counters, telemetry, errors) to `CT01_RUNS`.

#[path = "../src/stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerDecision, ControllerEvents, ControllerKind, ControllerTelemetry,
    OutputSchedule, Rodas5pFastResult, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_observed_with_telemetry,
};
use serde_json::{Value, json};
use stiff_benchmark::{BenchmarkProblem, benchmark_problems};

/// The problems, in export order.
const PROBLEMS: [&str; 4] = [
    "van-der-pol-mu1000",
    "hires",
    "robertson",
    "brusselator-1d-50",
];
/// The fresh gated seed corpus.
const GATED_SEEDS: [f64; 6] = [2.0e-6, 7.0e-6, 2.0e-5, 7.0e-5, 2.0e-4, 7.0e-4];
/// ALG05's seed carrying the shared outlier; reported only.
const ANCHOR_SEED: f64 = 3.0e-6;
/// Every seed, in export order (ascending).
const SEEDS: [f64; 7] = [2.0e-6, 3.0e-6, 7.0e-6, 2.0e-5, 7.0e-5, 2.0e-4, 7.0e-4];
/// The arms: reference, reported, gated.
const ARMS: [&str; 3] = ["I", "PREDcap", "PREDcap2"];
/// Interior grid points (the last is tf).
const GRID_POINTS: usize = 40;
/// The reference run: arm I, this initial step, rtol 1e-13; check rtol 1e-12.
const REFERENCE_SEED: f64 = 1.0e-6;
const REFERENCE_RTOL: f64 = 1.0e-13;
const CHECK_RTOL: f64 = 1.0e-12;
/// The stiff benchmark's attempt budget.
const MAX_ATTEMPTS: usize = 1_000_000;
/// Floor of the benchmark's componentwise error metric.
const ERROR_FLOOR: f64 = 1.0e-10;
/// Registered run count.
const RUNS: usize = 1932;
const SCHEMA: &str = "vigilode-ct01-runs-v1";
const ERROR_METRIC: &str = "max over the 40 grid points of max_i |y_i - r_i| / max(|r_i|, 1e-10) \
     against the reference trajectory at the same points";
const ENDPOINT_METRIC: &str = "max_i |y_i - r_i| / max(|r_i|, 1e-10) at tf against the reference";
/// The telemetry event names, in export order.
const EVENTS: [&str; 6] = [
    "clipped_landings",
    "sliver_landings",
    "sliver_landings_while_rejection_pending",
    "informative_clipped_landings_after_rejection",
    "zero_error_accepts_while_rejection_pending",
    "accepted_next_request_exceeds_trial",
];

/// `10^(-q/4)`; whole decades are the exact literals (ALG05).
fn quarter_decade(q: u32) -> f64 {
    if q.is_multiple_of(4) {
        format!("1e-{}", q / 4).parse().unwrap()
    } else {
        10.0_f64.powf(-(q as f64) / 4.0)
    }
}

/// ALG05's quarter-decade ladder of a problem.
fn ladder(problem: &str) -> Vec<f64> {
    let last = match problem {
        "van-der-pol-mu1000" | "brusselator-1d-50" => 28,
        "hires" | "robertson" => 40,
        other => panic!("no ladder for {other}"),
    };
    (12..=last).map(quarter_decade).collect()
}

/// The output schedule: t0 and the 40 grid points t0 + k (tf - t0) / 40.
fn grid(problem: &BenchmarkProblem) -> Vec<f64> {
    let (t0, tf) = problem.t_span;
    let mut times = vec![t0];
    times.extend((1..=GRID_POINTS).map(|k| t0 + (k as f64) * (tf - t0) / GRID_POINTS as f64));
    times
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

/// The benchmark metric at one point.
fn point_error(y: &[f64], reference: &[f64]) -> f64 {
    assert_eq!(y.len(), reference.len());
    y.iter()
        .zip(reference)
        .map(|(a, r)| (a - r).abs() / r.abs().max(ERROR_FLOOR))
        .fold(0.0, f64::max)
}

/// The grid error: the metric's maximum over the grid points.
fn grid_error(states: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    assert_eq!(states.len(), GRID_POINTS);
    assert_eq!(reference.len(), GRID_POINTS);
    states
        .iter()
        .zip(reference)
        .map(|(y, r)| point_error(y, r))
        .fold(0.0, f64::max)
}

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// NATIVE.json reference final states by problem id.
fn native_reference(problem: &str) -> Vec<f64> {
    let path = workspace_path("research/stiff_native_benchmark_20261001/NATIVE.json");
    let native: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    native["references"][problem]["final_state"]
        .as_array()
        .unwrap_or_else(|| panic!("no NATIVE reference for {problem}"))
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

/// Writes `value` (compact JSON) to the path in `variable`, if set; a
/// relative path is taken from the workspace root. An existing file is never
/// overwritten.
fn write_output(variable: &str, value: &Value) {
    if let Ok(path) = std::env::var(variable) {
        let path = workspace_path(&path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string(value).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

fn problems() -> Vec<BenchmarkProblem> {
    let mut all = benchmark_problems().unwrap();
    PROBLEMS
        .iter()
        .map(|id| {
            let at = all.iter().position(|p| p.id == *id).unwrap();
            all.swap_remove(at)
        })
        .collect()
}

/// ALG05's configuration with the seed as initial step; the production
/// Integral controller.
fn base_config(problem: &BenchmarkProblem, rtol: f64, seed: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * problem.atol_scale,
        rtol,
        initial_step: seed,
        min_step: 1.0e-14,
        max_step: problem.t_span.1 - problem.t_span.0,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

/// The configuration of one arm.
fn arm_config(arm: &str, base: AdaptiveStepConfig) -> AdaptiveStepConfig {
    match arm {
        "I" => base,
        "PREDcap" => AdaptiveStepConfig {
            controller: ControllerKind::PredictiveCapped,
            ..base
        },
        "PREDcap2" => AdaptiveStepConfig {
            controller: ControllerKind::PredictiveCapped2,
            ..base
        },
        other => panic!("unknown arm {other}"),
    }
}

fn telemetry_json(t: &ControllerTelemetry) -> Value {
    json!({
        "clipped_landings": t.clipped_landings,
        "sliver_landings": t.sliver_landings,
        "sliver_landings_while_rejection_pending": t.sliver_landings_while_rejection_pending,
        "informative_clipped_landings_after_rejection": t.informative_clipped_landings_after_rejection,
        "zero_error_accepts_while_rejection_pending": t.zero_error_accepts_while_rejection_pending,
        "accepted_next_request_exceeds_trial": t.accepted_next_request_exceeds_trial,
        "updates": t.updates,
        "rejection_pending_at_end": t.rejection_pending,
    })
}

fn event_names(e: &ControllerEvents) -> Vec<&'static str> {
    let flags = [
        e.clipped_landing,
        e.sliver_landing,
        e.sliver_landing_while_rejection_pending,
        e.informative_clipped_landing_after_rejection,
        e.zero_error_accept_while_rejection_pending,
        e.accepted_next_request_exceeds_trial,
    ];
    EVENTS
        .iter()
        .zip(flags)
        .filter(|(_, f)| *f)
        .map(|(n, _)| *n)
        .collect()
}

/// One run: the result fields, the grid states and the errors against the
/// reference grid states (`None` for a reference run itself, whose errors
/// are zero by definition and recomputed as such by the checker).
fn result_row(
    result: &rodas5p_core::CoreResult<(Rodas5pFastResult, ControllerTelemetry)>,
    reference: Option<&[Vec<f64>]>,
) -> serde_json::Map<String, Value> {
    let value = match result {
        Ok((r, telemetry)) => {
            let complete = r.observed.success;
            let c: &WorkCounters = &r.observed.counters;
            // The states at the grid points reached (t0 excluded).
            let states: Vec<Vec<f64>> = r.observed.y[1..].to_vec();
            let (grid, endpoint) = if complete {
                match reference {
                    Some(reference) => (
                        json!(grid_error(&states, reference)),
                        json!(point_error(
                            states.last().unwrap(),
                            reference.last().unwrap()
                        )),
                    ),
                    None => (json!(0.0), json!(0.0)),
                }
            } else {
                (Value::Null, Value::Null)
            };
            json!({
                "ok": true,
                "success": complete,
                "message": r.observed.message,
                "attempts": r.attempts,
                "accepted": r.accepted_steps,
                "rejected": r.rejected_steps,
                "jacobian_reuses": r.jacobian_reuses,
                "internal_steps": r.observed.internal_steps,
                "output_clipped_steps": r.observed.output_clipped_steps,
                "rhs_evaluations": c.rhs_evaluations,
                "lu_factorizations": c.direct_factorizations,
                "jacobian_builds": c.jacobian_builds,
                "linear_solve_failures": c.linear_solve_failures,
                "nonfinite_step_failures": c.nonfinite_step_failures,
                "counters": serde_json::to_value(c).unwrap(),
                "grid_t": hexes(&r.observed.t[1..]),
                "grid_states": states.iter().map(|y| hexes(y)).collect::<Vec<_>>(),
                "telemetry": telemetry_json(telemetry),
                "grid_error": grid,
                "endpoint_error": endpoint,
            })
        }
        Err(e) => json!({"ok": false, "success": false, "error_message": e.to_string()}),
    };
    match value {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn integrate(
    problem: &BenchmarkProblem,
    config: &AdaptiveStepConfig,
    telemetry: ControllerTelemetry,
) -> rodas5p_core::CoreResult<(Rodas5pFastResult, ControllerTelemetry)> {
    let schedule = OutputSchedule::new(grid(problem)).unwrap();
    integrate_rodas5p_fast_observed_with_telemetry(
        &problem.problem,
        problem.t_span,
        &problem.y0,
        config,
        &schedule,
        telemetry,
    )
}

fn keyed(problem: &BenchmarkProblem, arm: &str, config: &AdaptiveStepConfig, seed: f64) -> Value {
    let mut row = serde_json::Map::new();
    row.insert("problem".into(), json!(problem.id));
    row.insert("arm".into(), json!(arm));
    row.insert("rtol".into(), json!(config.rtol));
    row.insert("atol".into(), json!(config.atol));
    row.insert("seed".into(), json!(seed));
    Value::Object(row)
}

/// The reference and check runs of a problem and the reference grid states.
fn reference_record(problem: &BenchmarkProblem) -> (Value, Vec<Vec<f64>>) {
    let run = |rtol: f64| {
        let config = base_config(problem, rtol, REFERENCE_SEED);
        integrate(problem, &config, ControllerTelemetry::default())
    };
    let reference = run(REFERENCE_RTOL);
    let states = match &reference {
        Ok((r, _)) if r.observed.success => r.observed.y[1..].to_vec(),
        _ => panic!("reference run of {} failed", problem.id),
    };
    let check = run(CHECK_RTOL);
    let mut ref_row = keyed(
        problem,
        "I",
        &base_config(problem, REFERENCE_RTOL, REFERENCE_SEED),
        REFERENCE_SEED,
    );
    ref_row
        .as_object_mut()
        .unwrap()
        .extend(result_row(&reference, None));
    let mut check_row = keyed(
        problem,
        "I",
        &base_config(problem, CHECK_RTOL, REFERENCE_SEED),
        REFERENCE_SEED,
    );
    check_row
        .as_object_mut()
        .unwrap()
        .extend(result_row(&check, Some(&states)));
    let native = native_reference(problem.id);
    let native_difference = point_error(states.last().unwrap(), &native);
    eprintln!(
        "{} reference: attempts {} u {} native difference {native_difference:e}",
        problem.id, ref_row["attempts"], check_row["grid_error"]
    );
    (
        json!({
            "reference": ref_row,
            "check": check_row,
            "uncertainty": check_row["grid_error"],
            "native_endpoint_difference": native_difference,
        }),
        states,
    )
}

/// The first controller update at which two traces differ (1-based), if
/// any; a strict prefix differs at the first update past it.
fn first_difference(a: &[ControllerDecision], b: &[ControllerDecision]) -> Option<usize> {
    let same = |x: &ControllerDecision, y: &ControllerDecision| {
        x.requested_h.to_bits() == y.requested_h.to_bits()
            && x.trial_h.to_bits() == y.trial_h.to_bits()
            && x.error.to_bits() == y.error.to_bits()
            && x.accepted == y.accepted
            && x.clipped == y.clipped
            && x.next_h.to_bits() == y.next_h.to_bits()
    };
    match a.iter().zip(b).position(|(x, y)| !same(x, y)) {
        Some(i) => Some(i + 1),
        None if a.len() != b.len() => Some(a.len().min(b.len()) + 1),
        None => None,
    }
}

/// The `PREDcap2` events up to and including its first difference from
/// `PREDcap`, and the update at which it differs.
fn divergence(predcap: &[ControllerDecision], predcap2: &[ControllerDecision]) -> Value {
    let Some(at) = first_difference(predcap, predcap2) else {
        return Value::Null;
    };
    let mut observer = ControllerTelemetry::default();
    let mut at_events = ControllerEvents::default();
    let mut pending_before = false;
    for d in predcap2.iter().take(at) {
        pending_before = observer.rejection_pending;
        at_events = observer.observe(*d);
    }
    let decision = predcap2.get(at - 1);
    let previous_accepted = at >= 2 && predcap2[at - 2].accepted;
    // The two kinds differ only in the cap's trigger: PREDcap caps after a
    // rejected or failed previous attempt; PREDcap2 caps while a rejection is
    // pending (kept by sliver landings) and caps err = 0 too.
    let cause = match decision {
        Some(d) if d.accepted && pending_before && d.error == 0.0 => "zero_error_while_pending",
        Some(d) if d.accepted && pending_before && previous_accepted => "pending_kept_by_sliver",
        _ => "other",
    };
    json!({
        "update": at,
        "cause": cause,
        "pending_before": pending_before,
        "previous_attempt_accepted": previous_accepted,
        "events_at_update": event_names(&at_events),
        "events_through_update": {
            "sliver_landings_while_rejection_pending": observer.sliver_landings_while_rejection_pending,
            "zero_error_accepts_while_rejection_pending": observer.zero_error_accepts_while_rejection_pending,
            "informative_clipped_landings_after_rejection": observer.informative_clipped_landings_after_rejection,
            "sliver_landings": observer.sliver_landings,
            "clipped_landings": observer.clipped_landings,
            "accepted_next_request_exceeds_trial": observer.accepted_next_request_exceeds_trial,
        },
    })
}

/// The three arms of one cell (problem, seed, rtol), in arm order.
fn cell_rows(
    problem: &BenchmarkProblem,
    seed: f64,
    rtol: f64,
    reference: &[Vec<f64>],
) -> Vec<Value> {
    let mut traces = Vec::new();
    let mut rows = Vec::new();
    for arm in ARMS {
        let config = arm_config(arm, base_config(problem, rtol, seed));
        let mut result = integrate(problem, &config, ControllerTelemetry::with_trace());
        let trace = match result.as_mut() {
            Ok((_, telemetry)) => telemetry.trace.take().unwrap(),
            Err(_) => Vec::new(),
        };
        let mut row = keyed(problem, arm, &config, seed);
        row.as_object_mut()
            .unwrap()
            .extend(result_row(&result, Some(reference)));
        eprintln!(
            "{} {arm} seed={seed:e} rtol={rtol:e}: attempts {} rejected {} clipped {} grid error {}",
            problem.id,
            row["attempts"],
            row["rejected"],
            row["output_clipped_steps"],
            row["grid_error"]
        );
        traces.push(trace);
        rows.push(row);
    }
    let divergence = divergence(&traces[1], &traces[2]);
    for (arm, row) in ARMS.iter().zip(rows.iter_mut()) {
        let value = if *arm == "PREDcap2" {
            divergence.clone()
        } else {
            Value::Null
        };
        row.as_object_mut()
            .unwrap()
            .insert("first_difference_from_PREDcap".into(), value);
    }
    rows
}

#[test]
fn grid_seeds_ladders_and_cells_are_the_registered_ones() {
    assert_eq!(ladder("van-der-pol-mu1000").len(), 17);
    assert_eq!(ladder("brusselator-1d-50").len(), 17);
    assert_eq!(ladder("hires").len(), 29);
    assert_eq!(ladder("robertson").len(), 29);
    let hires = ladder("hires");
    assert_eq!(hires[0], 1.0e-3);
    assert_eq!(hires[8], 1.0e-5);
    assert_eq!(hires[28], 1.0e-10);
    assert_eq!(ladder("van-der-pol-mu1000")[16], 1.0e-7);
    for l in PROBLEMS.map(ladder) {
        for pair in l.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((ratio - 10.0_f64.powf(0.25)).abs() < 1.0e-12, "{pair:?}");
        }
    }
    // Fresh gated seeds: none of ALG02's or ALG05's; the anchor is ALG05's.
    for old in [1.0e-6, 1.0e-4, 1.0e-2, 3.0e-6, 3.0e-5, 3.0e-3] {
        assert!(!GATED_SEEDS.contains(&old));
    }
    assert!(!GATED_SEEDS.contains(&ANCHOR_SEED));
    let mut all = GATED_SEEDS.to_vec();
    all.push(ANCHOR_SEED);
    all.sort_by(f64::total_cmp);
    assert_eq!(all, SEEDS);
    let problems = problems();
    assert_eq!(problems.iter().map(|p| p.id).collect::<Vec<_>>(), PROBLEMS);
    let rungs: usize = problems.iter().map(|p| ladder(p.id).len()).sum();
    assert_eq!(rungs, 92);
    assert_eq!(rungs * SEEDS.len() * ARMS.len(), RUNS);
    // The grid: t0, then 40 uniform points; the last is tf exactly.
    let spans = [2000.0, 321.8122, 40.0, 10.0];
    for (problem, span) in problems.iter().zip(spans) {
        assert_eq!(problem.t_span, (0.0, span));
        let g = grid(problem);
        assert_eq!(g.len(), GRID_POINTS + 1);
        assert_eq!(g[0], problem.t_span.0);
        assert_eq!(*g.last().unwrap(), problem.t_span.1);
        assert_eq!(g[20], span / 2.0);
        OutputSchedule::new(g).unwrap();
        assert_eq!(native_reference(problem.id).len(), problem.y0.len());
        for seed in SEEDS.iter().chain([&REFERENCE_SEED]) {
            let config = base_config(problem, 1.0e-6, *seed);
            config.validate().unwrap();
            assert_eq!(config.controller, ControllerKind::Integral);
            assert_eq!(config.safety, 0.9);
            assert_eq!(config.initial_step, *seed);
            assert_eq!(config.max_attempts, MAX_ATTEMPTS);
        }
        let base = base_config(problem, 1.0e-5, SEEDS[3]);
        assert_eq!(arm_config("I", base.clone()), base);
        for (arm, kind) in [
            ("PREDcap", ControllerKind::PredictiveCapped),
            ("PREDcap2", ControllerKind::PredictiveCapped2),
        ] {
            let config = arm_config(arm, base.clone());
            assert_eq!(config.controller, kind);
            assert_eq!(
                AdaptiveStepConfig {
                    controller: ControllerKind::Integral,
                    ..config
                },
                base
            );
        }
    }
}

/// Telemetry parity on a few registered cells: the exported row without
/// its telemetry is the same with the observer off (the plain driver) and
/// on (with a trace).
#[test]
fn telemetry_parity_on_registered_cells() {
    let problems = problems();
    for (problem, rtol, seed) in [
        (&problems[0], 1.0e-3, 7.0e-4),
        (&problems[2], quarter_decade(17), 2.0e-6),
        (&problems[1], 1.0e-4, ANCHOR_SEED),
    ] {
        // Any finite stand-in reference: parity concerns the run, not its error.
        let stand_in = vec![problem.y0.clone(); GRID_POINTS];
        for arm in ARMS {
            let config = arm_config(arm, base_config(problem, rtol, seed));
            let schedule = OutputSchedule::new(grid(problem)).unwrap();
            let off = integrate_rodas5p_fast_observed(
                &problem.problem,
                problem.t_span,
                &problem.y0,
                &config,
                &schedule,
            )
            .map(|r| (r, ControllerTelemetry::default()));
            let on = integrate(problem, &config, ControllerTelemetry::with_trace());
            let mut off_row = result_row(&off, Some(&stand_in));
            let mut on_row = result_row(&on, Some(&stand_in));
            let telemetry = on_row.remove("telemetry").unwrap();
            off_row.remove("telemetry");
            assert_eq!(off_row, on_row, "{} {arm}", problem.id);
            assert_eq!(on_row["success"], json!(true));
            assert_eq!(telemetry["updates"], on_row["attempts"]);
            assert_eq!(
                telemetry["clipped_landings"],
                on_row["output_clipped_steps"]
            );
            assert!(on_row["output_clipped_steps"].as_u64().unwrap() > 0);
            assert_eq!(on_row["grid_states"].as_array().unwrap().len(), GRID_POINTS);
        }
    }
}

/// The references, then every run of every arm.
#[test]
#[ignore = "run export of research/ct01_controller_grid_holdout_20261010; release build; set CT01_RUNS"]
fn export_runs() {
    let mut references = serde_json::Map::new();
    let mut rows = Vec::new();
    let problems = problems();
    let mut reference_states = Vec::new();
    for problem in &problems {
        let (record, states) = reference_record(problem);
        references.insert(problem.id.into(), record);
        reference_states.push(states);
    }
    for (problem, reference) in problems.iter().zip(&reference_states) {
        for seed in SEEDS {
            for rtol in ladder(problem.id) {
                rows.extend(cell_rows(problem, seed, rtol, reference));
            }
        }
    }
    assert_eq!(rows.len(), RUNS);
    write_output(
        "CT01_RUNS",
        &json!({
            "schema": SCHEMA,
            "driver": "integrate_rodas5p_fast_observed_with_telemetry",
            "max_attempts": MAX_ATTEMPTS,
            "grid_points": GRID_POINTS,
            "gated_seeds": GATED_SEEDS,
            "anchor_seed": ANCHOR_SEED,
            "seeds": SEEDS,
            "arms": ARMS,
            "reference": {"arm": "I", "seed": REFERENCE_SEED, "rtol": REFERENCE_RTOL, "check_rtol": CHECK_RTOL},
            "error_metric": ERROR_METRIC,
            "endpoint_metric": ENDPOINT_METRIC,
            "telemetry_events": EVENTS,
            "references": references,
            "rows": rows,
        }),
    );
}
