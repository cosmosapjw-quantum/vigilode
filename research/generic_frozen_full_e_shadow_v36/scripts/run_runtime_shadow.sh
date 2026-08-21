#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 MEASUREMENT_BINARY OUTPUT_ROOT" >&2
  exit 2
fi

binary=$1
output_root=$2
profiles=(calibration96 calibration192 calibration256 holdout320 holdout384)
families=(
  robertson
  hires
  van-der-pol
  rotating-nonnormal
  nonautonomous-forcing
  semilinear
)

if [[ ! -x "$binary" ]]; then
  echo "measurement binary is not executable: $binary" >&2
  exit 2
fi

for profile in "${profiles[@]}"; do
  mkdir -p "$output_root/$profile"
  for family in "${families[@]}"; do
    "$binary" generic-frozen-full-e-shadow \
      --profile "$profile" \
      --family "$family" \
      --output "$output_root/$profile/$family.json"
  done
done

echo "RUNTIME_SHADOW_COMPLETE"
