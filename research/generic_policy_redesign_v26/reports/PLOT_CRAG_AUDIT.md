# P1-01R v2.6 Plot CRAG Audit

## Correctness
`ATTEMPT_CAUSALITY_REJECTED_TARGETS.png` shows a small but nonzero frozen-k1 failure fraction in both N=128 and N=512. The k=3 diagnostic remains zero. The small pooled percentage must not hide the fact that a single rejected next proposal is sufficient to invalidate accepted-grid runtime causality.

`ATTEMPT_CAUSALITY_H_SHIFT.png` shows the two k=1 counterexamples below the identity line: accepted-grid replay acts at 0.8 of the first runtime proposal. The k=2 N=512 counterexamples are even more severe (`h_accepted/h_first = 0.2`) and are recorded in the CSV evidence.

## Retrieval
The result is consistent with ordinary adaptive-controller causality: an accepted step does not reveal in advance whether the next proposed step will be accepted.

## Augmented mutations
- Dimension mutation N=128 -> N=512: frozen k=1 miss reproduces in the same nonautonomous family/decision location.
- Persistence mutation k=1 -> 2: the original miss disappears, but new semilinear N=512 misses appear.
- Persistence mutation k=2 -> 3: current development corpus has zero rejected-first targets.
- Work mutation: rejected trials are retained explicitly rather than represented by the eventually accepted row.

## Generation
Prediction for the next node: actual prefix measurement must be attached to the first proposed trial `h_trial`; pricing it at accepted-grid `h` will understate or mischaracterize speculative work in the counterexample cases. k=3 is the only current persistence survivor worth spending real prefix work on, with frozen k=1 retained as a comparator.

## Claim status
- Accepted-grid runtime authority: REJECTED.
- Attempt-level R-JF trace mechanism: SURVIVING.
- k=2 causal-filter claim: REJECTED on N=512.
- k=3 causal-filter claim: NARROWED / development survivor only.
