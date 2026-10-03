# INT-06: native q=2 activation, decision record

The external review (`20261002_thread_transfer/REVIEW.json`, `native_q2_activation_requires`) made native q=2
activation conditional on N1, N2, N3, new preregistered cost evidence, and the original target-distance budget and
binding gates. This record collects that evidence and states the decision. It runs nothing new; the margins below
are derived from recorded outputs.

## Evidence

| Requirement | Evidence | Status |
|---|---|---|
| N1 action-first certificate | L-0035: exact containment, 0.565x the counted operations of the matrix order, workers 1/2/4/8 bit identical | met |
| N2 residual-seeded radius | L-0036: closes every R4 fixture in one preflight and one check; the old policy kept | met |
| N3 structural witness and prepared context | L-0053: bitwise the dense bounds and decisions on 87 candidates, 2n instead of 2n^2 slots, one construction per attempt, slope 1.00 in n. The node is FAIL on a threshold for the dense comparison arm, not on the structured path | met in substance; node verdict FAIL |
| Budget and binding gates | the native path's checks (step `(y, h)` identity, model agreement at `f(y)`, at one JVP and at every stage state, `is_bound_to`, certified budget) are kept unchanged in the prepared path (L-0053 contract tests) | met |
| Preregistered cost evidence | L-0050: net margins in sequential-solve units, ideal 8 workers, no dispatch charge | see below |

## Net margins with the structured certificate (derived)

L-0053's dense arm replays L-0050 attempt for attempt (same lanes, acceptance and certificate operations). Taking
L-0050's per-attempt solve unit `c_solve` and critical-path depth, and replacing the serial certificate's operations
by the structured certificate's (L-0053), gives these total margins over the run:

| n | attempts | serial certificate (L-0050) | structured certificate (derived) | mean per attempt, structured |
|---|---|---|---|---|
| 1 | 3 | -61.6 | -61.6 | -20.5 |
| 2 | 5 | -60.2 | -49.8 | -10.0 |
| 4 | 6 | -33.6 | -19.3 | -3.2 |
| 8 | 10 | -20.7 | -3.3 | -0.33 |
| 16 | 15 | -6.3 | +9.4 | +0.63 |

The unit assumes 8 ideal workers and charges no dispatch, synchronization or allocation. The positive margin at
n = 16 is under one solve unit per attempt.

## Decision

**Native q=2 admission is not activated by default.** `Q2Admission::NativeTargetCertificate` and
`Q2Admission::PreparedStructuredCertificate` stay opt-in, and the router keeps abstaining for n <= 8, where every
margin is negative.

- **n >= 16 is a candidate, not a decision.** It would need a new preregistered node. That node would measure the
  real dispatch and synchronization cost, use the structured certificate, and run on a host where the timing
  authority holds. The timing authority is on HOLD, and a counter margin of +0.63 solve units is smaller than any
  realistic dispatch cost would need to be.
- **The structured path is retained as research code.** It is correct (bitwise equal) and cheaper, and it is the
  right certificate for any future activation.

Not decided here: automatic backend routing, a new default radius policy, deprecation of v2, timing promotion, and
holdout release. These are the review's own "decisions intentionally not made".
