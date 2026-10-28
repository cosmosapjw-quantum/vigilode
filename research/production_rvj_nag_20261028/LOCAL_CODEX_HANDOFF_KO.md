# Local Codex handoff — PR90–96 및 native RVJ 변경의 독립 누적 검증 PR

아래 내용을 local Codex에 그대로 전달한다. 저장소는 `cosmosapjw-quantum/vigilode`이다. 기존 결과를 다시 주장하기 위한 반복 실행이 아니라, 실제 local 배포 환경에서 남은 누적 검증·구현 의무를 마무리하는 작업이다. 현재 revision의 보고서·machine-readable plan·실행 receipt를 먼저 읽고, 이미 유효한 증거는 source identity와 범위를 확인하여 재사용한다.

---

## 1. 수행할 작업과 완료 범위

최신 main과 PR90–96, 그리고 `research/production-rvj-nag-20261028` branch의 변경을 확인하여 **별도 worktree와 독립 validation PR**에서 누적 검증하라. 먼저 남은 이론적 의무와 구현 의무를 구별하고, 필요한 이론 판단 후 구현을 진행하라. `$physmath-research-loop`가 있으면 현재 source-bound 연구 계약과 함께 사용하고, 독립 subagent를 이론·원래 방정식 검증·코드/API/프로파일링에 배정하라. Native build/test/benchmark/profile은 공통 lock 아래 직렬화한다.

새 RVJ 구현은 opt-in module 두 개를 추가한 bounded component이다. 기본 dispatcher, 기본 solver 선택, 기존 provider 계약을 변경하지 않았다. 다음 사실을 작업 시작 시 유지하라.

- Cyclic closure는 지정된 n=4,8,16 모델의 원래 방정식과 선택된 작은 실근 branch에 관한 결과이다.
- Trace는 **모든 복소근을 대수적 중복도로 합산한 polynomial observable**이다. selected real branch나 real-only observable로 바꾸어 비교하지 않는다.
- 새 component의 PASS를 전체 N03/N05, 여섯 N 과제, 열 interaction gate 또는 broad production PASS로 승격하지 않는다.
- 임의의 safety multiplier를 만들지 않는다. `1/8`, `8/7`, `6/7`, `3/4`, `25/32`, `39/64` 등은 원래 증명·norm·domain에 연결한다. 반복 수·timeout·thread 수는 accuracy 상수가 아닌 자원 정책으로 별도 기록한다.
- Jacobian/matrix/Newton-free 경로가 기본 연구 방향이다. supplied JVP/AD를 도입할 때는 동일 물리 target에서 derivative-free 방식이 불리한 실제 근거와 명시적 opt-in 계약을 먼저 마련한다.

최종 결과는 구현된 route와 실제 client별 supported/unsupported 조합, 누적 검증 결과, 측정된 total cost, 남은 이론 의무, 갱신된 machine-readable DAG와 local 개발 문서를 포함해야 한다. main에 직접 merge하거나 reset/force-push하지 않는다. 새 validation branch에 non-force push하고 review 가능한 PR을 만든다.

## 2. 고정 source와 이미 수행한 결과

### 2.1 반드시 구별할 source identity

| 역할 | 고정 값 |
|---|---|
| 이번 native 실행 source | `64004e23042f2cf076da5378be724f50c3856f4c` |
| 이번 native 실행 tree | `67564b455011252fdc1fb811c09baeea9f2bd31f` |
| RVJ delivery branch | `research/production-rvj-nag-20261028` |
| RVJ PR의 현재 base branch | `validation/cumulative-polyalgorithm` |
| PR96 source | `13f3298bd0213f121e07385903f9508c435ad6a1` |
| PR94 delivery source | `e935878fefe795a1b7e88aba64e5e30ef7be1578` |
| PR96 기록상 native 실행 source | `5c57bf146f2f5faf69f711f133a83fcb265cee90` |
| 별도로 읽은 PR95 | `36104bfc65d1fdc695c8acea383c54b50a1c4e78` — **merge하지 않음** |

PR90–93의 원래 full SHA와 original-source replay 제한은 `research/integrated_polyalgorithm_20261028/INDEPENDENT_VALIDATION.json::source_contract`에 있다. 이전에 관측한 main SHA를 지금의 main이라고 가정하지 말고 fetch 후 실제 값을 기록하라. 새 RVJ PR 번호는 branch로 조회하며 추정하지 않는다. 보고서 추가 delivery commit과 실제 실험 source commit은 다를 수 있다. 실행 후 runtime이 바뀌지 않았다는 주장은 diff와 manifest hash로 확인한다.

### 2.2 현재 실제 실행 결과

등록 source `64004e23042f2cf076da5378be724f50c3856f4c`가 원격에 게시되고 readback된 후 다음 cohort를 수행했다.

