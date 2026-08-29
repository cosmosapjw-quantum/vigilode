# Matrix-free W correction: frozen design, 2026-08-30

Base: c5fbd6d5703fc396bdf30eb3acfacb6c6bd2b921 (PR31).
This additive research-only work does not activate a production solver.

## Contract fixed before numerical outcomes

Reuse the already-projected target and same supplied trial K. Solve each lower
block row with the existing incremental-Givens candidate, W=M-h*gamma*J0.
Require a genuinely matrix-free input context and analytic JVP. Factory setup
is invoked once; the returned fixed left preconditioner and GMRES workspace
are reused across all eight rows, then dropped. No cross-step cache exists.
The factory may report its own setup work. Its internals and any dense mass
input are NOT covered by a universal matrix-free/storage claim.

Each returned row is independently checked in the UNPRECONDITIONED Euclidean
norm against max(atol,rtol*norm(rhs)). This is a linear solve tolerance, never
an independent observable budget or an outer-error certificate. Zero RHS has
no relative residual. K_new=K-z. No coefficient, default, or historical data
changes. Preserve failures and every already-returned row/report/diagnostic.
A kernel error does not expose its unfinished iterate: mark it unavailable,
never synthesize a zero iterate or a successful row.

Wrapper W/JVP/mass and preconditioner applications count attempts/completions.
Inherited preparation counters describe completed operations; on preparation
failure the unknown failed callback work is explicitly marked incomplete.
Do not label that total exhaustive. The original APIs are not silently changed.

## Implement/test sequence

1. RED public matrix-free contracts; implement child module under audit2_research.
2. Compare against existing small explicit full-target correction at identical K.
3. Adversarial setup, JVP, preconditioner, nonfinite, zero, exhausted/late failure
   and explicit-input tests. Re-evaluate original action independently on small
   systems. Keep failure raw observations, not only successful cells.
4. Extend existing readiness runner. Execute targeted + feature-off checks,
   scoped lint, fmt, old bridge tests, and baseline normal/partial examples.
5. Push a new stacked Draft PR with evidence and a bounded local fresh-review
   handoff. No scientific merge or production/timing admission.

## Fixed numerical probes

Small oracle: inherited n=4,8,16 and h=.001,.01,.05,.1, plus inherited
2-dimensional nonsymmetric mass fixture. Trial K=1e-5*sin(i*n+j+1), chosen
by coordinates (no trajectory or outcome selection). GMRES rtol=1e-11,
atol=1e-13, restart=32, max_arnoldi=256. Small-system relative difference
is recorded and compared with a pre-test 1e-7 regression threshold; direct full-target residual must satisfy a condition-independent
linear residual test <= 1e-8*max(1,norm(projected_rhs)), fixed here, not an
output budget. Retain failed cases without tolerance widening.

Storage probes: n=32,128,512, h=.01, analytic tridiagonal dissipative RHS/JVP,
identity mass and identity preconditioner, no explicit matrix or oracle.
Report retained GMRES workspace capacity, not peak memory/zero allocations.
No timing, historical campaign, N=2048, holdout or external BDF run.

## Next boundary

An independent fresh host review and whole-step transactional integration are
not accomplished by this function. The latter must preserve original-target
accept/reject, rollback/fallback and all diagnostic/candidate work. Real-client
accuracy needs a justified independent B and reference uncertainty treatment.

Additional predetermined control: n=128 diagonal rates 1+100*i^2 and
analytic inverse diagonal 1/(1+h*gamma*rate). One setup, same GMRES config,
no assembled matrix. This tests nonidentity preconditioner reuse, not speed.
Existing callback/iteration details not exposed on error remain explicitly
incomplete/unavailable; no full failure-accounting closure is claimed.
