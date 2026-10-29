# VigilODE 범용 ODE 선택 구성: 재감사와 개발 보고서

## 판정

**선정된 범위의 사용 후보로는 DOPRI5와 보호된 RODAS5P를 유지한다. 임의의 강성·비선형·고속 진동 문제에 대한 포괄적 production 적합성과 보편적 속도 우위는 아직 입증되지 않았다.** 이는 사용 가능한 수치 엔진이 없다는 뜻이 아니다. 이번 변경은 지원 범위를 좁혀 명시하고, 실제 인터페이스 결함을 고치며, 검증되지 않은 방법이 기본 경로에 자동 진입하지 않도록 한다. 최종 실행 수와 측정값은 [VALIDATION.json](VALIDATION.json), [SESSION_BENCHMARK.json](analysis/SESSION_BENCHMARK.json), [CORE_SURVEY.json](analysis/CORE_SURVEY.json)에 기록한다.

통상의 ODE 솔버처럼 국소 오차 추정에 기반한 허용오차 계약을 제공할 수 있다. 임의 RHS에 대한 전역오차 인증, PDE 공간 이산화, 특정 물리 모형, DAE, 이벤트 또는 연속 출력의 완성을 endpoint 전용 일반 ODE 인터페이스의 필수조건으로 부과하지 않는다. 이번 보류는 현재 소스가 약속하는 폭넓은 수치적 강건성과 성능에 대한 증거의 한계를 가리킨다. 명시적으로 거절하는 기능은 지원 결함과 구분한다.

외부 독자용 사양은 별도의 [설계문서](../../docs/general_ode_design_20261024/DESIGN_KO.md)와 [PDF](../../docs/general_ode_design_20261024/DESIGN.pdf)로 제공한다. 그 문서는 선택된 알고리즘의 동기·수학·구현 계약만 설명한다. 이 보고서는 비교·실패·연구 후보의 처분 및 다음 개발 업무를 다룬다.

## 조사한 소스와 근거 재사용

197개 원격 브랜치의 이름과 tip을 기록하고, 최신 소스 및 별도 범용 개발 계보를 대조했다. 기준 커밋은 `057fecf0698fa262d881f101625e6b0bb93791cf`다. 그 native solver 코드는 `1cdf2c3071498888f2d8ff3ddc0629cb3cf443ef`의 후속 개발과 같다. 별도 VigilODE 개발 tip `c03ca90f19a9e64ba7751c069325a48b54b39c2e`도 기준 소스의 조상에 포함되어 있으므로 코드를 다시 복사하거나 중복 병합하지 않았다. 갈라진 readiness 브랜치의 준비 자료는 별도로 검토했으며, 이번 일반 ODE 범위와 관계없는 응용 연구를 가져오지 않았다. 자세한 경로·판정은 [BRANCH_AUDIT.json](analysis/BRANCH_AUDIT.json)에 있다. 브랜치 목록을 수집했다는 사실을 모든 브랜치의 모든 파일에 대한 완전 감사로 해석하지 않는다.

기존 연구를 다시 실행해 새 성능 결과로 만들지 않았다. 알고리즘군 25개의 판단은 내용 해시로 고정한 기존 자료 13개에 연결한다. [PORTFOLIO.json](analysis/PORTFOLIO.json)과 재현 검사기가 이 연결을 확인한다. 최신 후속 개발의 긴 진동 실험은 2차원 동차 선형 세 문제에서 100/1000주기와 4,097개 관측점을 검사했다. 보충 실행 36행이 표본 상태·위상·진폭 기준을 만족했지만 timing은 여전히 불확정이다. 이를 Lawson의 일반 속도 이득으로 승격하지 않았다.

Rust 1.94.1과 정확한 잠금 의존성을 제공된 파일에서 복구했다. vendor 118개와 파일 해시 4,382개, offline metadata의 125개 패키지를 확인했다. 새 검사는 이 환경에서 수행했다. NCP를 사용하지 않았다. 변경된 경로의 회귀 검사와 새 세션 비용 측정은 필요하므로 수행하되, 관련 없는 기존 대규모 캠페인을 다시 돌리지 않았다.

