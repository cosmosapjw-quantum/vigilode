# API, source-pinned execution and frozen readback

**For the saved measurement, follow [ARCHIVE_RECOVERY.md](ARCHIVE_RECOVERY.md) first.** REFERENCES.json was excluded by an initial publication-size predicate; its byte-exact recovered gzip must be restored into a temporary copy before running the historical auditor. RECONSTRUCTION.json alone does not restore this particular file. All101archived hashes have already been checked with the recovered bytes.

## Public APIs

The old `coupled_step` and `coupled_or_sequential` entrypoints remain eager. New opt-in APIs are:

```rust
use vigilode_coupled::{coupled_step_residual_first, coupled_or_sequential_budgeted};
let attempt = coupled_step_residual_first(&problem, t, &y, h, &config, &execution);
let selected = coupled_or_sequential_budgeted(
    &problem, t, &y, h, &config, &execution, &linear_config, candidate_jvp_cap,
);
```

These snippets assume caller-owned values; they are not standalone programs. The effective candidate JVP cap is the smaller of config.max_jvp and the supplied cap. The fallback remains separately budgeted/accounted. Pure callbacks and a frozen model epoch are required. A returned candidate is not permission to advance physical time: embedded acceptance and any required output-error authority remain separate.

## Executed-source tests and a NEW benchmark

Use an isolated worktree at the measured source:

```bash
git worktree add --detach /tmp/vigilode-economy-source f08d152a35fb2db748692570969cb1723c287c26
cd /tmp/vigilode-economy-source
rustup toolchain install 1.94.1 --profile minimal
cargo +1.94.1 test --manifest-path crates/vigilode-coupled/Cargo.toml --locked
cargo +1.94.1 build --manifest-path crates/vigilode-coupled/Cargo.toml \
  --profile measurement --example economy_bench --locked
crates/vigilode-coupled/target/measurement/examples/economy_bench \
  run few-mode 12288 1 0.001 lazy-4 3 1
```

Registered arms: eager-1,eager-4,lazy-1,lazy-4,sequential-mf,txn,cap64,cap256. txn uses the original Config ceiling. The model/untimed reference came from the frozen prior benchmark template; candidate solvers never receive its assembled reference matrix.

For a NEW full experiment, use Python3.13 with numpy2.3.5/scipy1.16.3. Leave multiple CPUs available and record the host and affinity:

```bash
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
OUT=$(mktemp -d /tmp/vigilode-economy-run.XXXXXX)
python3 research/coupled_economy_20261020/benchmark.py \
 --binary "$PWD/crates/vigilode-coupled/target/measurement/examples/economy_bench" \
 --fixture fixtures/rodas5p_coefficients_snapshot.json \
 --source "$(git rev-parse HEAD)" --out "$OUT"
python3 research/coupled_economy_20261020/audit.py "$OUT"
```

The benchmark imports unchanged reference helpers from the earlier node, so keep that file in the checkout. No prior timing samples are loaded. New execution produces new host/time evidence; it must not overwrite or be attributed to the historical run.

## Work-counter compatibility

Added fields: derivative_batches,residual_only_stops,unused_stage_jvp_avoided. Actual task identity is

`stage_tasks_scheduled = 8 * (stage_batches + derivative_batches + validation_batches)`.

The eager mode's new fields are zero. Residual-first max_batches includes additional derivative dispatches; a tight old batch budget can therefore bind differently. avoided is counterfactual work not executed and must not be added to JVP cost. Strict JSON consumers must admit the new fields/schema.

Full raw timing metadata lives in accuracy.jsonl.gz/timing.jsonl.gz. state_object_sha256 resolves to states/<hash>.json.gz with complete output/error/result objects. It addresses canonical JSON bytes representing observed binary64 values, not original stdout bytes. The auditor checks numerical bits across methods rather than equating different work-ledger object hashes with different physical outputs.
