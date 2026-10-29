# Reproduce the reconsideration package

Use the commit publishing this directory and a clean checkout. Keep a different
working checkout intact. Rust/Cargo 1.94.1 and this repository's `Cargo.lock`
identify the tested toolchain/dependencies. Offline commands require the matching
Cargo cache or vendor configuration. No NCP account or remote compute is used.

The historical `general_ode_selection_20261024` package refers to its original
published tree. Its manifest is deliberately preserved; validate that historical
package at its original commit, rather than expecting its hashes to describe the
newly modified source and document.

```bash
export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_TEST_CODEGEN_UNITS=1
export CARGO_PROFILE_MEASUREMENT_CODEGEN_UNITS=1
export RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
cargo test --locked --offline -p vigilode-coupled --tests
cargo test --locked --offline -p rodas5p-integrators \
  --test polynomial_refinement_contracts --test frontier_polynomial \
  --test frontier_polynomial_provider
cargo test --locked --offline -p vigilode --tests
python3 research/candidate_reconsideration_20261024/src/check_theory.py
cargo build --locked --offline --profile measurement \
  -p vigilode-coupled --example reconsideration_bench \
  -p rodas5p-integrators --example algebraic_refinement_bench
```

Use your actual Cargo target directory below; the commands assume its default.
Raw stdout is JSONL and must be retained even for failed/inaccurate rows.

```bash
mkdir -p /tmp/vigilode-reconsideration-results
export VIGILODE_SOURCE_COMMIT=$(git rev-parse HEAD)
for case in few-mode expensive-few-mode broad riccati nonnormal oscillatory changing-rate; do
  target/measurement/examples/reconsideration_bench --case "$case" --repeats 5 \
    > "/tmp/vigilode-reconsideration-results/causal-$case.jsonl"
done
target/measurement/examples/reconsideration_bench --case few-mode --basis-limit 1 --repeats 5 \
  > /tmp/vigilode-reconsideration-results/causal-few-mode-basis1.jsonl
for case in few-mode expensive-few-mode riccati; do
  for threads in 1 2; do
    target/measurement/examples/reconsideration_bench --case "$case" --repeats 5 \
      --root-variants --threads "$threads" \
      > "/tmp/vigilode-reconsideration-results/roots-$case-$threads.jsonl"
  done
done
target/measurement/examples/algebraic_refinement_bench \
  > /tmp/vigilode-reconsideration-results/algebraic.jsonl
```

The root-variant cases explicitly allocate the requested stage workers. They are
separate from the single-thread primary comparison. Report thread budget,
same-method scaling, and comparison to the serial protected alternative.

The primary experiment uses seven workload cases from six analytic ODE families
(the expensive-RHS case duplicates the few-mode ODE), plus a basis-limit-1 control.
Each case uses 17 physical output
landings, `rtol=1e-6`, `atol=1e-8`, and a sampled absolute-error cap of `1e-5`.
The same settings apply to every route. Failed or incomplete rows cannot enter
speed ratios. The expensive-RHS case deliberately adds computation without
altering the ODE; it is a synthetic cost crossover experiment.

The primary executable includes both the final priced switching policy and the
legacy exponential policy in the same cyclic measurement. With five repetitions,
the eight primary configurations produce 275 timed trajectories; the six
root-variant configurations produce 240. Run the analyzers below to retain all
rows and exclude inaccurate or incomplete rows from speed comparisons. The
analyzers require Python, NumPy and Matplotlib. They create new analysis files in
the scratch result directory and do not overwrite the published evidence.

```bash
python3 research/candidate_reconsideration_20261024/src/analyze_causal_benchmark.py \
  /tmp/vigilode-reconsideration-results/causal-*.jsonl \
  --output /tmp/vigilode-reconsideration-results/analysis
python3 research/candidate_reconsideration_20261024/src/analyze_root_variants.py \
  /tmp/vigilode-reconsideration-results/roots-*.jsonl \
  --output /tmp/vigilode-reconsideration-results/analysis
python3 research/candidate_reconsideration_20261024/src/verify_package.py
```

The algebraic experiment includes the former one-shot polynomial, bounded
refinement, dense LU with and without reusable factorization, and an exact small
block comparator where applicable. Its manufactured trajectory uses fixed-step
implicit Euler to isolate the linear-algebra change. It is not a fifth-order
RODAS trajectory measurement.

Commands executed here, source hashes, executable hashes where applicable,
elapsed time, exit codes and raw streams are in `evidence/runs/`. The two cache
recovery operations touched only task-local Cargo build outputs. Their preceding
archive/metadata failures remain in the package. They are environment failures,
distinct from numerical rejection and the development revision limit.
