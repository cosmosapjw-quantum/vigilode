# VigilODE NCP 재감사와 production/FVM 후속 연구

2026-10-21 · 기준 게시물 `7a63876a97fc17f640771ec06658269947049994` · NCP numerical source `de4c10a9f41321beef2313a3d7fb503bab35d714`.

## 판정

**범용 production solver 승격은 HOLD다.** 그러나 이전 결과의 전면 실패 해석은 교정해야 한다. 회전계 tolerance sweep은 0/140 통과가 아니라 **7/140 통과**이며, VigilODE의 내부 `rtol=1e-10` 7회가 저장된 정확도 기준을 만족했다. 이번에는 판정 오류를 바로잡는 데 그치지 않고, 불필요한 Jacobian 계산을 제거하는 opt-in adapter, band/border Schur 전처리기, 보존형 remap 커널, BASS/rei용 물리 FVM oracle를 구현했다.

새 코드는 변경 의존성 검증 9개 명령을 모두 exit0으로 마쳤다. Rust 테스트19개(core4, Krylov1, adapter6, remap8), Python 테스트15개(물리12, 회전 진단3)가 통과했다. 제조해 선형계3개와 기존 endpoint 재계산도 수행했다. 검증 전후 소스484파일의 해시가 같다. 이것은 **구현된 계약의 PASS_SCOPED**다. 새 NCP 성능 campaign, 전체 workspace 재실행, live BASS/rei Rust 통합은 수행하지 않았다.

| 대상 | 현재 결정 | 근거와 제한 |
|---|---|---|
| NCP 산출물 identity | 채택 | 759개 열거 파일 hash 일치; manifest 포함760개 |
| 기존 report/일부 plot | 교정 후 재사용 | reference time, timing scope, decision tolerance, VODE 설정 의미 수정 |
| VigilODE 회전계 | 좁은 설정의 정확도 근거 복원 | `rtol=1e-10`, 기존41개 grid의 기록된 통과7회; native full trajectory bytes는 없어 endpoint만 독립 재계산 |
| Analytic heat Jacobian | optimization_possible, opt-in 구현 채택 | Jacobian마다 n회 부가 RHS 제거; dense 저장/LU는 유지 |
| Bordered Schur kernel | specialized_for_specific_cases_only | band phase block의 strict row dominance 및 가역 Schur, 실제 residual 검사 |
| Conservative remap kernel | specialized_for_specific_cases_only | 동일 domain, 양의 cell integral, 선택한 measure에서 1차 재구성 |
| BASS/rei 물리 FVM | prototype 채택 | 비기울어진 LRS Bianchi I, scalar/unpolarized, cold elastic Thomson |
| 전체 production/global accuracy/speed | HOLD | 지원 범위별 reference·driver 연결·전체오차·통제된 성능 근거가 남음 |

## 1. NCP 결과의 적대적 감사

자세한 재계산은 `NCP_AUDIT.md/json`, 실행 코드는 `audit_ncp_artifacts.py`다. 원본 `external_audits/...`는 수정하지 않았다. measurement attempts1070 및 오류42개를 보존한다. 과거 원시 파일에 없는 trajectory를 만들어 채우지 않았다.

핵심 교정은 다음과 같다.

1. **회전계 통과 누락:** native `rtol=1e-10`의 scaled error는 약0.387683이다. `rtol=1e-6`에서 약5156.6이라는 실패도 유지한다. 고정 application budget을 지키도록 internal tolerance를 강화하는 경로가 있다. 두 tolerance를 schema에서 분리해야 한다.
2. **틀린 출력 시각:** Julia throughput rotation에 실제 `2πi/40` 대신 `0.01i`를 대입했다. 저장 상태를 정확한 시각으로 평가하면 약35,126,829에서7,123으로 줄지만 실패 판정은 같다.
3. **비교 시간 범위 불일치:** native process wall과 Julia/diffsol warmed solve를 한 work-precision plot에 섞었다. native 회전계 tight run의 중앙값5.559548ms는 process wall이며 내부 timed solve는1.515231ms다. 이 교정만으로 paired timing 또는 host 통제 부족을 해소하지는 않는다.
4. **VODE 비교 설정:** 게시 wrapper는 Jacobian 없는 functional iteration BDF(MF20)다. stiff chord/Newton BDF를 대표하는 구성이 아니며, 'default finite-difference Jacobian' 메타데이터를 교정해야 한다. 해당 경로의 실패를 VODE 전체 prune으로 확대하지 않는다.
5. **판정 scope와 work 의미:** 10개 row의 rtol 오기, Julia 선형 풀이 수를 nlu로 부른 혼동, metric evaluator의 shape/truncation 허점을 확인했다. 새 audit evaluator는 빈 배열·shape 불일치·비유한값을 거부한다.

