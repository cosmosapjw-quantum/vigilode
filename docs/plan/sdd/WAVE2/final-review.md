# Final whole-branch review: wave 2 integration

Branch `production/wave2-integration`, base `a5a48f7`, head `00973c7`. Requirements: `docs/plan/IMPLEMENTATION_WAVE2.md`, Tasks 1–6.
The ledger rulings in `progress.md` are treated as decided. I reviewed the work in passes: facade seams, then driver partial records and dense output, then the GMRES refactor and routing, then the ledger and provenance, then the bench crate's build against the merged facade. No subagents were used.

## Verification performed

| Command / check | Result |
| --- | --- |
| `cargo test -p vigilode -j 2 --locked` (stable, target `finalreview2`) | lib 4, facade_contracts 18, p0_contracts 13, doctest 1: all pass |
| `cargo test -p rodas5p-integrators --test p0_partial_contracts --test c1_transaction_contracts --test a1_zero_tolerance_scale` | 9 / 20 / 9 pass |
| `cargo check --tests --locked` in `crates/vigilode-bench` (own workspace) against the merged facade | compiles; lock consistent |
| `vigilode-bench report --in research/bench_wave2_20261021/results_v1.json` (dev build at HEAD) | sha256 `6b5380df…` = committed `REPORT.md` and the L-0139 output digest |
| sha256 of every L-0138 / L-0139 `inputs_sha256` and `outputs_sha256` entry at HEAD | all outputs match; one input (`PREREGISTRATION.md`) differs at HEAD (Minor 1) |
| `tools/check-research-node.py --base a5a48f7` | exit 1, only the two L-0124 lines (known; h-checker node pending) |
| Ledger ids | 139 rows, unique, L-0138 and L-0139 appended as a pure suffix |
| Lagged-driver call sites (`grep integrate_rodas5p_lagged_observed`, `StageSolver::Gmres`) | only `rodas5p_lagged.rs`, `lib.rs` and three test files. Neither the router nor the facade reaches it, so `Tier::Performance` cannot route it. |
| Scratch probe crate (path dependency on the facade, outside the repo) | see Important 1 and Minor 7 |

## Strengths

- **Facade seam.** One `Run` record carries `partial` and `segments` from every route, and `failure_reason` is a pure function with its own unit test. `CallbackFailed` takes precedence through `guard.fatal()`, as documented.
- **Partial records.** `DriverPartial::after_loop` is shared by the fast, banded, matrix-free and sequential dense drivers. It reads no step arithmetic. The budget-first loop condition makes the `AttemptBudget`/`MinimumStep` split correct.
- **Dense-output refactor.** `integrate_adaptive_dense_observed_with_dense_error_control` now wraps the shared implementation with `InteriorSampling::OutputTimes` and no segment sink, so the wave-1 entry point is unchanged. `Rodas5pDenseSegment::evaluate` rejects NaN and out-of-step `t`. The probe confirmed `at(-1)`, `at(NaN)` → `None`, `at(t1)` = last state, and a zero span returns `[t0]`.
- **GMRES refactor.** `run_cycle`/`arnoldi_column`/`least_squares_correction` keep the old column order and the happy-breakdown handling (zero fill, no division) for both sides. The protected matrix-free route therefore keeps its arithmetic.
- **Tier routing.** `tier_executed` is hard-wired to `Protected`. The probe shows `Tier::Performance` with dense output runs the sequential dense entry point.
- **Bench crate.** It builds against the merged facade and regenerates the committed `REPORT.md` bit for bit.

## Issues

### Critical (Must Fix)

None.

### Important (Should Fix)

