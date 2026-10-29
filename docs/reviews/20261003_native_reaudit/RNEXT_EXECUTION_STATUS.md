# Remaining-only DAG: execution status

This file records how `NEXT_DEVELOPMENT_DAG.json` (R-NEXT-01 to R-NEXT-09) was executed on this branch
(`audit/rvj-native-followup-20261003`, PR #70). It is separate from the audit's own files, which are left byte for
byte as they were. Each research node was preregistered and pushed before its code and runs, has its ledger row in
`research/LEDGER.jsonl`, and has its results appended below its preregistration. Amendments made before a recorded
run are in the preregistrations, with the development observation that prompted them. Older ledger rows and verdicts
are unchanged: L-0038 stays FAIL, and timing authority stays on HOLD.

| DAG node | Research node | Ledger | Verdict | Finding |
|---|---|---|---|---|
| R-NEXT-01 | `rnext01_residual_output_20261003` | L-0049 | **FAIL** | the residual-to-output budget encloses every completed one-step case and explains all 20 completed L-0038 item-3 failures; at Robertson h = 1e-2 (stage states 1e117) the budget is 2e122 and cannot resolve a reject decision that is right |
| R-NEXT-02 | `rnext02_gmres_into_20261003` | L-0045 | PASS | `solve_gmres_into` is bitwise the existing GMRES on 408 frozen solves; per-solve allocations 0.025-0.24x at >= 4 columns per cycle; the MF driver switch is bitwise identical and allocates 0.02-0.28x of the sequential MF step per attempt |
| R-NEXT-03 | `rnext03_gcrodr_attribution_20261003` | L-0046 | **FAIL** | recycled GCRO-DR fails on 21 of 336 frozen Brusselator solves, cold GMRES and cold GCRO-DR on none; a stagnation reset removes every failure. A post-hoc probe shows recycle updates that break `A U = C`, reused unverified. Fails on the preregistered accounting rule (aborted cycles are charged but untraced); there is no false convergence |
| R-NEXT-04 | `rnext04_laguerre_admission_20261003` | L-0047 | PASS | opt-in Laguerre total admission: all 52 admitted cases enclose a 50-digit target (bound/error 7.4-1452x); unverified, timing, Chebyshev, over-degree and bad-budget cases are rejected; `total_error` stays `EstimateOnly`; depth capped at 12 |
| R-NEXT-05 | `rnext05_chart_provider_20261003` | L-0048 | PASS | model-specific certified chart provider and local-error controller: 19,652 points enclosed against a 50-digit reference, fast mode resolved, fail-closed refusals, and nothing enclosed without the cofactor forcing |
| R-NEXT-06 | `rnext06_homotopy_net_cost_20261003` | L-0050 | **FAIL** | on actual q=2 candidates both certificate arms close and accept with equal bounds; net margins are negative for n <= 8 (abstain) and +0.04 solve units per step at n = 16. Fails because the test left the certificate's finishing work out of the action-first arm, which its own check caught (corrected margins reported) |
| R-NEXT-07 | `rnext07_model_epoch_20261003` | L-0044 | PASS | an optional client model epoch rebuilds `f`, `f_t`, the operator and the recycle images on change; without it an interior change mixes stale and live model data (GCRO-DR then fails) |
| R-NEXT-08 | `rnext08_perturbation_stability_20261003` | L-0051 | **FAIL** | RVJ5's one-step error and step-map derivative deviation grow like K^2 on a semilinear two-mode system (a negative result: the no-chart RVJ path stops); RODAS5P's decrease. Fails on a scalar tolerance too strict for the binary64 RODAS5P tableau (`|R(iy)| = 1 + 1.3e-19`) |
| R-NEXT-09 | - | - | see below | publication on PR #70 without force, validation of the original and final heads |

Product changes, each behind an opt-in or research interface with the defaults unchanged:

- `OdeProblem::with_model_epoch` and the epoch check in the U-form MF workspace (R-NEXT-07).
- `solve_gmres_into` with `GmresCapacity`, the MF driver's `set_gmres_into` switch and attempt observer (R-NEXT-02).
- `solve_gcrodr_traced` with the stagnation reset (R-NEXT-03); the untraced solve is unchanged (contract test).
- `JointPhiReport::admit_laguerre_total`, `LAGUERRE_TOTAL_COMPONENTS`, `LAGUERRE_ADJOINT_MAX_DEPTH` and resource limits
  in `EnvelopeKey` (R-NEXT-04).
- The chart stepper in `chart_transport` (feature `audit2-research`) and `exp_neg_enclosure` (R-NEXT-05).
- `TransactionalQ1Q2StepReport::q2_candidate_stages` (R-NEXT-06).

## What the four FAIL verdicts mean

None of the four FAILs is a regression in an existing path.

- **R-NEXT-01.** The budget is valid but loose: a mean-value bound over large boxes. It is not a practical
  step-acceptance guard.
- **R-NEXT-03.** The attribution and the reset rule hold. The gate fails on a bookkeeping rule of the study.
- **R-NEXT-06.** The test undercharged one arm. Its own check caught it, and the economics are reported with the
  correction.
- **R-NEXT-08.** The gate fails on a tolerance. The scientific result (the RVJ5 negative result) stands.

No FAIL was turned into a PASS after the run.

## R-NEXT-09: publication and validation

- **Publication.** Every commit went to `audit/rvj-native-followup-20261003` without force and on top of `cbf1631`.
  There is no new branch or PR, no merge, and the workflow approval policy is untouched.
- **Original head `cbf1631`.** The repository's validation matrix ran locally (Rust 1.94.1): fmt, clippy (default
  and three feature sets), workspace tests, three feature-gated test runs, ignored tests in the measurement profile,
  readiness, research-node, authority and ignored-in-CI checks, the Python tool tests and the audit's algebra
  checks. The result is in the section below. The full workspace test, which the audit stopped at 600 s, completed
  here.
