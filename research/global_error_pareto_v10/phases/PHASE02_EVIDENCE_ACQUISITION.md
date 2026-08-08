# Phase 2 — Evidence acquisition

## Primary-source observations

- SUNDIALS CVODE uses variable-step, variable-order BDF 1–5 for stiff ODEs, predicts the nonlinear initial guess from history, and can use Newton, inexact Newton/Krylov, or fixed-point nonlinear solvers. Thus internal tolerance and work semantics are method-specific.
- PETSc exposes weighted error norms where a normalized value of one balances absolute and relative tolerances.
- SciML benchmark practice builds work-precision diagrams against analytic or high-accuracy numerical reference solutions and preserves solver failures rather than comparing only accepted runs.
- PETSc TSBDF exposes method order and history extrapolation controls, reinforcing that history/startup behavior is part of total method cost.

## Consequence

Equal internal `rtol`/`atol`, equal accepted step count, or equal nonlinear iteration count are not cross-family authorities. The common authority must be external global error on a common set of output times, followed by total work, wall time, and memory comparison.
