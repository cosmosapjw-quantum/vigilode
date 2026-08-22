#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 RODAS5P_BINARY OUTPUT_ROOT VERIFICATION_JSON" >&2
  exit 2
fi

binary=$(realpath "$1")
output_root=$(realpath -m "$2")
verification_output=$(realpath -m "$3")
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
node_root=$(cd "$script_dir/.." && pwd -P)
repo_root=$(cd "$node_root/../.." && pwd -P)
contract="$node_root/contracts/V37_TIMING_REPLICATION_CONTINUATION_TRANSACTION_CONTRACT.json"
v36_root="$repo_root/research/generic_frozen_full_e_shadow_v36/results/runtime"
expected_contract_sha=66f082aeec8c70e0ef23926d2c6f7057fb40fe280c45fd02c200be8778a6e659
profiles=(calibration96 calibration192 calibration256 holdout320 holdout384)
families=(robertson hires van-der-pol rotating-nonnormal nonautonomous-forcing semilinear)

if [[ ! -x "$binary" ]]; then
  echo "rodas5p binary is not executable: $binary" >&2
  exit 2
fi
if [[ -e "$output_root" || -e "$verification_output" ]]; then
  echo "refusing to overwrite existing replay output" >&2
  exit 2
fi
actual_contract_sha=$(sha256sum "$contract" | awk '{print $1}')
if [[ "$actual_contract_sha" != "$expected_contract_sha" ]]; then
  echo "sealed contract hash mismatch before runtime output: $actual_contract_sha" >&2
  exit 1
fi

mkdir -p "$(dirname "$output_root")" "$(dirname "$verification_output")"
tmp_root=$(mktemp -d "$(dirname "$output_root")/.v37-continuation-replay.XXXXXX")
tmp_verification=$(mktemp "$(dirname "$verification_output")/.v37-continuation-verification.XXXXXX")
completed=false
cleanup() {
  status=$?
  if [[ "$completed" != true ]]; then
    failed_root="${output_root}.failed.$(date -u +%Y%m%dT%H%M%SZ)"
    if [[ -d "$tmp_root" ]]; then
      mv "$tmp_root" "$failed_root"
      echo "retained failed replay shards at $failed_root" >&2
    fi
    rm -f "$tmp_verification"
  fi
  exit "$status"
}
trap cleanup EXIT

for profile in "${profiles[@]}"; do
  mkdir -p "$tmp_root/$profile"
  for family in "${families[@]}"; do
    "$binary" generic-v37-continuation-transaction \
      --profile "$profile" \
      --family "$family" \
      --output "$tmp_root/$profile/$family.json"
  done
done

python "$script_dir/verify_continuation_replay.py" \
  --contract "$contract" \
  --v36-root "$v36_root" \
  --runtime-root "$tmp_root" \
  --binary "$binary" \
  --output "$tmp_verification"

mv "$tmp_root" "$output_root"
mv "$tmp_verification" "$verification_output"
completed=true
trap - EXIT
echo "V37_CONTINUATION_REPLAY_PASS"
