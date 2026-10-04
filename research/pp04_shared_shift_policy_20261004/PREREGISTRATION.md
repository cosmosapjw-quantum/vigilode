# PP04 — prospective cluster/degree selection and all-in comparison for the shared shift jet (prospective registration)

RVJ DAG node `PP04` (depends on PP02, L-0063). Base commit: the commit that
adds this file. No timing is run; the comparison unit is counted
floating-point work of executed code, and no speed claim follows from it.

## Question

Given one current dissipative real `J`, `h >= 0`, `m` positive real shifts
and `r` supplied RHS columns, which of the following produces certified
solutions of `(I - gamma_i h J) x = b_j` with the least counted all-in work,
and does a prospective selector (deciding before execution from sizes and
shifts only) pick the method that turns out cheapest?

- M1 `individual-lu`: one LU per target shift (m factorizations).
- M2 `common-shift-lu`: one LU per distinct shift value (bitwise).
- M3 `hessenberg-reuse`: `J = Q H Q^T` once (Householder), then per distinct
  shift an LU of the Hessenberg `I - gamma h H` (partial pivoting) and
  `x = Q (I - gamma h H)^{-1} Q^T b`.
- M4 `shared-jet`: shifts split into clusters with normalized radius
  `rho = max |gamma - gamma0| / gamma0 <= 1/2`, `gamma0` the cluster
  midpoint; degree `d` the least with `rho^(d+1) / (1 - rho) ||b_j||_2 <=
  tol / 2` for every column (exact-arithmetic tail with `||R0||_2 <= 1`),
  capped at 128 (beyond the cap the cluster is split). A target whose
  certificate rejects is redone with M3 and both attempts are charged.

Every candidate of every method is accepted only by the existing
`certify_shift_candidate` directed current-target residual (absolute
Euclidean tolerance per column); certification work is charged to every
method alike.

## Counted work (unit: one binary64 flop; directed work counted separately)

LU of order n: 2n^3/3; LU solve: 2n^2 per column; Householder Hessenberg:
10n^3/3 (Q accumulated: +4n^3/3); Hessenberg LU: n^2; Hessenberg solve:
3n^2 per column (with the pivot row swaps); `Q^T b`, `Q y`: 2n^2 each per
column; matrix build `I - gamma h J`: 2n^2; jet recurrence: 2n^2 + n per
column and level; Horner: 2n per column, target and level; directed residual
products: counted as `m r n^2` directed operations (D units), reported
separately and identical across methods except failed attempts. These counts
are incremented by the executing code paths, not by a formula applied
afterwards (the LU of faer is charged by its standard count).

## Selector

`SharedActionPolicy::plan(n, r, shifts, column norms, tol)` predicts each
method's flops from the same table, before any factorization, and picks the
minimum; typed abstentions for the jet: `CommonShift` (all shifts equal),
`WideCluster` (more clusters than half the distinct shifts), `HighRank`
(`r (d+1) >= m` within every cluster, a coarse screen reported but not used
alone), `DegreeCap`. Predicted and executed costs are both recorded.

## Fixtures (fixed now)

Seeded random dissipative `J = S - (|S| row sums + 0.1) I`-type matrices
(S random with skew and symmetric parts), n in {8, 32, 96}; h = 0.1;
r in {1, 4}; m in {3, 17, 65}; shift sets: narrow (rho 0.02), medium
(rho 0.2), wide (shifts spread over [0.05, 2], forcing splits), all-equal
(common shift), and two-cluster; tol = 1e-10 absolute. 3 x 2 x 3 x 5 = 90
configurations, seed fixed in the test.

## Gate

G1 Correctness: every Certified candidate of every method on n = 8 encloses
   the exact error (exact rational oracle, `fractions.Fraction`, of the exact
   binary inputs); every target ends Certified or explicitly Rejected (a
   rejected target after fallback is reported, never dropped).
G2 Accounting: for every method and configuration the executed flop count
   equals the sum of its charged components, failed attempts included, and
   the predicted count of M1-M3 equals the executed count (they have no
   data-dependent branch); M4 predicted vs executed difference is reported.
G3 Selector: on at least 90 % of configurations the selector's choice has
   the least executed all-in flop count (ties count as correct).

Reported: where M4 has a positive executed margin over the best of M1-M3,
by (n, r, m, set); zero such configurations is a valid outcome (the node
then records "no positive all-in margin" for the jet in this domain).
No timing, no speed claim, no default dispatch.

## Results (append only after the recorded run)
