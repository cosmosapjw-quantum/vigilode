# Unified RODAS5P Candidate Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and validate one Rust experiment harness that applies identical output certification, scientific gates, work accounting, threading, and failure semantics to every currently implemented sequential, Krylov, SABR, and homotopy candidate.

**Architecture:** Add a candidate registry and a solver-independent execution/result layer in `rodas5p-integrators`. Add second-correction and refined-root reference certification beside the existing first-correction oracle. Run independent cases through a local Rayon pool, canonicalize results, and expose one CLI screen plus machine-readable reports.

**Tech Stack:** Rust 1.94.1, faer 0.24.4, Rayon 1.12.0, serde/serde_json, existing offline vendored dependencies.

## Global Constraints

- Preserve official RODAS5P coefficients and existing protected sequential behavior.
- No new production dependency.
- All tests are written and observed failing before implementation.
- Research reference certification may be expensive but may not be labelled a fast certificate.
- No candidate may silently turn numerical failure into success.
- One-thread and four-thread scientific outputs must be identical after canonical sorting.
- BDF, Radau/IRK, Peer/W, SDC, ROK/BOROK, and Leja remain visible but deferred until implemented.

---

### Task 1: Candidate registry and common identifiers

**Files:**
- Create: `crates/rodas5p-integrators/src/candidates.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/unified_candidate_contracts.rs`

**Interfaces:**
- Produces: `CandidateFamily`, `CandidateStatus`, `CandidateSpec`, `CandidateCatalog`.
- Candidate IDs must be stable and include all algorithm-defining parameters.

- [ ] Write a failing test requiring executable sequential, SABR, and homotopy entries plus deferred prior-art entries.
- [ ] Run the focused test and confirm the missing-module failure.
- [ ] Implement the minimal registry and stable IDs.
- [ ] Run focused and integrator tests.
- [ ] Commit `feat: add unified candidate registry`.

### Task 2: Unified certification diagnostics

