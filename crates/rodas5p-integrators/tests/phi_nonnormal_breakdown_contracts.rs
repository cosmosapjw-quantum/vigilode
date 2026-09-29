//! A small Arnoldi residual is not a small forward error (external audit
//! VIG-A02, 2026-09-29).
//!
//! `A = [[-2, 2^k], [2^-k, -2]]` has eigenvalues -1 and -3 for every k, and
//! `N = A + 2I` satisfies `N^2 = I`, so
//! `exp(tA) e1 = exp(-2t) (cosh t, 2^-k sinh t)` exactly. From `v = e1` the
//! first Arnoldi step leaves the residual `2^-k e2`, which is tiny for large
//! k, but `A` feeds it back with gain `2^k`. Stopping on that residual and
//! trusting `|t h_{2,1} e_1^T phi_1(t H_1) e_1|` returns `exp(-2t) e1` with a
//! true error near 7e-2 at t = 1.
//!
//! These contracts pin the near-breakdown path only: a report that says
//! `converged` from a *nonzero* residual below the old breakdown tolerance
//! must not hide that feedback. The KIOPS residual term at an ordinary
//! checkpoint remains an estimate, not a bound, for nonnormal operators.
//!
//! Accuracy is judged against the requested threshold plus the normwise
//! conditioning allowance `eps ||tA||_1 ||exp(tA) v||`: `exp(tA) v` has a
//! relative condition number of order `||tA||`, so no normwise backward-stable
//! method reaches 1e-10 at `||A|| = 2^46`. Measured on the full space, the
//! relative error is about `0.15 eps ||tA||_1` (2e-3 at k = 46); the hidden
//! near-breakdown error was 0.35 relative, about 22 times the allowance.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, DenseOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_integrators::{
    ExponentialKrylovConfig, FusedPhiKrylovConfig, fused_phi_action, krylov_phi_action,
};

/// Operator, `2^-k`, and `||A||_1 = 2^k + 2` (for k >= 0).
fn nonnormal(k: i32) -> (Arc<dyn LinearOperator>, f64, f64) {
    let small = 2.0_f64.powi(-k);
    let large = 2.0_f64.powi(k);
    let matrix = DenseMatrix::from_rows(&[&[-2.0, large], &[small, -2.0]]).unwrap();
    (
        Arc::new(DenseOperator::new(matrix).unwrap()),
        small,
        large + 2.0,
    )
}

fn exact(small: f64, t: f64, c: f64) -> Vec<f64> {
    let decay = (-2.0 * t).exp();
    vec![c * decay * t.cosh(), c * decay * small * t.sinh()]
}

fn assert_no_hidden_error(
    label: &str,
    converged: bool,
    value: &[f64],
    exact: &[f64],
    beta: f64,
    t_norm: f64,
) {
    let error = safe_l2(
        &value
            .iter()
            .zip(exact)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    let threshold = 1.0e-10 * safe_l2(value).max(beta);
    let conditioning = f64::EPSILON * t_norm * safe_l2(exact);
    assert!(
        !converged || error <= threshold + conditioning,
        "{label}: converged with true error {error:e} > threshold {threshold:e} \
         + conditioning allowance {conditioning:e}"
    );
}

fn legacy_config() -> ExponentialKrylovConfig {
    ExponentialKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: 2,
        dimension_increment: 1,
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 0.0,
        reorthogonalize: true,
    }
}

fn fused_config() -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 0.0,
        ..FusedPhiKrylovConfig::default()
    }
}

#[test]
fn legacy_near_breakdown_does_not_hide_stable_nonnormal_feedback() {
    let (operator, small, norm) = nonnormal(46);
    let report = krylov_phi_action(
        operator,
        1.0,
        0,
        &[1.0, 0.0],
        legacy_config(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert_no_hidden_error(
        "legacy k=46",
        report.converged,
        &report.value,
        &exact(small, 1.0, 1.0),
        1.0,
        norm,
    );
    assert!(
        report.converged,
        "the full two-dimensional space is reached"
    );
    assert_eq!(
        report.krylov_dimension, 2,
        "a nonzero residual extends the basis"
    );
}

#[test]
fn fused_near_breakdown_does_not_hide_stable_nonnormal_feedback() {
    let (operator, small, norm) = nonnormal(46);
    let report = fused_phi_action(
        operator,
        1.0,
        &[vec![1.0, 0.0]],
        fused_config(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert_no_hidden_error(
        "fused k=46",
        report.converged,
        &report.value,
        &exact(small, 1.0, 1.0),
        1.0,
        norm,
    );
    assert!(
        report.converged,
        "the full two-dimensional space is reached"
    );
    assert_eq!(
        report.maximum_krylov_dimension, 2,
        "a nonzero residual extends the basis"
    );
}

#[test]
fn nonnormal_family_across_feedback_vector_and_time_scales() {
    for k in [0, 10, 20, 30, 40, 46, 52] {
        for c in [1.0e-8, 1.0, 1.0e8] {
            for t in [0.5, 1.0] {
                let (operator, small, norm) = nonnormal(k);
                let expected = exact(small, t, c);
                let legacy = krylov_phi_action(
                    operator.clone(),
                    t,
                    0,
                    &[c, 0.0],
                    legacy_config(),
                    &mut WorkCounters::default(),
                )
                .unwrap();
                assert_no_hidden_error(
                    &format!("legacy k={k} c={c:e} t={t}"),
                    legacy.converged,
                    &legacy.value,
                    &expected,
                    c,
                    t * norm,
                );
                let fused = fused_phi_action(
                    operator,
                    t,
                    &[vec![c, 0.0]],
                    fused_config(),
                    &mut WorkCounters::default(),
                )
                .unwrap();
                assert_no_hidden_error(
                    &format!("fused k={k} c={c:e} t={t}"),
                    fused.converged,
                    &fused.value,
                    &expected,
                    c,
                    t * norm,
                );
            }
        }
    }
}

#[test]
fn an_exactly_invariant_start_vector_still_stops_at_dimension_one() {
    // Diagonal control: A e1 = -e1 leaves an exactly zero residual.
    let matrix = DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -3.0]]).unwrap();
    let operator: Arc<dyn LinearOperator> = Arc::new(DenseOperator::new(matrix).unwrap());
    let report = krylov_phi_action(
        operator,
        1.0,
        0,
        &[1.0, 0.0],
        legacy_config(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(report.converged);
    assert!(report.happy_breakdown);
    assert_eq!(report.krylov_dimension, 1);
    assert!((report.value[0] - (-1.0_f64).exp()).abs() <= 1.0e-15);
    assert_eq!(report.value[1], 0.0);
}
