# VigilODE 통합 solver 심사 · 2026-11-24

**판정은 패키지 전체 HOLD, 지정 문제에 대한 조건부 채택이다.** 보호 RODAS5P는 일반 stiff IVP의 기준기로 유지할 만하다. 다른 방법을 같은 crate에서 선택하고 원래 문제로 fallback하는 인터페이스, 독립 참조와 대조한 신규 수치 경로까지 구현했다. 전체 패키지의 블라인드 work–precision, 모든 노출 방법의 강건성, 독립 환경 재현에는 미충족 항목이 남는다. 보호 기준기 자체의 검증과 후보 전체의 release 자격은 별개다.

## 기준점과 기록

GitHub 기본 브랜치의 8월 커밋은 `8d0c7918`이다. 작업 기준은 PR #190의 11월 통합 브랜치 `a66215e7`와 CI 감사가 하나 더 있는 로컬 `ab490717`이었다. 사용자가 제공한 `a8d196b4` 이후 두 커밋은 문서·감사 변경이다. 새 연구 브랜치에는 로컬 형제 V25 `3768dac5` 및 V26 `de7f18ac`의 코드와 최종 측정 기록을 merge했다. Git HEAD 370개, PR 190개의 인덱스와 주제별 실제 판독 근거는 [계보](history/lineage_audit.json) 및 [원격 인덱스](history/source_index.json)에 있다. 개인 자료에서는 프로젝트 관련 제목 검색·폴더 인벤토리와 24개 인계 및 원본을 확인했다. PR #183의 대형 peer 사례는 6개 궤적 바이트를 검산했다. 과거 대화 조회는 이 세션에서 비활성이고 브라우저는 로그아웃 상태였다. 프로젝트 Pages 검색에서도 해당 기록을 찾지 못했다. 8월 복구 키트는 183개 파일 스냅샷이며 원문 전체 export가 아니어서, 기억에만 있는 주장을 채택하지 않았다. [이전 결과 대조](history/historical_evidence.json)

## 고등수학의 이식 계보

| 수학 | 이전 PR에서 확인한 구현 | 현재 판정과 다음 판별 실험 |
|---|---|---|
| 대수·특이기하 | #77, #93–103의 다항 cyclic closure와 중복도를 세는 quotient trace. 원래 36개 조건 중 24개 허용, 12개 비허용. | 모든 복소근 위의 trace와 사용자가 추적하려는 실수 해는 서로 다른 관측량이다. 7개 궤적의 시간비 중앙값은 1.008이었다. 판별식 영점 양쪽에서 물리적 실근의 격리, 가지 거부, 다변수 원식 residual을 시험한다. |
| ODE 동역학 | #94, #97–100, #104, #108, #176에서 비정규 과도성장, 가중 로그 노름, 국소 구조 관측량을 확인했다. DOPRI5의 secant 강성 추정은 실제 switching에 쓰인다. | 비정규 증폭의 유한시간 상계가 지나치게 느슨한 입력이 있다. 구조가 바뀌는 holdout에서 관측가능한 조기신호와 실패비용을 함께 측정한다. |
| 기호 동역학 | #104, #108, #176, #188, #190 계보에 유한 guard cell과 전이 trace, 재진입 제한을 검사하는 automaton이 있다. | automaton은 오프라인 검증기다. 온라인 정책은 V27에서 결정 시점에 실제로 관측한 guard 값만 사용하고 pilot 및 잘못된 전환 비용을 합산한다. |
| Homotopy·위상 | #74, #91–92, #115–125의 제한된 원천식에서 역함수·연속변형을 사용했다. P04의 oriented incidence/Hodge는 기존에 테스트 한 건이었다. | 원래 stage 잔차를 검사하는 R5 전체 IVP를 아래에서 측정했다. 구면 스칼라 확산의 검증 범위와 일반 미분형식·복합격자의 위상 검증은 다르다. |
| 구조 그래프와 계산 순서 | #185–187의 affine 경로 소거, 피드백 core 및 scan 분해. 예전 #187 측정에서는 유효 경로 길이가 1이었다. | 이번에는 긴 경로에서 실제 합성 횟수를 줄이고 전체 궤적을 검증했다. 희소 원식 잔차의 비용까지 포함하는 CSR 비교를 남겼다. |

