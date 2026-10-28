//! Bounded public-API adversaries registered in
//! `research/reaudit_accuracy_speed_20261010/PREREGISTRATION.md`.
//! No production repair, integration trajectory, or timing claim is made.
//!
//! Source pointers below refer to the frozen source cfed140 (2026-10-10).
//! In particular, the overflow case invokes the public staged solver with
//! the driver's fallback predicate reproduced in its callback. It does not
//! call the private `staged_stage_solve` or claim a full-driver reproduction.

use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters, safe_l2};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, RODAS5P_ESTIMATOR_ORDER,
    adaptive_next_step_after_attempt,
};
use rodas5p_krylov::{
    STAGED_STALL_FACTOR, StagedGmresConfig, StagedGmresOutcome, StagedGmresReport,
    StagedGmresWorkspace, solve_staged_gmres,
};
use serde_json::{Value, json};

#[path = "../../../research/reaudit_accuracy_speed_20261010/gain_witness.rs"]
mod gain_witness;

/// Preserve nonfinite observations explicitly, rather than silently turning
/// them into JSON null. All scientific input vectors in this example are
/// finite; NaN is also used as the untouched-output sentinel.
fn number(x: f64) -> Value {
    if x.is_finite() {
        json!(x)
    } else if x.is_nan() {
        json!("NaN")
    } else if x.is_sign_positive() {
        json!("+Inf")
    } else {
        json!("-Inf")
    }
}

fn vector(x: &[f64]) -> Value {
    Value::Array(x.iter().copied().map(number).collect())
}

fn report_value(report: &StagedGmresReport) -> Value {
    json!({
        "outcome": format!("{:?}", report.outcome),
        "failure": report.failure.map(|v| format!("{v:?}")),
        "accepted": report.accepted(),
        "threshold": number(report.threshold),
        "final_threshold": number(report.final_threshold),
        "residual_norm": number(report.residual_norm),
        "residual_is_finite": report.residual_norm.is_finite(),
        "projected_residual": number(report.projected_residual),
        "right_norm": number(report.right_norm),
        "columns": report.columns,
        "cycles": report.cycles,
        "true_residuals": report.true_residuals,
        "confirmations": report.confirmations,
        "failed_confirmations": report.failed_confirmations,
        "budget_exhausted": report.budget_exhausted,
        "guard_abort": report.guard_abort.map(|v| format!("{v:?}")),
        "matvecs": report.matvecs,
        "nu_max": number(report.nu_max),
        "nu_flops": report.nu_flops,
    })
}

fn controller_case(h: f64, error: f64, id: &str) -> Value {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::PredictiveCapped2,
        min_step: 1.0e-150,
        ..AdaptiveStepConfig::default()
    };
    let mut state = AdaptiveControllerState::default();
    let history = adaptive_next_step_after_attempt(
        &mut state,
        &config,
        1.0,
        1.0,
        0.5,
        RODAS5P_ESTIMATOR_ORDER,
        true,
        false,
    );
    let observed = adaptive_next_step_after_attempt(
        &mut state,
        &config,
        h,
        h,
        error,
        RODAS5P_ESTIMATOR_ORDER,
        true,
        false,
    );
    // Independent algebraic evaluation: clamp the sum of logarithms before
    // exponentiation, without forming error^2 or h/h_acc. All inputs > 0.
    let k = RODAS5P_ESTIMATOR_ORDER as f64;
    let log_predictive = config.safety.ln() + h.ln() + (0.5_f64.ln() - 2.0 * error.ln()) / k;
    let predictive = log_predictive
        .clamp(config.min_factor.ln(), config.max_factor.ln())
        .exp();
    let integral = (config.safety.ln() - error.ln() / k)
        .clamp(config.min_factor.ln(), config.max_factor.ln())
        .exp();
    let expected_factor = predictive.min(integral);
    let (next, observed_error) = match observed {
        Ok(v) => (Some(v), None),
        Err(e) => (None, Some(e.to_string())),
    };
    json!({
        "id": id,
        "execution_scope": "public_controller_updates",
        "source": "crates/rodas5p-integrators/src/adaptive.rs:403-413",
        "controller": "PredictiveCapped2",
        "history": {"h": 1.0, "error": 0.5, "update_succeeded": history.is_ok()},
        "trial": {"h": h, "error": error, "order": RODAS5P_ESTIMATOR_ORDER},
        "all_inputs_finite_positive": h.is_finite() && h > 0.0 && error.is_finite() && error > 0.0,
        "squared_error_binary64": number(error * error),
        "expected": {
            "oracle": "logarithmic algebraic factor; not the implementation expression",
            "log_predictive_unclamped": number(log_predictive),
            "factor": number(expected_factor),
            "next_h": number(h * expected_factor)
        },
        "observed": {
            "next_h": next.map(number),
            "factor": next.map(|v| number(v / h)),
            "error": observed_error,
            "matches_log_oracle": next.is_some_and(|v| (v / h - expected_factor).abs() <= 1.0e-12 * expected_factor),
        },
    })
}

