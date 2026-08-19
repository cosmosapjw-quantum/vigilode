# Global-Error Pareto Authority Design

## Goal

Add a method-independent exact-reference work-precision authority for complete integrators without modifying any solver algorithm.

## Architecture

The authority lives in `rodas5p-fair-ab`, which already depends on `rodas5p-integrators`. It builds exact reference trajectories on nested common output grids, executes fixed-step anchors through their existing APIs, computes external error metrics, records complete work/timing/storage data, and constructs per-problem Pareto fronts and target-attainment summaries. The CLI exposes a deterministic `global-error-pareto` command. Scientific fields are sorted and hashed independently of timing.

## Components

1. `global_error.rs`: contracts, exact reference provenance, common-grid validation, error metrics, fixed-anchor dispatch, timing protocol, Pareto algorithms, canonical screen.
2. `global_error_contracts.rs`: unit, scientific, determinism, failure-preservation, and thread-identity tests.
3. CLI command: smoke/canonical profiles, explicit thread count, output JSON.
4. Research artifacts: report, tables, validation receipts, independent review, next-branch handoff.

## Candidate scope

- sequential RODAS5P/direct;
- BDF1 fixed;
- BDF2 fixed;
- Radau IIA1 fixed;
- Radau IIA3 fixed.

No candidate internals are changed.

## Error contract

For a common output grid that every nested step ladder hits exactly, store endpoint, maximum-grid, and RMS-grid L2 and WRMS errors. `max_grid_l2` is the P0 primary error. Missing grid points are an error; interpolation is forbidden.

## Cost contract

Store all `WorkCounters`, repeated one-thread wall samples, median and IQR, accepted/rejected steps, attempts, and deterministic stored-state bytes. No weighted scalar work score is used. Pareto fronts are computed separately for wall time, RHS evaluations, Jacobian builds, direct factorizations, nonlinear iterations, accepted steps, and stored-state bytes.

## Timing and parallelism

One-thread runs use warm-up plus randomized repeated task order and provide authoritative latency. Multi-thread runs parallelize independent case/control tasks with a local Rayon pool, preserve canonical ordering, and provide aggregate throughput plus a scientific checksum; their per-task latency is non-authoritative.

## Failure semantics

Every failure is serialized with candidate/problem/control identity. Failed/nonfinite points never enter successful Pareto fronts or target-attainment winners, but are never dropped from the full result.

## Promotion boundary

This cycle promotes the authority implementation only. It does not promote a solver or establish a production ranking.
