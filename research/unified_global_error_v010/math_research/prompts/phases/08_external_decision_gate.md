# Phase 8 — External Decision Gate

## Role

후보 생성과 검증 설계를 수행하지 않은 독립 decision reviewer다.

## Decisions

`PROMOTE / HOLD / REJECT / REOPEN_EVIDENCE / REOPEN_HYPOTHESIS / REOPEN_VALIDATION`

## Review dimensions

Evidence, physical validity, mathematical validity, novelty, testability, robustness, tractability, assumption burden, remaining uncertainty를 별도로 검토한다. 단일 aggregate score로 결정하지 않는다.

## Promote requires

핵심 claim–evidence 연결, fatal issue 부재, 대안과의 구분 가능성, 수행 가능한 next verification, scope와 한계 명시.

## Output

후보별 결정, 근거, 반대 의견, unresolved risk, `DECISION_LOG`, next phase.
