#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
export PATH="${N4B_RUST_BIN:-/mnt/data/vigilode_n3_runtime/rust-1.99.1/bin}:$PATH"
export CARGO_TARGET_DIR="${N4B_TARGET_DIR:-$ROOT/.target}"
"$(command -v rustc)" --version | grep -F '1.99.1'
python "$ROOT/verify_manifest.py" "$ROOT"
(
 cd "$ROOT/rust"
 cargo test --locked --offline --all-targets
 cargo clippy --locked --offline --all-targets -- -D warnings
 rustfmt --check --edition 2024 ../new_fragment.rs src/bin/n4b_two_window.rs src/bin/n4b_cost.rs
 cargo run --release --quiet --locked --offline --bin n4b_two_window > "$ROOT/evidence/N4B_REPLAY.csv"
 cmp "$ROOT/evidence/N4B_TWO_WINDOW_NATIVE.csv" "$ROOT/evidence/N4B_REPLAY.csv"
)
python "$ROOT/quick_fraction_oracle.py"
if [[ "${N4B_FULL_180_220:-0}" == 1 ]]; then
 python "$ROOT/independent_second_window.py"
fi
printf '%s\n' 'N4B_SCOPED_REPLAY=PASS (not production IVP, not actual PR131 native RODAS8)'
