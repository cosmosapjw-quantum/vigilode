# Reproduce observations or run a distinct new experiment

## Immutable readback, no Rust required

In a checkout containing this run, use a NEW output/work directory and Python3.13:

```bash
python3 - <<'PY'
from pathlib import Path
import hashlib,json
r=Path('research/coupled_native_20261020/runs/39349031422')
for x in json.loads((r/'RECONSTRUCTION.json').read_text()):
    b=b''.join((r/p).read_bytes() for p in x['parts_in_order'])
    assert hashlib.sha256(b).hexdigest()==x['sha256']
    p=r/x['original']
    if p.exists(): assert hashlib.sha256(p.read_bytes()).hexdigest()==x['sha256']
    else: p.write_bytes(b)
PY
(cd research/coupled_native_20261020/runs/39349031422 && sha256sum --check SHA256SUMS)
cp -r research/coupled_native_20261020/runs/39349031422 /tmp/coupled-readback-new
python3 research/coupled_native_20261020/audit.py /tmp/coupled-readback-new
```

The auditor writes derived AUDIT.json/REPORT.md into the copy, not the historical directory. It verifies canonical state hashes, full output/reference shapes and finite values, scalar-fsum metrics, attempted work, fixed-source worker-count bits, calibration/timing equality and ratios. This reproduces observed root-target agreement, not a continuous-time enclosure.

## New Rust experiment

Use an isolated worktree at the actual measured source; do not accidentally build a later documentation HEAD and call it the old binary.

```bash
git worktree add --detach /tmp/vigilode-coupled-source 1e4a993874d6930ea2bb41bb0ff7392f21c33df7
cd /tmp/vigilode-coupled-source
rustup toolchain install 1.94.1 --profile minimal
cargo +1.94.1 test --manifest-path crates/vigilode-coupled/Cargo.toml --locked
cargo +1.94.1 build --manifest-path crates/vigilode-coupled/Cargo.toml --locked --profile measurement --example coupled_bench
python3 -m venv /tmp/coupled-benchmark-env
source /tmp/coupled-benchmark-env/bin/activate
python3 -m pip install 'numpy==2.3.5' 'scipy==1.16.3'
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
# Keep multiple CPUs available; pinning the process to one CPU defeats the
# intended worker1/2/4 experiment. Record the allowed affinity and actual host.
python3 research/coupled_native_20261020/benchmark.py \
 --binary "$PWD/crates/vigilode-coupled/target/measurement/examples/coupled_bench" \
 --fixture fixtures/rodas5p_coefficients_snapshot.json \
 --source "$(git rev-parse HEAD)" --out /tmp/coupled-benchmark-new
python3 research/coupled_native_20261020/audit.py /tmp/coupled-benchmark-new
```

These commands follow the executed workflow but are a new run. Save its own host, source, binary hash, dependency versions and all failure/timing records. Do not overwrite old records or expect new wall-clock samples to match their bytes. The source and comparator/accuracy contracts may reproduce while machine time differs.

## CodeRabbit prerequisite

In an authenticated local/Codex environment, run `coderabbit auth login --agent`, confirm `coderabbit auth status --agent` actually reports authentication, then run the review against the pinned base. Exit0 from status alone does not establish login. This task's recorded login failed and review did not execute; no zero-issue report is implied.
