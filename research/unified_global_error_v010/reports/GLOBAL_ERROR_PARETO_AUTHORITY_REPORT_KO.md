# Global-error / total-cost Pareto authority — v0.10 연구·구현 보고서

Date: 2026-08-08 Asia/Seoul
Base tag: `unified-native-v0.9.0-alpha1`
Branch: `unified-global-error-pareto-v0.10`

## 1. 목적

이 cycle의 목적은 solver 내부 tolerance, timestep 수, Newton iteration 수를 직접 비교하지 않고, 동일한 IVP와 동일한 외부 output grid에서 측정한 global error와 실제 비용의 Pareto frontier를 구축하는 것이다.

이번 P0 범위는 평가 authority 자체의 검증이며 production solver ranking이 아니다. 대상은 다음 fixed-step anchor 다섯 개다.

- protected sequential RODAS5P/direct;
- BDF1;
- BDF2 with BDF1 startup;
- Radau IIA 1-stage;
- Radau IIA 3-stage, order 5.

## 2. 비교 계약

### 2.1 Reference

P0에서는 analytic exact solution이 있는 네 문제만 사용했다.

- scalar linear;
- Prothero–Robinson;
- manufactured nonnormal vector;
- manufactured nonlinear noncommuting-mass system.

각 reference는 problem id, common-grid hash, state hash, source kind와 uncertainty를 저장한다. Reference uncertainty는 analytic exact lane이므로 0이다.

### 2.2 Common output grid

모든 fixed-step ladder가 정확히 통과하는 coarsest nested grid를 사용했다. 후보에 grid time이 없으면 interpolation하지 않고 explicit failure로 처리한다.

### 2.3 External error

각 output time에서

\[
e_j(t)=y_j^{\rm candidate}(t)-y_j^{\rm ref}(t)
\]

를 계산하고 endpoint, maximum-grid, RMS-grid의 L2/WRMS를 저장했다. P0 primary error는

\[
E_{\max,L^2}=\max_{t\in\mathcal T}\|e(t)\|_2
\]

이다. WRMS weight는 reference state에 고정해 solver family와 독립적으로 정의했다.

### 2.4 Cost

임의의 weighted scalar score를 만들지 않고 다음을 별도 Pareto 축으로 유지했다.

- one-thread wall median and IQR;
- RHS evaluations;
- JVP vectors;
- Jacobian builds;
- direct factorizations;
- nonlinear iterations;
- linear iterations;
- accepted steps;
- returned trajectory bytes.

마지막 항목은 deterministic retained bytes이며 peak RSS가 아니다.

### 2.5 Timing

T1은 warm-up/calibration 후 candidate/problem/h 순서를 seed `20260808`로 무작위화하고 5회 반복했다. 각 timing sample이 최소 2 ms가 되도록 동일 candidate execution을 4–542회 batch하여 per-execution latency로 정규화했다.

T4는 local Rayon pool에서 independent scientific jobs를 병렬 실행한다. T4 per-task latency는 authoritative하지 않으며, one-pass scientific-suite throughput과 scientific checksum만 비교한다.

## 3. 구현

`rodas5p-fair-ab::global_error`에 다음을 추가했다.

- `CommonOutputGrid`;
- `ExternalErrorScale`;
- `ReferenceSolutionProvenance` / `ReferenceTrajectory`;
- `GlobalErrorMetrics`;
- `IntegratorRunRecord`;
- `GlobalErrorParetoFront`;
- `GlobalErrorTarget` / `TargetAttainment`;
- fixed-anchor dispatch;
- randomized calibrated timing;
- vector-cost Pareto construction;
- timing-independent scientific checksum;
- local-Rayon deterministic screen.

CLI에는 다음 명령을 추가했다.

```text
rodas5p global-error-pareto --profile smoke|canonical --threads N --output FILE
```

## 4. Canonical 결과

```text
references       4
runs            70
failures         0
fronts         252
targets          4
attainments    144
```

T1, T1 rerun, T4 scientific checksum은 모두 다음으로 일치했다.

```text
72cb6ba257685f2bd73f16976717a98eefeab6d0821851e29fa335147c8f706e
```

Comparable one-pass scientific-suite wall time은 다음이었다.

```text
T1  0.005845779 s
T4  0.002144324 s
T4 throughput speedup  2.726x
```

이 수치는 independent-case throughput이며 solver 내부 stage parallelism의 증거가 아니다.

T1 timing campaign은 0.7403 s였고, 별도 T1 rerun에서 record별 wall median ratio는 다음 범위였다.

