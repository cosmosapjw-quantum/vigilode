# RVJ 연구 이력 원본 백업

상태: **로컬 전체 준비 완료 / GitHub는 인덱스만 게시, 원본 전송 대기**.

이 브랜치는 추가 연구, 코드 통합, 과거 결과의 정정 또는 검증 승격 없이 이 대화에서 생성된 연구 산출물을 보존하기 위한 별도 연구 브랜치다. 기존 main 및 PR70 브랜치에는 변경하지 않는다.

- 저장소: `cosmosapjw-quantum/vigilode`
- 브랜치: `research/rvj-full-history-20261004`
- 생성 기준: 조회한 `main`의 `8d0c79184e09efb5bdadc24a6315c60a71a44264`
- 대상: 독립 연구 01–11, Vigilode 적용 리뷰 V01, native/PR70 재감사 V02.
- 원본 ZIP: 13개, 합계 77,284,475 bytes.
- ZIP 내부 원본 파일 경로: 4,452개. 부모 snapshot 중복을 제거하거나 정규화하지 않았다.
- 원본 ZIP, 펼친 파일, 개별 첨부·receipt를 포함한 로컬 보존 경로: 4,550개.

## 원본 보존 규칙

각 단계의 `archive/`에는 원본 ZIP 바이트, `files/`에는 그 내부 전체 파일의 원본 바이트, `attachments/`에는 대화의 개별 첨부와 delivery·publication receipt를 둔다. `ARCHIVE_INVENTORY.json`은 ZIP 내부 경로와 열람용 경로 대응을 기록한다. 컨테이너 최상위 prefix만 열람용 경로에서 벗기며 내용은 바꾸지 않는다. 이미 각 ZIP에 포함된 부모 snapshot과 source는 그대로 둔다.

실패, 경고, timeout, 반례, HOLD, 미해결 주장, 당시의 push 미완료 기록을 고치거나 제거하지 않는다. 과거 test count는 그때의 기록이다. 이번 백업에서는 과학 계산·CAS·테스트를 재실행하지 않았다.

원본 파일을 고친 통합 보고서를 만들지 않았다. 여기의 인덱스, 보존 manifest, 인계 및 게시 상태만 새 백업 메타데이터다. Stage 04 ZIP 밖의 두 하네스 ZIP은 개별 첨부로 별도 보존했다.

## 인계

`HANDOFF_KO.md`를 먼저 읽고, 단계 11의 원래 `HANDOFF_KO.md`, `RESEARCH_NOTE_KO.md`, `NEXT_DEVELOPMENT_DAG.json`, `FINAL_VERIFICATION.json`, `CLAIM_LEDGER.json` 순서로 원본을 확인한다. 이 백업은 다음 연구의 자동 실행 승인이 아니다.

## 전송 상태

현재 GitHub 연결의 쓰기 도구는 텍스트 content를 받지만 로컬 바이너리 파일 전송 인자가 없고, 이 컨테이너의 직접 Git/HTTPS 연결은 DNS/connection 오류로 실패했다. 따라서 원본 13개 ZIP과 4,550개 보존 경로가 GitHub에 올라갔다고 보고하지 않는다. `ARCHIVE_CATALOG.json`의 SHA-256은 전송 대기 중인 로컬 원본을 식별한다.

대용량 전송을 위해 다른 계정/서비스를 임의로 사용하거나 공개하지 않았다. PR은 만들지 않았고 merge/force push도 하지 않았다.

## 범위의 빈틈

첫 패키지 이전의 아이디어·증명 대화 두 응답은 대화에만 있고, 별도의 원문 파일 바이트가 확인되지 않았다. 이를 추측으로 다시 써서 원본이라고 만들지 않았다. Rust 배포본·vendor·offline repo 등 독립 실행환경 첨부는 연구 결과 패키지에 새로 포함하지 않았으며, 원래 ZIP 내부의 입력 사본은 그대로 보존했다.
