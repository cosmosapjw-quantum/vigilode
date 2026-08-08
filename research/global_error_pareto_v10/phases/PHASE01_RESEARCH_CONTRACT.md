# Phase 1 — Research contract

## Question

How should fixed-step and later adaptive ODE/DAE integrators of different families be compared under one external accuracy authority without conflating internal tolerance semantics, interpolation quality, startup/history cost, or backend-specific work counters?

## Scope

This cycle implements only the comparison authority and a fixed-step anchor replay. It does not alter any numerical solver, add variable-order BDF, add adaptive Radau, or promote a solver.

## Protected baseline

- tag: `unified-native-v0.9.0-alpha1`
- branch start: `68fe67260befd3901cf0b211a46c1d0d85c03932`
- existing RODAS5P, SABR, homotopy, BDF1/2, Radau IIA1/3 algorithms and counters are read-only inputs.

## Required outputs

1. Method-independent external global-error definitions.
2. Reference-solution provenance and uncertainty budget.
3. Complete integrator work/timing record.
4. Failure-preserving fixed-step anchor replay.
5. Per-cost Pareto fronts and target-matched winners without a composite cost score.
6. Deterministic 1-thread/4-thread scientific output.

## Non-goals

- Dense-output ranking.
- Singular-mass/index>0 DAE support.
- Production performance claims.
- Comparing methods at equal internal tolerances.
- Choosing one universal best solver.
