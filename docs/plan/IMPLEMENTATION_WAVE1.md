# Implementation plan — production wave 1

Spec: `docs/plan/PRODUCTION_PLAN_20261020.md` (the production plan; sections cited as §N, theorems as T1–T9).
Base branch: `production/vigilode-plan-20261021`.

Wave 1 implements the parts of phases A and B that do not depend on each other, as five nodes developed in
parallel in separate git worktrees, each on its own branch, each pushed to `origin` after every commit group.
Task 6 integrates them.

## Global Constraints

- Toolchain: the repository pins Rust 1.94.1 in `rust-toolchain.toml`, which cannot be downloaded in this
  environment. Every cargo command runs with `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0). Do not edit
  `rust-toolchain.toml`. Record the toolchain in the node report.
- Every cargo invocation uses `--locked`. No new external crates. Path dependencies between workspace crates are allowed.
- Each node works only in its own worktree `/home/claude/wt/<node-id>` on branch `production/<node-id>`
  created from the tip of `production/vigilode-plan-20261021`, with
  `CARGO_TARGET_DIR=/home/claude/targets/<node-id>`. The machine has 2 cores and 7 GB RAM: run
  `cargo` with `-j 1`, and restrict test runs to the crates and test targets the node touches
  (`-p <crate> --test <name>` or `--lib`); the full workspace suite runs once, in Task 6.
- Default behaviour of every existing public entry point stays bit-for-bit unchanged unless the task says
  otherwise. New behaviour is opt-in through a new option, enum variant or function.
- Statuses are type-distinct (§4 idea 3): an estimate is never labelled a certificate; a wall time on this
  shared 2-core host is diagnostic only and is reported as such.
- Failed attempts and their work stay counted (attempted vs completed).
- Commit messages end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01HyDTYiTaAHRs6wHVKDkVKU`.
  Use `git -c user.name="Jiwon Park" -c user.email="cosmosapjw@gmail.com" commit`.
- Push after every commit group: `git push -u origin production/<node-id>`. Never push to any other branch,
  never force-push, never merge into another branch.
- Each node writes `docs/plan/nodes/<node-id>.md`: what was done, commands run with exit codes and test
  counts, measured numbers with their status, deviations from the brief.
- `cargo fmt` the touched files; `cargo clippy -p <crate> --all-targets --locked` must add no new warnings in touched files.

## Task 1: A1-defects — close the verified correctness defects (node id `a1-defects`)

Plan §6 "Defects to close inside A1" and §2 "Sibling branches". Scope, all in `crates/`:

