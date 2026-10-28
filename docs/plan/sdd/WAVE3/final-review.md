# Wave-3 final whole-branch review: production/wave3-integration 75a0c6c..4945683

Reviewer: final whole-branch reviewer, one pass by area and then one pass by seam. No subagents were used and no repo files were edited.

## What was checked and run

The review covered these files:

- Facade: `solve.rs`, `solution.rs`, `events.rs`, `error.rs`, `options.rs`, `problem.rs` and `lib.rs`.
- Integrators: `rodas5p_fast_dense.rs`, the `rodas5p_fast.rs` diff (the sink, `atol_vector` and `solve_into`), and the `lib.rs` exports.
- `production.yml`, the 4 examples, the bench crate's use of the facade, the ledger, and the T3/T4/T5/T7 reviews for parked items.

Commands run on HEAD 4945683, with stable 1.97 and the requested env:

| Command | Result |
| --- | --- |
| `cargo test -p rodas5p-integrators --test fast_dense_allocations --test fast_dense_contracts --test fast_vector_atol_contracts` | 1 + 8 + 4 passed |
| `cargo test -p vigilode --all-targets` | lib 9, events 15, facade 20, p0_api 9, p0 21; all passed |
| `cargo test -p vigilode --doc` | passed, including the `compile_fail` doctest |
| `cargo run --example {robertson, dense_output, dae_mass, banded_fd}` | all exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode --no-deps` | clean |
| `cargo clippy -p vigilode --all-targets --no-deps -- -D warnings` | clean |
| `cargo check --manifest-path crates/vigilode-bench/Cargo.toml --all-targets --locked` | exit 0, so `#[non_exhaustive]` does not break the bench |
| `python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee` | **PASS** (141 rows; new nodes `b1c_staleness_20261021`, `bench_wave2_20261021`), exit 0 |

The scratch seam probe is `/tmp/claude-0/-home-claude/a52bd605-7e54-56e7-9ec5-97aec2d2b931/scratchpad/probe/src/main.rs`. It is a separate crate on the facade, and it runs. All its assertions held:

- **P1, seam (i) at the facade.** A uniform `Atol::Vector` gives the scalar run bit for bit under dense output. This held on both fast dense entry points (dense and banded), under both `WFilteredEnforce` and `Off`. The probe compared `t` and `y` bits, `counters`, `driver_work`, `adaptive`, and `at()` at every step midpoint.
- **P2, seam (ii) on all 4 stores and routes** (fast WFiltered, fast Off, sequential Enforce, sequential mass). After a terminal event:
  - `t.last() == t_e` and `t` is strictly increasing;
  - `at(t_k)` is bitwise `y[k]` for every kept k;
  - `at()` is `None` on `(t_e, t_{k+1}]`;
  - interior `at()` equals the untruncated run bitwise.
- **P3.** A terminal event exactly at a step end (`g = t - t_k`) gives one event and no duplicate `t`. `y_e` is the stored state bitwise.
- **P4.** In the terminal-before-blow-up case, `stop.t = 1.0000000041` while `t.last() = 0.5`, and `is_success() == true` (see M1).
- **P5.** A vector atol with an event on Robertson runs on the fast dense `_with_atol` route and finds the event.

## Strengths

- **Seam (i) is correct by construction.**
  - `DenseSink::estimate` reads `work.atol_vector[c]` when the vector is set and the scalar otherwise (`rodas5p_fast_dense.rs:350-356`).
  - The embedded norm has the same split (`rodas5p_fast.rs` error-norm hunk).
  - The facade's `AtolSpec::adaptive_atol` (max entry) makes a uniform vector produce the scalar `adaptive` record exactly.
  - The driver-level test `uniform_atol_vector_is_the_scalar_dense_run` covers dense HIRES and banded Brusselator.
- **Seam (ii) is correct on both stores.**
  - `Extension::truncate_after` uses the same rule "keep segments with t_old < t" for `Sequential` and `Fast` (`solution.rs:504-516`).
  - `Solution::at` is bounded by `t.last()` before any extension lookup (`solution.rs:543`), so a kept segment that contains `t_e` is never extrapolated.
  - `locate` appends `(t_e, y_e)` only when `t_e` is past the kept outputs (`events.rs:278`), which gives the step-end case its correct no-duplicate behaviour.
