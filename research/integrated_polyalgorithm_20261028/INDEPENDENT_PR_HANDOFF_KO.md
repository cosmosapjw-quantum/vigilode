# Local Codex handoff — PR90–93와 통합 연구 전체를 검증하는 독립 PR

아래 본문을 local Codex의 작업 지시로 전달하라. `DELIVERY_SHA`에는 이 파일을
전달한 최종 응답의 **전체 40자리 commit SHA**를 넣는다. 이 문서에 등장하는
PR93 SHA나 검산 사전등록 SHA를 최종 전달 SHA로 대신하지 않는다.

## 1. 목표와 이미 제공된 권한

VigilODE의 이 대화에서 만든 PR90–93 및 그 위의 통합 연구를 한 누적 소스로
검증하는 **새 독립 validation PR**을 완성하라. 저장소는
`cosmosapjw-quantum/vigilode`다. 새로운 branch는 전달된 통합 tip에서 시작하고
PR의 base는 `main`으로 한다. 선행 PR이 merge되기를 기다리지 않는다.

기존 PR들은 중첩된 ancestry이므로 네 head를 다시 cherry-pick하지 않는다.
단순히 commit이 ancestor라는 사실만으로 구현이 보존됐다고 결론내리지 말고,
최종 tree의 변경 경로·실제 동작·물리 오차·실패 작업량을 함께 검증한다.

사용자는 검증·필요한 수정·새 branch push·draft PR 및 handoff 갱신을 이미
요청했다. 정상적인 새 worktree 생성, read-only 조사, 범위 내 구현·수정 및
검증마다 다시 허락을 묻지 않는다. 기존 checkout을 reset하거나 기존 PR을
force push하지 않는다. `main` merge 및 실제 production 배포는 이 작업에
포함하지 않는다.

이번 통합 연구의 완료 범위는 physmath-research-loop 1.3.0의 R3/R5/R9,
지원 exact 검산 및 후속 C4/C10 실행 설계다. 이 결과를 검토해 이어갈 native
코딩·과학 검증 단계를 선택한다. 완료된 이론 연구를 처음부터 반복하거나
검토자의 검토자를 만드는 재귀 심사를 시작하지 않는다. 구체적인 구현 결함이나
새로운 반례가 발견되면 해당 범위의 최소 수정을 수행한다.

## 2. 기준 소스와 새 작업 공간

조사 시점의 ancestry는 다음과 같다. 실행 시점에는 원격 상태를 다시 읽는다.

| 역할 | 고정 commit |
|---|---|
| 당시 관측한 main | `8d0c79184e09efb5bdadc24a6315c60a71a44264` |
| 이 대화의 구현 이전 baseline | `3064d453091af1dec7f99f0c8afabd98b447ea34` |
| PR90 | `68a28357d960c4c2743c299770f0a33ef63245b2` |
| PR91 | `206298b4068d923bb47c7160b111b7439098660c` |
| PR92 | `7eb2dd0ffda39afb2871d76700a720986ff09fe0` |
| PR93 | `384dba091a3d9e1aac0ecddec264958298239253` |
| 이번 통합 exact 검산의 사전등록 소스 | `af7f3e9f55d652554042d2386bf07abb7d857d75` |
| 실제 독립 PR의 시작점 | **최종 전달된 `DELIVERY_SHA`** |

마지막 행은 최종 manifest와 handoff를 포함한 실제 전달 commit이다. 움직이는
branch HEAD를 pin으로 바꾸거나, 아직 최종 manifest가 없는 사전등록 commit을
helper에 전달하지 않는다.

현재 checkout이 이전 PR에 머물러 helper 파일이 없어도 기존 작업을 건드릴
필요가 없다. 전달 commit에서 helper 하나만 추출하고 저장소 root에서 실행한다.

