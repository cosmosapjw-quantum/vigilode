# Homotopy Coding Research Loop 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build and validate a research-only Rust oracle for the partial-coupling RODAS5P homotopy, then prepare a clean handoff to the actual numerical-experiment branch.

**Architecture:** Add one isolated `homotopy` module to `rodas5p-integrators`, reuse the existing dense matrix and LU contracts, and expose one deterministic CLI design check. The patch does not alter the protected sequential/SABR algorithms and does not implement adaptive continuation.

**Tech Stack:** Rust 1.94.1, `faer 0.24.4`, existing workspace crates, Serde JSON, offline locked Cargo.

## Global Constraints

- No new dependency.
- No change to official RODAS5P coefficients or sequential/SABR acceptance semantics.
- No adaptive homotopy solver, Anderson acceleration, Chebyshev/Padé, or performance benchmark.
- All feature code follows test-first Red–Green–Refactor.
- All success is certified by the original affine target residual, not a homotopy residual alone.
- All builds use `--offline --locked`.

---

### Task 1: Freeze the coding-harness contract

**Files:**
- Create: `research/homotopy_loop2/harness/SCIENTIFIC_CONTRACT.md`
- Create: `research/homotopy_loop2/harness/VALIDATION_MATRIX.md`
- Create: `research/homotopy_loop2/harness/PLANS.md`
- Create: `research/homotopy_loop2/harness/RUN_STATE.md`
- Create: `research/homotopy_loop2/harness/DECISION_LOG.md`
- Create: `research/homotopy_loop2/harness/FAILURE_LOG.md`

- [x] Record the exact in-scope/out-of-scope boundary and preserved behavior.
- [x] Record baseline test evidence and toolchain/backend versions.
- [x] Define software, scientific, numerical, reproducibility, and review gates.
- [x] Commit with `docs: lock homotopy coding loop 2 contract`.

### Task 2: Define the partial-coupling oracle API by failing tests

**Files:**
- Create: `crates/rodas5p-integrators/tests/homotopy_research_contracts.rs`
- Create: `crates/rodas5p-integrators/src/homotopy.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`

**Interfaces:**
- Produces `PartialCouplingParameters::new(theta, lambda)`.
- Produces `AffinePartialCouplingOracle::new(mass, jacobian, beta, gamma, h, rhs_rows, weights)`.
- Produces `path_operator`, `solve_path`, `target_residual`, `homotopy_residual`.

- [x] Write tests for parameter validation and dimension rejection.
- [x] Run the focused test and verify RED due to missing API.
- [x] Implement only validation and explicit `D`, `C`, path/target operator construction.
- [x] Run the focused test and verify GREEN.
- [x] Commit with `feat: add affine partial-coupling homotopy oracle`.

### Task 3: Implement exact and truncated nilpotent inverse actions

**Files:**
- Modify: `crates/rodas5p-integrators/tests/homotopy_research_contracts.rs`
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`

**Interfaces:**
- Produces `truncated_inverse_apply(parameters, q, rhs)`.
- Produces `normalized_coupling_apply(parameters, vector)`.

- [x] Write a noncommuting `M,J` affine test where `q=7` must match a direct path solve.
- [x] Verify RED because inverse actions do not exist.
- [x] Implement `D^{-1}` block solves and the finite sum `sum_{m=0}^q Q^m D^{-1}r`.
- [x] Verify `q=7` exactness and explicit rejection of `q>=8`.
- [x] Add a canonical nearest-neighbor chain test for low-`q` error exposure.
- [x] Commit with `feat: add truncated nilpotent homotopy inverse`.

### Task 4: Add original-output certification

**Files:**
- Modify: `crates/rodas5p-integrators/tests/homotopy_research_contracts.rs`
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`

**Interfaces:**
- Produces `AffineOutputCertificate { residual_norm, relative_residual, output_wrms, correction_norm }`.
- Produces `certify_target(stages, scale)`.

- [x] Write a test that perturbs an exact endpoint and compares certificate output with exact target correction.
- [x] Verify RED.
- [x] Implement target residual solve and weighted RODAS-output contraction.
- [x] Verify GREEN and non-finite/shape error paths.
- [x] Commit with `feat: add affine original-output certificate`.

### Task 5: Add deterministic structural screens and CLI

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Modify: `crates/rodas5p-cli/src/main.rs`
- Modify: `crates/rodas5p-cli/tests/cli_contracts.rs`

**Interfaces:**
- Produces `HomotopyDesignCheckReport`.
- Adds `rodas5p homotopy-design-check --output <path>`.

- [x] Write CLI RED test for schema and required screen fields.
- [x] Implement official `L^m` norm screen, affine endpoint screen, `q` screen, and nonnormal `cond_1` screen.
- [x] Verify deterministic byte-identical JSON across two runs.
- [x] Commit with `feat: add homotopy design-check CLI`.

### Task 6: Software and scientific validation

**Files:**
- Update: `research/homotopy_loop2/harness/VALIDATION_MATRIX.md`
- Create: `research/homotopy_loop2/validation/SOFTWARE_VALIDATION.md`
- Create: `research/homotopy_loop2/validation/SCIENTIFIC_VALIDATION.md`

- [x] Run focused tests.
- [x] Run full release workspace tests.
- [x] Run `cargo fmt --check` and strict Clippy.
- [x] Run CLI twice and compare SHA-256.
- [x] Audit dimensions, signs, endpoint identities, affine limit, nonnormal counterexample, and protected-method non-regression.
- [x] Commit with `test: validate homotopy research oracle`.

### Task 7: Independent diff review and closeout

**Files:**
- Create: `research/homotopy_loop2/reviews/INDEPENDENT_DIFF_REVIEW.md`
- Create: `research/homotopy_loop2/closeout/PROMOTE_HOLD.md`
- Create: `research/homotopy_loop2/reports/HOMOTOPY_CODING_LOOP2_REPORT_KO.md`
- Create: `research/homotopy_loop2/handoff/NUMERICAL_EXPERIMENT_BRANCH_HANDOFF_KO.md`

- [x] Review the base-to-head diff for correctness, hidden scope expansion, numerical instability, missing tests, and error swallowing.
- [x] Resolve blocking findings or mark the loop HOLD.
- [x] State precisely which API is ready for the next branch and which hypotheses remain untested.
- [x] Create branch `homotopy-numerical-experiments-v0.5` at the validated head without adding experiment code.
- [x] Commit with `docs: close homotopy coding research loop 2`.

### Task 8: Final verification and delivery

- [x] Run exact-HEAD `fmt`, Clippy, release tests, release build, CLI smoke, and Git integrity checks.
- [x] Build source ZIP, complete Git bundle, report package, SHA-256 ledger, and manifest.
- [x] Preserve any failed validation transcripts.