1. **`InteriorError::Enforce` is silently disabled for the whole run when any component has a zero error scale (`atol = 0` and a component that is zero at both ends of a step), and the run reports `Status::Success`.**
   - **Where:**
     - `crates/rodas5p-integrators/src/dense_output_v2.rs:1158-1159`: `rodas5p_dense_interior_error_estimate` ends in the strict `wrms`.
     - `dense_output_v2.rs` (`match estimate { … Err(_) => dense_report.unavailable += 1 }`, in the main loop after the estimate dispatch): the error is swallowed.
     - The facade exposes the path at `crates/vigilode/src/solve.rs` (`InteriorError::Enforce => DenseErrorControl::Enforce` with `InteriorSampling::EveryStep`).
   - **Failure scenario (measured with the scratch probe):** `y0' = -y0`, `y1' = 0`, `y0 = [1, 0]`, `rtol 1e-6`, `atol = Atol::Scalar(0.0)`, `dense_output = true`, `interior_error = Enforce`. The result is `Status::Success` with 7 accepted steps, `samples: 0`, `unavailable: 7`, `max_accepted_estimate: null`. Every step's estimate returned `InvalidInput("WRMS scale must be finite and positive")`. Each step still paid one Jacobian and one LU (`estimate_counters.jacobian_builds = 7`, `direct_factorizations = 7`), and every step was accepted with no interior control. The same problem under `WFilteredEnforce` takes 7 samples with 0 unavailable, because its norm at `:1190-1191` already carries the A1 zero-scale rule. With `atol = 1e-9` and `Enforce`, it also takes 7 samples.
   - **Why it matters:**
     - The user chose an enforcing interior control and got `Off` with a success status.
     - `atol = 0` is a supported input: the wave-1 A1 ruling and `a1_zero_tolerance_scale.rs` cover every driver.
     - The wave-1 ruling puts acceptance norms under `step_error_wrms`, and this estimate decides acceptance under `Enforce`.
     - This is a deterministic, whole-run trigger at a seam that is new in wave 2: the facade exposes `Enforce` with `EveryStep`. It is distinct from the deferred "an occasional unavailable estimate is accepted" minor.
   - **Fix:**
     - Use `step_error_wrms(&estimate, &scale)` in `rodas5p_dense_interior_error_estimate`. It is bit-identical to `wrms` on a positive scale, so existing `Enforce` results do not change.
     - Decide whether `rodas5p_dense_interior_defect_estimate` (`:1238-1239`, `ReportDefect`, not exposed by the facade) follows. It is a report-only norm, so it may stay strict.
     - Add a facade contract test: the probe problem under each `InteriorError` mode with `atol = 0`, asserting `unavailable == 0` and `samples == accepted_steps`.

### Minor (Nice to Have)

1. **L-0139 binds files from two different commits under one `commit` field.**
   - **Where:** `research/LEDGER.jsonl:139`.
   - **What:**
     - The row says `commit: 1e3ca6d`.
     - Its `crates/vigilode-bench/src/report.rs` input digest (`16e17c41…`) and its `REPORT.md` output digest are those of `94c5b0a`/`387f87a`. At `1e3ca6d`, `report.rs` hashes to `535b8a02…`.
     - Its `PREREGISTRATION.md` input digest (`985f0157…`) is the pre-append version from `1e3ca6d`. At HEAD the file is `dc495599…`, because the results section was appended as the node rule prescribes.
   - **Failure scenario:** a reproducer checks out the row's commit and runs the recorded `report` command. The result does not hash to the recorded `REPORT.md` digest. The checker does not notice, because it verifies only outputs at HEAD.
   - **Status:** the in-place update is a ruling. This item records only that the `commit` field no longer names one consistent tree.
   - **Fix:**
     - When the h-checker supersession lands, append a correction row that binds `report.rs` and `REPORT.md` to `94c5b0a`, with command `vigilode-bench report`.
     - Alternatively, state in L-0139's `profile` or `command` that the report step ran at `94c5b0a`.
   - **Checked:** `REPORT.md` regenerates bit for bit at HEAD from `results_v1.json`.
