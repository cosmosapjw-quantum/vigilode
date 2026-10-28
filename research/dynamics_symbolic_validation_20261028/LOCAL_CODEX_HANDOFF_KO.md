# Local Codex handoff — VigilODE 누적 production qualification

이 문서를 로컬 Codex에 그대로 전달한다. 저장소는 `cosmosapjw-quantum/vigilode`이고,
이번 validation branch는 `validation/dynamics-symbolic-20261028-f9615d`다.
최종 응답의 commit URL에 있는 40자리 SHA를 `DELIVERY_SHA`로 고정한다.
역사적 실행 source `58ec9b053163da6b6f0906010e0422c104db72fd`를 문서가 추가된
최종 delivery SHA와 혼동하지 않는다.

## 목표와 승인 범위

현재 main과 PR90–97 및 이번 validation PR의 실제 상태를 조회하고 새 worktree에서
필요한 변경을 한 번만 통합하여 실제 최종 tree를 검증하라. 필요한 구현·수정·검증,
새 branch non-force push와 base=main draft PR은 승인되어 있다. 기존 PR/main을
reset하거나 force-push하지 말고, main merge/production 배포는 하지 않는다.

physmath-research-loop를 적용하고 남은 이론을 먼저, 코딩을 다음에 수행한다.
완료된 trace identity, cyclic contraction 및 snapshot ownership proof를 다시
처음부터 연구하지 않는다. 새 input/assumption/source 변경이 영향 주는 의무만
다시 열어라. 이론·코드·독립 검토를 subagent에 분담하며 native job은 한 lock으로
직렬화한다. 합법적인 prepare/observe word에 대한 controller refinement와 실제
ODE phase-space의 symbolic abstraction을 구별한다.

## 첫 실행

```bash
set -euo pipefail
DELIVERY_SHA=REPLACE_WITH_40_HEX_SHA_FROM_FINAL_COMMIT_URL
VIGILODE_REPO=/absolute/path/to/vigilode
cd "$VIGILODE_REPO"
git status --short
git fetch origin --prune
git cat-file -e "${DELIVERY_SHA}^{commit}"
VIGILODE_STAMP=$(date -u +%Y%m%dT%H%M%SZ)
VIGILODE_BRANCH="validation/local-dynamics-${VIGILODE_STAMP}"
VIGILODE_TREE="$(dirname "$VIGILODE_REPO")/vigilode-local-${VIGILODE_STAMP}"
git worktree add -b "$VIGILODE_BRANCH" "$VIGILODE_TREE" "$DELIVERY_SHA"
cd "$VIGILODE_TREE"
git log --oneline --left-right HEAD...origin/main
```

先행 PR90–94→96→97는 nested ancestry다. 이미 포함된 commit들을 다시 cherry-pick하지
않는다. PR95는 chemical Jacobian/remainder의 별도 조건부 입력이며 이 branch에
merge되지 않았다. 현재 main이 ancestor면 그대로 진행하고, 실제 독립 변경이
있으면 새 worktree에서 일반 merge하여 충돌과 물리 의미를 검토한다. 추가
선행 PR도 최신 metadata로 확인한다. worktree/branch가 이미 있으면 새 이름을 쓴다.

## 읽을 자료

이번 node `research/dynamics_symbolic_validation_20261028/`의 REPORT_KO.md,
RESULTS.json, PRESERVATION.json, WORKFLOW.json, MANIFEST.json,
THEORY_ADMISSION.md, theory/{DYNAMICS,ATTACHMENT}.md, reviews/CODE_REVIEW.md,
PREREGISTRATION.md 및 evidence/를 먼저 읽는다. 실행 stdout/stderr와 exact bits는
압축 원자료 안에 보존되어 있으며 manifest에 압축·원본 byte identity가 있다.

다음 기존 문서를 연결한다.

- `research/production_rvj_nag_20261028/{CURRENT_STATE.json,LOCAL_DEVELOPMENT_PLAN.json,LOCAL_CODEX_HANDOFF_KO.md}`
- `research/cumulative_validation_20261028/{SUMMARY.json,NEXT_HANDOFF_KO.md}`
- `research/integrated_polyalgorithm_20261028/{INDEPENDENT_VALIDATION.json,INDEPENDENT_PR_HANDOFF_KO.md}`
- 기존 `research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json`

원래23-node/34-edge DAG,8 suites,73 paths,10 interactions와 negative evidence는
그대로 유지한다. 이번 bounded PASS를 전체 interaction/N task/production PASS로
승격하지 않는다. `PRESERVATION.json`의 최신 correction counter와 동결된 옛
0/3 snapshot을 구별한다. R-STRUCT/R-ADAPT=3/3, R-HOMO/I-PROD=1/3,
Q-LEDGER=1/3,N03=2/3,N04=1/3,N05=2/3이다. 과제 이름을 바꿔 예산을
초기화하지 않는다. 추가 correction은 기존 controlling task에 기록한다.

## 로컬 환경과 source 게시

Rust1.94.1, 현재 Cargo.lock을 유지한다. cloud의 `/tmp`·`/dev/shm` 경로를 복사하지
말고 새 target과 evidence directory를 지정한다. 모든 source를 받는 full checkout을
권장한다. sparse checkout의 include_str fixture 누락은 INPUT_DEPENDENCY다.

