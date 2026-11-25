//! Loop node V32 (owner decision OD-L1-28), diagnostic only (ignored; run by hand): how long the
//! bounded rendezvous of the stage-batch concurrency probe waits on this host.
//!
//! The probe itself is private, so this builds what it builds - a fresh `ParallelExecution::rayon(4)`
//! and eight stage rows through `eval_rhs_stage_rows`, each task 1 ms, with the same rendezvous - and
//! records, per fresh pool, the time from the first task's arrival until a second task is active at
//! the same time: the time the probe's first task waits. Run under a provocation (strace -f on a busy
//! CPU) it shows how far below the 2 s bound the waits stay
//! (research/production_loop_20261116/stage_batch_probe/latency.sh). It prints the distribution and
//! asserts nothing about time.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rodas5p_core::{CoreResult, WorkCounters};
use rodas5p_integrators::{OdeProblem, ParallelExecution};

const RENDEZVOUS_BOUND: Duration = Duration::from_secs(2);
const RENDEZVOUS_POLL: Duration = Duration::from_micros(100);
const STAGES: usize = 8;

/// Seconds from the first task's arrival until two tasks were active at once; None if never.
fn one_fresh_pool() -> CoreResult<Option<f64>> {
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    // first: arrival of the first task; second: the moment a second task was active at once.
    let first = Arc::new(OnceLock::<Instant>::new());
    let second = Arc::new(OnceLock::<Instant>::new());
    let (active_rhs, maximum_rhs) = (Arc::clone(&active), Arc::clone(&maximum));
    let (first_rhs, second_rhs) = (Arc::clone(&first), Arc::clone(&second));
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let until = *first_rhs.get_or_init(Instant::now) + RENDEZVOUS_BOUND;
        let now = active_rhs.fetch_add(1, Ordering::SeqCst) + 1;
        if maximum_rhs.fetch_max(now, Ordering::SeqCst) < 2 && now >= 2 {
            let _ = second_rhs.set(Instant::now());
        }
        while maximum_rhs.load(Ordering::SeqCst) < 2 && Instant::now() < until {
            std::thread::sleep(RENDEZVOUS_POLL);
        }
        std::thread::sleep(Duration::from_millis(1));
        out.copy_from_slice(y);
        active_rhs.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out.copy_from_slice(v);
        Ok(())
    });
    let problem = OdeProblem::new(
        "stage-concurrency-latency",
        4,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;
    let mut counters = WorkCounters::default();
    problem.eval_rhs_stage_rows(
        &[0.0; STAGES],
        &vec![vec![1.0; 4]; STAGES],
        &ParallelExecution::rayon(4)?,
        &mut counters,
    )?;
    let start = *first.get().expect("eight tasks ran");
    Ok(second
        .get()
        .map(|instant| instant.duration_since(start).as_secs_f64()))
}

#[test]
#[ignore = "diagnostic: rendezvous wait on fresh pools; run by hand (stage_batch_probe/latency.sh)"]
fn rendezvous_wait_on_fresh_pools_diagnostic() -> CoreResult<()> {
    let pools: usize = std::env::var("V32_LATENCY_POOLS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(200);
    let mut waits = Vec::with_capacity(pools);
    let mut never = 0;
    for _ in 0..pools {
        match one_fresh_pool()? {
            Some(seconds) => waits.push(seconds),
            None => never += 1,
        }
    }
    waits.sort_by(f64::total_cmp);
    if waits.is_empty() {
        println!("diagnostic pools={pools} second_task_never={never}");
        return Ok(());
    }
    let at = |q: f64| waits[((waits.len() - 1) as f64 * q).round() as usize];
    println!(
        "diagnostic pools={pools} second_task_never={never} wait_seconds min={:.6} median={:.6} \
         p90={:.6} p99={:.6} max={:.6}",
        at(0.0),
        at(0.5),
        at(0.9),
        at(0.99),
        at(1.0)
    );
    for (rank, seconds) in waits.iter().rev().take(5).enumerate() {
        println!("diagnostic largest[{rank}]={seconds:.6}");
    }
    Ok(())
}
