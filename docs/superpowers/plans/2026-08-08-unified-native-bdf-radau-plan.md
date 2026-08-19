# Unified Native BDF/Radau Anchors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement fixed-step BDF1/BDF2 and Radau IIA1/3 anchors on a shared Rust nonlinear/work-accounting layer and connect them to the candidate registry and global scientific gates.

**Architecture:** Add a dense full-Newton reference engine and dense-Jacobian adapter, then build multistep and collocation modules on it. Keep the existing RODAS one-step screen unchanged and add a complete-integrator native gate.

**Tech Stack:** Rust 1.94.1, faer 0.24.4, serde, existing pure-Rust workspace, offline vendored crates.

## Global Constraints

- Preserve official RODAS5P coefficients and existing public behavior.
- Fixed-step anchors only; no variable-order/adaptive production claims.
- Nonsingular mass matrices only.
- TDD red-green-refactor for each task.
- No new production dependency.
- Every nonlinear residual/Jacobian/factorization is counted.

---

### Task 1: Nonlinear work contract and dense Jacobian

**Files:**
- Modify: `crates/rodas5p-core/src/work.rs`
- Modify: `crates/rodas5p-integrators/src/problem.rs`
- Create: `crates/rodas5p-integrators/src/nonlinear.rs`
- Test: `crates/rodas5p-integrators/tests/native_implicit_contracts.rs`

**Interfaces:**
- Produces `NewtonConfig`, `NewtonReport`, `solve_dense_newton`, `OdeProblem::dense_jacobian`.

- [ ] Write a failing test that solves `x^2-2=0`, verifies convergence and exact nonlinear counters.
- [ ] Run `cargo test -p rodas5p-integrators --test native_implicit_contracts dense_newton --offline` and verify failure due to missing API.
- [ ] Add `nonlinear_solves`, `nonlinear_iterations`, `nonlinear_residual_evaluations`, `nonlinear_jacobian_evaluations`, and `nonlinear_failures` to `WorkCounters` and `delta`.
- [ ] Implement dense-Jacobian materialization and full Newton with finite/residual checks.
- [ ] Re-run the targeted test and full core/integrator tests.
- [ ] Commit `feat: add common dense nonlinear solve contract`.

### Task 2: BDF1/BDF2 fixed-step anchors

**Files:**
- Create: `crates/rodas5p-integrators/src/bdf.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/native_implicit_contracts.rs`

**Interfaces:**
- Produces `BdfOrder`, `BdfConfig`, `BdfHistory`, `BdfStepReport`, `bdf_step`, `integrate_bdf_fixed`.

- [ ] Write failing tests for BDF1 scalar amplification, BDF2 startup/history rollback, orders 1/2, and noncommuting mass affine accuracy.
- [ ] Run targeted tests and verify missing API failures.
- [ ] Implement mass-matrix residual/Jacobian and transactional history.
- [ ] Run targeted and workspace regression tests.
- [ ] Commit `feat: add fixed-step BDF1 and BDF2 anchors`.

### Task 3: Radau IIA one-/three-stage anchors

**Files:**
- Create: `crates/rodas5p-integrators/src/radau.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/native_implicit_contracts.rs`

**Interfaces:**
- Produces `RadauIiaStages`, `RadauConfig`, `RadauStepReport`, `radau_step`, `integrate_radau_fixed`, `radau_iia3_tableau`.

- [ ] Write failing exact-tableau, Radau1=BDF1, order-5, stiff-damping and mass-matrix tests.
- [ ] Run tests and verify missing API failures.
- [ ] Implement exact coefficients, coupled residual/Jacobian and full Newton.
- [ ] Run targeted and regression tests.
- [ ] Commit `feat: add native Radau IIA anchors`.

### Task 4: Candidate registry and complete-integrator gates

**Files:**
- Modify: `crates/rodas5p-integrators/src/candidates.rs`
- Create: `crates/rodas5p-integrators/src/native_gates.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/native_candidate_contracts.rs`

**Interfaces:**
- Adds executable candidate variants and `run_native_integrator_gates` report.

- [ ] Write failing tests that require four executable native anchors and retained deferred production families.
- [ ] Implement catalog variants and native gate dispatch.
- [ ] Add deterministic global order/stiff/mass result rows and checksum-friendly stable ordering.
- [ ] Run targeted and full tests.
- [ ] Commit `feat: register native complete-integrator anchors`.

### Task 5: Harness validation, scientific report and delivery

**Files:**
- Update: `research/unified_native_v09/coding_harness/*`
- Create: `research/unified_native_v09/coding_harness/research/phases/PHASE05_*.md` through `PHASE10_*.md`
- Create: `research/unified_native_v09/reports/UNIFIED_NATIVE_V09_REPORT_KO.md`

- [ ] Run format, strict Clippy, release tests and release build with the pinned offline toolchain.
- [ ] Run native gates twice and compare scientific serialization.
- [ ] Perform an independent diff review; fix all critical/important findings with regression tests.
- [ ] Validate both harnesses and Git integrity.
- [ ] Commit closeout and create source ZIP, Git bundle, result archive and SHA-256 ledger.
