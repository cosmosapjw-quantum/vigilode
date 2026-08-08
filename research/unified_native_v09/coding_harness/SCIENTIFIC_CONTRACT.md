# SCIENTIFIC_CONTRACT.md

## Scientific objective

Add fixed-step native BDF1/BDF2 and Radau IIA one-/three-stage anchors to the existing Rust-only stiff-integrator registry while preserving the protected RODAS5P/SABR/homotopy baseline and exposing every nonlinear residual, Jacobian, factorization and failure through a common work ledger.

## Governing definitions

For a regular mass-matrix problem

\[
M\dot y=f(t,y),
\]

BDF1 and BDF2 solve

\[
M(y_{n+1}-y_n)-h f(t_{n+1},y_{n+1})=0,
\]

\[
M(3y_{n+1}-4y_n+y_{n-1})-2h f(t_{n+1},y_{n+1})=0.
\]

Radau IIA stages use increments \(K_i=h\dot y_i\):

\[
M K_i=h f\!\left(t_n+c_i h,y_n+\sum_j a_{ij}K_j\right).
\]

## Conventions

- unit system: solver algebra is unit-agnostic; each residual term must have the units of \(My\)
- normalization: WRMS scales `atol + rtol*max(|reference|,|candidate|)`
- time: \(h>0\); fixed-step anchors shorten only the last step to hit the endpoint
- nonlinear success: internal convergence plus external finite residual check
- failure: no history or candidate state is committed on failure

## Valid regime

- regular ODEs and nonsingular dense mass matrices
- binary64 dense anchors
- fixed steps only
- explicit Jacobian or JVP from which a dense Jacobian can be assembled
- no singular-mass DAE claim

## Required invariants

- BDF1 order 1; BDF2 order 2
- BDF2 startup is BDF1 and is charged
- BDF2 zero-stability roots \(1,1/3\)
- Radau IIA1 equals backward Euler/BDF1 at the one-step algebraic level
- Radau IIA3 order 5, stiff accuracy, exact tableau identity \(A\mathbf1=c\)
- scalar stiff limits agree with exact stability functions
- existing RODAS5P tests remain unchanged

## Known limits

| Limit | Expected result | Tolerance | Reference/test |
|---|---|---:|---|
| scalar linear, BDF1 | \(R(z)=1/(1-z)\) | 5e-13 | exact one-step test |
| scalar linear, BDF2 | characteristic roots of \((3-2z)\xi^2-4\xi+1\) | 5e-12 | two-step test |
| Radau IIA1 | same result as BDF1 | 5e-12 | cross-method test |
| Radau IIA3 | observed order >=4.5 before roundoff | fixed-step sweep | convergence test |
| Radau stiff limit | \(|R(-10^k)|\to0\) | monotone/control | scalar screen |
| noncommuting mass affine | direct residual and exact solution agreement | 1e-9 | manufactured test |

## Numerical requirements

- Newton maximum iterations: 12 in anchors
- external residual tolerance: `max(atol, rtol*||rhs/reference||)` in WRMS-compatible scaling
- all work counters are monotone and serializable
- deterministic single-thread scientific results
- performance is reported but not a promotion condition in this anchor cycle

## Failure semantics

The following are not success: NaN/Inf, singular factorization, nonlinear maximum-iteration exit, residual growth without recovery, BDF2 with missing/corrupt history silently treated as BDF2, unsupported singular mass matrices, or deferred candidates ranked as executed.

## Change control

The official RODAS5P tableau, existing homotopy policies, current tolerances and public serialization fields are protected. New counters and candidate variants may be added compatibly.
