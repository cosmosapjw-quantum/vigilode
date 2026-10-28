# 두 분기 브랜치의 실제 통합

이번 통합은 `33e45625`의 인증 API만 옮기는 데서 끝내지 않는다. production 두 분기의 통합 기반 `4a5fc656730fe8243f06b6e603cb8aa758b00b2f`와 연구 브랜치 최신 head `ee65d4dbecf3b3cd5f4587c27f87ef297d2bb406`를 대상으로, solver 모듈·benchmark 도구·연구 증거·ledger를 함께 보존하는 functional union을 구성했다. 최종 native gate, 독립 검토, 원격 merge commit 확인은 최상위 감사 절차에서 수행한다. 브랜치 통합 자체는 production·전역 정확도·속도 승격의 근거가 아니다.

## 정확한 차이와 반영 범위

공통 조상은 `59d95eb7404882f024eb8bd3856f2ce4f1f830c9`다. 공통 조상에서 연구 head까지 변경된 경로는 **1,133개**다. production 통합 기반과 비교하면 1,110개가 없었고, 같은 경로가 있던 23개는 내용이 달랐다.

이미 수행한 scoped solver port와 최신 연구 증거 반영 이후, 전체 분기 통합 직전의 working tree와 비교한 결과는 다음과 같다.

| 분류 | 경로 수 | 처리 |
|---|---:|---|
| 연구 head와 이미 동일 | 331 | 유지 |
| 현재 트리에 없는 순수 추가 | 788 | 연구 head의 blob을 그대로 복사하고 SHA-256 재확인 |
| 같은 경로에서 다른 내용 | 14 | 아래 명시적 호환성 결합 또는 보존 결정 |
| 연구 쪽 삭제 | 0 | 삭제 없음 |
| 연구 쪽 수정이나 production에서 제거된 경로 | 0 | 되살릴지 판단해야 하는 경로 없음 |

788개 추가의 총량은 **13,268,925 bytes**다. Rust example 2개, benchmark 도구·테스트 42개, `cloud_completion_20261018` 연구 자료 260개, `component_benchmark_20261018` 자료 484개가 포함된다. 복사 시점에는 전부 원래 blob과 동일했다. 이후 새 example의 rustfmt, registry 호환성 시험, HPC telemetry 교정처럼 실제 통합에 필요한 변경은 `BRANCH_INTEGRATION.json`의 `post_import_modified_paths`에 별도로 기록한다. 최종 트리가 788개 모두 과거와 동일하다고 주장하지 않는다.

`BRANCH_IMPORT_RECEIPT.json`은 복사 시점의 정확한 blob·hash·경로를 보존한다. `BRANCH_INTEGRATION.json`의 `safe_add_paths`는 요청된 machine-readable 후보 목록이며, 해당 추가 작업은 이미 수행했다. 최종 source identity는 root의 최종 manifest를 사용한다.

## 14개 같은 경로의 결합

| 경로/범위 | 결정과 이유 |
|---|---|
| `README.md` | 최신 통합 감사 링크를 유지한다. 기존 cloud 보고서는 원문 경로에 보존한다. |
| core `leja_action.rs`, 두 Leja/polynomial 시험 | 원래 의미를 유지하는 rustfmt 차이를 수용한다. |
| core `lib.rs` | production export와 연구 polynomial router export를 함께 유지한다. |
| integrator `bound_pr101.rs` | 현재 frozen `BoundPr101Problem` 파일을 그대로 유지한다. 연구의 parameterized `Pr101Model`은 별도 `pr101_verified.rs`와 `Pr101VerifiedError`로 분리한다. |
| `dense_output_v2.rs` | production W-filter estimate 경로와 새 `VerifiedResidual` 소유권 경로를 함께 둔다. 연구의 outward Horner 평가와 현재 오류 처리 교정을 결합한다. |
| `generic_verified.rs` | parameterized PR101 provider가 별도의 오류 형식을 사용하도록 연결한다. |
| integrator `lib.rs` | production의 band/cache/lagged/resolvent API와 연구 API를 함께 export한다. |
| `packet_curve_output.rs` | 기존 weighted center displacement와 새 carryable weighted radius를 동일한 outward 계산으로 제공한다. |
| `all_dag_a01_packet.rs` | 기존 oracle을 유지하고 두 반경 API 사이의 삼각부등식 계약을 검사한다. |
| `production_audit_bound_collective.rs` | libtest 잡음과 측정 대상 thread를 구분하는 production allocation 시험 교정을 유지한다. |
| `production_audit_packet_curve_output.rs` | production의 실제 추가 연산 `3n+1`을 반영한 계수 56/88을 유지한다. |
| `research/LEDGER.jsonl` | 아래 원문 archive와 commit-qualified ID를 사용한다. 충돌하는 과거 행을 덮어쓰거나 같은 ID로 단순 연결하지 않는다. |

