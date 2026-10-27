//! Runnable deployment example. `--template rhs-only|selective-jvp` prints the
//! complete fixture configuration; `<config.json> <rate>` runs y'=-rate*y.
//! Replace the model closure and independent accuracy oracle for a real client.
use serde_json::json;
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, ProductionConfig, ProductionSolver,
    SwitchingConfig,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--template" {
        // Numerical values describe this explicit smoke fixture, not a
        // universal choice of physical tolerance or operational schedule.
        let options = Options {
            rtol: 1e-7,
            atol: Atol::Scalar(1e-10),
            h0: 1e-4,
            max_steps: 50_000,
            ..Options::default()
        };
        let config = match args[1].as_str() {
            "rhs-only" => ProductionConfig::rhs_only(options, 8),
            "selective-jvp" => ProductionConfig::selective_jvp(
                options,
                SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
                true,
            ),
            _ => return Err("template must be rhs-only or selective-jvp".into()),
        };
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }
    if args.len() != 2 {
        return Err("usage: production_from_json <config.json> <positive-rate> | --template rhs-only|selective-jvp".into());
    }
    let config: ProductionConfig = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let rate: f64 = args[1].parse()?;
    if !(rate.is_finite() && rate > 0.0) {
        return Err("rate must be positive and finite".into());
    }
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let r = rhs.clone();
    let j = jvp.clone();
    let problem = Problem::new(16, move |_, y, out| {
        r.fetch_add(1, Ordering::Relaxed);
        for (d, x) in out.iter_mut().zip(y) {
            *d = -rate * x;
        }
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_, _, v, out| {
        j.fetch_add(1, Ordering::Relaxed);
        for (d, x) in out.iter_mut().zip(v) {
            *d = -rate * x;
        }
        Ok(())
    }))
    .autonomous();
    let result = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0; 16], &config)?;
    let error = result
        .t
        .iter()
        .zip(&result.y)
        .flat_map(|(t, state)| state.iter().map(move |x| (x - (-rate * t).exp()).abs()))
        .fold(0.0, f64::max);
    let accounting = result.stats.accounting_complete
        && result.stats.rhs_calls == rhs.load(Ordering::Relaxed)
        && result.stats.jvp_vectors == jvp.load(Ordering::Relaxed)
        && result.stats.jacobian_builds == 0
        && result.stats.ft_calls == 0;
    let passed = result.success && result.final_time == 1.0 && accounting && error <= 1e-6;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"vigilode.production-smoke.v1", "fixture":"y'=-rate*y, n=16, y(0)=1",
            "rate":rate, "independent_grid_error_cap":1e-6, "maximum_returned_state_error":error,
            "actual_callback_accounting_matches":accounting, "pass":passed,
            "configuration":config, "solution":result,
        }))?
    );
    if !passed {
        return Err("production fixture failed; partial solution is in JSON output".into());
    }
    Ok(())
}
