# Node p0-facade: protected-tier completeness of the `vigilode` facade

Wave 2, Task 3. Branch `production/p0-facade` from `a5a48f7`. Plan §3, P0 tier.
Toolchain: `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0). Env: `CARGO_TARGET_DIR=/home/claude/targets/p0-facade
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `-j 1`, `--locked`.

## Commits

| SHA | Subject |
| --- | --- |
| 7da5ff3 | feat(integrators): partial records on failure and a segment-recording dense driver |
| 7ca2161 | feat(vigilode): mass matrices, dense output with interior control, partial solutions |

## What was done

### Drivers (`rodas5p-integrators`)

- `DriverPartial { t, y, reason: DriverStopReason::{AttemptBudget, MinimumStep}, last_failure:
  Option<AdaptiveFailureKind> }` (`rodas5p_fast.rs`). `last_failure` is the kind of the last
  rejection after the last accepted step. It is filled only when a run stops before `tf`: the
  state is copied once, after the loop. The loop itself only assigns one `Option` per attempt,
  and the step arithmetic never reads it.
- Banded: `Rodas5pFastBandedResult::partial`. Matrix-free: `Rodas5pMfFastResult::partial`.
  Dense fast driver: `integrate_rodas5p_fast_observed_with_partial`, which returns `(result,
  partial)`. `Rodas5pFastResult` itself is unchanged because `rodas5p_lagged.rs` (node b1b, not to
  be touched) builds it as a struct literal. The router records it in `RoutedResult::partial`.
  The only edit outside the integrators is in `rodas5p-cli/src/stiff_benchmark.rs`: a destructuring
  pattern there gets `..`.
- `integrate_adaptive_dense_observed_with_segments(..., control, interior: InteriorSampling)`
  returns `AdaptiveDenseSegmentRun { result, report, segments: Vec<Rodas5pDenseSegment>, partial }`.
  `InteriorSampling::OutputTimes` is the rule of `integrate_adaptive_dense_observed_with_dense_error_control`,
  bit for bit; that function now wraps the same implementation. `EveryStep` forms the interior
  estimate on every step that the embedded estimate accepts. `Rodas5pDenseSegment::evaluate(t)`
  computes θ the same way the output collector does, through `rodas5p_dense_output`'s arithmetic,
  which is factored into a parts function with the same expression. `RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID =
  "rodas5p-sequential-dense-v2"`.
- `rodas5p-core`: `LinearSolverConfig` derives `PartialEq`, for the facade record.

### Facade (`vigilode`)

- A run with `.mass(M)` (constant, singular allowed) or with `dense_output = true` goes to the
  sequential dense driver: `IntegrationMethod::Sequential`, default (direct LU) linear solver,
  entry `integrate_adaptive_dense_observed_with_segments`.
  - With dense output, the control comes from `Options::interior_error: InteriorError::{WFilteredEnforce
    (default), Enforce, Off}` (`#[serde(default)]`), with `InteriorSampling::EveryStep`.
  - With a mass matrix and no dense output, the control is `Off` with `OutputTimes`: only the endpoint
    is returned and controlled.
  - The executed control and its counts are recorded in `DriverWork::SequentialDense`.
- Refusals with `Unsupported`:
  - a JVP-only problem with a mass matrix or dense output (any interior mode);
  - a dense Jacobian with a declared band together with a mass matrix or dense output. Only the router
    verifies that declaration, and the sequential route would ignore it.

  Refusals with `InvalidInput`: a mass matrix of the wrong length or with NaN/Inf entries.
- `Solution::at(t)`:
  - with dense output, the extension of the step that contains `t`, for `t0 <= t <= last accepted`;
  - otherwise, the stored sample when `t` equals one of `Solution::t`;
  - `None` in every other case, including NaN.

  The segments are held in a private field that serde skips.
- Partial solution:
  - Without dense output, a failed run returns `t = [t0 (, reached samples), t_last]` with the last
    accepted state.
  - With dense output, it returns every accepted step.
  - `Stats::stop: Option<StopRecord { t, reason, last_failure }>` keeps the driver's record.