```text
median  1.0062
Q25     0.9768
Q75     1.0223
min     0.8819
max     1.1357
```

즉 batching으로 중앙부 안정성은 개선됐지만, 가장 작은 dense kernels의 개별 record에는 여전히 약 10% 수준의 tail variation이 있다.

## 5. Numerical observations

Observed order는 expected anchor behavior와 대체로 일치했다.

- BDF1/Radau IIA1: 약 1차;
- BDF2: 약 2차;
- Radau IIA3: 약 5차;
- RODAS5P: roundoff floor 전 약 5차.

고차 방법의 finest pair 일부에서는 error가 binary64 floor에 접근해 local slope가 무의미해졌다. 이 slope는 ranking gate로 쓰지 않았다.

Wall-error frontier 참여와 target attainment는 문제·error level에 따라 달라졌다. BDF2는 낮은 비용/중간 정확도 영역, Radau IIA3는 높은 정확도 영역에서 자주 frontier에 나타났고, RODAS5P도 일부 problem/error 구간의 frontier에 남았다. 이 corpus만으로 일반 순위를 주장할 수 없다.

## 6. 발견하고 수정한 blocker

### 6.1 Execution clock 의미 혼동

초기 draft는 T1의 전체 repeated timing campaign과 T4의 one-pass scientific suite를 같은 `aggregate_wall_seconds`에 넣었다. 이 값으로 thread speedup을 계산하면 수백 배의 가짜 speedup이 생긴다.

다음을 분리했다.

- `scientific_suite_wall_seconds`: T1/T4 비교 가능;
- `timing_campaign_wall_seconds`: authoritative T1에만 존재.

회귀 테스트로 두 clock의 존재 조건을 잠갔다.

### 6.2 Microbenchmark noise

원래 candidate latency가 수십 microsecond라 one-shot timing이 불안정했다. 2 ms minimum batch timing을 도입해 median/IQR를 안정화했다.

## 7. 남은 blocker

### B1 — output storage contamination

현재 fixed integrator API는 every internal step을 반환한다. 더 작은 h를 쓰는 후보가 더 많은 trajectory allocation/storage 비용을 부담한다. Production work-precision 비교 전에 `save_internal_steps=false` 또는 common-output observer가 필요하다.

### B2 — adaptive common-output contract

Adaptive BDF/Radau가 arbitrary step을 택하면 현재 no-interpolation grid를 직접 통과하지 않는다. 다음 중 하나를 명시적으로 선택해야 한다.

1. common output times에서 step clipping;
2. family-specific dense output을 엄격히 검증;
3. endpoint-only P1 authority.

권장안은 P1에서 step clipping을 공통 observer contract로 구현하고, dense output은 별도 gate로 둔다.

### B3 — corpus scale

현재 문제는 small dense authority-validation corpus다. Wall ranking에는 larger stiff kinetics, nonnormal systems, sparse semi-discretized PDE, index-1 DAE가 필요하다.

### B4 — reference hierarchy

다른 benchmark에는 analytic solution이 없을 수 있다. high-precision single-solver reference가 특정 family를 편향하지 않도록 consensus/reference-disagreement lane이 필요하다.

### B5 — peak memory

현재 stored-state bytes는 deterministic returned trajectory size뿐이다. Peak RSS/allocator traffic은 별도 계측이 필요하다.

## 8. 판정

### PROMOTE

- method-independent external global-error contract;
- exact-reference provenance and checksum;
- no-interpolation common-grid validation;
- vector-cost Pareto fronts and target attainment;
- randomized calibrated T1 timing;
- deterministic T4 throughput screen;
- failure-preserving result schema;
- `global-error-pareto` CLI.

### HOLD

- 현재 다섯 solver의 일반 성능 순위;
- adaptive solver 비교;
- sparse/MPI/GPU/DAE ranking;
- peak-memory claim;
- 2.7x T4 throughput을 solver-internal speedup으로 해석하는 주장.

## 9. 다음 cycle

다음 branch에서는 authority를 확장한 뒤 variable-order BDF와 adaptive Radau를 추가한다.

1. output observer와 `save_internal_steps=false`;
2. adaptive common-grid step clipping;
3. external tolerance ladder and target-matched global error;
4. variable-order BDF 1–5;
5. adaptive Radau IIA3;
6. larger stiff corpus;
7. 그 후 parallel SDC, Peer/W, ROK/BOROK, exponential/Leja를 순차 편입.
