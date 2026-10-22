# 재현 방법

Rust 1.94.1과 `Cargo.lock`에 고정된 의존성이 필요하다. 이 실행은 NCP나 SSH를 사용하지 않는다. offline cache가 없으면 별도 환경 준비 단계에서 locked dependencies를 확보한다. lockfile을 바꾸어 실행 결과를 맞추지 않는다.

```bash
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
python3 research/coupled_stage_adoption_20261022/run_targeted_checks.py --output /tmp/vigilode-coupled-checks-new
cargo build --locked --offline --profile measurement -p vigilode-coupled --example adaptive_bench
python3 research/coupled_stage_adoption_20261022/run_local_bench.py \
  --binary target/measurement/examples/adaptive_bench \
  --out /tmp/vigilode-coupled-bench-new \
  --source-commit "$(git rev-parse HEAD)" --budget-seconds 900
python3 research/coupled_stage_adoption_20261022/analyze_local_bench.py --help
```

`CARGO_TARGET_DIR`를 사용하면 `--binary`를 해당 경로로 바꾼다. 출력 디렉터리는 새 경로여야 한다. 기존 실패 기록을 덮어쓰지 않는다. 실행 시간은 같은 물리 상태·관측 시각·실제 달성 오차를 만족한 설정에서만 비교한다. 후보 경로와 순차 raw-U 경로 사이의 비교이며, VODE/DVODE·diffsol·Julia에 대한 새로운 실행 결과는 아니다.

새로운 API 예제는 `crates/vigilode-coupled/examples/adaptive_bench.rs`이다. 기존 `coupled_or_sequential`의 native-K fallback과 새 `RawStepSession`의 raw-U fallback은 서로 다른 계약이다. 실사용 이식에는 `integrate_coupled_observed`와 명시적인 `RootErrorPolicy`를 사용한다. 관측 시각에는 실제 착지하며 dense interpolation은 제공하지 않는다.
