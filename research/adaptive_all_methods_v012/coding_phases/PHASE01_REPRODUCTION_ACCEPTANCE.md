# Phase 1 — Reproduction and Acceptance Baseline

## Baseline

- Commit: `ea7303c280d9c2e4e8769d06f5660a42361318aa`
- Branch source: `unified-adaptive-output-v0.11`
- Rust: `1.94.1`
- Cargo mode: `--offline --locked --profile measurement`

## Fresh baseline command

```bash
cargo test --workspace --profile measurement --offline --locked -- --test-threads=1
```

Result: PASS.  Existing adaptive Sequential/SABR tests, fixed BDF/Radau anchors, common-output
observer, global-error authority, Krylov, SABR, and Homotopy suites are green.

## Acceptance observables

1. Common controller factor formulas and validation.
2. Unequal-step BDF2 quadratic exactness.
3. Adaptive BDF/Radau/Homotopy output-grid completion.
4. Transactional rollback after forced rejection/failure.
5. Expected order/tolerance response on analytic problems.
6. Fixed-step public regression unchanged.
7. Deterministic unified adaptive report in one and four threads.
