# 제외 후보의 재검토: 방법 이름에서 계산 구조로

2026-10-24. 기존 25개 solver family, 2026-10-08의 16개 세부 후보, 별도로 묻혀 있던 16개 대수·기하·인증 기법을 다시 분류했다. 이 문서는 후보 범위와 구성적 복구 계약이다. 실제 수정·실행 결과는 이번 연구의 통합 검증 문서가 담당한다. 여기서 과거 실험을 새 결과로 세지 않는다.

핵심 판단은 **제외 사유의 적용 범위를 좁히고 살아 있는 계산 구조를 다른 역할로 옮기는 것**이다. Cheap-JVP에서 진 shared-space는 expensive-JVP·작은 고정 rank에서 다시 시험할 수 있다. 비수축 Neumann의 실패는 작은 polynomial defect의 반복 보정을 배제하지 않는다. 작은 문제에서 느렸던 stage parallelism은 expensive RHS의 span 문제로 남는다. 반대로 형식만 바꾼 동일 연산이나 가장 강한 직접 풀이보다 비싼 구조는 속도 성과로 재명명하지 않는다.

## 실제 열람과 범위

GitHub의 모든 PR 81개 metadata를 한 페이지로 조회했고, 11개 주제 검색과 PR #50, #60, #62, #65, #68, #69, #70, #74, #75, #77, #79, #81의 상세 조회를 수행했다. 최신 관측 PR #81의 head는 `cc49cfd1017fc0639bf9ef6cc08535ff5a06ff7e`다. 내용은 application-specific norm enclosure이며 일반 solver core 변경으로 취급하지 않았다. 전체 81개 diff·댓글을 모두 재감사한 것은 아니다. 원격 원문은 `../evidence/pr_inventory.json`, `pr_search.json`, `pr_details.json`에 보존했다.

기준은 local `3079c62713aefa62ad2cf7fbf6d2daf60cc0a0d8`, 동일 tree를 가진 published `14171ed0c4f7dc453f47870cf6c1399f41afc4f2`다. 아래 식들은 알려진 항등식 또는 저장소의 기존 유도를 재사용·연결하며, 학술적 최초 발견을 주장하지 않는다.

## 25개 family를 다시 여는 조건

