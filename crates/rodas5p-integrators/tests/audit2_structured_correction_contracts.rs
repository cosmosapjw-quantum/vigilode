#![cfg(feature = "audit2-research")]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LinearSolverConfig, WorkCounters, safe_l2};
use rodas5p_integrators::audit2_research::{
    AUDIT2_STRUCTURE_PROJECTION_TOLERANCE, Audit2ComparisonOutcome, Audit2CorrectionBackend,
    Audit2CorrectionComparison, Audit2CorrectionOutcome, Audit2FailurePhase,
    Audit2OriginalTargetAccuracyDisposition, Audit2OriginalTargetBridgeComparison,
    Audit2OriginalTargetBridgeOutcome, Audit2ResearchConfig, audit2_original_residual_bridge,
    compare_audit2_original_target_bridge, compare_audit2_research_corrections,
    run_audit2_research_correction,
};
use rodas5p_integrators::{
    OdeProblem, build_step_context, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, sequential_stages,
};

type Jvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type StateKey = (u64, Vec<u64>);

fn scalar_problem(
    name: &str,
    mass: Option<f64>,
    jacobian_value: f64,
    rhs_value: f64,
    jvp: Option<Jvp>,
) -> OdeProblem {
    let jacobian = Arc::new(move |_: f64, _: &[f64]| DenseMatrix::from_rows(&[&[jacobian_value]]));
    OdeProblem::new(
        name,
        1,
        Arc::new(move |_, _, out| {
            out[0] = rhs_value;
            Ok(())
        }),
        None,
        Some(jacobian),
        jvp,
        None,
        true,
        mass.map(|value| DenseMatrix::from_rows(&[&[value]]).unwrap()),
        None,
    )
    .unwrap()
}

fn exact_jvp(value: f64) -> Jvp {
    Arc::new(move |_, _, input, output| {
        output[0] = value * input[0];
        Ok(())
    })
}

fn perturbed_trial_stages<'a>(
    problem: &'a OdeProblem,
    y0: &[f64],
    h: f64,
    magnitude: f64,
) -> (rodas5p_integrators::StepContext<'a>, Vec<Vec<f64>>) {
    let mut setup = WorkCounters::default();
    let context = build_step_context(problem, 0.0, y0, h, &mut setup).unwrap();
    let mut stages = sequential_stages(&context, &LinearSolverConfig::default(), None, &mut setup)
        .unwrap()
        .stages;
    let n = problem.dimension;
    for (i, row) in stages.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value += magnitude * ((i * n + j + 1) as f64).sin();
        }
    }
    (context, stages)
}

fn original_only_stage_key(
    context: &rodas5p_integrators::StepContext<'_>,
    stages: &[Vec<f64>],
) -> (usize, f64, f64) {
    let original_states: Vec<f64> = (0..context.coeffs.stages())
        .map(|stage| {
            let mut state = 0.0;
            for (alpha, row) in context.coeffs.alpha.row(stage).iter().zip(stages) {
                if *alpha != 0.0 {
                    state += alpha * row[0];
                }
            }
            state
        })
        .collect();
    let projected_states: Vec<f64> = (0..context.coeffs.stages())
        .map(|stage| {
            let mut state = 0.0;
            for (previous, row) in stages.iter().enumerate().take(stage) {
                let alpha = context.coeffs.alpha[(stage, previous)];
                if alpha != 0.0 {
                    state += alpha * row[0];
                }
            }
            state
        })
        .collect();
    (0..context.coeffs.stages())
        .find_map(|stage| {
            let time_bits = (context.t + context.coeffs.c[stage] * context.h).to_bits();
            let original_bits = original_states[stage].to_bits();
            let absent_from_projected = (0..context.coeffs.stages()).all(|candidate| {
                (context.t + context.coeffs.c[candidate] * context.h).to_bits() != time_bits
                    || projected_states[candidate].to_bits() != original_bits
            });
            (original_bits != projected_states[stage].to_bits() && absent_from_projected)
                .then_some((stage, original_states[stage], projected_states[stage]))
        })
        .expect("official decimal tableau must expose a projected original-state difference")
}

fn completed_comparison(outcome: Audit2ComparisonOutcome) -> Audit2CorrectionComparison {
    match outcome {
        Audit2ComparisonOutcome::Completed(report) => *report,
        Audit2ComparisonOutcome::Failed(failure) => {
            panic!("shared comparison preparation failed: {failure:?}")
        }
    }
}

