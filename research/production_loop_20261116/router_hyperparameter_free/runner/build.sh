#!/usr/bin/env bash
# Build the two binaries of the V27 evaluation runner (node V27_ROUTER_HYPERPARAMETER_FREE).
#
#   arm N: this package against the crates of this worktree, with the feature `pilots`;
#          target dir w3-v27, binary $N_TARGET/release/v27-runner
#   arm T: a copy of this package under $T_DIR whose `vigilode` dependency is the base
#          checkout (absolute path), without the feature; target dir w3-v27base,
#          binary $T_TARGET/release/v27-runner
#
# Usage: build.sh lock    copy the repository Cargo.lock of each checkout next to its
#                         manifest and build once with --offline (this writes the lock of
#                         the runner), then build again with --locked
#        build.sh         build both with --locked (the locks must exist)
#
# One cargo process at a time, -j 4, cores 0-5 and 12-17 (loop L1 rules).
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../../.." && pwd)
BASE=/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-v27base
N_TARGET=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v27
T_TARGET=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v27base
T_DIR=$N_TARGET/runner_t
MODE=${1:-locked}

cargo_build() { # manifest target_dir features flag
    CARGO_TARGET_DIR=$2 taskset -c 0-5,12-17 cargo +1.94.1 build "$4" -j 4 --release \
        --manifest-path "$1" $3
}

# arm T: the copy with the base checkout as its dependency
mkdir -p "$T_DIR/src"
cp "$HERE/src/main.rs" "$T_DIR/src/main.rs"
sed "s#^vigilode = { path = \"../../../../crates/vigilode\" }#vigilode = { path = \"$BASE/crates/vigilode\" }#" \
    "$HERE/Cargo.toml" > "$T_DIR/Cargo.toml"
grep -q "path = \"$BASE/crates/vigilode\"" "$T_DIR/Cargo.toml"

if [ "$MODE" = lock ]; then
    cp "$REPO/Cargo.lock" "$HERE/Cargo.lock"
    cargo_build "$HERE/Cargo.toml" "$N_TARGET" "--features pilots" --offline
    cp "$BASE/Cargo.lock" "$T_DIR/Cargo.lock"
    cargo_build "$T_DIR/Cargo.toml" "$T_TARGET" "" --offline
fi
cargo_build "$HERE/Cargo.toml" "$N_TARGET" "--features pilots" --locked
cargo_build "$T_DIR/Cargo.toml" "$T_TARGET" "" --locked

sha256sum "$HERE/Cargo.toml" "$HERE/Cargo.lock" "$HERE/src/main.rs" \
    "$T_DIR/Cargo.toml" "$T_DIR/Cargo.lock" "$T_DIR/src/main.rs" \
    "$N_TARGET/release/v27-runner" "$T_TARGET/release/v27-runner"
