# Validation harness failure analysis

The first full pre-commit validation completed the Rust tests and numerical checks, then exited with status `141` in the source-audit section.

Root causes:

1. `objdump -d ... | grep -Eq ...` ran under `set -o pipefail`. Once `grep -q` found a vector instruction and closed the pipe, `objdump` received `SIGPIPE`; the pipeline therefore reported `141` despite positive SIMD evidence.
2. The initial `! grep -R 'unsafe {' crates` expression printed intentionally allowed `unsafe` blocks from the allocation-audit executable and allocator-counting tests, while the `!` conditional context prevented `set -e` from enforcing the intended scope.

The corrected validation:

- writes `objdump` output to a temporary file before searching it;
- checks production `crates/*/src/*.rs` while excluding only the dedicated `src/bin/allocation_audit.rs` exception;
- stores `ldd` output before checking for BLAS/LAPACK/MKL dependencies;
- sets `TERM=dumb` to avoid terminal-control noise.

No numerical or production-source change was needed. The corrected full validation completed with exit status `0` and `FINAL_VALIDATION=PASS`.
