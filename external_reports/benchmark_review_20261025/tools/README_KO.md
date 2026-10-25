# 종료된 로컬 측정값 내보내기

이 도구는 솔버를 실행하지 않는다. 현재 벤치마크를 그대로 둔 채, 종료된
영수증과 원자료에서 시간·오차·작업량·호스트 자원 수치를 읽는다. 상태 벡터는
내보내지 않는다. 출력은 기존 실행 폴더 밖의 **새 디렉터리**에 만든다.

```bash
python3 -B external_reports/benchmark_review_20261025/tools/export_local_measurements.py \
  --run-root "$RUN_ROOT" \
  --output "$NEW_EXPORT_DIRECTORY" \
  --task-index research/local_frontier_native_diffsol_20261024/audit_packet_20261025/LEGACY_STANDARD_1539_STATUS_INDEX.json
```

`RUN_ROOT`는 사용자의 실제 벤치마크 출력 디렉터리다. Git checkout이나 도구가
놓인 디렉터리가 아니다. `--task-index`는 기존 1,539개 작업과 대응시킬 때만
필요하다. 최종 출력은 `MEASUREMENTS.jsonl`, `EXPORT_SUMMARY.json`이다.

`RAW_VERIFIED`는 원자료 해시 및 저장 결과의 동일성을 뜻한다. 기존 정확도
판정을 바꾸지 않는다. `timing_reanalysis_candidate`는 같은 문제·소스·호스트·
출력조건·시간측정 범위를 확인한 뒤 재분석할 후보다. 이 필드 자체가 새로운
정확도 보증이나 속도 비교 결과는 아니다.

START만 있고 종료 영수증이 없는 작업은 `INCOMPLETE_ACTIVE_OR_INTERRUPTED`로
기록한다. 이 도구는 프로세스를 확인하거나 정지시키지 않는다. timeout,
솔버 실패, 참조 미확정, 원자료 누락 및 해시 불일치는 서로 구분해 남긴다.
읽는 도중 파일이 바뀌거나 사라지면 해당 항목을 비교 후보에서 제외한다.

같은 실행의 `RECEIPT.json`과 `COMPARISON_RECEIPT.json`이 모두 있으면 공통
`raw_evidence_key`를 사용해 중복 관측으로 세지 않는다. 여러 경로에 복사한
동일 영수증도 `receipt_sha256`으로 중복을 제거한다. 시도 이력은 보존하며
성공한 재시도만 고르는 방식으로 실패를 지우지 않는다.

검사는 다음과 같이 실행한다.

```bash
python3 -B -m unittest discover \
  -s external_reports/benchmark_review_20261025/tools \
  -p test_export_local_measurements.py -v
```

영수증과 개별 원자료 파일의 읽기 상한은 64 MiB다. 더 큰 파일은 건너뛴
이유를 남긴다. 표준 Python만 사용하며 패키지를 설치하지 않는다.
