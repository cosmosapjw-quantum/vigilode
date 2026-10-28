# VigilODE cloud handoff: audit fixes and stage-certificate repairs

You are continuing work on `cosmosapjw-quantum/vigilode` in a cloud session. The repository is a public research project. Two things were published on 2026-09-28:

- **Adversarial audit.** An audit of the solver, with fix designs, is in `research/adversarial_audit_20260927/`. The audit changed no source code.
- **Stage-certificate work.** Work that had existed only in the owner's local worktree is now on GitHub, byte for byte: the stage-certificate module, its tests and proof sources.

Your job is to implement the fixes, one work unit at a time, tests first.

## 0. Starting state

| Item | Value |
|---|---|
| Repository | `cosmosapjw-quantum/vigilode`, public |
| Start branch | `audit/adversarial-audit-20260927` |
| Branch history | Draft PR #42 head `b3e8165`, then audit commits `1aa3d71` and `1366083`, then merge commit `88c4890` of the stage-certificate branch, then this handoff update |
| Stage-certificate branch | `research/audit2-stage-certificate-worktree-20260831`: PR #41 head `9fbdd84`, then `5ca1365` (worktree bytes), then `c4c152c` (predecessor receipts). Do not rewrite it |
| Rust source | PR #40 head `426d37c`, plus the stage-certificate module behind the non-default feature `audit2-stage-certificate` |
| `main` | `8d0c791`, older. Do not base fixes on `main` |
| Toolchain | Rust 1.94.1, pinned in `rust-toolchain.toml`. Normal crates.io access; `.cargo/config.toml` is a placeholder |

### Baseline

Measured at `b3e8165`, before the merge:

| Command | Result |
|---|---|
| `cargo test --workspace --locked` | 438 passed, 0 failed, 2 ignored |
| `cargo test -p rodas5p-integrators --features audit2-research --locked` | 315 passed |
| `cargo test -p rodas5p-integrators --features audit2-bateman-authority --locked` | 321 passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test -p rodas5p-integrators --test v37_continuation_transaction_contracts --locked -- --ignored` | fails, `left: 17, right: 18` at line 110, in dev and optimized profiles |

Measured on merge commit `88c4890`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo test -p rodas5p-integrators --features audit2-stage-certificate --test audit2_stage_certificate_contracts --locked` | 13 passed |
| `cargo clippy -p rodas5p-integrators --features audit2-stage-certificate --all-targets --locked -- -D warnings` | clean |
| `cargo check --workspace --all-targets --locked` | clean |
| `cargo check -p rodas5p-integrators --no-default-features --locked` | clean |
| `python3 tools/validate_audit2_stage_certificate_repair_handoff.py` | `STAGE_CERTIFICATE_REPAIR_HANDOFF_FINAL_VALID` |
| `python3 tools/test_audit2_stage_certificate_repair_handoff.py -v` | 10 tests OK |

The full default suite was not re-run after the merge. The merge touched only `Cargo.lock`, the integrators `Cargo.toml`, a feature-gated `lib.rs` export and new files.

Reproduce the baseline before changing anything. The default suite takes about 9 minutes in the dev profile on 24 cores. If your numbers differ, stop and report the difference.

### Provenance of the stage-certificate bytes

Commit `5ca1365` holds the owner's worktree bytes unchanged: 3 modified tracked files and 9 new files. Its message lists the SHA-256 of every file. `research/audit2_stage_certificate_repair_20260831/EXECUTION_CONTRACT.json` binds eight hashes of that run: the external manifest, `SHA256SUMS`, the formal receipt, and five proof sources. All eight match the published bytes.

## 1. Read first, in this order

Unless noted, paths are relative to `research/adversarial_audit_20260927/`.