- Rust 1.94.1, `measurement` profile, locked/offline dependency 상태에서 cyclic native tests **6개**, trace native tests **9개**가 통과했다.
- 두 native campaign example이 성공했고 raw endpoint bits, coefficient intervals, timing samples, work counters를 보존했다.
- 독립 Fraction oracle은 **732,947개 check**, cyclic **720개 서로 다른 slab / 2,160개 arm-slab 기록 / 383,040개 coefficient interval**, trace **992개 slab**을 모두 확인했다. 대표 slab만 검사한 결과가 아니다.
- Cyclic의 첫 passing **tested** grid는 세 차원 모두 historical certificate N=64, direct certificate 두 방식 N=32였다. N=16 실패와 historical N=32 실패는 보존했다. 이것은 모든 가능한 정수 grid의 최적성을 증명한 결과가 아니다.
- Trace 두 방식은 N=256에서 처음 공통 whole-path cap을 통과했으며, 외부 residual certificate 상한은 약 `5.316525230098761e-9`였다. N=16,32,64,128의 실패를 삭제하지 않는다.
- Trace의 whole-path certificate는 현재 **외부 exact Fraction 구현**이다. Native module 자체의 interval 인증은 point/parameter-box observable 및 field 범위이다. Trace native integration-only 시간을 완전한 인증 trajectory 시간으로 부르지 않는다.

최신 사후 집계 `analysis/BENCHMARK_ANALYSIS.{md,json}`도 반드시 읽는다. Cyclic의 같은 `direct_bernstein_tight` first-passing arm은 기존 대비 8.7206/9.3338/9.1984배의 native whole-call 중앙값 개선을 보였다. 반면 trace N=128 궤적 16개 block의 7-sample 중앙값은 direct **607.695 μs**, shared **667.394 μs**, direct/shared **0.91055**였다. 반복 범위가 겹치며 shared가 빠른 반복은 4/7회이므로 안정적인 개선이나 보편적 regression을 확정하지 않는다. Field-only 개선과 구별한다. Route별 외부 whole-path 인증 timing과 native root-enumeration baseline이 없어 전체 workflow speedup은 미측정이다. N=64·128 residual-gate 미통과는 실제 오차 위반의 증명이 아니며 exact-reference whole-path bound는 N=64부터 cap 이하이다. 공식 gate를 사후 변경하지 않는다.

원래 cloud run directory는 `/tmp/vigilode-rvj-registered-01`이었다. Local에서 그 절대 경로가 존재할 것이라고 가정하지 않는다. Delivery의 `research/production_rvj_nag_20261028/evidence/`에 보존된 receipt, raw JSON의 압축본, manifest를 사용하고 **압축 전 원본 byte hash**와도 대조하라. 핵심 원본 hash는 다음과 같다.

| 원본 파일 | SHA256 |
|---|---|
| `closure.json` | `54efb925d608083d29dc219636899be8b7120cd12bf01146407fc5f6cd464d3d` |
| `trace.json` | `d331510f849d985119142daed7ff87ac6351a8f3c283a06321bb4687bb6451c6` |
| `ORACLE_RESULTS.json` | `19bf376383b816cb6189271839f4b9fd77e74d1c4fb868f0e4aad307243b0381` |
| 실행 oracle `code/verify_native.py` | `8c58f348d00300210d3983eb8e9ebd52c1ae30a2d20ad7db1e731d374c9974d6` |

등록 runner의 `RUN_RECEIPT.json`이 compiler, platform, argv, source hashes, binary hashes, stdout/stderr hashes와 actual timestamps의 기준이다. Native binary hash는 플랫폼별 결과이며 다른 local binary와 같아야 한다는 조건이 아니다. 새로운 local 결과는 새로운 source/binary/environment에 연결한다.

PR96 compact evidence는 감사 당시 24/24 manifest hash가 일치했고 선택된 source hash 5개도 일치했다. 그러나 PR96가 예전 `/tmp`에만 보존한 일부 원본 로그·rapid raw data·Givens oracle receipt는 이번 workspace에서 회수되지 않았다. 요약과 hash를 읽었다고 원본을 재실행하거나 확보했다고 쓰지 않는다. 이번 RVJ raw evidence와 오래된 회수 불가 evidence의 상태를 구분한다.

## 3. 먼저 읽을 문서와 현재 수정 예산

다음 순서로 읽되 이미 읽은 동일 byte 자료를 다시 전부 검토하는 루프를 만들지 않는다.

1. `research/production_rvj_nag_20261028/LOCAL_DEVELOPMENT_PLAN.json`, `RESUME.md`, `SOURCE_PUBLICATION.json` 및 최종 report/results/manifest, `analysis/BENCHMARK_ANALYSIS.{md,json}`.
2. `THEORY_ADMISSION.md`, `theory/INTEGRATION_SCOPE.md`, `theory/CLOSURE.md`, `theory/TRACE.md`와 각 contract JSON.
3. `theory/PR96_AUDIT.md`, `theory/PRODUCTION_GATES.json`, `reviews/CODE_REVIEW.md`, `reviews/CODE_REVIEW.json`.
4. `code/ORACLE_CONTRACT.md`, `code/run_registered.py`, `code/verify_native.py`, 두 implementation 문서와 실제 Rust source.
5. `research/cumulative_validation_20261028/{SUMMARY.json,REPORT_KO.md,REPRODUCE.md,EVIDENCE_MANIFEST.json}`.
6. `research/integrated_polyalgorithm_20261028/INDEPENDENT_VALIDATION.json`, `EXECUTION_CONTRACT.md`, `algebra_contract/THEORY.md`, `error_contract/THEORY.md`, `validation_design/{AUDIT.md,COMMANDS.json}`.
7. 원래 23-node/34-edge 계획인 `research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json`, 그리고 eight-suite 실행 계획인 `research/production_solver_closure_20261027/DEVELOPMENT_PLAN.json`.

**현재 누적 correction budget**은 다음과 같다. 과제 이름을 바꾸어 새 3회를 부여하지 않는다.

