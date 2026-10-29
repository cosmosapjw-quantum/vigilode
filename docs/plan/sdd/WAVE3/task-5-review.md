# Task 5 review: p0-events (997fefe..547cd79)

Review is from the diff and the brief. I ran no build or tests and rely on the implementer's reported results. Ledger rulings (STATS_SCHEMA deferred to T3; NaN truncates to step start) and R-C3/R-C4 are applied and not re-raised.

### Spec Compliance

- Spec compliant. The seven checklist points hold; details below.
- Cannot verify from diff (controller, at T3 rebase):
  - `truncate_after` covers only the sequential `Vec<Rodas5pDenseSegment>`. That is correct at this base. T3 must extend it to the fast store and keep the `t > last` bound in `at()` (solution.rs:653).
  - The claim that `Solution::y[k+1]` is bitwise equal to `at(t[k+1])` is assumed from the `at()` doc ("bit for bit"). The terminal-at-step-end path in events.rs:279-285 relies on it, because it skips the push and keeps the stored `y`.

### Checklist

- **Illinois (events.rs:111-154): correct.**
  - The secant point is c = (a·hb − b·ha)/(hb − ha).
  - A retained endpoint's value is halved only when the same side moves twice in a row (`last_side`, :137-148). `ha` is halved when b moves twice, `hb` when a moves twice.
  - Termination matches the brief: width ≤ 4ε·max(|a|,|b|,1), or 60 iterations (:123, :78). The bisection fallback when c is outside (a,b) prevents a stall.
  - Bracket signs are normalised by `sign`, so Down is handled as Up on −g. The reported end is `b`, the after-crossing side.
- **Direction filter (:178-184): correct.**
  - Up and down are mutually exclusive, and the `match` arms apply the filter for Up, Down and Either.
  - The observed direction is recorded as `up`.
  - The strict tests (`ga<0`, `ga>0`) give the t0 rule and the shared-endpoint rule. A step start with g = 0 starts no crossing, so the previous step reports it once. The midpoint is covered the same way.
- **Terminal events: consistent.**
  - Cut selection sorts by (t, index), takes the first terminal crossing, and keeps same-time non-terminal ones and drops later ones (:256-273).
  - `truncate_after(te)` keeps `t ≤ te` and segments with `t_old < te` (solution.rs:672-680). For `te` inside a step it pushes `(te, ye)`. For `te == t[k+1]` it does not push.
  - `at()` returns None for `t > t.last()` (solution.rs:653), so the kept segment is not extrapolated.
  - `Stats.terminal_event` indexes `found`, which becomes `events`, so it is not the declaration index. Test `terminal_index_is_into_events_not_declaration` pins this in both declaration orders.
  - Status becomes Success with `stats.stop` kept. Later work stays charged in the counters, and this is documented in problem.rs, events.rs and lib.rs.
- **Non-finite g:** truncates to `t[k]` and keeps earlier events (the ruling). The failing step's crossings are dropped. The evaluation count includes the failed call (:244 runs before the `match`).
- **Boundary cases:** t0 (`zero_at_t0_is_not_an_event`) and step end plus midpoint shared endpoint (`crossing_at_step_endpoint_reported_once`) are tested. Several events in one step are ordered by the sort. Multiple event functions are handled with per-spec `previous`.
- **Error typing:** `validate()` refuses events without `dense_output` with `Unsupported` before any step (solve.rs:833-838).
- **Accounting:** `event_evaluations` is a separate Stats field and does not touch `counters`. Test `events_do_not_change_integration` compares t, y (bitwise) and counters.

### Strengths

- Post-processing leaves integration untouched. The bitwise test (h) holds by construction.
- The sign-normalised Illinois is small and the bisection fallback is safe.
- The `at()` bound and `truncate_after` have four focused unit tests, including boundary and extreme arguments.
- The index-semantics test closes B5b.
- The doc comments say plainly that stopping is post hoc and that work stays charged.

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
None.

#### Minor (Nice to Have)

1. **Missed-root limitation is understated.**
   - problem.rs:456-457 says "Two roots between neighbouring samples can be missed". The real limits are broader:
     - Any even number of crossings in a half interval is missed.
     - A tangent touch with no sign change is missed.
     - With three or more roots in a half interval, the reported direction is the half interval's sign change, not the local one.
   - lib.rs "Events" (lib.rs:324-332) does not mention the limitation at all, and `Solution::events` has no note.
   - Fix: one sentence in the lib.rs section and in `Problem::event` that states the three-sample rule and its consequences.
2. **`two_crossings_in_one_step_are_both_found` (events_contracts.rs:1086-1102) is misnamed and weak.**
   - It passes with zero events, and its own comment concedes a double crossing may be missed.
   - Inferred, not run: if the solver takes one step on [0,4], the samples 0, 2, 4 give g = 2, 0, 6. Only a Down event at t = 2 would be found, and the root at t = 1 would be missed.
   - Fix: rename it (for example `events_found_are_correct_and_ordered`), or assert the documented outcome. A deterministic test of several events in one step, with sample-separated roots, is still missing: the test would use g = sin(k t) on a large step and assert the order. The same goes for a terminal event at a shared step end.
3. **The 60-iteration exit returns `b` unchecked (events.rs:122-152).** If the loop ends on the cap with a wide bracket, the reported time can be far from the root. This is unlikely for Illinois with bisection fallback on smooth g. Consider documenting it or returning the better of a and b.
4. **`events_do_not_change_integration` is run only with one non-terminal event on an oscillator.** That is as the brief states. A fast-route rerun belongs to T3's rebase, since the seam test lives in T8.
5. **Silent no-op paths.**
   - `locate` returns no events if `extension.is_none()` (:200). Validation prevents that today, but a dense run whose driver returns no segments would then produce silently empty `events` with no signal.
   - The `unwrap_or_else(|| solution.y[k + 1].clone())` at :267 hides an impossible state.
   - An `at()` miss inside Illinois is reported as `EventFunctionNonFinite` (:104), which mislabels it.
   - A `debug_assert!` on each would be better.
6. **The non-finite path overwrites a driver failure.** The solution then reports `EventFunctionNonFinite` in place of the driver's own failure reason. `stats.stop` still holds the driver record. A line in `FailureReason::EventFunctionNonFinite`'s docs would make this explicit.

### Assessment

**Task quality:** Approved

**Reasoning:** The Illinois method, the half-interval and endpoint rules, terminal truncation across t, y and the extension with the `at()` bound, the Unsupported refusal and the separate accounting are all implemented as the brief and rulings require, with the brief's tests present. The remaining items are documentation breadth and test strength.
