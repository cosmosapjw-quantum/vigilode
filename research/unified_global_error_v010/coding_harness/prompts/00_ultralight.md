# Research Code Run — Ultralight 5.6

## Outcome

아래 작업을 가장 작은 coherent change로 완료한다.

TASK: [작업]

SCOPE: [허용 파일/모듈]

NON_GOALS: [이번에 하지 않을 것]

## Preconditions

`AGENTS.md`, `SCIENTIFIC_CONTRACT.md`, `VALIDATION_MATRIX.md`와 관련 코드를 읽는다.

작업 유형별 선행 조건:

- 버그: 최소 reproduction
- 기능: observable acceptance criterion
- 리팩터링: characterization test
- 수치 변경: analytic/reference benchmark

문제 위치와 원인이 충분히 확인되기 전에는 수정하지 않는다.

## Implementation

관련 위치만 수정한다. unrelated cleanup, 새 abstraction, 새 dependency, scientific convention 변경은 피하거나 승인 전에 멈춘다.

## Validation

1. targeted test
2. reproduction/acceptance
3. affected regression
4. relevant invariant/known limit
5. diff review

검증할 수 없으면 이유와 차선 검증을 보고한다.

## Final

변경 내용, 파일, 검증 결과, 잔여 위험/막힌 점을 간단히 보고한다.
