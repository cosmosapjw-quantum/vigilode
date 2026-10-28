# Task 3 report: p0-fastdense

Node `p0-fastdense`, branch `production/p0-fastdense` (pushed), worktree `/home/claude/wt/p0-fastdense`.

**Status: DONE_WITH_CONCERNS.** The concerns are deviation D1 (test (e)) and the open questions at the end.

## Commits

| sha | content |
| --- | --- |
| 90b8826 | Driver level: sink hook, `solve_into`, `rodas5p_fast_dense.rs`, tests (a)–(g) |
| 4702b4f | Merge of the integration branch with p0-atol 997fefe. R-C1: the sink goes after `atol_vector`, the estimate reads `work.atol_vector`, and the `_with_atol` entry points are added |
| 7b1eb8b | Merge of the integration tip e9c5bbd (p0-events f65d968, b1c-staleness). Clean textual merge |
| 169cbf2 | Facade: `Plan::FastDense`, `Extension` enum, stats v3, docs, test (h) |

Both integration merges are real merges (`--no-ff`). No rebase was done and no history was rewritten.

## Requirements → file:line

Paths are relative to `crates/`.

**Driver level (`rodas5p-integrators`)**

- **Sink hook** (additive; no `pub(crate)` signature that lagged uses was changed):
  - `src/rodas5p_fast.rs:1487` defines `trait StepSink`; `:1512` defines `NoSink`, whose methods are `#[inline(always)]` identity/empty.
  - `integrate_fast<S: StepSink>` is at `:1855`. Its sink parameter comes after `atol_vector` (R-C1).
  - Loop hooks: `:1931` (an attempt with embedded error ≤ 1) and `:1972` (an accepted step).
  - Every existing entry point passes `&mut NoSink`.
- **`solve_into(&mut self, b)`**, banded-capable, is at `rodas5p_fast.rs:953` (FP2).
  - It charges no `WorkCounters`.
  - Band solves are counted in `BandedWork::solve_operations`.
  - The faer path allocates once per call, as `solve_dense_into` already does.
- **`rodas5p_dense_output_parts` is `pub(crate)`**: `src/dense_output_v2.rs:991`. Only the visibility changed; test (c) uses it.
- **`src/rodas5p_fast_dense.rs`:**
  - Driver ids at `:41` and `:44`.
  - `FastInteriorControl` at `:52`, `FastInteriorWork` at `:63`.
  - `FastDenseSegments` at `:87`: flat `t`, `(m+1)·n` states and `m·8·n` u-rows, with `len`, `t_old`, `t_new`, `times`, `state`, `stage_rows`, `truncate` (`:176`) and `evaluate_into` (`:188`).
  - The `_into` polynomial is at `:250` (FP3).
  - The W-filtered estimate is at `:296`. It forms the midpoint state, the derivative `(y_new − y_old + (d1+d2)/4)/h`, one RHS into `work.stage_rhs` and one `solve_into`, then `/(hγ)`. It takes the norm with `scaled_error_component` on the step scale, and the scale reads `work.atol_vector` (`:335`).
  - The controller sees `max(err, est)` (`:367`).
  - `Rodas5pFastDenseResult` (`:383`) has `banded: Option<BandedWork>` (R-C1/B3c).
  - Entry points: `:448` and `:461`, plus `_with_atol` at `:504` and `:518` (R-C1).
  - The output is `[t0, t1]` only (`[t0]` when `t1 == t0`).
- **Exports**: `src/lib.rs`, a `pub use rodas5p_fast_dense::{..}` block after the `rodas5p_fast` block.

**Coordinates.** The fast rows are `u = Γk` of the sequential `k` rows, and `dense_d = dense_h·Γ`. So the extension applies `dense_h` to the `u` rows directly. θ and the polynomial form are those of `Rodas5pDenseSegment::evaluate`, and endpoints return the stored states bitwise.

**Facade (`vigilode`)**

- **`Plan::FastDense { band }`** (R-C2): `src/solve.rs:92`, with routing just after `build()`. The arm is at `:274` and records `entry_point` as one of `integrate_rodas5p_fast(_banded)_dense_observed(_with_atol)`.
- **`sequential_route` = mass ∥ (dense_output ∧ Enforce)**: `solve.rs:753`.
- **Refusals:**
  - The vector-atol refusal is narrowed to the sequential route (`:694`).
  - JVP-only + dense output stays refused, and so does Dense + declared band + dense output (B3f). Both use `dense_or_mass` (`:674`).
