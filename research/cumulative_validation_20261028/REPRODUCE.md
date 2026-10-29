# 재현 절차

작업 디렉터리 `/tmp/vigilode-pr90-94-validation.SkqyxWxw/checkout`, compact evidence는 `research/cumulative_validation_20261028/evidence/`, frozen contract는 repository-relative `research/integrated_polyalgorithm_20261028/INDEPENDENT_VALIDATION.json`이다. 원시 evidence는 `/tmp/vigilode-cumulative-validation-20261028/evidence`에 있다. `CARGO_TARGET_DIR=/tmp/vigilode-cumulative-validation-20261028/target-candidate`, `CARGO_BUILD_JOBS=2`, `RAYON_NUM_THREADS=OMP_NUM_THREADS=OPENBLAS_NUM_THREADS=1`로 실행했고, 직접 Cargo 명령은 `/tmp/vigilode-cargo-20261027.lock`으로 직렬화했다. 원시 결과 디렉터리는 create-only이므로 재실행에는 새 경로를 지정한다.

핵심 명령:

```sh
cargo test --profile measurement --locked --offline -p vigilode --lib --tests -- --test-threads=1
cargo test --profile measurement --locked --offline -p vigilode-coupled --lib --tests -- --test-threads=1
cargo check --profile measurement --locked --offline -p vigilode -p vigilode-coupled --all-targets
cargo test --release --locked --offline -p vigilode -p vigilode-coupled -p rodas5p-core -p rodas5p-krylov --lib --tests -- --test-threads=1
cargo doc --profile measurement --locked --offline -p vigilode -p vigilode-coupled --no-deps
```

나머지 native suite의 정확한 argv, exit code, stdout/stderr는 각 `final-suites/<suite>/SUITE_RECEIPT.json` 및 command 디렉터리에 있다. smoke는 `python3 research/production_solver_closure_20261027/run_smoke.py --profile release --output <새 출력 디렉터리>`이며, local-benchmarks/profile은 해당 suite receipt의 argv를 사용한다. 최초 integrators release의 실패 후 추가로 실행한 명령은 다음과 같다.

```sh
python3 research/production_solver_closure_20261027/record_command.py --output /tmp/vigilode-cumulative-validation-20261028/evidence/final-suites/release-integrators-extension --timeout 3600 --input research/production_solver_closure_20261027/DEVELOPMENT_PLAN.json -- cargo test --release --locked --offline -p rodas5p-integrators --lib --tests -- --test-threads=1
```

rapid 비교는 측정 바이너리 SHA256 `2d741d8fd9563e90f4bc28e9097b5b6dd261044527cc9eb2e2e5da5aaf5f1053`, CPU 0, 스레드 1에서 실행했다. 첫 `1e-6` 코호트는 공통 달성 오차 상한을 넘었고, 두 번째 `1e-8` 코호트는 통과했다.

```sh
cargo build -p vigilode --example adaptive_causal_bench --profile measurement --locked --offline
python3 research/adaptive_causal_20261023/src/run_adaptive_bench.py --binary /tmp/vigilode-cumulative-validation-20261028/target-candidate/measurement/examples/adaptive_causal_bench --source-commit 5c57bf146f2f5faf69f711f133a83fcb265cee90 --out /tmp/vigilode-cumulative-validation-20261028/evidence/rapid-matched --cases rapid --tolerances 1e-6 --modes switch-rhs,dopri-whole --repeats 3 --budget-seconds 900 --timeout-seconds 30 --cpu 0
python3 research/adaptive_causal_20261023/src/analyze_adaptive_bench.py /tmp/vigilode-cumulative-validation-20261028/evidence/rapid-matched
python3 research/adaptive_causal_20261023/src/run_adaptive_bench.py --binary /tmp/vigilode-cumulative-validation-20261028/target-candidate/measurement/examples/adaptive_causal_bench --source-commit 5c57bf146f2f5faf69f711f133a83fcb265cee90 --out /tmp/vigilode-cumulative-validation-20261028/evidence/rapid-matched-tight --cases rapid --tolerances 1e-8 --modes switch-rhs,dopri-whole --repeats 3 --budget-seconds 900 --timeout-seconds 30 --cpu 0
python3 research/adaptive_causal_20261023/src/analyze_adaptive_bench.py /tmp/vigilode-cumulative-validation-20261028/evidence/rapid-matched-tight
python3 research/production_solver_closure_20261027/run_instruction_profile.py --binary /tmp/vigilode-cumulative-validation-20261028/target-candidate/measurement/examples/adaptive_causal_bench --output /tmp/vigilode-cumulative-validation-20261028/evidence/rapid-profile --arms switch-rhs dopri-whole --repetitions 16 32 --valgrind /usr/bin/valgrind --timeout 300 -- --case rapid --mode '{arm}' --dimension 2 --rtol 1e-8 --windows '{repetitions}' --source-commit 5c57bf146f2f5faf69f711f133a83fcb265cee90
```

Q-BENCH adapter 시험은 `cargo test --manifest-path crates/vigilode-bench/Cargo.toml --locked --offline --test adapters -- --test-threads=1`, Q-NEGATIVE 재현은 `cargo test -p rodas5p-integrators --profile measurement --locked --offline --test dense_wfiltered_contracts prothero_robinson_wfiltered_enforce_kill_gate -- --ignored --exact --nocapture --test-threads=1`이었다. 후자는 등록된 비용 게이트 실패로 exit 101이다. 원본 exact check는 별도 delivery/PR90/PR91/PR93 source worktree에서 실행했으며 외부 evidence root의 `historical-*`에 있다. 수정된 tree에서 통합 artifact verifier가 원본-source 제한으로 거절되는 것은 예상된 결과다. 원시 로그와 callgrind는 외부 evidence root에만 둔다.

주의: integrators release는 최초 timeout receipt와 연장 receipt를 별도로 보존하며 double count하지 않는다. benchmark/profile exit 0은 qualification이 아니다. `evidence/RELEASE_BASELINE_FAILURE.txt`, `evidence/AUTHORITY_FAILURE_SUMMARY.txt`, `evidence/Q_NEGATIVE_FAILURE.txt`, `evidence/GENERATED_OVERWRITES.json`, `evidence/REVIEW_DECISIONS.json`이 실패·분류·검토 결정을 요약한다. 10 interactions와 PR-N01..N06은 이 packet에서 실행되지 않았다.
