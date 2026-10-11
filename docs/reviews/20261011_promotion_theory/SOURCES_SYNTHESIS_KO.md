# VigilODE 이론 승격 연구를 위한 공개 증거 통합

기준 source는 `8ce9bda0d72d308facd615ac42ec137560fd68c6` (`audit/rvj-reaudit-remaining-20261011`)다. 이 메모는 공개 연구의 **재사용 지도와 현재 이식 경계**이며 새 수학 정리의 증명서나 production 승격 판정서가 아니다. 과거 과학 실험은 재실행하지 않았다. `research/LEDGER.jsonl`의 103개 claim 및 supersession을 모두 읽었고, 현재 91개 행이 superseded되지 않았다. 행별 원문 claim·source commit·출력 경로·출력 hash는 `REUSED_RESULTS.json`에 보존했다. PASS는 각 등록 gate 통과를 뜻한다. 모듈의 존재, 증거의 identity, 실험적 유용성, 일반 정리, default 승격, wall time은 별개다.

## 1. 최신 변경을 먼저 반영해야 한다

`4c64317` 감사를 현재 결함 목록으로 그대로 복사하면 틀린다. 그 뒤 `f5a1a7b03af55f2bd986ce439a20a709542a9a43` wave1, `0ce7c3275625dcac46e323dd7eaf17efa961fbdf` CT01, `8ce9bda` remaining 등록이 이어졌다.

| 대상 | 현재 공개 상태 | 후속 이론에서의 해석 |
|---|---|---|
| AS01/F102 | `ProductionFallbackRule` 구현, overflowing norm에서는 exact power-of-two scaling; finite 조건·candidate 검사를 추가. `f0a15e8`, wave1 status | `Inf <= Inf`는 수정됐다. 새로운 미해결 항목은 finite residual의 forward-error 의미와 whole-stage/global 운반이다. |
| AS02/F101 | 정상 intermediate는 기존 bit pattern 보존; 비정상 intermediate는 log-domain predictive factor로 계산. `84335c4` | underflow 반례는 닫혔다. controller의 global-error 증명으로 해석하면 안 된다. |
| AS03/F104 | strict JSON, key/row/arm uniqueness, finite typed exact-dimension state, immutable twin/reference identity로 malformed evidence INVALID. `53b7689`, `b0a5472` | 과거 malformed PASS 경로는 닫혔다. ALG05 item6의 실행 receipt는 여전히 test-name 정적 검색이며 별도의 versioned check가 필요하다. |
| SP01/L-0100 | PASS, `ResidualAccounting::ReuseConfirmed` opt-in. 44/44 driver 및10/10 kernel parity; JVP 감소가 confirmed exit수와 일치. Ir ratio0.962–0.996 | bitwise output 보존 + 불필요한 matvec 제거는 이미 native 결과다. default는9개 기존 counter-contract 때문에 `RecomputeFinal`; 새로운 성능 아이디어로 재발견하지 않는다. |
| SP03/L-0101 | FAIL(item5). 구조 router와 scaled norm fix는 구현. n1000 routed/banded Ir1.0738/1.0347, 제한1.02 초과 | JVP 없는 band validation이 dense J를 만들며 O(n²) 초기비용. 실제 정확한 구조 증명과 O(nb) 검증이 새 결합 지점이다. |
| CT01/L-0103 | L-0102를 wording/input-binding으로 supersede. 40-point grid,6 fresh fixed seeds에서 PASS. vdP frontier0.801–0.863, others worst0.989–1.017, catastrophic0 | fixed-corpus evidence이지 통계 모집단 보증이나 전역정리가 아니다. PREDcap2=PREDcap 모든644 cells이므로 v2-specific corrections의 causal gain은 입증되지 않았다. |
| remaining14nodes | `8ce9bda`에는 PREREGISTRATION만 있다. numeric results 및 새 production code는 없다. | AS04/05,SP02/04,PY01–05,HT01/02,ME01,PC01,EX01은 `PREREGISTERED_NOT_EXECUTED`다. AS06/AS07 역시 완료로 취급하지 않는다. |

