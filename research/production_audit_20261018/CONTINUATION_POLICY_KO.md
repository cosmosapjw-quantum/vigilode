# DAG 연속 실행 정책 및 중단 원인

## 중단 원인

2026-10-18 continuation에서 클라우드 로컬 검증이라는 종속 하위 요청을 확인한 뒤, 그 하위 결과를 보고하고 상위 production-audit DAG로 돌아오지 않았다. 당시 원격 audit ref, dirty user work, 미완료 A01과 A05 HOLD가 남아 있었고, A01 구현 프로세스/작업도 계속 진행 중이었다. 이는 scoped 결과를 전체 목표 완료와 혼동한 제어 흐름 오류다. 기존 AGENTS 정책은 scoped PASS마다 READY 후속을 재계산하도록 이미 요구했지만, 대화 종료 직전에 그 완료 조건을 검사하지 않았다.

## 재발 방지 실행 규칙

각 노드 실행, 독립 검토, 임시 하위 요청이 끝날 때마다 아래 순서를 수행한다.

1. 실제 checkout/remote, dirty 작업, 관련 프로세스나 실행 handle 및 최신 DAG를 다시 읽는다.
2. 결과를 해당 노드의 좁은 범위로만 분류한다. `PASS_SCOPED`는 전체 노드 승격이 아니며 `HOLD`는 그 노드의 후속 의존성만 막는다.
3. DAG의 실제 edge와 수용 조건으로 모든 노드를 `READY`, `RUNNING`, `BLOCKED`, `HOLD`, `PASS_SCOPED`, `CLOSED` 중 하나로 갱신한다. 기계 DAG에 없는 의존성을 만들거나 지우지 않는다.
4. `READY`가 있으면 우선순위와 명시된 dependency에 따라 다음 노드를 즉시 시작한다. 필요한 리뷰가 기다리는 동안에도 독립 `READY` 노드를 실행한다. `RUNNING`이면 정확한 process/tool handle을 관측하며, timeout만으로 재시작하지 않는다.
5. 실행조건이 모호하거나 불충족이면 그 조건 자체를 연구 loop로 해소한다. 원본 입력/가정/수치 기준을 유지하고, 실패 로그와 누적 비용을 보존한다. 해석이 필요한 수학적 실패는 독립 연구 검토 및 별도 승인 절차로 되돌린다.
6. 사용자에게 최종 상태를 반환하기 직전 DAG를 다시 계산한다. `READY` 또는 `RUNNING` 작업이 남아 있으면 최종 종료하지 않고 해당 실행을 이어간다. 모든 나머지 작업이 명시적으로 `BLOCKED`/`HOLD`이고, blocker를 해소할 안전한 작업도 없을 때만 현재 턴을 요약한다. 이는 전체 scientific goal 완료 판정과 구분한다.

## 현재 감사에서의 적용

- 클라우드 로컬 검증: 별도 전달 브랜치에서 check/smoke/bench를 완료하고 원격에 기록했다. 이것은 production DAG의 종결 조건이 아니다.
- A01-LCERT-01: 6b722a9에서 κ=0 rotating-frame frozen-window solution inclusion이 독립 PASS_SCOPED이고, 7e28738은 accessor 의미 주석만 독립 확인됐다. Full A01은 legacy compute_certificate와 나머지 인증 의무 때문에 HOLD다.
- A02: full A01이 아니라 PASS_SCOPED A01-LCERT-01에 의존하도록 DAG를 좁혔다. D1–D5 수학은 독립 검토 READY지만, C1 시험별 config/tolerance identity와 C2 retained-window immutable seed/stale-snapshot refusal가 빠져 contract-only 보완과 재검토 중이다. 구현은 독립 계약 PASS 전까지 승인되지 않았다.
- A03: full damped trajectory는 packet certificate/trajectory dependency로 BLOCKED다. 별도 local damped-window의 exp(-2κt) forcing, frame/carrier 및 κ→0 연결을 research loop로 진행한다.
- A04: historical tail lemma 교정은 PASS_SCOPED다. 나머지 proof-oracle 계약 보완은 독립 리뷰 중이며 full Leja authority는 EstimateOnly/HOLD다.
- A10은 현재 DAG의 full A01 dependency가 해소될 때까지 BLOCKED다.
- A05: 독립 검토 HOLD, 추가 구현 승인 없음. 이를 이유로 독립 READY 노드를 멈추지 않는다.
- A06/A11: 기존 scoped pass를 유지하고 전체 trajectory/production claim으로 확대하지 않는다.
- A07: 고정 공통 tolerance에서 Diffsol accuracy gate 실패가 남아 있다. 재실행으로 바꾸지 않는다.
- A08/A09: 명시된 A04/A07 의존성이 닫힐 때까지 blocked다.
- A12: 각 scope dependency가 닫힌 부분만 재판정한다. 전체 production과 speed는 열린 증거 없이 승격하지 않는다.
