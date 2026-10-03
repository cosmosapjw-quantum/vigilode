# Preregistration: physical residual-to-output budget for the raw-U and K drivers (remaining-only DAG node R-NEXT-01)

## Question

L-0038 item 3 compared one step of the U-form MF driver with the K-form sequential MF step at a tight Krylov
tolerance. 22 of 36 comparisons failed a relative criterion: mostly error norms of 1e-13 to 1e-9 that differed by
1e-6 to 7x relative, plus the Robertson h = 1e-2 divergence and GCRO-DR failures on the Brusselator. Can each
driver's deviation from its own exact step be bounded from quantities a driver can obtain? Those are the stage
residuals, an inverse-operator bound and a Jacobian bound on a box. Does that additive budget, plus the exact
coefficient difference of the two forms, explain every discrepancy? And do the drivers ever accept a step whose
error norm is not resolved by its budget? L-0038 is not rewritten; this node has its own gate.

## Exact targets

For a frozen `(t, y, h)` and the stored binary64 coefficients (taken as exact reals), the exact U step solves
`W U_i = h gamma f(t + c_i h, y + sum_j A_ij U_j) + gamma sum_j C_ij U_j + h^2 gamma gamma_i f_t` with
`W = I - h gamma J(t, y)`, `y_new = y + sum_j b_code_j U_j`, `e = U_(s-1)`. The exact K step solves
`W K_i = h f(t + c_i h, y + sum_j alpha_ij K_j) + h J sum_j Gamma_ij K_j + h^2 gamma_i f_t`,
`y_new = y + sum_j b_j K_j`, `e = sum_j btilde_j K_j`. `f`, `J` and `f_t` are the exact functions (exact `sin`/`cos`
for Prothero-Robinson). The two exact steps differ only through the coefficient transform: the
`E_native_coefficient` term.

## Budget (per driver X in {U, K})

The computed stages `S_i^c` (exported bit for bit) satisfy exactly `W S_i^c = b_i(S^c) + r_i`, where `b_i` is the
exact right-hand side evaluated at the computed earlier stages. `r_i` contains both the linear-solve residual and the
rounding made while forming the right-hand side. With `d_i >= ||S_i^c - S_i^*||_2`:

- U: `d_i = ||W^-1|| (h gamma L_i sum_j |A_ij| d_j + gamma sum_j |C_ij| d_j + ||r_i||)`;
- K: `d_i = ||W^-1|| (h L_i sum_j |alpha_ij| d_j + h ||J|| sum_j |Gamma_ij| d_j + ||r_i||)`,

where `L_i` bounds `||J||_2` on the box of radius `sum_j |A_ij| d_j` (resp. `alpha`) around the computed stage
state. This is an interval evaluation of the polynomial Jacobian, using the Frobenius norm. `||W^-1||_2` is bounded
by `sqrt(||W^-1||_1 ||W^-1||_inf)` of a 50-digit inverse. Then
`||y_new^c - y_new^*|| <= sum_j |b_j| d_j + rho_y`, where `rho_y` is the exact rounding of the reported `y_new`. The
embedded vector deviates by `d_(s-1)` (U) or `sum_j |btilde_j| d_j` (K). The error-norm deviation `B_X` adds the WRMS
of that deviation (with the scale's lower bound `atol + rtol |y|`), the scale's sensitivity to `y_new`, and the
rounding of the reported norm (`4 n u` relative).

`E_native_coefficient = |err(U^*) - err(K^*)|` and `E_projection_rounding` (`rho_y`) are computed exactly in 50-digit
arithmetic. The nonlinear remainder is inside the `L_i` term (a mean-value bound over the box), so no separate
linearization is assumed.

## Cases and command

The 36 L-0038 one-step comparisons: the six problems of L-0038 at their two step sizes, from the initial state, with
GMRES, LGMRES and GCRO-DR at Krylov rtol 1e-12 (maxiter 4000, atol 1e-14) and output tolerances `atol = 1e-6 x
scale`, `rtol = 1e-6`. A comparison in which a driver's Krylov solve fails is recorded as a solver failure (R-NEXT-03
scope) and is not part of items 1-3.

1. `RNEXT01_EXPORT=research/rnext01_residual_output_20261003/stages.json cargo test --release -p rodas5p-integrators --locked --test rnext01_residual_output -- --ignored --nocapture --test-threads=1`
   exports, per case, the inputs, coefficients, both drivers' stages, `y_new`, error vectors and norms (bits).
2. `python3 tools/rnext01_residual_output.py --stages research/rnext01_residual_output_20261003/stages.json --output research/rnext01_residual_output_20261003/RESULTS.json`
   computes the exact steps (mpmath, 50 digits), residuals, budgets and the classification.

## Gate

**PASS** if all hold:

1. **Budget validity.** For every completed case and both drivers, `|err_X - err(X^*)| <= B_X` and
   `||y_new^c - y_new^*|| <=` its bound. A violation means the budget is wrong.
2. **Classification.** Every L-0038 item-3 failure among completed cases is explained:
   `|err_U - err_K| <= B_U + B_K + E_native_coefficient`. None is left unexplained.
3. **No unresolved acceptance.** Wherever a driver's step would be accepted (`err_X <= 1`), `err_X + B_X <= 1`.
   Wherever it would be rejected, `err_X - B_X > 1`. A case violating this is an unresolved decision, and its
   presence fails the gate (the drivers have no guard for it).
4. **Bound availability.** Every budget is finite. If `||W^-1||` or some `L_i` cannot be bounded (non-finite, or the
   interval Jacobian on the box is unbounded), the case is reported as unavailable and the gate fails (DAG stop
   condition). The boxes need no iteration: stage `i`'s box radius depends only on `d_j`, `j < i`.

Otherwise **FAIL**. Reported: per case, each budget term, the ratio `|err_U - err_K| / (B_U + B_K + E_coef)`, and
how much of `B_X` comes from the residuals vs the nonlinear propagation. No driver changes; dense output, clock and
fallback accounting are untouched (this node changes no driver). No speed or stiff-order claim. L-0038 stays FAIL.

## Prior information

L-0038's RESULTS.json (the failing comparisons and their sizes); L-0037's exact raw-target identity
(`r_U = gamma r_K`, coefficient terms 1e-16 to 1e-14). No code of this node exists before this commit.
