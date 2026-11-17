//! Measurement programs of loop node V07 (speculative Jacobian, stage a).
//!
//! Every test here is ignored and asserts nothing about the numbers: run one
//! at a time with
//! `-- --ignored --exact <name> --nocapture --test-threads=1`.
//! Each prints one `SPECJ {json}` line per record, a header line first and a
//! footer line last. Floating-point values that enter a result are printed
//! as the sixteen hexadecimal digits of their IEEE-754 bits.
//! `research/production_loop_20261116/speculative_jacobian/build_results.py`
//! builds `RESULTS.json` from these lines; the protocol is `PREDECLARED.json`
//! in the same directory.
//!
//! * `measure_model_probes`: the right-hand side of every model at three
//!   states, for the comparison with the Python definitions of the
//!   references.
//! * `measure_adaptive`: the adaptive set at three tolerances; arms
//!   protected, exact, g8, g7, lagged and the two tolerance-jitter controls;
//!   in the exact arm the Jacobian differences of the speculative points.
//! * `measure_fixed_step`: the order set at sixteen constant step sizes;
//!   arms protected, exact, g8, g7, lagged; the Jacobian differences.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, rodas5p_coefficients};
use rodas5p_integrators::stage_clients::speculative::{
    SPECULATIVE_JACOBIAN_STAGE_A_ID, SpeculationMode, SpeculationStats, SpeculativeJacobianClient,
    StageArgumentRecorder,
};
use rodas5p_integrators::stage_clients::{DenseLuMode, DenseLuStageClient};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, ControllerTelemetry, DriverPartial,
    RODAS5P_FAST_CLIENT_DRIVER_ID, RODAS5P_FAST_DRIVER_ID, Rodas5pFastResult,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
    integrate_rodas5p_fast_observed_with_atol, integrate_rodas5p_fast_observed_with_telemetry,
};
use serde_json::{Value, json};

#[path = "stage_clients_speculative_problems.rs"]
mod problems;
use problems::{
    FIXED_STEP_TOLERANCE, Model, ORDER_STEPS, RTOLS, adaptive_models, order_models, probe_state,
};

/// The cell whose per-step Jacobian differences are logged in full
/// (`PREDECLARED.json`, E2).
const SERIES_CELL: (&str, f64) = ("brusselator-1d-32", 1.0e-6);

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hex_vec(v: &[f64]) -> Vec<String> {
    v.iter().copied().map(hex).collect()
}

fn emit(record: Value) {
    println!("SPECJ {record}");
}

fn header(program: &str) {
    let coefficients = rodas5p_coefficients().unwrap();
    emit(json!({
        "kind": "header",
        "program": program,
        "client_id": SPECULATIVE_JACOBIAN_STAGE_A_ID,
        "protected_driver": RODAS5P_FAST_DRIVER_ID,
        "client_driver": RODAS5P_FAST_CLIENT_DRIVER_ID,
        "package_version": env!("CARGO_PKG_VERSION"),
        "stages": coefficients.stages(),
        "c": coefficients.c,
        "gamma_bits": hex(coefficients.gamma),
        "rtols": RTOLS,
        "order_steps": ORDER_STEPS,
        "fixed_step_tolerance": FIXED_STEP_TOLERANCE,
    }));
}

fn footer(program: &str, records: usize) {
    emit(json!({
        "kind": "footer",
        "program": program,
        "records": records,
        "complete": true,
    }));
}

/// One run of one arm.
struct Outcome {
    result: Option<Rodas5pFastResult>,
    partial: Option<DriverPartial>,
    /// The error of a run the driver ended with `Err`.
    error: Option<String>,
    stats: Option<SpeculationStats>,
    /// The smallest and the largest accepted step, where the arm exposes
    /// them.
    h_range: Option<(f64, f64)>,
}

impl Outcome {
    fn success(&self) -> bool {
        self.result
            .as_ref()
            .is_some_and(|result| result.observed.success)
    }

