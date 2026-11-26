#!/usr/bin/env bash
# Build the two binaries of the V30 evaluation runner (node V30_ROUTER_BREAKEVEN_V27B, phase 2).
#
#   arm N: this package against the crates of this worktree, with the feature `pilots`;
#          target dir w4-v30, binary $N_TARGET/release/v30-runner
#   arm T: a copy of this package under $T_DIR (outside both checkouts) whose `vigilode`
#          dependency is the read-only T checkout (absolute path), without the feature;
#          target dir w4-v30base, binary $T_TARGET/release/v30-runner
#
# Cargo.lock of both: the V27 runner's lock with the root package renamed (the facade's
# dependency graph is the same in both checkouts), so both builds run with --locked.
# Every cargo command runs under the host-wide build lock, -j 4, cores 0-5 and 12-17,
# toolchain 1.94.1 (loop L1 rules). Output: cargo's log, then sha256 of inputs and binaries.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
BASE=/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w4-v30base
N_TARGET=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-v30
T_TARGET=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-v30base
T_DIR=$N_TARGET/runner_t
LOCK=/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad/locks/cargo_b.lock

cargo_build() { # manifest target_dir features
    CARGO_TARGET_DIR=$2 flock -w 14400 "$LOCK" taskset -c 0-5,12-17 \
        cargo +1.94.1 build --locked -j 4 --release --manifest-path "$1" $3
}

# arm T: the copy with the T checkout as its dependency
mkdir -p "$T_DIR/src"
cp "$HERE/src/main.rs" "$T_DIR/src/main.rs"
cp "$HERE/Cargo.lock" "$T_DIR/Cargo.lock"
sed "s#^vigilode = { path = \"../../../../crates/vigilode\" }#vigilode = { path = \"$BASE/crates/vigilode\" }#" \
    "$HERE/Cargo.toml" > "$T_DIR/Cargo.toml"
grep -q "path = \"$BASE/crates/vigilode\"" "$T_DIR/Cargo.toml"

cargo_build "$HERE/Cargo.toml" "$N_TARGET" "--features pilots"
cargo_build "$T_DIR/Cargo.toml" "$T_TARGET" ""

sha256sum "$HERE/Cargo.toml" "$HERE/Cargo.lock" "$HERE/src/main.rs" \
    "$T_DIR/Cargo.toml" "$T_DIR/Cargo.lock" "$T_DIR/src/main.rs" \
    "$N_TARGET/release/v30-runner" "$T_TARGET/release/v30-runner"