fn completed_original_bridge(
    outcome: Audit2OriginalTargetBridgeOutcome,
) -> Audit2OriginalTargetBridgeComparison {
    match outcome {
        Audit2OriginalTargetBridgeOutcome::Completed(report) => *report,
        Audit2OriginalTargetBridgeOutcome::Failed(failure) => {
            panic!("shared original-target bridge preparation failed: {failure:?}")
        }
    }
}

fn assert_condition_aware_agreement(report: &Audit2CorrectionComparison) {
    assert_eq!(report.independent_validation_apply_attempts, 2);
    assert_eq!(report.independent_validation_apply_completed, 2);
    assert_eq!(report.independent_validation_counters.diagnostic_matvecs, 2);
    let condition = report
        .target_condition_f
        .expect("finite full-target condition estimate required");
    assert!(condition.is_finite());
    let full_eta = report
        .full_target_backward_error
        .expect("finite oracle backward error required");
    let common_eta = report
        .common_w_backward_error
        .expect("finite common-W backward error required");
    assert!(full_eta <= 4096.0 * f64::EPSILON, "{full_eta:e}");
    assert!(common_eta <= 4096.0 * f64::EPSILON, "{common_eta:e}");
    if let Some(relative) = report.state_relative_difference {
        assert!(
            relative <= 8192.0 * f64::EPSILON * condition,
            "relative={relative:e}, condition={condition:e}"
        );
    }
}

fn assert_success_accounting(outcome: &Audit2CorrectionOutcome) {
    let success = outcome
        .completed()
        .expect("common-W correction must complete");
    let work = &success.work;
    assert_eq!(work.common_w_setup_attempts, 1);
    assert_eq!(work.common_w_setup_completed, 1);
    assert_eq!(work.factorization_attempts, 1);
    assert_eq!(work.factorization_completed, 1);
    assert_eq!(work.solve_attempts, 8);
    assert_eq!(work.solve_completed, 8);
    assert_eq!(work.correction_jvp_attempts, 14);
    assert_eq!(work.correction_jvp_completed, 14);
    assert_eq!(work.linear_diagnostic_apply_attempts, 1);
    assert_eq!(work.linear_diagnostic_apply_completed, 1);
    assert_eq!(work.diagnostic_shifted_apply_attempts, 8);
    assert_eq!(work.diagnostic_shifted_apply_completed, 8);
    assert_eq!(work.diagnostic_jvp_attempts, 14);
    assert_eq!(work.diagnostic_jvp_completed, 14);
    assert_eq!(work.nonlinear_residual_after_attempts, 1);
    assert_eq!(work.nonlinear_residual_after_completed, 1);
    assert_eq!(work.counters.direct_factorizations, 1);
    assert_eq!(work.counters.direct_solve_calls, 8);
    assert_eq!(work.counters.diagnostic_matvecs, 8);
    assert_eq!(work.counters.jvp_vectors, 52);
}

