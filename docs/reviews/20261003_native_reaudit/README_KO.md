# Native 재감사 복구 산출물

- `REVIEW_REMAINING_KO.md`, `NEXT_DEVELOPMENT_DAG.json`: 아직 하지 않은 연구·개발만 다룬다.
- `EXECUTION_RECORD_KO.md`: 복구, 재현, 실제 완료한 수정과 검증을 다룬다.
- `DERIVATIONS_KO.md`, `verify_algebra.py`: 이번에 검증한 조건부 수학 계약이다.
- `FINAL_VERIFICATION.json`, `evidence/`: 실제 native 실행 결과와 한계다.
- `SOURCE_MANIFEST.json`: 원격/로컬 source identity와 외부 자료 확인 범위를 구분한다.

기존 PR은 #70이며 새 PR을 중복 생성하지 않는다. 원격 게시 여부와 테스트 통과 여부는 `FINAL_VERIFICATION.json`의 서로 다른 필드를 확인한다. 더 오래된 commit의 CI success를 최종 code의 success로 계승하지 않는다.
