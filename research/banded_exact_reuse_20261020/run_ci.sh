#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
ROOT=$(pwd)
NODE="$ROOT/research/banded_exact_reuse_20261020"
PARENT=b56bb19ce85bc28a021138ee29689701a81bc4f0
STAGE=setup
trap 's=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$s" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal --component rustfmt > "$OUT/toolchain_install.log" 2>&1
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; python3 -m pip freeze; } > "$OUT/VERSIONS.txt"
STAGE=baseline_build
# The prior RED run is the executed unmodified-parent regression receipt.
grep -q '5 passed; 4 failed' "$NODE/runs/39298620202/BASELINE_RED.log"
git fetch --depth=1 origin "$PARENT"
BASE="$RUNNER_TEMP/banded-baseline"
git worktree add --detach "$BASE" "$PARENT"
mkdir -p "$BASE/research/banded_exact_reuse_20261020"
cp -r "$NODE/bench" "$BASE/research/banded_exact_reuse_20261020/bench"
BASEMAN="$BASE/research/banded_exact_reuse_20261020/bench/Cargo.toml"
export CARGO_TARGET_DIR="$RUNNER_TEMP/banded-target"
cargo +1.94.1 generate-lockfile --manifest-path "$BASEMAN" > "$OUT/lock_resolution.log" 2>&1
cp "$(dirname "$BASEMAN")/Cargo.lock" "$NODE/bench/Cargo.lock"
cp "$NODE/bench/Cargo.lock" "$OUT/BENCH_Cargo.lock"
cargo +1.94.1 build --manifest-path "$BASEMAN" --profile measurement --locked -j2 > "$OUT/baseline_build.log" 2>&1
cp "$CARGO_TARGET_DIR/measurement/band-study" "$RUNNER_TEMP/parent-band-study"
sha256sum "$RUNNER_TEMP/parent-band-study" > "$OUT/PARENT_BINARY_SHA256"
STAGE=source_change
if [[ "$(git hash-object crates/rodas5p-integrators/src/rodas5p_fast.rs)" == 7756df18367b84813f7d314a27d72b45c80b6a0d ]]; then
  python3 "$NODE/scripts/apply.py" "$OUT"
  rustfmt +1.94.1 --edition 2024 crates/rodas5p-integrators/src/rodas5p_fast.rs
  git diff --stat > "$OUT/production_patch_stat.txt"
  git config user.name 'github-actions[bot]'
  git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
  git add crates/rodas5p-integrators/src/rodas5p_fast.rs crates/rodas5p-integrators/src/exact_band_cache.rs crates/rodas5p-integrators/src/banded_exact_tests.rs
  git commit -m 'fix: reject nonfinite band solves; add opt-in fixed-width exact factor reuse [skip ci]'
  git push origin HEAD:refs/heads/research/banded-exact-reuse-20261020
else
  grep -q FixedWidth2ExactCache crates/rodas5p-integrators/src/rodas5p_fast.rs
fi
SOURCE=$(git rev-parse HEAD)
printf '%s\n' "$SOURCE" > "$OUT/EXEC_SOURCE_COMMIT.txt"
git diff "$PARENT" -- crates/rodas5p-integrators/src/rodas5p_fast.rs > "$OUT/production.patch"
STAGE=scoped_tests
cargo +1.94.1 test -p rodas5p-integrators --test banded_recovery_contract --locked -- --test-threads=1 > "$OUT/GREEN_REGRESSION.log" 2>&1
cargo +1.94.1 test -p rodas5p-integrators --lib rodas5p_fast --locked -- --test-threads=1 > "$OUT/INTERNAL_TESTS.log" 2>&1
STAGE=workspace_tests
set +e
timeout 660 cargo +1.94.1 test --workspace --no-fail-fast --locked -- --test-threads=2 > "$OUT/WORKSPACE_TESTS.log" 2>&1
WS=$?
set -e
printf '{"exit_code":%s,"timeout_seconds":660,"command":"cargo +1.94.1 test --workspace --no-fail-fast --locked -- --test-threads=2"}\n' "$WS" > "$OUT/WORKSPACE_STATUS.json"
STAGE=candidate_build
cargo +1.94.1 build --manifest-path "$NODE/bench/Cargo.toml" --profile measurement --locked --features improvement -j2 > "$OUT/candidate_build.log" 2>&1
cp "$CARGO_TARGET_DIR/measurement/band-study" "$RUNNER_TEMP/candidate-band-study"
sha256sum "$RUNNER_TEMP/candidate-band-study" > "$OUT/CANDIDATE_BINARY_SHA256"
STAGE=accuracy_then_timing
CPU=$(python3 -c 'import os;print(min(os.sched_getaffinity(0)))')
taskset -c "$CPU" python3 "$NODE/scripts/study.py" --parent "$RUNNER_TEMP/parent-band-study" --candidate "$RUNNER_TEMP/candidate-band-study" --parent-source "$PARENT" --candidate-source "$SOURCE" --out "$OUT" 2>&1 | tee "$OUT/study.log"
STAGE=readback
python3 "$NODE/scripts/audit.py" "$OUT" 2>&1 | tee "$OUT/audit.log"
STAGE=complete
