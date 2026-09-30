# R3 시간·출력 감사와 국소 수정 연구

대상 source는 `cc2cd041737e7ff543624d1b59893a3b4397369f`, tree는 `182fa306067b84ee10ec1f1b1e5fbad9e769aae1`이다. 비교 기준은 R2 보고서가 게시된 `1ba914dc234e25c532a9598b1c30ab156c75218d`이다. 생산 코드는 변경하지 않았다. 이번 native probe는 현재 소스의 **default feature** 공개 API에 연결하여 실행했다.

판정은 **원 R2-OUT-01 반례는 닫힘, 일반적인 시간·출력 일관성은 REWORK**다. 기존 closure 문서가 명시한 물리적 step과 표현된 시간 간격의 불일치를 추가 소비자에서 재현했다. 또한 공개 schedule 생성기가 비영 구간에 `[tf]` 한 점만 허용하고, 실제 적분을 수행한 뒤에도 초기값을 `tf`의 값으로 반환하는 잔여 경로를 확인했다. 후자는 이번에 새로 확보한 반례이지만 그 원인인 허용오차 기반 endpoint 검사는 업데이트 이전부터 존재하므로 도입 회귀로 분류하지 않는다.

## 1. 실제 실행과 원 반례의 closure

`probe/src/main.rs`의 실행은 exit 0, JSON 29행이다. `exact_oracle.py`는 Rust 구현이나 보간계수를 가져오지 않고 Python `Fraction.from_float`로 입력과 출력의 이진64 수를 정확한 유리수로 변환한다. 초기조건 (y(t_0)=0), 일정한 속도 (v)에 대해 독립적으로 (y(t)=v(t-t_0))를 평가했다. 이 검산도 exit 0이며, 여섯 묶음 검사가 모두 참이다. 아래 오차는 반환된 각 표본에서 이 독립 해와 비교한 절대오차다.

| 현재 소스의 native 사례 | 결과 |
|---|---|
| (t_0=\pm10^{12}), 8 ULP 구간, (h=\mathrm{span}/2) | 실제 두 step 수행, 끝점 값 (1.0000000000000029); 오차 (2.89\times10^{-15}) |
| (t_0=1), 1 ULP 구간 | 실제 한 step 수행, 오차 (2.89\times10^{-15}) |
| 한 endpoint 다음 ULP의 dense 요청 | 독립적인 표본으로 계산; 반환 전체 표본 최대오차 (2.36\times10^{-11}) |
| 인접한 clipped 요청 | 중복으로 거절되지 않음; 최대오차 (3.11\times10^{-15}) |
| (t=1)에서 반 ULP step | 표현된 시간이 전진하지 않는다는 typed error |

따라서 원래 세 종류의 반례와 비전진 step 회귀 방어는 실제 구현 수준에서 확인했다. 전체 solver의 시간 원점 불변성을 확인한 것은 아니다. 현행 `r2_output_time_identity_contracts` 9개는 공통 runtime campaign에 포함되므로 이 lane에서 중복 실행하지 않았다. 그 campaign의 실행·timeout 판정은 별도 runtime receipt가 결정한다. 복사한 테스트 파일의 존재만으로 통과를 주장하지 않는다.

빌드 중 audit probe 상대경로 오류 한 번과, 생성된 zero-length object를 Cargo가 archive로 묶지 못하는 환경 문제를 보존했다. 후자의 근본 원인은 미확정이다. runtime 담당자가 생성물만 정리하고 incremental compilation을 끈 후 default library build를 복구했다. 사용하지 않는 `audit2-research` feature를 probe manifest에서 제거한 마지막 실행이 성공했다. 생산 소스의 실패나 수학적 반례와 이 빌드 사건을 섞지 않았다. `BUILD_FAILURES.json`, `native_build_attempt*.log`, `EXECUTION_RECEIPT.json`이 실제 경위를 담는다.

## 2. R3-TIME-01 — 이미 인정된 P1 잔여 결함을 여러 소비자에서 확인

`docs/REAUDIT_R2_CLOSURE_20260930.md`의 limit 1은 이 문제를 이미 명시한다. 이번 검증은 그 내용을 새 발견으로 세는 것이 아니라, 범위와 수치적 영향을 독립적으로 확인한다.

이진64에서 (t_0=10^{12}) 부근의 인접 간격은

