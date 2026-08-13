# G2 Parallel Exponential Rosenbrock Foundation Report

Date: 2026-08-13  
Base: `generic-q1-q2-transactional-v1.1.0-alpha1`  
Feature commit: `fd9f3f64fa87e5a6463f1b846381be191860fb61`

## 1. 연구 질문과 범위

G1의 transactional q1→q2 RODAS homotopy는 explicit Jacobian과 fast-path Newton을 제거했지만 adaptive generic screen에서 q1 fast acceptance가 0이고 protected fallback이 약 50%였으며, same-error wall speedup도 1보다 훨씬 작았다. G2는 같은 목적함수를 다른 수학 구조로 공격한다.

목표는:

\[
N_J^{\rm explicit}=0,\qquad N_{\rm Newton}^{\rm primary}=0,
\]

을 유지하면서 5차 accepted-step economy와 짧은 dependency depth를 얻는 것이다.

이번 checkpoint는 구현 범위를 다음으로 제한했다.

- autonomous regular ODE;
- identity mass matrix;
- user-supplied matrix-free JVP;
- binary64;
- small dense projected-space oracle;
- correctness/order/structure gate.

Adaptive time stepping, nonautonomous augmentation, constant mass support, KIOPS production φ-action, same-error performance ranking은 범위 밖이다.

## 2. 구현 구조

### 2.1 Projected dense oracle

Higham scaling-and-squaring Padé `(13,13)` 공식을 small projected matrix에 구현했다. `φ_k(τA)v`는 augmented matrix exponential의 마지막 column을 통해 계산한다. Large-state path가 아니라 projected-space correctness oracle이다.

### 2.2 Matrix-free φ-action reference

Arnoldi basis에서

\[
\varphi_k(\tau A)v\approx V_m\varphi_k(\tau H_m)(\beta e_1)
\]

를 계산한다. Operator application은 JVP callback만 사용한다. Work ledger는:

- JVP vector 수;
- orthogonalization inner products/vector updates;
- projected exponentials;
- φ-action 호출 수

를 분리한다.

현재 error estimate는 두 nested Krylov dimension의 action 차이다. 이는 research diagnostic이지 KIOPS residual/substepping certificate가 아니다.

### 2.3 Integrators

구현한 방법은:

- EXPRB2;
- embedded EXPRB43;
- `pexprb54s4`, order 5 / embedded order 4.

`pexprb54s4` coefficient는 Luan–Ostermann (2016)의 exact rational values를 잠갔다.

\[
c_2=\frac14,\qquad c_3=\frac12,\qquad c_4=\frac9{10},
\]

\[
a_{32}=4\varphi_3(c_3z),\qquad
a_{42}=\frac{2916}{125}\varphi_3(c_4z),\qquad a_{43}=0.
\]

Main/embedded endpoint coefficients도 machine-readable typed record로 저장한다.

### 2.4 실제 parallel dependency graph

Published logical graph를 metadata로만 기록하지 않았다.

1. U2 path와 full-step base `φ_1(hJ)F_n` action을 local Rayon에서 동시 실행;
2. U3와 U4를 동시 실행;
3. main endpoint와 embedded endpoint correction을 동시 실행.

Atomic-concurrency test가 실제 JVP overlap `>=2`를 검출한다. 따라서 dependency depth 3은 실제 code-path property다.

## 3. 수치 결과

### 3.1 Published coefficient / selected stiff-order identities

6개 scalar `z`에서 selected order-condition residual을 계산했다.

\[
\max |r_{\rm condition}|=2.220446049250313\times10^{-16}.
\]

이는 published coefficient transcription과 선택한 identities를 검증한다. 전체 abstract stiff-order theorem을 다시 증명한 것은 아니다.

### 3.2 φ-action oracle

Nonnormal upper-chain과 damped oscillatory block, `k=1,...,5`에서:

\[
\max E_{\varphi,\rm rel}
=6.5693111217791135\times10^{-16}.
\]

Independent 100-digit mpmath reconstruction은 120개 vector comparison을 수행했다.

\[
\max |\varphi_{\rm dense,Rust}-\varphi_{\rm mp}|
=2.9526372625242674\times10^{-16},
\]

\[
\max |\varphi_{\rm Krylov,Rust}-\varphi_{\rm mp}|
=3.0392422424123546\times10^{-16}.
\]

### 3.3 Nonlinear observed order

문제:

\[
y'=y^2,\qquad y(0)=1,\qquad T=0.25,
\]

exact solution:

\[
y(t)=\frac1{1-t}.
\]

Minimum observed order:

| Method | Observed order |
|---|---:|
| EXPRB2 | 1.9559710426 |
| EXPRB43 | 3.9248606117 |
| pexprb54s4 | 4.9287704282 |

### 3.4 Stiff and oscillatory linear limits

For `y'=zy`, `z=-1,-20,-100,-500`, all three methods reproduce `exp(z)` within:

\[
1.3988810110276972\times10^{-14}.
\]

Damped complex block에서는 amplitude error `<=2.78e-16`, phase error `<=1.54e-15 rad`였다.

이 결과는 nonlinear remainder가 0인 linear problem에서 φ-action 정확도를 검증한다. Nonlinear stiff/oscillatory holdout을 대신하지 않는다.

## 4. 핵심 negative result: depth와 work는 다르다

Coarsest nonlinear order row의 step당 비용:

| Method | Logical depth | separate φ-actions | JVP vectors |
|---|---:|---:|---:|
| EXPRB2 | 1 | 1 | 1 |
| EXPRB43 | 3 | 9 | 11 |
| pexprb54s4 | 3 | 15 | 18 |

따라서 `pexprb54s4`가 3-level이라고 해서 현재 구현이 RODAS5P보다 빠르다는 뜻은 아니다. 여러 `φ_k` linear combination을 각각 별도 Arnoldi로 계산한 것이 현재 가장 큰 blocker다.

다음 단계는 method coefficient tuning이 아니라 **fused φ linear-combination engine**이다.

## 5. Claim boundary

### Supported

- published pexprb54s4 coefficients의 exact transcription;
- selected stiff-order identities;
- nonlinear observed orders 2/4/5;
- matrix-free JVP-only primary path;
- explicit Jacobian build / direct factorization / Newton iteration 0;
- 실제 3-level parallel code path;
- deterministic canonical artifact와 high-precision parity.

### Not supported

- KIOPS가 구현됐다는 주장;
- adaptive time integration;
- nonautonomous problems;
- constant/nontrivial mass matrices;
- production φ residual certificate;
- same-error speedup vs protected JF RODAS5P;
- BDF2/Radau IIA3보다 빠르다는 주장;
- physical-client validation.

## 6. 판정

```text
PROMOTE
  dense projected φ oracle
  matrix-free full-Arnoldi φ reference
  EXPRB2 / EXPRB43 correctness foundation
  pexprb54s4 coefficient/order/parallel foundation
  independent high-precision receipt

HOLD
  accelerated generic solver
  adaptive integrator
  KIOPS label
  same-error performance claim
  mass/nonautonomous/DAE claims
```

G2 foundation은 통과했다. 다음 node는 fused/adaptive φ engine과 adaptive exponential integrator다.