```bash
set -euo pipefail
DELIVERY_SHA=REPLACE_WITH_EXACT_40_CHARACTER_SHA_FROM_FINAL_DELIVERY
VIGILODE_REPO=/absolute/path/to/vigilode
cd "$VIGILODE_REPO"
[[ "$DELIVERY_SHA" =~ ^[0-9a-f]{40}$ ]]
git status --short
git fetch origin \
  refs/heads/main:refs/remotes/origin/main \
  refs/heads/research/integrated-polyalgorithm-20261028:refs/remotes/origin/research/integrated-polyalgorithm-20261028
git cat-file -e "${DELIVERY_SHA}^{commit}"
VIGILODE_PREPARE_DIR=$(mktemp -d /tmp/vigilode-validation-prepare.XXXXXXXX)
git show "${DELIVERY_SHA}:research/integrated_polyalgorithm_20261028/prepare_validation.py" \
  > "$VIGILODE_PREPARE_DIR/prepare_validation.py"
python3 "$VIGILODE_PREPARE_DIR/prepare_validation.py" \
  --source-sha "$DELIVERY_SHA" \
  --destination ../vigilode-independent \
  --branch validation/cumulative-polyalgorithm
```

Helper의 기본 동작은 dry-run이다. 출력의 source pin·ancestry·destination·manifest
확인을 검토한 뒤, 이상이 없으면 같은 명령에 `--create`를 붙여 새 worktree를
생성한다. 이 검토는 추가 사용자 승인 요청이 아니다.

```bash
python3 "$VIGILODE_PREPARE_DIR/prepare_validation.py" \
  --source-sha "$DELIVERY_SHA" \
  --destination ../vigilode-independent \
  --branch validation/cumulative-polyalgorithm \
  --create
cd ../vigilode-independent
test "$(git rev-parse HEAD)" = "$DELIVERY_SHA"
```

이미 존재하는 destination이나 branch가 있으면 삭제·reset하지 말고 새로운
이름을 정해 기록한다. Helper는 소스 준비와 구조 검사를 지원한다. 그 성공이
native scientific validation의 PASS를 뜻하지 않는다.

새 `main`과 PR90–93 및 추가 PR의 상태를 확인하고 원격 metadata를 보존한다.
`main`이 전달 소스의 ancestor이면 그 관계를 기록한다. `main`에 독립 변경이
추가되었다면 정확한 새 main SHA와 merge base를 기록하고 **새 validation
worktree 안에서 정상 merge**하여 reconcile한다. Conflict를 해결한 내용과
의미를 기록하고 그 최종 tree에서 다시 검증한다. 변경된 선행 PR head는 기존
pin을 덮어쓰지 말고 별도의 integration input으로 검토한다.

최종 PR은 main에 비해 이 대화 이전의 inherited research도 포함할 수 있다.
다음 세 범위를 보고서에서 구분한다.

1. **Source inclusion:** PR90–93와 전달 통합 commit이 최종 head의 ancestor인가.
2. **Conversation coverage:** 이전 baseline→PR93의 73개 고유 crate/workflow
   변경 경로와 이번 통합 추가 내용의 의미가 최종 code/test에 반영됐는가.
3. **Inherited/release qualification:** 전체 baseline suite, dependency·feature·
   API·환경 및 기존 Q-* 게이트가 해당 배포 범위에서 통과했는가.

경로 개수나 ancestry 검사는 coverage bookkeeping이며 과학적 검증을 대체하지
않는다. 이후 코드가 이전 변경을 supersede하거나 revert할 수 있으므로 원래
불변량과 요구 동작을 최종 구현에서 확인한다.

## 3. 먼저 읽고 재사용할 자료

통합 node `research/integrated_polyalgorithm_20261028/`에서 다음을 읽는다.

- `RESUME.md`, `EXECUTION_CONTRACT.md`, `PR_SCOPE.json`, `SOURCES.json`.
- `REPORT_KO.md`, `INDEPENDENT_VALIDATION.json`, `DEVELOPMENT_DAG.md`.
- `error_contract/THEORY.md`, `CONTRACTS.json`, `CHECK.py`.
- `algebra_contract/THEORY.md`, `CONTRACTS.json`, `CHECK.py`.
- `evidence/ERROR_RESULTS.json`, `ERROR_RECEIPT.json`,
  `ALGEBRA_RESULTS.json`, `ALGEBRA_RECEIPT.json`.
