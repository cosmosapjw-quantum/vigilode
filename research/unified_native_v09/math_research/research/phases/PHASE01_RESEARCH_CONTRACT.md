# Phase 1 — Research Contract

DATE: 2026-08-08
PROJECT: Unified Stiff Integrator Expansion v0.9

## PRIMARY_RQ

How should the existing Rust-only RODAS5P/SABR/homotopy candidate harness be extended so that (i) the nonlinear-output certification blocker is addressed without false safety claims, and (ii) BDF and Radau anchors, followed by SDC, Peer/W, Rosenbrock–Krylov/BOROK, and exponential/Leja candidates, can be compared under mathematically legitimate and computationally comparable contracts?

## SUB_RQS

1. What is the weakest computable assumption set that turns a nonlinear stage residual into a reliable state- or output-error bound?
2. Which fixed-step BDF and Radau methods form the smallest decisive native anchors before variable-order/adaptive implementations?
3. What common work, accuracy, failure, DAE and concurrency ledgers are required without erasing family-specific costs?
4. Which comparisons are universal, and which must be capability-tier specific?
5. What implementation order minimizes duplicated infrastructure and makes each subsequent solver independently testable?

## IN_SCOPE

- Regular ODEs and nonsingular mass-matrix systems \(M\dot y=f(t,y)\).
- A future residual-form index-1 DAE lane, but only after consistent initialization and constraint gates exist.
- Dense binary64 anchors in the current pure-Rust/faer backend.
- Fixed-step BDF1/BDF2 and Radau IIA 1/3-stage mathematical anchors.
- A two-level nonlinear certificate architecture: operational estimator plus rigorous/offline verifier.
- External global-error-versus-cost Pareto curves as the authoritative cross-family comparison.

## OUT_OF_SCOPE

- Claiming a cheap universal nonlinear certificate without derivative/Lipschitz information.
- Variable-order BDF1–5, production adaptivity, events, sparse/MPI/GPU, or singular-mass DAE support in this cycle.
- Implementing all seven deferred solver families at once.
- Declaring a universal winner from equal input tolerances or accepted-step counts.

## CONVENTIONS

- State dimension: \(n\); step size \(h\) carries time units.
- \(M\dot y=f(t,y)\): \(M\) is dimensionless unless the application supplies another scaling; \(h f\) has the units of \(M y\).
- WRMS norms use the same `atol + rtol*scale` contract for every family.
- A solver success means both internal convergence and an external residual/correctness certificate.
- Exact/reference computations are not charged as production work but are separately recorded.

## EVIDENCE TYPES

- Primary numerical-analysis papers, official solver documentation, exact symbolic derivations, reproducible Rust tests and frozen benchmark artifacts.
- Reviews and secondary documentation only for context, not for novel or quantitative claims.

## COMPLETION BAR

- A claim-audited mathematical design for nonlinear certification and cross-family comparison.
- Exact BDF1/BDF2/Radau IIA identities and kill tests.
- An externally reviewed coding design that adds native anchors without weakening the protected RODAS5P baseline.
- Explicit blockers and a bounded next implementation branch.
