# 외부 감사용 상세 자료: switching adapter economy (2026-10-26/27)

Written for: 외부 독립 감사자. 무거운 원자료(campaign raw 864개 파일, 바이너리)는 저장소에 넣지 않았고 로컬 archive `/home/cosmosapjw/vigilode-local-archive/adapter-economy-20261026/`에 `SHA256SUMS.txt`와 함께 보존했다. 원자료 디렉터리별 정렬 해시 요약은 `research/adapter_switcher_economy_20261026/evidence/HOST.json`에 있다.

## 읽는 순서

1. `research/adapter_switcher_economy_20261026/ANALYSIS_KO.md` — 2026-10-25 패킷의 비판적 독해, 병목 순위, 수정, 리뷰 처리(6절).
2. `research/adapter_switcher_economy_20261026/PREREGISTRATION.md` — 등록된 질문·게이트·예측, 리뷰 amendment, run 1/run 2 결과.
3. `research/adapter_switcher_economy_20261026/evidence/` — 게이트(`GATES.json`), 비교표(`COMPARISON.md`), drift 대조, 테스트 기록, 바이너리·호스트 identity. 원장 `research/LEDGER.jsonl` L-0170(대체됨), L-0171.

## 이 디렉터리

- `analysis_full/` — `tools/analyze.py`의 전체 출력(segment별 행 포함). 노드에 커밋된 JSON은 이것에서 `segments`만 뺀 것이다. before(패킷 binary), before-2(drift 대조), after run 1(`636cdc7a`), after run 2(`596dcdfa`).
- `diagnosis/` — 수정 전 진단 분해(P2): 패킷 binary로 bucket별 attempts/RHS/JVP, RODAS 창별 재시작 비용, 결정 순서. 이 자료가 병목 순위의 근거다(공식 게이트 측정 아님).
- `review/` — 독립 리뷰 3개 lens 보고서와 구조화 결과. 24건 중 독립 검증이 완료된 것은 3건(세션 한도). 나머지는 수동 재검토했고 처리 결과는 `ANALYSIS_KO.md` 6절에 있다. MAJOR 1건(band 검증 중 ceiling → 전체 solve 중단)은 `596dcdfa`에서 수정됐다.
- `logs/` — 캠페인·빌드 체인 로그(경로는 `<scratch>`로 치환).

## 재현

`research/adapter_switcher_economy_20261026/tools/{run_campaign.sh,analyze.py,compare.py,results_section.py}`. 호스트 AMD Ryzen 9 5900X, `taskset -c 0`, warm-up 1 + timed 5. 벽시계는 `diagnostic_only`이며 이번 drift 대조(1.83배)로 신뢰 불가 표기됐다. 판정은 결정론적 카운터로만 한다.

## 주장 범위

opt-in switching adapter의 낭비 작업 감소만 주장한다. 물리적 강성 분류, 범용 production 승격, 큰 격자/병렬 성능은 주장하지 않는다. 직접 `Dopri5`/`Rodas5p` 경로는 비트 단위로 동일하다.