NCP campaign의 native arm은 dense-fast 하나여서 SP02 대비3–5배, 64CPU 내부 scaling, adaptive-grid crossover를 입증하지 않는다. 28개 독립1-thread worker의 throughput과 single-core latency도 별개다. 기존 실패를 환경/adapter/수치 원인별로 보존하고 새 성공에 섞지 않는다.

## 2. 바로 적용한 정확도·계산량 개선

### 2.1 열확산 Jacobian

기존 component benchmark는 basis JVP를 n회 호출하여 dense Jacobian을 만들었다. periodic heat의 JVP는 RHS를 다시 호출한다. NCP n128의42회 Jacobian에서 **5376회 부가 RHS**가 발생했고, solver에 보고된336회와 합치면5712회였다.

`component_benchmark_support/mod.rs`에 `algorithm_config.jacobian_assembly="analytic_stencil"` 옵션을 추가했다. periodic_heat와 advection_diffusion의 stencil을 직접 기입한다. 기본 basis-JVP 경로는 그대로 둔다. 실행된 경로에만 stencil complexity를 기록하고 부가 derivative RHS work를 별도 출력한다.

이 변경의 확정된 효과는 **Jacobian당 n회 부가 RHS를0회로 제거**하는 것이다. dense buffer clear는 O(n²), dense LU는 여전히 O(n³)다. sparse solver로 바뀌었다거나 전체 wall time이 특정 배수 줄었다고 주장하지 않는다. n1의 주기적 이웃 상쇄와 n2의 중복 이웃을 처리했고, Fourier 정확해와 실제 fast-driver trajectory까지 검사했다. 실패한 초기 compile과 reviewer 회귀 테스트도 보존했다.

### 2.2 Band/border Schur와 기존 GMRES 연결

`rodas5p_core::bordered::BorderedBandedPreconditioner`는 A의 packed band LU, W=A^-1 B, S=D-CW의 pivoted dense LU를 보유한다. constructor는 용량·byte overflow와 allocation failure를 처리하며, 실패한 refactor는 이전 factor를 무효화한다. 성공한 refactor/apply는 가변 active dimension에서도 allocation0회라는 계약을 allocator로 검사했다. arithmetic failure에서 apply output은 폐기해야 한다.

반폭 b, phase n, matter m일 때 setup은 O(n b²+n b m+n m²+m³), apply는 O(n b+n m+m²)다. 따라서 b,m이 고정되면 n에 선형이다. 가정이 맞는 문제에 대해서 dense LU의 입방 비용을 피하는 **구체적 알고리즘 보장**이다. wall-clock crossover는 별도 측정한다.

기존 `solve_gmres_into`에 실제 연결해 exact 모델과 off-band perturbation 모두 true residual과 해 오차를 검사했다. 제조해 n16/64/256의 최대 residual은1.11e-15, factor storage는132/516/2052개의f64다. 고정 전처리의 norm defect q<1이면 q^k 잔차 bound, exact triangular Schur이면 최대2회의 정확 산술 GMRES라는 유도를 `SCHUR_THEORY.md`에 수록했다. arbitrary coupled JVP에3–5회를 약속하지 않는다.

periodic wrap이나 Thomson gain을 band 밖이라는 이유로 버려서는 안 된다. finite-rank 항은 auxiliary border로 넣는 정확한 구성(`LOW_RANK_EXTENSION.md`)이 가능하다. energy group당 rank가 작더라도 전체 rank가 N_E와 함께 자라면 큰 전역 dense Schur 대신 local angular factorization을 사용한다.

