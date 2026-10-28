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
//! Accuracy is judged against the requested threshold plus an acceptance
//! allowance `eps ||tA||_1 ||exp(tA) v||`. The allowance is a tolerance for
//! this implementation, not a lower bound on what any algorithm can attain:
//! `||tA||` bounds the sensitivity to perturbations of the input, not the
//! error achievable on this exact input. SciPy's `expm` reaches 4e-16 at
//! k = 46, and a dyadic balancing that makes the matrix symmetric followed
//! by the closed form reaches 1e-17 (external re-audit RA-03, 2026-09-29).
//! With the Al-Mohy–Higham squaring rule of audit F-042 the full-space error
//! here is about 1e-11 relative, well inside the allowance.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, DenseOperator, LinearOperator, WorkCounters, safe_l2};
use rodas5p_integrators::{
    ExponentialKrylovConfig, FusedPhiKrylovConfig, FusedPhiPrefixSession, PhiConvergenceBasis,
    fused_phi_action, fused_phi_action_incremental, krylov_phi_action,
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

fn l2_error(value: &[f64], exact: &[f64]) -> f64 {
    safe_l2(
        &value
            .iter()
            .zip(exact)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    )
}

#[test]
fn full_space_reports_carry_a_bound_basis_and_meet_the_threshold() {
    // Re-audit RA-02: only an invariant or full Krylov space removes the
    // projection error. Those reports say so, and on this case they are
    // accurate to the requested 1e-10 without the conditioning allowance.
    let (operator, small, _) = nonnormal(46);
    let expected = exact(small, 1.0, 1.0);
    let legacy = krylov_phi_action(
        operator.clone(),
        1.0,
        0,
        &[1.0, 0.0],
        legacy_config(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    let fused = fused_phi_action(
        operator,
        1.0,
        &[vec![1.0, 0.0]],
        fused_config(),
        &mut WorkCounters::default(),
    )
    .unwrap();
    for (label, basis, value) in [
        ("legacy", legacy.convergence_basis, &legacy.value),
        ("fused", fused.convergence_basis, &fused.value),
    ] {
        assert!(basis.error_bound_available(), "{label}: {basis:?}");
        let error = l2_error(value, &expected);
        eprintln!("{label} k=46 full space: basis {basis:?}, error {error:e}");
        assert!(error <= 1.0e-10 * safe_l2(&expected), "{label}: {error:e}");
    }
}

#[test]
fn an_ordinary_checkpoint_is_an_estimate_not_a_bound() {
    // Re-audit RA-02, known limitation kept visible. At an ordinary
    // checkpoint the KIOPS residual term measures one direction; A feeds the
    // 2^-46 leak back with gain 2^46. With minimum_dimension = 1, and on the
    // prefix session, the action "converges" at dimension 1 with a true
    // error near 7e-2. The report must not present that as bounded.
    let (operator, small, _) = nonnormal(46);
    let expected = exact(small, 1.0, 1.0);
    let one_dimensional = FusedPhiKrylovConfig {
        minimum_dimension: 1,
        ..fused_config()
    };
    let fused = fused_phi_action(
        operator.clone(),
        1.0,
        &[vec![1.0, 0.0]],
        one_dimensional,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let single_substep = FusedPhiKrylovConfig {
        maximum_substeps: 1,
        ..fused_config()
    };
    let incremental = fused_phi_action_incremental(
        operator.clone(),
        1.0,
        &[vec![1.0, 0.0]],
        single_substep,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let prefix = FusedPhiPrefixSession::begin(
        operator,
        1.0,
        &[vec![1.0, 0.0]],
        single_substep,
        1,
        &mut WorkCounters::default(),
    )
    .unwrap()
    .finish(&mut WorkCounters::default())
    .unwrap();
    for (label, report) in [
        ("fused minimum_dimension = 1", &fused),
        ("incremental", &incremental),
        ("prefix session", &prefix),
    ] {
        let error = l2_error(&report.value, &expected);
        eprintln!(
            "{label}: converged {}, dimension {}, estimate {:e}, basis {:?}, true error {error:e}",
            report.converged,
            report.maximum_krylov_dimension,
            report.error_estimate,
            report.convergence_basis
        );
        if report.converged && error > 1.0e-10 * safe_l2(&expected) {
            assert_eq!(
                report.convergence_basis,
                PhiConvergenceBasis::ResidualEstimate,
                "{label}: an inaccurate converged report must be labelled an estimate"
            );
            assert!(!report.convergence_basis.error_bound_available());
        }
        if report.convergence_basis.error_bound_available() {
            assert!(error <= 1.0e-10 * safe_l2(&expected), "{label}: {error:e}");
        }
    }
    // The limitation is still present; this sentinel fails, and must be
    // updated, once an ordinary checkpoint stops accepting this case.
    assert!(
        fused.converged
            && fused.maximum_krylov_dimension == 1
            && l2_error(&fused.value, &expected) > 1.0e-3,
        "the one-dimensional checkpoint no longer accepts the k = 46 case"
    );
}
