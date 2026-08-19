# Homotopy Order-Aware Policy and Rayon Screen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add validated order-aware output budgets, deterministic Rayon execution, calibration/
holdout replay, and fixed-step scientific screens without changing the homotopy path kernel.

**Architecture:** `homotopy_policy.rs` owns dimensionless budget semantics. The existing
`HomotopyStepConfig` delegates acceptance-budget calculation to it. `homotopy_order_policy.rs`
owns policy grids, deterministic split/ranking, Rayon scheduling, and trajectory screens. The CLI
constructs a local thread pool through a `threads` argument and emits stable JSON.

**Tech Stack:** Rust 1.94.1, faer 0.24.4, rayon 1.12.0, serde/serde_json, offline vendored crates.

## Global Constraints

- Preserve the protected sequential RODAS5P and SABR implementations.
- Preserve the exact nonlinear target certificate and transactional fallback.
- Use one Rust binary and one `faer` backend for every comparison arm.
- Do not use nightly portable SIMD or add unsafe architecture intrinsics.
- Sort parallel results before serialization.
- Use TDD for every production change.

---

### Task 1: Validated output-budget policies

**Files:**
- Create: `crates/rodas5p-integrators/src/homotopy_policy.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Modify: `crates/rodas5p-integrators/src/homotopy.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_policy_contracts.rs`

**Interfaces:**
- Produces: `OutputBudgetPolicy`, `OutputBudgetDecision`,
  `HomotopyStepConfig::with_policy(path, policy)`.

- [ ] Write tests for all four budget formulas and invalid/nonfinite parameters.
- [ ] Run focused tests and verify RED.
- [ ] Implement the minimal policy API.
- [ ] Replace the hard-coded absolute budget lookup in `homotopy_step` with the policy decision.
- [ ] Run focused and existing homotopy tests.
- [ ] Commit `feat: add order-aware homotopy output budgets`.

### Task 2: Deterministic local Rayon backend

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/rodas5p-integrators/Cargo.toml`
- Create: `crates/rodas5p-integrators/src/parallel.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_policy_contracts.rs`

**Interfaces:**
- Produces: `ParallelExecution::sequential()` and `ParallelExecution::rayon(threads)` with
  `install` and stable collection helpers.

- [ ] Write a test that one-thread and four-thread execution return the same sorted keys and data.
- [ ] Run focused test and verify RED.
- [ ] Add vendored `rayon = 1.12.0` and implement a local pool.
- [ ] Run focused and workspace tests.
- [ ] Commit `feat: add deterministic local rayon execution`.

### Task 3: Calibration/holdout policy replay

**Files:**
- Create: `crates/rodas5p-integrators/src/homotopy_order_policy.rs`
- Modify: `crates/rodas5p-integrators/src/homotopy_experiments.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_policy_contracts.rs`

**Interfaces:**
- Produces: `run_homotopy_order_policy_screen(profile, threads)` and serializable replay rows,
  calibration rankings, holdout summaries.

- [ ] Write tests for deterministic disjoint split and zero-false-accept ranking precedence.
- [ ] Verify RED.
- [ ] Expose only the required cycle-1 case/config helpers as `pub(crate)`.
- [ ] Implement policy grid, replay, low-depth/work metrics, stable sorting, and family winners.
- [ ] Verify one-thread/four-thread equality excluding timing fields.
- [ ] Commit `feat: add calibration holdout policy replay`.

### Task 4: Global order, stiff, and nonnormal trajectory screens

**Files:**
- Modify: `crates/rodas5p-integrators/src/homotopy_order_policy.rs`
- Test: `crates/rodas5p-integrators/tests/homotopy_policy_contracts.rs`

**Interfaces:**
- Extends the report with selected-policy trajectory rows and observed orders.

- [ ] Write a test that the protected sequential trajectory remains fifth order.
- [ ] Verify RED for the absent new trajectory report.
- [ ] Implement independent trajectory jobs for manufactured-vector, Prothero–Robinson, and
  noncommuting-mass problems.
- [ ] Compute orders only from positive finite errors and stable step ratios.
- [ ] Add explicit fifth-order/stiff-regression/low-depth gates.
- [ ] Run focused tests.
- [ ] Commit `feat: add order policy trajectory gates`.

### Task 5: CLI and reproducibility contract

**Files:**
- Modify: `crates/rodas5p-cli/src/main.rs`
- Modify: `crates/rodas5p-cli/tests/cli_contracts.rs`

**Interfaces:**
- Produces: `rodas5p homotopy-order-policy-screen --profile ... --threads N --output ...`.

- [ ] Write CLI contract test and verify RED.
- [ ] Implement the subcommand and execution metadata.
- [ ] Run CLI twice at one thread and compare bytes.
- [ ] Run at one and four threads and compare scientific fields after removing timing metadata.
- [ ] Commit `feat: expose homotopy order policy screen`.

### Task 6: Execute scientific screens and thread scaling

**Files:**
- Create: `research/homotopy_order_policy_v06/results/*.json`
- Create: `research/homotopy_order_policy_v06/receipts/*.log`
- Create: `research/homotopy_order_policy_v06/reports/HOMOTOPY_ORDER_POLICY_REPORT_KO.md`

- [ ] Run smoke screen at one and four threads.
- [ ] Run canonical screen at one and four threads.
- [ ] Run repeated 1/2/4-thread noninstrumented timings with randomized order.
- [ ] Analyze calibration/holdout false accepts, low-depth retention, observed order, fallback,
  scalar work, and thread throughput.
- [ ] Write the scientific report with explicit negative results.
- [ ] Commit `research: record order policy and rayon screens`.

### Task 7: Independent review, full validation, and delivery

**Files:**
- Create: `research/homotopy_order_policy_v06/reviews/INDEPENDENT_DIFF_REVIEW.md`
- Create: `research/homotopy_order_policy_v06/closeout/PROMOTE_HOLD.md`
- Create: `research/homotopy_order_policy_v06/closeout/NEXT_CYCLE_PLAN_KO.md`
- Create: `research/homotopy_order_policy_v06/receipts/FINAL_VALIDATION_RAW.log`

- [ ] Review the base-to-head diff against the design and scientific gates.
- [ ] Run format, strict Clippy, release tests, release build, CLI determinism, and Git checks.
- [ ] Preserve failed validation attempts as receipts.
- [ ] Tag the verified checkpoint and create the next branch only after validation.
- [ ] Build source ZIP, complete Git bundle, release binary, evidence archive, SHA-256 ledger.
- [ ] Commit `chore: close homotopy order policy cycle`.
