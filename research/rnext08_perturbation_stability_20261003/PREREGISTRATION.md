# Preregistration: uniform perturbation stability of a no-chart RVJ step (remaining-only DAG node R-NEXT-08)

## Question

L-0040 reproduced the RVJ5 counterexamples (the semilinear two-step no-go and embedded blindness) and found that
RODAS5P, EXPRB43 and PEXPRB54S4 share neither. A generic no-chart replacement needs more than Taylor order and
scalar L-stability: the one-step map `Psi_h` must pass small perturbations on with a stiffness-independent constant.
Does RVJ5's step map do so, and does RODAS5P's? We compute both on (a) linear, constant-coefficient two-mode
non-normal systems and (b) a semilinear two-mode non-normal system, with stiffness swept over ten decades.

## Methods (fixed now)

- **RVJ5:** `reference_rvj.AnalyticRVJ(order 5)` from the vendored L-0040 scripts. It uses exact symbolic jets up to
  order 5, the rational phi functions of the (2,3) Pade approximant, and one solve with a cubic matrix polynomial.
- **RODAS5P:** the K form with the stored coefficients (`fixtures/rodas5p_coefficients_snapshot.json`) and the exact
  Jacobian, in 60-digit mpmath: 8 sequential solves with `I - h gamma J`.
- **Exact flow:** in closed form for both systems (below).

## Systems (fixed now)

(a) `y' = A y`, `A = [[-k1, c], [0, -k2]]`, with `k1 = K`, `k2 = 10 K`, `c in {0, K, 100 K}`, and K in
`{1e2, 1e4, ..., 1e12}`, h = 1/16. One step is `R(hA)` for both methods.

(b) `x' = x^2`, `y' = A y + 2 x y + x^2 e`, with `e = (1, 1)`, `A` as in (a) with `c = K`, h = 1/16, `x0 = 1/2`, and
`y0` on the slow manifold perturbed by `1e-3 * e`. The exact flow is
`x(t) = x0 / (1 - x0 t)`, `y(t) = (1 - x0 t)^-2 [e^{At} y0 + x0^2 A^-1 (e^{At} - I) e]`.

## Measurements

- (a) `||R(hA)||_2` for both methods, against the Crouzeix-Palencia bound `(1 + sqrt 2) sup_{Re z <= 0} |R(z)|`.
  `A` has its numerical range in the left half plane whenever `c^2 < 4 k1 k2`. We check that condition per case,
  and only those cases are gated against the bound.
- (b) the one-step error `E(K) = ||Psi_h(x0, y0) - phi_h(x0, y0)||`, and the step-map Jacobian deviation
  `D(K) = ||d Psi_h - d phi_h||_2` at `(x0, y0)`. The Jacobian is formed by central differences at `delta = 1e-20` in
  60-digit arithmetic.
- The derivative/solve DAG: per step, the derivative order needed (RVJ5: jets to order 5; RODAS5P: J only), the
  linear solves and their dependency depth (RVJ5: 1 solve with a degree-3 matrix polynomial; RODAS5P: 8 sequential
  solves).

## Commands

`python3 tools/rnext08_perturbation_stability.py --output research/rnext08_perturbation_stability_20261003/RESULTS.json`

## Gate

**PASS** if all hold:

1. **Linear limited-domain result.** For both methods and every gated case of (a), `||R(hA)||_2 <= (1 + sqrt 2)` (both
   stability functions satisfy `|R| <= 1` on the closed left half plane, which is also checked numerically on a grid).
2. **Semilinear measurement complete.** `E(K)` and `D(K)` are finite for both methods at every K, and each method
   gets a label. **Uniformly stable** if `max_K D(K) <= 10 min_K D(K)` and `max_K E(K) <= 10 min_K E(K)`. **Not
   uniform** if `max_K D(K) > 100 min_K D(K)` or the same for `E`. Otherwise **inconclusive**.
3. **Labels kept apart.** Every number carries its method label, and nothing is inferred from one method to
   another. A changed map or chart would be stated explicitly; none is used here.

Otherwise **FAIL**. The labels are measurements, and either answer is recorded. Stop condition (DAG): no generic
stiff-uniform order-5 promotion of RVJ5 without uniform error stability. If RVJ5 is labelled "not uniform", the
no-chart RVJ path stops at this negative result.

