#!/usr/bin/env bash
set -uo pipefail
runroot=/home/cosmosapjw/vigilode-nonstiff-run-Ioh017
source_tree=/home/cosmosapjw/vigilode-nonstiff-source-WjxDXM
runtime=/workspace/scratch/0dc1e721cf82/task_runtime
if [[ -x "$runtime/installed/bin/rustc" ]]; then export PATH="$runtime/installed/bin:$PATH"; fi
if [[ -d "$runtime/cargo" ]]; then export CARGO_HOME="$runtime/cargo"; fi
export CARGO_TARGET_DIR="$runroot/build"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export PYTHONDONTWRITEBYTECODE=1
export MPLCONFIGDIR="$runroot/matplotlib-cache"
cd "$source_tree" || exit 99
printf '%s\n' 'CARGO_TARGET_DIR=/home/cosmosapjw/vigilode-nonstiff-run-Ioh017/build' 'CARGO_BUILD_JOBS=2' 'CARGO_PROFILE_DEV_DEBUG=0' 'CARGO_PROFILE_TEST_DEBUG=0' "CARGO_HOME=${CARGO_HOME:-UNSET}" > "$runroot/validation-environment.txt"
rustc --version > "$runroot/rustc-version.txt" 2> "$runroot/rustc-version.stderr.log"; printf '%s\n' "$?" > "$runroot/rustc-version.exit.txt"
cargo --version > "$runroot/cargo-version.txt" 2> "$runroot/cargo-version.stderr.log"; printf '%s\n' "$?" > "$runroot/cargo-version.exit.txt"
if [[ "$(cat "$runroot/rustc-version.txt")" != 'rustc 1.94.1 '* ]]; then exit 98; fi
date -u +%FT%TZ > "$runroot/cargo-metadata.started.txt"
cargo metadata --locked --offline --format-version 1 > "$runroot/cargo-metadata.json" 2> "$runroot/cargo-metadata.stderr.log"
rc=$?; printf '%s\n' "$rc" > "$runroot/cargo-metadata.exit.txt"; date -u +%FT%TZ > "$runroot/cargo-metadata.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
date -u +%FT%TZ > "$runroot/tests-kernel.started.txt"
cargo test --locked --offline -p rodas5p-integrators --test dopri5_contracts --test dopri5_allocation > "$runroot/tests-kernel.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$runroot/tests-kernel.exit.txt"; date -u +%FT%TZ > "$runroot/tests-kernel.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
date -u +%FT%TZ > "$runroot/tests-public-ensemble.started.txt"
cargo test --locked --offline -p vigilode --test explicit_contracts --test explicit_ensemble_contracts > "$runroot/tests-public-ensemble.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$runroot/tests-public-ensemble.exit.txt"; date -u +%FT%TZ > "$runroot/tests-public-ensemble.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
date -u +%FT%TZ > "$runroot/build-measurement.started.txt"
cargo build --locked --offline --profile measurement -p vigilode --example nonstiff_bench --example nonstiff_ensemble_bench > "$runroot/build-measurement.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$runroot/build-measurement.exit.txt"; date -u +%FT%TZ > "$runroot/build-measurement.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
sha256sum "$runroot/build/measurement/examples/nonstiff_bench" "$runroot/build/measurement/examples/nonstiff_ensemble_bench" > "$runroot/binary-sha256.txt"
sha256sum "$runroot/cargo-metadata.json" "$runroot/tests-kernel.log" "$runroot/tests-public-ensemble.log" "$runroot/build-measurement.log" > "$runroot/validation-artifact-sha256.txt"
