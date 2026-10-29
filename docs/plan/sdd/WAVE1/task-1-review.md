# Task 1 review: A1-defects (9b5d083..cf697d6)

Reviewed from the review package `review-9b5d083..cf697d6.diff` (one pass, all hunks complete). Outside-diff reads were limited to the four named risks plus one consequence of risk 3 (listed under "Checks run").

### Spec Compliance

- ✅ Spec compliant for items 1–5 as briefed:
  - Item 1: `rodas5p_fast.rs` at 2552a7d is byte-identical to `b51c051`. `exact_band_cache.rs` and `banded_exact_tests.rs` are identical to the source. `tests/banded_recovery_contract.rs` matches `rustfmt --edition 2024` of the source exactly. 7d1778f changes only clippy items (an `#[allow]` and a let-chain). Default selection is unchanged: `BandedKernel::Indexed` stays `#[default]`. `factor`/`solve` reach the new kernels only for `FixedWidth2*` with l=u=2 (rodas5p_fast.rs diff L736–746, L767–777). `factor_with_policy` with `exact_cache: None` counts one factorization and then factors, the same as the old `Workspace::factor` (L958–964 vs L883–907).
  - Item 2: there is one helper, `scaled_error_component` (norms.rs L85–91). The brief names five driver files; all five now call it (rodas5p_fast.rs L987, fast_small.rs L1150/L1174, small_batch L1224, matrix_free_fast L1280), and `wrms`/`error_scale` accept a zero scale. There is one regression test per driver family (a1_zero_tolerance_scale.rs) plus a core contract test.
  - Item 3: `get(..16)` (budget.rs L247–250); tests cover the short, non-ASCII and valid cases.
  - Item 4: reconciliation is called in both named drivers (integrate.rs L629, dense_output_v2.rs L498). The test checks the sum, the split and the retained steps.
  - Item 5: the eager stage-0 skip (solver.rs L1868–1871) and the reservation `(S−1)·m` (L1846). The old and new counts are documented.
- ⚠️ Cannot verify from diff, or outside the brief (the report names all three):
  - The homotopy drivers (integrate.rs ~704, dense_output_v2.rs ~2271) have the same unreconciled `accepted && error <= 1` pattern.
  - `rodas5p_matrix_free_fast.rs:2674` (`staged_weight = 1/scale`) still rejects a zero scale, and so does `vigilode-coupled/src/solver.rs:326`. The "one rule" therefore does not hold on the opt-in staged MF path or in the coupled crate.
  - The controller should decide whether these go into a follow-up node.

### Checks run (named risks)

1. **Zero-scale contract (risk 1).**
   - For s > 0, `scaled_error_component(e,s)` returns `e / s`, so positive-scale results are bit-identical; `error_scale`'s positive output is unchanged.
   - In the fast drivers the old behaviour differs only at s = 0 and e = 0 (it was NaN); for e ≠ 0 the norm was ±inf → INF before and is INF now.
   - I grepped all 101 non-test `error_scale(`/`wrms(` call sites for calls not propagated with `?`. Only two branch on the `Result`, and both change only at a zero scale, where they used to error:
     - `g1_transactional_gate.rs:335` (`.ok()`, a diagnostic row);
     - `dense_output_v2.rs:1802–1809` (`.unwrap_or(INF)`, a dense-audit delta).
   - `rodas5p-krylov/src/common.rs:52` passes scales through `validate_residual_scale` (common.rs:6–20, still positive-only), so it fails closed.
   - `phi_error_proxy` (adaptive_exponential.rs:82) uses min-scale weighting, which the report says is floored.
   - No caller uses the old error for control flow. **Pass.**
2. **PR #73 port (risk 2).** `b56bb19` and `b51c051` are present locally. Per-file comparison: identical, as above. Substance matches exactly. **Pass.**
3. **Eager stage-0 failure (risk 3).**
   - For i = 0 the loop `for k in 0..i` is empty, so `xz = 0`, `x = 0` and `state == f.y`, the base state whose linearization and m JVPs (`basis.jq`) were already evaluated, and charged, when the basis was built.
   - `values[0].1` is never read (`for j in 0..i`, solver.rs ~205).
   - What disappears is a recomputation of an already-successful callback at the same point. It can fail only for a nondeterministic or stateful callback, and its result was discarded anyway.
   - No reported error on a value the step uses is swallowed. Not an Important finding; see Minor 1 for the asymmetry it introduces.
4. **accepted + rejected == attempts (risk 4).**
   - `reconcile_outer_rejection` (adaptive.rs:115–124) moves one count from accepted to rejected, so the sum is preserved.
   - The `saturating_sub` cannot fire, because the kernel has already added `accepted_steps += 1` for this step through the shared `&mut counters` (transactional_q1_q2.rs:1908–1912, integrate.rs L1170–1181).
   - The test asserts the sum, the split and accepted == retained steps. **Pass.**

### Strengths

- The port is exact. It was verified against the source commits instead of being retyped.
- The zero-scale rule is implemented once, documented, and has a bit-level unit test, including the signed-zero and NaN cases (norms.rs L152–168).
- The RED/GREEN evidence is concrete for every item. The counter test reproduces the real drift, (176, 169) against the ledger (91, 254).
- The report lists its own open points honestly.

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
1. `crates/vigilode-coupled/src/residual_first.rs:189,194`: `unused_stage_jvp_avoided += S * m`.
   - The field is documented as "Counterfactual eager JVP vectors avoided" (lib.rs:100–101). After item 5 an eager evaluation performs `(S−1)·m` stage JVPs, so every residual-only stop now overstates the avoided work by m. In `economy_robustness.rs:61` that is 24 where the definition now gives 21.
   - This is a reported economy figure made wrong by this task's change, and the brief asked for JVP expectations to be updated.
   - Fix: charge `(S − 1) * basis.len()` and update the expectation in `economy_robustness.rs:61`. Alternatively, rename or redocument the field as a count against the old eager baseline.

#### Minor (Nice to Have)
1. Eager and residual-first now differ in stage-JVP work per derivative (residual_first.rs:112 still reserves and computes `S·m`, stage 0 included). Residual-first can still fail on a stage-0 recomputation at the base, which eager no longer performs. The brief limited the change to eager, but cross-mode economy comparisons are now slightly biased against residual-first. Apply the same skip in a follow-up.
2. `rodas5p_fast.rs` L1081–1084: the two new driver ids are inline string literals, while the Slices variant uses a `const` (`RODAS5P_FAST_BANDED_SLICES_DRIVER_ID`). This came verbatim from PR #73; promote them to consts if they are meant to be stable record ids.
3. `a1_zero_tolerance_scale.rs` L1629–1647: the MF-fast regression test runs only the default (unstaged) path, so it cannot detect the remaining zero-scale abort at matrix_free_fast.rs:2674. Add a staged case when that site is brought under the rule.
4. Test-output noise: the touched-crate suite exits 101 with two failures that are pre-existing and depend on scheduling (`leja_setup_work_contract::counted_action_records_cold_coefficient_table_without_changing_authority`, `production_audit_pr101_provider::pr101_warmed_provider_operations_make_no_heap_allocations`). They are outside this diff, but the Task 6 full-suite run will meet them. The controller should track them.

### Assessment

**Task quality:** Needs fixes

**Reasoning:** All five defects are closed correctly. Positive-scale and default-kernel behaviour is bit-identical, and the port matches PR #73 exactly. The one Important item is a one-line accounting fix: item 5 left the residual-first "eager counterfactual" counter stale.
