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
