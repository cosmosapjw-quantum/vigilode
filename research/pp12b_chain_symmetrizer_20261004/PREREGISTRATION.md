# PP12b — chain-symmetrizer metric for the log-norm certificate (prospective registration)

Follow-up of PP12 (L-0075, FAIL on stiff usefulness). Motivated by that
result (post hoc): for convection-dominated central differences the
power-of-two Osborne metric equals the identity, and Osborne's fixed point
does not symmetrize a Toeplitz tridiagonal matrix (its row and column
sums are equal under every geometric scaling). The classical symmetrizer
of a tridiagonal matrix is `d_1 = 1`, `d_i = d_{i-1} sqrt(|a_{i-1,i}| /
|a_{i,i-1}|)` (when both entries are nonzero, else `d_i = d_{i-1}`); then
`|b_{i,i-1}| = |b_{i-1,i}|` and the symmetric part loses opposite-sign
off-diagonals. Base commit: the commit that adds this file.

## Change

`chain_symmetrizer_metric(a)` (real-valued, binary64; the certificate
encloses `D A D^-1` in interval arithmetic, so non-power-of-two `d` only
widens the enclosure) and `certify_exp_action_lognorm_auto` gaining it as
a third metric (the smallest of the three valid bounds is returned).

## Families

The 18 REV-02 families (unchanged) and a new holdout family not seen
before: central differences with n in {24, 48}, Pe in {100, 400}, tau in
{0.05, 0.2}, v = sin(pi i h) (8 cases), and the upwind variant
(first-order upwind convection) with n = 32, Pe in {50, 200}, tau in
{0.01, 0.1} (4 cases).

## Gate

G1 Every certificate (all metrics) encloses the 50-digit error.
G2 `mu_up >= lambda_max` with `lambda_max` from 100-digit eigenvalues and
   an allowance of `1e-90 (1 + |lambda_max|)` for that computation.
G3 Every F2 case and every holdout case: chosen bound `<= 1e-8
   ||exp(tau A) v||`.
Reported: metric chosen, transport factor of the metric, `mu_up`.

## Results (append only after the recorded run)
