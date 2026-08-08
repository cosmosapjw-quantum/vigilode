# RODAS5P homotopy order-policy + Rayon 수치실험 보고서

Date: 2026-08-08 Asia/Seoul
Branch: `homotopy-order-policy-v0.6`
Base: `homotopy-numerical-v0.5.0-alpha1` (`1e322cfbf5802dbabc6c94afce0e072a667f2081`)

## 1. 요약 판정

이번 cycle은 cycle 1의 동일한 8,208개 one-step homotopy candidate를 재사용해 다음 네
acceptance-policy family를 calibration/holdout으로 비교했다.

\[
E_{\rm out}\le\epsilon_0,
\qquad
E_{\rm out}\le\eta E_{\rm emb},
\qquad
E_{\rm out}\le\epsilon_{\rm ref}\left|\frac h{h_{\rm ref}}\right|^6,
\]

\[
E_{\rm out}\le
\min\left[
\eta E_{\rm emb},
\epsilon_{\rm ref}\left|\frac h{h_{\rm ref}}\right|^6
\right].
\]

결론은 다음과 같다.

1. calibration과 holdout에서 false acceptance가 0인 정책은 세 개의 absolute budget뿐이다.
2. embedded-relative, step-power, mixed winner는 holdout에서 각각 85, 59, 53개의 false
   acceptance를 냈다.
3. order-aware policy가 너무 느슨해서가 아니라, 현재 first-order output certificate
   \(\mathcal J_R^{-1}\mathcal R\)가 강한 비선형·비정규 regime에서 실제 oracle defect를
   수십–수백 배 과소평가할 수 있기 때문이다.
4. absolute budget은 low-depth acceptance를 유지하지만 \(\theta=0,q\le2\) 경로의 fifth
   order를 복구하지 못한다.
5. fifth order 또는 stiff regression을 보존하는 \(\theta=1,q=2,7\) 경로는 low-depth가
   아니며 common-\(W\) vector work가 매우 크다.
6. 따라서 homotopy candidate 중 모든 gate를 통과한 것은 0개다.
7. Rayon 실행은 1-thread와 4-thread에서 과학적 결과가 정확히 동일했지만, 전체 CLI
   speedup은 4 threads에서 1.077배에 그쳤다. 현재 runtime은 serial source-screen 생성과
   108 MiB JSON materialization에 지배된다.

최종 판정은 다음이다.

- **PROMOTE:** order-policy API, deterministic calibration/holdout replay, local Rayon execution,
  trajectory gate와 negative-result evidence.
- **HOLD:** 현 policy를 production acceptance rule로 사용, low-depth homotopy fast path,
  wall-time speedup 주장.
- **NEXT:** nonlinear a posteriori certificate와 source candidate generation parallelization.

## 2. 보호된 구조

다음 항목은 바꾸지 않았다.

- 공식 RODAS5P tableau와 protected sequential oracle;
- cycle-1 fixed homotopy path candidates;
- exact nonlinear target Jacobian을 사용하는 reference certificate;
- transactional sequential fallback;
- Rust 1.94.1, `faer 0.24.4`, 같은 binary와 같은 numerical kernels.

이번 cycle은 path parameter를 재조율하지 않고 acceptance policy만 replay했다. 따라서
policy effect와 path-generation effect를 분리할 수 있다.

## 3. 실험 구성

Canonical source screen:

```text
cases             57
controls         114
candidates      8208
source accepts  3510
source fallbacks 4698
source failures     0
```

Policy grid는 19개다.

- absolute: 3
- embedded-relative: 4
- step-power: 3
- mixed: 9

각 policy를 8,208 candidates에 적용해 총 155,952 replay row를 생성했다. Case ID의 고정
FNV-1a parity로 calibration/holdout을 분리했으며, thread count가 달라도 split과 모든 과학적
row가 동일하다.

Trajectory screen은 다음 세 문제에 대해 수행했다.

