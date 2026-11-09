# VigilODE 통합 연구 아키텍처
## 원래 stiff-solver 문제를 위한 selective adaptive portfolio

**역할:** 연구계보·수학적 기능·검증계약을 하나로 연결하는 설계 정본. 문서 overlay이며 solver source 변경이나 성능/production PASS를 의미하지 않는다.

## 1. 원래 목표

VigilODE는 stiff IVP에서 accepted steps는 적은 RODAS5P가 8개의 인과적 stage 때문에 BDF보다 wall time에서 불리해질 수 있다는 문제에서 출발했다. Jacobian construction/factorization, nonlinear stage solve, Krylov/preconditioner, stage critical path, allocations, rejected work도 주요 비용이다.

원래 G0 목표(PR176)는 observed global order ≥4.8, protected R-JF 대비 accepted steps ≤1.2×, 같은 global error에서 median wall speedup ≥1.15×, fast lane explicit Jacobian builds=0, Newton iterations=0, critical depth P95≤6/max≤8, fallback≤0.10이다. 이는 목표이지 달성 결과가 아니다. Official G0 remains 23 nodes/34 edges, historical progress 7/23, successor admitted 0/12, production HOLD.

## 2. 핵심 설계 원칙: 모든 기능을 쓰는 게 아니라 필요한 조합만 선택한다

VigilODE는 모든 기법을 매번 실행하는 pipeline이 아니다. **서로 다른 수학적 기능을 후보로 보유하고, 현재 IVP의 source, dynamics, error state, cost evidence와 resource permissions에 맞는 가장 작은 조합만 switcher가 활성화하는 adaptive solver portfolio**다.

Active route는 대략
`(whole-IVP method, stage/linear backend, optional structural worker(s),
execution lane, error/certificate policy)`
로 표현한다. 이 조합은 무제약 Cartesian product가 아니라 수학적·구현적 compatibility registry에서 허용한 sparse tuples만 가능하다. 미사용 기능, 비싼 observer, probe와 transform은 OFF 상태로 남는다.

선택은 다음 다섯 단계로 분리한다.

- **S0 Admission:** source/model epoch, IVP/output contract, derivative permissions, callback replay-safety, resource ceilings로 후보를 정적 필터링한다. 이 단계는 user callback을 호출하지 않는다.
- **S1 Whole-IVP method:** RODAS5P, DOPRI5/Lawson, BDF, Radau, EXPRB, Peer-W/SEPP 등 수학적으로 서로 다른 적분기 중 하나를 고른다.
- **S2 Stage/linear backend:** 선택된 method의 수학을 보존하는 direct/banded/sparse/Krylov/graph backend를 고른다. 원래 RODAS5P의 8-stage dependency는 유지된다.
- **S3 Structural workers:** homotopy, polynomial/phi, quotient/Lawson, Fourier/Volterra, chart, graph scan 등은 source guard·residual·amortization gate를 통과할 때만 0개 또는 소수 활성화한다.
- **S4 Execution lane:** operator parallel, ensemble, spatial MOL, true Peer-stage parallelism, SIMD 중 실제 method가 허용하는 것만 켠다.

자동 선택은 미래를 완벽히 아는 global optimizer가 아니다. 현재 허용된 후보·관측값·계산예산 아래에서의 최선의 증거기반 선택이며, global optimality/regret guarantee는 별도 증명이 없으면 주장하지 않는다. 상세 실행계약은 `ADAPTIVE_PORTFOLIO_ROUTER_SPEC_KO.md`와 `ADAPTIVE_ROUTER_CONTRACTS.json`을 따른다.

## 3. 혼동하면 안 되는 수학적 구분