### 2.3 보존형 remap

`ConservativeRemapWorkspace`는 정렬된 cell overlap을 O(n_old+n_new)에 순회하고, 적분량을 양의 overlap fraction으로 분배한다. exact arithmetic에서 positivity, integral conservation, weighted-L1 contraction을 증명했다. 구현은 finite/positive grid width, capacity, domain, nonnegative integral, underflow와 mass-defect threshold를 검사하고 거절 시 caller output을 보존한다.

이 kernel은 cell-average occupation f를 어떤 측도에 대해 재구성하는지 호출자가 명시해야 한다. f를 number measure E²dE에서 piecewise constant로 볼 때 E³/3 좌표를 사용할 수 있다. LRS oracle의 U=a³κE³f를 log-energy q에서 재구성하는 선택은 같은 photon integral을 다른 재구성 공간으로 근사한다. 두 방식을 묵시적으로 혼합하지 않는다. number-conservative remap이 energy moment까지 자동 보존하지는 않으므로 energy ledger가 추가로 필요하다.

누적 오차 e_(k+1)≤a_k e_k+d_k+δ_k에 remap defect δ_k를 포함한다. 매 step remap하면서 fixed-space 시간5차를 보존하려면 예를 들어 δ_k=O(h_k^6)라는 충분조건과 다른 오차항 제어가 필요하다. 이번 piecewise-constant kernel은 자동5차 AMR가 아니다. driver의 accepted-step transaction과 tolerance/cache/output identity 갱신은 P04에서 연결한다.

## 3. 첨부 조언에서 유지할 방향과 바꿀 부분

구조화 Jacobian, one-step integrator, 고정 격자부터 시작하는 전략은 채택한다. 반면 현재 solver는6개가 아니라8개 stage이며, smooth/frozen limiter만으로 고차·positivity가 보장되지 않는다. 실제 nonlinear RHS를 동결하면 다른 ODE가 된다. y'=y²에 대해 frozen coefficient 정확해와 원래 정확해의 차이가 -y0³h²/2+O(h³)임을 유도하고 Wolfram으로 확인했다. Jacobian/preconditioner 동결과 nonlinear flux 동결을 분리한다.

one-step 방식은 history 이관 부담을 줄이지만 spatial/remap error, stiff order reduction, 새 stiffness에 따른 h 재평가를 없애지는 않는다. `ADVICE_REVIEW_KO.md`에 항목별 판정, 증명, 논문 대응을 정리했다. Schur/GMRES와 remap 결과는 기존 수치해석 이론의 문제 맞춤 구성 및 구현이며 새 일반 정리의 novelty를 주장하지 않는다.

## 4. 별도 BASS/rei_bianchi 물리·코딩 연구

`BASS_REI_PORT_RESEARCH.md`와 `BASS_REI_DAG.json`을 별도 lane으로 작성했다. 조회된 최신 관련 pilot branch는 BASS `26f4a89a…`, rei `d32cd99f…`다. 읽은 C14/C15 세 source blob과 BASS split contract는 이 pilot에서도 동일했다(`BASS_REI_RECENCY.json`). main이 오래된 상태임을 확인하여 main을 최신 연구 source로 사용하지 않았다. 별도 run18/run19/bolometric branch는 metadata만 확인했으며 이번에 통합했다고 주장하지 않는다.

첫 이식 범위는 **non-tilted LRS Bianchi I, scalar photon transport**다. q=ln(E/E_ref), μ=cosθ, σ=diag(-s,-s,2s), κ=g/[(2πħ)^3 c³]에서

    U=a³ κ E³ f,
    v_q=-H-s(3μ²-1),   v_μ=-3s μ(1-μ²),
    3H+3v_q+∂_μv_μ=0.

따라서 적절한 angular measure를 포함한 cell-integral U는 conservative transport 형식으로 진화한다. azimuth 적분2π와 proper/comoving volume을 명시했다. 전체 일반 Bianchi에서는 추가 angular/tetrad 처리와 spatial-flux 조건이 필요하다.

