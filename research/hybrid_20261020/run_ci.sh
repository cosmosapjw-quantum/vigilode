#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
ROOT=$PWD
NODE=crates/vigilode-hybrid
RESEARCH=research/hybrid_20261020
STAGE=setup
trap 'c=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$c" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal --component rustfmt > "$OUT/rust_install.log" 2>&1
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
python3 "$RESEARCH/repair_bridge.py"
if [[ ! -f "$NODE/Cargo.lock" ]]; then cargo +1.94.1 generate-lockfile --manifest-path "$NODE/Cargo.toml" > "$OUT/lock.log" 2>&1; fi
cargo +1.94.1 fmt --manifest-path "$NODE/Cargo.toml"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add "$NODE"
if ! git diff --cached --quiet; then
 git commit -m 'fix: freeze repaired hybrid bridge and lock before tests and timing [skip ci]'
 git push origin HEAD:refs/heads/research/hybrid-structure-ad-20261020
fi
SOURCE=$(git rev-parse HEAD)
printf '%s\n' "$SOURCE" > "$OUT/EXEC_SOURCE_COMMIT.txt"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; python3 -m pip freeze; } > "$OUT/VERSIONS.txt"
STAGE=native_tests
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --locked -- --nocapture --test-threads=1 > "$OUT/TESTS.log" 2>&1
tail -90 "$OUT/TESTS.log"
STAGE=external_sources
sudo apt-get update -qq > "$OUT/apt.log" 2>&1
sudo apt-get install -y -qq gfortran libblas-dev >> "$OUT/apt.log" 2>&1
PIN=9c6a6d9145795e35b978652a567b806ddc84a6bd
ORIG=research/strict_solver_comparison_20261019
SRC="$RUNNER_TEMP/hybrid-netlib"
mkdir -p "$SRC"
git fetch --depth=1 origin "$PIN"
for f in vode dgefa dgesl dgbfa dgbsl; do git show "$PIN:$ORIG/runs/39257390707/sources/$f.f" > "$SRC/$f.f"; done
git show "$PIN:$ORIG/src/vode_bridge.f90" > "$SRC/vode_bridge.f90"
git show "$PIN:$ORIG/runs/39257390707/NETLIB_SHA256SUMS" > "$OUT/NETLIB_SHA256SUMS"
(cd "$SRC"; sha256sum -c "$OUT/NETLIB_SHA256SUMS") > "$OUT/NETLIB_VERIFICATION.log"
sha256sum "$SRC/vode_bridge.f90" > "$OUT/BRIDGE_SHA256SUM"
export HYBRID_VODE_SOURCE="$SRC"
STAGE=measurement_build
cargo +1.94.1 build --manifest-path "$NODE/Cargo.toml" --locked --profile measurement --features external --example hybrid_bench -j2 > "$OUT/BUILD.log" 2>&1
BINARY="$ROOT/$NODE/target/measurement/examples/hybrid_bench"
sha256sum "$BINARY" > "$OUT/BINARY_SHA256SUM"
STAGE=external_preflight
for ARM in auto structured diffsol dvode; do "$BINARY" 16 0 "$ARM" 1e-6 1 1 > "$OUT/PREFLIGHT_${ARM}.json"; done
python3 - "$OUT" <<'PY'
import json,sys
from pathlib import Path
for p in Path(sys.argv[1]).glob('PREFLIGHT_*.json'):
 r=json.loads(p.read_text());assert r['success'] and r['deterministic'],(p,r.get('error'))
PY
STAGE=accuracy_and_timing
python3 "$RESEARCH/measure.py" --binary "$BINARY" --source "$SOURCE" --out "$OUT" 2>&1 | tee "$OUT/measure.log"
STAGE=rational_proof_and_raw_readback
python3 "$RESEARCH/audit.py" "$OUT" 2>&1 | tee "$OUT/audit.log"
STAGE=complete
