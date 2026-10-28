//! Fixed PR101 inputs/output schedule, separate sampled accuracy and authority.
//! All three arm times include generation and validation; timing is descriptive.
use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, DenseErrorControl, IntegrationMethod, OutputSamplingPlan, OutputSchedule,
    Pr101Model, VerifiedContinuousProblem,
    integrate_adaptive_dense_observed_with_dense_error_control, integrate_verified_residual,
    prothero_robinson_problem,
};
use std::time::Instant;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AdaptiveStepConfig {
        atol: 1e-11,
        rtol: 1e-8,
        initial_step: 0.1,
        max_step: 0.1,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    };
    let times: Vec<_> = (0..=100).map(|k| k as f64 / 100.0).collect();
    let linear = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };
    for control in [DenseErrorControl::Off, DenseErrorControl::Enforce] {
        let started = Instant::now();
        let (problem, initial) = prothero_robinson_problem(-1e6, 10.0, 0.0);
        let sampling = OutputSamplingPlan::dense(OutputSchedule::new(times.clone())?);
        let result = integrate_adaptive_dense_observed_with_dense_error_control(
            &problem,
            (0.0, 1.0),
            &initial,
            IntegrationMethod::Sequential,
            Some(&linear),
            None,
            &cfg,
            &sampling,
            control,
        );
        let elapsed = started.elapsed().as_secs_f64();
        match result {
            Ok((run, report)) => {
                let max_abs = run
                    .observed
                    .t
                    .iter()
                    .zip(&run.observed.y)
                    .map(|(&t, y)| (y[0] - t.sin()).abs())
                    .fold(0.0_f64, f64::max);
                let max_normalized = run
                    .observed
                    .t
                    .iter()
                    .zip(&run.observed.y)
                    .map(|(&t, y)| (y[0] - t.sin()).abs() / (cfg.atol + cfg.rtol * t.sin().abs()))
                    .fold(0.0_f64, f64::max);
                println!(
                    "{}",
                    serde_json::json!({"arm": format!("{control:?}"),
                    "success":run.observed.success,"seconds":elapsed,"sampled_max_abs":max_abs,
                    "sampled_max_normalized":max_normalized,"solver_work":run.observed.counters,
                    "dense_estimator_work":report.counters,"continuous_authority":false,
                    "counter_note":"Enforce estimator costs are already included in solver_work; do not add twice"})
                );
            }
            Err(error) => println!(
                "{}",
                serde_json::json!({"arm":format!("{control:?}"),
                "success":false,"seconds":elapsed,"failure":error.to_string()})
            ),
        }
    }
    let started = Instant::now();
    let owner = VerifiedContinuousProblem::from_pr101(Pr101Model::new(-1e6, 10.0)?)?;
    let run = integrate_verified_residual(&owner, (0.0, 1.0), &[0.0], 0.0, &times, &cfg)?;
    let elapsed = started.elapsed().as_secs_f64();
    let max_abs = run
        .outputs()
        .iter()
        .map(|o| (o.center()[0] - o.time().sin()).abs())
        .fold(0.0_f64, f64::max);
    let max_radius = run
        .outputs()
        .iter()
        .map(|o| o.radius_l2())
        .fold(0.0_f64, f64::max);
    println!(
        "{}",
        serde_json::json!({"arm":"VerifiedResidual","success":run.success,
        "seconds":elapsed,"sampled_max_abs":max_abs,"max_certified_radius":max_radius,
        "atol":cfg.atol,"rtol":cfg.rtol,"solver_work":run.solver_work,
        "verification_work":run.verification_work,"continuous_authority":run.success,
        "measured_speed_promoted":false,"outputs":run.outputs().len(),"failure":run.failure})
    );
    if !run.success {
        std::process::exit(1);
    }
    Ok(())
}
