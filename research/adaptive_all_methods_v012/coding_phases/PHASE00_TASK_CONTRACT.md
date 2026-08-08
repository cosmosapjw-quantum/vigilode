# Phase 0 — Code Task Contract

## User-visible outcome

Every currently executable complete-integrator family gains an adaptive requested-output API:
Sequential RODAS5P (all existing inner solvers), SABR5P, Homotopy RODAS, BDF1/2, and Radau IIA1/3.

## Scientific purpose

Separate estimator evidence from controller policy, preserve every fixed-step baseline, and make
future cross-family comparison at matched external global error possible without hiding rejected or
step-doubling work.

## Preserved behavior

- `unified-fair-measurement-v0.11.0-alpha1` fixed-step outputs and counters.
- Existing Sequential/SABR default adaptive numerical path unless a bug is proven.
- Protected sequential fallback and typed failure semantics.
- Requested-output-only storage and clipping contract.

## Scope

In scope: shared bounded controller, variable-step BDF2, step-doubling BDF/Radau, adaptive Homotopy,
unified adaptive screen.  Out of scope: BDF3–5, native RADAU5 estimator, new dependencies, dense
output, sparse/MPI/GPU/DAE claims, and deferred integrator implementations.

## Completion bar

All current families execute adaptive analytic/stiff/mass screens transactionally; expected order is
observed before roundoff; tighter tolerances do not systematically worsen external global error;
all work is counted; fixed-step regressions pass; exact-HEAD format/Clippy/tests/build and independent
diff review pass.
