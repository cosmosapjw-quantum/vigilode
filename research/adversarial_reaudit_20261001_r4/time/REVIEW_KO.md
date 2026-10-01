# R4 시간·출력 재감사

원 R3 시간 반례 두 개는 해당 입력에서 해결됐다. 그러나 시간 간격을 표현된
clock에 맞추는 것만으로 integrator의 시간 의미론이 전부 닫히지는 않는다.
이번 실행에서는 **고정 BDF의 시간 단위 의존성**과 **비대칭 분할에 잘못 붙은
Radau1 오차 추정식**을 확인했다. 둘 다 P2로 제안하며 이 lane의 판정은
`REWORK`다. 전자는 이미 공개된 absolute floor의 새 수치 반례이고, 후자는
갱신된 `split_clock`과 기존 Richardson 식 사이에서 새로 재현한 geometry
불일치다. 동일 witness를 이전 source에서 실행하지 않았으므로, 이전 버전은
정확했고 이번 변경이 오류를 도입했다는 regression 판정은 하지 않는다.

검토 생산 source는 `1c54194123ee6abc6daa512e8574922f510b4e2c`다. 입력·gate·실행기는
원격 preregistration commit `b2914f3e3c03eda60a8547619db6284aac8250a2`가 확인된 뒤
실행했다. Rust build/run은 각각 exit 0, 36개 raw row이며 원 도구·수치 코드와
의존 패키지 93개의 version/checksum이 일치한다. 생산 파일을 고치지 않았다.
원 source와 preregistration HEAD의 차이는 보고서 입력 파일 추가다.

## 기존 수정의 실제 효과

| 항목 | 이번 관측 | 판정의 범위 |
|---|---|---|
| R3-TIME-01, 8 ULP·h=1e-4·원점 ±1e12 | 고정 Rodas/BDF/Radau 6개 관측 모두 최대 2.89e-15 오차 | 원 반례 종료. BDF 계수·오차추정까지 일반적으로 종료했다는 뜻은 아님 |
| R3-TIME-02 uniform singleton | 4 ULP 구간·spacing=1을 명시적으로 거부 | 원 생성자 반례 종료 |
| R3-TIME-02 explicit [tf] | endpoint 불일치로 거부; [t0,tf] control은 2.89e-15 오차 | 원 출력 오표기 반례 종료 |
| indexed fixed grid | runtime 담당 실행에서 represented-clock 계약 9개 통과 | 1000-step micro-step 계약 포함; 이 lane에서 중복 실행하지 않음 |
| representable midpoint | 1 ULP 구간에서는 Radau1/BDF가 명시적 오류; 2·3 ULP 상수 흐름은 정확히 완료 | clock 타일링의 개선을 직접 확인 |
| 비자율성·원점/단위 변경 | 네 quadratic primitive가 등록된 conditioning allowance 안에 있음 | 큰 원점에서의 bitwise covariance나 조건수 없는 작은 오차를 주장하지 않음 |
| 영점 통과 | (-0.7,0.3), (-1e-300,1e-300)의 상수 흐름 최대 2.52e-15 오차 | 이 두 유한 범위에 한정 |

공통 runtime의 `r3_represented_clock_contracts` 9개와
`r2_output_time_identity_contracts` 9개 통과는 별도
`evidence/runtime/native/suite_summary.json` 및 원 로그로 연결된다.

## R4-TIME-01: 고정 BDF의 절대 간격 floor

`bdf.rs:384–386`의 `same_step`은

\[
 |h_n-h_{n-1}|\le32\epsilon\max(|h_n|,|h_{n-1}|,1)
\]

이면 간격이 같다고 판정한다. 이 결과가 `475–495`의 variable-spacing
선택을 결정한다. 실제 간격이 서로 두 배 차이나도 둘 다 충분히 작으면
등간격 식 (3y_{n+1}-4y_n+y_{n-1}=2h_nf_{n+1})로 들어간다.

