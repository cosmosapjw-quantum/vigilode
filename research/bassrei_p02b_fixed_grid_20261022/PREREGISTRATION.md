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

---

## Results

Run on the clean tree at `c4d9eb9f76ac1fbcbb4a12fb920d05e4c2317fa9` (harness `7dec6aef` plus this file;
code under test unchanged since P02). The command was exactly the preregistered one. Wall time was
1101 s on the local workstation and is diagnostic only. The harness built the runner itself and ran the
cargo contract tests, 14/14 ok with 0 ignored. The tree was clean at start and end, and no input
changed during the run.

Output hashes:

- `results.json`: `0b7b3295f6896d12a65c5104102f218f96497ccebec5da0f8deba79b4454a4f0`
- `raw/MANIFEST.json`: `591e8bc60894aae683d93fbb16a0c666a106c06b5a2a8b6315232c48dab06788`; it covers
  106 raw files, all of which match.

**Verdict: PASS**: every [C] gate passed, per the verdict rule.

| gate | pass | key numbers |
|---|---|---|
| C1 | True | 22/22 fixtures; transport bitwise = oracle; ledger per-boundary ≤ 3.7e-16; RHS/Jacobian vs Python A(t) ≤ 5.6e-16 |
| C2 | True | cargo |
| C3 | True | 47 runs; max drift 4.67e-11, under every per-run bound |
| C4 | True | orders G2→G3 / G3→G4 / G4→G5, rising: R+ 0.663/0.786/0.875; R− 0.664/0.799/0.884; C+ 0.568/0.727/0.827; R+abs 0.640/0.774/0.870; R+abs-com 0.851/0.934/0.970 |
| C5 | True | last pre-floor slopes C+_g0 5.825, C+_g0.5 5.718, R+_g0 4.997, R+_g0.5 4.985; negative control (drop f_t) 1.989 |
| C6 | True | see the table below |
| C7 | True | M2 decay observed/predicted at end 1.0000000256; M0 relative drift 1.2e-15 |
| C8 | True | cargo |
| C9 | True | cargo |
| C10 | True | cargo |

C6, with errors at rtol 1e-4, 1e-6 and 1e-8 (banded bit-identical to dense in every rung):

| scenario | errors | u_ref (Radau 1e-13 vs 1e-14) | 0.25 × min error | [H] method floor |
|---|---|---|---|---|
| S1_R+_phys_g1e4 | 1.09e-07, 2.18e-09, 2.81e-11 | 7.0e-15 | 7.0e-12 | 1.8e-13 |
| S2_R+_phys_g1e6 | 1.08e-07, 2.17e-09, 3.61e-11 | 3.6e-13 | 9.0e-12 | 1.2e-11 |
| S3_R+_com_abs1e4_g0.5 | 6.40e-07, 1.40e-08, 5.09e-11 | 1.0e-14 | 1.3e-11 | 4.0e-14 |

The fixed-step banded run reproduces the fixed-step dense run exactly (endpoint difference 0, 64/64
steps). The [H] method floor is VigilODE dense at rtol 1e-12 vs the Radau reference. At Γ = 1e6 it is
1.2e-11, a real double-precision accuracy floor of RODAS5P on this problem, reported and not gated.

**Consequence (approved plan):** P03 and P04 are unblocked. This row supersedes L-0148, which stays in
the ledger unchanged. No claim is made about live BASS/rei integration, physical rates or wall time.
Broad production remains HOLD.
