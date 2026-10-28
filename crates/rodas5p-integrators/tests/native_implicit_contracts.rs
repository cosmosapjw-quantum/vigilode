use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_integrators::{NewtonConfig, solve_dense_newton};

#[test]
fn dense_newton_freezes_one_jacobian_and_reuses_one_factorization() {
    let mut counters = WorkCounters::default();
    let report = solve_dense_newton(
        &[1.0],
        &[2.0_f64.sqrt()],
        &NewtonConfig {
            atol: 1.0e-15,
            rtol: 1.0e-13,
            max_iterations: 64,
            ..NewtonConfig::default()
        },
        &mut counters,
        |x, _| Ok(vec![x[0] * x[0] - 2.0]),
        |x, _| DenseMatrix::new(1, 1, vec![2.0 * x[0]]),
    )
    .unwrap();

    assert!(report.converged);
    assert!(report.iterations > 1);
    assert!((report.x[0] - 2.0_f64.sqrt()).abs() < 1.0e-12);
    assert_eq!(counters.nonlinear_solves, 1);
    assert_eq!(counters.nonlinear_iterations, report.iterations as u64);
    assert!(counters.nonlinear_residual_evaluations > report.iterations as u64);
    assert_eq!(counters.nonlinear_jacobian_evaluations, 1);
    assert_eq!(counters.direct_factorizations, 1);
    assert_eq!(counters.direct_solve_calls, report.iterations as u64);
    assert_eq!(counters.linear_solves, report.iterations as u64);
    assert_eq!(report.jacobian_refreshes, 0);
    assert_eq!(report.line_search_refreshes, 0);
    assert_eq!(report.stagnation_refreshes, 0);
}

#[test]
fn dense_newton_refreshes_a_stale_jacobian_after_line_search_failure() {
    let mut counters = WorkCounters::default();
    let mut jacobian_calls = 0_usize;
    let report = solve_dense_newton(
        &[0.0],
        &[1.0],
        &NewtonConfig {
            atol: 1.0e-14,
            rtol: 0.0,
            max_iterations: 8,
            max_jacobian_refreshes: 1,
            ..NewtonConfig::default()
        },
        &mut counters,
        |x, _| Ok(vec![x[0] - 1.0]),
        |_x, _| {
            jacobian_calls += 1;
            DenseMatrix::new(1, 1, vec![if jacobian_calls == 1 { -1.0 } else { 1.0 }])
        },
    )
    .unwrap();

    assert!(report.converged);
    assert_eq!(report.x, vec![1.0]);
    assert_eq!(report.jacobian_evaluations, 2);
    assert_eq!(report.jacobian_refreshes, 1);
    assert_eq!(report.line_search_refreshes, 1);
    assert_eq!(report.stagnation_refreshes, 0);
    assert_eq!(counters.nonlinear_jacobian_evaluations, 2);
    assert_eq!(counters.direct_factorizations, 2);
    assert_eq!(counters.direct_solve_calls, 2);
    assert_eq!(counters.nonlinear_iterations, 2);
}

#[test]
fn dense_newton_refreshes_after_bounded_residual_stagnation() {
    let mut counters = WorkCounters::default();
    let mut jacobian_calls = 0_usize;
    let report = solve_dense_newton(
        &[0.0],
        &[1.0],
        &NewtonConfig {
            atol: 1.0e-14,
            rtol: 0.0,
            max_iterations: 8,
            max_jacobian_refreshes: 1,
            stagnation_ratio: 0.85,
            ..NewtonConfig::default()
        },
        &mut counters,
        |x, _| Ok(vec![x[0] - 1.0]),
        |_x, _| {
            jacobian_calls += 1;
            DenseMatrix::new(1, 1, vec![if jacobian_calls == 1 { 10.0 } else { 1.0 }])
        },
    )
    .unwrap();

    assert_eq!(report.x, vec![1.0]);
    assert_eq!(report.jacobian_refreshes, 1);
    assert_eq!(report.line_search_refreshes, 0);
    assert_eq!(report.stagnation_refreshes, 1);
    assert_eq!(counters.nonlinear_jacobian_evaluations, 2);
    assert_eq!(counters.direct_factorizations, 2);
}

use rodas5p_integrators::{
    BdfConfig, BdfHistory, BdfOrder, bdf_step, integrate_bdf_fixed, scalar_linear_problem,
};

