# Global-Error Pareto Authority Design

## Goal

Add a method-independent, failure-preserving external global-error versus total-cost authority and validate it by replaying unchanged fixed-step RODAS5P, BDF1/2, and Radau IIA1/3 anchors.

## Architecture

The authority lives in `rodas5p-fair-ab`, above all integrator families. It receives complete trajectories and immutable analytic-reference cases, evaluates reference-scaled WRMS errors only at exact common output times, records full work/timing data, and builds one Pareto front per cost metric. Numerical methods remain unchanged.

## Components

### Reference and grid

- `ReferenceSolutionProvenance`: problem, kind, description, reference ID, uncertainty radius.
- `OutputGrid`: strictly increasing finite times and immutable grid ID.
- Initial version: analytic reference only; numerical-reference schema is supported but not generated.

### Error authority

- `GlobalErrorTarget`: scalar absolute/relative tolerances and threshold.
- Fixed reference weights `atol + rtol * abs(reference)`.
- Reports endpoint WRMS, max-grid WRMS, RMS-grid WRMS, and conservative max error including reference uncertainty.
- Pass iff conservative max error is at most threshold.

### Work authority

- `IntegratorWorkReport`: unchanged `WorkCounters`, setup/solve/sample/total wall time, optional peak resident bytes.
- No weighted aggregate score.

### Pareto authority

- Separate front for wall time, RHS evaluations, JVP vectors, nonlinear iterations, and direct factorizations.
- Failed runs remain in raw rows and are excluded from fronts.
- Target-matched summary chooses least-cost successful row per candidate and target.

### Fixed-step replay

Candidates:

- `sequential-direct-off`
- `bdf1-fixed`
- `bdf2-fixed`
- `radau-iia1-fixed`
- `radau-iia3-fixed`

Corpus uses analytic scalar linear, Prothero–Robinson, manufactured vector, and manufactured nonlinear mass-matrix problems. Step ladders divide every common output-grid interval exactly.

### Parallelism and determinism

Independent run specifications are evaluated through the existing local Rayon `ParallelExecution`. Results are sorted canonically before serialization. Thread count is execution metadata, not scientific state.

## Failure handling

Invalid targets, invalid grids, missing exact output times, nonfinite states, solver failures, and reference dimension mismatch are explicit row failures. No interpolation or fallback is silent.

## Testing

TDD covers error formulas, reference uncertainty, exact landing, hand-worked Pareto dominance, failure retention, order recovery, common IDs, and 1/4-thread scientific identity.

## Non-goals

No solver changes, adaptive control, dense output, singular DAE, external SUNDIALS/PETSc execution, or production ranking.
