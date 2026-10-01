# VigilODE R4 적대적 재감사 및 수학·코딩 연구 보고서

**판정: 업데이트의 여러 수정은 재현됐지만, “모든 검증 문제가 닫혔다”는 해석은 성립하지 않는다.** 새 반례는 참조 BDF/Radau의 시간 기하, 연구용 변환·stage 인증, timing 판정/receipt 검증에 집중된다. 기본 순차 matrix-free RODAS5P 경로에서 같은 오작동을 입증한 것은 아니다. 아래의 범위를 유지하면서 correctness와 authority 경계를 먼저 고친 뒤 성능 연구를 진행하는 것이 좋다.

| 식별자 | 값 |
|---|---|
| 감사한 생산 소스 | `1c54194123ee6abc6daa512e8574922f510b4e2c` |
| 생산 소스 tree | `e50f33fd2b336ed146b25f68e38bb29b19c44d0b` |
| 이전 R3 보고서 커밋 | `6598abd0db790162ab26f50bc13943f24a0c7e4a` |
| R4 실행 전 사전등록 | `b2914f3e3c03eda60a8547619db6284aac8250a2` |
| 게시 브랜치 | 기존 `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3` |
| 작업 범위 | 생산 코드 변경 없이 보고서, 재현 probe, 정확 산술 후보, 원시 결과, 개발 계획 추가 |

이전 보고서를 재포장하지 않았다. 최신 소스를 새로 고정하고, 계약과 probe bytes를 **원격 커밋한 뒤** 새 실행을 시작했다. 실제 실행 HEAD는 보고서 사전등록 커밋이고 생산 코드의 바이트는 위 감사 소스와 같다. `evidence/source/SOURCE_MANIFEST.json`, `INPUTS_SHA256.json`, 실행 receipt가 그 연결을 기록한다. 요청한 GPT-6 Astra v4 수학/코딩 하네스는 연구 절차로 적용했으며, 실제 모델 식별을 주장하지 않는다.

## 1. 무엇이 개선됐고 무엇을 다시 열어야 하는가

업데이트는 단순 문서 수정 이상이다. 표현된 시간 간격을 실제 적분에 사용하고, 잃어버린 phi weight의 권위를 내려놓으며, 원시 timing admission·독립 process receipt·Monte-Carlo gate를 구현했다. Homotopy는 순차 stage target을 명시하고 outward certificate와 native q2 인증을 도입했다. Joint Chebyshev/Laguerre도 실제 구현, 오차 상태 구분, coefficient cache, 사전등록 측정까지 이어졌다. 이 점들은 이번 실행에서 확인한 closure와 연결된다.

그러나 원래 반례가 통과한다는 것과 입력·단위·차수·stage 수·receipt 구조를 바꿔도 invariant가 유지된다는 것은 별개다. 이번 결과는 경계 확장 과정의 누락을 보여준다. 상세 원시 증거와 정확한 소스 위치는 각 lane의 결과 JSON/보고서와 통합 `FINDINGS.json`에 있다. 우선순위 P2는 해당 명시된 domain에서 고쳐야 할 결함이며, 기본 생산 경로 전체에 대한 치명도 판정이 아니다.

