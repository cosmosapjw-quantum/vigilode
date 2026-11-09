# Peer-W 우선 연구·구현 계획

**작성일:** 2026-11-09 (KST)  
**상태:** R2/R4/R5 및 C2/C3/C4 scoped research 완료; Rust 구현·실행 검증은 NOT_RUN  
**코드 authority:** research/vigilode-c5-native-graph-schur-20261102 @ 569ce02629498fec133db9ce368eb303ba943b69, tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae  
**계획 authority:** 공식 G0 DAG는 그대로 두고 SHORT_TERM_DAG_20261109.json의 additive Peer-W overlay로 연결한다.  
**실행 정책:** CPU only. GPU dependency, kernel, fallback 및 benchmark 금지.

## 1. 결정 요약

VigilODE의 우선 연구축을 **독립 stage를 갖는 parallel two-step W/Peer 적분기**로 고정한다. 이는 RODAS5P의 strict-lower causal 8-stage tableau를 억지로 동시 실행하는 최적화가 아니라, 처음부터 현재 step의 stage equations가 서로 독립이 되도록 설계된 별도 whole-IVP method family다.

첫 구현 실험은 정확 coefficient가 복원된 두 개의 소형 stiff Peer-W를 대상으로 한다.

| 우선순위 | 후보 | stage/order | 핵심 용도 | 판정 경계 |
|---|---|---:|---|---|
| 1 | **PTSW2C** | 2 / variable-step order 3 | 첫 구현 prototype. 2 stage, common shift, method-specific embedded estimate | source A(81.1°)-stable, rho_inf 약 0.6796. L-stable 아님. 전체 sector는 interval proof 미완 |
| 2 | **PTSW2B** | 2 / order 2 | L(90)-stable stiff-damping control | source/exact-symbolic prior certificate. 낮은 차수로 step 수가 커질 위험 |
| 3 | **PTSW3B / PTSW3C** | 3 stages | 다음 고차·강성 trade-off comparator | PTSW3B full coefficient array/estimator 미전사. PTSW3C는 look-ahead node와 비-L-stable damping |
| G0 path | **PTSW5 (p=q=5) 또는 6-stage Peer-W (p=5)** | 5+ order | 원래 G0의 order >=4.8을 만족시킬 승격 후보 | 원전 고차 table의 exact transcription·independent verification 전까지 unqualified |

PTSW2C와 PTSW2B는 같은 W/Peer adapter와 test harness를 공유할 수 있지만 method identity, tableau, startup 및 estimator는 서로 분리한다. 둘 중 어느 것도 원래 G0 승격 후보가 아니다. G0의 global order >=4.8은 최소 5차 이상의 candidate가 별도 검증되어야 한다.

## 2. 원래 acceptance target과 보호 범위

PR #176 (2026-11-01)에서 복원된 원래 G0 목표는 달성 수치가 아닌 승격 기준이다.

- Global order >= 4.8
- RODAS5P 대비 accepted steps <= 1.20x
- 동일 achieved global error에서 median speedup >= 1.15x
- Fast lane explicit Jacobian construction = 0
- Fast lane Newton iterations = 0
- Parallel critical depth P95 <= 6, maximum 8
- Fallback fraction <= 0.10

공식 G0 DAG 23 nodes / 34 edges, historical progress 7/23, successor 0/12, Production HOLD 및 PR #176의 28-method inventory는 불변이다. 이번 수정은 실행 가능한 short-DAG overlay만 확장한다.

## 3. Method identity와 수학적 구조

### 3.1 PTSW common-shift W-Peer

일반적인 two-step W/Peer form을 다음처럼 둔다. $h_m$은 새 step, $sigma_m=h_m/h_{m-1}$, $T_m$은 source/solver가 허용한 W-operator다.

$Y_m = 1 tensor u_m + h_m (A_m tensor I) K_{m-1}$,

$(I_s tensor W_m) K_m = F(t_m 1 + h_m c, Y_m) + h_m (Gamma_m tensor T_m) K_{m-1}$,

$u_{m+1}=u_m+h_m[(b^T tensor I)K_m+(v^T tensor I)K_{m-1}]$,

