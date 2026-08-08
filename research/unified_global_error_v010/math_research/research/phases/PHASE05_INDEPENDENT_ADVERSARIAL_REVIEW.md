# Phase 5 — Independent adversarial review

## Principal attacks

1. **Grid bias:** sampling each method at its own step points rewards or penalizes different resolutions.  
   **Disposition:** use one common grid per problem that is a subset of every nested step grid.
2. **Interpolation contamination:** linear interpolation would reduce observed order of high-order methods.  
   **Disposition:** no interpolation in P0.
3. **Timing contamination:** parallel case execution makes per-candidate timing contended.  
   **Disposition:** wall ranking only from one-thread runs; parallel runs report aggregate throughput.
4. **Reference circularity:** using one candidate as the reference can favor it.  
   **Disposition:** exact analytic references only in P0.
5. **Memory overclaim:** stored trajectory bytes are not process peak RSS.  
   **Disposition:** call it `stored_state_bytes`; peak RSS remains optional/not measured.
6. **Scalar-score gaming:** arbitrary work weights can promote a preferred family.  
   **Disposition:** nondominated fronts per cost metric and target-attainment tables.
7. **Short-run startup omission:** BDF2 can appear artificially cheap.  
   **Disposition:** execute complete trajectories from startup and retain all counters.
8. **Roundoff floor:** ultra-small errors can invert local slopes.  
   **Disposition:** P0 reports points and fronts; no order fit is used for ranking.

## Review verdict

Proceed with a fixed-step, exact-reference authority. Do not claim production ranking or generality beyond the exact-solution corpus.
