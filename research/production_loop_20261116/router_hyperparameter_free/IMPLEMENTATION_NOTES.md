# V27 implementation notes (phase 1: forward break-even pilots)

Node V27_ROUTER_HYPERPARAMETER_FREE, phase 1 of `PREDECLARED.json` `order_of_work`. Owner decision OD-L1-22
(D1-D6). Base commit `1ffbda20f0d8b7afab6d31134a1bea683f04ffda`, branch `research/production-loop-20261116-w3-v27`.
Line numbers below are those of the base commit; this file is updated as the work proceeds (section 6 is the log).

## 1. What changes, where (base file:line)

| File (base) | Lines | Change |
|---|---|---|
| `crates/vigilode/src/session.rs` | 122-238 (`impl ExplicitSession`) | add crate-private `set_attempt_telemetry(bool)` and `last_attempt() -> Option<Dopri5Attempt>`, plain pass-throughs to the native session (`rodas5p-integrators/src/dopri5.rs:1007-1016`); add crate-private `adaptive_max_factor()` reading the session's `AdaptiveStepConfig::max_factor` (no new number). |
| `crates/vigilode/src/unified/breakeven.rs` | new | `break_even_length(n)` in integers; the FBP loop of DESIGN_RECOMMENDED 3.3; `PilotRecord` (public, re-exported). |
| `crates/vigilode/src/unified/analyzer.rs` | 26-32 | remove `EXPLICIT_TRIAL_MAX_DIMENSION`, `DOPRI5_REAL_STABILITY_BOUND` (D5). |
|  | 74-149 | `RouteBasis`: remove `DimensionAboveTrialLimit`, `NoBudgetForTrial`, `StiffnessScreen`, `UndeclaredRegimeTrial`, `PreflightCallbackFailed` (D5); add `BreakEven { b }`. |
|  | 151-193 | remove `StiffnessScreen` (D5). |
|  | 195-222 | `PreflightAnalysis`: remove the screen fields (`screen`, `rhs_calls`, `callback_failure`); `first_leg_attempts` = `max_steps` on every route; `fallback_to_rodas5p` true on the break-even route. |
|  | 261-444 | `analyze`: rules 5-6 (dimension limit, trial budget, screen) replaced by the break-even route; both entry points validated before any callback (unchanged rule); the analysis makes no callback. |
|  | 446-517 | remove `stiffness_screen`. |
| `crates/vigilode/src/unified/fallback.rs` | 10-25 | remove `EXPLICIT_TRIAL_ATTEMPT_LIMIT`, `explicit_trial_attempts` (D5); keep `remaining_attempts`, `follows_explicit_stop`. |
| `crates/vigilode/src/unified/router.rs` | 47-82 | no preflight charging, no preflight stop path; `Tier1Explicit` with `fallback_to_rodas5p` runs the break-even route, declared `Nonstiff` keeps the plain DOPRI5 leg. |
|  | 133-188 | `explicit_then_fallback` split: the declared-Nonstiff DOPRI5 leg (unchanged) and `break_even` (DOPRI5 session leg from snapshots, pilots charged into `ws.work_counters`, RODAS5P leg from `(t_D + H, y_pilot)` with `h0 = H` and the attempts left). |
|  | 249-362 | `finish`: no `UnifiedStop::Preflight`; pilot records into the solution. |
| `crates/vigilode/src/unified/legs.rs` | 83-87, 174, 186, 237 | remove `UnifiedStop::Preflight` (nothing needs it once the analysis makes no callback; D5). |
|  | new fn | `session_leg`: a `UnifiedLeg` built from two session snapshots (`stats_between`, `session.rs:192-229`), same fields as `registry_leg` reads from a DOPRI5 run record. |
| `crates/vigilode/src/unified/solution.rs` | 34-80 | `VigilodeSolution::pilots: Vec<PilotRecord>`; the handover is a route-history entry (from DOPRI5 to the RODAS5P pilot step) whose reason states t_D, H, the window cost and the pilot cost. |
| `crates/vigilode/src/unified/mod.rs` | 1-65 | module documentation; `mod breakeven`; exports without the removed names, with `PilotRecord`, `PilotVerdict`, `break_even_length`. |
| `crates/vigilode/src/unified/options.rs` | 16-22 | documentation of `SolverStrategy::Auto`. |
| `crates/vigilode/tests/unified_breakeven_contracts.rs` | new | failing-first tests (commit A), then U1-U5 and the session pass-through test. |
| `crates/vigilode/tests/unified_router_contracts.rs` | see section 5 | D4 and D5 only; every edit listed with its reason in section 5. |