fn assert_original_target_diagnostic(report: &Audit2OriginalTargetBridgeComparison) {
    assert!(report.matching_trial_stage_states);
    assert_eq!(
        report.accuracy_disposition,
        Audit2OriginalTargetAccuracyDisposition::BudgetNotSpecified
    );
    let diagnostic = report
        .original_target
        .completed()
        .expect("original-target diagnostic must complete");
    assert!(diagnostic.original_target_condition_f.is_finite());
    assert!(
        diagnostic.original_oracle_backward_error <= 4096.0 * f64::EPSILON,
        "oracle eta={:e}",
        diagnostic.original_oracle_backward_error
    );
    let common_eta = diagnostic
        .common_w_original_backward_error
        .expect("common-W original-target backward error required");
    assert!(
        common_eta <= 4096.0 * f64::EPSILON,
        "common eta={common_eta:e}"
    );
    if let Some(relative) = diagnostic.common_w_original_state_relative_difference {
        assert!(
            relative <= 8192.0 * f64::EPSILON * diagnostic.original_target_condition_f,
            "relative={relative:e}, condition={:e}",
            diagnostic.original_target_condition_f
        );
    }
    let bridge = diagnostic
        .bridge
        .as_ref()
        .expect("completed common-W arm needs bridge decomposition");
    let bridge_scale = safe_l2(&bridge.rho_projected)
        + safe_l2(&bridge.jacobian_difference_action)
        + safe_l2(&bridge.residual_difference)
        + safe_l2(&bridge.rho_original_direct);
    assert!(
        bridge.identity_error_l2 <= 4096.0 * f64::EPSILON * bridge_scale.max(1.0),
        "identity error={:e}, scale={bridge_scale:e}",
        bridge.identity_error_l2
    );
    assert_eq!(
        diagnostic.projected_residual.len(),
        diagnostic.original_residual.len()
    );
    assert!(
        diagnostic
            .projected_residual
            .iter()
            .chain(&diagnostic.original_residual)
            .flatten()
            .all(|value| value.is_finite())
    );
    assert!(
        diagnostic
            .common_w_output_projection
            .as_ref()
            .unwrap()
            .iter()
            .chain(&diagnostic.original_oracle_output_projection)
            .chain(
                diagnostic
                    .common_w_embedded_error_projection
                    .as_ref()
                    .unwrap(),
            )
            .chain(&diagnostic.original_oracle_embedded_error_projection)
            .all(|value| value.is_finite())
    );
    assert!(
        diagnostic
            .output_projection_absolute_difference_l2
            .unwrap()
            .is_finite()
    );
    assert!(
        diagnostic
            .embedded_projection_absolute_difference_l2
            .unwrap()
            .is_finite()
    );

    let work = &diagnostic.work;
    assert_eq!(work.original_residual_attempts, 1);
    assert_eq!(work.original_residual_completed, 1);
    assert_eq!(work.original_snapshot_attempts, 1);
    assert_eq!(work.original_snapshot_completed, 1);
    assert_eq!(work.original_target_setup_attempts, 1);
    assert_eq!(work.original_target_setup_completed, 1);
    assert_eq!(work.factorization_attempts, 1);
    assert_eq!(work.factorization_completed, 1);
    assert_eq!(work.original_solve_attempts, 1);
    assert_eq!(work.original_solve_completed, 1);
    assert_eq!(work.condition_estimate_attempts, 1);
    assert_eq!(work.condition_estimate_completed, 1);
    assert!(work.condition_solve_attempts > 0);
    assert_eq!(
        work.condition_solve_attempts,
        work.condition_solve_completed
    );
    assert_eq!(work.projected_diagnostic_apply_attempts, 1);
    assert_eq!(work.projected_diagnostic_apply_completed, 1);
    assert_eq!(work.original_diagnostic_apply_attempts, 2);
    assert_eq!(work.original_diagnostic_apply_completed, 2);
    assert_eq!(work.bridge_reconstruction_attempts, 1);
    assert_eq!(work.bridge_reconstruction_completed, 1);
    assert_eq!(work.output_projection_attempts, 2);
    assert_eq!(work.output_projection_completed, 2);
    assert_eq!(work.embedded_projection_attempts, 2);
    assert_eq!(work.embedded_projection_completed, 2);
    assert_eq!(work.counters.direct_factorizations, 1);
    assert_eq!(
        work.counters.direct_solve_calls,
        1 + work.condition_solve_completed
    );
    assert_eq!(work.counters.diagnostic_matvecs, 3);
}

#[test]
fn original_target_bridge_sign_formula_rejects_wrong_sign_mutant() {
    // Production defect caught: using `+ (r_o-r_p)` in the bridge would report
    // a false original-target residual even when all four actions are correct.
    let projected_residual = [2.0, 3.0];
    let projected_image = [5.0, 7.0];
    let original_residual = [11.0, 13.0];
    let original_image = [17.0, 19.0];
    let report = audit2_original_residual_bridge(
        &projected_image,
        &projected_residual,
        &original_image,
        &original_residual,
    )
    .unwrap();

    assert_eq!(report.rho_projected, vec![3.0, 4.0]);
    assert_eq!(report.jacobian_difference_action, vec![12.0, 12.0]);
    assert_eq!(report.residual_difference, vec![9.0, 10.0]);
    assert_eq!(report.rho_original_direct, vec![6.0, 6.0]);
    assert_eq!(report.rho_original_decomposed, report.rho_original_direct);
    assert_eq!(report.identity_error_l2, 0.0);

    let wrong_sign: Vec<f64> = report
        .rho_projected
        .iter()
        .zip(&report.jacobian_difference_action)
        .zip(&report.residual_difference)
        .map(|((&rho_p, &delta_a_z), &delta_r)| rho_p + delta_a_z + delta_r)
        .collect();
    let wrong_sign_gap = safe_l2(
        &wrong_sign
            .iter()
            .zip(&report.rho_original_direct)
            .map(|(mutant, direct)| mutant - direct)
            .collect::<Vec<_>>(),
    );
    assert!(wrong_sign_gap > 20.0, "wrong-sign gap={wrong_sign_gap:e}");
    println!(
        "AUDIT2_SIGN_MUTANT {}",
        serde_json::json!({
            "correct_identity_error_l2": report.identity_error_l2,
            "wrong_sign_gap_l2": wrong_sign_gap,
            "mutant_rejected": true
        })
    );
}

