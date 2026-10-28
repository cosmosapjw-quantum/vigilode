# Node a1-defects (wave 1, Task 1)

Branch `production/a1-defects` from `9b5d083`. Scope: the verified correctness defects of
`PRODUCTION_PLAN_20261020.md` §6 named in the task brief (items 1–5).

Toolchain: `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0; `rust-toolchain.toml` pins 1.94.1, which
cannot be downloaded here and was not edited). Env: `CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `-j 1`, `--locked`. Early runs used
`CARGO_TARGET_DIR=/home/claude/targets/shared`. After the controller ruling, every result below
was re-run in the isolated `CARGO_TARGET_DIR=/home/claude/targets/a1-defects` (see "Re-runs").

## Commits

| SHA | Item | Subject |
| --- | --- | --- |
| 2552a7d | 1 | fix(integrators): reject nonfinite band solves; checked size arithmetic (port of PR #73) |
| 2a619c7 | 2 | fix(core,integrators): one zero-tolerance-scale rule for every error norm |
| 4e82db7 | 3 | fix(fair-ab): char-boundary-safe operator id prefix in StableDenseOperator |
| 827e9ce | 4 | fix(integrators): reconcile outer tail rejections in the transactional drivers |
| a83cebd | 5 | fix(coupled): do not evaluate or charge the unused stage-0 derivative JVPs |
| 7d1778f | 1 | style(integrators): clear two clippy warnings in the ported band kernel |

## Items

1. **PR #73 band-solve hardening.** `git diff b56bb19..b51c051 -- crates/` applied cleanly:
   `rodas5p_fast.rs` was byte-identical at `b56bb19` and at `9b5d083`. `tests/banded_recovery_contract.rs`
   is in that range, so `f8e89b7` was not needed. Ported: nonfinite input/output rejection, checked
   size arithmetic and an `isize` byte bound in `rodas5p_fast_banded_solve`; opt-in
   `BandedKernel::FixedWidth2` and `FixedWidth2ExactCache` (`exact_band_cache.rs`); the internal
   tests `banded_exact_tests.rs`. Default kernel selection is unchanged: the new variants are
   reached only when a caller names them, and `Workspace::factor` → `factor_with_policy` without a
   cache does what the old code did (count one factorization, then factor). No research
   directories or ledger rows were ported. The test file was rustfmt-formatted.
   - RED (base source): 4 of 9 failed: `nonfinite_upper_entry_is_not_a_successful_solution`,
     `nonfinite_rhs_is_rejected`, `finite_inputs_with_overflowing_solution_are_rejected`,
     `size_arithmetic_overflow_is_a_typed_error_not_a_panic` (panic "attempt to add with overflow"
     at `rodas5p_fast.rs:1078`). The 4 retry probes and the tiny-solution test passed already.
   - GREEN: 9/9; lib `banded`/`exact_band` tests 14/14.
2. **Zero tolerance scale.** New `rodas5p_core::scaled_error_component(error, scale)`:
   scale 0 and error ±0 gives 0; scale 0 and error nonzero or NaN gives +inf; otherwise
   `error / scale`, the same bits as before. `wrms` uses it and accepts scale ≥ 0 (negative or
   non-finite is still a typed error). `error_scale` accepts a zero scale. Called from
   `rodas5p_fast.rs`, `rodas5p_fast_small.rs` (2 sites), `rodas5p_fast_small_batch.rs` and
   `rodas5p_matrix_free_fast.rs` (which still rejects a negative/non-finite scale).
   The protected/sequential, BDF, Radau, SABR, homotopy, transactional, … paths reach the rule
   through `error_scale` + `wrms`.
   - RED: `core_contracts::zero_scale_follows_the_one_rule` failed with `InvalidInput("invalid error scale")`;
     `tests/a1_zero_tolerance_scale.rs` 6/6 failed: fast dense, fast banded, fast small and small
     batch with "maximum step count or minimum step reached" (NaN norm, perpetual rejection);
     matrix-free fast with `InvalidInput("every output error scale must be finite and positive")`;
     protected sequential with `InvalidInput("invalid error scale")`.
   - GREEN: 6/6 and 1/1; the zero component is exactly zero and has no rejected step.
3. **`budget.rs` slice.** `operator_id.get(..16)` then `from_str_radix`; failure is the existing
   `InvalidInput("invalid operator digest")`.
   - RED: `short_operator_id_…` panicked "end byte index 16 is out of bounds for string of length 0";
     `non_ascii_operator_id_…` panicked "end byte index 16 is not a char boundary; it is inside 'é'".
   - GREEN: 3/3.
4. **Counter drift.** `reconcile_outer_rejection(&mut counters, report.step.accepted, accepted)`
   in `integrate_transactional_q1_q2_adaptive_observed_with_admission` (integrate.rs) and
   `integrate_transactional_q1_q2_adaptive_dense_observed` (dense_output_v2.rs). The test drives
   Prothero–Robinson λ = −1e5 with a permissive tail allowance (`absolute_output_budget = 10`,
   `embedded_budget_fraction = 10`, `gmres_rtol = 1e-2`, `max_target_relative_residual = 1`).
   - RED: both drivers reported counters (accepted, rejected) = (176, 169), but the ledger was
     (91, 254) for 91 retained steps. accepted + rejected = attempts = 345 held even before the fix:
     the drift is in the split.
   - GREEN: 3/3; counters = ledger = (91, 254); accepted = retained steps.
5. **vigilode-coupled eager stage-0 JVPs.** `evaluate` now reserves `(S−1)·m` stage JVPs
   (it was `S·m`), and for stage 0 skips the linearization and the m JVPs (its derivative stays the
   zero matrix, and the coupling loop never reads it). Output bits are unchanged: eager equals
   residual-first bitwise in the new test and in the existing
   `worker_and_theta_variants_preserve_eager_solution_bits` / `policy_preserves_complete_state_and_embedded_bits`;
   residual-first is untouched. Old → new counts on the three-mode model (n = 96, sequential):

   | case | evaluations (basis) | stage JVP | total JVP |
   | --- | --- | --- | --- |
   | eps 0, h 1e-3 | [3] | 24 → 21 | 35 → 32 |
   | eps 1, h 1e-2 | [3, 3, 3] | 72 → 63 | 83 → 74 |
   | eps 10, h 1e-3 | [2, 2] | 32 → 28 | 42 → 38 |

   The first row was measured on both sources. In the other two rows the old values follow from the
   identical evaluation sequence: the old test stops at its first failing case. No existing test
   expectation needed a change.
   - RED: `eager_evaluation_charges_no_stage_zero_derivative` failed: 24 vs 21 (eps 0, bases [3]).
   - GREEN: 2/2; crate suite 35/35.

## Touched-target suites (isolated target dir)

- `cargo test -j 1 --locked --no-fail-fast -p rodas5p-core -p rodas5p-fair-ab -p rodas5p-integrators`:
  exit 101. 253 binaries, 1150 passed, 2 failed, 70 ignored. Both failures fail on base sources too and
  pass with `--test-threads=1`:
  - `rodas5p-core --test leja_setup_work_contract::counted_action_records_cold_coefficient_table_without_changing_authority`
    (depends on parallel test scheduling; cause not investigated, not in a touched file).
  - `rodas5p-integrators --test production_audit_pr101_provider::pr101_warmed_provider_operations_make_no_heap_allocations`
    (global allocation counter sees sibling-thread allocations; base source: left 2 and 16 in two runs).
- `crates/vigilode-coupled`: `cargo test -j 1 --locked --no-fail-fast`: 35 passed, 0 failed.
- `cargo check -p rodas5p-cli --locked`: ok (new `BandedKernel` variants compile; no exhaustive match).
- `cargo clippy --all-targets --locked` on core, fair-ab, integrators and vigilode-coupled: the two
  warnings that came with the PR #73 code were fixed in 7d1778f. The remaining warnings in touched
  files (`vigilode-coupled/src/solver.rs` lines 317, 348, 478) are in code that was already there.

## Re-runs in the isolated target dir

Every RED result of items 1–3 was re-run on the base sources (`git checkout 9b5d083 -- <crate>/src`,
tests kept, then restored) in `/home/claude/targets/a1-defects`. Results were identical to the
shared-dir runs. Items 4 and 5 were built in the isolated dir from the start. Every GREEN result
and the suites above come from the isolated dir.

## Deviations and open points

- Item 2 changes the public `error_scale`/`wrms` contract: a zero scale is no longer a typed error.
  The brief asked for this. Positive-scale results are bit-identical. Consumers that use the scale
  vector outside `wrms` fail closed at a zero scale: `phi_error_proxy` floors at `f64::MIN_POSITIVE`
  (gives a huge estimate, so the step is rejected); the inner-forcing target rejects an infinite norm
  with a typed error.
- Not changed (outside the brief): the coupled stage-target weights `1/scale` in
  `rodas5p_matrix_free_fast.rs` (~2666) still reject a zero scale; `vigilode-coupled/src/solver.rs:326`
  has its own "invalid error scale"; the Krylov residual weights keep their positive-scale check.
- Item 4: the homotopy drivers (`integrate_homotopy_adaptive_observed`, integrate.rs ~704, and
  `integrate_homotopy_adaptive_dense_observed`, dense_output_v2.rs ~2271) have the same
  `accepted && error <= 1` pattern without reconciliation. The plan names them; the brief does not.
- Item 5: the residual-first derivative phase still computes S·m JVPs, stage 0 included, and
  `unused_stage_jvp_avoided` still counts the counterfactual as S·m. The brief limited the change to
  the eager path. Eager and residual-first therefore now differ in stage-JVP work per derivative.
  Also, a failing stage-0 linearization or JVP no longer fails an eager evaluation.

## TDD evidence (raw excerpts, isolated target dir, base sources)

```
test zero_scale_follows_the_one_rule ... FAILED
thread 'zero_scale_follows_the_one_rule' (23171) panicked at crates/rodas5p-core/tests/core_contracts.rs:32:71:
called `Result::unwrap()` on an `Err` value: InvalidInput("invalid error scale")
test result: FAILED. 12 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test non_ascii_operator_id_is_a_typed_error_not_a_panic ... FAILED
test short_operator_id_is_a_typed_error_not_a_panic ... FAILED
thread 'non_ascii_operator_id_is_a_typed_error_not_a_panic' (23174) panicked at crates/rodas5p-fair-ab/src/budget.rs:59:53:
end byte index 16 is not a char boundary; it is inside 'é' (bytes 15..17 of string)
thread 'non_ascii_operator_id_is_a_typed_error_not_a_panic' (23174) panicked at crates/rodas5p-fair-ab/tests/stable_operator_id_contracts.rs:38:55:
thread 'short_operator_id_is_a_typed_error_not_a_panic' (23175) panicked at crates/rodas5p-fair-ab/src/budget.rs:59:53:
end byte index 16 is out of bounds for string of length 0
thread 'short_operator_id_is_a_typed_error_not_a_panic' (23175) panicked at crates/rodas5p-fair-ab/tests/stable_operator_id_contracts.rs:22:55:
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test dense_driver_reconciles_outer_tail_rejections ... FAILED
test observed_driver_reconciles_outer_tail_rejections ... FAILED
thread 'dense_driver_reconciles_outer_tail_rejections' (23178) panicked at crates/rodas5p-integrators/tests/a1_transactional_counter_reconciliation.rs:67:5:
  left: (176, 169)
 right: (91, 254)
thread 'observed_driver_reconciles_outer_tail_rejections' (23179) panicked at crates/rodas5p-integrators/tests/a1_transactional_counter_reconciliation.rs:67:5:
  left: (176, 169)
 right: (91, 254)
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test fast_dense_driver_accepts_a_zero_component_with_zero_atol ... FAILED
test fast_banded_driver_accepts_a_zero_component_with_zero_atol ... FAILED
test fast_small_driver_accepts_a_zero_component_with_zero_atol ... FAILED
test fast_small_batch_driver_accepts_a_zero_component_with_zero_atol ... FAILED
test protected_sequential_driver_accepts_a_zero_component_with_zero_atol ... FAILED
test matrix_free_fast_driver_accepts_a_zero_component_with_zero_atol ... FAILED
thread 'fast_dense_driver_accepts_a_zero_component_with_zero_atol' (23182) panicked at crates/rodas5p-integrators/tests/a1_zero_tolerance_scale.rs:117:5:
thread 'fast_banded_driver_accepts_a_zero_component_with_zero_atol' (23181) panicked at crates/rodas5p-integrators/tests/a1_zero_tolerance_scale.rs:144:5:
thread 'fast_small_driver_accepts_a_zero_component_with_zero_atol' (23184) panicked at crates/rodas5p-integrators/tests/a1_zero_tolerance_scale.rs:159:5:
thread 'fast_small_batch_driver_accepts_a_zero_component_with_zero_atol' (23183) panicked at crates/rodas5p-integrators/tests/a1_zero_tolerance_scale.rs:179:9:
thread 'protected_sequential_driver_accepts_a_zero_component_with_zero_atol' (23186) panicked at crates/rodas5p-integrators/tests/a1_zero_tolerance_scale.rs:228:6:
protected driver must not abort on a zero scale: InvalidInput("invalid error scale")
thread 'matrix_free_fast_driver_accepts_a_zero_component_with_zero_atol' (23185) panicked at crates/rodas5p-integrators/tests/a1_zero_tolerance_scale.rs:204:6:
matrix-free fast driver must not abort on a zero scale: InvalidInput("every output error scale must be finite and positive")
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test finite_inputs_with_overflowing_solution_are_rejected ... FAILED
test nonfinite_rhs_is_rejected ... FAILED
test nonfinite_upper_entry_is_not_a_successful_solution ... FAILED
test size_arithmetic_overflow_is_a_typed_error_not_a_panic ... FAILED
---- finite_inputs_with_overflowing_solution_are_rejected stdout ----
thread 'finite_inputs_with_overflowing_solution_are_rejected' (23192) panicked at crates/rodas5p-integrators/tests/banded_recovery_contract.rs:27:5:
thread 'nonfinite_rhs_is_rejected' (23193) panicked at crates/rodas5p-integrators/tests/banded_recovery_contract.rs:22:5:
thread 'nonfinite_upper_entry_is_not_a_successful_solution' (23194) panicked at crates/rodas5p-integrators/tests/banded_recovery_contract.rs:17:5:
---- size_arithmetic_overflow_is_a_typed_error_not_a_panic stdout ----
thread 'size_arithmetic_overflow_is_a_typed_error_not_a_panic' (23195) panicked at crates/rodas5p-integrators/src/rodas5p_fast.rs:1078:35:
attempt to add with overflow
  10: <banded_recovery_contract::size_arithmetic_overflow_is_a_typed_error_not_a_panic::{closure#0} as core::ops::function::FnOnce<()>>::call_once
thread 'size_arithmetic_overflow_is_a_typed_error_not_a_panic' (23195) panicked at crates/rodas5p-integrators/tests/banded_recovery_contract.rs:39:5:
    finite_inputs_with_overflowing_solution_are_rejected
    size_arithmetic_overflow_is_a_typed_error_not_a_panic
test result: FAILED. 5 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

# item 5 (vigilode-coupled, isolated dir)
test eager_solution_bits_equal_residual_first ... ok
test eager_evaluation_charges_no_stage_zero_derivative ... FAILED

thread 'eager_evaluation_charges_no_stage_zero_derivative' (19103) panicked at tests/eager_stage_jvp_contracts.rs:69:9:
assertion `left == right` failed: eps 0: stage JVPs 24 for evaluations with bases [3]
  left: 24
 right: 21
stack backtrace:
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

error: test failed, to rerun pass `--test eager_stage_jvp_contracts`
```

## Self-review

- Diff reviewed against 9b5d083 (19 files). Default paths: positive-scale norms are the same expression; banded default kernels unchanged; reconciliation only alters counters, not step decisions; coupled eager output bits verified equal to residual-first.
- Two clippy warnings introduced by the PR #73 port fixed (7d1778f).
- Earlier in the session I used path-limited `git stash push/pop` twice (before the shared-stash warning); the stash stack was verified empty afterwards and the tree clean, so no other session's entry was touched.

# Fix report, round 1 (review task-1-review.md)

Target dir: /home/claude/targets/a1-defects. Env: RUSTUP_TOOLCHAIN=stable, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0, -j 1, --locked. vigilode-coupled commands were run from crates/vigilode-coupled.

## 1. Residual-first counterfactual counter (Important): fixed in b5bf1f3
- `residual_first.rs` (both residual-only stops): `unused_stage_jvp_avoided += (S - 1) * m`, previously `S * m`. The doc of `Work::unused_stage_jvp_avoided` in lib.rs now states (S - 1) m.
- `economy_robustness.rs:61`: expectation 24 -> 21. No other test encodes the value (grep for `unused_stage_jvp_avoided` in tests/ and src/).
- RED, `cargo test -j 1 --locked --test economy_robustness lazy_policy` with the expectation updated and the code unchanged:
  `lazy_policy_makes_the_same_valid_candidate_with_smaller_jvp_budget ... FAILED  left: 24  right: 21`
- GREEN, `cargo test -j 1 --locked --no-fail-fast` (vigilode-coupled): every binary passes.

## 2a. vigilode-coupled zero scale (solver.rs:326): fixed in f739069
- Evidence: the `scale <= 0` check at solver.rs:326 could not be reached, because `validate` already rejected `atol <= 0` ("tolerances must be finite and positive") and `rtol > 0`. A zero scale therefore surfaced as an Input error at validation.
- Change:
  - `validate` accepts `atol == 0` when `rtol > 0`. A negative atol, a non-finite atol and `rtol == 0` stay Input errors.
  - `full_check` rejects only a negative scale.
  - The residual WRMS and the embedded WRMS use `rodas5p_core::scaled_error_component`. Positive-scale components give the same bits as `x / s`. A zero-scale component with a zero value contributes 0. One with a nonzero value sets that norm to +inf after the overflow check, so the outcome is a rejection (`embedded_test_passes = false`; the residual gate fails and the basis expands), not a NonFinite failure.
- Test `tests/zero_tolerance_scale.rs`, which runs both eager and residual-first with atol 0, rtol 1e-6, odd components identically zero, eps 0 and 1:
  - RED: `zero_atol_with_identically_zero_components_gives_a_candidate ... FAILED`, `eps 0: Some(Failure { kind: Input, message: "tolerances must be finite and positive" })`. `zero_atol_and_zero_rtol_is_still_an_input_error` passed.
  - GREEN: 2/2. The zero components of y_new are exactly 0.
- Suite: `cargo test -j 1 --locked --no-fail-fast` (vigilode-coupled): 37 passed, 0 failed (5+2+2+4+10+12+2).
- Clippy (`--all-targets`): only the existing warnings remain, at solver.rs 318/351/511 (previously 317/348/478, shifted by the inserted lines), residual_first.rs:155 and tests/contracts.rs. None are on changed lines.

## 2b. Staged matrix-free path (rodas5p_matrix_free_fast.rs ~2666-2674): NOT changed. Needs a decision
- Reproduced with a temporary probe that was not committed: `integrate_rodas5p_mf_fast_observed_with_stage_target` with policies Coupled and CoupledGuarded, atol 0, rtol 1e-6, the y1 ≡ 0 problem of a1_zero_tolerance_scale.rs.
  - Result: `Err("invalid numerical input: the coupled stage target needs finite positive error weights")`.
  - With atol 1e-12 the same run gives `Ok(success, 0 rejections)`.
- Why I did not apply the rule mechanically: this scale is not an endpoint error norm.
  - It is `atol + rtol |y|` of the START state only.
  - It is used as a change of variables for the Krylov system, `DiagonalScaledOperator`: x = D z, rows weighted by 1/D. The stage-residual target is measured in that metric.
  - A zero entry therefore forces x_i ≡ 0 in the Krylov space and makes the weight row infinite.
  - Applying the norm rule literally (residual_i must be exactly 0, otherwise +inf and reject) gives perpetual rejection for any component that starts at 0 and must grow, e.g. y_i' = 1 with y_i(0) = 0. Shrinking the step cannot help, because the start state stays 0.
  - The unstaged MF path uses max(|y|, |y_new|) and does not have this problem.
- Options:
  - (a) Keep the typed refusal. Document the staged metric as an exception to the endpoint-norm rule (start-state weights are a preconditioning metric).
  - (b) Apply the rule literally: x_i ≡ 0, and a nonzero residual_i means inf and rejection. Correct only for identically-zero components; otherwise the attempt is rejected forever.
  - (c) Give zero-scale components a substitute weight, e.g. operator weight 1 there plus an exact zero-residual post-check, or a weight from an endpoint predictor. This is a new metric or policy and needs a decision and its own test design.
- Recommendation: (a) now, with (c) as a research card if the staged path is promoted. I will implement whichever option the controller picks, with a RED/GREEN test on the staged path.

## Not touched (per controller)
Homotopy counter reconciliation; residual-first stage-0 JVPs; FixedWidth2 id literals.

## Fix round 1, ruling (a): staged matrix-free zero weight kept as a documented exception
- Commit: see `git log` on production/a1-defects (docs(integrators): document the staged-target zero-weight exception).
- `rodas5p_matrix_free_fast.rs` (the `staged_scale`/`staged_weight` loop): a doc comment states the exception. The code is unchanged and still returns the typed `InvalidInput("the coupled stage target needs finite positive error weights")`.
- Test `a1_zero_tolerance_scale.rs::staged_coupled_target_refuses_a_zero_start_weight_with_a_typed_error` uses atol 0, rtol 1e-6 and the y1 ≡ 0 problem. It asserts:
  - under `catch_unwind`, a typed InvalidInput with no panic for Coupled and for CoupledGuarded;
  - success with all-finite outputs on the unstaged MF path.
  The test pins behaviour that is kept, so it has no RED phase.
- Command: `cargo test -j 1 --locked -p rodas5p-integrators --test a1_zero_tolerance_scale --test alg01_coupled_stage_target` (isolated dir).
  - a1_zero_tolerance_scale: 7 passed.
  - alg01_coupled_stage_target: 10 passed, 4 ignored (release-only exports).
- `docs/plan/nodes/a1-defects.md` has a new section for the ruling.
