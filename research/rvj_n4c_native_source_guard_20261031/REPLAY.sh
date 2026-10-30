#!/usr/bin/env bash
set -euo pipefail
# Must be run with EXACT offline workspace zip and user-supplied Rust1.99.1 on PATH.
# Original source remains in a newly extracted workspace, never edited in-place.
if [[ $# -lt 1 ]]; then echo 'Usage: bash REPLAY.sh /path/to/vigilode-offline-install(2).zip [target-dir]' >&2; exit 2; fi
ORIGINAL_ZIP="$1"
OUT="${2:-$(mktemp -d -t n4c_native.XXXXXX)}"
mkdir -p "$OUT"
python3 VERIFY.py
sha256sum "$ORIGINAL_ZIP" | grep -q '^f8caea8ac3435dfde6636947bd2618f659ef9930e3a6910148972a96fc2e6045  '
unzip -q "$ORIGINAL_ZIP" -d "$OUT"
cp rust/tests/n4c_native_rank2_source_owned.rs "$OUT/vigilode/crates/rodas5p-integrators/tests/"
cp rust/tests/n4c_original_tableau.rs "$OUT/vigilode/crates/rodas5p-integrators/tests/"
(cd "$OUT/vigilode"; CARGO_TARGET_DIR="$OUT/target" cargo test --locked --offline -p rodas5p-integrators --test n4c_native_rank2_source_owned -- --nocapture)
echo 'N4C_SOURCE_SPECIFIC_NATIVE_TEST_PASS; production hold.'
