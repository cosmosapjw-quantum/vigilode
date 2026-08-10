# Vectorized/Jacobian-Free RODAS5P Path Controller v0.17

## 1. 연구 질문

이번 cycle은 이전 integrated screen에서 확인된 핵심 blocker,

\[
q=0,1,2\text{ low-depth homotopy가 거의 항상 protected RODAS5P로 fallback한다}
\]

를 linear-backend 문제와 분리해 조사한다. 목적은 BDF/Radau 또는 block GMRES를 더 최적화하는 것이 아니라, RODAS5P의 lower-triangular nonlinear stage system에 대해 어떤 bounded Newton-free propagation schedule이 original target defect를 실제로 줄이는지 판정하는 것이다.

이번 source tree는 durable v0.15 telemetry tag에서 출발했다. 이전 v0.16 integrated-dispatch 결과는 수치 artifact로만 존재하고 재현 가능한 source commit은 없으므로, source state로 상속하지 않았다.

## 2. 수학적 대상

RODAS5P의 frozen-step all-at-once stage equation을

\[
(D-C)K=g+hN(K)
\]

로 둔다. 여기서

\[
D=I_8\otimes W,\qquad W=M-h\gamma J_n,
\]

\[
C=hL\otimes J_n
\]

이고, \(N(K)\)는 frozen linear Taylor part를 제거한 nonlinear remainder다.

각 research round는

\[
(\lambda_r,\theta_r,q_r,\omega_r,c_r)
\]

로 표현한다.

- \(\lambda_r\): round endpoint
- \(\theta_r\): start operator에 포함하는 frozen coupling fraction
- \(q_r\): finite nilpotent inverse truncation depth
- \(\omega_r\): predictor damping
- \(c_r\): endpoint residual correction 횟수

\(\theta\)가 round 사이에서 바뀌는 경우는 하나의 smooth classical homotopy가 아니라 nonstationary preconditioned sweep으로 해석한다.

## 3. 구현 범위

### 구현

- `HomotopyRoundSpec`
- `HomotopyScheduleConfig`
- 최대 4 path rounds validation
- monotone \(\lambda\), final \(\lambda=1\), bounded \(q,\theta,\omega,c\)
- `run_scheduled_homotopy_path`
- round별 path/target residual telemetry
- partial failure에서 point와 attempted work 보존
- original RODAS nonlinear output certificate
- sequential direct oracle 비교
- deterministic CLI `homotopy-path-controller`

### 구현하지 않음

- runtime common-W dispatcher
- strict matrix-free common-W solve
- production online certificate
- adaptive trajectory integration
- BDF/Radau 변경
- global order 또는 wall-time promotion

## 4. 검증된 code contract

1. 고정 \((\theta,q,r,c)\) schedule은 기존 `run_fixed_homotopy_path`와 binary64 roundoff 내에서 같은 stage vector를 반환한다.
2. nonfinite RHS가 중간에 발생하면 report가 partial points와 work를 보존한다.
3. algebraic path success와 embedded timestep acceptance를 분리한다.
4. BDF/Radau source는 base tag 이후 변경되지 않았다.
5. canonical screen 두 번이 byte-identical하다.

## 5. Canonical corpus

총 6개 one-step problem을 사용했다.

1. affine noncommuting mass matrix
2. complex Dahlquist block \(-120\pm180i\)
3. moderate nonlinear/nonnormal manufactured vector
4. stiff nonlinear Prothero–Robinson
5. nonlinear noncommuting mass matrix
6. hostile nonlinear/nonnormal manufactured vector

총 22개 bounded schedule, 132개 candidate row를 생성했다.

```text
cases                  6
schedules             22
candidate rows       132
completed            132
numerical failures     0
false accepts          0
```

Algebraic acceptance는

\[
E_{\rm out}\le0.1
\]

로 판정하고, embedded timestep acceptance와 combined acceptance는 별도 field로 저장했다.

## 6. 핵심 negative results

### 6.1 Fixed low-q path marching은 defect를 거의 줄이지 못함

대표적인 3-round fixed paths의 non-hostile median target-residual ratio는:

\[
q=0:\;\approx1,
\qquad
q=1:\;\approx1,
\qquad
q=2:\;\approx0.99.
\]

즉 path point를 세 개 둔다고 causal information이 충분히 전달되는 것이 아니다.

