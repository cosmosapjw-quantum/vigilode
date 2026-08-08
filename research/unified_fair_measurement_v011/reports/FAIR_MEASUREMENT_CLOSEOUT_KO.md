# Unified fair-measurement authority v0.11 — 종료 보고서

Date: 2026-08-08 Asia/Seoul
Base tag: `unified-global-error-v0.10.0-alpha1`
Scope: fixed-step, small dense, regular-ODE anchor comparison

## 1. 결론

이번 cycle은 solver 구현을 추가하기보다 비교 절차의 마지막 구조적 편향을 제거했다.

1. 모든 candidate는 동일한 analytic reference와 동일한 requested output grid에서 평가된다.
2. internal integration step은 더 이상 반환 trajectory나 storage cost에 포함되지 않는다.
3. dense-output interpolation을 사용하지 않고 requested time 직전 step clipping을 모든 family에 동일하게 적용한다.
4. internal step 수와 output 때문에 잘린 step 수는 storage와 분리해 work ledger에 기록한다.
5. 동일 Rust 1.94.1, `faer`, 동일 optimized measurement binary와 동일 thread contract를 사용한다.
6. 실패는 보존되고, scientific checksum은 T1/T4에서 동일하다.

따라서 **fixed-step anchor의 공정 비교 기반 구축은 종료한다.** 이것은 전체 stiff-solver 연구나 production ranking의 종료가 아니다. Adaptive BDF/Radau, dense output, sparse/DAE, SDC·Peer/W·ROK·BOROK·Leja는 별도의 후속 연구 단계다.

## 2. 제거한 편향

v0.10까지는 각 integrator가 모든 internal step을 반환했다. 작은 timestep을 쓰는 method는 실제 계산비 외에도 더 많은 state를 할당·보존하는 비용을 부담했다. 이는 solver 자체의 비용과 output policy 비용을 혼합한다.

v0.11에서는 `OutputSchedule`과 `ObservedIntegrationResult`를 도입했다.

- `save_internal_steps = false`
- `dense_output_used = false`
- `landing = step-clipping`

Fixed-step RODAS5P, BDF1/2, Radau IIA1/3와 기존 adaptive RODAS/SABR observer path가 같은 collector를 사용한다. Rejected adaptive trial은 collector를 갱신하지 않으므로 observer state도 transactional하다.

## 3. Scientific invariance

v0.10과 v0.11의 동일 70개 record를 비교했다.

- status mismatch: 0
- work-counter mismatch: 0
- problem/output-grid/reference identity mismatch: 0
- 최대 L2 error-metric 절대 차이: `2.7277171407019685e-16`
- old L2 error가 `1e-10`보다 큰 경우 최대 상대 차이: `1.0125955984515875e-10`

즉 trajectory storage policy를 바꾸면서 solver work path와 scientific status는 유지됐다. WRMS의 최대 절대 차이 `1.920125214383006e-7`은 큰 scaling factor가 곱해진 값이며 대응 L2 차이는 roundoff 수준이다.

## 4. Storage bias 제거 결과

모든 problem/output-grid pair에서 candidate별 `stored_state_bytes`가 정확히 같아졌다.

- new/old storage ratio 최소: `0.1463414634`
- 중앙값: `0.5454545455`
- 최대: `1.0`
- 절약 byte 중앙값: `148`
- 최대: `1400`

Candidate family별 중앙 storage ratio도 모두 `0.5454545455`로 같았다. Storage는 이제 timestep 수가 아니라 requested output count와 state dimension만 반영한다.

## 5. Timing protocol과 재현성

Fat-LTO link는 현재 제한된 실행환경에서 반복적으로 완료되지 않았으므로, 별도의 optimized measurement profile을 고정했다.

```toml
[profile.measurement]
inherits = "release"
lto = false
codegen-units = 16
```

모든 candidate는 같은 binary와 같은 flags를 사용한다. 이 profile의 절대 시간은 fat-LTO v0.10 binary와 직접 비교하지 않으며, v0.11 내부의 paired relative comparison에만 사용한다.

CPU affinity와 thread-related environment를 고정해 5개의 T1 campaign과 3개의 T4 campaign을 실행했다.

- T1 scientific-suite median: `0.007335916 s`
- T4 scientific-suite median: `0.004663924 s`
- case-level throughput ratio T1/T4: `1.5729064`
- record wall-median CV 중앙값: `0.04630`
- 75 percentile: `0.05973`
- 최대: `0.12433`
- 8개 campaign scientific checksum: 모두 동일

T4 수치는 independent case-level throughput이며 solver 내부 stage 병렬 speedup으로 해석하지 않는다.

## 6. Canonical fixed-anchor descriptive result

현재 작은 dense fixed-step corpus에서 Pareto-front membership은 다음과 같다.

- sequential RODAS5P direct: 562
- Radau IIA3 fixed: 259
- BDF2 fixed: 126
- BDF1 fixed: 21

Target-attainment winner count는 각각 110, 52, 12, 2였다. 이 수치는 현 corpus의 descriptive result일 뿐 production/adaptive ranking이 아니다. Radau IIA1은 BDF1과 같은 implicit-Euler anchor이고 현재 cost/error surface에서 지배됐다.

## 7. 공정성 gate

| Gate | 결과 |
|---|---|
| 동일 pure-Rust backend | PASS |
| campaign별 동일 optimized binary | PASS |
| 동일 IVP/reference/output grid | PASS |
| internal-step 저장 제거 | PASS |
| 동일 step-clipping landing | PASS |
| 실패 보존 | PASS |
| vector work ledger 보존 | PASS |
| randomized repeated timing | PASS |
| pinned T1 closeout campaign | PASS |
| T1/T4 scientific identity | PASS |
| nondividing step clipping path | PASS — contract test |
| adaptive rejection observer rollback | PASS — contract test |

## 8. 종료 범위와 금지 주장

### 종료

- fixed-step complete-integrator anchor의 global-error/total-cost authority
- common-output observer
- internal-step storage bias 제거
- step-clipping reference lane
- same-binary T1/T4 scientific reproducibility

### 종료하지 않음

- production adaptive BDF/Radau ranking
- dense-output interpolation correctness와 cost
- analytic reference가 없는 문제의 consensus/uncertainty hierarchy
- large sparse, MPI, GPU, singular-mass/index-1 DAE ranking
- 아직 구현되지 않은 parallel SDC, Peer/W, ROK, BOROK, exponential/Leja
- 일반적인 “RODAS5P가 BDF/Radau보다 빠르다”는 주장

## 9. 최종 판정

```text
PROMOTE:
  fixed-step fair-measurement authority
  requested-output-only storage contract
  step-clipping landing contract
  optimized same-binary measurement profile
  T1/T4 deterministic scientific reporting

HOLD / SEPARATE FUTURE PHASE:
  adaptive production solvers
  dense output
  broad sparse/DAE corpus
  remaining solver families
  general solver ranking
```

이로써 **공정 측정 기반 구축 작업은 종료**한다. 이후 solver 추가와 adaptive 비교는 이 authority를 소비하는 별도 연구 project로 취급한다.
