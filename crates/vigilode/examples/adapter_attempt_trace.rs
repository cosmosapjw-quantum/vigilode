//! Bounded, opt-in attempt trace for the local stiff-control cases 5 and 12.
//!
//! Example: `cargo run --release -p vigilode --example adapter_attempt_trace
//! -- --case 5 --max-attempts 2000`. JSON serialization/printing happens only
//! after `advance_to` returns; the native solver's attempt path stays allocation
//! free when telemetry is enabled.

#[path = "local_bottleneck_support/models.rs"]
mod models;

use models::Model;
use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5AttemptErrorNorm, Dopri5Config, Dopri5Session,
    NativeSessionAdvance, NativeSessionIdentity, NativeSessionMethod, OutputSchedule,
};
use serde_json::json;

const HARD_MAX_ATTEMPTS: usize = 200_000;

fn value(args: &[String], key: &str, default: &str) -> Result<String, String> {
    match args.iter().position(|arg| arg == key) {
        Some(i) => args.get(i + 1).cloned().ok_or_else(|| format!("missing value for {key}")),
        None => Ok(default.to_owned()),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case: usize = value(&args, "--case", "5")?
        .parse()
        .map_err(|_| "--case must be 5 or 12")?;
    if !matches!(case, 5 | 12) {
        return Err("--case must be 5 or 12".into());
    }
    let max_attempts: usize = value(&args, "--max-attempts", "2000")?
        .parse()
        .map_err(|_| "--max-attempts must be a positive integer")?;
    if max_attempts == 0 || max_attempts > HARD_MAX_ATTEMPTS {
        return Err(format!("--max-attempts must be in 1..={HARD_MAX_ATTEMPTS}"));
    }

    let model = Model::new(case, false)?;
    let initial = model.initial();
    let end = model.end;
    let rhs_model = &model;
    let rhs = move |t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        rhs_model.apply(t, y, None, false, out, false);
        Ok(())
    };
    let identity = NativeSessionIdentity::new(NativeSessionMethod::Dopri5, model.n);
    let schedule = OutputSchedule::new(vec![0.0, end]).map_err(|e| e.to_string())?;
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 1.0e-4_f64.min(end / 10.0),
        max_attempts,
        ..AdaptiveStepConfig::default()
    };
    let config = Dopri5Config {
        adaptive,
        ..Dopri5Config::default()
    };
    let mut session = Dopri5Session::new(
        &rhs,
        (0.0, end),
        &initial,
        &config,
        &schedule,
        identity,
    )
    .map_err(|e| e.to_string())?;
    session.set_attempt_telemetry(true);

    println!(
        "{}",
        json!({
            "kind": "header",
            "schema": "vigilode.adapter-attempt-trace.v1",
            "case": case,
            "size": "standard",
            "model_id": format!("vigilode.local-bottleneck.case{case}.v1"),
            "model_metadata": model.metadata(),
            "initial_state": initial,
            "t_span": [0.0, end],
            "rtol": 1.0e-6,
            "atol": 1.0e-9,
            "initial_step": 1.0e-4_f64.min(end / 10.0),
            "max_attempts": max_attempts,
            "timing_authority": "none",
            "trace_state_semantics": "state_before and state_after are full vectors; rejected attempts retain state"
        })
    );

    let mut state_before = vec![0.0; model.n];
    loop {
        let checkpoint = session.checkpoint();
        state_before.copy_from_slice(checkpoint.y);
        let attempts_before = checkpoint.attempts;
        let advance = session
            .advance_to(end, 1, identity)
            .map_err(|e| e.to_string())?;
        let Some(attempt) = session.last_attempt().filter(|_| {
            session.checkpoint().attempts == attempts_before + 1
        }) else {
            match advance {
                NativeSessionAdvance::Complete => break,
                NativeSessionAdvance::Stopped => {
                    let cp = session.checkpoint();
                    println!(
                        "{}",
                        json!({"kind":"terminal","case":case,"t":cp.t,"attempts":cp.attempts,"accepted_steps":cp.accepted_steps,"rejected_steps":cp.rejected_steps})
                    );
                    break;
                }
                NativeSessionAdvance::Yield | NativeSessionAdvance::Checkpoint => continue,
            }
        };
        let checkpoint = session.checkpoint();
        let error_norm = match attempt.error_norm {
            Dopri5AttemptErrorNorm::Finite(value) => json!(value),
            Dopri5AttemptErrorNorm::NonFinite => json!("nonfinite"),
            Dopri5AttemptErrorNorm::Unavailable => json!("unavailable"),
        };
        println!(
            "{}",
            json!({
                "kind": "attempt",
                "case": case,
                "attempt": checkpoint.attempts,
                "t": attempt.t_before,
                "requested_h": attempt.requested_h,
                "trial_h": attempt.trial_h,
                "error_norm": error_norm,
                "accepted": attempt.accepted,
                "clipped": attempt.clipped,
                "t_after": attempt.t_after,
                "rhs_delta": attempt.rhs_delta,
                "state_before": state_before,
                "state_after": checkpoint.y
            })
        );
        if advance == NativeSessionAdvance::Complete {
            break;
        }
        if advance == NativeSessionAdvance::Stopped {
            let cp = session.checkpoint();
            println!(
                "{}",
                json!({"kind":"terminal","case":case,"t":cp.t,"attempts":cp.attempts,"accepted_steps":cp.accepted_steps,"rejected_steps":cp.rejected_steps})
            );
            break;
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("adapter_attempt_trace: {error}");
        std::process::exit(2);
    }
}