- **The refusal matrix is consistent.** `validate()` (`solve.rs:671-697`), the 7 `Unsupported` variants (`error.rs:39-82`), the "What is refused" paragraph and routing table (`lib.rs`), the `Atol::Vector` and `Options::dense_output` docs (`options.rs`), and `p0_api_contracts.rs` (one test per variant) all agree on these cases:
  - vector atol on the matrix-free, sequential and declared-band routes;
  - JVP with mass or dense output;
  - a declared band on a dense Jacobian with mass or dense output;
  - events without dense output;
  - a band declaration on a JVP problem.
- **The precedence is deterministic.** Checks run in a fixed order: JVP+dense/mass, then declared band, then vector atol, then events. For example, JVP + dense output + vector atol gives `JvpWithMassOrDenseOutput`.
- **The NoSink path is generic and `#[inline(always)]`.** Test (a) pins that the existing entry points are unchanged bit for bit.
- **The lagged driver stays unrouted.** No reference to it exists in the facade or the router.
- **CI seam (iii) is satisfied structurally.** The test job runs whole packages, so the 5 new test targets are included without a list (`production.yml:96-102`).
- **`#[non_exhaustive]` does not break the examples or the bench.** The examples use only `is_success()`, field reads and `Options { .., ..Default::default() }`. The bench uses only `==` and `matches!`. Both build.

## Issues

### Critical (Must Fix)

None.

### Important (Should Fix)

#### I1. Seam (i) has no facade-level test

- **Where:** plan Task 8 (i) requires the test in `crates/vigilode/tests/p0_contracts.rs`; it is missing there. The only facade vector-atol tests are `facade_contracts.rs:1032` and `:1236`. They cover the non-dense routes, plus one dense-output `is_success` and entry-point check (`facade_contracts.rs:1265-1279`).
- **Gap:** no facade test compares a uniform vector with the scalar *under dense output*.
- **Failure scenario:** the 4-way `match (&band, &atol)` in `solve.rs:282-305` is the only place the facade picks a `_with_atol` fast dense entry. A later edit could do any of these without any test failing:
  - cross the arms;
  - pass `adaptive.atol` where the vector is expected;
  - change `adaptive_atol` (for example to the mean or the min).

  The driver-level test cannot see the facade dispatch. The probe (P1) shows the code is correct today.
- **Fix:** add to `p0_contracts.rs`:

  ```rust
  #[test]
  fn uniform_vector_atol_equals_scalar_under_dense_output() {
      for ie in [InteriorError::WFilteredEnforce, InteriorError::Off] {
          let s = dense_opts(1e-6, 1e-10, ie);
          let v = Options { atol: Atol::Vector(vec![1e-10; 3]), ..s.clone() };
          let a = Rodas5p::solve(&robertson(), ROBERTSON_SPAN, &ROBERTSON_Y0, &s).unwrap();
          let b = Rodas5p::solve(&robertson(), ROBERTSON_SPAN, &ROBERTSON_Y0, &v).unwrap();
          assert_eq!(b.stats.route.entry_point, "integrate_rodas5p_fast_dense_observed_with_atol");
          assert_eq!(bits(&a.t), bits(&b.t));
          for (x, y) in a.y.iter().zip(&b.y) { assert_eq!(bits(x), bits(y)); }
          assert_eq!(a.stats.counters, b.stats.counters);
          assert_eq!(a.stats.driver_work, b.stats.driver_work);   // includes FastInteriorWork
          assert_eq!(a.stats.adaptive, b.stats.adaptive);
          for w in a.t.windows(2) { let m = 0.5 * (w[0] + w[1]);
              assert_eq!(bits(&a.at(m).unwrap()), bits(&b.at(m).unwrap())); }
          // Banded twin: the TRIDIAGONAL_N problem with .banded(1, 1), vector of length TRIDIAGONAL_N,
          // entry "integrate_rodas5p_fast_banded_dense_observed_with_atol", same assertions.
          // Nonuniform control: atol [1e-10, 1e-10, 1e-6] must change driver_work.interior.
      }
  }
  ```

#### I2. Seam (ii) has no facade terminal-event test across the stores and routes

