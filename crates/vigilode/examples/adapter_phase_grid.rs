//! Same-grid physical/chart DOPRI diagnostic for the exponential chirp.
//!
//! Timing fields are single-run diagnostics only. The phase residual is
//! principal-valued at each time and is not unwrapped.

use serde_json::{json, Value};
use std::time::{Duration, Instant};
use vigilode::{Atol, CallbackError, Dopri5, ExponentialChirpChart, Options, Problem};

const END: f64 = 8.0;
const GAMMA: f64 = 0.8;
const BETA: f64 = 1.2;
const OMEGA0: f64 = 1.0;
const EPSILON: f64 = 0.3;
const DEFAULT_PHYSICAL_ATOL: f64 = 1.0e-6;
const MAX_STEPS: usize = 300_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Physical,
    Chart,
    Both,
}

#[derive(Clone, Copy, Debug, Default)]
struct Timings {
    setup: Duration,
    solve: Duration,
    reconstruction: Duration,
}

impl Timings {
    fn total_seconds(self) -> f64 {
        self.setup.as_secs_f64() + self.solve.as_secs_f64() + self.reconstruction.as_secs_f64()
    }
    fn json(self) -> Value {
        json!({
            "setup_seconds": self.setup.as_secs_f64(),
            "solve_seconds": self.solve.as_secs_f64(),
            "reconstruction_seconds": self.reconstruction.as_secs_f64(),
            "total_seconds": self.total_seconds(),
            "calibration_seconds": 0.0,
            "probe_seconds": 0.0,
            "transition_seconds": 0.0
        })
    }
}

struct ArmResult {
    states: Vec<[f64; 2]>,
    timings: Timings,
    rhs_calls: u64,
    rhs_completed: u64,
    attempts: usize,
    accepted: usize,
    rejected: usize,
    success: bool,
    grid_matches: bool,
    chart_atol: Option<f64>,
}

fn cli_value(args: &[String], key: &str, default: &str) -> Result<String, String> {
    match args.iter().position(|arg| arg == key) {
        Some(index) => args
            .get(index + 1)
            .cloned()
            .ok_or_else(|| format!("missing value after {key}")),
        None => Ok(default.to_owned()),
    }
}

fn parse_positive_finite(args: &[String], key: &str, default: f64) -> Result<f64, String> {
    let text = cli_value(args, key, "")?;
    let value = if text.is_empty() {
        default
    } else {
        text.parse::<f64>()
            .map_err(|_| format!("{key} must be numeric"))?
    };
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{key} must be positive and finite"));
    }
    Ok(value)
}

fn parse_args() -> Result<(Mode, usize, f64), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match cli_value(&args, "--mode", "both")?.as_str() {
        "physical" => Mode::Physical,
        "chart" => Mode::Chart,
        "both" => Mode::Both,
        _ => return Err("--mode must be physical, chart, or both".into()),
    };
    let points = cli_value(&args, "--grid-points", "129")?
        .parse::<usize>()
        .map_err(|_| "--grid-points must be an integer")?;
    if !(65..=257).contains(&points) {
        return Err("--grid-points must be in 65..=257".into());
    }
    let physical_atol = parse_positive_finite(&args, "--physical-atol", DEFAULT_PHYSICAL_ATOL)?;
    Ok((mode, points, physical_atol))
}

fn reference_chart() -> Result<ExponentialChirpChart, String> {
    ExponentialChirpChart::new(OMEGA0, BETA, GAMMA, (0.0, END))
}

fn exact_state(
    t: f64,
    chart: &ExponentialChirpChart,
) -> Result<([f64; 2], f64, f64), String> {
    let phase = chart.phase(t)?;
    let radius = (-GAMMA * t).exp()
        / (1.0 + (EPSILON / GAMMA) * (1.0 - (-2.0 * GAMMA * t).exp())).sqrt();
    let omega = chart.frequency(t)?;
    let state = [
        radius * phase.cos(),
        radius * (-GAMMA * phase.cos() - omega * phase.sin()),
    ];
    Ok((state, radius, phase))
}

fn amplitude_phase(
    state: [f64; 2],
    t: f64,
    chart: &ExponentialChirpChart,
) -> Result<(f64, f64), String> {
    let omega = chart.frequency(t)?;
    let p = -(state[1] + GAMMA * state[0]) / omega;
    Ok((state[0].hypot(p), p.atan2(state[0])))
}

fn principal_residual(phase: f64, reference: f64) -> f64 {
    let delta = phase - reference;
    delta.sin().atan2(delta.cos())
}

fn max_error(state: [f64; 2], reference: [f64; 2]) -> f64 {
    (state[0] - reference[0])
        .abs()
        .max((state[1] - reference[1]).abs())
}

