#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=$(cd "$(dirname "$0")/.." && pwd)
PREVIOUS="$NODE/runs/39257390707"
trap 's=$?; printf "{\"exit_code\":%s,\"stage\":\"%s\"}\n" "$s" "${STAGE:-startup}" > "$OUT/EXIT_STATUS.json"' EXIT
STAGE=setup
sudo apt-get update -qq
sudo apt-get install -y -qq gfortran libblas-dev
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' 'matplotlib==3.10.7' > "$OUT/python_install.log" 2>&1
rustup toolchain install 1.94.1 --profile minimal > "$OUT/rust_install.log" 2>&1
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; gfortran --version; python3 -m pip freeze; } > "$OUT/versions.txt"
STAGE=bind_banded_source
python3 "$NODE/refinement/patch_driver.py" > "$OUT/SOURCE_EXTENSION_RECEIPT.json"
if ! git diff --quiet -- "$NODE/src/main.rs"; then
  git config user.name 'github-actions[bot]'
  git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
  git add -- "$NODE/src/main.rs"
  git commit -m 'bench: bind separately named banded-slices arm before refined measurement [skip ci]'
  git push origin HEAD:refs/heads/research/benchmark-results-20261019-dual
fi
git rev-parse HEAD > "$OUT/EXEC_SOURCE_COMMIT.txt"
git fetch --depth=1 origin a21d573980f66fe02891bd216153443b5352f921
git diff --exit-code a21d573980f66fe02891bd216153443b5352f921 -- crates Cargo.toml Cargo.lock > "$OUT/production_source_diff.txt"
cp "$PREVIOUS/Cargo.lock" "$NODE/Cargo.lock"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
mkdir -p "$OUT/sources"
cp "$PREVIOUS"/sources/*.f "$OUT/sources/"
export STRICT_VODE_SOURCE="$OUT/sources"
(cd "$OUT/sources" && sha256sum *.f) > "$OUT/NETLIB_SHA256SUMS"
cmp "$OUT/NETLIB_SHA256SUMS" "$PREVIOUS/NETLIB_SHA256SUMS"
STAGE=tests_and_build
python3 "$NODE/scripts/test_contract.py" 2>&1 | tee "$OUT/contract_tests.log"
python3 "$NODE/scripts/test_serialization.py" 2>&1 | tee "$OUT/serialization_tests.log"
cargo +1.94.1 build --manifest-path "$NODE/Cargo.toml" --profile measurement --locked -j2 2>&1 | tee "$OUT/build.log"
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --profile measurement --locked -j2 -- --test-threads=1 2>&1 | tee "$OUT/native_tests.log"
STAGE=calibration_then_timing
BINARY="$NODE/target/measurement/strict-compare"
sha256sum "$BINARY" > "$OUT/BINARY_SHA256SUM"
CPU=$(python3 -c 'import os; print(min(os.sched_getaffinity(0)))')
taskset -c "$CPU" python3 "$NODE/refinement/refine.py" --binary "$BINARY" --previous "$PREVIOUS" --out "$OUT" 2>&1 | tee "$OUT/campaign.log"
STAGE=independent_readback
python3 "$NODE/refinement/verify_links.py" "$OUT" 2>&1 | tee "$OUT/link_audit.log"
python3 "$NODE/refinement/audit.py" --out "$OUT" --previous "$PREVIOUS" 2>&1 | tee "$OUT/audit.log"
STAGE=complete
