//! Registered linear long-phase observations; oracle/scoring live outside Rust.
//! Each invocation performs one complete solve. Reference work and serialization
//! are excluded from diagnostic solve time. No solver-wide qualification follows.
use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    integrate_dopri5_observed, integrate_lawson_dopri5_observed, DampedRotationBlock,
    DampedRotationFlow, Dopri5Config, OutputSchedule,
};
use serde_json::{json, Value};
use std::time::Instant;

fn argument(args: &[String], name: &str) -> Result<String, String> {
    args.windows(2)
        .find(|x| x[0] == name)
        .map(|x| x[1].clone())
        .ok_or_else(|| format!("missing {name}"))
}
fn run() -> Result<Value, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6
        || args
            .chunks_exact(2)
            .any(|x| !["--case", "--method", "--tolerance-index"].contains(&x[0].as_str()))
    {
        return Err("exactly --case, --method and --tolerance-index are required".into());
    }
    let case = argument(&args, "--case")?;
    let method = argument(&args, "--method")?;
    let tolerance_index = argument(&args, "--tolerance-index")?;
    let (cycles, frequency, damping) = match case.as_str() {
        "normal_100_f100" => (100_u64, 100.0, 0.0),
        "damped_100_f10000" => (100, 10000.0, 1.0),
        "normal_1000_f1" => (1000, 1.0, 0.0),
        _ => return Err("unregistered case".into()),
    };
    if !["full-rhs-dopri5", "lawson", "direct-carrier"].contains(&method.as_str()) {
        return Err("unregistered method".into());
    }
    let (rtol, atol) = match (method.as_str(), tolerance_index.as_str()) {
        ("direct-carrier", "none") => (0.0, 0.0),
        ("direct-carrier", _) => return Err("direct carrier has no solver tolerance".into()),
        (_, "0") => (1e-6, 1e-8),
        (_, "1") => (1e-8, 1e-10),
        _ => return Err("unregistered tolerance".into()),
    };
    let end = 2.0 * std::f64::consts::PI * cycles as f64 / frequency;
    let times: Vec<f64> = (0..=4096).map(|i| end * i as f64 / 4096.0).collect();
    let schedule = OutputSchedule::new(times.clone()).map_err(|e| e.to_string())?;
    let flow = DampedRotationFlow::repeated(DampedRotationBlock { damping, frequency }, 1)
        .map_err(|e| e.to_string())?;
    let mut config = Dopri5Config::default();
    config.adaptive.rtol = rtol;
    config.adaptive.atol = atol;
    config.adaptive.initial_step = end / 4096.0;
    config.adaptive.max_step = end;
    config.adaptive.max_attempts = 500000;
    let full = |_: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        out[0] = -damping * y[0] - frequency * y[1];
        out[1] = frequency * y[0] - damping * y[1];
        Ok(())
    };
    let residual = |_: f64, _: &[f64], out: &mut [f64]| -> CoreResult<()> {
        out.fill(0.0);
        Ok(())
    };
    let begin = Instant::now();
    let result = if method == "direct-carrier" {
        let mut states = Vec::with_capacity(times.len());
        let mut failure = None;
        let mut actions_attempted = 0;
        for &time in &times {
            actions_attempted += 1;
            let mut state = [0.0; 2];
            if let Err(e) = flow.apply_into(time, &[1.0, 0.0], &mut state) {
                failure = Some(e.to_string());
                break;
            }
            states.push(state);
        }
        let seconds = begin.elapsed().as_secs_f64();
        let observed = states.len();
        json!({"success":failure.is_none() && observed==4097,"failure":failure,
            "times":&times[..observed],"states":states,"last_accepted_time":times[observed.saturating_sub(1)],
            "attempts":observed,"accepted_steps":null,"rejected_steps":0,"counters":null,
            "direct_output_actions_attempted":actions_attempted,"direct_output_actions_completed":observed,"solve_seconds":seconds,"quotient_work":null})
    } else {
        let solved = if method == "full-rhs-dopri5" {
            integrate_dopri5_observed(&full, (0.0, end), &[1.0, 0.0], &config, &schedule)
                .map(|r| (r, None))
        } else {
            integrate_lawson_dopri5_observed(
                &flow,
                &residual,
                (0.0, end),
                &[1.0, 0.0],
                &config,
                &schedule,
            )
            .map(|r| (r.integration, Some(r.quotient_work)))
        };
        let seconds = begin.elapsed().as_secs_f64();
        match solved {
            Err(e) => {
                json!({"success":false,"failure":e.to_string(),"solve_seconds":seconds,"times":[],"states":[]})
            }
            Ok((r, work)) => {
                json!({"success":r.observed.success && r.final_time==end && r.observed.t==times,
                "failure":r.stop_reason,"message":r.observed.message,"times":r.observed.t,"states":r.observed.y,
                "last_accepted_time":r.final_time,"last_accepted_state":r.final_state,
                "attempts":r.attempts,"accepted_steps":r.accepted_steps,"rejected_steps":r.rejected_steps,
                "counters":r.observed.counters,"rhs_completed":r.rhs_completed,"fsal_reuses":r.fsal_reuses,
                "quotient_work":work,"solve_seconds":seconds})
            }
        }
    };
    Ok(
        json!({"schema":"vigilode.frontier-long-phase.v1","case":case,"method":method,
        "cycles":cycles,"frequency":frequency,"damping":damping,"end":end,"threads":1,
        "observation_count":4097,"tolerance_index":tolerance_index,"rtol":rtol,"atol":atol,
        "initial_step":config.adaptive.initial_step,"max_step":config.adaptive.max_step,
        "max_attempts":config.adaptive.max_attempts,"initial_state":[1.0,0.0],
        "timing_authority":"diagnostic_only","timing_scope":"complete integration including workspace/output allocation, all rejected work; excludes model/schedule/reference/serialization",
        "scope":"three linear homogeneous 2D cases, sampled observations only; nonlinear/noncommuting control not supported by exact carrier",
        "result":result}),
    )
}
fn main() {
    match run() {
        Ok(value) => {
            let success = value["result"]["success"].as_bool() == Some(true);
            println!("{value}");
            if !success {
                std::process::exit(1);
            }
        }
        Err(error) => {
            println!(
                "{}",
                json!({"schema":"vigilode.frontier-long-phase.v1","status":"CONFIGURATION_ERROR","error":error})
            );
            std::process::exit(2);
        }
    }
}
