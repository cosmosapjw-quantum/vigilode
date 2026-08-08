# Phase 9 — Independent Diff Review

Decision: **PASS WITH SCIENTIFIC AND PERFORMANCE HOLD**.

## Strengths

- The adaptive controller is separated from family-specific estimators.
- Fixed-step public paths are preserved and regression tested.
- Variable-step BDF2 uses the correct nonuniform formula rather than reusing equal-step coefficients.
- Rejection is transactional across BDF history, output state and RODAS recycle state.
- Coarse/fine/rejected work is counted rather than hidden.
- The same external global-error schema and requested-output contract are used by all ten candidates.
- T1/T4 scientific identity is exact.

## Important limitations

1. **Output-grid clipping masks high-order adaptivity.** RODAS5P, Radau IIA3, SABR and Homotopy are limited to five requested-output steps, so tolerance-response ranking is not yet informative for those families.
2. **Step doubling is a reference estimator, not a production implementation.** It imposes one coarse plus two half integrations per macro-step and duplicates nonlinear setup work.
3. **Homotopy q=7 is an oracle-like path.** It is included to exercise the adaptive transaction and certificate, not to claim acceleration.
4. **Timing is not authoritative.** Single-pass per-row wall fields and one T1/T4 suite timing are insufficient for cross-family ranking.
5. **Failure-row partial-work preservation is not stress-tested in the unified corpus.** All 120 rows succeed; focused tests cover rejection, but a future failure corpus should verify partial counter serialization.
6. **Production family logic remains missing.** Variable-order BDF1–5, a native Radau estimator/controller, adaptive SDC, Peer/W, ROK/BOROK and exponential/Leja are still separate follow-up work.

## No blocking defect found

Within the declared dense regular-ODE reference scope, no stale-state mutation, false success, nonfinite path, fixed-step regression or thread-dependent scientific result was found.
