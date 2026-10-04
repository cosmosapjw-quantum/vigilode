# PP08 — opt-in Chebyshev/Laguerre total-budget router (prospective registration)

RVJ DAG node `PP08`. Base commit: the commit that adds this file. The
R-NEXT-04 campaign (L-0047) is not rerun; its six large-h-rho budget
rejections stay historical results.

## Router

`route_joint_phi(op, h, input, total_budget, work) -> RoutedPhi` for a
`SymmetricNonpositiveOperator` (verified Gershgorin domain only):

1. Chebyshev `joint_phi_action` with truncation budget `total_budget / 4`;
   admitted iff `admit_total_error(total_budget)` admits.
2. Laguerre `joint_phi_action` (default scales) with the same truncation
   budget; admitted iff `admit_laguerre_total(total_budget)` admits.
3. Choice: among admitted results the one with fewer operator vector
   products (`poly_vector_products`), ties to Chebyshev; if none is
   admitted, `Fallback` with both rejection reasons (the caller uses its
   protected path). Both attempts' work stays in the caller's counters.

The router never admits: a declared (unverified) enclosure, the unbounded
timing execution, a degree above the adjoint limit for Laguerre, an
estimate-only total. The report records each attempt's degree, scale,
bound, vector products and coefficient setups.

## Fixtures (new; not the R-NEXT-04 cases)

Symmetric `A = Q diag(lambda) Q^T` rounded and symmetrized, n in {5, 16},
spectra in `[-rho, -lambda_min]` with rho in {2, 30, 200} and h in
{1e-3, 3e-2, 0.2}; distinct and same-vector inputs; total budgets 1e-8 and
1e-12; scalar branch `A = -3 I`; declared enclosure; zero h.

## Gate

G1 Every admitted routed result's bound is at least the actual error
   against a 50-digit eigendecomposition reference.
G2 Negative and boundary cases: declared enclosure, unbounded execution and
   Laguerre degree above the limit are never admitted; an unmet budget ends
   in `Fallback`, never in a relaxed tolerance.
G3 Accounting: the caller's counters after routing equal the sum of the
   attempts' counters.
Reported: which basis was chosen per case, both bounds and costs; the
fraction routed to Laguerre; cases where only one basis admits.

## Results (append only after the recorded run)
