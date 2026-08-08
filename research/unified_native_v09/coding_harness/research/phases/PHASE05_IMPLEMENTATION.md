# Phase 5 — Isolated Implementation

## Scope executed

The implementation remained in the isolated `unified-native-integrators-v0.9` worktree and added only the bounded native-anchor layer selected by Phase 3/4:

- a common dense full-Newton reference solver with typed failures and full work accounting;
- explicit/JVP-materialized dense Jacobians;
- fixed-step BDF1 and BDF2 with transactional history and BDF1 startup;
- fixed-step Radau IIA one-stage and three-stage methods in mass-matrix form;
- executable catalog entries and a dedicated complete-integrator scientific gate;
- CLI serialization for the native gate and integration into the unified candidate document.

No variable-step/order controller, embedded Radau estimator, singular-mass DAE support, SDC, Peer/W, ROK, BOROK or Leja implementation was added.

## TDD evidence

Red-green tests were executed for:

- missing common Newton API;
- BDF scalar amplification, BDF2 startup/order and failure rollback;
- Radau tableau identities, Radau1=BDF1, fifth order and mass-matrix accuracy;
- native candidate registry and deterministic gates;
- missing `native-integrator-gates` CLI command;
- missing native-gate data in the unified candidate document;
- redundant second Newton factorization after an exact linear correction.

The last issue was fixed without relaxing nonlinear accuracy: a one-step solve may terminate after a roundoff-level residual even if the preceding correction is large; otherwise both residual and correction convergence remain required.

## Protected behavior

The RODAS5P tableau, sequential stage equations, SABR5P, homotopy configurations, existing certificate hierarchy and Rust-only linear solver behavior were not altered. The old RODAS-stage screens explicitly exclude complete-integrator candidates and report them as evaluated by the native gate.