Not touched: every protected path of the task, `crates/vigilode/src/lib.rs` (already `pub use unified::*`),
`crates/vigilode/src/prelude.rs` (re-exports `PreflightAnalysis`, `SelectedTier`, ... which stay),
`vigilode-bench` (uses no removed name: `grep` at the base finds `RouteBasis`, `StiffnessScreen`,
`EXPLICIT_TRIAL_*`, `DOPRI5_REAL_STABILITY_BOUND`, `UnifiedStop::Preflight`, `explicit_trial_attempts` only in
`crates/vigilode/src/unified/` and `crates/vigilode/tests/unified_router_contracts.rs`).

## 2. How the DOPRI5 session exposes attempts

* `Dopri5::with_session(problem, (t0, t1), y0, opts, times = [t0, t1], SessionEpochs::default(), driver)`
  (`crates/vigilode/src/session.rs:354-482`) runs the native `Dopri5Session` (`rodas5p-integrators/src/dopri5.rs:911-1296`)
  and returns `(R, ExplicitSolution)` with the same record as `Dopri5::solve`.
* `advance_to(t1, 1, epochs)` (`dopri5.rs:1040-1273`) makes at most one attempt and returns `Yield` (quota used),
  `Complete` (t == t1), `Stopped` (fatal or malformed callback, step underflow, non-finite, controller, attempt budget
  `max_attempts == Options::max_steps`). Pausing does not change the step sequence (`session.rs:338-342`;
  `tests/switching_persistent_explicit.rs`).
* With `set_attempt_telemetry(true)` (`dopri5.rs:1007-1010`) every attempt writes `Dopri5Attempt { t_before,
  requested_h, trial_h, error_norm, accepted, clipped, t_after, rhs_delta }` (`dopri5.rs:872-882, 1100-1111,
  1236-1239`), read by `last_attempt()` (`dopri5.rs:1014-1016`). No callback and no change of controller or counters
  (doc comment `dopri5.rs:1005-1006`). The facade session (`ExplicitSession`, `session.rs:112-120`) holds the native
  session privately; the two pass-throughs make them reachable inside the crate only.
