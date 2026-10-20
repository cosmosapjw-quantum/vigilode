# API and exact-source reproduction

The unchanged entrypoints remain `coupled_step` and `coupled_or_sequential`. New opt-in APIs:

```rust
use vigilode_coupled::{coupled_step_residual_first, coupled_or_sequential_budgeted};

// Same Config and actual raw-U/root acceptance tolerances as the eager path.
let attempt = coupled_step_residual_first(&problem, t, &y, h, &config, &execution);

// Bound only candidate JVP attempts. Respect a smaller config.max_jvp.
// Fallback is the existing strict supplied-JVP sequential GMRES path.
let selected = coupled_or_sequential_budgeted(
    &problem, t, &y, h, &config, &execution, &linear_config, candidate_jvp_cap,
);
```

These snippets bind caller-owned problem/config/pool values and are not standalone programs. Callbacks must be pure at a frozen epoch. Never use `candidate.is_some()` alone as physical time-step acceptance. Check the separately returned embedded criterion; operational residual is not a certified output bound. This node contains no adaptive driver or dense-output integration.

## Tests and a new benchmark run

Use a new isolated worktree at the measured source, not a later documentation HEAD. Commands follow the executed workflow; running them again creates new host/time evidence, not identical historical samples.

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

Registered arms are eager-1,eager-4,lazy-1,lazy-4,sequential-mf,txn,cap64,cap256. txn uses residual-first with the original Config JVP ceiling. The model/independent raw reference is cloned from the prior source-checked benchmark adapter; candidates do not receive its explicit matrices.

For a full NEW campaign, provision Python3.13 with numpy2.3.5/scipy1.16.3, leave multiple CPUs available and record the host:

```bash
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
OUT=$(mktemp -d /tmp/vigilode-economy-run.XXXXXX)
python3 research/coupled_economy_20261020/benchmark.py \
 --binary "$PWD/crates/vigilode-coupled/target/measurement/examples/economy_bench" \
 --fixture fixtures/rodas5p_coefficients_snapshot.json \
 --source "$(git rev-parse HEAD)" --out "$OUT"
python3 research/coupled_economy_20261020/audit.py "$OUT"
```

The benchmark imports the historical node's unchanged Python model/reference helpers. Keep that file in the checkout. Only the new benchmark's80cells,640processes and newly generated reference data are used; old wall-clock samples are never read.

## Frozen observation readback

In a checkout containing runs/39355449893, first apply RECONSTRUCTION.json if it lists split files, concatenating parts_in_order and checking each original SHA256. Verify SHA256SUMS. Copy the run into a NEW directory and invoke audit.py there because it writes derived AUDIT.json/REPORT.md. The original raw results must stay immutable.

`accuracy.jsonl.gz` and `timing.jsonl.gz` retain record metadata and all per-repeat nanosecond samples. `state_object_sha256` points to `states/<hash>.json.gz`, which contains the complete output/error/result object. The digest is of canonical JSON bytes containing binary64 numbers, not the original stdout bytes. The auditor explicitly checks float bits across methods/workers rather than treating differing work-ledger hashes as differing physical outputs.

## Counter compatibility

New Work fields: derivative_batches, residual_only_stops, unused_stage_jvp_avoided. Actual task identity is

`stage_tasks_scheduled = 8 * (stage_batches + derivative_batches + validation_batches)`.

For eager mode derivative_batches and avoided counters are zero, preserving old accounting. For residual-first mode max_batches counts the additional derivative dispatches; tight old batch budgets can consequently bind differently. avoided is counterfactual saved work and must not be added to actual JVP cost. Strict JSON consumers must admit the new fields or use the new row schema.