전체 PR 번호, 당시 소스 경로, 이식 여부 및 원자료 해시는 [계보의 10개 연구 가족과 P01–P16 대조](history/lineage_audit.json)에 있다.

## 세 가지 서로 다른 Jacobian 비용

RHS만 받는 BDF/Radau는 내부에서 유한차분으로 밀집 Jacobian을 만들고 Newton을 수행한다. 보호 matrix-free RODAS5P는 사용자 JVP 또는 원천 모델에서 얻은 작용으로 GMRES를 수행한다. source-owned cubic의 대수적 역함수 및 조건부 구간 검증 경로는 특정 다항식과 원래 식의 잔차를 이용한다. 세 방법의 “미분 불필요” 범위가 다르다. 각 방법의 상태 의존성, operator·root·fallback 계약은 [METHODS.json](METHODS.json)에 있다.

일반 문제에는 `VigilODE::solve`, 방법을 지정할 때는 `VigilODE::solve_method`를 사용한다. 새 BDF/Radau FD lowering의 RHS 계수를 외부 카운터와 대조했다. `MethodChoice::Auto`의 registry fallback은 수치 실패에서 t₀부터 다시 실행할 수 있고, 일반 unified Auto는 수용 상태·남은 시도 예산을 이어받는다. 치명적인 사용자 callback 뒤 registry가 다른 방법을 실행하던 회귀를 먼저 재현하고 수정했다. Lawson 잔여항이 전체 RHS로 잘못 전달되는 요청도 사전에 거부한다. [포트폴리오 측정](results/portfolio_results.json)

`VigilODE::solve_matrix_free`는 선언된 JVP와 호환되는 출력에서 shipped, P1, P2를 명시적으로 실행한다. `VigilODE::solve_modal`은 전체 RHS나 dense output이 필요한 경우 보호 경로를 돌려준다. `structured` feature는 `solve_cubic_decay`, `solve_coupled_cubic` 및 `solve_graph_cubic`을 제공한다. 원래 상태, 실제 driver, 실패 시 마지막 수용점과 작업량은 각 solver의 반환형에 보존된다. `Solution::certified_affine_event_windows`는 이미 계산한 궤적에서 선언된 스칼라 affine event를 사후 검사한다. 거부된 근거를 성공으로 바꾸지 않는다. [P11 실행](affine_events/EVIDENCE.json)

## 8단계 문제에서 줄일 수 있는 비용

일반 비선형 transformed ROW의 i번째 RHS는 앞서 결정된 단계 상태에서 평가된다. V05의 tableau와 black-box 가정에서는 8단계 의존성이 남는다. 반면 아래 세 경우에는 *무엇이 미리 알려지는가*가 다르다. [수식 및 외부 문헌](REFEREE_THEORY.md)

| 입력·계산 단위 | 이번 실행 | 채택 범위 |
|---|---|---|
| 고정 실수 비양수 고유율의 독립 정상모드 | 원래 tableau에서 endpoint와 signed embedded error를 두 개 다 유도했다. 유리수 recurrence 293건, 종점 차 ≤6.01e−16. 512모드에서 같은 종점 오차 약 5.53e−9, modal 326 µs, 보호 banded 1,859 µs, 정확 지수 3.95 µs였다. | 순수 모드 좌표의 전체 적응 적분. 기저변환·강제항·비정상 행렬은 별도 문제다. 단일 커널 재구성은 순차 raw-U보다 느렸다. [근거](../modal_transfer_20261124/README.md) |
| 비선형 상태 잔여항과 독립 시간 forcing | 8개 단계에 등장하는 6개의 서로 다른 clock에서 순수 g(t)를 선계산했다. 비싼 합성 forcing 2-worker 11.73 ms, 직접 15.89 ms. 싼 경우 1,834 µs 대 5.42 µs. 잘못된 출력의 fatal 처리와 speculative 작업 계수를 검증했다. | 지정 trial용 opt-in. 준비한 clock 밖은 직접 호출한다. 동적 도중 매번 cache를 재구축하는 자동 정책은 후속이다. [실행](results/split_forcing_native.json) |
| cooperative cubic source의 W 내부 공간 경로 | C5를 원래 8-stage driver에 연결한 48/96점 고리에서 Hillis–Steele 9,855/24,210 affine 합성, Blelloch 3,735/7,965. 전체 출력 차 ≤4.51e−17. 실제 손상·margin 퇴화에서 원본 LU fallback. | 내부 공간 경로. 이 규모에서 순차 전진대입이 가장 빨라 Blelloch는 opt-in이다. [내부 시험](history/p12_native_evidence.json) · [facade 및 푸리에 기준해](history/c5_facade_evidence.json) |