| 영역 | 이번 재감사 결과 | 해석 |
|---|---|---|
| R3 큰 시간 원점의 일정 유량, 출력 endpoint | 고정 반례 통과 | 표현된 구간과 strict 출력 endpoint 수정은 유효 |
| Fixed BDF 시간 단위 변경 | 같은 무차원 문제에서 약 2.083% 차이 | 문서가 알린 absolute floor의 실제 오차 반례를 추가; 새 미공개 회귀로 분류하지 않음 |
| Radau1 불균등 half-step | 3-ULP split에서 오차 추정이 4/5로 축소 | 업데이트된 경로에서 새로 재현한 기하/공식 불일치; 이전 버전의 동일 입력 정상 여부는 미검증 |
| R3 phi lost-weight | 잘못된 convergence/권위 승격이 차단됨 | 기존 수정 유지 |
| 연구용 transform bound | 정상 생성 p=2 weight에도 false upper bound | `ExpBound`의 ZERO 비교를 우선 수정 |
| Chebyshev/Laguerre action | 새 40 action에서 32 Certified 상한이 oracle 오차를 포함, 8 EstimateOnly | 유한 corpus의 지지 증거; 범용 인증 증명 또는 속도 승격 아님 |
| Timing 원래 5 malformed case | 모두 거절 | 기존 raw admission 수정 유효 |
| Timing 전체 authority hold | verifier가 여전히 `Promote` 반환 | 문서의 보류가 기계적 consumer 경계에 전달되지 않음 |
| Session receipt 세부 구조 | 뒤 session의 부족한 warmup/29·31 pair count가 통과 | 병합본 검증만으로 원 session의 완전성을 보장하지 못함 |
| Homotopy 정상 constructor와 native 8-stage | 고정 controls 통과 | 정상 인증 전제의 수학 구조를 지지 |
| Homotopy public witness / generic stage count | 손상 witness와 9·16-stage에 false certificate | 공개 API 전제·지원 범위의 fail-closed 검증 필요 |
| HOM06 / POLY03 / coverage | 기존 8개 arm receipt 재생 성공 | 이전 속도 수치는 상속 증거이며 새 timing 측정 아님 |

## 2. 실제 실행과 증거의 한계

Rust 1.94.1을 첨부 패키지에서 준비했고 locked registry dependency 118개 전부의 checksum을 확인했다. Python 정확 산술에는 표준 `Fraction`과 `Decimal`을 사용했다. native와 oracle은 생산 구현을 공유하지 않도록 분리했다. 모든 Cargo build는 같은 lock으로 직렬화했다.

전체 workspace / all-targets / all-features offline 빌드는 **230.67초, exit 0**으로 완료됐다. 발견한 149개 harness의 735개 named test 중 **671 PASS, 4 ignored, 20 timeout 미확정, 40 총예산상 미실행**이다. Assertion failure는 관측하지 않았지만 **전체 suite PASS는 아니다**. 변경된 15개 harness는 모두 정상 종료했고 그 안의 68개 test가 통과, 1개가 ignored였다. Doctest는 실행하지 않았다. `evidence/runtime/TEST_COVERAGE.json`은 모든 이름과 상태를 보존하고 `LOCAL_REMAINDER_PLAN.json`은 남은 검사만 지정한다.

네 lane의 새 native 관측은 시간 36행, 다항식/산술 78행, homotopy 9행, 통계 21행이다. 각 프로세스의 종료 성공은 “문제가 없다”는 판정이 아니라 관측을 완료했다는 뜻이다. 반례는 exit 0 JSON 안에 정상적으로 보존된다.

- 시간/변환/유한 경로합의 다항식 값은 정확한 binary64 입력을 유리수로 해석해 검산했다.
- 지수·phi action 비교는 140-digit Decimal oracle을 사용했다. 관측 오차와 상한의 비교를 지지하지만 arbitrary-precision interval proof와 같다고 하지 않는다.
- 정리의 전제, 유한 정확 검산, native 구현 결과, 빌드/환경 상태를 별도 기록한다.
- 새로운 blind 성능 holdout, 전체 플랫폼 교차검증, release 속도 실험, 일반 비정규 행렬 인증은 실행하지 않았다.
- 독립 검토자는 같은 고정 binary를 직접 재생하고 통계 조합확률을 다른 Fraction convolution으로 검산한다. 후보 튜닝이나 재귀적 감사 루프는 수행하지 않는다.

첫 runtime 압축 해제의 소유권 오류는 보존 후 소유권 복원 없이 다시 해제했고, target directory 상대경로 문제는 실제 cache를 가리키는 alias로 바로잡았다. 생산 코드는 바뀌지 않았다. 사전등록 runner는 원래 bytes를 유지하고, 실행하지 않은 경로 정규화 개선본은 별도 파일과 patch로 구분했다. 상세 incident와 stdout/stderr를 함께 게시한다.

## 3. 시간 기하: 올바른 endpoint만으로 충분하지 않다

### 3.1 Fixed BDF의 절대 비교 바닥값

