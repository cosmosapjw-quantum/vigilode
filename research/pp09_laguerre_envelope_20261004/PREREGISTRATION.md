# PP09 — where the stiff Laguerre bound is lost, and a certified finite-degree envelope (prospective registration)

RVJ DAG node `PP09` (depends on PP08, L-0071). Base commit: the commit
that adds this file. The six large-h-rho budget rejections of R-NEXT-04
(L-0047) stay historical results; no admission changes in this node.

## Part A: decomposition (published data only)

From `research/rnext04_laguerre_admission_20261003/cases.json`, for every
Laguerre case with `h rho >= 20` (the six rejections included): the
components truncation, coefficient, recurrence_adjoint, summation,
normalization as fractions of the total, and which term is the
bottleneck.

## Part B: certified envelope

The coefficient and tail terms use `||L_n(X)|| <= e^{L'/2}` on the
verified spectrum `[0, L']`. A certified finite-degree replacement
`E_n = max_{x in [0, L']} |L_n(x)|` is computed for `n <= 128` with
interval arithmetic (mpmath `iv`, 60 digits): `[0, L']` split into
subintervals, the three-term recurrence evaluated on each in interval
arithmetic, subdivided until each enclosure is within 1 % of its upper
end or 2^20 pieces. The bound uses the maximum upper end. Then the
coefficient and tail terms of the six cases are recomputed with `E_n` in
place of `e^{L'/2}` (post hoc, from published coefficient radii and tails
where the fields allow it; otherwise only the ratio `e^{L'/2} / max E_n`
is reported).

## Gate

G1 Part A covers every case with `h rho >= 20` in the published file.
G2 Every `E_n` is an enclosure: checked against 60-digit point
   evaluations at 10,000 points per n (no point above the bound).
G3 Reported, not gated: the factor gained and whether the recomputed
   totals would meet the published budgets (which would justify a native
   implementation next; this node does not admit anything).

## Results (append only after the recorded run)
