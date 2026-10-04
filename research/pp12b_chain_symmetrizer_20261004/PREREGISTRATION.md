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

### Executed result — 2026-10-04 (source `e698541`)

Commands: `PP12B_CASES=research/pp12b_chain_symmetrizer_20261004/cases.json
cargo test --release -p rodas5p-core --locked --test pp12b_chain_symmetrizer
export -- --ignored --nocapture`, then `python3 tools/pp12b_chain_check.py
--cases .../cases.json --output .../RESULTS.json`. Contract test
`chain_symmetrizer_makes_pairs_equal` passes.

**Verdict: FAIL (G3: 16 of 18 stiff cases); G1 and G2 PASS.**
- G1: all 90 certificates (30 cases x 3 metrics) enclose the 50-digit
  error. G2: every `mu_up` is at or above the 100-digit `lambda_max`.
- G3: all six REV-02 F2 cases now meet `1e-8 ||exp(tau A) v||` (relative
  1.7e-14 to 3.0e-11; the chain symmetrizer is chosen for Pe 50 at tau 0.1
  and both Pe 200 cases, with `mu_up = -2/h^2` exactly as predicted), and
  10 of the 12 holdout cases do (relative 4e-14 to 3.5e-10). The two that
  fail are central differences with n = 48 at tau = 0.2 (Pe 100 and 400):
  their exact solution norms are 6.2e-373 and 5.0e-414, below the binary64
  range, so the binary64 candidate underflows and no binary64 result can
  meet a relative criterion there. That is a defect of the holdout design
  (decay was not checked against the representable range before fixing
  it), not of the bound, which is still valid (absolute 2.4e-209 and
  9.4e-156 with the chain metric, 1.8e-14 and 1.1e-14 with the identity).
- Transport of the chain metric reaches 5.2e45 (n = 48, Pe 100, cell
  Peclet near 1); it is charged in the bound.
- F3 mu = 30 and the Jordan block mu = 100 are unchanged (useless bounds,
  transient growth); no metric of the three helps there.
REV-02's FAIL (L-0060) and PP12's FAIL (L-0075) stand as recorded.
