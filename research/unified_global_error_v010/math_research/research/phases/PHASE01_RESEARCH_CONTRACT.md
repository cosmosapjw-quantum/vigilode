# Phase 1 — Research contract

## Question

Can all complete-integrator candidates be compared without changing their algorithms by matching an **external global error** on a common output grid, then comparing total work, wall time, and stored-state memory as a Pareto set?

## Scope

- Fixed-step correctness anchors only: sequential RODAS5P/direct, BDF1, BDF2, Radau IIA1, Radau IIA3.
- Dimensionless exact-solution corpus, nonsingular mass matrices.
- No variable-order BDF, adaptive Radau, SDC, Peer/W, ROK/BOROK, Leja, or homotopy retuning.
- No scalar weighted score across unlike costs.

## Conventions

- State vectors are dimensionless in this cycle.
- External error is evaluated against an exact analytic reference.
- Primary metric: maximum common-grid L2 error; endpoint L2 and common-grid RMS L2 are retained.
- Normalized WRMS metrics use fixed external scales, never solver-internal tolerances.
- Wall-time ranking is authoritative only for one-thread randomized execution; multi-thread runs test throughput and scientific determinism.

## Completion bar

1. Exact-reference provenance and checksums.
2. Common output grids with no interpolation.
3. Method-independent run/work/error schema.
4. Deterministic Pareto front and target attainment algorithms.
5. Fixed-step anchor replay without solver modifications.
6. One-/four-thread scientific checksum identity.
7. Independent diff review and exact-HEAD validation.
