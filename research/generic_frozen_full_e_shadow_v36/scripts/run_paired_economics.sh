#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 MEASUREMENT_BINARY OUTPUT_ROOT" >&2
  exit 2
fi

binary=$1
output_root=$2
profiles=(calibration96 calibration192 calibration256 holdout320 holdout384)

if [[ ! -x "$binary" ]]; then
  echo "measurement binary is not executable: $binary" >&2
  exit 2
fi

mkdir -p "$output_root"
for profile in "${profiles[@]}"; do
  "$binary" generic-frozen-full-e-shadow-economics \
    --profile "$profile" \
    --output "$output_root/$profile.json"
done

echo "PAIRED_ECONOMICS_COMPLETE"
