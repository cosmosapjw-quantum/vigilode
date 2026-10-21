## Task 5: P0-events — event location with root finding on the dense extension (node id `p0-events`)

Plan §3 P0 ("events with root finding"), §3 API (`.event(|t, y| y[0] - 1.0, Direction::Down)`).

**Files:**
- Create: `crates/vigilode/src/events.rs`.
- Modify: `crates/vigilode/src/{problem.rs, solution.rs, solve.rs, lib.rs}`.
- Test: `crates/vigilode/tests/events_contracts.rs`.

**Interfaces (new):**
- `pub enum Direction { Up, Down, Either }`.
- `Problem::event<G>(self, g: G, direction: Direction) -> Self` and `Problem::terminal_event<G>(self, g: G, direction: Direction) -> Self`,
  with `G: Fn(f64, &[f64]) -> f64 + Send + Sync + 'static`.
- `pub struct EventRecord { pub index: usize, pub t: f64, pub y: Vec<f64>, pub direction: Direction }`.
  `index` is the declaration order, and `direction` is the observed Up or Down.
- `Solution::events: Vec<EventRecord>` (time order). `Stats::terminal_event: Option<usize>` (an index into
  `events`) and `Stats::event_evaluations: u64`.
- `FailureReason::EventFunctionNonFinite`.
- `pub(crate) fn Solution::truncate_after(&mut self, t: f64)`, the only access to the segment store.

**Exact rules (post-processing over the accepted steps; the integration is unchanged):**
- Events require `dense_output = true`; otherwise the solve is refused with `Unsupported`.
- On each accepted step [t_k, t_{k+1}], evaluate g at t_k, at the midpoint (via `at`) and at t_{k+1}. A
  crossing on a half-interval [a, b]: Up when g(a) < 0 ≤ g(b), Down when g(a) > 0 ≥ g(b).
- Localize by Illinois (modified regula falsi) on g(at(t)) until the bracket width is
  ≤ 4ε·max(|t|, 1) or 60 iterations. Report the bracket end on the after-crossing side.
- g(t0) = 0 is not an event. A crossing whose b is a shared step endpoint is reported once.
- A non-finite g value gives `Status::Failed` with `EventFunctionNonFinite`. The solution is truncated at
  that step.
- The first terminal event in time truncates `t`, `y` and the segments after it, and appends `(t_e, y_e)`.
  The status is `Success` even if the driver failed later, with `Stats::stop` kept. Work after the event
  stays charged in the counters, and the docs say so. An in-loop stop is a wave-4 item.

- [ ] **Step 1: Write failing tests:**
  - (a) `exponential_decay_half_life`: y' = −y, g = y − 0.5 Down, rtol 1e-8; |t − ln 2| ≤ 1e-6.
  - (b) `oscillator_crossings`: y'' = −y as a 2-system on [0, 10], g = y₀, rtol 1e-8. Either gives 3 events
    at π/2, 3π/2 and 5π/2 within 1e-6; Up gives only 3π/2.
  - (c) `terminal_event_truncates`: (a) as terminal. `t.last() == t_e`, `at(1.0) == None`, status
    `Success`, `terminal_event == Some(0)`.
  - (d) `events_without_dense_output_refused`.
  - (e) `zero_at_t0_is_not_an_event`.
  - (f) `crossing_at_step_endpoint_reported_once`: run once, take an interior accepted t_k, rerun with
    g = t − t_k; exactly one event.
  - (g) `nonfinite_event_function_fails_typed`.
  - (h) `events_do_not_change_integration`: `t`, `y` and counters are bit-identical with and without a
    non-terminal event.
- [ ] **Step 2: Run** `cargo test -p vigilode --locked -j 1 --test events_contracts`; expect a compile failure.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p vigilode --locked -j 1`; all pass.

