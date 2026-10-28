# Final fix wave re-review — VigilODE wave 1 (15eb5f6..a5a48f7)

Seat: scoped re-review (read-only; diff file read once; no tests rerun; no subagents).

## Finding Verdicts

- **1. Lagged end-of-step norm bypasses the zero-scale rule** — ADDRESSED. `rodas5p_lagged.rs:~430`
  now `scaled_error_component(*e, atol + rtol*max(|y|,|y_new|))`; test
  `a1_zero_tolerance_scale.rs:290` (forced refresh, atol = 0, bit-identical to fast, 0 rejections).
- **2. Lagged Richardson zero weight (ruling: typed InvalidInput)** — ADDRESSED. Guard at the top of
  `Lagged::iterate` (`rodas5p_lagged.rs:~264`) returns `CoreError::InvalidInput` before any `x/0`;
  documented in module docs next to the staged-MF exception; test `a1_zero_tolerance_scale.rs:341`
  rejects `Ok` and any non-InvalidInput error.
- **3. W-filtered silently degrades to Off on JVP-only** — ADDRESSED. Up-front refusal in the
  Sequential arm (`dense_output_v2.rs:~1564`, `!problem.has_explicit_jacobian()`); doc updated;
  test `wfiltered_modes_refuse_a_problem_without_an_explicit_jacobian`.
- **4. Homotopy counter drift (ruling: reconcile required)** — ADDRESSED. `reconcile_outer_rejection`
  added at `integrate.rs:708` and `dense_output_v2.rs:2421`, same signature/order as the
  transactional sites. New ledger tests (`a1_homotopy_counter_reconciliation.rs`) pass before and
  after (outer-only rejection not forceable via public API); they pin the ledger, not the fix.
- **5. `wrms` scope (ruling: strict `wrms`, separate step-error helper)** — ADDRESSED. `wrms`
  (`norms.rs:~91`) rejects `s <= 0` with "WRMS scale must be finite and positive"; new
  `step_error_wrms` carries the rule; bit identity on positive scales pinned in
  `core_contracts.rs`. Only `sequential.rs:741,844` (protected driver / coupled fallback) and the
  WFilteredEnforce acceptance estimate switched; audit list recorded in the fix report.
  Specific check — Task 1 item 2 drivers still apply the rule in their end-of-step norms, all
  untouched by the fix diff: `rodas5p_fast.rs:929` (dense + banded), `rodas5p_fast_small.rs:263,372`,
  `rodas5p_fast_small_batch.rs:424`, `rodas5p_matrix_free_fast.rs:3008`, `vigilode-coupled
  solver.rs:391,394`; protected driver via `step_error_wrms`. The Task 1 regression tests
  (`a1_zero_tolerance_scale.rs:110-233`) still exercise live rule sites, so they remain meaningful.
- **6. Untested seams (JVP branch of apply_w; WFilteredEnforce counter sum)** — ADDRESSED.
  `lagged_jvp_branch_runs_inside_iterating_stages` asserts `richardson_iterations > 0` and
  `jvp_applications == operator_applications`; `wfiltered_enforce_interior_rejections_keep_the_counter_ledger`
  asserts interior rejections > 0 and counters == ledger.
- **7. FixedWidth2 driver ids as literals** — ADDRESSED. `pub const` at `rodas5p_fast.rs:149,154`,
  used at `:1652-1655`, re-exported in `lib.rs`; values unchanged; test added.
- **11. B1 constants not bound to RESULTS.json** — ADDRESSED. a_7 = 1.02 (ceil to 2 decimals, >=
  verified 1.01099); test reads RESULTS.json and checks every entry is the rounded-up verified value.
- **13. W-filtered capture clones the LU per trial step** — NOT ADDRESSED. `sequential.rs:452-454`
  (`slot.clone_from(&factor)`) unchanged. The fix report treats the coupled validation message
  (final-review item 14) as the assigned item and fixed that (`solver.rs` messages, test); final
  review triaged 13 as defer. Likely a numbering slip in the dispatch (13 vs 14); controller to confirm.
- **Facade test with atol = 0** — ADDRESSED. `facade_contracts.rs`
  `zero_atol_with_an_identically_zero_component_succeeds_on_dense_and_jvp_routes` (Atol::Scalar(0.0),
  Success, driver id, 0 rejections on both routes).

## New Breakage in the Fix Diff

- None Critical/Important.
- Minor: a_7 1.01 -> 1.02 changes rho_7 and therefore lagged-driver arithmetic; the B1 kill-gate
  numbers in the Task 3 node report were measured with 1.01. Opt-in driver, not routed, gate already
  FAIL; note it in the ledger as measured-with-old-constant.
- Minor: protected sequential driver with an outer-tolerance inner-forcing config (Krylov path,
  `sequential.rs:491-492`) uses strict `wrms`, so atol = 0 + zero component is a typed error there
  while the Direct path succeeds. This is what the ruling chose; it is in the audit list.

## Out-of-Scope Observations

None.

## Verdict

**Fix round:** Findings remain open — item 13 only (unchanged; probably meant item 14, which is fixed).
All other items addressed, no new Critical/Important breakage.
