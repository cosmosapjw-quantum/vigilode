# Verification record

## TDD evidence

The first test compile failed because the three new public session types did
not exist. After implementation, the session contracts passed. A separate
readiness-coverage test first failed because the new Rust contract was absent
from `tools/check-audit2-readiness.sh`; after the script was updated, the
Python scope suite passed 12/12.

## Fresh commands actually executed

On the exact local e77 tree plus this worktree delta:

```text
cargo test --offline --locked -p rodas5p-integrators \
  --features audit2-research \
  --test audit2_matrix_free_common_w_contracts -- --nocapture
exit 0; 5 passed

cargo test --offline --locked -p rodas5p-integrators \
  --features audit2-research \
  --test audit2_structured_correction_contracts -- --test-threads=1
exit 0; 11 passed

cargo fmt --all -- --check
exit 0

cargo clippy --offline --locked \
  -p rodas5p-integrators -p rodas5p-fair-ab --all-targets \
  --features rodas5p-integrators/audit2-research -- -D warnings
exit 0

AUDIT2_OUTPUT_DIR=<temporary> bash tools/check-audit2-readiness.sh
exit 0
```

The readiness command executed 20 Python tests and 60 Rust tests:

```text
8 global-error
9 output-accuracy
6 matrix-free common-W
11 pre-existing Audit-2 correction
15 dense-output
6 homotopy
5 solve_stiff example
```

It also built the research-off integrator, ran affected clippy/format checks,
and exercised both complete and deliberately exhausted default-solver example
paths.

## Mutation

The first stage-dependent correction coupling was changed from

```text
corrected += h * J_i p_i
```

to

```text
corrected -= h * J_i p_i
```

The focused compatibility contract failed with exit 101:

```text
relative difference  4.899133405529948e-1
fixed bound           1.2660497088853964e-7
```

After exact source restoration, the same focused test exited 0 and recovered
relative difference `4.760950091765468e-16`.

## What was not executed

- exact c5/PR31 local checkout and its 15-test original-target bridge suite;
- full workspace tests;
- wall timing or performance ranking;
- production/default routing;
- real physical client, independent observable budget, or holdout;
- GitHub branch/PR/merge write operations.

The exact c5 replay is not optional. It is the first publication gate because
this local source was reconstructed from e77. The patch touches no PR31-changed
path, which reduces merge risk but does not substitute for execution.
