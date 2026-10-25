//! Same-grid analytic diagnostic for catalog case 10, uniform-field gyromotion.
//!
//! This example measures state/error along one DOPRI trajectory. The facade's
//! RODAS5P solve API currently exposes only endpoint observations, so this
//! example does not claim a matched RODAS comparison.

#[path = "local_bottleneck_support/models.rs"]
mod models;

use models::Model;
use serde_json::json;
use vigilode::{Atol, Dopri5, Options, Problem};

const GRID_POINTS: usize = 65;
const END: f64 = 1.0;
const DEFAULT_RTOL: f64 = 1.0e-6;
const DEFAULT_ATOL: f64 = 1.0e-9;
const INITIAL_STEP: f64 = 1.0e-4;
const MAX_STEPS: usize = 200_000;

fn parse_positive_finite_arg(args: &[String], name: &str, default: f64) -> Result<f64, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(default);
    };
    let value = args
        .get(index + 1)
        .ok_or_else(|| format!("missing value after {name}"))?
        .parse::<f64>()
        .map_err(|_| format!("{name} must be numeric"))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be positive and finite"));
    }
    Ok(value)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rtol = parse_positive_finite_arg(&args, "--rtol", DEFAULT_RTOL)?;
    let atol = parse_positive_finite_arg(&args, "--atol", DEFAULT_ATOL)?;
    let model = Model::new(10, false)?;
    let initial = model.initial();
    let exact_start = model.exact(0.0).ok_or("case 10 exact state missing at t=0")?;
    if model.end != END || initial.len() != 6 || exact_start.len() != 6 {
        return Err("case 10 model descriptor differs from the registered standard setup".into());
    }

    let rhs_model = model.clone();
    let problem = Problem::new(6, move |t, y, out| {
        rhs_model.apply(t, y, None, false, out, false);
        Ok(())
    });
    let times: Vec<f64> = (0..GRID_POINTS)
        .map(|i| END * i as f64 / (GRID_POINTS - 1) as f64)
        .collect();
    let options = Options {
        rtol,
        atol: Atol::Scalar(atol),
        h0: INITIAL_STEP,
        max_steps: MAX_STEPS,
        ..Default::default()
    };
    let solution = Dopri5::solve_observed(
        &problem,
        (0.0, END),
        &initial,
        &options,
        &times,
    )
    .map_err(|e| e.to_string())?;

    let gamma = 2.04_f64.sqrt();
    let omega = 500.0 / gamma;
    println!(
        "{}",
        json!({
            "kind": "header",
            "schema": "vigilode.adapter-gyro-grid.v1",
            "case": 10,
            "model_id": "vigilode.local-bottleneck.case10.v1",
            "model_metadata": model.metadata(),
            "parameters": model.parameters(),
            "span": [0.0, END],
            "grid_points": GRID_POINTS,
            "initial_state": initial,
            "rtol": rtol,
            "atol": atol,
            "initial_step": INITIAL_STEP,
            "max_steps": MAX_STEPS,
            "setup_count": 1,
            "solve_count": 1,
            "rhs_calls": solution.stats.counters.rhs_calls,
            "rhs_completed": solution.stats.rhs_completed,
            "attempts": solution.stats.attempts,
            "accepted_steps": solution.stats.accepted_steps,
            "rejected_steps": solution.stats.rejected_steps,
            "success": solution.success,
            "time_grid_matches": solution.t == times,
            "analytic_omega": omega,
            "timing_authority": "none",
            "comparison_scope": "one physical DOPRI trajectory against Model::exact at requested landing times",
            "rodas_comparison": "not included: Rodas5p::solve exposes endpoint observations only"
        })
    );
    if !solution.success || solution.t != times {
        return Err("DOPRI solve failed or did not return the registered time grid".into());
    }

    for (index, (&t, state)) in times.iter().zip(&solution.y).enumerate() {
        let exact = model
            .exact(t)
            .ok_or_else(|| format!("case 10 exact state missing at t={t}"))?;
        let max_abs_error = state
            .iter()
            .zip(&exact)
            .map(|(actual, reference)| (actual - reference).abs())
            .fold(0.0_f64, f64::max);
        let px = state[3];
        let py = state[4];
        let exact_px = exact[3];
        let exact_py = exact[4];
        let amplitude = px.hypot(py);
        let exact_amplitude = exact_px.hypot(exact_py);
        let phase = (-py).atan2(px);
        let reference_phase = omega * t;
        let phase_delta = phase - reference_phase;
        let principal_phase_residual = phase_delta.sin().atan2(phase_delta.cos());
        println!(
            "{}",
            json!({
                "kind": "sample",
                "index": index,
                "t": t,
                "state": state,
                "analytic_exact_state": exact,
                "max_abs_error": max_abs_error,
                "transverse_momentum_amplitude": amplitude,
                "analytic_transverse_momentum_amplitude": exact_amplitude,
                "principal_phase_residual": principal_phase_residual,
                "analytic_reference_phase": reference_phase,
                "phase_residual_semantics": "principal wrapped residual in [-pi, pi], independently at each time; not cumulative or unwrapped"
            })
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("adapter_gyro_grid: {error}");
        std::process::exit(2);
    }
}