`bdf.rs`의 `same_step`은 양의 두 step을 비교할 때 시간 단위와 무관하지 않은 absolute floor를 사용한다. `t`가 power-of-two 경계를 넘으면 ULP가 달라져 실제 간격이 바뀔 수 있다. 이때 서로 다른 간격을 “같다”고 판정하면 상수 step BDF2 계수를 사용한다. Newton solver가 그 잘못 선택된 residual을 정확히 푸는 것은 시간 기하의 정확성을 복구하지 못한다.

사전등록한 경계 `2^p`, p∈{-40,0,10,40}에 대해 일정 유량을 같은 무차원 문제로 만들었다. p=-40,0에서 끝값은 약 **0.9791691102325774**, 정확값은 1이다. p=10,40의 오차는 약 1.8e-15였다. 동일한 native time list에서 기존 공개 `bdf_step_variable`을 쓰는 standalone driver는 약 1.8e-15로 돌아왔다. 잘못된 branch의 정확 유리수 recurrence도 native 결과와 약 1.1e-16 이내에서 일치했다.

BDF2의 일정 유량 문제에서는 정확한 variable-step 계수가 주어졌을 때 discretization 오차가 생길 이유가 없다. 실제 이전 간격 k와 현재 h가 다르면 상수 step 식의 잔차 결함은 이 설정에서 v(h−k)로 드러난다. 따라서 다음 패치는 heuristic threshold 조정보다 **실제 step bits가 같을 때만 shortcut을 허용하거나 variable 계수를 항상 사용하는 것**이 적절하다. 실제 비율이 stability restart 기준을 넘는지도 같은 값으로 검사해야 한다.

범위는 README가 reference-implementation-only로 분류한 내부 BDF다. 기존 closure가 absolute floor를 공개했으므로, 이번에는 그 잔여 한계에 새 정량 반례를 붙여 재개방한다.

### 3.2 Radau1의 2+1 ULP 분할과 Richardson 계수

`t0=1e12`, `H=3 ULP(t0)`, `y'=2(t−t0)/H²`, `y(t0)=0`이면 정확한 최종값은 1이다. Radau1은 implicit Euler이고, 표현 가능한 half-step은 2 ULP와 1 ULP다. 정확 계산은 다음과 같다.

\[
y_{coarse}=2,\quad y_{fine}=14/9,\quad |e_{fine}|=5/9,\quad |y_{fine}-y_{coarse}|=4/9.
\]

기존 equal-half 공식은 p=1에서 차이를 그대로 error estimate로 쓴다. `atol=.5`, `rtol=1e-12`에서 native가 보고한 normalized estimate는 약 **0.888888888886**, 이 정확 family의 실제 fine error는 약 **1.111111111108**이고 adaptive wrapper는 이를 받아들였다. 이것은 일반적인 local estimate를 global error guarantee로 오독한 비판이 아니라, 이 정확 family에서 분할 기하에 맞지 않는 계수가 쓰였다는 반례다.

실제 분할 r=h1/H와 1−r에서 공통 leading local-error 계수라는 가정 아래

\[
\eta=r^{p+1}+(1-r)^{p+1},\qquad
|\widehat e_{fine}|=\frac{\eta}{1-\eta}|y_{fine}-y_{coarse}|.
\]

r=1/2이면 기존 `1/(2^p−1)`로 돌아가고, r=2/3,p=1이면 5/4다. standalone correction은 위 반례의 5/9를 약 3.3e-16 이내로 재현했다. 이 식은 일반 nonlinear ODE의 엄밀 상한이 아니므로, 먼저 Radau1에 geometry metadata를 넣고 필요한 조건을 못 만족하면 typed refusal로 처리하는 것이 좋다. BDF의 BDF1+BDF2 mixed-order startup에는 이 공식을 그대로 복사하지 말고 별도 유도를 해야 한다.

현재 `max_step`에는 one-resolution slack이 문서화돼 있다. 이를 strict cap 위반 버그로 중복 집계하지 않았다. 다만 공개 설정 이름/설명과 실제 정책을 맞추고 실제 초과량을 기록해야 한다.

## 4. 변환 오차 상한: 표현 범위를 늘려도 순서 관계가 틀리면 인증은 깨진다

### 4.1 정상 생성된 low-order weight의 반례

