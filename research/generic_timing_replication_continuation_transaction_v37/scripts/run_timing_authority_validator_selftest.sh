#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
CONTRACT="$REPO_ROOT/research/generic_timing_replication_continuation_transaction_v37/contracts/V37_TIMING_REPLICATION_CONTINUATION_TRANSACTION_CONTRACT.json"
V36_ECONOMICS="$REPO_ROOT/research/generic_frozen_full_e_shadow_v36/results/economics"

cd "$SCRIPT_DIR"
python -m unittest test_timing_authority_validator -v

cd "$REPO_ROOT"
python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_shadow_economics.py
python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_full_e_shadow_ledger.py
python -m py_compile \
  research/generic_timing_replication_continuation_transaction_v37/scripts/timing_authority_validator.py \
  research/generic_timing_replication_continuation_transaction_v37/scripts/test_timing_authority_validator.py

TMPDIR_SELFTEST="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SELFTEST"' EXIT
for run in 1 2; do
  python research/generic_timing_replication_continuation_transaction_v37/scripts/timing_authority_validator.py \
    retrospective-v36 \
    --contract "$CONTRACT" \
    --economics-root "$V36_ECONOMICS" \
    --output "$TMPDIR_SELFTEST/diagnostic-$run.json"
done
cmp "$TMPDIR_SELFTEST/diagnostic-1.json" "$TMPDIR_SELFTEST/diagnostic-2.json"

if find research/generic_timing_replication_continuation_transaction_v37 -type d -name 'attempt-*' -print -quit | grep -q .; then
  echo "unexpected real timing attempt directory" >&2
  exit 1
fi

echo "PASS: timing-authority validator selftest; no real wall campaign created"
