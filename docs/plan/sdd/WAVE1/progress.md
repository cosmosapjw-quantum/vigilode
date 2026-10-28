# SDD ledger — plan: docs/plan/IMPLEMENTATION_WAVE1.md
Spec: docs/plan/PRODUCTION_PLAN_20261020.md
Base: 9b5d083 (production/vigilode-plan-20261021)

## Pre-flight scan
| Pair / task | Shared surface | Finding |
|---|---|---|
| T1 vs T3 | rodas5p_fast.rs (T1 band-solve + error norm; T3 makes helpers pub(crate)) | Different hunks; T3 must not change arithmetic; merge conflict possible in visibility edits — resolve in T6 |
| T1 vs T2 | dense_output_v2.rs (T1 counter reconcile ~2403; T2 new enum variants ~1053, estimator ~1111) | Different regions |
| T2 vs T3 | none | — |
| T5 vs all | root Cargo.toml/Cargo.lock members; lib.rs exports read-only | T5 only adds a member; T3 adds lib.rs export; trivial merge |
| T4 vs all | research/LEDGER.jsonl append | Only T4 appends the ledger |
| T1 self | item 5 edits vigilode-coupled (own workspace) | OK; tests in its own dir |
| T2 self | Lemma 3 test needs constant defect — synthetic, not through the driver | OK, test the estimator formula |
| T3 self | "outputs equal fast within 1e-12" vs Richardson start x=P⁻¹c when fresh | Consistent: zero extra iterations when residual already below ρ |
| T5 self | Atol::Vector may be unsupported by drivers | Brief already says return Unsupported |

## Rulings
- Ruling: implementers run in parallel in separate worktrees/branches, contrary to the skill's "never parallel" — user explicitly asked for maximal parallelism and per-node push; isolation by worktree removes file races — cost if wrong: merge conflicts in T6, resolved by controller-dispatched fixer.
- Ruling: toolchain stable 1.97 instead of pinned 1.94.1 (download blocked) — only available toolchain — cost if wrong: an MSRV-only failure undetected; T6 notes it.
- Ruling: nodes push their own branches — user asked for push per node — cost: none beyond public draft branches.
- Ruling: removed an unrelated rec_bianchi hand-off block pasted into plan §5 T9 before committing — it targets another repository — cost if wrong: user wanted it kept; it is preserved in the scratchpad.
- Ruling: one shared target dir with debuginfo off instead of per-node target dirs — a full debug build hit the 27 GB disk allowance — cost: builds serialize on the cargo lock.

