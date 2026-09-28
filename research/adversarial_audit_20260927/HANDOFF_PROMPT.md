# VigilODE cloud handoff: implement the adversarial-audit fixes

You are continuing work on `cosmosapjw-quantum/vigilode` in a cloud session. An adversarial audit of the solver was completed on 2026-09-28 and is checked in on this branch. Your job is to implement the code fixes it recommends, one work unit at a time, test first. The audit changed no source code.

## 0. Starting state

| Item | Value |
|---|---|
| Repository | `cosmosapjw-quantum/vigilode` (public) |
| Start branch | `audit/adversarial-audit-20260927` |
| Its content | Draft PR #42 head `b3e8165c8dc3b5016702821d280daea1a3f1feb7` plus one commit that adds `research/adversarial_audit_20260927/` |
| Rust source | identical to PR #40 head `426d37c` |
| `main` | `8d0c791`, older. Do not base fixes on `main` |
| Toolchain | Rust 1.94.1, pinned in `rust-toolchain.toml` |
| Dependencies | normal crates.io access; `.cargo/config.toml` is a placeholder |

Baseline measured at `b3e8165` on the audit host:

| Command | Result |
|---|---|
| `cargo test --workspace --locked` | 438 passed, 0 failed, 2 ignored |
| `cargo test -p rodas5p-integrators --features audit2-research --locked` | 315 passed |
| `cargo test -p rodas5p-integrators --features audit2-bateman-authority --locked` | 321 passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `cargo test -p rodas5p-integrators --test v37_continuation_transaction_contracts --locked -- --ignored` | fails, `left: 17, right: 18` at line 110, in dev and optimized profiles |

Reproduce this baseline before changing anything. The default suite takes about 9 minutes in the dev profile on 24 cores. If your numbers differ, stop and report the difference.

## 1. Read first, in this order

All paths are relative to `research/adversarial_audit_20260927/`.

1. `VIGILODE_ADVERSARIAL_AUDIT_20260927.md`, chapters 1, 7 and 8. The report is in Korean. Chapter 1 is the summary, chapter 7 the improvement plan, chapter 8 the limits and the corrections made during the final check.
2. `VIGILODE_ADVERSARIAL_AUDIT_LEDGER_20260927.json`. Machine-readable. `findings[]` carries id, severity, status, locations, evidence, refutations and `proposed_fix`. Also read `improvement_plan`, `experiments`, `hypotheses_tested`, `corrections`, `nonclaims`. The schema is next to it.
3. `fixes/G1_*.json` to `fixes/G6_*.json` and `fixes/G3b_*.json`. One file per fix group, with anchors, effort, risk, verification and code sketches. `fixes/roadmap_delta.json` and `fixes/completeness_critic.json` hold the roadmap proposal and the audit's own gaps.
4. `experiments/E-xx/README.md` and `results.json`. These are the evidence and the numeric baselines for verification.
5. `harness/`. A standalone crate with the experiment binaries. It is not a member of the repository workspace.

```bash
cd research/adversarial_audit_20260927/harness
cargo build --locked --profile measurement
ls target/measurement/
```

The harness has its own `.gitignore` for `target/`. Keep it that way: `rodas5p-fair-ab/build.rs` embeds a dirty flag from `git status`, and any untracked file in the repository flips it. Write experiment outputs outside the repository.

Run the harness binaries with `RAYON_NUM_THREADS=1`. Each experiment README lists the arguments its binary expects. Large raw inputs were not checked in. Files above 300 KB were left out, including the E-02 row dump and the E-05 and E-06 generated matrices. Regenerate them with the Python scripts in the experiment directories.

## 2. Rules

These follow the project's own governance. Do not break them.

- **Test first.** Every work unit starts with a failing test that encodes the defect. Then make the smallest fix. Then run the focused suite, the full workspace suite, clippy with `-D warnings`, and fmt.
- **One work unit, one branch, one Draft PR.** Base each PR on `audit/adversarial-audit-20260927`. Stack a unit on an earlier one only when both touch the same file. Never merge, tag or release. Never push to `main` or to any `research/audit2-*` branch.
- **No tolerance widening.** Do not loosen a tolerance, threshold or acceptance rule to make a test pass. If a fix changes a pinned snapshot, record the old value, the new value and the reason in the PR body.
- **Ledgers are append-only.** Add dated addenda under `research/`. Never rewrite a historical ledger or receipt.
- **No performance claims.** No speedup, ranking, scaling or holdout claim. Do not read `tools/reference_v2/artifacts/*-holdout-v2.json` for any tuning purpose.
- **Preserve failures.** If a unit cannot be finished, report what was tried and what failed. Do not drop it silently.
- **Re-anchor line numbers.** Every line number in the audit refers to `b3e8165`. Confirm with `grep -n` before editing.
- **Keep defaults stable.** Default behavior for library callers must not change unless the work unit says so. Research code stays behind its feature flag.

