# Task 5 report: A1-api (`vigilode` facade crate)

Status: DONE_WITH_CONCERNS
Branch: production/a1-api (from 9b5d083), pushed.
Commits: 53a7c42 (crate + tests + workspace member + lock), 1da0425 (node doc), f76d60f (isolated-dir re-run section).
Toolchain: RUSTUP_TOOLCHAIN=stable, cargo 1.97.0 / rustc 1.97.0.

## Isolated target dir
Every recorded result (cargo test -p vigilode incl. doctests, clippy, build) was re-run in
CARGO_TARGET_DIR=/home/claude/targets/a1-api from a clean build: same pass counts, measured errors
identical to every digit. Only the RED stub run was not repeated.

## Self-review
- Bit identity: no arithmetic is added to the success path; the guard only forwards and counts. Tests use to_bits on the Robertson (dense fast driver), the solve_stiff problem (MF driver), and the user band (banded driver), plus Performance vs Protected and a serde-replayed run.
- Accounting: FD calls are counted on attempt and merged into counters; the driver's own counters are kept separately; short-circuited calls after a fatal error are counted in total and per RHS.
- Refusals never fall back silently. The RED run's refusal test passed only because the stub returned Unsupported (recorded).
- Found and fixed during GREEN: the user-entry accounting had first subtracted all short-circuited calls (Jacobian ones included); added short_circuited_rhs_calls.

## Concerns / decisions for the controller (also in the node doc's deviations list)
1. dense_output=true is refused as Unsupported: no allowed driver returns per-step states without changing its step sequence. So a failed run's partial solution holds only t0 (the failure time sits in FailureReason::CallbackFailed.t, and the work in stats). Wave 2 should decide what dense output means (per-step states vs sampled interpolant).
2. .mass() is refused as Unsupported: every allowed protected driver refuses a non-identity mass.
3. Atol::Vector is refused as Unsupported, as the brief says (AdaptiveStepConfig.atol is scalar-only).
4. Added a .autonomous() builder (not in the brief), needed for bit identity on autonomous problems.
5. The callback type is Result<(), CallbackError{Recoverable, Fatal}>. A failed run is Ok(Solution{status: Failed(..)}); Err is reserved for refusals and driver errors that leave no partial result. Solution::ensure_success converts.
6. max_steps maps to max_attempts (accepted + rejected). The small-N const-generic driver is not routed. JVP + .banded is refused.
7. If a driver returns Err for a non-rejection error after a fatal callback, no partial result exists (SolveError::Driver with the fatal message). The RHS path is covered by the tests; JVP failures inside GMRES were not tested.

## Commands and results

| Command | Exit | Result |
| --- | --- | --- |
| `cargo metadata --locked` (after adding the member) | 101 | lock needs update (expected) |
| `cargo update -p vigilode --offline` | 0 | no-op: "package ID specification `vigilode` did not match" (not yet in the lock) |
| `cargo metadata --offline --format-version 1` | 101 | wrote the lock entry, then failed downloading metadata-only crates for other platforms |
| `cargo metadata --locked --format-version 1` | 0 | lock accepted |
| `git diff --stat Cargo.lock` | 0 | `+11` lines: only the `[[package]] name = "vigilode"` entry |
| `cargo test -p vigilode --locked -j 1 --test facade_contracts` (RED, stub `solve`) | 101 | 1 passed, 13 failed (the refusal test passed trivially against the stub, which returned `Unsupported`) |
| `cargo test -p vigilode --locked -j 1` (GREEN) | 0 | lib 3 passed; `facade_contracts` 14 passed; doctests 1 passed |
| `cargo clippy -p vigilode --all-targets --locked -j 1` | 0 | no warnings in `crates/vigilode` (4 pre-existing dead-code warnings in `rodas5p-integrators`, untouched) |
| `cargo fmt -p vigilode` | 0 | formatted |
| `cargo build -p vigilode --locked -j 1` | 0 | — |

Tests (`crates/vigilode/tests/facade_contracts.rs`, exact `f64::to_bits` equality where stated):

- Robertson, dense Jacobian, rtol 1e-6, atol 1e-10, span (0, 40): bit-identical states and
  identical counters, attempts, accepted and rejected steps versus
  `integrate_rodas5p_fast_observed(robertson_problem())` and versus the router on the library problem.
