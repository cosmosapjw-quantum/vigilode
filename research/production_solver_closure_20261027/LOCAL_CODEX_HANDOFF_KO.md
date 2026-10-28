# Local Codex 인계: VigilODE production 검증과 다음 연구 PR

이 문서는 local Codex에 그대로 전달할 작업 지시다. 기준은 최종 전달 메시지에
명시된 **`EXPECTED_COMMIT` 전체 SHA**다. 아래의 PR-N01~06은 개발 과제 ID이며,
실제 GitHub PR 번호가 아니다. 게시된 브랜치의 최신 HEAD를 기준 SHA 대신 쓰지 않는다.

## 1. 목표와 먼저 읽을 근거

현재 구현을 실제 배포 머신에서 검증하고, 확인된 병목에 대한 이론 연구와 코딩
연구를 `$physmath-research-loop`로 병렬 수행하라. 각 과제는 가정·정리·증명·코드
계약·실험 결과를 연결하고, 수정 후 재실험은 **과제당 최대 3회**다. 새 PR의
과제 ID, 선행 조건, 산출물은 `DEVELOPMENT_PLAN.json`을 따른다. 단순히 실패한
과제의 이름을 바꾸어 수정 예산을 다시 시작하지 않는다.

작업 루트에서 적용되는 `AGENTS.md`와 사용 가능한 해당 skill을 읽은 뒤, 다음
순서로 현재 자료를 비판적으로 검토하라. 외부 문헌 확인이 필요하면 사용자가
지정한 SciSpace/Exa/Wolfram을 활용하되, 계산 결과와 인용을 증명으로 대체하지 않는다.

1. `research/production_solver_closure_20261027/PRODUCTION_SETUP.md`
2. 같은 디렉터리의 `DEVELOPMENT_PLAN.json`, `SOURCE_MANIFEST.json`,
   `evidence/RAW_MANIFEST.json` 및 최종 보고서
3. `adapter/THEORY.md`, `adapter/REPORT_KO.md`, `adapter/PERFORMANCE.json`
4. `homotopy/THEORY.md`, `homotopy/REPORT_KO.md`, `homotopy/ANALYSIS.json`
5. `structural/THEORY.md`, `structural/README_KO.md`,
   `structural/FINAL_EXACT_ORACLE_RESULTS.json`, `structural/FINAL_BENCHMARK_SUMMARY.json`
6. `theory/HOMOTOPY_REVIEW.json`, `theory/STRUCTURAL_REVIEW.json`와 기존 PR91의
   `research/derivative_free_causal_homotopy_20261027/`

이후 문서의 상대 경로는 별도 표시가 없으면
`research/production_solver_closure_20261027/` 기준이다. 원문, 실행 receipt,
source/binary hash가 결론과 일치하는지 확인한다. 포장 검증 PASS는 수학적 증명이나
native test PASS를 뜻하지 않는다.

## 2. 정확한 commit으로 격리하고 준비하기

사용자의 기존 checkout을 reset하거나 덮어쓰지 않는다. 아래 경로와 SHA를 실제
전달값으로 바꾸고 새 작업 브랜치와 worktree를 만든다. 같은 이름이 이미 있으면
새 이름을 선택한다. 브랜치가 진전되어도 검증 기준은 전달받은 SHA다.

```bash
set -euo pipefail
VIGILODE_REPO=/absolute/path/to/vigilode
VIGILODE_WORKTREE=/absolute/path/to/vigilode-local-production
EXPECTED_COMMIT=REPLACE_WITH_FULL_SHA_FROM_FINAL_DELIVERY
cd "$VIGILODE_REPO"
git status --short
git fetch origin research/production-solver-closure-20261027
git cat-file -e "${EXPECTED_COMMIT}^{commit}"
git worktree add -b research/local-production-validation "$VIGILODE_WORKTREE" "$EXPECTED_COMMIT"
cd "$VIGILODE_WORKTREE"
test "$(git rev-parse HEAD)" = "$EXPECTED_COMMIT"

rustup toolchain install 1.94.1 --profile minimal --component rustfmt --component clippy
rustup override set 1.94.1
rustc --version
cargo metadata --locked --offline --format-version 1 --no-deps
python3 research/production_solver_closure_20261027/verify_package.py --extract-evidence
```

