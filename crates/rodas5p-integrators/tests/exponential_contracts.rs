use std::sync::Arc;

use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters, dense_phi_action, safe_l2};
use rodas5p_integrators::{
    ExponentialKrylovConfig, OdeProblem, ParallelExecution, exprb2_step, exprb43_step,
    krylov_phi_action, pexprb54s4_step, pexprb54s4_tableau,
};

fn square_problem() -> OdeProblem {
    OdeProblem::new(
        "scalar-square",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
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
        Some(Arc::new(|t| vec![1.0 / (1.0 - t)])),
    )
    .expect("square problem")
}

fn config() -> ExponentialKrylovConfig {
    ExponentialKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 12,
        dimension_increment: 1,
        relative_tolerance: 1.0e-13,
        absolute_tolerance: 1.0e-15,
        reorthogonalize: true,
    }
}

fn integrate(method: &str, h: f64) -> (f64, WorkCounters) {
    let problem = square_problem();
    let execution = ParallelExecution::sequential();
    let final_time = 0.25;
    let steps = (final_time / h).round() as usize;
    let mut y = vec![1.0];
    let mut total = WorkCounters::default();
    let mut t = 0.0;
    for _ in 0..steps {
        let report = match method {
            "exprb2" => exprb2_step(&problem, t, &y, h, config()).expect("exprb2"),
            "exprb43" => exprb43_step(&problem, t, &y, h, config()).expect("exprb43"),
            "pexprb54s4" => {
                pexprb54s4_step(&problem, t, &y, h, config(), &execution).expect("pexprb54s4")
            }
            _ => panic!("unknown method"),
        };
        y = report.y_new;
        total.accumulate(report.work);
        t += h;
    }
    ((y[0] - 1.0 / (1.0 - final_time)).abs(), total)
}

fn observed_order(method: &str) -> f64 {
    let (coarse, _) = integrate(method, 0.025);
    let (fine, _) = integrate(method, 0.0125);
    (coarse / fine).log2()
}

#[test]
fn matrix_free_krylov_matches_dense_nonnormal_phi_action() {
    let matrix = DenseMatrix::from_vec_rows(vec![
        vec![-5.0, 8.0, 0.0, 0.0],
        vec![0.0, -6.0, 7.0, 0.0],
        vec![0.0, 0.0, -7.0, 6.0],
        vec![0.0, 0.0, 0.0, -8.0],
    ])
    .expect("matrix");
    let operator = Arc::new(DenseOperator::new(matrix.clone()).expect("operator"));
    let vector = vec![1.0, -0.25, 0.5, -0.75];
    for phi_index in 1..=5 {
        let expected = dense_phi_action(&matrix, 0.7, phi_index, &vector).expect("dense phi");
        let mut counters = WorkCounters::default();
        let report = krylov_phi_action(
            operator.clone(),
            0.7,
            phi_index,
            &vector,
            config(),
            &mut counters,
        )
        .expect("Krylov phi");
        assert!(report.converged);
        let defect: Vec<f64> = report
            .value
            .iter()
            .zip(&expected)
            .map(|(a, b)| a - b)
            .collect();
        assert!(safe_l2(&defect) <= 2.0e-11 * safe_l2(&expected).max(1.0));
        assert_eq!(counters.jacobian_builds, 0);
        assert_eq!(counters.direct_factorizations, 0);
        assert!(counters.jvp_vectors > 0);
    }
}

#[test]
fn pexprb54s4_coefficients_are_exactly_locked() {
    let t = pexprb54s4_tableau();
    assert_eq!(t.c2, 0.25);
    assert_eq!(t.c3, 0.5);
    assert_eq!(t.c4, 0.9);
    assert_eq!(t.a32_phi3, 4.0);
    assert_eq!(t.a42_phi3, 2916.0 / 125.0);
    assert_eq!(t.a43, 0.0);
    assert_eq!(t.b3_phi3, 18.0);
    assert_eq!(t.b3_phi4, -60.0);
    assert_eq!(t.b4_phi3, -250.0 / 81.0);
    assert_eq!(t.b4_phi4, 500.0 / 27.0);
    assert_eq!(t.embedded_b2_phi3, 64.0);
    assert_eq!(t.embedded_b2_phi4, -60.0);
    assert_eq!(t.embedded_b3_phi3, -8.0);
    assert_eq!(t.embedded_b3_phi4, -285.0 / 8.0);
    assert_eq!(t.embedded_b4_phi3, 0.0);
    assert_eq!(t.embedded_b4_phi4, 125.0 / 8.0);
}

#[test]
fn declared_orders_are_observed_on_nonlinear_problem() {
    assert!(observed_order("exprb2") >= 1.8);
    assert!(observed_order("exprb43") >= 3.7);
    assert!(observed_order("pexprb54s4") >= 4.7);
}

#[test]
fn exponential_primary_path_builds_no_explicit_jacobian_or_newton_system() {
    for method in ["exprb2", "exprb43", "pexprb54s4"] {
        let (_, work) = integrate(method, 0.025);
        assert_eq!(work.jacobian_builds, 0, "{method}");
        assert_eq!(work.direct_factorizations, 0, "{method}");
        assert_eq!(work.nonlinear_iterations, 0, "{method}");
        assert!(work.phi_actions > 0, "{method}");
        assert!(work.jvp_vectors > 0, "{method}");
    }
}

#[test]
fn parallel_order_five_method_has_three_level_logical_critical_path() {
    let problem = square_problem();
    let execution = ParallelExecution::rayon(2).expect("execution");
    let report =
        pexprb54s4_step(&problem, 0.0, &[1.0], 0.01, config(), &execution).expect("pexprb54s4");
    assert_eq!(report.logical_critical_depth, 3);
    assert!(report.y_embedded.is_some());
    assert!(report.error_estimate.is_some());
}

#[test]
fn parallel_order_five_method_executes_dependency_levels_concurrently() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let jvp_active = active.clone();
    let jvp_maximum = maximum.clone();
    let problem = OdeProblem::new(
        "scalar-linear-concurrency",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = -2.0 * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _: &[f64], v: &[f64], out: &mut [f64]| {
            let now = jvp_active.fetch_add(1, Ordering::SeqCst) + 1;
            jvp_maximum.fetch_max(now, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(4));
            out[0] = -2.0 * v[0];
            jvp_active.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        })),
        None,
        true,
        None,
        Some(Arc::new(|t| vec![(-2.0 * t).exp()])),
    )
    .expect("concurrency problem");
    let execution = ParallelExecution::rayon(2).expect("execution");
    let report = pexprb54s4_step(
        &problem,
        0.0,
        &[1.0],
        0.01,
        ExponentialKrylovConfig {
            minimum_dimension: 1,
            maximum_dimension: 1,
            dimension_increment: 1,
            relative_tolerance: 1.0e-13,
            absolute_tolerance: 1.0e-15,
            reorthogonalize: true,
        },
        &execution,
    )
    .expect("parallel pexprb54s4");
    assert_eq!(report.logical_critical_depth, 3);
    assert!(maximum.load(Ordering::SeqCst) >= 2);
}