| ID | 이번에 살릴 역할 | 적용·전환 조건 |
|---|---|---|
| EXPLICIT | RHS-only explicit integration is the genuine fully derivative-free general smooth branch. It also supplies a cheap control against speculative structure. | Use existing persistent session, reset FSAL only on state/model changes; rising rejection/work triggers a bounded optional implicit pilot. |
| RODAS_DENSE | Dense protected solves remain valuable for small dimensions and difficult nonnormality. A special candidate can increase robustness even if this fallback remains the fastest broad route. | Use as charged fallback when a structural certificate, residual, projected closure or finite check fails. Fatal callback errors stay fatal. |
| RODAS_BANDED | A band declaration can be supplied by a verified local expression graph instead of repeatedly checking a dense reference. | Epoch-bound support proof selects band mode; unknown callback structure keeps prior probing/fallback. |
| RODAS_MATRIX_FREE | Krylov is useful fallback where a low-degree structural action does not fit. Bounded shared spaces and polynomial preconditioners can lower failure cost. | Use measured action cost, successful true residual and bounded restart budget; terminate speculative preconditioner on deterioration. |
| RODAS_SMALL | Compile-time arrays can improve allocation and latency without changing mathematical method. Reconsider size crossover as hardware dependent. | Caller-declared dimension selects specialization after a measured crossover; resource cap independent of speed evidence. |
| SWITCHING | Specialization should be admitted by eligibility and measured charged progress, with bounded probing and cooldown after failure. Stability and completion are first-class benefits. | Combine declared structure, operational residual, current workload counters, hysteresis and maximum speculation debt. Re-evaluate only after model/scale/regime changes. |
| LAWSON_GENERIC | Generic Lawson should be judged by residual smoothness after factoring a known fast flow, not solely by oscillation of the full RHS. | Use flow declaration plus short charged trials and same physical error norm; fall back when residual variation or phase error rises. |
| LAWSON_AFFINE | Exact affine/harmonic actions are a central algebraic specialization, not merely a generic RK wrapper. Extend stable confluent functions before broadening inferred structure. | Admission by exact model contract, frequency/damping and forecasted effort; slow/zero cases can route to simpler exact or explicit branch. |
| COUPLED_FORWARD | Causal elimination is a viable custom root engine. Its failure on broad spectra motivates bounded dimension and early exit, not discarding causality. | Initial low-dimensional attempt; full original residual; bounded expansion then cooldown/fallback. Reuse only with epoch-bound valid operator images. |
| COUPLED_PICARD | Cheap-RHS losses do not settle expensive independent stage callbacks or accelerator batching. Restrict the question to measured span crossover. | Require explicit workers and cost model predicting q·wave_cost+barriers<serial_cost; cap q and fall back to forward. |
| SABR | The block formulation may contribute a preconditioner or finite causal action even when its complete driver loses. | Use triangular or block solve when estimated stage coupling is weak; generic block GMRES only when batched actions amortize its enlarged vector size. |
| HOMOTOPY | Use homotopy to organise difficult root charts; directly eliminate causal/low-degree roots where continuation is unnecessary. | Try bounded derivative-free algebraic chart first when declared; original residual; chart change or permitted corrector only on failure. |
| TRANSACTIONAL | Rollback/counter preservation is the enabling mechanism for safe adaptive rescue and robustness, even without standalone speedup. | Bound trial work, preserve failure counters, blacklist/cooldown rejected route; do not mask fatal malformed callback. |
| EXPONENTIAL_KRYLOV | Large known linear dynamics and inexpensive nonlinear residual can favour phi actions. Algebraic quotient actions can replace Krylov where relation is known. | Compare predicted total basis/transform cost to protected solve; no free reuse unless same operator, RHS span and unfused/form-specific assumptions hold. |
| LAGUERRE | Preserve Laguerre coefficients and signed-adjoint analysis; move recurrence into a small quotient algebra so its long vector recurrence disappears. | Cost/denominator/defect admission selects compressed action; otherwise strongest already-supported action. |
| LEJA | Legacy EstimateOnly and current EnclosedLeja are distinct APIs. Reuse the enclosed component and reopen different geometry only with a corresponding valid enclosure. | Reuse supported symmetric-domain private admission; complex or nonnormal extension needs a new bound before adaptive use. |
| POLYNOMIAL_ROUTER | PY03 already executes a planned recurrence and A08 has FirstAdmitted. Reuse those and add quotient-aware forecasting and setup amortization. | Current FirstAdmitted remains the baseline; structural forecasting and bounded retries are the new hypotheses. |
| LOW_DEGREE_RESOLVENT | The existing one-shot quotient can become a convergent correction engine when the approximate annihilator is small even if ||B|| is large. | Choose bounded correction count from η and tolerance; true residual each check; reject stale witness/failed estimate; bounded fallback. |
| NEUMANN | Noncontractive B rules out only that infinite-series argument. A nilpotent or polynomial-preconditioned defect can be small or terminate. | Check declared finite degree/defect before choosing recurrence; enforce work cap and true residual. |
| PACKET_COLLECTIVE | Separate generic invariant/low-rank algebra from the application-specific sealed model. Exact low-rank structure can accelerate arbitrary relevant ODE clients. | Declared structural identity and physical-lift error norm; reject closure leakage or rank change, retain full-space fallback. |
| REFERENCE_BDF_RADAU | Low-order implicit methods are useful stiff-start or nonsmooth-segment fallbacks even without matching order-five efficiency. | Use only a declared startup/discontinuity rescue with an allocated error budget and return to high order after regularity recovery. |
| INDEPENDENT_ENSEMBLE | Even one worker can benefit from structure-of-arrays vectorization of independent trajectories; threads are not required. | Batch only compatible dimensions/parameter groups; split divergent histories; worker budget explicit. |
| PARALLEL_STAGE_THEORY | Parallelism can target stage blocks, spatial causal scans or independent algebraic factors separately. Use the axis with genuine parallel work. | Select parallel route only if measured callback/scan sizes amortize barriers; serial route remains default on tiny systems. |
| VERIFIED_ERROR | Tighter dissipative/metric error transport can allow larger steps or safe inexact solves at the same physical accuracy target. | Use optional validated provider; ambiguous numerical norm returns indeterminate/retry rather than accepted zero. |
| GRAPH_SUBFLOW | Graph diffusion/Markov semigroups are ordinary structured ODE operators and need not remain excluded merely because prior client was PDE-specific. | Admit on verified generator/contractivity; nonsymmetric advection or wrong boundary returns to general route. |

