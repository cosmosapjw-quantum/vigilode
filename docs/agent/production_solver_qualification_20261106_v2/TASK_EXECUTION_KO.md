# 실행 규약

1. 패키지 파일, MANIFEST, checksums와 validator를 먼저 확인한다.
2. branch/HEAD/tree/origin/status가 manifest와 일치하는지 확인한다. 다르면 과거 finding을 현재 사실로 취급하지 말고 preflight를 다시 한다.
3. TASK_QUEUE에서 하나의 task만 선택하고 dependencies 및 exact read/write allowlist를 따른다.
4. Owner가 task ID, base SHA, exact write_allowlist, acceptance를 승인하기 전까지 read-only다.
5. Task 전용 worktree를 exact base에서 만들고 원래 worktree, 기존 untracked 파일, ledger와 official DAG를 보존한다.
6. .env, credential, token, private key를 읽거나 출력하지 않는다. commit/push/merge/rebase/tag/publish/release 하지 않는다.
7. 수식의 정의·가정·norm·interval·rounding·stage·residual을 코드와 매핑한다.
8. 최소 반증 테스트를 먼저 만들어 base에서 RED를 확인한다. 의도한 결함과 빌드/환경 실패를 분리한다. Golden 값은 권위 결정 없이 바꾸지 않는다.
9. Allowlist 밖 수정 금지. tolerance/default/API/threshold/DAG/ledger 변경은 별도 승인.
10. 타깃 테스트, negative controls, fmt/lint 실행. 명령, 환경, exit, 로그, 해시를 남긴다. 미실행은 NOT_RUN.
11. Read-only fresh review 최대 1회, review-of-review 금지.
12. Evidence schema를 따라 PASS/FAIL/CONCERN/BLOCKED/NOT_RUN을 구분한다.
13. Relevant gate가 검증되고 owner가 scope를 승인하기 전까지 최종 verdict는 HOLD다.