연구용 `bound_transform_error`는 `ExpBound = mantissa × 2^exponent`로 underflow 아래 값까지 유지한다. 그러나 norm 최대값을 고를 때 ZERO의 저장 exponent=0을 양수와 똑같이 비교한다. 예를 들어 양수 .25의 exponent는 -1이므로 뒤에 나온 ZERO가 이를 덮을 수 있다.

`A=[[0,.25],[0,0]]`, `h=1000.1`, `b2=[0,1]`, `stored=weight_phi_vectors(...)`는 위조하지 않은 정상 생성 입력이다. A²=0이므로 phi를 유한식으로 정확하게 계산할 수 있다. 반환 상한은 **5.8207660913467407e-11**, 실제 변환 오차의 2-norm은 약 **1.5071782093206548e-9**로 상한의 약 25.9배다. operator norm에서 nilpotent coupling이 사라진 것이 원인이다.

우선 ZERO가 모든 양수보다 작도록 하나의 공통 order를 정의하고 row/column max, final norm 선택, class 비교를 같은 규칙으로 통일해야 한다. transpose·동시 순열·subunit 스케일을 바꾼 정확 반례를 regression gate로 삼는다. 현재 이 함수는 research-only이며 이번 소스에서 production consumer 연결이 확인되지 않는다. 기존 lost-weight convergence 거절을 되돌려서는 안 된다.

### 4.2 높은 phi order와 공개 입력 검증

- k=171,172에서 binary64 factorial이 overflow한다. `1/k!`를 직접 만들지 않는 표현의 이점이 denominator 생성에서 사라져 false upper bound가 생긴다. p=171의 한 입력에서는 실제 오차 약 2.0810e-317을 약 2.7792e-330으로 보고했다. 현재 joint polynomial의 p≤4 오류라고 확대하지 않는다.
- k=0의 임의 stored 입력을 허용하는 공개 API에서 b0=1, stored0=−1이면 실제 오차는 2인데 상한은 0이다. source-generated w0에서는 발생하지 않으므로 입력 계약 검증 문제로 분리한다.
- 음수 tolerance가 절댓값 변환을 거쳐 일부 양수 bound를 admit한다. 음수/NaN을 API 입구에서 거절하고 +infinity 정책도 명시해야 한다.

역팩토리얼 후보 `r0=1`, `rk=div_up(r{k−1},k)`를 mantissa/exponent 표현으로 유지하면 양의 정수 k를 정확히 나타내는 범위에서 귀납적으로 rk≥1/k!다. 등록된 여섯 order에서 정확 유리수 검산을 통과했다. 이는 독립 후보의 제한된 통과이며 생산 코드에는 아직 적용하지 않았다.

## 5. Laguerre·Chebyshev: 현재 구현을 발전시키는 경로

이번 40개 action 중 32개 Certified 결과는 140-digit oracle 오차를 포함했고, 8개 EstimateOnly는 권위를 올리지 않았다. 관측 최대 절대오차는 약 **4.739e-14**다. 기존 cache-key/domain 검증도 새 regression tranche와 함께 확인했다. 큰 amplitude에서 naive squared norm이 overflow하는 경우는 명시적 거절이므로 false certificate와 구분한다. 새 scaled outward norm 후보는 다섯 극단 입력의 정확 sum-of-squares 상한을 포함했다. norm 하나를 고쳤다고 전체 recurrence의 dynamic range가 해결되는 것은 아니다.

현재 다항식 target은 주어진 w_k에 대한 `Σ φ_k(hA)w_k`다. w_k=h^k b_k 형성 오차와 action의 총오차는 따로 관리해야 한다. `truncation_budget`을 충족한 것이 `total_error<=requested_budget`과 같지 않다. 추가할 consumer admission은 **Certified이고 총상한이 요청한 절대 budget 이하일 때만** 참이어야 한다.

Laguerre의 유용한 출발점은 X=−A/β, a=hβ, q=a/(1+a)에서

\[
 e^{-aX}=(1-q)\sum_{n\ge0}q^n L_n(X),
\quad
\|tail_m\|_2\le e^{\rho/(2\beta)}q^{m+1}\sum_k\|w_k\|_2/k!
\]

