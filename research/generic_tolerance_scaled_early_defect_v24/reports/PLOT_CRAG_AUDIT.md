# P1-00 Plot-driven CRAG adversarial audit

## C — Correctness

The quadratic triplet plot shows the intended mutation directly: the raw score
collision is broken by the tolerance scale, and both scaled defect and adaptive
error grow as tolerance tightens. The all-attempt scatter also shows why no
`rho=1` theorem is permissible: many accepted attempts have scaled scores far
above one.

The entry-window scatter has much cleaner separation than the full-residence
scatter, consistent with the complete threshold sweep rather than merely with a
visual impression.

## R — Retrieval / authority comparison

The code's own WRMS controller scale remains the primary authority. Standard
adaptive ODE practice likewise weights error norms by relative/absolute
tolerances; that supports the scaling principle but does not prove the proposed
early remainder is a local-error estimator.

## A — Augmented adversarial checks

1. **Tolerance mutation:** three frozen `rtol` values break the raw collision and
   preserve monotone scaled ordering on the predeclared quadratic triplet.
2. **Normalization mutation:** replacing raw `eta` with tolerance-scaled `rho`
   improves AUC, score/error rank correlation, and the false-count frontier.
3. **View mutation:** all-attempt geometry is mediocre; predeclared entry-window
   geometry is substantially stronger. This narrows the claim rather than
   allowing post-hoc choice.
4. **Problem mutation / leave-one-problem-out:** scaled entry geometry remains
   better than all-admit in every leave-one-problem-out subset, but the exact
   frontier moves; therefore a universal frozen threshold is not established.
5. **Sign mutation:** changing the sign of `D2` leaves the WRMS magnitude
   unchanged by design, confirming this feature carries no directional defect
   information.
6. **Timing mutation:** seven alternating-order pairs fluctuate from below to
   above unity; the plot does not support a precise positive-overhead estimate.

## G — Generated prediction

The surviving prediction is narrow: `rho_c2_wrms` may be useful **after an
independent R-JF-side event has opened an E-K shadow trial**, especially at the
first two E-K attempts. It does not by itself solve when to pay for that trial.

## Figure readings

- `QUADRATIC_TOLERANCE_TRIPLET.png`: decisive tolerance-information repair.
- `SCALED_SCORE_VS_ERROR_ALL.png`: broad positive relation but substantial
  accepted-score span; blanket threshold remains weak.
- `SCALED_SCORE_VS_ERROR_ENTRY.png`: much cleaner entry geometry; one/few
  frontier conflicts remain.
- `THRESHOLD_SWEEP_ENTRY_RAW_VS_SCALED.png`: conservative frontier improves from
  six to two false rejects at zero false accepts.
- `PAIRED_OVERHEAD.png`: visible timing scatter prevents a strong tail-overhead
  claim.

## Final claim status

- tolerance-aware diagnostic mechanism: **SURVIVING CLAIM**;
- all-attempt blanket admission rule: **NARROWED / not promotion-ready**;
- entry-conditioned read-only feature: **SURVIVING TO NEXT SHADOW NODE**;
- universal threshold / active early abort / switching speedup: **NOT CLAIMED**.
