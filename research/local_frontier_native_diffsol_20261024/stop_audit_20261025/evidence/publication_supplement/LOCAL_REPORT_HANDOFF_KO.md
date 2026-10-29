# VigilODE 로컬 측정 읽기 전용 인계 결과

- 보고서 worktree: `/tmp/vigilode-independent-report.Iavhh9Az/checkout` (detached `abf9c5683d810a703d818f8f0c8abec54fbe8850`, tree `df44f97cffdeb9c707bdbea72524bbc8b0902e79`, clean).
- 실행 RUN_ROOT: `/tmp/vigilode-local-run-20261024.geLVCp9Q` (`IDENTITY.json`에서 확인). 기존 checkout, 실행, 바이너리, config, raw에는 쓰지 않았다.
- 주 export: `standard_snapshot_20261025T1140/MEASUREMENTS.jsonl`, `EXPORT_SUMMARY.json`. 읽기 전용 분석 보충: `READONLY_AUDIT_INDEX.json`. 분석 스크립트와 각 명령의 stdout/stderr는 이 디렉터리 상단에 있다.
- exporter 단위 검사 15개 exit 0, 독립 검사 9개 exit 0, compare 검사 8개 exit 0. exporter exit 0, 보충 인덱스 exit 0. 명령은 아래에 있다.
- `MEASUREMENTS.jsonl` SHA256 `d4467ecdf53ca88fa9c668a2b3d51d34dd9bf5b8c26ae1323874e3d7c68b8d8d`; `EXPORT_SUMMARY.json` SHA256 `fb43b3e11e0a6ae900b5f47d459ce6a19c19aa9875040d7bd377f69782375bb9`; 보충 인덱스 SHA256 `7bc97d232ab981a7925dd6a47e7f2c4f7527f64ff961cf2d71a290cdf6dc8111`.
- 1539개 원래 standard task 모두 `EXACT_RECEIPT_BYTES_FOUND`. 전체 RUN_ROOT에서 2565 receipt row: raw 검증 2417, 증거 이슈 124, 형식 미지원/비종결 16, 읽기/형식 오류 8. 실행 상태는 완료 2315, timeout 224, process failure 2, 미상 24. 이는 RUN_ROOT 여러 campaign의 합계이며 standard 1539개와 같은 모수가 아니다.
- `raw_evidence_key` 중복 0개. 이 결과가 같은 실행의 모든 wrapper 중복 부재를 일반적으로 증명하지는 않는다. 각 receipt의 SHA와 raw key를 보존하고 다른 campaign으로 합산할 때 재확인한다.
- 추출기 incomplete START 31개는 합성 시험 fixture 27개와 controller 형식의 참조 작업 4개다. 후자 8/9/11/20에는 별도 `CAMPAIGN_SUMMARY.json`이 있다: 8/9/20 controller 완료, 11 첫 필수 출력 실패. 11의 독립 과학 판정은 `FAILED_REFERENCE`; 20은 독립 리뷰상 `PASS_SCOPED_ADMITTED_REFERENCE`. 추출기의 incomplete 표시는 이 controller 형식에 대한 schema 한계로 보충 기록했으며 원래 status를 수정하지 않았다.
- 124개 증거 이슈에는 일반 exporter가 Julia/조건부 시험 출력 형식을 지원하지 않는 경우가 포함된다. 8개 대용량 파일은 exporter 64 MiB 읽기 상한에 걸렸다. 이는 해당 raw가 삭제됐다는 뜻이 아니다. 참조 SHA가 각 scalar receipt에 없으면 `MISSING_AT_MEASUREMENT`로 유지하고 독립 참조 영수증으로 별도 연결한다.
- 최초 고정 source `aaf1f834d4c589e734ca7851a56c11bb44f7047a`, tree `03b12c682c688155e0f8b08c52508e04738acb93`; 최초 measurement binary SHA256 `979ffe205b861184231b94b9d30217ad5ccf924878886ae4c19cbeafbd7a80f2`; Cargo.lock `efb379071fa42c2b6bdbe969163ac2f41cf318c262c5c67d0910450298333351`; rustc 1.94.1. 다른 후속 바이너리와 source는 영수증별 identity를 사용한다.
- 실행 당시 preflight: 물리 12/허용 논리 24, affinity CPU 0–23, Linux 7.0.0-38-generic. CPU 모델과 메모리 총량은 preflight 기록이 없어 각각 현재 `/proc/cpuinfo`의 Ryzen 9 5900X와 현재 `/proc/meminfo`의 32759712 kB로 별도 표시한다. 영수증의 per-attempt thread 환경, timing scope, 실제 오차, work counters가 있는 행만 해당 값으로 연결한다.
- 비용: 기존 누적 원장의 188-event 시점 예약/충전 3936.824270822988초에 case20 실제 terminal 45.445903952931985초가 추가되어 3982.27017477592초. 이는 합성 fixture 예약 354초를 포함한다. 190-event append-only 원장 SHA256 `0e03f6a1f32996ecc43cc2909b5a39b70d36b187770b6f2da5a0f66ba674ad76` (마지막 확인). 48시간 새 cap/2026-10-27 08:07 UTC deadline 유지.
- 다음 새 비교에만 report branch `compare.py` SHA256 `d74c2732d70871349574c3c84fdbf5637e459fe59a50df66a4f0179c85065396`을 별도 source lock으로 반영해야 한다. 현재 benchmark checkout이나 과거 판정에는 적용하지 않았다.
- 남은 작업: 참조 미승인 문제의 독립 근거 또는 `FAILED_REFERENCE`, 기존 timeout의 제한된 stall 감시 진단, 소스 고정된 전체 구성원 정확도 cohort, 오류 패밀리 최소 수정과 재심사, 독립 감사 패킷. 이 인계에서는 새 장시간 benchmark와 NCP를 시작하지 않았다.

## 실행 명령과 종료 코드

```text
python3 -B -m unittest discover -s external_reports/benchmark_review_20261025/tools -p 'test_export_local_measurements.py' -v  # exit 0
python3 -B -m unittest discover -s external_reports/benchmark_review_20261025/reviews -p 'test_exporter_adversarial.py' -v  # exit 0
python3 -B -m unittest discover -s research/local_frontier_native_diffsol_20261024 -p 'test_compare.py' -v  # exit 0
python3 -B external_reports/benchmark_review_20261025/tools/export_local_measurements.py --run-root /tmp/vigilode-local-run-20261024.geLVCp9Q --output /tmp/vigilode-benchmark-review-export.wAlHdmeP/standard_snapshot_20261025T1140 --task-index research/local_frontier_native_diffsol_20261024/audit_packet_20261025/LEGACY_STANDARD_1539_STATUS_INDEX.json  # exit 0
python3 -B /tmp/vigilode-benchmark-review-export.wAlHdmeP/build_audit_index.py  # exit 0
```