| 구분 | 의미 | 제거되지 않는 비용 |
|---|---|---|
| Jacobian-free | 명시적 J 조립·저장을 피함 | JVP, Krylov, preconditioner, residual |
| JVP-based | 방향미분 Jv를 사용 | 여전히 derivative action |
| derivative-free/RHS-only | J/Jv/AD/FD derivative action을 금지 | nonlinear solve, continuation, source/domain proof |
| matrix-free | 행렬 materialization을 피함 | operator application과 linear solve |
| Newton-free | Newton 반복을 피함 | fixed-point/BE/homotopy 수렴·branch 증명 |
| stage-parallel | tableau가 current stage 독립성을 허용 | history/startup/stability/estimator 검증 |
| SIMD/vectorized | lane-wise 연산 | 인과적 stage depth는 자동 감소하지 않음 |
| ensemble-parallel | independent IVP들을 병렬 계산 | 단일 IVP latency 감소를 뜻하지 않음 |

원래 RODAS5P의 하삼각 stage coupling은 순차적이다. 같은 W/JVP 재사용이나 Krylov parallelism은 stage independence가 아니다. Peer-W는 선택된 coefficient family에서 current stages가 독립일 수 있지만, 원래 RODAS5P의 parallelized version이 아니라 별개의 two-step/general-linear integrator다.

## 4. 계층별 연구내용

### L0: Original problem / source identity
원래 (f(t,y)), (f_t), mass/DAE scope, IC/span/output/event/norm, physical invariant domain, source blob/hash, model epoch, callback purity/reentrancy를 고정한다. Applied REI/BASS는 별도 client이며 generic core의 theorem evidence로 합산하지 않는다.

### L1: Dynamical / geometric model
Dissipativity와 logarithmic norm, nonnormal transient gain, invariants/energy/weighted observable, quotient/semigroup, invariant manifold/chart, source-specific Fourier/Volterra 구조를 다룬다. (alpha(J)<0)만으로 nonnormal transient growth를 배제할 수 없다. Pseudospectral/resolvent 계산은 매 step이 아니라 disagreement/uncertainty가 있을 때만 probe한다.

### L2: Derivative and operator permissions
ANALYTIC_J, EXACT_JVP, AD_JVP, FD_JVP, OPERATOR_ONLY, RHS_ONLY를 분리한다. OPERATOR_ONLY는 matrix materialization을 피하면서 operator action은 허용할 수 있지만, RHS_ONLY는 Jv조차 허용하지 않는다.

### L3: Integrator method identity
각 method는 equation/tableau/coefficient hash, order, step-ratio domain, stability, startup, full history, rejection/restart, embedded/global estimator, controller, dense output/events를 가진다. BDF/Radau/Peer/EXPRB/RODAS를 하나의 solver로 합치지 않는다.

### L4: Linear/nonlinear solve and execution
Dense/banded/sparse direct, GMRES/FGMRES/GCRO-DR, preconditioner, multi-RHS, stage DAG, SIMD, ensemble 및 spatial CPU kernel을 관리한다. 기존 ParallelExecution, stage_batch, Dopri5Ensemble, LinearOperator, WorkLedger, benchmark schema를 재사용한다.

### L5: Optional structure-aware transform
Causal raw-U/q1-q2, graph-Schur/affine scan, homotopy/inverse-Picard, polynomial/resolvent/Leja/Laguerre/Chebyshev, quotient/Lawson, Fourier/Volterra, invariant lift는 source-specific transform/corrector다. 원래 식으로 돌아가는 map과 original residual가 없으면 승격하지 않는다.

### L6: Original-target residual and global error
시간 이산화 결함, original-W/Jacobian mismatch, true linear/Krylov residual, nonlinear root residual, transform/chart/alias defect, roundoff, output/interpolation, multistep history transfer를 포함한다. 누락된 항목은 0이 아니라 NOT_PROVIDED로 남긴다.

### L7: Adaptive supervisor
Capability registry, regime alphabet, hysteresis/dwell, accepted-boundary switching, per-regime evidence memory, source/history/cache epochs와 rollback을 관리한다. 단순 rejection/pressure만으로 stiffness를 확정하지 않는다.

### L8: Complete work/performance authority
RHS/JVP/Jacobian/linear/preconditioner, setup/startup, rejected/failed/pilot work, transform/certificate/router, history transfer, fallback, output/alloc/RSS, paired same-achieved-global-error CPU cost를 기록한다.

## 5. 구조별 재사용/승격 경계

