# Phase 6 — Software validation

Date: 2026-08-07
Code HEAD validated: `db13d42657ccc1fee07774ffb26d49dfd4214813`

## Environment

- Rust toolchain: `rustc/cargo 1.94.1`
- linear algebra: `faer 0.24.4`, existing workspace configuration
- build mode: release, offline, locked
- arithmetic: IEEE-754 binary64
- execution controls: `RAYON_NUM_THREADS=1`, `OMP_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, `MKL_NUM_THREADS=1`

The repository-local Cargo source mapping resolves to the verified offline crate bundle. No dependency was added by this cycle.

## Commands and results

```text
cargo fmt --all -- --check
PASS

cargo clippy --workspace --all-targets --release --offline --locked -- -D warnings
PASS

cargo test --workspace --release --offline --locked -- --test-threads=1
PASS: 33 contract/integration tests, 0 failures

cargo build --workspace --release --offline --locked
PASS
```

The six new integrator research tests cover parameter/shape rejection, endpoint identities, the affine FLRH limit, exact full nilpotent inverse action for noncommuting `M,J`, low-depth causal truncation, and original-output certification. The new CLI contract verifies schema, required screens, and byte determinism.

## Deterministic CLI

Two independent invocations of

```text
rodas5p homotopy-design-check --output <file>
```

produced byte-identical JSON with SHA-256

```text
cc6cb1c963e75415f5b35ba879d83b081a08fd7103e0f2cc27c234e63ec7743a
```

The machine-readable result is `research/homotopy_loop2/results/HOMOTOPY_DESIGN_CHECK.json`.

## Protected-method regression

The full pre-existing test suite passed, including:

- fifth-order fixed-step RODAS5P convergence;
- sequential direct/Krylov stage parity;
- adaptive rejection and recovery;
- SABR affine fast path and transactional fallback;
- Fair A/B trace/budget/certificate contracts;
- GMRES/LGMRES/GCRO-DR invariants and allocation contracts.

The base-to-head changed-file list contains no edits to `sequential.rs`, `sabr.rs`, `block.rs`, coefficient fixtures, Krylov kernels, or Fair A/B implementation.

## Failure handling

Invalid parameter ranges, dimensions, non-finite input, invalid truncation depth, and failed LU actions return typed errors. No catch-all exception, silent clipping, or speculative fallback was added.

## Evidence

- `receipts/FULL_VALIDATION_STAGE1.log`
- `receipts/TASK2-RED.log` through `receipts/TASK5-GREEN.log`
- `results/HOMOTOPY_DESIGN_CHECK.json`
