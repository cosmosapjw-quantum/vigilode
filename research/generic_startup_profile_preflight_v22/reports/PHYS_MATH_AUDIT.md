# PHYS–MATH AUDIT — G4-S5B2 Preflight

## PASS

1. Cost authority uses wall / proposed physical interval, not raw current-step wall.
2. Regret and overhead ratios are dimensionless.
3. E-K inadmissible steps cannot be counted as E winners.
4. Failed E work and R fallback work remain in total cost.
5. The complete-future profile oracle is explicitly labeled nonrealizable and used only as an upper bound.
6. No global-order or switching claim is made because no actual solution switching occurs.

## P0 finding

A single startup decision cannot follow transition-bearing trajectories. This remains true even with complete future knowledge.

## P1 finding

Short trajectories cannot amortize a dual or one-sided startup prefix under the existing 3%/5% gate.

## Hidden assumptions

- Replay assumes method-local next-step estimates from immutable campaigns.
- Future states are not recomputed under actual switching.
- The optimistic re-entry lane is an upper bound, not a deployable controller.

## Verdict

- startup-only architecture: FAIL
- event-triggered re-entry hypothesis: PASS WITH CONSTRAINT
- actual prefix bake-off at this node: NOT JUSTIFIED