직접 근거: `docs/reviews/20261010_accuracy_speed/WAVE1_STATUS.md`, `research/ct01_controller_grid_holdout_20261010/PREREGISTRATION.md`, L-0100/0101/0103. Wave1의 full matrix는 pre-S1 merged tree에서 workspace1007PASS/76ignored, measurement ignored76PASS였고, S1 이후 관련 integrator tests575/633PASS와 clippy/process checks가 재실행됐다는 **공개 기록**이다. 여기서 다시 실행하지 않았다.

## 2. 별도 연구 브랜치를 통합한 범위

`research/rvj-full-history-20261004`의 `6852b4af9ecadfb8c3aa50c4d240301dd1f3a4db`는 main에서 갈라진 archive-index branch다. 직접 읽은 `research/rvj_full_history_20261004/README_KO.md`와 `ARCHIVE_CATALOG.json`은 `INDEX_ONLY_REMOTE__RAW_PAYLOAD_PENDING`이라고 명시한다. 따라서 원본13ZIP/4550보존경로가 그 브랜치에 전부 게시되었다고 주장하지 않는다.

현재 구현 계보에는 이미 아래 자료가 통합되어 있다.

- `research/rvj_integration_20261004/evidence/history_matrix.md`: Loops01–11 및 V01/V02 primary handoff/claim/negative의 한 번 감사 지도.
- `research/rvj_integration_20261004/evidence/archive_audit.md`: Loop08–11의 조건부 정리·실제 source audit; source의 정확한 범위와 API 이식 위험.
- `research/rvj_integration_20261004/inputs/loop11_RESEARCH_NOTE_KO.md`, `loop11_CLAIM_LEDGER.json`, `rvj_independent_loop11_20261004.zip`: retained Loop11 원본.
- `research/rvj_integration_20261004/MATH_EXTENSION_KO.md`: normalized current-operator shifted jet의 native bridge.
- L-0063 이후: 실제 primitive, independent-RHS compression, complex shifts, Fourier client, lognorm, Laguerre·Leja·Taylor 후속 결과.

이전의 source audit가 핵심 정리의 조건부 유효성을 수용했으므로, 이를 다시 전부 증명·실험하지 않았다. source audit의 범위는 모든 nested snapshot/code/raw oracle의 line-by-line 검증이 아니다. 이전 archive push blocker를 현재 구현 branch의 push blocker로 상속하지도 않는다.

## 3. 전체 연구 가족의 유지할 양의 결과와 경계

