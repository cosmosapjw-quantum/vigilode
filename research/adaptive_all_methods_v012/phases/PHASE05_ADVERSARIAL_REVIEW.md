# Phase 5 — Independent Adversarial Review

## DEFENDED

- H1 shared controller/native estimators: clean separation of policy and evidence.
- H2 step doubling as an oracle/reference, not a universal performance claim.
- H4 exact variable-step BDF2 coefficients with transactional history.

## TRADEOFF

- H5 Radau step doubling: robust but intentionally expensive.
- H6 cost-aware control: likely necessary later, but must not precede estimator reliability.

## INFORMATIVE FAILURE RISKS

1. A step-doubling path that accepts two half steps changes the actual BDF history grid; this must be explicit in counters and later coefficient ratios.
2. Output-time clipping changes the proposed step and therefore controller history. The clipped value, not the unclipped proposal, must enter the next-step update.
3. SABR/homotopy fallback may hide that the speculative method rarely succeeds. Fast-path and fallback work must remain separate.
4. A common PI formula does not imply common stability. Method order, estimator order, step-ratio sensitivity, and iterative cost remain method-specific.
5. Tolerance ladders must be calibrated per method and evaluated by external global error, not raw tolerance equality.

## REJECTED SHORTCUTS

- Reusing fixed-step BDF2 coefficients after arbitrary step changes.
- Assigning exponent \(-1/5\) to every method.
- Counting rejected trial work as zero.
- Updating history/recycle state before acceptance.
- Calling step doubling a free embedded pair.
