# PLOT-DRIVEN CRAG AUDIT — v2.9

## Correctness

`CALIBRATION_A34_VS_FULL_E_ERROR.png` directly contradicts monotone safety ordering: unsafe rows occur below and above A34=1, while safe rows span both sides as well.

## Retrieval

Classical automatic method selection supports causal method-state adaptation from already-computed integration information; it does not supply an A34 safety theorem. Modern risk-control/selective-prediction frameworks motivate abstention and held-out calibration, but their formal guarantees require sampling assumptions not established for sequential ODE events. Therefore this project keeps the solver holdout as scientific authority.

## Augmented checks

- Dimension/tolerance mutation from v2.8 discovery to N=96/256 fresh calibration destroys the old strict gap.
- Family mutation shows unsafe rows in HIRES, nonautonomous forcing and Van der Pol; the failure is not confined to one family.
- Budget mutation is benign: all prefixes remain under the fixed reserve.
- Threshold mutation shows unsafe enters at recommendation 9, before the predeclared minimum coverage 12.

## Generation

The next feature should encode the three-stage *shape* of nonlinear-defect evolution, preferably retaining rho2, rho3, rho4 individually and deriving scale-free log-slope/curvature quantities. A34 alone should be retired from authority status but retained as a diagnostic component.
