# BASS/rei P02b fixed-grid adapter (corrected C6): preregistration

Node `bassrei-p02b-fixed-grid` (BASS/rei research DAG node P02, fix node). Ledger row **L-0149**
(pre-assigned), which **supersedes L-0148** (P02, verdict FAIL). The L-0148 row stays in the ledger.
This file was written, committed and pushed before the measurement run. The results are appended below
the line "Results" after the run, and nothing above that line changes.

Code under test: branch `research/bassrei-port-20261021` at `7dec6aef858f5fb9a8ac90cb1b79dac28954bf54`.
The harness is `research/bassrei_p02b_fixed_grid_20261022/run_p02b.py`. The adapter, runner, Rust tests
and reference module are byte-identical to the P02 code under test `e2258191`, and the reference module
is imported read-only from `research/bassrei_p02_fixed_grid_20261021/reference/`. P02's own files are
unchanged.

## Question, mathematics, rules, holdout, host and timing, known limitations

These are identical to `research/bassrei_p02_fixed_grid_20261021/PREREGISTRATION.md` as committed at
`d66badadbdc368a6c87d2c5c988633ada8b1b250` (everything above its "Results" line), which is incorporated
here by reference. The kill gate is the same C1–C10, with one change.

## The single change (C6 reference uncertainty)

In P02, u_ref was defined as "VigilODE dense at rtol 1e-12 vs Radau(1e-13)". The P02 post-verdict
diagnosis (`research/bassrei_p02_fixed_grid_20261021/POST_VERDICT_DIAGNOSIS.json`) showed that this
quantity is the method's own tight-tolerance roundoff floor at Γ = 1e6 (about 1e-11), not the
reference's uncertainty: Radau converges to about 4e-13. P02b therefore uses:

- **C6 reference:** Radau at rtol 1e-14 (scipy clamps it to 2.22e-14), with the analytic sparse Jacobian
  as before.
- **C6 reference uncertainty:** u_ref = max-over-outputs relative ℓ1 of Radau(1e-13) vs Radau(1e-14). It
  must be ≤ 0.25 × the smallest measured error. The 0.25 factor is unchanged.
- **[H] only:** VigilODE dense at rtol 1e-12 vs the reference, recorded as
  `H_method_tight_tolerance_floor`.

Every other C6 criterion and every other gate, threshold, scenario and command option is unchanged.

## Disclosure (this is not a blind test)

- Every P02 outcome is known. All gates except C6 passed, and C6 failed only on its u_ref criterion.
- The adapter and runner are deterministic (C9), so the P02b runs are expected to reproduce P02's
  VigilODE trajectories and counters bit for bit.
- The post-verdict diagnosis already computed Radau(1e-13) vs Radau(1e-14) for S2: 3.6e-13, against
  0.25 × 3.61e-11 = 9.0e-12.
- P02b confirms the corrected criterion and the reproducibility of every other gate in a full,
  independent run. It supplies no new blind evidence about C6.

## Verdict rule

As in P02. **PASS iff C1–C10 all pass**, never re-scored. INVALID on a dirty tree, a stale binary or a
non-empty `raw/`. If the run cannot complete, no ledger row is written. A FAIL keeps P03–P06 blocked;
a PASS unblocks them under the approved plan.

## Command

Run from the worktree root at the code-under-test commit plus this file, with no other changes:

    python3 research/bassrei_p02b_fixed_grid_20261022/run_p02b.py \
      --out research/bassrei_p02b_fixed_grid_20261022/results.json

The harness builds the runner, runs the cargo contract tests, and writes `results.json` plus
`raw/MANIFEST.json`. The defaults are the same as P02.

## Outputs

`results.json`, `raw/` (with `MANIFEST.json`) and `DAG_STATUS.json`, all in
`research/bassrei_p02b_fixed_grid_20261022/`.
