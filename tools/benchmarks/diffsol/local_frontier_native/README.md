# Local-frontier Diffsol native adapter

This isolated Rust runner targets Diffsol 0.16.1 BDF with the Faer backend for the frozen local-frontier descriptor cases 1, 10, 15, and 16. It does not modify the VigilODE root Cargo manifests or the earlier `a07` adapter. `Cargo.toml` and `Cargo.lock` are byte copies of `../a07`; the executable therefore retains the package/binary name `vigilode-a07`.

The runner accepts:

```text
vigilode-a07 --contract FILE --rtol R --atol A --error-limit E
```

Before timing, it checks the supported fixed initial state and model parameters and evaluates every supplied RHS probe (at least two required). It then times fresh Diffsol problem construction, BDF construction, and integration to the descriptor endpoint. It emits one JSON object with solver statistics, endpoint state/time, probe evidence, and typed status. Cases 10, 15, and 16 compare the maximum component endpoint error with the supplied error limit using their analytic endpoint formulas. Case 1 has no independent endpoint reference here and remains `UNVALIDATED_REFERENCE` even when the solver succeeds.

Build offline without changing the lock:

```sh
CARGO_TARGET_DIR=/tmp/vigilode-local-run-20261024.geLVCp9Q/native_diffsol_followup/target \
  cargo build --locked --offline --jobs 2 --profile measurement \
  --manifest-path tools/benchmarks/diffsol/local_frontier_native/Cargo.toml
```

The observed build used Rust 1.94.1. Initial compile failures and the successful repaired build are retained separately in the external run root's `receipts/`. The four one-shot smoke outputs are in `smoke/{1,10,15,16}/`; they are correctness/adapter checks, not timing qualification or a speed ranking. No broad production claim follows from these runs.
