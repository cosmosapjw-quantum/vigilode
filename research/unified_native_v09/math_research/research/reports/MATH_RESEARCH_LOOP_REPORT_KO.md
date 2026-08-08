# Unified Native Integrators v0.9 — Mathematics/Physics Research Report

This report compacts Phases 1–10. The decisive result is a change in problem ordering:

1. do not tune homotopy acceptance further before defining nonlinear-tail evidence;
2. establish one common nonlinear-solve and cost contract;
3. add fixed-step BDF1/BDF2 and Radau IIA1/3 as exact native anchors;
4. compare complete methods by external global error–cost Pareto sets;
5. only then add variable-order BDF, SDC, ROK/BOROK, Peer/W and Leja in capability tiers.

## Core blocker resolution

The previous first-correction estimate is retained as an operational signal, not a proof. A rigorous result requires a root radius based on Newton–Kantorovich/Krawczyk/radii-polynomial assumptions and a nonlinear remainder term in the output bound. If those assumptions cannot be computed cheaply, the method must fall back rather than falsely certify.

## Why BDF and Radau first

BDF exposes history, startup and multistep controller requirements. Radau exposes fully coupled nonlinear stage work and supplies a high-order L-stable oracle. Together they test whether the unified registry and ledger truly compare complete integrators, rather than only alternative RODAS internal solvers.

## Deferred solver preparation

- Parallel SDC will reuse Radau nodes, collocation residuals and stage storage.
- ROK/BOROK will reuse the operator/JVP/Krylov ledger but enter only large-system capability tests.
- Peer/W needs a separate two-step state and restart contract.
- Exponential/Leja needs a semilinear split and matrix-function action ledger.

No solver is promoted merely by being present in the registry.
