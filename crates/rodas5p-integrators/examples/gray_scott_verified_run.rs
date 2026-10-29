//! Bounded A11 native trajectory replay; this is not a timing benchmark.
use rodas5p_integrators::gray_scott_verified::gray_scott_2d_verified;
use rodas5p_integrators::{AdaptiveStepConfig, integrate_verified_residual};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let grid = std::env::args()
        .nth(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(4_usize);
    let (owner, mut initial) = gray_scott_2d_verified(grid)?;
    let cells = grid * grid;
    for i in 0..cells {
        initial[i] = 0.7 + 0.01 * (i % 5) as f64;
        initial[cells + i] = 0.2 + 0.005 * (i % 3) as f64;
    }
    let times: Vec<_> = (0..=10).map(|k| k as f64 / 100.0).collect();
    let config = AdaptiveStepConfig {
        atol: 1e-4,
        rtol: 1e-7,
        initial_step: 0.025,
        max_step: 0.025,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    };
    let run = integrate_verified_residual(&owner, (0.0, 0.1), &initial, 0.0, &times, &config)?;
    let outputs: Vec<_> = run
        .outputs()
        .iter()
        .map(|output| {
            serde_json::json!({
                "time": output.time(), "center": output.center(), "radius_l2": output.radius_l2()
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1, "node": "A11", "model": "periodic-gray-scott-represented-constants",
            "grid": grid, "dimension": owner.dimension(), "span": [0.0, 0.1],
            "atol": config.atol, "rtol": config.rtol, "initial_radius": 0.0,
            "certificate": "continuous-residual-full-tube-euclidean-lognorm",
            "success": run.success, "failure": run.failure,
            "accepted": run.segments().len(), "attempts": run.attempts, "rejected": run.rejected_attempts,
            "rhs_evaluations": run.solver_work.rhs_evaluations,
            "verification_work": run.verification_work, "outputs": outputs,
            "timing_claim": "NOT_EVALUATED"
        }))?
    );
    if !run.success {
        std::process::exit(1);
    }
    Ok(())
}
