# Fast / lagged / banded integrator 재감사

대상은 `production/wave3-integration`의 `f2f20c1` 코드와 `production/vigilode-plan-20261021`의 `cc2e1a0` 계획이다. 이번 변경은 lagged 경로의 수치 방어, B1d resolvent 후보, 이론의 적용 범위를 수정한다. 보호된 fast 경로의 routing·tableau·허용오차는 바꾸지 않았다. 기존 B1c FAIL은 유지한다.

## 판정

**보호된 fast solver의 제한된 기존 사용은 유지할 수 있다. Lagged solver를 일반 production 경로로 승격할 근거는 아직 부족하다.** 기존 B1c에서 무한대가 되었던 일부 staleness 검사를 구제하는 B1d를 구현했지만 HIRES에서 factorization/accepted=0.4412로 고정 기준 0.4를 넘었다. 또한 실제 추가 LU solve 수가 커서 factorization 감소를 속도 향상으로 해석할 수 없다.

정확도 측면에서는 두 가지 독립적 결함을 특정했다. 첫째, 유한 입력에서도 내부 weight ratio가 overflow하면 `0*infinity=NaN`이 발생하고 `f64::max`가 해당 행을 버려 staleness 추정이 0이 되는 구현 결함을 수정했다. 둘째, nonstiff stage sensitivity가 모든 정상(normal) 좌반평면 스펙트럼에 보수적이라는 설명은 거짓이다. 정상 회전 행렬 하나로 반례를 정확 유리수 계산으로 확인했고 설명을 교정했다. 기존 target 상수는 재현성 때문에 그대로 보존하며 **진단용 target**으로 명시했다.

## Finding I-01: NaN 행 소거로 잘못 작은 staleness 값

`rodas5p_staleness_bound`의 기존 weighted row 합은 `s[k]/s[i]`를 먼저 계산했다. 다음 모두 유한한 입력을 취한다.

- `h=h_P=gamma=1`, `J_P=-I_3`, `J=J_P+E_02`.
- `s=[f64::MIN_POSITIVE, f64::MAX, f64::MIN_POSITIVE]`.

실제 `P=2I`, `W=2I-E_02`이고 `T=I-P^-1 W=E_02/2`이므로 `||T||_S=1/2`이다. 그러나 0번째 행에 있는 `0*(s_1/s_0)`이 NaN을 만들고 이어지는 nonzero drift 항도 NaN 안에 가려진다. 기존 함수는 `q=0`을 반환한다. 원본 함수를 그대로 옮긴 `evidence/integrator/legacy_staleness_reproducer.rs`가 이를 실제 재현한다.

수정은 finite positive ratio·유한 row sum·유한 positive `h_P gamma`·유한 positive denominator를 확인하고 부적합하면 `+infinity`를 반환한다. 빈 행렬도 거부한다. 원본 finite 영역의 연산 순서는 유지한다. 이는 범위를 좁힌 방어이며 outward rounding 전체를 추가한 것은 아니다.

## Finding I-02: 비강성 target의 일반 정상행렬 확장 오류

계수 snapshot으로 `w=(1-gamma z)^-1`, `T_ij=gamma C_ij w+A_ij(w-1)`, `p=b^T(I-T)^-1`, `m=w p`를 정의한다. `T`는 strictly lower triangular이므로 모든 계산은 유한 다항식이다.

정상이고 dissipative인 실수 행렬 `J=[[0,-1],[1,0]]`, `h=1`에서 스펙트럼은 `z=±i`이다. 정확 유리수 복소 연산으로

`|m_1(i)| = 0.85190467449… > 0.58`

임을 확인했다. oracle은 제곱 절댓값과 `0.58²`를 **정확 Fraction**으로 비교하므로 반례 자체는 출력된 소수 근사에 의존하지 않는다. snapshot의 decimal convention과 실제 binary64 파싱 convention 양쪽에서 성립한다. `z<0` 표본 결과를 전체 복소 좌반평면으로 확장할 수 없다. 현재 target 상수만으로 일반 비선형/진동계의 tolerance 보존이나 5차 order 보존을 인증해서는 안 된다.

## 구성적 연구 R-1: Cayley 표현으로 잔차 증폭 상한을 개선

**정리.** 유한차원 실수 또는 복소 Hilbert 공간에서 `Re<x,Jx> <= 0`, `a>0`, `R=(I-aJ)^-1`라 하자. 그러면 `C=2R-I`는 contraction이다. 다항식 `f`를 `f((1+z)/2)=sum_k d_k z^k`로 다시 쓰면

`||f(R)|| <= sum_k |d_k|`.

**증명.** `v=Rx`이면 `x=v-aJv`, `Cx=v+aJv`이고

