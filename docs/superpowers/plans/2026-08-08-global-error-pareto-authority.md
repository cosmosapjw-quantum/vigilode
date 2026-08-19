# Global-Error Pareto Authority Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans task-by-task.

**Goal:** Build an exact-reference, common-grid, vector-cost Pareto authority and replay the five fixed-step native anchors without changing solver algorithms.

**Architecture:** Add a new comparison module to `rodas5p-fair-ab`, use existing integrator APIs, expose one CLI command, then produce deterministic one-/four-thread artifacts and independent review.

**Tech Stack:** Rust 1.94.1, faer 0.24.4, serde, rand_pcg, Rayon 1.12, existing offline workspace.

## Global constraints

- No solver algorithm changes.
- No new dependency.
- No interpolation in P0.
- Exact analytic references only.
- One-thread wall time is authoritative; four-thread timing is throughput only.
- Full failures and work counters retained.

### Task 1: Error, reference, and Pareto contracts

**Files:**
- Create `crates/rodas5p-fair-ab/src/global_error.rs`
- Modify `crates/rodas5p-fair-ab/src/lib.rs`
- Create `crates/rodas5p-fair-ab/tests/global_error_contracts.rs`

- [ ] Write failing tests for zero/known error metrics, missing-grid rejection, Pareto dominance, and target attainment.
- [ ] Run targeted tests and record RED output.
- [ ] Implement minimal validated contracts and algorithms.
- [ ] Run targeted tests and record GREEN output.
- [ ] Commit.

### Task 2: Fixed-anchor execution and work reports

- [ ] Write failing tests requiring five candidates, exact provenance, BDF2 startup accounting, and failure preservation.
- [ ] Implement fixed candidate dispatch through existing sequential/BDF/Radau APIs.
- [ ] Add nested exact-solution corpus and common grids.
- [ ] Run targeted and regression tests.
- [ ] Commit.

### Task 3: Deterministic screen and Rayon

- [ ] Write failing one-/four-thread scientific checksum identity test.
- [ ] Implement local Rayon case/task execution, canonical sort, timing exclusion from scientific checksum, and T1 randomized repeated timing.
- [ ] Verify deterministic rerun and thread identity.
- [ ] Commit.

### Task 4: CLI and artifacts

- [ ] Write failing CLI contract for `global-error-pareto`.
- [ ] Implement smoke/canonical profile, thread count, and JSON output.
- [ ] Run canonical T1/T4 screens and analysis.
- [ ] Commit.

### Task 5: Validation, review, and closeout

- [ ] Run fmt, strict release Clippy, release tests/doc tests, release build, CLI validation, checksum identity, Git checks, harness validators.
- [ ] Perform independent base-to-head review and fix critical/important findings with tests.
- [ ] Write report, result summary, blocker disposition, next-cycle handoff.
- [ ] Tag, branch, source ZIP, Git bundle, binary, SHA ledger, delivery ZIP.