## 분기 탐색과 가지치기

세 정책을 분기한 뒤, 기능 계약·정확도·실패·총비용을 순서대로 심사했다. 이것은 유한한 근거 기반 tree search다. 무작위 rollout을 수행한 MCTS나 전역 최적 정책을 찾았다고 주장하지 않는다.

| 정책 | 판단 | 이유 |
|---|---|---|
| 작은 explicit pilot의 작업 압력으로 즉시 implicit 전환 | 기본 경로에서 제외, 최적화 연구 유지 | 제공된 JVP의 일부 이득과 달리 유한차분·전이 문제에서 2배 이상 손실이 있었다. 재시작·탐색 비용과 미래 구간 길이를 무시할 수 없다. |
| 문제 선언·미분 허가·구조·자원 제한에 따른 두 방법 선택 | 이번 기본 구성 | 비용과 지원 범위를 실행 가능한 검사로 명시하며 기존 엔진을 사용한다. |
| 샘플에서 보인 구조만으로 Lawson·causal·다항식·homotopy 자동 선택 | 특수 선언 또는 연구 경로로 제한 | 구조 증명, 실제오차, 강한 비교 대상에 대한 이득을 동시에 충족한 일반 근거가 없다. |

`GeneralOde`는 기본적으로 RHS-only DOPRI5를 선택한다. 사용자가 강성을 선언하면 미분 허가를 검사한 뒤 RODAS5P를 선택한다. `ProvidedOnly`는 제공된 Jacobian/JVP와 비자율 문제의 시간 편미분을 요구한다. `AllowFiniteDifference`만 유한차분을 허가한다. JVP/GMRES는 별도 허가가 필요하다. 조밀 행렬 생성·분해의 기본 차원 제한 64는 수정 가능한 자원 상한이며, 최적 전환 차원의 실측값이 아니다. compact band와 banded FD는 이 조밀 상한에서 제외되지만, dense callback에 band를 선언한 경우에는 조밀 생성 비용이 있어 상한을 적용한다.

새 선택기는 적분 중 자동으로 방법을 바꾸지 않는다. `Unknown`은 강성이 없다는 판정이 아니라 예산이 제한된 explicit 시도다. 실패를 강성의 증명이나 미분 사용 허가로 해석하지 않는다. 이벤트·질량행렬·연속 출력 요청은 선택 전에 거절한다. 기존 방법별 API는 보존한다.

| 알고리즘군 | 개발 처분 |
|---|---|
| DOPRI5, 보호된 dense/banded RODAS5P | 기본 구성 유지, 실제 계약 회귀 검사 |
| 제공된 JVP/GMRES | 명시적 허가와 지원 조합 아래 특수 경로 유지 |
| 정확한 affine/harmonic, 작은 고정 차원, 독립 ensemble | 선언된 적용 범위에서만 유지; 기본 선택으로 자동 진입하지 않음 |
| causal-forward | 제한된 구조·비용 영역의 명시적 실험; 약한 coupled 비교군에 대한 이득을 주력 엔진 우위로 전용하지 않음 |
| switching pilot | 지속 세션 통합과 비용 상환 검증 후 재평가 |
| Lawson 일반 RHS, homotopy, SABR, 근사·지연 Jacobian 후보 | 현재 기본 선택에서 제외; 강한 비교 기준과 새로운 구체적 근거가 있을 때만 재진입 |
| Laguerre/Chebyshev/Taylor, 저차 resolvent | 구성요소 연구 또는 정확한 구조 선언으로 제한; operator action 감소와 전체 실행시간 이득을 구분 |
| Leja, 비수축 Neumann, 저비용 RHS의 병렬 Picard | 시험된 영역의 자동 채택 제외; 조건 또는 실측 이득이 바뀌지 않으면 주력 개발 시간을 추가 투입하지 않음 |
| 내부 BDF1/2·Radau1/3 | 참조용 유지; VODE/DVODE·diffsol·Julia의 경쟁 성능을 대변하지 않음 |

