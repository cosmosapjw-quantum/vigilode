use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
    Pexprb54s4Level2ContinuationOutcome, pexprb54s4_fused_step_resume_level2,
    pexprb54s4_fused_step_resume_level2_accounted,
    pexprb54s4_fused_step_resume_level2_accounted_jvp_budget,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
    pexprb54s4_level2_prefix_resume_level1,
};

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

fn counted_square_problem(
    rhs_calls: Arc<AtomicU64>,
    jvp_calls: Arc<AtomicU64>,
    fail_jvp: Arc<AtomicBool>,
) -> OdeProblem {
    OdeProblem::new(
        "pexprb-continuation-budget-square",
        1,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            rhs_calls.fetch_add(1, Ordering::SeqCst);
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y: &[f64], v: &[f64], out: &mut [f64]| {
            jvp_calls.fetch_add(1, Ordering::SeqCst);
            if fail_jvp.load(Ordering::SeqCst) {
                return Err(CoreError::LinearSolve(
                    "injected continuation operator failure".into(),
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

fn assert_round_trip(prefix: WorkCounters, continuation: WorkCounters, cumulative: WorkCounters) {
    assert_eq!(cumulative.checked_delta(prefix), Some(continuation));
    let mut recomposed = prefix;
    recomposed.accumulate(continuation);
    assert_eq!(recomposed, cumulative);
}

fn fresh_problem() -> (OdeProblem, Arc<AtomicU64>, Arc<AtomicU64>, Arc<AtomicBool>) {
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let fail = Arc::new(AtomicBool::new(false));
    (
        counted_square_problem(Arc::clone(&rhs), Arc::clone(&jvp), Arc::clone(&fail)),
        rhs,
        jvp,
        fail,
    )
}

#[test]
fn zero_cap_returns_charged_exhaustion_without_continuation_jvp_or_prefix_recompute() {
    let (problem, rhs_calls, jvp_calls, _) = fresh_problem();
    let prefix = retained_level2(&problem);
    let prefix_work = prefix.report().cumulative_work;
    let rhs_after_prefix = rhs_calls.load(Ordering::SeqCst);
    let jvp_after_prefix = jvp_calls.load(Ordering::SeqCst);

    let outcome = pexprb54s4_fused_step_resume_level2_accounted_jvp_budget(
        prefix,
        &ParallelExecution::sequential(),
        0,
    )
    .unwrap();

    match outcome {
        Pexprb54s4Level2ContinuationOutcome::BudgetExhausted {
            jvp_cap,
            used_jvp_vectors,
            ledger,
        } => {
            assert_eq!(jvp_cap, 0);
            assert_eq!(used_jvp_vectors, 0);
            assert_eq!(ledger.prefix_work, prefix_work);
            assert_eq!(ledger.continuation_work.jvp_vectors, 0);
            assert_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        other => panic!("zero cap must exhaust, got {other:?}"),
    }
    assert_eq!(rhs_calls.load(Ordering::SeqCst), rhs_after_prefix);
    assert_eq!(jvp_calls.load(Ordering::SeqCst), jvp_after_prefix);
}

#[test]
fn exact_budget_exhaustion_charges_every_successful_jvp_and_emits_no_endpoint() {
    let (probe_problem, _, _, _) = fresh_problem();
    let probe = pexprb54s4_fused_step_resume_level2_accounted(
        retained_level2(&probe_problem),
        &ParallelExecution::sequential(),
    )
    .unwrap();
    let required = match probe {
        Pexprb54s4Level2ContinuationOutcome::Complete { ledger, .. } => {
            ledger.continuation_work.jvp_vectors
        }
        other => panic!("unbounded probe must complete, got {other:?}"),
    };
    assert!(required > 0);
    let cap = required - 1;

    let (problem, _, jvp_calls, _) = fresh_problem();
    let prefix = retained_level2(&problem);
    let prefix_work = prefix.report().cumulative_work;
    let jvp_after_prefix = jvp_calls.load(Ordering::SeqCst);
    let outcome = pexprb54s4_fused_step_resume_level2_accounted_jvp_budget(
        prefix,
        &ParallelExecution::sequential(),
        cap,
    )
    .unwrap();

    match outcome {
        Pexprb54s4Level2ContinuationOutcome::BudgetExhausted {
            jvp_cap,
            used_jvp_vectors,
            ledger,
        } => {
            assert_eq!(jvp_cap, cap);
            assert_eq!(used_jvp_vectors, cap);
            assert_eq!(ledger.prefix_work, prefix_work);
            assert_eq!(ledger.continuation_work.jvp_vectors, cap);
            assert_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        other => panic!("cap below required work must exhaust, got {other:?}"),
    }
    assert_eq!(jvp_calls.load(Ordering::SeqCst) - jvp_after_prefix, cap);
}

#[test]
fn generous_cap_is_exactly_equivalent_to_unbounded_compatibility_resume() {
    let (expected_problem, _, _, _) = fresh_problem();
    let expected = pexprb54s4_fused_step_resume_level2(
        retained_level2(&expected_problem),
        &ParallelExecution::sequential(),
    )
    .unwrap();

    let (bounded_problem, _, _, _) = fresh_problem();
    let outcome = pexprb54s4_fused_step_resume_level2_accounted_jvp_budget(
        retained_level2(&bounded_problem),
        &ParallelExecution::sequential(),
        1_000,
    )
    .unwrap();
    match outcome {
        Pexprb54s4Level2ContinuationOutcome::Complete { report, ledger } => {
            assert_eq!(*report, expected);
            assert_eq!(report.work, ledger.cumulative_work);
            assert_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        other => panic!("generous cap unexpectedly failed: {other:?}"),
    }
}

#[test]
fn non_budget_operator_error_remains_a_hard_failure() {
    let (problem, _, _, fail) = fresh_problem();
    let prefix = retained_level2(&problem);
    fail.store(true, Ordering::SeqCst);
    let outcome = pexprb54s4_fused_step_resume_level2_accounted_jvp_budget(
        prefix,
        &ParallelExecution::sequential(),
        80,
    )
    .unwrap();
    match outcome {
        Pexprb54s4Level2ContinuationOutcome::Failed { error, ledger } => {
            assert_eq!(
                error.to_string(),
                "linear solve failed: injected continuation operator failure"
            );
            assert_round_trip(
                ledger.prefix_work,
                ledger.continuation_work,
                ledger.cumulative_work,
            );
        }
        other => panic!("operator error was misclassified: {other:?}"),
    }
}

#[test]
fn bounded_continuation_rejects_parallel_shared_budget_execution_before_jvp() {
    let (problem, _, jvp_calls, _) = fresh_problem();
    let prefix = retained_level2(&problem);
    let jvp_after_prefix = jvp_calls.load(Ordering::SeqCst);
    let execution = ParallelExecution::rayon(2).unwrap();
    let error = pexprb54s4_fused_step_resume_level2_accounted_jvp_budget(prefix, &execution, 80)
        .unwrap_err();
    assert!(error.to_string().contains("sequential"));
    assert_eq!(jvp_calls.load(Ordering::SeqCst), jvp_after_prefix);
}
