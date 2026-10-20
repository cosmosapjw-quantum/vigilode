#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
rustup toolchain install 1.94.1 --profile minimal > "$OUT/rust_install.log" 2>&1
set +e
cargo +1.94.1 test --manifest-path crates/vigilode-coupled/Cargo.toml --locked --test economy_contracts -- --test-threads=1 > "$OUT/RED.log" 2>&1
CODE=$?
set -e
printf '{"phase":"RED","exit_code":%s,"source":"%s"}\n' "$CODE" "$(git rev-parse HEAD)" > "$OUT/STATUS.json"
tail -100 "$OUT/RED.log"
[[ "$CODE" == 101 ]]
grep -q 'test result: FAILED. 1 passed; 3 failed' "$OUT/RED.log"