- manufactured vector order screen;
- Prothero–Robinson stiff/nonlinear screen;
- noncommuting-mass nonnormal screen.

비교 방법은 protected sequential RODAS5P와 다음 fixed paths다.

```text
theta0-q0-r2-ab2-c0
theta0-q1-r2-ab2-c0
theta0-q2-r2-ab2-c0
theta1-q2-r2-ab2-c1
theta1-q7-r2-ab2-c1
```

## 4. Calibration/holdout 결과

### 4.1 Family winner

| family | calibration winner | holdout accepts | low-depth accepts | holdout false accepts |
|---|---|---:|---:|---:|
| absolute | `absolute-e1.000000e-1` | 1645 | 196 | 0 |
| embedded-relative | `embedded-relative-eta2.000000e-1` | 710 | 4 | 85 |
| step-power | `step-power-e1.000000e-2-href4.000000e-2-p6` | 425 | 0 | 59 |
| mixed | `mixed-eta1.000000e-1-e1.000000e-2-href4.000000e-2-p6` | 327 | 0 | 53 |

세 absolute policies만 calibration과 holdout 모두에서 false accept 0을 유지했다.

```text
absolute epsilon=0.10: holdout accepts 1645, low-depth 196
absolute epsilon=0.03: holdout accepts 1530, low-depth 167
absolute epsilon=0.01: holdout accepts 1418, low-depth 129
```

### 4.2 Order-aware family의 false acceptance

Winner의 holdout false accept는 주로 manufactured-mass와 Prothero–Robinson family에
집중됐다.

| family | false accepts | manufactured-mass | Prothero–Robinson | 기타 | median oracle/certificate | max |
|---|---:|---:|---:|---:|---:|---:|
| embedded-relative | 85 | 66 | 10 | 9 manufactured-vector | 32.06 | 502.50 |
| step-power | 59 | 45 | 13 | 1 scalar-linear | 64.55 | 502.50 |
| mixed | 53 | 43 | 9 | 1 scalar-linear | 66.28 | 502.50 |

이는 policy scaling 자체의 문제만은 아니다. Acceptance에 사용한

\[
E_{\rm lin}
=
\left\|
(b^T\otimes I)\mathcal J_R^{-1}\mathcal R(K)
\right\|_{\rm WRMS}
\]

은 local first-order correction이지 rigorous nonlinear upper bound가 아니다. 실제 root와의
차이를 \(E_{\rm oracle}\)라 하면 일부 row에서

\[
\frac{E_{\rm oracle}}{E_{\rm lin}}>500
\]

까지 나타났다. 따라서 더 작은 \(h^6\) budget을 적용해도 certificate 자체가 과소평가하면
false acceptance를 막지 못한다.

### 4.3 Path-specific gate correction

Independent review에서 최초 trajectory gate가 winner policy 전체의 holdout false-accept
수를 모든 path configuration에 그대로 붙이고 있음을 발견했다. 이는 특정
\((\theta,q,r,\text{predictor},c)\) 경로의 안전성을 다른 경로의 failure로 오염시키는
과도하게 보수적인 attribution이었다.

TDD regression을 추가하고 gate를 정확히 같은 path configuration의 holdout rows로
제한했다. 수정 후 winner별 method-specific false accept는 다음과 같다.

| family | theta0 q0 | theta0 q1 | theta0 q2 | theta1 q2 c1 | theta1 q7 c1 |
|---|---:|---:|---:|---:|---:|
| absolute | 0 | 0 | 0 | 0 | 0 |
| embedded-relative | 0 | 0 | 0 | 2 | 2 |
| step-power | 0 | 0 | 0 | 2 | 2 |
| mixed | 0 | 0 | 0 | 2 | 2 |

이 수정은 최종 scientific disposition을 바꾸지 않는다. \(\theta=0,q\le2\) 경로는
method-specific false accept는 0이지만 global-order 또는 stiff gate를 통과하지 못하고,
\(\theta=1,q=2,7\)은 fifth-order 쪽은 강하지만 low-depth가 아니며 일부 order-aware
policy에서 false accept가 남는다. Homotopy promotion count는 여전히 0이다.