의존성 캐시가 없어서 offline 준비가 실패하면 `cargo fetch --locked`로 먼저
준비하고, 실제 검증은 `--locked --offline`로 실행한다. `Cargo.lock`을 다시
생성하지 않는다. CPU, OS, compiler/target features, libm, affinity와 thread
설정을 기록한다. Outward enclosure는 binary64 round-to-nearest, gradual
underflow와 명시된 FMA 의미를 전제로 한다. fast-math나 FP 재결합을 켜지 않는다.

## 3. 먼저 dry-run, 다음에 suite를 순서대로 실행하기

아래는 POSIX/WSL 명령이다. `run_local.py`는 `--execute`가 없으면 argv만
표시한다. `VIGILODE_LOCAL_OUTPUT`은 기존 결과가 없는 새 경로로 정하고,
최초 독립 release 검증에는 이전 실행에서 사용하지 않은 `CARGO_TARGET_DIR`를 지정한다. `1800`초는 호출자가
조정 가능한 **명령별 실행 자원 상한**이며 수치해석 상수가 아니다.

```bash
set -euo pipefail
VIGILODE_NODE=research/production_solver_closure_20261027
VIGILODE_LOCAL_OUTPUT=/absolute/path/to/new-vigilode-local-results
export CARGO_TARGET_DIR="$VIGILODE_WORKTREE/target-local-production"
export CARGO_BUILD_JOBS=2
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export RUSTDOCFLAGS="-D warnings"

for suite in release-contracts docs smoke-release local-benchmarks local-profiles; do
  python3 "$VIGILODE_NODE/run_local.py" --suite "$suite" \
    --output "$VIGILODE_LOCAL_OUTPUT/$suite"
done
```

계획된 argv와 호스트 자원이 맞으면, 먼저 L-RELEASE를 실행한다. 중간 실패는
그대로 보존하고 원인을 분석한 뒤 다음 단계 진행 여부를 판단한다.

```bash
for suite in release-contracts docs smoke-release; do
  python3 "$VIGILODE_NODE/run_local.py" --suite "$suite" \
    --output "$VIGILODE_LOCAL_OUTPUT/$suite" --execute --timeout 1800
done
```

L-RELEASE 통과 후 Valgrind를 준비하고 L-HARDWARE를 수행한다. 기본은 PATH의
`valgrind`이고, 별도 설치라면 `VIGILODE_VALGRIND`에 실행 파일의 절대 경로를
설정한다. 압축 해제형 설치는 해당 설치의 `VALGRIND_LIB`도 필요하다. 정상
설치의 라이브러리 경로를 임의로 덮어쓰지 않는다.

```bash
"${VIGILODE_VALGRIND:-valgrind}" --version
for suite in local-benchmarks local-profiles; do
  python3 "$VIGILODE_NODE/run_local.py" --suite "$suite" \
    --output "$VIGILODE_LOCAL_OUTPUT/$suite" --execute --timeout 1800
done
```

모든 build·native 실행·timing·profiling은 계획의 하나의 `native_lock`을
공유한다. `run_local.py`가 이를 획득하므로 외부에서 같은 lock으로 이 runner를
다시 감싸지 않는다. 직접 실행하는 추가 명령은 같은 lock과 receipt 규약을
사용한다. 증명·문헌 조사·서로 겹치지 않는 코드 수정은 병렬로 진행할 수 있다.
실행 중인 소스나 binary를 바꾸지 않는다. timeout/중단 시 자식 process group의
종료와 partial receipt를 확인하고, 다른 사람의 계산은 종료하지 않는다.