$W_m=I-h_m gamma T_m$.

동일한 gamma와 $T_m$을 사용하고 현재-step coupling이 없으면 각 $K_{m,i}$의 RHS는 이미 commit된 이전-step peer data에만 의존한다. 그러므로 같은 $W_m$를 공유하는 stage linear solves는 수학적으로 독립이다. 시간 step $m$에서 $m+1$로의 marching 자체는 여전히 causal/sequential이다. 병렬화 대상은 한 time step 내부의 stage RHS, operator application, linear solves다.

Common-shift direct route는 step당 factorization 1회와 여러 RHS solve를 목표로 한다. Matrix-free route는 각 stage GMRES 또는 검증된 block-GMRES를 비교한다. block-GMRES 호출 수 하나를 stage solve 하나로 잘못 계산하지 않고 vector matvec, block matvec, basis rank, iteration, preconditioner 및 residual 비용을 모두 기록한다.

### 3.2 PTSW2C: 첫 order-3 prototype

Podhaisky (2002)에서 복원된 정확 계수:

- $c=(-1,5/9)^T$, $gamma=12/25$, $b=(1/28,27/28)^T$, $v=(0,0)^T$
- $A(sigma)$ 행렬 성분: $(8-9 sigma)/28$, $9(sigma-4)/28$, $-5(8+5 sigma)/252$, $5(36+5 sigma)/252$
- $Gamma(sigma)$ 행렬 성분: $(24-54 sigma)/175$, $54(sigma-2)/175$, $(24+30 sigma)/175$, $(-108-30 sigma)/175$
- $W_m=I-(12/25)h_mT_m$

해당 convention의 simplifying conditions는 다음과 같다.

$C(q): sum_j a_ij(c_j-1)^(ell-1) = sigma^(ell-1)c_i^ell/ell$,

$Gamma(q): sum_j Gamma_ij(c_j-1)^(ell-1) = -gamma sigma^(ell-1)c_i^(ell-1)$,

$B(p): sum_j b_j sigma^(ell-1)c_j^(ell-1) + sum_j v_j(c_j-1)^(ell-1) = sigma^(ell-1)/ell$.

기존 repository certificate와 이번 turn의 독립 exact-rational check에서 $C(2), Gamma(2), B(3)$ residual은 symbolic sigma에 대해 0이다. Source convergence theorem상 $p_* = min(q+1,p)=3$, 단 coefficient boundedness on the admitted ratio interval, smooth/Lipschitz RHS 및 compatible startup 조건을 만족해야 한다.

Startup는 backward integration을 요구하지 않는다. 상대 node 집합 {-2,-4/9,0}에 대해 $t_1=t_0+2h_0$, 이전 stage derivative evaluation times는 $t_0$와 $t_0+(14/9)h_0$다. Solution startup error $O(H^3)$, stage derivative startup error $O(H^2)$ 요구조건을 adapter test로 강제한다. DOPRI5 starter 사용은 후보이며 intermediate output 정확도를 검증하기 전에는 qualification하지 않는다.

Method-specific embedded pair:
- $b_e=(19/560,513/560)$
- $v_e(sigma)=(-(8+27sigma)/560, 9(4+3sigma)/560)$

기존 source recipe는 normalized RMS component estimator, reject if estimate > 1, $0.85 E^(-1/2)$ step scaling clipped to [0.5,2] 및 history/hysteresis를 사용한다. 이 estimator를 RODAS5P 또는 generic RK pair와 혼용하지 않는다. 이는 자동 step controller의 estimate이지, 그 자체로 rigorous finite-step global-error enclosure라는 뜻은 아니다.

Stability evidence: Podhaisky source-reported A(81.1°)-stability와 rho_inf=0.6796; repository prior audit의 independent floating root-locus scan은 boundary 약 81.16974941°, rho_inf=0.6795805635, 81.1° sampled sector max rho=0.99999984529 및 82° outside witness rho=1.0105069736을 보고했다. 이 수치는 **NUMERICALLY_CHECKED / source-supported**이며 unbounded sector의 interval-certified proof나 variable-step product stability proof가 아니다. PTSW2C는 L-stable로 부르지 않는다.

