# VigilODE production solver programme

현재 continuation은 **2026-11-15**, 연구 기반은 `bee5563c51264a7da19c519295bdda2fb317caef`다. 작업 브랜치는 `research/production-unification-20261115`. **Production HOLD**이며 main은 최신 연구 기준이 아니다.

현재 상태는 [CHECKPOINT.json](../../../research/production_unification_20261115/CHECKPOINT.json), 결과는 [REPORT_KO.md](../../../research/production_unification_20261115/REPORT_KO.md), 실행 지시는 [RESUME_PROMPT_KO.txt](RESUME_PROMPT_KO.txt)를 읽는다. SOURCE_LOCK의 authority 및 LOOP_EXECUTION_RECORD의2026-11-12 필드는 역사적 intake다. 2026-11-15 continuation이 현 작업 범위와 근거를 명시한다.

## 실행 계약

- [RESEARCH_DAG.json](RESEARCH_DAG.json): T01–T06 목표와 U01–U11 구체 작업의 관계. U 완료가 G0 완료는 아니다.
- [TASK_CONTRACTS.json](TASK_CONTRACTS.json): 입력 가정·식·허용 경로·테스트·실패 처리·완료 근거.
- [MUTATION_GATES.json](MUTATION_GATES.json): 현 승인과 보호 경계. 이전 docs-only 조건은 역사적 교정 기록으로 보존했다.
- [ARCHITECTURE_CONTRACT.json](ARCHITECTURE_CONTRACT.json), [PACKAGE_LAYOUT_PLAN.json](PACKAGE_LAYOUT_PLAN.json): 기존 vigilode frontend를 확장한다.
- [MATCHED_ERROR_PROTOCOL.json](MATCHED_ERROR_PROTOCOL.json): T01 참조·출력·달성오차 계약. 역사적 실패를 유지한다.
- [NOVELTY_MATRIX.json](NOVELTY_MATRIX.json): 신규성 미확립. 기존 기법의 이름이나 병렬 실행만으로 신규성을 주장하지 않는다.

## 과거 근거

[lineage.json](../../../research/production_unification_20261115/lineage.json)은 branch/PR/commit/blob을 연결한다. [HISTORY_RECOVERY.json](../../../research/production_unification_20261115/HISTORY_RECOVERY.json)은 회수한 자료의 과학적 내용을 요약한다. 모든 과거 대화·삭제된 ref·모든 PR diff의 심층 검증을 뜻하지 않는다.

TURN_01_REPORT_KO.md와 REFERENCE_REPLAY_RECEIPT.json은 이전 범위의 근거다. 이번 체크포인트는 병렬 RHS와 session 경계 수정, SDC 판별 연구를 추가한다. 전체 stiff qualification·matrix-free backend·전체 frontend 통합·geometry·holdout은 남아 있다.

```bash
python3 -I docs/agent/production_solver_program_20261112/validate_program.py
python3 -I research/production_unification_20261115/check_checkpoint.py
```

구조 검사는 이론·수치 성능의 증명이 아니다. 실행 근거는 체크포인트에 별도로 연결한다.
