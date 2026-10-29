# R3 독립 판정

고정 소스 `cc2cd041737e7ff543624d1b59893a3b4397369f`에 대해 **범위가 제한된 탐색적 감사·연구 결과는 보고서에 수록할 수 있으며, 생산 readiness는 HOLD / REWORK**다. 원 R2 재현의 해결은 인정하지만 더 일반적인 시간 정합성, 원 φ 입력의 오차 보증, 측정 원자료의 프로토콜 권위까지 닫히지는 않았다. 통합 `REPORT_KO.md`의 주장 범위·수식·네 finding 심각도를 최종 검토해 수락했다. `8-batch sequential baseline`은 폭 8의 homotopy batch와 혼동되지 않도록 `8회의 순차 단일-vector W solve`로 표현하는 편집 수정만 요청했다. 최종 runtime aggregate는 이 검토 시점에 미확정이므로 owner는 실제 timeout·ignored·미실행 상태를 보존해야 한다.

Reviewer는 후보 구현·검증 설계에 참여하지 않았다. 연구 core와 사전등록, 원 R2 finding/status, 새 closure 문서, 실제 변경 코드와 lane 결과를 읽었다. 동일 workspace를 읽는 별도 reviewer context이며 모델 다양성이나 완전한 정보 독립성을 주장하지 않는다.

## 확인한 finding과 한계

- **R3-TIME-01 P1:** 문서에 이미 인정된 nominal h와 represented displacement 불일치를 7개 native 출력에서 재현했다. 상수 flow의 정확한 최종 상태 1 대신 약 0.8192를 성공으로 반환한다. 새 회귀로 부르지 않는다.
- **R3-TIME-02 P2:** nonzero span에서 singleton schedule이 통과해 초기 상태 0을 최종 시간에 기록한다. dense/clipped 두 소비자 모두 직접 재현했다.
- **R3-ARITH-01 P2:** 유한한 극단적 nilpotent 입력에서 부분 weighting underflow가 원 문제의 표현 가능한 첫 성분 약 ±1.67e−27을 잃지만 fused/prefix는 convergence와 error=0을 보고한다. exact Fraction 참조로 확인했다. 보통 범위의 ODE나 실제 campaign 오염으로 확대하지 않는다.
- **R3-STAT-01 P2:** 잘못된 order·batch·warmup을 가진 raw 자료로 새 assessment를 만들면 raw verification도 Promote를 반환한다. 5종 native 재현을 다시 실행했다. 현재 CLI wall runner의 실제 오승격이나 producer 자체의 잘못된 순서를 주장하지 않는다.
- **계수 구조 전제:** native α/L의 strict-upper/diagonal 항과 8제곱은 정확 유리수 해석에서 0이 아니다. reviewer도 해당 bit pattern에서 이를 재계산했다. 이는 nilpotency 인증 전제의 실패이며, 거시적 native solver 실패를 증명한 것은 아니다.

## 수학·코딩 연구 판정

**공동 polynomial action:** Bessel 계수 표현, φ 적분과 Skellam/Chernoff tail의 유도는 명시한 real symmetric A≤0, h≥0 및 검증 가능한 spectral enclosure 범위에서 타당하다. Reviewer가 별도로 고른 2×2 dyadic symmetric A, 다섯 서로 다른 입력, h=.75에 대해 후보 action만 재사용하고 독립적인 Decimal160·250항 급수·exact spectral projector 참조를 만들었다. Laguerre/Chebyshev 모두 관측 목표 1e−10을 만족했다. 이 결과와 이론을 bounded 연구 보고서로 승격한다. 전체 floating-point 인증, 일반 비정규 문제, 실제 wall speedup은 승인하지 않는다.

**Homotopy 성분별 상계:** quadratic remainder 차이에 대한 귀납 상계는 타당하다. 미리 정한 D와 strict-lower 비음수 H에 대해 유한 inverse path sum을 doubling으로 계산하고 radius closure를 검사하는 논리도 타당하다. Reviewer의 별도 2D quadratic dyadic fixture에서 독립 Fraction forward root를 작성하여 q1/q2의 stage·output·embedded bound 및 닫힌 doubling bound를 확인했다. 최종 문서 초반에 explicit strict-lower projection target임을 명시한 수정도 확인했다. native full-block target 또는 native sequential arithmetic와의 exact parity는 미확립이며, 현재 결과를 해당 target 인증으로 옮기지 않는다. ODE discretization/global error와 실제 병렬 성능도 미확립이다.

**Bootstrap MC gate:** Bernoulli exponential moment 유도와 두 방향 δ 배분은 이상적 iid resampling 아래 fixed empirical law·B·threshold에 대한 조건부 Monte Carlo 오차 통제로 타당하다. Reviewer는 6⁶ empirical bootstrap endpoint를 다른 exact-product enumeration으로 확인하고, 별도 B=64·유리수 p grid에서 exact binomial tail을 계산해 방향·δ 배분을 점검했다. 모집단 bootstrap CI coverage나 PRNG의 확률 인증을 뜻하지 않으며 optional stopping은 포함하지 않는다.

**수정 후보:** clock의 h_eff, partial-weight-loss fail-closed, raw-protocol validator는 실행된 standalone repair evidence다. 생산 적용·비자율 order·전체 BDF history·일반 amplified-loss bound를 검증한 것은 아니다. 구조 특화 nilpotent 재배열은 일반 backend로 승격하지 않는다.

## 실행과 provenance

Reviewer는 이미 빌드된 native binary를 별도로 실행했다: time 29행, arithmetic 529행, statistics 10행, 모두 exit 0. 행 수는 독립 테스트 수가 아니며 workspace test 총계에 더하지 않는다. `native_oracle_checks.json`, `polynomial_independent_check.json`, `homotopy_independent_check.json`, `independent_math_checks.json`은 reviewer가 직접 실행한 검산 결과다. 전체 suite를 중복 실행하지 않았다.

`docs/RESEARCH_LEDGER.md`는 preregistration을 실행 전에 **작성하고 commit**할 것을 요구한다. root가 보고한 상태는 작성은 선행했지만 commit은 선행하지 않았다는 것이다. 따라서 이번 결과를 그 규칙을 만족한 preregistered confirmation 또는 독립 holdout PASS로 부를 수 없다. 연구 ledger는 INCONCLUSIVE로 남겨야 한다. 기록이나 예외 규칙을 소급 변경하면 안 된다. 이 process 제한은 재현된 exact/native 반례를 없애지는 않지만, 이번 연구 결과의 confirmatory 지위는 제한한다.

이 한계를 보존하는 bounded 보고서 게재를 수락한다. 통합 본문의 주장 범위와 실행 미완료를 확인했고 독립 검토를 종료한다. manifest·publication identity 확인은 owner의 전달 작업이며 추가 과학 감사 재귀 호출을 요구하지 않는다.
