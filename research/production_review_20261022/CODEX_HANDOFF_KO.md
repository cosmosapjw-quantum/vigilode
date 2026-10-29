# Codex 재개 프롬프트

아래를 그대로 전달한다. 이 작업은 원래 checkout을 변경하지 않고 새 worktree에서 수행한다. 보고서의 production 보류 판단은 개별 회귀시험 성공만으로 바꾸지 않는다.

```text
cosmosapjw-quantum/vigilode의 research/production-review-20261022 브랜치를 검증해라.
기존 checkout의 브랜치/HEAD/수정 파일은 보존한다. fetch 후 원격 tip을 기록하고,
그 commit에서 새 detached worktree를 만들어 실행한다. 기존 checkout에 reset/clean을 실행하지 마라.

1. 기존 repo 경로에서:
   git fetch origin research/production-review-20261022
   REVIEW_COMMIT=$(git rev-parse refs/remotes/origin/research/production-review-20261022)
   REVIEW_ROOT=$(mktemp -d /tmp/vigilode-review-XXXXXXXX)
   git worktree add --detach "$REVIEW_ROOT/checkout" "$REVIEW_COMMIT"
   cd "$REVIEW_ROOT/checkout"
   git status --porcelain
   git rev-parse HEAD HEAD^{tree}
   두 identity와 hostname을 receipt에 기록한다. 작업 트리가 clean인지 확인한다.

2. Rust1.94.1과 이 commit의 Cargo.lock에 맞는 dependencies를 사용한다.
   캐시가 없으면 기존 cache/vendor를 확인하고 cargo fetch --locked로 준비한다.
   임의로 lockfile을 갱신하거나 dependency 버전을 내리지 않는다.
   빌드는 CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_DEBUG=0,
   CARGO_PROFILE_TEST_DEBUG=0으로 시작한다. 이는 검증용이며 성능 측정 설정은 아니다.

3. 아래 변경 관련 검사만 실행한다:
   python3 research/production_review_20261022/run_targeted_checks.py \
     --expected-commit "$REVIEW_COMMIT" --out "$REVIEW_ROOT/receipts"
   실패하면 첫 원인을 분류하고 로그를 보존한다. 환경 실패를 수치 실패로 보고하지 않는다.
   새 결함을 설명할 최소 수정만 하고 관련 시험을 다시 실행한다.

4. DECISIONS.json, NEXT_STEPS.json, 외부 연구보고서를 읽어라.
   우선순위는 최신 NCP 원자료 공개와 전체 상태 오차 검증, affine norm의
   실제 수락 경로 연결, 화학 구조화 선형해법이다. C99 norm 파일을 가져왔다는
   사실만으로 ODE에 연결되었다고 말하지 마라.

5. NCP에서는 먼저 저장된 attempt3 원자료를 찾고 SHA256을 확인한다:
   ebd3b90fad13be50491073bf47acbded1ed0560fc39814c3e54995dfc772c965
   새 계산 없이 요청·전체 궤적·두 기준해·실패 stdout의 감사용 묶음을 복구한다.
   접속은 승인된 로컬 키를 사용한다:
   ssh -i /home/cosmosapjw/.ssh/ncp_key root@49.50.142.2 -p 22
   키 내용을 읽거나 출력하지 않는다. 다른 host로 복사하지 않는다.
   현재 disk/RAM/CPU를 실제 확인한다. 기존 캠페인과 동시에 무거운 작업을 실행하지 않는다.

6. 새 NCP 성능 캠페인은 NEXT_STEPS의 변경 구현이 준비된 뒤 별도 수치 소스를
   고정하여 실행한다. 비교는 VODE/DVODE, diffsol, Julia 계열로 제한한다.
   같은 전체 상태/관측량 정확도에서 setup 포함 solve와 process wall을 분리한다.
   64코어 독립문제 throughput을 한 trajectory의 parallel speedup으로 표현하지 않는다.
   기존 반복을 단순히 늘리기보다 censored tolerance와 timing-scope 차이를 먼저 해결한다.

7. 최종 답변에는 source commit/tree, 실행 host, 명령/exit, 시험 수, 원자료 경로,
   실제 수정, 남은 입력 의존성을 기록한다. 과거 NCP 시간은 새 코드 시간으로 쓰지 않는다.
```
