# Phase 7 — Scientific and numerical validation

Date: 2026-08-07

## Definitions and sign audit — PASS

The implementation uses

\[
W=M-h\gamma J,\qquad D=I_s\otimes W,
\]

\[
L=\beta-\gamma I_s,\qquad C=hL\otimes J,
\]

and the affine partial-coupling path operator

\[
A_{\theta,\lambda}=D-\eta C,
\qquad
\eta=\theta+\lambda(1-\theta).
\]

At `lambda=1`, `eta=1` for every `theta`, hence

\[
A_{\theta,1}=D-C=I_s\otimes M-h\beta\otimes J,
\]

which is the protected affine RODAS block operator. At `theta=1`, `eta=1` for every `lambda`, so the affine FLRH path is constant. Both identities are tested numerically to binary64 accuracy.

`theta` and `lambda` are dimensionless. `D` and `C` have the same dimensions as `M`; the stage vector and affine right-hand side use the existing RODAS code-variable convention. No physical-unit conversion is introduced.

## Lower-triangular structure — PASS in declared regime

The constructor accepts only `beta = gamma I + L` with numerically zero diagonal/upper part of `L`. The common block `W` is checked by solving against all coordinate vectors. For

\[
Q=\eta D^{-1}C,
\]

strict lower stage structure implies `Q^s=0` in exact arithmetic. Therefore

\[
A_{\theta,\lambda}^{-1}
=
\sum_{m=0}^{s-1}Q^mD^{-1}.
\]

For the official eight-stage coefficients and a deliberately noncommuting two-dimensional `M,J`, the `q=7` finite action agrees with a direct `faer` LU solve with relative error

\[
2.493\times10^{-16}.
\]

The physical-matrix commutator is nonzero:

\[
\|MJ-JM\|_2=1.67556,
\]

so the check does not rely on simultaneous diagonalization.

## Low-depth semantics — PASS as a structural screen, not a performance claim

For the official affine screen at `theta=0.4`, `lambda=0.7`, the relative inverse-action errors are

| q | relative error |
|---:|---:|
| 0 | `1.33937e-1` |
| 1 | `1.28245e-3` |
| 2 | `8.72020e-6` |
| 7 | `2.49277e-16` |

A separate nearest-neighbor chain verifies that a depth-`q` action propagates information across only the corresponding number of causal stage edges. These values motivate the next experiment but do not establish that `q<=2` is adequate for nonlinear or stiff problems.

## Official stage-coupling powers — PASS

Frobenius norms computed from the official snapshot are

\[
\|L\|_F=6.97614,
\quad
\|L^2\|_F=0.912340,
\quad
\|L^3\|_F=0.150115,
\]

\[
\|L^6\|_F=1.69876\times10^{-4},
\quad
\|L^8\|_F=6.15\times10^{-20}.
\]

The tiny nonzero final value is consistent with floating reconstruction of the printed decimal tableau; the implementation does not replace source coefficients by hand-rounded structural zeros.

## Original-output certificate — PASS for affine reference system

For target residual

\[
r=A_*K-g,
\qquad A_*=D-C,
\]

the oracle solves

\[
A_*\delta K=r
\]

and reports

\[
E_{\rm out}
=
\left\|
\sum_i b_i\delta K_i
\right\|_{\rm WRMS}.
\]

An exact endpoint perturbation test independently constructs `delta K`; the reported correction norm and WRMS output error agree within `2e-11`. Invalid/nonpositive scale and non-finite stage vectors are rejected.

This is an affine reference certificate. It is not yet the nonlinear first-order certificate involving the full `D N(K)` Jacobian.

## Nonnormality counterexample — PASS

For the unit lower-triangular shift family `A=I-mu*S`, every determinant is exactly one while the one-norm condition estimate grows from

\[
\kappa_1(A)=2.988\quad (\mu=0.5)
\]

to

\[
\kappa_1(A)=1.22222\times10^8\quad (\mu=10).
\]

Thus determinant/eigenvalue/nilpotency checks alone cannot certify a fast path. The next branch must monitor an original-output defect and adversarial nonnormal amplification.

## Independent implementation check — PASS

A separate NumPy reconstruction, using the coefficient snapshot rather than Rust internal matrices, repeated 42 scalar/vector comparisons. All passed; maximum Rust–NumPy absolute discrepancy was

\[
1.15\times10^{-14}.
\]

Evidence: `results/INDEPENDENT_NUMPY_CHECK.json`.

## Validity limits

| Item | Status |
|---|---|
| affine explicit dense reference system | PASS |
| exact endpoint and finite inverse identities | PASS |
| deterministic structural screening | PASS |
| nonlinear remainder path | NOT TESTED |
| adaptive lambda/theta continuation | NOT IMPLEMENTED |
| Anderson or polynomial/rational acceleration | NOT IMPLEMENTED |
| fifth-order global convergence of a homotopy fast path | NOT TESTED |
| stiff decay / L-stability preservation | NOT TESTED |
| index-1 DAE algebraic constraint preservation | NOT TESTED |
| runtime speedup or break-even surface | NOT TESTED |
| sparse, matrix-free, threaded/GPU execution | NOT TESTED |

No claim beyond the first three rows is authorized.
