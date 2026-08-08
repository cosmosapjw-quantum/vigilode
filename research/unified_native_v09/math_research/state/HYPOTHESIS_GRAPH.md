# Phase 4 — Hypothesis Space

## H-001 — Two-level nonlinear certificate

CORE_CLAIM: Use a cheap operational lane for online rejection and a rigorous/offline lane when derivative bounds are available.

- Operational: first and second corrections, residual decrease, nonnormal amplification proxy, and conservative fallback.
- Rigorous: Newton–Kantorovich/Krawczyk/radii-polynomial radius plus an adjoint output remainder.
- Distinctive prediction: operational false accepts disappear after calibration only on bounded test domains; rigorous lane succeeds only when derivative-bound assumptions close.
- Fatal vulnerability: derivative bounds are so expensive that the rigorous lane is never useful, or operational diagnostics still false-accept holdout cases.

## H-002 — Stage-recursive output certificate

CORE_CLAIM: The lower-triangular RODAS stage Jacobian permits a cheaper forward error-radius recursion and backward adjoint recursion than a generic full 8n inverse.

- Minimal assumptions: invertible common W, computable stagewise Jacobian-drift bounds.
- Distinctive prediction: same certified output radius as a generic norm bound with lower factorization/memory cost.
- Fatal vulnerability: nonnormal cross-stage amplification makes the recursion as pessimistic as the full bound.

## H-003 — Native BDF1/BDF2 anchor

CORE_CLAIM: Fixed-step BDF1/BDF2 with the common dense Newton layer is the smallest decisive multistep control and reveals history/startup/accounting requirements before variable-order work.

- Expected: orders 1 and 2, zero-stability, stiff decay, mass-matrix parity.
- Fatal vulnerability: no shared nonlinear contract can express both BDF and Radau without solver-specific hidden work.

## H-004 — Native Radau IIA anchors

CORE_CLAIM: Backward Euler (one-stage Radau IIA) and three-stage fifth-order Radau IIA provide robust fully implicit controls on the same backend.

- Expected: order 1 and 5, stiff accuracy, L-stable scalar limit, robust nonlinear solve.
- Fatal vulnerability: dense block Newton cost dominates so completely that the anchor cannot be exercised beyond toy dimensions; still useful as correctness control.

## H-005 — External-error Pareto contract

CORE_CLAIM: Same-tolerance runs are diagnostic, but candidate promotion must use external global-error-versus-total-cost Pareto dominance under matched problem/output/event requirements.

- Distinctive prediction: rankings may change when input tolerances are calibrated to equal observed global error.
- Fatal vulnerability: reference-error computation dominates or exact/reference solutions are unavailable; then use nested high-accuracy controls with uncertainty labels.

## H-006 — Variable-order BDF1–5

HOLD until H-003 closes. Requires variable coefficients, history transformation, local error estimator, order/step controller and nonlinear forcing.

## H-007 — Parallel SDC on Radau nodes

HOLD until H-004 closes. Reuses collocation residual and node storage, adds diagonal/nonstationary sweeps and node-parallel scheduling.

## H-008 — Peer/W

HOLD. Independent stage solves are structurally attractive but startup, variable-step zero-stability and embedded estimation are separate blockers.

## H-009 — ROK/BOROK

HOLD. Requires matrix-free large-system traces, basis/error accounting, and for BOROK likely transpose-JVP support.

## H-010 — Exponential/Leja

HOLD as semilinear capability lane. Must not be ranked on arbitrary mass-matrix/DAE cases it does not support.
