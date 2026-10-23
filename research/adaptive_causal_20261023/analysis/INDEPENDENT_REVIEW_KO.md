# 새 adaptive·causal·quotient 경로의 독립 검토

2026-10-23. 검토자 `adaptive_independent_review`. 기준 commit
`14c74978b93339a79d7228a324fa77552dae548e`에서 추가된 수학과 native 경로를
별도로 읽고, 작성자와 독립적인 exact arithmetic 검산을 수행했다.
확정된 파일 identity와 후속 결과 검토 범위는 `../REVIEW.json`에 기록한다.

## 판정 범위

새 기능을 명시적으로 선택하는 연구·이식 범위에서 채택할 수 있다. 검토한 것은
아래 네 수학 구성과 코드의 연결, 실패 처리, derivative 사용 권한, 실제 작업량이다.
범용 production 승인과 모든 종류의 강성에 대한 성능 우위는 이번 검토의 결론이
아니다. 이전 구면 정보기하 패키지의 과학 계산은 변경되지 않아 재실행하지 않았다.

1. **몫대수 resolvent.** AT1의 polynomial identity는 정확하다. `q(A)=0`과
   `q(1/s)!=0`이면 repeated root와 Jordan multiplicity를 허용하며 inverse를
   degree `d-1` polynomial로 계산할 수 있다. near-annihilator bound에는 실제
   full-space operator norm과 submultiplicative norm을 사용해야 한다.
2. **감쇠·회전 action과 Lawson 변환.** AT2의 inverse, flow, Euclidean contraction은
   직접 2×2 곱과 `R²=-I`에서 얻어진다. 코드가 받는 callback은 잔여항 `N`이고,
   원래 식은 `y'=Ay+N`이다. 일반 RHS를 그대로 넣으면 다른 식을 계산한다.
   원래 변수의 exponential RK 식은 classical DOPRI5의 Lawson 변환과 일치한다.
   비선형 잔여항과 경계 regularity를 포함하는 stiff-uniform order는 별도 조건이다.
3. **Quadratic continuation chart.** AT3의 유리화한 root와 error identity를
   직접 역대입하고 exact rational perturbation으로 확인했다. 코드의 residual
   indicator는 rounding enclosure가 없는 operational 값이다. exact zero·linear
   차수 감소를 먼저 분리하고 unresolved discriminant는 거절하도록 개선했다.
4. **Causal stage elimination.** 현재 raw-U의 `A,C` strict lower 조건과 공통
   invertible `W_m`에서 forward solve는 같은 projected root를 유일하게 구한다.
   동시에 갱신하는 Picard는 앞 stage부터 확정되므로 global contraction을 가정하지
   않고 유한 횟수에 끝난다. 현재 coefficient의 상삼각·대각 누출을 정확히 거절하며,
   이후 기존 full-space residual 검사를 유지한다. frozen JVP와 작은 projected
   matrix는 남으므로 **nonlinear corrector만 Newton/current-stage-JVP-free**다.

독립 script `src/independent_review_checks.py`는 2·5차 strict lower matrix inverse,
강하게 비선형인 triangular recurrence의 finite front, 세 quadratic chart의
perturbed-root identity, 세 rotation inverse를 exact rational로 확인했다.
추가로 90자리 Decimal에서 `1e308 exp(-d)`의 극한 입력 참조를 계산했다.
10개 검산 묶음이 통과했다.

## 발견 사항과 수정

