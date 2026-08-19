# RODAS5P Rust Parity Port Implementation Plan

> **For agentic workers:** implement task-by-task with test-first development and fresh
> verification before every completion claim.

**Goal:** Produce a durable Rust-only parity checkpoint for the prior Python research code.

**Architecture:** Five-crate workspace with a shared core numerical layer, common Krylov
kernels, protected integrators, a Rust-only Fair A/B harness, and one CLI.

**Tech Stack:** Rust 1.94.1, Cargo offline/vendor lock, faer 0.24.4, serde, clap, rand_pcg.

## Global Constraints

- No homotopy or continuation implementation.
- Offline and locked builds only.
- Single-thread strict benchmark default.
- External true-residual certification for every solver.
- Transactional recycle state on failures and rejected steps.
- Same binary and release profile for all benchmark arms.

## Tasks

1. Supply-chain lock, workspace scaffold, environment receipt.
2. Core coefficients, norms, matrices, operators, counters, direct oracle.
3. Common vector kernels and restarted GMRES.
4. Transactional LGMRES.
5. Fixed-left GCRO-DR with refresh and invariant tests.
6. Protected sequential RODAS5P and convergence tests.
7. Adaptive controller and rejection rollback.
8. Structured block system and SABR5P fallback.
9. Rust-only Fair A/B accounting, trace identity, and state lifetimes.
10. CLI, strict benchmark, independent validation, and delivery artifacts.
