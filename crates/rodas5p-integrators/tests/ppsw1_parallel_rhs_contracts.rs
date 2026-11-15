//! Execution-only contracts: independent Peer RHS rows, not RODAS stage parallelism.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use rodas5p_core::{CoreError, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    OdeProblem, ParallelExecution,
    ppsw1::{self, Config, Session, State},
};

#[derive(Default)]
struct Probe {
    calls: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
}

fn rhs(t: f64, y: &[f64]) -> [f64; 2] {
    [
        -80.0 * y[0] + 0.5 * y[1] * y[1] + t.sin(),
        -3.0 * y[1] + t.cos() * y[0] * y[0],
    ]
}

fn state() -> State {
    let h = 0.001;
    let y = std::array::from_fn(|j| vec![1.0 + 0.01 * j as f64, 0.7 - 0.02 * j as f64]);
    let f = std::array::from_fn(|j| rhs(0.5 + (ppsw1::nodes()[j] - 1.0) * h, &y[j]).to_vec());
    State {
        t: 0.5,
        y,
        f,
        h_prev: h,
    }
}

// Failure modes: 0 successful, 1 callback errors at two distinct stage times,
// 2 nonfinite output. Work is independently measured by callback-entry atomics.
fn problem(probe: Arc<Probe>, delay: bool, failure: u8) -> OdeProblem {
    OdeProblem::new(
        "peer-parallel-rhs-contract",
        2,
        Arc::new(move |t, y, out| {
            probe.calls.fetch_add(1, Ordering::SeqCst);
            let active = probe.active.fetch_add(1, Ordering::SeqCst) + 1;
            probe.peak.fetch_max(active, Ordering::SeqCst);
            if delay {
                std::thread::sleep(Duration::from_millis(30));
            }
            probe.active.fetch_sub(1, Ordering::SeqCst);
            if failure == 1 && t < 0.5 {
                return Err(CoreError::InvalidInput(if t < 0.49925 {
                    "first-stage-error".into()
                } else {
                    "second-stage-error".into()
                }));
            }
            out.copy_from_slice(&rhs(t, y));
            if failure == 2 && t < 0.5 {
                out[0] = f64::NAN;
            }
            Ok(())
        }),
        None,
        Some(Arc::new(|t, y| {
            DenseMatrix::from_vec_rows(vec![vec![-80.0, y[1]], vec![2.0 * t.cos() * y[0], -3.0]])
        })),
        None,
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn parallel_rhs_callbacks_overlap_within_explicit_thread_bound() {
    let probe = Arc::new(Probe::default());
    let p = problem(probe.clone(), true, 0);
    let mut work = WorkCounters::default();
    ppsw1::advance(
        &p,
        &state(),
        0.001,
        &ParallelExecution::rayon(2).unwrap(),
        &mut work,
    )
    .unwrap();
    assert_eq!(probe.calls.load(Ordering::SeqCst), 6);
    let peak = probe.peak.load(Ordering::SeqCst);
    assert!(peak > 1, "RHS callbacks never overlapped: peak={peak}");
    assert!(peak <= 2, "explicit local pool bound exceeded: peak={peak}");
    assert_eq!(probe.active.load(Ordering::SeqCst), 0);
    assert_eq!(work.rhs_calls, 6);
    assert_eq!(work.rhs_evaluations, 6);
    assert_eq!(
        work.rhs_batch_calls, 0,
        "scalar callback identity must be preserved"
    );
}

#[test]
fn nonlinear_nonautonomous_steps_preserve_bits_and_every_work_counter() {
    let p = problem(Arc::new(Probe::default()), false, 0);
    let (mut serial, mut parallel) = (state(), state());
    let (mut ws, mut wp) = (WorkCounters::default(), WorkCounters::default());
    let exec = ParallelExecution::rayon(3).unwrap();
    for h in [0.001, 0.0008, 0.00088, 0.0007] {
        serial = ppsw1::advance(&p, &serial, h, &ParallelExecution::sequential(), &mut ws)
            .unwrap()
            .next;
        parallel = ppsw1::advance(&p, &parallel, h, &exec, &mut wp)
            .unwrap()
            .next;
        assert_eq!(serial.t.to_bits(), parallel.t.to_bits());
        for (a, b) in serial
            .y
            .iter()
            .chain(&serial.f)
            .zip(parallel.y.iter().chain(&parallel.f))
        {
            assert_eq!(
                a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
        }
        assert_eq!(ws, wp);
    }
}

#[test]
fn failed_parallel_attempt_joins_charges_all_jobs_and_leaves_session_unchanged() {
    let probe = Arc::new(Probe::default());
    let p = problem(probe.clone(), true, 1);
    let initial = state();
    let mut session = Session::new(
        &p,
        initial.clone(),
        Config::default(),
        ParallelExecution::rayon(2).unwrap(),
    )
    .unwrap();
    let controller = session.controller().clone();
    let err = session.attempt(0.001).err().expect("callback must fail");
    assert!(
        matches!(err, CoreError::InvalidInput(ref s) if s == "first-stage-error"),
        "stage-index first error required: {err:?}"
    );
    assert_eq!(session.state(), &initial);
    assert_eq!(session.controller(), &controller);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 6);
    assert_eq!(probe.active.load(Ordering::SeqCst), 0);
    assert_eq!(session.counters().rhs_calls, 6);
    assert_eq!(session.counters().rhs_evaluations, 6);
    assert_eq!(session.counters().accepted_steps, 0);
    assert_eq!(session.counters().linear_solves, 6);
    assert_eq!(session.counters().direct_factorizations, 1);
}

#[test]
fn serial_failure_keeps_original_prefix_only_execution_semantics() {
    let probe = Arc::new(Probe::default());
    let p = problem(probe.clone(), false, 1);
    let mut work = WorkCounters::default();
    assert!(
        ppsw1::advance(
            &p,
            &state(),
            0.001,
            &ParallelExecution::sequential(),
            &mut work
        )
        .is_err()
    );
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    assert_eq!(work.rhs_calls, 1);
    assert_eq!(work.rhs_evaluations, 1);
}

#[test]
fn nonfinite_parallel_output_charges_all_executed_callbacks() {
    let probe = Arc::new(Probe::default());
    let p = problem(probe.clone(), false, 2);
    let mut work = WorkCounters::default();
    let result = ppsw1::advance(
        &p,
        &state(),
        0.001,
        &ParallelExecution::rayon(2).unwrap(),
        &mut work,
    );
    assert!(matches!(result, Err(CoreError::NonFinite(_))));
    assert_eq!(probe.calls.load(Ordering::SeqCst), 6);
    assert_eq!(work.rhs_calls, 6);
    assert_eq!(work.rhs_evaluations, 6);
}
