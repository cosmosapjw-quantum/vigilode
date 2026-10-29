# Task 2 review: A2-wfiltered (W-filtered interior error control)

Base 9b5d083, head 7bea78c. I read the whole diff file (1253 lines) in one pass, plus three focused out-of-diff checks listed below. I ran no tests: no doubt came up that the implementer's runs do not already answer.

### Spec Compliance

- ✅ Spec compliant. The change matches brief items (a)–(c), the two new variants, the estimate formula, factorization reuse, "no new Jacobian", unchanged existing modes, and the kill-gate handling. The kill gate FAILs on step count (15/7 = 2.143x > 1.3x at rtol 1e-6). As the brief requires, it is recorded as an `#[ignore]`d measurement test with its numbers, the threshold is not loosened, and the code is kept (tests/dense_wfiltered_contracts.rs, `prothero_robinson_wfiltered_enforce_kill_gate`).
- ⚠️ Cannot verify from the diff: the bit-identity of `Off`/`Report`/`Enforce` rests on `dense_error_control_contracts` (existing, unchanged) and on the DP-01 counts matching DEFAULT_PATH_KO.md. That is the implementer's run evidence; I did not re-run it. Code reading supports it. Off/ReportDefect/Report/Enforce still call the unchanged `sequential_step` (dense_output_v2.rs, driver `match method`). The only change on the Enforce branch is `let rejected` becoming `let mut rejected`. The new `factor_capture` parameter is `None` at every pre-existing call site (sequential.rs hunks at `sequential_stages`, `_with_inner_forcing`, the inner-forcing refinement, and the matrix-free step).
- ⚠️ Gate wording: plan §6 says "protected step count", and the brief says `Off`. The test scores against Off as briefed. The controller should decide which reading binds (against Enforce: 1.25x accepted, 0.58x attempts).

### Named-risk checks

1. **Factorization is of M − hγJ₀ with the step's own J₀ and h.** Checked sequential.rs `build_step_context` (around lines 236–244). There `ShiftedOperator::new(mass, J = linearize(t, y), h, coeffs.gamma)` is built, and rodas5p-core/src/operator.rs:538–547 forms `explicit = M.sub(J.scale(h*gamma))`. The captured LU is `factor` from `sequential_stages_refined`, the same object the stage solves use, cloned after it is built. The fallback `matrix` is `context.shifted.explicit()` of the same step. The driver creates a fresh `StepIterationMatrix::default()` for each trial, and `trial_h` is the h passed to the step and recorded in `report.h`. So the matrix is never h/2 and never left over from a rejected trial. PASS. `rodas5p_wfiltered_iteration_matrix` uses exactly the same `mass.sub(&jacobian.scale(h*gamma))` expression.
2. **WFilteredEnforce max(embedded, estimate) and counter consistency.** The sampling block runs only when `accepted` is already true, meaning embedded error ≤ 1 and finite y (dense_output_v2.rs, `let mut accepted = report.accepted && error <= 1.0 && ...`). The block then sets `error = max(error, interior)`, mapping a non-finite estimate to INF, and sets `accepted = !(error > 1 || NaN)`. It can only turn an accept into a reject, never a reject into an accept. There is one rejection path: `reconcile_outer_rejection` (adaptive.rs:115–124) moves exactly one accepted step to rejected, and `rodas_next_step_after_attempt` receives the combined `error`. Nothing is rejected twice. The Enforce branch is skipped because `control == Enforce` is false. accepted + rejected == attempts holds by construction. PASS on code reading, but no test exercises it (see I-1).
3. **The kill-gate ratio is computed against Off.** `pr_row` runs `[Off, Enforce, WFilteredEnforce]`, and the gate test computes `accepted[2] / accepted[0]` from `observed.internal_steps`, with the threshold left at 1.3. PASS.

### Strengths

- The capture is clean. The public `sequential_step_with_residual_accounting` forwards to `sequential_step_impl(..., None, ...)`, so the existing arithmetic path is literally unchanged and `StepResult` is untouched.
- The work accounting is honest. LU reuse with Direct is asserted (0 extra LU, one back-solve per sample, `jacobian_builds == 0`). The GMRES fallback is asserted to factor once per sample. `WFilteredReport` is asserted bit-identical to the legacy path in counters, diagnostics and y bits.
- SABR is refused with a typed error. The alternative would have been a silent new Jacobian.
- The kill-gate FAIL is preregistered, numeric, and not loosened.

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
None.

#### Minor (Nice to Have)

- **I-1. No test covers the core enforcement branch.** The interior-rejection branch of `WFilteredEnforce` (estimate > 1 with embedded ≤ 1) is never exercised. PR shows 15/15 attempts and DP-01 shows 21/0 rejected (dense_wfiltered_contracts.rs, `wfiltered_enforce_charges_its_work_to_the_run` and `dp01_...`). Add a case that forces an interior rejection, for example a looser max_step or a larger initial_step on PR. Assert `rejected_macro_steps > 0` and `counters.accepted_steps + counters.rejected_steps == diagnostics.attempts`.
- **I-2. The driver tests cannot tell the hγ shift from the h/2 shift.** `wfiltered_estimate_is_the_report_estimate_up_to_lemma3_on_linear_pr` accepts ratios in [1 − 1e-9, 2.3592], so a driver that wrongly used the h/2 shift (ratio exactly 1) would pass. On PR, z = hλ is about −1e4, so assert ratio > 2, or compare against (1 − z/2)/(1 − γz) per sample. The Lemma-3 test (a) checks the formula through `rodas5p_wfiltered_iteration_matrix`, not the matrix the step captures.
- **I-3. The W-filtered path allocates on every trial step.** It clones the LU, or the n×n W, on every trial step (sequential.rs `slot.clone_from(&factor)` and `capture.matrix = ...cloned()`). That includes trials that will never be sampled. The cost is opt-in and documented, but it works against the plan's P0 contract of "zero heap allocation per step after setup" (PRODUCTION_PLAN §2 P0 row) if this mode becomes the default (plan line 447). Consider capturing lazily, or only when the step interval contains an output time.
- **I-4. Two public helpers go beyond the brief.** `rodas5p_wfiltered_iteration_matrix` and `rodas5p_dense_wfiltered_interior_error_estimate` are new public API (lib.rs re-export) the brief does not ask for. They are small and used by the tests, so this is acceptable, but they widen the public surface.

### Assessment

**Task quality:** Approved

**Reasoning:** The estimate reuses the step's own M − hγJ₀ factorization with the correct h and no stale state. WFilteredEnforce feeds max(embedded, estimate) through the single existing rejection path, keeping counters consistent, and the kill gate is scored against Off as preregistered and correctly reported as FAIL. What remains is test-coverage tightening (the interior-rejection branch, and a test that tells the two shifts apart) and an allocation note.