## Dispatches (wave 1, parallel, BASE 9b5d083 for all)
- Task 1 a1-defects: implementer opus dispatched
- Task 2 a2-wfiltered: implementer opus dispatched
- Task 3 b1-lagged: implementer opus dispatched
- Task 4 t-verify: implementer sonnet dispatched
- Task 5 a1-api: implementer opus dispatched
- Task 4: implementer DONE_WITH_CONCERNS (5811ca9..f2250b7, pushed; 6/6 claims PASS; checker exit 1 from pre-existing L-0124 mismatch; plan note: Cor 2a sign and loose factor 2). Reviewer sonnet dispatched.
- Task 4: review clean (2 verdicts approved; 5 minors deferred: dead vars verify_theory.py:606-607; claim-2 min at grid end; no cross-check vs rodas5p_stage_transfer_constants; Cor 2a grid-only; cosmetic tracking ref).
- Task 4: complete (commits 9b5d083..f2250b7, review clean)
- Ruling: check-research-node exit 1 from pre-existing L-0124 sha mismatch (CONTINUATION_DAG/FINDINGS) is out of scope for wave 1 — identical on base — cost: CI research-process job stays red until a tombstone row is added (wave 2 card).
- Ruling: plan T2 Cor 2a note (sign of T, factor 2 loose; true t/(1+t)) recorded, plan text not edited (append-only discipline) — cost: none.
- Task 3: implementer DONE_WITH_CONCERNS (c1dfb9e..16f6631): forced-refresh bit-identical to fast; kill gate FAIL (HIRES 0.654 fact/step, err 1.59x; Bruss 0.854). Cause: 6-iteration cap vs q≈0.1–0.2.
- Task 2: implementer DONE_WITH_CONCERNS (b61707c..7bea78c): interior 0.350 tol units; kill gate FAIL on steps 2.14x vs Off (1.25x vs Enforce); DP-01 0.335 with 21 vs 79 Jacobians.
- Ruling: shared target dir corrupted cross-worktree builds (same package ids); switched remaining nodes to per-node target dirs; Task 6 reruns every suite in one clean target — cost: wave-1 per-node test evidence for T2/T3 is provisional until T6.
- Ruling: Task 2 kill gate stays scored against Off as preregistered (FAIL); the Enforce comparison is reported as context only — cost if wrong: WFilteredEnforce not promoted to default in wave 2 without a new card.
- Task 2: review clean (spec ✅, quality approved). Minors deferred: I-1 no test of interior-forced rejection / counter sum; I-2 Lemma-3 bound cannot distinguish hγ from h/2 shift; I-3 LU/W clone every trial step (allocation); I-4 two extra public helpers.
- Task 2: complete (commits 9b5d083..7bea78c, review clean; kill gate FAIL as preregistered)
- Task 3: review clean (spec ✅, quality approved). ⚠️ resolved by controller: brief requires 1e-12 equivalence (bit-identity is an extra claim, not required); shared-target contamination is re-checked in Task 6 clean build. Minors deferred: WRMS weights |y_n| only; 6th correction not credited (convention undocumented); JVP-path test passes with zero iterations; contraction ratio skips x0; Rodas5pLaggedResult undocumented and not Serialize; codegen-units workaround in recorded runs.
- Task 3: complete (commits 9b5d083..16f6631, review clean; kill gate FAIL as preregistered)
- Task 5: implementer DONE_WITH_CONCERNS (53a7c42..f76d60f): 14 contract + 3 lib + 1 doctest; mass/dense/vector-atol refused Unsupported; added .autonomous().
- Task 5: review spec ❌ (2 Important: Stats missing driver counters/ids; stiff-example comparator choice). Minors deferred: StepSizeTooSmall catch-all label; Stats not Deserialize; BandJacobianFn zeroed-buffer doc; no Fatal/Recoverable tests for ft/J/JVP callbacks; clippy pre-existing count unexplained.
- Ruling: stiff-example test compares against the driver the facade routes to (MF fast), not the example's sequential driver — the brief's requirement is the problem bit-identical to the routed direct call — cost if wrong: one more test in wave 2.
- Ruling: partial solution after failure holds only t0 in wave 1 (drivers expose no last accepted state); JVP-fatal-in-GMRES path untested — deferred to wave 2 (driver must return partial state) — cost: facade's partial-solution promise is incomplete until then.
- Ruling: mass/dense_output refused Unsupported in wave 1; sequential dense path supports both but is outside the brief's driver set — wave 2 routes them — cost: P0 contract gap until wave 2.
- Task 5: fix round 1 dispatched (resume implementer).
- Task 5: fix round 1/5 (2 addressed, 0 open; commits f76d60f..a605573)
- Task 5: complete (commits 9b5d083..a605573, review clean)
- Task 1: implementer DONE_WITH_CONCERNS (9b5d083..cf697d6): 5 defects fixed, RED/GREEN each; 1150 pass, 2 pre-existing parallel-only failures; homotopy counter drift unfixed (out of brief).
- Task 1: review — 1 Important (residual-first avoided-JVP counter S·m vs (S−1)·m) + confirmed ⚠️ gap (zero-scale still rejected at rodas5p_matrix_free_fast.rs:2674 staged path and vigilode-coupled solver.rs:326). Minors deferred: residual-first stage-0 JVPs; FixedWidth2 id literals; MF zero-scale test only unstaged; 2 pre-existing scheduling-dependent failures (leja_setup_work_contract, pr101).
- Ruling: homotopy-driver counter drift (integrate.rs ~704, dense_output_v2.rs ~2271) deferred to wave 2 — not named in the brief — cost: homotopy accepted/rejected counts can still drift at the boundary.
- Task 1: fix round 1 dispatched.
- Ruling: staged MF zero-scale (rodas5p_matrix_free_fast.rs:2674) keeps its typed refusal, documented as an exception — that scale is a Krylov system weight from the start state, not an end-of-step error norm; the literal rule would reject components that start at 0 forever — cost if wrong: atol=0 unsupported on the staged Coupled path.
- Task 1: fix round 1/5 (3 addressed, 0 open; commits cf697d6..a8ec412). Minor deferred: fallback.rs:110 now receives atol=0.
- Task 1: complete (commits 9b5d083..a8ec412, review clean)
- Task 6: integration branch production/wave1-integration merged all five nodes with zero conflicts.
- Task 6: full suite (toolchain stable 1.97, debuginfo off): workspace 1530 passed / 3 failed / 109 ignored; vigilode-coupled and vigilode-hybrid exit 0. The 3 failures (leja_setup_work_contract, collective_admission_contracts, production_audit_pr101_provider) are global-allocation-counter tests that pass with --test-threads=1 (5/5, 7/7, 9/9); two were reported failing on base by Task 1.
- Ruling: the 3 scheduling-dependent allocation-counter failures are pre-existing test-isolation defects, not wave-1 regressions — they pass single-threaded on the integration branch — cost if wrong: a real allocation regression hidden; wave 2 card: serialize those tests.
- Final review: ready WITH FIXES (6 Important, 8 Minor). Full: final-review.md.
- Ruling: final-review item 5 — keep `wrms` strict (typed error on zero scale) for iteration-convergence callers and add a separate step-error norm helper carrying the zero-scale rule, used only by end-of-step error norms — the convergence callers measure contraction, not acceptance — cost if wrong: one more helper.
- Ruling: final-review item 2 — lagged driver zero Richardson weight returns a typed InvalidInput refusal (staged-MF precedent), not NonFinite — cost: atol=0 unsupported on the lagged driver.
- Ruling: final-review item 4 accepted — homotopy counter drift is named in spec §6; earlier deferral ruling withdrawn; fixed in the fix wave.
- Ruling: Tier::Performance routes no lagged driver until a new B1 card passes its kill gate (A2 and B1 gates FAILed).
- Ruling: final-review item 8 (#[non_exhaustive]) deferred — adding it is itself a breaking change; decide with the semver baseline in wave 2.
- Final fix wave dispatched (items 1–7, 11, 13, facade atol=0 test).
- Final fix wave DONE (15eb5f6..a5a48f7): workspace 1484 pass / 4 fail (known allocation-counter tests, pass single-threaded) / 109 ignored; coupled 37, hybrid 19. Count reconciled by controller: 1597 listed tests now vs 1586 before (earlier awk total 1642 included companions) = +11 new tests.
- Final fix wave re-review: all assigned items ADDRESSED; "item 13 NOT ADDRESSED" is a numbering slip — the controller's dispatch item 13 is the coupled message (review item 13), done; the LU clone (review item 12, minor) was not assigned.
- Ruling: wave-1 B1 kill-gate numbers were measured with a_7 = 1.01; a_7 is now 1.02 — the FAIL verdict stands (it was dominated by the iteration cap), and wave-2 B1b re-measures both solvers on the current constants — cost: none.
- Parked (deferred minors, wave 3): W-filtered LU clone every trial step; #[non_exhaustive] semver decision; facade Stats exposes internal types; homotopy ledger tests cannot fail first; coupled bins never forget successes (C1).
- Wave 1: complete (production/wave1-integration a5a48f7)
