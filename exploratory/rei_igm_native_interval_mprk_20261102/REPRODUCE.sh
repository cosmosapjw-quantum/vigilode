#!/usr/bin/env bash
# Original REI 46,080-cell full/half1/half2 MPRK22 corrector interval science.
# Usage: REPRODUCE.sh EXACT_REI_OWNER_WORKTREE ISOLATED_OUTPUT_DIR
# Original REI source/worker unchanged. Draft-only, nonproduction.
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo "USAGE: REPRODUCE.sh EXACT_REI_OWNER_WORKTREE ISOLATED_OUTPUT_DIR" >&2
  exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCI="$ROOT/exploratory/rei_igm_native_interval_mprk_20261102"
OWNER="$(realpath "$1")"
mkdir -p "$2"
OUT="$(realpath "$2")"
export OPENBLAS_NUM_THREADS="${OPENBLAS_NUM_THREADS:-4}"
export NUMEXPR_MAX_THREADS="${NUMEXPR_MAX_THREADS:-4}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$OUT/cargo-target}"
python3 "$SCI/export_owner_corrector.py" \
  --owner "$OWNER" \
  --out "$OUT/ORIGINAL_46080_X3_MPRK_CORRECTOR_INTERVAL.raw" \
  --receipt "$OUT/ORIGINAL_CORRECTOR_SOURCE_MANIFEST.json" \
  --owner_out "$OUT/owner_original14" \
  > "$OUT/ORIGINAL_EXPORT.log" 2> "$OUT/ORIGINAL_EXPORT_STDERR.log"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
  -p rodas5p-integrators --example rei_original46k_interval_mprk \
  > "$OUT/NATIVE_TESTS.log" 2>&1
cargo run --manifest-path "$ROOT/Cargo.toml" --offline --locked --release \
  -p rodas5p-integrators --example rei_original46k_interval_mprk -- \
  "$OUT/ORIGINAL_46080_X3_MPRK_CORRECTOR_INTERVAL.raw" \
  "$OUT/ORIGINAL_CORRECTOR_SOURCE_MANIFEST.json" \
  "$OUT/NATIVE_ORIGINAL_46K_X3_MPRK_INCREMENT_INTERVAL.raw" \
  > "$OUT/NATIVE_ORIGINAL_46K_MPRK_RESULT.json" \
  2> "$OUT/NATIVE_RUN_STDERR.log"
python3 "$SCI/audit_native_interval_corrector.py" \
  --source "$OUT/ORIGINAL_46080_X3_MPRK_CORRECTOR_INTERVAL.raw" \
  --native "$OUT/NATIVE_ORIGINAL_46K_X3_MPRK_INCREMENT_INTERVAL.raw" \
  --receipt "$OUT/NATIVE_ORIGINAL_46K_MPRK_RESULT.json" \
  --owner_csv "$OUT/owner_original14/REI_ORIGINAL26_COMMON_PARENT_INCREMENT_46080.csv.gz" \
  --owner_receipt "$OUT/owner_original14/RESULT.json" \
  --out "$OUT/INDEPENDENT_NATIVE_INTERVAL_AUDIT.json"
python3 "$SCI/negative_controls.py" \
  --binary "$CARGO_TARGET_DIR/release/examples/rei_original46k_interval_mprk" \
  --source "$OUT/ORIGINAL_46080_X3_MPRK_CORRECTOR_INTERVAL.raw" \
  --manifest "$OUT/ORIGINAL_CORRECTOR_SOURCE_MANIFEST.json" \
  --output "$OUT/NATIVE_ORIGINAL_46K_X3_MPRK_INCREMENT_INTERVAL.raw" \
  --out "$OUT/NEGATIVE_CONTROLS.json"
sha256sum "$OUT/ORIGINAL_46080_X3_MPRK_CORRECTOR_INTERVAL.raw" \
  "$OUT/NATIVE_ORIGINAL_46K_X3_MPRK_INCREMENT_INTERVAL.raw" \
  "$OUT/INDEPENDENT_NATIVE_INTERVAL_AUDIT.json" \
  "$OUT/owner_original14/REI_ORIGINAL26_COMMON_PARENT_INCREMENT_46080.csv.gz"
echo "SOURCE_CORRECTOR_INTERVAL_ONLY: PASS. OWNER_TICK161=NOT_ACCEPTED. PRODUCTION=HOLD."
