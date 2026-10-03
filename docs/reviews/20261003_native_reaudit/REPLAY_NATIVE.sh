#!/usr/bin/env bash
set -euo pipefail
# Run from the repository root. Set VENDOR_DIR to the source directory produced
# by `cargo vendor --locked`, not an unrelated project's dependency archive.
: "${VENDOR_DIR:?Set VENDOR_DIR to the locked VigilODE vendor directory}"
test -f Cargo.lock && test -d crates/rodas5p-integrators
VENDOR_DIR=$(realpath "$VENDOR_DIR")
config=$(mktemp)
trap 'rm -f "$config"' EXIT
printf '[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "%s"\n' "$VENDOR_DIR" > "$config"
export RAYON_NUM_THREADS=${RAYON_NUM_THREADS:-1}
export PYTHONDONTWRITEBYTECODE=1
cargo fmt --all --check
cargo test --config "$config" --frozen -p rodas5p-core --test native_reaudit_laguerre_contracts
cargo test --config "$config" --frozen -p rodas5p-integrators --all-features \
  --test native_reaudit_cache_contracts --test native_reaudit_radius_contracts \
  --test native_reaudit_chart_transport --test thread_transfer_radius \
  --test thread_transfer_radius_policy --test thread_transfer_path_action \
  --test thread_transfer_raw_target --test thread_transfer_negative_controls \
  --test matrix_free_problem_contracts
cargo test --config "$config" --frozen -p rodas5p-krylov \
  --test gcrodr_state_validity_contracts --test workspace_contracts
cargo clippy --config "$config" --frozen --workspace --all-targets --all-features -- -D warnings
PYTHONPATH=tools python -m unittest discover -s tools -p 'test_native_reaudit_chart_domain.py'
python docs/reviews/20261003_native_reaudit/verify_algebra.py
