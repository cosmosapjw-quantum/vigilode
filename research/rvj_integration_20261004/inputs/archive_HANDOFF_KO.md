# RVJ 전 연구 이력 보존 및 다음 스레드 인계

이 문서는 백업·인계를 위한 운영 지시다. 새로운 연구 결론이나 변경된 개발 계획이 아니다.

## 시작 상태

저장소 `cosmosapjw-quantum/vigilode`, 연구 브랜치 `research/rvj-full-history-20261004`, 보존 루트 `research/rvj_full_history_20261004`.

먼저 `TRANSFER_STATUS.json`을 읽어 실제 원격 게시 범위를 확인한다. 인덱스만 있을 때 원본 전체가 복원되었다고 가정하지 않는다. 다음으로 `ARCHIVE_CATALOG.json`, `BACKUP_INDEX.json`, `SOURCE_FILE_MANIFEST.json`이 있으면 대조하고, archive SHA-256과 펼친 원본 바이트를 확인한다. 검증은 백업 무결성 검사이며 과학 테스트 통과를 뜻하지 않는다.

## 원본의 권위

각 단계 원본 ZIP과 그 안의 파일을 기준으로 삼는다. 단계별 상충·철회·경고는 그대로 유지한다. 뒤 단계의 주장으로 앞 단계의 기록을 덮어쓰지 않는다. Delivery 및 push 영수증은 과거 시점의 provider 상태이며 현재 HEAD나 현재 test 결과를 증명하지 않는다.

현재 가장 뒤의 연구 원본은 `stages/11_rvj_independent_loop11_20261004/files/`에 있다. 원래의 `HANDOFF_KO.md`, `RESEARCH_NOTE_KO.md`, `NEXT_DEVELOPMENT_DAG.json`, `CLAIM_LEDGER.json`, `FINAL_VERIFICATION.json`, `code/`, `tests/`, `evidence/`를 읽는다. 파일이 아직 원격에 없으면 전송 대기 blocker를 유지한다.

전체 연구 이력은 `stages/01_*`부터 `stages/11_*`까지 원래 순서로 놓았다. `V01_*`은 Vigilode 적용 리뷰, `V02_*`는 native audit/PR70 재개 패키지다. V02 overlay나 patch를 이 연구 브랜치의 제품 소스에 자동 적용하지 않는다. 이는 연구 결과의 보존본이다.

## 다음 연구 스레드가 지켜야 할 경계

1. 현재 사용자의 별도 연구 실행 요청이 있기 전까지 백업 확인과 인계만 한다.
2. 연구 실행을 요청받으면 단계 11의 원래 남은 DAG에서 고른다. 이 백업 작업에서 새로운 연구를 추가하거나 우선순위를 바꾸지 않았다.
3. 원래 generic RVJ 반례·approximate-W/embedded 실패·원본 누락 및 독립 심사 HOLD를 유지한다.
4. 원래 reference code, exact algebra, numerical oracle, native 구현, wall-clock 성능 주장을 구분한다. 과거 PASS를 이번 실행의 PASS로 상속하지 않는다.
5. 원본 geometry missing inputs를 추측 생성하지 않는다. 과거 원본 suite나 과학 테스트를 백업 완료 조건처럼 임의 재실행하지 않는다.
6. Vigilode에 다시 통합하려면 그 시점의 원격 branch/PR/commit을 실제로 읽고 별도 요청 범위에서 수행한다. 이 브랜치는 archival research lane이며 기존 PR70을 merge하지 않는다.

## 사용자가 새 스레드에 붙여 넣을 시작 지시

“vigilode의 research/rvj-full-history-20261004 브랜치에 있는 research/rvj_full_history_20261004를 보존 기준으로 사용해. 먼저 TRANSFER_STATUS와 원본 hash를 확인하고, 단계 01–11 및 V01/V02의 원본 인덱스를 읽어. 단계 11 원 HANDOFF/NEXT_DEVELOPMENT_DAG를 기준으로 아직 하지 않은 작업만 계승하되 과거 반례·경고·HOLD·실패 기록을 수정하지 마. 파일 또는 원본이 빠져 있으면 정확히 분류하고, 복원·백업 검증과 새로운 과학 검증을 혼동하지 마.”
