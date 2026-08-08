use rodas5p_core::DenseMatrix;
use rodas5p_fair_ab::{
    BenchmarkCell, BenchmarkPlan, FairSolveConfig, LinearSystemCase, PreconditionerKind,
    RecycleLifetime, SequenceConfig, SequenceKind, SolveStatus, SolverKind, SolverSession,
    TraceDocument, build_execution_order, generate_trace, run_comparison, run_trace, solve_case,
};

fn tiny_config(kind: SequenceKind) -> SequenceConfig {
    SequenceConfig {
        kind,
        dimension: 12,
        steps: 2,
        stages: 8,
        seed: 20260806,
        stiffness: 100.0,
        nonnormality: 0.05,
    }
}

fn solver_config(solver: SolverKind) -> FairSolveConfig {
    FairSolveConfig {
        solver,
        rtol: 1e-9,
        atol: 1e-12,
        restart: 10,
        recycle_dim: 3,
        hard_operator_budget: 500,
        preconditioner: PreconditionerKind::None,
        use_previous_oracle_guess: true,
    }
}

#[test]
fn deterministic_trace_roundtrip_preserves_every_identity() {
    let trace = generate_trace(&tiny_config(SequenceKind::SlowDrift)).unwrap();
    let repeated = generate_trace(&tiny_config(SequenceKind::SlowDrift)).unwrap();
    assert_eq!(trace.trace_id, repeated.trace_id);
    assert_eq!(trace.system_ids(), repeated.system_ids());
    assert_eq!(trace.cases.len(), 16);

    let document = TraceDocument::from_trace(&trace);
    let json = serde_json::to_vec(&document).unwrap();
    let decoded: TraceDocument = serde_json::from_slice(&json).unwrap();
    let restored = decoded.into_trace().unwrap();
    assert_eq!(restored.trace_id, trace.trace_id);
    assert_eq!(restored.system_ids(), trace.system_ids());
}

#[test]
fn external_certificate_and_hard_budget_cover_diagnostic_work() {
    let a = DenseMatrix::from_rows(&[&[4.0, 1.0], &[1.0, 3.0]]).unwrap();
    let case =
        LinearSystemCase::from_matrix(a.clone(), a.matvec(&[1.0, -2.0]).unwrap(), 0, 0).unwrap();
    let mut config = solver_config(SolverKind::Gmres);
    config.hard_operator_budget = 1;
    let result = solve_case(&case, &config, None, None).unwrap();
    assert_eq!(result.status, SolveStatus::BudgetExhausted);
    assert!(!result.certificate.passed);
    assert_eq!(result.ledger.budget_exhaustions, 1);
    assert!(result.ledger.operator_total() <= 1);
}

#[test]
fn every_rust_solver_consumes_the_same_immutable_trace_and_certifies_residuals() {
    let trace = generate_trace(&tiny_config(SequenceKind::Fixed)).unwrap();
    let expected_ids = trace.system_ids();
    for (solver, lifetime) in [
        (SolverKind::Gmres, RecycleLifetime::Off),
        (SolverKind::Lgmres, RecycleLifetime::Stage),
        (SolverKind::Gcrodr, RecycleLifetime::Persistent),
    ] {
        let run = run_trace(&trace, &solver_config(solver), lifetime, 0).unwrap();
        assert_eq!(run.system_ids, expected_ids, "solver={solver:?}");
        assert_eq!(run.failures, 0, "solver={solver:?}");
        assert!(run.solves.iter().all(|r| r.certificate.passed));
        assert!(run.ledger.operator_diagnostic >= trace.cases.len() as u64);
    }
}

#[test]
fn recycle_lifetimes_have_distinct_and_auditable_reset_semantics() {
    let trace = generate_trace(&tiny_config(SequenceKind::SlowDrift)).unwrap();
    let stage = run_trace(
        &trace,
        &solver_config(SolverKind::Gcrodr),
        RecycleLifetime::Stage,
        0,
    )
    .unwrap();
    let persistent = run_trace(
        &trace,
        &solver_config(SolverKind::Gcrodr),
        RecycleLifetime::Persistent,
        0,
    )
    .unwrap();
    let off = run_trace(
        &trace,
        &solver_config(SolverKind::Gcrodr),
        RecycleLifetime::Off,
        0,
    )
    .unwrap();

    assert_eq!(stage.policy_resets, 1, "one reset at the step boundary");
    assert_eq!(persistent.policy_resets, 0);
    assert_eq!(off.policy_resets, trace.cases.len() as u64);
    assert!(stage.transition_log.iter().all(|t| !t.reason.is_empty()));
}

#[test]
fn comparison_randomizes_order_but_keeps_paired_trace_identity() {
    let trace = generate_trace(&tiny_config(SequenceKind::Abrupt)).unwrap();
    let cells = vec![
        BenchmarkCell::new(SolverKind::Gmres, RecycleLifetime::Off),
        BenchmarkCell::new(SolverKind::Lgmres, RecycleLifetime::Stage),
        BenchmarkCell::new(SolverKind::Gcrodr, RecycleLifetime::Persistent),
    ];
    let plan = BenchmarkPlan {
        cells,
        repetitions: 2,
        warmups: 0,
        seed: 17,
    };
    let result = run_comparison(&trace, &plan, solver_config).unwrap();
    assert_eq!(result.runs.len(), 6);
    assert!(result.runs.iter().all(|run| run.trace_id == trace.trace_id));
    assert_eq!(result.execution_order.len(), 2);
    assert_eq!(
        result.execution_order,
        build_execution_order(&plan.cells, plan.repetitions, plan.seed).unwrap(),
    );
    let mut canonical = plan.cells.clone();
    canonical.sort();
    for order in &result.execution_order {
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted, canonical, "every repetition must be a permutation");
    }
}

#[test]
fn recycle_reset_preserves_solver_workspace_but_clears_algorithmic_state() {
    let matrix = DenseMatrix::from_rows(&[
        &[4.0, 0.5, 0.0, 0.0],
        &[-0.1, 5.0, 0.4, 0.0],
        &[0.0, -0.2, 6.0, 0.3],
        &[0.0, 0.0, -0.1, 7.0],
    ])
    .unwrap();
    let rhs = matrix.matvec(&[1.0, -0.5, 0.25, 0.75]).unwrap();
    let case = LinearSystemCase::from_matrix(matrix, rhs, 0, 0).unwrap();
    let config = solver_config(SolverKind::Gmres);
    let mut session = SolverSession::new(SolverKind::Gmres);

    let result = solve_case(&case, &config, Some(&mut session), None).unwrap();
    assert_eq!(result.status, SolveStatus::Converged);
    let capacity_before = session.workspace_capacity_f64();
    assert!(capacity_before > 0);
    assert!(session.generation > 0);

    session.clear_recycle_state();
    assert_eq!(session.generation, 0);
    assert!(session.previous_solution.is_none());
    assert_eq!(session.workspace_capacity_f64(), capacity_before);
}
