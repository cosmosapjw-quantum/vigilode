# VigilODE adapter 통합 감사 · 2026-10-27

[보고서](REPORT_KO.md) · [판정 JSON](DECISIONS.json) · [다음 개발 단계](NEXT_STEPS.json) · [로컬 Codex 인계서](LOCAL_CODEX_HANDOFF_KO.md)

최신 native 성능 계열과 joint-controller 계열을 통합했다. 최종 소스는 263개 계약 테스트를 통과했고, 두 차례의 구현·측정에서 개선과 반례를 함께 보존했다. 범용 production 승격은 보류하며, 조건을 만족하는 초기 스텝 재사용과 일회 재확인은 명시적 적응 옵션으로 제공한다.

- 강제 감쇠: 기존 선택 정책보다 짝지은 실행시간 중앙값 **10.2% 감소**.
- case5: 일회 재확인만 끈 대조군보다 **20.4% 감소**.
- 완화: 동일 구간 재비교 비용으로 **12.2% 증가**.
- 기본 허용 오차 gyromotion: 물리적 오차 상한 실패를 그대로 보존.

수치는 [최종 분석](benchmark/analysis_r2/ANALYSIS.md)의 5개 짝 블록 결과다. 전체 원자료·실패·출처 해시와 과거 연구 재사용 범위를 함께 제공한다.

검증:

```bash
python3 research/adapter_integration_20261027/tools/verify_package.py
```

이 검사는 파일 무결성·검증 기록의 일치 범위이며 solver를 다시 실행하지 않는다. 실제 재현은 인계서의 명령을 사용한다.