### 실행 범위와 정확한 suite 이름

| Suite | 검증/측정 범위 |
|---|---|
| `facade-contracts` | measurement profile의 `vigilode` 전체 lib/integration tests |
| `coupled-contracts` | measurement profile의 `vigilode-coupled` 전체 lib/integration tests |
| `all-targets` | 위 두 package의 모든 target compile check |
| `release-contracts` | release profile의 `vigilode`, `vigilode-coupled`, `rodas5p-core`, `rodas5p-krylov`, `rodas5p-integrators` 전체 lib/integration tests |
| `docs` | facade/coupled rustdoc와 doctests |
| `smoke-release` | 직렬화된 RHS-only/selective-JVP 설정 × rate 1/1000의 네 실제 solve |
| `local-benchmarks` | cubic 4-arm, structural n=1/64/2048와 n=2048 workers 1/2, adapter ABBA/BAAB paired helper와 instruction profile |
| `local-profiles` | 동일 release binary의 cubic generic/scalar/algebraic/rational 및 structural pr91/fused/enclosed 반복 차분 profile |

정확한 argv는 JSON의 `execution_suites`가 권위 있는 기준이다. 추가 local
coverage가 필요하면 첫 세 suite도 같은 runner로 실행한다. 위 suite 실행과
repository의 전체 CI는 구별한다. `.github/workflows/production.yml`의 fmt,
clippy, 전체 package/doctest, examples와 별도 bench workspace 요구사항을 확인한다.
`ignored-tests.yml`의 장시간/ignored 작업은 실행 여부를 따로 기록한다. 이번
전달에서는 기존 format/lint 문제를 정리한 뒤 `fmt_final`과 `clippy_final`이
실제 통과했다. 초기 실패와 수정 범위는 `CI_HYGIENE.md`에 보존했다.
전체 native 검증과 별도 CI의 판정은 `evidence/VALIDATION_SUMMARY.json`을 확인한다.
Cloud의 이전 전체 다섯 package sweep은 1,312 passed / 1 failed / 45 ignored였다.
실패한 lagged JVP 과금은 수치 연산과 기존 assertion을 유지하며 A−P 식으로 보정했다.
보정 후 facade/coupled 424개를 다시 통과했고, 영향을 받는 8개 lagged target의
실제 결과와 기존 oracle 증거 재사용 근거를 요약 파일에서 확인한다. 전체 다섯
package의 보정 후 release sweep은 이 local 단계가 수행해야 한다.
고차 Fraction oracle은 단일 테스트에도 수분이 걸린다. 자원 상한으로 중단되면
partial receipt를 보존하고, 필요한 예산을 명시하여 이어 검증하되 정확도 기준을 낮추지 않는다.

## 4. 보존해야 할 수학·API 계약

**기본 경로는 persistent RHS-only DOPRI5**다. `ProductionConfig::rhs_only`에서
시작한다. 현재 `ProductionSolver`의 범위는 smooth forward, identity-mass,
endpoint-output IVP다. mass/events/dense output 등 지원하지 않는 조합은 preflight
거부 계약을 유지한다. 일반 비선형 문제 전체에 대한 production 인증으로 확대하지
않는다. 실제 client의 장시간 적분·restart·자원 고갈 검증은 `R-QUALIFY`에 남아 있다.

- JVP/AD는 순수 RHS 방식이 해당 문제에서 불리함을 실제로 확인한 경우에만 명시적
  opt-in으로 허용한다. supplied-JVP, replay-safe callback, provider/model epoch
  계약을 지키고 자동 finite-difference Jacobian, dense matrix 또는 Newton fallback을
  추가하지 않는다. 기존 small projected LU/JVP 경로까지 derivative-free라고 부르지 않는다.
