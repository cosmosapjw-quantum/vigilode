# Strict Matrix-Free Common-W Multiple-RHS Gate v0.14

## 1. 연구 질문

RODAS5P의 8개 stage가 동일한 shifted operator

\[
W=M-h\gamma J_n
\]

를 사용한다는 사실을 이용해, 명시적 Jacobian 행렬과 LU factorization 없이 여러 RHS를 동시에 처리할 때 실제 wall time과 operator work를 줄일 수 있는가?

이번 cycle은 nonlinear stage-coupling 또는 homotopy round 자체를 아직 변경하지 않는다. 목적은 다음 homotopy branch가 사용할 선형 kernel을 동일한 Rust/`faer` 환경에서 선별하는 것이다.

## 2. 수학·수치 계약

상태방정식은

\[
M\dot y=f(t,y),\qquad J_n=\partial f/\partial y
\]

이고, `gamma`는 무차원 RODAS coefficient다. 따라서 `M`과 `h J_n`은 같은 차원을 가지며

\[
Wv=Mv-h\gamma J_nv
\]

는 차원적으로 일관된다.

Strict matrix-free의 이번 정의는 다음과 같다.

```text
explicit Jacobian matrix construction = 0
shifted-matrix factorization           = 0
operator access                        = Mv + Jv only
Newton iteration                       = not part of this linear gate
```

모든 solver는 같은 analytic diagonal preconditioner와

\[
\|b-Wx\|_2\le\max(\mathrm{atol},\mathrm{rtol}\|b\|_2)
\]

의 unpreconditioned true-residual certificate를 사용한다.

## 3. 구현 경로

### Baseline

8개의 independent restarted GMRES를 serial로 실행한다.

### RHS concurrency

동일한 independent GMRES 8개를 local Rayon pool에서 1/2/4 thread로 실행한다. Krylov work는 변하지 않으며 wall-time concurrency만 노린다.

### Block GMRES

8개 RHS의 preconditioned initial block을 rank-revealing orthogonalization으로 압축하고, block Arnoldi 공간에서 모든 RHS를 동시에 최소제곱 투영한다. Nearly dependent direction은 cycle 중에 제거한다. 현재 구현은 nonrestarted이며 128 search-direction hard cap을 갖는다.

### Seeded shared GMRES

대표 RHS의 Krylov basis를 다른 RHS에 투영한 뒤, 각 RHS를 독립 true-residual refinement로 마무리한다. 낮은 numerical rank에서만 큰 이득을 기대하는 specialized path다.

## 4. Canonical matrix

\[
N\in\{32,128,512,2048\},
\]

\[
\operatorname{rank}(B)\in\{1,2,4,8\},
\]

\[
\eta_{\rm nonnormal}\in\{0,0.2,0.9\},
\]

\[
|\lambda_{\rm stiff}|\in\{10^2,10^4,10^6\},
\]

\[
n_{\rm thread}\in\{1,2,4\}.
\]

총 432 cases와 1728 solver rows를 사용했다. 각 timed row는 warm-up 후 최소 5 ms가 되도록 동일 solve를 batch하고 5회 반복해 중앙값을 취했다.

## 4.1 독립 canonical rerun

Canonical campaign을 같은 exact HEAD와 CPU/thread contract에서 두 번 실행했다. Timing field는 물리적으로 변동 가능하므로 byte identity를 요구하지 않고, timing field를 제외한 모든 scientific/work field와 scientific checksum의 exact identity를 요구했다.

```text
scientific checksum identity: PASS
timing ratio rerun/original Q25: 0.9630
median:                         0.9977
Q75:                            1.0332
rows compared:                  1692
```

따라서 주요 중앙값과 regime 판정은 run-to-run timing noise에 비해 안정적이다.

## 5. 전체 결과

| Solver | 성공/전체 | median speedup | IQR | median operator ratio | max true relative residual |
|---|---:|---:|---:|---:|---:|
| independent serial | 432/432 | 1.000 | 1.000–1.000 | 1.000 | 1.070e-10 |
| independent Rayon | 432/432 | 1.828 | 1.011–2.553 | 1.000 | 1.070e-10 |
| block GMRES | 396/432 | 5.018 | 2.216–11.068 | 0.331 | 9.749e-10 |
| seeded shared | 432/432 | 1.030 | 0.967–1.111 | 0.974 | 8.677e-10 |

