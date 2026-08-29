use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
    load_rodas5p_coefficients, wrms,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, G4S5B0InnerToleranceLane, G4S5B0InnerTolerancePolicy,
    G4S5B0LinearToleranceArm, OutputSchedule, RODAS5P_INNER_FORCING_CLAIM_SCOPE,
    Rodas5pInnerForcingClaimScope, committed_g4_s5b0_linear_tolerance_arm,
    integrate_sequential_matrix_free_adaptive_observed, manufactured_vector_problem,
    rodas5p_inner_forcing_target, scalar_linear_problem, sequential_matrix_free_step,
    sequential_matrix_free_step_with_inner_forcing,
};

#[test]
fn forcing_target_uses_the_frozen_flow_rhs_residual_allocation() {
    // Defect caught: the legacy tolerance is a fixed fraction of outer rtol and
    // cannot distinguish a large h*f scale from a smaller assembled RHS scale.
    let target = rodas5p_inner_forcing_target(4.0, 2.0, 3.0).unwrap();
    let expected_eta: f64 = 0.1 / (3.0 * 4.0);
    let expected_tau = expected_eta * 2.0;
    assert_eq!(target.eta.to_bits(), expected_eta.to_bits());
    assert_eq!(target.tau.to_bits(), expected_tau.to_bits());
    assert!(3.0 * target.tau <= 0.1);
}

#[test]
fn protected_matrix_free_step_applies_stage_specific_wrms_forcing() {
    // Defect caught: merely computing eta/tau without threading the scale into
    // each true-residual certification leaves the production stage path L2-controlled.
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    };
    let mut work = WorkCounters::default();
    let coarse = sequential_matrix_free_step_with_inner_forcing(
        &problem, 0.0, &y0, 0.1, &config, None, 1.0e-8, 1.0e-4, true, &mut work,
    )
    .unwrap();
    let mut work = WorkCounters::default();
    let fine = sequential_matrix_free_step_with_inner_forcing(
        &problem, 0.0, &y0, 0.05, &config, None, 1.0e-8, 1.0e-4, true, &mut work,
    )
    .unwrap();

    let coefficients = load_rodas5p_coefficients().unwrap();
    let output_weight_l1 = coefficients
        .b
        .iter()
        .map(|weight| weight.abs())
        .sum::<f64>();
    assert_eq!(coarse.stage_forcing.len(), coefficients.stages());
    assert_eq!(fine.stage_forcing.len(), coefficients.stages());
    for row in &coarse.stage_forcing {
        let oracle =
            rodas5p_inner_forcing_target(row.flow_wrms, row.rhs_wrms, output_weight_l1).unwrap();
        assert_eq!(row.eta.to_bits(), oracle.eta.to_bits());
        assert_eq!(row.tau.to_bits(), oracle.tau.to_bits());
        assert!(row.achieved_residual_wrms <= row.eta * row.rhs_wrms);
        assert!(row.eta * row.rhs_wrms <= row.tau);
        assert!(output_weight_l1 * row.tau <= 0.1);
    }
    assert!(fine.stage_forcing[0].eta > coarse.stage_forcing[0].eta);
    assert!(coarse.step.accepted);
    assert!(fine.step.accepted);
}

#[test]
fn residual_allocation_explicitly_does_not_certify_nonnormal_endpoint_error() {
    // Defect caught: treating ||r||_WRMS <= tau as an endpoint-error theorem
    // silently drops the amplification in W^{-1}r. For this nonnormal W,
    // r is accepted by the residual heuristic while the exact correction is
    // arbitrarily large as K grows.
    assert_eq!(
        RODAS5P_INNER_FORCING_CLAIM_SCOPE,
        Rodas5pInnerForcingClaimScope::StageResidualHeuristicRequiresResolventBound
    );
    let target = rodas5p_inner_forcing_target(4.0, 2.0, 3.0).unwrap();
    let residual = [0.0, target.tau];
    let scale = [1.0, 1.0];
    assert!(wrms(&residual, &scale).unwrap() <= target.tau);

    // W = [[1, K], [0, 1]], hence W^{-1}r = [-K*tau, tau].
    let nonnormality = 1.0e6;
    let correction = [-nonnormality * target.tau, target.tau];
    assert!(wrms(&correction, &scale).unwrap() > 0.1);
}