- **`with_jacobian_into` for a user dense Jacobian**: the buffer is zeroed first (B3d, `:797`); the hookup is at `:910`. The allocating `JacobianFn` stays for the sequential path and the band declaration.
- **`src/solution.rs`:**
  - `STATS_SCHEMA = "vigilode.stats.v3"` (`:28`). Its doc note covers both the fast-dense route and p0-events' `terminal_event`/`event_evaluations`, per the controller ruling.
  - `DriverWork::FastDense { banded, jacobian_reuses, lu, band_work, interior, segments }` is at `:280`.
  - The private `enum Extension { Sequential, Fast }` is at `:449`, with `truncate_after` for both variants at `:489` (R-C3).
  - `at()` keeps p0-events' bound "`t > t.last()` ⇒ None" (`:528`).
- **Docs (B3e):** `src/options.rs` (`dense_output`, `Atol::Vector`, `InteriorError::Enforce`) and the routing table and refusal list in `src/lib.rs`.

## Test evidence

**RED.** I compiled the new test files against base code by setting the implementation aside temporarily. Both failed with `error[E0432]: unresolved imports rodas5p_integrators::FastInteriorControl, ..::integrate_rodas5p_fast_dense_observed, ..` (`fast_dense_contracts`, `fast_dense_allocations`).

**GREEN** (final, at 169cbf2):

- **Integrators:** `rodas5p_fast_contracts` 4, `rodas5p_fast_allocations` 2, `int03_banded_fast_contracts` 4, `p0_partial_contracts` 9, `dense_wfiltered_contracts` 10 (+1 ignored), `b1_lagged_contracts` 5 (+1), `b1b_gmres_contracts` 8 (+1), `b1c_staleness_contracts` 6 (+1), `fast_vector_atol_contracts` 4, `fast_dense_contracts` 7, `fast_dense_allocations` 1, and lib `rodas5p_fast*` 18 (including test (c)). All pass.
- **`cargo test -p vigilode --locked -j 1` (all targets):** unit 9, `events_contracts` 15, `facade_contracts` 20, `p0_contracts` 21, doctest 1. All pass.
- **Clippy:** nothing new in touched files (all lib warnings are pre-existing).
- **Docs:** `RUSTDOCFLAGS=-D warnings cargo doc -p vigilode` passes.
- **Formatting:** `cargo fmt --check` shows only the known base file.
- **Ledger:** `check-research-node.py` prints PASS.

**Measured numbers** (deterministic):

- **(a)** Off is bitwise the fast driver: same endpoint bits, attempts, accepted, rejected, counters, LU and reuses.

  | problem | rtol 1e-4 | rtol 1e-6 | rtol 1e-8 |
  | --- | --- | --- | --- |
  | Robertson [0, 40], attempts / accepted | 26/25 | 45/43 | 103/99 |
  | HIRES [0, 321.8122], attempts / accepted | 82/58 | 210/204 | 784/778 |
  | Brusselator n = 100, banded, attempts / accepted | 54/45 | 100/92 | 219/215 |

  On Brusselator `BandedWork` and `partial` are equal too.
- **(d)** PR λ = −1e5, rtol 1e-6, atol 1e-8, 201 points:
  - WFilteredEnforce: **0.3495 units, 15 accepted** (15 attempts, 0 rejected, max estimate 0.6315).
  - Off: 311.47 units, **7 accepted**.
  - The sequential driver gives 15 / 7, so the fast route matches it. Through the facade: 0.3495 units in 15 steps, Off 321.05 units in 7 steps.
- **(e)** Linear PR: 0 interior rejections in 15 attempts. Nonlinear PR (μ = 1e5): **2 interior rejections in 27 attempts**. The ledger closes on both. See D1.
- **(f)** The latch at t = 1.56e-4 kept 4 segments, a bit-identical prefix (times, states and u rows) of the clean run.
- **(g)** Allocations on Robertson:

  | run | fast | dense Off | dense WFilteredEnforce | budget |
  | --- | --- | --- | --- | --- |
  | rtol 1e-6 | 46 | 61 | 62 (44 accepted) | 64 |
  | rtol 1e-9 | 46 | 67 | 69 (197 accepted) | 70 |

  The margin is small: 2 and 1 under the budget.