| 연구 가족 | 재사용할 양의 기반 | 유효한 claim ceiling / 다음 연결 |
|---|---|---|
| v3.5–3.7 transaction / full-E | L-0002 frozen prefix parity, L-0003 bounded continuation completion62+charged exhaustion2, budget breaches0 | descriptive transaction contract. v3.5 L-0001은 source/command 부재 INCONCLUSIVE. 통계 safety 또는 speedup 아님. |
| statistical timing authority | L-0025의 조건부 finite-sample coverage, L-0010의 corrected study | one-case paired timing authority는 FAIL. 고정된 과거 holdout이나 fixed h0 corpus가 iid session을 대신하지 않는다. complexity theorem은 이와 별도로 증명 가능하다. |
| raw RVJ / rational jet / algebraic charts Loops01–03 | classical rational-jet5, constant-rate PR의 RVJ uniform5, H-dissipative complex gain, formal Hensel lift | arbitrary nonlinear/raw-coordinate stiff-uniform5와 reliable embedded estimator는 기존 반례로 배제. 이를 새 no-go로 재탕하지 말고 certified regular chart/physical residual에 제한해 새 theorem을 세운다. |
| closure/slow–fast Loops04–08 | whole-path signed closure, cofactor forcing, exact primitive shear, fast initial impulse, bidirectional comparison box, EXP_CONV_INTERPOLANT | endpoint-only or branch-label-only certificate 부족. regular tube·path token·physical reconstruction·nonzero fast data를 보존한 새 client가 필요. |
| oscillatory/Fourier Loops09–11 | phase equivariance, physical phase budget, Gaussian-rational Fourier–Volterra full product, noncyclic original-target certificate, FFT candidate predictor | known carrier 제거가 unknown frequency ODE 해법은 아님. actual Fourier client PP07은 shifted solve0이므로 jet 삽입은 불필요한 work 생성. |
| native benchmarks/fast drivers | L-0028/29 matched-error comparator fidelity; L-0032 lean driver instructions0.119–0.259x; L-0041 small-n0.393–0.602x | native RODAS/RADAU/CVODE가 실질 비교대상. 과거 diagnostic time을 일반 speed promotion으로 승격하지 않는다. |
| MF K/U residual/driver | L-0037 exact target transport; L-0045 solve_into bitwise parity408solves; L-0062 all68acceptances resolved by budget | stage-target transport와 global LTE를 분리. L-0038 overall FAIL 및 L-0049 uninformative huge reject budgets는 유지. |
| recycling | L-0059 recycle update후 M^-1AU=C refresh가 failure를 제거, L-0066 driver wiring 완료 | projected orthogonality만 고치는 세 접근은 실패. refresh가 cold보다1.08–1.32x products라 이 corpus에서 savings 없음. |
| causal homotopy certificates | L-0035 action-first directed operations0.565x, L-0036 residual-seeded radius one-preflight+one-check; L-0053 structured storage2n vs2n², linear work | L-0053 gate FAIL은 dense comparator slope rule 때문; 양의 구조 identity는 유지. serial certificate가 더 쌌고 HOM06/L-0050 net speed gate는 FAIL. |
| native chart physical provider | L-0043 cofactor chart identity; L-0048 19,652step/dense/output points with global bound and nonzero fast-mode preservation | 이미 one-model global certificate가 있다. 일반 model로 확대할 때 geometry·tube·forward reconstruction authority가 새 의무. |
| shared shifted resolvent | L-0063 current Euclidean-dissipative residual cert; L-0067 selector/count margins49/90; L-0068 rank-compressed RHS16vs256LU solves | d sequential recurrence levels와 full certificate cost 존재. common-γ RODAS에는 factorization이 원래 공유되므로 다중 독립 LU 비교를 적용하지 않는다. |
| complex action | L-0070 H=I native complex gain | Loop02의 gain \|γ\|/Reγ는 기존 증명이며480single+16partial fraction evidence. 일반H metric/physical transport는 별도. |
| nonnormal geometry/lognorm | L-0056 weighted bounds, L-0060 Osborne metric, L-0075 verified negative μ, L-0076 chain symmetrizer | chain model은 큰 개선. binary64 아래로 소멸한 solutions에 relative-only gate를 강제하지 않는다. 일반 nonnormal을 eigenvalues만으로 취급하지 않는다. |
| Laguerre | L-0039 signed output-adjoint exact identity와 large bound improvement; L-0042 cache; L-0047 verified symmetric total admission52cases | already implemented. generic `total_error`는 여전히 EstimateOnly이며 별도 admissible total 사용. scalar \|L_n\| envelope만으로 >1e9total gap을 닫을 수 없다(L-0077). |
| polynomial router | L-0071 verified symmetric fixtures79 중76admit, all Chebyshev, Laguerre0 | 실행 후 두 backend의 비용을 이미 지불. PY03의 plan-only single-backend routing이 의미 있는 다음 축. |
| Leja | L-0080 72/72 actual accuracy, 8cases fewerproducts,58more | EstimateOnly. last-two-term bound는 total certificate가 아니다. favorable niche와 certified divided-difference/roundoff propagation이 필요. |
| fused Taylor | L-0078 bounded54certificates 모두 enclosure, perturbation18power-of-two controls=0 | subnormal absolute-floor 및1e100overflow 해결을 위해 homogeneous RHS normalization(PY04). |
| native banded/fixed-stage/LS cost | L-0054/L-0083 banded O(n) evidence, L-0085 const-stage savings, L-0086 thresholded columnextent, L-0087 exactly11allocations/LS eliminated | symmetric/dense mathematical acceleration과 달리 already useful structural code. Sparse general LU/driverLGMRES caller는 아직 없음. |
| initial guess/batching | SPD07 L-0084 FAIL, default previous는 zero보다1.02–1.57xJVP; SPD08 L-0089 lanes FAIL | zero-start/recycling interaction SP04는 미실행. 같은 lane batch를 theorem만으로 promote하면 안 됨. |
| target/controller/guard ALG01–06 | coupled target own JVP gain0.738gm overProjL2, predictive vdP work improvement, stresscompletion | L-0094 tightnonnormal error4.18x, L-0097 endpoint sensitivity andinactiveG3 유지. CT01는 controller evidence를 넓혔으나 whole-methodcertificate 아님. |
| local inverse gain | L-0099 represented-real2×2 exact Fraction verified30positive,12reject | Wformation/JVP/nonlinear/global excluded. intervals loose up to1.393e15ratio. AS05 is onlyregistered, no core implementation yet. |