* `checkpoint()` (`session.rs:150-161`) gives the accepted `(t, y)`, `proposed_h` (the controller's next proposal),
  cumulative attempts and counters. "Growth-capped" is `accepted && !clipped && proposed_h == trial_h * max_factor`
  (exact equality; the controller returns `h * factor` with `factor == max_factor` when it clamps,
  `rodas5p-integrators/src/adaptive.rs`), `max_factor` read from the session's `AdaptiveStepConfig` (no new number).
* `snapshot()` / `stats_between()` (`session.rs:174-229`) give the DOPRI5 leg's `ExplicitStats` between two points
  without a second run; the `ExplicitSolution` returned by `with_session` is the DOPRI5 leg's record when the
  session completes or stops; on a handover it is the prefix up to `t_D` (the session is finished after the pilot
  is accepted, and a paused session's record is the accepted prefix, `dopri5.rs:1274-1295`).

## 3. How a pilot calls RODAS5P with max_steps = 1 and h0 = H

`Rodas5p::solve_observed_bounded(problem, (t_D, t1), y_D, &pilot_options, &[], None)`
(`crates/vigilode/src/solve.rs:256`, `pub(crate)`, the body of `Rodas5p::solve`), with `pilot_options` = the user's
`Options` with `h0 = H` and `max_steps = 1`. It is the first attempt of a direct `Rodas5p::solve` from that state:
same driver family selection (the start rule keeps every supplied derivative or band away from this route, so the
pilot is the finite-difference dense route), the FD Jacobian (n + 1 calls), the FD time derivative for a
non-autonomous problem, 7 stage calls. Outcomes:

* `final_time > t_D` (its one attempt accepted): handover. Legs: DOPRI5 `t0..t_D`, the pilot as a RODAS5P leg
  `t_D..t_D+H` (its own run record), then the RODAS5P leg from `(t_D + H, y_pilot)` with `h0 = H` and the attempts
  left, through the registry like every RODAS5P leg (`registry_leg`). If `t_D + H == t1` the pilot leg is the last.
* `Status::Failed(CallbackFailed { .. })`: the solve ends (D1) with the accepted prefix (DOPRI5 up to `t_D`) and
  the pilot's work.
* otherwise (`MaxStepsReached` after a rejected attempt, a recoverable callback error, a linear-solve failure):
  the pilot is lost; its work is charged to `ws.work_counters` and its attempt to `max_steps`; the DOPRI5
  session was not touched and continues.
* `Err` (a refusal before any callback): not expected (both entry points were validated); treated as "nothing
  integrated by the pilot", recorded with zero cost, and pilots stop.

Budget: before every DOPRI5 attempt and every pilot, `dopri5_attempts + pilot_attempts < max_steps` is checked;
the DOPRI5 session is built with `max_steps` as its own limit and is only advanced while the shared budget has
room, so exhaustion is reported as `MaxStepsReached { max_steps }` of the whole solve.

## 4. Plan of the tests

`crates/vigilode/tests/unified_breakeven_contracts.rs` (new):

* Commit A (failing first, compiles against the base API and the new one: only `VigilODE::solve`,
  `VigilodeSolution { success, legs, stats, status }`, `UnifiedLeg { tier, attempts, counters }`,
  `SelectedTier`, `Dopri5::solve`, `Rodas5p::solve`, `Problem`, `Options`, `CallbackError`):
  * `f2_fatal_above_one_completes_under_auto`: F2 (y' = -y, fatal for y > 1, y0 = 1, [0, 1]) succeeds under Auto.
    Base: the screen's FD column at y0 + delta is fatal, the solve stops (expected assertion failure).
  * `lorenz96_40_has_no_rodas5p_leg_and_costs_at_most_twice_dopri5`: n = 40 > 10 runs RODAS5P on the base.
  * `robertson_40_charges_at_most_two_direct_rodas5p_plus_window_and_pilots`: charged right-hand sides
    <= 2 cost(R) + 2 b c_D + pilots (pilots bounded by the pilot count from the attempts, each n + 9 calls);
    base: 6233 > 2 * 415 + ....
  * `oscillator_1000_has_no_rodas5p_leg`, `arenstorf_has_no_rodas5p_leg`: base: the 1000-attempt trial runs out
    (oscillator) / the screen fires (Arenstorf).
  * The log of the run against the base sources is committed with the tests
    (`router_hyperparameter_free/logs/failing_first_base.log`).
* Step 2: `session_telemetry_and_quota_one_keep_the_bits_of_dopri5_solve` (crate-private pass-throughs are
  exercised through the public route: a run without handover equals `Dopri5::solve` bit for bit and in its
  DOPRI5 counters; plus a direct `Dopri5::with_session` quota-1 loop against `Dopri5::solve`).
* Step 3: U1 neutrality and the 2 C_D bound (oscillator, Lotka-Volterra, Arenstorf: final-state bits and
  DOPRI5 counters of `Dopri5::solve`; pilots' counters added exactly; cost <= 2 cost(D) in callbacks and in the
  operation formula); U2 one budget (Robertson with max_steps 500 and 50: attempts over all legs and pilots
  <= max_steps; exhaustion is `MaxStepsReached { max_steps }`); U3 no callback before DOPRI5's first stage (a
  recording closure: the first entry is DOPRI5's first stage at (t0, y0); no entry at t0 other than DOPRI5's
  stages; F2); U4 evidence of a handover (Robertson, P_heat_fd: a recorded handover, a route-history entry with
  t_D and H, accepted pilot with H = the window length); U5 determinism (two runs: same route digest, counters,
  bits); D1 (N2: CallbackFailed at the first pilot, accepted prefix kept); b(n) table of the design.

## 5. Contract edits in `unified_router_contracts.rs` (D4, D5)

Filled in as the edits are made; each entry: test name, base line, what changed, which decision requires it.

## 6. Log

* Notes written first (this file), before any code change.
