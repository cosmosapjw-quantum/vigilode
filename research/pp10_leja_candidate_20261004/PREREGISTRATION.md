# PP10 — Newton-Leja phi-action candidate with an EstimateOnly status (prospective registration)

RVJ DAG node `PP10`. Base commit: the commit that adds this file.

## Candidate backend (new, opt-in, `core/src/leja_action.rs`)

Real Leja points on a verified spectral interval `[a, b]` of a
`SymmetricNonpositiveOperator` (Gershgorin), with the scaling and
`phi_k` Newton divided differences computed in extended care (the
standard scaled recurrence of Caliari et al.); the action `p(hA) w_k` by
the Newton recurrence with `apply_rows` (charged as operator products); a
stopping estimate from the last two Newton terms. Status is always
`EstimateOnly`: no total witness is claimed (spectrum-only bounds are not
certificates, and a complete bound is not available).

## Comparison (fixed now)

The PP08 fixtures (n in {5, 16}, rho in {2, 30, 200}, h in {1e-3, 3e-2,
0.2}, distinct and same-vector inputs) at estimated tolerances 1e-8 and
1e-12: Leja vs Chebyshev `joint_phi_action` on the same target, recording
actual error against the 50-digit reference, operator vector products,
coefficient setups, and the Leja estimate.

## Gate

G1 Status is `EstimateOnly` in every report; no Leja result is admitted
   through any certified API (contract test).
G2 Accuracy of the candidate: actual error <= 10 x the requested
   estimate tolerance in at least 90 % of cases (a property of the
   candidate, not a certificate); every case where the estimate
   under-reports the actual error is listed.
Reported: products vs Chebyshev at matched actual error; no speed claim.

## Results (append only after the recorded run)

Recorded run at source commit `0925f34f770ec6ebcdc9135444cbdbb36a546715` (module `crates/rodas5p-core/src/leja_action.rs`,
contract tests, exporter and checker committed before any output existed). Commands, from the repository root:

`PP10_CASES=research/pp10_leja_candidate_20261004/cases.json cargo test --release -p rodas5p-core --locked --test pp10_leja_study -- --ignored --nocapture`
`PYTHONDONTWRITEBYTECODE=1 python3 tools/pp10_leja_check.py --cases research/pp10_leja_candidate_20261004/cases.json --output research/pp10_leja_candidate_20261004/RESULTS.json`

Outputs: `cases.json` (native export, IEEE-754 hex bits; sha256 `c82c5b11...35ca2e`), `RESULTS.json` (mpmath 1.3.0,
50 digits, the PP08 reference `exact_target` imported from `tools/pp08_laguerre_router_check.py`; sha256
`227c3032...312da25a`). Both commands exited 0; each ran once. Before the commit, `cargo fmt --all -- --check`,
`cargo clippy -p rodas5p-core --all-targets --locked -- -D warnings` and `cargo test --release -p rodas5p-core --locked`
exited 0.

Precondition: all 72 cases have `A`, `h` and `w_k` bit-identical to the PP08 main case with the same label
(`fixtures_match_pp08: true`).

**Verdict: PASS** (G1, G2 hold).

| Gate | Outcome |
|---|---|
| G1 | **holds**: all 72 Leja reports are `EstimateOnly` with reason `LEJA_TOTAL_NOT_CERTIFIED` (bounded components 0). Put into the case's certified Chebyshev report (the most admissible-looking carrier), the Leja result is rejected in all 72 cases at a budget of 1e300 by `admit_total_error` and `route_admission` (reason `LEJA_TOTAL_NOT_CERTIFIED`) and by `admit_laguerre_total` ("not a Laguerre report"). The contract test `no_certified_api_admits_a_leja_result` shows the same, also for a Laguerre-labelled carrier; `LejaReport` is a separate type that none of these APIs takes |
| G2 | **holds**: actual error <= 10 x tolerance in 72 of 72 cases (100 %); in fact <= tolerance in all 72. Largest actual error 9.7e-10 at tolerance 1e-8 and 1.6e-14 at 1e-12. The estimate under-reports the actual error in **0** cases: estimate / actual error 4.8 to 7.2e5 (median 507) |

