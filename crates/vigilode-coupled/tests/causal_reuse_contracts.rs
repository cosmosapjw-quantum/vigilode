use rodas5p_core::CoreError;
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use vigilode_coupled::{Attempt, CausalSchedule, Config, FailureKind, coupled_step_causal};

fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}

fn model(n: usize, eps: f64, calls: Arc<AtomicUsize>) -> OdeProblem {
    OdeProblem::new(
        "causal-cache-contract",
        n,
        Arc::new(move |_, y, f| {
            calls.fetch_add(1, Ordering::SeqCst);
            for i in 0..n {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i] + eps * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, f| {
            for i in 0..n {
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

fn run(
    p: &OdeProblem,
    y: &[f64],
    h: f64,
    c: &Config,
    e: &ParallelExecution,
    reuse: bool,
) -> Attempt {
    coupled_step_causal(
        p,
        0.0,
        y,
        h,
        c,
        e,
        if reuse {
            CausalSchedule::BatchedPicardReuse
        } else {
            CausalSchedule::BatchedPicard
        },
    )
}

fn identical(a: &Attempt, b: &Attempt) {
    let ca = a
        .candidate
        .as_ref()
        .unwrap_or_else(|| panic!("legacy {:?}", a.failure));
    let cb = b
        .candidate
        .as_ref()
        .unwrap_or_else(|| panic!("reuse {:?}", b.failure));
    assert_eq!(bits(&ca.y_new), bits(&cb.y_new));
    assert_eq!(bits(&ca.error_vector), bits(&cb.error_vector));
    for (a, b) in ca.stages_u.iter().zip(&cb.stages_u) {
        assert_eq!(bits(a), bits(b));
    }
    assert_eq!(
        ca.full_residual_wrms.to_bits(),
        cb.full_residual_wrms.to_bits()
    );
    assert_eq!(a.work.causal_picard_updates, b.work.causal_picard_updates);
    assert_eq!(a.work.jvp_attempted, b.work.jvp_attempted);
    assert_eq!(a.work.validation_jvp, b.work.validation_jvp);
    assert_eq!(
        a.work.rhs_attempted - b.work.rhs_attempted,
        b.work.causal_rhs_reused
    );
    assert_eq!(
        a.work.small_block_solves - b.work.small_block_solves,
        b.work.causal_block_solves_reused
    );
}

#[test]
fn cached_suffix_is_bitwise_identical_on_affine_and_nonlinear_targets() {
    let mut saved_rhs = 0;
    let mut saved_solves = 0;
    for n in [3, 6, 96] {
        for eps in [0.0, 1.0, 10.0] {
            for h in [0.001, 0.01, 0.1] {
                let p = model(n, eps, Arc::new(AtomicUsize::new(0)));
                let y = vec![0.1; n];
                let c = Config::default();
                let e = ParallelExecution::sequential();
                let a = run(&p, &y, h, &c, &e, false);
                let b = run(&p, &y, h, &c, &e, true);
                identical(&a, &b);
                saved_rhs += b.work.causal_rhs_reused;
                saved_solves += b.work.causal_block_solves_reused;
            }
        }
    }
    assert!(saved_rhs > 0 && saved_solves > 0);
}

#[test]
fn cached_parallel_workers_keep_bits_and_observed_callback_counts() {
    let calls = Arc::new(AtomicUsize::new(0));
    let p = model(96, 10.0, calls.clone());
    let a = run(
        &p,
        &[0.1; 96],
        0.1,
        &Config::default(),
        &ParallelExecution::sequential(),
        true,
    );
    let before = calls.load(Ordering::SeqCst);
    let b = run(
        &p,
        &[0.1; 96],
        0.1,
        &Config::default(),
        &ParallelExecution::rayon(4).unwrap(),
        true,
    );
    assert_eq!(
        bits(&a.candidate.as_ref().unwrap().y_new),
        bits(&b.candidate.as_ref().unwrap().y_new)
    );
    assert_eq!(a.work.rhs_attempted, b.work.rhs_attempted);
    assert_eq!(
        (calls.load(Ordering::SeqCst) - before) as u64,
        b.work.rhs_attempted
    );
    assert_eq!(b.work.stage_tasks_scheduled, b.work.stage_tasks_completed);
    assert!(b.work.causal_rhs_reused > 0);
}

#[test]
fn smaller_callback_budget_qualifies_identical_target_with_reuse() {
    let p = model(96, 10.0, Arc::new(AtomicUsize::new(0)));
    let e = ParallelExecution::sequential();
    let full = run(&p, &[0.1; 96], 0.1, &Config::default(), &e, true);
    assert!(full.candidate.is_some());
    assert!(full.work.causal_rhs_reused > 0);
    let c = Config {
        max_rhs: full.work.rhs_attempted,
        ..Config::default()
    };
    let cheap = run(&p, &[0.1; 96], 0.1, &c, &e, true);
    let old = run(&p, &[0.1; 96], 0.1, &c, &e, false);
    assert_eq!(
        bits(&full.candidate.unwrap().y_new),
        bits(&cheap.candidate.unwrap().y_new)
    );
    assert_eq!(old.failure.unwrap().kind, FailureKind::Budget);
}

#[test]
fn joined_failure_accounts_for_all_scheduled_callbacks_and_preserves_input() {
    let calls = Arc::new(AtomicUsize::new(0));
    let cc = calls.clone();
    let p = OdeProblem::new(
        "cache-failed-batch",
        3,
        Arc::new(move |_, y, f| {
            let k = cc.fetch_add(1, Ordering::SeqCst);
            if k == 2 {
                return Err(CoreError::NonFinite("manufactured callback failure".into()));
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
    let y = [0.1; 3];
    let r = run(
        &p,
        &y,
        0.01,
        &Config::default(),
        &ParallelExecution::rayon(4).unwrap(),
        true,
    );
    assert_eq!(r.failure.unwrap().kind, FailureKind::Callback);
    assert_eq!(r.work.rhs_attempted, 8);
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    assert_eq!(r.work.rhs_completed, 7);
    assert_eq!(r.work.stage_tasks_scheduled, 7);
    assert_eq!(r.work.stage_tasks_completed, 7);
    assert_eq!(y, [0.1; 3]);
}

#[test]
fn epoch_change_invalidates_whole_cached_attempt() {
    let epoch = Arc::new(AtomicU64::new(0));
    let changed = epoch.clone();
    let queried = epoch.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let p = OdeProblem::new(
        "cache-epoch",
        3,
        Arc::new(move |_, y, f| {
            if observed.fetch_add(1, Ordering::SeqCst) == 2 {
                changed.fetch_add(1, Ordering::SeqCst);
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
    .with_model_epoch(Arc::new(move || queried.load(Ordering::SeqCst)));
    let r = run(
        &p,
        &[0.1; 3],
        0.01,
        &Config::default(),
        &ParallelExecution::rayon(4).unwrap(),
        true,
    );
    assert_eq!(r.failure.unwrap().kind, FailureKind::ModelChanged);
    assert_eq!(r.work.rhs_attempted, calls.load(Ordering::SeqCst) as u64);
    assert!(r.candidate.is_none());
}

#[test]
fn inadequate_basis_still_fails_original_space_residual_screen() {
    let p = model(96, 1.0, Arc::new(AtomicUsize::new(0)));
    let cfg = Config {
        initial_basis: 1,
        max_basis: 1,
        max_expansions: 0,
        ..Config::default()
    };
    let r = run(
        &p,
        &[0.1; 96],
        0.01,
        &cfg,
        &ParallelExecution::sequential(),
        true,
    );
    assert_eq!(r.failure.unwrap().kind, FailureKind::ProjectionLeakage);
    assert_eq!(r.work.validation_jvp, 8);
    assert!(r.candidate.is_none());
}