- **(c)** `u` matches `Γk` within 1e-12, and the extension matches the sequential one at θ ∈ {.1, .25, .5, .75, .9} within 1e-12 (see D2 for how "relative" is measured).

## Deviations

- **D1, test (e): `interior_rejections > 0` on linear PR at rtol 1e-6 is false, as the brief states it.** WFilteredEnforce takes 15 attempts and accepts all 15 there, and the sequential driver does the same (15 / 15). The controller feeds `max(err, est)` into the step proposal, so no proposed step is rejected.
  - I probed rtol 1e-4…1e-8, h0 ∈ {1e-3, 1e-6} and λ from −1e2 to −1e8 on linear PR: there were zero interior rejections every time.
  - Rejections do occur on nonlinear PR (μ = 1e5 or 1e6) and on HIRES at rtol 1e-4 (13).
  - The test keeps the ledger checks on linear PR and gates `> 0` on PR μ = 1e5 at the same tolerances.
- **D2, test (c): "1e-12 relative" is measured against the condition of the transform.** Per-entry relative error is ill-conditioned here: entries of 3.4e-9 are sums of 4e-5 terms, and per-entry relative error came out at 1e-11.
  - The u-row test is therefore relative to `Σ_j |Γ_ij k_j|`.
  - The sum runs over `j ≤ i`. Γ is lower triangular, but its numeric inverse holds roundoff of 1e-17 above the diagonal.
  - The extension test is relative to the step's state magnitude.
- **D3, error inside the estimate.** An `Err` from the estimate's midpoint RHS or back-solve is classified by `failure_kind`, like an attempt error: it rejects with that kind, and an unclassifiable error ends the run with `Err`. The sequential driver instead counts such samples as "unavailable" and accepts the step. The brief's `FastInteriorWork` has no `unavailable` field, so I chose the conservative rule.
- **D4, the estimate's work is charged to the run counters.**
  - The midpoint RHS and the back-solve go into `rhs_calls`, `rhs_evaluations`, `linear_solves` and `direct_solve_calls`, as the sequential Enforce modes do. `FastInteriorWork` also lists them.
  - The midpoint flow uses `work.stage_rhs` as scratch, which is free between attempts.
