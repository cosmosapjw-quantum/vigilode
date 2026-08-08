# SCIENTIFIC_CONTRACT.md

## Scientific objective

Implement a small Rust correctness oracle for the affine/reference form of the partial-coupling homotopy associated with the protected eight-stage RODAS5P lower-triangular stage system. The oracle prepares, but does not perform, the next numerical experiment branch.

## Governing definitions

\[
\mathcal R(K)=(D-C)K-g-hN(K),
\]

\[
D=I_8\otimes W,\quad W=M-h\gamma J_n,\quad C=hL\otimes J_n,
\]

\[
H_\theta(K,\lambda)=(D-\theta C)K-g-\lambda[(1-\theta)CK+hN(K)].
\]

This loop implements the affine case `N=0`, explicit block matrices, path solves, finite nilpotent inverse truncations, and an original-target output certificate.

## Conventions

- scalar arithmetic: IEEE-754 binary64;
- stage count: `s=8` for official RODAS5P screens, generic positive `s` in the oracle;
- mass/Jacobian convention: `W=M-h*gamma*J`;
- coupling convention: `C=h*L kron J`, `L=beta-gamma*I`;
- path parameter: `theta, lambda in [0,1]`, dimensionless;
- row-major stage flattening: stage index outer, physical index inner;
- norm: Euclidean for residual/correction, WRMS for output defect;
- no stochastic algorithm in this loop.

## Valid regime

- finite explicit dense `M`, `J`, `beta`, `g`;
- invertible `W` and path/target block operators;
- regular ODE or regular index-1 DAE algebraic regime only as a structural assumption;
- small-scale reference/oracle use, not production sparse scaling;
- no nonlinear remainder continuation.

## Required invariants

- `H_theta(K,1) == R(K)`;
- `H_theta(K,0) == (D-theta*C)K-g`;
- strictly lower stage coupling implies `(D^{-1}C)^s=0`;
- full finite inverse uses exactly terms `0..s-1`;
- `theta=1`, affine-in-state path is independent of `lambda`;
- target residual, not homotopy residual, controls certification;
- existing sequential RODAS5P and SABR behavior is unchanged.

## Known limits

| Limit | Expected result | Tolerance | Reference/test |
|---|---|---:|---|
| `lambda=0` | start operator residual | `64 eps * scale` | unit test |
| `lambda=1` | target residual | `64 eps * scale` | unit test |
| `theta=1`, `N=0` | path solution constant in `lambda` | `1e-12` | unit test |
| full truncation `q=s-1` | direct path inverse action | `5e-11` | noncommuting `M,J` test |
| exact target stage vector | output defect zero | `1e-13` | certificate test |
| nonnormal shift growth | `cond_1(mu=10)>cond_1(mu=1)` | strict inequality | design-check screen |

## Reference cases

- analytic toy: scalar nearest-neighbor stage chain;
- trusted numerical reference: direct `faer` LU solve of explicit block matrices;
- previous implementation: protected Rust pre-homotopy tag `rust-pre-homotopy-v0.2.0-alpha1`;
- published structure: RODAS5P stage equations in Steinebach (2023).

## Numerical requirements

- target precision: binary64-scaled residual checks;
- no convergence-order claim in this loop;
- deterministic JSON design-check output;
- no timing/performance promotion;
- no new dependency;
- offline locked build with Rust 1.94.1 and faer 0.24.4.

## Failure semantics

NaN/Inf, invalid parameter range, dimension mismatch, singular factorization, invalid truncation depth, non-finite certificate, or failed direct solve is an explicit error. No silent clipping, empty success, or fallback is introduced.

## Change control

Changing the official coefficients, sequential/SABR algorithms, tolerances, target residual semantics, or scope to adaptive continuation requires a new approved cycle.
