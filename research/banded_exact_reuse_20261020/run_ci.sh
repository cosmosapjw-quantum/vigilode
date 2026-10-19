#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
ROOT=$(pwd)
NODE="$ROOT/research/banded_exact_reuse_20261020"
PARENT=b56bb19ce85bc28a021138ee29689701a81bc4f0
PRODUCTION=b51c051b045e977c038e5eec468de0004840872b
STAGE=setup
trap 's=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$s" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal > "$OUT/toolchain_install.log" 2>&1
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; python3 -m pip freeze; } > "$OUT/VERSIONS.txt"
STAGE=baseline_build
git fetch --depth=1 origin "$PARENT" "$PRODUCTION"
git diff --exit-code "$PRODUCTION" -- crates Cargo.toml Cargo.lock > "$OUT/PRODUCTION_UNCHANGED.patch"
BASE="$RUNNER_TEMP/banded-baseline"
git worktree add --detach "$BASE" "$PARENT"
mkdir -p "$BASE/research/banded_exact_reuse_20261020"
cp -r "$NODE/bench" "$BASE/research/banded_exact_reuse_20261020/bench"
BASEMAN="$BASE/research/banded_exact_reuse_20261020/bench/Cargo.toml"
export CARGO_TARGET_DIR="$RUNNER_TEMP/banded-target"
cp "$NODE/runs/39301232025/BENCH_Cargo.lock" "$(dirname "$BASEMAN")/Cargo.lock"
cp "$(dirname "$BASEMAN")/Cargo.lock" "$NODE/bench/Cargo.lock"
cp "$NODE/bench/Cargo.lock" "$OUT/BENCH_Cargo.lock"
cargo +1.94.1 build --manifest-path "$BASEMAN" --profile measurement --locked -j2 > "$OUT/baseline_build.log" 2>&1
cp "$CARGO_TARGET_DIR/measurement/band-study" "$RUNNER_TEMP/parent-band-study"
sha256sum "$RUNNER_TEMP/parent-band-study" > "$OUT/PARENT_BINARY_SHA256"
SOURCE=$(git rev-parse HEAD)
printf '%s\n' "$SOURCE" > "$OUT/EXEC_SOURCE_COMMIT.txt"
STAGE=scoped_tests
cargo +1.94.1 test -p rodas5p-integrators --test banded_recovery_contract --locked -- --test-threads=1 > "$OUT/GREEN_REGRESSION.log" 2>&1
cargo +1.94.1 test -p rodas5p-integrators --lib rodas5p_fast --locked -- --test-threads=1 > "$OUT/INTERNAL_TESTS.log" 2>&1
cp "$NODE/runs/39301232025/WORKSPACE_STATUS.json" "$OUT/WORKSPACE_STATUS.json"
cp "$NODE/runs/39301232025/WORKSPACE_TESTS.log" "$OUT/WORKSPACE_TESTS.log"
printf '{"source_run":39301232025,"numerical_source":"%s","relation":"BYTE_IDENTITY of crates and root Cargo files","status":"TIMEOUT124_PRESERVED_NOT_RERUN"}\n' "$PRODUCTION" > "$OUT/WORKSPACE_REUSE.json"
STAGE=candidate_build
cargo +1.94.1 build --manifest-path "$NODE/bench/Cargo.toml" --profile measurement --locked --features improvement -j2 > "$OUT/candidate_build.log" 2>&1
cp "$CARGO_TARGET_DIR/measurement/band-study" "$RUNNER_TEMP/candidate-band-study"
sha256sum "$RUNNER_TEMP/candidate-band-study" > "$OUT/CANDIDATE_BINARY_SHA256"
STAGE=adapter_smoke
python3 "$NODE/scripts/smoke.py" "$RUNNER_TEMP/parent-band-study" "$RUNNER_TEMP/candidate-band-study" "$OUT" 2>&1 | tee "$OUT/smoke.log"
STAGE=accuracy_then_timing
CPU=$(python3 -c 'import os;print(min(os.sched_getaffinity(0)))')
taskset -c "$CPU" python3 "$NODE/scripts/study.py" --parent "$RUNNER_TEMP/parent-band-study" --candidate "$RUNNER_TEMP/candidate-band-study" --parent-source "$PARENT" --candidate-source "$SOURCE" --out "$OUT" 2>&1 | tee "$OUT/study.log"
STAGE=readback
python3 "$NODE/scripts/audit.py" "$OUT" 2>&1 | tee "$OUT/audit.log"
STAGE=complete