- `validation_design/AUDIT.md`, `COVERAGE.json`, `COMMANDS.json`,
  `CI_OBSERVATIONS.json`, `READ_ONLY_CHECKS.json`, `REVIEW.md`.

선행 node에서 다음을 이어받는다.

- `research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json`,
  `DEVELOPMENT_DAG.md`, `REPORT_KO.md`, 세 THEORY 및 17개 candidate.
- `research/production_solver_closure_20261027/post_push/STATUS_KO.md`,
  `LOCAL_DEVELOPMENT_PLAN.json`, seven Q-* 진단과 `POLICY_REVIEW.md`.
- PR92의 `PRODUCTION_SETUP.md`, `DEVELOPMENT_PLAN.json`, `run_local.py`,
  `SOURCE_MANIFEST.json`, `evidence/VALIDATION_SUMMARY.json`와 실제 receipts.
- PR90/91의 `REPRODUCE.md`, 이론, 원자료 manifest 및 보존된 음성 대조군.

기존 23-node/34-edge DAG와 8개 실행 suite는 그대로 보존한다. 이번 coverage는
그 위에 놓인 검사 매핑이다. 기존 노드의 이름을 바꾸거나 17개 후보를 17개의
새 예산으로 바꾸지 않는다. Machine-readable plan에서 dependency를 따라
`L-RELEASE → L-HARDWARE → R-QUALIFY`의 실제 local qualification을 수행한다.

## 4. 연구 원칙과 통합 설계의 핵심 계약

기본은 RHS-only/derivative-free다. JVP/AD는 fully derivative-free 경로가 불리한
명시적 비교에서 허용된 selective provider이며, 허용 여부와 비용을 기록한다.
Matrix-free와 derivative-free는 별개다. 일반 Jacobian을 숨은 finite difference로
구성하거나 Newton/dense solve를 자동 fallback으로 넣어 원래 조건을 바꾸지 않는다.
기존 일반 causal 경로에 frozen JVP와 작은 projected solve가 남아 있다는 사실도
그대로 보고한다.

공통 구조는 다음과 같이 구현·검증한다.

1. 같은 original model, physical checkpoint, clock, chart/norm, output request를
   고정한다. `PhiAction`, `ShiftedSolve`, `OriginalStageRoot`, `PhysicalStep`의
   결과와 인증을 서로 바꿔 사용하지 않는다.
2. 정확한 구조/quotient, fused polynomial, causal/homotopy, 기존 Krylov의 각
   적합 조건을 확인한다. 비용 비교는 정확도·출력 자격을 충족한 후보들 사이에서
   한다. 실패한 준비와 검증 비용도 사라지지 않는다.
3. `error_contract`의 두 induction anchor를 보존한다. Exact-flow stability에는
   numerical-input temporal defect가 필요하다. Exact-trajectory truncation에는
   해당 discrete-map stability를 사용한다. PR93 cubic rational decay는 후자와
   midpoint nonexpansiveness를 사용한다.
4. Algebra compiler의 original raw-U residual은 같은 numerical input에서의
   discrete endpoint error로 변환한다. Temporal/physical-flow error는 별도의
   증명이다. Omitted/virtual stages의 residual과 near-affine remainder가 빠지면
   endpoint-only shortcut을 승인하지 않는다.
5. 이미 포함된 mean/RHS/lift rounding을 중복해서 계산하지 않으면서, 실제로
   미포함인 endpoint update, norm/chart conversion과 clock error를 빠짐없이
   연결한다. Internal-clock error와 output-label displacement를 구분한다.
6. 임의의 accuracy safety multiplier 대신 증명된 상수, outward bound와
   정수 degree/grid 판정을 사용한다. Thread/degree/timeout 제한은 명시적
   resource policy이며, hardware service cost는 측정값이다.
7. Finite-menu Pareto budget 선택은 고정 request/tube와 명시된 cost model에서
   검증한다. Cache/session이 다른 partial state를 비용만 보고 지우지 않는다.
   Toy work 7 대 10, KKT allocation, (5m\to m), (512\to256)을 실제 elapsed
   speedup으로 보고하지 않는다.
