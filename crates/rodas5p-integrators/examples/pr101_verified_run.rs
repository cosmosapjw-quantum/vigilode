//! Ready-to-run verified PR101 witness; no exact callback enters the solver.
//! cargo run --locked --offline -p rodas5p-integrators --example pr101_verified_run --release
use rodas5p_integrators::{AdaptiveStepConfig, Pr101Model};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Pr101Model::new(-1.0e6, 10.0)?;
    let times: Vec<_> = (0..=100).map(|k| k as f64 / 100.0).collect();
    let config = AdaptiveStepConfig {
        atol: 1.0e-11,
        rtol: 1.0e-8,
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    };
    let run = model.integrate_verified((0.0, 1.0), 0.0, 0.0, &times, &config)?;
    let record = serde_json::json!({
        "schema": "vigilode-pr101-verified-run-v1",
        "model": {"lambda": -1.0e6, "eta": 10.0, "initial_center": 0.0, "initial_radius": 0.0},
        "atol": config.atol, "rtol": config.rtol,
        "success": run.success, "failure": run.failure,
        "attempts": run.attempts, "accepted": run.segments().len(), "rejected": run.rejected_attempts,
        "solver_work": run.solver_work, "verification_work": run.verification_work,
        "output": run.outputs(),
        "bound_contract": "same-curve continuous error <= atol/2; each returned binary64 center radius <= atol",
        "certificate_premises": "identity mass, immutable PR101 model, exact initial value within supplied radius, existing outward-primitive contracts",
        "fair_speed_comparison": false
    });
    println!("{}", serde_json::to_string_pretty(&record)?);
    if !run.success {
        std::process::exit(1);
    }
    Ok(())
}
