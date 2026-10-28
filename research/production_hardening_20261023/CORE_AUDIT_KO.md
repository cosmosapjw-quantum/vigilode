# DOPRI5 커널·공개 API 독립 코드 감사

기준: `fbc3c272fa06b443de148631ea78bfa6a34f1b2b`의 `crates/rodas5p-integrators/src/dopri5.rs`, `adaptive_output.rs`, `output.rs`, `crates/vigilode/src/explicit.rs`, `explicit_ensemble.rs`와 해당 계약 테스트. 코드와 IEEE 754 연산 순서를 감사했으며 `ieee_subnormal_repro.py`의 독립 Python 전사 계산을 실행했다. Rust solver의 실제 회귀 실행과 이 전사 계산을 구분한다. Rust 실행 결과는 최종 검증 기록을 따른다.

## F-CORE-01: 극소 시간 구간에서 계수 곱의 선행 underflow

**우선순위: P1, 범위: DOPRI5 고정 스텝 및 마지막 짧은 구간. 상태: 독립 binary64 전사 재현 완료, 수정 구현 완료, Rust 회귀검증은 최종 실행 기록 참조.**

`dopri5.rs:273–277,303–309`는 각각 `weight=h*a`, `weight=h*e`를 먼저 만들고 그 뒤 `weight*k`를 합산한다. 최종 물리적 증분 `h*k`는 충분히 표현 가능해도 `h*a`가 먼저 underflow하면 계수 정확도를 잃는다.

`eta=f64::from_bits(1)`, `h=2*eta`, `y(0)=0`, `y'=1e308`을 고려한다. 다섯 번째 차수 endpoint의 비영 계수에 대한 `fl(h*b_i)/eta`는 `[0,1,1,-1,0]`이다. 합은 1이므로 계산 결과는 약 `eta*1e308`이고 정확한 결과는 `2*eta*1e308`이다. embedded 차의 모든 계수는 절댓값이 1/4보다 작으므로 `fl(h*e_i)=0`; 오차 추정은 정확히 0이 된다. 따라서 약 50%의 상대 endpoint 오차가 작거나 0인 embedded 오차로 가려진다.

독립 전사 결과는 두 부호에서 기존 연산의 상대 오차가 정확히 0.5, 수정한 곱 순서의 상대 오차가 약 `1.996e-16`임을 보였다(`evidence/IEEE_SUBNORMAL_REPRO.json`). 이 파일은 Rust solver 실행을 주장하지 않는다.

이는 low-level 전용 입력이 아니다. 공개 `Dopri5::solve`에서 `(t0,t1)=(0,f64::from_bits(2))`, 상수 RHS, `atol=1e-30`을 주면 기본 `h0`가 마지막 구간에 맞게 잘린다. `AdaptiveOutputSession`은 마지막 도착 스텝의 `min_step` 예외를 허용하므로 이 구간에 도달할 수 있다. 전체 해가 subnormal인 예가 아니라, 잘못 계산된 최종 해가 대략 `1e-15`인 예다.

최소 수정은 정상적인 계수 곱 경로를 유지하고 `h*coefficient`가 subnormal/0으로 반올림되거나 overflow한 예외 경로에서 곱의 순서를 안전하게 재배열하는 것이다. 극소 `h`에서는 `(h*k)*coefficient`가 충분하다. 모든 범위에 단순 재배열을 적용하면 반대로 큰 `h*k`가 먼저 overflow할 수 있으므로 일반 곱셈 helper에는 별도 범위 계약이 필요하다. 통상 범위의 bitwise 결과 보존, 극소 구간의 상수 RHS 정확성, 비선형 작은 구간의 수렴성, FSAL 작업량을 함께 검증해야 한다.

## F-CORE-02: 관측점 재착지가 거절 후 엄격 축소를 되돌릴 수 있음

**우선순위: P2, 범위: 사용자 지정 controller 및 거의 같은 endpoint 재시도. 상태: 기존 소스도 문서화한 상호작용, 최종 landing 뒤 엄격 축소를 확인하는 수정과 최종/내부 관측점 회귀 테스트 구현 완료.**

`AdaptiveOutputSession::trial`은 먼저 `adaptive_end_step`으로 거절된 represented interval보다 짧은 스텝을 고른 뒤(`adaptive_output.rs:192`), `collector.limit_step`을 호출한다(`:206`). 뒤의 호출은 endpoint residue 규칙으로 그 작은 간격을 원래 관측점/최종점까지 다시 연장할 수 있다. `output.rs`의 `limit_landed_step` 설명에도 이 차이가 명시되어 있다.

검증 가능한 예는 `safety=1`, `min_factor=1`, `reject_max_factor=1`, `initial_step=max_step=1`인 유효 설정과, 오차가 1보다 큰 첫 trial이다. controller는 nominal proposal 1을 돌려주고, 거절 후 clock 규칙은 `next_down(1)`을 선택한다. collector가 이를 1까지 재연장하면 같은 trial을 budget까지 반복한다. 기본 공개 options는 일반적으로 0.9 이하의 축소를 사용하므로 이 항목은 기본 설정의 무조건적 실패라고 해석하지 않는다. 수정의 보장은 동일 represented trial을 다시 만들지 않는 것이다. `min_factor=1` 같은 비실용적 controller에서 빠른 수렴을 보장하지는 않는다.

