# Node A1-api: the `vigilode` production facade crate

Branch `production/a1-api` (from `9b5d083`). Plan §3, "The public API".
Toolchain: `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0, rustc 1.97.0); the pinned
1.94.1 is not installable here and `rust-toolchain.toml` is unchanged. Build
environment as ruled: `CARGO_TARGET_DIR=/home/claude/targets/shared`,
`CARGO_PROFILE_{DEV,TEST}_DEBUG=0`, `CARGO_INCREMENTAL=0`, `-j 1`.

## What was done

New workspace crate `crates/vigilode` (root `[workspace] members` += `crates/vigilode`).
Dependencies: `rodas5p-core`, `rodas5p-integrators` (path), `serde`, `thiserror`;
dev: `serde_json`. No new external crate. No existing file other than the root
`Cargo.toml` and `Cargo.lock` was changed.

Public surface:

- `Problem::new(n, rhs)` with `.jacobian(Jacobian::{Dense(fn), Banded{lower, upper, fill}, Jvp(fn), FiniteDifference})`,
  `.mass(matrix)`, `.time_derivative(fn)`, `.banded(lower, upper)`, `.autonomous()`;
  closure helpers `Jacobian::dense / banded / jvp`. Callbacks return
  `Result<(), CallbackError>` with `Recoverable` (step rejected, retried smaller) and `Fatal` (run ends).
- `Options { rtol, atol: Atol::{Scalar, Vector}, h0, max_steps, dense_output, tier: Tier::{Protected, Performance} }`,
  defaults equal to `AdaptiveStepConfig::default()`; `Serialize + Deserialize`.
- `Rodas5p::solve(&problem, (t0, t1), &y0, &opts) -> Result<Solution, SolveError>`.
- `Solution { t, y, stats, status }`, `Status::{Success, Failed(FailureReason)}`,
  `FailureReason::{MaxStepsReached, StepSizeTooSmall, CallbackFailed{callback, t, message}}`,
  `Solution::ensure_success() -> Result<Solution, SolveError::Failed{reason, partial}>`.
- `Stats` (serde): schema tag, the options, the effective `AdaptiveStepConfig`, the matrix-free
  `LinearSolverConfig` when used, span, `y0`, dimension, autonomy, Jacobian record, requested and
  executed tier, route record (entry point, driver id, the router's `RoutingRecord`), attempts /
  accepted / rejected / internal / clipped steps, merged counters, the driver's own counters,
  finite-difference work, callback-error work, driver message.

Routing (`Tier::Protected`; `Tier::Performance` executes the same route, recorded as
`tier_executed = protected`):

| Provider | Structure | Entry point | Driver |
| --- | --- | --- | --- |
| Dense, FiniteDifference | none | `integrate_rodas5p_routed_observed` rule 3 | `rodas5p-fast-transformed-v2` |
| Dense | `.banded(l, u)` | `integrate_rodas5p_routed_observed` rule 1 (band read from the dense callback, verified at `t0`) | `rodas5p-fast-banded-v1` |
| Banded, FiniteDifference | band | `integrate_rodas5p_fast_banded_observed` | `rodas5p-fast-banded-v1` |
| Jvp | none | `integrate_rodas5p_routed_observed` rule 4 | `rodas5p-mf-fast-transformed-v1` |

Finite differences: column increment `sqrt(eps) max(|y_j|, 1)` taken as the represented
difference `(y_j + inc) - y_j`; dense costs `n + 1` RHS calls per build; a declared band uses
Curtis–Powell–Reid groups `j mod (l + u + 1)`, `min(l + u + 1, n) + 1` calls per build. The
calls are counted (on attempt) in `stats.finite_difference.rhs_calls` and added to
`stats.counters.rhs_calls` / `rhs_evaluations`; `stats.driver_counters` keeps the driver's own.

Failure handling: every callback runs through a per-solve guard. Recoverable and fatal errors
reach the driver as `CoreError::NonFinite`, which every protected driver treats as a rejected
attempt; after the first fatal error the guard answers without entering user code
(`short_circuited_calls`, `short_circuited_rhs_calls`), so the driver stops at its minimum step
and returns its partial result, which the facade returns as `Ok` with `CallbackFailed`. The
guard performs no arithmetic on callback values, so successful runs are bit-identical to the
direct driver call.

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

## Fix round 1 (review findings)

1. **Driver work in `Stats`.** `Stats::driver_work: DriverWork` (serde, tagged by `driver`) now
   carries every driver-specific counter and id the routed driver returns:
   `Dense { jacobian_reuses, lu }` (`Rodas5pFastResult`),
   `Banded { jacobian_reuses, lu, banded: BandedWork }` (`Rodas5pFastBandedResult`, from both the
   router's rule 1 and the direct banded call),
   `MatrixFree { state_reuses, gcrodr_policy, krylov_workspace, start_policy, recycle_scope, start_telemetry }`
   (`Rodas5pMfFastResult`; `KrylovStartTelemetry` is not re-exported by `rodas5p-integrators`, so
   its five counts are copied into `KrylovStartRecord`). The driver id was already in
   `stats.route.driver_id`. New tests, one per route: `dense_route_records_jacobian_reuses_and_lu`,
   `banded_routes_record_band_work` (direct and routed band), and
   `matrix_free_route_records_state_reuses_and_policy_ids`. Each compares the values with the
   direct driver call where one exists and checks the serialized keys.
2. **Which driver the stiff-example test compares against.** `examples/solve_stiff.rs` calls
   `integrate_sequential_matrix_free_adaptive_dense_observed` (sequential matrix-free path, GMRES,
   with a 106-point dense sampling plan). The facade does not route there. A JVP-only problem
   goes through `integrate_rodas5p_routed_observed` rule 4 to the U-form matrix-free driver
   `integrate_rodas5p_mf_fast_observed_gmres_into` with `RoutingOptions::default().matrix_free`.
   The bit-identity requirement is that the facade equal the direct call of the driver it runs,
   so the test solves the example's problem (same RHS, JVP, y0, rtol 1e-8, atol 1e-11, h0 1e-3)
   and compares against that routed driver. A comparison with the sequential driver would compare
   two different algorithms, which cannot be bit-identical. The example's accuracy budget (1e-6
   absolute at t = 1) is also asserted through the facade. Controller ruling: correct as written.

Re-run in `CARGO_TARGET_DIR=/home/claude/targets/a1-api`: `cargo test -p vigilode --locked -j 1`
exit 0. Lib 3 passed, `facade_contracts` 17 passed, doctests 1 passed. Clippy for the crate shows
no warnings.
