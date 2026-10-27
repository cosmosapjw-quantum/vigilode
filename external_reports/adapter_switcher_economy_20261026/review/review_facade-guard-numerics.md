# Adversarial review — lens: facade, Guard and numerical governance

Worktree `/home/cosmosapjw/vigilode-adapter-perf`, branch `research/adapter-switcher-perf-20261026`,
diff `4fe67602..HEAD` (code commit `636cdc7a`, ledger commit `0d3c7c51`). Read-only review (git + file reads).

Verified and not reported (no defect found): `Guard::call_charged` charge-after-`Ok` ordering and latch are
consistent with the `fatal()`-first status mapping; `ceiling_hit()` cannot coexist with a fatal record;
every core counter (`rhs_calls`, `ft_calls`, `jacobian_builds`, `jvp_vectors`) is incremented before or
regardless of the callback outcome, so the call that trips the ceiling stays counted inside the drivers'
attempt loops (`integrate_fast`, matrix-free, banded all share the `FatalCallback` arm and
`DriverPartial::after_loop` is `Some` whenever `t < tf`); the public `Rodas5p::solve` delegates with
`(&[], None)` and is behaviourally unchanged; the probe loop's `advance_to(target, 1, ..)` is bounded by
the session's global budget (`SessionAdvance::Stopped`); `stats_since` deltas are of monotone cumulative
counters; `floor() as u64` saturates for huge rates and the inputs cannot be NaN; division denominators
`(batch_target - start)`, `(target - start)` are strictly positive and `accepted_steps > 0` is guarded.

## F1 — BLOCKER — A ceiling hit during routed band verification aborts the whole switching solve

- `crates/vigilode/src/solve.rs:208` (`RoutingError::Structure(s) => SolveError::StructureRefused`), with
  `solve.rs:779` (`guard.call_charged(Callback::Jacobian, t, n as u64, ..)` inside the dense provider that the
  declared-band `fill` at `solve.rs:803-813` calls), `crates/rodas5p-integrators/src/routing.rs:479`
  (`verify_declared_band(problem, t_span.0, y0)?` runs before any integration step), `routing.rs:321-322`
  and `:336` (the verification calls the band fill, i.e. the dense Jacobian, and then `dense_jacobian`
  again: `2n` charged units, errors wrapped as `StructureError::BandVerificationFailed`), and
  `crates/vigilode/src/switching.rs:828-836` (an `Err` from the pilot `run` sets
  `accounting_complete = false`, commits the explicit record and `break 'windows`).
- Claim: on the `JacobianRecord::DenseDeclaredBand` route (user dense Jacobian + `Problem::band`, explicitly
  allowed by the switching `validate`, `switching.rs:1407-1416`) a bounded entry pilot whose ceiling is below
  `2n` does not become a discarded pilot; the guard's `FatalCallback` is converted by the router into a
  structure refusal, the facade returns `Err(SolveError::StructureRefused)` instead of
  `Status::Failed(WorkCeilingReached)`, and the adapter ends the entire solve with `success = false`,
  `accounting_complete = false`. The verification's charged Jacobian work (`charged` is local to
  `verify_declared_band` and dropped on `Err`) is in no segment.
- Scenario: `Problem::new(64, rhs).jacobian(Jacobian::dense(..)).band(1, 1)`, `SwitchingConfig::evidence_driven
  (ProvidedOnly)` with `max_dense_dimension: 64`. An explicit batch reaches pressure at 24 attempts
  (~145 RHS) → `entry_ceiling = floor(0.8 * 145) = 116`. The pilot's verification charges 64 (band fill) + 64
  (dense reference) = 128 > 116 → second call answered `FatalCallback` → `StructureRefused("band
  verification failed: dense Jacobian: ... exceeds the work ceiling 116")` → the adapter stops at that
  checkpoint. With the default `explicit_attempt_budget = 32` (ceiling at most ~154) every pilot aborts for
  `n >= 78`; before this commit the same pilot ran to its attempt budget and was discarded benignly.
- Fix: never let a ceiling stop surface as `Err`. In `solve_observed_bounded`, when `guard.ceiling_hit()`
  is set after a `RoutingError`/driver `Err`, return `Ok(Solution)` with `Status::Failed(WorkCeilingReached)`,
  `t = [t0]`, `y = [y0]` and the verification work counted (have `verify_declared_band` return its partial
  `charged` on error, or arm the ceiling only after routing via a `Guard::arm_ceiling()` so the verification
  is counted in `work_units` but not bounded — `work_units >= charged` still holds). Add a contract test on
  the dense-declared-band route with a ceiling below `2n`.

## F2 — MAJOR — Coalesced landings are not deduplicated; equal window endpoints abort the solve

- `crates/vigilode/src/switching.rs:638-645` (landings = `window_target` of `window..window+windows-1`
  filtered only by `> start`), `crates/vigilode/src/solve.rs:156-168` (filter `> t0 && < t1`, then
  `OutputSchedule::new`), `crates/rodas5p-integrators/src/output.rs:475-479` (schedule must be strictly
  increasing → `InvalidInput`), `switching.rs:662-666` (`Err` → `accounting_complete = false`, `break`).