- `FailureReason`:
  - `MaxStepsReached`, when the stop is the attempt budget.
  - After a minimum-step stop, chosen by the last rejection kind: `LinearSolveFailed`, `NonFinite`,
    `NonlinearSolveFailed`, or `StepSizeTooSmall` (local error, or no rejection).
  - `CallbackFailed` takes priority over all of these.
- `STATS_SCHEMA = "vigilode.stats.v2"`.

## TDD record

RED: with the source changes stashed, the new tests did not compile:
- `cargo test -p rodas5p-integrators --test p0_partial_contracts` exited 101 with 8 errors (missing
  `DriverPartial`, `InteriorSampling`, `partial` fields, ...).
- `cargo test -p vigilode --test p0_contracts --test facade_contracts` exited 101 with 33 errors.

GREEN after the implementation. Three expectations were wrong in the first GREEN run and were
corrected; the code did not change:
- A JVP or Jacobian callback gets the step-start `t`, so the last accepted time is past 0.5, not
  before it.
- `y' = y²` stops with `StepSizeTooSmall` at t = 1.0000007, beyond the exact pole.

Wave-1 tests changed by the new contract (`facade_contracts.rs`):
- the failing-RHS and attempt-budget tests now expect `[t0, t_last]`;
- the mass and `dense_output` refusals were removed (their refusals are now in `p0_contracts.rs`).

## Tests

`crates/vigilode/tests/p0_contracts.rs` (13). Exact `to_bits` equality where it says bit-identical.
- Dense output (Robertson) is bit-identical to the direct `..._with_segments(WFilteredEnforce, EveryStep)`:
  step endpoints, counters, attempts and sample count; `at(t)` inside steps; `None` outside the span.
- `Off` is bit-identical to the named driver `integrate_adaptive_dense_observed_with_dense_error_control`
  sampled at the facade's step endpoints. `Enforce` matches the direct call.
- Prothero–Robinson, λ = −1e5, rtol 1e-6, atol 1e-8, maximum error over 201 points:
  `WFilteredEnforce` 0.3495 tol units with 15 accepted steps, against `Off` at 321.05 with 7 steps.
- `at` without dense output.
- Mass route, `M = [[2,1],[1,3]]`: bit-identical to the direct named driver (`Off`, with the mass
  in the `OdeProblem`), and within 10 tol units of the folded `M⁻¹A` system.
- Robertson as an index-1 DAE (`M = diag(1,1,0)`), analytic and finite-difference Jacobians, with
  and without dense output: the constraint holds to ≤ 1e-12 at every returned state.
  Scaled error against the ODE reference (rtol 1e-10): 0.914 / 0.689 / 0.853 / 0.789.
- Refusals: JVP with mass, JVP with dense output, declared band with mass, bad or NaN mass.
- Partial solutions after a fatal callback. Each is bit-identical (state, time, driver counters)
  to the direct driver on a twin problem whose callbacks latch the way the guard does:
  - fatal RHS on the dense fast route (≥ 3 accepted steps);
  - fatal RHS on the banded route;
  - fatal JVP inside GMRES on the matrix-free route, the wave-1 untested path. Last accepted
    t = 0.83975 after 6 accepted steps; last rejection `NonFinite`.
- Dense-output partial: the accepted steps of the failed run are bit-identical to the prefix of the
  clean run.
- Failure kinds:
  - `NonFinite` (recoverable RHS error beyond 0.5);
  - `LinearSolveFailed` (M = diag(1,0) with a vanishing algebraic Jacobian row, so W is singular);
  - `MaxStepsReached` on the dense route;
  - `StepSizeTooSmall` (`y' = y²`).
- Serde: options without `interior_error` deserialize to the default.

Also run:
- `vigilode` lib unit test of the reason mapping, including `NonlinearSolveFailed`.
- `crates/rodas5p-integrators/tests/p0_partial_contracts.rs` (9):
  - the `_with_partial` dense fast run is bit-identical to the plain one, on success and on an
    attempt-budget failure;
  - latched RHS partial records on the fast, banded and matrix-free drivers, and `None` on success;
  - the router's partial equals the driver's;
  - the segment driver with `OutputTimes` equals the named driver for `Off` / `Enforce` /
    `WFilteredEnforce` (observed result, report, interior samples equal to `evaluate`, tiling);
  - `EveryStep` with `Off` equals the named driver, and `WFilteredEnforce` takes one sample per
    accepted step;
  - partial record on the segment driver.