#[test]
fn full_target_is_default_and_common_w_requires_explicit_opt_in() {
    assert_eq!(
        Audit2ResearchConfig::default().backend,
        Audit2CorrectionBackend::FullTargetOracle
    );
    let (problem, y0) = manufactured_vector_problem(4, 50.0, 5.0, 0.1, 0.0).unwrap();
    let (context, stages) = perturbed_trial_stages(&problem, &y0, 0.01, 1e-5);
    let default =
        run_audit2_research_correction(&context, &stages, Audit2ResearchConfig::default());
    assert_eq!(
        default.completed().unwrap().backend,
        Audit2CorrectionBackend::FullTargetOracle
    );
    let explicit = run_audit2_research_correction(
        &context,
        &stages,
        Audit2ResearchConfig {
            backend: Audit2CorrectionBackend::CommonWBlockForward,
        },
    );
    assert_eq!(
        explicit.completed().unwrap().backend,
        Audit2CorrectionBackend::CommonWBlockForward
    );
}

#[test]
fn original_target_bridge_matches_condition_aware_full_oracle() {
    for n in [4, 8, 16] {
        for h in [0.001, 0.01, 0.05, 0.1] {
            let (problem, y0) = manufactured_vector_problem(n, 50.0, 5.0, 0.1, 0.0).unwrap();
            let (context, stages) = perturbed_trial_stages(&problem, &y0, h, 1e-5);
            let bridge =
                completed_original_bridge(compare_audit2_original_target_bridge(&context, &stages));
            let report = &bridge.projected;
            assert!(report.matching_trial_stage_states);
            assert!(report.projection.result_independent_fixed_rule);
            assert!(report.projection.projected_structure_bit_exact);
            assert_eq!(
                report.projection.tolerance,
                AUDIT2_STRUCTURE_PROJECTION_TOLERANCE
            );
            assert!(report.projection.max_alpha_forbidden_abs <= report.projection.tolerance);
            assert!(report.projection.max_gamma_upper_abs <= report.projection.tolerance);
            assert!(report.projection.max_gamma_diagonal_error_abs <= report.projection.tolerance);
            assert!(
                report.projection.projected_alpha_entries
                    + report.projection.projected_gamma_entries
                    > 0
            );
            assert_condition_aware_agreement(report);
            assert_success_accounting(&report.common_w);
            assert_original_target_diagnostic(&bridge);
            let original = bridge.original_target.completed().unwrap();
            // Fixed before looking at any result: retain one compact raw-vector
            // example by input coordinates, not by an extremum or outcome.
            let raw_vectors = (n == 4 && h == 0.01).then(|| {
                let decomposition = original.bridge.as_ref().unwrap();
                serde_json::json!({
                    "selection_rule": "n == 4 && h == 0.01",
                    "projected_residual": original.projected_residual,
                    "original_residual": original.original_residual,
                    "rho_projected": decomposition.rho_projected,
                    "jacobian_difference_action": decomposition.jacobian_difference_action,
                    "residual_difference": decomposition.residual_difference,
                    "rho_original_direct": decomposition.rho_original_direct,
                    "rho_original_decomposed": decomposition.rho_original_decomposed,
                    "common_w_output_projection": original.common_w_output_projection,
                    "original_oracle_output_projection": original.original_oracle_output_projection,
                    "common_w_embedded_error_projection": original.common_w_embedded_error_projection,
                    "original_oracle_embedded_error_projection": original.original_oracle_embedded_error_projection
                })
            });
            println!(
                "AUDIT2_ORIGINAL_TARGET_BRIDGE {}",
                serde_json::json!({
                    "n": n,
                    "h": h,
                    "condition_f": report.target_condition_f,
                    "oracle_backward_error": report.full_target_backward_error,
                    "common_w_backward_error": report.common_w_backward_error,
                    "state_absolute_difference_l2": report.state_absolute_difference_l2,
                    "state_relative_difference": report.state_relative_difference,
                    "original_condition_f": original.original_target_condition_f,
                    "original_oracle_backward_error": original.original_oracle_backward_error,
                    "common_w_original_backward_error": original.common_w_original_backward_error,
                    "original_state_relative_difference": original.common_w_original_state_relative_difference,
                    "bridge_identity_error_l2": original.bridge.as_ref().map(|value| value.identity_error_l2),
                    "residual_difference_l2": safe_l2(
                        &original.original_residual.iter().flatten().zip(
                            original.projected_residual.iter().flatten()
                        ).map(|(a,b)| a-b).collect::<Vec<_>>()
                    ),
                    "output_projection_absolute_difference_l2": original.output_projection_absolute_difference_l2,
                    "embedded_projection_absolute_difference_l2": original.embedded_projection_absolute_difference_l2,
                    "projection": report.projection,
                    "common_w_work": report.common_w.completed().unwrap().work,
                    "original_target_work": original.work,
                    "accuracy_disposition": bridge.accuracy_disposition,
                    "raw_vectors": raw_vectors,
                    "production_activation": false
                })
            );
        }
    }
}