2. **Commit identity.** `387f87a` (the controller's digest refresh) is authored as `Claude <noreply@anthropic.com>`, against the global commit-identity rule. It is the third such commit on the branch; the two `p0-facade` commits are already accepted by the user. Fix: use the configured identity for the remaining wave-2 commits. Do not rewrite pushed history.
3. **Stale `Problem::banded` doc.**
   - **Where:** `crates/vigilode/src/problem.rs:150-161`.
   - **What:**
     - The doc still says a finite-difference band runs "the banded driver" and that a dense-Jacobian band is verified by the router.
     - Under `.mass(M)` or `dense_output = true`, a dense Jacobian with a band is refused (`solve.rs` validate). A finite-difference band, or `Jacobian::Banded`, is assembled into a dense matrix for the sequential driver's direct dense LU, which is O(n³) per factorization.
   - **Why it matters:** a banded user who turns on dense output gets a silent cost change and finds no mention of it in the API docs.
   - **Fix:** add both cases to the doc. Optionally add a line to the route table in `lib.rs`.
4. **The mass-matrix route without dense output records and then discards every segment.**
   - **Where:** `crates/vigilode/src/solve.rs`, `Plan::SequentialDense`, `segments: opts.dense_output.then_some(run.segments)`.
   - **What:** each accepted step keeps `y_old`, `y_new` and 8 stage rows, which is 10·n f64 per step. All of it is dropped after the run. A long, large DAE run holds memory that it never returns (node report deviation 8 acknowledges this).
   - **Fix:** expose the segment-sink `Option` (the private `integrate_adaptive_dense_impl` already takes `None`), or add a `_with_partial` variant without segments, and use it when `!opts.dense_output`. The run stays bit-identical, because the segment push does no arithmetic.
5. **`FailureReason::StepSizeTooSmall` wording.**
   - **Where:** `crates/vigilode/src/solution.rs:43-46` (doc) and `:222-225` (`Display`, "after local error rejections"); `crates/vigilode/src/solve.rs` `failure_reason`, `Some(LocalError) | None`.
   - **What:** the reason is also returned when no attempt was rejected after the last accepted step. The `y' = y²` contract test documents exactly this case.
   - **Fix:** word it as "after local error rejections, or the controller's step falling below the minimum".
6. **Stale fidelity note in the bench crate.** `crates/vigilode-bench/src/arms.rs:151` emits "the facade at a5a48f7 refuses dense_output" into every new results JSON. On the integrated branch that is no longer true. Fix: update it in the wave-3 re-run through the facade (already required by the E1 ruling). Leave the committed `results_v1.json` as it is.
7. **No facade contract test covers the sequential route with `Jacobian::Banded` or a finite-difference band.**
   - **What:** the provider (`band_as_dense`, grouped FD columns) feeds the sequential driver on the mass and dense-output route, and no facade test exercises it.
   - **Probe result:** on a 6-component tridiagonal problem with `dense_output = true`, `Jacobian::Banded` was bit-identical to `Jacobian::Dense` (43 states). The FD band matched to 4e-18, and mass with an FD band succeeded. The behaviour is correct but unguarded.
   - **Fix:** add one bit-identity test (`Banded` vs `Dense`) and one FD-band smoke test to `p0_contracts.rs`.

## Recommendations

- Close Important 1 in the fix wave together with its test. It is a one-line change in the integrators plus one facade test.
- Fold Minor 1 into the h-checker merge so that the ledger correction and the checker change land together.

## Declined to judge

- B1 and B1b kill-gate FAIL: a recorded research outcome (ruling).
- `check-research-node.py` exit 1 on L-0124: known; the h-checker node `4f92ec3` is pending user-approved merge.
- Stable toolchain instead of the 1.94.1 pin: environment constraint.
- An interior estimate that is unavailable on an occasional step is accepted: the general case is a deferred minor in the Task 3 review. Only the deterministic whole-run trigger is raised (Important 1).
- A partial solution without dense output keeps only `[t0, t_last]`, and mass without dense output runs interior `Off`: rulings.
- E1 ratios read as "grid-landing VigilODE vs interpolating comparators", with no ranking: ruling.
- The L-0139 in-place hash update itself: ruling. Only the resulting `commit`-field inconsistency is noted (Minor 1).
- Lagged GMRES driver internals (refresh rule, residual units, staleness bound) and the C1 transaction deferred minors (bins never forget, caller-supplied causality, overshoot untested, `declines ≤ m` off-by-one): reviewed per task. Re-checked here only for routing, panics and exports.
- Dead-code warnings in `rodas5p-integrators` (`packet_trajectory.rs`, `output.rs`) seen while building the bench crate: they predate this branch.
- Dense-output memory of about 11·n f64 per accepted step (segments plus returned states): inherent to returning a continuous extension; it is a documentation concern, not a defect.
- The full workspace suite and clippy: running elsewhere, so I did not repeat them.

## Assessment

**Ready to merge?** With fixes.

**Reasoning:** The nodes integrate cleanly at the seams. Partial records, failure reasons, the dense-output extension and tier routing behave as documented, the bench rebuilds against the merged facade, and its report reproduces. One user-visible defect remains on the newly exposed `Enforce` path, where a zero error scale turns the requested interior control off while the run reports success.

VERDICT: FIXES NEEDED