fn clipped_controller_case() -> Value {
    let config = AdaptiveStepConfig {
        controller: ControllerKind::PredictiveCapped2,
        ..AdaptiveStepConfig::default()
    };
    let mut state = AdaptiveControllerState::default();
    let mut setup = Vec::new();
    for (h, error, accepted) in [(1.0, 0.5, true), (1.1, 3.0, false)] {
        let result = adaptive_next_step_after_attempt(
            &mut state,
            &config,
            h,
            h,
            error,
            RODAS5P_ESTIMATOR_ORDER,
            accepted,
            false,
        );
        setup.push(match result {
            Ok(next) => {
                json!({"h": h, "error": error, "accepted": accepted, "next_h": number(next)})
            }
            Err(e) => json!({"error": e.to_string()}),
        });
    }
    let pending_before = state.rejection_pending();
    let requested = 1.0;
    let trial = 0.75;
    let error = 0.001;
    let next = adaptive_next_step_after_attempt(
        &mut state,
        &config,
        requested,
        trial,
        error,
        RODAS5P_ESTIMATOR_ORDER,
        true,
        true,
    );
    let observed = match next {
        Ok(h) => {
            json!({"next_h": number(h), "next_over_trial": number(h / trial), "next_grows_from_trial": h > trial})
        }
        Err(e) => json!({"error": e.to_string()}),
    };
    json!({
        "id": "informative_clip_cap_semantics",
        "execution_scope": "public_controller_updates",
        "classification": "policy_contract_gap_not_proved_integration_failure",
        "source": "crates/rodas5p-integrators/src/adaptive.rs:415-420,565-582",
        "setup": setup,
        "trial": {"requested_h": requested, "trial_h": trial, "error": error, "clipped": true},
        "expected": {
            "literal_registered_restoration_next_h": requested,
            "actual_trial_no_growth_interpretation_upper": trial,
            "interpretation": "ALG05 preserves request restoration; factor cap does not imply next_h <= trial_h"
        },
        "observed": observed,
        "pending_before": pending_before,
        "pending_after": state.rejection_pending(),
        "predicted_request_error": number(error * (trial / requested).powf(-(RODAS5P_ESTIMATOR_ORDER as f64))),
    })
}

