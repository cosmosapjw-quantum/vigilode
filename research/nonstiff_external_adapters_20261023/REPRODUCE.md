# Reproduce the compatible external-adapter supplement

Use the `adapter_source_commit` in the final EXTERNAL_ADAPTERS.json, not the
later report HEAD. Native/problem source remains
45037fc24e8f8101605f309e1fdb3a9a4e3df56d. Its completed native/VODE/ensemble
campaign is reused without executing those solvers again.

Preserve the original checkout. Record its HEAD/branch/status and create a new
`mktemp -d` detached worktree at the recorded adapter source. Put all outputs,
targets and Julia depot outside that worktree in another new directory.

```bash
export VIGILODE_ADAPTER_SOURCE_COMMIT=<full adapter execution commit from EXTERNAL_ADAPTERS.json>
export VIGILODE_SOURCE=45037fc24e8f8101605f309e1fdb3a9a4e3df56d
export VIGILODE_RUN_ROOT=$(mktemp -d /tmp/vigilode-external-run-XXXXXX)
export ADAPTER_WORKTREE=$(mktemp -d /tmp/vigilode-external-source-XXXXXX)
git worktree add --detach "$ADAPTER_WORKTREE" "$VIGILODE_ADAPTER_SOURCE_COMMIT"
cd "$ADAPTER_WORKTREE"
export STUDY=research/nonstiff_external_adapters_20261023
export CARGO_TARGET_DIR="$VIGILODE_RUN_ROOT/target"
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
rustc -Vv
cargo --version
# Require observed Rust/Cargo1.94.1. Use the supplied task_runtime paths only
# when they actually exist on this host. Do not assume another host's cache.
cargo metadata --locked --offline --format-version 1 --manifest-path "$STUDY/adapters/diffsol/Cargo.toml"
# If cache is missing: retain the failure; with network available prepare it via
# cargo fetch --locked --manifest-path "$STUDY/adapters/diffsol/Cargo.toml"
# then retry the exact locked/offline command. Never edit the lockfile.
cargo build --locked --offline --profile measurement --manifest-path "$STUDY/adapters/diffsol/Cargo.toml"
export DIFFSOL_BINARY="$CARGO_TARGET_DIR/measurement/vigilode-nonstiff-diffsol-adapter"
export VIGILODE_JULIA_DEPOT="$VIGILODE_RUN_ROOT/julia-depot"
export VIGILODE_JULIA_EXECUTABLE=/absolute/path/to/verified/julia-1.12.7
JULIA_DEPOT_PATH="$VIGILODE_JULIA_DEPOT" JULIA_NUM_PRECOMPILE_TASKS=2 \
  "$VIGILODE_JULIA_EXECUTABLE" --startup-file=no --project="$STUDY/adapters/julia" \
  -e 'using Pkg; Pkg.instantiate(); Pkg.precompile()'
# Keep Project/Manifest byte-identical; dependency preparation is outside timing.
python3 "$STUDY/verify_adapters.py" --preflight-out "$VIGILODE_RUN_ROOT/preflight" \
  --diffsol-adapter "$DIFFSOL_BINARY" --julia-adapter "$PWD/$STUDY/adapters/julia/run.sh" \
  --adapter-source-commit "$VIGILODE_ADAPTER_SOURCE_COMMIT"
python3 "$STUDY/run_external.py" --out "$VIGILODE_RUN_ROOT/external-comparison" \
  --diffsol-adapter "$DIFFSOL_BINARY" --julia-adapter "$PWD/$STUDY/adapters/julia/run.sh" \
  --adapter-source-commit "$VIGILODE_ADAPTER_SOURCE_COMMIT"
python3 "$STUDY/analyze_external.py" \
  --baseline /path/to/original/45037fc2/full-comparison \
  --external "$VIGILODE_RUN_ROOT/external-comparison" \
  --merged-out "$VIGILODE_RUN_ROOT/merged-comparison"
```

The baseline directory must contain the preserved original PLAN/RECEIPTS,
SUMMARY, RAW_MANIFEST and raw stdout/stderr. Extract its bundled RAW_OUTPUTS
archive into a new directory when individual raw files are not present.
No baseline solver rerun is required for merging. Pin the recorded Python
package versions using the copied environment evidence. Missing dependencies
or failed processes remain explicit failures/NOT_RUN; do not relax validators.

Run the adapters sequentially with45s per-process timeout and one cumulative
7200s supplement budget. Julia's launcher fixes one Julia/OpenBLAS thread;
each fresh process includes one same-request JIT preparation solve outside the
solver timer but inside the45s cap. Campaign warmup is additional. Keep first
failures and raw stdout/stderr, commands, exit codes and input/output hashes.
Reference evaluation and JSON are outside solver time. Plotting is postprocess.
Cross-session comparisons with the native/VODE baseline are diagnostic only.
