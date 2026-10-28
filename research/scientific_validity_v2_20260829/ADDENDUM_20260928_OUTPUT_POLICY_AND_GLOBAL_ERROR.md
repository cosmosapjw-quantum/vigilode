# Addendum 2026-09-28: output-policy rule and global error vs case tolerance

This addendum is new and append-only. It amends no existing file in this
directory. `CANONICAL_EXECUTION_EVIDENCE.md`, `CLAIM_SCOPE_AND_INVALIDATION.md`,
`PROTOCOL.md` and the committed 54 records stay as they are, and so does their
54/54 `output-policy-dominated` outcome under the frozen rule.

Audit findings: F-007, F-004, F-029, F-033 and F-006, from
`research/adversarial_audit_20260927/`.

## Source of the numbers

Every number below is printed by `addendum_20260928/regenerate_numbers.py`.
That script reads only these two files:

- `research/adversarial_audit_20260927/experiments/E-02/results.json`
- `research/adversarial_audit_20260927/experiments/E-03/results.json`

Its output is saved in `addendum_20260928/numbers.txt`.

About the inputs:

- E-02 re-ran the 18 n=96 rows at tree `b3e8165`. It reproduced the committed
  `ab8fbcd` arm checksums bitwise, 36 of 36.
- E-03 is the audit's SciPy Radau run (scipy 1.17.0) on the same six families.
  It uses each case's own (rtol, atol) and an analytic Jacobian.
- The SciPy numbers come from that audit run. They do not come from the 54
  committed external-calibration records.

On units: the published `*_wrms` values use the tight reference basis
(atol 1e-10, rtol 1e-8, anchored on the reference). They are not in
case-tolerance units. The campaign's case weights are (0.01·rtol, rtol), which
is proportional to the tight basis. So a value in case units equals the tight
WRMS times 1e-8/rtol.

## 1. The frozen 0.1 rule also fails for SciPy Radau

The frozen rule is `gap <= 0.1 * dense error`. It compares two trajectories
that were stepped independently and both carry O(tol) global error, so their
difference is also O(tol).

SciPy Radau breaks the same rule on its own policy pairs:

- First-step pair `h0` against `0.7 h0`: 17 of 18 admissible pairs violate it.
  The median gap/error ratio is 1.29.
- `max_step = span` against `max_step = grid spacing`: 15 of 17 admissible
  pairs violate it.

So the rule cannot be met by construction. The committed 54/54
`output-policy-dominated` result says nothing about solver accuracy or about
how output is handled.

## 2. Dense global error exceeds the case tolerance in 12 of 18 rows at n = 96

In case-tolerance units, the dense arm's max-grid error is above 1 in 12 of the
18 n=96 rows.

The largest values are in the semilinear advection-diffusion family:

| rtol | dense error, case units |
|---|---|
| 1e-4 | 17 |
| 1e-6 | 53.2 |
| 1e-8 | 207 |

Every one of the 36 n=96 arms had a maximum accepted embedded error norm of at
most 1. The error grows globally while local error control is satisfied.

## 3. The median error ratio against SciPy Radau is 5.3

At equal (rtol, atol) on the same problems:

- The VigilODE dense-arm max-grid error is larger than SciPy Radau's in 17 of
  18 n=96 rows.
- The median per-row ratio is 5.28. The largest is 438.3, at semilinear
  rtol 1e-8.
- SciPy Radau's largest error in case units is 1.57, and it exceeds 1 in only
  1 of 18 rows.

The "~2.6x" side observation in the E-03 README is superseded by the
recomputed per-row median above. The audit ledger records the same value, 5.3.

## Attribution: open

This addendum does not say why the error exceeds the tolerance. The cause is an
owner decision. The candidates named by the audit are:

- inexact stage solves, which the embedded estimator cannot see;
- order reduction.

Neither has been measured on these rows. E-03c rules out the dense interpolant:
on the semilinear rows, the interpolated-grid error is 1.00 to 1.02 times the
accepted-endpoint error.

## Replacement criterion (code only, not run)

The replacement rule is declared under a new protocol id,
`TWO_ARM_ADMISSIBILITY_PROTOCOL_ID`, in
`crates/rodas5p-fair-ab/src/output_admissibility.rs`. The frozen
`classify_output_policy_dominance` rule is unchanged.

The new rule has three parts:

- **Per-arm budget.** Each arm is checked against a budget B = 10 in case units,
  with reference-uncertainty bands.
- **Gap as an integrity check.** The clipped-dense gap is checked against the
  triangle bound, not used as a gate.
- **Interpolant check.** A row without the same-step interpolant check cannot
  pass.

It has been applied offline to the E-02/E-03 records:

| Arm | Within budget | Exceeds budget |
|---|---|---|
| VigilODE dense | 15 of 18 | 3 of 18 (the semilinear rows) |
| SciPy Radau | 18 of 18 | 0 |

No campaign row has been executed under the new protocol. The same-step
interpolant delta is not computed yet. No freeze, holdout, performance,
ranking or scaling claim follows from this addendum.
