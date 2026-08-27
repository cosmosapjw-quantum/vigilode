# K0 Stage Telemetry Invariants

## Scientific and numerical boundary

K0 observes the current RODAS5P stage solves. It does not change the equations, method coefficients, solver tolerances, convergence authority, controller, accepted output, or recycle transaction.

### Load-bearing equations

For stage state
\[
Y_i = y_n + \sum_{j<i}\alpha_{ij} k_j,
\]
the recorded nonlinear remainder is
\[
N_i = f(t_i,Y_i)-f(t_n,y_n)-J_n(Y_i-y_n)-c_i h f_{t,n}.
\]

The receipt scalar `scaled_nonlinear_remainder` is the Euclidean-norm ratio
\[
\rho_i
:=
\frac{\lVert N_i\rVert_2}
{\max\!\left(
\lVert f_i\rVert_2,
\lVert f_0\rVert_2,
\lVert J_n(Y_i-y_n)\rVert_2,
\lVert c_i h f_{t,n}\rVert_2,
\mathrm{f64::MIN\_POSITIVE}
\right)},
\]
where `f_i = f(t_i,Y_i)` and `f_0 = f(t_n,y_n)`. The four input vectors are scaled together when testing scale invariance. The floor is exactly Rust `f64::MIN_POSITIVE`, not an absolute scientific tolerance. `rho_i` is dimensionless and nonnegative, is exactly zero for an affine-linear witness up to floating-point roundoff, and is **not** bounded above by one (the triangle inequality permits values through four under this denominator).

The work identity is
\[
N_{\rm apply}^{\rm total}=N_{\rm linear}+N_{\rm diagnostic}.
\]

The unpreconditioned true residual is the sole convergence authority. Projected residuals are advisory.

## Invariant list

- `INV-K0-001`: production and unobserved paths are unchanged.
- `INV-K0-002`: true residual remains authoritative.
- `INV-K0-003`: checked operator accounting closes exactly.
- `INV-K0-004`: telemetry overhead is named, not hidden.
- `INV-K0-005`: every attempted stage/cell has an explicit terminal state.
- `INV-K0-006`: nonlinear remainder equation and sign are exact.
- `INV-K0-007`: RHS novelty angle is non-tautological and scale invariant.
- `INV-K0-008`: residual-sign coverage is separate from norm telemetry.
- `INV-K0-009`: recycle rollback is unchanged.
- `INV-K0-010`: replay is exactly two arms by six frozen families.
- `INV-K0-011`: claim class remains exploratory/non-authoritative.
- `INV-K0-012`: GitHub/Jira/Confluence authority roles and pointers agree.
- `INV-K0-013`: `scaled_nonlinear_remainder` uses the exact `rho_i` formula above; no implementation-selected norm, WRMS scale, or denominator substitution is permitted.

A prose warning is not a guard. Each P0/P1 above must have a regression, invariant check, or fail-closed STOP.