| 기존 과제 | 사용 / 최대 | 현재 의미 |
|---|---:|---|
| R-STRUCT | 3/3 | 소진; 새 이름으로 재시작하지 않음 |
| R-ADAPT | 3/3 | 소진; 새 이름으로 재시작하지 않음 |
| R-HOMO | 1/3 | 기존 counter 유지 |
| I-PROD | 1/3 | 기존 counter 유지 |
| Q-LEDGER | 1/3 | PR96에서 거절된 authority repair 포함 |
| PR-N03 | 1/3 | 이번 C9 prepublication coherent repair episode 포함 |
| PR-N05 | 1/3 | 같은 C9 episode와 bounded trace repair 포함 |
| PR-N01, N02, N04, N06 | 각각 0/3 | 전체 과제는 pending |
| 나머지 Q counters | 기존 각 0/3 | scoped PASS와 correction 사용량은 별도 |

원래 frozen plan의 역사적 `0/3`과 최신 누적 ledger를 혼동하지 않는다. 초기 구현, dependency/toolchain 준비, 실제 과학적 실패에 따른 수정, 같은 coherent episode의 targeted readback을 분리하여 기록한다. 새 실패가 발생하면 원본 receipt를 보존하고 controlling task를 정한 뒤 남은 예산만 사용한다. 한 번의 독립 심사 후 해당 finding의 제한된 readback은 새 full audit로 세지 않는다.

## 4. Local branch/worktree와 통합 절차

먼저 현재 checkout의 사용자 변경을 확인하고 보존하라. 아래는 새 worktree를 위한 예시이며, 같은 블록을 무작정 재실행하여 branch/worktree를 중복 생성하지 않는다.

```bash
git status --short
git fetch origin --prune
gh pr list --repo cosmosapjw-quantum/vigilode \
  --state all --head research/production-rvj-nag-20261028 \
  --json number,url,headRefOid,baseRefName,mergedAt

RVJ_EXEC_SOURCE=64004e23042f2cf076da5378be724f50c3856f4c
RVJ_REPO=$(git rev-parse --show-toplevel)
RVJ_MAIN=$(git rev-parse origin/main)
RVJ_DELIVERY=$(git rev-parse origin/research/production-rvj-nag-20261028)
RVJ_STAMP=$(date -u +%Y%m%dT%H%M%SZ)
RVJ_BRANCH="validation/rvj-cumulative-${RVJ_STAMP}"
RVJ_WORKTREE="$(dirname "$RVJ_REPO")/vigilode-rvj-local-${RVJ_STAMP}"
RVJ_LOG_ROOT="$(dirname "$RVJ_REPO")/vigilode-rvj-evidence-${RVJ_STAMP}"

git cat-file -e "${RVJ_EXEC_SOURCE}^{commit}"
git log --left-right --cherry-pick --oneline "$RVJ_MAIN...$RVJ_DELIVERY"
git diff --stat "$RVJ_MAIN...$RVJ_DELIVERY"
git worktree add -b "$RVJ_BRANCH" "$RVJ_WORKTREE" "$RVJ_MAIN"
cd "$RVJ_WORKTREE"
```

Fresh worktree는 **fetch한 main에서 시작**한다. 이미 main에 들어간 ancestor나 equivalent squash를 확인한 뒤, 아직 필요한 RVJ delivery branch를 이 validation branch에 **한 번만** 통합하라. PR90–96를 순서대로 다시 cherry-pick하지 않는다. 기록된 PR96 ancestry와 현재 main의 patch equivalence를 확인하고, 필요한 경우 `git merge --no-ff --no-commit "$RVJ_DELIVERY"`로 한 번 통합하여 충돌을 실제 코드 계약에 따라 해결하고 commit한다. 이미 통합되어 있으면 이를 생략한다. 충돌 해결 자체가 수치 코드를 바꾸면 새로운 실행 source에 포함하고 해당 위험만 검증한다. main branch에 merge하지 않는다.

검증 harness/preregistration 수정이 필요한 경우 먼저 완성하고 source commit에 포함한다. 원래 역사적 보고서나 raw evidence를 고쳐 일치시키지 않는다. 새로운 연구 node는 `research/local_rvj_validation_<UTC-id>/`처럼 구분하고 original DAG와의 mapping을 기록한다.

Source publication 전에는 source reading·환경 준비·format·compile 준비를 할 수 있다. **새 과학적 결과로 주장할 tests/benchmark/oracle를 실행하기 전**, 실제 현재 source와 protocol을 commit하여 validation branch에 non-force push하고 readback한다.

```bash
# 새 source/protocol 변경을 검토하여 명시적인 파일들만 commit한 뒤 실행한다.
git push -u origin "$RVJ_BRANCH"
git fetch origin "$RVJ_BRANCH"
RVJ_LOCAL_SOURCE=$(git rev-parse HEAD)
test "$RVJ_LOCAL_SOURCE" = "$(git rev-parse "origin/$RVJ_BRANCH")"
git status --short
```

`64004e...`는 역사적 실행 pin이다. 현재 HEAD가 보고서 추가나 main 통합으로 달라졌다면 runner에 이 pin을 거짓으로 전달하지 않는다. **현재 실제 published HEAD**를 `--source-sha`로, **현재 실제 validation remote branch**를 `--remote-ref`로 전달한다. Remote-tracking ref를 수동으로 덮어 검사를 우회하거나 source check를 제거하지 않는다. Source가 실행 중 바뀌면 해당 receipt는 stale/FAIL로 처리한다.

