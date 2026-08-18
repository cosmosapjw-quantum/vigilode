# Plot-Driven CRAG Audit — v3.0

## C — Correctness

`KAPPA_VS_FULL_E_ERROR.png` shows the five unsafe events on both sides of \(\kappa=0\), with safe events occupying the same horizontal interval. The plot therefore supports the numerical AUC result and directly rejects a simple signed-curvature safety interpretation.

`STAGE_SLOPE_PHASE_GEOMETRY.png` shows unsafe events inside the safe cloud in the \((s_{23},s_{34})\) plane. Drawing a new separating line from these same 48 points would be post-hoc feature fitting and is not admitted by the contract.

## R — Retrieval / external consistency

The exponential Rosenbrock literature supports stepwise flow linearization and the use of Jacobian-related matrix functions, but does not provide a theorem turning these three norm-shape quantities into an admissibility certificate. Classical automatic ODE method selection likewise supports causal use of information from completed steps, not this specific safety rule.

## A — Augmented adversarial checks

- Dimension mutation: N=96 and N=256 both give the same orientation for \(\kappa\), but the strength drops from 0.792 to 0.684 AUC; N=96 has only one unsafe point.
- Family mutation: leave-nonautonomous-out AUC falls to 0.533, nearly chance.
- Feature mutation: the other six predeclared scalar features are not more robust across dimensions.
- Wall-clock mutation: timing fields were excluded from parity authority; all non-wall numerical/work fields match v2.9 exactly.

## G — Generation

The surviving inference is not “fit a better scalar boundary.” The next information source should preserve vector geometry discarded by norms, especially alignment/rotation among \(D_2,D_3,D_4\).

Candidate predeclared next features should remain low-complexity and dimensionless, e.g. physical-component cosine similarities and orthogonal-remainder fractions. They must again be derived read-only from already-computed remainders before any threshold is considered.

## Final claim status

- `rho2/rho3/rho4` retention: SURVIVES.
- `s23/s34/kappa234` as descriptive diagnostics: SURVIVES.
- `kappa234` as a family-robust safety witness: REJECTED/HOLD.
- opening N=192 calibration now: REJECTED.
