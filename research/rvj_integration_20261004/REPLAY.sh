#!/usr/bin/env bash
# New native/changed-boundary replay only. Supply Rust1.94.1 and offline vendor.
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
node=research/rvj_integration_20261004
# Preserve publication evidence; replay output goes to caller-selectable fresh dir.
out="${RVJ_REPLAY_OUTPUT:-/tmp/vigilode-rvj-replay-$(date -u +%Y%m%dT%H%M%SZ)}"
mkdir -p "$out"
cargo test --offline --locked -p rodas5p-core --test shared_shift_jet_contracts
cargo test --offline --locked -p rodas5p-integrators --features audit2-research \
  --test rvj_integration_boundaries --test native_reaudit_chart_transport --test int04_stage_chart_contracts
cargo run --offline --locked -p rodas5p-core --example shared_shift_jet_study > "$out/native_study.json"
python3 "$node/verify_exact.py" --input "$out/native_study.json" --output "$out/exact_oracle.json"
cargo fmt --all -- --check
cargo clippy --offline --locked -p rodas5p-core --lib --test shared_shift_jet_contracts \
  --example shared_shift_jet_study -- -D warnings
cargo clippy --offline --locked -p rodas5p-integrators --features audit2-research --lib \
  --test rvj_integration_boundaries --test native_reaudit_chart_transport \
  --test int04_stage_chart_contracts -- -D warnings
printf 'Fresh replay evidence: %s\n' "$out"