`BASS_REI_FVM_REFERENCE.py`는 fixed nonuniform q×μ 격자의 upwind RHS, analytic JVP, closed/outflow boundary ledger, explicit CFL positivity oracle, cold Thomson angular redistribution, photon-ion-heat exchange와 signed anisotropic stress 검사를 제공한다. cold unpolarized LRS Thomson gain은 energy group당 rank2로 O(N_mu)에 계산한다. 반응 source는 number/ionization/heat를 동일 event rate로 연결하고 matter/radiation π_ab를 별도로 보존하여 상쇄를 정확히 표현한다.

12개 경량 검사와 별도 symbolic 계산은 이 식의 구현 대응을 지지한다. 이 Python oracle은 실제 BASS/rei Rust production integration과 구분된다. 현재 source를 손대지 않고 VigilODE branch에 이식 가능한 참조와 계약을 게시한다.

## 5. 구체적인 다음 개발 순서

`NEXT_STEPS.json`은 의존성·수정 위치·수용 기준을 기계가 읽도록 담았다.

1. **P01 평가기 계약:** application budget/internal tolerance를 분리하고 전체 times/states를 보존한다. 빈/누락/비유한 출력을 성공으로 판단하지 않는다. 새 protocol version을 쓰고 이전 frozen gate를 우회하지 않는다.
2. **P02 heat crossover:** 제공된 두 request를 시작으로 dense basis-JVP, analytic stencil, banded, 실제 matrix-free 경로를 비교한다. 큰 periodic wrap과 경계조건을 정확히 포함하고 같은 achieved-error에서 비교한다.
3. **P03 stage preconditioner factory:** mesh generation, hγ, coefficient identity에 결합한 band/border PC를 실제 ODE driver가 구성하도록 한다. factorization work와 실패 fallback을 계상하고 true residual을 확인한다. 현재 generic Preconditioner 연결 시험을 full trajectory로 확장한다.
4. **P04 accepted-step remesh:** state/mesh/measure/tolerance/error-budget를 원자적으로 commit하고 J/PC/recycle/dense/event 정보를 갱신한다. remap 실패 시 accepted old-mesh 상태를 유지한다. 공간·시간·remap refinement를 독립 실행한다.
5. **P05 기준 solver/reference:** VODE chord/Jacobian arm, diffsol·Julia의 구체적 algorithm, nonlinear 독립3단계 reference를 갖춘다. incomplete reference를 positivity로 대체하지 않는다.
6. **P06 production qualification:** 지원하는 문제·API 범위를 먼저 고정하고 해당 범위의 정확도·실패복구·자원·재현성 gate를 모두 통과시킨다. 한 범위의 성공을 arbitrary callback 전체로 확대하지 않는다.

BASS/rei에서는 P02 fixed-grid Rust adapter와 native formula parity를 먼저 완료하고, source-owned rates/energy/stress ledger, local low-rank scattering+border PC, signed stress feedback을 연결한 후 ALE/AMR로 이동한다. 전체 physical observable budget을 시간적분·격자·remap·linear solve·reference 항으로 분리해야 정확도와 속도를 동시에 조절할 수 있다.

## 6. 검증·실패·재현 기록

최종 근거는 `evidence/final_03/SUMMARY.json`, `EXECUTION.json`, before/after source manifests와 raw logs다. run1은 dependency metadata E0463에서 실패했고, 해당 local Cargo package artifacts만 cargo clean 후 재빌드했다. run2는 native tests를 통과했지만 새 probe의 JSON format brace compile 오류에서 중단했다. 이를 교정한 run3은9/9 exit0이다. 최초 unsigned-negation/PartialEq 테스트 compile 오류 및 이전 실패 logs도 남겼다. cache 오류의 정확한 내부 원인은 확정하지 않았으며 scientific failure로 분류하지 않는다.

`INDEPENDENT_REVIEW.md/json`은 별도 reviewer의 code/index, Schur/remap, 물리 normalization 검토와5개 지적의 교정을 기록한다. `CODEX_HANDOFF_KO.md`는 저예산 실행자가 source 확인·컴파일·targeted check·NCP 후속 비교를 이어가게 한다. 이 보고서의 새 source에 과거 frozen package verifier를 실행하여 과거 identity PASS로 위장하지 않는다.
