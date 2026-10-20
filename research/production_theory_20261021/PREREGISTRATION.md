# Preregistration: exact verification of the production plan's numerical claims (node `production_theory_20261021`)

Written and committed before `verify_theory.py` is run. Results are appended below the line `## Results` after the run.

## Question

Do the numerical and analytic claims of `docs/plan/PRODUCTION_PLAN_20261020.md` section 5 (T1 "Numbers for RODAS5P",
T2 Corollary 2a, T3 Lemma 3, T9 example, and the det(W)^s statement used in T5) hold when recomputed from the
repository coefficient snapshot?

## Command

    python3 research/production_theory_20261021/verify_theory.py --out research/production_theory_20261021/RESULTS.json

Python 3, sympy (exact rationals), mpmath (50 digits), numpy (seeded, `seed = 20261021`). No cargo.

## Inputs

`fixtures/rodas5p_coefficients_snapshot.json` keys `gamma`, `A`, `C`, `c`, `b_code`; every decimal string is converted
to the exact rational of that literal (`sympy.Rational(str)`).

## Definitions fixed in advance

U-form stage recursion `W U_i = h*gamma*f(y + sum_{j<i} A_ij U_j) + gamma*sum_{j<i} C_ij U_j + r_i`. For the scalar test
equation, z = h*lambda, w = 1/(1 - gamma z), the (i,j) block of the iteration matrix is
`T_ij = gamma*C_ij*w + A_ij*(w - 1)` (strictly lower, so `(I-T)^-1 = sum_{k=0}^{7} T^k`, built symbolically in w).
Output sensitivity to a residual on stage j: `m_j(w)`, with `m = b_code^T (I-T)^-1 w`. The preconditioned-residual
sensitivity is `p = b_code^T (I-T)^-1` (no trailing w).

## Claims and pass gates (each claim is judged PASS or FAIL separately)

1. **T1 numbers.**
   (a) `|m_j(1)|` rounds to (0.577, 0.237, 6.246, 5.966, 3.729, 4.568, 1.011, 1.000) at three decimals, sum 23.335 (sum within 1e-3).
   (b) The coefficient-sum bound `sum_k |c_jk|` over the coefficients of the polynomial `m_j(w)` rounds to
   (100.456, 15.401, 24.467, 18.847, 13.655, 14.583, 1.011, 1.000), sum 189.418 (sum within 1e-3).
   (c) `sum_j |m_j|` at z = -1, -10, -100 is 17.819, 3.825, 0.125 (each within 1e-3).
   Ungated extra z values (-0.1, -3, -1000) are reported.
2. **Preconditioned-residual sensitivity.** On z = -10^t, t on 400 equispaced points in [-4, 8]: max over grid and j of
   `|p_j|` <= 6.25 and `sum_j |p_j|` in [1, 23.34] on the whole grid. The true max and min found are reported, with the
   signed sum reported beside the absolute sum. Primary reading of "entry" and "sum" is absolute value; the signed
   reading is reported but not gating.
3. **Lemma 3.** With x = -z/2 > 0, `R = x / ((1 + 2 gamma x)(1 - e^-x))`. Gate: R'(x) > 0 on a 10^4-point log grid
   x in [1e-8, 1e4] evaluated in mpmath at 50 digits, `R(x)` within [1, 1/(2 gamma)] on the grid, and the limits
   R -> 1 (x -> 0) and R -> 1/(2 gamma) (x -> inf) by sympy. The report states that the derivative-sign check is a grid
   check in 50-digit arithmetic and not a symbolic proof, unless a symbolic proof is obtained.
4. **Corollary 2a.** P = 1 - h_old gamma lambda, W = 1 - h gamma lambda, lambda <= 0: `|1 - W/P| <= 2|1 - h/h_old|`,
   checked in exact rationals on a grid of ratios h/h_old and values of h_old*|lambda|, and symbolically. The plan's
   identity `T = (1 - h/h_old)(P^-1 - I)` is also checked against `I - P^-1 W`; a sign discrepancy is recorded as a
   remark and does not by itself fail the claim, since the gate is the norm bound. A random symmetric negative semidefinite
   3x3 J0 in the 2-norm is reported as an additional, ungated check.
5. **T9 example.** a = (4096, 1, 1, 1), q = 1/2, B = 1/16, unit costs and unit initial residuals: smallest common k with
   `4099 q^k <= B` is 17 and the total is 68; `(18,6,6,6)` gives exactly `4096*2^-18 + 3*2^-6 = 1/16` with total 36.
   Exact `Fraction` arithmetic. An exhaustive integer minimum of `sum k_i` under the constraint is reported (ungated).
6. **Stage-homotopy determinant.** Random (seeded) 3x3 J0, h = 1/10, a nonlinear remainder N in sympy, the snapshot A and C:
   the full 24x24 Jacobian `dH/dU` of `H(U,lambda;theta) = [D - eta B]U - b - lambda h gamma R(U)` at random points for
   `(lambda, theta)` in {(1,*), (1/2, 0), (1/2, 1/2), (0, 1)} has all diagonal blocks equal to W, is block lower triangular,
   and `det = det(W)^8` to relative error < 1e-30 in 50-digit arithmetic. For N = 0 the identity is checked in exact rationals.

## Kill / pass rule

Each claim: PASS if every gated item holds, else FAIL with the observed value. The ledger row gets verdict PASS only if all
six claims PASS; if any claim FAILs, the ledger verdict is FAIL (a preserved negative result) and the claim sentence lists
the failing claims. The plan document is not edited by this node; corrections go in the node report
`docs/plan/nodes/t-verify.md`.

## Holdout

None: the claims are deterministic identities and exact computations. The extra z values in claim 1 and the symmetric
negative semidefinite check in claim 4 are the only out-of-claim probes, and are not gated.

## Results

Run on 2026-10-21 (a dry run to a scratch path preceded the recorded run; it changed nothing but a dead line in the script).
All six claims met their pre-registered gates. Raw numbers: `RESULTS.json`; per-claim values and remarks:
`docs/plan/nodes/t-verify.md`. Ledger verdict: PASS.
