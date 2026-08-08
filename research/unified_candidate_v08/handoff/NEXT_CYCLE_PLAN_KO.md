# 다음 통합 후보 연구 계획 — native Rust integrator expansion v0.9

## 출발 원칙

현재 v0.8 registry, C0/C1/C2/C3 record, global-order/stiff/nonnormal gates, Tier-L/Tier-N performance overlay, Rayon deterministic runner를 변경하지 않는다. 새 후보가 기존 후보보다 유리하도록 tolerance, output norm, test case 또는 promotion threshold를 바꾸지 않는다.

## 구현 순서

### E1 — BDF control

- variable-order history가 아니라 먼저 fixed BDF1/BDF2 correctness adapter;
- 동일 IVP/output/event/failure contract;
- nonlinear solve와 linear solve work를 별도 계측;
- 이후 order 1–5, step/order controller;
- discontinuity/restart semantics 테스트.

### E2 — Radau/IRK control

- 1-stage Radau IIA/backward Euler를 contract anchor로 시작;
- Radau IIA 3-stage fifth-order 또는 equivalent high-order control;
- stage-coupled nonlinear residual과 Jacobian work 계측;
- transformed/stage-parallel linear algebra는 correctness 이후 추가.

### E3 — Parallel SDC

- Radau collocation nodes와 serial SDC oracle;
- diagonal stiff/nonstiff sweeper;
- nonstationary/FLEX schedule;
- sweep count, order gain, stability, actual case-level/stage-level parallel work.

### E4 — Peer/W

- fixed-step peer state and startup;
- variable-step zero-stability gate 이전에는 fixed-step comparison만 허용;
- independent stage solves and work ledger.

### E5 — Rosenbrock–Krylov / BOROK

- method coefficients와 order conditions를 별도 registry entries로 고정;
- shared Krylov basis dimension and JVP work;
- transpose-JVP requirement가 있는 BOROK path는 명시적으로 계측.

### E6 — Exponential Rosenbrock / Leja

- semilinear problem class를 별도 capability claim으로 표시;
- phi-function action accuracy and spectral/scaling work;
- general mass-matrix/DAE claim 금지.

## 공통 gate

각 family는 다음을 모두 통과해야 executable로 승격한다.

1. deterministic exact/known-solution oracle;
2. declared temporal order;
3. stiff stability appropriate to the method claim;
4. same global solution WRMS tolerance;
5. fallback/rejection and restart accounting;
6. 1/4-thread scientific identity;
7. no hidden external backend difference;
8. 1.15x measured wall gate for performance promotion;
9. no ranking until the complete-integrator adapter is present.

## 병렬화 순서

1. independent cases via Rayon;
2. candidate/configuration parallelism only when it does not oversubscribe case threads;
3. stage/multiple-RHS parallelism with explicit scheduler ownership;
4. SIMD/manual layout work only after profiling;
5. nested parallelism disabled by default.

## 다음 branch 이름

`unified-native-integrators-v0.9`