## 5. Global order와 stiff screen

### 5.1 Absolute winner

Absolute budget은 one-step false accept를 막고 low-depth 경로를 일부 유지하지만,
manufactured-vector trajectory에서 다음 현상이 재현됐다.

- \(\theta=0,q=0\): finest observed order 약 1.45
- \(\theta=0,q=1\): finest observed order 약 4.20
- \(\theta=0,q=2\): 한 구간에서 약 4.98 뒤 finest에서 약 0.70
- \(\theta=1,q=2,c=1\): roundoff 이전 4.96, 4.85지만 low-depth 아님
- \(\theta=1,q=7,c=1\): protected fifth-order 추세 재현, low-depth 아님

따라서 absolute policy에 대해

```text
fifth order + stiff regression + low depth + zero false accepts
```

을 동시에 만족한 homotopy gate는 0개다.

### 5.2 Order-aware winners

Embedded-relative, step-power, mixed policy는 \(\theta=0,q\le2\) trajectory를 대부분
fallback시켜 protected order를 겉으로 복구했다. 그러나 이는 all-fast fifth-order recovery가
아니며, holdout false accept gate도 실패한다.

\(\theta=1,q=2,7\)은 all-fast가 가능하지만 batch depth와 vector work가 커 low-depth gate를
통과하지 못했다. Stiff/noncommuting-mass 문제에서는 사실상 \(q=7\)만 sequential oracle에
가까운 regression을 보였다.

## 6. Rayon과 SIMD 결과

### 6.1 결정론성

Local `rayon::ThreadPool`을 사용했으며 global pool을 변경하지 않았다. Indexed parallel
iterator 결과를 stable sort한 후 serialize한다.

- 1-thread와 4-thread canonical scientific fields: exact equality
- 1-thread 반복 실행: byte deterministic
- thread count 차이는 execution metadata에만 남음

### 6.2 End-to-end scaling

Canonical screen을 `/dev/null` output으로 warm-up 후 5회씩, 각 repetition에서
1/2/4-thread 실행 순서를 고정 seed로 섞어 측정했다.

| threads | median seconds | speedup | efficiency |
|---:|---:|---:|---:|
| 1 | 2.8157 | 1.000 | 1.000 |
| 2 | 2.8068 | 1.003 | 0.502 |
| 4 | 2.6176 | 1.076 | 0.269 |

4-thread 이득은 약 7.6%다. Policy replay와 trajectory jobs는 Rayon으로 병렬화됐지만,
cycle-1 source candidate generation과 108 MiB JSON object construction/serialization은 직렬
구간으로 남아 있다. 따라서 이 결과는 Rayon이 작동하지 않았다는 뜻이 아니라, 현재 CLI의
Amdahl fraction이 작다는 뜻이다.

Manual `std::simd`는 사용하지 않았다. Stable Rust 1.94.1에서 portable SIMD API는 여전히
nightly feature이며, 현재 `faer`/`pulp` kernel과 compiler auto-vectorization을 유지했다.
Release binary disassembly에는 AVX-512 `zmm` instructions가 포함돼 있음을 확인했다. 다만
그 존재만으로 해당 experiment의 hotspot이 SIMD에 의해 가속됐다고 주장하지 않는다.

## 7. Negative results

1. \(h^6\)-scaled budget은 그 자체로 fifth-order safety를 보장하지 않는다.
2. Embedded estimator에 비례한 budget도 nonlinear certificate가 lower bound처럼 작동하면
   안전하지 않다.
3. Mixed policy는 두 unsafe estimator의 minimum을 취해도 false acceptance를 제거하지 못했다.
4. Low-depth와 all-fast fifth-order/stiff robustness가 현재 candidate set에서 동시에 나타나지
   않았다.
5. Rayon을 policy replay에만 붙여서는 end-to-end speedup이 작다.
6. Manual SIMD를 추가하기 전에 실제 hotspot을 source generation, certificate solve,
   serialization으로 분리해야 한다.