가지치기는 자동 선택과 개발 우선순위의 조정이다. 과거 코드·실패 기록·재현 자료를 삭제하지 않았다. 서로 다른 소스·호스트·실제오차의 speed ratio를 평균하여 하나의 순위로 만들지 않는다.

## 실제 수정한 사항과 적대적 검사

### 불완전한 RHS 출력의 재사용 차단

암시적 native 경로는 출력의 유한성은 검사하지만 재사용 벡터를 매 호출 새로 표시하지 않았다. 따라서 callback이 일부 성분을 쓰지 않고 `Ok`를 반환하면 초기 0 또는 이전 stage 값이 유효한 결과처럼 통과할 수 있었다. 새 `Guard::vector_call`은 호출 전 출력에 NaN을 채우고 모든 성분의 유한한 쓰기를 요구한다. 실패한 `Ok`는 상세 정보를 가진 terminal callback error가 된다. RHS, 유한차분 내부 RHS, 시간 편미분, JVP가 같은 규칙을 사용한다. 비유한 입력은 사용자 함수에 전달하기 전에 거절한다. Jacobian의 문서화된 sparse-write 방식은 zero-fill 계약을 유지한다.

첫 호출·이후 호출의 출력 누락, analytic-Jacobian·FD 경로, 불완전한 시간 편미분, 정상적인 sparse Jacobian을 회귀 검사한다. 이 수정은 벡터 쓰기·검사 O(n)을 추가한다. correctness 검사를 생략한 이전 속도를 새 보호 경로의 비용이라고 광고하지 않는다. native API를 직접 사용하는 모든 실험이 이 façade의 callback 계약을 자동으로 얻는 것은 아니다.

### 유한 입력으로부터 넘친 오차 스케일 거절

`atol + rtol max(|y_n|, |y_{n+1}|)`가 infinity로 넘치면 유한 오차를 나눈 값이 0이 될 수 있다. fast RODAS의 scalar/vector 두 경로에 유한성 검사를 추가했다. 이는 DOPRI5·공통 norm과 계약을 맞추는 수정이다. 이 사례만으로 허용오차를 어긴 수락이 입증되었다고 주장하지 않는다. 상수 상태와 극단적인 유한 허용오차의 경계 검사 및 정상적인 대조군을 함께 둔다.

### 지속 세션과 관측 시각 검색

`Dopri5::with_session`은 scoped lifetime 안에서 accepted state, FSAL, controller, 전체 시도 예산과 출력 순서를 유지한다. callback을 소유한 객체가 그 내부 callback을 다시 빌리는 self-reference나 unsafe를 도입하지 않는다. quota가 소진되면 마지막 수락 상태를 빌려 읽고 재개할 수 있다. 활성 세션의 잘못된 target·epoch은 작업 없이 거절한다. 종료된 세션은 epoch을 확인한 뒤 target을 다시 검사하지 않고 종료 결과를 유지한다. fatal은 이후 callback을 다시 호출하지 않는다. 일찍 closure를 반환하면 실패 원인이 없는 incomplete partial solution을 돌려준다. 이것을 성공한 전체 적분으로 표시하지 않는다.

일련의 관측점마다 solver를 다시 만드는 대신 동일 native session을 재개하면 같은 일정에서 수치 경로를 유지할 수 있다. 정렬된 일정에서 target 존재 여부를 매번 선형 검색하던 부분도 이진 검색으로 바꾸었다. M개의 관측점을 차례로 요청할 때 membership 비교량은 O(M²)에서 O(M log M)으로 감소한다. 이는 적분 전체 복잡도가 그 비율로 줄어든다는 뜻은 아니다. signed zero의 기존 동등성과 비유한 target 거절을 보존한다.

