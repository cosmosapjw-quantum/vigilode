# Homotopy Numerical Experiments Cycle 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [x]`) syntax for tracking.

**Goal:** Build and execute the first fixed-schedule nonlinear partial-coupling homotopy screen
against protected sequential RODAS5P and SABR5P.

**Architecture:** Extend `StructuredBlockSystem` with read-only decomposition and exact
certificate operations, then implement a bounded homotopy stepper in `homotopy.rs`. A CLI command
runs deterministic problem/configuration grids and emits numerical and cost ledgers.

**Tech Stack:** Rust 1.94.1, `faer 0.24.4`, existing offline Cargo lock, serde JSON, same Rust
RODAS5P/Krylov kernels.

## Global Constraints

- Do not change the official RODAS5P coefficient snapshot.
- Do not add dependencies.
- Do not add adaptive schedules, Anderson, Chebyshev/Padé, BDF/Radau, or GCRO-DR tuning.
- Keep protected sequential RODAS5P as the transactional fallback.
- Use explicit errors for non-finite values and invalid contracts.
- Develop with test-first red/green cycles.

---

### Task 1: Lock experiment contract and baseline

**Files:**
- Create: `research/homotopy_numerical_v05/harness/SCIENTIFIC_CONTRACT.md`
- Create: `research/homotopy_numerical_v05/harness/VALIDATION_MATRIX.md`
- Create: `research/homotopy_numerical_v05/harness/PLANS.md`
- Create: `research/homotopy_numerical_v05/harness/RUN_STATE.md`

**Interfaces:**
- Consumes: Loop-2 handoff and branch contract.
- Produces: fixed scope and validation gates for all later tasks.

- [x] Record the recovered environment, base SHA, baseline test command, and runtime interruption.
- [x] Commit the design and plan before production code.

### Task 2: Read-only nonlinear decomposition

**Files:**
- Modify: `crates/rodas5p-integrators/src/block.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_numerical_contracts.rs`

**Interfaces:**
- Produces:
  - `NonlinearRemainderSnapshot`
  - `StructuredBlockSystem::nonlinear_remainder_snapshot`
  - `diagonal_apply`, `coupling_apply`, `partial_linear_apply`, `target_residual`

- [x] Write a failing test asserting
  `partial_linear_apply(k,1) == StructuredBlockSystem::raw_apply(k)`.
- [x] Run the focused test and observe the missing API failure.
- [x] Implement the minimum read-only API and make existing `nonlinear_rhs` delegate to it.
- [x] Run focused and integrator regression tests.
- [x] Commit.

### Task 3: Exact nonlinear output certificate

**Files:**
- Modify: `crates/rodas5p-integrators/src/block.rs`
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_numerical_contracts.rs`

**Interfaces:**
- Produces:
  - `NonlinearOutputCertificate`
  - `StructuredBlockSystem::target_jacobian_matrix`
  - `certify_nonlinear_target`

- [x] Write a failing affine-limit test comparing nonlinear and affine certificates.
- [x] Implement the target Jacobian
  \(A_s-h\alpha_{ij}(J_i-J_n)\).
- [x] Reject JVP-only or non-finite certificate inputs explicitly.
- [x] Run focused and regression tests.
- [x] Commit.

### Task 4: Fixed-schedule homotopy path kernel

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_numerical_contracts.rs`

**Interfaces:**
- Produces:
  - `HomotopyPredictor`
  - `HomotopyPathConfig`
  - `HomotopyPathPoint`
  - `HomotopyWorkLedger`
  - `run_fixed_homotopy_path`

- [x] Write failing tests for invalid schedule/depth and `q=7` affine endpoint exactness.
- [x] Implement common-`W` truncated inverse action with explicit work accounting.
- [x] Implement Euler and AB2 predictor plus bounded frozen-linear corrections.
- [x] Run focused and regression tests.
- [x] Commit.

### Task 5: Transactional homotopy step and fallback

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_numerical_contracts.rs`

**Interfaces:**
- Produces:
  - `HomotopyStepReport`
  - `homotopy_step`

- [x] Write a failing test that forces certificate failure and checks sequential fallback parity.
- [x] Implement endpoint certification and StepResult conversion.
- [x] Preserve/restore Krylov state on failure or rejected fallback.
- [x] Run focused and regression tests.
- [x] Commit.

### Task 6: Canonical numerical screen and CLI

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Modify: `crates/rodas5p-cli/src/main.rs`
- Test: `crates/rodas5p-cli/tests/cli_contracts.rs`

**Interfaces:**
- Produces:
  - `HomotopyExperimentReport`
  - `run_homotopy_experiment_screen`
  - CLI `homotopy-experiment-screen --output <json>`

- [x] Write a failing CLI schema test.
- [x] Add affine/scalar-linear, Prothero–Robinson, and manufactured-vector rows.
- [x] Add sequential and SABR controls with the same backend/tolerances.
- [x] Emit failures and fallback rows rather than dropping them.
- [x] Run the canonical screen twice and compare deterministic numerical fields.
- [x] Commit.

### Task 7: Scientific and numerical validation

**Files:**
- Create: `research/homotopy_numerical_v05/results/HOMOTOPY_EXPERIMENT_SCREEN.json`
- Create: `research/homotopy_numerical_v05/reports/HOMOTOPY_NUMERICAL_CYCLE1_REPORT_KO.md`
- Update: validation matrix and run state.

- [x] Run affine exactness and nonnormal kill screens.
- [x] Run Prothero–Robinson stiffness/nonlinearity grid.
- [x] Run manufactured noncommuting-mass/nonnormal grid.
- [x] Check false accepts against exact certificate.
- [x] Record work/round/fallback distributions and compare equal-work SABR.
- [x] Record whether order/stiff-decay gates are ready or remain HOLD.
- [x] Commit.

### Task 8: Independent diff review and closeout

**Files:**
- Create: `research/homotopy_numerical_v05/reviews/INDEPENDENT_DIFF_REVIEW.md`
- Create: `research/homotopy_numerical_v05/closeout/PROMOTE_HOLD.md`
- Create: validation receipts.

- [x] Run format, strict Clippy, workspace tests, release build, CLI smoke, and Git integrity.
- [x] Review base-to-head diff for signs, hidden fallback, false acceptance, and scope creep.
- [x] Fix blocking findings with new failing tests.
- [x] Mark every plan item Done/Blocked/Cancelled.
- [x] Create a durable source ZIP and Git bundle only after exact-HEAD verification.
