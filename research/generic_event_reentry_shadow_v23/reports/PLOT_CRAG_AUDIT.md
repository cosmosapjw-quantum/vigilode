# Plot-Based CRAG Adversarial Audit — G4-S5B3

## Correctness

The pooled CDF and complete false-count curve agree with the CSV rows. The
accepted and rejected distributions overlap substantially. The Pareto frontier
contains only five count pairs:

```text
false accept / false reject
0 / 201
1 / 99
3 / 51
4 / 6
7 / 0
```

The exact-collision plot independently exposes the strongest counterexample:
the same raw score crosses the adaptive acceptance boundary solely because the
tolerance changes. Coverage bars are identically one for all 12 trajectories,
confirming that the negative result is not caused by missing telemetry or work.

## Retrieval

The canonical fused method is a four-stage fifth-order exponential Rosenbrock
candidate whose dependency structure permits three sequential evaluation
levels. The present diagnostic is sampled from the first dependency level. This
method structure explains why the measurement is early and cheap; it does not
make the first-level nonlinear remainder an embedded local-error estimator.

## Augmented checks

- Per-problem plots show that only oscillatory-PR and quadratic supply rejected
  rows.
- Per-tolerance plots do not produce a stable separating gap.
- Removing oscillatory-PR leaves only one rejected example, so pooled separation
  is not family-robust.
- 83.9% problem concentration and 62.1% strict-tolerance concentration are
  visible in dedicated count plots.
- Alternating timing order crosses both sides of unity; the observed spread is
  consistent with measurement noise rather than systematic telemetry gain or
  loss.

## Generation

The evidence predicts that a useful early diagnostic must retain the cheap
first-level remainder but add the missing adaptive scale. A natural next
contract candidate is a component-wise tolerance-scaled score such as

\[
\rho_{c_2}^{\rm WRMS}
 =\left[\frac1m\sum_{i=1}^m
 \left(
 \frac{|h|D_{2,i}}
 {\mathrm{atol}_i+\mathrm{rtol}_i\max(|y_{n,i}|,|U_{2,i}|)}
 \right)^2\right]^{1/2},
\]

computed from already available \(y_n,U_2,D_2\) with no new RHS/JVP/phi action.
This remains a diagnostic hypothesis, not a local-error theorem.

A second prediction is that event conditioning matters: all observed rejects
occur in the first two attempts at \(t=0\). This must be tested by a sealed
entry/re-entry shadow protocol, not selected post hoc from this campaign.

## Adversarial mutations

1. **Tolerance mutation:** exact same score, same problem, state, and proposed
   step changes from accept to reject. Blanket global-score claim fails.
2. **Problem removal:** removing oscillatory-PR removes six of seven rejects.
   Generic separation claim fails.
3. **All-admit control:** any threshold that catches at least one reject raises
   or preserves total false-count relative to all-admit. Economic classifier
   claim fails on the full-attempt corpus.
4. **Timing-order reversal:** ratios move above and below one. Negative-overhead
   claim fails; only negligible-resolved-overhead survives.
5. **Missing-data mutation:** no rows are missing, so coverage cannot explain
   the negative geometry.

## Claim status

- **Surviving:** telemetry is cheap, complete, and numerically read-only.
- **Narrowed:** pooled rank separation exists descriptively but is not an
  admissible blanket rule and is concentrated in one family.
- **Rejected:** raw \(\eta_{c_2}\) alone supports a generic global threshold.
- **Still open:** tolerance-scaled and event-conditioned read-only diagnostics.