라는 구조다. 대칭 비양 A와 검증된 spectral interval 아래에서의 bound이며, 계산한 recurrence의 rounding error는 별도다. 구현의 EstimateOnly는 이 점에서 정직하다. 우선 local residual propagation majorant를 correctness baseline으로 구현하고, 그 bound가 지나치게 느슨하면 tightness 결과를 음성 결과로 남긴 뒤 개선해야 한다.

추가 분석에서 고정 m에 대한 위 tail bound의 내부 최적 scale은, `2(m+1)>hρ`일 때

\[
\beta_* = \rho/[2(m+1)-h\rho]
\]

로 얻어진다. 기존 `ρ/β≤16` cap을 유지하는 constrained 후보부터 검토하는 것이 타당하다. 반대 조건에서 이 특정 bound로 W=Σ||w_k||/k!보다 작은 상한을 얻지 못한다는 것은 Laguerre 알고리즘 자체의 불가능성 정리가 아니다. 이 scale 분석은 **사후 분석 제안이며 새 실행·승격 근거가 아니다**. `polynomial/ANALYTIC_ADDENDUM_KO.md`에 따로 보존했다.

비-Arnoldi 방향은 Chebyshev/Laguerre를 먼저 유지하고 Leja/Newton 또는 scaled Taylor/block-phi를 별도 backend 후보로 추가하는 편이 좋다. p(A)v가 수학적으로 Krylov 공간에 속한다는 것과 Arnoldi 직교화를 수행한다는 것은 다르다. 문헌 선택과 상세 도메인은 [연구 방향 메모](literature/RESEARCH_DIRECTION_KO.md)에 있다. NIST DLMF의 정확식은 recurrence rounding 증명을 대신하지 않으며, 최근 block-phi 논문의 parameter heuristic도 곧바로 엄밀 total certificate가 되지 않는다.

## 6. Homotopy: 인증 자체의 정확성과 총비용을 함께 낮춰야 한다

정상 witness `U≥|W^{-1}|`와 정확한 sequential quadratic target이라는 전제 아래, serial outward stage-error recurrence는 수학적으로 타당하다. 이 stage target 오차는 ODE의 local/global truncation error가 아니다. 업데이트가 native q2에서 diagnostic eighth W batch를 없애 7 batches로 바꾼 사실도 지지된다. q1은 여전히 operational gate이며 q2와 같은 certificate로 표기하면 안 된다.

### 6.1 공개 witness가 검증된 증명 객체로 유지되지 않는다

공개 mutable/deserialize 가능한 witness의 identity는 그대로 두고 upper matrix만 zero로 바꾸거나 inner row를 비워도 certificate가 성공한다. 새 native 8-stage 반례에서는 stage bound가 0인데 첫 실제 stage error는 정확히 `140737488355328/2281627374017361`≈0.06168294다. 정상 constructor control은 이를 포함한다.

이 결과는 caller가 `U≥|W^-1|` 전제를 깨도 공개 검증 경계가 잡지 못한다는 뜻이다. 정상 생성 witness에 관한 수학 정리를 반박하거나 현재 built-in q2 campaign이 잘못 admit했다고 입증한 것은 아니다. `UncheckedWitness`와 private-field `VerifiedWitness`를 나누고 shape/nonnegative/finite 검증 및 수학적 재검증을 거쳐야 한다. 단순 content hash만 덧붙여도 caller가 다시 만들 수 있으므로 증명 전제 자체가 확보되는 것은 아니다.

### 6.2 세 번의 doubling은 8-stage까지만 충분하다

strict-lower H에 대해 H^s=0이면

\[
(I-H)^{-1}=\sum_{j=0}^{s-1}H^j,
\quad
\prod_{\ell=0}^{L-1}(I+H^{2^\ell})=\sum_{j=0}^{2^L-1}H^j,
\quad L=\lceil\log_2s\rceil.
\]

현재 공개 generic target은 s=9,16도 받지만 구현은 세 번만 doubling한다. 정확근이 Ki=i+1인 chain에서 output bound는 둘 다 8이고 정확 출력은 9,16이다. native s=8 control과 serial recurrence는 통과한다. 단계수에 맞춰 L을 계산하거나 지원 범위를 명시적으로 거절하면 된다. 새 adaptive-depth Fraction 후보는 여섯 stage-count fixture에서 유한 Neumann identity를 정확히 만족했다.

