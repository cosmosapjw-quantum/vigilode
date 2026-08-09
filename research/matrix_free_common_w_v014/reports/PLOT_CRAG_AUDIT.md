# Plot-Driven CRAG Audit — Matrix-Free Common-W v0.14

## Generated plots

1. `SOLVER_SPEEDUP_SUMMARY.png`
2. `BLOCK_SPEEDUP_VS_DIMENSION.png`
3. `BLOCK_SUCCESS_RATE.png`
4. `OPERATOR_RATIO_BY_RANK.png`
5. `RAYON_SPEEDUP_BY_THREADS.png`
6. `BLOCK_ROBUSTNESS_REGIME.png`

## Plot reading

### Solver summary

Block GMRES has the largest median speedup and the widest dispersion. The dispersion is not harmless: it includes both very large gains and successful rows slower than serial, while 36 failures are not present in the bar itself. Therefore the bar cannot support a universal-default claim.

### Dimension/rank crossover

Rank-1 and rank-2 block paths retain large median speedup as dimension grows. Rank-4 and rank-8 speedup falls with dimension, but remains above 1.15 in the median. This pattern predicts that actual homotopy usefulness will depend more on numerical RHS rank and nonnormality than on dimension alone.

### Success fraction

Success is perfect at N=32/128. At N=512/2048 it falls monotonically with RHS rank, reaching 0.67 at rank 8. This visually rejects the claim that a fixed 128-direction nonrestarted block basis is generally robust.

### Operator ratio

Block GMRES reduces operator vectors across all ranks. Seeded shared GMRES only produces a major reduction at rank 1. This narrows the seeded claim to a specialized low-rank path.

### Rayon scaling

Independent GMRES has no work reduction but gives stable median speedup at 2 and 4 threads. One thread is neutral. Thus it is a robust concurrency fallback, not an algorithmic Krylov-work improvement.

### Robustness regime

Block failure probability grows jointly with stiffness and nonnormality. Mild stiffness is fully safe in the finite grid; strong nonnormality at 1e4–1e6 is the main hostile region.

## CRAG

### Correctness

All successful plotted rows passed the original-system true-residual certificate. Failed rows are preserved separately. PASS.

### Retrieval

The pattern agrees with multiple-RHS Krylov literature: block methods profit from shared subspaces but require rank-deficiency handling, restart/deflation, and robust orthogonalization. PASS with scope.

### Augmented checks

The pattern was checked over N, RHS rank, stiffness, nonnormality, and threads. It has not yet been checked on actual nonlinear homotopy stage RHS. INCOMPLETE.

### Generation

Prediction: q=0/1/2 homotopy rounds with rapidly decaying singular values should favor block or seeded paths; high-rank/nonstationary rounds should favor independent Rayon. This is a testable next-cycle hypothesis, not a current result.

## Adversarial mutations

- Increased nonnormality: block success falls.
- Increased stiffness: block success and median gain fall.
- Increased RHS rank: seeded advantage disappears; block failures increase.
- Thread mutation: failure patterns do not change, excluding a Rayon race as the primary cause.

## Claim status

```text
SURVIVING:
  strict matrix-free multiple-RHS opportunity exists;
  block Krylov is high-reward in broad mild/moderate regimes;
  independent Rayon is the robust fallback;
  seeded sharing is useful for rank-one RHS.

NARROWED:
  block GMRES is not a universal default;
  synthetic exact rank is not enough to choose the target backend.

REJECTED:
  one common backend is optimal across all stage-RHS structures;
  microkernel speedup already proves integrated homotopy speedup.
```
