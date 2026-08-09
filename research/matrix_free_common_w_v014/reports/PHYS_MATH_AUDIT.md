# PHYS–MATH AUDIT — Matrix-Free Common-W Gate

## Conventions and dimensions

- Mass-form convention: `M ydot = f(t,y)`.
- Frozen Jacobian: `J_n = df/dy`, with the same units as `M/time`.
- Shifted operator: `W = M - h gamma J_n`; `h` has time dimension and `gamma` is dimensionless.
- Therefore `Mv` and `h gamma Jv` have identical dimensions.
- Complex stiff/oscillatory eigenpairs are represented by real 2×2 blocks; sign convention produces negative real damping and nonzero imaginary rotation.

## Exact requirements checked

| Check | Result |
|---|---|
| Explicit Jacobian build in candidate paths | 0 |
| Factorization build in candidate paths | 0 |
| True residual measured in original unpreconditioned system | yes |
| Same tolerance for all four solvers | yes |
| Same analytic diagonal preconditioner | yes |
| Failed block work retained | yes |
| Serial/Rayon solution identity | exact within stored output |
| Successful candidate residual tolerance | pass on all 1692 rows |

## Strongest failure modes

### P0 — none observed in bounded gate

No false success, NaN/Inf success, or explicit-J leakage was found in the canonical grid.

### P1 — block Krylov nonnormal/high-rank breakdown

`block-gmres` fails in 36 rows, exclusively at `N=512/2048` and severe stiffness/nonnormality/high-rank combinations. All fail by exhausting 128 search directions. This is not a proof of nonexistence; it is a hard robustness blocker for default use.

### P1 — synthetic rank is not physical stage-rank evidence

The RHS rank is constructed exactly. Actual q=0/1/2 homotopy and SABR RHS can have tolerance-dependent numerical rank, changing singular vectors, and nonlinear round-to-round drift. A dispatcher cannot be frozen from this gate alone.

### P2 — determinant/spectrum do not predict nonnormal convergence

The matrix-free operator has controlled damping and upper-block nonnormal coupling, but the finite grid cannot cover arbitrary pseudospectral amplification. Actual physics matrices may be more strongly nonnormal.

### P2 — analytic preconditioner scope

The diagonal preconditioner uses known diagonal mass/stiffness structure. It is explicit-J-free, but not universally available from a black-box residual. A future general solver needs a declared physics preconditioner or JVP-only fallback policy.

### P2 — no integrator-level order/stability test here

This is a linear common-W gate only. Fifth order, L-stability, phase error, nonlinear certificate, and fallback frequency remain untested for the integrated homotopy solver.

## Surviving mathematical claim

> For a common matrix-free shifted operator with eight moderate-sized RHS, block Krylov can materially reduce operator-vector work, while independent RHS concurrency supplies a robust fallback. The choice must depend on the actual RHS subspace and convergence diagnostics.