모든 candidate row에서:

```text
explicit_jacobian_builds = 0
factorization_builds     = 0
```

이다.

## 6. Block GMRES의 유효 영역

### RHS rank

| Exact RHS rank | 성공률 | median speedup | median operator ratio |
|---:|---:|---:|---:|
| 1 | 1.000 | 6.470 | 0.331 |
| 2 | 0.944 | 5.389 | 0.368 |
| 4 | 0.889 | 5.068 | 0.353 |
| 8 | 0.833 | 3.679 | 0.238 |

높은 rank에서도 성공한 경우 operator work는 크게 감소한다. 그러나 dimension 512/2048에서 rank가 커질수록 robustness가 떨어진다.

### Stiffness와 nonnormality

| stiffness | nonnormality | 성공률 | median speedup |
|---:|---:|---:|---:|
| 1e2 | 0.0 | 1.000 | 12.423 |
| 1e2 | 0.9 | 1.000 | 8.900 |
| 1e4 | 0.9 | 0.750 | 1.557 |
| 1e6 | 0.2 | 0.875 | 2.314 |
| 1e6 | 0.9 | 0.625 | 1.937 |

36개 failure는 thread 수와 무관하고 모두 128 search directions 소진으로 끝났다. 이는 concurrency race가 아니라 현재 nonrestarted block Krylov 공간의 robustness 한계다.

## 7. Independent Rayon

| threads | median speedup | operator ratio | `>=1.15x` rows |
|---:|---:|---:|---:|
| 1 | 0.996 | 1.000 | 1/144 |
| 2 | 1.835 | 1.000 | 144/144 |
| 4 | 2.919 | 1.000 | 144/144 |

이 경로는 operator work를 줄이지 않지만 가장 단순하고 robust한 within-step concurrency fallback이다.

## 8. Seeded shared path

Rank 1에서는 median speedup 8.014, operator ratio 0.0735로 강하다. Rank 2 이상에서는 median speedup이 1.01–1.03, operator ratio가 약 0.974로 사실상 독립 GMRES와 같다.

따라서 seeded shared path는 general default가 아니라 numerical-rank-one detector가 확실한 경우에만 사용할 specialized backend다.

## 9. 문헌상 위치

Multiple-RHS nonsymmetric systems에서 한 Krylov 공간 또는 block Krylov 공간을 공유하는 방법은 확립된 연구 계열이다. Simoncini–Gallopoulos는 하나의 Krylov subspace와 projection을 이용하는 multiple-RHS 방법을 제안했고(DOI 10.1137/0916053), block GMRES의 nearly rank-deficient block과 deflated restart를 다루는 알고리즘도 존재한다(DOI 10.1137/140961912). Iteration마다 block deflation과 flexible preconditioning을 결합한 변형도 보고됐다(DOI 10.1137/120883037).

Rosenbrock–Krylov는 이번 shared linear backend와 구별해야 한다. ROK는 시간 discretization과 Krylov projection을 하나의 order theory로 공동 설계하는 별도 integrator family이며, 단순히 RODAS stage RHS를 shared basis로 푸는 backend가 아니다(DOI 10.1137/130923336).

## 10. Claim boundary

### 허용

- strict explicit-J-free common-W action을 실제로 구현했다.
- multiple-RHS block Krylov가 broad mild/moderate regime에서 operator work와 wall time을 줄였다.
- independent Rayon GMRES는 2/4 threads에서 robust한 concurrency fallback이다.
- seeded shared basis는 rank-1 RHS에서 강하다.

### 금지

- block GMRES가 universal default다.
- synthetic exact rank가 실제 homotopy stage RHS rank를 대표한다.
- 이번 microkernel speedup이 integrated RODAS/homotopy speedup이다.
- explicit Jacobian-free가 곧 JVP-free라는 주장.
- current diagonal preconditioner가 arbitrary physics problem에서 동일하게 이용 가능하다는 주장.

## 11. 최종 판정

```text
PROMOTE:
  matrix-free Wv contract
  batched LinearOperator API
  independent Rayon GMRES fallback
  block/seeded research backends
  common true-residual and work ledger

HOLD:
  block GMRES universal default
  hard-coded synthetic-rank dispatcher
  integrated q=0/1/2 homotopy speedup
  production preconditioner claim
  sparse/GPU/DAE generalization
```
