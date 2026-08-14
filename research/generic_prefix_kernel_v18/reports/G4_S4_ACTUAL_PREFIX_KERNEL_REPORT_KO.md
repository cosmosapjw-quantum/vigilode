# G4-S4 Actual Reusable Prefix Kernel — Closeout Report

## 1. 연구 질문

G4-S3의 retrospective model은 E-K Arnoldi와 R-JF GMRES의 짧은 prefix가 method-entry 비용을 충분히 낮출 수 있다고 예측했다. G4-S4는 이 가정을 실제 Rust Krylov state로 바꾸어 다음을 묻는다.

> 1–2개의 실제 Arnoldi/GMRES vector를 계산한 뒤, 선택된 method가 그 basis/Hessenberg/least-squares state를 cold restart 없이 이어서 완료할 수 있는가? 그리고 두-vector prefix가 E-K/R-JF의 최종 wall-cost ratio를 sealed 19% 오차 안에서 예측하는가?

이 node에서는 active switching, physical client, BDF/Radau 수정이 금지된다.

## 2. 구현 경로

### E-K fused-phi prefix

`FusedPhiPrefixSession`은 augmented fused-phi operator에 대해 다음을 보존한다.

- full-MGS Arnoldi basis `V_m`;
- Hessenberg matrix `H_m`;
- projected residual history;
- latest projected action;
- convergence and happy-breakdown state.

`begin(..., prefix_dimension)`에서 계산한 state를 `finish()`가 그대로 이어서 확장한다. G4-S4 연구 kernel은 full-MGS, one-substep로 제한된다.

### R-JF GMRES prefix

`GmresPrefixSession`은 다음을 보존한다.

- preconditioned residual basis;
- search directions;
- Hessenberg matrix;
- small least-squares state;
- approximate solution and residual history.

`finish()`는 동일 state를 이어 쓰며, 최종 판정은 original-system true residual이다.

### 비용·실패 의미론

- explicit Jacobian build: 0;
- direct factorization: 0;
- Newton iteration: 0;
- prefix work와 resumed work를 모두 장부에 보존;
- prefix-selected result는 active solution으로 commit하지 않음.

## 3. Timing authority correction

첫 canonical attempt는 microsecond-scale operation을 seven one-shot timing으로 측정했다. 이 결과는 scheduler/clock noise가 지배하여 evidence에서 제외했다.

수치·threshold·prefix 선택은 바꾸지 않고, canonical timing만 다음처럼 고쳤다.

- automatic batching;
- canonical sample당 최소 10 ms;
- smoke sample당 최소 2 ms;
- warm-up과 batch calibration은 reported time에서 제외;
- operation당 median wall time 보고.

해당 amendment는 corrected canonical campaign보다 먼저 commit했다.

## 4. Canonical corpus

- dimensions: `N = 32, 128, 512`;
- regimes:
  - mild: stiffness 20, nonnormality 0.1, oscillation 5;
  - medium: stiffness 200, nonnormality 0.7, oscillation 20;
  - severe: stiffness 1000, nonnormality 0.9, oscillation 50;
- prefix dimensions: 1 negative control, 2 selected candidate;
- seven CPU-0-pinned corrected campaigns;
- all scientific/work fields identical across campaigns.

Scientific SHA-256:

```text
e4d80a2fcb04b5952f2e63ad7ca6eda4c4694197d8b27a4350c48e72222b3604
```

## 5. Exact reuse parity

두 selected method 모두 cold path와 prefix-resume path가 bitwise-equivalent output을 만들었다.

```text
maximum E-K action relative defect  0
maximum R-JF solution relative defect  0
```

즉 actual basis/Hessenberg reuse 자체는 correctness gate를 통과했다.

## 6. Prefix economics

Selected two-vector prefix의 ideal work-share 대비 비용 inflation은 다음이다.

| Method | Median | P95-higher | Gate |
|---|---:|---:|---:|
| E-K Arnoldi prefix | 0.422 | 0.656 | 1.9 |
| R-JF GMRES prefix | 0.595 | 0.751 | 1.9 |

둘 다 넉넉히 통과했다. Prefix를 계산한 뒤 같은 state를 이어서 완료한 total wall도 cold solve와 거의 같았다.

| Method | Median `(prefix+resume)/cold` | P95-higher |
|---|---:|---:|
| E-K | 1.0016 | 1.0741 |
| R-JF | 1.0012 | 1.0197 |

따라서 선택된 prefix를 재사용하는 구현은 full method 비용을 중복시키지 않는다.

## 7. Cost-ratio prediction failure

두-vector prefix가 예측한

\[
\widehat\rho_C
=
\frac{\widehat T_{E-K}}{\widehat T_{R-JF}}
\]

와 실제 cold wall ratio

\[
\rho_C
=
\frac{T_{E-K}}{T_{R-JF}}
\]

의 상대오차를 검사했다.

| Split | P95-higher relative error | Sealed gate |
|---|---:|---:|
| corrected N=32 calibration | 0.257 | 0.19 |
| N=128,512 holdout | 0.289 | 0.19 |
| combined median | 0.131 | — |
| combined P95-higher | 0.289 | 0.19 |

Seven canonical campaign의 selected-prefix P95는 모두 gate를 실패했다.

```text
range  0.2179 – 0.3998
campaigns passing  0 / 7
```

가장 큰 campaign-median miss는 `N=512`, stiffness 1000, nonnormality 0.9, oscillation 50에서 약 28.9%였다.

## 8. 실패 원인 해석

GMRES iteration 예측은 대체로 exact 또는 1–2 iteration 차이였다. 반면 fused exponential path에서는 predicted Krylov dimension이 실제보다 보수적으로 컸고, projected exponential·orthogonalization 비용이 Krylov dimension에 비선형으로 의존했다.

따라서 짧은 prefix에서 본 residual contraction과 prefix wall을 단순 선형 외삽하면 method 간 비용비율을 19% 이내로 안정적으로 맞추지 못한다. 이는 basis reuse의 실패가 아니라 point cost predictor의 실패다.

## 9. 판정

### PROMOTE

- actual reusable E-K Arnoldi prefix kernel;
- actual reusable R-JF GMRES prefix kernel;
- exact cold/prefix-resume parity;
- failed/speculative work preservation;
- auto-batched timing authority;
- one-vector negative control의 명시적 보존.

### HOLD

- two-vector point cost-ratio predictor;
- active method switching;
- post-hoc prefix-length 또는 19% threshold 재조정;
- full-MGS one-substep 결과를 general KIOPS predictor로 확대;
- physical-client execution.

```text
G4_S4_PREFIX_KERNEL              PROMOTE
G4_S4_PREFIX_REUSE_PARITY        PROMOTE
G4_S4_TWO_VECTOR_COST_PREDICTOR  HOLD
G4_S4_ACTIVE_SWITCHING           HOLD
```

## 10. 다음 blocker

다음 node는 point estimate를 더 맞추는 post-hoc tuning이 아니라, expanded sealed telemetry에서

\[
[\rho_C^-,\rho_C^+]
\]

형태의 conservative cost-ratio interval을 calibration하고 holdout에서 검증해야 한다. Selector는 interval 전체가 economic margin의 한쪽에 놓일 때만 추천하고, 나머지는 abstain해야 한다.