- `Tier::Performance` is bit-identical to `Tier::Protected`.
- The `examples/solve_stiff.rs` problem (JVP only, rtol 1e-8, atol 1e-11, h0 1e-3), span (0, 1):
  bit-identical versus `integrate_rodas5p_mf_fast_observed_gmres_into` with
  `RoutingOptions::default().matrix_free`; zero Jacobian builds.
- User band Jacobian (tridiagonal, n = 20): bit-identical versus `integrate_rodas5p_fast_banded_observed`.
- FD Robertson at rtol 1e-6 within 2x of the analytic-Jacobian error; FD banded (3 groups) within 2x;
  FD calls = 4 per build in both; merged counters = driver counters + FD calls.
- Dense + declared band: router rule 1 with a verification record; a wrong band is refused
  (`StructureRefused`) before any step.
- Fatal RHS beyond t = 0.5: `Ok` with `CallbackFailed { Rhs, t in (0.5, 1] }`, partial `t = [0]`,
  work kept; user RHS entries = `rhs_calls - short_circuited_rhs_calls`; `ensure_success` gives
  `SolveError::Failed` with the partial solution. Recoverable error: one rejection, run succeeds.
- `max_steps = 3`: `MaxStepsReached`. Refusals: vector atol, mass matrix, `dense_output`, band on JVP
  (`Unsupported`); malformed span, `y0`, `h0`, `max_steps`, `rtol`, atol length, `n = 0`,
  autonomous + time derivative, band mismatch / outside matrix (`InvalidInput`).
- Explicit time derivative on `y' = -50 (y - cos t)`: error < 1e-6 against the closed form.
- Stats serialize every adaptive field and counter; the options round-trip through serde and the
  replayed run is bit-identical.

Measured numbers (status: *measured, same-solver reference*; the reference is this solver at
rtol 1e-12, atol 1e-16, not an independent code; final-state error scaled by `atol + rtol |ref|`):

| Case | analytic Jacobian | finite differences |
| --- | --- | --- |
| Robertson, t = 40, rtol 1e-6, atol 1e-10 | 9.139e-1 | 8.530e-1 |
| Tridiagonal n = 20, t = 2, banded | 1.7353e-1 | 1.7353e-1 |

No wall times were measured.

## Deviations from the brief and decisions

1. **Vector atol** is refused (`Unsupported`): `AdaptiveStepConfig::atol` is scalar and no
   protected driver takes a per-component tolerance. As the brief prescribes.
2. **Mass matrix**: `.mass()` exists, but every allowed protected driver (dense, banded,
   matrix-free fast) refuses a non-identity mass, so the facade refuses it as `Unsupported`.
3. **`dense_output = true`** is refused as `Unsupported`: no allowed driver returns per-step states
   without changing its step sequence (an output schedule clips steps). With `false` the solution
   holds `t0` and `t1`; a failed run holds only `t0` (the partial solution is thin until dense
   output exists). This is an API decision the brief does not settle; refusal keeps it
   forward-compatible.
4. **Added builder `.autonomous()`** (not in the brief): needed for bit identity with drivers called
   on autonomous problems (the default treats `f` as time-dependent, which adds the `f_t` path).
5. **Callback signature** `-> Result<(), CallbackError>` with `Recoverable` / `Fatal`: the brief's
   "failing RHS" needs an error channel; a fatal error is latched by the facade so the driver
   returns its partial result instead of an `Err`.
6. **A failed run is `Ok(Solution)` with `Status::Failed`**; `Err` is reserved for refusals before
   integration and driver errors that leave no partial result. `Solution::ensure_success` converts.
7. **`max_steps`** maps to the drivers' `max_attempts` (accepted + rejected).
8. **Small-N const-generic driver** is not routed (its dimension is a const generic; the facade's is
   runtime). **JVP + band** is refused as `Unsupported`.
9. **Banded routes** pass `OdeProblem::new` a dense Jacobian assembled from the band (the
   constructor requires a provider); the banded driver reads only the band (`jvp_calls = 0` asserted).
10. **Lock update**: `cargo update -p vigilode --offline` cannot add a package that is not yet in the
    lock; `cargo metadata --offline` wrote the entry. Diff is the vigilode entry only.

## Re-run in the isolated target dir (controller ruling)