## Prior information

L-0040 (RVJ5 two-step no-go: `y` error over `h^3` tends to 4/3 as `kappa = h^-6` grows; RODAS5P does not share it).
The review cites Roberts-Shirokoff-Biswas-Seibold (arXiv:2505.15099v4) for semilinear RK stiff-order analysis. That
theorem is not applied here, and this node does not check the citation. No code of this node exists before this
commit.

---

## Results (appended after the run at `28863c2`)

Output: `RESULTS.json`. Ledger row L-0051.

**Gate: FAIL** (items 2 and 3 hold; item 1 fails on its scalar tolerance).

| Gate item | Outcome |
|---|---|
| 1. Linear limited-domain result | **fails as written**. Every gated matrix case is far inside the bound: `||R(hA)||_2` is at most 0.111 for RODAS5P and 0.037 for RVJ5, against `1 + sqrt 2 = 2.414` (c = 0 and c = K, K from 1e2 to 1e12). The scalar check required `max |R| <= 1 + 1e-30` on the left-half-plane grid. RVJ5's exact rational `R5` meets it, but RODAS5P with its *stored binary64* coefficients has `|R(iy)| = 1 + 1.3e-19` at y = 0.01 (1.3e-21 at y = 0.001; post-hoc check). The rounded coefficients satisfy the order conditions only to about 1e-16, so A-stability holds up to that rounding. The tolerance was too strict for a binary64 tableau, and the item fails as written |
| 2. Semilinear measurement complete | **holds**: `E(K)` and `D(K)` are finite for both methods at all six K |
| 3. Labels kept apart | **holds** |

Semilinear two-mode system (h = 1/16, `x0 = 1/2`, `y0` = quasi-steady state + 1e-3 `e`, c = K):

| K | RODAS5P E | RODAS5P D | RVJ5 E | RVJ5 D |
|---|---|---|---|---|
| 1e2 | 1.1e-4 | 0.11 | 1.1e-3 | 1.1 |
| 1e4 | 2.1e-5 | 1.9e-2 | 12.8 | 1.3e4 |
| 1e6 | 2.2e-7 | 2.0e-4 | 1.3e5 | 1.3e8 |
| 1e8 | 2.2e-9 | 2.0e-6 | 1.3e9 | 1.3e12 |
| 1e10 | 4.5e-11 | 2.0e-8 | 1.3e13 | 1.3e16 |
| 1e12 | 3.9e-11 | 5.6e-10 | 1.3e17 | 1.3e20 |

**RVJ5: not uniform, a negative result.** Its one-step error and step-map derivative deviation grow like `K^2` once
the initial state carries a fast component of 1e-3. The mechanism matches the method's structure. Its sources
`s_j = jet_j - J jet_(j-1)` (j up to 5) carry `K^(j-2)` times the fast amplitude through the nonlinear terms, while
`Q(Z)^-1 N_j(Z)` damps them only like `1/(hK)` (the `N_j` have degree 2, `Q` degree 3), so the `s_5` term grows like
`K^2`. Under the DAG stop condition, the no-chart RVJ5 path stops here: no generic stiff-uniform promotion.

**RODAS5P:** error and derivative deviation *decrease* with K, from their K = 1e2 values to 3.9e-11 and 5.6e-10. The
preregistered label rule is two-sided (`max > 100 min` gives "not uniform"), so it labels RODAS5P "not uniform"
too, because the values fall by more than 100x. That label is recorded as preregistered. The data show a bound
uniform in K, set by the least stiff case, which is what the question was about. The two-sided rule was a design
error.

Derivative/solve DAG per step: RODAS5P needs `J` only and 8 sequential solves (dependency depth 8); RVJ5 needs
exact jets to order 5 and one solve with a degree-3 matrix polynomial (depth 1).

Disclosure: the first invocation stopped before writing anything (`lu_solve` with a matrix right-hand side), and
the fix is the ledger commit. The imaginary-axis values above are a post-hoc check with the same functions. The
arXiv citation of the review was not checked. Claim ceiling: one semilinear two-mode family at one step size; a
negative result for RVJ5 and a measurement for RODAS5P, not a theorem for either.
