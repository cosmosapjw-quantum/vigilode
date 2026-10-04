# PP07 — does the actual Fourier client need shared shifted actions? (prospective registration)

RVJ DAG node `PP07` (depends on PP04, PP06). Base commit: the commit that
adds this file. The DAG's kill conditions are evaluated on the actual
client, not on a constructed workload.

## Questions (answered from the native PP05/PP06 client)

Q1 Does the client's algorithm (Picard predictor, original-target
   certificate, step halving) perform any shifted linear solve? Counted by
   instrumentation of the native client.
Q2 For a frozen-Jacobian Newton correction of the step map (the natural
   place a shifted solve would appear: `(I - h J) d = residual` on the
   4 real unknowns of `(a, b)` at the step start), which distinct shifts
   would the actual runs produce (one per step size tried, including
   halvings), and how many per frozen J?
Q3 Does the current J have the Euclidean dissipativity witness of PP01
   (symmetric-part row bound <= 0)? The client is conservative on the leaf,
   so the expectation is no; then the alternative inverse gain
   `1 / (1 - h ||J||_1-row bound)` is computed where `h ||J|| < 1`.

## Decision rule (fixed now)

- If Q1 finds no shifted solve: record kill condition "client has no
  nearby distinct shifts" for the client as it is.
- For the Newton variant (Q2): the shared jet is admissible only for a
  frozen J with >= 2 distinct shifts known before the first solve; step
  halving reveals shifts one at a time, so a jet centred at the first shift
  is evaluated at later ones only if the radius is < 1/2; its counted flops
  (PP04 accounting) are compared with one LU per shift on exactly those
  systems.
- If Q3 fails and no alternative gain is < infinity: record "current J
  lacks required witness".
No 65-shift or other artificial workload is substituted. The correction
variant is not committed into the client's acceptance path.

## Gate

G1 Q1-Q3 answered with counts from the actual runs of PP05 (omega 0, 1,
   40, -40, 1e4) and stored.
G2 Every shared-jet evaluation done under the decision rule is certified
   by the residual with the gain actually used, or rejected; no gain 1 is
   used without the witness.
Verdict: PASS when G1 and G2 hold, whatever the decision (abstaining is a
valid outcome).

## Results (append only after the recorded run)