| 연구 stream | 과거 이력과 재사용 | 승격 제한 |
|---|---|---|
| JVP/coupled stage | PR74/75/77 | source scoped; JVP ≠ derivative-free |
| RHS-only/homotopy | PR91/92/117, PR154/156/177 | exact original equation/source/epoch/residual 필요 |
| Krylov/phi/polynomial | PR90–93, PR105–112 | matrix-action component ≠ full IVP method |
| RVJ/Volterra/Fourier | PR135/136, PP05/06/15 | source-specific; PP05/06 negative, PP15 timing absent |
| Global error/observable | PR158/160/163/167/170/171/172/174 | reference uncertainty와 OOS gap 유지 |
| Causal graph/C5 | PR177–187 | 8-stage causality 유지; C5 full-IVP negative 결과 보존 |
| Dynamics/symbolic router | C8 R2/R4 | C8 numeric validation PENDING |
| Guarded symbolic execution | PR98 | controller-state proof; physical ODE Markov partition 아님 |
| Peer-W/SEPP | published Peer family literature and 2026 variable-grid Peer | established prior art; native VigilODE qualification remains separate |
| Spatial MOL clients | Gray-Scott 2D/P06 sphere FV | scoped geometry clients, generic 3D backend 아님 |

## 6. 공통 오차·비용 계약의 수학적 출발점

지정된 one-step map (Psi_n)의 Lipschitz factor가 (Lambda_n), local defect가 (d_n), 실제 실행 map perturbation이 (eta_n)이면
[
E_{n+1}leq Lambda_nE_n+d_n+eta_n.
]
Peer/multistep에서는 전체 history를 augmented state로 포함하고, method switch 때 history/state transfer defect를 더해야 한다. 이 recurrence 자체는 표준적인 error propagation이지 새로운 theorem이 아니다.

원래 stage 식 (Wk=b), 실제 실행 연산자 (widetilde W), 계산값 (widetilde k), residual (widetilde r=b-widetilde Wwidetilde k)에 대해
[
W(widetilde k-k)=(W-widetilde W)widetilde k-widetilde r,
]
[
|widetilde k-k|leq|W^{-1}|
left(|W-widetilde W||widetilde k|+|widetilde r|ight).
]
이는 Jacobian/W mismatch와 inexact solve residual을 분리한다. 실제 nonlinear multistage method의 global bound로 가려면 (|W^{-1}|), tableau stage propagation, nonlinear remainder, nonnormality, variable-step/history transfer, transform conditioning, roundoff/output을 추가로 제어해야 한다.

## 7. Novelty verdict

현재 새로운 scientific novelty는 확립되지 않았다. Parallel Peer-W, global error-controlled peer methods, variable-grid stiff norm, Krylov residual, homotopy, symbolic switching, CAD, nonnormal diagnostics 모두 관련 선행연구가 있다.

잠재적 기여는 **source-attested compositional global-error/work certificate**다. Original-W mismatch, actual inexact solve residual, time defect, source-transform/chart defect, output/roundoff, variable-step history와 accepted-boundary method transitions를 하나의 fail-closed global bound 및 complete work receipt로 묶는 정리 후보다. 이는 CONJECTURAL/UNRESOLVED이며, 선행연구 대조·method-specific proof·adversarial nonnormal tests·full-IVP Pareto gain 전에는 novelty를 주장하지 않는다.

## 8. 다음 실행

T02 current fair-ab/global-error authority replay → T03/T04 C5 long-path full-IVP economics → T06 BDF/Radau/Peer-W/SEPP method qualification → T07 existing switcher의 adaptive multi-method extension → T08 geometry-specific MOL → T09 matched full-IVP/release review. T05 scan은 long-path holdout에서 crossover가 보일 때만 구현한다.

상세 이력은 HISTORY_MATRIX, LINEAGE_DAG, SOLVER_COMPONENT_INVENTORY, NEGATIVE_EVIDENCE_INDEX와 INTEGRATION_HISTORY_MAP에 연결한다. Official G0 remains unchanged and production HOLD.
