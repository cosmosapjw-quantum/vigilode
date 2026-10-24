# Independent review of the conditional chart sweep

The fixed-P continuation worker independently read the complete `conditional_chart_sweep.rs` example and the invoked `ConditionedPolynomialPlan::apply` stopping logic. No source changes were made during this review. Reviewed example SHA256: `0b1e4ac97da526bff07cc58f6112919239057348cba5492eece5ba8f46b0009b`.

No blocking defect was found in the stated finite-dimensional structural experiment. This conclusion does not extend to arbitrary user-supplied operators, whole ODE trajectories, or a guaranteed wall-time cap.

## Mathematical checks

Each operator is an exact dyadic block-Jordan matrix with diagonal `beta` and superdiagonal `kappa`; each pair solves `(I-B)x=b`. The direct inverse and directed reference use `a=1-beta`, `x_odd=b_odd/a`, and `x_even=(b_even+kappa*x_odd)/a`. The infinity-norm inverse bound `1/a+|kappa|/a^2` matches the nonnegative absolute row sums for every supplied case. The outward reference solves the same exact represented matrix and RHS as every method; it is not a manufactured solution obtained by a rounded matrix-vector product.

The final residual enclosure evaluates the original matrix, with the correct sign `b_even-a*x_even+kappa*x_odd`. Multiplication by the outward inverse bound gives a valid final solution-error upper bound. Separately, interval subtraction against the directed reference gives upper and lower error bounds. The lower bound is used only to establish failure; an interval that merely straddles the requested tolerance remains unresolved. This avoids treating a loose upper bound as proof of inaccuracy.

The callback residual allowance uses a standard floating gamma bound and a generous underflow allowance. The original-action callback refuses candidate infinity norm above 32, which supplies the previously required state magnitude premise. Matrix entries and their row sums are dyadic in these fixtures, so their represented values and the derived norm inputs do not conceal an unaccounted rounded physical operator. The dense implementation changes action cost, not the mathematical matrix. The final interval residual remains an additional independent check rather than accepting the chart's surrogate residual alone.

For the degenerate chart, the original matrix is exactly zero while the normalized denominator loses its resolvent margin. The setup refusal is correctly restricted to that chart representation. The direct original system remains regular and is solved; no original-system singularity claim follows from the chart failure.

## Execution and timing checks

Every route is subjected to the same final outward original residual certificate. A rejected chart candidate leaves the output sentinel unchanged, and the direct fallback solve is charged inside the timed route. Candidate capture and directed-reference comparisons occur in a separate diagnostic execution outside the timing loop. Setup and calibration costs are reported separately because the experiment reuses the plan/workspace; these costs must be charged for a one-off application or amortized explicitly.

The capped route estimates available chart work from measured first-pass and direct-route times. The factor-two pass proxy is empirical, as documented. Its action/correction count is bounded, but operating-system scheduling, data-dependent callback cost and calibration variance prevent interpreting the estimated ratio as a universal wall-time guarantee. The final observed time ratio and fallback counts remain necessary evidence.

`chart_accepted` is distinct from `final_accepted` and scientific success. A direct fallback can yield a valid final answer while the chart failed; the JSON retains both events. Instrumentation and certificate timers are present in every route and are disclosed. This is a structural-kernel experiment, not an uninstrumented end-to-end solver ranking.

## Retained evidence checked

The reviewer read `conditional-chart-round3-{tests,build,sweep}.receipt.json` under `evidence/runs/`; all report exit zero. Parsing the final sweep independently gives 384 samples: 336 `PASS_SCOPED` and 48 `HOLD_UNRESOLVED_ERROR_BOUND`. Every passing sample has both the final inverse-residual certificate and directed-reference error upper bound within its requested tolerance. No passing sample had `final_accepted=false`.

The 48 unresolved samples at the tightest tolerance are retained as uncertainty-limited results; they are not proved mathematical failures. No additional native rerun was performed because the existing final receipts cover unchanged reviewed source, and the review found no new execution defect requiring another run.
