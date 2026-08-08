# Unified Native Integrator v0.9 — Math/Physics then Coding Research Report

Date: 2026-08-08

## Executive result

The math/physics research loop first separated three previously conflated questions:

1. whether an approximate nonlinear root is locally valid;
2. whether the output of interest is accurate;
3. whether a complete integrator is efficient at matched global error.

The coding loop then added only the minimum complete-integrator anchors needed to make the third question testable: fixed-step BDF1/BDF2 and Radau IIA1/3, all using the same pure-Rust dense nonlinear layer and work ledger.

The catalog now contains 167 candidates, of which four new complete-integrator anchors are executable. All four pass their bounded order/stiff/mass gates, but all remain HOLD because no matched global-error/total-cost performance surface exists yet.

## Mathematical conclusions

### Nonlinear certificate

A first Newton/output correction is not a rigorous bound. The research loop formalized a sufficient local contraction lane with

\[
Y+Z_0r+\frac12L_Br^2\le r,
\qquad
Z_0+L_Br<1,
\]

and an output-adjoint estimate

\[
|G(K_*)-G(\widehat K)|
\le
|z^TF(\widehat K)|
+\frac12\|z\|_*Lr^2.
\]

This lane requires computable derivative bounds and is therefore retained as rigorous/offline research certification, not silently substituted by a residual threshold.

### Cross-family fairness

A complete BDF/Radau solver cannot be fairly ranked inside the RODAS stage-equation screen. The future authority is a Pareto surface in

\[
(E_{\rm global},T_{\rm wall},N_f,N_J,N_{\rm fact},N_{Jv},N_{\rm PC},M_{\rm peak}).
\]

The current native gate only establishes bounded correctness anchors.

## Implemented anchors

| Candidate | Observed order | Stiff amplification | Mass L2 error | Scientific gate |
|---|---:|---:|---:|---|
| BDF1 fixed | 0.986790599249 | 6.83013455365e-05 | 2.82419965038e-05 | PASS |
| BDF2 fixed | 2.02189344652 | 0.000321286340847 | 1.96408446209e-06 | PASS |
| Radau IIA1 fixed | 0.986790599249 | 6.83013455365e-05 | 2.82419965038e-05 | PASS |
| Radau IIA3 fixed | 4.99283017895 | 7.15769262853e-06 | 5.55111512313e-17 | PASS |

Radau IIA3 uses the exact three-stage fifth-order tableau and is stiffly accurate. BDF2 uses one BDF1 startup step and only applies the two-step formula when the previous step size matches.

## Unified registry result

The unified smoke report contains:

- catalog entries: 167;
- executable candidates: 160;
- scientific checksum: `3819bbddaec6fdc27ea01159535e75018ce9e0b1d26f4276f7e94c1c8381db32`;
- joint verdicts: reference 2, hold 158, deferred 7, promote 0.

The four native anchors are `scientific_eligible=true` and `verdict=hold`, with the sole blocker:

> complete-integrator global-error versus total-cost performance assessment required

## Software findings

The common Newton layer initially performed two factorizations for an exact linear correction. The root cause was a conjunction of residual and previous-correction stopping tests. The corrected rule permits immediate exit only when the residual is both tolerance-converged and at the unscaled roundoff floor; otherwise correction convergence is still required. Linear anchor work was halved from two to one Newton factorization per step without changing the computed trajectories.

## Validity range

Supported in this checkpoint:

- binary64 regular ODEs;
- nonsingular dense mass matrices;
- fixed steps;
- explicit Jacobians or dense JVP materialization;
- small reference problems.

Not supported or claimed:

- variable step/order;
- adaptive local-error controllers for BDF/Radau;
- singular-mass DAEs;
- sparse/distributed large systems;
- production performance ranking.

## Next solver preparation

### Gate 1 — cross-family Pareto harness

For each IVP and each method, calibrate method-specific controls to a target global error, then compare total work and wall time. Do not compare equal raw tolerances.

### Gate 2 — BDF production lane

Implement variable-step coefficients, orders 1–5, Nordsieck/history management, order/step selection, rejection rollback and Jacobian/preconditioner lagging. Test variable-step zero-stability and stiff behavior before performance claims.

### Gate 3 — Radau production lane

Choose and document an embedded/defect/step-doubling estimator, implement adaptive control, simplified versus full Newton, stage transformations and preconditioning.

### Gate 4 — parallel and Krylov families

- parallel SDC: Radau/collocation nodes, diagonal/nonstationary sweeps, iteration/order/stiff gates;
- Peer/W: startup and variable-step zero-stability;
- ROK/BOROK: integrator-specific Krylov order conditions, not merely a different linear solver;
- exponential/Leja: semilinear applicability, spectral scaling and matrix-function error control.

## Final disposition

The current problem is no longer “missing BDF/Radau controls.” It is now sharply localized to two blockers:

1. a defensible global-error/total-cost comparison authority;
2. production-quality adaptive/sparse implementations for each family.

The new anchors and tier separation provide the test bed for solving those blockers without contaminating the protected RODAS5P/homotopy results.