### 6.3 순차 loop를 가속할 구체적 후보

J와 U가 diagonal이면 stage-major H를 물리성분별로 재정렬해 n개의 s×s block으로 분리할 수 있다. 이 독립 block들은 각각 doubling 또는 triangular substitution으로 계산한다. 새 Fraction 후보에서 n=1,2,4 모두 full/분리/serial 값이 정확히 같았다. s=8,n=4에서는 storage 원소 수가 1024→256, dense-product 형식 곱셈 수가 32768→2048로 줄었다. 이는 구조·연산량 결과이며 wall-time speedup 측정이 아니다.

현재 실제 q2 consumer는 serial `certify_stage_target`을 호출한다. 연구용 doubling이 production q2 critical path를 이미 줄였다고 해석하면 안 된다. 먼저 persistent pool, 구조화 witness 표현, 공통 W/preconditioner reuse, block RHS/JVP, 실제 allocation bytes 계측을 넣는 것이 좋다. 현 `stored_values`가 비영 원소 수와 실제 n² allocation을 혼동하는 것도 구분할 필요가 있다.

대략 한 W-vector solve 비용을 c_W로 두고 q1 선택 비율 p1, fallback 비율 pf, s=8, P≥8이면 가속에 남는 overhead 예산은 이상화된 모델에서 `(1+p1−8pf)c_W` 미만이다. q1=0,pf=0이면 certificate·추가 RHS·pool·scheduling 모두에 solve 한 번분밖에 남지 않는다. fallback이 많거나 W solve가 싼 작은 문제에서는 parallelism만 늘려도 빨라질 이유가 없다. 세부 비용식·반지름 closure 증명은 `homotopy/THEORY_KO.md`에 있다.

추가 lambda continuation, componentwise radius, banded/sparse certificate는 그 다음 후보로 두되, 각 lambda의 순차 의존성·iteration 증가·실패 비용을 함께 계상해야 한다. 임의 ODE callback은 몇 점의 RHS/JVP 일치만으로 quadratic family와 전역적으로 같다고 보장할 수 없다. 동일 typed model에서 RHS/JVP/certificate를 생성하거나 tube 내 model defect bound가 필요하다.

## 7. 통계: 데이터 무결성, Monte-Carlo 안정성, 모집단 coverage는 다른 gate다

### 7.1 문서상의 HOLD가 실제 판정에 전달되지 않는다

원래 malformed raw case 다섯 가지는 모두 거절됐다. 그러나 올바른 형식의 여섯 session/한 case 합성 receipt에 대해 `verified_decision`은 `Ok(Promote)`를 반환했다. 게시된 warm-Chebyshev receipt도 같은 값을 반환한다. closure 문서는 모든 timing 결정을 `STATISTICAL_AUTHORITY_HOLD`로 보류한다고 선언한다. 이 상태를 검사하는 enum/필드가 consumer의 유일한 gate로 전달되지 않는다. CLI가 이 값을 `Passed`로 매핑하는 것은 소스 경로 확인이며, 이번에 새 end-to-end CLI timing을 실행했다는 뜻은 아니다.

해결은 raw numerical decision을 없애는 것이 아니라 `(decision, authority, reason, design_version)`을 분리해서, 모든 소비자가 authority hold일 때 `NotEvaluated`/보류를 유지하게 하는 것이다. 기존 PASS ledger L-0009를 삭제하거나 다시 쓰지 말고, 이후 authority 판단을 append-only로 연결한다.

### 7.2 병합 receipt의 일관성만으로 각 session을 검증할 수 없다

뒤 session의 warmup을 0개 또는 1개로 줄여도 통과했다. 한 session에서 candidate 또는 reference sample을 하나 빼고 다른 session에 더해 29/31개로 만들면 전체 병합 벡터가 같아서 재계산을 거친 뒤에도 `Promote`가 나왔다. `end_before_start` provenance도 통과했다. 반면 wrong batch는 거절되고 missing case는 Inconclusive가 되어 controls를 만족했다.