같은 시각 c=1에 도착하는 6·7·8단계도 상태는 서로 다르다. 시간 forcing 재사용으로 상태 잔여항까지 재사용해서는 안 된다. 정상모드에서 단계 소거가 되는 이유는 원래 비선형 함수의 상태 값을 추정했기 때문이 아니라 선형 연산자와 frozen tableau의 유리함수가 정확산술에서 같은 연산을 나타내기 때문이다. 이 가족에서는 정확 지수 전파가 가장 싼 비교군이었다.

## V25, V26 및 회수한 실패

- V25 P1은 높은 정확도부터 느슨한 정확도까지 네 문제족에서 동일 달성오차 명령어 비 0.198, 0.791, 0.516, 0.470을 달성했다. 완료 9,287개 선형 풀이의 true residual 위반 0. Robertson에서 예산 소진 38→41로 원래 B 판별자를 실패했다. 같은 P1 경로의 **고정** 계 1,488개를 양쪽으로 풀면 41 대 41, S 경로 1,443개에서는 38 대 38이다. 기존 자료가 가리키는 차이는 개별 고정 선형계보다 적응 스텝과 상태가 만드는 서로 다른 계의 열이다. 따라서 같은 단계에서 S를 한 번 더 부르는 해법은 이 자료로 지지되지 않는다. [V25 원자료와 분석](../production_loop_20261116/mf_p1_confirm/REPORT_KO.md)
- V26은 8열 표본 검사가 놓친 오류 576건 중 575건을 전체 검사와 multicolouring에서 복원했다. 한 건은 단일열 FD 자체도 반올림으로 구분하지 못했다. 이 검사는 해당 입력에서의 일치 검사다. 모든 가능한 미선언 비영항 탐지를 증명하지 않는다. Dense/declared sparse, RHS 비용 및 정확성 요구에 따라 선택한다. [V26 원자료](../production_loop_20261116/assembly_check/REPORT_KO.md)
- V06의 계획 문서에 JVP 비의 분자와 분모가 뒤바뀌어 있었다. 공유 basis는 여섯 셀에서 JVP를 줄였고 실제 명령어는 약 64–376배 늘었다. R7의 원본 24개 trace는 4.746배, C5의 36개 trace는 2.242배 느린 이전 판정을 재현한다. 채택 기준을 유지하고 잘못 기술된 비용 원인을 바로잡았다. [재해석 및 commit 계보](history/lineage_audit.json)
- PR #110/#112의 triggered FGMRES 및 조건부 구간인증을 복원했다. 64-cell shifted diffusion 네 조건에서 동일 원잔차 허용치의 곱 횟수는 22→9, 43→28, 64→56, 127→115였다. 기준 GMRES의 잔차가 더 작다. interval stage bound에는 출처·epoch가 맞는 J,b enclosure가 필요하다. [Krylov 실행](history/n02_n03_native_evidence.json)
- PPSW1은 원래 G0 holdout에서 실패했다. 한편 PR #183의 184,320변수 점 IVP에서는 같은 궤적을 직렬 3.3683 s에서 병렬 2.7306 s로 계산했다. 해결한 것은 해당 점 IVP의 처리속도이며, 원래 ROW의 stage DAG와 전체 불확도 전파는 별도로 남는다. [재검산](history/historical_evidence.json)

## Homotopy의 원래 8단계 검산

