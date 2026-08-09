# Independent Diff Review — v0.13 → v0.14

## Scope reviewed

Base: `1a1f4f566b8851ac07183aa2232caab1efc024d4`

Changes reviewed:

- batched `LinearOperator` contract;
- local ordered Rayon mutation path;
- block GMRES and seeded shared GMRES;
- matrix-free common-W gate and CLI;
- correctness, failure, and budget tests;
- canonical result/report generation.

## Strengths

1. The candidate operator is genuinely explicit-J-free and factorization-free.
2. Every successful solve is checked against an unpreconditioned true residual.
3. Failed block solves retain attempted operator and orthogonalization work.
4. The same tolerance, preconditioner, RHS, and operator are used across solver arms.
5. A subtle tiny-nonzero-RHS rank-loss bug was fixed with a regression test.
6. Thread-dependent failures are absent; the canonical failure cluster is numerical, not a race.

## Important limitations

1. `block-gmres` is nonrestarted and has a hard 128-vector cap; 36 canonical failures prohibit default promotion.
2. Current block least-squares and orthogonalization are research implementations, not optimized production kernels.
3. Synthetic exact RHS rank is not evidence for actual RODAS/homotopy stage rank.
4. The analytic diagonal preconditioner is fair across arms but target-problem availability is not established.
5. Integrated fifth order, stiff decay, phase fidelity, nonlinear certification, and fallback economics are outside this gate.

## Review verdict

```text
PASS WITH SCIENTIFIC HOLD
```

The matrix-free common-W gate is valid as a research/correctness and backend-selection experiment. Block GMRES must remain optional with protected independent-GMRES fallback until actual stage-RHS telemetry and integrated solver gates are available.
