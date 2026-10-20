#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=crates/vigilode-hybrid
STAGE=setup
trap 'c=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$c" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal --component rustfmt > "$OUT/rust_install.log" 2>&1
cargo +1.94.1 generate-lockfile --manifest-path "$NODE/Cargo.toml" > "$OUT/lock.log" 2>&1
cargo +1.94.1 fmt --manifest-path "$NODE/Cargo.toml"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add "$NODE"
if ! git diff --cached --quiet; then
 git commit -m 'build: freeze hybrid lock and formatted source before tests [skip ci]'
 git push origin HEAD:refs/heads/research/hybrid-structure-ad-20261020
fi
git rev-parse HEAD > "$OUT/EXEC_SOURCE_COMMIT.txt"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
rustc +1.94.1 -Vv > "$OUT/RUST_VERSION.txt"
STAGE=native_tests
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --locked -- --nocapture --test-threads=1 > "$OUT/TESTS.log" 2>&1
tail -90 "$OUT/TESTS.log"
STAGE=complete
