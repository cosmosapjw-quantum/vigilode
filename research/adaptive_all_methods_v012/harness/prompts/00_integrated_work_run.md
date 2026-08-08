# Integrated GPT-5.6 Work Run

## Outcome

현재 Project의 자료와 상태 문서를 이용해 아래 연구 목표를 끝까지 진전시킨다.

RESEARCH_GOAL: [구체적 결과]

CURRENT_PHASE: [evidence / hypothesis / review / validation / decision / synthesis]

PRIMARY_RQ: [질문]

AVAILABLE_SOURCES: [Project sources / uploaded files / web / connected apps]

## State restoration

`RESEARCH_STATE`, `EVIDENCE_LEDGER`, `HYPOTHESIS_GRAPH`, `DECISION_LOG`, `NEGATIVE_RESULTS`를 읽고 현재 상태를 복원한다. 기존 결론을 자동 수용하지 않고 version과 active assumptions를 확인한다.

## Success criteria

1. 직접 관련 증거가 확보되거나 부족하다고 명시됨
2. 사실, inference, hypothesis가 분리됨
3. candidate의 효과와 약점이 차원별로 분석됨
4. 주요 claim이 evidence ledger와 연결됨
5. 결정이 `promote / hold / reject / reopen` 중 하나로 기록됨
6. state와 closeout이 갱신됨

## Workstream policy

독립 조사만 병렬화한다. 한 결과가 다음 판단을 결정하는 유도·검증·판정은 순차적으로 수행하고, 병렬 결과를 통합한 뒤 진행한다.

## Independent gate

후보 생성자와 분리된 reviewer 관점으로 evidence, hidden assumptions, known limits, physical/mathematical consistency, alternatives, conflicts, testability, overformalization을 검사한다.

## Stop rule

현재 질문에 답할 충분한 증거가 생기면 종료한다. 필수 evidence가 빠졌다면 가장 작은 유용한 후속 검색 또는 검증을 최대 2회 시도한다. 표현 개선을 위해 검색을 반복하지 않는다.

## Deliverables

- research outcome summary
- updated state files
- independent gate report
- decision log
- closeout
