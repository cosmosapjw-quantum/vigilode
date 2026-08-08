# Adaptive Step Control for All Current Rust Integrators Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add validated adaptive step-size APIs to Sequential RODAS5P, SABR5P, Homotopy RODAS, BDF1/2, and Radau IIA1/3 while preserving all fixed-step trajectories and the v0.11 comparison authority.

**Architecture:** A method-independent controller consumes a dimensionless local-error estimate and estimator order. RODAS endpoints retain the native embedded estimate; BDF/Radau use coarse-versus-two-half-step reference estimates. Every trial is transactional, and every unit of rejected/coarse/fine work remains counted.

**Tech Stack:** Rust 1.94.1, `faer 0.24.4`, offline Cargo vendor tree, existing pure-Rust workspace, Rayon only for independent experiment jobs.

## Global Constraints

- Preserve fixed-step public APIs and reference trajectories.
- No new dependency.
- No BDF3–5, native RADAU5 estimator, dense output, sparse/MPI/GPU/DAE claim.
- All normalized errors and controller factors are dimensionless.
- Accepted state/history/recycle/output mutations are transactional.
- External global error, not equal internal tolerance, remains the cross-family authority.

---

### Task 1: Common adaptive controller and diagnostics

**Files:**
- Create: `crates/rodas5p-integrators/src/adaptive.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Test: `crates/rodas5p-integrators/tests/adaptive_controller_contracts.rs`

**Interfaces:**
- Produces: `AdaptiveStepConfig`, `ControllerKind`, `AdaptiveControllerState`, `AdaptiveRunDiagnostics`, `AdaptiveObservedIntegrationResult`, and `step_doubling_wrms_error`.

- [ ] Write failing tests for invalid bounds, I/PI factors, rejected-step cap, accepted-error history, and a synthetic step-doubling estimate.
- [ ] Run the focused test and verify failures are caused by missing APIs.
- [ ] Implement validated controller types and pure factor/estimator functions.
- [ ] Run focused and full integrator tests.
- [ ] Commit `feat: add common adaptive controller contract`.

### Task 2: Refactor native RODAS adaptive paths

**Files:**
- Modify: `crates/rodas5p-integrators/src/integrate.rs`
- Test: `crates/rodas5p-integrators/tests/adaptive_rodas_contracts.rs`

**Interfaces:**
- Consumes: Task 1 controller.
- Produces: report-returning Sequential/SABR adaptive observed APIs while preserving existing wrappers.

- [ ] Write characterization tests for the previous default Sequential/SABR accepted states and attempts.
- [ ] Verify the tests pass before refactor, then add failing diagnostics assertions.
- [ ] Delegate the existing default controller constants to `AdaptiveStepConfig::rodas_default()`.
- [ ] Ensure rejected attempts restore stage/recycle history and keep work counters.
- [ ] Run fixed/adaptive regression tests.
- [ ] Commit `refactor: unify rodas adaptive controller semantics`.

### Task 3: Variable-step BDF2 algebra

**Files:**
- Modify: `crates/rodas5p-integrators/src/bdf.rs`
- Test: `crates/rodas5p-integrators/tests/adaptive_bdf_contracts.rs`

**Interfaces:**
- Produces: internal variable-step BDF mode and step-ratio diagnostics; public fixed `bdf_step` remains unchanged.

- [ ] Write failing tests for unequal-step quadratic exactness, `r=1` coefficient regression, and predictor formula.
- [ ] Run focused tests and verify RED.
- [ ] Implement the nonuniform BDF2 residual/Jacobian coefficients behind an internal mode.
- [ ] Preserve fixed-step fallback-to-BDF1 behavior at clipped unequal steps.
- [ ] Run native BDF fixed regression tests.
- [ ] Commit `feat: add exact variable-step bdf2 kernel`.

### Task 4: Adaptive BDF1/2 reference integrator

**Files:**
- Modify: `crates/rodas5p-integrators/src/bdf.rs`
- Test: `crates/rodas5p-integrators/tests/adaptive_bdf_contracts.rs`

**Interfaces:**
- Produces: `integrate_bdf_adaptive_observed`.

- [ ] Write failing tests for rejection rollback, fine-history commit, startup estimator order, output clipping, and tolerance response.
- [ ] Implement cloned coarse/fine histories and accept the two-half-step state.
- [ ] Count all coarse/fine/rejected work and distinguish macro versus internal steps.
- [ ] Run BDF order, stiff, noncommuting-mass, and global-error tests.
- [ ] Commit `feat: add adaptive bdf reference lane`.

### Task 5: Adaptive Radau IIA1/3 reference integrator

**Files:**
- Modify: `crates/rodas5p-integrators/src/radau.rs`
- Test: `crates/rodas5p-integrators/tests/adaptive_radau_contracts.rs`

**Interfaces:**
- Produces: `integrate_radau_adaptive_observed`.

- [ ] Write failing tests for step-doubling error, rollback, clipping, tolerance response, and expected estimator order.
- [ ] Implement coarse/two-half-step transactional trials.
- [ ] Count full Newton and factorization work from all trial paths.
- [ ] Run Radau order/stiff/mass/global-error tests.
- [ ] Commit `feat: add adaptive radau reference lane`.

### Task 6: Adaptive Homotopy RODAS integration

**Files:**
- Modify: `crates/rodas5p-integrators/src/integrate.rs`
- Test: `crates/rodas5p-integrators/tests/adaptive_homotopy_contracts.rs`

**Interfaces:**
- Produces: `integrate_homotopy_adaptive_observed`.

- [ ] Write failing tests for fast acceptance, protected fallback, rejection rollback, and output-grid identity.
- [ ] Implement the wrapper using native RODAS endpoint error plus existing homotopy certificate/fallback outcome.
- [ ] Keep path/fallback work and reasons auditable.
- [ ] Run homotopy false-success and global-order screens.
- [ ] Commit `feat: add adaptive homotopy integration lane`.

### Task 7: Unified adaptive global-error screen

**Files:**
- Create: `crates/rodas5p-fair-ab/src/adaptive_global_error.rs`
- Modify: `crates/rodas5p-fair-ab/src/lib.rs`
- Modify: `crates/rodas5p-cli/src/main.rs`
- Test: `crates/rodas5p-fair-ab/tests/adaptive_global_error_contracts.rs`
- Test: `crates/rodas5p-cli/tests/cli_contracts.rs`

**Interfaces:**
- Produces: deterministic report across current executable families and T1/T4 execution modes.

- [ ] Write failing tests for candidate coverage, common output grid, failure preservation, tolerance ladder, and scientific checksum identity.
- [ ] Implement bounded analytic corpus and method-specific tolerance sweeps.
- [ ] Run 1-thread and 4-thread screens with stable sorting.
- [ ] Analyze error–work Pareto behavior without declaring a universal winner.
- [ ] Commit `bench: add unified adaptive global-error screen`.

### Task 8: Harness closeout and delivery

**Files:**
- Create/update: `research/adaptive_all_methods_v012/phases/PHASE00_*.md` through `PHASE10_*.md`
- Create/update: `research/adaptive_all_methods_v012/harness/*`
- Create: reports, validation receipts, result JSON/CSV, closeout and handoff.

- [ ] Run format, strict Clippy, full measurement-profile tests, and build.
- [ ] Run fixed-step regression and adaptive scientific/reproducibility suites.
- [ ] Perform independent base-to-head diff review and fix blocking findings.
- [ ] Validate both copied harnesses.
- [ ] Build source ZIP, complete Git bundle, binary, SHA-256 ledger, and delivery ZIP.
- [ ] Tag a verified checkpoint and create the next research branch only after all exact-HEAD gates pass.