### 3.3 PTSW2B: L-stable order-2 control

- $gamma=(3-sqrt(3))/2$, $c=(3-2sqrt(3),1)^T$, $b^T=(0,gamma)$
- $A(sigma)$ 행렬 성분: $3(5-3sqrt(3))sigma/8$, $(24-16sqrt(3)+3(-5+3sqrt(3))sigma)/8$, $-(1+sqrt(3))sigma/8$, $(8+(1+sqrt(3))sigma)/8$
- $Gamma(sigma)$ 행렬 성분: $3(sqrt(3)-2)sigma/4$, $(-6+2sqrt(3)+6sigma-3sqrt(3)sigma)/4$, $sqrt(3)sigma/4$, $(-6+2sqrt(3)-sqrt(3)sigma)/4$
- $v^T(sigma)=(sqrt(3)-1)(sigma,4-sigma)^T/8$
- $W_m=I-h_m gamma T_m$

$C(2),Gamma(2),B(2)$ exact residuals are zero. Podhaisky's source classifies it as L(90)-stable with rho(M_inf)=0; the existing exact certificate reports characteristic polynomial lambda^3, $M_inf^3=0$, $M_inf^2 !=0$. This current turn independently rechecked the order moments, not the full stiff-limit proof script.

The source's exact RHS rearrangement defines $p_{m,i}=sum_j(Gamma_ij(sigma_m)/gamma)k_{m-1,j}$, $x_{m,i}=k_{m,i}+p_{m,i}$, so $W_mx_{m,i}=f(t_m+c_ih_m,Y_{m,i})+p_{m,i}$, then $k_{m,i}=x_{m,i}-p_{m,i}$. This removes the explicit $T_m Gamma_m K_{m-1}$ multiplication from RHS assembly by exact algebra; it does not remove linear solves or make a generic matrix-vector action free.

The startup from Podhaisky Algorithm 4.1 uses $t_1=t_0+(2sqrt(3)-2)h_0$, stage derivative times $t_0,t_1$, and first step ratio 1. The embedded pair and hysteresis are recorded in PTSW2B_EXACT_TABLEAU_CERTIFICATE_20261109.md. Order two can increase accepted steps; this is a damping control, not a G0 promotion candidate.

### 3.4 Higher-order path to G0

Primary literature establishes existence, not a machine-qualified coefficient file in the current worktree:
- Podhaisky–Schmitt–Weiner (2002) reports linearly implicit $s$-stage parallel two-step W methods with $p=s$, $q=s$ and favourable generalized A/L stability. This makes $s=5,p=q=5$ the minimum-stage family-level G0 target.
- Schmitt–Weiner (2004) proves order $p=s-1$ and stability for general step sequences for the independent-stage Peer-W family, reporting candidate methods through order seven. This makes $s=6,p=5$ another family-level G0 target.
- The exact five-/six-stage coefficient arrays, shift structure, step-ratio domain, startup and estimator must be obtained from the primary tables and independently transcribed before a concrete method is registered.

Do not infer that every Peer tableau is parallel. Check the actual current-stage coefficient matrix: same-step off-diagonal coupling must vanish for independent stage execution. If different gamma_i occur, treat it as a multi-implicit family and charge each distinct shift's factorization/preconditioner. If it is triangular, stage graph is sequential, irrespective of the Peer name.

## 4. Keep adjacent method families separate

