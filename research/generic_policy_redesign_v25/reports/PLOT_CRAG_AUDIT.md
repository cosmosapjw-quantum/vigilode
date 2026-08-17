# Plot-driven CRAG audit

## Correctness
The calibration cloud shows a clear Pareto trade: k=2/3 persistence sharply
raises precision for small opportunity loss. The frozen k=1 star is an artifact
of lexicographically maximizing opportunity before precision.

## Retrieval / contract cross-check
The result is consistent with the prior mathematical T13 persistence argument:
1–2 accepted-step confirmation can lose little residence value. It is also
consistent with T15/T19: safety/precision and economics should be separate, and
pooled metrics can hide group failures.

## Augmented adversarial checks
- N=128→512 frozen policy: pooled metrics are stable.
- Family split: HIRES and Van der Pol expose precision failures hidden by pooling.
- Candidate objective mutation: k=2 gains ~13 percentage points pooled precision
  for <1 percentage point worst-dimension opportunity loss; k=3 is the unique
  development candidate with pooled precision 1.0 and opportunity >=0.75.
- Causal-index mutation: accepted-grid next-row scoring is invalid when rejected
  next attempts exist; 18/21 such attempts are observed.

## Generation
Prediction for the next implementation node: attempt-level accounting should
increase the measured speculative burden whenever an event is followed by an R
rejection, and may change both probe precision and the prospective wall ratio.

## Claim status
- R-only feature reproducibility: SURVIVES.
- Cross-dimension accepted-grid economics: NARROWED to retrospective development evidence.
- Runtime-causal event/prefix policy: REJECTED pending attempt-level replay.
- N=2048 promotion run: BLOCKED.