## 수학 연구 결과와 보장 범위

1. **DOPRI5의 실제 계수 검증.** 코드의 유리수 계수를 직접 읽어 5차까지 rooted-tree 조건 17개와 내장 4차 조건 8개를 exact arithmetic으로 검사했다. 별도 Wolfram 계산으로 안정 다항식과 내장 추정기의 차이를 확인했다. RODAS5P는 실제 구현상 8단계다. 저장된 소수 계수의 scalar series 검사는 수행했지만, 이것을 모든 비선형 order condition의 새 증명으로 부풀리지 않는다.
2. **선언된 주파수의 명시적 오차 상계.** `y'=iωy`, `|ω|h_max≤0.9`에서 DOPRI5 안정 다항식의 수축성과 Taylor remainder를 이용하여
   \[
   |e(T)|\le |y_0|T|\omega|^6h_{\max}^5
   \left(\frac1{3600}+\frac{0.9e^{0.9}}{5040}\right)
   \]
   를 유도했다. 정상 skew-Hermitian 선형 연산자에도 단위적 대각화로 확장된다. 비정규·비선형·샘플로 추정한 주파수에는 그대로 적용하지 않는다. 현재 선택기에 이 별도의 step-cap 계약을 구현했다고 주장하지 않는다.
3. **방법 전환의 충분조건.** 실제 선택된 각 one-step map의 안정성 `1+Lh`, 국소 결함 `C_m h^{p+1}`, 전환·roundoff·부정확 선형해의 추가 교란 `η_n`이 공통 근방에서 성립하면
   \[
   E_N\le e^{LT}\{E_0+C_{max}T h_{max}^p+\sum_n\eta_n\}.
   \]
   같은 방법을 exact state에 적용해 차를 빼고 이산 Grönwall 부등식을 적용하면 된다. 선택 규칙 자체가 매끄러울 필요는 없지만, 방법별 가정과 cache 무효화는 필요하다. 이는 아직 구현하지 않은 자동 switching의 무조건적 인증이 아니다.
4. **재개 불변성과 작업량.** deterministic RHS, 같은 전체 native state·일정·허용오차·부동소수 환경에서 pause가 상태를 바꾸지 않으면 다음 transition이 동일하다. 귀납법으로 trial과 출력·작업 계수가 일치한다. FSAL을 쓰는 완전한 DOPRI trial A개의 RHS 수는 `1+6A`다. K번 독립 restart와 비교한 차는 `6(A_restart−A_persistent)+(K−1)`이다. trial 수가 같으면 정확히 `K−1`개의 시작 RHS와 재구성을 피한다. 실제 adaptive trial 수는 달라질 수 있으므로 보편적 wall-time 이득은 따로 주장하지 않는다.
5. **특수 다항식의 계산량 정리.** `q(B)=0`, `q(1)≠0`이면 `q(1)−q(z)=(1−z)r(z)`를 이용해 `(I−B)^{-1}=r(B)/q(1)`이다. 차수 d의 관계가 주어지면 Horner 평가에 d−1회 operator action이면 충분하다. 그 관계를 모르는 일반 연산자에서 자동 적용할 수 있다는 결론은 아니다. 현재 기본 구성으로 넣지 않았다.

전체 유도, exact checker, 외부 계산의 원본 응답과 한계는 [NUMERICS.md](analysis/NUMERICS.md)와 [PORTFOLIO.md](analysis/PORTFOLIO.md)에 있다. SciSpace 검색 결과는 원전 검증과 동일시하지 않았다. Academic Writing Toolkit은 최종 외부 문서의 문단·인용을 점검하는 데 사용했으며, 수학적 정확성의 자동 인증으로 사용하지 않았다.

## 새 검증과 측정의 읽는 법

