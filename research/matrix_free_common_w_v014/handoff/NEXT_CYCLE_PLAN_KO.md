# 다음 cycle — Actual Stage-RHS Telemetry + Transactional Backend Dispatch

## Branch

```text
vectorized-jf-homotopy-dispatch-v0.15
```

## 중심 질문

q=0/1/2 partial-coupling homotopy와 SABR의 실제 8개 common-W RHS가 각 round에서 어떤 numerical rank와 subspace drift를 가지며, 그 정보로 seeded/block/independent backend를 안전하게 선택할 수 있는가?

## Task 1 — read-only RHS telemetry

각 homotopy/SABR round에서 다음을 저장한다.

\[
\sigma_1\ge\cdots\ge\sigma_8,
\qquad
r_\epsilon=\#\{\sigma_i/\sigma_1>\epsilon\},
\]

- raw and preconditioned numerical rank;
- RHS pairwise cosine matrix;
- round-to-round principal angles;
- residual norm spread;
- stiffness/nonnormality proxy;
- stage and timestep identifiers.

첫 commit은 solver behavior를 바꾸지 않는다.

## Task 2 — deterministic transactional dispatcher

초기 policy 후보:

```text
rank approximately 1:
  seeded-shared GMRES
low rank + acceptable risk proxy:
  block GMRES
otherwise:
  independent GMRES using 2/4 local Rayon threads
```

Block failure, stagnation, or residual growth 시 같은 timestep에서 independent GMRES로 retry한다. Failed block work는 retained cost다.

## Task 3 — integrated q=0/1/2 experiment

같은 adaptive RODAS contract에서:

- sequential protected RODAS5P;
- SABR;
- homotopy q=0,1,2;
- q=7 correctness oracle;
- fixed backend and dispatcher backend

를 비교한다.

## Scientific gates

\[
p_{\rm global}\ge4.8,
\qquad
R(-\infty)\approx0,
\]

- oscillatory amplitude/phase regression 없음;
- false acceptance 0;
- median propagation rounds ≤ 4;
- P95 rounds ≤ 6;
- fallback rate ≤ 10%;
- explicit Jacobian builds = 0;
- fast-path Newton iterations = 0.

## Performance gate

동일 external global error에서

\[
T_{\rm sequential}/T_{\rm candidate}\ge1.15
\]

을 holdout median으로 요구한다. Failed speculative work와 certificate/fallback 비용을 모두 포함한다.

## Deferred

Block GMRES-DR/deflated restart는 actual stage traces에서 current block failure/stagnation rate가 실질적일 때만 구현한다. 먼저 fallback 가능한 dispatcher가 실제 relevance를 측정한다.
