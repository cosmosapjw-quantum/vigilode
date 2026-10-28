# Local Codex handoff: polynomial polyalgorithm 후속 구현

## 그대로 전달할 작업 지시

VigilODE의 최신 이론 보충 연구를 이어서 구현하고 검증하라. 사용자가 최종
전달한 **EXPECTED_COMMIT 전체 40자리 SHA**가 시작 기준이다. 이동하는 branch
HEAD로 대신하지 말라. 저장소는 `cosmosapjw-quantum/vigilode`, 전달 branch는
`research/polyalgorithm-alternatives-20261028`이며 PR92 위에 쌓인 연구 변경이다.

이전 checkout을 reset하지 말고 새 worktree를 만든다. 먼저 원격 PR/CI가
추가되었는지 조사하고 baseline과 차이를 기록한다. GitHub PR 번호와 계획의
PR-N01~06 과제 ID를 구분하라. 기본 branch에 merge하거나 force push하지 말라.
사용자는 검증된 작업의 새 branch push와 handoff 갱신을 이미 요청했다.

```bash
set -euo pipefail
EXPECTED_COMMIT=REPLACE_WITH_FULL_SHA_FROM_FINAL_DELIVERY
VIGILODE_REPO=/absolute/path/to/vigilode
VIGILODE_WORKTREE=/absolute/path/to/vigilode-polyalgorithm-local
cd "$VIGILODE_REPO"
git status --short
git fetch origin research/polyalgorithm-alternatives-20261028
git cat-file -e "${EXPECTED_COMMIT}^{commit}"
git worktree add -b research/local-polyalgorithm-next "$VIGILODE_WORKTREE" "$EXPECTED_COMMIT"
cd "$VIGILODE_WORKTREE"
test "$(git rev-parse HEAD)" = "$EXPECTED_COMMIT"
python3 research/polyalgorithm_alternatives_20261028/verify.py
```

## 이미 완료된 것과 아직 완료되지 않은 것

`physmath-research-loop` 1.3.0의 R2/R4를 세 native subagent로 병렬 수행했다.
세 CHECK.py와 protocol은 `866668348f840bd43558859d2795576e292b6ca7`에 먼저
공개했으며 그 후 실제 exact-rational 검산이 모두 통과했다. 이전 exploratory
CAS와 등록 후 실행을 혼합하지 않는다. 이번 보충 연구는 Rust runtime을
바꾸거나 solver native test/benchmark/profile을 새로 실행하지 않았다.

일반 증명과 계산 예제는 구별하라. 다음의 새 결과는 native 구현의 근거이지
production promotion 결과가 아니다.

- distinct 5-vector fused-only Clenshaw: operator products 5m→m. SameVector는
  이미 m회이고 개별 column 출력에는 shortcut을 적용하지 못한다.
- Cubic local constant 1/8→1/12와 rational decay sum: 지정 exact fixture에서
  최소 dyadic N 512→256. 1/12만으로는 N이 줄지 않는다.
- Variable-step two-channel transfer, causal residual majorant와 invariant
  drift identity. Stage residual이 곧 physical accuracy라는 주장은 하지 않는다.
- Value-only continuation station margin과 조건부 residence payback.
  Generic branch/미래 cost variation certificate는 여전히 필요하다.

## 먼저 읽을 파일

1. `research/polyalgorithm_alternatives_20261028/REPORT_KO.md`
2. 같은 node의 `LOCAL_DEVELOPMENT_PLAN.json`, `DEVELOPMENT_DAG.md`, `SOURCES.json`
3. `polynomial/THEORY.md`, `homotopy/THEORY.md`, `geometry/THEORY.md`와 각각의
   `candidates.json`
4. `evidence/*_RESULTS.json`, `evidence/*_RECEIPT.json`, `evidence/REVIEW.md`
5. `research/production_solver_closure_20261027/post_push/STATUS_KO.md`,
   `LOCAL_DEVELOPMENT_PLAN.json` 및 seven Q-* 진단 문서
6. PR92의 `PRODUCTION_SETUP.md`, `SOURCE_MANIFEST.json`, 각 mode/adapter 원본
   결과와 보존된 실패 기록

기존 23-node/34-edge DAG, eight execution suites와 correction budgets를
보존했다. 17개 candidate는 여섯 과제 안의 대안이다. 17개 모두를 새 독립 과제로
만들어 수정 예산을 리셋하지 않는다. R2/R4가 끝났다는 이유로 `pending_local`
구현을 완료로 표시하지 말라.

## 구현 순서

**Qualification과 이론 보충을 분리하라.** 기존 Q-GIVENS, Q-FAIR-AB, Q-BENCH,
Q-LINT, Q-API, Q-NEGATIVE, Q-LEDGER는 아직 열려 있다. Q-GIVENS는 정확 입력과
독립 residual을 확인하기 전에 역사적 false 기대값을 true로 바꾸지 않는다.
Q-FAIR-AB는 A=E+R, E<=B의 attempted/admitted/refused 구분이다. Q-NEGATIVE는
이전 15/7>1.3 실패를 보존한다. Q-LEDGER는 full history/remote refs에서 검사하며
새 allowlist로 missing authority를 숨기지 않는다. 자세한 재현 절차는 기존 plan에 있다.

L-RELEASE → L-HARDWARE → R-QUALIFY가 배포 검증의 중심이다. 이론 검토와 겹치지
않는 코드 작성은 병렬로 하되, builds/native tests/timings/profiles는 기존
`/tmp/vigilode-cargo-20261027.lock`을 공유해 직렬화한다. Rust1.94.1, unchanged
Cargo.lock, 실제 deployment release/LTO/libm/CPU 조건을 기록한다. 고정 suite는
기존 `research/production_solver_closure_20261027/run_local.py`를 사용한다.

