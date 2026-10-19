#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$NODE/../.." && pwd)
trap 'code=$?; printf "{\"exit_code\":%s,\"stage\":\"%s\"}\n" "$code" "${STAGE:-startup}" > "$OUT/EXIT_STATUS.json"' EXIT
STAGE=contract_tests
python3 "$NODE/scripts/test_contract.py" 2>&1 | tee "$OUT/contract_tests_GREEN.log"
STAGE=environment
sudo apt-get update -qq
sudo apt-get install -y -qq gfortran libblas-dev
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
rustup toolchain install 1.94.1 --profile minimal > "$OUT/rust_install.log" 2>&1
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; gfortran --version; python3 -m pip freeze; } > "$OUT/versions.txt"
STAGE=bounded_source_repair
set +e
python3 "$NODE/scripts/test_serialization.py" > "$OUT/serialization_before_repair.log" 2>&1
BEFORE=$?
set -e
printf '{"test_exit_before_repair":%s}\n' "$BEFORE" > "$OUT/serialization_red_status.json"
python3 "$NODE/scripts/repair_source_v1.py" "$OUT" 2>&1 | tee "$OUT/source_repair.log"
python3 "$NODE/scripts/test_serialization.py" 2>&1 | tee "$OUT/serialization_GREEN.log"
if ! git diff --quiet -- "$NODE/src/main.rs" "$NODE/scripts/campaign.py"; then
  git config user.name 'github-actions[bot]'
  git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
  git add -- "$NODE/src/main.rs" "$NODE/scripts/campaign.py"
  git commit -m 'fix(benchmark): serialize NumPy counters and respect native success/band buffer bounds [skip ci]'
  git push origin HEAD:refs/heads/research/benchmark-results-20261019-dual
fi
git rev-parse HEAD > "$OUT/EXEC_SOURCE_COMMIT.txt"
git fetch --depth=1 origin a21d573980f66fe02891bd216153443b5352f921
git diff --exit-code a21d573980f66fe02891bd216153443b5352f921 -- crates Cargo.toml Cargo.lock > "$OUT/production_source_diff.txt"
STAGE=freeze_netlib
mkdir -p "$OUT/sources"
curl -fsSL --connect-timeout 15 --max-time 60 https://www.netlib.org/ode/vode.f -o "$OUT/sources/vode.f"
for name in dgefa dgesl dgbfa dgbsl; do
  curl -fsSL --connect-timeout 15 --max-time 60 "https://www.netlib.org/linpack/${name}.f" -o "$OUT/sources/${name}.f"
done
(cd "$OUT/sources" && sha256sum *.f) > "$OUT/NETLIB_SHA256SUMS"
export STRICT_VODE_SOURCE="$OUT/sources"
STAGE=build
if [[ -f "$ROOT/research/strict_solver_comparison_20261019/runs/39255702592/Cargo.lock" ]]; then
  cp "$ROOT/research/strict_solver_comparison_20261019/runs/39255702592/Cargo.lock" "$NODE/Cargo.lock"
else
  cargo +1.94.1 generate-lockfile --manifest-path "$NODE/Cargo.toml" 2>&1 | tee "$OUT/lock_resolution.log"
fi
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
cargo +1.94.1 metadata --manifest-path "$NODE/Cargo.toml" --locked --format-version 1 > "$OUT/cargo_metadata.json"
cargo +1.94.1 build --manifest-path "$NODE/Cargo.toml" --profile measurement --locked -j2 2>&1 | tee "$OUT/build.log"
STAGE=native_tests
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --profile measurement --locked -j2 -- --test-threads=1 2>&1 | tee "$OUT/native_tests.log"
STAGE=accuracy_and_timing
BINARY="$NODE/target/measurement/strict-compare"
sha256sum "$BINARY" > "$OUT/BINARY_SHA256SUM"
CPU=$(python3 -c 'import os; print(min(os.sched_getaffinity(0)))')
taskset -c "$CPU" python3 "$NODE/scripts/campaign.py" --binary "$BINARY" --out "$OUT" 2>&1 | tee "$OUT/campaign.log"
STAGE=scipy117_differential_audit
python3 -m venv "$NODE/.scipy117"
"$NODE/.scipy117/bin/python" -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.17.0' > "$OUT/scipy117_install.log" 2>&1
taskset -c "$CPU" "$NODE/.scipy117/bin/python" "$NODE/scripts/scipy117_diag.py" --out "$OUT" 2>&1 | tee "$OUT/scipy117_diagnosis.log"
STAGE=hash_and_close
python3 - "$OUT" <<'PY'
from pathlib import Path
import hashlib,sys
root=Path(sys.argv[1])
with (root/'SHA256SUMS').open('w') as out:
    for p in sorted(root.rglob('*')):
        if p.is_file() and p.name not in ('SHA256SUMS','execution.log','EXIT_STATUS.json','large_files_excluded.txt'):
            out.write(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(root))+'\n')
PY
STAGE=complete
