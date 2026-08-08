# Phase 9 — Survivor formalization

The surviving comparison authority consists of:

1. `ReferenceSolutionProvenance` with immutable reference and grid IDs.
2. `GlobalErrorTarget` with fixed reference-based absolute/relative scales.
3. `GlobalErrorReport` containing endpoint, maximum-grid, RMS-grid, and conservative error.
4. `IntegratorWorkReport` containing full `WorkCounters`, timing breakdown, and optional peak memory.
5. Failure-preserving run rows.
6. Separate Pareto fronts for each cost metric.
7. Target-matched selection without equal-internal-tolerance assumptions.