`||v+aJv||² - ||v-aJv||² = 4a Re<v,Jv> <= 0`.

따라서 `||C||<=1`. 항등식 `R=(I+C)/2`에 다항식을 대입하고 submultiplicativity와 triangle inequality를 적용한다. 정상성은 필요 없다. ∎

RODAS5P의 정확 계수로 이 정리를 적용한 상한은 다음과 같다. 각 수치는 근사 표시이며 재현 파일에 정확 유리수 계수를 보존했다.

| 전달 대상 | 기존 power coefficient 합 | Cayley coefficient 합 | 상한 개선 비율 |
|---|---:|---:|---:|
| normalized raw residual → output (`m`) | 189.4183670139 | 31.3346885040 | 6.0450055851 |
| isolated stage error → output (`p`) | 189.4183670139 | 37.6028423227 | 5.0373417357 |

이는 **같은 선형 frozen J·같은 Hilbert norm·dissipativity 가정** 아래의 증명된 majorant 개선이다. 실제 nonlinear stage Jacobian drift가 0이 아니면 아래 확장이 필요하다. 일반 weighted infinity norm에는 Cayley contraction 결론을 자동 적용할 수 없다. 실제 코드의 U 좌표 `W=I/(h gamma)-J`와 normalized `W=I-h gamma J` 사이에는 잔차 단위 변환이 있으므로 두 형태의 residual을 혼합하면 안 된다.

**반복 횟수에 대한 조건부 보장.** 같은 norm에서 inner residual을 `r_k<=q^k r_0`, `0<q<1`로 줄이는 iteration과 요구 output budget `epsilon`이 주어지면 증명된 충분 반복 수는 `max(0,ceil(log(G r_0/epsilon)/log(1/q)))`이다. `G`를 189.418…에서 31.334…로 바꾸면 이 충분 횟수는 증가하지 않는다. `q=1/2`이고 두 ceil 값이 양수인 경우 2 또는 3회 줄어든다. 이는 보장된 iteration budget의 감소이며, 본 감사에서 실제 전체 solver wall time 감소를 측정했다는 뜻은 아니다.

**비선형 확장 계약.** 선형 frozen-stage inverse를 `S=(I-B0)^-1`, 각 block의 Cayley norm bound를 `N_ij`, stage Jacobian variation을 `||a R DeltaJ_i||<=delta`라 하자. 여기서 `DeltaJ_i = integral_0^1 J(Y_i+s(Ytilde_i-Y_i)) ds - J0`는 선분 평균이다. 벡터 함수에 일반적으로 존재하지 않는 하나의 `J(xi)`를 가정하지 않는다. `D_ij=a R DeltaJ_i A_ij`를 두면

`e <= N d + N delta |A| e`.

`N delta |A|`는 strictly lower triangular이므로 spectral-radius 추측이나 smallness 가정 없이

`e <= (I-N delta |A|)^-1 N d`

이며 inverse는 stage 수 8의 유한합이다. 출력의 base cancellation을 보존하려면 `bS`의 Cayley bounds `pbar`를 사용하여

`output <= [pbar + pbar delta |A| (I-N delta |A|)^-1 N] d`

로 평가한다. 이론적 구성이며 실제 tube에서의 `delta` 생성·roundoff enclosure는 다음 구현 의무다. norm 가정이 없는 일반 callback에 대해 이 의무를 생략하지 않는다.

## 구성적 연구 R-2 / 구현: B1d resolvent staleness

실제 U 좌표에서 `P=I/(h_P gamma)-J_P`, `W=I/(h gamma)-J`, `R=P^-1/(h_P gamma)`이다. 직접 곱하면

`I-P^-1 W = R[(1-h_P/h)I+h_P gamma(J-J_P)]`.

따라서 유도 norm에 대해

`q_R=||R||_S (|1-h_P/h|+h_P gamma ||J-J_P||_S)`

가 contraction operator의 상한이다. log norm 조건이 성립하면 기존 resolvent theorem으로 `q_R<=q_B1c`이고, log norm 조건이 실패해도 nonsingular P이면 정확 산술의 q_R는 유한하다. `q_R<1`이면 `||x-W^-1c||_S <= ||P^-1r||_S/(1-q_R)`이고 기존 stage stop을 재사용한다. **LU로 산출한 abs(R) 및 row sum은 ordinary floating point이므로 이 구현은 진단용이며 엄밀 enclosure가 아니다.** 이 범위를 Rust 문서에 명시했다.

