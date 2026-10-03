# Integrated DAG: execution status

This file records how the integrated DAG of `INTEGRATED_PLAN.md` (INT-01 to INT-07) was executed on this branch
(`audit/rvj-native-followup-20261003`, PR #70). That DAG merges the open work of the external thread-transfer review
(`20261002_thread_transfer`) with the open items left by the native re-audit (`20261003_native_reaudit`). All three
audit packages are left byte for byte as they were.

- Each research node was preregistered and pushed before its code and runs, has its ledger row in
  `research/LEDGER.jsonl`, and has its results appended below its preregistration.
- One amendment was made before a recorded run (INT-03), and it is in that preregistration.
- Older ledger rows and verdicts are unchanged: L-0038 stays FAIL, and timing authority stays on HOLD.

| DAG node | Covers | Research node | Ledger | Verdict | Finding |
|---|---|---|---|---|---|
| INT-01 | R-NEXT-03 follow-up | `int01_gcrodr_verified_reuse_20261003` | L-0052 | **FAIL** | a 1e-8 check of the carried recycle pair removes 19 of L-0046's 21 failures, including all 13 trajectory failures. Two one-step solves still diverge from a pair whose defect (2.2e-9) is under the tolerance: the recycle projection amplifies the defect relative to the projected residual. Defaults unchanged, aborted cycles traced, no false convergence. The fresh family has no failures, and recycling uses 0.80x cold GCRO-DR's products there. Post hoc (not gated), 1e-10 removes both |
| INT-02 | N3, TF-04, TF-05 | `int02_structural_certificate_20261003` | L-0053 | **FAIL** | the structured diagonal certificate gives bitwise the dense bounds and identical trajectories on 87 candidates (n = 1..64). It stores 2n instead of 2n^2 slots, builds problem and witness once instead of twice, and its operations grow with slope 1.00 (27,200 against 349,760 at n = 64). It fails on the threshold for the *dense* arm's slope (1.76, required >= 1.8) |
| INT-03 | P1 (banded), TF-03 | `int03_banded_fast_20261003` | L-0054 | PASS | the banded pipeline reproduces v2 bit for bit on six shared cases (outputs and `WorkCounters`). Operations per attempt grow with slope 1.007 and 1.004 in n up to 4096. Storage is `n (3l + 2u + 2)`. Pivoting matches dense partial pivoting |
| INT-04 | M1, TF-06, TF-07 | `int04_stage_chart_20261003` | L-0055 | **FAIL** | the adapter keeps its contract: typed statuses, no acceptance field, and the certificate exposes bad and approximate-W candidates. But the polynomial triangular chart's Newton iteration diverges on 3 of 8 cases. Two thresholds were miscalibrated (1e-8 WRMS, a comparison against a binary64 reference), and the accounting check was restricted by the test |
| INT-05 | V1, TF-09 | `int05_nonnormal_metric_bound_20261003` | L-0056 | PASS | a Crouzeix-Palencia bound over a verified numerical-range box of `D A D^-1`, plus an interval Horner enclosure, encloses the 50-digit error in all 66 bounded cases. It pins the VIG-A02 Arnoldi candidate's error at 7.35e-2 from below. It certifies the method's own value at 1.1e-15 under the dyadic metric, where the identity gives no bound |
| INT-06 | TF-05 | - | - | decision | `Q2_ACTIVATION_DECISION.md`: native q=2 stays opt-in; derived margins with the structured certificate are negative for n <= 8 and +0.63 solve units per attempt at n = 16 (ideal workers, no dispatch charge) |
| INT-07 | publication | - | - | see below | publication on PR #70 without force; validation of the final head |

Product changes, each behind an opt-in or research interface with the defaults unchanged:

- `solve_gcrodr_with_policy` with `GcrodrReusePolicy` and aborted-cycle tracing (INT-01). The untraced and traced
  solves are unchanged (contract tests).
- `DiagonalStageProblem`, `InverseWitness::diagonal_structured`, `certify_stage_target_diagonal`,
  `DiagonalQuadraticModel`, `PreparedQ2Certificate` and `Q2Admission::PreparedStructuredCertificate` (INT-02). The
  dense certificate path and its binding are unchanged.
- `integrate_rodas5p_fast_banded_observed` with `BandedJacobian`, driver id `rodas5p-fast-banded-v1` (INT-03). v2 is
  unchanged.
- `stage_chart_candidate` and `StageChart` (INT-04): a research candidate provider, not a solver path.
- `rodas5p_core::nonnormal_certificate::certify_exp_action` (INT-05). The symmetric polynomial certificates and the
  `EstimateOnly` labels are unchanged.

## What the three FAIL verdicts mean

None is a regression in an existing path, and none was turned into a PASS after its run.

- **INT-01.** The reuse check works on the mechanism it targets, between solves. The remaining failures show that
  the right scale for the check is the projected residual, not an absolute defect. That is a design input for a
  future node.
- **INT-02.** The structured path met every claim about itself. The gate also predicted the dense comparison arm's
  growth rate, and that prediction was wrong.
- **INT-04.** The adapter's safety contract held. The finding is that a chart can ruin Newton's convergence even
  when it is regular everywhere. The node also miscalibrated two thresholds.

## Still open after this DAG

- `solve_into` for LGMRES and GCRO-DR (R-NEXT-02 extension).
- A residual-relative GCRO-DR reuse rule (INT-01).
- A decision rule for blown-up candidates in the residual-to-output budget (R-NEXT-01).
- Any q=2 activation at n >= 16, which needs measured dispatch cost and timing authority (INT-06).
- Scaling and squaring for the nonnormal certificate when `tau |W|` is large (INT-05).
- The timing campaign, which stays on HOLD.

## INT-07: publication and validation

Every commit went to `audit/rvj-native-followup-20261003` without force. There is no new branch or PR, and no merge.
The PR stays draft.

**Final code head `cc0cf96`** (local, Rust 1.94.1). All 16 steps exited 0:

- fmt, and clippy `-D warnings` (default and three feature sets);
- workspace tests and three feature-gated test runs;
- ignored tests in the measurement profile, including every research test of this DAG;
- readiness, research-node (56 ledger rows), authority and ignored-in-CI checks;
- the 211 Python tool tests and the audit's algebra checks.

Across all runs, 2,456 Rust tests passed and none failed. Every crate was rebuilt from this worktree first, so no
stale artifact from the shared build directory was used.

**Hosted CI on `cc0cf96`.** Seven checks passed: `a1-inner-tolerance-parity`, `a2-a3-gmres-candidate`,
`audit2-feature-and-usage`, `fresh-clone-build`, `ignored-tests`, `mode-and-authority` and `research-process`. Three
were skipped by their workflows' own conditions: `scientific-execution-aggregate`, `scientific-execution-cells` and
`receipt-validation`.

Before `cc0cf96`, the INT-05 case generator and the INT-01 post-hoc test panicked when run without their output
variables, as the workspace's ignored-test run does. `cc0cf96` makes both skip and keeps the INT-05 cases file
immutable. It changes no recorded command run with its variables set.

Later commits change only this file.