반례는 (y'=v, y(t_0)=0, v=1/(t_f-t_0))이다. 경계 (2^p) 아래의 8 ULP에서
위의 8 ULP까지, nominal step을 위쪽 ULP의 0.675배로 정했다. 문제와 시각들을
2의 거듭제곱으로 함께 바꾸므로 무차원 물리 경로는 같다.

| 경계 (2^p) | native fixed BDF 끝값 | 같은 시간점의 variable BDF 후보 끝값 | exact-input 최대 오차 |
|---|---:|---:|---:|
| p=-40 | 0.9791691102325774 | 1.0000000000000018 | 고정 경로 2.0830889767e-2 |
| p=0 | 0.9791691102325774 | 1.0000000000000018 | 고정 경로 2.0830889767e-2 |
| p=10 | 1.0000000000000018 | 1.0000000000000018 | 1.832e-15 |
| p=40 | 1.0000000000000018 | 1.0000000000000018 | 1.832e-15 |

작은 두 단위에서는 다섯 번의 서로 다른 간격을 equal로 분류했다. 독립
유리수 계산으로 그 잘못된 식을 그대로 풀면 0.9791691102325772가 되어 native와
1.11e-16 차이로 일치한다. Newton이 잘못 수렴한 것이 아니라, Newton에 준
방정식의 시간격자 가정이 틀렸다. 정확한 선형해를 등간격 잔차에 대입하면
(v(h_n-h_{n-1}))가 남는다. 상세 유도는 `DERIVATIONS_KO.md`에 있다.

최소 수정은 실제 (h_n/h_{n-1})를 항상 쓰거나, 간격이 정확히 같은 경우에만
등간격 shortcut을 허용하는 것이다. standalone 후보는 생산 코드에 patch를
넣지 않고 기존 public `bdf_step_variable`을 native 고정-driver의 실제 시간점에
적용했으며 네 단위 모두 1.832e-15 안에 들어왔다. 이 후보와 native는 Newton
kernel을 공유하므로, 후보 간 일치 자체는 독립 검증이 아니다. 독립 근거는
linear-flow 해와 Fraction으로 계산한 세 점 보간미분식이다.

closure 문서는 이 absolute floor를 이미 공개했다. 이번 결과는 숨겨진 새
발견이라고 포장할 수 없지만, 2.08%의 반환값 오류와 시간 단위 비공변성이
실제 확인됐으므로 단순 부동소수점 한계 설명으로 남겨둘 수는 없다.
BDF가 reference implementation인 점을 고려해 P2로 평가했다.

## R4-TIME-02: 갱신된 경로에서 재현한 substep·추정식 불일치

`split_clock`은 (t<t_m<t+H)를 확보하고 (h_1=t_m-t),
(h_2=t+H-t_m)를 반환한다. 이는 clock 측면에서는 옳다. 그러나
`radau.rs:582–600`은 이 두 길이가 달라도 `adaptive.rs:397`의
(2^p-1) divisor를 쓴다. 이 식은 동일한 두 half-step에서 유도됐다.

고정 witness는 (t_0=10^{12}), (H=3\,\mathrm{ULP}=0.0003662109375),
(y'=2(t-t_0)/H^2), (y(t_0)=0)이다. 반올림된 midpoint는
((h_1,h_2)=(2,1)\,\mathrm{ULP})를 만든다. Radau IIA 1-stage는 implicit Euler라
한 번의 coarse step은 (y_c=2), 두 번의 fine step은 (y_f=14/9)다.
정확한 끝값은 1이고 따라서 fine error는 (5/9), 기존 fine–coarse 차이는
크기로 (4/9)다.

실제 native 실행은 (y_f=1.5555555555555554)를 반환했다. atol=0.5,
rtol=1e-12에서 추정 norm은 **0.888888888886**, 실제 같은 scale로 잰 오차는
**1.111111111108**이었다. adaptive wrapper는 이 macro-step 하나를 받아들이고
`success=true`를 반환했다. callback은 analytic partial-time derivative와
영 Jacobian/JVP를 제공하므로 미분 근사 문제와 구별된다.

일반적인 smooth leading-error 모델에서

\[
 \eta=(h_1/H)^{p+1}+(h_2/H)^{p+1},\qquad
 |e_f|\simeq\frac{\eta}{1-\eta}|y_f-y_c|
\]

가 맞다. 이번 p=1 witness에서는 보정 계수가 5/4이고 관계가 정확한 대수식이
된다. standalone 후보의 error=0.5555555555555559는 정확한 5/9와 3.33e-16
차이다. 일반 비선형 ODE에서 이 식은 asymptotic estimator이며 엄밀한 전역
오차 상계가 아니다. 문제의 핵심은 tolerance가 항상 참오차를 보장해야 한다는
과도한 요구가 아니라, 실행한 partition과 estimator의 유도 조건이 다르다는
점이다.

BDF startup도 같은 함수에 도달하지만 두 번째 fine step의 차수와 history가
바뀔 수 있다. 이번에는 비상수 BDF startup 반례를 추가 실행하지 않았다.
Radau1의 factor를 그대로 이식하지 말고 mixed-order 시작 절차를 따로 유도해야
한다. 기존 `NEXT_STEPS.json`에 이 의존성을 분리했다.

## max_step 계약과 검사 해석

0.75 ULP 및 1.5 ULP를 `max_step`으로 주면 실제 accepted step은 각각 1 ULP,
2 ULP여서 상한의 4/3배다. 반환 상태값은 정확했다. 0.49 ULP는 진행하지 않고
`success=false`를 반환했다. 코드·closure에 one-resolution slack이 공개돼 있어
이를 별도 숨겨진 correctness finding으로 세지 않는다. 다만 `hard maximum`,
`typed failure`라는 설명 및 기존 테스트 주석과 실제 의미는 맞춰야 한다.
`StrictRepresentedCap`과 `AllowClockResolutionSlack`의 구분을 API에서 명시하고,
값이 정확한지만 보는 helper와 실제 cap 검사를 분리하는 것이 낫다.

동결된 `EXACT_ORACLE.json`의 일반 `constant_flow_gate`는 반환된 각 sample의
오차만 판정한다. 완료 판정에는 native `success`와 마지막 time도 함께 봐야
한다. 따라서 0.49 ULP 중단 결과를 PASS로 세지 않았다. 또한 일반
`conditioned_quadratic_gate`가 특수 estimator witness에도 부가 출력되지만,
그 epoch conditioning allowance는 CONTRACT 항목6에만 해당한다. 항목8의
판정에는 exact local geometry와 `actual_scaled_endpoint_error`를 사용한다.
이 부가 필드의 의미를 `FINDINGS.json`에 명시했으며 동결 입력·gate·코드를
사후 수정하지 않았다.

다음 개발은 실제 간격비 BDF, geometry-aware Radau1 estimator, mixed-order BDF
startup 유도, max_step 정책 명시의 순서로 구체화했다. 각각의 acceptance와
실패 처리, claim ceiling은 `NEXT_STEPS.json`에 있다. 이 lane은 native 과학
실행에서 환경 실패 없이 끝났고, 후보를 생산 코드에 반영하거나 속도 우위를
주장하지 않았다.