이 표의 숫자는 공개 ledger scope 그대로 재사용했다. 새 반복 실험, 현재 전체 suite PASS, 새 timing이라는 뜻이 아니다.

## 4. 이미 있는 수학을 새 정리로 세지 말 것

1. **Finite local algebra와 multiplicity**: Loop11 `C[ξ]/(ξ³−s²ξ)`는 rank3이며 s=0support1이 independent modal amplitudes1을 뜻하지 않는다. 이 구조를 이용한 새로운 certificate compiler/jet batching은 가능하지만 quotient-algebra identity 자체는 기존 결과다.
2. **Normalized shared resolvent**: R0=(I−γ0hJ)^−1, T=R0−I, z=(γ−γ0)/γ0, `(I−γhJ)^−1B=(I−zT)^−1R0B`; dissipativity이면 \|T\|H≤1, ρ<1 tail=ρ^d/(1−ρ). 이미 proof와 native real/complex 구현이 있다.
3. **Laguerre adjoint**: exact finite recurrence error를 signed output adjoint로 표현하는 identity와 Bernstein envelope가 이미 L-0039/L-0047에 있다. 새로운 이론은 degree/work 또는 eps_j(실제 recurrence residual), total bound tightness를 개선해야 한다.
4. **Causal lower-triangular nilpotence**: stage dependency의 inverse finite path-sum, action-first cost, causal box는 L-0035/36에 있다. 새 theorem은 compositional full-path accuracy나 prefix/parallel primitive의 actual cost에 연결해야 한다.
5. **Chart global error**: L-0048은 이미 special semilinear model global physical certificate다. 사용자 요청은 단순 global bound 존재가 아니라 더 넓은 실제 client와 계산 절감 보증으로 확장하는 것이다.
6. **Plain common-W reuse/fast driver**: W/LU 공유, U-form JVP assembly 제거, fixed stages, 11allocation 제거는 이미 구현되어 있다. 새 theorem이 이 savings를 최초 발견인 것처럼 보고하지 않는다.

## 5. 새 양의 정리를 위한 실제 빈틈과 code seams

### A. 근사 역연산자 defect를 이용한 인증과 재사용

AS05는 approximate-inverse route를 명시적으로 `Unavailable(RouteNotImplemented)`로 남긴다. 현재 dense2×2/isolatedblocks/lognorm(n≤8)만 등록되어 있다. 따라서 structured matrix P를 얻어 `||I−PW||≤q<1`를 실제 전체 공간에서 검증하고 `||W^-1r||≤||Pr||/(1−q)` 또는 block/componentwise analog를 제공하는 이론은 새 seam이다. **사전 conditioner의 reuse와 최종 current target W를 분리**해야 한다. q와Pr를 적은 비용으로 인증할 수 있는 structure/locality/low-rank defect 조건까지 정리에 포함해야 계산 감소가 나온다.

