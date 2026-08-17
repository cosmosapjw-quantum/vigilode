# Plot-driven CRAG audit — v2.7

## Plot 1: LEVEL1_RHO_VS_HISTORICAL_FULL_E_ERROR

**Reading.** There is broad positive structure, but no monotone safety boundary.
The only points above the E local-error threshold occur at moderate level-1 rho,
while both smaller and much larger rho values can be admissible. The HIRES
unsafe point at rho≈2.27 is the decisive counterexample.

**C — Correctness:** consistent with the numerical table and the claim that
rho is informative but not sufficient.

**R — Retrieval:** consistent with the nonlinear-remainder theory: one early
remainder is not a theorem about later-stage amplification.

**A — Augmentation:** the same unsafe HIRES location is reproduced at N=128 and
N=512, so it is not a one-dimension plotting accident.

**G — Generation:** a useful next diagnostic must add information after level 1
(stage-3/4 remainder or an equivalent curvature/amplification witness), not tune
another rho threshold.

## Plot 2: LEVEL1_PREFIX_JVP_COST

**Reading.** Level 1 is a small fraction of target R-JF vector work and roughly
one fifth of historical full-E JVP work at the median. It is cheap enough to be
an admission probe but is not free.

**Adversarial mutation:** replacing full-trajectory aggregate ratios by
per-event ratios raises apparent overhead to several percent. Therefore the
correct cost statement must specify the denominator and cannot quote only the
small trajectory-aggregate number.

## Plot 3: K3_FAMILY_E_STRONG_PRECISION

**Reading.** All families except HIRES are 1.0 in this small audit, while HIRES
is 2/3 in both dimensions. Pooled precision around 96% therefore hides a stable
family-local failure.

**Adversarial mutation:** groupwise reporting changes the scientific verdict
from “almost perfect” to “one repeated structural blind spot”. Promotion gates
must remain family-aware.

## Plot 4: K1_REJECTED_PROPOSAL_RHO

**Reading.** The frozen k=1 causal misses have extremely large actual level-1
rho (~204 and ~207). This directly demonstrates why probing the eventually
accepted smaller step would distort the diagnostic geometry.

## Final CRAG status

- **Surviving:** actual resumable level-1 prefix; causal first-proposal
  placement; k=3 as a development candidate; low aggregate vector-work cost.
- **Narrowed:** k=3 avoids rejected-next targets only on the observed N=128/512
  corpus, not generically.
- **Rejected:** level-1 rho as a standalone full-E safety certificate; pooled
  precision as sufficient promotion evidence; accepted-grid next-row prefix
  costing as runtime authority.