    /// The fields of a run record.
    fn fields(&self) -> Value {
        let mut fields = json!({
            "success": self.success(),
            "error": self.error,
        });
        let map = fields.as_object_mut().unwrap();
        if let Some(result) = &self.result {
            let end = result.observed.y.last().map(Vec::as_slice).unwrap_or(&[]);
            map.insert("attempts".into(), json!(result.attempts));
            map.insert("accepted".into(), json!(result.accepted_steps));
            map.insert("rejected".into(), json!(result.rejected_steps));
            map.insert("jacobian_reuses".into(), json!(result.jacobian_reuses));
            map.insert("driver".into(), json!(result.driver));
            map.insert("message".into(), json!(result.observed.message));
            map.insert(
                "counters".into(),
                serde_json::to_value(result.observed.counters).unwrap(),
            );
            map.insert(
                "t_end_bits".into(),
                json!(result.observed.t.last().copied().map(hex)),
            );
            map.insert("y_end_bits".into(), json!(hex_vec(end)));
        }
        if let Some(partial) = &self.partial {
            map.insert(
                "partial".into(),
                json!({
                    "t_bits": hex(partial.t),
                    "reason": format!("{:?}", partial.reason),
                    "last_failure": partial.last_failure.map(|kind| format!("{kind:?}")),
                }),
            );
        }
        if let Some(stats) = &self.stats {
            map.insert(
                "linearizations".into(),
                json!({
                    "total": stats.linearizations,
                    "speculative": stats.speculative,
                    "exact_by_mode": stats.exact_by_mode,
                    "exact_first": stats.exact_first,
                    "exact_after_failure": stats.exact_after_failure,
                    "exact_no_point": stats.exact_no_point,
                    "client_attempts": stats.attempts,
                }),
            );
        }
        if let Some((smallest, largest)) = self.h_range {
            map.insert("h_min_bits".into(), json!(hex(smallest)));
            map.insert("h_max_bits".into(), json!(hex(largest)));
        }
        fields
    }
}

/// `base` with the fields of `extra` added.
fn merged(base: &Value, extra: Value) -> Value {
    let mut record = base.clone();
    let map = record.as_object_mut().unwrap();
    for (key, value) in extra.as_object().unwrap() {
        map.insert(key.clone(), value.clone());
    }
    record
}

/// The protected dense driver on the plain problem.
fn run_protected(model: &Model, config: &AdaptiveStepConfig) -> Outcome {
    let atol = vec![config.atol; model.n];
    match integrate_rodas5p_fast_observed_with_atol(
        &model.problem(),
        model.t_span,
        &model.y0,
        config,
        &model.end_points(),
        &atol,
    ) {
        Ok((result, partial)) => Outcome {
            result: Some(result),
            partial,
            error: None,
            stats: None,
            h_range: None,
        },
        Err(error) => Outcome {
            result: None,
            partial: None,
            error: Some(error.to_string()),
            stats: None,
            h_range: None,
        },
    }
}

/// The protected dense driver with its controller trace (scalar `atol`,
/// which is the uniform vector bit for bit): the accepted steps.
fn run_protected_traced(model: &Model, config: &AdaptiveStepConfig) -> Outcome {
    match integrate_rodas5p_fast_observed_with_telemetry(
        &model.problem(),
        model.t_span,
        &model.y0,
        config,
        &model.end_points(),
        ControllerTelemetry::with_trace(),
    ) {
        Ok((result, telemetry)) => {
            let accepted: Vec<f64> = telemetry
                .trace
                .unwrap_or_default()
                .iter()
                .filter(|decision| decision.accepted)
                .map(|decision| decision.trial_h)
                .collect();
            let h_range = (!accepted.is_empty()).then(|| {
                (
                    accepted.iter().copied().fold(f64::INFINITY, f64::min),
                    accepted.iter().copied().fold(0.0, f64::max),
                )
            });
            Outcome {
                result: Some(result),
                partial: None,
                error: None,
                stats: None,
                h_range,
            }
        }
        Err(error) => Outcome {
            result: None,
            partial: None,
            error: Some(error.to_string()),
            stats: None,
            h_range: None,
        },
    }
}

/// What the probe measured after one accepted step.
#[derive(Clone, Copy)]
struct ProbeStep {
    t: f64,
    h: f64,
    /// Frobenius norm of the Jacobian at the accepted state.
    norm_j: f64,
    /// Frobenius norms of `J(point) - J(accepted)` for g7, g8 and the
    /// previous accepted state.
    e7: f64,
    e8: f64,
    e_lag: f64,
    /// The distance of the point from the accepted state: WRMS in
    /// tolerance units, and largest component.
    d7_tol: f64,
    d8_tol: f64,
    d7_inf: f64,
    d8_inf: f64,
    d_lag_inf: f64,
}

/// The Jacobian differences of the speculative points, evaluated after
/// every accepted step of the exact arm with the model's own Jacobian
/// (never through the recording wrapper, never charged to the run).
struct JacobianProbe<'m> {
    model: &'m Model,
    atol: f64,
    rtol: f64,
    accepted: DenseMatrix,
    other: DenseMatrix,
    t_previous: f64,
    y_previous: Vec<f64>,
    slot: Vec<f64>,
    steps: Vec<ProbeStep>,
    /// Accepted steps for which the recorder held no complete attempt.
    missing: usize,
}

