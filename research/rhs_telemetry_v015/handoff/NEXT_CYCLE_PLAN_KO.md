# Next cycle — transactional common-W dispatcher v0.16

Target branch: `vectorized-jf-transactional-dispatch-v0.16`

## Goal

Turn the v0.15 advisory telemetry into a cheap, shadow-tested, transactional dispatcher without changing the protected numerical result until counterfactual evidence is available.

## Task 1 — Cheap online telemetry

- Build an 8x8 directional Gram matrix from WRMS/preconditioner-scaled RHS columns.
- Estimate rank/energy/cosine with small eigendecomposition or rank-revealing QR.
- Compare decisions with the offline SVD oracle on calibration and holdout traces.
- Count diagnostic dot products, vector passes, allocations, and wall time.

Kill gate: more than 1% unsafe under-ranking on holdout, or diagnostic cost larger than 5% of independent-solve time.

## Task 2 — Shadow dispatcher

For selected holdout batches, execute seeded, block, and independent solvers transactionally. Commit only the protected independent result. Record counterfactual success, true residual, work, and time.

Policy prototype:

```text
high risk / nonfinite / uncertain            -> independent Rayon
rank 1 + high cosine + low drift             -> seeded shared
rank 2-4 + mild risk + bounded drift         -> block GMRES
otherwise                                    -> independent Rayon
```

Require two consecutive semantically matching recommendations before speculative activation.

## Task 3 — Active transactional selection

- Seeded/block miss, residual growth, stagnation, or basis exhaustion triggers same-step independent retry.
- Failed speculative work remains in the ledger.
- Homotopy/SABR/recycle state commits only after the true-residual-certified solve succeeds.
- Path-level nonfinite failure bypasses linear dispatch and invokes protected path fallback.

## Task 4 — Integrated scientific gate

Compare protected sequential RODAS5P, SABR, homotopy q=0/1/2, and q=7 oracle under the same adaptive/external-error authority.

Promotion requires:

- explicit Jacobian builds = 0 in the candidate fast path;
- Newton iterations = 0 in the candidate fast path;
- false convergence = 0;
- global order >= 4.8;
- median propagation rounds <= 4 and p95 <= 6;
- fallback rate <= 10%;
- phase/damping/stiff/nonnormal regression absent;
- holdout same-error median speedup >= 1.15.

## Deferred

Implement BGMRES-DR/deflated restart only if actual active/shadow traces reproduce block rank-loss or basis-exhaustion failures often enough to affect the integrated result.
