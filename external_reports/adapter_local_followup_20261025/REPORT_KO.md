# Adapter 로컬 후속 진단 (2026-10-25)

## 실행 범위와 보존

연구 시작점은 `research/adapter-diagnostics-20261025`의 `5db28c42dccaf57cafcebf3752aec2d50360a8a6` (tree `9cf8d5a450d5571083c2f7b2b023d28daad0fe71`)이다. 기존 checkout을 건드리지 않고 `/tmp/vigilode-adapter-review.rjwwJ4Cn/checkout`에 detached worktree를 만든 뒤 별도 브랜치 `research/adapter-local-followup-20261025`에서 작업했다. 최종 **측정 소스**는 `d914b4c8a53405ea846f710e07f77fff5f45b16f` (tree `438b9602dae0728121363fa0a40a8acb58bfb2c3`)이다. 아래에 붙을 보고서 commit은 측정 소스가 아니다.

기존 원자료 `/tmp/vigilode-local-run-20261024.geLVCp9Q`를 읽기 전용으로 조사한 후 `/home/cosmosapjw/vigilode-local-archive/n01-vigilode-local-run-20261024.geLVCp9Q`에 create-only 백업했다. 25,122개 정규 파일, 7,628개 디렉터리, 38개 심볼릭 링크 및 세 기존 checksum manifest가 양쪽에서 일치했다. [백업 영수증](BACKUP_RECEIPT_N01.json)은 원본 source/binary/reference 연결과 시도한 복사 방법을 보존한다. 기존 standard, continuation, 실패, timeout과 중지된 장기 campaign은 수정하거나 재개하지 않았다. 새 전체 원자료는 `/tmp/vigilode-adapter-local-followup.1PRDjIEs`에 남겨 두었다. [원자료 해시 목록](RAW_MANIFEST_FINAL.json)은 새 358개 파일·바이너리, 104,414,425 bytes를 연결하며 indexed 경로 누락과 SHA 불일치가 모두 0이다. 대용량 JSONL은 저장소에 넣지 않았다.

CPU는 AMD Ryzen 9 5900X (물리 12/논리 24), 메모리는 약 31 GiB이다. 새 실행은 affinity CPU 0, solver thread 1, 한 번에 한 timed process, Cargo jobs 2, Rust 1.94.1, locked offline 의존성, 외부 target `/tmp/vigilode-adapter-local-target-20261025`를 사용했다. `Cargo.lock` SHA-256은 `efb379071fa42c2b6bdbe969163ac2f41cf318c262c5c67d0910450298333351`이다. [measurement 바이너리 해시](BINARY_SHA256_FINAL.json) 및 각 시도 영수증이 소스와 연결된다. 단일 프로세스 벽시계는 전부 `diagnostic_only`이며 결정론적 작업량과 구분한다.

## 원인 진단

* **case5/KS:** 새 DOPRI 시도 계측의 각 2,000회 중 case5는 1,942회 수락, t=0.00636/2; KS case12는 1,946회 수락, t=0.03764/10이었다. 출력 착지로 잘린 시도는 없었다. 마지막 500개 수락 h 중앙값은 각각 3.30547e-6, 1.92373e-5이고 선언된 음의 실축 선형 안정 한계의 0.999668, 0.999630배다. 낮은 거절률과 이 h 일치는 **명시법 안정성 제한**을 강하게 지지한다. 비선형 전 단계의 안정성 증명이나 모든 적응 결함 부재 주장은 아니다. [계산과 원시 SHA](TRACE_SUMMARY.json), [분석 스크립트](analyze_attempt_trace.py)를 제공한다. 첫 계산에서 KS 선형항 부호를 잘못 둔 stdout도 로컬에 보존했고, 수정 후 별도 retry exit 0을 기록했다.
* **chirp:** 선언된 equivariant cubic chart와 원래 물리 DOPRI를 t=0..8의 같은 129개 관측점, 전체 원래 상태 `[x,v]`의 최대 성분 오차 상한 1e-4로 비교했다. chart 오차 최대 9.04e-9, 원래 좌표를 `atol=5e-9`로 조인 경로는 최대 5.83e-5이다. 기본 물리 `atol=1e-6`은 최대 0.01105로 상한에 실패하여 비교 후보에서 제외했다. 기본 물리 진폭 오차는 8.22e-7, 주값 위상 잔차는 2.39e-4 rad이었다. 주값 위상은 unwrap 누적 위상 인증이 아니다. mpmath 80자리와 binary64 해석식의 129점 차이는 최대 1.54e-10이고 경험적 참조 불확실도 3.09e-10을 사용했다. [참조 교차검사](ANALYTIC_REFERENCE_CROSSCHECK.json)는 엄밀 구간 인증이 아니다.
* **gyromotion:** 기본 `rtol=1e-6, atol=1e-9`의 같은 65점 전체 6D 오차는 최대 0.0313, 진폭 3.69e-4, 주값 위상 0.0319 rad로 실패했다. `rtol=1e-9, atol=1e-12`에서는 전체 오차 3.87e-5로 상한을 통과한다. 해석식의 mpmath 80자리 차이는 최대 4.90e-14이다. 원자료는 `final_diagnostics/gyro_*.stdout`에 있다.

