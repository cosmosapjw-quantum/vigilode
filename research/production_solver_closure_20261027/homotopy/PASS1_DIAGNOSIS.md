# Correction pass 1: transcendental work in the direct inverse

Pass0 met all 22 contract tests and all 18 accuracy arms. The actual cost
diagnosis is that the cbrt/hypot inverse cuts polynomial evaluations but can
lose against stack Scalar Picard. On the nonlinear timing fixture its five
block scalar/algebraic median was 0.77055; the broad range is retained in
TIMING_RESULTS.json. This is adverse evidence, not a universal slowdown claim.

The corrective alternative uses a rational inverse-series candidate. Solving
for a [1/1] rational expression matching 1-beta+3 beta² determines the exact
coefficients uniquely: (1+2 beta)/(1+3 beta). Its original normalized cubic
residual is beta³(3+8 beta)/(1+3 beta)³ >=0. These coefficients are algebraic,
not fitted to the measurements. The unchanged outward original residual decides
acceptance; missed budgets fall back to Scalar Picard/bisection and all work is
charged. Keep the expensive exact inverse as the diagnostic control.

This is correction iteration 1 of the user's maximum 3 for this task. No
physical tolerance, grid-selection inequality, interval certificate or
resource cap is relaxed. Pass0 raw data and source/binary identities remain.
