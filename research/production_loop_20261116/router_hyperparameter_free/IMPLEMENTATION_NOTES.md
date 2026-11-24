# V27 implementation notes (phase 1: forward break-even pilots)

Node V27_ROUTER_HYPERPARAMETER_FREE, phase 1 of `PREDECLARED.json` `order_of_work`. Owner decision OD-L1-22
(D1-D6). Base commit `1ffbda20f0d8b7afab6d31134a1bea683f04ffda`, branch `research/production-loop-20261116-w3-v27`.
Line numbers in sections 1 and 5 are those of the base commit. Section 0 was written first (commit `dbcde43b`);
the later sections record what was done.

## 0. Order of work as executed

| Step | Commit | Content |
|---|---|---|
| 0 | `dbcde43b` | these notes (plan) |
| 1 (commit A) | `4ab275b6` | failing-first tests `crates/vigilode/tests/unified_breakeven_contracts.rs` part 1 and `logs/failing_first_base.log` (run on the base sources: `git diff 1ffbda20 -- crates/vigilode/src` empty) |
| 2 | `c1fb6778` | `session.rs` pass-throughs; test `session_quota_one_advances_keep_the_bits_and_counters_of_dopri5_solve` |
| 3 | `29859eef`, `4eeb6d11` | `unified/breakeven.rs` and the router edits; tests U1-U5, D1 and the b(n) table |
| 4 | `1715dd64` | contract changes D4 and D5 in `unified_router_contracts.rs` (section 5), rustfmt |
| 5 | see section 6 | suites, clippy, fmt, doc, golden, loop checker |
| 6 | see section 6 | the evaluation runner and its plumbing check on the toy problem |

## 1. What changed, where (base file:line)

