# AGENTS.md — PHYS/MATH RESEARCH CODE HARNESS 5.6

## Mission

이 리포지터리는 물리학·수학 연구 코드를 포함한다. 코드는 실행되는 것뿐 아니라 과학적으로 올바르고, 수치적으로 안정적이며, 재현 가능해야 한다.

## Read first

작업 전에 다음을 읽는다:

1. `SCIENTIFIC_CONTRACT.md`
2. `VALIDATION_MATRIX.md`
3. 장기 작업이면 `PLANS.md`와 `RUN_STATE.md`
4. 관련 README와 하위 `AGENTS.md`

독립적인 파일 읽기는 묶되, 한 결과가 다음 탐색 위치를 결정하면 순차적으로 진행한다.

## Current task layer

현재 작업 층을 `diagnose / design / implement / validate / review / document` 중 하나로 명시한다. 다른 층으로 조용히 넘어가지 않는다.

## Task contract

코드 변경 전에 다음을 확정한다:

- 사용자에게 보이는 목표
- 변경 범위와 non-goals
- 보존할 동작
- reproduction 또는 acceptance criterion
- 과학적·수치적 성공조건
- approval boundary
- completion bar

중요한 ambiguity만 질문하고, 사소한 ambiguity는 가정을 명시하고 진행한다.

## Autonomy

승인 없이 가능:

- 파일·로그 읽기
- in-scope 코드 편집
- 비파괴적 테스트, lint, type check, build
- 임시 diagnostics와 local benchmark
- 작업 관련 문서·상태 갱신

사전 승인 필요:

- 새 production dependency
- 외부 시스템 쓰기
- 데이터 삭제·비가역 변환
- public API/파일 형식의 호환성 파괴
- scientific baseline, tolerance, convention 변경
- 요청 범위를 실질적으로 확대하는 리팩터링

## Implementation policy

- 버그는 가능하면 재현 후 수정한다.
- 기능은 observable acceptance criterion을 먼저 정한다.
- 리팩터링은 characterization test로 기존 동작을 잠근다.
- 문제 위치와 데이터 흐름을 확인한 뒤 수정한다.
- 가장 작은 coherent change를 우선한다.
- broad exception, silent fallback, 성공처럼 보이는 실패 처리를 추가하지 않는다.
- unrelated cleanup을 같은 patch에 섞지 않는다.
- 새 abstraction은 반복되는 실제 필요가 있을 때만 도입한다.

## Scientific invariants

관련되는 항목을 점검한다:

- 단위와 차원
- 부호와 normalization
- symmetry/covariance
- conservation law
- known analytic limit
- boundary/initial condition
- positivity/realizability
- convergence/stability
- stochastic seed와 ensemble statistics
- regime of validity

구체적인 값과 convention은 `SCIENTIFIC_CONTRACT.md`를 따른다.

## Validation ladder

낮은 비용부터 수행한다:

1. syntax/import/type/lint
2. targeted test
3. reproduction 또는 acceptance
4. 영향 범위 regression
5. known result/analytic limit/invariant
6. numerical convergence/stability
7. reproducibility/performance sanity
8. independent diff review

전체 검증이 너무 비싸면 가장 관련성 높은 subset과 smoke test를 실행한다. 실행하지 못한 검증은 이유와 차선 검증을 기록한다.

## Parallelism

- 독립된 repo 조사, 테스트 감사, reviewer, 서로 다른 후보 구현만 subagent/worktree로 병렬화한다.
- 동일 checkout의 동일 파일을 여러 agent가 동시에 수정하지 않는다.
- 의존적 작업은 순차적으로 수행하고 병렬 결과를 통합한 뒤 행동한다.

## Independent review

중요한 변경은 구현과 별도의 review pass를 수행한다. correctness, scientific inconsistency, numerical instability, regression, missing test, hidden scope expansion, silent error handling, reproducibility gap을 우선한다.

## Completion bar

- 요청한 동작 구현
- 관련 tests와 validations 통과
- 미실행 검증과 잔여 위험 기록
- diff에 불필요한 변경 없음
- 계획 항목이 `Done / Blocked / Cancelled`

## Final response

변경 내용, 변경 파일, 실행한 검증과 결과, 잔여 위험/막힌 점을 간단히 보고한다.
