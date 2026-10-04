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

Recorded run at source commit `f97f3e5d792a886ef5ceb2ed595ec0e3103f5823` (router, contract tests, exporter and
checker committed before any output existed). Commands, from the repository root:

`PP08_CASES=research/pp08_laguerre_router_20261004/cases.json cargo test --release -p rodas5p-core --locked --test pp08_laguerre_router_study -- --ignored --nocapture`
`python3 tools/pp08_laguerre_router_check.py --cases research/pp08_laguerre_router_20261004/cases.json --output research/pp08_laguerre_router_20261004/RESULTS.json`

Outputs: `cases.json` (native export, IEEE-754 hex bits; sha256 `0034a41d...e83da`), `RESULTS.json` (mpmath 1.3.0,
50 digits; sha256 `281a42ab...0aec`). Both commands exited 0; each ran once.

**Verdict: PASS** (G1, G2, G3 hold).

| Gate | Outcome |
|---|---|
| G1 | **holds**: all 76 admitted routed results enclose the 50-digit target; bound / actual error 4.3 to 3.0e3 (median 40.7). Every admitted attempt of either basis (not only the chosen one) also encloses; the 43 admitted Laguerre attempts are 4.3x to 1.4e3x loose (median 37.6) |
| G2 | **holds**: both declared-enclosure cases (h = 0.03 and h = 0) end in `Fallback`; no attempt on an unverified enclosure is admitted; the timing execution (Chebyshev and Laguerre; recurrence, scalar `-3 I` and h = 0) is never admitted, even at a budget of 1e300; the guard case (n = 5, rho = 200, h = 0.25, the first h of the fixed list 0.25/0.35/0.5) has Laguerre degree 140 > 128, rejected, and routes to Chebyshev; no Laguerre attempt above degree 128 is admitted anywhere; the total budget 1e-300 ends in `Fallback` (Chebyshev: action error "Bessel series did not converge", Laguerre: degree 240 above the limit); no fallback carries an output; no admitted bound exceeds its total budget; NaN, -1 and +inf budgets are errors with no work charged |
| G3 | **holds**: in all 79 cases the caller's counters (starting at zero) equal the field-by-field sum of the two attempts' counter deltas |

Reported items. Fixtures: 72 main cases (n in {5, 16} x rho in {2, 30, 200} x h in {1e-3, 3e-2, 0.2} x distinct/same
x total budget in {1e-8, 1e-12}) plus 7 boundary cases (scalar `-3 I` at both budgets, two declared, zero h,
budget 1e-300, Laguerre degree above the limit). Choice: Chebyshev 76, Laguerre **0**, Fallback 3 (the two declared
cases and the 1e-300 budget). The fraction routed to Laguerre is 0 of 79 (0 of 72 main cases).

- Both bases admit in 43 cases (40 main: 32 of 36 at budget 1e-8, 8 of 36 at 1e-12; plus the scalar and zero-h
  cases). In all 43 the Laguerre attempt needs at least as many vector products as Chebyshev (ratio 1.0 to 1.57,
  median 1.25): 7 ties (3 of them with zero products: scalar branch and h = 0), which go to Chebyshev by the rule,
  and 36 where Laguerre costs more. So no case routes to Laguerre.
- Only Chebyshev admits in 33 cases: 32 main cases where the Laguerre adjoint total exceeds the budget, and the
  degree-guard case. At budget 1e-8 these are the four rho = 200, h = 0.2 cases (Laguerre degree 88, totals 0.024 to
  0.15 against actual Laguerre errors 6.9e-13 to 2.3e-11). At 1e-12 they are 28 of 36 cases, every case except
  h = 1e-3 with rho in {2, 30}: totals 5.4e-12 to 3.9e-11, and 5.5e5 to 1.9e6 for rho = 200, h = 0.2 (degree 115,
  actual errors 1.5e-14 to 4.8e-14), the same looseness at large h rho that R-NEXT-04 reported.
- Only Laguerre admits in 0 cases. Chebyshev is admitted in every verified case except the 1e-300 budget.
- Per-case degrees, scales, both bounds (or candidate totals), costs and coefficient setups are in `RESULTS.json`
  (`rows[*].attempts`) and `cases.json`. Chebyshev degree 2-37 and Laguerre degree 3-115 on the main grid; the
  Laguerre scale chosen is 16 in every main case with h rho >= 1 (nominal rho).

Interpretation (within the claim ceiling): on these verified symmetric fixtures the router is sound and its
accounting is exact, but it never selects Laguerre, since the Laguerre recurrence is never cheaper in vector products
than Chebyshev at the same truncation budget, and its admitted total is often looser than 1e-12. The opt-in router
therefore adds cost (both attempts are charged) without changing the chosen basis here. No speed claim; no default
dispatch change.

Implementation notes and disclosures (all before the recorded run):

- The router adds two guards on top of `admit_total_error` / `admit_laguerre_total` (`route_admission`): certified
  execution and a Gershgorin-verified enclosure. They are needed because `admit_total_error` alone admits a
  Chebyshev report at h = 0 under a declared enclosure (the scalar branch certifies `phi_k(0) w_k`, which does not
  depend on `A`) and a scalar-branch report from the timing execution (the scalar branch computes its enclosures in
  both executions). Contract tests show both facts and that the router rejects them.
- An action error in one basis (for example a degree or coefficient-series failure) is recorded as that attempt's
  rejection reason, not a router error; the router checks the budget, h and the input itself first. A total budget
  of 0 is valid and ends in `Fallback`.
- Fixture construction parameters not fixed by the text above were chosen before any router output was seen:
  spectrum `[-rho, -0.1 rho]` with both ends present, `Q` one sweep of adjacent Givens rotations with angles in
  `[-0.3, 0.3]` (seeded splitmix), `A` per (n, rho) shared across h, inputs and budgets, the operator from
  `SymmetricNonpositiveOperator::gershgorin`. The fraction 0.1 and angle 0.3 were picked with a standalone Python
  pre-check of Gershgorin verifiability only (a dense Haar `Q` puts the Gershgorin upper end above 0, outside the
  verified domain); no router, admission or error quantity was computed for it.
- During development the contract test for the degree guard first used h = 1 on a rho = 400 contract matrix; there
  the Laguerre action fails (its recurrence majorant leaves the binary64 range) instead of returning a degree above
  the limit. The test now uses h = 0.25 for the degree guard and keeps h = 1 as an action-error case. This informed
  the exporter's fixed search list h in {0.25, 0.35, 0.5} for the degree-guard case. The study exporter and the
  Python check ran only once, as recorded above.
- The degree-guard h (0.25) and the 1e-300 budget lie outside the main grid of the fixture paragraph; they are
  the boundary cases G2 requires.
