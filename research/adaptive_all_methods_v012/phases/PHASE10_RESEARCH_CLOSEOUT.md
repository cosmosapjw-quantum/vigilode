# Phase 10 — Research Closeout and Coding Handoff

Date: 2026-08-08
Baseline: `unified-fair-measurement-v0.11.0-alpha1`

## Final research decision

### PROMOTE TO CODING STUDY

1. A shared bounded adaptive-controller state machine with family-specific estimator order.
2. Native RODAS5P 5(4) error for Sequential/SABR/Homotopy endpoint families.
3. Universal step-doubling as the first auditable reference estimator for BDF1/2 and Radau IIA1/3.
4. Exact nonuniform BDF2 coefficients and history-aware extrapolation.
5. Transactional rejection: state, stage history, BDF history, recycle space, and output observer commit only after acceptance.
6. Common requested-output grid and external global-error/total-cost authority.

### HOLD

- Step doubling as a production-performance winner.
- PI control as a default before holdout comparison against the I-controller.
- Native RADAU5 embedded estimator and predictive controller.
- Variable-order BDF3–5.
- Cost-aware controllers and joint inner-work adaptivity.
- Deferred SDC, Peer/W, ROK, BOROK, and exponential/Leja implementations.

## Required coding validity range

- binary64 regular ODEs;
- nonsingular dense mass matrices;
- explicit Jacobian or existing dense JVP materialization;
- requested-output step clipping rather than unverified dense interpolation;
- no general index-1 DAE, sparse, MPI, or GPU claim.

## Coding kill gates

1. Fixed-step public API or trajectory regression.
2. Variable-step BDF2 polynomial exactness failure.
3. Rejected-trial history/recycle/output mutation.
4. Failure to recover nominal order before roundoff.
5. Tighter tolerance producing systematically larger external global error.
6. NaN/Inf, false success, or unbounded rejection loop.
7. Hidden deletion of coarse/fine trial work from the ledger.
