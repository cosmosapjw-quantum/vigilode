# Independent diff review — global-error Pareto authority v0.10

Base: `68fe67260befd3901cf0b211a46c1d0d85c03932`
Reviewed branch: `unified-global-error-pareto-v0.10`

## Scope reviewed

- `rodas5p-fair-ab::global_error`;
- public exports;
- CLI command and contract test;
- exact reference and error definitions;
- timing semantics;
- Pareto/failure handling;
- T1/T4 reproducibility.

## Findings

### Critical

None after the execution-clock fix.

### Important — fixed

**I-001: incomparable wall clocks were stored in one field.**

The first draft put the full T1 randomized timing campaign and the T4 one-pass scientific suite in `aggregate_wall_seconds`. This produced a meaningless apparent throughput ratio. The field was split into comparable `scientific_suite_wall_seconds` and T1-only `timing_campaign_wall_seconds`. A RED/GREEN regression test verifies the semantics.

**I-002: one-shot microsecond timing was too noisy.**

Per-record calibrated batching was added, with minimum sample duration 2 ms and recorded batch iteration count.

### Important — remains a blocker, not a defect in P0

**I-003: every-step output storage contaminates future work-precision ranking.**

The current integrator APIs retain every internal step. P0 compares fixed-step anchors and reports retained bytes explicitly, but adaptive production ranking must introduce a common observer and disable internal trajectory storage.

**I-004: no-interpolation grid cannot yet consume arbitrary adaptive trajectories.**

Step clipping or verified dense output is required before variable-order BDF/adaptive Radau are admitted.

**I-005: corpus is too small for production wall ranking.**

Current results validate authority mechanics only.

### Minor

- `global_error.rs` is large but remains a single focused authority module; split only when adaptive observers/references are added.
- Reference WRMS scale is external and reference-based, intentionally not identical to each solver's internal error weights. The report must keep this distinction explicit.
- T4 throughput is case-level parallelism only.

## Test evidence

- targeted fair-ab contracts passed;
- CLI global-error command contract passed;
- canonical T1/T1-rerun/T4 scientific checksums matched;
- all 70 canonical runs succeeded;
- calibrated T1 wall medians showed central rerun ratio Q25–Q75 of 0.977–1.022.

## Verdict

**PASS WITH SCOPE HOLD.**

Promote the authority implementation. Do not promote a solver ranking until observer/storage, adaptive-grid, larger-corpus, and reference-hierarchy blockers are closed.