1. `VIGILODE_ADVERSARIAL_AUDIT_20260927.md`, chapters 1, 7 and 8. The report is in Korean. Chapter 1 is the summary, chapter 7 the improvement plan, chapter 8 the limits and the corrections made in the final check.
2. `VIGILODE_ADVERSARIAL_AUDIT_LEDGER_20260927.json`. Machine-readable. `findings[]` carries id, severity, status, locations, evidence, refutations and `proposed_fix`. Also read `improvement_plan`, `experiments`, `hypotheses_tested`, `corrections`, `nonclaims`. The schema is next to it.
3. `fixes/G1_*.json` to `fixes/G6_*.json` and `fixes/G3b_*.json`. One file per fix group, with anchors, effort, risk, verification and code sketches. `fixes/roadmap_delta.json` and `fixes/completeness_critic.json` hold the roadmap proposal and the audit's own gaps.
4. `experiments/E-xx/README.md` and `results.json`. The evidence and the numeric baselines for verification.
5. For WU-9 only, the stage-certificate material, with paths relative to the repository root:
   - `research/audit2_stage_certificate_repair_20260831/`: PR #42's control package. Read `FORMAL_SCOPE.md`, `EXECUTION_CONTRACT.json` (`repair_scope`) and `CLAIM_LEDGER.md`.
   - `research/audit2_stage_certificate_telemetry_20260831/predecessor_run_20260831T141627Z/`: receipts of the run that ended `STOP_INVALID`. Read `reviews/*.json` first.
   - The owner's M0 plan, which lives on another branch:
     ```bash
     git fetch origin planning/vigilode-research-coding-spiral-20260904
     git show FETCH_HEAD:docs/superpowers/plans/2026-09-04-vigilode-m0-stage-certificate-closeout.md
     ```

**Path mapping.** The ledger and report cite stage-certificate files as `overlay/untracked/<path>`. On this branch the file is at `<path>`. Strip the prefix.

**Harness.** `harness/` is a standalone crate with the experiment binaries. It is not a member of the repository workspace. Keep its `target/` untracked: `rodas5p-fair-ab/build.rs` embeds a dirty flag from `git status`, and any untracked file flips it. Write experiment outputs outside the repository.

```bash
cd research/adversarial_audit_20260927/harness
cargo build --locked --profile measurement
```

Run the binaries with `RAYON_NUM_THREADS=1`. Each experiment README lists the arguments its binary expects. Files above 300 KB were not checked in. Regenerate them with the Python scripts in the experiment directories.

## 2. Rules

These follow the project's own governance. Do not break them.

- **Test first.** Every work unit starts with a failing test that encodes the defect. Then make the smallest fix. Then run the focused suite, the full workspace suite, clippy with `-D warnings`, and fmt.
- **One work unit, one branch, one Draft PR.** Base each PR on `audit/adversarial-audit-20260927`. Stack a unit on an earlier one only when both touch the same file. Never merge, tag or release. Never push to `main`, to `research/audit2-*` branches, or to `research/audit2-stage-certificate-worktree-20260831`.
- **No tolerance widening.** Do not loosen a tolerance, threshold or acceptance rule to make a test pass. If a fix changes a pinned snapshot, record the old value, the new value and the reason in the PR body.
- **Ledgers are append-only.** Add dated addenda under `research/`. Never rewrite a historical ledger, receipt or the audit ledger.
- **Leave PR #42's control files alone.** `research/audit2_stage_certificate_repair_20260831/HANDOFF_INPUT_LOCK.json` binds nine files by SHA-256. Do not edit them. `python3 tools/validate_audit2_stage_certificate_repair_handoff.py` must keep passing.
- **No performance claims.** No speedup, ranking, scaling or holdout claim. Do not read `tools/reference_v2/artifacts/*-holdout-v2.json` for any tuning purpose.
- **No candidate execution.** Zero Bateman real-client candidate executions, as in PR #40–#42. The claim ceiling stays `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`.
- **Preserve failures.** If a unit cannot be finished, report what was tried and what failed. Do not drop it silently.
- **Re-anchor line numbers.** Every line number in the audit refers to `b3e8165` or to the stage-certificate bytes. Confirm with `grep -n` before editing.
- **Keep defaults stable.** Default behavior for library callers must not change unless the work unit says so. Research code stays behind its feature flag.

## 3. Work units

The order is a recommendation, and the owner may change it. WU-1 to WU-5 contain the four P1 findings. Effort uses the scale XS, S, M, L, XL.

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

### WU-9. Stage-certificate repairs. `F-057`, `F-060`, `F-061`, `F-059`, `F-064`, `F-101`, `F-102`, `F-104`, `F-058`. Effort M for the software part, L for the proofs.

The owner's Sep-4 plan calls this work M0 and schedules it first. The audit places it after the P1 fixes. Follow the owner if they say otherwise.

- Code: `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs` and `tests/audit2_stage_certificate_contracts.rs`, behind the feature `audit2-stage-certificate`.
- Scope: `repair_scope` in `research/audit2_stage_certificate_repair_20260831/EXECUTION_CONTRACT.json`, `FORMAL_SCOPE.md` in the same directory, and Tasks 7 to 10 of the M0 plan.

