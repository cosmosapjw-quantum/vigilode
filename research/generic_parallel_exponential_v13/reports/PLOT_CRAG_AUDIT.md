# G3 Plot-Based CRAG Audit

## Generated figures

- `G3_PHI_FUSION_WALL_SPEEDUP.png`
- `G3_PHI_FUSION_WORK_REDUCTION.png`
- `G3_RESIDUAL_VS_NESTED_ESTIMATOR.png`
- `G3_RESIDUAL_ESTIMATOR_BEFORE_AFTER.png`
- `G3_PRIMARY_SAME_ERROR_SPEEDUP.png`
- `G3_PRIMARY_STEP_RATIO.png`
- `G3_PRIMARY_SPEEDUP_BY_PROBLEM.png`
- `G3_OSCILLATORY_PR_WORK_PRECISION.png`
- `G3_FULL_MGS_T1_T4_SPEEDUP.png`

## Direct reading

1. Phi fusion is the most stable positive result: every plotted cell lies above unit speedup and work reduction is substantial.
2. Same-error performance is strongly regime-dependent. Complex Dahlquist and advection–diffusion are favorable; quadratic is below unit speedup; oscillatory PR is decisively unfavorable.
3. Step-economy and wall-time gains coincide only in the complex linearized regime. Oscillatory PR lies in the high-step/low-speed quadrant.
4. Four-thread within-step execution is below unit speedup for every current problem.
5. Residual and nested-dimension estimators do not track one another closely enough to be interchangeable.
6. Replacing the nested heuristic with the residual estimate adds substantial controller cost in most cells.

## CRAG

### Correctness

The plots use external error matching rather than equal internal tolerances. Missing target attainment is shown rather than imputed. Log scales are used where ratios span orders of magnitude.

### Retrieval/reference consistency

The fusion result matches the expected benefit of evaluating a phi linear combination through a single augmented exponential process. The small-state thread penalty is compatible with scheduler overhead dominating short kernels.

### Augmented checks

- nonnormality `0, 0.5, 0.9` included in phi fusion cells;
- dimensions `16, 64, 128` included;
- three tolerance levels included;
- T1/T4 execution compared with exact scientific identity;
- legacy nested estimator retained as a contrastive path.

### Generation/predictions

The current evidence predicts a thread crossover only at larger state/operator work, not at the present small dimensions. It also predicts that a new method/controller—not a simple Krylov micro-optimization—is required for oscillatory nonlinear PR.

## Adversarial mutations

| Mutation | Result | Claim disposition |
|---|---|---|
| External-error matching instead of equal `rtol` | aggregate advantage narrows and coverage becomes 11/12 | generic speedup narrowed |
| T1→T4 | same science, slower wall | small-state parallel speedup rejected |
| Nested→residual estimator | failures close, wall cost rises | residual estimator retained with caveat |
| Nonnormality increase | fused action stays accurate on bounded screen | fusion claim survives bounded range |
| Tight oscillatory target | one target missed, step ratio explodes | universal step economy rejected |

## Final claim classes

```text
SURVIVING
  fused augmented phi action reduces repeated Krylov work
  full-MGS pexprb54s4 is a serious expanded-holdout candidate

NARROWED
  same-error acceleration exists in favorable stiff/semilinear regimes
  parallel dependency depth is real but hardware speedup is size-dependent

REJECTED
  current candidate is a universal faster stiff solver
  four-thread within-step execution is beneficial on the current corpus
  IOP2 is ready as default
```