선택기·세션·callback·스케일 변경에 필요한 Rust 회귀 검사와, 동일 관측점을 가진 세션 microbenchmark 및 새 일반 ODE survey를 구분한다. 모든 새 실행은 단일 스레드 수치 연산으로 제한한다. 상태·실제오차·실패를 보존한다. 초기 컴파일 실패도 삭제하지 않는다. 최종 source digest와 binary hash, 명령·exit code는 실행 영수증에 있다.

세션 측정은 scalar decay, 32차원 diagonal decay, 2차원 회전에서 native one-shot, façade one-shot, persistent observation delivery, 관측점별 restart를 비교한다. 앞의 세 경로는 상태와 작업량이 동일해야 한다. restart는 자체 adaptive 경로를 유지하되 같은 실제오차 기준을 통과해야 한다. cyclic 순서와 수 밀리초 단위 grouping으로 짧은 단일 실행의 잡음을 줄인다. 이 비교는 관측 전달 비용을 격리하며 VODE/DVODE·diffsol·Julia보다 빠르다는 근거가 아니다.

세션 microbenchmark는 성공 시 전체 행을 마지막에 출력하는 assert 기반 하네스다. 향후 실패하면 stderr와 종료 코드는 보존되지만 구조화된 부분 측정행은 남지 않을 수 있다. 이번 측정은 모두 exit 0이었다. fatal·recoverable·예산·partial-state 계약은 별도 회귀 시험으로 검증했으며, 다음 대규모 하네스는 행 단위 즉시 기록으로 바꾸어야 한다.

새 core survey는 analytic/manufactured 해를 가진 비강성, 강성, 비정규, 진동·감쇠 및 비선형 사례를 여러 tolerance에서 검사한다. 성공한 endpoint 검사는 연속시간 최대오차 보장이 아니다. 이 작은 survey를 광범위 외부 work–precision 캠페인의 대체물로 사용하지 않는다. 외부 비교군의 새 실행은 이번에 하지 않았다.

## 통합 실행 결과

| 검증 | 결과 | 해석 |
|---|---:|---|
| 범용 façade 단위·통합 시험 | 154 통과 | selector, 세션, 기존 callback·FD·event·output 계약 포함 |
| 관련 native 회귀 시험 | 44 통과 | 두 native 세션, explicit, dense·band·vector tolerance |
| native 스케일 경계 시험 | 1 통과 | scalar/vector overflow 거절과 정상 대조군 |
| 문서 예제 | 2 통과 | 실행 예제와 compile-fail 계약 |
| 일반 ODE survey | 24/24 통과 | 8문제 × 3 tolerance, 사전 고정한 endpoint 기준 |
| 세션 측정 | 24/24 모드·일정 조합 통과 | 3문제 × 4방법 × 2관측 일정, 동일 실제오차 기준 |

새 세션의 restart 대비 중앙값 배수는 약 **1.58–3.08**이다. 이미 one-shot을 쓰는 경우보다 약 **1–17%** 느리므로, 반복 관측·재개에 한정한 개선이다. 65개 관측 scalar의 timing spread가 약2배였다는 점도 보존한다. 상세 시간·RHS·오차는 [측정 표](analysis/SESSION_BENCHMARK.md)와 원시 samples에 있다. 24개 core survey의 error cap은 `100*(atol+rtol*max_abs_reference_endpoint)`이며, 그 기준 자체가 global tolerance 보장이라는 뜻은 아니다. 모든 사례에서 허용오차 감소에 따라 endpoint 오차가 감소하는 경향도 확인되었다.

컴파일 중 trait-object coercion·builder 호출 오류를 수정했다. native 재컴파일 후 발생한 `E0463` artifact 문제는 같은 task의 `vigilode` build cache만 정리하고 단일 build job·incremental 비활성으로 재빌드하여 해소했다. 소스·잠금 의존성을 바꾸지 않았다. 실패 영수증을 성공 결과로 덮어쓰지 않았다. 새 측정은 numerical source18개 입력 파일의 해시 고정 이후 수행했고, 실행 후 일치를 다시 확인했다.

