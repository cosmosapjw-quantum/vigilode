# Reproduce the derivative-free causal / homotopy study

## Scope and source identity

The base is `68a28357d960c4c2743c299770f0a33ef63245b2` (PR90).
The branch changes only the code and research node listed in the manifest.
Execution receipts bind actual source and binary hashes. The source archive
also contains exact old control snapshots where a subsequent extension changed
code; do not attribute an old measurement to a later unexecuted binary.

Use Rust 1.94.1, the existing `Cargo.lock`, and the project's offline dependency
cache. `--profile measurement` means opt-level 3, LTO false, 16 codegen units.
On the execution host the paths were:

```sh
export PATH=/workspace/scratch/06cfffc081e2/tmp/rust1941/bin:$PATH
export CARGO_HOME=/workspace/scratch/06cfffc081e2/tmp/cargo_home
export CARGO_TARGET_DIR=/workspace/scratch/06cfffc081e2/tmp/vigil_target
export CARGO_BUILD_JOBS=2
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
```

Independent future hosts should point these variables to their own pinned
toolchain/cache. Avoid running timed measurements simultaneously with builds,
other agents, or each other. Every native receipt states the actual command.

## Tests and consumers

From the repository root, the focused coupled contracts are:

```sh
cargo test --profile measurement --locked --offline -p vigilode-coupled \
  --test causal_contracts --test causal_reuse_contracts \
  --test switching_contracts --test structural_mean_field_contracts \
  --test rhs_homotopy_contracts -- --test-threads=1
```

Adapter receipt commands run `entry_checkpoint_seed_contracts`,
`evidence_switching_contracts`, and the private arithmetic/helper filters.
Their exact test names and authoritative logs appear in the validation summary.
The final cross-target compile is:

```sh
cargo check --profile measurement --locked --offline \
  -p vigilode -p vigilode-coupled --all-targets
```

The current native examples are:

```sh
cargo run --profile measurement --locked --offline -p vigilode-coupled --example causal_reuse_bench
cargo run --profile measurement --locked --offline -p vigilode-coupled --example rhs_homotopy_bench_20261027
cargo run --profile measurement --locked --offline -p vigilode-coupled --example structural_mean_field_bench_20261027 -- 2048 64 5 1
cargo run --profile measurement --locked --offline -p vigilode-coupled --example structural_bound_bench_20261027
cargo run --profile measurement --locked --offline -p vigilode-coupled --example cubic_accuracy_bench_20261027
cargo run --profile measurement --locked --offline -p vigilode --example selective_jvp_20261027
```

Consult `causal/PROTOCOL.md` for the separate work and timing command arguments;
the default invocation is not a substitute for the stored full grid protocol.
`evidence/run_structural.py` records the initial structural experiment,
`evidence/run_exact_structural.py` runs only its derived-bound extension.
`homotopy/run_native.py` and its execution/replay receipts identify the initial
controls. The final adapter strict integer results are authoritative in
`adapter/EXACT_WORK_RESULTS.json`; earlier initial/seed/parity results are
explicitly historical controls. Reproduction of those exact old arms should
use the matching archived source snapshots, not silently replace code.

## Exact mathematics and analysis without rerunning native experiments

```sh
python3 research/derivative_free_causal_homotopy_20261027/theory/exact_checks.py
python3 research/derivative_free_causal_homotopy_20261027/theory/exact_constant_checks.py
python3 research/derivative_free_causal_homotopy_20261027/evidence/exact_structural_constants.py
python3 research/derivative_free_causal_homotopy_20261027/report/generate_results.py
```

The last command reads stored measurements and draws figures. It performs no
solver run. Fraction checks use exact rationals of represented binary64
tableau coefficients. The cubic native physical bound instead uses the
repository's directed arithmetic, with its explicitly stated binary64/FMA,
gradual-underflow and compiler-association assumptions.

## Evidence archive and report

Large row files, duplicate execution output and measured source snapshots are
stored in `evidence/raw_measurements.tar.xz` in the review commit. The source
deliverable also includes their expanded originals. Extract the archive into
the repository root before running the analysis scripts if using only the
compact review commit. The archive's member names are relative to that root;
the manifest records every member hash. No solver binaries, build caches,
credentials, or machine configuration are required in the source deliverable.

The REVTeX report uses unmodified REVTeX 4.2 support files. The accompanying
original distribution and its licensing material are included in the source
deliverable. Build the genuine REVTeX source with:

```sh
bash research/derivative_free_causal_homotopy_20261027/report/build_report.sh
```

The final PDF was text-extracted, checked for LaTeX errors/undefined references,
rendered to page images, and visually reviewed. Precise artifacts and source
hashes appear in `MANIFEST.json` and the report build/visual audit receipt.

Read `RESEARCH_LOOPS.md`, `README_KO.md`, and the lane reports for the scope of
each claim. Root-attempt timing, accepted physical-trajectory work, deterministic
proxy cost, directed error bounds, and ordinary f64 residual diagnostics are
different quantities and remain labeled separately.
