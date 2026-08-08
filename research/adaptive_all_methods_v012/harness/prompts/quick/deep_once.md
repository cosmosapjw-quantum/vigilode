# Research Loop — Deep Once 5.6

## Outcome

한 번의 Work run에서 문헌 지형, 가설 공간, 대안설명, 독립 비판, 물리·수학 validation, 최소 결정적 검증, 임시 판정을 완료한다.

## Context restoration

먼저 `state/` 파일과 Project sources를 읽고 현재 version, active assumptions, blockers를 복원한다.

## Execution contract

- 독립 문헌 계보와 독립 reviewer만 병렬화한다.
- evidence acquisition → claim audit → hypothesis → review → validation → decision 순서를 지킨다.
- 후보 생성과 최종 판정을 분리한다.
- 실질적으로 다른 candidate family는 최대 6개, serious candidate는 최대 2개로 줄인다.

## Completion bar

- 핵심 claim마다 evidence 상태가 있음
- strong alternatives가 검토됨
- serious candidate가 independent review와 physics/math validation을 거침
- 각 survivor에 high-information test와 kill criterion이 있음
- 임시 결정이 `promote / hold / reject / reopen` 중 하나임

## Deliverables

1. context/state summary
2. source map과 audited evidence ledger
3. hypothesis graph
4. independent review
5. validation ledger
6. minimal decisive verification package
7. decision log entry
8. updated closeout
