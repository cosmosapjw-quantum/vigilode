# rodas5p-rs

Rust parity reconstruction of the pre-homotopy RODAS5P numerical-method research stack.

## Workspace

- `rodas5p-core`: coefficients, norms, matrices, operators, counters, direct oracle.
- `rodas5p-krylov`: common kernels, restarted GMRES, LGMRES, fixed-left GCRO-DR.
- `rodas5p-integrators`: protected sequential/adaptive RODAS5P, block system, SABR5P fallback.
- `rodas5p-fair-ab`: immutable traces, solver adapters, hard budgets, certification, recycle lifetimes.
- `rodas5p-cli`: `validate`, `trace`, and `benchmark` commands.

Homotopy and continuation algorithms are intentionally excluded from this checkpoint.

## Locked toolchain

- Rust `1.94.1`
- `faer 0.24.4`
- offline and locked Cargo builds
- release profile: `opt-level=3`, fat LTO, one codegen unit, abort-on-panic

## Build and validate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
cargo test --workspace --release --offline --locked
cargo build --workspace --release --offline --locked

target/release/rodas5p validate --output validation.json
```

## Generate a deterministic trace

```bash
target/release/rodas5p trace \
  --kind fixed \
  --dimension 128 \
  --steps 4 \
  --stages 8 \
  --seed 20260806 \
  --stiffness 1000 \
  --nonnormality 0.2 \
  --output fixed_trace.json
```

Trace kinds are `fixed`, `slow-drift`, `abrupt`, and `rotating`.

## Strict Rust-only benchmark

```bash
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
export RAYON_NUM_THREADS=1 FAER_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
export NUMEXPR_NUM_THREADS=1

taskset -c 0 target/release/rodas5p benchmark \
  --trace fixed_trace.json \
  --output fixed_benchmark.json \
  --repetitions 5 \
  --warmups 1 \
  --restart 20 \
  --recycle-dim 6 \
  --operator-budget 4000 \
  --rtol 1e-9 \
  --atol 1e-12 \
  --preconditioner none
```

All solver arms run in the same executable and use the same Rust vector-kernel layer, operator
abstraction, initial guess, residual certificate, and operator-action budget.

## Current disposition

- **PROMOTE:** Rust workspace and R10-A allocation/workspace changes as the pre-homotopy baseline.
- **HOLD:** unconditional recycling and GCRO-DR as a default performance backend.
- **DEFER:** `(m,k)` tuning, adaptive recycling, nonlinear trace expansion and BDF/Radau ranking.
- **NEXT ACTIVE BRANCH:** homotopy-based acceleration of the lower-triangular nonlinear stage-coupling system.

See `research/closeout/PRE_HOMOTOPY_BASELINE_CLOSEOUT_KO.md` and
`research/HOMOTOPY_BRANCH_HANDOFF_KO.md`.