각 family의 이전 이유·근거, 계산식, 판별 실험은 JSON에 개별 기록했다. 선택된 네 baseline도 대조군으로 포함하여 특수 경로가 약한 baseline만 이기는 문제를 피한다.

## 고유한 대수적 기법과 코드 연결

### ALG-PRIMARY-JETS — Primary decomposition and confluent finite jets

For q=∏(x−λ_i)^m_i, coprime CRT gives K[x]/q ≅ ∏K[ε]/ε^m_i. Preserve each derivative jet through root coalescence.

Scalar coefficient work may be shared; it does not delete independent amplitudes. Compute stable entire/divided-difference limits instead of subtracting close modal values.

**전환 계약.** Switch from separated CRT to cluster/confluent evaluation when coefficient conditioning is poor; recompute original operator residual; preserve multiplicities.

**판별 예제.** Distinct/near-coincident/exactly-coincident roots with Jordan blocks; compare stable sinhc/expm1 formulas.

**근거와 상태.** `docs/reviews/20261011_promotion_theory/POLYNOMIAL_KO.md`; historical theorem reused; new general compiler not implemented by inventory.

### ALG-INVOLUTION — Involution-even/odd computation sharing

If S²=I and SAS=−A, even powers commute withS and odd powers anticommute; Π±=(I±S)/2 retain both sectors.

Evaluate common even coefficient functions once and recover odd signs; symmetry reduces duplicate transcendental work rather than deleting odd state.

**전환 계약.** Require declared symmetry identity and parameter compatibility; model changes invalidate grouping.

**판별 예제.** Matched ±frequency blocks with arbitrary amplitudes, broken-symmetry negative control.

**근거와 상태.** `research/rvj_integration_20261004/inputs/loop11_RESEARCH_NOTE_KO.md`; historical theorem reused; generic grouping extension proposed.

### ALG-AFFINE-MANIFOLD — Exact invariant affine manifold reduction

f(y_ref+Uz)=Ug(z), JU=UJ_r imply full causal stages K_i=Uk_i^r by induction with common invertible W.

Replace n-factorization by r-factorization while original target equality holds; lift and physical norm still cost O(nr).

**전환 계약.** Bound closure and reconstruction defects; keep nonzero transverse initial data in a separate proved evolution or refuse reduction.

**판별 예제.** Invariant and nearly-invariant families, ill-conditioned U, rank drift and physical error norm.

**근거와 상태.** `docs/reviews/20261011_promotion_theory/HOMOTOPY_KO.md`; historical derived theorem, scoped native pieces exist.

### ALG-LOCAL-IDEAL — Local reaction ideals and rationalized roots

Eliminate u from u+x=Q and x−x0−a u(1−x)=0; physical root x=2(x0+aQ)/(1+a(Q+1)+sqrt(D)), D≥1 under u0≥0,0≤x0≤1,a≥0.

Nonlinear local resolvent without runtime derivative or Newton; use as predictor/preconditioner in high-order target.

**전환 계약.** Check denominator/domain/physical interval and original balance; fall back near failed conditions; never inherit order5 from backward Euler.

**판별 예제.** Large a, depleted species, cancellation and overflow controls; full target accuracy after correction.

**근거와 상태.** `research/cumulative_20261020/original_sources/derivative_free/vigilode_rhs_only_research/REPORT_KO.md`; historical formula and24probes reused; wider generic source generation proposed.

### ALG-REGULAR-CHART — Rational jets, chart/cofactor transport and fast initial impulse

For y=x²(w+1/κ), reconstruction error is bounded by (2|x|r_x+r_x²)|w+1/κ|+(|x|+r_x)²r_w plus represented-center error.

Resolve stiff transverse dynamics analytically in a regular chart and retain nonzero fast data; physical defect certifies useful accuracy.

**전환 계약.** Whole error tube must remain away from chart denominator zero; chart change or original-coordinate fallback before boundary.

**판별 예제.** Approach x=0, nonzero fast impulse, εforcing, signed branches and reconstruction cancellation.

**근거와 상태.** `crates/rodas5p-integrators/src/chart_transport.rs`; existing one-model provider; generic chart compiler remains open.