| Family | Mathematical identity | Stage dependency | Derivative / nonlinear contract |
|---|---|---|---|
| Common-shift PTSW-W | linearly implicit two-step W/Peer | independent current stages, previous-step history | W operator $T_m$ still needs a declared source/approximation. No nonlinear Newton; a JVP-based $T_m$ is Jacobian-free but not derivative-free |
| Multi-implicit PTSW-W | independent shifts per stage | stage-independent but different systems | each shift/setup is costed separately |
| Implicit parallel Peer (IPP) | separate nonlinear implicit peer equations | independent roots if tableau says so | candidate for RHS-only homotopy; not the same stability/order family as PTSW-W |
| Rosenbrock-type Peer | Peer history plus same-step triangular correction can occur | often sequential unless off-diagonal terms are zero | do not call fully parallel without exact tableau graph |
| TASE/SEPP | explicit Peer plus Jacobian-related TASE operator | parallel stage updates | derivative/operator-dependent and potentially s*p inner linear solves; not derivative-free PTSW-W |
| EPPEER | explicit parallel Peer, orders 3–9 | independent RHS stages | nonstiff control; not a stiff W solver |
| 2026 diagonally implicit Peer | variable-step stiff Peer with common-norm analysis | diagonal/lower-triangular stage solve may be sequential | stability comparator, not assumed parallel |

**Derivative policy.** Three labels must be distinct: (i) no explicit dense Jacobian, (ii) no explicit Jacobian but JVP/matrix-free derivative action is used, (iii) genuinely derivative-free RHS-only. A W-Peer implementation may meet (i) and avoid Newton, but it does not automatically meet (iii). The G0 fast lane's strict zero-derivative route requires an admitted $T_m$ that is source-provided/derivative-free with its own validity evidence, or a separate implicit-peer RHS-only method. The Jacobian-free JVP path is an explicitly labelled fallback, not derivative-free.

## 5. Existing Rust infrastructure to reuse

Pinned code source is C5 above. The source inspection found these APIs and boundaries:

- crates/rodas5p-integrators/src/parallel.rs: bounded local Rayon ParallelExecution, deterministic map_ordered, try_for_each_ordered_mut, operator-row application. Reuse it; do not create a second thread pool.
- crates/rodas5p-integrators/src/stage_batch.rs: an 8-stage RODAS stage-batch feasibility/timing harness. It is not a generic Peer stepper and does not prove RODAS stage independence.
- crates/rodas5p-core/src/solver_types.rs: LinearSolverConfig, LinearMethod, LinearSolveReport. Keep method-specific work accounting.
- crates/rodas5p-krylov/src/block_gmres.rs: solve_block_gmres accepts multiple RHS, reports each true residual, block iterations, basis rank and operator work. Compare against independent parallel GMRES; a shared basis is not assumed optimal or equivalent.
- crates/rodas5p-krylov/src/workspace.rs: reusable GMRES/LGMRES/GCRO-DR workspaces and retained-memory accounting.
- crates/vigilode/src/session.rs: DOPRI5 accepted checkpoints/session model, not yet two-step Peer history.
- crates/vigilode/src/automatic.rs, switching.rs: current public choice is DOPRI5/authorized RODAS only. Switches occur at committed boundaries, but this is not a multi-method Peer adapter.
- crates/rodas5p-integrators/src/nonlinear_homotopy.rs: source-bound collective-model homotopy with packet certificates; do not relabel it a general RHS-only Peer root engine.
- crates/rodas5p-fair-ab/src/global_error.rs, adaptive_global_error.rs: common-grid/reference-WRMS/error-output-policy/Pareto/work/timing authority. Replay this current authority; do not rebuild it. It is not automatically a rigorous continuous global-error theorem for any newly added Peer.

**Implementation boundary:** create a Peer-specific method kernel/history state in the existing integrator architecture; reuse executor, operator/solver, work counters and fair-ab reporting. Add no duplicate frontend, controller, benchmark harness or thread pool. Factorization reuse, block solve semantics and error-estimator work must be measured, not presumed.

## 6. Causal, Newton-free, derivative-free and homotopy requirements

- **Causal:** each accepted Peer time step consumes only committed previous peer history. Time marching remains sequential. Within a step, independent stages are parallel.
- **Newton-free W path:** solve linear W systems without nonlinear Newton iteration. The count of linear solves remains nonzero.
- **Derivative-free strict path:** requires RHS-only construction of every stage residual and a valid derivative-free root process. A JVP/finite-difference/AD route violates this strict label.
- **Jacobian-free fallback:** JVP/Krylov may be used only when policy permits it. Report derivative action counts separately from explicit Jacobian builds.
- **Homotopy-assisted root worker:** candidate IPP equations are independent nonlinear stage roots. A possible worker solves Phi_i(x)=0 through an RHS-only homotopy path, but must verify the exact original stage residual after continuation. The existing source-specific homotopy module is not a drop-in generic solver.
- **Fail closed:** a candidate root is never accepted on its auxiliary homotopy residual alone. Verify the original stage equation, residual-to-local-error implication, contraction/domain hypotheses, floating-point/roundoff allowance and rollback to a protected method.

