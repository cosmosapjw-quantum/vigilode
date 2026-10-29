//! Minimal library use: y_s'=-y_s, y_f'=-1000*y_f, y(0)=(1,1), 0<=t<=1.
//! Time and states are nondimensional. Analytic JVP only, no assembled Jacobian.
//! This demo does not use the audit2 correction or assert general solver readiness.
use std::{io::Write, process::ExitCode, sync::Arc};

use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveStepConfig, DenseOutputError, OdeProblem,
    OutputSamplingPlan, OutputSchedule, integrate_sequential_matrix_free_adaptive_dense_observed,
};

const DEMO_ABSOLUTE_BUDGET: f64 = 1.0e-6;
const DEFAULT_MAX_ATTEMPTS: usize = 10_000;

fn schedule() -> OutputSchedule {
    let mut times = vec![0.0, 1.0e-4, 5.0e-4, 1.0e-3, 2.0e-3, 5.0e-3];
    times.extend((1..=100).map(|i| f64::from(i) / 100.0));
    OutputSchedule::new(times).expect("fixed increasing demo grid")
}

fn solve(
    max_attempts: usize,
    output: OutputSchedule,
) -> Result<AdaptiveObservedIntegrationResult, DenseOutputError> {
    let problem = OdeProblem::new(
        "two-timescale-relaxation",
        2,
        Arc::new(|_t, y, out| {
            out[0] = -y[0];
            out[1] = -1000.0 * y[1];
            Ok(())
        }),
        None, // No batch callback is needed.
        None, // Deliberately no assembled Jacobian callback.
        Some(Arc::new(|_t, _y, v, out| {
            out[0] = -v[0];
            out[1] = -1000.0 * v[1];
            Ok(())
        })),
        None,
        true,
        None,
        None, // Autonomous; identity mass; no exact solution fed to solver.
    )?;
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: 1.0e-8,
        atol: 1.0e-11,
        initial_step: 1.0e-3,
        max_step: 1.0,
        max_attempts,
        ..Default::default()
    };
    integrate_sequential_matrix_free_adaptive_dense_observed(
        &problem,
        (0.0, 1.0),
        &[1.0, 1.0],
        &linear,
        &adaptive,
        &OutputSamplingPlan::dense(output),
    )
}

fn sample_error(result: &AdaptiveObservedIntegrationResult) -> f64 {
    result
        .observed
        .t
        .iter()
        .zip(&result.observed.y)
        .fold(0.0_f64, |worst, (&t, y)| {
            worst
                .max((y[0] - (-t).exp()).abs())
                .max((y[1] - (-1000.0 * t).exp()).abs())
        })
}

fn run_cli() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let max_attempts = match args.as_slice() {
        [] => DEFAULT_MAX_ATTEMPTS,
        [help] if help == "--help" => {
            println!(
                "solve_stiff [--max-attempts N]\nWrites JSON to stdout; a partial or failed solve exits nonzero."
            );
            return Ok(ExitCode::SUCCESS);
        }
        [flag, value] if flag == "--max-attempts" => value.parse::<usize>()?,
        _ => return Err("usage: solve_stiff [--max-attempts N]".into()),
    };
    let grid = schedule();
    let result = solve(max_attempts, grid.clone())?;
    let error = sample_error(&result);
    let complete = result.observed.success && result.observed.t == grid.times();
    let demo_accuracy_pass = complete && error.is_finite() && error <= DEMO_ABSOLUTE_BUDGET;
    let payload = serde_json::json!({
        "problem": "dy/dt = (-y[0], -1000*y[1]); y(0) = (1,1)",
        "method": "existing-sequential-matrix-free-RODAS5P-with-dense-output",
        "audit2_correction_used": false,
        "reference": "analytic exp(-t), exp(-1000*t); not supplied to integrator",
        "rtol": 1.0e-8, "atol": 1.0e-11,
        "success": result.observed.success, "complete_output": complete,
        "message": result.observed.message,
        "max_abs_error_on_returned_samples": error,
        "demo_absolute_budget": DEMO_ABSOLUTE_BUDGET,
        "demo_accuracy_pass": demo_accuracy_pass,
        "scope": "demonstration on one analytically soluble system; not a general certificate",
        "t": result.observed.t, "y": result.observed.y,
        "internal_steps": result.observed.internal_steps,
        "output_clipped_steps": result.observed.output_clipped_steps,
        "counters": result.observed.counters, "diagnostics": result.diagnostics,
    });
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &payload)?;
    stdout.write_all(b"\n")?;
    Ok(if demo_accuracy_pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(code) => code,
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({"success": false, "error": error.to_string()})
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_solution_meets_the_predeclared_demo_budget() {
        let grid = schedule();
        let result = solve(DEFAULT_MAX_ATTEMPTS, grid.clone()).unwrap();
        assert!(result.observed.success, "{}", result.observed.message);
        assert_eq!(result.observed.t, grid.times());
        assert!(sample_error(&result) <= DEMO_ABSOLUTE_BUDGET);
        assert!(result.diagnostics.is_structurally_consistent());
    }

    #[test]
    fn jvp_only_dense_path_does_not_assemble_a_jacobian_or_clip_outputs() {
        let result = solve(DEFAULT_MAX_ATTEMPTS, schedule()).unwrap();
        assert!(result.observed.success);
        assert_eq!(result.observed.counters.jacobian_builds, 0);
        assert_eq!(result.observed.counters.direct_factorizations, 0);
        assert!(result.observed.counters.jvp_vectors > 0);
        assert_eq!(result.observed.output_clipped_steps, 0);
    }

    #[test]
    fn exhausted_attempt_budget_retains_partial_states_and_work() {
        let result = solve(1, schedule()).unwrap();
        assert!(!result.observed.success);
        assert_eq!(result.diagnostics.attempts, 1);
        assert!(result.observed.t.last().unwrap() < &1.0);
        assert!(!result.observed.y.is_empty());
        assert!(result.observed.counters.rhs_evaluations > 0);
        assert!(result.diagnostics.is_structurally_consistent());
    }

    #[test]
    fn zero_attempt_budget_is_rejected_before_execution() {
        assert!(solve(0, schedule()).is_err());
    }

    #[test]
    fn extra_observations_do_not_change_the_adaptive_step_sequence() {
        let first = solve(DEFAULT_MAX_ATTEMPTS, schedule()).unwrap();
        let mut times = schedule().times().to_vec();
        times.push(0.0123);
        times.sort_by(f64::total_cmp);
        let second = solve(DEFAULT_MAX_ATTEMPTS, OutputSchedule::new(times).unwrap()).unwrap();
        assert_eq!(
            first.diagnostics.accepted_step_sizes,
            second.diagnostics.accepted_step_sizes
        );
        assert_eq!(
            first.diagnostics.rejected_step_sizes,
            second.diagnostics.rejected_step_sizes
        );
        assert_eq!(first.observed.counters, second.observed.counters);
        assert!(first.observed.success && second.observed.success);
    }
}
