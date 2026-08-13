# G4-S0 Regime Telemetry + Offline Oracle Map 연구 보고서

## 1. Current DAG state

### 완료 노드

- G0 generic scope reset
- G1 transactional q1→q2 infrastructure — generic accelerator는 HOLD
- G2 exponential-Rosenbrock correctness foundation
- G3 fused adaptive `pexprb54s4` — expanded holdout candidate
- fixed-step fair-measurement authority
- adaptive common controller/output/work contract

### 현재 노드

`G4-S0 — Read-only regime telemetry and offline oracle map`

이 node의 목적은 candidate를 수정하거나 switching을 활성화하는 것이 아니라, 동결된 후보들 사이에
실제로 regime complementarity가 존재하는지와 현재 telemetry가 이를 값싸게 예측할 수 있는지를 검사하는 것이다.

### 금지 작업

- BDF/Radau source 변경
- G3 `pexprb54s4` coefficient/controller retuning
- active switching
- physical-client 실행
- homotopy path parameter 확장

## 2. CLEAN_CONTEXT

중심 문제는 하나의 method가 모든 regime에서 우월하지 않다는 것이다.
G3 fused exponential method는 linearization-dominated complex/stiff 및 semilinear problems에서 강했지만,
quadratic nonlinear 및 oscillatory nonlinear PR에서는 protected matrix-free RODAS5P가 강했다.
따라서 다음 질문은 “어느 method가 best인가?”가 아니라 다음이다.

> 동일 external error에서 다음 국소 구간의 unit-time cost가 가장 작은 admissible method를,
> 별도 분석비용을 거의 들이지 않고 예측할 수 있는가?

## 3. Contract

### 수학/수치 contract

- 수치형: IEEE binary64
- 대상: regular ODE, 현재 corpus의 identity mass matrix lane
- external target: 각 problem에서 protected JF RODAS5P가 달성한 endpoint error 세 수준
- same-error matching: candidate는 자신의 tolerance ladder 중 target 이하를 만족하는 가장 싼 run을 사용
- speedup:
  \[
  S=\frac{T_{\rm protected\ JF\ RODAS5P}}{T_{\rm candidate}}.
  \]
- strict-JF portfolio:
  - protected JF RODAS5P (`R-JF`)
  - fused full-MGS `pexprb54s4` (`E-K`)
  - held q1→q2 causal corrector (`C-JF`)
- auto portfolio: strict-JF + frozen Radau IIA3 (`R-J`) + frozen BDF2

### 비용 contract

- 7개 frozen G3 campaigns의 wall median 사용
- 실패 candidate는 target을 cover하지 못함
- rejected/speculative work는 G3 ledger의 값을 그대로 사용
- candidate code 또는 controller는 변경하지 않음
- target multiplier 1.00/1.05/1.20 sensitivity를 별도 기록

### selector contract

이번 node에서는 selector를 구현하지 않는다.
Offline oracle winner와 cheap-feature separability만 계산한다.

## 4. 실행 결과

### 4.1 Strict-JF offline oracle

| 항목 | 결과 |
|---|---:|
| target cells | 12 |
| covered | 12 |
| E-K winner | 6 |
| protected R-JF winner | 6 |
| aggregate sum-cost speedup | 1.5340× |
| median cell speedup | 1.2597× |
| maximum cell speedup | 7.5984× |

문제별 winner geometry는 명확했다.

- complex Dahlquist: E-K가 3개 target 모두 승리
  \[
  S=1.52,\;3.37,\;7.60.
  \]
- advection–diffusion: E-K가 3개 target 모두 승리
  \[
  S=1.68,\;1.68,\;2.05.
  \]
- quadratic: protected R-JF가 3개 target 모두 승리
- oscillatory PR: protected R-JF가 3개 target 모두 승리

따라서 strict-JF portfolio에는 실질적인 switching opportunity가 존재한다. 그러나 protected R-JF만이 12/12 target을 cover하는 가장 싼 full-coverage static method였다. E-K는 11/12, held q1→q2는 12/12지만 훨씬 비쌌다.

### 4.2 Auto offline oracle

| 항목 | 결과 |
|---|---:|
| target cells | 12 |
| covered | 12 |
| frozen Radau winner | 8 |
| E-K winner | 3 |
| protected R-JF winner | 1 |
| aggregate sum-cost speedup | 2.2504× |
| median cell speedup | 4.1882× |
| maximum cell speedup | 9.0719× |

Auto oracle은 Jacobian-based method가 건강한 영역에서 이를 사용하는 것이 경제적으로 타당할 수 있음을 보여준다. 다만 frozen Radau는 small dense, explicit-J/direct reference implementation이며, production sparse/large-scale switch authorization으로 해석하면 안 된다.

### 4.3 Feature separability

현재 추가 work 없이 얻는 feature는 다음이다.

- accepted/rejected counts
- embedded time-error telemetry
- \(\varphi\)-residual/error telemetry
- Krylov vectors/restarts/orthogonalization
- dimension, tolerance metadata

Full-sample에서는 `dimension` 또는 E-K의 JVP/Krylov work가 winner를 완전히 분리했지만, 이는 현재 problem family와 dimension이 강하게 confounded된 결과다. Leave-one-problem-out에서 최고 평균 accuracy는 0.75에 그쳤다. 따라서 한 개의 zero-extra-work threshold로 active switching을 허가할 근거는 없다.

### 4.4 JVP-quality probe cost

Fresh finite-difference JVP quality를 \(\epsilon\)과 \(\epsilon/2\)로 비교하는 3-RHS probe의 cost는 protected RODAS RHS work 대비:

\[
\operatorname{median}=5.357\%,
\qquad
\max=12.5\%.
\]

따라서 매 step probe는 너무 비싸다. Mode entry, 연속 rejection, cooldown expiration 같은 sparse event에서만 sampling해야 한다.

### 4.5 Timing robustness

7개 campaign의 scientific fields는 동일했다. Strict-JF winner는 대부분 안정적이지만 quadratic tight target에서 campaign winner가 RODAS 5회, E-K 2회로 갈렸다. Auto portfolio에는 complex loose target의 E-K/Radau near tie와 PR 일부 target의 Radau/RODAS near tie가 있다. 따라서 selector는 작은 predicted advantage에서 switch하면 안 된다.

## 5. 핵심 해석

1. Regime complementarity는 실제로 존재한다.
2. Best static method와 oracle portfolio 사이에 경제적 gap이 있다.
3. 그러나 현재 feature set은 out-of-family generalization을 입증하지 못했다.
4. 별도 JVP-quality probe는 매-step detector로는 비싸다.
5. 따라서 다음 단계는 active switching이 아니라 accepted-step-local shadow telemetry다.

## 6. Scope-limited conclusion

이번 결과는 four-problem, small-state, frozen G3 corpus의 read-only oracle map이다. LSODA식 switching의 가능성을 지지하지만 selector의 안전성, global order under switching, switch/cache cost, hysteresis, long-term residence dynamics는 아직 검증하지 않았다.