| File (base) | Lines | Change |
|---|---|---|
| `crates/vigilode/src/session.rs` | 122-238 (`impl ExplicitSession`) | crate-private `set_attempt_telemetry(bool)`, `last_attempt() -> Option<Dopri5Attempt>` (pass-throughs to the native session, `rodas5p-integrators/src/dopri5.rs:1007-1016`) and `max_factor()` (the session's `AdaptiveStepConfig::max_factor`; no new number). Nothing else. |
| `crates/vigilode/src/unified/breakeven.rs` | new | `break_even_length(n)` in integers; the FBP loop (`run`); `PilotRecord`, `PilotVerdict` (public); `Ending`, `BreakEvenRun` (module-private). |
| `crates/vigilode/src/unified/analyzer.rs` | 26-32 | `EXPLICIT_TRIAL_MAX_DIMENSION`, `DOPRI5_REAL_STABILITY_BOUND` removed (D5). |
|  | 74-149 | `RouteBasis`: `DimensionAboveTrialLimit`, `NoBudgetForTrial`, `StiffnessScreen`, `UndeclaredRegimeTrial`, `PreflightCallbackFailed` removed (D5); `BreakEven { b }` added. |
|  | 151-193 | `StiffnessScreen` removed (D5). |
|  | 195-222 | `PreflightAnalysis`: fields `screen`, `rhs_calls`, `callback_failure` removed (they described the screen); `first_leg_attempts` is `max_steps` on every route; `fallback_to_rodas5p` is true on the break-even route only. |
|  | 261-444 | `analyze`: rules 5-6 (dimension limit, trial budget, screen) replaced by rule 5, the break-even route for every dimension; both entry points validated before any callback (unchanged rule); no callback. |
|  | 446-517 | `stiffness_screen` removed. |
| `crates/vigilode/src/unified/fallback.rs` | 10-25 | `EXPLICIT_TRIAL_ATTEMPT_LIMIT`, `explicit_trial_attempts` removed (D5); `remaining_attempts`, `follows_explicit_stop` kept. |
| `crates/vigilode/src/unified/router.rs` | 47-82 | no preflight charging and no preflight stop path; dispatch: `BreakEven` -> `break_even`, declared `Nonstiff` -> `explicit`. |
|  | 133-188 | `explicit_then_fallback` split into `explicit` (declared Nonstiff, unchanged behaviour) and `break_even` + `handover`. |
|  | 249-362 | `finish`: attempts and steps include the pilots that are not part of a leg; a fatal pilot's stop; no `UnifiedStop::Preflight`; `pilots` into the solution. |
| `crates/vigilode/src/unified/legs.rs` | 83-87, 174, 186, 237 | `UnifiedStop::Preflight` removed (D5; nothing needs it: the analysis makes no callback). |
|  | new | `explicit_leg` (a `UnifiedLeg` from the session's `ExplicitSolution`, the fields `registry_leg` reads from `Dopri5::solve`) and `rodas5p_leg_after_pilot` (the accepted pilot and the RODAS5P run after it as one leg). |
| `crates/vigilode/src/unified/solution.rs` | 12-80 | `VigilodeSolution::pilots: Vec<PilotRecord>`; documentation of the handover entry, of `legs` and `inner_solution`. |
| `crates/vigilode/src/unified/mod.rs` | 1-65 | module documentation; `mod breakeven`; exports without the removed names, with `PilotRecord`, `PilotVerdict`, `break_even_length`. |
| `crates/vigilode/src/unified/options.rs`, `workspace.rs` | 17-20; 29 | documentation only. |
| `crates/vigilode/tests/unified_breakeven_contracts.rs` | new | 13 tests (section 4). |
| `crates/vigilode/tests/unified_router_contracts.rs` | section 5 | D4 and D5 only. |
| `research/.../router_hyperparameter_free/runner/` | new | the evaluation runner (section 7). |

Not touched: every protected path, `crates/vigilode/src/lib.rs` (`pub use unified::*` exports the new names),
`crates/vigilode/src/prelude.rs` (re-exports `PreflightAnalysis`, `SelectedTier`, ... which stay),
`unified_production_solver_contracts.rs` and `examples/unified_production_solver.rs` (they use no removed name;
`contract_4_fail_closed_protected_fallback` passes unchanged: the handover gives two legs), `vigilode-bench`
(uses no removed name). `grep` at the base finds the removed names only in `crates/vigilode/src/unified/` and
`crates/vigilode/tests/unified_router_contracts.rs`.

## 2. How the DOPRI5 session exposes attempts

* `Dopri5::with_session(problem, (t0, t1), y0, options, times = [t0, t1], SessionEpochs::default(), driver)`
  (`crates/vigilode/src/session.rs:354-482`) runs the native `Dopri5Session` (`rodas5p-integrators/src/dopri5.rs:911-1296`)
  and returns `(R, ExplicitSolution)` with the record of `Dopri5::solve`; the test
  `session_quota_one_advances_keep_the_bits_and_counters_of_dopri5_solve` checks bits, the whole `ExplicitStats`
  and the callback entries on four cases (one ending on the session's own budget).
* `advance_to(t1, 1, epochs)` (`dopri5.rs:1040-1273`) makes at most one attempt: `Yield` after exactly one attempt;
  `Complete`; `Stopped` (fatal or malformed callback, step underflow, non-finite, controller, its attempt budget,
  which is reported by the call that makes the last attempt). `Checkpoint` cannot occur with the only target `t1`;
  the route treats every outcome other than `Yield` as the end of DOPRI5.
* `set_attempt_telemetry(true)` makes every attempt write `Dopri5Attempt { t_before, requested_h, trial_h, error_norm,
  accepted, clipped, t_after, rhs_delta }`, read by `last_attempt()`; no callback and no change of controller or
  counters (`dopri5.rs:1005-1006, 1100-1111, 1236-1239`). U1 checks it through the route: eight runs with telemetry
  on, quota-one advances and lost pilots equal `Dopri5::solve` in bits and in the whole DOPRI5 record.
* "Growth-capped" is `accepted && !clipped && checkpoint.proposed_h == trial_h * max_factor`, exact: for an accepted,
  unclipped attempt the controller returns `trial_h * factor` with `factor = raw.clamp(min_factor, max_factor)`
  (`rodas5p-integrators/src/adaptive.rs:404-405, 692`).

## 3. How a pilot calls RODAS5P with max_steps = 1 and h0 = H

The pilot is `registry_leg(problem, TierProtectedReference, "rodas5p", (t_D, t1), y_D, options')` with
`options'` = the user's options with `h0 = H` and `max_steps = 1`. The registry adapter of `rodas5p` is
`Rodas5p::solve` (`selection/frontend.rs:1690`), whose body is `Rodas5p::solve_observed_bounded(problem, span, y,
opts, &[], None)` (`solve.rs:244`): the pilot is the first attempt of a direct RODAS5P run from DOPRI5's accepted
state, and its leg record (driver id, route id, counters, stop) is read from its run record as for every RODAS5P leg
of the router. Going through the registry rather than calling `solve_observed_bounded` directly gives the same call
and the route id of the run record without a second copy of the registry's route logic.

* `final_time > t_D` and no callback failure: accepted. Handover: legs [DOPRI5 `t0..t_D`, RODAS5P `t_D..t1`]; the
  RODAS5P leg is the pilot followed by `Rodas5p::solve` from the pilot's end point with `h0 = H` and the attempts
  left (`rodas5p_leg_after_pilot`: span, attempts, steps and work of both; driver, route, success and stop of the
  second). Its route-history entry is the handover (`time = t_D`, reason with `H`, `b`, window and pilot costs);
  `fallback_steps` counts it. `inner_solution` is the second run. If no attempt is left, the pilot alone is the leg.
* `Status::Failed(CallbackFailed)`: the solve ends (D1) with DOPRI5's accepted prefix (up to `t_D`), the pilot's
  work, `PilotVerdict::Fatal` and `stop = UnifiedStop::Rodas5p { reason: CallbackFailed { .. } }`.
* Otherwise lost: work into the counters, attempt into `max_steps` and into `stats.attempts`/`rejected_steps`;
  the session was not touched and continues.
* `Err` (a refusal before any callback; not expected, both entry points were validated): `PilotVerdict::Refused`
  with the refusal as its stop, no further pilot, DOPRI5 continues.

Budget: while no pilot attempt was made the session enforces `max_steps` itself (so that its record is the one of
`Dopri5::solve`); afterwards the route stops when DOPRI5 attempts + pilot attempts reach `max_steps` (stop
`Dopri5 { AttemptBudget }` with a message, status `MaxStepsReached { max_steps }`). A pilot is skipped when no
attempt is left for it. The window of the last `b` attempts is kept only when `b <= max_steps` (else no pilot can
fire), so it never holds more than `min(b, max_steps)` entries.

## 4. Tests of `crates/vigilode/tests/unified_breakeven_contracts.rs`

Failing-first (commit A, `logs/failing_first_base.log`, each fails on the base by assertion, passes now):

| Test | Assertion that failed on the base | Base numbers | Now |
|---|---|---|---|
| `f2_fatal_above_one_completes_under_auto` | `solution.success()` (:298): "F2 must complete under Auto as it does under Dopri5::solve, got Failed(CallbackFailed { callback: Rhs, t: 0.0, .. })" | screen column at y0 + delta fatal, 2 entries, no leg | success, 49 calls, 0 fatal entries |
| `lorenz96_40_has_no_rodas5p_leg_and_costs_at_most_twice_dopri5` | no RODAS5P leg (:336) | RODAS5P only (n > 10), 33467 calls = 11.02x D, 54.3x operations | DOPRI5 only, 3233 calls (1.065x D), 1.29x operations, 4 lost pilots |
| `robertson_40_charges_at_most_two_direct_rodas5p_plus_window_and_pilots` | callback bound (:392): "6233 charged callbacks above 2 x 415 (direct RODAS5P) + 2 b c_D + 8 pilots = 974" | trial of 1000 DOPRI5 attempts, then RODAS5P | 385 calls, 36 attempts (8 DOPRI5, 1 lost pilot, 27 RODAS5P incl. the pilot) |
| `oscillator_1000_has_no_rodas5p_leg` | no RODAS5P leg (:430) | trial exhausted at t = 168.7, RODAS5P 3462 attempts, 41406 calls | DOPRI5 only, 35582 calls, 11 lost pilots |
| `arenstorf_has_no_rodas5p_leg` | no RODAS5P leg (:459) | screen -> RODAS5P, 4343 calls | DOPRI5 only, 1369 calls, 6 lost pilots |

Every number of the "Now" column equals the model prediction of DESIGN_RECOMMENDED.md section 6 (385, 3233, 35582,
1369, 49; P_heat_fd 945 below).

Added after the implementation: `session_quota_one_advances_keep_the_bits_and_counters_of_dopri5_solve` (step 2),
`break_even_length_is_the_table_of_the_design` (n = 1..2000 against the formula; the callback unit never binds),
`u1_a_run_without_handover_is_dopri5_solve_plus_its_pilots_within_twice_its_cost` (8 non-stiff problems: bits and
the whole DOPRI5 record of `Dopri5::solve`, plus exactly the lost pilots' attempts and calls, at most 2x in calls and
operations, pilots <= 1 + floor(log2(A_D / b)), pilot spend <= DOPRI5 spend; 48 lost pilots in all),
`u2_dopri5_pilots_and_rodas5p_share_one_attempt_budget` (Robertson with max_steps 100000, 500, 50, 36, 35, 20, 9,
8, 5, 1), `u3_no_callback_before_dopri5s_first_stage_and_every_other_entry_is_a_pilots` (entry sequence = DOPRI5's
with pilot blocks inside [t_D, t_D + H] inserted; after a handover nothing before the pilot's end),
`u4_a_handover_follows_an_accepted_pilot_and_is_recorded` (Robertson; P_heat_fd: 945 calls, pilots at 8, 14, 28,
56 lost and 112 accepted), `u5_same_inputs_give_the_same_route_counters_and_bits`,
`d1_a_fatal_callback_inside_a_pilot_ends_the_solve_with_the_accepted_prefix` (N2: CallbackFailed at the first
pilot, t_D = 9.79e-4, 33 calls, one fatal entry).

## 5. Contract edits in `unified_router_contracts.rs` (D4, D5)

36 tests in the file; 26 are unchanged and pass. Every edit:

| Test (base line) | Change | Decision |
|---|---|---|
| `p1_label_equals_executed_driver_on_every_rodas5p_driver_family` (:395, case :456-462) | the case "heat n=16, finite differences without structure, Auto" declares the regime Stiff (label "... declared stiff"); the other eight cases unchanged | D5: the case depended on `EXPLICIT_TRIAL_MAX_DIMENSION` (n = 16 > 10 ran RODAS5P). Without the limit an undeclared request of any dimension starts DOPRI5, so it failed ("label differs from the executed driver"); the declared regime keeps the finite-difference dense family in the cases. |
| `p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit` (:573), renamed `..._is_bounded_and_hands_over` | new helper `assert_bounded_with_a_recorded_handover`: success, the same work bound W (5890 calls, entries and charged), the last pilot accepted and a route-history entry `handover ...` | D4 |
| `p1_stiff_contractive_scalar_without_jacobian_is_not_explicit` (:586), renamed `..._is_bounded_and_hands_over` | the same helper, W = 181818 | D4 |
| part 2 imports (:1185-1189) | `EXPLICIT_TRIAL_ATTEMPT_LIMIT`, `EXPLICIT_TRIAL_MAX_DIMENSION` removed; `break_even_length` added | D5 |
| `p2_every_tier_has_a_driver_and_every_report_replays_as_direct_calls` (:1307) | the count of screened solutions becomes the count of solutions with pilots (`with_pilots`); the callbacks outside the replayed legs are those of the lost pilots instead of `analysis.rhs_calls` | D5 (`StiffnessScreen` and `PreflightAnalysis::screen`/`rhs_calls` removed) |
| `p2_declared_nonstiff_runs_dopri5_with_the_whole_budget_and_no_second_method` (:1540) | `analysis.screen.is_none() && analysis.rhs_calls == 0` -> `pilots.is_empty()` | D5 |
| `p2_legs_are_direct_calls_and_their_work_adds_up` (:1681) | rewritten for the legs of the break-even route: the DOPRI5 leg is `Dopri5::solve` with `max_steps` = its attempts; the RODAS5P leg is the pilot (`Rodas5p::solve` from DOPRI5's last state, `h0 = H`, one attempt) plus `Rodas5p::solve` from its end with `h0 = H` and the attempts left (attempts, calls and final bits equal); legs + lost pilots = total = entries; `fallback_steps` = 1; `inner_solution` starts at the pilot's end; invariant drift unchanged. The assertions on the trial budget 250 and the screen's 4 calls are gone. | D5 (screen and trial budget removed) |
| `p2_stiffness_screen_is_a_charged_lower_bound` (:1885), replaced by `p2_the_analysis_makes_no_callback_and_an_undeclared_request_runs_the_break_even_route` | the same five problems plus heat n = 64: no callback entry, `BreakEven { b: break_even_length(n) }`, `fallback_to_rodas5p`, first-leg budget = `max_steps`; the declared case (heat n = 64, dense Jacobian: `SuppliedDerivative`, no callback) kept | D5 (`StiffnessScreen`, `DOPRI5_REAL_STABILITY_BOUND`, `EXPLICIT_TRIAL_*` removed) |
| `p2_fallback_rule_and_trial_budget` (:1953), renamed `p2_fallback_rule_and_one_attempt_budget` | `explicit_trial_attempts` assertions removed; `remaining_attempts` and `follows_explicit_stop` assertions kept; max_steps = 1: `BreakEven { b: 4 }`, one DOPRI5 leg, one attempt, `MaxStepsReached { 1 }` (was `NoBudgetForTrial` and RODAS5P) | D5 |
| `p2_records_are_strict_json_and_options_keep_their_file_format` (:2130-2135) | the third solve (fatal for t > 0.3, span (0.5, 1)) stops at DOPRI5's first stage: one leg, a `Dopri5` stop with the callback, no pilot (was: no leg, `UnifiedStop::Preflight`) | D5 (`UnifiedStop::Preflight` removed) |

Unchanged and passing although the route changed under them: `p1_fallback_shares_one_attempt_budget_and_keeps_the_work_of_the_failed_leg`
(DOPRI5 leg before a RODAS5P leg, budget, counters; the handover counts in `fallback_steps`),
`p2_exhausted_shared_budget_reports_the_total_and_keeps_the_leg_stop` (max_steps 20: 8 DOPRI5 + 1 lost pilot +
the accepted pilot leave the RODAS5P run 10 attempts, so its own stop is again `MaxStepsReached { 10 }`),
`p1_fatal_callback_on_the_explicit_leg_is_not_retried_by_another_method`,
`p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input` (the first DOPRI5 stage is the failing call:
final time 0, one entry, charged), `p1_preflight_callbacks_are_charged_*` (their names keep the word "preflight";
there is no preflight call any more, and nothing is uncharged).

## 6. Log

* Step 5 results: see the end of this section (filled in after the runs).

## 7. The evaluation runner (step 6)

`router_hyperparameter_free/runner/`: a standalone package `v27-runner` (empty `[workspace]`, path dependency on
`../../../../crates/vigilode`, the repository's `[profile.release]`), `Cargo.lock` from the repository lock built once
with `--offline`, then `--locked`. `build.sh lock` creates the locks and builds both binaries; `build.sh` rebuilds
with `--locked`. Arm T is the same `src/main.rs` copied to
`/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v27/runner_t` with the dependency
`/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-v27base/crates/vigilode`, built in the target dir `w3-v27base`.
Outside the cargo feature `pilots` (arm N only: `VigilodeSolution::pilots`) the runner uses only names of both APIs.

One process runs one case: `v27-runner --arm N|T|D|R --problem <id> [--rtol x --atol y] [--max-steps k]
[--regime unknown|stiff|nonstiff] [--no-log]` and prints one JSON line (`schema vigilode.v27.runner.v1`): the options,
status, typed stop, success, final time and state with bits, the charged counters, attempts and steps, DOPRI5 and
RODAS5P attempts, operations by the formula of PREDECLARED.json (as `operations3`, three times the count, exact),
callback entries counted in the closures (right-hand side, Jacobian, JVP, time derivative, fatal), the right-hand-side
entries classified as stage, finite-difference column or finite-difference time derivative (plus the entries at t0,
whether the first entry is (t0, y0)), the legs, the route history, the pilot records (arm N) and the FNV-1a 64 route
digest of legs, history and pilots. Problems: every id of PREDECLARED.json `problems`, plus `toy` (y' = -y on [0, 1]).
Choices fixed in the runner where PREDECLARED.json leaves them open: the start-rule fixtures with n = 8 use the spans
of their V03b fixtures (P_heat_dense (0, 10) with y0 = sin(pi x); P_scalar_dense (0, 1); P_heat_jvp and
P_heat_banded (0, 0.01) with y0 = 1); P_mass is the V03b DAE fixture on (0, 1); P_blowup on (0, 2); the scalar
fixtures and kt_vanishing are non-autonomous, all others autonomous. `check_toy.py` is the phase-1 plumbing check.