#[test]
fn bdf1_matches_the_exact_scalar_amplification_factor() {
    let lambda = -40.0;
    let h = 0.02;
    let (problem, y0) = scalar_linear_problem(lambda, 1.0);
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let report = bdf_step(
        &problem,
        0.0,
        &y0,
        h,
        &BdfConfig {
            order: BdfOrder::One,
            ..BdfConfig::default()
        },
        &mut history,
        &mut counters,
    )
    .unwrap();
    let expected = 1.0 / (1.0 - h * lambda);
    assert!((report.y_new[0] - expected).abs() < 5.0e-13);
    assert_eq!(report.applied_order, BdfOrder::One);
    assert!(!report.used_startup);
}

#[test]
fn bdf2_uses_bdf1_startup_then_is_second_order() {
    let (problem, y0) = scalar_linear_problem(-3.0, 1.0);
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let first = bdf_step(
        &problem,
        0.0,
        &y0,
        0.01,
        &BdfConfig {
            order: BdfOrder::Two,
            ..BdfConfig::default()
        },
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(first.applied_order, BdfOrder::One);
    assert!(first.used_startup);
    let second = bdf_step(
        &problem,
        0.01,
        &first.y_new,
        0.01,
        &BdfConfig {
            order: BdfOrder::Two,
            ..BdfConfig::default()
        },
        &mut history,
        &mut counters,
    )
    .unwrap();
    assert_eq!(second.applied_order, BdfOrder::Two);
    assert!(!second.used_startup);

    let mut errors = Vec::new();
    for h in [0.04, 0.02, 0.01, 0.005] {
        let result = integrate_bdf_fixed(
            &problem,
            (0.0, 0.2),
            &y0,
            h,
            &BdfConfig {
                order: BdfOrder::Two,
                ..BdfConfig::default()
            },
        )
        .unwrap();
        errors.push((result.y.last().unwrap()[0] - problem.exact(0.2).unwrap()[0]).abs());
    }
    let orders: Vec<f64> = errors
        .windows(2)
        .map(|pair| (pair[0] / pair[1]).ln() / 2.0_f64.ln())
        .collect();
    assert!(
        orders[1..].iter().sum::<f64>() / 2.0 > 1.8,
        "{orders:?} {errors:?}"
    );
}

#[test]
fn bdf_mass_matrix_path_is_accurate_and_failure_is_transactional() {
    use rodas5p_core::CoreError;
    use rodas5p_integrators::{OdeProblem, manufactured_mass_nonlinear_problem};
    use std::sync::Arc;

    let (problem, y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0).unwrap();
    let result = integrate_bdf_fixed(
        &problem,
        (0.0, 0.05),
        &y0,
        0.0025,
        &BdfConfig {
            order: BdfOrder::Two,
            ..BdfConfig::default()
        },
    )
    .unwrap();
    assert_eq!(
        result.counters.direct_factorizations,
        result.counters.nonlinear_solves
    );
    assert_eq!(
        result.counters.nonlinear_jacobian_evaluations,
        result.counters.nonlinear_solves
    );
    assert_eq!(
        result.counters.direct_solve_calls,
        result.counters.nonlinear_iterations
    );
    let exact = problem.exact(0.05).unwrap();
    let error = result
        .y
        .last()
        .unwrap()
        .iter()
        .zip(exact)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(error < 2.0e-4, "error={error:e}");

    let bad_rhs = Arc::new(|_t: f64, _y: &[f64], _out: &mut [f64]| {
        Err(CoreError::NonFinite("intentional RHS failure".into()))
    });
    let jac = Arc::new(|_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![1.0]));
    let bad_problem = OdeProblem::new(
        "intentional-bdf-failure",
        1,
        bad_rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut history = BdfHistory::with_previous(vec![0.9], 0.1).unwrap();
    let snapshot = history.clone();
    let mut counters = WorkCounters::default();
    assert!(
        bdf_step(
            &bad_problem,
            0.1,
            &[1.0],
            0.1,
            &BdfConfig::default(),
            &mut history,
            &mut counters,
        )
        .is_err()
    );
    assert_eq!(history, snapshot);
}

use rodas5p_integrators::{
    RadauConfig, RadauIiaStages, RadauStageSolveArchitecture, RadauTransformLimitation,
    integrate_radau_fixed, radau_iia3_tableau, radau_iia3_transform_oracle, radau_step,
};

#[test]
fn radau_iia3_tableau_is_exactly_consistent_and_stiffly_accurate() {
    let (a, b, c) = radau_iia3_tableau();
    for i in 0..3 {
        let row_sum: f64 = (0..3).map(|j| a[(i, j)]).sum();
        assert!((row_sum - c[i]).abs() < 5.0e-15);
    }
    for j in 0..3 {
        assert!((a[(2, j)] - b[j]).abs() < 5.0e-15);
    }
}

#[test]
fn radau_iia3_transform_is_oracle_bound_but_typed_as_deferred_on_real_only_lu() {
    let oracle = radau_iia3_transform_oracle();
    for row in 0..3 {
        for column in 0..3 {
            let product = (0..3)
                .map(|index| oracle.inverse_transform[row][index] * oracle.transform[index][column])
                .sum::<f64>();
            let expected = if row == column { 1.0 } else { 0.0 };
            assert!(
                (product - expected).abs() < 2.0e-14,
                "({row},{column})={product:e}"
            );
        }
    }

    let (problem, y0) = scalar_linear_problem(-4.0, 1.0);
    let mut counters = WorkCounters::default();
    let report = radau_step(
        &problem,
        0.0,
        &y0,
        0.02,
        &RadauConfig::default(),
        &mut counters,
    )
    .unwrap();
    assert_eq!(
        report.stage_solve_architecture,
        RadauStageSolveArchitecture::FullRealStageSystemTransformDeferred(
            RadauTransformLimitation::RealOnlyDenseLu
        )
    );
    assert_eq!(counters.direct_factorizations, 1);
}

#[test]
fn radau_iia1_matches_bdf1_and_radau_iia3_is_fifth_order() {
    let (problem, y0) = scalar_linear_problem(-4.0, 1.0);
    let mut bdf_history = BdfHistory::default();
    let mut bdf_counters = WorkCounters::default();
    let bdf = bdf_step(
        &problem,
        0.0,
        &y0,
        0.02,
        &BdfConfig {
            order: BdfOrder::One,
            ..BdfConfig::default()
        },
        &mut bdf_history,
        &mut bdf_counters,
    )
    .unwrap();
    let mut radau_counters = WorkCounters::default();
    let radau1 = radau_step(
        &problem,
        0.0,
        &y0,
        0.02,
        &RadauConfig {
            stages: RadauIiaStages::One,
            ..RadauConfig::default()
        },
        &mut radau_counters,
    )
    .unwrap();
    assert!((bdf.y_new[0] - radau1.y_new[0]).abs() < 5.0e-12);

    let mut errors = Vec::new();
    for h in [0.04, 0.02, 0.01, 0.005] {
        let result = integrate_radau_fixed(
            &problem,
            (0.0, 0.2),
            &y0,
            h,
            &RadauConfig {
                stages: RadauIiaStages::Three,
                ..RadauConfig::default()
            },
        )
        .unwrap();
        errors.push((result.y.last().unwrap()[0] - problem.exact(0.2).unwrap()[0]).abs());
    }
    let orders: Vec<f64> = errors
        .windows(2)
        .map(|pair| (pair[0] / pair[1]).ln() / 2.0_f64.ln())
        .collect();
    assert!(
        orders[1..].iter().sum::<f64>() / 2.0 > 4.5,
        "{orders:?} {errors:?}"
    );
}

#[test]
fn radau_iia3_matches_stability_function_and_mass_matrix_reference() {
    use rodas5p_integrators::manufactured_mass_nonlinear_problem;

    let z = -20.0;
    let h = 0.02;
    let (problem, y0) = scalar_linear_problem(z / h, 1.0);
    let mut counters = WorkCounters::default();
    let report = radau_step(
        &problem,
        0.0,
        &y0,
        h,
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..RadauConfig::default()
        },
        &mut counters,
    )
    .unwrap();
    let expected = (60.0 + 24.0 * z + 3.0 * z * z) / (60.0 - 36.0 * z + 9.0 * z * z - z * z * z);
    assert!((report.y_new[0] - expected).abs() < 2.0e-12);

    let (mass_problem, mass_y0, _, _) =
        manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0).unwrap();
    let result = integrate_radau_fixed(
        &mass_problem,
        (0.0, 0.05),
        &mass_y0,
        0.01,
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..RadauConfig::default()
        },
    )
    .unwrap();
    let exact = mass_problem.exact(0.05).unwrap();
    let error = result
        .y
        .last()
        .unwrap()
        .iter()
        .zip(exact)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(error < 5.0e-8, "error={error:e}");
}

#[test]
fn dense_newton_accepts_an_exact_linear_correction_without_a_redundant_second_factorization() {
    let mut counters = WorkCounters::default();
    let report = solve_dense_newton(
        &[0.0],
        &[2.0],
        &NewtonConfig {
            atol: 1.0e-13,
            rtol: 1.0e-11,
            ..NewtonConfig::default()
        },
        &mut counters,
        |x, _| Ok(vec![x[0] - 2.0]),
        |_x, _| DenseMatrix::new(1, 1, vec![1.0]),
    )
    .unwrap();

    assert_eq!(report.iterations, 1);
    assert_eq!(counters.nonlinear_jacobian_evaluations, 1);
    assert_eq!(counters.direct_factorizations, 1);
    assert_eq!(counters.direct_solve_calls, 1);
}
