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