### ALG-PARAMETER-CELL — Semialgebraic inverse-witness reuse

E(θ)=I−PW(θ)=Σβ_α(θ)E_α with Bernstein β≥0,Σβ=1; max||E_α||≤q<1 gives ||W^-1 r||≤||Pr||/(1−q).

A proof over a cell replaces repeated full inverse verification by membership and identity checks; total work reduced when N(C_full−C_member)>C_compile+C_extra.

**전환 계약.** Bind model/metric/preconditioner/epoch and interval parameter containment; outside cell choose another validated cell or fallback.

**판별 예제.** Cell edges, crossed boundary, coefficient corruption and changing metric; compile/replay/membership cost separated.

**근거와 상태.** `docs/reviews/20261011_promotion_theory/CERTIFICATE_ATLAS_KO.md`; existing derived theorem; no current arbitrary-RHS cell compiler.

### ALG-SUPPORT-GRAPH — Generated row-local support proofs

Variable-support union along a differentiable expression graph proves ∂f_i/∂y_j=0 for absent variables.

For bounded local width and graph length, whole-domain structural setup O(nb) replaces dense O(n²) verification.

**전환 계약.** Only a sealed/generated RHS shares certificate; arbitrary callback hash or finite probes do not certify identity.

**판별 예제.** Hidden nonlocal term, rational denominator, mesh epoch, shared-graph traversal complexity.

**근거와 상태.** `docs/reviews/20261011_promotion_theory/CERTIFICATE_ATLAS_KO.md`; source-generation opportunity, not invented universal linear-time AD.

### ALG-SCHUR-NILPOTENT — Band-border Schur elimination as finite algebra

W=[[A,B],[C,D]], S=D−CA^-1B, P=[[A,0],[C,S]] imply P^-1W=[[I,A^-1B],[0,I]], (P^-1W−I)²=0.

Exact preconditioning closes GMRES in at most2iterations; direct block elimination is the equally strong comparator.

**전환 계약.** Use exact/controlled block solves; inexact A/S changes nilpotence and needs actual residual/defect test, not a promised3–5iterations.

**판별 예제.** Border rank scaling, approximate Schur, nonnormal block, setup/reuse and strongest direct solver.

**근거와 상태.** `research/cumulative_20261020/original_sources/derivative_free/vigilode_rhs_only_research/REPORT_KO.md`; historical identity and scoped tests reused.

### ALG-SHARED-SHIFT — Shared normalised real/complex shifted jets and RHS rank

R0=(I−γ0hJ)^-1,T=R0−I,z=(γ−γ0)/γ0; (I−γhJ)^-1B=(I−zT)^-1R0B. For dissipative J,||T||≤1 and tail≤ρ^d||R0B||/(1−ρ).

One centerfactor, d·r RHSsolves for many genuinely distinct shifts; bound RHS compression defect separately.

**전환 계약.** Require actual multi-shift workload,ρ<1,current residual, resource cap; gamma shared by standardRODAS yields no new factor saving.

**판별 예제.** Clustered/wide shifts, rank1/fullrank RHS, complex nearimaginary shifts; compare Schur/multishift methods.

**근거와 상태.** `crates/rodas5p-core/src/shared_shift_jet.rs`; native kernels present; generic trajectory client open.

### ALG-INFLUENCE — Whole-stage influence and optimal residual allocation

With causal transfer L, w=(I−L)^−T|b|. For residual model R_i q_i^k and cost c_i, continuous optimum ρ_i=min(R_i,p_i/(λa_i)),p_i=c_i/(−log q_i).

Spend correction work on influential stages; conserve the same final output budget; rounding integercounts up preserves feasibility.

**전환 계약.** Need valid current influence/contractivity/tube bounds and actual costmodel; tentative estimates remain estimates.

**판별 예제.** Anisotropic stage sensitivity, nonnormal transport, equality of final errorbudget and all allocator overhead.

**근거와 상태.** `docs/reviews/20261011_promotion_theory/GLOBAL_ACCURACY_KO.md`; historical constructive theorem; new backend integration candidate.

### ALG-CAUSAL-PREFIX — Parallel affine error/transport scan

Maps(g,d):e↦ge+d compose as(g2g1,d2+g2d1); exact associative structure permits prefix tree.

O(N)work/O(log N)idealspan for prefix transport; outward composition encloses its own rounding.