## 3. Work units, in priority order

P1 findings come first. Effort uses the scale XS, S, M, L, XL.

### WU-1. GCRO-DR state validity. `F-010` (P1). Effort S, risk low.

- Evidence: `experiments/E-05/`. Design: `fixes/G3_krylov_robustness.json`.
- Defect: in `crates/rodas5p-krylov/src/gcrodr.rs` near lines 416 to 464, the warm start is taken from `previous_solution` and copied with `copy_from_slice` without a length or identity check. A dimension change from 32 to 16 panics, and the release profile uses `panic = "abort"`. After an operator change the stale solution seeds a different system. LGMRES already resets its state, see `lgmres.rs` near lines 69 to 77.
- Fix: reset `previous_solution` and the recycle space when the dimension or the exact Krylov system identity changes. Return a typed error instead of panicking.
- Tests to add: reuse of one state across a dimension change returns cleanly; an operator identity change clears the warm start; one invariant test with `recycle_dim > 1`. All existing multi-solve tests use `recycle_dim: 1`.

### WU-2. Krylov input validation and seeded certificate. `F-034`, `F-037`, `F-036`. Effort XS to S.

- Design: `fixes/G3b_krylov_robustness.json`.
- `rodas5p-core/src/solver_types.rs` near 55 to 57 and `gmres.rs` near 40 to 44 test only `< 0.0`, so NaN and infinite tolerances pass. `atol = +Inf` certifies `x = 0` as converged. `lgmres.rs` near 58 to 62 and `gcrodr.rs` near 401 to 407 validate no tolerance at all.
- `block_gmres.rs` near 493 to 511: the seeded shared-basis path returns `converged = true` with zero residuals and no true-residual check when the largest preconditioned right-hand side is near zero.
- `gmres_givens.rs` is a test-only kernel whose outcomes differ from production `gmres.rs`. Document it as a research candidate and pin the difference in a differential test.

### WU-3. Inner forcing rule. `F-009` (P1), `F-008`, `F-031`, `F-030`. Effort S for step B, M for step C.

- Evidence: `experiments/E-04/`. Design: `fixes/G2_inner_forcing_rule.json`, plus the `F-030` test plan in `fixes/G6_evidence_process_tests.json`.
- Code: `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs` near 58 to 76, `sequential.rs` near 359 to 369 and 508 to 512.
- Step A, tests first. Write a fixed-step halving ladder whose `rtol` does not depend on `h`, on Prothero-Robinson and on the advection-diffusion problem. Assert that the inexact arm stays within a stated factor of the direct-LU arm. The existing order test ties `rtol` to `h^6` and cannot see the defect. Keep it and rename it.
- Step B. Remove the step-0 hard error. Replace the guard with a residual floor expressed in the right-hand-side scale, and record a flag when the floor is active.
- Step C. Replace the residual target `tau = 0.1 / ||b||_1`, which does not depend on `h`, with a target that preserves fixed-step convergence.
- Baselines from E-04: on the advection-diffusion problem the direct-LU slopes are 5.33, 5.27, 5.15, 5.05. On stiff Prothero-Robinson even direct LU shows order about 4. This is known order reduction. State the pass criterion relative to the direct arm, not as a slope of at least 4.5.

### WU-4. Load the tableau once. `F-049`. Effort XS, risk low.

- Design: `fixes/G5_accounting_fairness.json`.
- `load_rodas5p_coefficients()` parses JSON and inverts an 8 by 8 matrix on every step attempt and on every dense-output sample: `sequential.rs` near 154 and 202, `dense_output_v2.rs` near 82, `stage_batch.rs` near 173.
- Fix: load once with `OnceLock`. Results must be bit-identical. Verify the provenance hash carried in the fixture at that single load.

### WU-5. φ-action accuracy. `F-011` (P1), `F-040`, `F-042`, `F-043`, `F-039`, `F-041`. Effort M.

- Evidence: `experiments/E-06/`. Design: `fixes/G4_exponential_phi.json`.
- Do this first: reproduce `F-011` in Rust. The audit's numeric example came from a numpy replica, not from the Rust kernel. Use `A = diag(-1, -3)`, `v = e1 + 4.5e-7 * e2`, `rtol = 1e-10`, and print `converged`, `error_estimate` and the true error. If it does not reproduce, report that and downgrade the unit.
- Code: `rodas5p-core/src/matrix_functions.rs` near 158 embeds the unscaled vector in the augmented matrix, and the squaring count near 80 to 86 then depends on the norm of `v`. `rodas5p-integrators/src/exponential.rs` near 881, 1078 and 1094 to 1099 declares happy breakdown at `64 * sqrt(eps)` and reports `error_estimate = 0`. Near 1357 to 1361 the fused combination divides by `scale^k`.
- Fix: normalize the vector before the augmented exponential, report a residual estimate at breakdown, and keep the work counters when a trial fails.
- Verification: all 54 cases of the E-06 harness, including the 18 with a vector norm of 1e8, reach a relative error below `1e3 * eps`.

