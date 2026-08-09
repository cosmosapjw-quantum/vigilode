# RODAS5P actual SABR/homotopy RHS telemetry cycle v0.15 — 연구·코딩 보고서

Date: 2026-08-10 Asia/Seoul
Base tag: `vectorized-jf-common-w-v0.14.0-alpha1`
Working branch: `vectorized-jf-homotopy-dispatch-v0.15`

## 1. 목적과 범위

이번 cycle의 목적은 synthetic exact-rank matrix로 backend를 고르는 것이 아니라, **실제 SABR5P와 partial-coupling homotopy \(q=0,1,2\) 경로에서 common-\(W\)에 들어가는 8개 RHS의 구조를 읽기 전용으로 계측**하는 것이다.

보호 경계는 다음과 같다.

- dispatcher는 비활성 상태다.
- backend recommendation은 advisory일 뿐 solver 동작을 바꾸지 않는다.
- protected homotopy path와 observed path의 결과·오류·work counter가 같아야 한다.
- 실패한 speculative path에서 이미 생성된 telemetry row를 보존한다.
- BDF/Radau comparator는 변경하지 않는다.
- 이번 결과는 explicit-J-free common-\(W\) 성능 측정이 아니다. Telemetry 생성의 reference path는 기존 dense Jacobian과 factorization을 그대로 사용한다.

## 2. 계측량

각 8-RHS batch를 component-by-stage column matrix

\[
B=[b_1,\ldots,b_8]\in\mathbb R^{N\times8}
\]

로 보고 다음을 기록했다.

- singular values \(\sigma_1\ge\cdots\ge\sigma_8\);
- relative numerical rank;
- 99%와 99.9% energy rank;
- stable rank
  \[
  r_s(B)=\frac{\|B\|_F^2}{\|B\|_2^2};
  \]
- RHS norm spread;
- pairwise cosine;
- matching phase/propagation-level batch 사이 principal angle;
- manufactured stiffness/nonnormality risk label.

Magnitude 차이만으로 rank-one처럼 보이는 문제를 줄이기 위해, 각 RHS를 unit norm으로 만든 directional matrix

\[
\widehat B=
\left[
\frac{b_1}{\|b_1\|_2},\ldots,
\frac{b_8}{\|b_8\|_2}
\right]
\]

도 별도로 분석했다. Backend recommendation은 transformed directional analysis에만 기반한다.

## 3. 실제 instrumented path

### SABR5P

Zero predictor에서 최대 3회의 fixed-point RHS를 계측했다. 각 iteration의 nonlinear RHS와 common-\(W\) solve 뒤의 transformed vectors를 기록했다.

### Partial-coupling homotopy

다음을 전수했다.

\[
\theta\in\{0,0.5,1\},\qquad
q\in\{0,1,2\},
\]

predictor는 Euler와 AB2, path round는 3, correction은 0이다. Path-start, tangent, correction-compatible propagation layer를 observer event로 분리했다.

Telemetry instrumentation은 concrete recorder를 production homotopy code에 직접 결합하지 않고 `HomotopyBatchObserver` interface를 통해 연결했다. Standard path는 no-op observer로 동일 internal routine을 사용한다.

## 4. Canonical corpus

총 9개 case를 사용했다.

- affine noncommuting mass matrix;
- complex Dahlquist \(N=8\);
- stiff complex Dahlquist \(N=32\);
- scalar Prothero–Robinson;
- manufactured vector \(N=8,16,32\);
- stiffness \(10^2,10^4,10^6\);
- nonnormality proxy \(0,0.2,0.9\).

Canonical 결과:

```text
cases                              9
telemetry rows                  1215
failed paths                       2
behavior comparisons             162
behavior mismatches                0
solver behavior changed        false
reference Jacobian builds        171
reference factorizations         189
dispatcher active              false
explicit-J builds in dispatcher    0
```

Scientific checksum:

```text
0d0933596500b58c52d01bac7d58b3cd95141e566afb8243a195ece1179ae6c5
```

Observer interface refactor 전후 canonical JSON은 byte-identical했다.

## 5. Backend recommendation 결과

전체 1,215 row에서 advisory recommendation은 다음과 같다.

| Backend | rows | fraction |
|---|---:|---:|
| Seeded shared GMRES | 237 | 19.51% |
| Block GMRES | 437 | 35.97% |
| Independent Rayon GMRES | 541 | 44.53% |

High-risk 405 row는 모두 independent fallback으로 보냈다. High-risk를 제외한 810 row에서는:

| Backend | rows | fraction |
|---|---:|---:|
| Seeded shared GMRES | 237 | 29.26% |
| Block GMRES | 437 | 53.95% |
| Independent Rayon GMRES | 136 | 16.79% |

즉 실제 stage RHS에는 low-rank/shared-subspace opportunity가 충분히 존재하지만, universal block backend를 정당화할 정도로 일관되지는 않는다.

## 6. Directional rank와 threshold sensitivity

Transformed directional numerical-rank 분포:

```text
rank 1: 243
rank 2: 369
rank 3: 181
rank 4: 108
rank 5:  96
rank 6:  75
rank 7: 126
rank 8:  17
```

99% energy rank가 1인 비율은 53.50%였고, transformed directional stable-rank 중앙값은 약 1.0094였다. 그러나 hard rank threshold는 안정적이지 않았다.

| relative tolerance | 1e-8 대비 분류 변화 |
|---:|---:|
| \(10^{-6}\) | 22.63% |
| \(10^{-10}\) | 28.72% |
| \(10^{-12}\) | 38.85% |