fn forced_fixed_endpoint(step: f64) -> f64 {
    let (problem, y0) = manufactured_vector_problem(8, 20.0, 0.0, 0.2, 0.0).unwrap();
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };
    let final_time = 0.4;
    let steps = (final_time / step).round() as usize;
    // A fixed-step global order-five check needs each local linear-solve
    // contamination budget to scale as h^6. The adaptive API instead derives
    // this scale from its local outer target on every attempted step.
    let rtol = step.powi(6);
    let atol = 1.0e-2 * rtol;
    let mut t = 0.0;
    let mut y = y0;
    let mut work = WorkCounters::default();
    for _ in 0..steps {
        let report = sequential_matrix_free_step_with_inner_forcing(
            &problem, t, &y, step, &config, None, atol, rtol, true, &mut work,
        )
        .unwrap();
        y = report.step.y_new;
        t += step;
    }
    let exact = problem.exact(final_time).unwrap();
    y.iter()
        .zip(exact)
        .map(|(actual, expected)| (actual - expected).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[test]
fn forced_fixed_step_refinement_retains_order_five_before_roundoff() {
    // Defect caught: an h-independent inner residual tolerance creates a global
    // error floor and makes at least one pre-roundoff refinement slope collapse.
    let errors = [0.08, 0.04, 0.02, 0.01].map(forced_fixed_endpoint);
    let orders = errors
        .windows(2)
        .filter(|pair| pair[1] > 256.0 * f64::EPSILON)
        .map(|pair| (pair[0] / pair[1]).log2())
        .collect::<Vec<_>>();
    assert!(!orders.is_empty());
    assert!(orders.iter().all(|order| *order >= 4.8), "{orders:?}");
}

fn assert_step_bits_equal(
    left: &rodas5p_integrators::StepResult,
    right: &rodas5p_integrators::StepResult,
) {
    assert_eq!(left.t_old.to_bits(), right.t_old.to_bits());
    assert_eq!(left.t_new.to_bits(), right.t_new.to_bits());
    assert_eq!(left.h.to_bits(), right.h.to_bits());
    assert_eq!(left.accepted, right.accepted);
    assert_eq!(left.method, right.method);
    assert_eq!(left.used_fallback, right.used_fallback);
    assert_eq!(left.counters, right.counters);
    for (left, right) in left
        .y_new
        .iter()
        .chain(&left.error_vector)
        .chain(left.stages.iter().flatten())
        .zip(
            right
                .y_new
                .iter()
                .chain(&right.error_vector)
                .chain(right.stages.iter().flatten()),
        )
    {
        assert_eq!(left.to_bits(), right.to_bits());
    }
    assert_eq!(left.error_norm.to_bits(), right.error_norm.to_bits());
}

#[test]
fn opt_in_forcing_does_not_change_the_ordinary_unforced_step() {
    // Defect caught: storing forcing state in the shared config or a global lane
    // would change the legacy unforced result after an opt-in forced solve.
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    };
    let ordinary = |work: &mut WorkCounters| {
        sequential_matrix_free_step(
            &problem, 0.0, &y0, 0.02, &config, None, 1.0e-10, 1.0e-7, true, work,
        )
        .unwrap()
    };
    let mut before_work = WorkCounters::default();
    let before = ordinary(&mut before_work);
    let mut forced_work = WorkCounters::default();
    sequential_matrix_free_step_with_inner_forcing(
        &problem,
        0.0,
        &y0,
        0.02,
        &config,
        None,
        1.0e-10,
        1.0e-7,
        true,
        &mut forced_work,
    )
    .unwrap();
    let mut after_work = WorkCounters::default();
    let after = ordinary(&mut after_work);
    assert_step_bits_equal(&before, &after);
}

#[test]
fn generic_protected_adaptive_path_uses_inner_forcing() {
    // Defect caught: exposing only a forced single-step helper leaves the
    // production protected adaptive integrator on the legacy unweighted solve.
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        atol: 1.0e30,
        rtol: 1.0,
        ..LinearSolverConfig::default()
    };
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-5,
        initial_step: 0.05,
        min_step: 1.0e-12,
        max_step: 0.05,
        max_attempts: 8,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_sequential_matrix_free_adaptive_observed(
        &problem,
        (0.0, 0.05),
        &y0,
        &linear,
        &adaptive,
        &OutputSchedule::new(vec![0.0, 0.05]).unwrap(),
    )
    .unwrap();
    assert!(result.observed.success);
    assert!(result.observed.counters.linear_iterations > 0);
    let endpoint = result.observed.y.last().unwrap()[0];
    assert!((endpoint - problem.exact(0.05).unwrap()[0]).abs() < 1.0e-7);
}

#[test]
fn forcing_is_separate_from_the_frozen_a1_two_arm_policy() {
    assert_eq!(
        committed_g4_s5b0_linear_tolerance_arm(),
        G4S5B0LinearToleranceArm::LegacyFixed
    );
    assert_eq!(G4S5B0LinearToleranceArm::ALL.len(), 2);
    let legacy = G4S5B0InnerTolerancePolicy::try_for_lane(
        G4S5B0InnerToleranceLane::RegimeAtlas,
        G4S5B0LinearToleranceArm::LegacyFixed,
        1.0e-5,
    )
    .unwrap()
    .linear_config();
    assert_eq!(legacy.rtol.to_bits(), 1.0e-10_f64.to_bits());
    assert_eq!(legacy.atol.to_bits(), 1.0e-12_f64.to_bits());
}