각 session/case/arm에서 warmup 최소 수, candidate/reference 길이, protocol pair 수, label/order/batch를 먼저 검사하고 그 뒤에 병합해야 한다. 재계산된 aggregate는 이 지역 검증의 대체물이 아니다. 이 결과는 unauthenticated JSON을 암호학적으로 진짜라고 주장하라는 요구가 아니라 내부 구조·완전성의 검증 누락이다.

### 7.3 독립 세션 수를 존중하는 정확 구간 후보

case c의 session-cell median log speedup을 Z_sc라 하고, 독립·동일분포인 완전한 session 벡터 Z_s를 가정한다. 관심량은 고정된 corpus의 각 population median m_c를 다시 median한 theta다. case 사이 상관은 허용한다.

\[
q(S,k)=2\,2^{-S}\sum_{j=0}^{k-1}\binom Sj,
\quad q(S,k)\le\alpha/C.
\]

이 조건을 만족하는 최대 k를 택해 case별 `[Z_(k),Z_(S−k+1)]`를 만들고 lower/upper의 case median을 취한다. union bound와 median의 좌표 단조성으로 coverage≥1−α다. k가 없으면 무한 구간을 반환한다. 이 construction은 S=6,C=1에서 coverage 31/32, S=6,C=5에서 무한 구간, S=8,C=5에서 동시 coverage 하한 123/128을 준다. 마지막 세션 수 조건은 이 construction의 성질이며 모든 통계 방법의 불가능성 정리가 아니다.

후보는 32 design, 6 exhaustive sign enumeration, 12 decision control, 4 invalid-input check를 통과했고 독립 reviewer가 다른 계산으로 32 design을 재현했다. 하지만 일반 분포에서는 기존 pooled-pair estimand와 다르므로 versioned estimand를 먼저 합의해야 한다. OS process ID가 다르다고 iid가 증명되는 것도 아니다. 실패 session 제거, outcome-dependent inclusion, data를 보며 세션 수 증가, 여러 후보 선택에는 이 정리를 그대로 쓰지 못한다. candidate의 exact 부분은 조합확률이며 binary64 log/threshold rounding까지 인증한 것은 아니다.

기존 C=5 coverage가 보수적이라는 결과도 arbitrary five-case 집합의 보장은 아니다. 다섯 case의 session response가 동일하면 case resampling이 무효가 되어 one-case bootstrap 법칙으로 돌아간다. 또한 현재 simulator는 case effects를 replication마다 다시 뽑으므로 여러 고정 case-effect vector를 별도로 얼려 검증하는 것이 다음 설계에 필요하다.

## 8. 기존 측정에서 읽을 수 있는 것

| 상속 arm | speedup point | 기존 판정 | 현재 해석 |
|---|---:|---|---|
| HOM06 P=1 | 약 0.174 | Block | 비교한 corpus에서 더 느림 |
| HOM06 P=2/4/8 | 약 0.072 / 0.048 / 0.037 | 모두 Block | thread 증가로도 이득 없음 |
| POLY03 cold Chebyshev | 약 0.565 | Inconclusive | 계수 setup 포함 이득 입증 없음 |
| POLY03 warm Chebyshev | 약 3.166, 구간 [1.167,13.414] | raw Promote | 고정 소규모 대칭 연산자·계수 reuse 조건; statistical authority HOLD |
| POLY03 cold/warm Laguerre | 약 0.122 / 1.180 | Block / Inconclusive | 이번 데이터로 Laguerre 우위 주장 불가 |

모든 8 receipt는 현재 verifier로 재생됐다. 새 timing 실험은 하지 않았다. POLY03가 timing한 것은 `joint_phi_action_unbounded` 경로이며, 전체 outward certificate 경로의 속도로 바꾸어 읽으면 안 된다. HOM06은 약 5배 RHS work와 step마다 pool 생성 비용을 지불했다. 두 coverage 연구의 실패도 이미 공개된 음성 결과이므로 새 결함 수에 중복 집계하지 않았다.

## 9. 구체적인 다음 개발 순서

