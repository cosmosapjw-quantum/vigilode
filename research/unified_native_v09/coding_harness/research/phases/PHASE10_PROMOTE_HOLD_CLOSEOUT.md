# Phase 10 — Promote / Hold Closeout

## PROMOTE

- common dense nonlinear work/failure contract;
- fixed-step BDF1/BDF2 correctness anchors;
- fixed-step Radau IIA1/3 correctness anchors;
- executable catalog entries for the four anchors;
- deterministic native order/stiff/mass gate;
- native gate CLI and unified-candidate-document integration;
- explicit separation of RODAS-stage and complete-integrator scientific tiers.

## HOLD

- any production/default solver change;
- variable-order BDF1–5;
- adaptive Radau;
- singular-mass/index-1 DAE claims;
- sparse/JFNK or large-scale performance claims;
- cross-family speed ranking;
- promotion of any native anchor from the unified joint gate.

## RESEARCH DISPOSITION

The four anchors are scientifically eligible under their bounded fixed-step gates but remain `HOLD` in the joint registry because the required global-error versus total-cost assessment is not yet implemented. This is intentional: equal input tolerance and equal step count are not fair cross-family comparison rules.

## Next authorized cycle

Build the external global-error/total-cost Pareto harness, then implement production capability in this order:

1. variable-step/variable-order BDF 1–5;
2. adaptive Radau IIA with an explicit estimator policy;
3. parallel SDC;
4. Peer/W methods;
5. Rosenbrock–Krylov and BOROK;
6. exponential Rosenbrock/Leja.

The rigorous certificate lane (Kantorovich/Krawczyk/radii-polynomial plus output adjoint) remains an offline/research certificate until computable derivative bounds exist; operational comparisons retain protected reference solutions and explicit failure semantics.
