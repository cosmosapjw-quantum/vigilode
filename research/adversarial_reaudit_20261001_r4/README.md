# VigilODE R4 re-audit · 2026-10-01

[상세 한국어 보고서](REVIEW_KO.md) · [기계 판독 bundle](AUDIT_BUNDLE.json) · [26개 개발 단계 상세](NEXT_DEVELOPMENT_KO.md) · [DAG JSON](NEXT_DEVELOPMENT_DAG.json) · [재현 절차](REPRODUCE.md)

감사 소스 `1c54194123ee6abc6daa512e8574922f510b4e2c`. 실행 전 원격 사전등록 `b2914f3e3c03eda60a8547619db6284aac8250a2`.

**REWORK: 9 findings (8 P2, 1 P3).** 144개 새 native 관측을 독립 재생했고 정확 산술/고정밀 oracle로 검산했다. 생산 코드는 수정하지 않았다. 기본 RODAS5P 경로 전체의 실패를 입증한 결과가 아니며 각 finding의 공개 research API/reference implementation/timing authority 범위를 확인해야 한다.

전체 offline build 통과. 735 named tests 중 671 PASS, 4 ignored, 20 timeout 미확정, 40 예산상 미실행. 변경된 15 harness는 68 PASS + 1 ignored. **전체 suite PASS 및 성능 승격은 하지 않는다.**

연구 후보: 실제 step 비율 BDF, unequal-half Radau 보정, scale-safe outward norm, inverse-factorial, 단계수별 doubling, diagonal certificate 분리, exact session-median interval. 이 후보들은 원시 증거와 가정 아래에서만 채택하며 생산 반영은 다음 개발 단계다.

| 읽을 항목 | 파일 |
|---|---|
| 결함과 우선순위 | [FINDINGS.json](FINDINGS.json) |
| 이전 21 node closure | [CLOSURE_MATRIX.json](CLOSURE_MATRIX.json) |
| 수학/구현 주장과 제한 | [RESEARCH_CLAIMS.json](RESEARCH_CLAIMS.json) |
| 다음 개발 단계와 acceptance | [NEXT_DEVELOPMENT_DAG.json](NEXT_DEVELOPMENT_DAG.json) |
| 실행 숫자 | [NUMERIC_RESULTS.json](NUMERIC_RESULTS.json) |
| 독립 검토 | [decision/FINAL_REVIEW.json](decision/FINAL_REVIEW.json) |
| 전체 테스트 상태 | [evidence/runtime/TEST_COVERAGE.json](evidence/runtime/TEST_COVERAGE.json) |
| 테스트 나머지 계획 | [evidence/runtime/LOCAL_REMAINDER_PLAN.json](evidence/runtime/LOCAL_REMAINDER_PLAN.json) |
| 원문 및 비-Arnoldi/homotopy 제안 | [literature/RESEARCH_DIRECTION_KO.md](literature/RESEARCH_DIRECTION_KO.md) |
| 출처/등록 bytes | [INPUTS_SHA256.json](INPUTS_SHA256.json), [SOURCE_MANIFEST.json](evidence/source/SOURCE_MANIFEST.json) |
| 무결성 검사 | [VALIDATION.json](VALIDATION.json), [MANIFEST.sha256](MANIFEST.sha256) |

각 lane 폴더에 계약, Rust probe, Python oracle, stdout/stderr와 execution receipt가 있다. 최초 환경/검토 script 오류와 미실행 사항도 보존했다. lane owner 문서의 `PENDING_ROOT_REVIEWER`는 작성 시점의 상태이며, 통합 claim의 최종 권위는 위 독립 `FINAL_REVIEW.json`을 따른다. 실패를 지우지 않는 append-only ledger는 저장소 `research/LEDGER.jsonl`에 있다.
