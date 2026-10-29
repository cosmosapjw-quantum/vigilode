//! One projected Newton correction is not a nonlinear error bound
//! (external re-audit RA-04, 2026-09-29).
//!
//! y1' = 1 - y1, y2' = c y1^2 from y = 0. With zero candidate stages every
//! stage state has y1 = 0, so the target Jacobian's coupling 2 c y1 vanishes
//! at the candidate and the correction to y2 is lost. The true output error
//! is taken from the exact sequential stages of the same target.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, LinearSolverConfig, WorkCounters, error_scale, wrms};
use rodas5p_integrators::{
    KrylovState, NonlinearOutputCertificate, OdeProblem, StructuredBlockSystem, build_step_context,
    certify_nonlinear_target, certify_second_correction, sequential_stages,
};

fn quadratic_coupling(c: f64) -> OdeProblem {
    OdeProblem::new(
        "quadratic-coupling",
        2,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            out[0] = 1.0 - y[0];
            out[1] = c * y[0] * y[0];
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, y: &[f64]| {
            DenseMatrix::from_rows(&[&[-1.0, 0.0], &[2.0 * c * y[0], 0.0]])
        })),
        Some(Arc::new(move |_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = -v[0];
            out[1] = 2.0 * c * y[0] * v[0];
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
fn a_projected_newton_correction_can_underestimate_the_output_error() {
    let (atol, rtol) = (1.0e-6, 1.0e-4);
    let problem = quadratic_coupling(100.0);
    let y0 = vec![0.0, 0.0];
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 0.5, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let exact = sequential_stages(
        &context,
        &LinearSolverConfig::default(),
        Option::<&mut KrylovState>::None,
        &mut counters,
    )
    .unwrap()
    .stages;
    let candidate = vec![vec![0.0; 2]; block.s];

    let certificate =
        certify_nonlinear_target(&block, &candidate, atol, rtol, &mut counters).unwrap();
    assert_eq!(
        NonlinearOutputCertificate::KIND,
        "linearized-output-correction"
    );
    assert!(!certificate.is_error_bound());

    let output = |stages: &[Vec<f64>]| {
        let mut y = y0.clone();
        for (weight, stage) in context.coeffs.b.iter().zip(stages) {
            for (value, increment) in y.iter_mut().zip(stage) {
                *value += weight * increment;
            }
        }
        y
    };
    let (candidate_output, exact_output) = (output(&candidate), output(&exact));
    let scale = error_scale(&y0, &candidate_output, &[atol], rtol).unwrap();
    let difference = candidate_output
        .iter()
        .zip(&exact_output)
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();
    let true_error = wrms(&difference, &scale).unwrap();
    // Measured: linearized 2.78e5 against true 2.08e6.
    assert!(
        true_error > 5.0 * certificate.output_wrms,
        "linearized {:e} vs true {true_error:e}",
        certificate.output_wrms
    );

    // The second-correction diagnostic sees the unresolved remainder: the
    // residual after the first correction does not contract.
    let diagnostic =
        certify_second_correction(&block, &candidate, atol, rtol, false, &mut counters).unwrap();
    assert!(
        diagnostic.second_output_wrms > 0.1 * diagnostic.first_output_wrms,
        "{diagnostic:?}"
    );
}

#[test]
fn at_the_root_the_linearized_correction_vanishes() {
    let problem = quadratic_coupling(100.0);
    let y0 = vec![0.0, 0.0];
    let mut counters = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 0.5, &mut counters).unwrap();
    let block = StructuredBlockSystem::new(&context);
    let exact = sequential_stages(
        &context,
        &LinearSolverConfig::default(),
        Option::<&mut KrylovState>::None,
        &mut counters,
    )
    .unwrap()
    .stages;
    let certificate =
        certify_nonlinear_target(&block, &exact, 1.0e-6, 1.0e-4, &mut counters).unwrap();
    assert!(certificate.output_wrms < 1.0e-6, "{certificate:?}");
}
