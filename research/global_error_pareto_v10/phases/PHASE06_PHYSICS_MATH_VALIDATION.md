# Phase 6 — Physics/mathematics validation

## Units and dimensions

For component \(i\), both \(a_i\) and \(r_i|y_i^{\rm ref}|\) have the unit of \(y_i\). Therefore

\[
\frac{y_i-y_i^{\rm ref}}{a_i+r_i|y_i^{\rm ref}|}
\]

is dimensionless.

## Error functionals

For \(m\) output times and state dimension \(n\):

\[
E_k=\sqrt{\frac1n\sum_i e_{k,i}^2},
\qquad E_{\max}=\max_kE_k,
\]

\[
E_{\rm rms}=\sqrt{\frac1{mn}\sum_{k,i}e_{k,i}^2},
\qquad E_{\rm end}=E_{m-1}.
\]

The primary pass criterion is \(E_{\max}+\epsilon_{\rm ref}\le1\). Endpoint and RMS values are diagnostics.

## Reference uncertainty

With fixed weights, the WRMS norm obeys the triangle inequality. If

\[
\|y^{\rm ref}-y^\ast\|_w\le\epsilon_{\rm ref},
\]

then

\[
\|y^{\rm cand}-y^\ast\|_w
\le
\|y^{\rm cand}-y^{\rm ref}\|_w+
\epsilon_{\rm ref}.
\]

Analytic references use \(\epsilon_{\rm ref}=0\).

## Pareto dominance

For one error coordinate \(E\) and one cost coordinate \(C\), point A dominates B iff

\[
E_A\le E_B,\quad C_A\le C_B
\]

with at least one strict inequality. Fronts are generated independently for each cost coordinate.