### 6.2 theta ramp는 강하게 불안정

\[
\theta:1\rightarrow0.5\rightarrow0,
\qquad
q:2\rightarrow1\rightarrow1
\]

schedule은 non-hostile median target residual을 약 \(1.9\times10^2\)배 증가시켰고, 어떤 non-hostile case도 algebraic budget을 통과하지 못했다.

따라서 coupling을 round별로 단순히 약화하는 방식은 rejected hypothesis다.

### 6.3 q=7은 정확하지만 지나치게 비쌈

고정 \(q=7\)은 5/5 non-hostile case에서 통과했지만,

```text
common-W batches   32
RHS-vector work   256
```

를 사용했다. correctness oracle로 유지하되 fast method로 분류하지 않는다.

## 7. 핵심 positive result

### 7.1 Path marching보다 endpoint correction이 강함

가장 강한 bounded candidate는

```text
lambda rounds       1
start split theta   0
q                   2
endpoint corrections 2
```

인 `direct-decoupled-q2-c2`였다.

이 경로는:

- 5/5 non-hostile case algebraic accept
- false acceptance 0
- common-W batch applications 8
- common-W RHS-vector work 64
- non-hostile maximum oracle output WRMS \(3.35\times10^{-3}\)

를 기록했다.

고정 \(q=7\) 대비 common-W batch depth와 vector work가 모두 정확히 1/4이다.

그러나 8 batch는 sequential RODAS의 8-stage causal depth보다 작지 않다. 따라서 이것은 `depth-parity correctness survivor`이지 strict low-depth speedup survivor는 아니다.

### 7.2 q=1 endpoint lane은 더 얕지만 PR에서 실패

`direct-decoupled-q1-c2`는:

```text
common-W batches       6
common-W vectors      48
non-hostile accepts  4/5
```

였다. Affine, oscillatory complex block, moderate nonnormal, noncommuting mass에서는 통과했지만 stiff nonlinear Prothero–Robinson에서 output WRMS가 약 \(4.78\times10^2\)로 실패했다.

이 결과는 다음 adaptive controller hypothesis를 연다.

> q=1 two-correction fast attempt를 먼저 수행하고, contraction/output gate가 실패할 때 endpoint에서 q=2 correction만 추가한다.

아직 구현·검증하지 않았으므로 다음 branch hypothesis다.

## 8. Novelty guard

`direct-decoupled-q2-c2`는 classical path tracking의 성공이라기보다,

1. decoupled endpoint predictor
2. truncated causal propagation
3. 두 번의 endpoint residual correction

으로 구성된 preconditioned parallel iteration에 가깝다.

따라서 이를 새 homotopy solver라고 곧바로 주장하면 안 된다. Parallel-iterated RK와 parallel SDC가 이미 stage/node-parallel correction sweep을 연구해 왔으므로, 독립적 기여는 다음 중 하나를 보여야 한다.

- RODAS common-W/nilpotent structure에 특수화된 더 낮은 work
- J-free matrix-free implementation
- same-error wall speedup
- stiff/oscillatory fidelity 보존
- q-adaptive endpoint correction이 기존 sweep보다 적은 critical depth를 달성

## 9. 현재 판정

### PROMOTE

- validated nonstationary schedule model
- schedule/reference engine
- partial-failure work preservation
- algebraic vs timestep acceptance separation
- deterministic path-controller screen
- `direct-decoupled-q2-c2` as bounded correctness survivor
- `direct-decoupled-q1-c2` as speculative low-depth lane

### HOLD

- production solver
- strict Jacobian-free claim
- global fifth order
- L-stability/stiff decay
- same-error speedup
- q=1→q=2 adaptive escalation
- online certificate
- hostile nonnormal coverage

## 10. 결론

이번 cycle은 fixed nonstationary path marching보다 **one-round decoupled endpoint jump + bounded causal corrections**가 훨씬 강하다는 것을 확인했다. 따라서 다음 연구는 path node를 더 늘리는 것이 아니라,

\[
q=1\text{ fast attempt}
\rightarrow
q=2\text{ endpoint escalation}
\rightarrow
\text{protected sequential fallback}
\]

을 strict matrix-free common-W backend와 결합해 adaptive trajectory 수준에서 검증해야 한다.