- **Where:** `events_contracts.rs` and `p0_contracts.rs`.
- **Plan requirement:** Task 8 (ii) is "Task 5's terminal-event tests pass on the fast dense route".
- **What exists:**
  - `events_contracts.rs` runs on the default `WFilteredEnforce` dense route. Only `events_do_not_change_integration` (`:203`) asserts the driver id, and it is non-terminal.
  - The terminal tests (`:79`, `:97`, `:216`, `:225`) never assert which store they ran on.
  - None runs on the banded fast dense store, under `Off`, or on the sequential store (Enforce or mass).
  - The unit test `solution.rs:652` covers `truncate_after` on two stores, but only via a hand-built truncation, not `locate`.
- **What is untested:**
  - the `at()` bound after truncation;
  - `at(t_k) == y[k]` on the kept outputs;
  - the terminal-at-step-end no-duplicate path (`events.rs:278`).
- **Failure scenario:** a change to `FastDenseSegments::truncate`, `Extension::truncate_after` or the `t > t.last()` guard (for example, keeping `at()` bounded by the store's `t_new` again, as before ruling R-C3) breaks terminal events on one store only. The current terminal tests may then still pass, or not notice the route they ran on. The probe (P2, P3) shows the code is correct today.
- **Fix:** add a table-driven test to `p0_contracts.rs` or `events_contracts.rs`:

  ```rust
  #[test]
  fn terminal_event_on_every_dense_store() {
      let cases = [ // (problem, options, expected driver id)
          (decay(), opts(WFilteredEnforce), "rodas5p-fast-dense-v1"),
          (decay(), opts(Off), "rodas5p-fast-dense-v1"),
          (tridiagonal.banded(1,1), opts(WFilteredEnforce), "rodas5p-fast-banded-dense-v1"), // g = y[0] - c
          (decay(), opts(Enforce), RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID),
          (decay().mass(vec![1.0]), opts(WFilteredEnforce), RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID),
      ];
      for (p, o, id) in cases {
          let base = Rodas5p::solve(&p, span, &y0, &o).unwrap();
          let sol = Rodas5p::solve(&p.clone().terminal_event(g, Direction::Down), span, &y0, &o).unwrap();
          assert_eq!(sol.stats.route.driver_id, id);
          let te = sol.events[0].t;
          assert_eq!(*sol.t.last().unwrap(), te);
          assert!(sol.t.windows(2).all(|w| w[0] < w[1]));
          for k in 0..sol.t.len() { assert_eq!(bits(&sol.at(sol.t[k]).unwrap()), bits(&sol.y[k])); }
          let next = *base.t.iter().find(|&&t| t > te).unwrap();
          assert!(sol.at(0.5 * (te + next)).is_none() && sol.at(next).is_none());
          for w in sol.t[..sol.t.len() - 1].windows(2) { let m = 0.5 * (w[0] + w[1]);
              assert_eq!(bits(&sol.at(m).unwrap()), bits(&base.at(m).unwrap())); }
          // Terminal exactly at a step end: tk = base.t[base.t.len()/2], g = t - tk, Up:
          // one event, t.last() == tk, no duplicate time, y.last() bitwise base.y[k].
      }
  }
  ```

### Minor (Nice to Have)

#### M1. Success and stop docs contradict the terminal-event semantics

- **Where:** `solution.rs:138` (`Status` doc: "`Success` means the run reached `t1`"), `:144` (`Success`: "The run reached `t1`"), `:520` (`is_success`: "Whether the run reached `t1`"), `:337-338` (`StopRecord::t`: "the last entry of [`Solution::t`]") and `:437-440` (`Solution::t`: "every accepted step's end, up to `t1` or the last accepted time").
- **Failure scenario (probe P4):** with a terminal event before a later driver failure, the documented behaviour is status `Success` with `stop` kept. The result is:
  - `is_success() == true`, yet the run reached neither `t1` nor `stop.t`;
  - `stop.t == 1.0000000041`, while `t.last() == 0.5`;
  - the last `t` entry is `t_e`, which is not an accepted step's end.

  A non-finite event truncation also breaks `StopRecord::t == t.last()`.
- **Fix:** docs only.
  - `Success`: "reached `t1`, or a terminal event ended the outputs (see `Stats::terminal_event`; `Stats::stop` keeps any later driver stop)".
  - `StopRecord::t`: "the driver's last accepted time; after a terminal event or an event-function failure, `Solution::t` is cut before it".
  - `Solution::t`: add "or `t_e` after a terminal event".

#### M2. `segments` in `DriverWork` counts the steps before the cut

- **Where:** `solution.rs:303` (`FastDense::segments`) and `:328` (`SequentialDense::segments`): "Accepted steps whose continuous extension was kept."
- **Failure scenario (probe):** after a terminal event or a non-finite truncation, the record says `segments = 25` while the kept extension and outputs have 13. The counter is taken before `locate` truncates (`solve.rs:324`, `:451`).
- **Fix:** say "accepted steps whose extension the driver returned (before any event truncation)". Do not change the number: it is a work counter, and `segments == accepted_steps` is asserted at `p0_contracts.rs:177`.

#### M3. Three `Unsupported` messages are stale or imprecise after T3

- **Where:** `error.rs:43-45`, `:51-52` and `:58`.
- **The messages:**
  - `JvpWithMassOrDenseOutput` names "the protected sequential dense driver". Dense output now runs the fast dense driver, which needs the stage matrix too.
  - `DeclaredBandOnDenseWithMassOrDenseOutput` says "the sequential dense driver would ignore it". Dense output without mass now goes to the fast dense route.
  - `VectorAtolOnSequentialRoute` says "with a mass matrix or interior_error = Enforce". A vector with `Enforce` and without dense output is accepted, since the route is not sequential (`solve.rs:735`); only dense output under `Enforce` is refused.
- **Why it matters:** the T7 ruling pinned `Display` to the old text, so this needs a deliberate text change. `display_text_is_the_former_message` (`p0_api_contracts.rs:171`) will need its strings updated. The T7 minors (Stability wording, `SolveError::Unsupported` doc) do not cover these three messages.
- **Fix:** reword them, for example:
  - "...: the stage matrix M - h gamma J of the dense-output and mass-matrix drivers needs an explicit or finite-difference Jacobian";
  - "...: only the router verifies such a declaration, and the dense-output and mass-matrix routes bypass the router";
  - "a vector atol with a mass matrix, or with dense output under interior_error = Enforce: ...".

#### M4. The `STATS_SCHEMA` v3 note does not list every v2→v3 change in meaning

- **Where:** `solution.rs:21-27`.
- **What the note lists:** `DriverWork::FastDense` and the event fields.
- **What it omits (each changes what a v3 record can hold):**
  - `options.atol` can now be `Vector` in a record that ran (v2 refused every vector);
  - `adaptive.atol` is then the largest entry, not a value any driver read (`Stats::adaptive` documents this; the schema note does not);
  - the new `route.entry_point` strings: the four `..._with_atol` entry points and the fast dense entry points;
  - `FastDense { band_work, interior: FastInteriorWork }`, whose JSON shape is not pinned anywhere. The golden `tests/golden/robertson_stats.json` is a non-dense run, and `p0_contracts.rs:183` checks only `driver_work.driver == "fast-dense"`.
- **Counter consistency is otherwise right.** The interior estimates' RHS calls and back-solves are in `counters` (`rodas5p_fast_dense.rs:338-345`), as `FastInteriorWork` documents. `event_evaluations` is documented as outside `counters`, and that is true.
- **Fix:**
  - Extend the v3 note with the vector-atol and entry-point lines.
  - Optionally add a second golden of a dense-output Robertson run with one event, so that the `FastDense`/`interior` JSON shape is pinned. Write it with `serde_json::to_string_pretty(&sol.stats)` and compare it byte for byte.

#### M5. Allocation claims: the test has a margin of 1, and two claims are untested

- **Where:** `fast_dense_allocations.rs:107`; the module doc at `rodas5p_fast_dense.rs:30-37`; and the facade claim that `with_jacobian_into` removes the per-build matrix allocation (`solve.rs:758-760`).
- **The measured margin is 1 allocation**: rtol 1e-9 WFilteredEnforce gives dense 69 against a budget of 70; at rtol 1e-6, 62 against 64.
  - The test's own header concedes that the budget also absorbs a setup constant of 4-5, so the plan's "covers segment-store growth only" holds only loosely.
  - A harmless change, such as one more setup `Vec` or a different initial store capacity, would fail the test without any per-step allocation having been added.
- **The banded-pipeline claim is untested.** The module doc says "nothing is allocated per step" on the banded pipeline, but no test covers it.
- **The vector-atol fast dense run is untested for allocations.** This is distinct from T4 Minor 3, which concerns `rodas5p_fast_allocations.rs`.
- **The facade `with_jacobian_into` saving is untested.** So is the B3d zero fill (`solve.rs:778`).
- **Fix (test sketches):**
  1. Replace the absolute budget by a growth check. Run the same problem at two spans (or two rtols) whose accepted counts differ by about 4x, and assert that `(dense - fast)` grows by at most `3 * (ceil_log2(m2 + 1) - ceil_log2(m1 + 1)) + 1`. This isolates per-step allocation from the setup constant.
  2. Add a Brusselator n = 20 banded case through `integrate_rodas5p_fast_banded_dense_observed` against `integrate_rodas5p_fast_banded_observed`, with the same budget form.
  3. Add a uniform-vector case of `integrate_rodas5p_fast_dense_observed_with_atol`.
  4. Add a facade B3d test: a dense Jacobian callback that writes entry (0,1) only on its first call, and otherwise writes only the remaining entries. With the zero fill, the run is bitwise the run whose callback writes an explicit `0.0` at (0,1) after the first call. Without the fill, the stale entry survives.

#### M6. The CI integrators step runs about 200 test targets with an unmeasured wall time

- **Where:** `production.yml:101-102`, with `timeout-minutes: 120` at `:86`.
- **What it is:** a deviation from the plan's `--lib` plus `--test` list. It was raised in the T6 report and review, but `progress.md` records no ruling on it.
- **Why it matters:** the step is what makes seam (iii) automatic. It also puts every research and audit test of `rodas5p-integrators` into the production gate, in the debug profile, on a 2-core runner.
- **Fix:** record a ruling in the ledger. In the T8 local handoff (`LOCAL_TEST_HANDOFF_W3.md`), time `cargo test --locked -p rodas5p-integrators` in the debug profile with the pinned 1.94.1, and report it in VALIDATION_MATRIX against the 120-min timeout.

## Ledger integrity

- `check-research-node.py --base a5a48f7…` passes with exit 0 and 141 rows.
- L-0141 (b1c, FAIL) is the only new row. Its `inputs_sha256` and `outputs_sha256` are present.
- e1b-dense and L-0142 are not on this branch, as stated.

## Declined to judge

- **T1 e1b-dense** (a separate branch and bench workspace, pending a workstation run): out of scope by instruction.
- **The sequential route accepting an unavailable interior sample under Enforce or WFilteredEnforce:** parked for wave 4. No concrete new bug follows from it on this branch.
- **T4 Minors 2 and 3, T5 Minors 4 and 5** (including the `at()` miss reported as `EventFunctionNonFinite`), **the T7 doc-wording Minors and T6 clippy `--no-deps`:** parked by instruction. M3 and M5 above are distinct from them: M3 is about the three `Unsupported` message texts, not the Stability or `SolveError` docs; M5 is about the fast-dense allocation test, not `rodas5p_fast_allocations.rs`.
- **The numerical quality of the W-filtered estimate and the PR step counts:** T3's own measured results. They were not re-derived here.
- **The full `rodas5p-integrators` package suite and the bench crate tests:** these are workstation-handoff items. Only targeted tests ran here; the bench was compiled with `check --all-targets`, not run (it needs SUNDIALS at run time).
- **The router's choices for undeclared dense and FD Jacobians at large n**, which the facade reaches only without dense output: unchanged by wave 3.
- **The semver job outcome:** informational until a 0.x baseline is published.

## Assessment

**Ready to merge?** With fixes. The fixes are test additions (I1, I2) and doc corrections (M1–M4). No code change is needed.

**Reasoning:** every seam the per-task reviews could not see was checked:

- the vector-atol × fast-dense scale;
- the terminal event × `FastDenseSegments` truncation and the `at()` bound;
- the CI pickup;
- the refusal matrix;
- `#[non_exhaustive]` against the examples and the bench.

All of them behave correctly, both in the code read and in an independent probe. What is missing is the two facade seam tests that plan Task 8 requires, so that a later edit cannot silently break (i) or (ii). There are also doc statements that contradict the documented terminal-event semantics.

VERDICT: FIXES NEEDED
