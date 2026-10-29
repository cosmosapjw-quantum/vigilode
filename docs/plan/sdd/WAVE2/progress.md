# SDD ledger — plan: docs/plan/IMPLEMENTATION_WAVE2.md
Spec: docs/plan/PRODUCTION_PLAN_20261020.md
Base: a5a48f7 (production/wave1-integration after the final fix wave); Task 1 started earlier from 148b515

## Pre-flight scan
| Pair / task | Shared surface | Finding |
|---|---|---|
| T1 vs T2 | rodas5p-integrators lib.rs exports | trivial merge |
| T2 vs T3 | T2 rodas5p_lagged.rs; T3 rodas5p_fast.rs + MF result types | disjoint files; T2 must not edit rodas5p_fast.rs |
| T3 vs T5 | T5 path-depends on the facade crate T3 changes | T5 uses facade API as of base; T6 rebuilds T5 against merged facade |
| T4 vs T5 | both append research/LEDGER.jsonl | ids pre-assigned L-0138 (T4), L-0139 (T5); append-only conflict resolved in T6 by concatenation |
| T4 vs all | root Cargo.toml members/Cargo.lock (vigilode-coupled joins) | T5 is its own workspace; no other node edits the root lock |
| T1 self | AdmissionRule uses costs, no wall time | consistent with T8 |
| T2 self | preregistration before measurement within the node | OK |
| T5 self | own workspace inside crates/ needs its own [workspace] table | stated |