## 5. Toolchain·lock·증거 기록 준비

Local에서는 저장소의 `rust-toolchain.toml`을 따른 정상적인 Cargo/Rustup 환경을 사용한다. 기존 runner는 POSIX `fcntl`/process-group 동작을 사용하므로 Windows에서는 WSL 환경 또는 먼저 게시·검증한 동등 runner가 필요하다. WSL 결과를 native Windows qualification으로 바꾸어 쓰지 않는다. 필요한 경우 설치 명령은 다음과 같다.

```bash
rustup toolchain install 1.94.1 --profile minimal --component rustfmt --component clippy
cargo -V
rustc -Vv
python3 --version

RVJ_LOCK=/tmp/vigilode-cargo-20261027.lock
mkdir -p "$RVJ_LOG_ROOT"
export CARGO_TARGET_DIR="$RVJ_LOG_ROOT/target"
export CARGO_BUILD_JOBS=2
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1

flock "$RVJ_LOCK" cargo fetch --locked
```

Cloud receipt의 `/tmp/vigilode-rvj-rust/bin/cargo`, `RUSTC` 또는 Python 절대 경로를 local에 복사하지 않는다. 현재 `RUSTC`, `RUSTFLAGS`, target CPU flags, linker, allocator, `RUSTDOCFLAGS`, FTZ/DAZ 상태 등 실제 환경을 기록하고, 이전 cloud override가 남아 있으면 해당 task shell에서 정상 toolchain으로 정리하라. 단순히 compiler 버전 문자열만 맞추었다고 arithmetic/compiler-transformation 가정을 입증했다고 쓰지 않는다.

Dependency를 먼저 `--locked`로 확보한 후 scientific commands는 `--locked --offline`으로 실행한다. Cargo.lock을 최신 버전으로 갱신하여 환경 문제를 덮지 않는다. 외부 comparator의 SUNDIALS/MPI 버전이 다르면 명시적인 experiment factor로 기록하고 qualification한다.

`run_registered.py`와 `run_local.py`는 **자체적으로 flock을 획득한다**. 같은 lock의 outer `flock`으로 이 둘을 감싸면 deadlock이 생길 수 있으므로 감싸지 않는다. 개별 Cargo 명령만 공통 lock과 기존 `record_command.py`로 기록한다. 예를 들면:

```bash
rvj_record_native() {
  local rvj_label="$1"
  shift
  flock "$RVJ_LOCK" python3 research/production_solver_closure_20261027/record_command.py \
    --output "$RVJ_LOG_ROOT/$rvj_label" --timeout 3600 \
    --input research/production_rvj_nag_20261028/LOCAL_DEVELOPMENT_PLAN.json \
    --input research/production_rvj_nag_20261028/code/verify_native.py \
    -- "$@"
}
```

`3600`은 local 자원 예시이며 mathematical constant가 아니다. 실제 각 command cap을 실행 전에 protocol에 고정한다. Timeout 후 기존 directory를 재사용하거나 partial pass 수와 연장 실행 pass 수를 합산하지 않는다. 새로운 output directory에 extension/repair receipt를 만들고 관계를 연결한다. Source hash, command argv, stdout/stderr, exit/timeout, reached tests, ignored tests, binary/profile/CPU identity를 함께 보존한다.

## 6. LOCAL-01 — 새 bounded 결과의 source 확인과 필요한 local 검증

우선 저장된 raw/receipt를 hash 검증하고 scientific outcomes를 재사용한다. 단지 안심하려고 같은 cloud source/profile cohort를 반복하지 않는다. 새로운 local CPU/compiler/release profile이나 main 통합의 영향 때문에 실제 local replay가 필요한지 명시하고, 필요한 경우 한 번 실행한다.

```bash
python3 research/production_rvj_nag_20261028/code/run_registered.py \
  --source-sha "$RVJ_LOCAL_SOURCE" \
  --remote-ref "origin/$RVJ_BRANCH" \
  --output "$RVJ_LOG_ROOT/registered-01" \
  --cargo cargo --lock "$RVJ_LOCK"
```

현재 registered runner는 **measurement profile 전용**이다. `--profile release` 인자가 있다고 가정하지 않는다. Actual deployment의 fat-LTO release는 별도로 아래 명령처럼 준비하고 같은 original-equation oracle를 적용하라. 이때 command와 source/protocol을 먼저 게시한다.

```bash
rvj_record_native rvj-release-build cargo build --release --locked --offline \
  -p vigilode-coupled --example rvj_closure_campaign --example rvj_trace_campaign
```

Release binary는 `$CARGO_TARGET_DIR/release/examples/rvj_closure_campaign` 및 `rvj_trace_campaign`이다. 각 binary를 기존 recorder로 한 번 실행하여 `stdout.log`를 JSON input으로 보존하고 SHA256를 기록한다. 외부 oracle는 다음처럼 새 output 파일에 적용한다.

```bash
rvj_record_native rvj-release-closure "$CARGO_TARGET_DIR/release/examples/rvj_closure_campaign"
rvj_record_native rvj-release-trace "$CARGO_TARGET_DIR/release/examples/rvj_trace_campaign"

rvj_record_native rvj-release-oracle python3 research/production_rvj_nag_20261028/code/verify_native.py \
  --closure "$RVJ_LOG_ROOT/rvj-release-closure/stdout.log" \
  --trace "$RVJ_LOG_ROOT/rvj-release-trace/stdout.log" \
  --output "$RVJ_LOG_ROOT/rvj-release-oracle/ORACLE_RESULTS.json"
```

