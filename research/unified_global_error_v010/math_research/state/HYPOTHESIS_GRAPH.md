# Phase 4 — Hypothesis space

## H-001 — External-error authority

At matched maximum common-grid L2 error, total-cost Pareto fronts give a more defensible cross-family comparison than same tolerance or same step count.

## H-002 — Multi-error diagnostics

Endpoint, maximum-grid, and RMS-grid error can disagree; all three must be stored even if maximum-grid L2 is the initial authority.

## H-003 — Exact-grid, no-interpolation design

Nested fixed-step ladders with a common grid equal to the coarsest step eliminate interpolation-order bias in the first authority implementation.

## H-004 — Vector-valued cost

Wall time, RHS evaluations, Jacobian builds, factorizations, nonlinear iterations, accepted steps, and stored-state bytes must remain separate Pareto dimensions.

## H-005 — Reference provenance

Analytic references with a content hash and zero declared reference uncertainty are sufficient for P0; later numerical references require dual-solver consensus and a nonzero uncertainty field.

## H-006 — Timing separation

One-thread randomized repeated timing is authoritative. Four-thread execution is a throughput/determinism experiment, not a per-candidate latency ranking.

## H-007 — Startup costs matter

BDF2 startup and all nonlinear/factorization work must be included; dropping startup can reverse low-cost rankings on short trajectories.

## H-008 — Failure-preserving Pareto

Failed points remain in the dataset and cannot disappear from target-attainment summaries.
