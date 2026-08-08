# Phase 9 — Survivor formalization

## Core data model

- `GlobalErrorTarget`: metric and threshold, independent of solver-internal tolerance.
- `ReferenceSolutionProvenance`: problem, source kind, output-grid hash, state hash, uncertainty.
- `GlobalErrorMetrics`: endpoint/max/RMS L2 and WRMS.
- `IntegratorWorkReport`: full `WorkCounters`, wall samples/median/IQR, stored-state bytes.
- `IntegratorRunRecord`: candidate, problem, fixed-step control, status, error, work, checksums.
- `ParetoFront`: problem, error metric, cost metric, nondominated record ids.
- `TargetAttainment`: cheapest successful record below an external error target.

## Initial corpus

1. scalar linear, mild stiffness;
2. Prothero–Robinson, strong stiffness with exact solution;
3. nonlinear noncommuting mass matrix;
4. manufactured vector with nonnormality.

## Initial candidates

- sequential RODAS5P/direct;
- BDF1 fixed;
- BDF2 fixed;
- Radau IIA1 fixed;
- Radau IIA3 fixed.

This is an authority validation set, not a production ranking set.
