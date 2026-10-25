# D5 CI 범위 — 고정 S2 workflow 규칙 판정

**지정된 변경에 매치하는 자동 PR workflow는 `research-process.yml` 1개다. D5 branch push에 매치하는 workflow는 0개다.**
과거 solver/test/benchmark/NCP 캠페인의 자동 재실행 경로는 이 변경 범위에서 매치하지 않는다.
이는 규칙의 정적 판정이며 실제 Actions 실행 결과나 CI PASS 판정은 아니다.

## 실제 읽은 범위

- Repo: `cosmosapjw-quantum/vigilode`.
- 고정 source commit: `3107693de084341129c47c592204225c9045aa2e`.
- [S2의 .github/workflows](https://github.com/cosmosapjw-quantum/vigilode/tree/3107693de084341129c47c592204225c9045aa2e/.github/workflows) 17개 파일을 connector GET으로 읽었고 directory/file의 Git blob identity가 일치했다.
- [Draft PR #88](https://github.com/cosmosapjw-quantum/vigilode/pull/88)의 실제 head는 `research/d5-callback-contract-20261025`, base는 `research/d4-effect-journal-20261025`, draft=true다.
- S2 PR files는 100/page 조회에서 95개, 다음 페이지 0개였다. 모두 `research/d5_callback_contract_20261025/` 아래이며 ledger는 이 S2 diff에 아직 없다.
- 판정 대상 추가 변경: `research/d5_callback_contract_20261025/**` 및 `research/LEDGER.jsonl` append. 각 workflow의 전체 event 규칙과 blob pin은 `evidence/CI_SCOPE.json`에 기록했다.

## 17개 규칙의 판정

| Workflow | 지정 변경의 자동 실행 | 실제 제외 또는 매치 조건 |
|---|---|---|
| research-process.yml | PR에서 매치 | `pull_request.paths: research/**`; draft 제외 조건 없음. push는 main 전용 |
| production.yml | 매치 없음 | PR은 crates/root Cargo/toolchain/workflow 경로; push는 main 전용 |
| ignored-tests.yml | 매치 없음 | PR은 crates/root Cargo/특정 v37 results/checker 경로; push는 main 전용 |
| a1-inner-tolerance-parity.yml | 매치 없음 | A1·crate·지정 문서 경로; push는 main 전용 |
| a1-two-arm-receipt.yml | 매치 없음 | A1·crate·지정 scope/receipt tool의 PR 경로 |
| a2-a3-gmres-inner-cycle.yml | 매치 없음 | Krylov/integrator 및 해당 workflow의 PR 경로 |
| audit2-research.yml | 매치 없음 | audit2·crate·fixture·지정 tool 경로; push는 main 전용 |
| audit2-stage-certificate-repair-handoff.yml | 매치 없음 | 지정 audit2 연구·source·test·tool의 PR 경로 |
| e4-fresh-clone-build.yml | 매치 없음 | root build/tool/계획 문서의 PR 경로; research/** 없음 |
| coupled-closeout-20261020.yml | 매치 없음 | coupled-matrix-free 연구 branch의 지정 closeout 경로 push |
| coupled-native-20261020.yml | 매치 없음 | coupled-matrix-free 연구 branch의 지정 run_ci/workflow push |
| coupled-economy-20261020.yml | 매치 없음 | coupled-economy 연구 branch의 지정 run_ci/workflow push |
| coupled-economy-closeout-20261020.yml | 매치 없음 | coupled-economy 연구 branch의 지정 closeout/workflow push |
| coupled-reference-recovery-20261020.yml | 매치 없음 | coupled-economy 연구 branch의 해당 workflow 변경 push |
| cumulative-private-transfer-20261020.yml | 매치 없음 | cumulative-evidence 연구 branch의 해당 workflow 변경 push |
| hybrid-structure-ad-20261020.yml | 매치 없음 | hybrid-structure-ad 연구 branch의 지정 run_ci/workflow push |
| rvj-audit-offline-source.yml | 매치 없음 | audit/rvj-native-followup-20261003 branch의 해당 workflow 변경 push |

이 17개의 event 규칙에는 `workflow_run`/`repository_dispatch` 연쇄 트리거가 없다.
선언된 diff 밖의 파일이나 workflow 자체를 바꾸면 이 판정 범위를 다시 적용해야 한다.

## 유일하게 매치하는 job의 내용

[research-process.yml](https://github.com/cosmosapjw-quantum/vigilode/blob/3107693de084341129c47c592204225c9045aa2e/.github/workflows/research-process.yml)은
contents:read, ubuntu-latest, timeout 10분, checkout fetch-depth:0, Python3.13을 사용한다.
명시된 작업은 아래 metadata 검사다. 아래 명령은 원문에서 읽었으며 이 조사에서 실행하지 않았다.

```text
python3 tools/test_check_research_node.py -v
python3 tools/test_check_authority_refs.py -v
python3 tools/check-research-node.py --base "origin/${BASE_REF}"
python3 tools/check-authority-refs.py
```

앞의 두 suite는 임시 Git 저장소에서 metadata checker를 시험한다.
뒤의 두 checker는 node/preregistration/numeric-result 존재, append-only ledger·저장 파일 해시,
게시된 authority commit의 도달성을 확인한다. ledger의 command 필드를 과학 명령으로 실행하지 않는다.
D5 Rust 35개 시험이나 과거 solver·벤치마크·NCP 캠페인의 실행 명령은 이 job에 없다.
모든 호출 대상 script를 같은 S2 ref로 읽고 subprocess/import 경로도 확인했다.

매치하는 workflow는 1개지만 exactly-once guard는 없다. Draft 상태에서도 조건을 만족하는 PR 갱신 이벤트마다
metadata job이 다시 생성될 수 있다. 이 조사에서는 workflow dispatch나 rerun을 요청하지 않았다.

## Authority checker의 실제 경계

[check-authority-refs.py](https://github.com/cosmosapjw-quantum/vigilode/blob/3107693de084341129c47c592204225c9045aa2e/tools/check-authority-refs.py)는
tracked research/docs의 md/json/jsonl/toml에서 authority 문맥의 40hex를 추출한다.
SHA256처럼 더 긴 hex 문자열은 후보에서 제외한다. 로컬 Git에서 blob/tree로 확인된 객체는 non-commit으로 건너뛰고,
commit은 remote branch 도달성으로 판정한다. 다른 repo의 source pin을 해당 repo에서 조회하는 기능은 없다.
따라서 외부 repo pin의 실제 검사 결과까지 이번 workflow 규칙 조사로 PASS라고 할 수 없다.
checker 또는 allowlist를 변경하지 않았다.

## 수행 기록

이번 작업은 read-only GitHub connector 25회와 이 두 metadata 문서 작성이다.
원격 mutation·dispatch·shell 명령·시험·과학 재검증·Desktop Commander 실행은 모두 0이다.
기존 D5 과학/R6 판정과 closed budget은 이 조사에서 다시 검증하지 않았다.
production=HOLD, global completion=null을 유지한다.