- Claim: `window_target` is monotone but not injective when `(t1 - t0) / window_count` is below the clock
  resolution at `t0`; two equal interior landings make the facade refuse the run before any callback, and
  the adapter aborts. The pre-change code tolerated this: per-window runs skipped `target <= final_time`
  (`switching.rs:536-538`) and both explicit schedules deduplicate (`switching.rs:417`, `:944`). Only the new
  implicit coalesced path lost that protection.
- Scenario: `t0 = 1e6`, `t1 = 1e6 + 1e-6`, `window_count = 100_000` (validated up to 1e6), spacing 1e-11 <
  ulp(1e6) = 1.16e-10 so consecutive targets collapse; `implicit_pilot_attempt_budget: 0`, `pressure_steps:
  1` → the first explicit window shows pressure and enters RODAS; the batch after residence coalesces
  `probe_interval = 4` windows with three equal landings → "output schedule must be strictly increasing" →
  `success = false`, `accounting_complete = false`. (The codebase audits such spans: R3-TIME-02.)
- Fix: build `landings` with strict monotonicity (`filter(|t| *t > last_pushed)`), or in
  `solve_observed_bounded` keep only `t > previous` (and `< t1`), and add a test with a sub-ulp window spacing.

## F3 — MINOR — The return-hysteresis baseline silently changed from the last window's rate to the coalesced batch mean

- `crates/vigilode/src/switching.rs:878` (`rate = record.work_units / (batch_target - start)` over `windows`
  windows) and `:890` (`implicit_rate = Some(rate)`), consumed by the return ceiling `:625-627` and the
  economical test `:686-688`.
- Claim: `return_cost_ratio` is unchanged but is now applied to the mean rate of up to `probe_interval`
  windows instead of the last window's rate. The preregistration lists "no entry/return threshold relaxed"
  and the doc comment says "neither bound changes a decision" (true for the bounds), but the coalescing
  itself changes which return decisions the registered fraction makes, and this is not named in the change
  list.
- Scenario: implicit per-window work 40, 30, 20, 10 units over one 4-window batch (decaying transient):
  legacy baseline 10/window, new baseline 25/window. A probe costing 15 units/window is discarded by the
  legacy rule (15 > 8) and returned by the new rule (15 <= 20) — an effectively looser return in decaying
  regimes (tighter in growing ones).
- Fix: register the baseline definition explicitly (batch mean) in `PREREGISTRATION.md`/`SwitchingConfig`
  docs, or keep the per-window baseline by deriving the last window's share from the landed rows.

## F4 — NOTE — Recoverable-error callbacks are not charged against the ceiling

- `crates/vigilode/src/fd.rs:133-153`: `charged_units` grows only on `Ok(())`; a `CallbackError::Recoverable`
  call enters user code, is counted in `WorkCounters` (core charges before the call, `problem.rs:594`) and
  in `recoverable_errors`, but not against the ceiling.
- Claim: the economy bound is weaker than "stops as soon as its charged callback work exceeds": a pilot whose
  RHS/JVP keeps returning `Recoverable` (domain guards on a stiff trial step) is bounded only by its attempt
  budget / minimum step. Decisions stay preserved (`work_units >= charged`).
- Fix: charge before entering user code (`fetch_add` then call, keep the charge on `Err`); the bound becomes
  tight and still never stops a pilot that could pass.

## F5 — NOTE — Legacy probes (`carry_explicit_step = false`) now land on `target` as an interior observation

- `crates/vigilode/src/switching.rs:1098` (`if carry_explicit_step || probe`) and `:1116-1121`
  (`times = [start, target, solve_end]`, span `(start, solve_end)`). `end_step_capped` equals `land_capped`
  to `tf` (`output.rs:143-145`), so the step sequence and RHS work are the same as the legacy
  `Dopri5::solve(start, target)`; but the final step is now reported `shortened`/`clipped = true`
  (`output.rs:110-117`) where the legacy terminal step was `clipped = false` because `adaptive_end_step`
  (`adaptive_output.rs:244-250`) had already shortened the request. `output_clipped_steps` in the probe's
  driver record differs by one; no decision input depends on it.
- Fix: none required; note it in the record docs if `output_clipped_steps` is compared across versions.

## F6 — NOTE — Additive public record changes without a schema note

- `crates/vigilode/src/solution.rs:109` (`FailureReason::WorkCeilingReached`, serialized as
  `kind = "work-ceiling-reached"` under `STATS_SCHEMA = "vigilode.stats.v4"` whose doc enumerates per-version
  changes, `solution.rs:30-37`), `crates/vigilode/src/switching.rs:211` (`SwitchingSegment::windows`) and
  `:236` (`decision: &'static str`) under `schema: "vigilode.switching.v1"` (`switching.rs:360`).
- Claim: the serialized contracts gained a variant and a field (and the `decision` strings changed) with no
  schema-version note; the `compile_fail` doctest in `lib.rs:140-158` still holds (`#[non_exhaustive]`).
- Fix: add a v4/v1 doc note or bump the schema strings per the facade's own convention.