- selective entry/return의 정수 비교는 관측한 동일 목표 구간의 work 비교다.
  callback-unit 절감, elapsed time, instruction count와 미래 구간 성능은 서로 다르다.
  `reuse_observed_entry_step=false`를 유지하고 sentinel interval을 일괄 1로 바꾸지
  않는다. 한 window의 `2I+6` 상한은 전 구간 residence/exploration 정리가 아니다.
- `CubicDecayAccuracyPlan`의 scalar cubic decay 증명과 outward physical bound는
  허용된 모델·구간·부동소수 조건에만 적용한다. ScalarPicard와 RationalPredictor를
  강한 기준끼리 비교한다. rational `(1+2β)/(1+3β)`는 증명된 계수이며, 새로운
  predictor도 **원래 식의 outward residual 검사**를 통과해야 한다.
- mean-field enclosure는 full raw-U stage residual 보증이다. temporal truncation과
  endpoint rounding을 더하지 않은 채 physical trajectory 보증으로 승격하지 않는다.
  `StructuralResult`/charged API로 실패·재시도 비용을 보존한다. 호환
  `step`/`sequential_step` wrapper는 실패 work payload를 버리므로 retry/fallback
  ledger에는 쓰지 않는다. `n=1`의 `P=I` 단순화는 정확한 대수식이다.
- 임의의 epsilon 분할, 안정성 multiplier, 차원 임계값 또는 hysteresis를 튜닝해
  결론을 맞추지 않는다. 허용 오차와 자원 제한은 호출자 입력으로 구분하고,
  알고리즘 상수는 가정과 증명에서 도출한다. 실측 비용 구간은 경험적 자료임을
  밝히고 환경·모델 epoch 및 보정 비용을 연결한다.

오차 검증은 원래 target residual, **모든 상태 성분·모든 반환 관측점**, 동일한
physical accuracy cap을 포함한다. 독립 exact/rational/interval oracle를 재사용하되
변경된 수학적 의존성은 새로 검사한다. 모든 accepted/rejected/discarded attempt,
predictor/fallback, RHS/JVP/ft, validation, allocation, pool startup과 실패한
worker의 작업을 집계한다. callback atomics와 ledger를 대조한다. `Ok(result)`만
보지 말고 `success`, `final_time`, `message`, `accounting_complete`를 확인한다.

## 5. 다음 PR의 DAG와 연구 목적

게시 결과의 `P-PUSH` 뒤에 `L-RELEASE → L-HARDWARE`를 진행한다. 아래 선행 조건을
충족한 연구는 병렬 수행한다. 현재 세 lane의 수정 횟수는 structural **3/3**,
homotopy **1/3**, adapter **3/3**다. 새 PR별 예산은 JSON에 기록된 별도 과제의
0/3에서 시작하되, 같은 실패의 숨은 네 번째 수정으로 사용하지 않는다.

| 과제 | 선행 조건 | 목적과 필수 증명/실험 |
|---|---|---|
| PR-N01 | L-HARDWARE | safe_l2/MGS/QR/GMRES의 실제 차원·restart별 연산/byte 비용. rank/breakdown 및 원래 residual을 보존하고 full trajectory로 검증 |
| PR-N02 | L-RELEASE | cubic decay envelope의 finite-step defect와 outward 전파합으로 최소 grid와 잔여 algebraic 예산 도출. root 비용 절감과 step 수 절감 분리 |
| PR-N03 | L-RELEASE | 가변 h의 product-adjoint residual transport, temporal/endpoint rounding과 nonlinear invariant-domain closure. charged structural consumer 구현 |
| PR-N04 | L-RELEASE | 완전한 native session continuation과 generic derivative-free homotopy workspace 재사용. state/controller/FSAL/clock/epoch/callback identity와 invalidate 조건 증명 |
| PR-N05 | L-HARDWARE, PR-N04 | 실제 carrier 정책의 finite causal-DAG full residual/비선형 remainder 및 order 조건. forward substitution 대비 work/span·communication·validation 비용 |
| PR-N06 | L-HARDWARE, PR-N01 | 명시적 variation/dwell/domain 가정 아래 temporal residence·exploration loss 정리. failed pilot, 재검증과 calibration까지 포함한 window/예산 규칙 |
| R-QUALIFY | L-RELEASE, L-HARDWARE | 실제 client/model/provider/output 조합만 release 판정. 새 연구 PR은 각자의 계약 통과 후 별도 편입 |

