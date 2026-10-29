use std::sync::{
    Arc, Barrier,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use vigilode::{
    Atol, CallbackError, Dopri5, Dopri5Ensemble, Dopri5EnsembleJob, ExplicitSolution, Options,
    Problem,
};

fn decay(rate: f64) -> Problem {
    Problem::new(1, move |_t, y, out| {
        out[0] = -rate * y[0];
        Ok(())
    })
    .autonomous()
}

fn assert_same(left: &ExplicitSolution, right: &ExplicitSolution) {
    assert_eq!(left.success, right.success);
    assert_eq!(left.final_time.to_bits(), right.final_time.to_bits());
    assert_eq!(left.t, right.t);
    assert_eq!(left.y.len(), right.y.len());
    for (a, b) in left.y.iter().zip(&right.y) {
        assert_eq!(
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        left.final_state
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        right
            .final_state
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        serde_json::to_value(&left.stats).unwrap(),
        serde_json::to_value(&right.stats).unwrap()
    );
}

#[test]
fn ensemble_order_counters_and_tolerances_match_independent_solves() {
    let problems = [decay(0.3), decay(3.0), decay(1.2), decay(0.8)];
    let initial = [[1.0], [0.25], [4.0], [2.0]];
    let options: Vec<_> = (0..problems.len())
        .map(|i| Options {
            rtol: if i == 0 { 1e-10 } else { 1e-5 },
            atol: Atol::Scalar(if i == 0 { 1e-13 } else { 1e-8 }),
            h0: 0.05,
            ..Options::default()
        })
        .collect();
    let jobs: Vec<_> = (0..problems.len())
        .map(|i| Dopri5EnsembleJob {
            problem: &problems[i],
            t_span: (0.0, 1.0 + i as f64 * 0.25),
            y0: &initial[i],
            options: &options[i],
        })
        .collect();
    let serial = Dopri5Ensemble::new(1).unwrap().solve(&jobs).unwrap();
    let pool = Dopri5Ensemble::new(2).unwrap();
    assert_eq!(pool.workers(), 2);
    let parallel = pool.solve(&jobs).unwrap();
    let reused_pool = pool.solve(&jobs).unwrap();
    for (i, job) in jobs.iter().enumerate() {
        let independent = Dopri5::solve(job.problem, job.t_span, job.y0, job.options).unwrap();
        assert!(independent.success);
        assert_same(&independent, serial[i].as_ref().unwrap());
        assert_same(&independent, parallel[i].as_ref().unwrap());
        assert_same(&independent, reused_pool[i].as_ref().unwrap());
    }
}

#[test]
fn ensemble_preserves_refusal_partial_failure_and_success_without_cancellation() {
    let good = decay(1.0);
    let bad_callback = Problem::new(1, |_t, _y, _out| {
        Err(CallbackError::Fatal("intentional member failure".into()))
    });
    let opts = Options::default();
    let jobs = [
        Dopri5EnsembleJob {
            problem: &good,
            t_span: (0.0, 1.0),
            y0: &[],
            options: &opts,
        },
        Dopri5EnsembleJob {
            problem: &bad_callback,
            t_span: (0.0, 1.0),
            y0: &[1.0],
            options: &opts,
        },
        Dopri5EnsembleJob {
            problem: &good,
            t_span: (0.0, 1.0),
            y0: &[1.0],
            options: &opts,
        },
    ];
    let pool = Dopri5Ensemble::new(2).unwrap();
    let results = pool.solve(&jobs).unwrap();
    assert_eq!(results.len(), jobs.len());
    assert!(results[0].is_err());
    let failed = results[1].as_ref().unwrap();
    assert!(!failed.success);
    assert_eq!(failed.final_time, 0.0);
    assert_eq!(failed.final_state, vec![1.0]);
    let succeeded = results[2].as_ref().unwrap();
    assert!(succeeded.success);
    assert!((succeeded.final_state[0] - (-1.0_f64).exp()).abs() < 2e-7);
}

#[test]
fn ensemble_two_workers_really_overlap_rhs_calls() {
    let barrier = Arc::new(Barrier::new(2));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let problems: Vec<_> = (0..2)
        .map(|_| {
            let first = AtomicBool::new(true);
            let barrier = barrier.clone();
            let active = active.clone();
            let peak = peak.clone();
            Problem::new(1, move |_t, y, out| {
                if first.swap(false, Ordering::SeqCst) {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    // Both independent first calls must enter before either
                    // leaves. This checks concurrency, not a timing speedup.
                    barrier.wait();
                    active.fetch_sub(1, Ordering::SeqCst);
                }
                out[0] = -y[0];
                Ok(())
            })
        })
        .collect();
    let opts = Options::default();
    let jobs: Vec<_> = problems
        .iter()
        .map(|problem| Dopri5EnsembleJob {
            problem,
            t_span: (0.0, 0.1),
            y0: &[1.0],
            options: &opts,
        })
        .collect();
    let results = Dopri5Ensemble::new(2).unwrap().solve(&jobs).unwrap();
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert!(results.into_iter().all(|r| r.unwrap().success));
}

#[test]
fn ensemble_requires_positive_workers_and_handles_empty_batches() {
    assert!(Dopri5Ensemble::new(0).is_err());
    let pool = Dopri5Ensemble::new(1).unwrap();
    assert_eq!(pool.workers(), 1);
    assert!(pool.solve(&[]).unwrap().is_empty());
}
