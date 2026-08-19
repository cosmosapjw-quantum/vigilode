# NEXT NODE — v3.2 Tolerance-Weighted Quadratic-Remainder Drift Discovery

## Parent

Use the final committed v3.1 negative-discovery HEAD only.

## Why

v3.1 shows `D2,D3,D4` are almost collinear, which is exactly what the leading Hessian expansion predicts. Raw direction therefore retains little generic information and is additionally component-scale dependent.

The next low-complexity object should cancel the shared leading quadratic term instead of measuring its direction.

## Predeclare before output

For stage fractions `c2=1/4`, `c3=1/2`, `c4=9/10`, define

\[
Z_{23}=D_3/c_3^2-D_2/c_2^2,\qquad
Z_{34}=D_4/c_4^2-D_3/c_3^2.
\]

Use a **common pairwise tolerance scale** for each component,

\[
s_k^{ij}=\mathrm{atol}+\mathrm{rtol}\max(|y_{n,k}|,|U_{i,k}|,|U_{j,k}|),
\]

and predeclare at most two dimensionless witnesses, e.g.

\[
\zeta_{ij}=|h|\left[\frac1m\sum_k\left(\frac{Z_{ij,k}}{s_k^{ij}}\right)^2\right]^{1/2},
\]

plus one bounded relative drift based on the same scaled vectors. Do not fit a threshold on discovery data.

## Asymptotic motivation

If `F in C^3` and `D_i=(1/2)c_i^2h^2 F''[F,F]+O(h^3)`, then

\[
Z_{ij}=O(h^3),\qquad |h|Z_{ij}/s=O(h^4),
\]

so the shared quadratic contribution is removed and the new quantity targets the higher-order stage variation that the rejected direction cosines suppress.

## Data roles

- N=96/256: replay-only discovery.
- N=192: remains sealed until one drift witness survives dimension + LOFO robustness.
- N=384/N=2048: sealed.

## Forbidden

- active switching;
- runtime E-K activation;
- N=192/384/2048 before a discovery survivor;
- threshold fitting on N=96/256;
- family-specific rules;
- high-capacity classifier.