## 8. 다음 연구 우선순위

### N1 — rigorous 또는 safeguarded nonlinear certificate

다음 branch의 최우선 과제다. Exact oracle을 유지하면서 다음 candidate를 비교한다.

1. second correction을 이용한 nonlinear remainder indicator;
2. local Lipschitz/Kantorovich 또는 radii-polynomial형 a posteriori bound;
3. \(\|\mathcal J_R^{-1}\|\), residual, Jacobian drift를 결합한 safety factor;
4. first/second corrected output의 ratio가 불안정하면 무조건 fallback;
5. nonnormal amplification proxy를 포함한 output-specific bound.

승격 조건은 calibration뿐 아니라 holdout false accept 0이다.

### N2 — source candidate generation의 Rayon 병렬화

현재 가장 큰 serial compute 구간인 57 case × 144 candidate generation을 case 단위로
parallelize한다. 각 case 안의 trajectory/step은 순차로 유지하고, 결과는 case/config key로
정렬해 결정론을 보존한다.

### N3 — compute와 serialization 분리

108 MiB raw JSON은 evidence에는 유용하지만 thread benchmark를 지배한다. 다음 cycle에서는

- full raw artifact;
- compact scientific summary;
- compute-only benchmark checksum

을 분리한다. Raw artifact는 zstd archive로 보존한다.

### N4 — SIMD는 profiling 이후

`faer`/`pulp` vector kernels를 우선 유지한다. Hand-written stage-batch kernel이 hotspot으로
확인될 때만 stable `pulp` runtime dispatch 또는 별도 stable SIMD crate를 검토한다. Nightly
`std::simd`는 reproducible stable-toolchain 경계를 깨므로 사용하지 않는다.

### N5 — adaptive path schedule은 certificate 이후

Certificate가 안전해진 뒤에만 \(\theta,\lambda,q\) adaptive schedule과 Anderson을 연다.
현재 unsafe certificate 위에서 path를 최적화하면 false-accept 영역만 넓힐 위험이 있다.

## 9. 최종 scientific disposition

### PROMOTE

- validated `OutputBudgetPolicy` API;
- deterministic calibration/holdout replay;
- local Rayon backend와 thread-count reproducibility;
- global-order/stiff/nonnormal trajectory gates;
- raw negative-result evidence.

### HOLD

- embedded-relative, step-power, mixed policy의 acceptance 사용;
- low-depth homotopy fast path;
- current homotopy candidate의 production use;
- wall-time speedup 주장;
- manual portable SIMD implementation.

### RESEARCH NEXT

\[
\boxed{
\text{nonlinear certified output bound}
+
\text{parallel source generation}
+
\text{compute/serialization separation}
}
\]

이 세 항목이 닫힌 뒤에만 adaptive homotopy schedule과 기존 GCRO-DR tuning을 공동 비교한다.

## 10. 최종 validation과 실패 보존

최종 pre-commit 전체 gate에서 첫 실행은 Rust test와 scientific checks를 모두 통과한 뒤
source-audit pipeline에서 exit `141`로 종료됐다. 원인은 `set -o pipefail` 아래의
`objdump | grep -q`가 positive match 후 `objdump`에 SIGPIPE를 준 것과, allocation-audit용
의도된 `unsafe`까지 광범위하게 검색한 validation scope였다.

수치 코드나 solver를 수정하지 않고 validation harness만 교정했다. 수정된 gate는 다음을
처음부터 다시 실행해 통과했다.

- format check;
- strict release Clippy;
- release workspace와 doc tests;
- release build;
- CLI validation;
- one/four-thread scientific identity;
- analysis JSON contract;
- raw archive SHA-256/listing;
- production source safe-code audit;
- dynamic dependency와 vector-instruction audit;
- Git diff/status/fsck.

최초 실패와 수정 후 성공 로그를 모두 receipts에 보존했다.