\[
u=2^{-13}=0.0001220703125,
\qquad t_f-t_0=8u=2^{-10}.
\]

(y'=1/(t_f-t_0)=1024), (y(t_0)=0)를 택하면 정확한 최종값은 1이다. nominal (h=10^{-4})는 한 ULP보다 작지만 반 ULP보다 커서 매번 표현된 시간은 한 ULP씩 증가한다. 현재 stage 방정식은 그와 다른 (h)를 소비한다. 따라서 일정한 RHS를 정확히 적분하는 방법이라도 누적 state 증가량은

\[
y_N= v\sum_{n=0}^{7}h_n \simeq 1024\cdot8\cdot10^{-4}
=0.8192
\]

가 된다. 마지막 등호의 십진수는 반올림 표시이며, `exact_oracle.json`에는 입력 binary64 (h)의 정확한 유리수 합을 저장했다. 시간 라벨의 누적 간격은 1을 요구한다. 이 차이는 truncation error가 아니라 계산에 소비한 물리적 시간과 상태를 붙인 시간 라벨의 불일치다.

| 공개 driver | 최종값 | 성공 반환 |
|---|---:|---|
| fixed RODAS, (t_0=+10^{12}) 및 (-10^{12}) | 0.8192000000000021 | true |
| adaptive RODAS | 0.8192000000000021 | true |
| fixed Radau | 0.8192000000000002 | true |
| fixed BDF | 0.8192000000000029 | true |
| adaptive fused exponential | 0.8192000000000002 | true |
| fixed dense RODAS | 0.8192000000000021 | true |

동일한 국소 구간을 원점 0에서 적분하면 오차 (2.44\times10^{-15})로 1에 도달한다. 큰 epoch에서의 18.08% 오차는 양·음 원점 모두 재현된다. 일정한 RHS의 두 내장 근사식이 같은 잘못된 (h)를 공유하므로 embedded estimator를 엄격하게 설정하는 것만으로 이 문제를 해결할 수 없다.

관련 위치는 `output.rs:26-57`, 각 driver의 `end_step` 호출과 native step 호출이다. 특히 `land`의 `natural == target` 분기는 `t + proposed`가 목표로 반올림되면 `proposed`를 그대로 반환한다. 이 시간 라벨 전진 검사는 실제 stage 간격과 endpoint 차이를 묶지 않는다.

**수정 연구.** 생산 API `sequential_step`을 사용하는 별도 driver에서 먼저 표현할 다음 시각 (t_{n+1})을 고르고, (h_{\rm eff}=t_{n+1}-t_n)을 stage에 넘겼다. (t_n+h_{\rm eff}=t_{n+1})가 성립하지 않으면 진행하지 않는다. 이번의 같은 부호·근접한 endpoint에서는 Sterbenz 조건 때문에 뺄셈이 정확하다. 원점 (0,+10^{12},-10^{12})의 세 사례 모두 최대 표본오차 (3.11\times10^{-15}) 이내였다. 이는 `derived / numerically checked / implementation-verified`인 **상수 RHS 국소 prototype**이며 생산 패치가 아니다.

실제 통합에서는 `h_eff`를 error controller, dense extension, BDF history, 사용량 기록까지 같은 값으로 전달해야 한다. adaptive `max_step`도 `h_eff`에 다시 적용해야 한다. 한 ULP가 사용자가 정한 최대 step보다 크면 무단 확장 대신 time-resolution failure가 적절하다. BDF/Radau의 내부 half-step 각각이 전진하는지도 별도 확인해야 한다. 부호가 다른 endpoint의 cancellation, 비자율 RHS의 stage 시간, 가변 BDF 계수까지 이 작은 prototype이 해결했다고 주장하지 않는다. `TIME-01`에 구현 및 수락 조건을 구체화했다.

## 3. R3-TIME-02 — 비영 구간의 한 점 schedule이 초기값을 최종값으로 반환

입력은 (t_0=10^{12}), (t_f=t_0+4\,\mathrm{ULP}(t_0)), (y'=2048), (y_0=0), 실제 적분 step은 구간의 절반이다. 모든 시각과 값은 유한하며 시간 구간은 정확히 비영이다. 정확한 최종값은 1이다.

공개 호출 `OutputSchedule::uniform(t0, tf, 1.0)`은 간격 수를 0으로 반올림한다. 원점 크기에 비례한 `time_tolerance`가 구간 전체보다 크므로 이 잘못된 나눔 검사를 통과한다. 이후 마지막 원소를 `tf`로 바꾸어 schedule `[tf]`를 만든다. `OutputSchedule::new(vec![tf])`도 동일한 endpoint 검사를 통과한다.

collector는 `schedule[0]`에 초기값을 저장하고 `next_index=1`로 시작한다. 요청이 이미 소진되었으므로, 두 번의 실제 적분을 수행한 뒤에도 저장된 유일한 값은 `y0=0`이다. fixed clipped RODAS와 fixed dense RODAS에서 모두 **success=true, internal_steps=2, returned t=[tf], returned y=[[0]]**를 얻었다. 과거 반례의 zero-attempt success와는 다른 경로다. 같은 source의 정확한 `[t0,tf]` control은 (1.0000000000000029)를 반환한다.

근거 위치는 `output.rs:173-186`의 uniform 생성, `193-203`의 tolerant endpoint 검증, `361-378`의 collector 초기화다. 이 실패를 **P2**로 둔다. 입력 scheduling 계약의 경계 문제지만, 오류로 거부하지 않고 공적 API가 생성·승인한 schedule에서 수치 상태를 틀린 시각의 값으로 성공 반환하기 때문이다.

관련된 표기 문제도 있다. (t_0, t_f=t_0+1)의 양 끝 요청을 각각 한 ULP 뒤로 이동한 두 점 schedule은 받아들여진다. (v=8192)에서 반환값과 반환 시각의 정확한 해가 1만큼 어긋난다. `due`가 마지막 요청을 `tf`에서 소비하면서 원래 요청 라벨을 보존하기 때문이다. 해당 alias 의도는 source 주석에 드러나므로 이를 은폐된 새 알고리즘으로 서술하지 않는다. 다만 공개 API가 이 동작을 지원하려면 실제 시각과 요청 라벨을 분리하거나 명시적인 alias 정책이 필요하다.

**수정 연구.** 비영 구간은 최소 두 점과 정확한 표현 시각 endpoint를 요구하는 별도 admission 함수를 실행했다. uniform singleton, explicit singleton, 양 끝 alias는 거절하고 정확한 two-point control은 허용하는 4개 판별 결과를 얻었다. 생산 API의 호환성을 유지하려면 대안으로 endpoint를 canonicalize하되 반환한 실제 시각과 원 요청 metadata를 분리할 수 있다. 어떤 정책을 택하더라도 비영 구간의 초기값을 단순히 `tf`로 붙여서는 안 된다. `TIME-02`의 수락 조건을 먼저 생산 회귀 테스트로 옮기는 것이 가장 작은 다음 작업이다.

## 4. 새 결함으로 세지 않은 한계와 개발 순서

문서 limit 3의 긴 fixed-step drift도 재현했다. `(0,10)`, nominal step `0.01`은 1001 step을 만들었고 (y'=0.1)의 endpoint 오차는 (1.79\times10^{-14})였다. 이 사례는 새 정확도 결함으로 세지 않았다. 다만 추가 micro-step이 BDF order/history와 작업량 측정에 미치는 영향은 시간 grid 정책을 정할 때 같이 다뤄야 한다. 정확한 step 수만 맞추려고 tolerance를 확대하면 원래의 짧은 구간 문제를 다시 만들 수 있다.

연구 driver의 구 slack은 closure 문서가 명시적으로 보존한다. 이 lane에서는 sealed replay를 새로 실행하거나 수정하지 않았다. replay를 변경할 때는 시간 정책 버전과 새 실행 증거를 남겨야 한다.

개발은 `TIME-01`의 physical-step/clock 연결과 `TIME-02`의 schedule endpoint 계약을 우선 병행하고, 두 계약 뒤에 `TIME-03`의 indexed/compensated fixed grid와 BDF history 검증을 진행하는 순서가 적절하다. 상세 목표, source 위치, 최소 구현, 수락 조건, 실패 정책은 `NEXT_STEPS.json`에 있다. 이 분석은 일반 강직 문제의 차수, 큰 비자율 시스템의 정확성, 실제 병렬 속도 향상을 검증한 결과가 아니다.
