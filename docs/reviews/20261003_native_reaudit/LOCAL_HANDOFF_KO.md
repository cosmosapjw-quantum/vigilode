# 로컬 작업 스레드 인계

대상 repo는 cosmosapjw-quantum/vigilode, 기존 PR은 #70, branch는 audit/rvj-native-followup-20261003이다. 중복 PR을 만들지 않는다. 전달 incremental patch의 검토 기준은 312aa57ada821f383f2a8b4da7b2800e4d227d54, tree fb2bc19fccdf34153b86b1dc69fabc4404a0e478다.

먼저 current remote branch와 local clean state를 확인한다. 원격이 더 진행되었으면 diff를 읽고 충돌/중복 수정 여부를 판단하며 무조건 기존 파일을 덮어쓰지 않는다. 이 패키지의 tests와 evidence는 실제 로컬 실행을 기록하지만 새 source의 hosted CI를 대신하지 않는다.

1. 기존 PR branch의 검토 HEAD 위에 incremental patch를 git apply --check로 점검하고 적용한다. COMPLETE_FROM_BASE patch는5a8d7fe9에서 PR 전체를 복구하는 별도 용도이며 두 패치를 연속 적용하지 않는다.
2. Rust1.94.1, lockfile-compatible vendor를 사용해 VENDOR_DIR=/absolute/vendor bash docs/reviews/20261003_native_reaudit/REPLAY_NATIVE.sh를 실행한다. Cargo의 --frozen을 유지한다.
3. 전달 MANIFEST와 적용된 파일을 대조하고, 완료된 수정은 EXECUTION_RECORD, 아직 하지 않은 작업은 REVIEW_REMAINING/NEXT_DEVELOPMENT_DAG로 분리된 것을 유지한다.
4. 추가 source는 causal_majorant.rs와 native_reaudit_radius_contracts.rs뿐이다. 과거 ledger/fixture/holdout/timing authority를 변경하지 말고 기존 PR branch에 non-force push한다. 자동 merge하지 않는다. PR_BODY.md로 기존 #70의 설명을 갱신한다.
5. 새 exact HEAD의 CI와 전체 테스트가 완료되어야 한다. 기존 로컬 debug 전체 실행은600초 timeout이며 완전 PASS로 바꾸면 안 된다. 최종 commit/tree/changed paths와 remote acknowledgement를 새 receipt로 반환한다.

이후 연구는 remaining-only DAG에서 하나를 선택한다. 우선 raw/K physical residual-to-output 판정 또는 bounded-domain Laguerre total admission이다. 완료된 RVJ chart transport helper, input/cache guards와 이미 실행된 L-0034..L-0043을 다시 개발과제로 등록하지 않는다.