fn physical_arm(times: &[f64], physical_atol: f64) -> Result<ArmResult, String> {
    let mut timings = Timings::default();
    let started = Instant::now();
    let problem = Problem::new(2, move |t, y, out| {
        let omega = OMEGA0 * (BETA * t).exp();
        let factor = -EPSILON * (y[0] * y[0] + ((y[1] + GAMMA * y[0]) / omega).powi(2));
        out[0] = y[1] + factor * y[0];
        out[1] = -(2.0 * GAMMA - BETA) * y[1]
            - (omega * omega + GAMMA * GAMMA - GAMMA * BETA) * y[0]
            + factor * y[1];
        Ok(())
    });
    let options = Options {
        rtol: 0.0,
        atol: Atol::Scalar(physical_atol),
        h0: 1.0e-3,
        max_steps: MAX_STEPS,
        ..Default::default()
    };
    timings.setup = started.elapsed();

    let started = Instant::now();
    let solution = Dopri5::solve_observed(
        &problem,
        (0.0, END),
        &[1.0, -GAMMA],
        &options,
        times,
    )
    .map_err(|e| e.to_string())?;
    timings.solve = started.elapsed();

    let started = Instant::now();
    let states = solution.y.iter().map(|y| [y[0], y[1]]).collect();
    timings.reconstruction = started.elapsed();
    Ok(ArmResult {
        states,
        timings,
        rhs_calls: solution.stats.counters.rhs_calls,
        rhs_completed: solution.stats.rhs_completed,
        attempts: solution.stats.attempts,
        accepted: solution.stats.accepted_steps,
        rejected: solution.stats.rejected_steps,
        success: solution.success,
        grid_matches: solution.t == times,
        chart_atol: None,
    })
}

fn chart_arm(times: &[f64], physical_atol: f64) -> Result<ArmResult, String> {
    let mut timings = Timings::default();
    let started = Instant::now();
    let chart = reference_chart()?;
    let chart_for_rhs = chart;
    let problem = Problem::new(2, move |t, y, out| {
        let physical = chart_for_rhs
            .to_physical(t, [y[0], y[1]])
            .map_err(CallbackError::Fatal)?;
        let radius2 = y[0] * y[0] + y[1] * y[1];
        let factor = -EPSILON * radius2;
        let value = chart_for_rhs
            .rotating_rhs(
                t,
                [y[0], y[1]],
                [factor * physical[0], factor * physical[1]],
            )
            .map_err(CallbackError::Fatal)?;
        out.copy_from_slice(&value);
        Ok(())
    });
    let chart_atol = chart.conservative_local_atol(physical_atol)?;
    let options = Options {
        rtol: 0.0,
        atol: Atol::Scalar(chart_atol),
        h0: 1.0e-3,
        max_steps: MAX_STEPS,
        ..Default::default()
    };
    timings.setup = started.elapsed();

    let started = Instant::now();
    let solution = Dopri5::solve_observed(
        &problem,
        (0.0, END),
        &[1.0, 0.0],
        &options,
        times,
    )
    .map_err(|e| e.to_string())?;
    timings.solve = started.elapsed();

    let started = Instant::now();
    let mut states = Vec::with_capacity(solution.y.len());
    for (i, y) in solution.y.iter().enumerate() {
        states.push(
            chart
                .to_physical(times[i], [y[0], y[1]])
                .map_err(|e| format!("physical reconstruction failed: {e}"))?,
        );
    }
    timings.reconstruction = started.elapsed();
    Ok(ArmResult {
        states,
        timings,
        rhs_calls: solution.stats.counters.rhs_calls,
        rhs_completed: solution.stats.rhs_completed,
        attempts: solution.stats.attempts,
        accepted: solution.stats.accepted_steps,
        rejected: solution.stats.rejected_steps,
        success: solution.success,
        grid_matches: solution.t == times,
        chart_atol: Some(chart_atol),
    })
}

