# Reproduce the scoped homotopy result

Run from the repository root with Rust1.94.1 and the locked dependencies.
Keep the scientific inputs in `cubic_production_bench.rs` unchanged. A populated
cache permits `--offline`; dependency acquisition belongs to environment setup.

```bash
cargo test -p vigilode-coupled --profile measurement --locked --offline \
  --test rhs_homotopy_contracts --test cubic_solver_production_contracts
cargo build -p vigilode-coupled --profile measurement --locked --offline \
  --example cubic_production_bench
target/measurement/examples/cubic_production_bench > local_cubic_audit.json
target/measurement/examples/cubic_production_bench --bench 128 > local_cubic_timing.json
```

If CARGO_TARGET_DIR is set, replace `target` by that directory. The current
campaign used `/workspace/scratch/06cfffc081e2/tmp/vigil_target`, two build jobs,
and a shared execution flock; all commands/source/binary hashes are recorded in
`PASS1_EXECUTION.json`. Keep local outputs separate from these cloud receipts.

The executable's profiling interface is deterministic:

```bash
target/measurement/examples/cubic_production_bench --arm generic 20
target/measurement/examples/cubic_production_bench --arm generic 40
```

Repeat those two repetition counts for `scalar`, `algebraic`, and `rational`.
Run each through a locally installed Callgrind with cache and branch simulation
disabled and save a different output file for every arm/count. Subtract the
20-run instruction total from the 40-run total, then divide by 20. Include the
whole successful trajectory, output allocation and original interval verifier;
do not compare the profiler's slow instrumented wall time to native wall time.
Current exact commands are in `instruction_profile_r1/RUNS.json`.

All 24 audit arms must finish with every returned point below its own outward
physical bound and the unchanged model cap. Scalar/generic bit identity and
equal work are independent test contracts. Candidate-specific predictor counts
must accompany polynomial counts. Resource exhaustion and rounding-floor tests
must retain the accepted prefix and all entered evaluations.

For the existing captured raw files, run

```bash
python3 research/production_solver_closure_20261027/homotopy/analyze.py
```

This recomputes Decimal80 exact-ODE comparisons, exact Fraction identities and
paired timing summaries. It does not regenerate or relabel the measurements.
Local campaigns should adapt its input/output filenames explicitly and preserve
the original cloud baseline. The task used correction pass 1 of a maximum 3;
remaining passes 2 and 3 require a concrete new diagnosis, not repeated timing
until a favorable number appears.