## 같은 물리 오차 상한의 제한 비교

선택 경로와 직접 대안 하나를 같은 시작 물리 상태, 구간, 호스트, 물리 오차 및 출력 요구에서 비교했다. 각 후보에 warmup 1회, timed 5회, 실행 순서 교대를 적용했다. 총 60개 process가 모두 exit 0이었고, 50개 timed 표본 모두 각 cohort의 오차 상한을 통과했다. [입력 50행](SELECTION_WINDOWS.json)을 고정 [분석기](../../research/adapter_research_20261025/selection_benchmark/analyze_selection.py)로 검사한 [결과](SELECTION_ANALYSIS.json)는 25/25 블록을 `COMPARABLE`, rejected 0으로 분류한다. 분석기 자체도 `performance_claim_authorized=false`를 기록한다. 아래 시간은 후보 wrapper 내부의 setup/solve/pilot/전환을 한 번 포함한 중앙값이며 단위는 초다. endpoint wrapper에서는 이 항목을 따로 계측하지 않아 전체 시간을 `solve_seconds` 봉투에 한 번 기록했고, 별도 `probe_seconds=0`은 측정된 pilot 비용 0을 뜻하지 않는다. 매우 짧은 실행이므로 통계적 승격 근거가 아니다. 범위와 controller wall 시간은 [그룹 요약](NARROW_GROUP_SUMMARY.json)에 있다.

| 범위/출력 | 선택 경로 (초) | 직접 대안 (초) | 블록별 선택/최소 후보 중앙값 | 최대 물리 오차 선택/대안 |
| --- | ---: | ---: | ---: | ---: |
| chirp 129점, cap 1e-4 | 선언 chart 0.000204 | 원래 좌표 0.063428 | 1.00 | 9.04e-9 / 5.83e-5 |
| rotation endpoint, cap 1e-5 | evidence switch 0.002059 | DOPRI 0.000416 | 8.01 | 9.25e-7 / 9.27e-7 |
| stiff-forced endpoint, cap 1e-5 | evidence switch 0.000270 | RODAS 0.000172 | 1.96 | 3.24e-11 / 1.36e-10 |
| gyromotion endpoint, cap 1e-4 | 기본 switch 0.009146 | DOPRI 0.002707 | 3.09 | 3.89e-5 / 3.90e-5 |
| case5 endpoint, cap 1e-4 | 기본 switch 0.002620 | RODAS 0.000193 | 14.60 | 9.86e-10 / 1.04e-7 |

chirp의 이득은 **선언된 특화 chart**에 한정한다. rotation에서 선택 adapter는 RODAS 진입 pilot 6개를 버리며 1,150 RHS를 썼다(총 6,395 RHS); 직접 DOPRI는 5,227 RHS다. gyromotion 기본 switch도 버린 RODAS pilot 4개에 1,019 RHS를 쓰며 총 26,018 RHS이고, 직접 DOPRI는 24,403 RHS다. case5는 초기 DOPRI 및 버린 pilot을 포함해 switch 1,633 RHS/232시도, 직접 RODAS 307 RHS/39시도다. 전환 선택의 손실 원인을 **반복 pilot 및 처음 명시법을 택한 비용**으로 분리할 수 있으나, 고정 임계값만 낮춰 늦은 강성 전이를 다시 놓치게 하는 변경은 적용하지 않았다. 같은 바이너리의 추가 24개 제한 실행에서 rotation의 지속 DOPRI tenure는 reset 대조군 대비 6,395 대 6,720 RHS로 작업량을 줄였지만 직접 DOPRI보다 여전히 비싸다. stiff-forced는 양쪽 706 RHS였다. [진단 그림](COST_RATIO_DIAGNOSTIC.svg)은 각 5개 블록의 최소·중앙·최대를 표시한다.