세부 경로별 원래 blob과 현재 hash, 해결 종류는 JSON에 14행으로 기록했다. packet 삼각부등식, quartic 계수 대응, PR101 소유권 분리의 증명과 검증은 `evidence/port/PORT.md`에 있다. 모든 연구 경로는 opt-in으로 유지되며 public facade에 자동 선택 규칙을 새로 붙이지 않았다.

## Ledger: 두 원본을 모두 읽을 수 있게 보존

두 브랜치 ledger는 각각 142행이다. 같은 ID의 135행은 JSON 의미가 같지만, **L-0136부터 L-0142까지 7개 ID는 서로 다른 결과를 가리킨다**. 숫자가 같다는 이유로 같은 연구 결과로 취급할 수 없다.

두 원본을 다음 위치에 byte-for-byte 보존했다.

- `research/branch_integration_20261021/production_LEDGER.jsonl`
- `research/branch_integration_20261021/all_dag_LEDGER.jsonl`
- `research/branch_integration_20261021/LEDGER_INDEX.json`

production archive는 published wave3 head `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`의 ledger와 byte-identical하다. 따라서 clone 후에도 직접 조회할 수 있도록 production 쪽 qualified ID는 local-only `4a5fc65` 대신 이 published head를 사용한다.

index의 284개 기록은 `cosmosapjw-quantum/vigilode@<full commit SHA>:<original ID>`를 식별자로 사용한다. 원문 행 번호, raw-line hash, 정규화된 JSON hash, 전체 archive hash도 포함한다. 충돌 7개는 두 기록을 그대로 유지하며 어느 쪽이 다른 쪽을 폐기한다는 결론을 자동으로 만들지 않는다. 과거의 unqualified `supersedes`는 반드시 같은 archive/commit namespace 안에서만 해석한다. production 16개와 연구 14개 edge를 각각 qualified ID로 연결했으며, 숫자가 같은 다른 분기 결과로 연결하지 않는다. root는 현재 ledger에 새 통합 항목을 추가할 수 있지만, 과거의 ID·claim·receipt를 수정하지 않는다.

## 실행과 의미의 통합

과거 연구나 측정 campaign을 다시 돌려 기존 결론을 재포장하지 않았다. 이전 source-bound receipt는 그 source에 대한 증거로 남는다. 새 통합 source에 대한 시험은 변경된 연결·계약만 검증한다.

scoped solver port에서는 native 85개와 compile-fail doctest 2개가 통과했다. 추가 호환성 시험에서 발생한 shared-artifact 빌드 실패는 삭제하지 않았으며, root의 새 target 최종 matrix가 그 미충족 의무를 판정한다. 이 숫자를 새 solver benchmark의 속도 근거로 사용하지 않는다.

component 도구를 합친 직후 Python unittest 발견 실행은 53개를 실행하여 실패 4개, import error 5개, skip 1개를 남겼다. 이 실패는 다음 통합 작업을 구체적으로 결정했다.

1. `pytest`와 `mpmath`가 없는 실행 환경을 확인하고, repo 밖의 재현 가능한 runtime 경로에 고정 버전 `pytest==8.4.2`, `mpmath==1.3.0`을 설치했다.
2. 과거 registry 246개를 현재 254개와 무조건 같다고 비교하던 시험을 교정했다. 과거 map은 바꾸지 않고, 기존 ID의 제거가 없고 production/호환성 모듈 8개가 정확히 추가되었음을 검사한다. 94개 algorithm variant는 그대로다.
3. 매우 짧은 자식 process에서 RSS sample을 얻지 못했을 때 `0 bytes`를 실측으로 기록하던 문제를 benchmark 검토자가 교정했다. 측정 불능은 `null`과 `UNAVAILABLE` status/source로 표현하며, 임의의 양수나 실측 0으로 바꾸지 않는다. RSS가 없으면 P3는 `HOLD_UNKNOWN`을 유지한다.

