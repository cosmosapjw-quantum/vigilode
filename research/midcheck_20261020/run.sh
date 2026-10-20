#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
ROOT=$PWD
NODE=research/midcheck_20261020
NATIVE=research/midcheck_native_20261020
PIN=9c6a6d9145795e35b978652a567b806ddc84a6bd
STAGE=setup
trap 'c=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$c" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal > "$OUT/rustup.log" 2>&1
sudo apt-get update -qq
sudo apt-get install -y -qq gfortran libblas-dev
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
{ git rev-parse HEAD; rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; gfortran --version; python3 -m pip freeze; } > "$OUT/ENVIRONMENT.txt"
STAGE=source_pinning
git fetch --depth=1 origin "$PIN"
mkdir -p "$NATIVE/src" "$NATIVE/refinement" "$NATIVE/sources"
ORIG=research/strict_solver_comparison_20261019
for f in Cargo.toml src/main.rs src/build.rs src/vode_bridge.f90 refinement/banded.rs; do
    git show "$PIN:$ORIG/$f" > "$NATIVE/$f"
done
for f in vode dgefa dgesl dgbfa dgbsl; do
    git show "$PIN:$ORIG/runs/39257390707/sources/$f.f" > "$NATIVE/sources/$f.f"
done
git show "$PIN:$ORIG/runs/39257390707/NETLIB_SHA256SUMS" > "$NATIVE/NETLIB_SHA256SUMS"
git show "$PIN:$ORIG/runs/39257390707/references.json" > "$OUT/references.json"
git show "$PIN:$ORIG/refined_runs/39261895983/FROZEN_MATCHED_SELECTION.json" > "$OUT/FROZEN_MATCHED_SELECTION.json"
(cd "$NATIVE/sources" && sha256sum -c ../NETLIB_SHA256SUMS) > "$OUT/NETLIB_VERIFICATION.log"
python3 "$NODE/patch_native.py" "$NATIVE" | tee "$OUT/PATCH.log"
cp "$NATIVE/PATCH_RECEIPT.json" "$OUT/PATCH_RECEIPT.json"
# The resolved dependency set is recorded exactly, not implied by a prior lock.
cargo +1.94.1 generate-lockfile --manifest-path "$NATIVE/Cargo.toml" > "$OUT/dependency_resolution.log" 2>&1
cp "$NATIVE/Cargo.lock" "$OUT/Cargo.lock"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add "$NATIVE/Cargo.toml" "$NATIVE/Cargo.lock" "$NATIVE/src" "$NATIVE/refinement" "$NATIVE/NETLIB_SHA256SUMS" "$NATIVE/PATCH_RECEIPT.json"
# The five downloaded Fortran source bytes are reconstructible from the pinned
# source commit and their hashes, so they need not be published twice.
if ! git diff --cached --quiet; then
    git commit -m 'bench: freeze expanded native three-solver comparator before measurement [skip ci]'
    git push origin HEAD:refs/heads/research/midcheck-three-solver-20261020
fi
git rev-parse HEAD > "$OUT/EXEC_SOURCE_COMMIT.txt"
sha256sum "$NATIVE/src/main.rs" "$NATIVE/Cargo.toml" > "$OUT/SOURCE_SHA256SUMS"
STAGE=build
export STRICT_VODE_SOURCE="$ROOT/$NATIVE/sources"
cargo +1.94.1 build --manifest-path "$NATIVE/Cargo.toml" --profile measurement --locked -j2 > "$OUT/build.log" 2>&1
BIN="$ROOT/$NATIVE/target/measurement/strict-compare"
sha256sum "$BIN" > "$OUT/BINARY_SHA256"
STAGE=preflight
"$BIN" models > "$OUT/models.json"
python3 - "$OUT/models.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]))
assert any(c['name']=='few-mode-96' and c['dimension']==96 for c in m['cases'])
assert 'coupled-lazy' in m['arms'] and 'netlib-dvode' in m['arms'] and 'diffsol-bdf' in m['arms']
PY
"$BIN" bench few-mode-96 coupled-lazy 1e-6 endpoint 1 1 2 > "$OUT/coupled_preflight.json"
"$BIN" bench few-mode-96 netlib-dvode 1e-6 endpoint 1 1 2 > "$OUT/dvode_preflight.json"
"$BIN" bench few-mode-96 diffsol-bdf 1e-6 endpoint 1 1 2 > "$OUT/diffsol_preflight.json"
python3 - "$OUT" <<'PY'
import json,sys
from pathlib import Path
root=Path(sys.argv[1])
for name in ('coupled','dvode','diffsol'):
    r=json.loads((root/(name+'_preflight.json')).read_text())
    assert r['status']=='ok' and r['deterministic'] and len(r['states'])==2, (name,r.get('error'))
PY
STAGE=measurement
CPU=$(python3 -c 'import os;print(min(os.sched_getaffinity(0)))')
taskset -c "$CPU" python3 "$NODE/measure.py" "$BIN" "$OUT" "$OUT" 2>&1 | tee "$OUT/measure.log"
STAGE=readback
python3 "$NODE/audit.py" "$OUT" 2>&1 | tee "$OUT/audit.log"
STAGE=complete
