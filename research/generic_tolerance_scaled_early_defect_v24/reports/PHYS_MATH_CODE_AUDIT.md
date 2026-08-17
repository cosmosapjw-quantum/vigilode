# P1-00 PHYS-MATH-CODE AUDIT

## Verdict

**PASS WITH ACTIVATION HOLD.** The implementation preserves the numerical path
and adds tolerance scaling without duplicating expensive E-K work. The next
admissible step is a read-only event-conditioned shadow, not active dispatch.

## Equation-to-code map

- controller scale authority: `rodas5p-core/src/norms.rs::error_scale`;
- raw level-1 telemetry: `rodas5p-integrators/src/exponential.rs`;
- scaled score: the same `error_scale` helper evaluated on the physical prefix
  `(y_n, U2)` and WRMS of `|h| D2`;
- adaptive tolerance source: `AdaptiveStepConfig::{atol,rtol}` passed directly
  into the scaled telemetry path;
- attempt/outcome ledger: `adaptive_exponential.rs`;
- frozen 12-trajectory measurement: `g4_s5b3_attempt_geometry.rs`;
- machine-readable CLI: `generic-tolerance-scaled-early-defect`.

## Numerical identity

Focused tests demonstrate exact disabled/read-only parity for:

- final state and embedded state;
- error vector / phi reports;
- requested outputs;
- accepted/rejected attempt sequence;
- controller-generated step history;
- existing `WorkCounters`;
- per-trial work summed against aggregate work.

Canonical report hard gates pass on all 12 trajectories. No scaled score is used
inside acceptance, step-size control, or state mutation.

## Work accounting

Additional canonical solver work remains exactly zero:

```text
RHS calls             0
JVP calls             0
JVP vectors           0
phi actions           0
partial-t calls       0
Jacobian builds       0
Newton iterations     0
```

Only component scale evaluations and one WRMS reduction are added to the
separate diagnostic ledger. This statement applies to telemetry attached to a
**full E-K attempt**. In a future early-abort state machine, the E-K level-1
prefix needed to obtain `U2,D2` is speculative probe work and must be charged in
full; it cannot be relabelled as zero-cost telemetry.

## Regression evidence

Focused and compatibility gates pass:

- 3 scaled score contract tests;
- 1 scaled campaign smoke test;
- 4 legacy raw telemetry tests;
- 1 legacy raw geometry smoke test;
- scaled CLI schema test;
- legacy serde/fail-closed test;
- raw CLI regression test;
- measurement-profile Clippy with `-D warnings`;
- formatting check.

The full release/serial/doc closeout receipts are recorded separately and are
required before final node promotion.

## Performance honesty

Seven paired optimized measurement rows give a median scaled-read-only/disabled
wall ratio of about `1.0033`, MAD `0.0129`, and observed P95 `1.0637`.
This is too noisy to claim either exact zero overhead or a robust `<5%` tail.
The median suggests small cost, but the next shadow node must budget the actual
E-K prefix plus telemetry and retain failed speculative work.

## P0-P3 ledger

- **P0: none found** in numerical identity, score coverage, or expensive-work
  accounting.
- **P1:** corpus concentration and lack of a perfect/selected threshold;
  entry-window observations are not yet real R-JF re-entry events.
- **P1:** future early-abort bookkeeping must charge the E-K level-1 prefix.
- **P1:** scalar `atol` limits heterogeneous-unit genericity.
- **P2:** paired timing tail is noisy and presently exceeds a strict 5% P95
  interpretation if such a gate were imposed.
- **P3:** vector-`atol` support and broader dimensions/backends remain later
  generalization tasks.

## What is genuinely fixed

The exact failure mode identified in P0-05—raw score invariance across tolerance
at the same physical attempt—is fixed by construction and by the frozen
quadratic triplet test.

## What remains uncertain

Whether a **real event-conditioned** R-JF-default shadow policy can use this
feature economically. P1-00 contains startup/entry E-K attempts, not actual
post-residence re-entry events, so the two concepts must not be conflated.
