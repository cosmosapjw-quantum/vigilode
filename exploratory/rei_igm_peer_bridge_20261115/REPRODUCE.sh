#!/usr/bin/env bash
# Original full 46,080-cell selected-closure point-IVP cross-branch experiment.
# OWNER must be the immutable REI source checkout at f1b20b13; no source mutations.
# Requires the checkout's Cargo.lock and a compatible NumPy/SciPy Python runtime.
# Usage: REPRODUCE.sh ORIGINAL_REI_OWNER_WORKTREE NEW_OUTPUT_DIRECTORY
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo 'usage: REPRODUCE.sh ORIGINAL_REI_OWNER_WORKTREE NEW_OUTPUT_DIRECTORY' >&2
  exit 2
fi
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCI="$ROOT/exploratory/rei_igm_peer_bridge_20261115"
OWNER="$(realpath "$1")"
mkdir -p "$2"
OUT="$(realpath "$2")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$OUT/target}"
export OPENBLAS_NUM_THREADS=1
export OMP_NUM_THREADS=1
export NUMEXPR_MAX_THREADS=1
python3 "$SCI/export_peer_source.py" --owner "$OWNER" --out "$OUT/source" \
  > "$OUT/EXPORT.log" 2> "$OUT/EXPORT.err"
python3 "$SCI/independent_owner_reference.py" --owner "$OWNER" \
  --source "$OUT/source" --out "$OUT/independent" \
  > "$OUT/OWNER_REFERENCE.log" 2> "$OUT/OWNER_REFERENCE.err"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
  -p rodas5p-integrators --example rei_original46k_peer_w > "$OUT/TESTS.log" 2>&1
cargo build --manifest-path "$ROOT/Cargo.toml" --offline --locked --release \
  -p rodas5p-integrators --example rei_original46k_peer_w > "$OUT/BUILD.log" 2>&1
BIN="$CARGO_TARGET_DIR/release/examples/rei_original46k_peer_w"
# Keep the absolute-state negative result separate from the centered repair.
REI_PEER_FORM=absolute "$BIN" "$OUT/source/SOURCE.json" "$OUT/source/SOURCE_POINT.raw" \
  "$OUT/source/OWNER_RHS_ORACLE.raw" "$OUT/run1" > "$OUT/ABSOLUTE.log" 2> "$OUT/ABSOLUTE.err"
REI_PEER_FORM=centered "$BIN" "$OUT/source/SOURCE.json" "$OUT/source/SOURCE_POINT.raw" \
  "$OUT/source/OWNER_RHS_ORACLE.raw" "$OUT/run2_centered" > "$OUT/CENTERED.log" 2> "$OUT/CENTERED.err"
for RUN in run1 run2_centered; do
  python3 "$SCI/audit_full_ivp.py" --source "$OUT/source" --runs "$OUT/$RUN" \
    --independent "$OUT/independent" --out "$OUT/$RUN/INDEPENDENT_AUDIT.json"
done
REI_PEER_FORM=centered REI_PEER_BENCH=paired "$BIN" \
  "$OUT/source/SOURCE.json" "$OUT/source/SOURCE_POINT.raw" \
  "$OUT/source/OWNER_RHS_ORACLE.raw" "$OUT/paired" > "$OUT/PAIRED.log" 2> "$OUT/PAIRED.err"
PEER_EVIDENCE_ROOT="$OUT" python3 "$SCI/test_audit_contract.py"
PEER_EVIDENCE_ROOT="$OUT" python3 -O "$SCI/test_audit_contract.py"
python3 "$SCI/negative_controls.py" --source "$OUT/source" --runs "$OUT/run2_centered" \
  --independent "$OUT/independent" --binary "$BIN" --out "$OUT/NEGATIVE_CONTROLS.json"
echo 'SOURCE-SCOPED POINT EXPERIMENT COMPLETE; NOT NATIVE INTERVAL-14 OR AXBI HISTORY ACCEPTANCE.'
