# A01–A12 collective completion

원래 12개 노드 모두의 native 구현·증명·실행·독립 감사를 통합한 전달 패키지다. 선언된 인증 API 범위는 PASS이며, broad production 및 일반 속도 승격은 HOLD다.

- [전체 한국어 보고서](REPORT_KO.md)
- [Local Codex 전달 prompt](CODEX_HANDOFF_KO.md)
- [Machine-readable DAG](DAG.json), [local 작업 목록](HANDOFF_TASKS.json), [추가 발전 단계](NEXT_STEPS.json)
- [독립 최종 gate](nodes/A12/FINAL_GATE_REVIEW.md), [scope별 결정](nodes/A12/DECISION.json)
- [최종 실행 기록](FINAL_EXECUTION.json), [source manifest](SOURCE_MANIFEST.json), [evidence manifest](EVIDENCE_MANIFEST.json)
- [작업 중 추가된 upstream 연구 검토](REMOTE_DELTA_REVIEW.json)

최종 묶음 검사는 native 65개, compile-fail 2개, Python 12개가 통과했다. 추가 legacy Leja oracle는 native 1개 및 numeric comparisons 218개가 통과했다. 새 36행 benchmark는 정확도 비교 gate를 통과했지만 8-mode에서 structured 경로가 더 느렸으며, 속도 HOLD를 유지했다. 원시 실패와 이전 증거도 보존한다.

컴파일 없이 전달된 source identity와 전체 node coverage를 확인한다.

```bash
python3 tools/validate_all_dag_completion.py
```

필요한 local compile/replay 명령은 handoff에 있다. 변경 없는 published 연구 전체를 재실행하지 않는다.
