# 동일 작성 맥락의 적대적 수학·물리 검토

## 치명적인 계층 혼동을 피할 것

1. VigilODE의 reduced six-state photon-source IVP는 원본 REI F05 26-event thermal+OTS map이 **아니다**. 원본 owner 46,080개 셀의 photon-source 구조를 재사용하지만, 공간-시간적 forcing, 충돌/재결합, implicit thermal root, Hummer–Seaton table 변화와 Bianchi geometry는 완전히 제외했다. 원래 `2e-4` logT gate와 tick160→161의 FAIL은 여전히 유지한다.
2. PR332 단일 source 각 그룹 J, species S, p, external e를 고정한 순간식으로부터의 3-moment closure는 정확하지만 이것은 full original time-dependent J_g(t),C(t),E(t)를 포함한 소스가 아니다. "original 26-event solved"라는 표현은 금지.
3. 그룹3 `J=5e-324`는 숫자 subnormal 한계다. 직접 HeII 광흡수 0이라는 물리적 결론을 요구하지 않는다. 이 범위의 데이터에서 RESOLVED_PHOTO_HEII=NUMERICALLY_UNRESOLVED.
4. 본 고정 source 부분계의 각 종 흡수 hazard는 동일하기 때문에 46,080 셀별 누적 photoelectron은 원래 PR332 photon rate 분포를 전역 총수 변화로 재가중한 결과다. 이는 46,080개의 **원본 source 기반 파생 계산행**이지 46,080개의 새 독립 native 26-event IVP나 observational measurement가 아니다.
5. 기존 `repo-wide research ledger` 및 `check-authority-refs` 실패가 있으면 scoped 실험의 성공으로 지우지 않는다. 자동 default solver/허용오차/기존 소스 수정, main merge, production 전환을 하지 않는다.

## 기록된 실제 결함과 수정

- 첫 source fixture 해시 전사에서 64-hex state SHA에 중복된 `9`가 들어가 **input mismatch**로 중단됐다. PR332 source JSON이 실제로 갖는 64자리 원본 SHA로 교정한 뒤 데이터 추출을 통과했다. 원본 데이터 바이트는 바꾸지 않았다.
- 처음 3-moment 단계에서는 `S_HeII^0(1-w_HeII)`와 HeI 누적량을 빼서 HeII photon absorption을 추정했다. 상대적으로 매우 큰 HeII 총수와 거의 같은 값의 차이는 부동소수점 cancellation을 만들어 **음의 가짜 photon absorption**을 반환했다. 이를 수정해 누적 photon absorption 성분 `a_s`를 명시적으로 적분하는 6D 회계로 변경했다. 물리적 음의 광흡수라는 실패가 아니라 구현·수치 취약성이다.
- 최초 테스트에서 tangent state 길이를 점검하지 않아 부적절한 길이의 input이 조용히 통과했다. `ValueError`의 명시적 fail-closed 검사를 추가하고 표적 회귀 테스트를 통과시켰다.

## 미해결

- RODAS5P native Rust 실행 및 SciPy cross-language comparison은 실제 CI/실행 로그를 읽은 뒤에만 PASS를 주장한다.
- 새로운 결과의 full physical owner callback/thermal coefficient match, calibrated group3, Bianchi observable, 외부 distinct-author referee는 NOT_RUN.
- Native subagent orchestration interface는 현 세션에 제공되지 않아 NOT_RUN; 이 적대적 검산은 동일 맥락이며 별도 모델 심사가 아니다.