### WU-6. Output-policy protocol and clipped controller. `F-007` (P1), `F-006`, `F-029`, `F-033`, `F-004`. Effort M, risk high for the controller part.

- Evidence: `experiments/E-01/`, `E-02/`, `E-03/`. Design: `fixes/G1_output_policy_protocol.json`.
- Code: `crates/rodas5p-fair-ab/src/global_error.rs` near 240 to 284 and 701 to 720, with the 0.1 rule at about line 715. Also `scientific_validity_v2_campaign.rs`, `rodas5p-integrators/src/adaptive.rs` near 264 to 282, and `output.rs` near 281 to 282.
- Add the new admissibility criterion under a new protocol id. Keep the old rule in place so old receipts still validate.
- The controller change alters clipped-lane outputs and breaks the pinned test in `dense_output_v2_contracts.rs` near 343 to 360. Treat this as a separate PR.
- Add a dated addendum to the ledgers in `research/scientific_validity_v2_20260829/`. State that the 0.1 rule also fails for SciPy Radau pairs, that dense global error exceeds the case tolerance in 12 of 18 rows at n = 96, and that the median error ratio against SciPy Radau is 5.3.

### WU-7. Work counter contract. `F-048`, `F-022`, `F-050`, `F-051`. Effort S to M.

- Evidence: `experiments/E-08/`. Design: `fixes/G5_accounting_fairness.json`.
- `jvp_calls` means different things on the two lanes. On the generic path it reported 140 against 760 real JVP callbacks. See `rodas5p-core/src/operator.rs` near 274 to 284 and 123 to 125, and `rodas5p-integrators/src/problem.rs` near 239 to 251.
- Start with one contract test that wraps the callbacks in atomic counters and runs both lanes. It must fail at `b3e8165`.

### WU-8. Build and sealed replay. `F-067`, `F-003`. Effort S.

- `crates/rodas5p-fair-ab/build.rs` calls git with `expect` and `assert`. Make it degrade to an `unknown` provenance value when git is absent, and compute the dirty flag from tracked files only.
- Bisect the failing sealed replay test. `fixes/G6_evidence_process_tests.json` lists `84a3b0f..b3e8165` as the candidate range. Do not change the pinned 18 to 17. Record the first bad commit and the reason as a dated note. Later pinned literals in the same test may also have drifted, because the assertion stops at the first failure.

### After WU-8

Take the remaining P2 and P3 findings from the ledger in severity order. The test plan in `fixes/G6_evidence_process_tests.json` covers the missing accuracy and order tests.

## 4. Blocked items and owner decisions

Do not attempt these.

- **Stage-certificate findings.** `F-057` to `F-064` and `F-101` to `F-104` concern a module that exists only in the owner's local worktree. Its source is not on this branch. Wait until the owner publishes it.
- **Roadmap order.** Reordering M0, M1 and later nodes is the owner's decision. The proposal is in `fixes/roadmap_delta.json`.
- **Cause of the tolerance exceedance.** The audit did not establish why global error exceeds the case tolerance on the semilinear rows (`F-033`). This is an investigation, not a fix. After WU-3, rerun the 18 rows at n = 96 with direct LU and with the new forcing rule. The six reference artifacts are in `research/scientific_validity_v2_20260829/external_reaudit_bundle/reference/selected_raw/`.

## 5. Known limits of the audit

- Each finding had one refuter. Only the top 11 had a second refuter, a severity judge and a provenance check.
- Line anchors were re-opened for a sample of 18 finding blocks.
- The six findings on the forcing rule rest on one harness, `e04_order_krylov`.
- The final check corrected 14 numbers. They are listed in `ledger.corrections` and in report section 8.6. The largest one: the median error ratio against SciPy Radau is 5.3, not 2.6.
- Not run: the controller-exponent mutant, the finite-difference JVP arm, and the exponential arm of the counter experiment.

## 6. Report back

For each work unit, put this in the PR body:

1. Finding ids.
2. Name of the failing test and its output before the fix.
3. Summary of the diff.
4. Commands run, with exit codes.
5. Test counts before and after.
6. Every changed snapshot, with old and new values.
7. What remains unknown.

At the end of the session, give one table of work units with status `done`, `partial` or `blocked`, and the PR number for each.
