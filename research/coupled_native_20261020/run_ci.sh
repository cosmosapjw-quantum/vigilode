#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=crates/vigilode-coupled
rustup toolchain install 1.94.1 --profile minimal > "$OUT/rust_install.log" 2>&1
cp Cargo.lock "$NODE/Cargo.lock"
cargo +1.94.1 metadata --manifest-path "$NODE/Cargo.toml" --format-version 1 > "$OUT/metadata.json" 2> "$OUT/resolve.log"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
set +e
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --locked --test contracts -- --test-threads=1 > "$OUT/RED.log" 2>&1
CODE=$?
set -e
printf '{"phase":"RED","test_exit_code":%s,"source":"%s"}\n' "$CODE" "$(git rev-parse HEAD)" > "$OUT/STATUS.json"
tail -80 "$OUT/RED.log"
[[ "$CODE" == 101 ]]
grep -q 'test result: FAILED' "$OUT/RED.log"