## Commands and results

| Command | Exit | Result |
| --- | --- | --- |
| `cargo test -p vigilode --locked -j 1` | 0 | lib 4, `facade_contracts` 18, `p0_contracts` 13, doctest 1 passed |
| `cargo test -p rodas5p-integrators --locked -j 1 --test p0_partial_contracts --test rodas5p_fast_contracts --test int03_banded_fast --test int03_banded_fast_contracts --test matrix_free_problem_contracts --test declared_structure_routing --test dense_wfiltered_contracts --test dense_error_control_contracts --test dense_output_v2_contracts --test rodas5p_fast_allocations` | 0 | 9, 4, 0 (1 ignored), 4, 2, 5 (1 ignored), 10 (1 ignored, the A2 kill gate), 4, 15, 2 passed |
| `cargo test -p rodas5p-integrators --locked -j 1 --lib --test spd07_mf_step_warm_start --test a05_dense_global_output --test audit2_matrix_free_common_w_contracts` | 0 | lib 75 (1 ignored), 4 (2 ignored), 15, 0 passed |
| `cargo check -p rodas5p-cli --locked -j 1` | 0 | — |
| `cargo clippy -p vigilode --all-targets --locked -j 1`; `cargo clippy -p rodas5p-integrators --lib --test p0_partial_contracts --locked -j 1` | 0 | no warnings in touched files (pre-existing ones in `damped_moments.rs`, `output.rs`, `packet_trajectory.rs`, `bound_pr101.rs`, `collective_model.rs`, `packet_curve_output.rs`) |
| `cargo fmt` (touched crates) | 0 | touched files formatted; the pre-existing unformatted `tests/production_audit_packet_curve.rs` was left as it was |

The allocation contract `rodas5p_fast_allocations` stays green, so the partial copy is made on
failure only. Status of the numbers: measured, deterministic, and same-solver for the DAE
reference. No wall times were measured.

## Deviations and decisions

1. **Interior sampling under dense output.** The named driver forms the interior estimate only on
   steps that contain an output time. With output `[t0, t1]` it would control nothing, while `at(t)`
   may evaluate any step. The facade therefore uses the new `InteriorSampling::EveryStep` (plan §5 T3,
   "production use: θ = 1/2 on every step"). With `OutputTimes`, the new entry point is
   bit-identical to the named driver.
2. **The dense fast driver's partial record travels beside `Rodas5pFastResult`**, through
   `integrate_rodas5p_fast_observed_with_partial`, not as a field. Adding a field would break the
   struct literal in `rodas5p_lagged.rs`, which this node must not touch. A merge may move it into
   the struct.
3. **"Accepted samples"** is read as the driver's output samples reached before the failure, plus
   the last accepted state. Under dense output, every accepted step is kept. Without it, no per-step
   state is stored, to keep the P0 no-allocation-per-step property of the fast drivers.
4. **Mass without dense output** runs interior `Off`. `Options::interior_error` applies only under
   dense output, as its docs say. The executed control is recorded.
5. **JVP-only plus dense output is refused in every mode**, `Off` included. A matrix-free sequential
   dense route would need its own linear configuration and tests. The refusal keeps that option open.
6. **Dense Jacobian with a declared band, on the sequential route, is refused.** That route cannot
   verify the band, and it would otherwise ignore the band silently. A finite-difference band keeps
   its grouped columns there, and `Jacobian::Banded` is assembled dense.
7. **Commit identity:** both commits were made as `Claude <noreply@anthropic.com>`. They should have
   used `git -c user.name="Jiwon Park" -c user.email="cosmosapjw@gmail.com"`. They were pushed before
   this was noticed and, under the no-force-push rule, were not rewritten. The trailers are correct.
8. On the mass route without dense output, the segment driver keeps one segment per accepted step,
   and the facade discards them. The sequential dense path allocates per step anyway.