| ID | 관측한 문제 | 영향 | 최소 수정 및 확인 |
|---|---|---|---|
| IR01 | `exp(-1000)`을 먼저 binary64 zero로 만든 뒤 큰 상태에 곱했다 | `y=1e308`에서 참값 약 `5.076e-127`을 0으로 반환하고 pure-flow embedded error도 0이 될 수 있었다 | exponent가 작을 때 half-exponential을 상태에 두 번 적용. `d=1000`과 subnormal coefficient `d=744`의 독립 고정밀 참조 회귀 검사 추가 |
| IR02 | shifted inverse의 합을 denominator로 나누기 전에 계산했다 | `a=v=1`, 입력 `[MAX,MAX]`에서 표현 가능한 결과 `[0,MAX]`를 중간 overflow로 거절 | normalized coefficient를 먼저 denominator로 나눈 뒤 action. `MAX` 회귀 검사 통과 |
| IR03 | `ProvidedOnly`가 state Jacobian만 검사했다 | 비자율 문제에 `df/dt`가 없으면 암묵적인 time finite difference가 실행될 수 있었다 | autonomous 또는 supplied time derivative를 요구. callback 진입 전 음성 검사 추가 |
| IR04 | causal 단계의 8개 실제 backsolve를 기존 completed projected solve 1개와 혼동할 수 있었다 | 단일 스레드 작업량 통계의 의미가 불분명했다 | 기존 `small_solves` 의미를 유지하고 `small_block_solves`를 실제 호출 전에 증가. 공통 `direct_solve_calls`를 새 counter로 연결 |
| IR05 | quadratic root의 exact zero·linear case도 일반 discriminant scaling을 먼저 거쳤다 | `a=1e-200,b=1,c=0` 같은 유효한 linear root가 square underflow 때문에 거절됐다 | exact exceptional chart를 먼저 처리하고 extreme scale 회귀 검사 추가 |

IR01은 결과 정확도 문제였으며, IR02·IR05는 유효한 입력의 불필요한 실패,
IR03은 권한·설정 의미, IR04는 작업량 기록 문제였다. 서로 다른 원인을 같은
solver stability 실패로 합치지 않았다. 수정에 영향을 받는 검사만 다시 실행했다.

## Adapter와 비용 계약

`SwitchingSolver`는 DOPRI와 기존 보호 RODAS 경로 사이의 window adapter다.
원래 드라이버의 local-error 수락을 사용하며 accepted endpoint에서만 전환한다.
실패한 explicit probe의 출력은 경제성 조건을 만족하지 않으면 버리지만, 실행한
attempt·callback·linear solve는 전체 예산과 segment 기록에 남는다. fatal callback은
다른 방법으로 재시도하지 않는다. 기본 설정은 RHS-only이며 FD는 별도 명시적 권한이다.

`LawsonDopri5`는 같은 공개 facade에 추가된 구조 선언형 entry point다.
`SwitchingSolver`가 이 구조를 자동 발견하여 세 방법을 전환하는 구현은 아니다.
causal companion 역시 별도의 opt-in이며 full-space JVP fallback이 기록된다.
이 구분을 유지해야 어떤 비용과 어떤 수학적 계약을 측정했는지 추적할 수 있다.

FD/JVP fallback을 허용한 switching은 window마다 driver workspace를 다시 만들며
h0도 다시 적용한다. 작은 문제에서 이 비용이 커질 수 있고, switching의
deterministic work score는 wall time의 보장이 아니다. 동일 window schedule로
강제 실행한 baseline과 비교하는 것은 adapter overhead 비교에 적합하지만,
최적의 연속 baseline과 동일하다는 뜻은 아니다.

이 비용을 확인한 뒤 기본 `Forbidden` 경로는 하나의 persistent observed DOPRI
session으로 수정했다. 선택 가능한 다른 방법이 없는 설정에서는 각 window에서
재시작할 이유가 없기 때문이다. 새 독립 대조 검사는 같은 observation schedule의
native DOPRI와 출력·accepted partial state·attempt·RHS·FSAL이 비트 단위로 같음을
확인한다. 기존 12개와 추가 2개 switching contract가 함께 통과했다. fallback을
허용한 경로의 코드와 해당 864개 benchmark record는 변경되지 않았다.

## 성능 근거의 별도 검토

