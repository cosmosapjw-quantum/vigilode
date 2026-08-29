# VigilODE

**Causal, Budgeted Jacobian-Free Polyalgorithms for Stiff and Oscillatory ODEs**

VigilODE is a research platform for generic stiff and oscillatory ordinary differential equations. The project studies evidence-gated polyalgorithm design that combines a protected matrix-free Rosenbrock path with guarded exponential-Rosenbrock candidates while preserving complete speculative-work accounting.

## Current research scope

The current program focuses on:

- matrix-free, Jacobian-free stiff integration;
- explicit-Jacobian-free and Newton-free fast-path candidates;
- resumable Krylov / exponential-Rosenbrock stage prefixes;
- causal regime telemetry and event-conditioned method admission;
- tolerance-aware nonlinear-flow diagnostics;
- transactional speculative-work budgets;
- same-error, same-output, failure-preserving performance comparisons.

Active method switching is treated as a research target rather than a completed production capability. Safety, fifth-order recovery after real switches, controller/cache transfer, and full same-error economics remain explicit validation gates.

## Development policy

Scientific claims are evidence-gated. Research branches retain failed, speculative, fallback, and diagnostic work in their ledgers. Comparator implementations are not silently tuned to improve headline results, and holdout problems are not used to retune core policy thresholds.

## Scientific-validity boundary

The internal BDF1/BDF2/RadauIIA1/RadauIIA3 solvers are reference implementations, not competitive production baselines. Existing receipts support no comparison with RADAU5, CVODE, OrdinaryDiffEq, SciPy, or SUNDIALS. The synthetic stage-batch/parallel probe measures thread-pool and compute-bound overhead only; it does not predict memory-bandwidth-bound JVP scaling.

`AcceptedSteps` and `InternalSteps` count accepted state-advancing substeps, not
adaptive controller macro-attempts. Thus the explicit BDF startup and the Radau IIA1
step-doubling path count their two retained half steps; their discarded coarse probe is
not counted. Steady-state BDF1/BDF2 and embedded Radau IIA3 count one substep per
accepted macro-step. Adaptive diagnostics separately expose accepted/rejected macro-step
counts, so these axes must not be treated as interchangeable.

The scientific-validity-v2 implementation is locally committed at
`ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`. Its source-bound canonical calibration
completed all 54 cases without an execution failure, but all 54 rows were classified
`output-policy-dominated`. The pass-only freeze was therefore not created and the sealed
Oregonator holdout was not opened. External calibration retained 54 successful SciPy
Radau records and 54 typed-unavailable CVODE records; it is not a complete production
baseline. Consequently no v2 performance, scaling, ranking, equal-error, or publication
claim is admitted. Legacy v3.5/v3.6/v3.7 receipts remain valid only under their original
corpus, comparator, inner-solve, and output policies and are not transplantable to v2.

## Build reproducibility

A normal fresh clone uses Cargo's default crates.io configuration:

```bash
cargo metadata --locked --format-version 1
cargo test --workspace --all-targets --no-run --locked
```

No repository-external sibling directory is required by default. Network access is needed only when the pinned registry artifacts are not already present in the local Cargo cache.

For air-gapped or deliberately offline development, create a standard Cargo vendor directory and opt in explicitly:

```bash
cargo vendor --locked /absolute/path/to/vendor
bash ./tools/cargo-offline.sh \
  --vendor-dir /absolute/path/to/vendor \
  metadata --frozen --format-version 1
bash ./tools/cargo-offline.sh \
  --vendor-dir /absolute/path/to/vendor \
  test --workspace --all-targets --no-run --frozen
```

`tools/cargo-offline.sh` accepts either `--vendor-dir PATH` or the `VIGILODE_CARGO_VENDOR_DIR` environment variable. The vendor directory must be a standard Cargo directory source created by `cargo vendor`; the wrapper validates its structure, uses an isolated temporary Cargo home with an absolute vendor path, and leaves `Cargo.toml`, `Cargo.lock`, and tracked Cargo configuration unchanged.

The repository keeps `vendor/` untracked. `.cargo/config.offline.toml` is a documented template for users who prefer a repository-local `vendor/` directory and an explicit manual Cargo configuration.

## License

MIT
