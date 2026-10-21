# 전체 DAG continuation 정책 — 2026-10-18

## 적용 범위

이 정책은 `DAG.json`의 원래 A01–A12 식별자·acceptance·dependency를 고정한 채, DAG 단위 continuation을 끝까지 실행하고 기록하기 위한 controller 규칙이다. `HANDOFF_TASKS.json`의 local validation task에도 같은 refresh/close 규칙을 적용한다. 노드 구현 또는 기존 수학적 증명을 재작성하는 지시가 아니다.

## 이전 중단의 원인

2026-10-18 local continuation에서 LOCAL-01 뒤 LOCAL-02를 실행했고, 이후 기존 기록에 LOCAL-03/04/05 성공이 이미 존재했다. 첫 응답은 새 LOCAL-02 실행 로그와 client 실행을 추가했지만 이를 canonical receipt, evidence manifest, continuation report에 연결하지 않은 채 응답을 끝냈다.

중단 원인은 테스트가 막힌 것이 아니었다. bounded 실행의 성공을 해당 시점의 사용자 목표 완료와 혼동했고, 완료 상태를 증명하는 마감 절차를 실행 결과와 분리했다. 기존 `LOCAL_COMPLETION.json`은 이전 source commit에서 모든 local task가 닫혔음을 보였지만, 새 실행 revision을 포함하지 않았다. 그래서 새 결과는 로컬 파일로만 남았고, 최신 source identity에 묶인 machine-readable 결과가 없었다.

## 필수 continuation 절차

1. 현재 worktree, dirty state, branch, HEAD, remote ref와 관련 실행 프로세스를 다시 확인한다. 다른 checkout의 사용자 변경은 그대로 보존한다.
2. 매 검증 결과 뒤 authoritative DAG/task state를 다시 읽고 dependency를 계산한다. `PASS`한 노드 하나에서 멈추지 않는다. 모든 `READY` 작업을 수행하고, 그 결과에 따라 새 `READY` 작업을 계속 연다.
3. 실행한 각 bounded task는 원래 acceptance와 scope를 유지하고, command, exit code, source/config identity, raw output, 실패를 보존한다. 미실행 또는 예전 source의 결과를 현재 실행으로 표시하지 않는다.
4. 과거 증거를 재사용할 때는 그 증거의 source/model/assumptions와 현재 입력 identity를 비교해 적용 가능성을 기록한다. 그대로인 경우에만 대규모 campaign을 반복하지 않는다.
5. 실행 후 receipt와 evidence manifest를 같은 revision으로 갱신한다. 역사 기록은 덮지 않고 append-only 보충본을 만든다. 해시 갱신만으로 검증을 대체하지 않는다.
6. 최종 closeout 전에 original DAG의 모든 node ID·acceptance·dependency, local task dependency, 각 상태와 증거를 대조한다. `READY` 또는 `RUNNING`이 남거나 결과가 receipt/manifest에 연결되지 않았으면 계속한다.
7. 완료 판정은 다음을 모두 확인한 뒤에만 내린다: required nodes/tasks가 모두 terminal; terminal 상태마다 적절한 evidence가 있음; source identity가 검증됨; 독립 review 범위가 올바르게 유지됨; remaining HOLD와 실행하지 않은 범위가 보고됨; 공개가 허용된 경우 정확한 bundle이 지정 branch에 게시되고 원격 ref가 확인됨.
8. 노드 완료를 production 또는 speed promotion으로 확장하지 않는다. `HOLD`, `DIAGNOSTIC_ONLY`, `EstimateOnly`, 미측정 항목은 그대로 유지한다.

## 현재 적용된 재개 상태

원래 A01–A12 DAG는 유지된다. 2026-10-18 R02 local supplement는 최신 branch source에서 source coverage, changed-scope native contracts와 handoff nonlinear client를 검증했다. 이전 450행 benchmark는 code input이 바뀌지 않았음을 확인해 재사용하고, timing을 `DIAGNOSTIC_ONLY`로 둔다. `LOCAL_VALIDATION_R02.json`과 `LOCAL_EVIDENCE_DELTA_R02.json`이 이번 revision의 receipt와 파일 hash를 결속한다.
