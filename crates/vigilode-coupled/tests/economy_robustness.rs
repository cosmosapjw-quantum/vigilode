use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters};
use rodas5p_integrators::{OdeProblem, ParallelExecution, sequential_step};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use vigilode_coupled::{
    Config, FailureKind, coupled_or_sequential_budgeted, coupled_step, coupled_step_residual_first,
};
fn model(eps: f64) -> OdeProblem {
    OdeProblem::new(
        "few-mode-policy",
        96,
        Arc::new(move |_, y, f| {
            for i in 0..96 {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i] + eps * y[i] * y[i];
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
fn linear() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        rtol: 1e-12,
        atol: 1e-14,
        maxiter: 512,
        ..LinearSolverConfig::default()
    }
}
fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}
#[test]
fn lazy_policy_makes_the_same_valid_candidate_with_smaller_jvp_budget() {
    let p = model(1.0);
    let c = Config {
        max_jvp: 64,
        ..Config::default()
    };
    let e = ParallelExecution::rayon(4).unwrap();
    let old = coupled_step(&p, 0.0, &[0.1; 96], 0.01, &c, &e);
    assert!(old.candidate.is_none());
    let new = coupled_step_residual_first(&p, 0.0, &[0.1; 96], 0.01, &c, &e);
    assert!(new.candidate.is_some(), "{:?}", new.failure);
    assert_eq!(new.work.jvp_attempted, 59);
    assert_eq!(new.work.unused_stage_jvp_avoided, 24);
    assert_eq!(new.work.derivative_batches, 2);
    assert_eq!(new.work.rhs_attempted, 33);
    assert_eq!(new.work.validation_jvp, 8);
}
#[test]
fn zero_candidate_cap_falls_back_without_spending_any_jvp() {
    let p = model(1.0);
    let y = [0.1; 96];
    let c = Config::default();
    let l = linear();
    let e = ParallelExecution::sequential();
    let r = coupled_or_sequential_budgeted(&p, 0.0, &y, 0.001, &c, &e, &l, 0);
    assert_eq!(r.candidate_attempt.work.jvp_attempted, 0);
    assert_eq!(
        r.candidate_attempt.failure.unwrap().kind,
        FailureKind::Budget
    );
    assert!(r.fallback_attempted);
    let fallback = r.fallback.unwrap();
    let mut w = WorkCounters::default();
    let direct =
        sequential_step(&p, 0.0, &y, 0.001, &l, None, c.atol, c.rtol, false, &mut w).unwrap();
    assert_eq!(bits(&fallback.y_new), bits(&direct.y_new));
    assert_eq!(bits(&fallback.error_vector), bits(&direct.error_vector));
    assert_eq!(y, [0.1; 96]);
}
#[test]
fn candidate_cap_never_enlarges_a_stricter_caller_budget() {
    let c = Config {
        max_jvp: 2,
        ..Config::default()
    };
    let r = coupled_or_sequential_budgeted(
        &model(1.0),
        0.0,
        &[0.1; 96],
        0.001,
        &c,
        &ParallelExecution::sequential(),
        &linear(),
        256,
    );
    assert!(r.candidate_attempt.work.jvp_attempted <= 2);
    assert!(r.candidate_attempt.candidate.is_none());
    assert!(r.fallback_attempted);
}
#[test]
fn invalid_input_does_not_trigger_a_hidden_fallback() {
    let r = coupled_or_sequential_budgeted(
        &model(1.0),
        0.0,
        &[0.1; 96],
        f64::NAN,
        &Config::default(),
        &ParallelExecution::sequential(),
        &linear(),
        64,
    );
    assert!(!r.fallback_attempted);
    assert_eq!(
        r.candidate_attempt.failure.unwrap().kind,
        FailureKind::Input
    );
}
#[test]
fn derivative_phase_obeys_the_actual_batch_budget() {
    let c = Config {
        max_batches: 1,
        ..Config::default()
    };
    let r = coupled_step_residual_first(
        &model(1.0),
        0.0,
        &[0.1; 96],
        0.01,
        &c,
        &ParallelExecution::sequential(),
    );
    assert_eq!(r.failure.unwrap().kind, FailureKind::Budget);
    assert_eq!(r.work.rhs_attempted, 9);
    assert_eq!(r.work.stage_jvp, 0);
    assert_eq!(r.work.stage_batches, 1);
    assert_eq!(r.work.derivative_batches, 0);
    assert_eq!(r.work.stage_tasks_completed, 8);
}
#[test]
fn failed_residual_batch_never_launches_derivatives_and_counts_all_tasks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cc = calls.clone();
    let p = OdeProblem::new(
        "failed-residual",
        3,
        Arc::new(move |_, y, f| {
            let k = cc.fetch_add(1, Ordering::SeqCst);
            if k == 2 {
                return Err(CoreError::NonFinite("test RHS failure".into()));
            }
            for i in 0..3 {
                f[i] = (-1.0 - i as f64) * y[i] + y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y, v, f| {
            for i in 0..3 {
                f[i] = (-1.0 - i as f64 + 2.0 * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let r = coupled_step_residual_first(
        &p,
        0.0,
        &[0.1; 3],
        0.01,
        &Config::default(),
        &ParallelExecution::rayon(4).unwrap(),
    );
    assert!(r.candidate.is_none());
    assert_eq!(r.work.rhs_attempted, calls.load(Ordering::SeqCst) as u64);
    assert_eq!(r.work.rhs_attempted, 9);
    assert_eq!(r.work.rhs_completed, 8);
    assert_eq!(r.work.stage_jvp, 0);
    assert_eq!(r.work.derivative_batches, 0);
    assert_eq!(r.work.stage_tasks_completed, 8);
}
#[test]
fn failed_derivative_batch_joins_and_preserves_every_attempt() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cc = calls.clone();
    let failed = Arc::new(AtomicBool::new(false));
    let ff = failed.clone();
    let p = OdeProblem::new(
        "failed-derivative",
        3,
        Arc::new(|_, y, f| {
            for i in 0..3 {
                f[i] = (-1.0 - i as f64) * y[i] + y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, f| {
            cc.fetch_add(1, Ordering::SeqCst);
            if y.iter().any(|x| x.to_bits() != 0.1f64.to_bits()) && !ff.swap(true, Ordering::SeqCst)
            {
                return Err(CoreError::NonFinite("test derivative failure".into()));
            }
            for i in 0..3 {
                f[i] = (-1.0 - i as f64 + 2.0 * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let r = coupled_step_residual_first(
        &p,
        0.0,
        &[0.1; 3],
        0.01,
        &Config::default(),
        &ParallelExecution::rayon(4).unwrap(),
    );
    assert!(r.candidate.is_none());
    assert!(failed.load(Ordering::SeqCst));
    assert_eq!(r.work.jvp_attempted, calls.load(Ordering::SeqCst) as u64);
    assert_eq!(r.work.jvp_attempted - r.work.jvp_completed, 1);
    assert_eq!(r.work.derivative_batches, 1);
    assert_eq!(r.work.newton_updates, 0);
    assert_eq!(r.work.stage_tasks_scheduled, 16);
    assert_eq!(r.work.stage_tasks_completed, 16);
}
#[test]
fn epoch_change_between_phases_is_detected_before_derivative_dispatch() {
    let epoch = Arc::new(AtomicU64::new(0));
    let ep = epoch.clone();
    let query = epoch.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let cc = calls.clone();
    let p = OdeProblem::new(
        "epoch-boundary",
        3,
        Arc::new(move |_, y, f| {
            let k = cc.fetch_add(1, Ordering::SeqCst);
            if k == 2 {
                ep.fetch_add(1, Ordering::SeqCst);
            }
            for i in 0..3 {
                f[i] = (-1.0 - i as f64) * y[i] + y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y, v, f| {
            for i in 0..3 {
                f[i] = (-1.0 - i as f64 + 2.0 * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_model_epoch(Arc::new(move || query.load(Ordering::SeqCst)));
    let r = coupled_step_residual_first(
        &p,
        0.0,
        &[0.1; 3],
        0.01,
        &Config::default(),
        &ParallelExecution::rayon(4).unwrap(),
    );
    assert_eq!(r.failure.unwrap().kind, FailureKind::ModelChanged);
    assert_eq!(r.work.stage_jvp, 0);
    assert_eq!(r.work.derivative_batches, 0);
    assert_eq!(r.work.stage_tasks_completed, 8);
}
#[test]
fn full_space_validation_is_not_skipped_on_affine_convergence() {
    let r = coupled_step_residual_first(
        &model(0.0),
        0.0,
        &[0.1; 96],
        0.001,
        &Config::default(),
        &ParallelExecution::sequential(),
    );
    assert!(r.candidate.is_some());
    assert_eq!(r.work.stage_jvp, 0);
    assert_eq!(r.work.validation_jvp, 8);
    assert_eq!(r.work.validation_batches, 1);
    assert_eq!(r.work.rhs_attempted, 17);
}
#[test]
fn worker_and_theta_variants_preserve_eager_solution_bits() {
    for eps in [0.0, 1.0, 10.0] {
        for theta in [0.0, 0.4, 1.0] {
            for threads in [1, 4] {
                let c = Config {
                    theta,
                    ..Config::default()
                };
                let p = model(eps);
                let e = ParallelExecution::rayon(threads).unwrap();
                let a = coupled_step(&p, 0.0, &[0.1; 96], 0.001, &c, &e);
                let b = coupled_step_residual_first(&p, 0.0, &[0.1; 96], 0.001, &c, &e);
                let a = a.candidate.unwrap();
                let b = b.candidate.unwrap();
                assert_eq!(bits(&a.y_new), bits(&b.y_new));
                assert_eq!(bits(&a.error_vector), bits(&b.error_vector));
                assert_eq!(
                    a.full_residual_wrms.to_bits(),
                    b.full_residual_wrms.to_bits()
                );
            }
        }
    }
}