8. Upper certificate floor와 실제 representability lower bound를 구분한다.
   현재 enclosure가 예산에 맞지 않는다는 이유만으로 수학적 불가능성을 주장하지
   않는다. 실패한 동일 표현에서 iteration을 무한히 늘리거나 허용 오차를 넓히지 않는다.

## 5. 실행 환경과 native baseline

먼저 source/tree, Rust1.94.1, Cargo.lock, CPU, OS, libm, threading, compiler flags,
measurement/release profile과 dependency source를 기록한다. 기존 local baseline과
candidate는 **서로 다른 새로운 `CARGO_TARGET_DIR`**를 사용한다. 이전 PR90에서
공유 target의 stale local-package artifact가 발견됐으므로 오래된 binary를 새
소스 결과로 사용하지 않는다.

기존 plan의 환경 계약은 `CARGO_BUILD_JOBS=2`, `RAYON_NUM_THREADS=1`,
`OMP_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, `RUSTDOCFLAGS=-D warnings`다.
변경이 필요하면 환경 요인으로 명시한다. 기존 lockfile을 유지한 fresh default
graph와 vendor/offline graph를 각각 확인한다. 다른 프로젝트의 vendor bundle을
이 저장소의 locked dependency graph로 간주하지 않는다.

새 구현이나 gate 수정을 하기 전에 제공된 누적 source에서 native baseline을
실행하고, 예상된 Q 실패와 새 regression을 구분한다. `COMMANDS.json`의 argv를
source of truth로 사용해 focused targets와 whole-workspace/default-feature,
research-feature, formatter/Clippy, docs 및 실제 release 검증을 수행한다.
필터에 걸린 test가 0개면 PASS가 아니다. Compile 성공은 test 실행 성공이 아니다.

기존 여덟 suite는 다음과 같다.

| Suite | 목적 |
|---|---|
| `facade-contracts` | facade/session/adapter/production 계약 |
| `coupled-contracts` | causal/structural/homotopy 계약 |
| `all-targets` | 변경한 package의 모든 target compile |
| `release-contracts` | 실제 release/LTO profile의 native 계약 |
| `docs` | rustdoc/doctest |
| `smoke-release` | 실제 release binary의 serialized solve와 callback accounting |
| `local-benchmarks` | 정확도가 일치하는 실제 hardware 비교 |
| `local-profiles` | 준비·검증·실패까지 포함한 profile |

`run_local.py`는 원래 `DEVELOPMENT_PLAN.json`에서 동일한 suite argv를 읽는다.
새 expanded DAG는 스케줄링·완료 상태에 사용한다. 예를 들어 먼저 dry-run한다.

```bash
python3 research/production_solver_closure_20261027/run_local.py \
  --suite facade-contracts \
  --output research/cumulative_validation_local/evidence/suites/facade-contracts