fn staged_case(id: &str, matrix: &DenseMatrix, rhs: &[f64], mode: &str) -> Value {
    let mut config = StagedGmresConfig::new(rhs.len(), 16, 1.0e-30, 0.0);
    config.stall_rule = mode == "stall";
    config.floor_at_confirmations = mode == "floor";
    let op = DenseOperator::new(matrix.clone()).expect("declared finite square matrix");
    let mut output = vec![f64::NAN; rhs.len()];
    let mut work = WorkCounters::default();
    let result = solve_staged_gmres(
        &op,
        rhs,
        &config,
        None,
        &mut output,
        &mut StagedGmresWorkspace::default(),
        &mut work,
    );
    let observed = match result {
        Ok(report) => {
            let mut independent_residual = None;
            let mut independent_floor = None;
            if report.accepted() {
                let ax = matrix.matvec(&output).expect("declared dimensions");
                let r: Vec<_> = rhs.iter().zip(&ax).map(|(b, a)| b - a).collect();
                let defect: Vec<_> = output.iter().zip(&ax).map(|(x, a)| x - a).collect();
                independent_residual = Some(safe_l2(&r));
                independent_floor = Some(
                    STAGED_STALL_FACTOR * (safe_l2(rhs) + safe_l2(&output) + safe_l2(&defect)),
                );
            }
            json!({
                "report": report_value(&report),
                "output": vector(&output),
                "output_all_finite": output.iter().all(|v| v.is_finite()),
                "independent_true_residual": independent_residual.map(number),
                "independent_attainable_floor": independent_floor.map(number),
                "accepted_above_requested_target": report.accepted() && report.residual_norm > report.threshold,
                "driver_g3_would_charge_category": report.outcome == StagedGmresOutcome::FallbackAccepted,
                "charge_classification_evidence": "source inspection only; no driver invocation",
                "counters": work,
            })
        }
        Err(e) => json!({"error": e.to_string(), "output": vector(&output), "counters": work}),
    };
    json!({
        "id": format!("{id}_{mode}"),
        "execution_scope": "public_staged_solver",
        "matrix_rows": (0..matrix.nrows()).map(|i| (0..matrix.ncols()).map(|j| matrix[(i,j)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "rhs": rhs,
        "config": {"restart": config.restart, "max_columns": config.max_columns, "rtol": config.rtol, "atol": config.atol, "stall_rule": config.stall_rule, "floor_at_confirmations": config.floor_at_confirmations},
        "expected": {
            "identity_control": id == "identity2",
            "criterion": "record all outcomes; any accepted residual above target is admissible only as an explicitly labeled heuristic outcome"
        },
        "source": [
            "crates/rodas5p-krylov/src/gmres_staged.rs:437-453,550-568",
            "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:870-875"
        ],
        "observed": observed,
    })
}

fn overflow_callback_case(scale: f64, id: &str) -> Value {
    let mut matrix = DenseMatrix::zeros(4, 4);
    for j in 0..4 {
        matrix[((j + 1) % 4, j)] = 1.0;
    }
    let op = DenseOperator::new(matrix).expect("cyclic shift");
    let scaled_rhs = [1.5, 1.5, 0.0, 0.0];
    let physical_rhs: Vec<_> = scaled_rhs.iter().map(|b| b * scale).collect();
    let physical_rhs_norm = safe_l2(&physical_rhs);
    let production_rtol = 1.0e-10;
    let production_atol = 1.0e-14_f64;
    // Literal source predicate from matrix_free_fast.rs:810-811,843-849.
    // Copying this arithmetic into the callback is intentional: this is a
    // callback-seam witness, not a direct private-driver execution.
    let production_threshold = production_atol.max(production_rtol * physical_rhs_norm);
    let mut callback_observation = None;
    let mut fallback = |x: &[f64], scaled_residual: &[f64]| {
        let unscaled: Vec<_> = scaled_residual.iter().map(|r| r * scale).collect();
        let residual_norm = safe_l2(&unscaled);
        let literal_accepts = residual_norm <= production_threshold;
        let ratio = safe_l2(scaled_residual) / safe_l2(&scaled_rhs);
        callback_observation = Some(json!({
            "physical_candidate": vector(&x.iter().map(|v| v * scale).collect::<Vec<_>>()),
            "scaled_true_residual": vector(scaled_residual),
            "unscaled_true_residual": vector(&unscaled),
            "unscaled_residual_components_finite": unscaled.iter().all(|v| v.is_finite()),
            "unscaled_residual_norm": number(residual_norm),
            "unscaled_residual_norm_is_finite": residual_norm.is_finite(),
            "literal_driver_predicate_accepts": literal_accepts,
            "finite_checked_predicate_accepts": residual_norm.is_finite() && production_threshold.is_finite() && literal_accepts,
            "stable_relative_residual_uniform_scale": number(ratio),
            "independent_relative_rule_accepts": ratio <= production_rtol,
        }));
        literal_accepts
    };
    let config = StagedGmresConfig::new(1, 1, 1.0e-14, 0.0);
    let mut output = [f64::NAN; 4];
    let mut counters = WorkCounters::default();
    let result = solve_staged_gmres(
        &op,
        &scaled_rhs,
        &config,
        Some(&mut fallback),
        &mut output,
        &mut StagedGmresWorkspace::default(),
        &mut counters,
    );
    let observed = match result {
        Ok(report) => {
            json!({"report": report_value(&report), "scaled_output": vector(&output), "counters": counters})
        }
        Err(e) => json!({"error": e.to_string(), "counters": counters}),
    };
    json!({
        "id": id,
        "execution_scope": "public_staged_solver_with_literal_driver_callback_predicate",
        "direct_driver_reproduction": false,
        "full_ode_trajectory_reproduction": false,
        "source": "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:810-811,843-849",
        "scale": scale,
        "physical_rhs": vector(&physical_rhs),
        "physical_rhs_components_finite": physical_rhs.iter().all(|v| v.is_finite()),
        "physical_rhs_norm": number(physical_rhs_norm),
        "physical_rhs_norm_is_finite": physical_rhs_norm.is_finite(),
        "production_rtol": production_rtol,
        "production_atol": production_atol,
        "production_threshold": number(production_threshold),
        "production_threshold_is_finite": production_threshold.is_finite(),
        "config": {"restart": 1, "max_columns": 1, "scaled_rtol": config.rtol},
        "expected": {
            "relative_rule_accepts": false,
            "uniform_scale_ratio": "sqrt(27/8)/sqrt(9/2) = sqrt(3/4), up to solver rounding",
            "ceiling": "stage fallback predicate only; no full-driver accepted endpoint claim"
        },
        "callback": callback_observation,
        "observed": observed,
    })
}

/// All registered implementation adversaries, with eleven staged-solver
/// configurations (below the frozen maximum of twelve).
pub fn run_repros() -> Value {
    let systems = [
        ("identity2", DenseMatrix::identity(2), vec![1.0, 0.0]),
        (
            "triangular2",
            DenseMatrix::from_rows(&[&[3.1, 0.7], &[0.0, 2.3]]).expect("finite matrix"),
            vec![0.13, 0.29],
        ),
        (
            "triangular4",
            DenseMatrix::from_rows(&[
                &[1.3, 0.31, 0.17, 0.19],
                &[0.0, 2.7, 0.31, 0.17],
                &[0.0, 0.0, 3.1, 0.31],
                &[0.0, 0.0, 0.0, 4.9],
            ])
            .expect("finite matrix"),
            vec![0.7, 1.1, -0.3, 0.9],
        ),
    ];
    let mut stages = Vec::new();
    for (id, matrix, rhs) in &systems {
        for mode in ["strict", "stall", "floor"] {
            stages.push(staged_case(id, matrix, rhs, mode));
        }
    }
    json!({
        "schema": "vigilode.reaudit.implementation-adversaries.v1",
        "nonfinite_encoding": "finite numbers are JSON numbers; nonfinite observations are explicit NaN/+Inf/-Inf strings",
        "source_base_short": "cfed140",
        "protected_source_edited": false,
        "staged_configurations": 11,
        "controller": [
            controller_case(1.0e-100, 1.0e-200, "predictive_squared_error_underflow"),
            controller_case(0.1, 0.5, "predictive_moderate_control"),
            clipped_controller_case()
        ],
        "stage_acceptance": stages,
        "fallback_seam": [
            overflow_callback_case(1.0e308, "overflowing_norms"),
            overflow_callback_case(1.0e100, "finite_norm_control")
        ],
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).ok_or("expected JSON output path")?;
    let document = json!({
        "implementation_adversaries": run_repros(),
        "gain_study": gain_witness::study()
    });
    std::fs::write(output, serde_json::to_vec_pretty(&document)?)?;
    Ok(())
}