PR-N01에서 **SPD04 least-squares workspace는 이미 켜져 있다**.
`crates/vigilode/src/solve.rs`의 `krylov_ls_workspace=true`와
`research/spd04_ls_workspace_20261007/`을 먼저 재사용한다. 같은 기능을 새 최적화로
재구현하지 않는다. `crates/rodas5p-krylov/tests/givens_production_differential_contracts.rs`의
nonnormal Givens 실패 사례를 보존하며 QR 대체의 rank/true-residual 조건을 증명한다.
PR-N04의 남은 session/generic homotopy 재사용은 이 SPD04 기능과 별개다.

PR-N06에서는 유한한 과거 표본만으로 임의의 미래 smooth regime을 보증할 수 있다고
가정하지 않는다. 가정을 만족하는 모델에서 정리를 만들고, 범위 밖은 한 구간의
관측 비교 또는 명시적인 finite-budget 실패로 반환한다.

## 6. 부정적 결과, 종료 조건과 산출물

다음 부정적 결과를 benchmark에서 제거하지 않는다. Adapter의 stiff-cubic은
callback work가 줄어도 전체 instruction/time 개선이 자동으로 따라오지 않았다.
Nonlinear reentrant에서는 selective 경로가 RHS-only보다 크게 불리했다.
Homotopy algebraic inverse는 강한 scalar 기준보다 느린 경우가 있었고 rational의
전체 시간 우위도 모든 paired block에서 유지되지 않았다. Structural 초기 n=1
enclosure는 불리했고 정확한 scalar degeneration으로 수정되었다. 최종 큰 n의
fused 경로와 parallel overhead 역시 무조건 우세하지 않다.

ABBA/BAAB 원자료, 모든 block 범위와 median을 보존한다. 범위가 1을 가로지르는
speedup은 안정적인 시간 우위로 선언하지 않는다. 반복 차분 Callgrind Ir은 startup을
줄인 guest instruction 지표이며 실제 CPU cycle/time을 대신하지 않는다. Measurement와
release/LTO binary 결과를 혼합하지 않는다. 오차 상한을 완화하거나 실패 arm을
제외하여 성능 향상을 만들지 않는다.

각 과제는 R1~R10/C1~C10 기록, 증명/반례, 가정과 코드 계약 매핑, source/binary/input
hash, 명령·환경·stdout/stderr·exit·timeout/partial work, 정확도/총비용/시간을
분리한 JSON, 최대 3회 수정의 원인과 결과를 남긴다. 세 번째 수정 후에도 계약이
실패하면 최선의 검증된 구현과 부정적 결과를 보존하고 필요한 후속 이론을 명시한다.
변하지 않은 의존성의 증거를 재사용하고 불필요한 전체 재실험은 피한다.

최종 전달물은 `DEVELOPMENT_PLAN.json`의 local 결과와 각 PR 목적·선행 조건·상태,
REVTeX 보고서 갱신, `release_qualification.json`, 지원 조합/미지원 조합 및 local
실행 receipt다. 새 개발 브랜치로 commit/push하고 검토 가능한 PR을 준비하되,
force-push나 기본 브랜치/기존 PR의 자동 merge는 하지 않는다. 실제 SHA·PR URL·검증된
scope·남은 `pending_local`을 정확히 보고한다. 게시 완료와 일반적 production solver
인증을 같은 의미로 사용하지 않는다.
