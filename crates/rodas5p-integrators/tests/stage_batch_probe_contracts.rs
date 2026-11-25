//! Loop node V32 (owner decision OD-L1-28): the concurrency probe of the stage-batch feasibility
//! report waits, with a bound, until a second task is active, so that `parallel-observed` and the
//! scientific checksum no longer depend on when the probe pool's workers start.
//!
//! Repeated smoke reports in one process must all observe within-step concurrency and give one
//! scientific checksum. The checksum composition is unchanged: it is rebuilt here from the report
//! fields with the lines of `scientific_checksum` in src/stage_batch.rs (as
//! research/production_loop_20261116/ci/v3_first_hosted/checksum_flip.py does).
//! tests/stage_batch_probe_late_workers.rs covers the probe with late workers.

use rodas5p_core::{CoreResult, sha256_hex};
use rodas5p_integrators::{
    StageBatchFeasibilityProfile, StageBatchFeasibilityReport, run_stage_batch_feasibility,
};

/// Smoke reports per test; each builds three fresh local pools (2, 4 and the probe's 4 threads).
const REPEATS: usize = 16;

/// Smoke checksum with `parallel-observed=true` before this node (owner host, and the first CLI
/// run of hosted CI job 138517747499): research/production_loop_20261116/ci/v3_first_hosted/.
const SMOKE_CHECKSUM: &str = "4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950";

fn recomposed_checksum(report: &StageBatchFeasibilityReport) -> String {
    let mut signature = format!("parallel-observed={}\n", report.stage_parallelism_observed);
    for case in &report.cases {
        signature.push_str(&format!(
            "case|{}|{}|{}|{}|{}\n",
            case.case_id, case.family, case.dimension, case.work_repeats, case.stages
        ));
    }
    for row in &report.rows {
        signature.push_str(&format!(
            "row|{}|{}|{}|{}|{}|{}|{}|{:016x}|{}\n",
            row.case_id,
            row.kernel,
            row.backend,
            row.threads,
            row.stages,
            row.dimension,
            row.work_repeats,
            row.max_abs_difference.to_bits(),
            row.actual_stage_parallel
        ));
    }
    sha256_hex(signature.as_bytes())
}

#[test]
fn repeated_smoke_reports_observe_concurrency_with_one_unchanged_checksum() -> CoreResult<()> {
    let mut checksums = Vec::with_capacity(REPEATS);
    let mut pinned_comparisons = 0;
    for repeat in 0..REPEATS {
        let report = run_stage_batch_feasibility(StageBatchFeasibilityProfile::Smoke)?;
        assert!(
            report.stage_parallelism_observed,
            "repeat {repeat}: probe observed {} simultaneously active task(s)",
            report.observed_max_parallel_tasks
        );
        assert!(report.observed_max_parallel_tasks >= 2);
        assert_ne!(report.verdict, "stage-concurrency-not-observed");
        assert_eq!(
            recomposed_checksum(&report),
            report.scientific_checksum,
            "repeat {repeat}: checksum composition changed"
        );
        // The pinned value holds where every stage-batch path reproduces the sequential reference
        // bit for bit (all rows on the owner host and on the hosted runner). A host where a row
        // differs in the last bits has another, still deterministic, checksum; that host
        // dependence is not this node's and is not turned into a new bit anchor here.
        if report.rows.iter().all(|row| row.max_abs_difference == 0.0) {
            assert_eq!(
                report.scientific_checksum, SMOKE_CHECKSUM,
                "repeat {repeat}"
            );
            pinned_comparisons += 1;
        }
        checksums.push(report.scientific_checksum);
    }
    checksums.dedup();
    assert_eq!(
        checksums.len(),
        1,
        "checksums differ between repeats: {checksums:?}"
    );
    println!(
        "repeats={REPEATS} distinct_checksums={} checksum={} pinned_comparisons={pinned_comparisons}",
        checksums.len(),
        checksums[0]
    );
    Ok(())
}
