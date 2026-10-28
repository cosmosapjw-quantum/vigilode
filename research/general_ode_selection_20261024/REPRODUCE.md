# Reproducing the selected general ODE package

Use a clean checkout of the published branch. Preserve other worktrees and failed receipts. NCP is not used. The accompanying `SOURCE_IDENTITY.json` records the exact native inputs; the final Git publication may add only documentation and evidence to that code.

The supplied Rust 1.94.1 toolchain and exact Cargo.lock vendor were recovered offline in this audit. `evidence/runtime/RECOVERY_INVENTORY.json` gives their hashes. Its absolute paths are receipts from this host, not portable installation instructions. On another host, supply Rust 1.94.1 and an exact locked cache/vendor; do not silently update Cargo.lock to satisfy missing packages.

## Correctness

```sh
cargo metadata --locked --offline --format-version 1 > metadata.json
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo test -p vigilode --locked --offline --lib --tests
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo test -p vigilode --locked --offline --doc
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo test -p rodas5p-integrators --locked --offline --lib fast_error_norm_refuses_overflowed_scalar_and_vector_scales
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo test -p rodas5p-integrators --locked --offline \
  --test frontier_session_contracts --test rodas5p_fast_contracts \
  --test fast_vector_atol_contracts --test int03_banded_fast_contracts \
  --test fast_dense_contracts --test dopri5_contracts
python research/general_ode_selection_20261024/analysis/verify_numerics.py
python research/general_ode_selection_20261024/src/analyze_portfolio.py
```

The exact symbolic checks are not substitutes for Rust tests. The portfolio checker validates published evidence identity, not solver execution. Cargo failure and numerical failure are recorded separately.

## New measured workloads

Set numerical libraries to one thread and serialize compilation, tests and timing. The measurement profile is repository-defined; the audit additionally used one codegen unit and disabled incremental builds. Do not run the PDF compiler or another benchmark while timing.

```sh
export RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
CARGO_BUILD_JOBS=1 CARGO_PROFILE_MEASUREMENT_CODEGEN_UNITS=1 CARGO_INCREMENTAL=0 \
  cargo build -p vigilode --locked --offline --profile measurement \
  --example production_session_bench --example production_core_survey

target/measurement/examples/production_core_survey > core-survey.jsonl
target/measurement/examples/production_session_bench 31 64 > session-65.json
target/measurement/examples/production_session_bench 31 1024 > session-1025.json
```

If `CARGO_TARGET_DIR` is set, substitute that directory for `target`. Record the actual binary SHA256, host affinity, command, complete stdout/stderr, exit code and source digest. Keep nonzero exits. These commands are bounded correctness and observation-delivery experiments, not a broad external benchmark. Do not infer speed from a debug build, compare different observation schedules, or pool the two schedules into one headline number.

The benchmark calibrates/group-times each method and rotates method order. All three non-restart modes must have identical states and work. Each mode must satisfy the same analytic maximum-observation-error requirement. The core survey preserves every failure row and exits nonzero after printing its summary if any predeclared criterion fails.

## Next broad comparison

`BENCHMARK_PLAN.json` specifies the next local campaign. External comparators are restricted to VODE/DVODE, diffsol and Julia. Missing tools are `NOT_RUN`, not replaced by the repository's small BDF/Radau reference implementations. Pin compiler, versions, tolerances, output times, reference uncertainty and single-thread execution. Compare configurations satisfying the same actual error requirement; retain setup, rejected attempts, failed probes and timeout cost. Any special candidate remains explicitly opt-in unless the registered admission criteria are met.

## Reading and publication

The separate `docs/general_ode_design_20261024/DESIGN_KO.md`, `DESIGN.tex` and `DESIGN.pdf` describe selected methods for external readers. Editorial service responses are stored separately. The audit, unresolved work and algorithm pruning remain in this research folder. `ARTIFACT_MANIFEST.json` checks package files; it does not assert numerical truth. `PUBLISH_MAP.json` connects the local source freeze to the remote publication tree.