이 exact-oracle command도 같은 recorder와 공통 lock으로 기록되어 다른 timing workload와 겹치지 않는다. 실제 배포 profile 검증이므로 measurement 재현과 목적이 다르다. 결과가 다르면 처음 달라지는 original coefficient/residual/clock/branch fixture를 좁혀 분석하며 tolerance·domain·negative expectation을 완화하지 않는다.

## 7. LOCAL-02 — PR90–96 누적 API·native·interaction 검증

### 7.1 Eight suites를 원래 scope와 연결

기존 suite 이름은 다음 여덟 개이다. 모두 수행한 것처럼 단일 `PASS`를 쓰지 말고 `REUSED_WITH_IDENTITY`, `PASS_SCOPED`, `FAIL`, `TIMEOUT`, `NOT_RUN`, `NOT_APPLICABLE_WITH_REASON`를 각 scope에 기록한다.

| Suite | Local에서 확인할 목적 |
|---|---|
| `facade-contracts` | 기존 facade의 반환·관측·실패·state/work 계약 |
| `coupled-contracts` | coupled causal/structural/homotopy 계약과 새 opt-in module |
| `all-targets` | 기존 client/example/feature target의 build 호환성 |
| `release-contracts` | 실제 release/LTO에서 원래 assertions, reached/ignored 분류 |
| `docs` | 공개 API 문서와 warning policy |
| `smoke-release` | 실제 physical output 및 original oracle의 작은 배포 smoke |
| `local-benchmarks` | 같은 achieved error·output 계약의 전체 비용 |
| `local-profiles` | adapter, 개별 mode, certificate의 비용 원인 분리 |

`LOCAL_DEVELOPMENT_PLAN.json`은 DAG 파일이고 `execution_suites` 구조의 runner plan이 아니다. Old runner에는 실제 실행 계획을 전달한다. 먼저 dry-run으로 현재 argv와 적용 범위를 확인하라.

```bash
python3 research/production_solver_closure_20261027/run_local.py \
  --plan research/production_solver_closure_20261027/DEVELOPMENT_PLAN.json \
  --suite release-contracts --output "$RVJ_LOG_ROOT/release-contracts-01" --timeout 3600

# 계획한 범위가 맞고 source가 게시되었으면 같은 명령에 --execute를 추가한다.
```

다른 suite도 같은 방식으로 실행한다. 모든 suite를 무조건 반복하는 shell loop를 만들지 않는다. 이미 보유한 동일-source 증거를 재사용할 부분과 실제 local environment에서 새로 필요한 부분을 먼저 coverage table에 고정한다. PR96 integrators의 최초917 pass와 연장1196 pass는 중복되는 실행이다. 최종 extension의54 ignored, 별도 첫 release command의21 ignored, 기존 두 coupled formatting 실패를 숨기지 않는다. Ignore 이유·지원 기능과의 관계를 확인하고 필요한 ignored subset만 명시적으로 실행한다.

### 7.2 Exact Givens, FAIR-AB, API, authority

일반 `cargo test` 성공만으로 Givens Fraction oracle 실행을 주장하지 않는다. 실제 사용 profile에서 정확 oracle를 한 번 실행할 때는 두 환경 변수가 필요하다.

```bash
export Q_GIVENS_ORACLE_DIR="$RVJ_LOG_ROOT/q-givens-release-01/exports"
export Q_GIVENS_ORACLE_SCRIPT="$PWD/tools/q_givens_exact_residual_oracle.py"
rvj_record_native q-givens-release-01 cargo test --release --locked --offline \
  -p rodas5p-krylov --test givens_production_differential_contracts \
  -- --nocapture --test-threads=1
unset Q_GIVENS_ORACLE_DIR Q_GIVENS_ORACLE_SCRIPT

rvj_record_native fair-ab-release-01 cargo test --release --locked --offline \
  -p rodas5p-fair-ab --lib --tests -- --test-threads=1
```

Givens fixture/results bit exports와 exact oracle receipt를 모두 보존하라. 기존 output directory를 재사용하면 create-only 계약에 걸리므로 새 이름을 사용한다. FAIR-AB는 attempted A, admitted E, refused R, completed C, failed F에 대해 A=E+R, E=C+F, E≤budget를 독립 callback 관측으로 확인한다. 이것만으로 모든 threaded/rollback I-WORK가 끝났다고 하지 않는다.

Q-API는 **실제 downstream crate**로 검증한다. 기존 public `SwitchingConfig` 등의 exhaustive struct literal/enum match 소비자, 기본 production 설정, 새 `vigilode_coupled::{cyclic_closure,trace_quotient}` opt-in 사용을 분리하라. Main/기존 지원 source와 후보 source에 같은 downstream fixture를 적용하고 public compatibility와 semantic default를 확인한다. Workspace all-targets 성공은 외부 exhaustive consumer의 증거를 대신하지 않는다. 새 module 때문에 기존 default dispatcher를 변경하지 않는다.