fn frobenius(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum::<f64>().sqrt()
}

fn frobenius_difference(a: &DenseMatrix, b: &DenseMatrix) -> f64 {
    a.as_slice()
        .iter()
        .zip(b.as_slice())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

fn largest_difference(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .fold(0.0_f64, |m, (x, y)| m.max((x - y).abs()))
}

impl<'m> JacobianProbe<'m> {
    fn new(model: &'m Model, atol: f64, rtol: f64) -> Self {
        Self {
            model,
            atol,
            rtol,
            accepted: DenseMatrix::zeros(model.n, model.n),
            other: DenseMatrix::zeros(model.n, model.n),
            t_previous: model.t_span.0,
            y_previous: model.y0.clone(),
            slot: vec![0.0; model.n],
            steps: Vec::new(),
            missing: 0,
        }
    }

    /// WRMS of `point - accepted` in the weights `atol + rtol |accepted|`.
    fn tolerance_distance(&self, point: &[f64], accepted: &[f64]) -> f64 {
        let sum: f64 = point
            .iter()
            .zip(accepted)
            .map(|(p, a)| {
                let z = (p - a) / (self.atol + self.rtol * a.abs());
                z * z
            })
            .sum();
        (sum / accepted.len() as f64).sqrt()
    }

    fn observe(&mut self, recorder: &StageArgumentRecorder, view: &AcceptedStepView<'_>) {
        let stages = recorder.stages();
        (self.model.fill)(view.t, view.y, &mut self.accepted);
        let norm_j = frobenius(self.accepted.as_slice());
        let mut at_stage = [(0.0, 0.0, 0.0); 2];
        let mut complete = true;
        for (index, stage) in [stages - 1, stages].into_iter().enumerate() {
            match recorder.completed_stage_argument_into(stage, &mut self.slot) {
                Some(time) => {
                    (self.model.fill)(time, &self.slot, &mut self.other);
                    at_stage[index] = (
                        frobenius_difference(&self.other, &self.accepted),
                        self.tolerance_distance(&self.slot, view.y),
                        largest_difference(&self.slot, view.y),
                    );
                }
                None => complete = false,
            }
        }
        (self.model.fill)(self.t_previous, &self.y_previous, &mut self.other);
        let e_lag = frobenius_difference(&self.other, &self.accepted);
        let d_lag_inf = largest_difference(&self.y_previous, view.y);
        if complete {
            self.steps.push(ProbeStep {
                t: view.t,
                h: view.h_used,
                norm_j,
                e7: at_stage[0].0,
                e8: at_stage[1].0,
                e_lag,
                d7_tol: at_stage[0].1,
                d8_tol: at_stage[1].1,
                d7_inf: at_stage[0].2,
                d8_inf: at_stage[1].2,
                d_lag_inf,
            });
        } else {
            self.missing += 1;
        }
        self.t_previous = view.t;
        self.y_previous.copy_from_slice(view.y);
    }

    /// Minimum, quartiles and maximum of `value` over the steps.
    fn quantiles(&self, value: impl Fn(&ProbeStep) -> f64) -> Value {
        let mut values: Vec<f64> = self.steps.iter().map(value).collect();
        if values.is_empty() {
            return Value::Null;
        }
        values.sort_by(f64::total_cmp);
        let at = |q: f64| values[((values.len() - 1) as f64 * q).round() as usize];
        json!({
            "min": at(0.0),
            "q25": at(0.25),
            "median": at(0.5),
            "q75": at(0.75),
            "max": at(1.0),
        })
    }

    /// The summary record of a run.
    fn summary(&self) -> Value {
        json!({
            "steps_probed": self.steps.len(),
            "steps_without_recorded_attempt": self.missing,
            "h": self.quantiles(|s| s.h),
            "norm_j": self.quantiles(|s| s.norm_j),
            "e7_abs": self.quantiles(|s| s.e7),
            "e8_abs": self.quantiles(|s| s.e8),
            "e_lag_abs": self.quantiles(|s| s.e_lag),
            "e7_rel": self.quantiles(|s| s.e7 / s.norm_j),
            "e8_rel": self.quantiles(|s| s.e8 / s.norm_j),
            "e_lag_rel": self.quantiles(|s| s.e_lag / s.norm_j),
            "d7_tolerance_units": self.quantiles(|s| s.d7_tol),
            "d8_tolerance_units": self.quantiles(|s| s.d8_tol),
            "d7_inf": self.quantiles(|s| s.d7_inf),
            "d8_inf": self.quantiles(|s| s.d8_inf),
            "d_lag_inf": self.quantiles(|s| s.d_lag_inf),
        })
    }
}

/// The client driver with the speculative client in `mode` on the problem
/// behind a fresh recorder; `probe` observes every accepted step.
fn run_client(
    model: &Model,
    lu: DenseLuMode,
    mode: SpeculationMode,
    config: &AdaptiveStepConfig,
    mut probe: Option<&mut JacobianProbe<'_>>,
) -> Outcome {
    let (problem, recorder) = model.recorded_problem();
    let client = SpeculativeJacobianClient::new(
        DenseLuStageClient::new(model.n, lu).unwrap(),
        mode,
        Arc::clone(&recorder),
    )
    .unwrap();
    let handle = client.handle();
    let atol = vec![config.atol; model.n];
    let (mut smallest, mut largest) = (f64::INFINITY, 0.0_f64);
    let mut control = |view: &AcceptedStepView<'_>| {
        smallest = smallest.min(view.h_used);
        largest = largest.max(view.h_used);
        if let Some(probe) = probe.as_deref_mut() {
            probe.observe(&recorder, view);
        }
        AcceptControl::Continue
    };
    let run = integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &problem,
        Box::new(client),
        model.t_span,
        &model.y0,
        config,
        &model.end_points(),
        &atol,
        &mut control,
    );
    let h_range = (largest > 0.0).then_some((smallest, largest));
    let stats = Some(handle.snapshot());
    match run {
        Ok((result, partial, _client)) => Outcome {
            result: Some(result),
            partial,
            error: None,
            stats,
            h_range,
        },
        Err(error) => Outcome {
            result: None,
            partial: None,
            error: Some(error.to_string()),
            stats,
            h_range,
        },
    }
}

