# Global-Error Pareto Authority Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a method-independent global-error/total-cost authority and a deterministic fixed-step anchor replay without changing numerical solvers.

**Architecture:** Add a focused `global_error` module to `rodas5p-fair-ab`, expose a CLI command, and evaluate independent run specifications with the existing local Rayon pool. All error scales derive from immutable references, and all Pareto fronts are per-cost rather than scalarized.

**Tech Stack:** Rust 1.94.1, `faer` 0.24.4, Rayon 1.12, Serde/JSON, existing Rust-only solver workspace.

## Global Constraints

- No solver algorithm source changes.
- No new production dependency.
- Analytic references only in v0.10 replay.
- Exact landing on common output times; no interpolation.
- Preserve all failures and counters.
- Canonical deterministic serialization.

---

### Task 1: Error and reference contracts

**Files:**
- Create: `crates/rodas5p-fair-ab/src/global_error.rs`
- Modify: `crates/rodas5p-fair-ab/src/lib.rs`
- Test: `crates/rodas5p-fair-ab/tests/global_error_contracts.rs`

- [ ] Write failing tests for target validation, fixed reference weights, uncertainty addition, and output-grid validation.
- [ ] Run focused tests and confirm missing APIs fail.
- [ ] Implement minimal reference, grid, target, and error-report types.
- [ ] Run focused and existing fair-ab tests.
- [ ] Commit.

### Task 2: Work reports and Pareto dominance

**Files:**
- Modify: `crates/rodas5p-fair-ab/src/global_error.rs`
- Test: `crates/rodas5p-fair-ab/tests/global_error_contracts.rs`

- [ ] Write failing hand-worked Pareto and failure-retention tests.
- [ ] Implement work/timing reports, cost metrics, fronts, and target-matched selection.
- [ ] Verify fronts never include failed/nonfinite rows.
- [ ] Commit.

### Task 3: Fixed-step anchor replay

**Files:**
- Modify: `crates/rodas5p-fair-ab/src/global_error.rs`
- Test: `crates/rodas5p-fair-ab/tests/global_error_contracts.rs`

- [ ] Write failing replay tests for common IDs, deterministic rows, and expected order hierarchy.
- [ ] Implement immutable analytic corpus and adapters for unchanged fixed-step candidates.
- [ ] Use exact output-time lookup and preserve solver failures.
- [ ] Run smoke replay with 1 and 4 threads and compare scientific fields.
- [ ] Commit.

### Task 4: CLI and artifacts

**Files:**
- Modify: `crates/rodas5p-cli/src/main.rs`
- Modify: `crates/rodas5p-cli/tests/cli_contracts.rs`

- [ ] Write a failing CLI contract test.
- [ ] Add `global-error-pareto --profile smoke|canonical --threads N --output PATH`.
- [ ] Generate deterministic JSON and verify schema.
- [ ] Commit.

### Task 5: Validation and research closeout

**Files:**
- Create/update: `research/global_error_pareto_v10/**`

- [ ] Run fmt, strict Clippy, focused tests, full split tests, release build, 1/4-thread identity, dynamic dependency audit, and Git integrity checks.
- [ ] Perform independent diff review against `unified-native-v0.9.0-alpha1`.
- [ ] Record PROMOTE/HOLD decisions and next solver implementation gate.
- [ ] Package source, Git bundle, results, logs, and SHA-256 ledger.
