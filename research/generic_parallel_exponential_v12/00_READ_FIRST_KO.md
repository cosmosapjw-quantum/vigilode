# G2 Parallel Exponential Rosenbrock Foundation — 먼저 읽기

이번 checkpoint는 generic vectorized/Jacobian-free stiff solver DAG의 **G2 correctness foundation**이다.

구현·검증한 것은 다음이다.

- dense projected-space matrix exponential/φ-function oracle;
- strictly matrix-free Arnoldi φ-action reference;
- EXPRB2와 EXPRB43;
- published parallel fifth-order method `pexprb54s4`;
- 실제 local-Rayon dependency-level concurrency;
- published coefficients, selected stiff-order identities, nonlinear observed order, stiff/oscillatory linear limits;
- independent 100-digit cross-check.

Canonical 결과:

- EXPRB2 observed order: `1.9559710426`;
- EXPRB43 observed order: `3.9248606117`;
- pexprb54s4 observed order: `4.9287704282`;
- maximum φ-action relative error: `6.5693e-16`;
- maximum selected order-condition residual: `2.2204e-16`;
- maximum stiff-linear error: `1.3989e-14`;
- explicit Jacobian builds / direct factorizations / Newton iterations: all zero.

그러나 **performance promotion은 금지**다. 현재 full-Arnoldi reference는 separate φ-action을 method step마다 반복한다. Scalar nonlinear screen에서 pexprb54s4는 logical critical depth가 3이지만 `15`개의 separate φ-actions와 `18`개의 JVP vectors를 사용했다. KIOPS-style fused linear-combination action, adaptive Krylov substepping/restart, nonautonomous augmentation, adaptive timestep과 same-error wall benchmark가 아직 없다.

판정:

```text
G2 coefficient/order/φ foundation  PROMOTE
G2 generic accelerated solver       HOLD
next node                            generic-parallel-exp-adaptive-v1.3
```