/// Whether the exact arm is the protected arm bit for bit: success, step
/// statistics, every counter and the end state.
fn same_run(protected: &Outcome, exact: &Outcome) -> bool {
    match (&protected.result, &exact.result) {
        (Some(a), Some(b)) => {
            a.observed.success == b.observed.success
                && a.attempts == b.attempts
                && a.accepted_steps == b.accepted_steps
                && a.rejected_steps == b.rejected_steps
                && a.jacobian_reuses == b.jacobian_reuses
                && a.observed.counters == b.observed.counters
                && hex_vec(&a.observed.t) == hex_vec(&b.observed.t)
                && a.observed.y.len() == b.observed.y.len()
                && a.observed
                    .y
                    .iter()
                    .zip(&b.observed.y)
                    .all(|(x, y)| hex_vec(x) == hex_vec(y))
        }
        _ => false,
    }
}

const SPECULATIVE_ARMS: [SpeculationMode; 3] = [
    SpeculationMode::G8,
    SpeculationMode::G7,
    SpeculationMode::LaggedOneStep,
];

#[test]
#[ignore = "measurement program: run alone with --ignored --nocapture --test-threads=1"]
fn measure_model_probes() {
    header("model_probes");
    let mut records = 0;
    let sets = [("adaptive", adaptive_models()), ("order", order_models())];
    for (set, models) in &sets {
        for model in models {
            for seed in 0..3 {
                let (t, y) = if seed == 0 {
                    (model.t_span.0, model.y0.clone())
                } else {
                    (model.t_span.0 + 0.1 * seed as f64, probe_state(model, seed))
                };
                let mut f = vec![0.0; model.n];
                (model.rhs)(t, &y, &mut f).unwrap();
                let jacobian = model.jacobian(t, &y);
                emit(json!({
                    "kind": "model_probe",
                    "set": set,
                    "problem": model.id,
                    "n": model.n,
                    "seed": seed,
                    "t_bits": hex(t),
                    "y_bits": hex_vec(&y),
                    "f_bits": hex_vec(&f),
                    "jacobian_bits": hex_vec(jacobian.as_slice()),
                }));
                records += 1;
            }
        }
    }
    footer("model_probes", records);
}

