# PHYS–MATH–CODE AUDIT — Matrix-Free Common-W Gate

## Equation-to-code map

| Mathematical object | Code path |
|---|---|
| `Wv = Mv - h gamma Jv` | `MatrixFreeCommonWOperator::apply_formula` |
| batched `W V` | `LinearOperator::apply_rows` override |
| local stage concurrency | `ParallelExecution::try_for_each_ordered_mut` |
| common diagonal preconditioner | `AnalyticDiagonalPreconditioner` |
| independent GMRES | `solve_gmres` per RHS |
| block Arnoldi / projection | `solve_block_gmres` |
| seed projection + refinement | `solve_seeded_gmres` |
| original-system residual | `true_residuals` / common external certificate |
| attempted-work ledger | `WorkCounters` deltas and failed-row serialization |

## Code-path reality

### Implemented and used

- Candidate operator never materializes `J` or `W`.
- `apply_rows` actually enters a local Rayon pool for multiple input vectors.
- All solvers consume the same operator/preconditioner/tolerances.
- Canonical cases cover dimensions, exact RHS ranks, stiffness, nonnormality, and thread counts.
- Block failure records preserve JVP/Mv/preconditioner/orthogonalization work.

### Implemented but research-grade

- Block GMRES uses dynamic direction deflation, but has no restart or deflated-restart cycle.
- Least-squares projection is rebuilt with dense pivoted QR.
- `apply_left_rows` currently clones each output before applying the preconditioner, adding allocation overhead.
- `estimated_krylov_bytes` is algorithmic storage, not process peak RSS.
- Seeded shared refinement is robust but does not share work beyond the first seed when numerical rank exceeds one.

### Missing connection

- Actual `StructuredBlockSystem`, SABR, and q=0/1/2 homotopy stages are not yet wired to these backends.
- No live numerical-rank/angle telemetry from physical stage RHS exists.
- No transactional backend dispatcher exists.
- No deflated restart or independent-GMRES retry is integrated into the block solve API.

## Regression and fairness checks

- Small nonzero RHS cannot be falsely discarded by rank detection.
- Linearly dependent RHS are deflated without losing solution correctness.
- Ragged/nonfinite input is rejected.
- Exhausted budgets return explicit failure.
- 8-RHS solutions agree with a direct materialized-W test oracle in bounded tests.
- One-thread and multi-thread paths use the same scientific ordering.

## Ranked code risks

### P1

1. Nonrestarted block basis hits a hard 128-direction cap in severe cases.
2. No actual homotopy RHS telemetry, so backend applicability is unresolved.

### P2

3. Dense QR at every projection can dominate at larger block/basis sizes.
4. Per-row clone in left-preconditioned block action is avoidable allocation.
5. Memory report is not peak-memory measurement.
6. Synthetic operator/preconditioner may be easier than target physics.

### P3

7. Extremely high maximum speedups at very small N are timing-sensitive; medians and quantiles are the authoritative figures.

## Minimal fixes before integration

1. Add actual stage-RHS singular-value/angle telemetry without changing solver behavior.
2. Add transactional block→independent fallback; retain failed block work.
3. Use rank/risk evidence to select seeded, block, or independent backend.
4. Defer BGMRES-DR until production traces demonstrate that block failures are frequent enough to justify the extra algorithm.
