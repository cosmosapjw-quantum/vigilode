## Task 7: P0-api — typed refusals and `#[non_exhaustive]` (node id `p0-api`, stage 2)

Branches from `production/wave3-integration` after Tasks 3, 4 and 5 merge, so every refusal site exists.
Implements the decision above. No arithmetic change.

**Files:** `crates/vigilode/src/{error.rs, solve.rs, options.rs, problem.rs, solution.rs, events.rs, lib.rs}`,
`crates/vigilode/tests/*.rs`.

**Interfaces (new):**
- `SolveError::Unsupported(Unsupported)`, replacing `Unsupported(String)`.
- `#[non_exhaustive] pub enum Unsupported`, with one variant per distinct refusal site in `solve.rs`
  (enumerated at implementation time from `grep -n Unsupported`). `Display` equals the current message text.
- `#[non_exhaustive]` on `Tier`, `InteriorError`, `Jacobian`, `FailureReason`, `Status`, `SolveError`,
  `CallbackError`, `Callback`, `JacobianRecord`, `DriverWork`, `Direction` and `Unsupported`, and on the
  output structs `Stats`, `Solution`, `RouteRecord`, `StopRecord`, `CallbackWork`, `FiniteDifferenceWork`,
  `KrylovStartRecord` and `EventRecord`. Not on `Atol` or `Options`.

- [ ] **Step 1: Write failing tests:**
  - every existing refusal assertion changes from string matching to
    `matches!(e, SolveError::Unsupported(Unsupported::<Variant> { .. }))`, one test per variant;
  - a `compile_fail` doctest showing that an exhaustive `match` on `FailureReason` outside the crate does not
    compile;
  - `stats_json_unchanged`: the serde JSON of a Robertson run's `Stats` is byte-identical before and after.
- [ ] **Step 2: Run** `cargo test -p vigilode --locked -j 1`; expect failures.
- [ ] **Step 3: Implement**, and record the decision text in the crate docs (`lib.rs` "Stability" section).
- [ ] **Step 4: Run** `cargo test -p vigilode --locked -j 1`, the clippy job of Task 6, and
  `cargo build --manifest-path crates/vigilode-bench/Cargo.toml --locked -j 1` (the bench matches on facade
  types); all pass.

