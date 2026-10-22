# 비강성 explicit 경로 연구·구현·검증

검증 소스: `45037fc24e8f8101605f309e1fdb3a9a4e3df56d`. NCP 사용 없음.

- [전체 한국어 보고서](REPORT_KO.md) — 구현, 증명 요약, 성능 도표, 반대 결과, 생산 사용 범위.
- [상세 수학 유도](THEORY_KO.md) · [정확 기호 검산](SYMBOLIC_CHECK.json).
- [기계 판독 판정](DECISIONS.json) · [기존18개+신규10개 개발 단계](NEXT_STEPS.json).
- [재현 절차](REPRODUCE.md) · [복사 가능한 Codex 인계서](HANDOFF_KO.md).
- [벤치마크 상세 검토](BENCHMARK_REVIEW_KO.md) · [원자료 해시 목록](ARTIFACT_MANIFEST.json).

표적 계약 25개와 main90설정·360실행, ensemble4설정·16실행을 검증했다. `1e-6` 정규화 표본 오차 기준에서 9개 비강성 문제의 native RODAS 대비 관측 중앙값 시간비는2.73–8.93이었다. 강성 문제와 큰 벡터의 VODE 비교에서는 반대 결과가 나타났다. 작은 문제의 시간 변동을 포함한 로컬 결과이며 범용 성능 보증이 아니다.

DOPRI5는 지원 범위의 명시적 선택 경로로 채택한다. 범용 production 승격은 **보류**한다. 자동 방법 전환, explicit dense output/events/mass 지원, 장기 진동 정확도, 물리 상태 오차 기준과 대표적인 외부 비교가 남아 있다.
