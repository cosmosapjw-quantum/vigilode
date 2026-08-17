# P1-00 PHYS-MATH AUDIT — tolerance-scaled early-flow defect

## Verdict

**PASS WITH INTERPRETATION CEILING.** The tolerance-scaled diagnostic repairs the
known cross-tolerance information loss of raw `eta_c2` and survives both
predeclared threshold-free views. It is still a ranking/admission feature, not a
local-error theorem, and no numerical threshold is promoted.

## Authority and definition

The repository controller scales an error component with

\[
s_i=\mathrm{atol}+\mathrm{rtol}\max(|y_{n,i}|,|y_{n+1,i}|).
\]

The sealed early diagnostic uses the same functional scale but substitutes the
already-available dependency-level state `U2` for the unavailable final state:

\[
\rho_{c_2}^{\mathrm{WRMS}}
=\left[\frac1m\sum_{i=1}^m
\left(\frac{|h|D_{2,i}}
{\mathrm{atol}+\mathrm{rtol}\max(|y_{n,i}|,|U_{2,i}|)}\right)^2\right]^{1/2},
\qquad c_2=\frac14.
\]

The physical component prefix is used; a time-augmentation clock component is
excluded.

## Definition / sign / unit checks

1. `D2` has units state/time, so `|h| D2` has state units.
2. Each scale denominator has the corresponding state units when `atol` is
   dimensionally compatible. Hence every summand and `rho_c2_wrms` are
   dimensionless.
3. The absolute step magnitude and squared WRMS make the scalar insensitive to
   a pure sign flip of `D2`; this is correct for a magnitude diagnostic and must
   not be interpreted as directional information.
4. For an exactly linear/affine local flow, the nonlinear remainder tends to
   zero and the score tends to zero, matching the raw-defect limit.
5. `rho=1` has **no special acceptance theorem** here. The score uses an early
   nonlinear remainder, not the embedded final local error.
6. Scalar `atol` remains a genuine scope limitation: heterogeneous-unit state
   vectors require compatible nondimensionalization or future vector-`atol`
   support before a generic scale-invariance claim.

## Frozen-corpus kill tests

All predeclared kill tests passed.

- 12 trajectories, 248 attempts, 241 accepted, 7 rejected.
- 248/248 finite raw scores and 248/248 finite scaled scores.
- zero unscorable attempts.
- the quadratic first-attempt raw-score collision remains exactly as a control:
  `eta_c2 = 9.919814843468982e-4` at all three tolerances.
- the scaled triplet becomes approximately
  `0.15347`, `15.3474`, `1534.7374` as `rtol` tightens from `1e-4` to `1e-8`;
  the adaptive total-error norm moves in the same direction
  (`0.00310`, `0.23414`, `23.4141`).

This proves only that tolerance information has been restored to the diagnostic,
not that the two quantities are equal or asymptotically proportional.

## Threshold-free geometry

### All attempts

Raw control:
- rank AUC: 0.7232;
- Spearman against total-error norm: 0.1453;
- minimum total misclassification: 7, no better than all-admit;
- zero-false-accept descriptive point rejects 201/241 accepted attempts.

Scaled score:
- rank AUC: 0.8020;
- Spearman: 0.3562;
- minimum total misclassification: 4, strictly better than the all-admit count 7;
- zero-false-accept descriptive point rejects 185/241 accepted attempts.

Thus the all-attempt score survives the literal kill test but remains too blunt
for a safe blanket admission rule.

### Entry window: attempt index 0 or 1

There are 24 predeclared entry rows: 17 accepted and 7 rejected. All seven
canonical rejections are in this window.

Raw control:
- rank AUC: 0.9580;
- Spearman: 0.7504;
- minimum misclassification: 1;
- zero-false-accept retains 11/17 accepted entries.

Scaled score:
- rank AUC: 0.9832;
- Spearman: 0.8713;
- minimum misclassification: 1;
- zero-false-accept retains 15/17 accepted entries.

The scaled entry view therefore improves the conservative frontier materially,
but no perfect global threshold exists: its Pareto count frontier includes a
zero-false-accept point with two false rejects and a one-false-accept point with
zero false rejects.

## Concentration / generalization cautions

- 208/248 attempts are oscillatory-PR.
- 154/248 attempts are at `rtol=1e-8`.
- 6/7 rejections are oscillatory-PR and 1/7 is quadratic.
- No rejection is observed for complex-Dahlquist or advection-diffusion.
- Attempts within a trajectory are serially dependent. AUC, KS and Spearman are
  descriptive diagnostics, not independent-sample inferential statistics.

Leave-one-problem-out analysis is encouraging for the **entry** view: the scaled
score remains strictly better than all-admit for every left-out problem, with
zero-false-accept retained fractions from about 0.818 to 1.0. The all-attempt
view is much less useful, with retained fractions as low as about 0.163.

## Claim ceiling

Allowed:
- tolerance scaling removes the demonstrated cross-tolerance degeneracy;
- the scaled score has better descriptive entry-window geometry on the frozen
  corpus;
- it warrants a read-only event-conditioned shadow experiment.

Not allowed:
- `rho<=1` or any other universal acceptance rule;
- a selected production threshold;
- a local-error bound theorem;
- active switching, speedup, or generic universality.
