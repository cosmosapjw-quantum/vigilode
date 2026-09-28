//! WU-5 (audit F-011, F-040): a phi-action report that says `converged`
//! must not understate its own error, and the dense phi oracle must not lose
//! accuracy with the physical scale of the vector.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, DenseOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_integrators::{
    ExponentialKrylovConfig, FusedPhiKrylovConfig, fused_phi_action, krylov_phi_action,
};

/// `phi_k(z)` for scalar `z`, from the defining recurrence.
fn scalar_phi(k: usize, z: f64) -> f64 {
    let mut value = z.exp();
    let mut factorial = 1.0;
    for j in 0..k {
        // phi_{j+1}(z) = (phi_j(z) - 1/j!) / z
        value = (value - 1.0 / factorial) / z;
        factorial *= (j + 1) as f64;
    }
    value
}

/// A = diag(-1, -3), v = e1 + 4.5e-7 e2: the Arnoldi residual after one
/// vector is about 9e-7, just under the old breakdown test 64 sqrt(eps).
fn near_invariant_case() -> (Arc<dyn LinearOperator>, Vec<f64>, [f64; 2]) {
    let matrix = DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -3.0]]).unwrap();
    let operator: Arc<dyn LinearOperator> = Arc::new(DenseOperator::new(matrix).unwrap());
    (operator, vec![1.0, 4.5e-7], [-1.0, -3.0])
}

fn true_error(value: &[f64], vector: &[f64], eigenvalues: [f64; 2], k: usize) -> f64 {
    let exact: Vec<f64> = vector
        .iter()
        .zip(eigenvalues)
        .map(|(v, lambda)| v * scalar_phi(k, lambda))
        .collect();
    safe_l2(
        &value
            .iter()
            .zip(&exact)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    )
}

#[test]
fn legacy_krylov_phi_action_does_not_certify_an_understated_error() {
    let (operator, vector, eigenvalues) = near_invariant_case();
    let config = ExponentialKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 2,
        dimension_increment: 1,
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 0.0,
        reorthogonalize: true,
    };
    for k in 0..=2 {
        let mut work = WorkCounters::default();
        let report =
            krylov_phi_action(operator.clone(), 1.0, k, &vector, config, &mut work).unwrap();
        let error = true_error(&report.value, &vector, eigenvalues, k);
        let threshold = config.relative_tolerance * safe_l2(&report.value).max(safe_l2(&vector));
        println!(
            "legacy phi_{k}: converged={} happy_breakdown={} krylov_dimension={} \
             error_estimate={:.3e} true_error={error:.3e} threshold={threshold:.3e}",
            report.converged,
            report.happy_breakdown,
            report.krylov_dimension,
            report.error_estimate
        );
        assert!(
            !report.converged || error <= threshold.max(report.error_estimate),
            "phi_{k}: converged with estimate {:.3e} but true error {error:.3e} > threshold {threshold:.3e}",
            report.error_estimate
        );
    }
}

#[test]
fn fused_phi_action_does_not_certify_an_understated_error() {
    let (operator, vector, eigenvalues) = near_invariant_case();
    let config = FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 0.0,
        ..FusedPhiKrylovConfig::default()
    };
    let mut work = WorkCounters::default();
    let report = fused_phi_action(
        operator,
        1.0,
        std::slice::from_ref(&vector),
        config,
        &mut work,
    )
    .unwrap();
    let error = true_error(&report.value, &vector, eigenvalues, 0);
    let threshold = config.relative_tolerance * safe_l2(&report.value).max(safe_l2(&vector));
    println!(
        "fused exp: converged={} substeps={} krylov_dimension={} error_estimate={:.3e} \
         true_error={error:.3e} threshold={threshold:.3e} substep_reports={:?}",
        report.converged,
        report.substeps,
        report.maximum_krylov_dimension,
        report.error_estimate,
        report.substep_reports
    );
    assert!(
        !report.converged || error <= threshold.max(report.error_estimate),
        "converged with estimate {:.3e} but true error {error:.3e} > threshold {threshold:.3e}",
        report.error_estimate
    );
}

/// Stiff autonomous 2-D problem with non-commuting J and N; every right-hand
/// side evaluation is counted outside the solver.
fn counted_stiff_autonomous_problem(
    calls: Arc<std::sync::atomic::AtomicU64>,
) -> rodas5p_integrators::OdeProblem {
    use std::sync::atomic::Ordering;
    rodas5p_integrators::OdeProblem::new(
        "counted-stiff-autonomous",
        2,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            calls.fetch_add(1, Ordering::Relaxed);
            out[0] = -2.0e3 * y[0] + y[1] * y[1];
            out[1] = -y[1] + y[0] * y[1];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = -2.0e3 * v[0] + 2.0 * y[1] * v[1];
            out[1] = y[1] * v[0] + (y[0] - 1.0) * v[1];
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
fn adaptive_fused_exponential_keeps_the_work_of_failed_trials() {
    // Audit F-039: a trial whose phi action did not converge was rejected and
    // its right-hand-side and JVP work was dropped from the counters.
    use rodas5p_integrators::{
        AdaptiveStepConfig, OutputSchedule, ParallelExecution,
        integrate_pexprb54s4_fused_adaptive_observed,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    let calls = Arc::new(AtomicU64::new(0));
    let problem = counted_stiff_autonomous_problem(calls.clone());
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 0.5,
        ..AdaptiveStepConfig::default()
    };
    // Two Krylov vectors and one substep cannot resolve exp(0.5 * 2e3): the
    // first trials fail and force step rejections.
    let phi_config = FusedPhiKrylovConfig {
        minimum_dimension: 2,
        maximum_dimension: 3,
        dimension_increment: 1,
        maximum_substeps: 1,
        ..FusedPhiKrylovConfig::default()
    };
    let output = OutputSchedule::new(vec![0.0, 0.1]).unwrap();
    let run = integrate_pexprb54s4_fused_adaptive_observed(
        &problem,
        (0.0, 0.1),
        &[1.0, 0.5],
        &adaptive,
        &output,
        phi_config,
        &ParallelExecution::sequential(),
    )
    .unwrap();
    println!(
        "success={} accepted={} rejected={} rhs={} counted={}",
        run.observed.success,
        run.diagnostics.accepted_steps,
        run.diagnostics.rejected_steps,
        run.observed.counters.rhs_evaluations,
        calls.load(Ordering::Relaxed)
    );
    assert!(run.observed.success);
    assert!(
        run.diagnostics.rejected_steps > 0,
        "the configuration must force failed trials"
    );
    assert_eq!(
        run.observed.counters.rhs_evaluations,
        calls.load(Ordering::Relaxed),
        "every right-hand-side evaluation, including failed trials, must be counted"
    );
}
