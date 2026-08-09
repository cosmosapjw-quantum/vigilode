# PROMOTE / HOLD — Stage-Batch Feasibility v0.13

## PROMOTE

- Within-step 8-stage ordered batch API.
- Local Rayon stage execution with measured concurrent tasks.
- Component-major supplied batch path.
- common-W multi-RHS reference path.
- Exact numerical parity across all paths.
- Three-campaign deterministic scientific checksum.
- Machine-readable honesty fields.

## HOLD

- Strict Jacobian-free common-W solve.
- Explicit SIMD implementation/claim.
- End-to-end adaptive solver speedup.
- q=0/1/2 homotopy promotion.
- Production/default backend selection.
- Sparse, large-PDE, GPU or DAE claims.

## Decision

`CONDITIONAL PROMOTE AS FEASIBILITY INFRASTRUCTURE`.

The next scientific blocker is not whether batching exists; it is whether the dominant common-W gain can be reproduced with a matrix-free block/shared-Krylov solve and then retained inside low-depth Newton-free homotopy.