최소 수정은 관측점 landing을 마친 **최종** trial에 대해서도 `h < last_rejected_trial`을 검증하는 것이다. 이미 잘린 observation trial, 큰 epoch의 인접 float, `min_step` 예외, rejected callback 경로를 모두 포함해야 한다. 공유 clock을 바꾸면 DOPRI5뿐 아니라 다른 companion integrator 회귀 범위도 함께 확인한다.

## F-CORE-03: 공개 RHS 경로에서 중복 메모리 순회

**우선순위: P2 성능, 범위: 공개 DOPRI5 경로. 상태: facade 중복 poison 한 번을 제거함. finite scan은 상세 실패 진단을 위해 유지. 시간 이득은 paired benchmark 결과를 따름.**

커널 `evaluate_rhs`는 버퍼 전체를 NaN으로 채우고(`dopri5.rs:337`) callback 이후 finite 검사를 한다(`:348`). 공개 facade도 동일 callback 바로 안쪽에서 다시 NaN으로 채우고(`explicit.rs:218`) finite 검사를 한다(`:221`). 즉 RHS마다 필요 이상으로 한 번의 쓰기와 한 번의 읽기를 추가한다. FSAL을 쓰는 정상 trial은 warm-up 후 6 RHS이므로 trial당 약 12n개의 중복 원소 방문이다. 정확한 byte 이동이나 wall-time은 cache와 compiler의 최적화에 의존한다.

가장 작은 변경은 facade의 중복 poison만 제거하는 것이다. finite scan까지 없애려면 malformed-output 진단을 kernel이 정확히 전달할 계약이 필요하다. 단순히 검사 하나를 생략하면서 `completed`를 미리 증가시키면 malformed callback의 작업량과 실패 유형이 바뀔 수 있다. 부분 write, NaN/Inf, recoverable/fatal callback, 재사용 버퍼의 stale 값, 성공 경로의 bitwise 동일성을 검증해야 한다.

## 유지할 근거와 안전 계약

- 7번째 stage가 endpoint와 같은 상태에서 계산되고, 수락 뒤에만 첫 derivative 위치로 옮겨진다. 거절은 시작 상태를 바꾸지 않으므로 첫 derivative를 유지하는 구조가 적절하다.
- nonfinite `Ok` callback은 fatal이고, 명시적 recoverable callback error만 재시도하는 분리가 구현되어 있다.
- 관측값은 trial 수락 전 검증되고, 실패 시 마지막 accepted state와 실제 RHS 시도 횟수를 보존한다.
- DOPRI5의 streaming WRMS는 `z*z` overflow를 피한다. scale 자체의 overflow는 typed failure인 기존 공유 계약이다. 이를 조용한 acceptance 오류로 분류하지 않는다.
- ensemble은 각 member별 norm과 controller를 유지하고 입력 순서를 보존한다. 다른 member의 실패를 이유로 유효한 member를 취소하지 않는다.
- 공개 API는 mass, dense output, events를 callback 전에 거절한다. 현재의 명시적 지원 범위 제한이며, 제공하지 않는 기능이 성공한 것으로 기록되지는 않는다.

## 아직 넓은 production 주장을 지지하지 않는 항목

관측마다 실제로 착지하는 경로는 촘촘한 출력 시 비용이 커진다. dense extension은 보간 차수와 endpoint 차수를 구분해 별도 검증한 뒤 도입해야 한다. 물리적 축약 상태에서는 full-state 오차 norm을 제공해야 하며, 일반 WRMS만으로 물리 변수별 허용 오차나 보존량을 보장할 수 없다. stiff/nonstiff 자동 전환, 사건 탐지, 장시간 진동 위상 오차, 실제 client 조건은 각각 다른 증거가 필요하다. 이들은 현재 코드의 오동작을 가정한 주장이 아니라 production 사용 범위를 정하는 검증 의무다.

## 새 구현 독립 검토

quartic sampling은 각 accepted interval의 모든 관측값을 먼저 준비하고, clock/state transaction이 수락된 뒤에만 게시한다. 실패한 보간의 일부 관측값이 physical state보다 먼저 노출되지 않는다. 관측이 없거나 endpoint만 필요한 interval에서는 보간계수를 만들지 않는다. dense mode의 상태 오차는 smooth ODE의 국소 보간 오차 `O(h^5)`이며 미분 잔차 `p'−f(t,p)`의 차수와 구별해야 한다.

AffineErrorNorm은 고정 lift를 비영 원소 형태로 저장하고, offset은 상태 scale에만 포함한다. 보존/제거된 변수에 상관된 오차가 누적되는 문제를 물리 차원의 RMS로 다룬다. rank가 부족하면 reduced state 전체에 대한 norm이 아니라 seminorm이라는 한계가 명시되어 있다. 실제 client의 reduction 정당성과 global physical error를 이 타입 자체가 인증하지는 않는다.

공개 facade의 네 경로는 모두 kernel `evaluate_rhs`를 통과하므로 facade poison 제거 후에도 재사용 버퍼의 NaN 초기화가 유지된다. finite scan과 fatal/malformed callback의 세부 telemetry는 보존된다.