Reported items (no gate).

- Degrees: Leja 4-26 at 1e-8 and 5-33 at 1e-12; Chebyshev 2-29 and 3-36. No Leja run reached the cap 128; no
  action error in either basis. One coefficient setup per case for both.
- Operator vector products, Leja / Chebyshev, at the same requested tolerance: ratio 0.83 to 2.0 (median 1.15) over
  all 72 cases; Leja fewer in 8 (all eight rho = 200, h = 0.2 cases: 125 vs 145 and 165 vs 180 distinct, 24-26 vs
  29 and 33 vs 36 same-vector), equal in 6 (rho = 30, h = 0.2 for n = 5, and n = 5, rho = 200, h = 0.03 at 1e-12),
  more in 58 (ratio 2.0 in the four rho = 2, h = 1e-3, 1e-8 cases: degree 4-5 vs 2). By tolerance: median 1.2 at
  1e-8, 1.125 at 1e-12.
- "Matched actual error" as fixed in the checker before the run: both actual errors are at most the requested
  tolerance, which holds in all 72 cases, so the ratios above are the matched comparison. The match is coarse: the
  actual errors themselves differ, Leja / Chebyshev error ratio 4.2e-6 to 650 (median 0.10); Leja is the less accurate
  in 12 cases. Both stopping rules overshoot the tolerance: the Leja last-two-terms estimate by a median factor 507,
  the Chebyshev truncation bound similarly (PP08). The products were not compared at equal actual error degree by
  degree (no per-degree Chebyshev ladder was run; the preregistration fixes one Chebyshev run at the same tolerance).
- Failures: none (no action errors, no cap reached, no case above the tolerance, no under-report).

Interpretation (within the claim ceiling): on the 72 PP08 fixtures the Newton-Leja candidate meets its requested
tolerance in every case and its estimate never under-reports, so it is accurate as an estimate-only method here, and
no certified path admits it. It needs more operator products than certified Chebyshev at the same tolerance in 58 of
72 cases and fewer only at the largest h rho (40); it offers no certificate in exchange. No speed claim, no default
dispatch change, and the estimate is not a bound (its looseness on these fixtures is not evidence that it bounds
elsewhere).

Implementation notes and disclosures (all fixed before the recorded run):

- Leja points: discrete Leja points of `[-2, 2]` on the cosine grid `2 cos(pi i / 2^15)` (first maximum on ties),
  starting at 2; mapped to the outward-rounded interval `[-(h rho), -(h lambda)]` of `hA` from the Gershgorin
  enclosure. A declared (unverified) enclosure is an error, not a run.
- Divided differences: not the Caliari et al. scaled recurrence named in the registration, but the standard
  matrix-function route (Opitz): `phi_k[x_0..x_j] = exp[0^k, x_0..x_j]`, read from the first five columns of the
  exponential of the lower bidiagonal matrix with nodes `(0,0,0,0,x_0,...,x_128)`, shifted by `c - 2 gamma` so that
  it is entrywise nonnegative, by a binary64 vector Taylor series with Neumaier compensated summation (no
  cancellation, no scaling and squaring, no extended precision). A development check against 500-digit mpmath
  divided differences (`c = -22, gamma = 9` and `c = -1e-3, gamma = 4.5e-4`, 129 nodes; hand values, not the fixtures)
  gave relative errors at most 1.1e-15.
- Stopping: after term `j >= 1`, estimate `sum_k |s_k| (|d_{j-1,k}| ||r_{j-1}|| + |d_{j,k}| ||r_j||)` against the
  absolute tolerance on the fused 2-norm; cap 128 (`converged = false` there). The scalar branch (`h = 0`, `A = 0`,
  `A = -lambda I`) uses one node and no products.
- Counting: each Newton step is one `apply_rows` on the block and is charged as one `poly_block_products` and the
  block width in `poly_vector_products`, as in `joint_phi_action`; one `poly_coefficient_setups` per call;
  `poly_block_allocations` is not charged by the Leja action (Chebyshev charges 3), which does not enter any
  compared quantity.
- The checker treats a missing carrier (a failed Chebyshev action) as "not tested" rather than a G1 failure; it did
  not occur.