```bash
rustup toolchain install 1.94.1 --profile minimal --component rustfmt --component clippy
cargo -V
rustc -Vv
export CARGO_TARGET_DIR="$(dirname "$VIGILODE_TREE")/target-${VIGILODE_STAMP}"
export CARGO_BUILD_JOBS=2
export RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
VIGILODE_EVIDENCE="$(dirname "$VIGILODE_TREE")/evidence-${VIGILODE_STAMP}"
flock /tmp/vigilode-cargo-20261027.lock cargo fetch --locked
# 필요한 소스/실행계약 변경을 commit한 뒤:
git push -u origin "$VIGILODE_BRANCH"
git fetch origin "$VIGILODE_BRANCH"
VIGILODE_EXEC_SHA=$(git rev-parse HEAD)
test "$VIGILODE_EXEC_SHA" = "$(git rev-parse "origin/$VIGILODE_BRANCH")"
```

새 과학 실행 전에 실제 실행 소스와 protocol을 원격 게시·readback한다. runner의
source pin에는 위 actual HEAD를 사용한다. 임의 remote-tracking ref 덮어쓰기,
원래 source 검사 삭제, Cargo.lock 갱신으로 문제를 숨기지 않는다.

## 새 bounded 검증 재사용 및 변경 영향 검증

새 CPU/profile/integration 변경으로 필요한 경우 아래 registered runner를 실행한다.
이미 동일 identity에서 완료된 검사를 안심 목적으로 반복하지 않는다.

```bash
python3 research/dynamics_symbolic_validation_20261028/code/run_scoped.py \
  --source-sha "$VIGILODE_EXEC_SHA" \
  --remote-ref "origin/$VIGILODE_BRANCH" \
  --output "$VIGILODE_EVIDENCE" --cargo cargo
```

runner는 lock을 내부 획득하므로 같은 outer flock으로 감싸지 않는다. `--only`는
coupled-lib,facade-lib,givens-exact,trace-support,switch-timing,coupled-release,
all-targets-scoped,docs-scoped 중 반복 지정할 수 있다. 명령별 timeout/실패/partial
출력을 보존하고 새 output directory로 재개한다. 필터0 tests, ignored/skip,
compile-only를 scientific PASS로 세지 않는다.

Trace timing은 같은 exact singleton observable 기준이다. 오래된 eager 경로가
huge input을 거절한 경우는 범위 개선으로 기록하며 speedup ratio에 포함하지
않는다. Controller benchmark는 synthetic work receipt를 쓴 prepare+observe
비교다. 실제 RHS/ODE solve, 전체 application의 speedup으로 표현하지 않는다.
실제 반환된 상태의 독립 물리 검증은 기존 physical/full-state suite와 함께 수행한다.

## 남은 production 의무의 우선순위

1. **Q-GIVENS:** RESULTS.json의 실제 current-profile test status와 Fraction oracle를
   먼저 읽는다. 새로운 profile에서 fixture A,b,preconditioner,solution bits를
   export하여 `tools/q_givens_exact_residual_oracle.py`로 모든 Ok를 검증한다.
   역사적 Ok/Err flag 차이와 원래 방정식 residual 실패를 구분한다. 기대값을
   단순 변경하지 말고 필요하면 source/profile-bound 역사 재현과 portable residual
   계약을 분리하는 최소 패치를 독립 검토한다. 기존 failed pin과 최초 bits를 보존한다.
2. **Q-API 및 8-suite full qualification:** 기존 COMMANDS.json/DEVELOPMENT_PLAN.json
   의 실제 argv로 integration tests, supported features, fat-LTO release, docs,
   smoke, downstream enum/struct compatibility를 검증한다. 이번 library/scoped
   check가 full workspace/모든 지원 조합을 대체하지 않는다.
3. **Q-LEDGER:** typed repository/object authority를 고친다. PR94 missing53,
   PR96 missing49, PR97 CI missing56, partial-clone178은 다른 source/checkout
   관측이다. 이번 full/partial observation도 별도 기록한다. 일괄 allowlist나
   historical evidence 변경으로 green을 만들지 않는다. checker/ledger 통과와
   전체 authority gate를 분리한다.
4. **물리·성능 interaction:** 10개 I-* gate에 실제 client/original equation,
   norm/clock/epoch/cache/session identity를 연결한다. mode와 kernel 비용,
   adapter/session, preparation/certificate, 실패·fallback 비용을 구분한다.
   같은 achieved accuracy/output checkpoints/observable/permission의 가장 강한
   eligible comparator와 paired total-cost/profile을 수행한다. 기존 negative
   15/7>1.3 및 inaccurate Off arm을 보존한다.
5. **외부·환경:** 독립 vendor/offline graph와 배포 CPU/FP 환경, SUNDIALS/MPI
   headers/compiler/link qualification을 완료한다. registry cache 성공은 vendor
   qualification이 아니다. 기존 unsupported/ignored 기능을 조용히 지원 처리하지 않는다.
6. **다음 실제 개발:** trace whole-path native certificate는 아직 외부 Fraction
   residual consumer에 의존한다. 이미 증명된39/64 contraction/two-envelope와
   original-field residual/domain/rounding을 연결하여 RHS-only native 경로를
   구현하고 독립 exact oracle로 검증한다. selected singular branch/F5 global
   count/generic quotient/full chemical model은 별도 열린 이론이다. physical ODE
   symbolic abstraction은 incremental stability와 quantization error 계약부터
   유도한다. 이번 controller refinement를 그 증명으로 재사용하지 않는다.

## 완료와 반환

각 gate의 PASS_SCOPED/REUSED_WITH_IDENTITY/FAIL/TIMEOUT/NOT_RUN을 범위와 함께
기록한다. original-source replay와 현재 누적 native 결과를 분리한다. root commit,
actual execution SHA/tree, compiler/binary/input hashes, raw outputs, 실패,
resource 및 correction ledger, 분석/figure 원자료를 보존한다. 정당한 새로운
production claim이 없다면 HOLD를 유지한다. 검증된 최종 tree를 새 branch에
non-force push하고 base=main draft PR, 보고서와 다음 인계점을 반환한다.
