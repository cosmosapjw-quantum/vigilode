# G1 Transactional q1→q2 Generic Solver Research Report

## 1. 질문과 보호 경계

G1의 질문은 다음이었다.

> RODAS5P의 accepted-step economy를 유지하면서 8-stage sequential critical path를 q1 causal propagation과 retained q2 escalation으로 줄이고, explicit Jacobian과 fast-path Newton 없이 protected sequential matrix-free RODAS5P보다 빠르게 만들 수 있는가?

수치 문제는 regular ODE

\[
M\dot y=f(t,y)
\]

이며 이번 범위에서 \(M\)은 identity 또는 상수·비특이 mass matrix다. Shifted operator의 부호와 단위는 protected RODAS convention을 그대로 유지한다.

\[
Wv=Mv-h\gamma Jv.
\]

필수 gate는:

\[
N_J^{\rm explicit}=0,\qquad N_{\rm fact}=0,\qquad N_{\rm Newton}^{\rm fast}=0,
\]

\[
f_{\rm fast}\ge0.90,\qquad f_{\rm fallback}\le0.10,
\]

\[
D_{\rm median}\le8\;(\text{prefer }<8),\qquad D_{95}\le6\;(\text{target}),
\]

그리고 동일 external global error에서

\[
S_{\rm wall}=T_{\rm protected}/T_{\rm candidate}\ge1.15
\]

였다.

## 2. 구현한 candidate

### 2.1 q1 fast attempt

- decoupled stage start 1 batch;
- endpoint tangent 1 batch;
- q1 truncated inverse endpoint correction 2회, 각 2 batch;
- total common-\(W\) critical depth 6.

### 2.2 retained q2 escalation

q1의 두 번째 correction에서 이미 계산한 마지막 causal term을 보존하고, 다음 \(Q^2D^{-1}r\) term만 한 batch로 추가한다. q2 state를 처음부터 다시 만들지 않는다. 그 뒤 남은 defect를 평가하는 matrix-free diagnostic batch를 한 번 수행한다.

따라서 accepted q2 depth는 8이다. 이 경로는 정확히 말하면 fresh q2 solve가 아니라 **retained-tail nonstationary escalation**이다.

### 2.3 protected fallback

q2 gate가 실패하거나 nonfinite/linear failure가 발생하면 같은 timestep에서 sequential matrix-free RODAS5P 8 stages를 수행한다. Speculative 8 batches를 삭제하지 않으므로 fallback critical depth는 16이다.

### 2.4 operational certificate

Gate는 original target residual, correction contraction, post-q2 remaining-defect diagnostic, embedded RODAS error를 사용한다. 이 gate는 finite research certificate이며 arbitrary nonlinear/nonnormal system의 rigorously proven bound가 아니다. Dense offline Jacobian certificate를 fast path에 숨기지 않았다.

## 3. one-step eight-family gate

검사 family:

1. complex Dahlquist;
2. oscillatory Prothero–Robinson;
3. stiff van der Pol;
4. Robertson;
5. nonlinear nonnormal block;
6. diffusion–reaction;
7. advection–diffusion–reaction;
8. constant noncommuting mass.

결과:

```text
cases                    8
completed                8
q1 fast                  3
q2 escalated             4
sequential fallback      1
fast fraction        0.875
fallback fraction    0.125
false accepts             0
median depth              8
P95 depth              13.2
explicit-J builds         0
factorizations            0
fast-path Newton          0
```

구조적 정확성은 통과했지만 fast/fallback gate를 각각 근소하게 실패했다.

## 4. adaptive generic screen

8 generic analytic families, 3 tolerances, 5 candidates를 사용했다. 후보는:

- protected sequential JF RODAS5P;
- transactional q1→q2, one-thread common-\(W\);
- transactional q1→q2, four-thread common-\(W\);
- frozen adaptive BDF2;
- frozen adaptive Radau IIA3.

5개의 outer-T1 campaign과 3개의 outer-T4 campaign은 같은 scientific checksum을 냈다.

```text
scientific checksum
f772819f2bb151547115769d06bc766358e589a51cdd528338b8865be22e3e5a

runs per campaign  120
T1 campaigns         5
T4 campaigns         3
```

Transactional accepted-step composition은 두 internal-thread variant에서 동일했다.

```text
accepted steps          201
q1 fast                   0
q2 escalated             100
protected fallback       101
strict sub-8 fraction      0
nonfallback fraction  0.4975
fallback fraction     0.5025
median depth              16
P95 depth                 16
```

문제별로는 complex Dahlquist, scalar linear, noncommuting mass가 q2 depth-parity lane을 잘 사용했다. 반면 PR(-20), advection–diffusion, large nonnormal block은 전부 fallback했고, stiff oscillatory PR에서는 66 accepted steps 중 q2 accept가 1회뿐이었다.

## 5. 비용과 same-error 결과

동일 problem/rtol에서 one-thread candidate의 중앙값:

| Quantity | candidate / protected |
|---|---:|
| wall time | 4.708 |
| RHS evaluations | 5.125 |
| JVP vectors | 8.534 |
| accepted steps | 1.000 |
| external error | 1.000 |

4-thread internal stage solve는 작은 상태공간에서 더 불리했다.

| Quantity | candidate / protected |
|---|---:|
| wall time | 7.776 |
| RHS evaluations | 5.125 |
| JVP vectors | 8.534 |

동일 external-error target matching:

| Candidate | coverage | median speedup | median step ratio |
|---|---:|---:|---:|
| transactional t1 | 23/24 | 0.226 | 1.0 |
| transactional t4 | 23/24 | 0.123 | 1.0 |
| frozen BDF2 | 1/24 | 0.425 | 38.5 |
| frozen Radau IIA3 | 18/24 | 2.371 | 2.0 |

BDF2의 수치는 한 target뿐이므로 generic rank가 아니다. Radau IIA3는 covered high-accuracy subset에서 강하지만 explicit Jacobian/full Newton comparator라서 G1의 structural requirements를 만족하지 않는다.

## 6. 해석

Iterated Runge–Kutta 문헌은 parallel iteration으로 stage count 대신 iteration count를 critical depth로 만들 수 있음을 보여준다. 하지만 iteration splitting과 coefficient가 early convergence/stability를 결정한다. G1 결과도 같은 결론을 준다: q1은 adaptive step에서 terminal lane으로 한 번도 선택되지 않았고, q2는 depth parity에 머물렀다.

Triangularly implicit iteration 연구가 fully decoupled/block-diagonal 초기 iteration의 약점을 지적하는 것처럼, G1의 decoupled start와 제한된 causal hop은 stiff nonlinear·nonnormal cases에 충분한 초기 triangular information을 전달하지 못했다.

## 7. 최종 판정

### PROMOTE — infrastructure

- strict matrix-free q1→q2 transaction state machine;
- retained-endpoint escalation;
- original-system protected fallback;
- failed/speculative work preservation;
- generic one-step and adaptive same-error gates;
- internal one-/four-thread variants와 deterministic reporting.

### HOLD — numerical method

- q1→q2 as generic accelerated solver;
- sub-8 critical-path claim;
- same-error acceleration claim;
- standalone fast-path fifth-order claim;
- certificate weakening for performance.

Kill rule이 fast fraction, fallback, depth, same-error speed 네 축에서 동시에 발생했으므로 G1을 더 많은 \(\lambda\)/q tuning으로 연장하지 않는다. 다음 core node는 parallel fifth-order exponential Rosenbrock/EPIRK다.
