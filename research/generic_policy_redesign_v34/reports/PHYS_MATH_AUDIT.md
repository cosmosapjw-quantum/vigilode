# v3.4 PHYS–MATH AUDIT — Sealed zeta34 Safety Holdout

## Verdict

OVERALL HOLDOUT FAIL because the predeclared prefix-budget hard gate is violated once. The frozen zeta34 threshold itself has zero unsafe recommendations, but this N384 profile contains no unsafe audit-E events at all, so it is not a discriminating safety challenge.

## Frozen policy

`recommend E audit admissibility iff zeta34 <= 13.39706618860016`.

No feature, direction, threshold, k3 event rule, 80-JVP reserve, or 25% cumulative budget fraction was retuned after N192 calibration.

## Holdout observations

- eligible events: 27,
- audit-E unsafe events: 0,
- frozen-policy recommendations: 16,
- unsafe recommendations: 0,
- represented recommendation families: 5,
- maximum single-family fraction: 7/16 = 0.4375.

Thus the frozen safety/coverage rule passes as written, but because every eligible E audit is safe, the holdout cannot estimate false-safe discrimination on this profile.

## Budget counterexample

The N384 semilinear family has two eligible events. The second has

- actual prefix JVP vectors = 109,
- predeclared absolute reserve = 80,
- speculative JVP before target = 65,
- committed R-JF JVP before target = 29007.

Hence the absolute-reserve condition fails: 109 > 80.

The cumulative-fraction condition does NOT fail:

`(65+109)/29007 ~= 0.005999 < 0.25`.

Therefore the v3.4 hard-gate failure is specifically an absolute reserve-envelope failure, not exhaustion of the cumulative 25% token budget.

## Hard-cap invariant for the next node

Let R_k be committed R-JF JVP work and S_k cumulative speculative JVP work before an event. Let

`B_k = min(B_abs, floor(delta R_k - S_k))`

for nonnegative remaining budget, and let the speculative prefix abort before executing JVP vector `B_k+1`.

Then any completed or aborted prefix satisfies both

`incremental speculative work <= B_abs`

and

`S_{k+1} <= delta R_k`.

This is an exact pathwise invariant; it does not require a cost predictor. If the prefix cannot finish inside B_k, the safe action is abstention to committed R-JF.

The observed breach suggests implementing the reserve as an enforced work cap rather than treating 80 as a predicted upper cost.

## Claim ceiling

Allowed: the fixed zeta34 policy retained 16/27 recommendations and made zero unsafe recommendations on a holdout whose audit-E events were all safe; the full v3.4 policy nonetheless failed its budget contract.

Not allowed: “zeta34 safety is validated on N384”, “the v3.4 policy passed”, or any active switching/economic speedup claim.