The results above were first built in the shared `/home/claude/targets/shared`. After the
ruling that worktrees can overwrite each other's `rodas5p-*` artifacts there, all of them were
re-run with `CARGO_TARGET_DIR=/home/claude/targets/a1-api` (other variables unchanged, `-j 1`)
from a clean build of that directory, at commit `1da0425`:

| Command | Exit | Result |
| --- | --- | --- |
| `cargo test -p vigilode --locked -j 1 -- --nocapture` | 0 | lib 3 passed; `facade_contracts` 14 passed; doctests 1 passed |
| `cargo clippy -p vigilode --all-targets --locked -j 1` | 0 | no warnings in `crates/vigilode` (1 in `rodas5p-core`, 7 in `rodas5p-integrators`, pre-existing, untouched) |
| `cargo build -p vigilode --locked -j 1` | 0 | — |

The measured errors repeat to every printed digit (Robertson 9.139269622883528e-1 /
8.530278086763213e-1; tridiagonal 1.7352558843846827e-1 / 1.7353002047212848e-1). The RED run
(stub `solve`) was not repeated; it was a compile-and-fail check of the tests themselves.

## Fix round 1 (commit a605573, pushed)

Changes:
1. Important #1: added `Stats::driver_work: DriverWork` (serde, tag `driver`) with
   Dense { jacobian_reuses, lu }, Banded { jacobian_reuses, lu, banded: BandedWork } (router rule 1 and the direct banded call),
   and MatrixFree { state_reuses, gcrodr_policy, krylov_workspace, start_policy, recycle_scope, start_telemetry }.
   `KrylovStartTelemetry` is not re-exported, so its fields are copied into a new `KrylovStartRecord`. Files: crates/vigilode/src/{solution,solve,lib}.rs.
   New tests: dense_route_records_jacobian_reuses_and_lu, banded_routes_record_band_work,
   matrix_free_route_records_state_reuses_and_policy_ids. Each compares values with the direct driver and checks the serialized keys.
2. Important #2: the test is unchanged. docs/plan/nodes/a1-api.md now records that the facade routes JVP-only
   problems to integrate_rodas5p_mf_fast_observed_gmres_into (router rule 4), not to the example's sequential
   dense driver, and why the comparison target is the routed driver.
Nothing else changed.

Command (in /home/claude/wt/a1-api):
RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/a1-api CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test -p vigilode --locked -j 1   -> exit 0
Clippy (`cargo clippy -p vigilode --all-targets --locked -j 1`): no warnings in crates/vigilode.

Output:
```
     Running unittests src/lib.rs (/home/claude/targets/a1-api/debug/deps/vigilode-21b6770b2fdec029)
test fd::tests::a_fatal_error_latches_and_short_circuits ... ok
test fd::tests::grouped_band_equals_the_band_of_the_dense_difference ... ok
test fd::tests::dense_difference_is_accurate_to_the_increment ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/facade_contracts.rs (/home/claude/targets/a1-api/debug/deps/facade_contracts-e5924c0a849bb4c4)
test a_failing_rhs_returns_the_partial_solution_and_a_typed_reason ... ok
test an_exhausted_attempt_budget_is_a_typed_failure ... ok
test a_recoverable_rhs_error_rejects_the_step_and_the_run_succeeds ... ok
test banded_routes_record_band_work ... ok
test banded_finite_difference_uses_grouped_columns_and_the_banded_driver ... ok
test banded_user_jacobian_is_bit_identical_to_the_direct_banded_driver ... ok
test dense_jacobian_with_declared_band_is_verified_and_routed_to_the_banded_driver ... ok
test explicit_time_derivative_drives_a_nonautonomous_problem ... ok
test dense_route_records_jacobian_reuses_and_lu ... ok
test malformed_input_is_refused_before_any_step ... ok
test finite_difference_robertson_is_within_twice_the_analytic_error ... ok
test performance_tier_executes_the_protected_route_bit_for_bit ... ok
test robertson_dense_jacobian_is_bit_identical_to_the_direct_fast_driver ... ok
test stats_serialize_every_option_and_counter ... ok
test stiff_example_jvp_is_bit_identical_to_the_direct_matrix_free_driver ... ok
test unsupported_requests_are_refused_not_substituted ... ok
test matrix_free_route_records_state_reuses_and_policy_ids ... ok
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
   Doc-tests vigilode
test crates/vigilode/src/lib.rs - (line 7) ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
