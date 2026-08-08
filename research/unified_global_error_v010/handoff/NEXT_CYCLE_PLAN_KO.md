# 다음 cycle — adaptive-output authority와 production native anchors

권장 branch: `unified-adaptive-output-v0.11`

## Gate A — output observer

- integrator가 internal step을 저장하지 않고 common output times만 반환하는 observer API;
- fixed/adaptive 모든 family에 동일 적용;
- requested time을 넘어가지 않도록 step clipping;
- clipping event와 clipped-step cost 기록;
- failure/rejection rollback.

## Gate B — authority extension

- endpoint/max-grid/RMS-grid global error 유지;
- exact reference가 없는 문제를 위한 high-precision consensus reference schema;
- reference disagreement와 uncertainty 저장;
- tolerance ladder와 target-matched Pareto selection;
- randomized T1 repeated timing, T4 throughput 분리.

## Gate C — variable-order BDF

- variable step/order 1–5;
- Nordsieck/history state;
- LTE estimator와 PI controller;
- order 3–5 stability-limit monitor;
- Jacobian/factorization lagging;
- rejection transactional rollback.

## Gate D — adaptive Radau IIA3

- fixed anchor 보존;
- step-doubling 또는 embedded estimator 후보 비교;
- PI controller;
- full versus simplified Newton;
- stage transform/preconditioner 준비.

## Gate E — corpus

1. current analytic four-problem corpus;
2. Robertson/ROBER stiff kinetics;
3. Oregonator/E5-like nonlinear stiff system;
4. nonnormal semilinear PDE semi-discretization;
5. index-1 mass/DAE problem.

## Promotion rule

같은 external global-error bin에서 correctness/stability gate를 통과하고, paired T1 median wall speedup이 reference 대비 최소 1.15이며 false convergence와 stale state가 0일 때만 promote한다.