fn run() -> Result<(), String> {
    let (mode, points, physical_atol) = parse_args()?;
    let common_setup_started = Instant::now();
    let grid_chart = reference_chart()?;
    let times: Vec<f64> = (0..points)
        .map(|i| END * i as f64 / (points - 1) as f64)
        .collect();
    let common_harness_setup_seconds = common_setup_started.elapsed().as_secs_f64();

    let physical = if matches!(mode, Mode::Physical | Mode::Both) {
        Some(physical_arm(&times, physical_atol)?)
    } else {
        None
    };
    let transformed = if matches!(mode, Mode::Chart | Mode::Both) {
        Some(chart_arm(&times, physical_atol)?)
    } else {
        None
    };

    let timing = |arm: &Option<ArmResult>| arm.as_ref().map(|r| r.timings.json());
    let candidate_total_seconds = physical
        .as_ref()
        .map_or(0.0, |r| r.timings.total_seconds())
        + transformed
            .as_ref()
            .map_or(0.0, |r| r.timings.total_seconds());
    println!(
        "{}",
        json!({
            "kind": "header",
            "schema": "vigilode.adapter-phase-grid.v1",
            "source_equation": "oscillatory_chart_pilot.rs physical-dopri5 and declared-chart-dopri5, epsilon=0.3",
            "equation": "exponential chirp with equivariant cubic residual",
            "parameters": {"omega0": OMEGA0, "beta": BETA, "gamma": GAMMA, "epsilon": EPSILON},
            "span": [0.0, END],
            "grid_points": points,
            "mode": match mode { Mode::Physical => "physical", Mode::Chart => "chart", Mode::Both => "both" },
            "physical_atol": physical_atol,
            "max_steps": MAX_STEPS,
            "chart_conservative_local_atol": transformed.as_ref().and_then(|r| r.chart_atol),
            "rtol": 0.0,
            "physical_setup_count": if physical.is_some() { 1 } else { 0 },
            "chart_setup_count": if transformed.is_some() { 1 } else { 0 },
            "solve_count": (if physical.is_some() { 1 } else { 0 }) + (if transformed.is_some() { 1 } else { 0 }),
            "physical_rhs_calls": physical.as_ref().map(|r| r.rhs_calls),
            "chart_rhs_calls": transformed.as_ref().map(|r| r.rhs_calls),
            "physical_rhs_completed": physical.as_ref().map(|r| r.rhs_completed),
            "chart_rhs_completed": transformed.as_ref().map(|r| r.rhs_completed),
            "physical_attempts": physical.as_ref().map(|r| r.attempts),
            "chart_attempts": transformed.as_ref().map(|r| r.attempts),
            "physical_accepted": physical.as_ref().map(|r| r.accepted),
            "chart_accepted": transformed.as_ref().map(|r| r.accepted),
            "physical_rejected": physical.as_ref().map(|r| r.rejected),
            "chart_rejected": transformed.as_ref().map(|r| r.rejected),
            "physical_success": physical.as_ref().map(|r| r.success),
            "chart_success": transformed.as_ref().map(|r| r.success),
            "time_grid_matches": physical.as_ref().map_or(true, |r| r.grid_matches) && transformed.as_ref().map_or(true, |r| r.grid_matches),
            "timing_authority": "diagnostic_only",
            "timing_scope": "per-candidate setup constructs arm chart/provider/problem/options; solve integrates the full registered grid; reconstruction converts all chart outputs to physical state; analytic references, phase/error metrics, and JSON serialization are excluded",
            "common_harness_setup_seconds": common_harness_setup_seconds,
            "common_harness_setup_scope": "shared comparison/reference chart and registered grid construction; outside per-candidate adapter cost",
            "candidate_total_seconds": candidate_total_seconds,
            "all_attempt_total_seconds": common_harness_setup_seconds + candidate_total_seconds,
            "arm_timings": {"physical": timing(&physical), "chart": timing(&transformed)},
            "calibration_seconds": 0.0,
            "probe_seconds": 0.0,
            "transition_seconds": 0.0,
            "phase_residual_semantics": "principal wrapped residual in [-pi, pi], independently at each time; not cumulative or unwrapped",
            "chart_atol_scope": "conservative local embedded-error control bound; not a global-error certificate"
        })
    );

    for arm in [&physical, &transformed].into_iter().flatten() {
        if !arm.success || !arm.grid_matches || arm.states.len() != points {
            return Err("one selected solve failed or did not return the registered time grid".into());
        }
    }
    for i in 0..points {
        let t = times[i];
        let (reference_state, reference_amplitude, reference_phase) = exact_state(t, &grid_chart)?;
        let physical_state = physical.as_ref().map(|r| r.states[i]);
        let chart_state = transformed.as_ref().map(|r| r.states[i]);
        let physical_metrics = physical_state.map(|state| {
            let (amplitude, phase) = amplitude_phase(state, t, &grid_chart).expect("valid grid time");
            (state, amplitude, principal_residual(phase, reference_phase))
        });
        let chart_metrics = chart_state.map(|state| {
            let (amplitude, phase) = amplitude_phase(state, t, &grid_chart).expect("valid grid time");
            (state, amplitude, principal_residual(phase, reference_phase))
        });
        println!(
            "{}",
            json!({
                "kind": "sample",
                "index": i,
                "t": t,
                "physical_state": physical_metrics.map(|m| m.0),
                "chart_reconstructed_physical_state": chart_metrics.map(|m| m.0),
                "analytic_exact_state": reference_state,
                "physical_max_abs_error": physical_state.map(|s| max_error(s, reference_state)),
                "chart_max_abs_error": chart_state.map(|s| max_error(s, reference_state)),
                "physical_amplitude": physical_metrics.map(|m| m.1),
                "chart_amplitude": chart_metrics.map(|m| m.1),
                "analytic_amplitude": reference_amplitude,
                "physical_principal_phase_residual": physical_metrics.map(|m| m.2),
                "chart_principal_phase_residual": chart_metrics.map(|m| m.2),
                "analytic_reference_phase": reference_phase
            })
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("adapter_phase_grid: {error}");
        std::process::exit(2);
    }
}