기존 R5에서 비어 있던 native 검사를 실제 수행했다. cooperative cubic 원천에서 frozen 선형계는 $(D-C)u=b$이고, 단계 RHS $b$에는 음수 성분이 허용된다. $D_i=(hγ)^{-1}+k_i+3g_i y_i^2+Σ_j c_{ij}$일 때 $q_λ=λ\max_i Σ_jc_{ij}/D_i<1$이다. 소스 계수를 이용한 대각 작용이므로 opaque RHS에 대한 보편적 derivative-free 결과로 일반화할 수 없다. 원래 $λ=1$ 잔차의 outward upper bound와 엄밀한 행 우세 여유로 stage error를 제한하고, 조건이 깨지면 동일 W의 LU로 복귀했다.

6개 source/grid × 2 tolerance × 4 arm = 48개 전체 IVP를 실행했다. arm당 8,688 stage 중 direct는 7,618개 인증·1,070개 fallback, 3-station continuation은 7,622개 인증·1,066개 fallback이었다. 고정밀 Taylor 참조로 본 1% 동등 달성오차는 각 7/12 조건이다. 이 조건에서 direct는 보호 LU보다 2.38–6.42배, continuation은 2.66–6.94배 오래 걸렸다. 별도 actual-K 검산에서도 direct λ=1과 3개 station의 출력반경이 같았고 후보 RHS는 80 대 240이었다. 이 문제족에서 homotopy의 성과는 정확한 원식 검증과 안전한 fallback 구현이다. 성능 승격은 기각했다. [R5 원시 궤적·판정](r5_cubic_stages/README.md) · [actual-K 비교](actual_k_homotopy/COMPARISON.json)

## 독립 외부 solver와의 판별

동일 U11 3문제, 5개 허용오차, analytic dense Jacobian과 초기값을 Rust RODAS5P와 CVODE-BDF 6.4.1에 공유했다. 각 셀 2회 warm-up·7회 교차 반복, 원래 SciPy 참조, 별도 Julia Rodas5P 18개 종점(문제별 1e−13 포함)을 사용했다. 아래는 공통 달성오차 구간의 양끝과 기하중점에서 log 보간한 최소–최대이고 비율은 **Rust RODAS5P / C CVODE**다. CVODE는 마지막 내부 스텝을 넘어가 dense interpolation으로 종점 값을 낼 수 있다.

| U11 IVP | 수용 스텝 비 | 진단 wall 비 |
|---|---:|---:|
| HIRES | 0.096–0.149 | 0.102–0.143 |
| Oregonator | 0.090–0.180 | 0.091–0.127 |
| Brusselator MOL32 | 0.254–0.339 | 0.501–0.526 |

CVODE는 Oregonator rtol=1e−4에서 flag −3으로 미완주했다. 30개 native 셀 중 29개 성공과 이 실패의 원시 결과를 전부 보존했다. U11 경험적 참조불확도를 tight Julia와의 차이까지 확장해도 admissibility 변화는 0개였다. 반면 별도 비선형 제조해에서 rtol=1e−8일 때 RODAS5P/BDF/Radau는 42/40/22 수용 스텝, 오차는 2.96/2.97/2.60 ×10⁻¹⁰이었다. “항상 더 적은 스텝”도 “항상 더 느림”도 성립하지 않는다. 시간비는 공유 호스트 진단값으로 남긴다. [native/Julia 원시·평가](cvode/results.json) · [제조해 대조](results/split_forcing_native.json)

## 구면 및 2D·3D

주기적 평면/공간의 보존형 확산과 극축 무유량 구면 유한체적 연산자를 일반 frontend에 연결했다. 혼합 모드에서 독립 sparse expm 대비 종점 최대오차는 2D 4.72e−11, 3D 3.93e−11, 구면 2.85e−10이었다. 세 경우 shipped→P1의 JVP는 4,604→993, 5,712→932, 5,712→1,013, 수용/거절 스텝과 예산 소진 횟수는 동일했다. 행 병렬화 단독은 이 호스트의 모든 측정 셀에서 손해였다. [공간 검증](../spatial_review_20261124/RESULTS.json)