## 7. Correctness and performance gates

### Mathematical/tableau
1. Exact transcription with provenance, units/conventions and source page/table/equation.
2. Independent coefficient transcription; exact rational/algebraic order and stage-order conditions.
3. Zero-stability and variable-step admissible ratio set; common norm/product stability over permitted step sequences.
4. Full complex stability region, stiff-limit damping, W-perturbation robustness and nonnormal cases.
5. Correct startup vector, history initialization and startup-error order.
6. Source-matched estimator, controller asymptotics and output/dense interpolation contract.

### Kernel/implementation
1. Same-step stage graph audit: zero dependency edges between current Peer stages.
2. Same W factor reused correctly; direct factorization plus multiple RHS, independent Krylov, and block-Krylov measured separately.
3. Per-stage original residual, linear residual, iteration, matvec, preconditioner, allocations and memory.
4. Rejection restores prior accepted state, peer history, estimator/controller history and workspace generation atomically.
5. Event/discontinuity/restart/history invalidation semantics are explicit.
6. Concurrency is observed with CPU threads; SIMD is a separately benchmarked vectorization axis, not inferred from Rayon.
7. Failure and fallback cost charged to one complete work ledger.

### Whole-IVP
- Same source, initial state, output grid and dense/clipped policy; same achieved global error/reference uncertainty; full elapsed time including startup, factorization, all stages, estimator, rejected work, fallback, output and setup.
- Compare PTSW2C, PTSW2B, protected RODAS5P and BDF/Radau controls. Then compare a verified p>=5 PTSW candidate.
- Gate against original G0 targets unchanged. No candidate is promoted on stage kernel speed or accepted-step count alone.

## 8. Novelty and significance: defensible boundary

Parallel stiff Peer stages, high-order Peer-W, multi-implicit Peer-W, variable-step stability and Peer-specific global error control are established prior art. The 2011 Kulikov–Weiner work already addresses local/global error estimation and automatic step control for linearly implicit parallel two-step Peer-W. The 2026 Lang–Schmitt preprint provides common-norm estimates for variable-step diagonal-implicit Peer families. Therefore none of these alone is a novelty claim.

The best current conjecture remains a **source-attested compositional global-error and complete-work certificate across derivative-permitted method transitions**, specialized to independently solvable Peer stages. Candidate contributions to test:
1. Compose original-ODE residual, inexact W solve, homotopy stage-root defect, startup/history transfer and switch/restart defects into one global-error budget with explicit stability amplification.
2. Couple that certificate to complete CPU work/span evidence, so an adaptive controller can select among a small set of prequalified Peer-W / RODAS / explicit methods without a Cartesian feature search.
3. Show a nontrivial domain where the derivative-free stage-root lane beats W/JVP alternatives at matched global error while retaining original-equation residual certification.

Status: **CONJECTURAL / UNRESOLVED**. A literature matrix, exact theorem, adversarial counterexamples and matched-error benchmark are required before any novelty/significance claim.

## 9. Next staged research loop

| Node | Work | Exit condition |
|---|---|---|
| PW00 | completed source/history/API integration | this document, updated DAG, source locks and receipt are pushed/read back |
| PW01 | implement PTSW2C order-3 Peer kernel and original estimator | exact algebra tests, startup/history/reject/restart tests, deterministic serial-parallel parity |
| PW02 | implement PTSW2B L-stable order-2 control | exact tableau/embedded tests, stiff scalar and MOL tests |
| PW03 | recover and independently transcribe p>=5 W-Peer table | source page refs, two transcriptions, exact order/stage-order/zero-stability/stage graph |
| PW04 | integrate Peer session, startup, adaptive history and fair-ab reporting | transactional rejection/restart and method-specific estimator pass |
| PW05 | separate RHS-only implicit Peer homotopy worker | exact original residual and residual-to-LTE/domain theorem; otherwise NO-GO |
| PW06 | matched full-IVP portfolio evidence | same achieved global error and complete-work comparison; G0 gate retained |

