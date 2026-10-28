# Exact-constant extension after the user's precision requirement

This is a separately identified extension of the completed structural timing
experiment, not a retrospective replacement of its frozen 1e-9 screen.
No additional timing parameter search is authorized or needed.

For the declared affine mean-field model derive the sharp residual-to-output
infinity norm by two scalar adjoint recurrences, and derive the exact discrete
propagation norm from the two scalar stability values. A requested global
algebraic error budget E yields delta = E / C_N, where C_N is the sharp
accumulated norm obtained from the shared two-channel projectors. The simpler
A sum_{j=0}^{N-1} G^j bound is retained as a comparison.
There is no fitted safety factor, denominator regularizer, or arbitrary
fractional allocation in this formula. E is an accuracy specification, not a
universal mathematical constant. Truncation and floating evaluation errors
remain separate from the algebraic-stage error statement.

Before running: an independent dense original-stage operator tests the sharp
norm and sign-aligned equality cases for n=1,3,7, three rate pairs including
zero and equal rates. A finite trajectory diagnostic will use n=17, d=20,
kappa=19, N=64, the previously defined state/forcing family, and E=1e-8.
Record the derived screen, original-space residual maximum, full-state
reference error, same-target difference and charged successful work.
Run both prior sequential and quotient arms under this same derived screen.
The earlier native timing receipts remain unchanged.

The implementation evaluates the theorem's formulas in ordinary f64.
Neither its constants nor its sampled residual are labeled outward-rounded
interval certificates. Independent exact Fraction checks use the represented
tableau and problem coefficients to check the analytic construction.

Shared tableau initialization has one fixed 8x8 inverse in a cached metadata
setup. State-sized/per-step matrix factorizations remain zero. The original
timed diagnostics warmed this shared cache before every measured batch.