새 API는 `StalenessRule::Resolvent`, `rodas5p_resolvent_staleness_bound`, 두 solver별 driver ID이다. `Off`는 default 그대로다. refresh마다 n개의 LU solve로 abs(R)를 만들고 `StalenessWork.resolvent_solves`와 `WorkCounters.direct_solve_calls`에 모두 반영한다. `rescued_checks`는 B1c 값이 무한대였으나 새 값이 0.5 이하인 횟수이다. P 유효성은 factor와 모든 추가 solve가 성공한 후에만 켜진다. 추가 storage O(n²), check O(n²), refresh 추가 비용은 n개의 triangular solve다. 이는 dense에서는 O(n³)이므로 factorization 감소만으로 speedup을 보장할 수 없다.

계획의 closed-form test fixture도 교정했다. `h_P/h=0.8`이면 `|1-h_P/h|=0.2`이다. 기존 계획의 `0.25`는 `h/h_P-1`과 혼동한 값이었다. 계획 원본은 보존하고 새 테스트는 올바른 0.2를 사용한다.

### 고정 조건 진단 결과

기존 wave4 문서의 corpus·tolerance·기준을 그대로 사용했다. 별도 preregistration commit을 먼저 publish하지 않았으므로 새 결과는 `DIAGNOSTIC_NOT_PUBLISHED_PREREGISTRATION`이며 `would_pass_gate`만 기록한다. 과거 B1c ledger를 재평가하거나 덮어쓰지 않는다.

| 문제 / arm | accepted | factorizations | fact/accepted | steps/fast | error/fast | 추가 resolvent solves | 전체 LU solves / fast |
|---|---:|---:|---:|---:|---:|---:|---:|
| HIRES GMRES | 204 | 90 | 0.441176 | 1.0 | 1.035302 | 720 | 10546 / 1680 |
| HIRES Richardson (context) | 204 | 145 | 0.710784 | 1.0 | 1.002961 | 1160 | 6268 / 1680 |
| Brusselator n100 GMRES | 82 | 19 | 0.231707 | 1.0 | 0.999106 | 1900 | 8527 / 736 |
| Brusselator n100 Richardson (context) | 82 | 73 | 0.890244 | 1.0 | 0.999913 | 7300 | 9675 / 736 |

HIRES GMRES는 기존 B1c 120 factorization에서 90으로 줄었고 21회 무한대 검사를 구제했다. 그런데 고정 0.4 기준에는 실패한다. 평균 accepted step·endpoint accuracy는 유지되지만 추가 solve 비용이 크다. 모든 timing은 debug 공유 환경의 diagnostic이다. **새 routing을 열지 않았다.**

## 실제 검증과 재사용

- Rust 1.94.1, `--locked --offline`로 새 계약 6개 PASS, 별도 진단 실행 1개 PASS. 새 계약에는 원본 90개 explicit-system norm 비교, closed-form nonsymmetric example, 2문제×3 tolerance×2 solver 강제 refresh의 exact endpoint parity와 모든 추가 solve accounting, 두 rule×두 solver zero-weight completion이 포함된다.
- 기존 `a1_zero_tolerance_scale` 9, `b1_lagged_contracts` 5, `b1b_gmres_contracts` 8, `b1c_staleness_contracts` 6: 합계 28 PASS. 기존 measurement 3개 ignored를 유지했다. Off/Bounded bit pins와 기존 factor ledger가 통과했다.
- 기존 fast/banded exact-cache의 exact matrix identity·failed factor invalidation 경로를 소스로 확인했다. 해당 코드는 수정하지 않았으며 기존 증거를 재사용한다. 새로운 tolerance·성능 주장은 하지 않는다.
- 첫 guard 실행은 compiler lock 대기 중 소스가 바뀌어 수정 후 소스를 컴파일했다. 해당 PASS를 RED라고 보고하지 않고 `guards_first_concurrent_build.*`로 보존했다. 원래 결함은 별도 원본 함수 reproducer로 증명했다.
- 원본 T1 결과 전부를 재실행하지 않았다. 새 감사 질문인 복소 영역 확장과 새로운 Cayley 표현만 exact Fraction 계산했다.

## 남은 의무와 구체적 다음 단계