따라서 하나의 rank cutoff로 즉시 backend를 dispatch하는 설계는 거부한다. Hysteresis, semantic drift, true residual, transactional fallback이 필수다.

## 7. Subspace drift

동일 `(phase, propagation_level)` batch만 비교했다.

| group | median max angle | p90 | maximum |
|---|---:|---:|---:|
| seeded recommendation | \(0^\circ\) | \(0^\circ\) | \(1.48\times10^{-6}\,{}^\circ\) |
| block recommendation | \(8.54\times10^{-7}\,{}^\circ\) | \(66.24^\circ\) | \(90^\circ\) |
| independent recommendation | \(47.87^\circ\) | \(90^\circ\) | \(90^\circ\) |

Homotopy depth별로는:

| q | median | p90 | maximum |
|---:|---:|---:|---:|
| 0 | \(2.13^\circ\) | \(90^\circ\) | \(90^\circ\) |
| 1 | \(5.36\times10^{-2}\,{}^\circ\) | \(90^\circ\) | \(90^\circ\) |
| 2 | \(7.54\times10^{-4}\,{}^\circ\) | \(90^\circ\) | \(90^\circ\) |

중앙값만 보면 higher-q가 안정적으로 보이지만 모든 q에 90도 tail이 있다. 따라서 q만으로 backend를 고정할 수 없다.

## 8. 실패 분석

두 실패는 모두 다음 severe case에서 발생했다.

```text
case       mv-n32-s1e6-m1e3-eta0.9
theta      1
q          2
predictor  Euler / AdamsBashforth2
failure    batched RHS produced NaN/Inf
rows kept  12 per path
```

이는 linear backend가 선택되기 전 homotopy path 자체가 발산한 것이다. Block/seeded/independent dispatch가 고칠 수 없으며 protected path fallback이 필요하다.

## 9. PHYS–MATH 결론

1. Exact rank는 invertible \(W^{-1}\) 아래 보존되지만 numerical rank는 scaling과 conditioning에 따라 변한다.
2. Raw rank는 units와 RHS magnitude에 민감하다. Unit-column directional rank를 추가했지만 component-wise WRMS/preconditioner scaling 문제는 남는다.
3. Reference transformed vectors는 dense \(W^{-1}\) 결과이며 production left-preconditioned Krylov residual과 동일하지 않다.
4. Manufactured stiffness/nonnormality label은 online observable이 아니다. 실제 dispatcher에는 residual growth, block-QR rank loss, principal-angle drift 등 실행 중 proxy가 필요하다.
5. \(N=1,2\) 문제는 구조적으로 rank가 제한되므로 large-PDE 일반화 근거로 쓰면 안 된다.

## 10. PHYS–MATH–CODE 결론

- Instrumentation은 162개의 observed/reference path 비교에서 결과와 work counter를 전혀 바꾸지 않았다.
- Failed-path work와 partial rows를 보존했다.
- SVD 비용은 offline telemetry라 solver work counter에 포함하지 않았다. Active dispatcher에서는 반드시 비용을 센다.
- 매 batch full \(N\times8\) SVD는 online policy로 과도하다. 다음 cycle에서는 \(8\times8\) Gram matrix 또는 rank-revealing small QR를 사용해야 한다.
- Backend recommendation은 advisory다. 이번 cycle에서 linear solve backend는 변경되지 않았다.

## 11. 문헌과의 관계

Multiple-RHS block GMRES에서는 residual들의 linear dependence와 block-Arnoldi의 near-rank deficiency를 deflation/rank-revealing QR로 다루는 선례가 있다. Flexible/deflated restart와 seed selection도 확립된 확장이다. 그러나 실제 stage traces에서 활성 block solve가 실패하는지 확인하기 전 BGMRES-DR를 구현하면 다시 comparator-driven drift가 된다.

Rosenbrock–Krylov는 shared linear backend와 동일하지 않다. 시간 discretization과 Krylov projection을 하나의 order-condition theory로 다루는 별도 integrator family이므로 이후 complete-integrator comparator로 유지한다.

## 12. 최종 판정

### PROMOTE

- read-only actual SABR/homotopy RHS telemetry;
- directional rank/energy/stable-rank diagnostics;
- semantic subspace-drift tracking;
- conservative advisory recommendation;
- observer abstraction;
- failure preservation;
- deterministic canonical artifact;
- behavior-equivalence gate.

### HOLD

- active dispatcher;
- hard single-threshold rank policy;
- block GMRES default;
- BGMRES-DR implementation;
- actual q=0/1/2 integrated speedup;
- online risk classifier;
- global order/stiff/phase performance claim.

## 13. 다음 DAG node

다음 branch는 `vectorized-jf-transactional-dispatch-v0.16`이다.

우선순위:

1. full-SVD와 비교 가능한 cheap \(8\times8\) Gram/QR estimator;
2. shadow dispatcher: 모든 backend를 시험하되 independent result만 commit;
3. two-batch hysteresis와 semantic drift gate;
4. seeded/block 실패 시 same-step independent retry;
5. true residual miss 시 즉시 fallback;
6. failed speculative work 유지;
7. actual q=0/1/2 + SABR same-error integrated test.

이번 cycle의 단일 결론은 다음이다.

> Actual stage RHS는 guarded dispatch를 정당화할 만큼 자주 low-rank이지만, threshold sensitivity와 drift tail 때문에 block/seeded backend를 즉시 default로 만들 근거는 없다.
