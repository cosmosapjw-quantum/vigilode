# Plot-based CRAG adversarial audit

## Plot 1 — combined speedup versus dimension

`COMBINED_SPEEDUP_VS_DIMENSION.png`

Reading:

- Component-major batch + multi-RHS is above 1.15x for both physics families at all N.
- Rayon-2 and Rayon-4 cross above 1 only near N=512.
- Rayon-2 is generally better than Rayon-4, consistent with eight coarse tasks and nontrivial scheduling/allocation overhead.

## Plot 2 — kernel/backend crossover summary

`KERNEL_BACKEND_SPEEDUP_SUMMARY.png`

Reading:

- The stable load-bearing gain is the common-W multi-RHS solve.
- Standalone Rayon JVP is consistently below baseline.
- Combined medians can look good while some backends remain very poor; backend-specific reporting is mandatory.

## Plot 3 — best backend map

`BEST_COMBINED_CROSSOVER_MAP.png`

Reading:

- Component-major batch is the best backend in the overwhelming majority of workloads.
- Rayon-2 becomes competitive only for the heaviest N=512 oscillatory workloads.

## CRAG

### Correctness

All plotted rows have zero numerical difference from the sequential reference. The plots support a crossover claim, not an algorithmic-order claim.

### Retrieval

The observed pattern agrees with stage-parallel literature: batching can expose stage concurrency, but achievable speedup depends on stage count and the work/communication ratio.

### Augmented

The dimension and work-repeat mutations show that thread-level acceleration is workload dependent. The same claim must next survive nonnormal, mass-matrix, and expensive PDE kernels.

### Generation

Prediction: a generic component-major batch plus matrix-free multi-RHS/shared-basis Krylov should outperform per-stage Rayon on cheap-to-moderate kernels, while Rayon becomes useful for expensive stage physics.

## Claim status

- `actual within-step concurrency exists`: surviving.
- `batch layout can yield >1.15x microkernel speedup`: surviving, dense-reference scope.
- `Rayon stage parallelism is generally faster`: rejected.
- `strict Jacobian-free homotopy solver is faster`: not yet tested.
