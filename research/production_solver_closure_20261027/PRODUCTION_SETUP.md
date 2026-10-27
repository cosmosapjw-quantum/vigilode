# VigilODE production 실행 준비

## 제공하는 실행 범위

이 변경은 실제로 호출할 수 있는 배포 설정, provider 제한, 실패 결과 보존,
회귀 검증과 재현 도구를 완성한다. 기본 실행은 persistent RHS-only DOPRI5다.
`ProductionSolver`는 smooth forward identity-mass IVP의 endpoint 출력을
지원한다. 제공된 JVP를 사용하는 Rosenbrock 경로는 명시적으로 켜며 실제
같은 구간의 비용 비교를 거친다. 전체 프로젝트의 임의 비선형 RHS에 대한
전역 오차·수렴·고차 order 정리는 별도의 연구 과제다.

| 입력·용도 | 실제 public API | 이번에 확인한 근거 |
|---|---|---|
| 일반 smooth RHS, endpoint 관측 | `ProductionConfig::rhs_only` → `ProductionSolver::solve` | 기존 DOPRI session 유지, derivative callback 0, native local-error test와 partial work |
| 검증된 문제에서 selective supplied JVP | `ProductionConfig::selective_jvp` → 같은 solve | exact entry/return 정수 비교, positive pilot budget, callback replay 계약 |
| 선언된 scalar cubic decay | `CubicDecayAccuracyPlan` → `integrate_cubic_decay_accuracy_with_solver` | 모든 반환점의 outward physical-error bound; Scalar/Picard와 Rational 후보 선택 |
| 선언된 affine mean-field raw-U stage | `MeanFieldAffine::enclosure_plan` → `MeanFieldEnclosurePlan::step` | 실제 반환된 모든 stage 성분의 outward original-equation residual bound, charged failure |
| 일반 causal Picard 재사용 | 기존 `BatchedPicardReuse` | PR91의 exact-input/prefix 재사용 근거; frozen JVP와 작은 projected LU가 남음 |

Mean-field의 새 certificate는 stage 방정식에 관한 것이다. 물리 궤적 인증에는
시간 절단오차와 endpoint 반올림을 더해야 한다. `DEVELOPMENT_PLAN.json`의
PR-N03이 그 의무와 이식 위치를 고정한다. 기존 `step`/`sequential_step`은
호환성을 위해 `CoreResult`를 반환하므로 오류의 partial work를 잃는다.
새 adaptive/fallback 소비자는 `step_charged`, `sequential_step_charged` 또는
`MeanFieldEnclosurePlan::step`의 `StructuralResult`를 사용한다.

## 1. 소스와 도구 체인 고정

인계 메시지의 정확한 commit SHA를 사용한다. 기존 사용자 checkout을 reset하지
말고 별도 worktree에서 시작한다. `LOCAL_CODEX_HANDOFF_KO.md`에 전체 명령이 있다.
`rust-toolchain.toml`과 workspace MSRV는 Rust **1.94.1**이다. `Cargo.lock`을
유지한다. 필요한 dependency cache가 없을 때만 `cargo fetch --locked`로
준비하고, 검증은 `--locked --offline`으로 실행한다.

```bash
rustup toolchain install 1.94.1 --profile minimal --component rustfmt --component clippy
rustup override set 1.94.1
rustc --version
cargo metadata --locked --offline --format-version 1 --no-deps
python3 research/production_solver_closure_20261027/verify_package.py --extract-evidence
```

공유 호스트에서는 `measurement` profile을 사용했다: opt-level3, LTO 없음,
codegen-units16. 배포 확인은 기존 `release` profile의 fat LTO, codegen-units1로
다시 한다. 두 profile의 시간과 binary hash를 섞지 않는다. `RUSTFLAGS`에
fast-math나 FP 재결합 옵션을 넣지 않는다. Outward 산술의 전제는 binary64
round-to-nearest, gradual underflow, 명시된 FMA 동작이다.

## 2. 완전한 JSON 설정과 기본 실행

