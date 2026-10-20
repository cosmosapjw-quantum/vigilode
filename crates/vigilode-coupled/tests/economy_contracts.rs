//! These tests initially target the parent eager implementation (RED).
//! After the observed failure, only this import will bind the new policy.
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::Arc;
use vigilode_coupled::coupled_step_residual_first as subject;
use vigilode_coupled::{Config, FailureKind, coupled_step};
fn model(eps: f64) -> OdeProblem {
    OdeProblem::new(
        "three-mode-economy",
        96,
        Arc::new(move |_, y, f| {
            for i in 0..96 {
                let a = [-1.0, -20.0, -100.0][i % 3];
                f[i] = a * y[i] + eps * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, f| {
            for i in 0..96 {
                f[i] = ([-1.0, -20.0, -100.0][i % 3] + 2.0 * eps * y[i]) * v[i];
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
fn converged_affine_seed_spends_no_stage_derivative_jvps() {
    let p = model(0.0);
    let r = subject(
        &p,
        0.0,
        &[0.1; 96],
        0.001,
        &Config::default(),
        &ParallelExecution::sequential(),
    );
    assert!(r.candidate.is_some(), "{:?}", r.failure);
    assert_eq!(r.work.newton_updates, 0);
    assert_eq!(
        r.work.stage_jvp, 0,
        "a converged iterate needs no Newton derivative"
    );
    assert_eq!(
        r.work.validation_jvp, 8,
        "full-space verification must still execute"
    );
}
#[test]
fn nonlinear_derivatives_are_computed_only_for_actual_updates() {
    let p = model(1.0);
    let cfg = Config::default();
    let r = subject(
        &p,
        0.0,
        &[0.1; 96],
        0.01,
        &cfg,
        &ParallelExecution::rayon(4).unwrap(),
    );
    let c = r.candidate.as_ref().unwrap();
    assert_eq!(c.basis_dimension, 3);
    assert_eq!(
        r.work.stage_jvp,
        8 * c.basis_dimension as u64 * r.work.newton_updates,
        "the last converged iterate must not compute an unused Jacobian"
    );
}
#[test]
fn exhausted_update_budget_spends_no_unused_derivative() {
    let p = model(1.0);
    let cfg = Config {
        theta: 0.0,
        max_newton: 0,
        max_subdivisions: 0,
        ..Config::default()
    };
    let r = subject(
        &p,
        0.0,
        &[0.1; 96],
        0.01,
        &cfg,
        &ParallelExecution::sequential(),
    );
    assert_eq!(r.failure.as_ref().unwrap().kind, FailureKind::RootIteration);
    assert_eq!(r.work.stage_jvp, 0);
    assert_eq!(r.work.newton_updates, 0);
}
#[test]
fn policy_preserves_complete_state_and_embedded_bits() {
    let p = model(1.0);
    let y = [0.1; 96];
    let cfg = Config::default();
    let exec = ParallelExecution::rayon(4).unwrap();
    let a = coupled_step(&p, 0.0, &y, 0.01, &cfg, &exec)
        .candidate
        .unwrap();
    let b = subject(&p, 0.0, &y, 0.01, &cfg, &exec).candidate.unwrap();
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&a.y_new), bits(&b.y_new));
    assert_eq!(bits(&a.error_vector), bits(&b.error_vector));
    assert_eq!(
        a.full_residual_wrms.to_bits(),
        b.full_residual_wrms.to_bits()
    );
    assert_eq!(y, [0.1; 96]);
}
