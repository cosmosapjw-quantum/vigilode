#!/usr/bin/env bash
# Reproduce one read-only original REI F05 46,080×3×3 thermal source interval unit.
# Usage: REPRODUCE.sh EXACT_REI_SOURCE_WORKTREE ISOLATED_OUTPUT_DIR
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo "usage: REPRODUCE.sh EXACT_REI_SOURCE_WORKTREE ISOLATED_OUTPUT_DIR" >&2
  exit 2
fi
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCI="$ROOT/exploratory/rei_igm_native_thermal_interval_20261102"
OWNER="$(realpath "$1")"
mkdir -p "$2"
OUT="$(realpath "$2")"
export OPENBLAS_NUM_THREADS=4
export NUMEXPR_MAX_THREADS=4
export RAYON_NUM_THREADS=8
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$OUT/target}"
python3 "$SCI/export_owner_thermal_roots.py" \
  --owner "$OWNER" \
  --out "$OUT/ORIGINAL_46080_X9_THERMAL_ROOT_CONTEXT.raw" \
  --receipt "$OUT/ORIGINAL_THERMAL_SOURCE_MANIFEST.json" \
  --owner_out "$OUT/ORIGINAL_OWNER_FINAL14" \
  > "$OUT/OWNER_EXPORT.log" 2> "$OUT/OWNER_EXPORT_STDERR.log"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
  -p rodas5p-integrators --example rei_original46k_native_thermal_roots \
  > "$OUT/NATIVE_TESTS.log" 2>&1
cargo run --manifest-path "$ROOT/Cargo.toml" --offline --locked --release \
  -p rodas5p-integrators --example rei_original46k_native_thermal_roots -- \
  "$OUT/ORIGINAL_46080_X9_THERMAL_ROOT_CONTEXT.raw" \
  "$OUT/ORIGINAL_THERMAL_SOURCE_MANIFEST.json" \
  "$OUT/NATIVE_46K_X9_VERIFIED_THERMAL_ROOTS.raw" \
  > "$OUT/RUST_NATIVE_THERMAL_SCIENCE.json" 2> "$OUT/NATIVE_RUN_STDERR.log"
python3 "$SCI/audit_native_thermal_roots.py" \
  --source "$OUT/ORIGINAL_46080_X9_THERMAL_ROOT_CONTEXT.raw" \
  --rust "$OUT/NATIVE_46K_X9_VERIFIED_THERMAL_ROOTS.raw" \
  --receipt "$OUT/RUST_NATIVE_THERMAL_SCIENCE.json" \
  --owner_manifest "$OUT/ORIGINAL_THERMAL_SOURCE_MANIFEST.json" \
  --out "$OUT/INDEPENDENT_ROOT_AUDIT.json"
cargo run --manifest-path "$ROOT/Cargo.toml" --offline --locked --release \
  -p rodas5p-integrators --example rei_original46k_thermal_energy_ledger -- \
  "$OUT/ORIGINAL_46080_X9_THERMAL_ROOT_CONTEXT.raw" \
  "$OUT/NATIVE_46K_X9_VERIFIED_THERMAL_ROOTS.raw" \
  > "$OUT/NATIVE_THERMAL_ENERGY_LEDGER.json" 2> "$OUT/ENERGY_RUN_STDERR.log"
sha256sum "$OUT/ORIGINAL_46080_X9_THERMAL_ROOT_CONTEXT.raw" \
  "$OUT/NATIVE_46K_X9_VERIFIED_THERMAL_ROOTS.raw" \
  "$OUT/INDEPENDENT_ROOT_AUDIT.json" \
  "$OUT/NATIVE_THERMAL_ENERGY_LEDGER.json"
echo "REI_OWNER_V1_OFFICIAL_TICK=160/131072. RELEASE=HOLD."
