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