각 native 연구는 첫 실행 전에 question, source/input, command, pass/kill gate,
negative controls와 holdout이 필요한 주장인지를 정하고 공개 commit하라.
과제당 최대 3회 수정·재실험을 지키며 이미 사용한 예산을 이어받는다. 문헌·증명·
서로 다른 파일의 구현은 native subagent로 병렬화하라. 적용 중인 skill version과
요청된 단계 범위를 명시하고, 이전 10단계를 불필요하게 재실행하지 말라.

### PR-N01: 먼저 fused-only Clenshaw

현재 core의 계수 enclosure, complete-total admission, exact cache identity와
failure counters를 재사용한다. 새 backward recurrence는 새 local-defect enclosure를
필요로 한다. 기존 forward defect에 새 Laguerre 상수를 대입하지 않는다.
첫 구현은 현재 검증된 dense symmetric provider에서 비교하고, matrix-free
provider로 옮길 때 action-error/norm/domain/epoch witness를 별도로 요구하라.
DenseMatrix 구현을 matrix-free 성과로 표현하지 않는다.

검증 축은 Distinct/SameVector, fused-only/column-return, cold/warm,
degree<=128/>128, cancellation/range/subnormal, refusal/failure이다. 강한
비교자는 최신 forward signed-adjoint, enclosed Leja, 선언된 quotient/direct,
그리고 해당 linear solve에서 CG/shared coupled basis다. 모든 preparation,
certificate, copy, calibration, residual과 실패를 총비용에 포함하라.

### PR-N02: planner를 바꾸고 root backend는 비교에서 고정

`CubicDecayAccuracyPlan`에 1/12, 양수 geometric doubling, rational decay를
directed arithmetic으로 이식한다. 정확해의 감쇠를 수치 궤적에 가정하지 않는다.
Root residual, update rounding, time-label allowance를 포함하는 전체 prefix
gate를 유지한다. a_step은 이 전체 오차의 예산이다. 최소 dyadic 검증,
Y=0/g=0/strong nonlinear/rounding-floor/subnormal/overflow 사례를 포함한다.
Old/new planner × 동일 scalar/rational backend로 비교해 step 절감과 root 비용을
분리하라. 관측 N=256을 범용 tuning 상수로 하드코딩하지 않는다.

### PR-N03와 PR-N05: residual에서 endpoint와 physical error로

N03은 immutable projector/model 아래 variable-step scalar products와 adjoints,
clock/endpoint/temporal errors 및 charged adaptive consumer를 구현한다. 선언된
affine exact flow가 더 싼지 비교하라. 일반 exp/expm1 결과에 한 번 nextafter를
씌워 transcendental certificate로 만들지 않는다.

N05는 기존 nilpotent causal theorem을 재증명하는 대신 raw-U의 full residual,
stage-domain/inverse/nonlinear-remainder bounds와 endpoint weights를 연결한다.
Carrier를 바꾸면 nonlinear order 및 stiff-uniform accuracy를 별도 검증한다.
28회 cached RHS를 7회 forward comparator와 비교하고, pool/barrier/setup/검증을
포함한 work/span으로 판단한다. Polynomial recurrence 자체도 degree 방향으로
순차 의존적이다. Affine scan은 state-independent 사전 고정 grid에만 적용한다.

### PR-N04와 PR-N06: adapter overhead를 mode 비용과 분리

N04의 첫 후보는 기존 with_session 안에서 winning native session을 유지하는
구성이다. FSAL/controller/output schedule/epoch/누적 budget/failure state가
모두 같아야 trace parity가 성립한다. Root workspace capacity는 재사용하되
lambda/model/base-state 변경 시 Anderson history를 무효화한다. 중간 station이
필요한지 direct endpoint와 비교하고 원래 residual/physical branch를 유지한다.

N06은 observed work와 certified future variation을 다른 데이터로 저장한다.
인증된 d,V,S가 있을 때만 mV<d와 md−Vm(m+1)/2>S를 사용한다. 실측 work 차분을
V의 증명으로 바꾸지 않는다. Trace-margin 증명이 없으면 현재 productive
same-window comparator와 사용자 resource budget을 유지한다.

## 끝까지 보존할 원칙

기본은 RHS-only다. JVP/AD는 완전 derivative-free 경로가 불리한 사례에서
명시적으로 허용된 selective provider이며, matrix-free와 derivative-free는
다른 속성이다. Hidden finite-difference Jacobian, dense Newton, target parameter
수정을 fallback으로 넣지 않는다. Analytic proof에서 derivative를 쓰는 것과
runtime derivative callback을 실행하는 것도 구별한다.

모든 선택은 같은 수학적 target과 물리 오차 계약 안에서 한다. Joint phi action,
resolvent, nonlinear stage root, affine flow의 admission token을 혼용하지 않는다.
기하 불변량, 정확 symplecticity, fixed-step stability, adaptive product stability를
같은 판정으로 취급하지 않는다. 임의 safety multiplier 대신 증명된 상수와
outward integer decision을 쓰되, 하드웨어 시간은 측정해야 하는 양이다.

최종에는 실제 변경, 증명, 실행한 test/benchmark/profile, 남은 조건을 분리해
보고하고 새 branch/PR을 push하라. 기존 실패와 부정적 대조군을 보존하고,
REVTeX 보고서, machine-readable plan 및 이 handoff를 실제 최종 SHA와 함께 갱신하라.
