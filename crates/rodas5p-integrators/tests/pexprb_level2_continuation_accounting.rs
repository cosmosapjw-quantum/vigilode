use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
    Pexprb54s4AccountedBudgetedLevel2PrefixOutcome, Pexprb54s4Level2ContinuationOutcome,
    pexprb54s4_fused_step_resume_level2, pexprb54s4_fused_step_resume_level2_accounted,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
    pexprb54s4_level2_prefix_resume_level1,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted,
};

fn controlled_square_problem(
    fail_endpoint: Arc<AtomicBool>,
    successful_jvps_before_failure: Arc<AtomicU64>,
    observed_jvp_calls: Arc<AtomicU64>,
) -> OdeProblem {
    OdeProblem::new(
        "pexprb-level2-accounted-square",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y: &[f64], v: &[f64], out: &mut [f64]| {
            observed_jvp_calls.fetch_add(1, Ordering::SeqCst);
            if fail_endpoint.load(Ordering::SeqCst)
                && successful_jvps_before_failure
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                        remaining.checked_sub(1)
                    })
                    .is_err()
            {
                return Err(CoreError::LinearSolve(
                    "deterministic endpoint failure".into(),
                ));
            }
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn rhs_sentinel_square_problem(
    rhs_armed: Arc<AtomicBool>,
    rhs_calls: Arc<AtomicU64>,
) -> OdeProblem {
    OdeProblem::new(
        "pexprb-level2-no-prefix-recompute-square",
        1,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            rhs_calls.fetch_add(1, Ordering::SeqCst);
            if rhs_armed.load(Ordering::SeqCst) {
                return Err(CoreError::NonlinearSolve(
                    "retained prefix RHS was recomputed".into(),
                ));
            }
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn phi_config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 12,
        dimension_increment: 1,
        relative_tolerance: 1e-11,
        absolute_tolerance: 1e-14,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

fn retained_level2(problem: &OdeProblem) -> rodas5p_integrators::Pexprb54s4Level2Prefix {
    let level1 = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
        problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
    )
    .unwrap();
    pexprb54s4_level2_prefix_resume_level1(level1, &ParallelExecution::sequential()).unwrap()
}

fn assert_exact_round_trip(
    prefix: WorkCounters,
    continuation: WorkCounters,
    cumulative: WorkCounters,
) {
    assert_eq!(cumulative.checked_delta(prefix), Some(continuation));
    let mut recomposed = prefix;
    recomposed.accumulate(continuation);
    assert_eq!(recomposed, cumulative);
}

#[test]
fn accounted_success_matches_the_compatibility_wrapper_and_round_trips_work() {
    let fail = Arc::new(AtomicBool::new(false));
    let allowance = Arc::new(AtomicU64::new(0));
    let calls = Arc::new(AtomicU64::new(0));
    let problem = controlled_square_problem(fail, allowance, calls);
    let execution = ParallelExecution::sequential();

    let expected =
        pexprb54s4_fused_step_resume_level2(retained_level2(&problem), &execution).unwrap();
    let outcome =
        pexprb54s4_fused_step_resume_level2_accounted(retained_level2(&problem), &execution)
            .unwrap();

    match outcome {
        Pexprb54s4Level2ContinuationOutcome::Complete { report, ledger } => {
            assert_eq!(*report, expected);
            assert_eq!(report.work, ledger.cumulative_work);
            assert!(ledger.continuation_work.phi_actions > 0);
            assert!(ledger.continuation_work.jvp_vectors > 0);
            assert_exact_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        Pexprb54s4Level2ContinuationOutcome::Failed { error, .. } => {
            panic!("unexpected accounted continuation failure: {error}")
        }
        Pexprb54s4Level2ContinuationOutcome::BudgetExhausted { .. } => {
            panic!("unbounded accounted continuation exhausted a budget")
        }
    }
}

#[test]
fn accounted_resume_never_recomputes_the_retained_prefix_rhs() {
    let rhs_armed = Arc::new(AtomicBool::new(false));
    let rhs_calls = Arc::new(AtomicU64::new(0));
    let problem = rhs_sentinel_square_problem(Arc::clone(&rhs_armed), Arc::clone(&rhs_calls));
    let prefix = retained_level2(&problem);
    let calls_after_prefix = rhs_calls.load(Ordering::SeqCst);
    assert!(calls_after_prefix > 0);
    rhs_armed.store(true, Ordering::SeqCst);

    let outcome =
        pexprb54s4_fused_step_resume_level2_accounted(prefix, &ParallelExecution::sequential())
            .unwrap();

    match outcome {
        Pexprb54s4Level2ContinuationOutcome::Complete { ledger, .. } => {
            assert_eq!(rhs_calls.load(Ordering::SeqCst), calls_after_prefix);
            assert_eq!(ledger.continuation_work.rhs_calls, 0);
            assert_eq!(ledger.continuation_work.rhs_batch_calls, 0);
            assert_eq!(ledger.continuation_work.rhs_evaluations, 0);
            assert_exact_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        Pexprb54s4Level2ContinuationOutcome::Failed { error, .. } => {
            panic!("retained-prefix continuation unexpectedly failed: {error}")
        }
        Pexprb54s4Level2ContinuationOutcome::BudgetExhausted { .. } => {
            panic!("unbounded retained-prefix continuation exhausted a budget")
        }
    }
}

#[test]
fn accounted_failure_charges_all_completed_work_across_endpoint_actions() {
    let fail = Arc::new(AtomicBool::new(false));
    let allowance = Arc::new(AtomicU64::new(1));
    let calls = Arc::new(AtomicU64::new(0));
    let problem = controlled_square_problem(Arc::clone(&fail), allowance, calls);
    let prefix = retained_level2(&problem);
    let prefix_work = prefix.report().cumulative_work;
    fail.store(true, Ordering::SeqCst);

    let outcome =
        pexprb54s4_fused_step_resume_level2_accounted(prefix, &ParallelExecution::sequential())
            .unwrap();

    match outcome {
        Pexprb54s4Level2ContinuationOutcome::Failed { error, ledger } => {
            assert_eq!(
                error.to_string(),
                "linear solve failed: deterministic endpoint failure"
            );
            assert_eq!(ledger.prefix_work, prefix_work);
            assert_eq!(ledger.continuation_work.phi_actions, 2);
            assert_eq!(ledger.continuation_work.jvp_calls, 1);
            assert_eq!(ledger.continuation_work.jvp_vectors, 1);
            assert_exact_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        Pexprb54s4Level2ContinuationOutcome::Complete { .. } => {
            panic!("deterministic endpoint failure unexpectedly completed")
        }
        Pexprb54s4Level2ContinuationOutcome::BudgetExhausted { .. } => {
            panic!("unbounded deterministic endpoint failure exhausted a budget")
        }
    }
}

#[test]
fn accounted_prefix_failure_preserves_completed_work_and_legacy_api_returns_err() {
    let fail = Arc::new(AtomicBool::new(true));
    let allowance = Arc::new(AtomicU64::new(0));
    let calls = Arc::new(AtomicU64::new(0));
    let problem = controlled_square_problem(
        Arc::clone(&fail),
        Arc::clone(&allowance),
        Arc::clone(&calls),
    );

    let outcome = pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget_accounted(
        &problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
        80,
    )
    .unwrap();
    match outcome {
        Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Failed(report) => {
            assert_eq!(
                report.error.to_string(),
                "linear solve failed: deterministic endpoint failure"
            );
            assert_eq!(report.work.rhs_evaluations, 1);
            assert_eq!(report.work.phi_actions, 1);
            assert_eq!(report.work.jvp_calls, 0);
            assert_eq!(report.work.jvp_vectors, 0);
        }
        Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::Complete(_) => {
            panic!("injected prefix failure unexpectedly completed")
        }
        Pexprb54s4AccountedBudgetedLevel2PrefixOutcome::BudgetExhausted(_) => {
            panic!("non-budget prefix failure was misclassified as budget exhaustion")
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let legacy = pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget(
        &problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
        80,
    );
    assert!(legacy.is_err());
}

#[test]
fn compatibility_resume_preserves_sequential_fail_fast_callback_behavior() {
    let fail = Arc::new(AtomicBool::new(false));
    let allowance = Arc::new(AtomicU64::new(0));
    let calls = Arc::new(AtomicU64::new(0));
    let problem = controlled_square_problem(
        Arc::clone(&fail),
        Arc::clone(&allowance),
        Arc::clone(&calls),
    );
    let prefix = retained_level2(&problem);
    let calls_after_prefix = calls.load(Ordering::SeqCst);
    fail.store(true, Ordering::SeqCst);

    let result = pexprb54s4_fused_step_resume_level2(prefix, &ParallelExecution::sequential());

    assert!(result.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), calls_after_prefix + 1);
}
