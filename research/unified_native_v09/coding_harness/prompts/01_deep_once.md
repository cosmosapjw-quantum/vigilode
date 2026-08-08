# Phys–Math Codex Deep Run — GPT-5.6

## Outcome

아래 연구 코딩 작업을 계약 확정, 위치 특정, 해결안 선택, 구현, 계층적 검증, 독립 리뷰까지 완료한다.

TASK: [작업]

SCIENTIFIC_PURPOSE: [연구적 이유]

IN_SCOPE: [파일/모듈/동작]

OUT_OF_SCOPE: [제외]

## State restoration

`AGENTS.md`, `SCIENTIFIC_CONTRACT.md`, `VALIDATION_MATRIX.md`, `PLANS.md`, `RUN_STATE.md`, 관련 구현·테스트·로그를 읽는다.

## Pipeline

1. Task contract: task type, observable success, preserved behavior, scientific/numerical constraints, approvals, completion bar.
2. Localize: execution path, data/units flow, fault/change location, tests, baseline, minimal reproduction.
3. Design: 실제로 다른 해결안 최대 3개, 최소 충분한 해결안을 선택.
4. Plan: 파일, 순서, tests, scientific validation, risks, rollback.
5. Implement: minimal coherent patch, behavior change에 test, silent fallback 금지.
6. Validate: syntax/type/lint → targeted → acceptance → regression → invariant/limit → convergence/stability → reproducibility/performance.
7. Independent review: correctness, science, numerics, regression, tests, scope, errors.
8. Closeout: plans/matrix/logs/state 갱신.

## Stop rules

동일 실패가 반복되면 blocker로 기록한다. 전체 repo cleanup을 섞지 않는다. 미실행 검증을 성공처럼 보고하지 않는다.
