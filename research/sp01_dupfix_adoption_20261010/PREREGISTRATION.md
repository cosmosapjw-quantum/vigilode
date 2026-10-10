# Preregistration: adoption review of duplicate-residual removal alone (SP01)

Node SP01 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`), executed on branch `audit/rvj-accuracy-speed-wave1-20261010`.
It depends on AS03, the fail-closed evidence validator, which must be merged before this node's checker runs.

**Claim boundary.**
- Counted work (WorkCounters) and same-binary callgrind instructions only. No wall-time claim; the timing authority
  stays on HOLD.
- No bundling with the coupled target, the predictive controller or the guard.
- "Adoption" means the stated decision rule below, not a global accuracy or speed claim.

## Question

Production restarted GMRES, in both `gmres.rs` (`solve_gmres_with_workspace`) and `gmres_into.rs`
(`solve_gmres_into`), recomputes a final true residual after the loop. When the loop ended because a true residual of
the same iterate already met the threshold, this recomputation is a duplicate: it costs one JVP and decides nothing.

ALG04 measured the `DupFix` arm on the GMRES-into path. It gave bitwise the same trajectories at 0.63-0.89x the JVPs
on small problems and about 0.98x on the Brusselators.

Can the duplicate be removed in both kernels, under a versioned residual-accounting contract, with exact output and
decision parity and exactly accounted savings? And can the change become the default without breaking any
pre-existing test or recorded receipt contract?

## Change

1. **Both kernels get `ResidualAccounting::{RecomputeFinal, ReuseConfirmed}`.**
   - `RecomputeFinal` is today's behaviour.
   - `ReuseConfirmed` skips the final residual only when the loop exited on a true residual of exactly the current
     iterate, and reuses that residual's norm.
   - Every other exit (budget exhaustion, a happy breakdown without confirmation, zero RHS, a nonzero x0 with no
     iteration) keeps the recomputation or its existing equivalent, so the acceptance residual always exists.
   - The report's `residual_norm` must be bitwise the same under both accountings, or the report contract must be
     versioned explicitly.
2. **Audit first.**
   - Enumerate every caller of both kernels: the sequential K-form, the protected forcing path, the U-form driver's
     default and into paths, LGMRES/GCRO-DR shared code if any, and the preconditioned callers.
   - Enumerate every pre-existing test and export that pins counters to recorded values.
   - The audit is committed with the code, as a table in the Results.
3. **Decision rule for the default, fixed now.**
   - If no pre-existing non-ignored test fails with `ReuseConfirmed` as the default of both kernels, the default
     becomes `ReuseConfirmed`.
   - Otherwise the default stays `RecomputeFinal`, and `ReuseConfirmed` is adopted as an explicit versioned option
     ("v2 accounting"), with the list of pinning tests and receipts recorded.
   - Either outcome is admissible. The gate below decides the node, and the outcome is reported.

## Cells

- **U-form matrix-free driver, GMRES into.**
  - Brusselator-50 and -160 at 1e-6 and 1e-8: the ALG04 C1 cells `Legacy` and `DupFix`.
  - The SPD07 problem set: robertson, van-der-pol-mu1000, hires, prothero-robinson-forced, quadratic-4,
    brusselator-1d-50, brusselator-1d-160.
- **U-form driver, default (non-into) GMRES path:** the same 14 cells.
- **Sequential matrix-free K-form RODAS5P:** the default library path (`integrate` / `sequential_matrix_free_step`
  with GMRES), on robertson, van-der-pol-mu1000, hires and brusselator-1d-50 at 1e-6 and 1e-8.
- **Kernel boundary cases.** Zero RHS; a nonzero x0 already within threshold; budget exhaustion (failure); happy
  breakdown; Jacobi left preconditioning (explicit-W sequential path); a WRMS `residual_scale`; a restart smaller than
  the iteration count.

## Commands

    SP01_RUNS=research/sp01_dupfix_adoption_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test sp01_dupfix_adoption -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --locked -p rodas5p-krylov --test dupfix_promotion_contract
    python3 tools/sp01_dupfix_profile.py --output research/sp01_dupfix_adoption_20261010/PROFILE.json
    python3 tools/sp01_dupfix_check.py --runs research/sp01_dupfix_adoption_20261010/RUNS.json --profile research/sp01_dupfix_adoption_20261010/PROFILE.json --output research/sp01_dupfix_adoption_20261010/RESULTS.json

**Profiling.** The profile tool runs callgrind on one release binary, an example or CLI subcommand added by this node,
that integrates one cell per invocation under each accounting. Ir per trajectory is taken as the difference of 2 and 1
repetitions, with a repeated 1-repetition run as a determinism check, as in SPD01-SPD09.

**Checker.** The checker must call `tools/evidence_schema_v2.py` (AS03) first and return INVALID on malformed
evidence.

## Gate

**PASS** if all of the following hold.

1. **Parity.** In every cell, the two accountings give bitwise identical states, output times, attempts, accepted and
   rejected steps, linear-solve failures, iterations and report residual norms. Every kernel boundary case gives the
   same solution, outcome and residual norm.
2. **Exact accounting.** In every cell, JVP(v1) - JVP(v2) equals the number of solves that exited on a confirmed true
   residual of the same iterate. That count is computed independently by the export, not inferred from the difference.
   Every counter other than the JVP/matvec totals and the diagnostic category is equal.
3. **No instruction regression.** Same-binary Ir per trajectory under `ReuseConfirmed` is <= 1.000x
   `RecomputeFinal`'s on every profiled cell. The profiled cells are van der Pol, HIRES and Brusselator-50 at 1e-6 on
   the U-form into path, and HIRES at 1e-6 on the sequential K-form.
4. **Contract.** Every pre-existing test passes under the default chosen by the decision rule. No pre-existing test
   file is modified.

Everything else is **FAIL**, with every number preserved.

**Predictions.**
- U-form into path JVP ratios as ALG04 DupFix: van der Pol 0.75, Robertson 0.77, HIRES 0.89, Prothero-Robinson
  0.63-0.65, Brusselators about 0.98.
- The sequential K-form saves 8 JVPs per attempt out of a larger total (7 assembly JVPs), so its ratios are smaller
  gains: about 0.80-0.95.
- Ir ratios between 0.80 and 1.00.
- **Decision rule: probably "versioned".** Many pre-existing contract tests pin JVP and matvec counts.

## Prior information

- ALG04 (L-0094) `DupFix` identity on 65 cells.
- No code of this node exists before this commit. The existing `GmresIntoOptions::skip_final_residual` (ALG04) is
  reused or subsumed; it is not removed.

## Results (appended after the recorded run; the registered text above is unchanged)

**Verdict: PASS** (all four gate items). **Decision-rule outcome: "versioned".** The default of both kernels stays
`RecomputeFinal` (v1). `ReuseConfirmed` (v2, `gmres-residual-accounting-reuse-confirmed-v2`) is an explicit option.

**Commits** (branch `wave1/sp01`, from `0283c1f`).
- `b6799e2` (A): implementation, the decision-rule trial receipt, new tests and the profiling example.
  - `ResidualAccounting` in `crates/rodas5p-krylov/src/accounting.rs`, used by `gmres.rs` and `gmres_into.rs`.
  - Selection on every registered path (see the audit table).
  - `DEFAULT_TRIAL.json` and `DEFAULT_TRIAL_FAILURES.json` (panic sites).
  - `tests/dupfix_promotion_contract.rs` (krylov) and `tests/sp01_dupfix_adoption.rs` (integrators).
  - `examples/sp01_dupfix_profile.rs`.
- `3ea305c` (B): `tools/sp01_dupfix_profile.py`, `tools/sp01_dupfix_check.py` (AS03 validation first) and
  `tools/test_sp01_dupfix_check.py`, committed before any recorded run.
- `9b0f57c` (C): `RUNS.json`, `PROFILE.json` and `RESULTS.json`, all produced on `3ea305c` with a clean tree.

**Commands.** The four registered commands were run unchanged: release, `RAYON_NUM_THREADS=1`,
`OPENBLAS_NUM_THREADS=1`, `CARGO_INCREMENTAL=0`.
- The export took about 2.5 min.
- The profile builds the example with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` and runs callgrind 3.22.
- The checker also runs the kernel contract command and the item-4 suites: debug workspace all-targets, doc tests,
  the three `rodas5p-integrators` feature suites and the Python tool tests (about 3 h).

