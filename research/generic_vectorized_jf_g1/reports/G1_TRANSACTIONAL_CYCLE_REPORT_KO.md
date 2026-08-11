# G1 — Transactional q1→q2 Generic Solver Cycle Report

## 1. 연구 질문과 보호 경계

목표는 protected sequential matrix-free RODAS5P의 accepted-step economy를 유지하면서,
8-stage sequential critical path를 다음 transactional structure로 줄일 수 있는지 검사하는 것이다.

\[
q=1,\ c=2\;\text{fast attempt}
\longrightarrow
q=2\;\text{retained-endpoint escalation}
\longrightarrow
\text{protected sequential JF fallback}.
\]

Fast path의 hard constraints는

\[
N_J^{\rm explicit}=0,\qquad
N_{\rm factor}=0,\qquad
N_{\rm Newton}^{\rm fast}=0
\]

이다. BDF2와 Radau IIA3 source는 G0 byte-freeze test로 고정했고 comparator로만 사용했다.
물리 client는 실행하지 않았다.

## 2. 구현한 알고리즘

### 2.1 Strict matrix-free shifted operator

모든 fast-path common-W action은

\[
Wv=Mv-h\gamma Jv
\]

만 사용한다. 명시적 Jacobian이나 shifted matrix를 만들지 않는다. JVP가 없는 문제는
explicit-J path로 우회하지 않고 fail-closed한다.

### 2.2 q1 fast attempt

1. decoupled \(\lambda=0\) start;
2. target endpoint tangent;
3. q1 truncated causal inverse를 이용한 endpoint correction 2회;
4. original target residual, output-correction contraction, embedded error를 이용한 operational gate.

q1 accept의 common-W batch depth는 6이다.

### 2.3 Retained q2 escalation

q1의 두 번째 correction이 이미 계산한 마지막 causal term을 보존한다. q2는 start/path를
다시 계산하지 않고 \(Q^2D^{-1}r\) 항만 한 batch로 추가한다. 이후 post-q2 residual에 대한
q0 diagnostic batch를 한 번 수행한다. 따라서 q2 accept의 critical depth는 8이다.

### 2.4 Protected fallback

q2 gate 실패, nonfinite, linear solve failure에서는 동일 timestep의 protected sequential
matrix-free RODAS5P를 실행한다. q1/q2 speculative work는 장부에서 삭제하지 않는다.
Fallback critical depth는 16이다.

## 3. One-step generic gate

8개 family를 사용했다.

1. complex Dahlquist;
2. oscillatory Prothero–Robinson;
3. stiff van der Pol;
4. Robertson kinetics;
5. nonlinear nonnormal block;
6. diffusion–reaction;
7. advection–diffusion–reaction;
8. constant noncommuting mass matrix.

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
direct factorizations     0
fast-path Newton          0
```

One-step screen만으로도 fast fraction 0.90과 fallback 0.10 gate를 근소하게 실패했고,
median depth는 sequential baseline과 같은 8이었다.

## 4. Adaptive global-error screen

최종 corpus는 8개 analytic/generic problems, 5개 candidates, 3개 tolerance로 구성했다.
각 canonical campaign에는 120개 run이 있고, 5개의 T1 campaign과 3개의 outer T4 campaign의
scientific checksum은 모두 다음으로 같았다.

```text
f772819f2bb151547115769d06bc766358e589a51cdd528338b8865be22e3e5a
```

Transactional t1/t4 candidate의 science는 동일했다.

```text
accepted steps                 201
q1 fast                          0
q2 escalated                   100
protected fallback             101
fast fraction              0.4975
fallback fraction          0.5025
median critical depth           16
P95 critical depth              16
median common-W batches          8
explicit-J builds                0
direct factorizations            0
fast-path Newton                 0
```

문제별로 complex Dahlquist, noncommuting mass, scalar linear에서는 q2가 모든 accepted step을
처리했지만, PR, n=32 advection–diffusion, n=32 nonlinear nonnormal block에서는 전부 fallback했다.
Oscillatory PR의 fast fraction은 약 1.52%였다. q1은 adaptive trajectory에서 한 번도 최종 accept
lane이 되지 못했다.

## 5. Step economy와 비용

Accepted-step count의 median ratio는 정확히 1이었다. 즉 원래 목표 중 step economy는 보존했다.
그러나 동일 problem/tolerance에서 candidate/protected-JF-RODAS median ratio는 다음과 같다.

| Quantity | q1→q2 t1 | q1→q2 t4 |
|---|---:|---:|
| wall time | 4.708 | 7.776 |
| RHS evaluations | 5.125 | 5.125 |
| JVP vectors | 8.534 | 8.534 |
| accepted steps | 1.000 | 1.000 |

T4 내부 stage parallelism은 현재 \(n\le32\) corpus에서 Rayon overhead 때문에 오히려 더 느렸다.

동일 external error target matching에서는:

| Candidate | Coverage | Median speedup vs protected JF RODAS | Median step ratio |
|---|---:|---:|---:|
| q1→q2 t1 | 23/24 | 0.226 | 1.0 |
| q1→q2 t4 | 23/24 | 0.123 | 1.0 |
| frozen BDF2 | 1/24 | 0.425 | 38.5 |
| frozen Radau IIA3 | 18/24 | 2.371 | 2.0 |

BDF2의 4.2% coverage와 Radau의 75% coverage를 숨기지 않는다. Radau의 강한 결과는 작은 dense
corpus에서의 explicit-J/full-Newton comparator 결과이고 본 프로젝트의 JF/Newton-free 구조 조건을
만족하지 않는다.

## 6. Order와 certificate 해석

Mild PR의 roundoff 전 bounded test에서 fallback-capable path는 관측 order 약 5.90을 보였다.
하지만 adaptive path는 q2와 protected fallback의 혼합이므로 이를 all-fast fifth-order theorem이나
promotion claim으로 사용하지 않는다.

Operational certificate는 rigorous nonlinear bound가 아니다. Embedded error에 대한 6/5 scaling은
local algebraic defect를 \(O(h^6)\)로 제한하려는 설계 가설이며, arbitrary nonnormal nonlinear
system에서 증명된 상계가 아니다. Finite one-step corpus의 false accept가 0이라는 사실만 주장한다.

## 7. 최종 판정

다음 gate를 모두 실패했다.

- one-step fast fraction \(0.875<0.90\);
- one-step fallback \(0.125>0.10\);
- one-step P95 depth \(13.2>6\);
- adaptive fast fraction \(0.4975<0.90\);
- adaptive fallback \(0.5025>0.10\);
- adaptive median/P95 depth 16;
- same-error speedup \(0.226\) 또는 \(0.123\), gate 1.15 미달.

따라서 q1→q2 implementation infrastructure는 보존하지만, generic primary acceleration method로는
HOLD한다. 더 많은 threshold/path tuning으로 같은 구조를 밀지 않고, 다음 sibling으로 parallel
fifth-order exponential Rosenbrock/EPIRK를 연다.
