#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=crates/vigilode-coupled
STAGE=setup
trap 'code=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$code" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal --component rustfmt --component clippy > "$OUT/rust_install.log" 2>&1
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
python3 research/coupled_native_20261020/prepare.py
cargo +1.94.1 fmt --manifest-path "$NODE/Cargo.toml"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add "$NODE" .github/workflows/coupled-native-20261020.yml
if ! git diff --cached --quiet; then
 git commit -m 'style: freeze formatted coupled implementation before measurement [skip ci]'
 git push origin HEAD:refs/heads/research/coupled-matrix-free-rust-20261020
fi
SOURCE=$(git rev-parse HEAD)
printf '%s\n' "$SOURCE" > "$OUT/EXEC_SOURCE_COMMIT.txt"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; python3 -m pip freeze; } > "$OUT/versions.txt"
STAGE=tests
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --locked -- --nocapture --test-threads=1 > "$OUT/TESTS.log" 2>&1
cargo +1.94.1 clippy --manifest-path "$NODE/Cargo.toml" --all-targets --locked -- -W clippy::all > "$OUT/CLIPPY.log" 2>&1
STAGE=build
cargo +1.94.1 build --manifest-path "$NODE/Cargo.toml" --profile measurement --example coupled_bench --locked > "$OUT/BUILD.log" 2>&1
BINARY="$PWD/$NODE/target/measurement/examples/coupled_bench"
sha256sum "$BINARY" > "$OUT/BINARY_SHA256SUM"
STAGE=preflight
"$BINARY" run few-mode 96 1 0.01 coupled-4 1 0 > "$OUT/PREFLIGHT.json"
python3 - "$OUT/PREFLIGHT.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]));assert r['root_completed'] and r['deterministic'];assert r['result']['attempt']['candidate']['basis_dimension']==3
PY
STAGE=benchmark
python3 research/coupled_native_20261020/benchmark.py --binary "$BINARY" --fixture fixtures/rodas5p_coefficients_snapshot.json --source "$SOURCE" --out "$OUT" 2>&1 | tee "$OUT/benchmark.log"
STAGE=readback
python3 research/coupled_native_20261020/audit.py "$OUT" 2>&1 | tee "$OUT/AUDIT.log"
STAGE=coderabbit_prerequisite
bash research/coupled_native_20261020/coderabbit_prerequisite.sh "$OUT"
STAGE=complete