## Rulings
- Ruling: ledger ids pre-assigned (T4 L-0138, T5 L-0139) — two nodes append in parallel — cost: none.
- Ruling (superseded): Fortran comparators were out of scope; gfortran 13.3.0 and SUNDIALS 6.4.1 installed via apt, sources pinned from PR #71 branch and Assimulo 3.0 sdist — E1 now includes CVODE, DVODE, RADAU5, RODAS — cost: larger E1 node.
- Ruling: Task 1 (c1-transaction) starts early from 148b515 (fix-wave tip so far) while the wave-1 suite runs — it adds a new module only and does not depend on the remaining fix-wave commits — cost: a rebase/merge in T6 if the fix wave touches lib.rs.
- Task 1: implementer DONE_WITH_CONCERNS (148b515..f07c4d7): 20/20 new tests, lib 75. Added overhead guard (cap net overhead at 0.1 × counterfactual protected cost) because Beta(1,1) UCB 0.879 admits a failing candidate unthrottled.
- Ruling: accept the overhead guard — it is what T8 Proposition 8 promises (≤10% overhead whatever the candidate); the brief's literal rule could not meet its own test — cost: fresh bins wait ~m steps before the first attempt.
- Task 1: review clean. Minors deferred: bins never forget successes (regime change inside a bin; note for C4); causality check is caller-supplied charged_units>0, not tied to a Trial ledger; overshoot case untested; declines<=m off-by-one near integer 10τ.
- Task 1: complete (commits 148b515..f07c4d7, review clean)
- Ruling: Tasks 2–5 start from a5a48f7 while the wave-1 fix re-review is still running — the re-review is read-only; any follow-up fix merges in T6 — cost: possible rework if the re-review finds breakage.
- Task 4: implementer DONE_WITH_CONCERNS (a5a48f7..d41bf16): allocation tests deterministic (60/60 parallel runs); L-0138 appended; vigilode-coupled joined workspace (+10 lock lines, 37 tests). check-research-node still exit 1 — clearing it needs a checker change, which the sandbox denied as audit-tool modification; not retried — surfaced to user.
- Task 4: review clean. Minors deferred: codegen-units 1→16 note missing from crate README/REPRODUCE.md; coupled-*.yml manifest paths now resolve to root target; thread-local counters miss future spawned threads; L-0138 spacing style.
- Task 4: complete (commits a5a48f7..d41bf16, review clean; checker exit 1 pending user decision)
- Task 2: implementer DONE (a5a48f7..c8b7e11): kill gate FAIL — HIRES fact/step 0.073 but error 3.75x; Bruss fact/step 0.009 but steps 11.95x, error 11.9x. Implementer hypothesis: GMRES hides staleness, P⁻¹r underestimates W⁻¹r (‖W⁻¹r‖ ≤ ‖W⁻¹P‖‖P⁻¹r‖, ‖W⁻¹P‖ ≤ 1/(1−q) only when q<1).
- Task 2: review — FAIL is method-level, not an implementation defect (operator, residual units, preconditioner side, J₀ all verified); refresh rule has no h/h_P term, so ‖W⁻¹r‖ ≈ (h/h_P)‖P⁻¹r‖ (interpretation, unmeasured). 1 Important (GMRES column/LS block triplicated). Minors deferred: happy breakdown with Continue returns Err; krylov error mid-stage breaks ledger identity; no stage-level stale-P vs W⁻¹c test; explicit residual costs 3 LU + 2 W per column.
- Ruling: B1 family next card must bound P staleness explicitly (refresh when |1−h/h_P| or a Jacobian-drift estimate exceeds a T2-derived limit, and monitor an estimate of ‖W⁻¹r‖ = ‖(I−T)⁻¹P⁻¹r‖ with ‖T‖ ≤ 2|1−h/h_P| + drift) — the stopping quantity of T9 assumed P≈W — cost: B1 family stays out of Tier::Performance.
- Task 2: fix round 1 dispatched.
- Task 2: fix round 1/5 (1 addressed, 0 open; commits c8b7e11..16edef2)
- Task 2: complete (commits a5a48f7..16edef2, review clean; kill gate FAIL)
- Task 3: implementer DONE_WITH_CONCERNS (a5a48f7..a8b15bd): facade mass/dense/at()/partial solutions/failure reasons; 2 commits authored as Claude noreply (not rewritten; user chose to leave identities as-is). Dense output uses InteriorSampling::EveryStep (new); JVP-only + dense refused.
- Task 3: review clean. Minors deferred: interior estimate unavailable → step accepted, status Success (counted in `unavailable`); no at() check at interior step end; JVP-in-GMRES not asserted; eprintln! noise.
- Ruling: partial solution without dense output keeps [t0, t_last] — interior queries need dense output anyway — cost: users of sparse output lose intermediate output samples after a failure.
- Ruling: mass matrix without dense output runs interior control Off — no interior values are returned — cost: none for returned values.
- Task 3: complete (commits a5a48f7..a8b15bd, review clean)
- Task 6: merged h-hygiene, c1-transaction, p0-facade, b1b-gmres into production/wave2-integration (64bc5f4) with zero conflicts; pushed. Waiting for e1-bench before the full suite (to keep its diagnostic timing undisturbed).
- Task 5: implementer DONE_WITH_CONCERNS (a5a48f7..76032cd): 13 arms incl. CVODE, DVODE, RADAU5, RODAS, Diffsol; 102 rows, 1 failure kept; L-0139. Merged into wave-2 integration.
- Task 5: review — 1 Important (clipped-step cost undisclosed in REPORT.md; it biases every ratio against VigilODE). Minors deferred: allocation scope misses mmap/Fortran stack; report test runs on fresh small corpus (committed pair checked by reviewer: 0/2278 missing); HIRES RODAS 2.80 matched near reference gap; duplicate failure blocks in arms.rs; pre-prereg design change flipped vdP dispute (disclosed).
- Ruling: the E1 ratios are interpreted as "VigilODE with step landing on every grid point vs comparators with interpolation" until the facade's dense output (p0-facade) is benchmarked; no solver ranking is admitted — cost: wave-3 must re-run E1 through the merged facade with dense_output = true.
- Task 5: fix round 1 dispatched.
- Ruling: L-0139 REPORT.md hash updated in place instead of a superseding row — L-0139 never reached a base branch (only the unmerged node and integration branches), so the append-only invariant against any base holds — cost if wrong: one extra correction row later.
- Task 5 fix round 1: 94c5b0a; re-review OPEN only on stale report.rs digest in L-0139; controller fixed digest in place (387f87a); Task 5: complete
- Ruling/user-approved (2026-10-21 '수정 허용'): check-research-node skips digest check for (row,path) re-bound by a superseding row — node production/h-checker 4f92ec3; merge into wave2-integration after final review lands
- Ruling: heavy test runs move to owner's workstation via docs/plan/handoff/LOCAL_TEST_HANDOFF_W2.md (user request 07:49); cloud suite b8180ccsv left running as fallback — cost if wrong: duplicate CPU
- Final review (opus): FIXES NEEDED — Important 1 (Enforce disabled under zero error scale), Minor 7. h-checker merged into wave2-integration.
- Ruling: Minor 2 (commit identity) rejected — user chose "그대로 둠"; reviewer's claimed global rule does not override — cost if wrong: none to code.
- Ruling: Minor 6 (stale bench fidelity note) deferred to wave-3 E1 re-run, as reviewer suggested.
- Ruling: one fix dispatch on node production/w2-final-fix covers Important 1 + Minor 1 (correction row L-0140 superseding L-0139), 3, 4, 5, 7. Wave-3 ledger ids start at L-0141.
- Final fix: 8154810, d5285bd, 4c04c8b (DONE_WITH_CONCERNS). Ruling: cargo fmt failure in production_audit_packet_curve.rs pre-existing at 4f92ec3 (verified) — out of scope, candidate for wave-3 T6 CI task. Scoped re-review dispatched.
- Final re-review: ALL ADDRESSED (concern 1 judged no contract break). Cloud suite exposed flaky krylov_workspace_driver_contract zero-allocation tests (global counter, 2/40 parallel fails, file unchanged since before wave 2, missed by H hygiene); controller fixed with thread-local gate, 0/60 — 08fd052.
- Task 6 integration: production/wave2-integration 08fd052 — w2-final-fix + h-checker merged; checker PASS. Full-suite confirmation via local handoff.
- partial cloud suite (stopped, tree 00973c7): 255 binaries, 1283 passed, 2 failed (krylov alloc flake + poison, fixed 08fd052)
