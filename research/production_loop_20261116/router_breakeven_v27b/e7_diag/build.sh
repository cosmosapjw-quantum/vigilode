#!/usr/bin/env bash
# Build the V30 E7 diagnosis binary (node V30_ROUTER_BREAKEVEN_V27B, phase 1) against the crates
# of this worktree. One cargo process at a time under the host-wide build lock, -j 4, cores 0-5
# and 12-17, toolchain 1.94.1, --locked (Cargo.lock: the V27 runner's lock, root package renamed).
# Output: the build log on stderr, then sha256 of the inputs and of the binary.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
LOCK=/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad/locks/cargo_a.lock
TARGET=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-v30

CARGO_TARGET_DIR=$TARGET flock -w 14400 "$LOCK" taskset -c 0-5,12-17 \
    cargo +1.94.1 build --locked -j 4 --release --manifest-path "$HERE/Cargo.toml"

sha256sum "$HERE/Cargo.toml" "$HERE/Cargo.lock" "$HERE/src/main.rs" "$TARGET/release/v30-e7-diag"