**Files:**
- Create: `crates/rodas5p-integrators/src/certification.rs`
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/unified_candidate_contracts.rs`

**Interfaces:**
- Produces: `CertificationMode`, `CorrectionDiagnostic`, `RefinedRootCertificate`, `certify_second_correction`, `refine_target_root`.
- Consumes: `StructuredBlockSystem`, candidate stages, WRMS tolerances, `WorkCounters`.

- [ ] Write a failing test where the first correction underestimates a frozen nonlinear corpus case and the second-correction diagnostic records residual/output ratios.
- [ ] Run the focused test and verify the expected failure.
- [ ] Implement fixed-Jacobian second correction with nonfinite/growth guards.
- [ ] Add a failing test for refined-root convergence and transactional failure.
- [ ] Implement refreshed-Jacobian safeguarded refinement with strict residual/correction stopping.
- [ ] Run focused and full integrator tests.
- [ ] Commit `feat: add nonlinear output certification references`.

### Task 3: Solver-independent candidate result contract

**Files:**
- Create: `crates/rodas5p-integrators/src/unified_screen.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/unified_candidate_contracts.rs`

**Interfaces:**
- Produces: `UnifiedCandidateRow`, `UnifiedCandidateOutcome`, `UnifiedCandidateScreen`.
- Every row stores oracle defect, C1/C2/C3 diagnostics, fallback-inclusive counters, batch depth, and compute time.

- [ ] Write a failing serialization/contract test for required scientific and work fields.
- [ ] Implement the result schema and canonical sort key.
- [ ] Add tests that numerical failure and uncertified states cannot serialize as accepted.
- [ ] Run focused tests.
- [ ] Commit `feat: add unified candidate result contract`.

### Task 4: Executable candidate adapters

**Files:**
- Modify: `crates/rodas5p-integrators/src/unified_screen.rs`
- Modify: `crates/rodas5p-integrators/src/integrate.rs`
- Test: `crates/rodas5p-integrators/tests/unified_candidate_contracts.rs`

**Interfaces:**
- Consumes: `CandidateSpec`, `OdeProblem`, one-step case, tolerances.
- Produces: one `UnifiedCandidateRow` per executable candidate.

- [ ] Write failing tests for sequential/direct, sequential/GMRES/LGMRES/GCRO-DR, SABR block variants, and homotopy variants consuming the same case/oracle ID.
- [ ] Implement Tier-L sequential adapters.
- [ ] Implement Tier-N SABR adapters with fixed direct common-W controls.
- [ ] Implement registered homotopy adapters and protected fallback accounting.
- [ ] Verify all adapters use the same protected sequential/direct oracle.
- [ ] Commit `feat: execute all current candidates under one oracle`.

### Task 5: Deterministic scenario and Rayon runner

**Files:**
- Modify: `crates/rodas5p-integrators/src/unified_screen.rs`
- Test: `crates/rodas5p-integrators/tests/unified_candidate_contracts.rs`

**Interfaces:**
- Produces: `UnifiedScreenProfile`, `UnifiedScreenConfig`, `run_unified_candidate_screen`.

- [ ] Write a failing one-thread/four-thread identity test.
- [ ] Implement case-level local Rayon execution.
- [ ] Canonically sort rows and separate compute from serialization timing.
- [ ] Verify deterministic checksums and no nested stage parallelism.
- [ ] Commit `feat: add deterministic parallel unified screen`.

### Task 6: Global scientific gates

**Files:**
- Modify: `crates/rodas5p-integrators/src/unified_screen.rs`
- Test: `crates/rodas5p-integrators/tests/unified_candidate_contracts.rs`

**Interfaces:**
- Produces: `CandidateGateReport`, global-order/stiff/nonnormal gate summaries.

- [ ] Write failing tests for protected fifth order and a deliberately low-order homotopy configuration.
- [ ] Implement fixed-step order estimation before roundoff saturation.
- [ ] Add Prothero–Robinson stiff and noncommuting-mass/nonnormal gates.
- [ ] Require zero oracle false accepts for promotion.
- [ ] Commit `feat: add common scientific promotion gates`.

### Task 7: CLI and compact/full evidence modes

**Files:**
- Modify: `crates/rodas5p-cli/src/main.rs`
- Modify: `crates/rodas5p-cli/tests/cli_contracts.rs`

**Interfaces:**
- Produces CLI subcommand `unified-candidate-screen` with smoke/canonical profile, thread count, compact/full output modes.

- [ ] Write a failing CLI contract test.
- [ ] Implement command and deterministic JSON output.
- [ ] Verify compact summary and full evidence have matching scientific checksums.
- [ ] Commit `feat: expose unified candidate screen CLI`.

### Task 8: Execute coding-harness Phase 6–8 validation

**Files:**
- Create/update: `research/unified_candidate_v08/phases/PHASE06_SOFTWARE_VALIDATION.md`
- Create/update: `research/unified_candidate_v08/phases/PHASE07_SCIENTIFIC_VALIDATION.md`
- Create/update: `research/unified_candidate_v08/phases/PHASE08_NUMERICAL_REPRODUCIBILITY.md`
- Create: `research/unified_candidate_v08/results/*`

- [ ] Run fmt, strict Clippy, focused tests, workspace tests, and release build.
- [ ] Run smoke and canonical unified screens at one and four threads.
- [ ] Compare scientific checksums and record timing/work tables.
- [ ] Run calibration/holdout and global-order gates.
- [ ] Record failures and blockers without suppression.
- [ ] Commit `research: validate unified candidate screen`.

### Task 9: Independent diff review

**Files:**
- Create: `research/unified_candidate_v08/reviews/PHASE09_INDEPENDENT_DIFF_REVIEW.md`

- [ ] Review the full diff against the design and plan.
- [ ] Check false-accept attribution, fallback accounting, candidate completeness, and thread determinism.
- [ ] Fix all critical/important findings through test-first changes.
- [ ] Re-run affected validation.
- [ ] Commit `review: close unified candidate findings`.

### Task 10: Promote/Hold closeout and next handoff

**Files:**
- Create: `research/unified_candidate_v08/closeout/PHASE10_PROMOTE_HOLD.md`
- Create: `research/unified_candidate_v08/reports/UNIFIED_CANDIDATE_CYCLE_REPORT_KO.md`
- Create: `research/unified_candidate_v08/handoff/NEXT_CYCLE_PLAN_KO.md`
- Create: `research/unified_candidate_v08/results/RESULT_SUMMARY.json`

- [ ] Apply gates without weakening tolerances after seeing results.
- [ ] Separate PROMOTE/HOLD/REJECT/DEFERRED candidates.
- [ ] Document whether C2/C3 resolves the current false-acceptance blocker and its cost.
- [ ] Define the next joint experiment matrix including deferred candidates only after implementation.
- [ ] Commit `chore: close unified candidate research cycle`.

### Task 11: Final exact-HEAD validation and delivery

**Files:**
- Create: delivery ZIP, Git bundle, release binary, SHA-256 ledger, exact validation log.

- [ ] Run full exact-HEAD validation in the worktree.
- [ ] Rebuild and test from a clean Git archive.
- [ ] Verify dynamic dependencies and Git integrity.
- [ ] Create annotated tag and next branch only after all checks.
- [ ] Create source ZIP, complete bundle, evidence archive, manifest, and checksums.
