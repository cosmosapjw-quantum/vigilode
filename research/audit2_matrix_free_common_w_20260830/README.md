# Audit-2 matrix-free common-W substrate

## Purpose

This bounded research node replaces the small-system explicit-W/LU requirement
of the Audit-2 common-W correction with an explicitly opt-in matrix-free solve
session. It does not add a new Krylov algorithm. It binds the existing
`ShiftedOperator`, identity preconditioner, and `solve_gmres_with_workspace`
implementation into a reusable research session, then uses that same session
for the eight causally ordered block-forward stage solves.

The code is compiled only by the non-default `audit2-research` feature and is
not called by the production/default integrator route.

## What is actually reused

One session owns:

- one unchanged matrix-free shifted operator identity;
- one identity-preconditioner setup;
- one `GmresWorkspace` allocation whose capacity is retained;
- cumulative, failure-preserving work counters;
- any number of sequential RHS batches for the same `(operator token, h*gamma)`.

The session deliberately does **not** claim Krylov-basis recycling, subspace
reuse, a reusable factorization, or a production preconditioner. Every RHS is a
fresh GMRES solve. The measured zero workspace-capacity growth after the first
solve is allocation reuse, not spectral reuse.

## Implemented entries

- `Audit2MatrixFreeCommonWSession`: reusable matrix-free W solve session.
- `run_audit2_matrix_free_common_w_correction`: projected eight-stage
  block-forward correction using one session and the existing stage JVPs.
- typed batch/correction outcomes that retain partial solutions, reports,
  session state, and spent work.
- explicit-W and malformed/nonfinite input fail-closed contracts.
- feature-enabled CI/readiness coverage.

## Numerical observations

The declared `n=48` session case solved two batches of eight RHS vectors with
one setup, one workspace initialization, 16 completed solves, and no direct
factorization/solve. Against a separate explicit small-system oracle, the
maximum normalized backward error was `9.362005883856722e-17` and the maximum
relative solution difference was `1.0166466437472006e-15`.

The declared actual block-forward case (`n=16`, `h=0.01`) completed eight
causally ordered solves with one setup. Its correction differed from the
pre-existing explicit-W Audit-2 reference by `4.760950091765468e-16` relative,
while its independently reapplied projected linear residual was
`3.3065204906367934e-20`. These are compatibility observations, not output or
nonlinear accuracy admission.

## Scope boundary

The current result does not establish production scalability. The test sizes
are deliberately bounded and still use a separate explicit oracle for
validation. The candidate preconditioner is identity, no timing was measured,
and the full transactional step/rollback/controller path was not changed.

The exact-head `c5fbd6...` replay is recorded in
`evidence/HOST_EXACT_C5_VERIFICATION.json`. The next substantive node is
`REUSABLE_PRECONDITIONER_AND_TRANSACTIONAL_STEP`; it is not started here. That
node must retain the original-target bridge, failure accounting, and an
independent observable budget before any accuracy or production claim.