Q-LEDGER는 full-history checkout에서 `python3 tools/check-authority-refs.py`의 실제 결과를 기록하고 typed repository/object ownership을 고친다. PR96의49 missing과 PR94 CI의53 missing은 각각의 역사적 source 값이며 최신 main의 결과로 복사하지 않는다. External URL을 이 저장소 path로 collapse하거나 malformed record/특정 실패 줄을 전역 무시하는 repair는 이미 거절되었다. Bulk allowlist와 역사적 증거 재작성으로 green을 만들지 않는다.

Vendoring을 배포 계약에 포함한다면 실제 vendor 경로를 제공하여 다음을 실행한다. `vendor_dir` 없이 invocation한 exit2는 validation이 아니다.

```bash
python3 tools/validate-cargo-vendor.py "$RVJ_VENDOR_DIR" \
  --json-out "$RVJ_LOG_ROOT/vendor-validation-01.json"
```

`RVJ_VENDOR_DIR`은 실제 Cargo source-replacement 설정에서 확인한 경로이다. Vendor를 사용하지 않는 배포라면 근거와 지원 범위를 기록하고, registry locked/offline build를 vendor 검증이라고 부르지 않는다.

### 7.3 열 interaction gate는 각자 직접 receipt가 필요하다

| Gate | 실제 확인할 내용 |
|---|---|
| I-STATE | winning complete session을 이어 실행한 결과와 uninterrupted session 일치; rejection·중간 observation 포함 |
| I-EPOCH | step/chart/norm/model/provider/arithmetic epoch 변화에서 cache 무효화; 동일 key에서만 재사용 |
| I-WORK | 원래 target refusal, 진단 residual, fatal callback, rollback, joined worker의 모든 작업량 |
| I-BUDGET | continuous/discrete anchor, physical norm, edge 중복·누락, internal/observation clock, 같은 continuation state의 label |
| I-POLY | fused same/distinct-vector outputs, degree/rounding boundary, quotient defect, denominator/refinement floor와 fallback |
| I-ROOT | 동일 backend에서 old/new planner 비교; zero/linear/cubic, branch ambiguity, representation/update/clock floor |
| I-STRUCTURE | exact/near quotient relation을 원래 full-space stage residual에 대조; domain tube와 observer/effect compatibility |
| I-NEGATIVE | nonnormal spectrum trap, two-root ambiguity, variable-step instability, 기존 W-filtered 실패와 진짜 lower impossibility 구별 |
| I-PROFILE | test/measurement/actual release-LTO의 floating·branch-sensitive fixture와 executable/compiler/libm identity |
| I-CLIENT | 모든 반환 physical state/observation, 독립 oracle, strongest eligible persistent/structural/CG/coupled comparator와 전체 비용 |

새 cyclic endpoint/epoch tests는 I-EPOCH의 일부이고 전체 gate를 대체하지 않는다. Trace semantic rejection도 selected-root homotopy I-ROOT 전체를 대체하지 않는다. 등록 조건을 구현하지 못한 gate는 NOT_RUN 또는 명시적 unsupported로 남긴다.

Known negative인 W-filtered 비용15/7>1.3과 Off interior error311.467을 보존한다. 기존 ignored negative test를 검증한다면 원래 exact test 이름과 실패 기대를 사용하고, 일반 suite가 green이 되도록 삭제·weaken하지 않는다. 이전 cubic matched driver의 missing initial output, conflicting duplicate rows, Decimal precision-context 문제도 사용 전에 해결해야 한다.

역사적 `verify.py` 일부는 특정 PR source와 runtime unchanged 조건을 검사하고 protected 결과 JSON을 덮어쓴다. 새로운 통합 tree에서 해당 source-locked 검사를 억지로 통과시키지 않는다. 필요할 때만 원래 full SHA의 disposable worktree에서 실행하고, 기존 byte를 보존한 새 evidence로 결과를 비교한다. `INDEPENDENT_VALIDATION.json::historical_command_overrides`를 따른다.

## 8. Adapter·mode·certificate 성능 과제의 분리

LOCAL-04에서는 다음 세 비용을 분리해서 측정한다. API work counter의 단위와 실제 wall time/CPU instruction은 별개이다.

| 비용 축 | 포함할 실제 비용 | 과도한 결론을 막는 비교 조건 |
|---|---|---|
| Adapter/switcher | probe, discarded challenger, checkpoint/restore, session restart, observation dispatch, cache lookup·invalidation | 같은 initial state, complete continuation state, output schedule, physical norm·error cap |
| 개별 mode | candidate RHS, scalar/rational closure, Krylov/QR/MGS, polynomial action, causal preparation, root fallback | 같은 original equations·branch·observable와 달성 오차, strongest eligible 구조적 comparator |
| Certificate | endpoint preparation, original residual, Bernstein enclosure, domain/clock checks, radius transfer, error ledger | candidate-only와 인증 포함 total을 구별; shared 인증 최적화는 가능한 모든 candidate에 공정 적용 |

먼저 같은 endpoint path에서 direct/historical certificate 차이를 확인하고, 그 다음 **같은 physical cap의 첫 passing tested grid**에서 full trajectory cost를 비교한다. 같은 N에서만 빠른지, 줄어든 N 때문에 빨라진지, 준비·인증·mode 비용 중 무엇이 줄었는지를 나눈다. Phase medians의 합을 total median이라고 쓰지 않는다.