`configs/rhs-only.json`, `configs/selective-jvp.json`은 실제 native example이
출력하고 다시 읽어 성공한 설정이다. 수치 값은 `y'=-rate*y`, n16, [0,1]의
smoke fixture용이다. 실제 client는 물리량별 atol/rtol, 초기 step, attempt cap,
관측 구간을 자신의 계약에서 지정한다. 이 값들을 보편적인 최적 상수로 쓰지 않는다.

```bash
cargo build --release --locked --offline -p vigilode --example production_from_json
target/release/examples/production_from_json \
  research/production_solver_closure_20261027/configs/rhs-only.json 1000
target/release/examples/production_from_json \
  research/production_solver_closure_20261027/configs/selective-jvp.json 1000
```

`CARGO_TARGET_DIR`을 별도로 지정했다면 해당 경로의 `release/examples`를 쓴다.
출력 JSON에는 실제 설정, 모든 반환 상태, partial solution, 총비용, method
segment, oracle 오차와 callback atomics 대조가 들어간다. Fixture 실패는 exit1로
반환되고 JSON에 partial 결과가 남는다. 실제 모델은 example의 `Problem` closure와
독립 oracle을 교체하여 연결한다.

`ProductionConfig` schema1은 누락된 필수 필드를 임의로 채우지 않는다.
선택 필드 `switching.entry_probe_adaptation`만 생략 시 `None`이 된다. 최상위,
switching, nested options와 atol의 알 수 없는 필드를 거부한다. 기존 `Options`의
일반 deserialization 동작은 유지하고 production 전용 strict DTO에서 이 계약을
구현했다. 예를 들어 `options.max_stepz` 같은 오타는 무시되지 않는다.

## 3. 실제 Rust 소비자

```rust
use vigilode::{Atol, CallbackError, Options, Problem, ProductionConfig, ProductionSolver};

let problem = Problem::new(1, |_t, y, dy| {
    dy[0] = -y[0];
    Ok::<(), CallbackError>(())
});
let options = Options {
    rtol: 1e-7,                 // 이 예제의 local-error 계약
    atol: Atol::Scalar(1e-10),
    h0: 1e-4,
    max_steps: 50_000,          // accepted + rejected + probe 전체 attempt cap
    ..Options::default()
};
let config = ProductionConfig::rhs_only(options, 8);
let result = ProductionSolver::solve(&problem, (0.0, 1.0), &[1.0], &config)?;
assert!(result.success && result.final_time == 1.0);
assert!(result.stats.accounting_complete);
```

`Ok(result)`만으로 성공을 판정하지 않는다. `success`, `final_time`, `message`,
`stats.accounting_complete`를 함께 확인한다. 실패한 시도의 비용과 이미 승인한
prefix는 결과에서 유지한다. 콜백의 예상 오류는 `CallbackError`로 반환한다.
`panic=abort` profile에서 panic이나 외부 프로세스 강제 종료는 이 반환 계약을
완료하지 못한다.

모든 RHS는 같은 모델·입력에 같은 수치 함수를 나타내야 한다. Selective replay는
같은 영역을 추가 평가하므로 `replay_safe_callbacks=true`를 요구한다. 외부 효과는
버려질 수 있는 callback에서 확정하지 않는다. 제공된 JVP 안에서 AD를 쓰는 경우에도
그 비용은 실제 provider 비용으로 측정한다.

## 4. Selective 경로의 정확한 범위

Factory는 supplied-JVP only, persistent explicit session, checkpoint seed,
strict entry, strict return, dominated-comparator stop을 선택한다. JVP가 없는
문제, dense/banded provider, finite-difference 허용, zero pilot budget,
replay 계약 누락은 콜백 전에 거부한다. 비자율 문제는 supplied `f_t`가 필요하다.
Native route가 지원하지 않는 mass, event, dense-output 또는 tolerance 조합도
그대로 거부한다.

Entry는 `I<E`, return은 `E<I`, 미완료 implicit comparator는 `E<L`이라는
checked integer 조건을 사용한다. 이 비교는
`W=RHS+JVP+ft+n*Jacobian+solve`라는 기존 목적함수의 strict 감소 조건이다.
시간 최적성에 관한 정리가 아니다. 기존 pressure/backoff/residence 설정은
언제 비교할지 결정하는 operational 선택으로 JSON에 모두 기록한다. 과거 `.8`
비용 ratio 값도 남아 있으나 strict 판정에서는 사용되지 않는다.

