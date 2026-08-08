# Phase 4 — Hypothesis Space

## H1 — Shared controller, native estimators

- Core claim: share acceptance/factor/rejection semantics but keep family-specific estimators and estimator orders.
- Strength: preserves method identity and avoids unnecessary work.
- Fatal vulnerability: an unreliable native estimator can pass the shared controller while violating global order.
- Discriminator: external global-error holdout and rejection replay.

## H2 — Universal step-doubling reference

- Core claim: one coarse step versus two half steps supplies a common estimator for every current family.
- Strength: method-independent and easy to audit.
- Vulnerability: roughly three step executions; BDF history and startup require special treatment.
- Discriminator: estimator reliability versus work overhead.

## H3 — Native RODAS endpoint adaptivity for Sequential/SABR/Homotopy

- Core claim: use the RODAS5P 5(4) embedded error; add algebraic/certificate contribution only when a speculative stage path is used.
- Vulnerability: homotopy certificate may underestimate nonlinear stage error.
- Discriminator: protected sequential oracle and global-order gate.

## H4 — Variable-step BDF2 plus step-doubling

- Core claim: use exact nonuniform BDF2 coefficients, clone history for coarse/fine trials, and accept the two-half-step state.
- Vulnerability: startup and macro/fine history alter estimator asymptotics; high cost.
- Discriminator: polynomial exactness, variable-ratio convergence, rejection rollback.

## H5 — Radau step-doubling baseline before native RADAU5 estimator

- Core claim: implement a robust common reference first; later compare the Hairer estimator and predictive controller.
- Vulnerability: expensive and may obscure Radau's production advantage.
- Discriminator: same global error at matched total work.

## H6 — Cost-aware controller

- Core claim: after correctness, choose \(h\) to minimize measured work per simulated time subject to error acceptance.
- Vulnerability: exploration noise and nonstationary solver cost.
- Discriminator: holdout total-cost Pareto improvement over classical I/PI control.

## H7 — Joint \(h\) and inner-work adaptivity

- SDC: adapt \(h\) and sweeps.
- ROK/BOROK: adapt \(h\) and Krylov dimension.
- Homotopy: adapt \(h\), path rounds, \(q\), and fallback threshold.
- Exponential/Leja: adapt \(h\) and matrix-function work.
- Status: future-family contract, not current implementation.
