# Task 2 report: A2-wfiltered (W-filtered interior error control)

Status: DONE_WITH_CONCERNS. The implementation is complete and the tests are green. **The preregistered kill gate FAILS on the step count.**
Branch `production/a2-wfiltered`. Commits: `b61707c` (code and tests), `7bea78c` (node report). Both are pushed.

## Implementation
- `crates/rodas5p-integrators/src/dense_output_v2.rs`:
  - New variants `DenseErrorControl::WFilteredReport` and `WFilteredEnforce`.
  - New public helpers `rodas5p_wfiltered_iteration_matrix` and `rodas5p_dense_wfiltered_interior_error_estimate`, plus the private `wfiltered_estimate_from_step`.
  - Driver changes:
    - The W-filtered modes call a capturing variant of the sequential step.
    - `WFilteredEnforce` sets error = max(embedded, estimate) and accepted = error <= 1, then hands both to the normal controller. Its estimate work is charged to the run counters.
    - SABR is refused with InvalidInput.
- `crates/rodas5p-integrators/src/sequential.rs`:
  - The body of `sequential_step_with_residual_accounting` moved into a private `sequential_step_impl` with an optional capture slot. The public function only forwards to it, so its arithmetic is unchanged.
  - New crate-internal `sequential_step_capturing_iteration_matrix` and `StepIterationMatrix`. They keep the step's own LU of W = M - h gamma J0 when the step made one (Direct method or Direct preconditioner), and otherwise the explicit W.
  - The private `sequential_stages_impl` and `sequential_stages_refined` gained a `factor_capture` parameter. Every other caller passes None.
  - `StepResult` is unchanged.
- `crates/rodas5p-integrators/src/lib.rs`: re-exports the two helpers.
- `crates/rodas5p-integrators/tests/dense_wfiltered_contracts.rs` (new): 8 tests plus 1 ignored kill-gate test.
- `docs/plan/nodes/a2-wfiltered.md` (new).

## Factorization reuse
- Direct stage solves reuse the step's own LU. That gives 0 extra LU and 1 back-solve per sample; the PR test asserts this.
- GMRES with no preconditioner factors W once per sampled step, counted in `direct_factorizations`. The test asserts that count equals the number of samples.
- Neither path evaluates a new Jacobian; `jacobian_builds == 0` in the report counters is asserted.

## RED/GREEN
- RED: the new target failed to compile (E0432/E0599, missing variants and helper), exit 101.
- GREEN: `dense_wfiltered_contracts` passed 8 with 1 ignored, `dense_error_control_contracts` 4, `audit0930_accounting_contracts` 1, `dense_output_v2_contracts` 15, and `--lib` 62 with 1 ignored. All exit codes were 0.
- With `--include-ignored`, only the kill gate fails: "2.143x the Off accepted-step count > 1.3x".
- Clippy shows no warnings in the touched files; three that appeared were fixed. rustfmt is clean on the touched files.
- Toolchain: stable, cargo 1.97.0.

## Measured (diagnostic counts and tolerance units, no timings)
(a) Lemma 3 ratio R for z = -1e-3, -0.1, -1, -10, -1e3, -1e6: 1.000038, 1.003931, 1.048525, 1.613758, 2.348106, 2.359174. All lie in [1, 2.3592] and the sequence is monotone.

(b) PR lambda = -1e5, Direct solver. Columns are Off / Enforce / WFilteredEnforce.

| rtol | interior | accepted | attempts | Jacobians | LU |
|---|---|---|---|---|---|
| 1e-4 | 3.235 / 0.921 / 0.373 | 7/7/8 | 7/8/8 | 7/14/8 | 7/14/8 |
| 1e-6 | 311.467 / 1.052 / 0.350 | 7/12/15 | 7/26/15 | 7/50/15 | 7/50/15 |
| 1e-8 | 208.636 / 1.381 / 0.351 | 11/31/39 | 11/83/39 | 11/164/39 | 11/164/39 |

Kill gate at rtol 1e-6:
- Interior error 0.350 <= 2: pass.
- Steps 15/7 = 2.143x > 1.3x: **FAIL**.

The gate test is recorded as `#[ignore]` with these numbers and the threshold unchanged.

(c) DP-01 nonlinear witness (GMRES, as in the audit probe). Columns: max normalized error, accepted/rejected, RHS, Jacobians, LU.

| mode | max normalized | acc/rej | RHS | Jacobians | LU |
|---|---:|---|---:|---:|---:|
| Off | 1801.6160 | 6/0 | 48 | 6 | 0 |
| Enforce | 1.4986 | 17/24 | 366 | 79 | 38 |
| WFilteredEnforce | 0.3350 | 21/0 | 187 | 21 | 19 |

## Old modes bit-identical
- `dense_error_control_contracts` passes in the same build, including the Off/Report bit-for-bit check against the legacy path.
- Off and Enforce in DP-01 reproduce the DEFAULT_PATH_KO.md record exactly (1801.616..., 17/24, RHS 366, 79 Jacobians, 38 LU, max estimate 0.9216895307918039).
- The PR Off/Enforce columns match docs/DENSE_OUTPUT_ERROR.md.
- `WFilteredReport` is asserted bit-identical to Off: counters, diagnostics and y bits.

## Self-review
- The capturing step only clones the LU, or W when there is no LU, after the stages are formed. This happens only on the W-filtered path, so arithmetic is unchanged and the cost is an n^2 copy per trial step.
- Mass matrices are handled: W includes M (from ShiftedOperator) and the defect is M u' - f.
- If the estimate is unavailable (matrix-free Jacobian or a singular LU), the step is counted in `unavailable` and not enforced, which is the existing convention.
- The estimate is sampled only on steps the embedded estimate accepted and that contain an interior output time, the same rule as Report/Enforce.

## Concerns
1. Kill gate FAIL as preregistered.
2. Gate wording differs: the brief scores steps against Off, while plan section 6 says "the protected step count". Against Enforce the ratios would be 1.25x accepted and 0.58x attempts. I scored it against Off as briefed and loosened nothing.
3. Shared CARGO_TARGET_DIR hazard. Workspace crates hash identically across worktrees, so another node's `rodas5p-integrators` build can be treated as fresh. One run linked against a foreign lib and failed to compile. The workaround was to `touch` my sources and build and run in a single invocation. Task 6 and other nodes should be told.
4. `rustfmt --check` on the crate flags a pre-existing format diff in tests/production_audit_packet_curve.rs. That file is not mine and I left it as is.