매핑: future `crates/rodas5p-core/src/residual_gain.rs`(현재 없음), 기존 `nonnormal_certificate.rs::symmetric_part_upper`, `directed.rs`; driver `rodas5p_matrix_free_fast.rs`; `certified_budget.rs`; AS05등록 후 AS06whole-stage contamination. Jacobian point sampling이나 Arnoldi projected ν를 full-space q로 대신하지 않는다.

### B. 대수적 구조 certificate로 O(n²) router preflight 제거

SP03 `routing.rs::verify_declared_band`는 (t0,y0)의 두 seed directions에 대한 whole-vector tolerance 검사다. regular tube 전체 sparsity/zero-coupling 증명은 아니다. polynomial/rational model expression DAG에 대해 identically-zero off-band Jacobian entries를 symbolic support propagation/AD로 생성하고, denominator·branch의 regularity를 interval로 닫으면 **O(nb) storage/work**의 declared structural path에 모델 identity까지 결속할 수 있다. 이 정리는 dense2probe의 pointwise plausibility보다 강하고, 이미 측정된 O(n²) setup 병목을 직접 없앤다.

매핑: `problem.rs::{ProblemStructure,StructureDeclaration,OdeProblem}`, `routing.rs::{route_problem,verify_declared_band,integrate_rodas5p_routed_observed}`, banded native fill. Dense default와 driver역학은 변경하지 않고 새로운 typed structural witness route로 추가해야 한다. JVP-less callback black-box 전체 identity를 유한샘플로 증명한다는 주장은 불가하다.

### C. Whole-path defect와 dissipative transport를 결합한 accuracy–work theorem

기존 one-model chart certificate, comparison box, current-operator residual witness를 조합해 일반적인 **선언한 class의** full-path defect budget을 세울 수 있다. 목표는 local algebraic error, interpolation defect, coordinate reconstruction, stage residual, floating arithmetic를 모두 원래 physical ODE에 운반하고 안정적인 one-sided Lipschitz/lognorm으로 prefix global bound를 닫는 것이다. 동일 error target에서 exact fast transient/phase carrier를 표현에 넣어 필요한 temporal degree나 step수를 stiffness/frequency와 독립적으로 만들 수 있는 가정이 핵심이다.

매핑: `chart_transport.rs::{certify_reconstruction,chart_dense_point,chart_integrate}`, `outward_certificate.rs::{QuadraticStageProblem,DiagonalStageProblem,certify_stage_target*,blocked_box_certificate_with_execution}`, `fourier_path_certificate.rs::{FourierPath,FourierModel,certificate,commit}`, native sourcebinding. Existing diagnostic `certify_nonlinear_target`/embedded error를 global error bound라고 부르면 안 된다.

### D. Homotopy의 continuation과 endpoint parallelism을 구분

현재 HT01은 declared expensive real client 부재를 예상하며, HT02는 HT01선택 및 supplementaryregistration 없이는 실행하지 않는 계약이다. 기존 HOM06은 ~5×RHS, R-NEXT06은 tinycase negative net margin이다. 따라서 universal homotopy speedup은 자료가 지지하지 않는다. 새 theorem은 (i) endpoint causal system의 구조, (ii) block width/contraction or algebraic elimination degree, (iii) exact target certificate cost, (iv) setup/rejected work/fallback을 명시하고 그 조건에서만 strict work 또는 span inequality를 주어야 한다.

매핑: `homotopy.rs::{PartialCouplingParameters,HomotopyRoundSpec,HomotopyWorkLedger,run_scheduled_homotopy_path}`, `transactional_q1_q2.rs`, `parallel.rs`, `problem.rs::BatchRhsFn`. 현재 stage8개라는 상수가 주는 최대 parallelism도 포함한다. 긴 시간격자 parallel prefix나 multistep batch로 확장하려면 별도 algorithm이며 과거8stage결과로 대체할 수 없다.

