//! Order evidence for the fused exponential methods beyond the scalar
//! y' = y^2 test (audit F-041).
//!
//! The scalar test cannot see non-commuting J and N, stiff order reduction
//! or Krylov truncation. The thresholds below are failure-preserving floors
//! taken from the audit's exact-phi replica (exp/D4/pexprb54s4_order.out:
//! stiff lambda = -1e4 global slopes 4.15 / 4.09 / 4.08, nonstiff local
//! slopes 5.7-5.9). Do not lower them.

use std::sync::Arc;

use rodas5p_core::safe_l2;
use rodas5p_integrators::{
    FusedOrthogonalization, FusedPhiKrylovConfig, OdeProblem, ParallelExecution,
    pexprb54s4_fused_step,
};

/// Stiff floor: the documented stiff order of pexprb54s4 is about 4, not 5.
const STIFF_ORDER_FLOOR: f64 = 3.8;
/// Nonstiff floor for the declared order 5.
const NONSTIFF_ORDER_FLOOR: f64 = 4.7;

/// y1' = lambda (y1 - cos s) - sin s + y2^2
/// y2' = -y2 + y1 y2
/// s'  = 1
///
/// The time is the third state because the exponential steps take
/// autonomous problems only. J and the nonlinearity do not commute
/// (J couples y1 and y2 through 2 y2 and y2, and both depend on the state).
fn stiff_coupled_problem(lambda: f64) -> OdeProblem {
    OdeProblem::new(
        "stiff-coupled-autonomous",
        3,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            out[0] = lambda * (y[0] - y[2].cos()) - y[2].sin() + y[1] * y[1];
            out[1] = -y[1] + y[0] * y[1];
            out[2] = 1.0;
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = lambda * v[0]
                + 2.0 * y[1] * v[1]
                + (lambda * y[2].sin() - y[2].cos()) * v[2];
            out[1] = y[1] * v[0] + (y[0] - 1.0) * v[1];
            out[2] = 0.0;
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn tight() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 16,
        dimension_increment: 1,
        relative_tolerance: 1.0e-14,
        absolute_tolerance: 1.0e-16,
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 8,
    }
}

fn integrate(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    steps: usize,
    config: FusedPhiKrylovConfig,
) -> (Vec<f64>, bool, f64) {
    let execution = ParallelExecution::sequential();
    let h = final_time / steps as f64;
    let mut y = y0.to_vec();
    let mut all_converged = true;
    let mut largest_phi_error = 0.0_f64;
    for step in 0..steps {
        let report =
            pexprb54s4_fused_step(problem, step as f64 * h, &y, h, config, &execution).unwrap();
        for phi in &report.fused_phi_reports {
            all_converged &= phi.converged;
            largest_phi_error = largest_phi_error.max(phi.error_estimate);
        }
        y = report.y_new;
    }
    (y, all_converged, largest_phi_error)
}

/// Observed global orders from successive differences y_N - y_2N, which
/// need no exact solution.
fn richardson_orders(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    steps: &[usize],
    config: FusedPhiKrylovConfig,
) -> Vec<f64> {
    let solutions = steps
        .iter()
        .map(|&n| integrate(problem, y0, final_time, n, config).0)
        .collect::<Vec<_>>();
    let differences = solutions
        .windows(2)
        .map(|pair| {
            safe_l2(
                &pair[0]
                    .iter()
                    .zip(&pair[1])
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    differences
        .windows(2)
        .map(|pair| (pair[0] / pair[1]).log2())
        .collect()
}

#[test]
fn stiff_coupled_problem_keeps_the_documented_stiff_order() {
    // |lambda| h runs from 100 down to 6.25: the stiff regime, where the
    // audit's exact-phi replica measured global order about 4.1.
    let problem = stiff_coupled_problem(-1.0e4);
    let orders = richardson_orders(
        &problem,
        &[1.0, 0.5, 0.0],
        0.1,
        &[10, 20, 40, 80, 160],
        tight(),
    );
    eprintln!("stiff lambda = -1e4 observed orders: {orders:?}");
    assert_eq!(orders.len(), 3);
    for order in &orders {
        assert!(
            *order >= STIFF_ORDER_FLOOR,
            "stiff lambda = -1e4 observed orders {orders:?}"
        );
    }
}

#[test]
fn nonstiff_coupled_problem_shows_the_declared_order() {
    let problem = stiff_coupled_problem(-1.0);
    let orders = richardson_orders(&problem, &[1.0, 0.5, 0.0], 1.0, &[5, 10, 20, 40], tight());
    eprintln!("nonstiff lambda = -1 observed orders: {orders:?}");
    assert_eq!(orders.len(), 2);
    for order in &orders {
        assert!(
            *order >= NONSTIFF_ORDER_FLOOR,
            "nonstiff lambda = -1 observed orders {orders:?}"
        );
    }
}

/// u' = k L u + u^2 / 2 on 64 points, spectrum of k L in (-4 k, 0).
fn diffusion_reaction(k: f64) -> OdeProblem {
    let n = 64;
    let laplacian = move |u: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i > 0 { u[i - 1] } else { 0.0 };
            let right = if i + 1 < n { u[i + 1] } else { 0.0 };
            out[i] = k * (left - 2.0 * u[i] + right);
        }
    };
    OdeProblem::new(
        "diffusion-reaction-64",
        n,
        Arc::new(move |_, u: &[f64], out: &mut [f64]| {
            laplacian(u, out);
            for i in 0..n {
                out[i] += 0.5 * u[i] * u[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, u: &[f64], v: &[f64], out: &mut [f64]| {
            laplacian(v, out);
            for i in 0..n {
                out[i] += u[i] * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn krylov_truncation_is_reported_and_its_order_is_measured() {
    let problem = diffusion_reaction(50.0);
    let u0 = (0..64)
        .map(|i| (std::f64::consts::PI * (i + 1) as f64 / 65.0).sin())
        .collect::<Vec<_>>();
    let execution = ParallelExecution::sequential();
    // One substep of dimension 4 cannot resolve h k L: the step refuses with
    // the residual estimate instead of returning a truncated result.
    let truncated = FusedPhiKrylovConfig {
        maximum_dimension: 4,
        maximum_substeps: 1,
        ..tight()
    };
    let error = pexprb54s4_fused_step(&problem, 0.0, &u0, 0.01, truncated, &execution)
        .expect_err("truncation must not pass silently");
    assert!(error.to_string().contains("estimate"), "{error}");

    // With substepping allowed the dimension-4 action converges; the
    // reports carry the truncation (dimension at the cap, several substeps,
    // a positive residual estimate), and the order is measured.
    let capped = FusedPhiKrylovConfig {
        maximum_dimension: 4,
        maximum_substeps: 256,
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 1.0e-14,
        ..tight()
    };
    let report = pexprb54s4_fused_step(&problem, 0.0, &u0, 0.01, capped, &execution).unwrap();
    assert!(report.fused_phi_reports.iter().all(|phi| phi.converged));
    assert!(report
        .fused_phi_reports
        .iter()
        .any(|phi| phi.maximum_krylov_dimension == 4 && phi.substeps > 1));
    assert!(report
        .fused_phi_reports
        .iter()
        .any(|phi| phi.error_estimate > 0.0));
    let orders = richardson_orders(&problem, &u0, 0.2, &[10, 20, 40, 80], capped);
    eprintln!("maximum_dimension = 4 observed orders: {orders:?}");
    assert!(orders.iter().all(|order| order.is_finite()), "{orders:?}");
}
