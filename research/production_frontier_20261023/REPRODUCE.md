# Reproduction and evidence reuse

This is a new research branch based on `9b210c76dffb8909902f733e80d6cfa32405006e`. Native code and experiment sources were frozen locally at `55ad7ec6af8b127cb90849db56abff4f02b9920b`. The GitHub equivalent is `df5c6c56da0eb93b58147d6630a0f094cf4af212`; their entire Git trees are identical: `5430c642253cfc9f4b05f9531a6dccafce8eb6c8`. Different commit metadata explains the different commit IDs. The crate-based switching, causal and affine measurements use this first source freeze.

There is one later numerical-source exception: the standalone PDE prototype received the `Q=0`/`t=0` identity-path repair. Its final native campaign uses source `50d3052843d7bbce75bba55a104725c4cfcf6292`, recorded with source and executable hashes in `evidence/pde_campaign_final_receipt.json`. `results/PDE_NATIVE_BENCHMARK_FINAL.jsonl` is the final measured cohort; the earlier PDE cohort is retained separately. Later report/evidence commits do not change the source identity attached to either measurement cohort.

Use a clean, isolated checkout of `research/production-frontier-20261023`; do not reset an existing working branch. Record its actual commit and tree. Existing published campaigns stay archived and are not re-executed merely to refresh a report.

## Environment

Rust/Cargo1.94.1 and the locked dependencies are required. This session recovered them from the supplied offline archives;118registry packages and4382vendor files were checked. `evidence/runtime_recovery/` records the process. Its absolute paths describe this execution host, not a portable installer. Use your own approved toolchain/cache/vendor configuration locally. No NCP is used.

Native correctness was tested with the ordinary debug/test profile. Final optimized campaigns use the repository `measurement` profile, optimization3, LTO disabled, **codegen-units1**, one compiler job, incremental disabled. The initial two-job/codegen16 build failed because an object file had zero length; its log is preserved. Final timings must not be pooled with old profiles or hosts.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_MEASUREMENT_CODEGEN_UNITS=1
export RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
cargo metadata --locked --offline >/dev/null
cargo test --locked --offline -p rodas5p-integrators \
  --test frontier_affine_contracts --test quotient_flow_contracts --test frontier_polynomial
cargo test --locked --offline -p vigilode \
  --test frontier_affine_facade --test lawson_contracts \
  --test frontier_switch_contracts --test switching_contracts
cargo test --locked --offline -p vigilode-coupled --test frontier_causal_trajectory
cargo build --locked --offline --profile measurement \
  -p vigilode -p rodas5p-integrators -p vigilode-coupled \
  --example frontier_switch_bench --example frontier_affine_bench \
  --example frontier_causal_trajectory
```

An old shared build cache once produced an rlib/rmeta format mismatch during validation. Cleaning only the two affected package build caches resolved it; no source or prior research artifact was deleted. This is an environment failure, not a failed ODE solution.

## New measurements

Use a new output directory. Do not overwrite this package's receipts. The runners pin one CPU (two for the optional two-thread causal cohort), preserve failures, enforce time budgets, and reject uncommitted native sources.

```bash
run_dir=$(mktemp -d -t vigilode-frontier-XXXXXXXX)
bench_target=${CARGO_TARGET_DIR:-target}
python3 research/production_frontier_20261023/src/run_switch_frontier.py \
  --binary "$bench_target/measurement/examples/frontier_switch_bench" \
  --output "$run_dir/switching" --repeats 3 --budget 600
python3 research/production_frontier_20261023/src/run_causal_frontier.py \
  --binary "$bench_target/measurement/examples/frontier_causal_trajectory" \
  --output "$run_dir/causal" --repeats 3 --threads 1 --campaign-seconds 600
```

`frontier_affine_bench` emits192rows, including48warmups. Pin its process to one available CPU and preserve stdout/stderr and the executable hash. The exact endpoint method has no tolerance parameter; its duplicated tolerance rows are bookkeeping, not independent evidence. It and adaptive methods all provide the same17observation times. Exact endpoints apply only to declared structure; general residual support differs.

The standalone Rust PDE prototype is intentionally outside the production crate API:

```bash
rustc --edition=2024 --test research/production_frontier_20261023/src/pde_kernels.rs -o "$run_dir/pde_tests"
"$run_dir/pde_tests"
rustc --edition=2024 -O research/production_frontier_20261023/src/pde_native_driver.rs -o "$run_dir/pde_bench"
"$run_dir/pde_bench" 5 > "$run_dir/PDE_NATIVE_BENCHMARK.jsonl"
```

Pin the last process when collecting elapsed timings. Graph/workspace construction and reference evaluation are outside its timed hot loop; allocation instrumentation covers the timed solve. Each geometry/method uses controls selected from the exploratory accuracy sweep. These are calibrated cases, not a held-out general performance guarantee.

## Mathematical and artifact checks

```bash
python3 research/production_frontier_20261023/src/pde_frontier.py --test
python3 research/production_frontier_20261023/src/sphere_pole_split.py --test
python3 research/production_frontier_20261023/src/adversarial_exact_checks.py
python3 research/production_frontier_20261023/src/validate_campaign_contract.py
```

The Python PDE work requires NumPy; figures require Matplotlib. Wolfram/SciSpace/Exa are not needed to execute the native code or exact-rational checks. The existing2D/3D/sphere results and their independent references are already saved.

`BENCHMARK_PLAN.json` specifies the broader local qualification campaign. Its external VODE/DVODE, diffsol and Julia cohorts remain NOT_RUN in this package. Historical comparator results remain historical, and unavailable comparators must not be replaced by fabricated timing or another solver family.


## Verify the published package without rerunning a solver

```bash
python3 research/production_frontier_20261023/src/verify_package.py
python3 research/production_frontier_20261023/src/adversarial_evidence_audit.py
python3 research/production_frontier_20261023/src/analyze_causal_threads.py
```

The first command reads only. The latter two regenerate evidence summaries from existing raw measurements; they perform no integration. `RAW.tar.gz` retains every original raw JSON and stderr file listed in its `RAW_MANIFEST.json`, including failures. The readers stream these archives directly. If individual files are needed, verify the package first, then extract into a **new directory outside this source checkout**:

```bash
raw_dir=$(mktemp -d -t vigilode-frontier-raw-XXXXXXXX)
for campaign in switching_campaign causal_campaign causal_campaign_threads2; do
  mkdir "$raw_dir/$campaign"
  tar -xzf "research/production_frontier_20261023/results/$campaign/RAW.tar.gz" -C "$raw_dir/$campaign"
done
```

The package verifier checks SHA256 and exact archive member coverage without extracting paths. Its success establishes artifact identity and documented count consistency, not solver production qualification. `PUBLISH_MAP.json` maps receipt commits to identical GitHub source trees. The final report commit does not change the measured numerical implementations; report/figure/analysis-helper edits are not new timing evidence.

`results/polynomial_campaign.json` is a pre-freeze deterministic action-count development artifact, without an independent commit/binary receipt. It is not a pinned timing cohort. The final source-frozen polynomial regression suite has eight passing tests in `evidence/native_integrator_tests_final.log`; do not retroactively assign the development artifact a measured-source identity.