- **D5, `DriverWork::FastDense` has one more field than the brief lists.** It also carries `band_work: Option<BandedWork>`, besides `banded: bool`, `jacobian_reuses` and `lu`. This follows B3c's "every counter".
- **D6, `FastInteriorWork::max_estimate`** is the largest estimate of an *accepted* step, and 0 when there is none, matching the sequential `max_accepted_estimate`.
- **D7, banded vs dense provider no longer bit-identical under dense output.** The banded LU and the dense LU differ in rounding. `banded_provider_on_the_dense_output_route_matches_the_dense_provider` now checks driver ids and agreement within 1e-9, and accepted steps within ±1.
- **D8, `facade_contracts::vector_atol_refused_on_mf_sequential_and_declared_band`** (T4's test) now refuses dense output only under `Enforce`. It also asserts that default dense output with a vector atol is accepted on `..._fast_dense_observed_with_atol`.
- **D9, extra tests added:**
  - `uniform_atol_vector_is_the_scalar_dense_run`: a local guard of the R-C1 seam. T8 keeps its own seam test.
  - `empty_span_keeps_the_initial_state` (driver level).
  - A solution.rs unit test of `truncate_after` on both stores.
  - A route assertion in `events_do_not_change_integration`, which covers parked Minor 4: the events suite now runs on the fast route, and all 15 tests pass.

## Handoff commands

```
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build -p vigilode --examples --locked   # examples assert tolerances; dense-output bits changed (A14)
RUSTUP_TOOLCHAIN=stable cargo test -p rodas5p-integrators --release --locked --test fast_dense_allocations --test rodas5p_fast_allocations
```

## Open questions

1. **Test (e)'s threshold.** Is moving the `> 0` gate to nonlinear PR acceptable, or should the brief's linear-PR premise be recorded as false in the plan?
2. **FD dense Jacobian.** It still uses the allocating `JacobianFn` (`dense_fd_jacobian`); only the user dense provider got `with_jacobian_into`. Should T7 or a later node add an in-place FD build?
3. **Test (g) margin.** It is small (1–2 allocations) because the store grows by doubling from 2 steps. Larger initial capacity would cost up to 16·8·n f64 on large n. Keep it as is?
4. **Events.** They stay post-processing on the fast store. Stopping inside the loop, through the new sink, is a natural later item.

## Fix round 1

Commit 35d6604 on `production/p0-fastdense` (pushed). Inputs: the review in `task-3-review.md` (Approved, 6 Minor) and the controller's rulings. D3 is unchanged, and the sequential driver is untouched; its accept-unavailable behaviour is parked for T8 / wave 4.

| Minor | Change |
| --- | --- |
| M1 | The rule is now stated in the `rodas5p_fast_dense.rs` module doc and in `InteriorError::WFilteredEnforce` (`vigilode/src/options.rs`): an estimate error (midpoint RHS or back-solve) rejects the attempt like a stage error, while the sequential route counts the sample unavailable and accepts. New test `fast_dense_contracts::midpoint_rhs_error_rejects_then_retries`, described below. |
| M2 | Restored sequential-route bit identity in `p0_contracts::banded_provider_on_the_dense_output_route_matches_the_dense_provider`. Under dense output with `Enforce`, `Jacobian::Banded` and `Jacobian::Dense` both take the sequential driver and give bitwise-equal `t`, `y` and accepted steps. The fast-route tolerance check is kept. |
| M3 | The test (g) docs and the `DenseSink::new` comment now state that the 4–5 setup allocations (three store vectors, the schedule, the scratch) are charged against the `3·⌈log₂(m+1)⌉` budget. The comment notes the times vector starts at capacity 16 and the states/stage rows at 2 steps. Docs and comments only. |
| M4 | The module doc now says per-step zero allocation holds on the in-place LUs (dense ≤ 64 rows or sparse, and banded). On the faer LU the back-solve allocates once per sample, as the fast driver's own stage solves do. Doc only. |
| M5 | `polynomial_component` is the one per-component θ form, used by both `polynomial_into` (`evaluate_into`) and the estimate's midpoint at θ = 0.5. The expression is bitwise identical, since the complement 1 − 0.5 is exact. Test (a) still passes, and the test (d) and (g) numbers are unchanged. |
| M6 | In `integrate_fast`, a `sink_failed` flag (set only by an `Err` from `after_attempt`) keeps `fresh_state`. The retry therefore reuses the state's Jacobian, `f(t, y)` and `f_t`, which are valid because the attempt itself succeeded. `NoSink` never errors, so the non-error path's arithmetic is unchanged; test (a) and `rodas5p_fast_contracts` / `rodas5p_fast_allocations` confirm it. |

**The M1 test:**

- It runs Robertson with an RHS that fails only on call 27. From the clean run's call log it first checks that call 27 is the third step's midpoint (`t_old + h/2`) and that no RODAS5P `c_i` equals 1/2.
- It asserts one `nonfinite_step_failures`, the first two steps bit-identical to the clean run, and the third step starting at the same state with a smaller accepted step: h = 5e-6 instead of 2.5e-5.
- It asserts `samples = accepted + interior_rejections`, with the failed sample uncounted.
- It also checks M6: `jacobian_reuses` is 1 against 0 in the clean run, and call 28 is a stage call inside the retried step, not `f` at the step start.

**Tests run** (targeted, `-j 2`), all passing:

- `fast_dense_contracts` 8, `fast_dense_allocations` 1 (counts 61 / 62 / 67 / 69, budget 64 / 70, unchanged)
- `rodas5p_fast_contracts` 4, `rodas5p_fast_allocations` 2, `int03_banded_fast_contracts` 4, `p0_partial_contracts` 9, `fast_vector_atol_contracts` 4, `b1_lagged_contracts` 5
- vigilode: `p0_contracts` 21, `events_contracts` 15, lib 9

Clippy shows nothing in touched files. `cargo doc -D warnings -p vigilode` passes. The integrators doc errors are all pre-existing, and none is in the files this node touches.
