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

### Executed result — 2026-10-04 (source `f01ce4a`)

Commands: `PP04_CASES=research/pp04_shared_shift_policy_20261004/cases.json
cargo test --release -p rodas5p-core --locked --test pp04_shared_shift_policy
export_policy_study -- --ignored --nocapture`, then `python3
tools/pp04_shared_shift_check.py --cases .../cases.json --output
.../RESULTS.json`. Contract tests (Hessenberg reconstruction, all methods
certify a small family, typed abstentions) pass.

**Verdict: PASS.**
- G1: 8,500 candidate columns on n = 8 (all four methods), all Certified,
  every one enclosing the exact rational error (largest error / bound
  0.30). No target of any method in any of the 90 configurations was
  rejected, so the jet's fallback path was never taken (0 fallbacks).
- G2: executed flops equal the independent Python formulas for every
  method and configuration, and the planner's prediction for M1-M3.
- G3: the selector chose a cheapest method in 87 of 90 configurations
  (ties counted correct). The three misses are `two-cluster` with m = 3
  (n = 32, r = 1; n = 96, r = 1 and 4): the `WideCluster` rule (two
  clusters for three distinct shifts) made the jet abstain although it was
  cheaper by 4 % to 34 %.

Reported: the jet has a positive executed margin over the best of M1-M3 in
49 of 90 configurations: all 17 narrow configurations except (8, 4, 3)
(1.05x to 12.6x), 13 of 18 medium (1.1x to 8.2x), 14 of 18 two-cluster
(1.04x to 6.1x), 5 of 18 wide (1.0x to 1.7x; the wide set needs several
clusters) and none of the 18 all-equal (common-shift LU wins by
construction). The margin grows with m and n and shrinks with r and with
cluster width. Common-shift LU equals individual LU whenever shifts are
distinct, and Hessenberg reuse never had the least count here (its
reduction costs 7 LUs). These are counted flops with an opaque LU charged
by its standard count; directed certificate work is the same for every
method and excluded; no timing and no speed claim.
