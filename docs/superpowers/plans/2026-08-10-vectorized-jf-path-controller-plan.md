# Vectorized/Jacobian-Free Homotopy Path Controller Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic nonstationary homotopy schedule engine and rejection-telemetry screen that identifies whether low-depth schedules can reduce the original RODAS5P target defect within four rounds.

**Architecture:** Preserve all existing fixed homotopy APIs. Add validated schedule types and a schedule runner that reuses the existing partial-path kernels, returns partial outcomes on numerical failure, and feeds a standalone research screen/CLI. Frozen BDF/Radau files remain untouched.

**Tech Stack:** Rust 1.94.1, faer 0.24.4, serde/serde_json, existing pure-Rust RODAS5P workspace, Python/matplotlib only for post-run analysis and plots.

## Global Constraints

- No changes to `bdf.rs` or `radau.rs`.
- No runtime dispatcher activation.
- No explicit-J-free performance claim from this schedule-isolation cycle.
- Maximum schedule depth is four rounds in the research screen.
- Existing fixed-path behavior must remain unchanged.
- Numerical failures preserve partial telemetry and attempted work.

---

### Task 1: Validated schedule contract

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_path_controller_contracts.rs`

**Interfaces:**
- Produces: `HomotopyRoundSpec::new(...)`, `HomotopyScheduleConfig::new(...)`.

- [ ] Write failing tests for nonmonotone lambda, missing endpoint 1, invalid q/theta/damping, and a valid three-round schedule.
- [ ] Run focused tests and confirm failure because the types are absent.
- [ ] Implement the minimum validated types and getters.
- [ ] Re-run focused tests and commit.

### Task 2: Scheduled path engine with partial failure

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_path_controller_contracts.rs`

**Interfaces:**
- Produces: `ScheduledHomotopyPathReport`, `ScheduledHomotopyRoundPoint`, `run_scheduled_homotopy_path`.

- [ ] Write a failing fixed-schedule equivalence test against `run_fixed_homotopy_path`.
- [ ] Write a failing hostile-case test requiring partial work/points after nonfinite failure.
- [ ] Implement the schedule engine using existing path kernels and observer hooks.
- [ ] Verify exact/roundoff equivalence and partial failure preservation.
- [ ] Commit.

### Task 3: Deterministic path-controller research screen

**Files:**
- Create: `crates/rodas5p-integrators/src/path_controller.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_path_controller_contracts.rs`

**Interfaces:**
- Produces: `PathControllerProfile`, `PathControllerReport`, `run_path_controller_screen`.

- [ ] Write a failing smoke-report determinism test.
- [ ] Implement bounded cases, schedules, sequential oracle, original-target certificate, work and failure rows.
- [ ] Add checksums excluding timing fields.
- [ ] Run focused tests and commit.

### Task 4: CLI and schema contract

**Files:**
- Modify: `crates/rodas5p-cli/src/main.rs`
- Modify: `crates/rodas5p-cli/tests/cli_contracts.rs`

**Interfaces:**
- Produces: `rodas5p homotopy-path-controller --profile smoke|canonical --output <path>`.

- [ ] Write a failing CLI contract test.
- [ ] Add the command and JSON writer path.
- [ ] Run CLI tests and deterministic smoke twice.
- [ ] Commit.

### Task 5: Canonical experiment, analysis, plots, and audits

**Files:**
- Create: `research/path_controller_v017/scripts/analyze_path_controller.py`
- Create: `research/path_controller_v017/results/*`
- Create: `research/path_controller_v017/plots/*`
- Create: `research/path_controller_v017/reports/PATH_CONTROLLER_CYCLE_REPORT_KO.md`
- Create: `research/path_controller_v017/reports/PHYS_MATH_AUDIT.md`
- Create: `research/path_controller_v017/reports/PHYS_MATH_CODE_AUDIT.md`
- Create: `research/path_controller_v017/reports/PLOT_CRAG_AUDIT.md`
- Create: `research/path_controller_v017/receipts/BDF_RADAU_FROZEN_DIFF.txt`

- [ ] Run canonical screen twice and verify scientific identity.
- [ ] Analyze residual reduction, false acceptance, work, rounds, and failures.
- [ ] Generate and inspect at least three plots.
- [ ] Apply only evidence-supported micro-fixes, with tests first.
- [ ] Write audits and next-step handoff.
- [ ] Commit.

### Task 6: Final verification and durable delivery

**Files:**
- Create: `research/path_controller_v017/validation/FINAL_VALIDATION_EXACT_HEAD.log`
- Create: delivery ZIP, source ZIP, Git bundle, and SHA-256 ledger under `/mnt/data`.

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run strict Clippy under the measurement profile.
- [ ] Run full workspace/doc tests under the measurement profile.
- [ ] Build the measurement binary.
- [ ] Verify BDF/Radau frozen diff, clean tree, git fsck, clean-source rebuild, JSON/ZIP integrity.
- [ ] Tag the checkpoint and prepare exactly one successor branch.
- [ ] Commit closeout metadata.
