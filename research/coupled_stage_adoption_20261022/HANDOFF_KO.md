# 후속 개발 인계 프롬프트

```text
VigilODE의 research/coupled-stage-adoption-20261022 브랜치를 읽고 후속 개발을 진행하라. NCP/SSH를 사용하지 않는다. 기존 checkout에 reset, clean 또는 강제 checkout을 하지 말고, 필요하면 fetch한 최신 원격 tip으로 별도 worktree를 만든다. 저장소의 CHECKPOINT.json에서 구현·시험·benchmark source identity를 구분한다. 보고서만 추가된 이후 commit을 코드 변경으로 오인하지 않는다.

먼저 research/coupled_stage_adoption_20261022/REPORT_KO.md, CHECKPOINT.json, DECISIONS.json, NEXT_STEPS.json을 읽는다. 기존 연구와 데이터는 조건이 그대로이면 재사용한다. 과거 캠페인을 전부 다시 돌리거나 새로운 NCP 작업을 제출하지 않는다.

새 구현은 integrate_coupled_observed + RawStepSession이다. 기존 coupled_or_sequential의 native-K fallback과 새 raw-U fallback을 혼동하지 않는다. ConditionalL2는 사용자가 정당화하는 가정 아래의 f64 추정이며 interval/global error certificate가 아니다. 기본 dispatcher와 지원 범위를 임의로 넓히지 않는다.

우선순위는 N14의 h·상태·epoch에 묶인 정확도 가정과 root 예산, N15의 비용 기반 후보 생략, N16의 같은 세션 내 작은 factor/scratch 재사용이다. 작업별 수용 기준은 NEXT_STEPS.json에 있다. 한 항목을 선택했다면 그 변경에 필요한 테스트와 local benchmark만 수행한다. 비정규·진동·넓은 spectrum에서 실패 또는 비용 증가를 숨기지 않는다. 같은 requested tolerance를 같은 achieved accuracy라고 표현하지 않는다.

기존 검증을 재현해야 할 때에는 REPRODUCE.md의 locked/offline 명령을 사용한다. cache가 없으면 실행 미완료로 기록하고 lockfile을 변경하지 않는다. 결과 디렉터리는 새로 만들고 실패 로그를 보존한다. benchmark는 measurement profile을 사용하며, JSON serialization과 기준해 평가를 timed interval 밖에 둔다. 원래 공간 잔차 검증과 실패 callback 비용은 빼지 않는다.

변경 결과는 코드/시험/수치 결과를 각각 보고하고, 실제 source commit, command, input/output hashes를 보존한다. 범용 production 승격은 좁은 구성요소 검증과 구분한다. 이 작업을 시작하기 전에 이미 통과한 수학 증명이나 과거 캠페인을 새 근거 없이 다시 감사하지 않는다.
```