Trace shared/direct 비교는 동일한 all-root observable이다. Selected real-branch root solver와 trace sum을 서로 대체하여 speedup을 만들지 않는다. Newton/Krawczyk/homotopy fallback 비교를 새로 추가하면 branch identity, original residual, domain, failure work, physical whole-path cap과 error planner를 맞춰야 한다. Root algorithm 변경과 step planner 변경은 교차 비교해 효과를 분리한다.

PR96 rapid의 observation16→32 Callgrind 차이는 accepted steps/RHS도 달라져 순수 selector overhead가 아니었다. 새로운 profile은 고정된 동일 작업을 사용하거나 변경된 work를 명시적으로 분해한다. 측정 순서를 회전/교차하고 모든 raw sample·spread·실패를 보존한다. Host load, warm/cold state, pool startup/join, allocation/cleanup, calibration 및 실패한 탐색 비용을 기록한다.

초기 phase timers와 work counters로 병목이 충분히 특정되면 추가 profiler campaign을 하지 않는다. `perf`/Callgrind가 필요한 경우 실제 deployment binary와 반복 단위를 고정하여 한 번의 제한된 profile을 등록한다. Sequential nonlinear RK stage dependency를 근거 없이 병렬화하지 않는다. 독립 slab residual 구성, batch/state, 증명된 scalar recurrence composition의 병렬화 가능성과 thread overhead를 별도로 조사하라.

## 9. PR 목적과 DAG — 원래 계획을 축소하지 않음

Machine-readable 기준은 `research/production_rvj_nag_20261028/LOCAL_DEVELOPMENT_PLAN.json`이다. 그 파일은 원래 **23 nodes / 34 dependency edges**를 유지하고 `RVJ-CLOSURE`, `RVJ-TRACE` 두 bounded leaf를 별도로 연결한다. 새 leaf가 parent task를 자동 완료하지 않는다. Metadata의 역사적 상태와 최신 결과 override를 구별하고 새 local receipt를 해당 task/gate에 연결하라.

| Local PR/단계 | 목적 | 선행 조건 | 완료 evidence |
|---|---|---|---|
| LOCAL-01 | Source/byte/raw evidence 확인과 필요한 local native 재현 | 현재 source·branch·환경 식별 | source/binary manifest, 원래 방정식 oracle, negative outcomes |
| LOCAL-02 | PR90–96 + 새 leaf의 누적 native/API/interaction 검증 | LOCAL-01 | suite별 실제 receipt, downstream API, 필요한 열 interaction 결과 |
| LOCAL-03 | 실제 model provider와 physical continuation/session 연결 | LOCAL-02 | model/norm/clock/branch/error identity, original residual, rollback work |
| LOCAL-04 | 실제 hardware의 matched-error adapter/mode/certificate 비용 | LOCAL-02 | 같은 output·accuracy의 full trajectories, phase profiles, adverse cases |
| LOCAL-05 | 광고할 client/platform 조합의 production release setup | LOCAL-03 + LOCAL-04 | 필수 Q/API/authority/negative/지원 feature gate, 실제 client 및 package/release 문서 |

N 과제의 원래 목적도 유지하라.

| 과제 | 후속 PR에서 해결할 구체적 목적 |
|---|---|
| N01 | Fused phi/shifted solves와 Chebyshev/Laguerre/Leja/Krylov의 target-specific 선택; nonnormal original residual과 준비·reduction·인증 비용 포함 |
| N02 | Cubic homotopy의 올바른 finite-step/discrete stability·decay·clock budget으로 step 수 줄이기; root backend 효과와 분리 |
| N03 | Structural/raw-U 또는 bounded closure certificate를 physical adaptive consumer에 연결; endpoint·dense output·stage observer 계약 구별 |
| N04 | Winning complete session의 실제 소유권·FSAL/controller/workspace·관측 일정·global budget을 보존하여 restart 제거 |
| N05 | Finite causal/quotient scalar preparation과 vector action/work-span 분리; near/nonlinear remainder와 실제 병렬 비용 검증 |
| N06 | Nonlinear reentrant 손실을 continuation-state-aware selector로 다루기; 과거 비용이 미래 선택을 보장하려면 필요한 variation 조건 증명 |

문제별 이론/읽기 작업은 병렬로 맡겨도 되지만 공유 native 측정은 직렬화한다. Coordinator는 source·계약·gate mapping을 유지하고, 이론 agent는 필요한 정리와 exact constants, 구현 agent는 작은 변경, 독립 검증 agent는 원래 방정식과 adversarial controls를 담당한다. 이미 통과한 동일 증거를 다시 심사하는 무한 루프를 만들지 않는다.

## 10. 추가 이론이 필요한 부분과 구현 최적화의 경계

다음은 작은 kernel 최적화로 해결했다고 주장할 수 없는 별도 이론 과제이다.

