//! A1 defect (production plan 2026-10-20, section 6): accepted/rejected
//! counter drift in the transactional q1/q2 drivers.
//!
//! The step kernel counts a step accepted when its embedded error is at most
//! one, while the adaptive driver rejects it when the embedded error plus the
//! fixed-point contraction tail exceeds one. Without
//! `reconcile_outer_rejection` the work counters then report one accepted
//! step too many and one rejection too few. With a large algebraic-tail
//! allowance (`absolute_output_budget`, `embedded_budget_fraction`) the
//! operational gate passes steps whose tail is not small, so the outer
//! rejection happens; the counters must equal the attempt ledger.

use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSamplingPlan, OutputSchedule, TransactionalQ1Q2AdaptiveResult,
    TransactionalQ1Q2Config, integrate_transactional_q1_q2_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_observed, prothero_robinson_problem,
};

const TF: f64 = 0.5;

fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-6,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        min_step: 1.0e-12,
        max_step: TF,
        max_attempts: 20_000,
        ..AdaptiveStepConfig::default()
    }
}

/// A permissive tail allowance: the operational gate passes a step whose
/// contraction tail is of the order of the tolerance.
fn loose_tail() -> TransactionalQ1Q2Config {
    TransactionalQ1Q2Config {
        absolute_output_budget: 10.0,
        embedded_budget_fraction: 10.0,
        gmres_rtol: 1.0e-2,
        gmres_atol: 1.0e-4,
        max_target_relative_residual: 1.0,
        ..TransactionalQ1Q2Config::default()
    }
}

/// The work counters equal the attempt ledger: every attempt is counted
/// once, accepted attempts are the retained steps, and the outer rejections
/// are rejections.
fn check_ledger(result: &TransactionalQ1Q2AdaptiveResult, what: &str) {
    let c = &result.observed.counters;
    let d = &result.diagnostics;
    assert!(
        result.observed.success,
        "{what}: {}",
        result.observed.message
    );
    assert!(d.is_structurally_consistent(), "{what}: ledger");
    assert!(d.rejected_macro_steps > 0, "{what}: no rejection exercised");
    assert_eq!(
        c.accepted_steps + c.rejected_steps,
        d.attempts as u64,
        "{what}: accepted {} + rejected {} != attempts {}",
        c.accepted_steps,
        c.rejected_steps,
        d.attempts
    );
    assert_eq!(
        (c.accepted_steps, c.rejected_steps),
        (d.accepted_macro_steps as u64, d.rejected_macro_steps as u64),
        "{what}: counters (accepted, rejected) vs ledger"
    );
    assert_eq!(
        c.accepted_steps, result.observed.internal_steps as u64,
        "{what}: accepted steps vs retained steps"
    );
}

/// Prothero-Robinson with lambda = -1e5: with the loose tail allowance the
/// kernel accepts steps (embedded error <= 1) that the driver rejects
/// (embedded error + contraction tail > 1). Before the fix the observed
/// driver reported 176 accepted / 169 rejected for 91 retained steps.
fn stiff_case() -> (rodas5p_integrators::OdeProblem, Vec<f64>) {
    prothero_robinson_problem(-1.0e5, 10.0, 0.0)
}

#[test]
fn observed_driver_reconciles_outer_tail_rejections() {
    let (problem, y0) = stiff_case();
    let result = integrate_transactional_q1_q2_adaptive_observed(
        &problem,
        (0.0, TF),
        &y0,
        &loose_tail(),
        &adaptive(),
        &OutputSchedule::new(vec![0.0, TF]).unwrap(),
    )
    .unwrap();
    check_ledger(&result, "observed");
}

#[test]
fn dense_driver_reconciles_outer_tail_rejections() {
    let (problem, y0) = stiff_case();
    let sampling = OutputSamplingPlan::dense(
        OutputSchedule::new((0..=10).map(|i| i as f64 * TF / 10.0).collect()).unwrap(),
    );
    let result = integrate_transactional_q1_q2_adaptive_dense_observed(
        &problem,
        (0.0, TF),
        &y0,
        &loose_tail(),
        &adaptive(),
        &sampling,
    )
    .unwrap();
    check_ledger(&result, "dense");
}

/// The default configuration on the same problem keeps a consistent ledger.
#[test]
fn default_configuration_ledger_is_consistent() {
    let (problem, y0) = stiff_case();
    let result = integrate_transactional_q1_q2_adaptive_observed(
        &problem,
        (0.0, TF),
        &y0,
        &TransactionalQ1Q2Config::default(),
        &adaptive(),
        &OutputSchedule::new(vec![0.0, TF]).unwrap(),
    )
    .unwrap();
    let c = &result.observed.counters;
    let d = &result.diagnostics;
    assert_eq!(c.accepted_steps + c.rejected_steps, d.attempts as u64);
    assert_eq!(c.accepted_steps, d.accepted_macro_steps as u64);
    assert_eq!(c.rejected_steps, d.rejected_macro_steps as u64);
}
