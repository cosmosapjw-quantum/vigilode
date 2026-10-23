# Codex 후속 실행용 프롬프트

아래 블록 전체를 전달한다. NCP는 사용하지 않는다. 이미 게시된 증거는 재사용하고 변경된 입력·코드만 다시 검증한다.

```text
저장소: https://github.com/cosmosapjw-quantum/vigilode
검토 브랜치: research/production-hardening-20261023
실제 검증 소스: 5913fe062706c820e768b9e150e5cd1c274f7bf9
실제 검증 tree: 7a70caf85569dbe6c77ecb9a53fe666cd40ae618

목표:
새 dense observation 및 affine physical error norm 구현의 로컬 재현과
실제 client 적용 준비를 수행한다. NCP 접속·계정·자원은 사용하지 않는다.
원래 checkout을 reset/checkout으로 바꾸지 말고 별도 detached worktree를 만든다.
최신 리뷰 branch의 research/production_hardening_20261023/REPORT_KO.md,
DECISIONS.json, NEXT_STEPS.json, REPRODUCE.md를 먼저 읽는다.

원칙:
1. final 보고서 commit과 실행 소스 commit을 구분한다. 원격 fetch 후 위 고정 source를
   별도 worktree에 두고 HEAD/tree/clean을 확인한다. 임의 SHA로 결과를 표시하지 않는다.
2. Rust1.94.1과 Cargo.lock을 유지한다. 부족한 cache는 locked fetch 또는 검증된 vendor로
   준비하며, 의존성을 downgrade하거나 lock을 수정하지 않는다. 환경 실패 receipt는 보존한다.
3. REPRODUCE.md의 validation과 measurement build를 실행한다. 기존67Rust+7Python
   결과를 이미 신뢰할 수 있고 코드가 동일하면 불필요한 반복 실행을 하지 않는다.
4. 새 환경의 성능을 측정할 때 run_bench.py 기본72설정/288회, budget840초를 사용한다.
   실패·timeout·NOT_RUN을 숨기지 않는다. 실제 전체 상태 오차를 Python으로 재채점한다.
   CPU/RAM을 과도하게 쓰지 말고 build jobs2를 사용한다.
5. 원래 landing API를 유지한다. solve_dense_observed는 관측 표본만 반환한다.
   options.dense_output는 false로 둔다. retained interpolant/events/mass 지원으로
   해석하지 않는다. affine norm의 atol/rtol이 실제 수락 기준이며 반환 상태는 reduced다.
6. 우선 다음 개발은 실제 full33/reduced30 client norm 연결과 보존식/RHS 일관성 시험이다.
   이어서 큰벡터 메모리 프로파일, 장시간 진동의 phase/amplitude, bounded opt-in
   explicit/implicit switching을 진행한다. NEXT_STEPS의 기존 미완료 작업도 보존한다.
7. 무조건 broad production 승격을 하지 않는다. 범위별 promote/hold/
   optimization_possible/specialized_only/prune 판단과 그 근거를 각각 기록한다.
   동일 tolerance의 속도만으로 우위를 판단하지 말고 같은 달성 오차 상한을 사용한다.
8. 코드 수정이 필요하면 원래 raw 실패를 보존하고 최소 수정 후 관련 테스트만 재실행한다.
   새로운 속도 측정은 새 source commit을 고정한 뒤 실시한다. 전체 테스트 재반복이나
   오래된 이론의 재증명을 자동으로 시작하지 않는다.

완료 보고:
source/tree, 환경, 실제 명령과 exit code, 바뀐 파일, 테스트 개수,
전체 상태 재채점, 동일 오차 시간비와 불리한 사례, 아직 남은 production 제약,
결과 디렉터리와 후속 최소 작업을 제공한다. 게시 권한은 현재 사용자 지시를 따른다.
```
