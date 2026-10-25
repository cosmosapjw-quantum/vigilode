# 적응 제어·방법 선택·인과적 보정의 통합

2026-10-25. 원격 기준 소스는 `4fe67602502440d76aa9dd245c6b1d67558c14bf`다. 변경 코드는 opt-in이며, 범용 production 승격 판단은 HOLD다.

- [연구·감사 보고서](REPORT_KO.md): 현재 고유 알고리즘의 위치, 실제 증명, 구현, 측정 결과와 남은 병목.
- [판정 및 발견 사항](DECISIONS.json): 범위별 채택 판단과 필요한 후속 검증.
- [구체적인 다음 개발](NEXT_STEPS.json): 코드 위치·의존성·수용 기준·모델 라우팅.
- [최종 검증 기록](VALIDATION.json): 소스·실행 identity와 재사용 근거.
- [최종 pilot 분석](evidence/FINAL_PILOT_ANALYSIS.md), [수치 요약](evidence/NATIVE_PILOT_SUMMARY_FINAL.json).
- [로컬 Codex handoff](LOCAL_CODEX_HANDOFF_KO.md): 기존 checkout과 원자료를 보존한 후속 작업.
- [수학적 유도](theory/DERIVATIONS_KO.md), [기존 아이디어 재사용 지도](history/REUSE_KO.md), [독립 검토](review/REVIEW_KO.md).
- [연구 루프 기록](RESEARCH_WORKFLOW.json), [파일 무결성 목록](ARTIFACT_MANIFEST.json).

최초 pilot과 실패한 수정도 보존했다. `switching/native/`는 첫 정책, `validation/transient_repair/`와 `switching/transient_diagnostic/`는 즉시 재비교 수정의 실패, `switching/native_r2/`는 다음 수락 구간으로 비교를 옮긴 최종 정책이다. 서로 다른 버전의 결과를 하나의 표본으로 합치지 않는다.

최종 선택 pilot은 9개 문제, 3개 경로, 각 1회 warmup과 5회 측정으로 구성된다. 모든 경로·스레드 조합을 탐색하지 않았다. execution·causal pilot은 해당 코드가 변하지 않았으므로 검증된 실행을 재사용했다. 이 환경에서 측정했으며 NCP나 사용자 로컬 프로세스에는 접속하지 않았다.
