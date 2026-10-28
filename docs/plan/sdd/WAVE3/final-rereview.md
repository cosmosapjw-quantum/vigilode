VERDICT: ALL ADDRESSED

# Wave-3 final fix re-review, 4945683..5c6978d (commit 5c6978d)

Ran `cargo test -p vigilode --test p0_contracts --locked -j 2` (23 passed, including both new tests). Also ran the focused integrator targets `fast_vector_atol_contracts` (5 passed) and `rodas5p_fast_allocations` (3 passed).

## Finding verdicts

- **I1. Facade seam (i) test, uniform vector equals scalar under dense output** — ADDRESSED. `crates/vigilode/tests/p0_contracts.rs` `uniform_vector_atol_equals_scalar_under_dense_output`, diff lines 553-609.
  - Routes: the case table has Robertson (dense) and the tridiagonal `.banded(1,1)` problem, each under `WFilteredEnforce` and `Off`. `b.stats.route.entry_point` is asserted to be the `..._dense_observed_with_atol` and `..._banded_dense_observed_with_atol` entry points. `a` is asserted to differ from `b`.
  - Comparison: `bits(&a.t)` against `bits(&b.t)`, and `bits` of every `y` row. `at()` at every step midpoint is compared with `bits`. `counters`, `driver_work` (includes `FastDense` and `interior`), `adaptive` and `accepted_steps` are compared with `assert_eq!`.
  - Control: a nonuniform `[1e-10, 1e-10, 1e-6]` vector must change `driver_work`. It runs on Robertson only, so there is no banded control. That is acceptable.
- **I2. Facade seam (ii) test, terminal events across stores and routes** — ADDRESSED. `terminal_event_on_every_dense_store`, diff lines 625-745.
  - Routes: five cases (fast WFilteredEnforce, fast Off, fast banded, sequential Enforce, sequential mass). `driver_id` is asserted for each, on both the terminal and the base run. This covers all four routes: fast dense, fast banded, sequential Enforce, sequential mass.
  - Truncation: `t.last() == t_e`, `t` strictly increasing, `y.last()` bitwise equal to `events[0].y`, one event, `stats.terminal_event == Some(0)`.
  - `at()` bound: `at(t_e)` is `Some`. `at()` is `None` at the midpoint of `(t_e, next)`, at `next`, and at `t1`.
  - Extension consistency: `at(t_k)` is bitwise `y[k]` for every kept output. Interior `at()` midpoints are bitwise equal to the untruncated run.
  - Step-end case: a second loop runs `g = t - t_k` with `Up` on every case. It asserts one event, `t.len() == k+1`, no duplicate time, `y.last()` bitwise `base.y[k]`, `at(t_k)` is `Some` and `at(t_k + 1e-9)` is `None`.
- **M1. Success and stop docs vs terminal-event semantics** — ADDRESSED. `solution.rs` diff lines 351-364 (`Status` and `Success`), 417-426 (`StopRecord::t`), 444-448 (`Solution::t`) and 469-471 (`is_success`).
- **M2. `segments` doc** — ADDRESSED. Both `FastDense` and `SequentialDense` now read "before any event truncation" (`solution.rs` diff lines 386-388 and 410-412). The counter value is unchanged.
- **M3. Three stale `Unsupported` messages** — ADDRESSED. `error.rs` diff lines 212-236 reword all three. `display_text_is_the_former_message` in `p0_api_contracts.rs` (diff lines 495-516) is updated to the new text and passes.
- **M4. `STATS_SCHEMA` v3 note** — ADDRESSED. `solution.rs` diff lines 326-331 add the vector-atol, `adaptive.atol` max, entry-point and `FastDense` shape lines. The second golden was optional and was not added. The reviewer marked it optional, so this does not hold the verdict.
- **M5, M6** — parked to wave 4. Not judged.
- **p0-atol Minor 2, asymmetric vector** — ADDRESSED. `fast_vector_atol_contracts.rs` `vector_entries_weight_their_own_component`, diff lines 61-114.
  - Design: a diagonal 3-component problem with exactly one active component at index 0, 1 or 2. The other two components are constant and add exactly zero error. The vector puts `1e-12` at the active index and `1e-4` and `1e-8` at the others. The vector run must equal the scalar run with `1e-12` in `attempts`, `counters` and `y` bits.
  - Reversed zip: with the active component at index 0 or 2, a reversed zip would read `1e-8` instead of `1e-12`. That changes the scale by four orders of magnitude, so `attempts` or the `y` bits differ and the test fails. Index 1 is symmetric under reversal, but cases 0 and 2 catch it. A control asserts the atol changes the run. I judged this by reading the code. The mutation result (the test fails under a reversed zip at `rodas5p_fast.rs:1102`) is the implementer's claim and I did not rerun it.
- **p0-atol Minor 3, allocation case with a nonempty `atol_vector`** — ADDRESSED. `rodas5p_fast_allocations.rs` `a_vector_atol_step_allocates_nothing`, diff lines 159-197.
  - It calls `integrate_rodas5p_fast_observed_with_atol` with `atol = [1e-4*rtol; 3]`. That entry point does `work.atol_vector = atol_vector.to_vec()` (`rodas5p_fast.rs:1884`), so the vector is nonempty and the branch at `:1096` takes the vector path.
  - It asserts that the allocation count is the same at two rtols whose attempt counts differ by at least 3x, and at most the scalar count plus 2.
- **p0-api doc Minor 1, "Every refusal" overstated** — ADDRESSED. `lib.rs` diff lines 285-291 now say "Every unsupported-feature refusal".
- **p0-api doc Minor 2, stray `//!`** — ADDRESSED. The extra `//!` line is removed (diff line 294).
- **p0-api doc Minor 3, `compile_fail,E0004`** — ADDRESSED. `lib.rs` diff line 299.
- **p0-api doc Minor 4, future payload note** — ADDRESSED. "a future payload becomes a new variant" is in `lib.rs` diff lines 289-291.
- **p0-api doc Minor 5, `SolveError::Unsupported` doc** — ADDRESSED. `error.rs` diff line 259 now says "(see [`Unsupported`])".
- **p0-api doc Minor 6** — the reviewer found nothing to fix, so no verdict is needed.

## New defects in the fix diff

None. No solver arithmetic changed; the diff is tests, docs and message text.

- Minor, note only: the allocation test's bound `vector <= scalar + 2` carries a small margin. It is deterministic and passed, and wave 4 already plans an allocation margin rework (M5).
- Minor, note only: the I2 test only checks the event time against the analytic value for the decay routes. The banded case has no analytic time, which is why it is skipped there.

## Out-of-scope observations

None.
