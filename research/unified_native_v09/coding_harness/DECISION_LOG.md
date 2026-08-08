# DECISION_LOG.md

- D-001: use one dense full-Newton anchor rather than method-specific hidden loops.
- D-002: fixed-step BDF1/2 and Radau1/3 only in this cycle.
- D-003: complete-integrator native gates are separate from RODAS-stage one-step certification.
- D-004: complete integrators may be scientifically eligible but remain HOLD until global-error/total-cost performance is measured.
- D-005: accept an exact linear Newton correction after one factorization only when the converged unscaled residual is at roundoff level.
- D-006: future cross-family comparisons match external global error, not raw solver tolerance or timestep count.