**전환 계약.** Use for independently known local maps, never claim unknown future nonlinear IVP states become independent.

**판별 예제.** Long chain, huge/small gains, serialscan comparator and parallel setup threshold.

**근거와 상태.** `crates/rodas5p-integrators/src/affine_transport.rs`; already native supportkernel.

### ALG-ONE-SIDED — Dissipative geometry and weighted metrics

For incidenceB andK≥0,L=−M^-1BKBᵀ, eᵀML e≤0; two-point monotonicity similarly treats monotone nonlinear flux.

Avoid stiff negative eigenvalues becoming spurious exponential errorgrowth; stronger robustness/accuracy can reduce over-resolution.

**전환 계약.** Metric positive and bound valid on fulltube; eigenvalue sign alone cannot replace numerical-range condition.

**판별 예제.** Nonnormal coordinate changes, graphweights, near-singular metric and mixedexponent norms.

**근거와 상태.** `research/cumulative_20261020/original_sources/derivative_free/vigilode_rhs_only_research/REPORT_KO.md`; generic positive construction reusable from application sources.

### ALG-FOURIER-VOLTERRA — Fourier carriers and noncyclic nonlinear product

Known carrier e^{iωt} can be integrated analytically while full polynomial/Fourier convolution retains coupled amplitudes.

FFT candidate prediction and exact/validated original-target residual separate cheapgeneration from acceptance; no aliasing as closure.

**전환 계약.** Use declared bandwidth/closure/tube and phase budget; enlarge representation on residualfailure.

**판별 예제.** Rapidoscillation, resonant nonlinear modecoupling and aliasingnegativecontrol; direct moments as strongbaseline.

**근거와 상태.** `research/rvj_integration_20261004/inputs/loop11_RESEARCH_NOTE_KO.md`; historical Fourierclient; not generic unknownfrequency inference.

### ALG-RANK-STRATA — Rank/degree changes as switching boundaries

A monic degree-d quotient retains dimensiond when roots coalesce; parameter vanishing can cause a genuine degree drop only after identity verification.

Stable exact exceptional formulas at known degree-drop strata can avoid numerical underflow/folds; arbitrary threshold rankdiscard is unsafe.

**전환 계약.** Distinguish exactzero coefficient branches from nearzero perturbative branches with residualbudget; hysteresis applies only to performance choice.

**판별 예제.** Zero/nearzero frequency, vanishedquadraticcoefficient, nearlydependent modes, trace preservation.

**근거와 상태.** `research/rvj_integration_20261004/inputs/loop11_RESEARCH_NOTE_KO.md`; constructive synthesis of existing finitealgebra and quadraticbranch code.

### ALG-NEAR-ANNIHILATOR — Approximate annihilator correction rather than one-shot rejection

P from q(1)−q(z)=(1−z)r(z) gives residualoperatorD=q(B)/q(1); correction produces r_k=D^k r0 in exactarithmetic.

Repeatedshortpolynomial action can succeed when one-shot tolerancefails, even with large||B||; fixedη<1 predicts a boundedbudget.

**전환 계약.** True current residual check, no statecommit onfailure, finiteworkcap and denominator margin; finiteprecision floor handled separately.

**판별 예제.** Nonnormalnilpotentplusleakage,η→1,largeactions and stale provider; compare iterative/direct protectedbaseline.

**근거와 상태.** `crates/rodas5p-integrators/src/polynomial_resolvent.rs`; new implementation direction for sibling work; identity is consequence of priorquotienttheorem.

### ALG-PHYSICAL-NORM — Affine physical norm and exact accumulation

For y=yref+Uz, measure e in original physical scale ||D U e|| rather than compact Euclidean coordinates; outwardlower/upper bounds can represent cancellation safely.

Robust decisions near toleranceboundary, avoid overflow/underflow in mixedexponents without full liftedvector allocation.

**전환 계약.** Fixed model/lift/scale and denominator; ambiguous enclosure is Indeterminate rather than falseaccept.

**판별 예제.** Mixedsigncancellation,subnormal products,normboundary,platform portability; no BASS-specific code merged into generalcore.

**근거와 상태.** `research/candidate_reconsideration_20261024/evidence/pr_details.json`; newly observedPR81 is client-specific22files; generic lesson only.

## 이전 세부 후보 16개를 빠뜨리지 않는 대응

