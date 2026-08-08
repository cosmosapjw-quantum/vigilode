# Phase 10 — Fair-measurement authority closeout

## Decision

### CLOSED — fixed-step comparison authority

The following are now durable and verified:

- common external analytic references;
- common requested output grids;
- requested-output-only retained trajectories;
- identical step-clipping landing policy;
- separate internal-step and clipping counters;
- failure-preserving Pareto construction;
- same optimized Rust binary for all candidates;
- pinned repeated T1 timing;
- exact T1/T4 scientific checksum identity.

### NOT CLOSED — production solver campaign

Adaptive BDF/Radau, dense output, broad reference hierarchy, sparse/DAE systems and deferred candidate families remain separate future research.

## Termination declaration

The infrastructure task whose purpose was to make solver measurements sufficiently fair is complete. No successor implementation branch is required for this task. Future solver additions must consume this authority rather than modify its scientific contract without a new design review.
