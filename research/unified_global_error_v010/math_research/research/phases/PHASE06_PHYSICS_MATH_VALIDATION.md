# Phase 6 — Physics/mathematics validation

## Error definitions

For common output times \(t_j\), candidate state \(y_j\), and exact reference \(y_j^\star\):

\[
E_{\rm end}=\|y_m-y_m^\star\|_2,
\]

\[
E_{\max}=\max_j\|y_j-y_j^\star\|_2,
\]

\[
E_{\rm rms}=\left(\frac1{m+1}\sum_j\|y_j-y_j^\star\|_2^2\right)^{1/2}.
\]

For an external scale \(s_{j,i}=a_i+r\max(|y_{j,i}|,|y_{j,i}^\star|)\):

\[
E_{{\rm WRMS},j}=\left[\frac1n\sum_i\left(\frac{y_{j,i}-y_{j,i}^\star}{s_{j,i}}\right)^2\right]^{1/2}.
\]

All quantities are dimensionless in P0. Later physical problems require componentwise scales with declared units.

## Pareto definition

Point \(a\) dominates \(b\) for error \(E\) and cost \(C\) iff

\[
E_a\le E_b,\quad C_a\le C_b,
\]

with at least one strict inequality. Failed or nonfinite points are not placed on a successful front but remain visible in the full record.

## Reference uncertainty

Analytic references carry declared uncertainty zero at the model level; floating evaluation uncertainty remains binary64 roundoff and is recorded as such. Numerical references in later cycles must carry a nonzero uncertainty estimate and method provenance.
