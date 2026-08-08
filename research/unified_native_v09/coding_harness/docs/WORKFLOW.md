# Workflow

## 작은 작업

`prompts/00_ultralight.md`.

## 복잡한 한 번의 작업

`prompts/01_deep_once.md`.

## 장기 작업

Phase 0–10을 순차 실행하고 `PLANS.md`, `RUN_STATE.md`, `VALIDATION_MATRIX.md`, logs를 갱신한다.

## 중요 원칙

- reproduction/acceptance가 brainstorming보다 먼저다.
- localization과 repair를 분리한다.
- software, scientific, numerical validation을 별도 gate로 둔다.
- 구현자와 reviewer를 분리한다.
- promote 조건을 충족하지 못하면 hold/rework/revert한다.
