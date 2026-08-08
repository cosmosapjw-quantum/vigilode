# Phase 6 — Software Validation

## Commands

```text
cargo fmt --all -- --check
cargo clippy -p rodas5p-integrators -p rodas5p-cli --all-targets --offline --locked -- -D warnings
cargo test --workspace --release --offline --locked -- --test-threads=1
cargo build --workspace --release --offline --locked
```

## Results

- Full release workspace and doctest coverage: 82 passed, 0 failed.
- Strict Clippy for affected crates: PASS.
- Release build: PASS.
- Native CLI gate executed twice with byte-identical JSON.
- Unified smoke candidate screen completed at 1 and 4 threads with the same scientific checksum.
- `git diff --check`: PASS.

## Regression handling

Two harness-level issues were encountered and resolved:

1. New complete-integrator variants made old RODAS-stage match statements non-exhaustive. Root cause: one registry was being used by screens with different mathematical domains. Resolution: classify stage candidates versus complete integrators, exclude native methods from stage-only execution, and expose an explicit native-gate blocker in the old report.
2. The first full foreground test command exceeded the command wrapper timeout. The same exact test command was run in the background and completed with exit 0; no partial result was used as completion evidence.

No broad exception handling or silent fallback was introduced.
