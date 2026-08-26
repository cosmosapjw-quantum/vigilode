use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    GmresKernelArm, build_step_context_matrix_free, manufactured_vector_problem,
    production_gmres_kernel_arm, sequential_stages, sequential_stages_with_gmres_kernel,
};

#[test]
fn explicit_kernel_arm_selection_preserves_the_legacy_production_default() {
    assert_eq!(
        production_gmres_kernel_arm(),
        GmresKernelArm::LegacyRestartedGmres
    );
    assert_eq!(
        GmresKernelArm::ALL.map(GmresKernelArm::as_str),
        [
            "legacy-restarted-gmres",
            "incremental-givens-candidate",
        ]
    );

    let (problem, y0) = manufactured_vector_problem(4, 10.0, 1.0, 0.2, 0.0).unwrap();
    let problem = problem.jvp_only_clone().unwrap();
    let mut setup_counters = WorkCounters::default();
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-4, &mut setup_counters)
            .unwrap();
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };

    let mut production_counters = WorkCounters::default();
    let production = sequential_stages(&context, &config, None, &mut production_counters).unwrap();
    assert!(production.reports.iter().all(|report| report.converged));
    assert!(
        production
            .reports
            .iter()
            .all(|report| report.method == "gmres")
    );

    let mut explicit_legacy_counters = WorkCounters::default();
    let explicit_legacy = sequential_stages_with_gmres_kernel(
        &context,
        &config,
        GmresKernelArm::LegacyRestartedGmres,
        None,
        &mut explicit_legacy_counters,
    )
    .unwrap();
    assert_eq!(production_counters, explicit_legacy_counters);
    assert_eq!(production.stages, explicit_legacy.stages);
    assert!(
        explicit_legacy
            .reports
            .iter()
            .all(|report| report.method == "gmres")
    );

    let mut candidate_counters = WorkCounters::default();
    let candidate = sequential_stages_with_gmres_kernel(
        &context,
        &config,
        GmresKernelArm::IncrementalGivensCandidate,
        None,
        &mut candidate_counters,
    )
    .unwrap();
    assert!(candidate.reports.iter().all(|report| report.converged));
    assert!(candidate.reports.iter().all(|report| {
        report.method == "gmres-givens-candidate"
            && report.residual_norm.is_finite()
            && report.relative_residual.is_finite()
    }));
}
