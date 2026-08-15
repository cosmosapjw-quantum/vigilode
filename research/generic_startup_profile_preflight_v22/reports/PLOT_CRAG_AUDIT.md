# Plot-based CRAG Audit — G4-S5B2

## Correctness

The startup regret plot shows multiple trajectories where even the perfect fixed-profile oracle exceeds both regret gates, while the re-entry upper bound collapses most bars near one. This matches the tabulated decomposition.

## Retrieval

The prefix-overhead plot shows that the largest overhead outliers are concentrated at 3–6 accepted steps. Long trajectories lie near or below the gate.

## Augmented

Seven campaigns reproduce the same architecture verdict: fixed profile P95 regret remains above 1.5, while re-entry P95 stays below the gate.

## Generation

The surviving design must combine:
- short-horizon admission that skips startup probing;
- safety-triggered E→R fallback;
- bounded R→E re-entry after cooldown;
- actual reusable prefix continuation.

## Adversarial mutations

- Removing prefix cost does not fix fixed-profile P95 regret: regime drift remains.
- Perfect future profile choice does not fix the architecture.
- One-sided prefix reduces overhead but not drift.
- Increasing cooldown worsens missed E residence; decreasing it risks repeated failed entry.

## Claim status

- surviving: event-triggered re-entry is worth a bounded shadow node;
- narrowed: prefix bake-off is useful only when horizon and change-point economics justify it;
- rejected: one-time startup profile without periodic/event review.
