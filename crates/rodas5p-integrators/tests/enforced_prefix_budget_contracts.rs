use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
    Pexprb54s4BudgetedLevel2PrefixOutcome,
    pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry,
    pexprb54s4_level2_prefix_resume_level1,
    pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget,
};

fn counted_square_problem(calls: Arc<AtomicU64>) -> OdeProblem {
    OdeProblem::new(
        "pexprb-budget-counted-square",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y: &[f64], v: &[f64], out: &mut [f64]| {
            calls.fetch_add(1, Ordering::SeqCst);
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

#[test]
fn budget_guard_refuses_the_cap_plus_one_jvp_before_operator_call() {
    let calls = Arc::new(AtomicU64::new(0));
    let problem = counted_square_problem(Arc::clone(&calls));
    let outcome = pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget(
        &problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
        1,
    )
    .unwrap();

    match outcome {
        Pexprb54s4BudgetedLevel2PrefixOutcome::BudgetExhausted(report) => {
            assert_eq!(report.jvp_cap, 1);
            assert_eq!(report.used_jvp_vectors, 1);
            assert_eq!(report.work.jvp_vectors, 1);
        }
        Pexprb54s4BudgetedLevel2PrefixOutcome::Complete(_) => {
            panic!("one JVP cannot complete this prefix")
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn zero_budget_executes_no_jvp_and_reports_zero_spent_jvp_work() {
    let calls = Arc::new(AtomicU64::new(0));
    let problem = counted_square_problem(Arc::clone(&calls));
    let outcome = pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget(
        &problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
        0,
    )
    .unwrap();
    match outcome {
        Pexprb54s4BudgetedLevel2PrefixOutcome::BudgetExhausted(report) => {
            assert_eq!(report.used_jvp_vectors, 0);
            assert_eq!(report.work.jvp_vectors, 0);
        }
        Pexprb54s4BudgetedLevel2PrefixOutcome::Complete(_) => panic!("zero budget completed"),
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn generous_budget_reproduces_unbudgeted_level2_report_exactly() {
    let normal_calls = Arc::new(AtomicU64::new(0));
    let normal_problem = counted_square_problem(normal_calls);
    let level1 = pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry(
        &normal_problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
    )
    .unwrap();
    let expected = pexprb54s4_level2_prefix_resume_level1(level1, &ParallelExecution::sequential())
        .unwrap()
        .report()
        .clone();

    let budget_calls = Arc::new(AtomicU64::new(0));
    let budget_problem = counted_square_problem(budget_calls);
    let outcome = pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget(
        &budget_problem,
        0.0,
        &[1.0],
        0.0625,
        phi_config(),
        1,
        1e-10,
        1e-8,
        1_000,
    )
    .unwrap();
    match outcome {
        Pexprb54s4BudgetedLevel2PrefixOutcome::Complete(prefix) => {
            assert_eq!(prefix.report(), &expected);
        }
        Pexprb54s4BudgetedLevel2PrefixOutcome::BudgetExhausted(report) => {
            panic!("unexpected exhaustion at {} JVPs", report.used_jvp_vectors)
        }
    }
}
