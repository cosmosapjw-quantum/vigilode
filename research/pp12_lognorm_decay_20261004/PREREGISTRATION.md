# PP12 — decay-aware nonnormal bound with a verified logarithmic norm (prospective registration)

RVJ DAG node `PP12` (depends on SAFE-ENCLOSURE, done: L-0064/L-0065).
Base commit: the commit that adds this file. REV-02 (L-0060) stays FAIL;
this node records a new outcome on its families.

## Method

For `B = D A D^-1` (identity or Osborne metric, entries as directed
intervals), a verified upper bound `mu_up >= lambda_max((B + B^T)/2)` is
found by interval Cholesky of `mu I - S`, `S` the interval symmetric part:
if the interval Cholesky is feasible (every pivot's lower end > 0), every
symmetric matrix in `S` has all eigenvalues below `mu` (Alefeld-Mayer).
`mu` is searched from a binary64 estimate upward (estimate plus
`1e-12 (1 + |estimate|)`, the offset multiplied by 10 until feasible; at
most 40 tries; the Gershgorin bound otherwise). Then `||e^{tB}||_2 <=
e^{t mu_up}` (logarithmic norm). The stepped certificate of REV-02 is
evaluated with the propagation factor `e^{t mu_up}` (no Crouzeix-Palencia
factor on propagation; the local truncation term keeps it) and the same
steps, degree, Horner enclosure and repaired radius/decay compositions;
`certify_exp_action_lognorm_auto` takes the smaller of the identity and
Osborne bounds (both valid).

## Gate

G1 `mu_up >= lambda_max` of the exact binary symmetric part in every
   case and metric (50-digit eigenvalues).
G2 Every new bound encloses the 50-digit error of its candidate on all
   REV-02 families F1-F4 (same generator, same seeds).
G3 Stiff usefulness, REV-02 item 3's threshold unchanged: every F2 case
   has `error_upper <= 1e-8 ||exp(tau A) v||`.
Reported: absolute and relative bounds, `mu_up` vs Gershgorin `re_hi`,
the metric chosen, F3/F4 bounds (expected still loose for strong
transient growth), and the transport factor of the metric.

## Results (append only after the recorded run)

### Executed result — 2026-10-04 (source `f7343c1`)

Commands: `PP12_CASES=research/pp12_lognorm_decay_20261004/cases.json
cargo test --release -p rodas5p-core --locked --test pp12_lognorm_decay
export -- --ignored --nocapture`, then `python3 tools/pp12_lognorm_check.py
--cases .../cases.json --output .../RESULTS.json`. Contract test
`lognorm_is_negative_for_diffusion` passes; REV-02, INT-05 and
SAFE-ENCLOSURE contract tests still pass (the REV-02 path is unchanged).

**Verdict: FAIL (G3; G1 as computed).**
- G2 PASS: every one of the 36 certificates (18 families x 2 metrics)
  encloses the 50-digit error.
- G1 as computed FAIL, not a real violation: the 50-digit check reported
  `mu_up < lambda_max` by 1.4e-48 to 2.7e-51 in three 2x2 VIG-A02 rows where
  `mu_up` came from the Gershgorin fallback and equals `lambda_max` exactly.
  A post-hoc closed-form check in exact rationals (`posthoc_g1.py`,
  `POSTHOC_G1.json`) confirms exact equality in those three rows (and
  `mu_up - lambda_max = 0.0078` in the fourth 2x2 row). Every other row has
  `mu_up` from the interval Cholesky above `lambda_max`. The registered
  instrument did not allow for its own precision; the verdict does not
  rest on G1.
- G3 FAIL: three of six F2 cases meet `1e-8 ||exp(tau A) v||` (Pe 10 both
  taus, relative 1.7e-14 and 4.6e-13; Pe 50 at tau 0.01, 2.2e-14); Pe 50 at
  tau 0.1 and Pe 200 at both taus do not (relative 9.8e9, 2.0e-7 against a
  solution of norm 2.0e-7, and 1.1e77). The verified `mu_up` is -9.86 (the
  diffusion eigenvalue) where Gershgorin gave 0, so the bound now decays
  by `e^{-0.99}` over tau = 0.1, but the solution decays at the rate of the
  convection-dominated spectrum (about `e^{-218}`). The power-of-two Osborne
  metric equals the identity for Pe 200 and helps little for Pe 50, because
  the symmetrizing ratio `sqrt(|sup/sub|)` (about 0.71 at Pe 200) is not a
  power of two. A real-valued diagonal metric would make the off-diagonal
  part skew and `mu = -2/h^2`; that is a follow-up, not this node.
- Reported: F1 at relative 1.2e-15 (Osborne); F3 mu = 1 at 1.2e-14 to
  1.4e-14 relative, F3 mu = 30 and the Jordan block mu = 100 still useless
  (`mu_up` 34 to 93 > 0: transient growth), F4 mu = 10 at 1.1e-14.