최종 quotient 자료의 112개 record와 switching 자료의 864개 record에서 raw hash,
source identity, 측정 반복의 median, 성공·완주 조건, 공통 달성 오차 상한을 만족하는
설정의 선택을 별도 script로 재계산했다. 기존 solver 실행과 VODE 참조 계산은
반복하지 않았다. 큰 구조별 이득과 느린 control 결과를 모두 유지한 구성을 확인했다.
빠른 carrier를 정확히 제거한 결과를 일반 opaque RHS의 가속으로 전이하지 않는다.

검토 중 IR06으로, `causal_benchmark_summary.json`이 새 pinned raw 실행 전에 계산한 median을
담고 있음을 발견했다. 예를 들어 6성분 affine case의 새 raw 시간비는 약 2.389인데
이전 summary에는 1.089가 남아 있었다. 원시 자료를 바꾸지 않고 summary를 새 자료에서
재생성하도록 요청했다. 수정된 12개 비교의 시간비는 raw 자료와 일치한다. 해당 자료의
root 성공 36행 중 6행만 embedded error가 1 이하이며, 나머지는 수락된 ODE step을
의미하지 않는다.

기본 persistent 경로의 후속 324개 실행도 별도로 검사했다. `switch-rhs`와 하나의
native DOPRI 호출을 비교한 108쌍에서 출력 시각·상태·작업량·성공 여부가 완전히
일치했고, 실패한 partial state도 같았다. Jacobian·JVP·time derivative·linear solve는
0회였다. 전체 후속 campaign의 288개 성공과 Prothero의 예산 중단 36개를 구별했다.
이 검토를 포함한 artifact 일치 검사 1,866개가 통과했다. 같은 작업량에도 공유
호스트의 실행 시간은 흔들리므로 이 경우 확실한 개선 근거는 불필요한 restart의
제거와 native session의 정확한 보존이며, 단일 timing ratio가 아니다.

## 남은 개발 의무

- 동일 RHS를 가진 구조 선언형 split의 정확한 대응을 client별로 검증한다.
- switching·Lawson·causal의 workspace 수명을 통합하여 window restart 비용을 줄인다.
- 실제 달성 오차, 장시간 phase/amplitude 오차, 비정상·비선형 stiff forcing을 포함해
  선택 정책을 측정한다. 명목 tolerance만으로 speed ratio를 만들지 않는다.
- 일반 quotient compiler와 noncausal leakage 보정에는 operator witness와 원래 residual
  오차 계약이 필요하다. 현재 구현은 그 기능을 지원한다고 선언하지 않는다.

## 최종 보고서와 판정의 일치

최종 native commit `d4e8623bf8add9f505ec018532c4602e5a838a41`과 이전 측정
commit `548526add83a38a996875b3bd2f3cd3357465ed6`을 구별했다. 변경된 facade
122개 시험을 다시 실행했고, 변경되지 않은 integrator 17개와 coupled 66개 결과를
재사용하여 총 205개 최종 고유 시험의 근거를 확인했다.

통합 보고서의 일반 캠페인은 정규화 관측 오차, quotient 캠페인은 절대 관측 오차를
사용한다고 명시한다. actual BatchedPicard 식, 제한된 derivative-free 범위, 별도 API,
고정 단계 root와 수락된 전체 궤적의 구분도 원시 구현·자료와 일치한다. 빠른 진동에서
JVP 전환의 약 32.5배 손해 및 stiff forcing에서 Lawson의 약 12.16배 손해를 포함해
판정한 `DECISIONS.json`의 범용 production 보류와 구조별 채택은 근거에 부합한다.
`NEXT_STEPS.json`과 계획된 local 비교 캠페인은 아직 실행하지 않은 작업을 완료된
결과로 표시하지 않는다.

이 검토는 실제 독립 계산과 별도 코드 읽기를 수행한 agent 검토다. 형식 증명기나
독립 기관의 인증을 뜻하지 않는다. 최종 검토 범위와 파일 해시는 `REVIEW.json`에
기록한다. 원격 게시 성공 여부는 이 과학·구현 검토와 별도의 영수증으로 확인해야 한다.
