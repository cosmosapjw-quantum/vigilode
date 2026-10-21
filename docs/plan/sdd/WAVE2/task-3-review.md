# Task 3 review (p0-facade) — a5a48f7..a8b15bd

Reviewed the 160 KB package in four passes (integrator drivers; routing + integrator tests; facade src; facade tests).
Code outside the diff read only for named risks 2 and 4 (listed below).

### Spec Compliance

- ✅ Spec compliant.
  - Mass and dense output go to `integrate_adaptive_dense_observed_with_segments` (solve.rs, diff 2348-2407). `InteriorError` {WFilteredEnforce (default), Enforce, Off} is in options.rs (diff 1779-1797, 1837-1838, `#[serde(default)]`). JVP-only with mass or dense output is refused with `Unsupported` (diff 2564-2571).
  - `Solution::at` (diff 2156-2170): it evaluates the extension under dense output, otherwise returns a stored sample, otherwise `None`.
  - Partial records on the fast dense, banded and matrix-free drivers and on the router (diff 613-664, 873-874, 1019-1020, 1118-1166). The facade returns them (diff 2438-2468).
  - Typed reasons replace the `StepSizeTooSmall` catch-all (diff 2508-2534; LinearSolveFailed / NonFinite / NonlinearSolveFailed / MaxStepsReached).
  - Bit-identity tests against direct driver calls exist for every new route: dense output, Off/Enforce, mass, and the fast/banded/MF partial paths (p0_contracts.rs:145, 212, 348, 659, 696, 786, 831).
- ⚠️ Cannot verify from diff:
  - "Accepted samples" on failure without dense output is read as `[t0, t_last]` only (report deviation 3). The controller should confirm that this reading of the brief is acceptable.
  - Mass without dense output runs interior `Off` (deviation 4). This is consistent with the brief, since only endpoints are returned, but the controller should confirm it against plan §3 "interior error controlled".

### Named-risk checks

1. Hot-path arithmetic of the fast and MF drivers. Checked `integrate_fast` (diff 822-874) and the MF loop (diff 970-1020). Per attempt, the only addition is one `Option` assignment, which nothing in the step arithmetic reads. `after_loop` runs after the loop, before `collector.finish*`, and allocates only when `t < tf`. Nothing is reordered and no state is written. PASS.
2. `EveryStep` under `WFilteredEnforce`. Read dense_output_v2.rs:1751-1925 @HEAD. The interior block only edits `accepted`/`error`. After it there is a single `reconcile_outer_rejection`, a single `diagnostics.record_with_failure` and a single `rodas_next_step_after_attempt`, so a step cannot be rejected twice. `last_failure = failure` resets on acceptance. `OutputTimes` differs only through the `interior == EveryStep ||` short-circuit. Pushing a segment moves fields of `report` after its last use. The named driver wraps the impl with `None`. Bit identity holds by construction and is tested (p0_partial_contracts.rs:346, 404). PASS.
3. `Solution::at`. `partition_point(|s| s.t_new < t)` picks segment k for `t == t_new[k]`, and `evaluate` returns `y_new` there. For t < t0, t > the last time, and NaN, `evaluate` refuses or `get` gives None, so the result is `None`. `t == t0` is special-cased, which covers zero accepted steps. PASS (see Minor 2).
4. Fatal JVP inside GMRES. Checked `problem.rs:829-847` and `rodas5p_matrix_free_fast.rs:2627-2648`. `linearize_matrix_free` only wraps the closure, and the JVP is applied only through `ShiftedOperator` in the Krylov stage solves. So the first JVP call of the step that starts past 0.5 is inside GMRES. The test asserts the partial state, counters and `last_failure`, all bit for bit with the latched twin (p0_contracts.rs:786-828). PASS.

### Strengths

- The partial record does no arithmetic on the run, and every claim is backed by a `to_bits` comparison with a latched twin problem, not a mock.
- The `OutputTimes` path is shared code, not a copy, so the named driver cannot drift.
- `failure_reason` has a unit test over every kind (solve.rs tests, diff 2686-2724).

### Issues

#### Critical
None.

#### Important
None.

#### Minor
1. dense_output_v2.rs:1898 (`Err(_) => dense_report.unavailable += 1`, pre-existing). With `EveryStep` and the facade's default `WFilteredEnforce`, a step whose estimate cannot be formed (for example a recoverable RHS error at the midpoint) is accepted uncontrolled, and the status stays `Success`. It is counted in `DriverWork::SequentialDense::unavailable`, so it is not silent. Follow-up: under the enforcing modes, reject the step or surface the count in the status.
2. p0_contracts.rs:196-209. The `at(t)` boundary tests cover only t0 and t1. Add `at(s.t_new)` for an interior segment, compared with `sol.y[k]` bit for bit.
3. p0_contracts.rs:786-828. The test never asserts that the fatal JVP happened inside GMRES (for example through stage telemetry `arnoldi_jvps > 0` on the twin). It holds by construction (risk 4).
4. p0_contracts.rs:812 and others. The `eprintln!` calls in the tests add noise to `--nocapture` output.

### Assessment

**Task quality:** Approved

**Reasoning:** All brief items are implemented, with exact bit-identity evidence against direct driver calls. The success-path arithmetic and the counters are unchanged by construction. The remaining items are coverage gaps and one pre-existing uncontrolled-estimate path that is counted but should be closed in a follow-up.