- **Selected singular branch:** F1의 u/v charts는 같은 determinant coverage를 가진다. 다른 근의 collision을 통과한 regular selected branch 결과를 selected d=0 continuation/uniqueness 증명으로 확대하지 않는다.
- **Global NAG/F5:**116/58은 complete intrinsic count로 유효하지 않다. 조건부120/60과 전체 D=0 stratification은 atlas/exclusion/stabilizer/local-length 완결 증거가 필요하다.
- **Generic quotient provider:** multivariate finite-free basis, dimension constancy, rational observable denominator가 quotient에서 unit인 조건, 원래 physical target 복원과 certificate가 필요하다. 현재 finite quartic API는 generic compiler가 아니다.
- **PR95 실제 chemical provider:** 읽은 식 F=b+Pz+(eᵀz)Qz와 frozen-Q remainder `(eᵀd)Qd`는 constructive leaf이다.36 scalar entry extrema/72 witnesses를 whole-model Jacobian, actual rate/clock enclosure나 동시에 달성되는 matrix bound로 바꾸지 않는다. 실제 H/geometry/time, omitted blocks, callback rounding, native norm과 source binding이 더 필요하다. PR95는 이번 branch에 merge하지 않았다.
- **General polynomial routing/stiffness:** eigenvalue interval만으로 nonnormal operator를 Chebyshev/Laguerre에 넣지 않는다. Target-specific polynomial/quotient approximation, denominator, full trajectory truncation, residual transfer와 switching stability가 필요하다. Scalar stability 또는 node error만으로 generic stiff production solver라 부르지 않는다.

반대로 fixed-degree Bernstein arithmetic, endpoint reuse, target-specific trace invariant, workspace allocation, ownership/session retention은 기존 증명의 가정을 유지하면서 native 최적화로 개선할 수 있는 부분이다. 새로운 theorem이 필요 없는 변경에 불필요한 전면 이론 campaign을 붙이지 않는다.

## 11. Independent PR의 최종 산출물과 종료 조건

새 local node에는 최소한 다음 정보를 남겨라.

- 실제 source SHA/tree, main/delivery ancestry, source publication readback, toolchain/profile/CPU/libm/environment, binary hashes.
- 각 command의 raw stdout/stderr, actual exit/timeout와 source-change 검사, 압축 evidence의 원본·압축본 hash manifest.
- Eight suites, ten interactions, Q gates, N01–06와 bounded leaves의 분리된 상태 및 수행/재사용/미수행 이유.
- Original physical errors·domain/branch/clock validity와 phase/total cost를 결합한 결과. 가능한 경우 같은 cap에서 work–precision/phase-cost 그림을 만들고 figure data를 보존한다.
- 실패·coarse-grid refusal·ignored case·noncompetitive case와 수정 전 receipt. 기존 negative를 지우지 않는다.
- 갱신된 23-node/34-edge inheritance와 새 leaf/PR dependency, controlling task별 누적 budget, 실제 지원 조합의 production setup.
- 수학적 결과와 구현/측정 결과를 구별한 설명, REVTeX 보고서 갱신, 다음 local handoff.

PR 본문은 필요한 문제, 변경한 동작, 검증된 범위, 실제 total 성능, 남은 gate를 먼저 설명한다. Tool exit0만으로 scientific PASS를 만들지 않는다. 지원한다고 광고할 route에 필요한 의무가 모두 충족되어야 그 route만 qualification할 수 있다. 선택적 연구 N 과제를 모두 구현하지 않았다는 이유만으로 이미 증명·검증한 opt-in component를 폐기할 필요는 없지만, pending route가 active라고 주장해서는 안 된다.

실제 위험이 해소되고 등록 gate가 충분히 검증되면 추가 reassurance testing을 멈추고 결과·한계·다음 PR 목적을 정리하여 non-force push하라. Broad production HOLD가 남으면 정확한 blocker와 완료된 supported scope를 함께 보고하라. 이미 권한이 주어진 가역적 조사·수정·독립 PR 준비를 반복 승인 질문으로 중단하지 말고, 불가피한 접근 제한이 있으면 실제 거절된 action과 이유를 설명하라.

---

이 handoff의 실행 예시는 다음 local 검증을 위한 명령이다. 이 파일을 작성하면서 native campaign 또는 exact oracle를 다시 실행하지 않았으며, 위 PASS 수치는 고정 source의 기존 `RUN_RECEIPT.json`과 `ORACLE_RESULTS.json`을 읽어 인계한 것이다.

### 최종 metadata 검사 결과

연구 node/append-only ledger 검사는 PASS였다. 전체 authority 검사는 partial-clone의 자동 object 조회 중 120초 timeout을 먼저 기록했다. 같은 checker를 수정 없이 `GIT_NO_LAZY_FETCH=1`로 실행한 cached-object 감사는 FAIL이며, 813개 distinct SHA 중 missing 178개, allowlisted missing 27개, non-commit 186개, reachable 422개를 기록했다(실패 citation 919개). 이는 더 완전한 이전 checkout에서의 missing49와 직접 비교할 수 없다. 현재 missing에는 아직 보유하지 않은 blob도 포함될 수 있다. 이번 node의 실패 citation 3개는 byte-preserved PR95 입력의 외부 BASS commit/blob을 저장소 구별 없이 VigilODE authority로 해석한 항목이다. 원본·allowlist·checker를 고쳐 성공으로 꾸미지 않았다. Typed repository/object authority 정리가 local Q-LEDGER의 실제 후속 의무다. `FINAL_CHECKS.json`과 `evidence/metadata/`에 모든 완료 출력과 timeout 경위를 보존했다. 전체 whitespace diff의 두 경고는 원본 native stdout의 마지막 빈 줄이며, 이를 제외한 source/documentation 검사에는 오류가 없다.

Local에서는 정상적인 필요한 Git history/object 확보 후 authority를 다시 판정하라. 외부 repository commit/blob과 VigilODE commit의 type을 증거로 구별하며 blanket allowlist를 추가하지 않는다. 현재 cached-object counts를 이전 missing49와 같은 환경의 regression count로 사용하지 않는다.