case5의 외부 Wolfram 고정 endpoint 참조는 SHA-256 `3430e71022f24211cff9169a8878e018976de0d3ff09f479629a2014774aa9e8`, 보고 불확실도 1e-12이다. 다른 analytic 범위는 [그리드 교차검사](ANALYTIC_REFERENCE_CROSSCHECK.json)와 [짧은 endpoint 교차검사](ENDPOINT_ANALYTIC_CROSSCHECK.json)의 경험적 불확실도를 사용한다. 모두 각 cap의 10% 이하다. 서로 다른 **달성** 오차를 같은 것으로 취급하지 않았고, 공통 오차 **상한** 내 후보만 비교했다.

## 코드와 검증, 남은 작업

새 계측은 DOPRI 요청/실제 h, 오차 norm, 수락·거절, clipped 여부, RHS 차이와 native 세션 통계를 노출한다. `carry_explicit_step=true`의 연속 DOPRI 창은 PI/FSAL/workspace를 유지하고, 수락된 RODAS 진전 후에는 세션을 폐기한다. 기존 false 경로는 유지한다. 독립 감사가 마지막 attempt의 accepted probe 뒤에 max_steps=0 세션을 만들던 결함과 Forbidden zero-span 기록 누락을 발견했다. 별도 최소 수정 commit에서 두 결함을 고쳤고, 재심사는 **scoped publication PASS**로 판정했다. [36개 전환/세션 계약 시험](tests/switch_budget_repair/), [계측 사용 시 warmed finite advance 무할당 시험](tests/vg1_enabled/) 1개가 모두 exit 0이다. VG1의 범위는 한 수락 시도와 quota yield이며, 생성·출력·실패 경로까지 측정한 결과는 아니다. locked offline measurement 빌드도 exit 0이다. 이전 실패와 재시험 영수증은 로컬에 보존했다.

이전 case22의 4,096차원 병렬 calibration은 같은 endpoint 오차 1.2563e-7에서 data thread2/4/8/12의 3회 중앙값 2.3368/1.8688/2.1519/1.7802초로 산포가 크다. 원본 [standard identity](BACKUP_RECEIPT_N01.json)는 이전 소스 `aaf1f834...`에 속하며 새 측정과 합치지 않았다. 별도 case20 1,024차원 warmed RHS/Jacobian/JVP callback은 allocation 0 및 Jacobian 상대 residual 5.72e-13을 보였지만, integrated VigilODE solver의 output parity·시간 영수증으로 연결되지 않았다. 큰 격자 실행 adapter는 `BLOCKED_CONNECTION`이다. 이를 병렬 속도 성과로 적지 않는다.

[전체 N01–N08 판정](DECISIONS.json): N01 완료, N02/N03/N07 제한된 부분 완료, N04 chart 특화, N05 선택 정책 최적화 HOLD, N06 integrated callback 연결 차단, N08 scoped packet 준비. 특히 RODAS의 지속 tenure, 안전한 epoch/trend 재탐색, 일반 chart/noncommuting control, 큰 격자 integrated 출력, 독립 전이 holdout은 남았다. broad production 판정은 **HOLD**다. 다음 재개점은 N05에서 기존 늦은 강성 전이 반례와 위치가 다른 전이를 둘 다 지키며 반복 pilot 비용을 제한하는 정책을 설계·검증하는 것이다. 과거 standard 행은 소급 승격하지 않는다.

재현: 이 디렉터리의 [정확한 제한 실행 스크립트](run_narrow.py), [선택 입력 생성기](make_selection_packet.py), [참조 검사](check_analytic_reference.py), [계측 요약](analyze_attempt_trace.py)와 `tests/`의 명령·stdout·stderr·exit를 참조한다. `run_narrow.py`는 이번 호스트의 고정 경로를 사용하며 새 실행 시 새 RUN_ROOT로 복사해 경로를 바꿔야 한다. 새로운 측정은 먼저 소스와 바이너리 SHA를 다시 확인하고 기존 원자료/영수증을 덮어쓰지 않아야 한다. NCP·SSH·cloud는 사용하지 않았다.