| 후보 | 이전 기록 | 새 판별 질문 |
|---|---|---|
| KRY-PROJ-STOP | PROMOTE_TO_PROBE | Recheck only changed stopping/norm dependencies; pair early exit with outer-tolerance consistent targets and true residual. Historical useful mechanism retained, not relaunched as novelty. |
| INO-FORCE-ABS | PROMOTE_TO_PROBE | Replace uniform forcing with influence-aware budgets where verified bounds exist; preserve tight-tolerance stage/output fidelity and no loose fallback target. |
| CERT-OSA-STALL | PROMOTE_TO_PROBE | Use validated current residual-to-output bound as safe inexactness service; add nonlinear remainder before claiming full root error. Compare stall acceptance to simply adequate Krylov budgets. |
| KRY-SHARED-SPACE | CLOSE | Bound rank independently of n and admit only expensive-action or few-mode cases; charge orthogonalization and refresh. Old near-fullrank cheap-JVP failure does not exclude bounded lowrank. |
| PC-SECANT-WOODBURY | CLOSE | Reopen expensive-JVP, small fixed memory and singleoperator epoch case; true-residual admission, no fullrank SVD. Derivative-free structure provider can be cheaper than learned secants. |
| PC-LAGGED-HWINDOW | HOLD_NEEDS_RESOLUTION | Reopen 2D/3Dfill-dominated or expensive factorization regimes using exact current target and lagged preconditioner only; strongest sparse/band comparator mandatory. |
| KRY-BUDGET-PREDICT | PROMOTE_TO_PROBE | Separate baseline budget correction from predictor benefit; plateau alone is not convergence impossibility. Use hardworkcaps and recoverable coldrestart. |
| SOLVE-LIMITED-SWITCH | CLOSE | Route on measured progress/cost plus eligibility, not old maxit-induced failurecount or Ritzangle alone; distinguish malformedcallback from numerical difficulty. |
| CTRL-PRED-CAP | PROMOTE_TO_PROBE | Reuse current predictive control evidence; isolate any new cap effect against unchanged predictor and simple safetyfactor controls. |
| CTRL-EXPANSIVE-GATE | HOLD_NEEDS_RESOLUTION | Use verified/logged physical amplification as diagnostic or conservative safeguard; no claim expansionphase selective when thresholdfires broadly. |
| CTRL-NULLS | PROMOTE_TO_PROBE | Reopen only a changed failure mechanism; catastropheabort can save bounded discarded work with unchanged acceptance, while controllerchanges require fresh matchedaccuracy. |
| DENSE-OWNW | PROMOTE_TO_PROBE | Use step-factor reuse to reduce setup for denseerror estimator; validate nonnormal underestimation and total retained-griderror before permitting wideroutputsteps. |
| GE-AMP-DETECT | PROMOTE_TO_PROBE | Replace biased res5transport by method-consistent or validated affineprefixbound; stablecounterexample controls required. Detection neednot claim certification. |
| REG-EXPLICIT-TRIAGE | HOLD_NEEDS_RESOLUTION | Use declared dissipative generator or conservative stability region plus charged pilot; a Ritz cloud is insufficient for nonnormal stability. Compare banddirect. |
| EXPRB-OWN-H | HOLD_NEEDS_RESOLUTION | Reopen declared linearpart/structuredquotient phi where actual actioncost is low; avoid invalid reuse across preconditioning/fusion changes. |
| KLEGAL-ROK4-GATED | CLOSE | Preserve separate ROKorderconditions and exact targetresidual. Reopen only corrected method/tableau/space construction; do not reuse the failed claim that Klegality alone protects accuracy. |

## 우선 구현할 연결

