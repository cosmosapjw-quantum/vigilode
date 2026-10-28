# Task 7 report: p0-api (branch production/p0-api, base 6594d10)

Status: DONE_WITH_CONCERNS (see Concerns)

## Commits
- d816b9f RED: new tests, golden Stats JSON (generated from the untouched tip), compile_fail doctest
- 4945683 GREEN: implementation, test edits, example fix. Pushed.

## Requirements -> file:line
- Unsupported enum (7 variants, Display = former text): crates/vigilode/src/error.rs:40; `SolveError::Unsupported(Unsupported)` error.rs
- Refusal sites (7, `grep unsupported(`): solve.rs:677, 680, 686, 689, 692, 696, 725
- Re-export: lib.rs `pub use error::{.., Unsupported}`
- #[non_exhaustive]: 20 types (Tier, InteriorError, Jacobian, FailureReason, Status, SolveError, CallbackError, Callback, JacobianRecord, DriverWork, Direction, Unsupported, Stats, Solution, RouteRecord, StopRecord, CallbackWork, FiniteDifferenceWork, KrylovStartRecord, EventRecord). Not Options/Atol.
- Stability decision text + compile_fail doctest: lib.rs "## Stability" (line 115)
- Tests: tests/p0_api_contracts.rs (one per variant, display_text_is_the_former_message, stats_json_unchanged vs tests/golden/robertson_stats.json). Existing refusal assertions in facade_contracts.rs, p0_contracts.rs, events_contracts.rs now match specific variants.

## Rulings
- R-C5: no bench change needed. R-C7: examples build; Robertson fixed (below). FP4: bench `cargo check --locked -j 2 --all-targets` (CARGO_TARGET_DIR=/home/claude/targets/e1b-dense) passes. Bench doesn't match on these enums at this base.
- Carried atol Minor 1: chose documentation only (smaller change). `Stats.adaptive` doc (solution.rs) now says: with a vector atol its `atol` is the largest entry; the per-component values are in `options.atol` (already recorded in Stats). No new field; uniform-vector bit-identity untouched.
- Robertson example: check is now per-component relative, 1e-4 (observed errors 3.6e-7, 1.1e-6, 9.1e-7; the y1 reference has 7 digits). Old tolerance 1e-5 abs + 1e-3 rel could not catch y1 (9.2e-6).
- Seam: FastDense variant compiles under non_exhaustive; Stats schema v3 unchanged (golden is v3).

## Test evidence
- RED: p0_api_contracts fails to compile (unresolved import vigilode::Unsupported); doctest "compiled successfully, but it's marked compile_fail".
- GREEN: `cargo test -p vigilode --locked -j 2`: 9 + 15 + 20 + 9 + 21 + doctests 2 (lib + compile_fail) all pass. Clippy `-p vigilode --all-targets --no-deps -D warnings` (as production.yml) clean; `cargo build -p vigilode --examples` ok; `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode --no-deps` ok; `cargo fmt --check -p vigilode` clean; check-research-node PASS (141 rows).
- Used -j 2 per dispatch (overrides contract's -j 1).

## Deviations
- Unsupported variants are unit variants, not struct variants; tests match `Unsupported::X` (brief said `X { .. }`; clippy unneeded_struct_pattern rejects braces on unit variants). Adding fields later is non-breaking only if the variant is also non_exhaustive... note: unit variants can't gain fields without a break. Reason for the choice: no payload exists today.
- Unsupported derives Clone, Copy, PartialEq, Eq (so tests can assert_eq).

## Handoff commands
None needed beyond CI: `cargo test -p vigilode --locked`, `cargo test --doc -p vigilode --locked`.

## Open questions / concerns
- Unit variants: if the owner wants payloads later, that is a breaking change to the variant shape despite #[non_exhaustive] on the enum. Cheap to switch to `{}`-style variants now if preferred.
- Full `clippy -D warnings` without --no-deps still fails in rodas5p-core (pre-existing, as the workflow notes).
