# Next Cycle — Cross-Family Pareto Authority and Production Candidates

1. Freeze a representative IVP corpus and high-accuracy reference solutions.
2. Add a method-independent global-error evaluator and total work/memory ledger.
3. Calibrate each method's internal tolerance/step controls on calibration cases only.
4. Evaluate holdout Pareto fronts; preserve failures and rejected steps.
5. Implement variable-order BDF 1–5 first, then adaptive Radau.
6. Add parallel SDC and Peer/W only after the complete-integrator authority is stable.
7. Add ROK/BOROK and exponential/Leja as distinct integrator families, not as renamed linear solvers.
8. Re-run RODAS5P/SABR/homotopy, BDF and Radau together under the same external error targets.

Immediate first deliverable: `GlobalErrorTarget`, `IntegratorWorkReport`, reference-solution provenance, and a fixed-step anchor Pareto replay that does not alter any solver.
