# External static audit intake — post-Task-1 DAG only

The external report was a static source audit plus independent high-precision coefficient reconstruction; it did not execute the Rust repository because the checked-in Cargo source replacement requires an external sibling vendor.

## Findings admitted as planning inputs

- **E4 build reproducibility:** a fresh third-party clone cannot build without the external sibling vendor and forced offline configuration.
- **A1 tolerance parity:** the exponential inner tolerance is linked to outer `rtol`, while the GMRES linear tolerance is fixed.
- **A2 GMRES early convergence:** the reported inner Arnoldi loop lacks a tolerance-based early-exit gate.
- **A3 repeated least squares:** the reported implementation recomputes a column-pivoted QR at each Arnoldi step, risking wall/work-counter divergence for small systems.

## Claim boundary

These are source-audit findings, not runtime-verified closure results in this transaction. They must not be mixed into the exact Task-1 four-file publication surface.

## New DAG nodes after Task-1 publication

1. `PM-4.R0_E4_BUILD_REPRODUCIBILITY`
2. `PM-4.F1_A1_TOLERANCE_PARITY`
3. `PM-4.F2_A2_A3_GMRES_FAIRNESS`
4. PM-4 Task 2 only after explicit disposition of these blockers

Until then, wall-clock ranking and speedup claims remain forbidden.
