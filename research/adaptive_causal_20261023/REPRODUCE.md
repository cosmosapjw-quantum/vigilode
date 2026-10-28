# Reproduction and source identities

The original baseline is `14c74978b93339a79d7228a324fa77552dae548e`.
The first native snapshot is `548526add83a38a996875b3bd2f3cd3357465ed6`.
The persistent RHS-only correction is `d4e8623bf8add9f505ec018532c4602e5a838a41`.
Subsequent publication commits add research artifacts, without changing native code.

The shell had no GitHub write credential, so the connected GitHub API created
equivalent commits. Commit metadata differs; complete Git trees match exactly.
Use the remote equivalents in `PUBLISH_MAP.json` for checkout:

| Local evidence commit | Remote equivalent |
|---|---|
| `548526add83a38a996875b3bd2f3cd3357465ed6` | `0aeacfff9e95dc7d349b7cf4d0cc1acec60c78a4` |
| `d4e8623bf8add9f505ec018532c4602e5a838a41` | `d2775a7a0c0e5784d190158c2a3159f0f89d4f72` |
| `910dc3860b26739ba3d8330c157d5b2082899f18` | `a25406e0dab28333acbf1cfc17e4d6b0f55a0ddd` |

Raw receipts retain the commits actually used during execution. New runs must
record their actual remote checkout commit; source equivalence does not turn an
old run into a newly executed one.

Existing validated campaigns should be reused. Re-run only changed dependencies or
a deliberately different host/benchmark condition. Never overwrite an old campaign.

## Native verification

Use the repository's locked Rust dependencies and its normal toolchain/cache.
The recorded build used Rust 1.94.1, two compiler jobs, and the existing
`measurement` profile (optimization 3, LTO disabled). A dependency-cache failure
is an environment failure, not a numerical failure.

```bash
export CARGO_BUILD_JOBS=2
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export MKL_NUM_THREADS=1
cargo test -p vigilode --locked --offline --tests
cargo test -p rodas5p-integrators --locked --offline --test quotient_flow_contracts --test quadratic_branch_contracts
cargo test -p vigilode-coupled --locked --offline --lib --tests
```

Final distinct coverage is 122 facade + 17 kernel + 66 coupled tests. The latter
two suites did not change in the persistent-session correction. This is targeted
regression coverage, not a claim to have tested every crate in the workspace.

## Short single-thread campaign

The following runs all nine cases, all three tolerances and all nine routes,
including the new persistent RHS-only route. It contains 972 process records
(one warmup + three timed repetitions of 243 configurations). Select an allowed
CPU with `--cpu` if needed; otherwise the first allowed CPU is selected. The
runner defaults to a 900-second total budget and preserves budget/timeouts.
Analysis requires Python NumPy, SciPy, and Matplotlib for independent references
and figures. Missing packages should be reported explicitly.

```bash
cargo build -p vigilode --example adaptive_causal_bench --profile measurement --locked --offline
benchmark_commit=$(git rev-parse HEAD)
benchmark_target=${CARGO_TARGET_DIR:-target}
benchmark_out=$(mktemp -d -t vigilode-adaptive-validation-XXXXXXXX)
python3 research/adaptive_causal_20261023/src/run_adaptive_bench.py \
  --binary "$benchmark_target/measurement/examples/adaptive_causal_bench" \
  --source-commit "$benchmark_commit" \
  --out "$benchmark_out/campaign" \
  --modes dopri5,rodas-fd,rodas-jvp,switch-fd,switch-jvp,dopri-whole,dopri-whole-dense,rodas-whole-fd,switch-rhs
python3 research/adaptive_causal_20261023/src/analyze_adaptive_bench.py "$benchmark_out/campaign"
python3 research/adaptive_causal_20261023/src/analyze_rhs_followup.py "$benchmark_out/campaign"
```

For only the persistent-session correction use
`--modes switch-rhs,dopri-whole,dopri5` (324 records). `--cases` and
`--tolerances` select documented subsets. The campaign runner rejects unknown
registered modes; source code under `crates`, Cargo.toml and Cargo.lock must match
the supplied commit. No time from this future campaign may be pooled with an
older executable's timing.

The declared-flow and causal examples are independent cohorts:

```bash
cargo build -p rodas5p-integrators --example quotient_flow_bench --profile measurement --locked --offline
cargo build -p vigilode-coupled --example causal_bench --profile measurement --locked --offline
```

Their exact invocations, CPU affinity, executable hashes and source hashes are
in `results/quotient_run_receipt.json` and `results/causal_benchmark_receipt.json`.
The causal example measures fixed-step raw-U root work; retain the embedded
physical acceptance field when interpreting it.

## Evidence checking without re-running numerical experiments

```bash
python3 research/adaptive_causal_20261023/src/independent_benchmark_review.py
python3 research/adaptive_causal_20261023/src/make_report_figures.py
```

The first script reconciles existing raw records with summaries and verifies
cross-method parity. It does not rerun the published solver campaigns. The
manifest can be checked using `src/verify_manifest.py`; a successful hash check
establishes file identity only, not mathematical validity.

## Broader next campaign

`BENCHMARK_PLAN.json` defines the expanded suite, independent reference strategy,
hardware conditions and required outputs. VODE/DVODE, diffsol and Julia are the
only external comparator families. This broad external campaign is **not run**
in this package. The VODE Robertson reference in the short campaign is an
accuracy oracle, not a VODE speed comparison. Historical external timings remain
historical; unavailable comparators must be marked NOT_RUN, never simulated.