이 legacy component registry는 원래의 수치 crate 5개 범위를 유지한다. source 파일 목록에는 test 전용 모듈도 포함되므로 254를 production algorithm 수로 해석하지 않는다. 새 facade와 `vigilode-bench`는 본 감사의 별도 노드에서 검토한다.

환경 복원과 호환성 교정 후 최종 component pytest 결과는 **90 passed, 3 skipped in 21.35s** (exit 0)다. skip 3개는 frozen Hairer library 부재를 명시한다. pytest-only campaign 시험에서 추가로 발견한 과거 246개 고정 assertion도 정확한 8개 모듈 union과 원래 78개 case를 유지하도록 교정했으며, 첫 실패 로그를 보존했다. focused HPC 계약 28개도 통과했다. 이는 계약 시험이며 과거 scientific campaign의 재실행이 아니다. 전체 로그와 hash는 JSON execution 항목에 보존했다. native example 빌드 결과는 root 최종 검증 기록을 기준으로 한다. 새 example은 다음 명령으로 main worktree에서 검증한다.

```bash
cargo check --locked --offline -j 1 -p rodas5p-integrators --example component_benchmark_native
```

## 원격 commit은 실제 두 계보를 포함해야 한다

파일을 복사한 commit만으로 두 branch를 merge했다고 부르지 않는다. 원격에 게시할 commit은 검토한 union tree를 가지면서 다음 세 head가 모두 조상이 되도록 구성한다.

- production plan: `cc2e1a009ac6802ef01a289392e9715074a882ba`
- production wave3: `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`
- latest research: `ee65d4dbecf3b3cd5f4587c27f87ef297d2bb406`

`4a5fc65`가 이미 두 production head를 부모로 갖기 때문에, 그 commit 또는 후손과 `ee65d4db`를 부모로 하는 merge commit이면 된다. `4a5fc65`가 로컬에만 있다면 세 실제 remote head를 부모로 하는 동일 union tree의 merge commit도 가능하다. 게시 후 `git merge-base --is-ancestor`로 세 계보를 각각 확인하고 원격 tree, 최종 source manifest, ledger archive hash를 검증한다.

완료 선언의 조건은 이 원격 계보 확인과 최종 통합 gate다. 역사 자료의 누락 없이 소스 결합을 준비한 상태와, 실행·정확도·성능을 입증한 상태를 별도로 기록한다.

## Later authorized NCP implementation phase

The branch-union receipts above describe the first import and its 254-row registry/90-pass component snapshot. They are preserved as historical checks. The later implementation adds `global_budget` and `stage_target_policy`, making the current legacy registry 256 rows (historical 246 + eight imported-production modules + two new modules). Both registry/campaign contracts enumerate this exact ten-module difference. The earlier agent-local Python snapshot is 139 passed / 3 skipped, including 27 new NCP harness contracts; the final combined count is in `EXECUTION.json`; see `NCP_IMPLEMENTATION.json` and `evidence/ncp_harness/python_contracts_final.log`. No earlier published campaign was replayed. Final native validation and publication status are the root aggregate report's responsibility. Earlier per-file observed hashes remain time-of-check snapshots; the final `SOURCE_MANIFEST.json` binds the completed source.

The final uniform-source matrix (`evidence/final_04/EXECUTION.json`) passed all19 commands: 298 native tests, 4 doctests and 142 Python tests; 5 native measurement/kill-gate tests and 3 unavailable Hairer Python cases were excluded as ignored/skipped. Source before/after remained identical across890 bound files. The initial disk-exhaustion and subsequent Clippy failures are retained in final02/final03. This is targeted integration validation; full native benchmark execution and controlled-host performance promotion remain separate obligations.