**Change as implemented.**
- `ReuseConfirmed` reuses the loop's residual only when the loop left after at least one restart cycle, on a true
  residual that the operator computed for the current nonzero iterate.
- The zero-RHS / zero-iterate exit and the nonzero-x0 exit before any cycle keep the recomputation, as listed in
  the registration. Failures return before the final residual, as before.
- The report (`LinearSolveReport` / `GmresIntoReport`) is bitwise the same under both accountings. So the report
  contract is preserved, not versioned. Only `diagnostic_matvecs` and the JVP/matvec work it charges differ.
- ALG04's `GmresIntoOptions::skip_final_residual` is kept as is. It skips at every exit and overrides the
  accounting.
  - `dup_fix` reproduces ALG04's `RUNS.json` `dup_fix` records bit for bit on all 14 C1 cells.
  - v1 reproduces ALG04's `legacy` records bit for bit on all 14 cells.

**Gate.**

| Item | Result | Numbers |
|---|---|---|
| 1. Parity | PASS | 44/44 cells (uform_into 14, uform_default 14, kform_integrate 8, kform_step 8). Every record field is equal: output times, final state bits, attempts, accepted and rejected steps, error norms and failed attempts (K-form), and the stage-solve log (SHA-256 of every report's residual norm, relative residual and iterations). `linear_solve_failures` and `linear_iterations` are also equal. Kernel boundary contract: 10/10 tests pass |
| 2. Exact accounting | PASS | 44/44 cells. In every cell, `jvp_vectors` and `jvp_calls` v1 - v2 equal the observer's confirmed exits, which range from 83 (PR 1e-6) to 7,572 (vdP 1e-8). Only `diagnostic_matvecs`, `jvp_*` and `linear_matvec_vectors` differ. The wrapped runs reproduce the plain runs under both accountings. The v2 observer counts 0 confirmed duplicates in every cell |
| 3. No Ir regression | PASS | Ir per trajectory v2/v1: uform_into vdP 0.9725, HIRES 0.9620, Bruss-50 0.9962; kform_integrate HIRES 0.9890; kform_step HIRES 0.9933. Callgrind is deterministic, and every profiled run reproduces its RUNS record |
| 4. Contract | PASS | No pre-existing test file was modified: only 2 test files were added relative to `0283c1f`, and no `#[cfg(test)]` tail changed. The default follows the rule (9 trial failures, so `RecomputeFinal`), and RUNS records it. Suites under that default: workspace 975 passed / 0 failed (75 ignored); doc 3/0; audit2-research 606/0; bateman 612/0; stage-certificate 634/0; Python tools 249/0 |

**Decision-rule trial** (`DEFAULT_TRIAL.json`).
- Setup: the tree of `b6799e2` with `ResidualAccounting::DEFAULT = ReuseConfirmed`, debug, all five suites.
- Workspace: 966 passed, 9 failed. The feature suites failed 7 each, the same integrators tests. No new test failed.
- The 9 failing pre-existing tests are below. Every one pins a diagnostic count, a counter digest or a recorded
  export.

| Pinning test | What it pins |
|---|---|
| `rodas5p-krylov tests/krylov_contracts.rs::restarted_gmres_matches_direct_solution_and_certifies_true_residual` | `diagnostic_matvecs >= 1` per solve |
| `rodas5p-krylov tests/alg04_dup_fix_contracts.rs::skipping_the_final_residual_changes_only_the_diagnostic_counters` | `solve_gmres_into` = skip + one diagnostic |
| `tests/alg04_alg06.rs::dup_fix_is_legacy_without_the_diagnostic_residual` | Legacy: one diagnostic per stage solve |
| `tests/alg04_alg06.rs::legacy_ladder_at_the_alg04_budget_reproduces_the_alg01_base` | ALG01 `BASE.json` counters (receipt) |
| `tests/alg01_coupled_stage_target.rs::legacy_rows_reproduce_the_base_export_on_small_cases` | ALG01 `BASE.json` counters (receipt) |
| `tests/alg01_coupled_stage_target.rs::staged_policies_integrate_small_cases_without_a_duplicate_residual` | staged JVPs vs Legacy's diagnostic count |
| `tests/spd07_mf_step_warm_start.rs::default_guesses_reproduce_the_base_export` | SPD07 `BASE.json` counters (receipt) |
| `tests/frozen_full_e_shadow_contracts.rs::retained_level2_shadow_is_complete_charged_safe_and_rjf_identical` | charged shadow work equals recorded counters |
| `tests/a1_committed_trace_regression.rs::historical_external_digest_is_preserved_while_v2_trace_has_its_own_baseline` | committed G4/S5B0 trace digest (receipt) |

**Caller audit** (every caller of the two kernels; "default" = `ResidualAccounting::DEFAULT`, now v1).

| Caller | Kernel | Accounting selection |
|---|---|---|
| U-form driver, GMRES into (`rodas5p_matrix_free_fast.rs::attempt`) | `solve_gmres_into_with_accounting` | `Rodas5pMfFastWorkspace::set_residual_accounting`; `integrate_rodas5p_mf_fast_observed_with_residual_accounting(gmres_into = true)`. `DupFix` keeps ALG04's skip (overrides). Other entry points use the default |
| U-form driver, default path (same function) | `solve_gmres_with_workspace_and_accounting` | as above with `gmres_into = false` |
| Sequential K-form: `sequential_step`, `sequential_matrix_free_step`, the protected forcing path (`sequential_matrix_free_step_with_inner_forcing`, refinement passes included) | `solve_gmres_with_accounting` (with or without WRMS scale; Jacobi / factor PC on the explicit-W path) | `StageSolveAccounting` via `sequential_step_with_residual_accounting`, `sequential_matrix_free_step_with_residual_accounting`, `sequential_matrix_free_step_with_inner_forcing_and_residual_accounting`, `integrate_sequential_matrix_free_adaptive_observed_with_residual_accounting`. The old functions use the default (bitwise unchanged) |
| `integrate.rs` (`integrate_adaptive*`, `integrate_sequential_matrix_free_adaptive_observed`), `dense_output_v2.rs` | through the sequential steps | default |
| `block.rs`, `transactional_q1_q2.rs`, `common_w_gate.rs`, `audit2_matrix_free_research.rs` (feature), `rodas5p-krylov::solve_seeded_gmres`, `rodas5p-fair-ab` adapters | `solve_gmres` / `solve_gmres_with_workspace` | default only (no explicit selection; not registered paths) |
| LGMRES, LGMRES-into, GCRO-DR, Givens GMRES, staged GMRES, `GmresPrefixSession` / `solve_gmres_incremental` | own loops and final residuals | not changed (they share only `arnoldi_*` / breakdown helpers, not the final residual) |
| Pre-existing tests pinning counters | see the trial table | unchanged, pass under the v1 default |

**Reported, not gated.**
- **JVP ratios v2/v1** (attempts equal, so the same per attempt). uform_into and uform_default are identical:
  - Robertson 0.768 / 0.774, van der Pol 0.750 / 0.750, HIRES 0.889 / 0.889, quadratic-4 0.836 / 0.835.
  - Prothero-Robinson 0.683 / 0.677, Bruss-50 0.976 / 0.976, Bruss-160 0.987 / 0.980.
- **K-form ratios.**
  - kform_integrate: Robertson 0.843 / 0.837, vdP 0.830 / 0.830, HIRES 0.908 / 0.907, Bruss-50 0.977 / 0.977.
  - kform_step: 0.834 / 0.834, 0.826 / 0.826, 0.907 / 0.907, 0.977 / 0.977.
- **Predictions.**
  - The U-form ratios match ALG04 DupFix except Prothero-Robinson: 0.68 against the predicted 0.63-0.65.
    - v2 keeps the 13 zero-iterate final residuals (right-hand side below the absolute threshold) that DupFix
      skips.
    - On the into path DupFix is 4-13 JVPs below v2 per cell with an identical trajectory. The difference is
      exactly v2's remaining `diagnostic_matvecs`.
  - K-form, predicted 0.80-0.95: 12 of 16 cells are within; Bruss-50 (0.977) is outside.
  - Ir, predicted 0.80-1.00: all within. The savings are smaller than the JVP ratios (0.96-0.996), because a JVP
    is a small share of the instructions.
- **Decision rule**: predicted "probably versioned". Confirmed.
- **Observer decomposition** (v1 diagnostics = confirmed + nonzero-x0-before-any-cycle + zero-iterate).
  - Exact in 40 of 44 cells.
  - In 4 kform_integrate cells (Robertson 1e-6, vdP 1e-6, Bruss-50 1e-6 and 1e-8), 1-3 v1 final residuals are not
    classified. The v2 observer misses exactly the same number.
  - Only the no-cycle class is affected; the gated confirmed count equals the JVP difference in every cell. The
    likely cause is two no-cycle runs merging in the input stream, which the observer cannot separate. It was not
    investigated further.

**Deviations and disclosures.**
- **K-form entry points.** The registered K-form path ("integrate / sequential_matrix_free_step") was run, and
  profiled, on both entry points.
  - `kform_integrate` is the protected WRMS-forcing library driver.
  - `kform_step` is `rnext_common`'s adaptive loop over `sequential_matrix_free_step`.
  - Both use `LinearSolverConfig::default()` with GMRES. Both are gated, a superset of the registration; fixed in
    the checker docstring before the run.
- **Item 4.** The checker itself runs the pre-existing suites and audits test files against `0283c1f`; the
  registration did not say how item 4 is evaluated.
- **Interpretation.** The registration lists "a nonzero x0 with no iteration" among the exits that keep the
  recomputation. This was implemented literally: v2 never reuses a residual that preceded every cycle.
- **Absolute paths.** `RESULTS.json` records absolute paths for the trial receipt and ALG04's `RUNS.json`
  (`inputs`). Their SHA-256 binds them.
- **No other deviation**: no rerun and no checker change after the recorded run.
