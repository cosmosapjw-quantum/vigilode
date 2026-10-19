#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
STAGE=baseline_red
trap 's=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$s" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal > "$OUT/toolchain_install.log" 2>&1
rustc +1.94.1 -Vv > "$OUT/rustc.txt"
set +e
cargo +1.94.1 test -p rodas5p-integrators --test banded_recovery_contract --locked -- --test-threads=1 > "$OUT/BASELINE_RED.log" 2>&1
s=$?
set -e
printf '{"test_exit_code":%s,"source_commit":"%s"}\n' "$s" "$(git rev-parse HEAD)" > "$OUT/BASELINE_STATUS.json"
tail -100 "$OUT/BASELINE_RED.log"
[[ "$s" == 101 ]]
grep -q 'test result: FAILED' "$OUT/BASELINE_RED.log"
grep -q 'failed_banded_rhs_rebuilds_snapshot.*FAILED' "$OUT/BASELINE_RED.log"
STAGE=expected_red_recorded
