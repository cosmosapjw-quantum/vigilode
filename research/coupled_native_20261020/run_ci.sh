#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=crates/vigilode-coupled
STAGE=setup
trap 'code=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$code" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal --component rustfmt > "$OUT/rust_install.log" 2>&1
cp research/coupled_native_20261020/runs/39345503351/Cargo.lock "$NODE/Cargo.lock"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add "$NODE/Cargo.lock"
if ! git diff --cached --quiet; then
 git commit -m 'build: freeze coupled companion dependency lock before tests [skip ci]'
 git push origin HEAD:refs/heads/research/coupled-matrix-free-rust-20261020
fi
git rev-parse HEAD > "$OUT/EXEC_SOURCE_COMMIT.txt"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
rustc +1.94.1 -Vv > "$OUT/rustc.txt"
STAGE=green_contracts
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --locked -- --test-threads=1 > "$OUT/GREEN.log" 2>&1
tail -100 "$OUT/GREEN.log"
STAGE=complete