- **Final head.** The same matrix on the final head is in the section below.
- Hosted CI on each pushed head is GitHub's, reported separately from these local runs. `action_required` or a
  check that has not run is never counted as a pass.

## Validation results

**Original head `cbf1631`** (local, Rust 1.94.1). All 16 steps exited 0:

- fmt, and clippy `-D warnings` (default and three feature sets);
- workspace tests and three feature-gated test runs;
- ignored tests in the measurement profile;
- readiness, research-node, authority and ignored-in-CI checks;
- the 211 Python tool tests and the 4 algebra checks of `verify_algebra.py`.

Across all runs, 2,365 Rust tests passed and none failed. The full workspace test, which the audit stopped at 600 s
(exit 124), completed with exit 0.

**Code head `b36092e`** (local).

- fmt, clippy (4 configurations), workspace tests, the three feature-gated runs, research-node, authority,
  ignored-in-CI, Python (211) and algebra checks: exit 0.
- The `ignored` and `readiness` steps first failed to compile the new research tests in the measurement profile
  (unresolved imports from `rodas5p-krylov`). The cause was the build directory, shared with the `cbf1631` worktree:
  Cargo judged that worktree's measurement-profile `rodas5p-core`/`rodas5p-krylov` artifacts fresh for this one. Both
  steps were rerun on the same tree after the crates were rebuilt from this worktree, and both exited 0 (106 tests).
  The debug-profile steps had rebuilt those crates from this worktree and ran every new test.

**Hosted CI on `b36092e`**: one check failed. The job `a1-inner-tolerance-parity` ran the existing test
`rodas5p_fast_allocations`, which counted 50 allocations in one run and 46 in the other. The same job passed on
`cec144a`, which has the same Rust code. The test counted allocations with one global counter, so allocations by
the test harness's own thread during a measured run were counted too. The follow-up commit counts allocations per
thread; the measured drivers run on the calling thread. That run passes locally five times in a row. No assertion
was relaxed, and no test is skipped. Every other hosted check on `b36092e` passed or was skipped by its workflow's
own conditions.

Later commits change only this file and that test.