1. **몫다항식 보정.** 기존 one-shot resolvent가 residual 허용치를 못 맞출 때, 검증 가능한 작은 annihilator defect를 반복 보정해 복구한다. 원래 연산자 norm이 큰 nonnormal 문제도 포함할 수 있다. 실패·한계 사례와 직접 structured solve를 함께 센다.
2. **인과적 축소 단계의 전환.** 작은 basis의 causal forward를 제한된 예산으로 시험하고, 원공간 residual·비용·실패 이력으로 확장 또는 보호 경로를 선택한다. 전체 미분을 먼저 쓰는 것과 자동으로 동일시하지 않는다.
3. **안정적인 algebraic chart.** Degree drop·근의 합체·작은 frequency에서 정확한 exceptional formula와 entire-function limit를 사용한다. 실제 rank·독립 amplitude를 임의로 삭제하지 않는다.
4. **정확도와 강건성의 이득.** 구조적 one-sided bound, inverse-cell witness, physical error norm 및 영향도별 residual 배분으로 과도한 계산과 잘못된 수락을 줄인다. 이 이득은 같은 시간에 더 작은 오차 또는 더 적은 실패로도 평가한다.
5. **병렬화의 적절한 축.** Stage wavefront, 독립 IVP, 공간 causal scan, independent primary factors, shared-shift evaluation을 분리해 측정한다. 각각의 span과 work를 혼동하지 않는다.

## 최대 세 번의 수정 루프에 사용할 공통 계약

첫 루프는 선언된 구조와 안전한 거절·원문제 residual을 구현한다. 둘째 루프는 반례나 실제 실패 기작에 따라 수정하고 해당 의존성만 재실행한다. 셋째 루프는 독립 검토에서 남은 구체적 오류와 비용 병목을 처리한다. 변화 없는 실패를 반복해 성공 표본만 고르지 않는다. 각 루프에는 source identity, raw failure, 변경 이유, 수행 명령, 수용·거절 결과를 보존한다.

Adaptive selector는 현재와 과거의 관측 정보만 쓴다. 원래 상태·norm·모델 epoch가 달라지면 speculative cache를 무효화한다. NaN, 잘못된 callback, 사용자 권한 부족을 단순 수치 실패로 처리하여 우회하지 않는다. 통계적 성공 확률을 입증하지 않은 경험적 score는 성능 heuristic으로 명시한다.

## 남은 범위

모든 후보를 다시 고려하는 것과 모든 후보를 이번에 native 구현·벤치마크하는 것은 구별한다. 이 inventory의 모든 항목에는 구체적 복구 역할, 적용 조건, 다음 판별 실험이 있다. 통합 보고서는 이번에 실제 실행한 경로만 새 성능 결과로 표시해야 한다. 전 PR의 전체 diff, 모든 과거 원자료, 원전의 완전한 신규성 조사는 이 inventory의 완료 주장에 포함하지 않는다.

## 이후 이력으로 교정한 과거 상태

- **HIST-CORR-1** PY03 single-backend planning was subsequently implemented. L-0110 items1–4 pass; only winning Laguerre niche failed. Current A08 also has FirstAdmitted and CompareAll. New work must add a mechanism beyond those existing paths.
- **HIST-CORR-2** Current core/polynomial_router.rs calls leja_phi_action_enclosed_with_workspace and consumes a private fused admission token. Retain scope distinction between legacy estimated action, enclosed component action and general ODE accuracy.
- **HIST-CORR-3** L-0106 records later zero-start/recycling experiment: one E-normal1e-8 niche improved countedC, while broadnotwin and nonnormalfailures remain. Reopen current operator-valid boundedrank/cost scope, not assume allrecycling untested.
- **HIST-CORR-4** L-0113 found no declaredexpensiveclient and L-0114didnotrunparallelpilot. Those rows are lackofapplicableclient, not a mathematical rejection of parallelhomotopy.
- **HIST-CORR-5** L-0117 native sparse/right-preconditioner groundwork exists but referencebudgetblocked stage1; L-0141laterstaleness failedHIRES factorizationthreshold yet passedBrusselator100. Reopening must target certifiednewreusemechanism and fill-dominatedregime.

이 교정은 과거 유효한 실패를 지우지 않고 이후 구현을 같은 경로의 미완료로 오인하지 않게 한다. 현재 ledger의 166개 metadata/claim 요약을 확인했고 위 항목은 해당 원문 claim을 직접 대조했다. 모든 166개 과학 payload를 다시 실행하거나 전부 재증명하지 않았다.

## 공통 수용 기준

모든 pending 후보의 JSON에는 고정 physical error cap E와 reference allowance, 성공·거절·수치 실패·잘못된 입력의 구분, setup 및 실패 비용을 포함한 paired median, causal switching 및 최대 3회 수정 예산을 넣었다. 1.2배 시간비는 제안된 공학적 선택 기준이며 이 inventory가 달성한 결과나 수학 정리가 아니다. 정확도·완료율 개선도 별도 유효한 성과로 평가한다.
