# G2 PHYS–MATH AUDIT

## Conventions and dimensions

- State and time in the canonical foundation tests are dimensionless.
- JVP has the same units as `F/y`; `hJ` is dimensionless.
- `φ_k(hJ)` is dimensionless, so `h φ_k(hJ) F` has the units of the state.
- Sign convention in stiff linear tests is `z=hλ<0`; exact decay is `exp(z)`.

## Audit ledger

| Item | Status | Evidence / limitation |
|---|---|---|
| Published pexprb54s4 coefficient transcription | PASS | exact rational constants and 100-digit condition check |
| Selected stiff-order identities | PASS WITH SCOPE | residual `<=2.22e-16`; not a full theorem reconstruction |
| Matrix exponential / φ recurrence | PASS | scalar scales, nonnormal recurrence, 100-digit oracle |
| Nonlinear declared order | PASS | observed `1.956`, `3.925`, `4.929` |
| Stiff linear decay | PASS WITH SCOPE | linear remainder vanishes; nonlinear stiff gate deferred |
| Oscillatory amplitude/phase | PASS WITH SCOPE | linear damped block only |
| Explicit Jacobian avoidance | PASS | strict `linearize_matrix_free` path |
| Newton avoidance | PASS | zero nonlinear iterations in all G2 primary steps |
| Nonautonomous consistency | NOT TESTED | augmented clock/state deferred |
| Mass-matrix consistency | NOT TESTED | identity mass only |
| Positivity/invariant preservation | NOT CLAIMED | no generic guarantee supplied |
| φ-action a posteriori bound | HOLD | nested-dimension difference is heuristic |

## Strongest counterexamples not yet closed

1. Highly nonnormal nonlinear systems where nested Krylov-dimension differences underestimate the φ error.
2. Nonautonomous forcing where omitting the augmented time component changes the declared order.
3. Constant noncommuting mass matrices, for which `M^{-1}J` and JVP cost/conditioning must be handled explicitly.
4. Large Krylov dimensions where full reorthogonalization makes the short dependency graph economically irrelevant.
5. Branching/non-smooth RHS where exponential Rosenbrock smoothness assumptions fail.

## Final mathematical verdict

The method formulas and small-scale matrix-function layer are internally consistent in the stated autonomous identity-mass regime. The foundation supports the next adaptive/KIOPS experiment, but not a generic production-solver claim.