#[test]
#[ignore = "measurement program: run alone with --ignored --nocapture --test-threads=1"]
fn measure_adaptive() {
    header("adaptive");
    let mut records = 0;
    for model in &adaptive_models() {
        let lu = model.lu_mode();
        for rtol in RTOLS {
            let config = model.adaptive_config(rtol);
            let base = json!({
                "kind": "adaptive",
                "problem": model.id,
                "n": model.n,
                "rtol": rtol,
                "rtol_bits": hex(rtol),
                "atol": config.atol,
                "atol_bits": hex(config.atol),
                "t0": model.t_span.0,
                "t_end": model.t_span.1,
                "inner_lu": format!("{lu:?}"),
            });
            let protected = run_protected(model, &config);
            emit(merged(
                &merged(&base, json!({"arm": "protected"})),
                protected.fields(),
            ));
            records += 1;

            let mut probe = JacobianProbe::new(model, config.atol, rtol);
            let exact = run_client(model, lu, SpeculationMode::Exact, &config, Some(&mut probe));
            emit(merged(
                &merged(
                    &base,
                    json!({
                        "arm": "exact",
                        "exact_equals_protected": same_run(&protected, &exact),
                    }),
                ),
                exact.fields(),
            ));
            records += 1;
            emit(merged(
                &merged(&base, json!({"kind": "jdiff_adaptive"})),
                probe.summary(),
            ));
            records += 1;
            if model.id == SERIES_CELL.0 && rtol == SERIES_CELL.1 {
                for (index, step) in probe.steps.iter().enumerate() {
                    emit(json!({
                        "kind": "jdiff_series",
                        "problem": model.id,
                        "rtol": rtol,
                        "step": index + 1,
                        "t": step.t,
                        "h": step.h,
                        "norm_j": step.norm_j,
                        "e7_abs": step.e7,
                        "e8_abs": step.e8,
                        "e_lag_abs": step.e_lag,
                        "d7_tolerance_units": step.d7_tol,
                        "d8_tolerance_units": step.d8_tol,
                        "d7_inf": step.d7_inf,
                        "d8_inf": step.d8_inf,
                        "d_lag_inf": step.d_lag_inf,
                    }));
                    records += 1;
                }
            }

            for mode in SPECULATIVE_ARMS {
                let outcome = run_client(model, lu, mode, &config, None);
                emit(merged(
                    &merged(&base, json!({"arm": mode.label()})),
                    outcome.fields(),
                ));
                records += 1;
            }

            // Tolerance-jitter controls of the protected arm (E4).
            for (arm, jitter_rtol) in [
                ("protected_rtol_up", rtol * 1.02),
                ("protected_rtol_down", rtol / 1.02),
            ] {
                let jitter = model.adaptive_config(jitter_rtol);
                let outcome = run_protected(model, &jitter);
                emit(merged(
                    &merged(
                        &base,
                        json!({
                            "arm": arm,
                            "jitter_rtol": jitter_rtol,
                            "jitter_rtol_bits": hex(jitter_rtol),
                            "jitter_atol": jitter.atol,
                            "jitter_atol_bits": hex(jitter.atol),
                        }),
                    ),
                    outcome.fields(),
                ));
                records += 1;
            }
        }
    }
    footer("adaptive", records);
}

#[test]
#[ignore = "measurement program: run alone with --ignored --nocapture --test-threads=1"]
fn measure_fixed_step() {
    header("fixed_step");
    let mut records = 0;
    for model in &order_models() {
        let lu = model.lu_mode();
        for steps in ORDER_STEPS {
            let config = model.fixed_step_config(steps);
            let h = config.max_step;
            let base = json!({
                "kind": "fixed",
                "problem": model.id,
                "n": model.n,
                "steps": steps,
                "h": h,
                "h_bits": hex(h),
                "t0": model.t_span.0,
                "t_end": model.t_span.1,
                "tolerance": FIXED_STEP_TOLERANCE,
                "inner_lu": format!("{lu:?}"),
            });
            let protected = run_protected_traced(model, &config);
            emit(merged(
                &merged(&base, json!({"arm": "protected"})),
                protected.fields(),
            ));
            records += 1;

            // The probe's tolerance units are not meaningful here; its
            // absolute norms are.
            let mut probe = JacobianProbe::new(model, 1.0, 0.0);
            let exact = run_client(model, lu, SpeculationMode::Exact, &config, Some(&mut probe));
            emit(merged(
                &merged(
                    &base,
                    json!({
                        "arm": "exact",
                        "exact_equals_protected": same_run(&protected, &exact),
                    }),
                ),
                exact.fields(),
            ));
            records += 1;
            emit(merged(
                &merged(&base, json!({"kind": "jdiff_fixed"})),
                probe.summary(),
            ));
            records += 1;

            for mode in SPECULATIVE_ARMS {
                let outcome = run_client(model, lu, mode, &config, None);
                emit(merged(
                    &merged(&base, json!({"arm": mode.label()})),
                    outcome.fields(),
                ));
                records += 1;
            }
        }
    }
    footer("fixed_step", records);
}