Software repairs. Write the failing test first for each.

1. Enforce `max_arnoldi` per completed GMRES trace row, independent of the iteration limit and the restart length (`F-057`; the predecessor review rated this P1). Near lines 378 to 382 only the ordering is validated, and near 413 only `iteration_limit` bounds a trace.
2. Round-trip the receipt through `serde_json`: serialize, deserialize into the same type, compare structurally, and reserialize canonically (`F-102`). The current test near line 280 compares in-memory structs.
3. Fix directed rounding. Near line 561 the helpers bump every nonzero value by one ulp even when the result is exact, and a contract test pins that bump (`F-101`). Round nonnegative add and multiply correctly upward. Keep exact zero and exactly representable results unchanged. Map overflow to `+inf`, then to a typed rejection with no decision (`F-104`). Near line 504, residuals and norms are still rounded to nearest (`F-061`); extend directed rounding to them or document the gap in the claim ledger.
4. Near line 300, one linear solve's residual bound is copied to every stage with `vec![q_upper; dimension]`. Near 258 to 268 the stage count is confused with the state dimension (`F-060`). Separate the two quantities.
5. The only accepting case in the contract test near line 24 rests on a `kappa` premise that does not hold for its fixture matrix (`F-059`). Rebuild the fixture with a `kappa` that bounds the inverse, and add a negative test with a false `kappa`.
6. Near line 433 the provenance digest binds coefficients, operator, preconditioner and right-hand side only. Bind the inputs that drive the decision too: `T`, the weights, `kappa`, `ehat` and `x` (`F-064`).

Formal repairs (`F-058`, predecessor formal P1):

- The Lean and Rocq sources in `research/audit2_stage_certificate_telemetry_20260831/formal/` prove F01, F03 and F04 only for a fixed 3 by 3 case or numeric fixtures. Prove them for arbitrary finite `n >= 1`, as `FORMAL_SCOPE.md` states. F05 is already quantified.
- This needs Lean 4 with mathlib (via `elan` and `lake`) and Rocq (`coqc`, `coqchk`). Wolfram, SageMath and Singular are cross-checks only. If a tool cannot be installed, record that backend as unavailable. Do not substitute another tool and do not claim the proof.
- `tools/run_audit2_stage_certificate_formal.py` runs all five backends. Read it before use; it expects a mathlib project path.

PR #42's contract, and what it means here:

- The contract names `LOCAL_CODEX_JOB_ONLY` as executor.
- It requires two harness archives that are not in the repository.
- It requires 13 backend-role records, including Wolfram.
- The owner moved the work to the cloud on 2026-09-28 but has not amended the contract.

So open WU-9 as an ordinary Draft PR based on the audit branch. Then:

- Do not write under `research/audit2_stage_certificate_repair_20260831/evidence/`.
- Do not claim `REPAIR_CLOSEOUT_VERIFIED` or any other PR #42 terminal disposition.
- Report what passed and what could not run.

The owner decides whether the result closes PR #42.

### After WU-9

Take the remaining P2 and P3 findings from the ledger in severity order. The test plan in `fixes/G6_evidence_process_tests.json` covers the missing accuracy and order tests.

## 4. Owner decisions and open investigations

Do not decide these yourself.

- **Roadmap order.** Reordering M0, M1 and later nodes is the owner's decision. The proposal is in `fixes/roadmap_delta.json`.
- **PR #42 closeout.** Whether WU-9 closes PR #42, and under which amended contract, is the owner's decision.
- **Cause of the tolerance exceedance.** The audit did not establish why global error exceeds the case tolerance on the semilinear rows (`F-033`). This is an investigation, not a fix. After WU-3, rerun the 18 rows at n = 96 with direct LU and with the new forcing rule. The six reference artifacts are in `research/scientific_validity_v2_20260829/external_reaudit_bundle/reference/selected_raw/`.

## 5. Known limits of the audit

- Each finding had one refuter. Only the top 11 had a second refuter, a severity judge and a provenance check.
- Line anchors were re-opened for a sample of 18 finding blocks.
- The six findings on the forcing rule rest on one harness, `e04_order_krylov`.
- The audit reviewed the stage-certificate module statically. It was first compiled and tested on this branch at the merge commit.
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