작은 patch를 완료한 뒤 그 patch가 닫는 invariant를 따로 검사하는 순서를 권한다. 모든 항목의 target file, dependency, 구현 내용, acceptance 및 중단 조건은 `NEXT_DEVELOPMENT_DAG.json`에 있다. 이 문서의 제안은 아직 생산 코드에 반영하지 않았다.

1. **authority와 공개 검증 경계부터**: timing hold를 type/consumer로 전달하고, session raw 완전성 검증을 병합 전에 수행한다. witness는 checked/unchecked 타입을 분리한다. 이 단계의 성공 조건은 손상된 입력이 typed refusal/NotEvaluated로 끝나고 정상 controls가 유지되는 것이다.
2. **정확한 기하와 상한의 국소 수정**: BDF 실제 step 비율, Radau1 unequal split, ExpBound ZERO order, reciprocal factorial, signed stored input/tolerance를 각각 고친다. 위 exact witnesses와 단위/순열 변환을 gate로 삼는다. tolerance를 넓혀 결과를 맞추지 않는다.
3. **후보를 좁게 이식**: scaled norm, stage-count-aware doubling, diagonal factorization을 standalone exact 결과와 비교하며 이식한다. allocation/work counters와 certificate authority가 같은 계약을 유지해야 한다. q1/q2와 stage/ODE 오차를 합쳐 쓰지 않는다.
4. **통계 설계 버전을 먼저 고정**: 기존 pooled-pair 또는 새 session-cell estimand 중 목적에 맞는 것을 명시하고, 새로운 interval의 조건과 fixed-case coverage 설계를 사전등록한다. HOLD 해제는 별도 review gate다.
5. **그 뒤 성능 실험**: cold/warm×certified/unbounded×dimension/sparsity×JVP 비용을 분리한 matched-accuracy corpus를 사전등록한다. pool/계수/domain 검증/캐시 miss/거절/fallback/실패 비용 모두 포함한다. P=1/2/4/8이 유익하지 않으면 음성 결과로 종료한다.
6. **범위를 넓히는 연구**: Laguerre rounding 및 beta-degree 선택, Leja/Taylor block phi, typed quadratic/model-defect certificate, banded 구조를 순서대로 검토한다. 일반 비정규·비자율·mass-matrix 문제나 시간 전체 parallelism은 별도 전제·검증이 필요하다.

다음 성능 실험의 go/no-go는 “관측값이 빨랐다”가 아니라, 정확도/출력 계약 통과, complete failure-preserving receipt, 통계 authority, 전체비용 gate를 모두 통과했는지다. 이번 연구 후보의 유한 PASS를 그 승격 근거로 전용하지 않는다.

## 10. 재현과 기계 판독

- `PREREGISTRATION.md`와 lane `CONTRACT.md`: 실행 전 계약과 범위.
- `FINDINGS.json`: 새 결함, 인정된 잔여 문제, safe rejection/개선점을 구분한 통합 근거.
- `CLOSURE_MATRIX.json`: 이전 21 node의 재감사 상태와 claim ceiling.
- `RESEARCH_CLAIMS.json`: 수학 명제·유한 검산·native 결과·미실행 제안을 구분.
- `NEXT_DEVELOPMENT_DAG.json`: dependency 순서, 구현 및 acceptance/중단 조건.
- `AUDIT_BUNDLE.json`, `AUDIT_BUNDLE.schema.json`: 통합 machine-readable entry point와 schema.
- `evidence/runtime/`, lane raw JSONL/receipt/oracle: 실제 실행 증거와 미실행 목록.
- `decision/`: 독립 재생 및 최종 authority 판정.
- `MANIFEST.sha256`, `VALIDATION.json`: 게시 파일의 무결성 및 repo 규칙 검증.

재현 환경은 `REPRODUCE.md`에 설명했다. 등록된 원본 소스·계약은 사후 수정하지 않는다. dependency 경로만 임시 복사본에서 바꿀 수 있으며 그 변경도 기록한다. 전체 compile/run 결과가 없는 부분은 NOT_RUN/미확정으로 유지한다. README와 본문의 숫자는 원시 파일에서 추적할 수 있고, 최종 remote tree 확인은 게시 receipt에서 별도로 기록한다.