### E. Polynomial의 새 성능 정리는 basis 이름보다 spectrum/target 구조에 달린다

PY02는 이미 plan feasibility gate가 있고 prospective Laguerre-vs-Chebyshev degree superiority가 실패할 것으로 예상한다. 요청된 Laguerre exploration을 버릴 필요는 없으나, 기존 compact negative-real symmetric domain에서 coefficient envelope 개선만 반복하는 것보다 **weighted/unbounded spectral domain, rationally transformed operator, structured resolvent defect, repeat-RHS amortization** 등 새 mathematical regime를 선언해야 한다. Laguerre/Leja의 cutoffdegree+roundoffcertificate+setupcost를 포함한 bestbaseline inequality가 필요하다.

매핑: `polynomial_action.rs::{SymmetricNonpositiveOperator,JointPhiReport,TotalErrorAdmission,joint_phi_action}`, `laguerre_adjoint.rs::{EnvelopeKey,LaguerreEnvelopeCache,laguerre_adjoint_recurrence_bound}`, `shared_shift_jet.rs`, `complex_shift_jet.rs`, `taylor_phi_total.rs`. PY03plan/execute분리와 cache proofversion을 재사용한다. q<1rational preconditioning을 polynomial convergence bound와 묶으면 현재 미구현 AS05 route 및 PC01에 동시에 연결된다.

## 6. 승격 목표의 분리

- **정리 승격**: 명확한 class/assumptions에서 bound·complexity·span theorem을 완전 증명하고 독립 검토를 받는다. 이는 이번 theory request로 수행할 수 있다.
- **구현 승격**: 정리 assumptions가 실제 API 타입·현재target·outward arithmetic·resource accounting에 강제됨을 확인해야 한다. theory alone은 이 사실을 만들어내지 않는다.
- **전역 정확도 승격**: full-path residual + initialdata + regular tube + physicalnorm + stability + accumulated numerical error가 필요하다. 새 bound가 embedded scalar보다 강해도 candidate모든실행을 자동보증하지 않는다.
- **속도 승격**: exact counted-complexity reduction과 wall time을 구분한다. theorem은 조건부 operation/span절감을 증명할 수 있다. 실제 구현의 constants/cache/setup/dispatch에 대한 native evidence와 별도 timing authority 없이는 seconds/speedup을 주장하지 않는다.

건설적인 우선순위는 **구조 certificate 및 inverse-defect 재사용 → whole-path/global bound → 그 bound로 허용되는 coarser work → strict total-cost inequality → 코드 계약에 맞춘 prospectivestudy**다. 존재하는 safe primitive와 공개 positive work result를 활용하되, 범위가 겹치는 기존 정리를 새 발견으로 세지 않는 것이 이번 확장의 핵심이다.

## 7. 읽기·보존 범위

읽은 지침: harness research `PROJECT_INSTRUCTIONS.md`, repo `docs/RESEARCH_LEDGER.md`; tracked `AGENTS.md`는 검색에서 없었다. Ledger family map과 source/API seam은 읽기 작업이며 원래 repo 파일을 수정하지 않았다. 각 attachment는 BASS/CMB project title 및 Rust/runtime자료로 제시되어 있어, VigilODE 이론의 authority로 자동 적용하지 않았다. 별도 BASS theorem/protocol을 이 repo의 보호된 invariants로 끌어오지 않았다.

`REUSED_RESULTS.json`은 103개 공개 row 전체와 supersession/identity를 보존한다. 이것은 **재실행 receipt가 아니다**. source hashes는 identity일 뿐 theorem proof나 performance authority가 아니다. 이번 synthesis 자체로 production/global/speed의 PROMOTE 판정을 내리지 않는다.
