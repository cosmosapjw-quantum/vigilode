# PHYS–MATH AUDIT — persistence/budget accepted-grid preflight

## Verdict

**Mechanism evidence: PASS. Runtime-causal policy authority: HOLD.**

## Load-bearing findings

1. The frozen selected feature `error_curvature` is actually
   `-(log10(error_j)-log10(error_{j-2}))`: a two-accepted-step error drop, not a
   mathematical second difference. The value is dimensionless because the
   embedded adaptive error norm is dimensionless; the name is misleading.
2. `jvp_pressure` and `step_contraction` contain `log10(|h|)` and are not invariant
   under a change of time units. They are research features only. The selected
   policy does not use them.
3. The next-state causality amendment correctly forbids same-row E labels, but an
   accepted-row atlas still does not expose rejected next attempts. A trigger
   observed after accepted step `j` acts on the controller's *first proposed*
   next interval, not necessarily on the eventually accepted row `j+1`.
4. The new R-only runs show 18 rejected attempts at N=128 and 21 at N=512, so the
   distinction is real, not a zero-measure edge case.
5. `1.25 * max(calibration prefix_{j+1}/R_j)` is an empirical cap, not a certified
   mathematical upper bound. The T14 pathwise budget theorem applies only
   conditional on a genuine prospective upper bound or interruptible work cap.
6. Historical `exponential_prefix_wall_median` measures the G4-S4 fused-Arnoldi
   acquisition prefix, not the full pexprb54s4 dependency-level-1 `(U2,D2)` probe.
   Those quantities must not be conflated in later safety claims.

## Limits

No active switching, E-K continuation, or new safety claim is authorized by this
node. N=2048 remains sealed.