#[test]
fn nonsingular_nonidentity_mass_and_strong_nonnormality_are_supported() {
    let (problem, y0, mass, linear) =
        manufactured_mass_nonlinear_problem(1_000.0, 50.0, 20.0, 0.0).unwrap();
    let determinant = mass[(0, 0)] * mass[(1, 1)] - mass[(0, 1)] * mass[(1, 0)];
    assert!(determinant.abs() > 1.0);
    assert_ne!(mass, DenseMatrix::identity(2));
    assert!(linear[(0, 1)].abs() > 100.0 * linear[(1, 0)].abs());
    let (context, stages) = perturbed_trial_stages(&problem, &y0, 1e-4, 1e-7);
    let bridge =
        completed_original_bridge(compare_audit2_original_target_bridge(&context, &stages));
    let report = &bridge.projected;
    assert_condition_aware_agreement(report);
    assert_success_accounting(&report.common_w);
    assert_original_target_diagnostic(&bridge);
    let original = bridge.original_target.completed().unwrap();
    assert_eq!(
        report
            .common_w
            .completed()
            .unwrap()
            .work
            .counters
            .mass_matvecs,
        16
    );
    println!(
        "AUDIT2_MASS_NONNORMAL {}",
        serde_json::json!({
            "mass_determinant": determinant,
            "nonnormal_off_diagonal_ratio": linear[(0, 1)].abs()/linear[(1, 0)].abs(),
            "condition_f": report.target_condition_f,
            "common_w_backward_error": report.common_w_backward_error,
            "state_relative_difference": report.state_relative_difference,
            "original_condition_f": original.original_target_condition_f,
            "common_w_original_backward_error": original.common_w_original_backward_error,
            "original_state_relative_difference": original.common_w_original_state_relative_difference,
            "bridge_identity_error_l2": original.bridge.as_ref().map(|value| value.identity_error_l2),
            "original_target_work": original.work,
            "nonlinear_residual_before": report.common_w.completed().unwrap().initial_residual_l2,
            "nonlinear_residual_after": report.common_w.completed().unwrap().nonlinear_residual_after_l2
        })
    );
}

