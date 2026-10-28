# Task 5 report: p0-events

Branch production/p0-events, base 997fefe (holds p0-atol). Commit 547cd79, pushed.

## Requirements
- Direction, EventRecord (Serialize, per B5d): crates/vigilode/src/events.rs
- Problem::event / terminal_event: problem.rs:225, :240 (specs in `Problem::events`)
- Solution::events solution.rs:400; Stats::terminal_event :374, Stats::event_evaluations (next field)
- FailureReason::EventFunctionNonFinite (unit variant, no payload): solution.rs:77
- Refusal of events without dense_output -> Unsupported, built into validate() (kept AtolSpec return): solve.rs:620
- R-C3: `at()` returns None for t > *self.t.last() (solution.rs:425 area, extension branch); `truncate_after` solution.rs:449 over the sequential Vec<Rodas5pDenseSegment> (keeps outputs with time <= t, segments with t_old < t, at least t0). 4 unit tests in solution.rs.
- Locate (events.rs): half-interval rules, Illinois to 4eps*max(|t|,1) or 60 iterations, after-crossing end reported, g(t0)=0 no event, shared endpoint once (follows from the strict sign tests), first terminal in time truncates and appends (t_e, y_e), status forced to Success with stats.stop kept, counters untouched (documented in Problem docs, events.rs and lib.rs "Events").
- Call from solve.rs after Solution is built.

## Tests
RED: `cargo test --test events_contracts` failed to compile (52 errors: Direction, event, terminal_event, events missing). GREEN: 12/12 in events_contracts (a-h plus terminal-index-vs-declaration (B5a/B5b), work-charged, terminal-beats-later-driver-failure, double-crossing sanity). `cargo test -p vigilode --locked -j 1`: all pass (8 unit, 12 events, 20, 17, 1 doctest). clippy -D warnings clean for vigilode; cargo doc clean; check-research-node PASS.

## Decisions / deviations
- Non-finite g: truncated to the start t_k of the failing step; that step's crossings are dropped, earlier events kept. The brief says only "truncated at that step".
- `at()` returning None inside Illinois (cannot happen within the span) is treated as non-finite.
- If g(b) == 0 at a bracket, the endpoint b is returned without iterating (needed for endpoint events); with only three samples per step, a double root inside the half interval ending at a zero is reported at that end (documented limitation: roots between neighbouring samples can be missed).
- Several crossings at the same time as a terminal one are kept; later ones dropped.
- STATS_SCHEMA left at v2 although Stats gained two fields (the brief does not ask; T7 owns schema/golden questions). Open question for the controller.
- Event-function evaluations are counted separately (event_evaluations), not in counters.

## Handoff commands
None heavy. T3 rebase must keep the `t > last` bound and extend truncate_after to FastDenseSegments.

## Open questions
STATS_SCHEMA bump; non-finite truncation point.

## Fix round 1
- Minor 1: three-sample rule and its consequences (even crossings and tangent touches missed; 3+ roots report the half interval's direction) documented in Problem::event, lib.rs "Events" and Solution::events.
- Minor 2: renamed to `events_found_on_a_large_step_are_roots_and_ordered` (sin(3t), asserts true roots, strict order, at least one); new `even_crossings_in_a_half_interval_and_tangent_touches_are_missed` asserts the documented outcome deterministically.
- Minor 3: after 60 Illinois iterations the loop now finishes by bisection until the 4 eps max(|t|,1) tolerance holds (matches the brief's tolerance contract). `iteration_cap_falls_back_to_bisection` (step function with a -1e-10 low side): verified it FAILS (t = 1.70567...) with the bisection disabled and passes with it.
- Minor 6: the non-finite path still truncates but keeps an existing Status::Failed; docs on Problem::event and FailureReason::EventFunctionNonFinite; test `nonfinite_event_keeps_an_earlier_driver_failure`.
- Minors 4 and 5 left as instructed.
- Tests: events_contracts 15/15, vigilode suite green.