PW01/PW02/PW03 may progress as separate method-specific tasks after PW00. PW04 needs T02 authority replay. PW06 needs T02 plus a qualified p>=5 candidate. T03/T04/T05 C5 work remains preserved as a separate branch of research; it is not a prerequisite for Peer unit-kernel correctness.

## 10. Primary literature

1. H. Podhaisky, B. A. Schmitt, R. Weiner, “Design, analysis and testing of some parallel two-step W-methods for stiff systems,” Applied Numerical Mathematics 42 (2002), 381–395. https://doi.org/10.1016/S0168-9274(01)00162-3
2. H. Podhaisky, Parallele Zweischritt-W-Methoden, PhD thesis (2002). https://doi.org/10.25673/3017
3. B. A. Schmitt, R. Weiner, “Parallel Two-Step W-Methods with Peer Variables,” SIAM J. Numer. Anal. 42 (2004), 265–282. https://doi.org/10.1137/S0036142902411057
4. R. Weiner, B. A. Schmitt, H. Podhaisky, “Parallel ‘Peer’ two-step W-methods and their application to MOL-systems,” Applied Numerical Mathematics 48 (2004), 425–439. https://doi.org/10.1016/j.apnum.2003.10.005
5. B. A. Schmitt, R. Weiner, H. Podhaisky, “Multi-Implicit Peer Two-Step W-Methods for Parallel Time Integration,” BIT Numerical Mathematics 45 (2005), 197–217. https://doi.org/10.1007/s10543-005-2635-y
6. G. Yu. Kulikov, R. Weiner, “Global error estimation and control in linearly-implicit parallel two-step peer W-methods,” J. Comput. Appl. Math. 236 (2011), 1226–1239. https://doi.org/10.1016/j.cam.2011.08.006
7. G. Yu. Kulikov, R. Weiner, “A Singly Diagonally Implicit Two-Step Peer Triple with Global Error Control for Stiff Ordinary Differential Equations,” SIAM J. Sci. Comput. (2015). https://doi.org/10.1137/140997589
8. J. Lang, B. A. Schmitt, “Stiff Norm Estimates for Variable-Stepsize Peer Two-Step Methods,” arXiv:2608.26913 (2026-08-27). https://arxiv.org/abs/2608.26913
9. D. Conte, G. Pagano, B. Paternoster, “Time-accurate and highly-stable explicit peer methods for stiff differential problems,” Communications in Nonlinear Science and Numerical Simulation 119 (2023), 107136. https://doi.org/10.1016/j.cnsns.2023.107136
10. G. Pagano, “Stabilized explicit peer methods with parallelism across the stages for stiff problems,” Applied Numerical Mathematics 207 (2025), 156–173. https://doi.org/10.1016/j.apnum.2024.08.023


## 11. PW07 conditional error-certificate addendum

Before implementing transactional Peer history or automatic switching, use `PEERW_ERROR_CERTIFICATE_THEORY_20261109.md`. The conditional recurrence is E_(n+1) <= kappa_n(chi_n E_n + eta_n) + delta_n. kappa is a source/method/grid-specific propagation gain; chi and eta bound transfer of the complete history at a method switch; delta includes stage original-W residual and operator perturbation, temporal truncation, startup/history, homotopy-root, rounding and output defects. Missing constants imply fail-closed exclusion, not an assumed value of one.

The mock confirms scalar order smoke tests and adversarial finite-horizon behavior only. No Rust code has been implemented or run. Prior-art includes 2011/2015 Peer global-error control and Constantinescu 2017 coupled-GL estimation. The possible novelty is limited to the heterogeneous transactional switch/history-transfer certificate with original-source residual attestation; it is conjectural.
