use rodas5p_core::WorkCounters;
use rodas5p_integrators::{StructuredBlockSystem, build_step_context, manufactured_vector_problem};

fn max_abs_diff_rows(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

#[test]
fn partial_linear_decomposition_matches_the_protected_block_operator() {
    let (problem, y0) = manufactured_vector_problem(4, 50.0, 5.0, 0.1, 0.0).unwrap();
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 0.01, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let stages: Vec<Vec<f64>> = (0..block.s)
        .map(|stage| {
            (0..block.n)
                .map(|component| 1e-3 * (1 + stage * block.n + component) as f64)
                .collect()
        })
        .collect();

    let target = block.raw_apply(&stages).unwrap();
    let partial = block
        .partial_linear_apply(&stages, 1.0, &mut counters)
        .unwrap();
    assert!(max_abs_diff_rows(&target, &partial) < 2e-13);

    let diagonal = block.diagonal_apply(&stages, &mut counters).unwrap();
    let coupling = block.coupling_apply(&stages, &mut counters).unwrap();
    let reconstructed: Vec<Vec<f64>> = diagonal
        .iter()
        .zip(&coupling)
        .map(|(d, c)| d.iter().zip(c).map(|(x, y)| x - y).collect())
        .collect();
    assert!(max_abs_diff_rows(&target, &reconstructed) < 2e-13);

    let snapshot = block
        .nonlinear_remainder_snapshot(&stages, &mut counters)
        .unwrap();
    let (rhs, states, fvals, remainder) = block.nonlinear_rhs(&stages, &mut counters).unwrap();
    assert!(max_abs_diff_rows(&snapshot.rhs, &rhs) < 2e-13);
    assert!(max_abs_diff_rows(&snapshot.states, &states) < 2e-13);
    assert!(max_abs_diff_rows(&snapshot.rhs_values, &fvals) < 2e-13);
    assert!(max_abs_diff_rows(&snapshot.remainder, &remainder) < 2e-13);
}

#[test]
fn nonlinear_output_certificate_reduces_to_the_affine_exact_correction() {
    use rodas5p_core::{LinearSolverConfig, safe_l2};
    use rodas5p_integrators::{
        KrylovState, certify_nonlinear_target, constant_affine_mass_problem, sequential_stages,
    };

    let (problem, y0, _mass, _jacobian) = constant_affine_mass_problem();
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.2, &y0, 0.03, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let exact = sequential_stages(
        &context,
        &LinearSolverConfig::default(),
        Option::<&mut KrylovState>::None,
        &mut counters,
    )
    .unwrap()
    .stages;

    let snapshot = block
        .nonlinear_remainder_snapshot(&exact, &mut counters)
        .unwrap();
    let target_jacobian = block
        .target_jacobian_matrix(&exact, &snapshot, &mut counters)
        .unwrap();
    let affine_matrix = block.explicit_matrix().unwrap();
    assert!(
        target_jacobian
            .as_slice()
            .iter()
            .zip(affine_matrix.as_slice())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
            < 3e-13
    );

    let mut perturbation = vec![vec![0.0; block.n]; block.s];
    for (stage, row) in perturbation.iter_mut().enumerate() {
        row[0] = 2e-7 * (stage + 1) as f64;
        row[1] = -1e-7 * (stage + 1).pow(2) as f64;
    }
    let candidate: Vec<Vec<f64>> = exact
        .iter()
        .zip(&perturbation)
        .map(|(stage, delta)| stage.iter().zip(delta).map(|(x, d)| x + d).collect())
        .collect();
    let certificate =
        certify_nonlinear_target(&block, &candidate, 1e-5, 1e-6, &mut counters).unwrap();

    assert!(
        (certificate.correction_norm - safe_l2(&rodas5p_integrators::flatten(&perturbation))).abs()
            < 3e-11
    );
    assert!(certificate.output_wrms > 0.0);
    assert!(certificate.stage_residual_norm > 0.0);

    let exact_certificate =
        certify_nonlinear_target(&block, &exact, 1e-5, 1e-6, &mut counters).unwrap();
    assert!(exact_certificate.output_wrms < 2e-10);
    assert!(exact_certificate.correction_norm < 2e-11);
}

#[test]
fn fixed_homotopy_path_validates_schedule_and_reaches_the_affine_q7_endpoint() {
    use rodas5p_core::{CoreError, LinearSolverConfig};
    use rodas5p_integrators::{
        HomotopyPathConfig, HomotopyPredictor, KrylovState, constant_affine_mass_problem,
        run_fixed_homotopy_path, sequential_stages,
    };

    assert!(matches!(
        HomotopyPathConfig::new(0.5, 8, 2, HomotopyPredictor::Euler, 1),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        HomotopyPathConfig::new(0.5, 2, 0, HomotopyPredictor::Euler, 1),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        HomotopyPathConfig::new(f64::NAN, 2, 2, HomotopyPredictor::Euler, 1),
        Err(CoreError::NonFinite(_))
    ));

    let (problem, y0, _mass, _jacobian) = constant_affine_mass_problem();
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.2, &y0, 0.03, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let exact = sequential_stages(
        &context,
        &LinearSolverConfig::default(),
        Option::<&mut KrylovState>::None,
        &mut counters,
    )
    .unwrap()
    .stages;

    for predictor in [HomotopyPredictor::Euler, HomotopyPredictor::AdamsBashforth2] {
        let config = HomotopyPathConfig::new(0.35, 7, 3, predictor, 1).unwrap();
        let report = run_fixed_homotopy_path(&block, &config, &mut counters).unwrap();
        assert_eq!(report.points.len(), 4);
        assert_eq!(report.work.path_rounds, 3);
        assert_eq!(report.work.correction_rounds, 3);
        assert_eq!(report.work.w_factorizations, 1);
        assert!(report.work.w_solve_batches > 0);
        assert!(
            max_abs_diff_rows(&report.stages, &exact) < 2e-11,
            "{predictor:?} did not recover the affine endpoint"
        );
        assert!(report.points.last().unwrap().target_residual_norm < 2e-11);
    }
}

#[test]
fn low_depth_homotopy_paths_remain_finite_and_expose_a_residual_hierarchy() {
    use rodas5p_integrators::{HomotopyPathConfig, HomotopyPredictor, run_fixed_homotopy_path};

    let (problem, y0) = manufactured_vector_problem(4, 100.0, 8.0, 0.2, 0.0).unwrap();
    let mut final_residuals = Vec::new();
    for q in 0..=2 {
        let mut counters = WorkCounters::default();
        let context = build_step_context(&problem, 0.0, &y0, 0.01, &mut counters).unwrap();
        let block = StructuredBlockSystem::new(&context);
        let config = HomotopyPathConfig::new(0.5, q, 3, HomotopyPredictor::Euler, 1).unwrap();
        let report = run_fixed_homotopy_path(&block, &config, &mut counters).unwrap();
        assert!(
            report
                .stages
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
        assert!(
            report
                .points
                .iter()
                .all(|point| point.homotopy_residual_norm.is_finite()
                    && point.target_residual_norm.is_finite())
        );
        final_residuals.push(report.points.last().unwrap().target_residual_norm);
    }
    assert!(final_residuals[2] <= final_residuals[1] * 1.01);
    assert!(final_residuals[1] <= final_residuals[0] * 1.01);
}

#[test]
fn homotopy_step_accepts_the_affine_oracle_and_falls_back_transactionally() {
    use rodas5p_core::{LinearMethod, LinearSolverConfig};
    use rodas5p_integrators::{
        HomotopyPathConfig, HomotopyPredictor, HomotopyStepConfig, constant_affine_mass_problem,
        homotopy_step, manufactured_vector_problem, sequential_step,
    };

    let fallback = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    };

    let (affine, affine_y0, _mass, _jacobian) = constant_affine_mass_problem();
    let affine_path = HomotopyPathConfig::new(0.4, 7, 2, HomotopyPredictor::Euler, 1).unwrap();
    let affine_config = HomotopyStepConfig::new(affine_path, 0.1).unwrap();
    let mut affine_counters = WorkCounters::default();
    let affine_report = homotopy_step(
        &affine,
        0.2,
        &affine_y0,
        0.03,
        &affine_config,
        Some(&fallback),
        None,
        1e-5,
        1e-6,
        true,
        &mut affine_counters,
    )
    .unwrap();
    assert!(affine_report.fast_accepted);
    assert!(!affine_report.step.used_fallback);
    assert!(
        affine_report
            .output_certificate
            .as_ref()
            .unwrap()
            .output_wrms
            < 1e-7
    );

    let (nonlinear, y0) = manufactured_vector_problem(4, 80.0, 12.0, 0.35, 0.0).unwrap();
    let forced_path = HomotopyPathConfig::new(0.0, 0, 1, HomotopyPredictor::Euler, 0).unwrap();
    let forced_config = HomotopyStepConfig::new(forced_path, 0.0).unwrap();

    let mut homotopy_counters = WorkCounters::default();
    let report = homotopy_step(
        &nonlinear,
        0.0,
        &y0,
        0.02,
        &forced_config,
        Some(&fallback),
        None,
        1e-7,
        1e-6,
        true,
        &mut homotopy_counters,
    )
    .unwrap();

    let mut sequential_counters = WorkCounters::default();
    let sequential = sequential_step(
        &nonlinear,
        0.0,
        &y0,
        0.02,
        &fallback,
        None,
        1e-7,
        1e-6,
        true,
        &mut sequential_counters,
    )
    .unwrap();

    assert!(!report.fast_accepted);
    assert!(report.step.used_fallback);
    assert!(
        report
            .fallback_reason
            .as_deref()
            .unwrap()
            .contains("certificate")
    );
    assert!(max_abs_diff_rows(&report.step.stages, &sequential.stages) < 2e-12);
    assert!(
        report
            .step
            .y_new
            .iter()
            .zip(&sequential.y_new)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
            < 2e-12
    );
}

#[test]
fn manufactured_mass_problem_has_noncommuting_structure_and_exact_rhs() {
    use rodas5p_integrators::manufactured_mass_nonlinear_problem;

    let (problem, y0, mass, linear_part) =
        manufactured_mass_nonlinear_problem(100.0, 10.0, 0.4, 0.0).unwrap();
    let commutator = mass
        .matmul(&linear_part)
        .unwrap()
        .sub(&linear_part.matmul(&mass).unwrap())
        .unwrap();
    assert!(rodas5p_core::safe_l2(commutator.as_slice()) > 1e-2);

    let mut counters = WorkCounters::default();
    let rhs = problem.eval_rhs(0.0, &y0, &mut counters).unwrap();
    let exact_derivative = [1.0, -0.05];
    let expected = mass.matvec(&exact_derivative).unwrap();
    assert!(
        rhs.iter()
            .zip(expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
            < 2e-13
    );
}