등방·경도불변 S² 열 연산자에서 A=I−σL, P=I−σLφ의 면적 가중 generalized spectrum은 위상 모드와 경도 방향 선 풀이 비용을 분리한다. 균일 각격자와 반지름 R 아래 κ₂(P⁻¹A)의 **가중 대칭화 정의**에 대응하는 극값 비는 1+2σκ cot(Δθ/2)/(R²Δθ)이다. 일반 Euclidean singular-value condition number와 구별한다. 32×64의 한 shifted system에서 원래 잔차를 유지하며 GMRES 2,386→54 Arnoldi columns, 같은 separable 계의 직접 Fourier 해는 상대잔차 5.10e−15였다. 같은 구조를 Rust의 O(N) 주기적 경도선 역연산자로 구현했다. 별도 12×24 native full-stage GMRES에서 Arnoldi 열 90→30, 참 상대잔차 5.86e−15였다. 구면 적응형 IVP에서는 shift 갱신과 전체 비용 검증이 남는다. [native 로그](../spatial_review_20261124/native_polar.log)와 [시간적분 연결 계약](../spatial_review_20261124/latitude_stage_port.json)에 있다. [유도·참조 계산](../spatial_review_20261124/polar_lines.json)

## 외부 애플리케이션용 소스 패키지

잠긴 registry 의존성과 solver 계수 fixture를 포함한 소스 SDK를 구성했다. 빈 Cargo 홈과 새 target에서 외부 앱이 직접 의존성 하나인 `vigilode`로 기본 stiff 감쇠와 `structured` cubic을 모두 실행했다. 이 검증의 환경은 Linux, Rust 1.94.1이다. 배포물 재생성, 출처 목록 및 소비자 확인 절차는 [패키지 설명](packaging/README.md)과 [실행 영수증](packaging/RECEIPT.json)에 있다.

## 릴리스 심사

기존 V01 제안 판정은 correctness·robustness·API·reproducibility·packaging·evidence governance 6축 NOT_MET, performance·geometry 2축 PARTIAL이었다. 다음은 이번 변경이 해소한 범위와 여전히 릴리스에 필요한 근거다.

| 축 | 이번에 확인한 것 | 남은 범위 |
|---|---|---|
| 정확성·강건성 | 3개 U11 참조군, 제조해, 폐형식 해, 보존법칙, signed stage residual, 치명적 callback 재호출 수정 | 모든 노출 방법의 stiff order·nonnormal·불연속·dense-output·실패 전파 시험 |
| API·패키지 | 단일 Cargo facade의 일반/지정 경로, 명시적 typed problem과 계수, optional structured 기능 | 연구 경로별 지원 범위와 실패 결과를 장기 공개 API 계약으로 고정 |
| 성능 | 달성오차가 겹치는 비교와 실제 연산·fallback 계수. 이득·손실 동시 보존 | 격리된 호스트, 신규 holdout, 변환·전처리·초기화 포함 재현성 |
| 재현성·근거 관리 | 동결 계수, source tree, raw time/error, 독립 수학·수치 검산과 실행 계약 | 통합 버전에 대한 두 번째 환경·독립 실행자 재현 |
| 공간 기하 | 주기격자와 S² 단일 필드 검증, polar/constant-mode와 면적 norm | 비균질 계수, 경계/좌표 chart, coupled reaction/advection에서 적용 경계 |

기존 gate 전체를 통과했다는 판정은 내리지 않았다. [기존 release matrix](../../docs/agent/production_solver_program_20261112/RELEASE_GATE_MATRIX.json)와 [V01 최초 심사](../production_loop_20261116/audit/R_GATE_PROPOSAL.json)를 함께 보존한다.

## 다음 실험과 재현

수학적 신규성의 후보는 “최초의 병렬 Rosenbrock”가 아니다. 원래 단계식·출력의 의미를 유지하면서, 모델이 제공한 구조로 실제 절약 가능한 비용을 예측하고, 조건이 깨지면 동일한 물리 문제의 보호 풀이로 돌아가는 경계를 실증적으로 정하는 데 있다. 병렬 peer, SDC, Radau stage preconditioning, 행렬 없는 Newton–Krylov, affine prefix, 고유모드 전파는 각각 선행기술이 있다. 강한 우월성 주장은 독립적인 work–precision 및 재현을 기다린다.

[10+10 실행](RUN.json) · [방법별 계약](METHODS.json) · [후속 DAG](NEXT_PLAN.json) · [복사할 coding-agent 프롬프트](CODING_AGENT_KO.txt)
