# Decision Log

1. Use shared controller semantics and family-specific estimators.
2. Preserve fixed-step APIs.
3. Step doubling is a reference lane, not an efficiency claim.
4. Accept two-half-step BDF history transactionally.
5. Defer native RADAU5 estimator and BDF3–5.
6. Use local Rayon only across independent cases; do not claim solver-internal speedup.
7. Do not rank high-order methods from the clipping-limited canonical screen.
8. Promote the common adaptive research layer; hold every production/performance claim.