1. Port the band-solve hardening of PR #73. Source: branch `origin/research/banded-exact-reuse-20261020`
   (fetch it with `git fetch --depth=40 origin research/banded-exact-reuse-20261020`). Take only the
   `crates/` changes of commits `e27c703`, `8d2b2b7`, `b51c051` (diff `b56bb19..b51c051 -- crates/`):
   nonfinite input/output rejection and checked size arithmetic in `rodas5p_fast_banded_solve`,
   `BandedKernel::FixedWidth2`, `FixedWidth2ExactCache` (`exact_band_cache.rs`), `banded_exact_tests.rs`,
   and the test `tests/banded_recovery_contract.rs` if it is in that range (otherwise from `f8e89b7`).
   Resolve conflicts against the current tree; keep the default kernel selection unchanged. Do not port
   research directories or ledger rows (PR #73's L-0137 collides with another branch).
2. Zero tolerance scale: `atol = 0` with a component that is exactly zero at both ends of a step currently
   gives a NaN norm (perpetual rejection) in `rodas5p_fast.rs` (~770–781), `rodas5p_fast_small.rs`
   (~261, ~370), `rodas5p_fast_small_batch.rs` (~424); an `InvalidInput` abort in
   `rodas5p_matrix_free_fast.rs` (~2990–2998); a typed error in `rodas5p-core/src/norms.rs` (55–58).
   Make all of them follow one rule: a component whose scale is zero and whose error is exactly zero
   contributes 0; a component whose scale is zero and whose error is nonzero makes the norm +∞ (reject,
   step shrinks). Implement the rule once (a helper in `rodas5p-core::norms`) and call it from every
   driver. Add a regression test per driver family with `atol = 0`, `rtol = 1e-6` and a problem with
   one identically-zero component.
3. `rodas5p-fair-ab/src/budget.rs:59` slices `&operator_id[..16]`; make it char-boundary safe for short
   and non-ASCII ids; add a test.
4. Accepted/rejected counter drift: `transactional_q1_q2.rs` (~1908–1912) counts a step accepted when
   `error_norm <= 1` while the driver in `integrate.rs` (~1210–1251) rejects when
   `error_norm + contraction_tail > 1` without calling `reconcile_outer_rejection` (`adaptive.rs` ~115–124).
   Call the reconciliation in that driver and in the dense twin (`dense_output_v2.rs` ~2403–2420). Add a
   test that forces the outer rejection and checks `accepted_steps + rejected_steps == attempts` and the
   reconciled counts.
5. `vigilode-coupled/src/solver.rs` (~159–171 vs ~198–207): the eager path computes m stage-0 JVPs per
   Newton iteration whose derivative is never used and charges them. This crate is outside the root
   workspace (own `Cargo.toml`/`Cargo.lock`); build and test it from its own directory. Remove the unused
   evaluation only if output bits are unchanged (the crate has bit-identity tests between eager and
   residual-first paths); update the JVP count expectations in its tests and document the old/new counts.

Acceptance: every new regression test fails before the fix and passes after (show both in the report);
the touched crates' existing tests pass.

## Task 2: A2-wfiltered — W-filtered interior error control (node id `a2-wfiltered`)

Plan §5 T3. Add `DenseErrorControl::WFilteredReport` and `DenseErrorControl::WFilteredEnforce` to
`crates/rodas5p-integrators/src/dense_output_v2.rs`.

- Estimate: E = (M − hγJ₀)⁻¹ (h/2) d(1/2), with d the existing midpoint defect of
  `scaled_midpoint_defect` (θ = 1/2), J₀ the Jacobian the step itself used and γ the RODAS5P diagonal
  coefficient. Reuse the step's own factorization of M − hγJ₀ when the driver has it (expose it from the
  step without changing the step's arithmetic); otherwise factor M − hγJ₀ once per sampled step and record
  that in the report's work counters (`direct_factorizations`). Either way no new Jacobian evaluation.
- `WFilteredReport` records samples, never changes the run. `WFilteredEnforce` uses
  `max(embedded_error, estimate)` in the controller for steps that contain an output time, so an interior
  failure shrinks the step through the normal controller rather than through a separate rejection rule.
- Existing `Off`/`ReportDefect`/`Report`/`Enforce` stay bit-for-bit unchanged.
- Tests (new file `tests/dense_wfiltered_contracts.rs`): (a) Lemma 3 numerically — for y' = λy with a
  constant defect, the ratio estimate/true interior error at θ = 1/2 lies in [1, 1/(2γ)] = [1, 2.3592] for
  z = hλ ∈ {−1e-3, −0.1, −1, −10, −1e3, −1e6}; (b) Prothero–Robinson λ = −1e5, t ∈ [0, 2], 201 output
  times, rtol ∈ {1e-4, 1e-6, 1e-8} (the cases of `docs/DENSE_OUTPUT_ERROR.md` and
  `tests/dense_error_control_contracts.rs`): under `WFilteredEnforce` the true interior max is ≤ 2
  tolerance units at every rtol, and the accepted-step count is reported next to `Off` and `Enforce`;
  (c) the nonlinear witness of `docs/reviews/20261018_production_audit/DEFAULT_PATH_KO.md` DP-01
  (y' = −1e6(y − sin t) + cos t + 10(y − sin t)², y(0) = 0, [0, 1], rtol 1e-8, atol 1e-11, 101 outputs):
  report max normalized interior error and Jacobian/LU counts for Off, Enforce, WFilteredEnforce.
  Kill gate (plan §6 A2): interior error > 2 tolerance units at rtol 1e-6 on Prothero–Robinson, or
  more than 1.3x the `Off` step count. If the kill gate fails, keep the code, mark the result FAIL in the
  node report and in the test (as an `#[ignore]`d measurement test with the numbers), do not loosen it.

## Task 3: B1-lagged — lagged-factorization stage-root driver (node id `b1-lagged`)

Plan §5 T1, T2, T9; §6 B1. New file `crates/rodas5p-integrators/src/rodas5p_lagged.rs` (plus `lib.rs`
export), an opt-in driver `integrate_rodas5p_lagged_observed(problem, span, y0, adaptive, output, options)`.

- Same method and coordinates as the dense fast driver (`rodas5p_fast.rs`, U form with
  W = I/(hγ) − J, `u_i = Σ Γ_ij k_j`), same controller, clock and output rules (reuse its functions; make
  them `pub(crate)` if needed — no arithmetic change in `rodas5p_fast.rs`).
- Each stage system W u = c is solved by preconditioned Richardson iteration
  x ← x + P⁻¹(c − W x), starting from x = P⁻¹c, where P is a stored LU of I/(h_P γ) − J_P from an earlier
  step (h_P, J_P its step size and Jacobian). W x uses the current step's J₀: through the problem's JVP if
  present, else through the explicit current Jacobian's matvec. The Jacobian is evaluated each step (the
  exact RODAS5P target, T1); only the factorization is lagged.
- Stopping rule (T9): stage i stops when WRMS(P⁻¹ r) ≤ ρ_i, ρ_i = θ / (8 · max(a_i, 1)), θ = 0.1, with the
  nonstiff-limit sensitivities a = (0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.01, 1.00) as constants with a
  doc comment citing T1. WRMS uses the step's error scale. Note: in the scaled form W = I/(hγ) − J the
  residual must be converted to the state-increment units of the U form before the WRMS test; derive and
  document the factor.
- Refresh rule (T2): estimate q_k = WRMS(P⁻¹r_{k+1}) / WRMS(P⁻¹r_k). Refactor P at the current (h, J₀) and
  restart the stage when q_k > 0.5, or when the iteration count exceeds 6, or before the first step. A refresh
  is a counted event (`direct_factorizations` plus a new counter in the driver's result); the abandoned
  iterations stay counted.
- When P was refreshed in the current step (P = W exactly), the first iterate is the direct solve; the
  driver then reproduces `integrate_rodas5p_fast_observed` up to rounding. Test: with an option that forces
  a refresh every step, outputs equal the fast driver's within 1e-12 relative on Robertson and HIRES.
- Measurement test (`#[ignore]`d, recorded in the node report, labelled diagnostic): HIRES and Brusselator
  1D (n = 100, the definitions used by `rodas5p-cli/src/stiff_benchmark.rs`), rtol 1e-6, atol per those
  definitions: report accepted/rejected steps, Jacobian evaluations, factorizations, Richardson iterations,
  JVP/matvec count and endpoint error against a tight reference (fast driver at rtol 1e-11) for fast vs
  lagged. Kill gate (§6 B1): factorizations per accepted step > 0.4, or accepted steps more than +5% over
  the fast driver, or endpoint error more than 1.5x the fast driver's. Report PASS/FAIL per problem; never
  tune θ or the thresholds to pass.

## Task 4: T-verify — exact verification of the plan's numerical claims (node id `t-verify`)

New research node `research/production_theory_20261021/` with `PREREGISTRATION.md` (written and committed
before running), `verify_theory.py` (Python 3, sympy/numpy; `pip install --break-system-packages sympy`
if missing), raw results `RESULTS.json`, and one appended row in `research/LEDGER.jsonl` following
`docs/RESEARCH_LEDGER.md` (next free id after the last row; `outputs_sha256` covering `RESULTS.json`;
`commit` = the commit containing the script, which must be pushed). Run `tools/check-research-node.py
--base origin/production/vigilode-plan-20261021` and record its exit code.

Claims to verify from `fixtures/rodas5p_coefficients_snapshot.json` (exact rationals of the decimal
literals, sympy):
1. T1: output sensitivity polynomials m_j(w), w = W⁻¹, of the U-form stage recursion; their values at
   w = 1 (z = 0): |m| = (0.577, 0.237, 6.246, 5.966, 3.729, 4.568, 1.011, 1.000), sum 23.335; the
   coefficient-sum bounds (100.456, 15.401, 24.467, 18.847, 13.655, 14.583, 1.011, 1.000), sum 189.418;
   the scalar exact sensitivities at z ∈ {−1, −10, −100} (sums 17.819, 3.825, 0.125).
2. The preconditioned-residual sensitivity bᵀ(I − T(z))⁻¹: max entry ≤ 6.25 and sum in [1, 23.34] on a grid
   z ∈ −logspace(−4, 8, 400); report the true max and min found.
3. Lemma 3: R(z) = (z/2)/((1 − γz)(e^{z/2} − 1)) is increasing in −z with limits 1 and 1/(2γ); verify
   the derivative sign symbolically for w = −z/2 > 0 where possible, otherwise on a 10⁴-point grid in
   exact-rational or mpmath 50-digit arithmetic, and report which.
4. T2 Corollary 2a: for P = I − h_old γ J₀ and scalar J₀ = λ ≤ 0, ‖I − P⁻¹W‖ ≤ 2|1 − h/h_old|.
5. T9 example: a = (4096, 1, 1, 1), q = 1/2, B = 1/16 gives 68 iterations for the common threshold and 36
   for (18, 6, 6, 6), exactly.
6. The stage-homotopy determinant claim det(∂H/∂U) = det(W)^s for a random 3×3 J₀ and the RODAS5P A, C.
Any claim that fails is recorded as FAIL with the observed value; the plan document is not edited by this
node (a correction note goes in the node report).

## Task 5: A1-api — production facade crate (node id `a1-api`)

Plan §3 "The public API". New workspace crate `crates/vigilode` (added to the root `[workspace] members`;
`Cargo.lock` updated with `--offline` only for the new path crate; no external deps beyond those already in
`[workspace.dependencies]`).

- `Problem::new(n, rhs)` with builders `.jacobian(Jacobian::{Dense(fn), Banded{lower, upper, fill}, Jvp(fn), FiniteDifference})`,
  `.mass(matrix)`, `.time_derivative(fn)`; `Options { rtol, atol: Atol::{Scalar, Vector}, h0, max_steps,
  dense_output: bool, tier: Tier::{Protected, Performance} }`; `Rodas5p::solve(&problem, (t0, t1), &y0,
  &opts) -> Result<Solution, SolveError>`; `Solution { t, y, stats, status }` where a failed run returns
  its partial solution and a typed reason.
- Routing: `Tier::Protected` calls only existing protected drivers through `integrate_rodas5p_routed_observed`
  or the fast/banded/small/matrix-free drivers directly; `Tier::Performance` is accepted and currently
  behaves as `Protected` (documented; wave 2 attaches the lagged driver).
- `Jacobian::FiniteDifference`: dense forward differences with the standard scaled increment
  √ε·max(|y_j|, 1), counted as RHS calls in `stats`; a banded variant using column grouping (Curtis–Powell–Reid)
  when `.banded(lower, upper)` structure is declared.
- `Atol::Vector`: supported if the underlying drivers support a vector atol; if they do not, return
  `SolveError::Unsupported` rather than silently using a scalar.
- `stats` serializes every option and every work counter (serde), so a run is reproducible from its record.
- Tests: Robertson and the stiff example of `examples/solve_stiff.rs` solved through the facade equal the
  direct driver call bit for bit; FD Jacobian solves Robertson to rtol 1e-6 with error within 2x of the
  analytic-Jacobian run; a failing RHS returns the partial solution.
- Doc comments on every public item; a crate-level example that compiles as a doctest.

## Task 6: Integration (controller, after Tasks 1–5)

Branch `production/wave1-integration` from the plan branch; merge the five node branches in the order
`a1-defects`, `t-verify`, `a2-wfiltered`, `b1-lagged`, `a1-api`; resolve conflicts; run
`cargo test --workspace --locked` and the companion crates' tests; record the counts; push; append the
execution log; whole-branch review.
