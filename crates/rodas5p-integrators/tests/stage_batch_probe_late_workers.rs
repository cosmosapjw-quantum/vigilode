//! Loop node V32 (owner decision OD-L1-28): the concurrency probe of the stage-batch feasibility
//! report still observes two simultaneously active tasks when the probe pool's workers start late.
//!
//! The hosted failure of CI triage section 12.1 (research/production_loop_20261116/ci/TRIAGE_KO.md):
//! under `strace -f` on a loaded runner one worker of the probe's fresh pool ran all eight probe
//! tasks before any other worker ran, the probe reported 1 and `parallel-observed=false` changed
//! the scientific checksum. This binary makes that start order deterministic. Its global allocator
//! puts every rayon worker with index >= 1 to sleep for `LATE_START` at the worker's first
//! allocation after it has registered as a worker; that allocation happens when the worker first
//! looks for work (crossbeam-epoch registers the thread at its first steal), so before it can take
//! a job. Worker 0 starts normally. In every pool, the probe's included, worker 0 therefore runs
//! alone for `LATE_START`, far longer than the ~8 ms in which one worker runs the eight 1 ms probe
//! tasks. The probe before node V32 reported 1 here; with the bounded rendezvous (2 s) it must
//! report >= 2 (logs: research/production_loop_20261116/stage_batch_probe/).
//!
//! One test in this binary: the allocator is process-global.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rodas5p_core::CoreResult;
use rodas5p_integrators::{StageBatchFeasibilityProfile, run_stage_batch_feasibility};

/// Start delay of every rayon worker with index >= 1: well above the ~8 ms of the old probe's
/// window and well below the probe's 2 s rendezvous bound.
const LATE_START: Duration = Duration::from_millis(300);

/// Smoke reports in this test; each builds three fresh local pools (2, 4 and the probe's 4 threads).
const REPEATS: usize = 4;

/// Smoke checksum with `parallel-observed=true` (tests/stage_batch_probe_contracts.rs).
const SMOKE_CHECKSUM: &str = "4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950";

/// Rayon workers put to sleep so far (one sleep per worker thread).
static LATE_WORKERS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Set once this thread has been put to sleep. Const-initialised without destructor, so it is
    /// usable inside the allocator at any point of a thread's life.
    static STARTED_LATE: Cell<bool> = const { Cell::new(false) };
}

struct LateRayonWorkers;

impl LateRayonWorkers {
    /// Neither the index lookup (a const thread-local of rayon) nor the sleep allocates.
    fn delay_first_allocation_of_late_worker() {
        if matches!(rayon::current_thread_index(), Some(index) if index >= 1)
            && !STARTED_LATE.with(|started| started.replace(true))
        {
            LATE_WORKERS.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(LATE_START);
        }
    }
}

unsafe impl GlobalAlloc for LateRayonWorkers {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::delay_first_allocation_of_late_worker();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        Self::delay_first_allocation_of_late_worker();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        Self::delay_first_allocation_of_late_worker();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: LateRayonWorkers = LateRayonWorkers;

#[test]
fn probe_observes_concurrency_when_pool_workers_start_late() -> CoreResult<()> {
    let mut checksums = Vec::with_capacity(REPEATS);
    for repeat in 0..REPEATS {
        let late_before = LATE_WORKERS.load(Ordering::SeqCst);
        let report = run_stage_batch_feasibility(StageBatchFeasibilityProfile::Smoke)?;
        let late = LATE_WORKERS.load(Ordering::SeqCst) - late_before;
        // The probe pool alone has three workers with index >= 1; the probe waits for one of them.
        assert!(late >= 3, "repeat {repeat}: only {late} late workers");
        assert!(
            report.stage_parallelism_observed,
            "repeat {repeat}: probe observed {} simultaneously active task(s) with late workers",
            report.observed_max_parallel_tasks
        );
        assert!(report.observed_max_parallel_tasks >= 2);
        if report.rows.iter().all(|row| row.max_abs_difference == 0.0) {
            assert_eq!(
                report.scientific_checksum, SMOKE_CHECKSUM,
                "repeat {repeat}"
            );
        }
        println!(
            "repeat={repeat} late_workers={late} observed_max_parallel_tasks={} checksum={}",
            report.observed_max_parallel_tasks, report.scientific_checksum
        );
        checksums.push(report.scientific_checksum);
    }
    checksums.dedup();
    assert_eq!(
        checksums.len(),
        1,
        "checksums differ between repeats: {checksums:?}"
    );
    Ok(())
}