```

계획한 command/input/pass·kill 조건과 resource budget을 공개 commit으로 고정한
뒤 같은 호출에 `--execute --timeout 1800`을 붙여 실행한다. 1800초는 기존
handoff의 caller resource allowance이며 수치해석 상수가 아니다. 실제 장비에
맞춰 변경하면 그 사유와 도달한 결과를 receipt에 남긴다.

**중요한 lock 사용:** `run_local.py`는 내부에서
`/tmp/vigilode-cargo-20261027.lock`을 잡는다. 같은 lock을 잡는 외부 `flock`으로
이 runner를 다시 감싸지 않는다. 직접 실행하는 별도 Cargo/native/profile
command만 동일 lock 아래에서 직렬화한다. 문헌·증명·서로 다른 파일의 수정은
native subagent로 병렬 진행할 수 있다.

기본 workspace에서 제외된 `vigilode-bench` 및 `vigilode-hybrid`의 검증 범위는
명시한다. Bench는 Q-BENCH의 실제 SUNDIALS/MPI 환경에서 별도 manifest로
검증한다. Hybrid 전체를 이번 대화의 새 runtime 변경으로 오인하지 않는다.

## 6. Historical replay와 current-tree 검증을 분리

PR90–93의 원래 scientific claim을 전부 inventory에 포함하고 각각
`reproduced`, `reused_with_stated_identity`, `superseded`, `failed`,
`not_run` 중 실제 상태와 근거를 기록한다. 기존 evidence를 새 실행처럼
재라벨하지 않는다. 같은 테스트를 focused/whole-package로 반복한 수를 중복
합산하지 않는다.

원래 결과의 귀속·비교가 필요한 경우 해당 receipt가 지정한 original source와
input을 **disposable worktree**에 복원한다. 공개 local→published equal-tree
mapping이 있다면 실제 tree 관계를 확인하고 두 commit identity를 유지한다.
PR 번호나 현재 branch head가 원래 실행 source와 같다고 추정하지 않는다.

다음 과거 스크립트는 자기 주변의 결과 파일을 덮어쓸 수 있다.

- PR90 `dopri_phase_exact.py`.
- PR91 `exact_checks.py`, `exact_constant_checks.py`,
  `exact_structural_constants.py`.

원래 source commit의 disposable worktree 또는 필요한 fixture와 상대 경로를
보존한 replay mirror에서 실행하고, 새 출력과 receipt를 새 evidence node로
복사한다. Frozen 원본 결과·receipt를 덮어쓰지 않는다. Metadata의 elapsed/path
차이는 mathematical fields와 분리해 비교한다.

PR93의 `verify.py`는 의도적으로 **runtime bytes unchanged**를 검사한다. 이
스크립트는 PR93의 고정 source snapshot에서 artifact 재현을 확인하는 용도다.
새 runtime 구현을 추가한 현재 tree에서 그 assertion이 실패한다고 과거 script의
조건을 완화하지 않는다. 현재 tree에는 새 변경 scope에 맞춘 검증 receipt를
만든다. PR93의 세 `CHECK.py`는 stdout을 새 파일로 저장하되 geometry command의
원래 `--preregistration-sha 866668348f840bd43558859d2795576e292b6ca7`를 유지한다.

이번 통합의 ERROR/ALGEBRA 검산은 위의 사전등록 source에서 실제 통과한 결과다.
독립 실행에서는 같은 source/input을 새 receipt로 재현하고, 이후 native
이식은 **최종 current tree에서 별도로** 검증한다. 일반 정리의 증명, exact
finite check, floating outward 구현, 독립 physical oracle, 성능 측정을 한 가지
PASS로 합치지 않는다.

비용이 큰 모든 과거 campaign을 의미 없이 다시 실행하지 않는다. 최종 tree에서
각 고유 변경 동작과 필수 interaction을 검증하고, 실제 비교·실패 원인 판정에
필요한 원래 snapshot만 별도로 재현한다. Original dense equation, exact
rationals, 높은 정밀도 physical solution 등을 독립 oracle로 사용한다. 같은
production 함수를 다른 branch에서 한 번 더 부르는 것은 독립 수학 oracle가 아니다.

## 7. 먼저 해결할 일곱 Q 게이트

| 게이트 | 실행·수정 계약과 다음 행동 |
|---|---|
| **Q-GIVENS** | 정확한 A/b/Jacobi 입력 bits와 profile을 보존하고, 반환된 `Ok`의 independent true residual을 expected-status assertion보다 먼저 확인한다. 과거 RHS (10^{14})에서 `Ok(3640)`이 나왔다는 사실만으로 false convergence가 확정되지 않는다. 그 근거 없이 기대값을 뒤집지 않는다. |
| **Q-FAIR-AB** | attempted (A), admitted (E), refused (R), budget (B)를 독립 callback entry와 대조해 (A=E+R, E\le B)를 확인한다. 이미 수정한 lagged physical ledger (A-P)와 다른 계약이다. |
| **Q-BENCH** | 선언된 SUNDIALS 6.4.1 또는 명시적으로 자격화한 7.x bridge를 선택하고 compiler/MPI include·link discovery를 기록한다. Build 성공 이후 comparator의 동일 물리 문제·출력·정확도도 확인한다. |
| **Q-LINT** | 실제 23개 integrator diagnostic site를 arithmetic/API 의미를 보존하며 수정한다. Clippy가 통과하면 E4에서 건너뛰었던 vendor, frozen graph 비교, offline compile/test와 source-stability 단계를 이어 실행한다. |
| **Q-API** | PR92의 새 `SwitchingConfig` field와 PR90의 enum/API 변경까지 포함해 pre-conversation 및 PR91 baseline에 대한 downstream exhaustive literal/match를 확인한다. Compatibility 유지 또는 명시적인 version/migration 결정을 구현한다. |
| **Q-NEGATIVE** | 기존 (15/7>1.3) W-filtered research FAIL과 부정확한 Off arm을 보존한다. Known-failure 재현을 확인하는 expected-negative regression의 PASS는 원래 성능 기준의 PASS와 다르다. 나머지 ignored target의 실행 상태도 빠짐없이 기록한다. |
| **Q-LEDGER** | repository/object kind/identifier/required relation을 구분하여 authority를 검증한다. 실제로 누락된 scientific source는 unresolved로 남기고 정확한 복원 근거를 찾는다. |

PR93의 최신 CI 실패는 `validation_design/CI_OBSERVATIONS.json`에 있다.
E4는 default dependency graph·compile·touched default contract가 통과했지만
integrator Clippy에서 실패하여 offline 단계가 skipped였다. 전체 authority
workflow는 `fetch-depth: 0`인데도 실패했다. 이를 모두 shallow history 탓으로
설명하지 않는다.

Q-LEDGER에는 적어도 다음 identity 차이가 있다.

- Julia `git-tree-sha1`은 package source **tree**다.
- BASS/rei_bianchi의 commit은 그 **다른 repository**에 속한다.
- 과거 local commit과 published commit은 **별개의 identity**이며, 확인된
  equal-tree mapping은 source 재사용의 근거다.
- 실제 unmapped source commit은 여전히 해결할 provenance 문제다.

현재 scanner의 오분류는 최소한의 typed resolver와 해당 regression으로 수정한다.
모든 missing ID를 allowlist에 넣거나 whole research directory를 제외하여
통과시키지 않는다. 과거 receipt가 다른 commit에서 실행되었다고 다시 쓰지도 않는다.

## 8. 누적 current tree에서 추가할 열 가지 interaction

다음은 이번 handoff 시점에 계획된 native checks다. 실제 구현·실행 근거가
생기기 전에는 PASS로 표시하지 않는다.

| ID | 독립 검증 내용 |
|---|---|
| **I-STATE** | Winning probe session 지속 × rejection × requested output. 중단 없이 실행한 session과 전체 state/budget을 보존해 이어간 session의 관측·결정·callback parity. |
| **I-EPOCH** | LU/polynomial/causal warm reuse × step/provider/model/norm 변경. 동일 identity는 재사용하고 변경 identity는 무효화하며 원래 결과·인증을 유지. |
| **I-WORK** | Budget refusal, admitted callback failure, diagnostic residual, comparator early stop, scheduled worker 실패를 조합해 실제 callback entry와 ledger를 독립 대조. |
| **I-BUDGET** | Physical step budget × polynomial/root × lift/update/clock. E2 anchor 반례, norm conversion, double-count/누락, stage certificate와 temporal error의 구분. |
| **I-POLY** | Distinct/SameVector × fused-only/column-return × degree≤128/>128, cancellation, zero/subnormal, rounded chart 경계, refusal/fallback과 complete-total admission. |
| **I-ROOT** | Old/new planner × 같은 scalar/rational backend. (Y=0,g=0), 강한 cubic, rounding floor, update/clock budget exhaustion, 원래 branch/residual. |
| **I-STRUCTURE** | Exact quotient / near-structure / full RHS. Original full residual, annihilator/remainder enclosure, omitted stages, domain/provider invalidation. |
| **I-NEGATIVE** | Nonnormal spectral trap, two-root zero-residual ambiguity, 불안정한 variable-step Verlet product, 기존 W-filtered cost failure. |
| **I-PROFILE** | Floating/branch-sensitive fixture를 test, measurement, 실제 release/LTO profile에서 비교하고 source/binary/CPU/libm identity를 기록. |
| **I-CLIENT** | 실제 client의 전체 반환 궤적·관측량·긴 적분·실패와 restart/resource 상황. 같은 물리 오차에서 persistent RHS-only와 가장 강한 적합 comparator를 비교. |

Coverage의 73개 변경 경로 각각을 위 interaction 및 원래 G90/G91/G92/G93
gate와 연결한다. Production config, default-off behavior, phase/observable
오차, partial accepted-prefix 반환을 tests/physical oracle에서 확인한다.
목록에 없는 현재-tree 변경이 생기면 해당 scope만 명시적으로 추가한다.

## 9. 여섯 N 과제의 구현 순서와 채택 기준

기존 N 과제는 모두 `pending_local`이다. 연구 문서에 후보가 존재한다는 이유로
구현 완료로 바꾸지 않는다. Independent PR에서 우선 qualification을 확보하고,
선택해 활성화하는 후보는 해당 N 게이트와 interaction을 끝까지 충족한다.

| 과제 | 우선 구현·비교 | 유지할 이론 조건 |
|---|---|---|
| **PR-N01** | Distinct fused-only backward Clenshaw; 적합한 affine target에서는 quotient endpoint compiler도 비교. 기존 forward signed-adjoint, enclosed Leja, direct relation 및 해당 solve의 CG/shared coupled basis를 포함. | 새 backward local-defect enclosure, coefficient/action/norm/epoch 계약. Dense symmetric 성과를 matrix-free 성과로 부르지 않음. |
| **PR-N02** | (1/12\)+positive rational decay planner를 directed arithmetic으로 이식. Root backend를 고정하여 grid 절감과 root 비용을 분리. | 정확해 감쇠와 수치 오차 전달의 올바른 anchor; root/update/clock prefix 예산. 관측된 (N=256)을 범용 상수로 하드코딩하지 않음. |
| **PR-N03** | Variable-step physical transport, exact-flow comparator, charged adaptive consumer. | Immutable projector/model 범위, physical defect와 transcendental enclosure, 유효 clock convention. |
| **PR-N04** | 기존 `with_session`에서 winning native session을 이어가기, root workspace capacity 재사용. | FSAL/controller/schedule/cursor/budget/failure/epoch 전부 보존; operator가 바뀌면 history를 갱신. |
| **PR-N05** | Original raw-U finite causal budget, affine/near-affine compiler, strong forward와 batched work/span 비교. | Nonlinear remainder/domain, changed carrier의 temporal 및 stiff-uniform order. Cached RHS 28회를 forward 7회와 비교. |
| **PR-N06** | 관측 비용과 인증된 미래 변화량을 분리하고, productive comparator 및 charged revalidation을 연결. | (d,V,S)가 인증될 때만 residence/payback 부등식 사용. 매끄러운 RHS에서 smooth work를 추정하지 않음. |

Optional geometric/DAE/affine-scan 경로는 선언된 구조와 사용 사례가 있을 때
해당 parent N 과제 안에서 선택한다. 일반 adaptive product stability나 nonlinear
DAE 전체를 기존의 제한된 정리로 승인하지 않는다. 아직 구현하지 않은 후보는
최종 보고서에 그 상태를 명시한다. 기존 default의 qualification과 모든 새
연구 후보의 완료를 서로 혼동하지 않는다.

## 10. 수정 예산, 비용 분리와 결과 기록

기존 correction counter를 그대로 이어받는다.

| 과제 | 사용/최대 |
|---|---:|
| Structural 연구 | 3/3 |
| Adapter 연구 | 3/3 |
| Homotopy 연구 | 1/3 |
| 별도로 진단한 integration-accounting 수정 | 1/3 |
| N01–N06 각각 | 0/3 |
| Q-* 일곱 과제 각각 | 0/3 |

후속 대안의 이름을 바꿔 같은 과제에 새로운 3회를 부여하지 않는다. 한 productive
수정 episode 안의 여러 edit·관련 regression을 임의의 새 연구 loop로 쪼개지도
않는다. 동일한 실패를 변경 없이 반복하는 경우 initial attempt와 한 번의 진단
retry 후에는 원인에 맞는 다른 접근이나 checkpoint로 전환한다. 예산이 끝난
과제는 best verified 결과와 남은 조건을 보존한다.

Benchmark는 adapter/switcher와 실제 mode/kernel 비용을 구분한다. 전체 비용에는
coefficient/plan/certificate 준비, 변환·copy, allocation, pool/barrier, original
residual, interval 평가, 실패와 fallback을 포함한다. Callback-unit work,
instructions, cycles, elapsed time과 physical error를 별개로 기록한다.

실제 timing hypothesis와 입력·정확도·순서·pass/kill gate를 실행 전에 공개
commit에 고정하고, 프로젝트 정책상 필요한 holdout을 보존한다. 동일 장비와
정확도가 맞는 paired 비교, cold/warm 분리, 다른 fresh target의 exact binary를
사용한다. Clock/instruction 차이를 source-equivalent라는 이유로 추정하지 않는다.
선행 PR의 `diagnostic_only` timing은 새 local release authority로 승격하지 않는다.

각 command는 source commit/tree, source/input 및 executable hash, argv/cwd/env,
toolchain/CPU/libm/profile, timeout, exit, raw log, 실제 도달한 test와 skipped/
ignored 수를 기록한다. `COMMANDS.json`의 미실행 상태를 실행 로그 없이 PASS로
바꾸지 않는다. 실패한 전체 sweep은 실패로 남기고 이후 focused PASS는 그 범위로
표시한다. 구체적인 실패와 무관한 과제는 병렬로 계속 진행한다.

## 11. 최종 독립 PR의 완료 조건과 제출물

필수 qualification은 실제 최종 cumulative tree에서 이루어져야 한다. 최소한
다음 산출물을 새 validation evidence node에 만든다.

- 최신 main 및 선행 PR과의 관계, 고정 DELIVERY_SHA, merge/conflict 기록,
  최종 source tree와 변경 경로의 current semantic coverage.
- 모든 선행 claim의 reproduced/reused/superseded/failed/not-run 표와
  original-source replay의 새 receipt.
- 일곱 Q 게이트, 열 interaction, 여덟 suite 및 선택한 N 후보의 실제 상태;
  failed/skipped/ignored/inconclusive를 별도로 보존한 machine-readable summary.
- 독립 physical oracle, 정확도·안정성·수렴성 결과와 필요한 phase/invariant/
  observable 비교. Component residual과 physical accuracy를 구별.
- 같은 물리 오차와 전체 비용 경계의 benchmark/profile 및 불리한 결과.
- Public API/version/migration 결정, 엄격한 production config, 지원되는
  provider/model/output 조합과 현재 미지원 범위.
- 갱신한 REVTeX 보고서, 실행 DAG/계획, source/input/binary manifests,
  다음 local handoff와 실제 최종 전체 SHA.

모든 **해당 범위의 필수 상태**가 통과하기 전에는 production-ready 또는 전체
검증 완료라고 주장하지 않는다. Optional 미구현 연구는 명시적으로 pending으로
남겨도 되지만 그것이 검증된 기능 목록에 들어가면 안 된다. 불리한 benchmark나
미완료 gate를 숨기지 않고 draft PR을 제출할 수 있으며, 그때는 통과한 범위와
남은 원인을 정확히 적는다.

새 validation branch를 non-force push하고 **base=main인 독립 draft PR**을
생성한다. PR 본문은 해결하는 실제 문제, 누적 포함 범위, 이번 수정, 최종 동작,
실제 validation과 남은 gate를 설명한다. PR90–93에 대한 링크와 source pins를
남기되 선행 branch를 변경하거나 merge하지 않는다. 마지막 응답에는 새 PR URL,
전체 commit SHA, 실제 측정 개선과 잔여 조건, 갱신된 handoff 경로를 제공하라.