#[test]
fn original_action_failure_retains_projected_arms_and_attempted_work() {
    let sentinel: Arc<Mutex<Option<StateKey>>> = Arc::new(Mutex::new(None));
    let observed = sentinel.clone();
    let zero = DenseMatrix::zeros(1, 1);
    let problem = OdeProblem::new(
        "audit2-original-action-failure",
        1,
        Arc::new(move |time, state, out| {
            let key = (
                time.to_bits(),
                state.iter().map(|value| value.to_bits()).collect(),
            );
            if observed.lock().unwrap().as_ref() == Some(&key) {
                return Err(CoreError::LinearSolve(
                    "injected original-target RHS failure".into(),
                ));
            }
            out[0] = 1.0;
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| Ok(zero.clone()))),
        Some(exact_jvp(0.0)),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages: Vec<Vec<f64>> = (0..context.coeffs.stages())
        .map(|stage| vec![1.0e12 * (stage + 1) as f64])
        .collect();
    let (stage, original_state, projected_state) = original_only_stage_key(&context, &stages);
    assert_ne!(original_state.to_bits(), projected_state.to_bits());
    *sentinel.lock().unwrap() = Some((
        (context.t + context.coeffs.c[stage] * context.h).to_bits(),
        vec![original_state.to_bits()],
    ));

    let bridge =
        completed_original_bridge(compare_audit2_original_target_bridge(&context, &stages));
    assert!(bridge.projected.full_target.completed().is_some());
    assert!(bridge.projected.common_w.completed().is_some());
    let failure = bridge
        .original_target
        .failed()
        .expect("actual original residual path must retain the injected failure");
    assert_eq!(failure.phase, Audit2FailurePhase::OriginalResidual);
    assert_eq!(failure.work.original_residual_attempts, 1);
    assert_eq!(failure.work.original_residual_completed, 0);
    assert_eq!(failure.work.original_snapshot_attempts, 0);
    assert_eq!(failure.work.original_target_setup_attempts, 0);
    assert_eq!(failure.work.factorization_attempts, 0);
    assert_eq!(failure.work.original_solve_attempts, 0);
    assert_eq!(failure.work.counters.block_matvecs, 1);
    assert!(failure.work.counters.jvp_vectors > 0);
    assert_eq!(failure.projected_residual.len(), context.coeffs.stages());
    assert!(failure.partial.original_residual.is_none());
    println!(
        "AUDIT2_ORIGINAL_TARGET_FAILURE {}",
        serde_json::json!({
            "projected_full_target": "completed",
            "projected_common_w": "completed",
            "original_target": "failed",
            "phase": failure.phase,
            "projected_residual_rows_retained": failure.projected_residual.len(),
            "original_residual_retained": failure.partial.original_residual.is_some(),
            "work": failure.work,
            "production_activation": false
        })
    );
}

#[test]
fn original_jacobian_failure_rejects_projected_jacobian_substitution() {
    let sentinel: Arc<Mutex<Option<StateKey>>> = Arc::new(Mutex::new(None));
    let observed = sentinel.clone();
    let problem = OdeProblem::new(
        "audit2-original-jacobian-failure",
        1,
        Arc::new(|_, _, out| {
            out[0] = 1.0;
            Ok(())
        }),
        None,
        Some(Arc::new(move |time, state| {
            let key = (
                time.to_bits(),
                state.iter().map(|value| value.to_bits()).collect(),
            );
            if observed.lock().unwrap().as_ref() == Some(&key) {
                return Err(CoreError::LinearSolve(
                    "injected original-target Jacobian failure".into(),
                ));
            }
            Ok(DenseMatrix::zeros(1, 1))
        })),
        Some(exact_jvp(0.0)),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages: Vec<Vec<f64>> = (0..context.coeffs.stages())
        .map(|stage| vec![1.0e12 * (stage + 1) as f64])
        .collect();
    let (stage, original_state, projected_state) = original_only_stage_key(&context, &stages);
    assert_ne!(original_state.to_bits(), projected_state.to_bits());
    *sentinel.lock().unwrap() = Some((
        (context.t + context.coeffs.c[stage] * context.h).to_bits(),
        vec![original_state.to_bits()],
    ));

    let bridge =
        completed_original_bridge(compare_audit2_original_target_bridge(&context, &stages));
    assert!(bridge.projected.full_target.completed().is_some());
    assert!(bridge.projected.common_w.completed().is_some());
    let failure = bridge
        .original_target
        .failed()
        .expect("actual original Jacobian path must retain the injected failure");
    assert_eq!(failure.phase, Audit2FailurePhase::OriginalTargetAssembly);
    assert_eq!(failure.work.original_residual_attempts, 1);
    assert_eq!(failure.work.original_residual_completed, 1);
    assert_eq!(failure.work.original_snapshot_attempts, 1);
    assert_eq!(failure.work.original_snapshot_completed, 1);
    assert_eq!(failure.work.original_target_setup_attempts, 1);
    assert_eq!(failure.work.original_target_setup_completed, 0);
    assert_eq!(failure.work.factorization_attempts, 0);
    assert!(failure.partial.original_residual.is_some());
    assert!(failure.partial.original_oracle_correction.is_empty());
}

#[test]
fn late_embedded_failure_retains_completed_original_output_and_diagnostics() {
    let problem = scalar_problem(
        "audit2-late-embedded-failure",
        None,
        0.0,
        1.0,
        Some(exact_jvp(0.0)),
    );
    let mut context =
        build_step_context(&problem, 0.0, &[0.0], 1.0, &mut WorkCounters::default()).unwrap();
    context.coeffs.btilde.fill(f64::MAX);
    let stages = vec![vec![0.0]; context.coeffs.stages()];

    let bridge =
        completed_original_bridge(compare_audit2_original_target_bridge(&context, &stages));
    assert!(bridge.projected.full_target.completed().is_some());
    assert!(bridge.projected.common_w.completed().is_some());
    let failure = bridge
        .original_target
        .failed()
        .expect("overflowing embedded projection must remain a typed failure");
    assert_eq!(failure.phase, Audit2FailurePhase::EmbeddedProjection);
    assert_eq!(failure.work.output_projection_attempts, 1);
    assert_eq!(failure.work.output_projection_completed, 1);
    assert_eq!(failure.work.embedded_projection_attempts, 1);
    assert_eq!(failure.work.embedded_projection_completed, 0);
    assert!(failure.partial.original_residual.is_some());
    assert_eq!(
        failure.partial.original_oracle_correction.len(),
        context.coeffs.stages()
    );
    assert!(failure.partial.original_target_condition_f.is_some());
    assert!(failure.partial.original_oracle_backward_error.is_some());
    assert!(
        failure
            .partial
            .original_oracle_output_projection
            .as_ref()
            .is_some_and(|values| values.iter().all(|value| value.is_finite()))
    );
    assert!(
        failure
            .partial
            .original_oracle_embedded_error_projection
            .is_none()
    );
}

#[test]
fn zero_rhs_uses_finite_absolute_criteria_without_zero_over_zero() {
    let problem = scalar_problem("audit2-zero", None, 0.0, 0.0, Some(exact_jvp(0.0)));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    let full = report.full_target.completed().unwrap();
    let common = report.common_w.completed().unwrap();
    assert_eq!(full.initial_residual_l2, 0.0);
    assert_eq!(common.correction_l2, 0.0);
    assert_eq!(common.linear_residual_l2, 0.0);
    assert_eq!(common.nonlinear_residual_after_l2, 0.0);
    assert_eq!(report.full_target_backward_error, Some(0.0));
    assert_eq!(report.common_w_backward_error, Some(0.0));
    assert_eq!(report.state_absolute_difference_l2, Some(0.0));
    assert_eq!(report.state_relative_difference, None);
    assert_success_accounting(&report.common_w);
}

#[test]
fn missing_jvp_fails_only_the_opt_in_arm_before_setup() {
    let problem = scalar_problem("audit2-missing-jvp", None, -1.0, 1.0, None);
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    assert!(report.full_target.completed().is_some());
    let failure = report.common_w.failed().expect("missing JVP must be typed");
    assert_eq!(failure.phase, Audit2FailurePhase::JvpAccess);
    assert_eq!(failure.work.common_w_setup_attempts, 0);
    assert_eq!(failure.work.factorization_attempts, 0);
    assert_eq!(failure.work.solve_attempts, 0);
}

#[test]
fn failed_jvp_preserves_attempt_and_partial_progress() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let observed = attempts.clone();
    let failing: Jvp = Arc::new(move |_, _, _, _| {
        observed.fetch_add(1, Ordering::SeqCst);
        Err(CoreError::LinearSolve("injected JVP failure".into()))
    });
    let problem = scalar_problem("audit2-failed-jvp", None, 0.0, 1.0, Some(failing));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    assert!(report.full_target.completed().is_some());
    let failure = report
        .common_w
        .failed()
        .expect("JVP failure must be retained");
    assert_eq!(failure.phase, Audit2FailurePhase::CorrectionJvp);
    assert_eq!(failure.partial_correction.len(), 1);
    assert_eq!(failure.work.solve_attempts, 1);
    assert_eq!(failure.work.solve_completed, 1);
    assert_eq!(failure.work.correction_jvp_attempts, 1);
    assert_eq!(failure.work.correction_jvp_completed, 0);
    assert_eq!(failure.work.counters.jvp_calls, 1);
    assert_eq!(failure.work.counters.jvp_vectors, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[test]
fn singular_and_overflowing_solves_are_typed_and_counted() {
    for (name, mass) in [("singular", 0.0), ("overflow", 1e-320)] {
        let problem = scalar_problem(name, Some(mass), 0.0, 1.0, Some(exact_jvp(0.0)));
        let context =
            build_step_context(&problem, 0.0, &[0.0], 1.0, &mut WorkCounters::default()).unwrap();
        let stages = vec![vec![0.0]; context.coeffs.stages()];
        let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
        let failure = report
            .common_w
            .failed()
            .expect("unrepresentable solve must fail");
        assert_eq!(failure.phase, Audit2FailurePhase::Solve, "{name}");
        assert_eq!(failure.work.common_w_setup_attempts, 1, "{name}");
        assert_eq!(failure.work.common_w_setup_completed, 1, "{name}");
        assert_eq!(failure.work.factorization_attempts, 1, "{name}");
        assert_eq!(failure.work.factorization_completed, 1, "{name}");
        assert_eq!(failure.work.solve_attempts, 1, "{name}");
        assert_eq!(failure.work.solve_completed, 0, "{name}");
        assert!(failure.partial_correction.is_empty());
        assert!(
            failure.message.contains("LU"),
            "{name}: {}",
            failure.message
        );
    }
}

#[test]
fn overflow_after_a_completed_zero_row_retains_partial_solve_progress() {
    let zero = DenseMatrix::zeros(1, 1);
    let jacobian = zero.clone();
    let problem = OdeProblem::new(
        "audit2-late-overflow",
        1,
        Arc::new(|time, _, out| {
            out[0] = if time == 0.0 { 0.0 } else { f64::MAX };
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| Ok(jacobian.clone()))),
        Some(exact_jvp(0.0)),
        Some(Arc::new(|_, _, out| {
            out[0] = 0.0;
            Ok(())
        })),
        false,
        Some(DenseMatrix::from_rows(&[&[1e-200]]).unwrap()),
        None,
    )
    .unwrap();
    let context =
        build_step_context(&problem, 0.0, &[0.0], 1.0, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    let failure = report.common_w.failed().expect("later row must overflow");
    assert_eq!(failure.phase, Audit2FailurePhase::Solve);
    assert_eq!(failure.partial_correction, vec![vec![0.0]]);
    assert_eq!(failure.work.solve_attempts, 2);
    assert_eq!(failure.work.solve_completed, 1);
    assert_eq!(failure.work.correction_jvp_attempts, 2);
    assert_eq!(failure.work.correction_jvp_completed, 2);
}

#[test]
fn nonfinite_trial_input_fails_before_all_algorithmic_work() {
    let problem = scalar_problem("audit2-nonfinite", None, 0.0, 0.0, Some(exact_jvp(0.0)));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let mut stages = vec![vec![0.0]; context.coeffs.stages()];
    stages[0][0] = f64::NAN;
    let failure = match compare_audit2_research_corrections(&context, &stages) {
        Audit2ComparisonOutcome::Failed(failure) => failure,
        Audit2ComparisonOutcome::Completed(_) => panic!("NaN input was accepted"),
    };
    assert_eq!(failure.phase, Audit2FailurePhase::InputValidation);
    assert_eq!(failure.preparation_counters, WorkCounters::default());
}

#[test]
fn inconsistent_jvp_is_exposed_as_a_domain_counterexample_not_a_pass() {
    let problem = scalar_problem(
        "audit2-inconsistent-jvp",
        None,
        0.0,
        1.0,
        Some(exact_jvp(100.0)),
    );
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let stages = vec![vec![0.0]; context.coeffs.stages()];
    let report = completed_comparison(compare_audit2_research_corrections(&context, &stages));
    let full = report.full_target.completed().unwrap();
    let common = report.common_w.completed().unwrap();
    assert!(
        report.state_absolute_difference_l2.unwrap() > 1e-6,
        "inconsistent JVP must not look oracle-equivalent"
    );
    assert!(
        report.common_w_backward_error.unwrap() > 1e-8,
        "independent full-target residual must expose the mismatch"
    );
    assert!(common.nonlinear_residual_after_l2 > full.nonlinear_residual_after_l2 + 1e-8);
    assert!(common.correction_l2.is_finite());
}

#[test]
fn malformed_shape_is_a_typed_preparation_failure() {
    let problem = scalar_problem("audit2-shape", None, 0.0, 0.0, Some(exact_jvp(0.0)));
    let context =
        build_step_context(&problem, 0.0, &[0.0], 0.1, &mut WorkCounters::default()).unwrap();
    let failure = run_audit2_research_correction(
        &context,
        &[],
        Audit2ResearchConfig {
            backend: Audit2CorrectionBackend::CommonWBlockForward,
        },
    )
    .failed()
    .cloned()
    .expect("shape failure must be retained");
    assert_eq!(failure.phase, Audit2FailurePhase::InputValidation);
    assert!(failure.projection.is_none());
    assert_eq!(failure.work, Default::default());
}