`reuse_observed_entry_step`은 **false**를 유지한다. Nonlinear reentrant 사례에서
이 옵션과 sentinel은 RHS-only보다 비용과 실제 시간이 늘었다. `return_sentinel_interval`
역시 자동으로1로 바꾸지 않는다. 일반 배포 기본값은 RHS-only이며, selective 경로는
해당 client의 같은 정확도 검증과 전체 시간 비교 후 사용한다.

## 5. 모델 전용 derivative-free 경로

Cubic 소비자는 `ScalarHomotopy(RhsAcceleration::Picard)`를 보수적인 실행 선택으로,
`RationalPredictor`를 별도 검증된 대안으로 제공한다. 두 경로는 같은 정확도 계획과
원래 outward 검증을 사용한다. Rational은
`(1+2 beta)/(1+3 beta)`라는 유도식이며, 별도 cube-root 연산을 요구하지 않는다.
같은 scalar 경로에 대한 시간 우위는 사례별로 달라 자동 dimension/step threshold를
추가하지 않았다. 원래 generic homotopy API는 유지된다.

Mean-field는 `d>=kappa>=0`인 불변 모델의 모든 state/forcing 성분을 보존한다.
새 factorized enclosure는 원래 모든 stage 방정식의 잔차를 포함한다. 실패한
병렬 작업도 모두 회수하여 과금한다. `n=1`은 정확히 `P=I`인 대수적 별도식으로
평가하며, 경험적으로 맞춘 작은-n 경계가 아니다. 반환 residual limit은 호출자의
오차 계약에서 주며, 고정 h의 algebraic 예산에는 PR91의 `epsilon_alg/C_N` 유도를
재사용할 수 있다. 현재 ordinary-f64 `C_N` 계산과 outward stage enclosure의
지위는 구분한다.

## 6. 로컬 검증과 실행 자원

`run_local.py`는 기본적으로 실행할 argv를 보여준다. `--execute`는 명시한 suite를
하나씩 실행하고 각 stdout/stderr, source hash, 종료 코드와 wall time을 보존한다.
현재 runner는 POSIX/WSL의 flock을 사용한다. Windows native에서는 JSON의 argv를
동등한 격리 실행기로 수행하고 같은 receipt 필드를 반환한다.

```bash
python3 research/production_solver_closure_20261027/run_local.py \
  --suite release-contracts --output /tmp/vigilode-local-release --execute --timeout 1800
python3 research/production_solver_closure_20261027/run_local.py \
  --suite docs --output /tmp/vigilode-local-docs --execute
python3 research/production_solver_closure_20261027/run_local.py \
  --suite smoke-release --output /tmp/vigilode-local-smoke --execute
```

빌드·timing·profiling을 서로 겹치지 않는다. 환경과 resource cap은 실제 호스트에
맞춰 명시하고, 기존 사용자의 계산을 종료하지 않는다. 로컬 hardware 비교는
`DEVELOPMENT_PLAN.json` L-HARDWARE와 각 lane의 재현 명령을 따른다.
`run_instruction_profile.py`는 두 반복 횟수의 Ir 차분으로 startup을 제거한다.
그 값은 guest instruction이지 CPU cycle이나 실제 시간의 대체가 아니다.

## 7. 완료와 남은 release 판단

이번 공개 결과는 설정·구현·변경 의존성의 cloud 검증과 짧은 benchmark/profile을
제공한다. Actual deployment host의 release/LTO 실행, 실제 client 입력,
장시간 통합·재시작과 전체 repository gate는 L-RELEASE/L-HARDWARE/R-QUALIFY에서
확인한다. 이 상태를 `DEVELOPMENT_PLAN.json`에 `pending_local`로 표시한다.
확인되지 않은 전체 프로젝트 production 인증을 게시 완료와 혼동하지 않는다.
