# Persistence-Budget Policy Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Test whether R-JF-only sequential confirmation plus a hard speculative-prefix budget can recover regime-transition information without reintroducing P1-01 unsafe full continuation or tail-overhead failure.

**Architecture:** Protected R-JF remains the only committed trajectory. A new research-only screening layer consumes only already-committed R-JF telemetry to form point evidence and persistence/CUSUM candidates. E-K prefix/full-step data are computed only as shadow audit labels; runtime policy in this node never commits or continues E-K. Calibration replay (N=128) and cross-dimension regression (N=512) are separate R-JF-only CLI runs; N=512 is not called sealed because it was already inspected in the preceding math loop.

**Tech Stack:** Rust 1.94.1, existing matrix-free RODAS5P/pexprb54s4 research kernels, serde/CLI, Python analysis.

## Global Constraints

- Parent commit exactly `3726a98491728bd762fb6804c9127d799accdbe4`.
- No BDF/Radau modification.
- No active method switching; R-JF is the sole committed trajectory.
- Event features may use only committed/past R-JF data.
- Audit oracle may use E-K shadow labels, but policy decisions may not.
- Runtime speculative ledger counts only selected prefix probes; full E shadow is audit-only and excluded from proposed runtime overhead.
- Candidate family is frozen before calibration output.
- Calibration N=128; N=512 is cross-dimension regression only; no N=512-driven retuning.
- Same external R-JF trajectory and complete work accounting.

---

### Task 1: Freeze the research contract
**Files:** create `research/generic_policy_redesign_v25/contracts/P1_01R_PERSISTENCE_BUDGET_CONTRACT.json`.
- [ ] Specify information firewall, candidate family, promotion gates, and stop rules.
- [ ] Commit before any calibration solver output.

### Task 2: Add split-dimension atlas execution and R-only feature rows
**Files:** modify `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs`; tests in `crates/rodas5p-integrators/tests/g4_s5b0_regime_atlas_contracts.rs`.
- [ ] Write failing tests for N=128/N=512 split profiles and R-JF trajectory parity.
- [ ] Implement minimal profile split without changing canonical profile semantics.
- [ ] Verify RED→GREEN and old canonical/smoke tests.

### Task 3: Add read-only persistence/CUSUM policy screener
**Files:** create `crates/rodas5p-integrators/src/policy_redesign_v25.rs`; export from `lib.rs`; test `policy_redesign_v25_contracts.rs`.
- [ ] Write failing tests for information-firewall types, k-consecutive semantics, CUSUM update, token-budget invariant, and no E continuation action.
- [ ] Implement minimal deterministic screener.
- [ ] Verify unit tests and legacy tests.

### Task 4: Add research CLI and freeze implementation
**Files:** modify CLI command/test; research directory.
- [ ] Add calibration/holdout CLI profile producing rows and audit labels.
- [ ] Verify CLI schema and no active switching.
- [ ] Commit implementation before canonical calibration output.

### Task 5: Run R-only calibration replay and seal one policy
- [ ] Run N=128 campaign only.
- [ ] Analyze predeclared candidate grid; select by frozen lexicographic criteria.
- [ ] Commit selected policy and calibration raw SHA before holdout.

### Task 6: Run N=512 cross-dimension regression, dual audit, and CRAG
- [ ] Run N=512 exactly with frozen policy.
- [ ] Verify R-JF identity, pathwise prefix budget, event recall/precision, opportunity retention, and no policy E continuation.
- [ ] Generate plots and adversarial mutations.
- [ ] Decide PROMOTE/HOLD for staged safety-certificate node; do not open active switching.