## 다음 개발 단계

구체적인 의존성·완료 기준·중단 기준은 [NEXT_STEPS.json](NEXT_STEPS.json)에 있다.

**첫째, 현재 선택 구성의 범위를 고정한 release 검증을 수행한다.** 새 current-source survey에 더해 Robertson, Van der Pol의 여러 강성 계수, 비정규 과도 성장, 시간 의존 Jacobian, 주파수 변화, 장시간 감쇠를 사전 등록한 corpus로 확장한다. reference uncertainty를 오차 예산의 10% 아래로 두고 endpoint뿐 아니라 공통 관측점의 실제오차·위상·진폭을 검사한다. 비교군은 VODE/DVODE, diffsol, Julia로 제한한다. 외부 dependency가 준비되지 않으면 NOT_RUN을 보존한다. NCP를 사용하지 않는다.

**둘째, implicit façade의 지속 세션을 완성한다.** 기존 native RODAS session이 있다고 public session 지원이 완료된 것은 아니다. Jacobian/provider의 수명, FD 작업, fatal latch, partial state, tolerance와 route를 동일하게 보존해야 한다. one-shot과의 bitwise 또는 수치적으로 정당화한 동등성·작업 계수를 먼저 맞춘다. 이 작업은 반복 관측·선택적 switching의 비용을 줄이는 구체적인 최적화이며 endpoint-only 사용의 필수 기능은 아니다.

**셋째, switching은 비용을 상환할 구간이 있는 경우에만 연다.** 같은 정확도에서 구간별 explicit 비용 E, implicit 비용 I, pilot/startup 비용 P가 유지된다면 이득 조건은 `E>I` 및 `K>P/(E−I)`다. pilot과 버린 시도도 P에 포함한다. hysteresis·cache 무효화·accepted-state 전환을 구현하고, control 문제의 큰 손실을 막는 사전 선언 기준 아래 평가한다. 관측된 재사용 효과만으로 자동 stiff detection을 완료했다고 표시하지 않는다.

**넷째, profile로 확인한 메모리 이동과 작업만 줄인다.** explicit의 stage-vector loop, poisoned-output 쓰기, band 구조의 storage/factorization, JVP 재개를 따로 측정한다. error/callback 검사를 제거해 얻은 숫자는 채택하지 않는다. approximate Jacobian·다항식·homotopy 후보는 원 residual과 전체 trajectory accuracy를 만족하면서 가장 강한 적용 가능한 baseline을 이길 때만 특수 선택으로 복귀시킨다. 단일 trajectory의 latency와 독립 ensemble throughput은 별도로 다룬다.

## 자기비판과 남은 한계

선정은 보수적 선언 기반이며 자동 stiffness detector가 아니다. implicit의 derivative-free·matrix-free·Newton-free는 서로 다른 속성이다. 현재 강성 경로는 Newton 반복을 하지 않지만 미분 정보 또는 근사를 사용한다. 행렬을 생성하는 dense/banded 경로를 matrix-free라고 표현하지 않는다. 고속 진동 문제는 안정성과 위상 정확도 요구가 다르며 local tolerance 하나로 장시간 위상을 보증할 수 없다.

새 callback 보호는 옳은 계산에 필요한 비용이다. 세션 측정의 이득은 반복 restart를 하던 호출 패턴에 의존한다. 이미 one-shot을 사용하던 프로그램이 같은 배수로 빨라지지는 않는다. 변경되지 않은 과거 측정은 해당 소스의 증거로만 재사용했다. 해시 일치, exact symbolic 검사, Rust test, 실제오차, 타이밍은 각각 다른 질문에 답하며 어느 하나도 나머지를 대신하지 않는다.

이번 연구는 일반 ODE에 한정한다. 별도 물리 client 연구를 새로 수행하지 않았고, 그 성공이나 미완료를 이 솔버의 판정에 섞지 않았다.
