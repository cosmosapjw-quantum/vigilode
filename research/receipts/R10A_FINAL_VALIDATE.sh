#!/usr/bin/env bash
set -euo pipefail
export PATH=/mnt/data/rodas5p-rust-toolchain-1.94.1/bin:$PATH
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
export RAYON_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 NUMEXPR_NUM_THREADS=1
root=${1:-$(git rev-parse --show-toplevel)}
cd "$root"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
cargo test --workspace --release --offline --locked -- --test-threads=1
cargo build --workspace --release --offline --locked
