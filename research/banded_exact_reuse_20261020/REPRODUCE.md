# Reproduce without overwriting historical observations

The successful run is `runs/39304607635`. Use an isolated clone/worktree. The comparison uses the new common driver at commit f8e89b70528da8490fbe1febf0b0ede9ff84a01e with two different numerical-library source trees, not two unmodified whole historical repositories.

## Saved-data readback

From a checkout containing the receipts, in a Python3.13 environment with NumPy2.3.5/SciPy1.16.3:

```bash
NODE=research/banded_exact_reuse_20261020
(cd "$NODE/runs/39304607635" && sha256sum --check SHA256SUMS)
# Copy first; this auditor writes a new AUDIT and report beside its input.
cp -r "$NODE/runs/39304607635" /tmp/vigilode-banded-readback-new
python3 "$NODE/scripts/audit.py" /tmp/vigilode-banded-readback-new
```

Expected scope:432accuracy rows,576timing rows,324same-output pairs,36paired medians and87full-state objects. `accuracy.jsonl.gz` and `timing.jsonl.gz` retain metadata/per-repeat times; `state_sha256` resolves to a canonical state/time JSON object under states/. This is content identity of binary64 observations, not original stdout byte identity.

## New measurement

```bash
PUB=$(pwd)
NODE=research/banded_exact_reuse_20261020
OLD=b56bb19ce85bc28a021138ee29689701a81bc4f0
NEW=f8e89b70528da8490fbe1febf0b0ede9ff84a01e
git worktree add --detach /tmp/vigilode-band-parent "$OLD"
git worktree add --detach /tmp/vigilode-band-candidate "$NEW"
mkdir -p /tmp/vigilode-band-parent/$NODE
cp -r /tmp/vigilode-band-candidate/$NODE/bench /tmp/vigilode-band-parent/$NODE/bench
for ROOT in /tmp/vigilode-band-parent /tmp/vigilode-band-candidate; do
  cp "$PUB/$NODE/runs/39304607635/BENCH_Cargo.lock" "$ROOT/$NODE/bench/Cargo.lock"
done
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 RAYON_NUM_THREADS=1
cargo +1.94.1 build --manifest-path /tmp/vigilode-band-parent/$NODE/bench/Cargo.toml --profile measurement --locked
cargo +1.94.1 build --manifest-path /tmp/vigilode-band-candidate/$NODE/bench/Cargo.toml --profile measurement --locked --features improvement
PARENT=/tmp/vigilode-band-parent/$NODE/bench/target/measurement/band-study
CANDIDATE=/tmp/vigilode-band-candidate/$NODE/bench/target/measurement/band-study
OUT=/tmp/vigilode-band-new-run
mkdir "$OUT"
CPU=$(python3 -c 'import os;print(min(os.sched_getaffinity(0)))')
{ date -u; uname -a; lscpu; rustc +1.94.1 -Vv; python3 -m pip freeze; } > "$OUT/HOST.txt"
python3 "$NODE/scripts/smoke.py" "$PARENT" "$CANDIDATE" "$OUT"
taskset -c "$CPU" python3 "$NODE/scripts/study.py" --parent "$PARENT" --candidate "$CANDIDATE" --parent-source "$OLD" --candidate-source "$NEW" --out "$OUT"
python3 "$NODE/scripts/audit.py" "$OUT"
```

Commands assume no inherited CARGO_TARGET_DIR override; unset it or adjust binary paths. New hardware/timing receipts must remain separate. Full-workspace timeout from the old run is not a result of any new host's tests. Run `cargo +1.94.1 test --workspace --no-fail-fast --locked` on the candidate separately and retain its actual exit/status.

`run_ci.sh` is the bounded historical orchestration; `scripts/apply.py` is an exact-parent patch recipe and must not be reapplied to the already-patched source. The baseline RED and adapter failure are retained and never rewritten into green outcomes.