1. **Lagged target의 theorem-aware 생성:** frozen linear dissipativity를 immutable problem declaration으로 바인딩하고 Cayley `p` 계수의 정확 상한을 generated constant로 만든다. 비선형은 위 `delta` tube bound를 추가한다. admission에서 선언·norm·stage mapping이 일치할 때만 사용하고, 현재 일반 target은 heuristic으로 유지한다.
2. **Resolvent inverse enclosure:** computed B에 대해 outward `E=I-PB`를 만들고 `||E||<1`을 증명한 후 `||P^-1||<=||B||/(1-||E||)` 또는 componentwise enclosure를 사용한다. 비용이 커지므로 band/low-rank owner의 구조를 사용하고 certification cost를 같은 ledger에 반영한다.
3. **B1 다음 속도 카드:** n RHS inverse construction을 매 refresh에 수행하는 dense 방식은 실제 solve 수가 6–12배 늘었다. 동일 J·small step changes에 대한 선언된 Hilbert dissipativity bound, band/low-rank inverse norm 또는 sparse approximate inverse residual enclosure를 새 메커니즘으로 조사한다. no-go가 아니며 기존 실패를 이유로 이 구조적 경로를 폐기하지 않는다.
4. **O-35 GMRES 오류 work 보존 — 이번 감사에서 수정 완료:** `GmresWorkspace::monitored_progress()`가 attempted/completed columns, column operator attempts, cycles, least-squares attempts/completions를 성공·오류 모두에서 보존한다. `linear_iterations`는 각 column 완성 직후 증가하므로 그 뒤의 monitor 실패로 없어지지 않는다. 기존 monitored 함수의 signature는 유지했다. Generic core의 application counter는 원래의 successful-call 의미를 유지하고, 실제 lagged caller는 기존 atomic attempt count와 차이를 이용해 실패한 JVP/LU 호출까지 자신의 economic ledger에 계상한다. 오류로 폐기된 completed columns도 `abandoned_iterations`에 남는다. Arnoldi happy breakdown에서 monitor가 `Continue`하면 수렴을 선언하지 않고 `accepted:false`를 반환하며 output은 그대로 둔다. 이를 zero-residual 재시작 실패로 잘못 분류하던 경로도 교정했다.

   표적 검증은 ① 두 번째 column의 operator/preconditioner 실패, ② 두 column 뒤 monitor 실패와 다음 call의 progress 초기화, ③ happy-breakdown Continue와 이미 정확한 초기 iterate, ④ 실제 lagged 적분의 네 번째 JVP(첫 completed column 뒤 monitor residual) 실패 후 재시도·최종 정확도·전체 work partition이다. 새 테스트 4개 PASS, 영향받은 B1b 8개와 B1d 4개 재실행 PASS. shared target metadata 문제로 첫 빌드가 E0463을 냈고, 별도 `dense-target`에서 재빌드했다. 첫 독립 실행에서 core의 successful-call 의미와 테스트의 attempted-call 기대가 달라 한 assertion이 실패했다. 원래 core 계약을 확인한 뒤 기대를 두 개의 지표로 분리했고, lagged strong accounting은 추가 코드로 완성했다. 실패 로그를 모두 보존했다.

## R/C loop 실제 수행 대응

R1/C1: 최신 wave3 code와 wave4 plan 고정. R2/C2: 기존 B1c 증거와 U 좌표/소스 연결. R3: B1d exact-arithmetic claim과 floating implementation 구분. R4: weighted log norm 대신 resolvent 및 power 대신 Cayley 표현 탐색. R5: 위 두 정리·비선형 nilpotent 확장 유도. R6: 원본 코드 재현, 정확 Fraction complex 반례와 계수 계산. R7/C6–C8: native contracts·original corpus diagnostic·28개 영향 회귀. R8: B1d gate would-fail, 보호 경로 유지. R9/C5: opt-in Rust 구현과 구체적 tube/roundoff 후속 계약. R10/C10: raw logs·JSON·source mapping 및 수정된 GMRES failure 계약 기록. 독립 검토 C9는 통합 controller의 검토에 맡기며 작성자 자신의 검토를 독립 승인으로 세지 않는다.

독립 이론 검토는 `evidence/theory/INDEPENDENT_REVIEW.json`에 있다. 별도 연구 agent가 Cayley contraction 증명과 32개의 정확 다항식 reverse-substitution/coefficient-sum 검사를 수행했다. 이는 작성자 자신의 검사와 별개다.

## N05 후속 이식 완료

사용자의 추가 코드 이식 요청에 따라 위 I-N1/I-N3의 좁고 증명 가능한 부분을 실제 구현했다. immutable frozen-linear model에 묶인 Cayley/negative-real Bernstein target planner, 실제 U-stage·embedded estimator·returned-center의 outward assessment, 그리고 n개의 inverse-construction RHS solve를 0개로 줄이는 opt-in diagonal resolvent rule이다. 원래 controller budget을 유지하고 일반 비선형 drift나 floating LU 오차를 숨기지 않는다. 새 native 계약 7개가 첫 실행에 모두 PASS했다. 자세한 정리·API·비용·실행 로그·후속 client 계약은 `NEXT_N05_IMPLEMENTATION.md/json`을 따른다. 이전 표의 baseline/diagnostic 측정 결과를 덮어쓰거나 production speed gate 통과로 재해석하지 않았다.
