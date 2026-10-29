# Implementation plan — production wave 2

Spec: `docs/plan/PRODUCTION_PLAN_20261020.md` (§N, T1–T9). Wave-1 record: `docs/plan/IMPLEMENTATION_WAVE1.md`,
`docs/plan/nodes/*.md`.
Base branch: `production/wave1-integration` at the tip that closes the wave-1 final review (the controller
fills the exact SHA into each worktree).

Wave 2 continues phases A–C with five nodes that touch disjoint files, developed in parallel in separate
worktrees, each pushed after every commit group; Task 6 integrates.

## Global Constraints

- Everything in the wave-1 Global Constraints applies (toolchain `RUSTUP_TOOLCHAIN=stable`, `--locked`,
  opt-in new behaviour, bit-identical defaults, type-distinct statuses, counted failures, commit identity and
  trailers, push only to the node's own branch, node report `docs/plan/nodes/<node-id>.md`, fmt/clippy).
- Build environment: `export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/<node-id>
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `-j 1`. Each node has its own
  target dir (a shared one cross-links worktrees). Never `cargo clean`, never `cargo test --workspace`.
  Check `df -h /home/claude` before large builds; stop and report below 4 GB free.
- No new external crates in the root workspace. Exception: Task 5's benchmark crate is its own Cargo
  workspace (like `crates/vigilode-hybrid`) and may depend on `diffsol` with the exact version and lock
  entries already pinned in `crates/vigilode-hybrid/Cargo.lock`.
- Comparator toolchain (installed 2026-10-21): GNU Fortran 13.3.0 (`apt-get install gfortran`) and SUNDIALS 6.4.1
  (`apt-get install libsundials-dev`, headers in /usr/include, libs in /usr/lib/x86_64-linux-gnu). Pinned Fortran
  sources: Netlib DVODE + LINPACK `dgefa/dgesl/dgbfa/dgbsl` from branch `research/benchmark-results-20261019-dual`,
  `research/strict_solver_comparison_20261019/refined_runs/39261895983/sources/` (with NETLIB_SHA256SUMS); Hairer
  RADAU5/RODAS from the Assimulo 3.0 sdist (sha256 `61cdca4562745078c966cc4fab44d8bc0db11a9e6856b45c470ac65c16af9e8a`,
  `assimulo/thirdparty/hairer/{radau_decsol.f,rodas_decsol.f,LICENSE_HAIRER}`). Record every source hash.
- Wave-1 rulings bind: `wrms` stays strict for convergence callers, `step_error_wrms` carries the zero-scale
  rule for end-of-step norms; `Tier::Performance` routes no lagged driver until a B1 card passes its kill
  gate; preregistered kill gates are never re-scored or loosened.

## Task 1: C1-transaction — the Transaction and Telemetry types (node id `c1-transaction`)

Plan §4 idea 2, §6 C1, T8. New module `crates/rodas5p-integrators/src/transaction.rs` (plus `lib.rs` export).
No existing driver is migrated in this task.

- `Transaction<S>` over a committed state `S: Clone` (state, controller state, history, Krylov state are the
  caller's `S`): `begin(&committed) -> Trial`, `Trial::charge(WorkCounters)`, `Trial::commit(self) -> S`,
  `Trial::rollback(self)`; work charged to a trial stays in the ledger on rollback (attempted vs completed
  counters); dropping a `Trial` without commit or rollback is a rollback and is counted as such.
- `Budget` with a hard cap in work units reserved before scheduling (the `vigilode-coupled` `Work` ledger
  pattern, `crates/vigilode-coupled/src/lib.rs`), returning a typed `BudgetExhausted` with the work used.
- `AdmissionRule` implementing T8 Proposition 8: per-regime-bin Beta(α, β) posterior of candidate success
  updated only from charged attempts; admit when the upper confidence bound of p exceeds
  (C_a + C_v)/C_f; while the posterior is below threshold, allow at most one attempt per m accepted protected
  steps with m = ⌈10 (C_a + C_v)/C_f⌉. Costs are measured work units passed by the caller. Pure,
  deterministic, no wall time.
- `Telemetry` record: Ritz-value summary (spectral radius estimate, max |Im|/|Re| ratio), refresh q
  estimate, Jacobian drift estimate δ_J, rejection streak — plain data with serde, no driver logic.
- Tests: commit/rollback/drop accounting identities (attempted = committed + rolled back); budget
  reservation never exceeds the cap under concurrent reservation (use std threads with an AtomicU64 CAS like
  `Pexprb54s4BudgetedOperator`); AdmissionRule bounded-overhead property: for a candidate that always fails,
  charged candidate cost over N protected steps ≤ 0.10 × protected cost for N ∈ {100, 1000}; for a candidate
  that always succeeds and is cheaper, admission rate → 1 within 50 attempts.

## Task 2: B1b-gmres — Krylov-accelerated lagged factorization (node id `b1b-gmres`)

New hypothesis card (the wave-1 B1 Richardson card FAILED its kill gate: refreshes forced by the 6-iteration
cap because q ≈ 0.1–0.2 per Richardson step against a ~1e-6 reduction target). New mechanism: replace
Richardson by GMRES right-preconditioned with the same stored LU P (T2: GMRES residual ≤ κ(P) q^k, and
GMRES is never worse than Richardson's polynomial), so the same refresh rule fires far less often.

- In `crates/rodas5p-integrators/src/rodas5p_lagged.rs` add an option `StageSolver::{Richardson (default,
  wave-1 behaviour bit-identical), Gmres}` using `rodas5p-krylov` `gmres_into` with right preconditioning
  (`Preconditioner` side Right), restart = 10, stopping on the same T9 targets ρ_i measured on the
  preconditioned residual P⁻¹r (compute it explicitly at each GMRES exit check if gmres_into stops on the
  true residual; document which residual is tested).
- Refresh rule unchanged in spirit: refresh when the GMRES iteration count for a stage exceeds 6 or the
  observed convergence factor (‖r_k‖/‖r_0‖)^(1/k) > 0.5.
- Preregister before measuring (`docs/plan/nodes/b1b-gmres.md` first section, committed and pushed before
  the measurement run): same problems, tolerances, reference and kill gate as wave-1 Task 3 —
  factorizations per accepted step ≤ 0.4, accepted steps ≤ +5% over the fast driver, endpoint error ≤ 1.5x
  the fast driver's — on HIRES and Brusselator 1D n = 100 at rtol 1e-6. Report PASS/FAIL per problem with
  all counters; never tune the constants.
- Tests: forced refresh every step with `Gmres` still reproduces the fast driver within 1e-12 relative on
  Robertson and HIRES; JVP path exercised with iterations > 0.

## Task 3: P0-facade — protected-tier completeness of the facade (node id `p0-facade`)

Plan §3 P0 tier; wave-1 rulings on mass, dense output and partial solutions.

- Route `.mass(M)` and `dense_output = true` in `crates/vigilode` to the protected sequential dense driver
  `integrate_adaptive_dense_observed_with_dense_error_control` (`dense_output_v2.rs`) with
  `DenseErrorControl::WFilteredEnforce` by default for dense output and an `Options::interior_error`
  enum {WFilteredEnforce (default), Enforce, Off} exposed; a mass matrix requires an explicit or FD
  Jacobian (refuse JVP-only with `Unsupported`).
- `Solution::at(t)` evaluates the dense extension when dense output was requested; otherwise returns the
  stored sample or `None`.
- Partial solution after failure: extend the fast dense, banded and matrix-free drivers (`rodas5p_fast.rs`,
  `rodas5p_matrix_free_fast.rs` result types) to report the last accepted `(t, y)` and accepted samples on
  failure without changing any success-path arithmetic; the facade returns them. Test with a fatal RHS error
  after several accepted steps and a fatal JVP error inside GMRES (the wave-1 untested path).
- `SolveError`/`Status` reasons: replace the `StepSizeTooSmall` catch-all with the driver's actual failure
  kind (linear solve, non-finite, step size, attempts budget).
- Bit-identity tests against the direct driver calls for each new route.

## Task 4: H-hygiene — test isolation, ledger tombstone, workspace membership (node id `h-hygiene`)

Small independent items, one subagent:
1. The three allocation-counter tests that fail under parallel test threads
   (`rodas5p-core/tests/leja_setup_work_contract.rs`,
   `rodas5p-integrators/tests/collective_admission_contracts.rs`,
   `rodas5p-integrators/tests/production_audit_pr101_provider.rs`): make each measured region exclusive
   (a process-wide test mutex around every allocation-counting test in the same binary, or run the counting
   section on a dedicated thread with a thread-local counter — choose the smallest change that makes the
   tests deterministic under the default harness; show 5 consecutive parallel runs passing).
2. Ledger: append a correction row with id `L-0138` to `research/LEDGER.jsonl` that supersedes L-0124 with the current
   `outputs_sha256` of `research/production_audit_20261018/CONTINUATION_DAG.json` and
   `CONTINUATION_FINDINGS.json` (verdict INCONCLUSIVE, claim: hashes re-bound after post-run edits; cite the
   commit that last changed each file), so `tools/check-research-node.py --base <wave2 base>` exits 0; record the exit code.
3. Add `crates/vigilode-coupled` to the root workspace members (remove its own `[workspace]` table and
   `Cargo.lock`, take its dependency versions from `[workspace.dependencies]`); the root `Cargo.lock`
   diff must add only the `vigilode-coupled` package entry (its deps are already in the root lock — verify);
   its 37 tests must pass from the root with `cargo test -p vigilode-coupled --locked`. Leave
   `vigilode-hybrid` standalone (its optional `diffsol` dependency would enter the root lock).
4. Remove the five `coupled-*.yml` workflow steps that existed only because the crate was outside the
   workspace only if they do nothing else; otherwise leave them and say why.

## Task 5: E1-bench — matched-accuracy work–precision harness (node id `e1-bench`)

Plan §7. New standalone crate `crates/vigilode-bench` with its own `[workspace]` and `Cargo.lock`
(lock entries for `diffsol` and its dependencies copied from `crates/vigilode-hybrid/Cargo.lock`; path
dependencies on `vigilode` (facade) and `rodas5p-integrators`).

- Binary `vigilode-bench wp --corpus <file> --out <json>`: for each problem × solver × tolerance in a
  declarative corpus file, run fresh in-process repetitions, record achieved error on a common output grid
  (dense output through each solver's own interface: Diffsol `solve_dense`, VigilODE facade), all work
  counters, failure status and allocator-measured allocations (a counting global allocator in the bench
  binary), and wall time labelled `timing_authority = "diagnostic_only"`.
- Solvers: VigilODE facade (Protected), Diffsol 0.16.1 BDF with FaerLU (dense) and, where Diffsol offers it,
  its sparse/banded backend; SUNDIALS CVODE 6.4.1 (BDF, dense and band linear solvers, C FFI); Netlib DVODE (MF21
  dense, MF24 banded) and Hairer RADAU5 and RODAS (dense output via CONTR5/CONTRO), all through a C/Fortran
  bridge built by the crate's build.rs with gfortran, sharing the one Rust RHS/Jacobian; the internal BDF/Radau reference implementations only with
  `comparator_fidelity = "reference-implementation-only"` and excluded from any speed table.
- Corpus v1: Robertson (n = 3), HIRES (n = 8), van der Pol μ = 1000, Brusselator 1D n = 100 — same
  equations and atol vectors in every arm (one Rust RHS and Jacobian shared by all arms), output grid 101
  points, reference from VigilODE fast at rtol 1e-12 cross-checked against Diffsol at its tightest
  successful tolerance (agreement recorded; disagreement > 1e-8 marks the row `reference-disputed`).
- `vigilode-bench report --in <json> --out <md>` generates the Markdown tables from the JSON only; a test
  checks that every number in the generated Markdown appears in the JSON.
- Matched-error comparison: interpolate each arm's log(time) vs log(error) curve at common error levels;
  speed ratios only where both arms bracket the level; otherwise `DIAGNOSTIC_ONLY`.
- Failed arms are rows with status and work, never dropped.
- Run corpus v1 once at rtol {1e-4, 1e-6, 1e-8} and commit the JSON and generated Markdown under
  `research/bench_wave2_20261021/` with a PREREGISTRATION.md committed before the run and one ledger row with id `L-0139`
  (verdict INCONCLUSIVE: shared 2-core host; the row exists for provenance).

## Task 6: Integration (controller)

Branch `production/wave2-integration` from the wave-2 base; merge `h-hygiene`, `c1-transaction`,
`p0-facade`, `b1b-gmres`, `e1-bench`; run the full workspace suite and the companion crates; push; final
whole-branch review; one fix wave.
