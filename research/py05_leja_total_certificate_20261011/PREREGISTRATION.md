# Preregistration: Leja phi actions from EstimateOnly to a total certificate (PY05)

Node PY05 (P3, kind `math_research`) of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the
October 10 re-audit (`MATHEMATICS_PORTING_KO.md`, section 6.4). Registered on branch
`audit/rvj-reaudit-remaining-20261011` at base `0ce7c32`. It depends on AS03 (merged). It follows PP10 (L-0080).

**Claim boundary.**
- **Keep EstimateOnly unless every gate holds** (DAG kill rule). `leja_action.rs`'s `LejaReport` and its status
  `LEJA_TOTAL_NOT_CERTIFIED` stay unchanged. A certified Leja path, if one passes, is a separate opt-in API: no
  router, integrator or default dispatch calls it.
- The 72 PP10 fixture successes grant no authority, and neither does a last-two-term estimate. A certificate needs a
  backward error, a forward conversion and rounding, each with a proof and an independent oracle.
- Domain: dense `SymmetricNonpositiveOperator` with a *verified* (Gershgorin) enclosure, the PP08/PP10 domain. A
  declared enclosure or a nonnormal operator stays EstimateOnly.
- Counted cost and certified error only. No wall time.

## Question

Can the fused target F = sum_{k=0}^{4} phi_k(h A) w_k, computed by Newton-Leja interpolation, carry a total error
certificate that encloses the independent 50-digit error? And with every cost charged (products, coefficient setup,
certification, failed attempts), does it beat the strongest certified rival, Chebyshev's `TotalErrorStatus::Certified`,
in a declared real-spectrum expensive-JVP niche?

## Stage 0: feasibility (mathematics first; each item is a written proof plus an oracle check)

`docs/reviews/20261011_py05/LEJA_TOTAL_BOUND.md` (new) must contain items T1-T5.

**T1. Truncation, forward.** Spec(hA) lies in [a, b], a subset of (-inf, 0], and A is symmetric. Then

||(phi_k - p_{k,m})(hA)||_2 <= max_{x in [a,b]} |phi_k(x) - p_{k,m}(x)|
<= gamma^{m+1} max_{xi in [-2,2]} prod_j |xi - xi_j| / (m+1+k)!.

The second inequality uses |phi_k^{(j)}(x)| <= j!/(j+k)! for x <= 0. The node-polynomial maximum is enclosed by
outward interval evaluation on subintervals.

**T1'. Caliari et al. 2016 route (optional).** Their backward-error selection of the scaling s and degree m may be
used only with a proven forward conversion for the fused phi target. Their analysis is for exp in exact arithmetic,
and an augmented-matrix route is nonnormal. Without that conversion, T1 alone decides.

**T2. Divided differences.** The Opitz series of `leja_action.rs` has nonnegative terms after the shift s = c -
2 gamma. Directed upper and lower sums, a geometric tail bound and `directed::exp_interval` (L-0065 floor) give an
enclosure for every d_{j,k}.

**T3. Recurrence rounding.**
- The local residuals of r_{j+1} = ((hA - cI)/gamma - xi_j I) r_j are enclosed outward.
- They are propagated by the maximum, over [a, b], of the remaining Newton tail polynomials. This is the analogue of
  the Laguerre output-adjoint recurrence bound (L-0039); the tail maxima are enclosed by interval evaluation.

**T4. Summation and input conversion.** The fused summation error is enclosed as in `polynomial_action.rs`. The
`weight_phi_vectors` transform must report `Exact`, or its loss must be enclosed.

**T5. Stopping.** The stop rule uses the certified total of T1-T4, never the last-two-term estimate.

**Oracle.** Exact-rational checks of T1 and T2 on n <= 5. 50-digit `mpmath` checks on 60 fresh fixtures (seeds in
`stage0_cases.json`; none of the 72/79 PP08/PP10 fixtures, and none of the stage-1 cells). There must be 0
under-enclosures.

**Stage 0 record.** `tools/py05_stage0_check.py` (command below) writes a numeric `STAGE0.json`. It lists T1-T5, each
as `{proved, oracle_violations, reviewer}`: `proved` is whether the written proof is complete, `oracle_violations` the
count of under-enclosures in that item's oracle checks, and `reviewer` the independent reviewer of the proof. T1' is
listed in the same form only if it is used.

**Stage 0 outcome.**
- **FEASIBLE** iff every entry has `proved = true` and `oracle_violations = 0`.
- **HOLD(proof: Tk)** iff any entry has `proved = false` or `oracle_violations > 0`. Under HOLD, Leja stays
  EstimateOnly, and that is the registered result.
- **FAIL** iff any of the entries T1-T5 is missing from `STAGE0.json`.
- **Ledger.** FEASIBLE and HOLD are recorded as verdict PASS with claim "decision FEASIBLE" or "decision HOLD", covering
  `STAGE0.json`; a missing entry is recorded as FAIL.
- `STAGE0.json` is INVALID if the checker blob at its source commit differs from the checker that was run, or if any
  recorded tree status is dirty.

## Stage 1: measurement (only if FEASIBLE)

**Fixtures (fresh, fixed in `stage1_cases.json` before the run; disjoint from `stage0_cases.json`).**
- Verified symmetric nonpositive dense A, n in {8, 32}.
- h rho in {10, 20, 40, 80, 160}, with lambda = rho / 100.
- Inputs: distinct w_k and same-vector.
- Budgets: 1e-8 and 1e-12.
- That gives 2 x 5 x 2 x 2 = 40 cells.
- **Niche:** h rho >= 40, the only regime where PP10 Leja used fewer products (0.83-0.92x).

**Arms.**
- `leja-cert` (gated).
- `cheb-cert`: certified Chebyshev `joint_phi_action` under the same total budget. This is the strongest certified
  rival.
- `leja-est` (PP10) and the PP08 router: reported.

**Complete cost.** P x c_JVP + Ir(coefficient setup) + Ir(certification) + Ir(failed attempts).
- P is the number of vector products.
- c_JVP is the declared expensive-JVP cost, 100 n Ir per product (design choice for the niche).
- The setup and certification Ir are same-binary callgrind 2-minus-1. Cold and warm coefficient caches are both
  charged and reported.

## Commands

    cargo test --offline --locked -p rodas5p-core --test leja_total_contract
    python3 tools/py05_export_cases.py --stage0-output research/py05_leja_total_certificate_20261011/stage0_cases.json --stage1-output research/py05_leja_total_certificate_20261011/stage1_cases.json
    # stage 0
    PYTHONDONTWRITEBYTECODE=1 python3 tools/py05_stage0_check.py --proof docs/reviews/20261011_py05/LEJA_TOTAL_BOUND.md --cases research/py05_leja_total_certificate_20261011/stage0_cases.json --output research/py05_leja_total_certificate_20261011/STAGE0.json
    # stage 1 only if STAGE0.json is FEASIBLE:
    PY05_CASES=research/py05_leja_total_certificate_20261011/stage1_cases.json PY05_RUNS=research/py05_leja_total_certificate_20261011/RUNS.json cargo test --offline --locked --release -p rodas5p-core --test leja_total_holdout -- --ignored --nocapture
    python3 tools/py05_leja_profile.py --output research/py05_leja_total_certificate_20261011/PROFILE.json
    PYTHONDONTWRITEBYTECODE=1 python3 tools/py05_leja_total_check.py --cases research/py05_leja_total_certificate_20261011/stage1_cases.json --runs research/py05_leja_total_certificate_20261011/RUNS.json --profile research/py05_leja_total_certificate_20261011/PROFILE.json --output research/py05_leja_total_certificate_20261011/RESULTS.json

`stage0_cases.json` holds the 60 stage-0 oracle fixtures; `stage1_cases.json` holds the 40 stage-1 cells. Both
checkers call `evidence_schema_v2` first (new kind `py05`) and compute the 50-digit reference independently, as PP10
did. INVALID exits 2, FAIL exits 1. Each checker and its case file are committed before the recorded run it reads.

## Gate (stage 1)

**Validity.** INVALID if any of the following holds:
- the AS03 rules fail, or a record does not match the `py05` schema;
- the (cell, arm, cache state) row set differs from the 40 registered cells of `stage1_cases.json` times the arms and
  cache states above, or a row is duplicated;
- a recomputed quantity differs from the exported one: the 50-digit error, a complete cost from its recorded
  components, or a profiled run from its RUNS record;
- the checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.

1. **Enclosure.** On all 40 cells, `leja-cert`'s total bound is at least the 50-digit actual error: 0 violations.
   Every admitted result is within its budget.
2. **No false authority.**
   - `leja-est` reports are still rejected by every certified API.
   - A declared enclosure and a nonsymmetric A are refused by `leja-cert`.
3. **Complete cost wins.** On the niche cells (h rho >= 40), the ratio `leja-cert` / `cheb-cert` must satisfy:
   - median complete cost <= 0.95 (design-choice margin under "wins");
   - no cell > 1.10;
   - ratio = infinity if `leja-cert` does not admit.

**Outcomes.**
- Item 1 or 2 fails: FAIL, and EstimateOnly is kept.
- Item 3 fails: FAIL, and EstimateOnly is kept for dispatch purposes. The certified API stays research-only, unrouted.
- No threshold is revised after the run.

## Coordination with sibling nodes

PY01, PY02, PY03 and PY05 all edit `crates/rodas5p-core/src/polynomial_action.rs`. Their changes are merged in the
fixed order PY01 -> PY03 -> PY02 -> PY05. PY05 stays out of PY03's shared router: the certified Leja path is a
separate API and is not wired into `route_joint_phi_single` or `route_joint_phi`. PY02's v2 envelope key must not
change any v1 digest or PY03's cache-key tests; PY05 adds no key to those caches.

## Prior information (disclosed)

- **PP10 (L-0080), on 72 PP08 fixtures.**
  - Products ratio Leja/Chebyshev: 0.83-2.0, median 1.15.
  - Leja used fewer products only in the 8 rho = 200, h = 0.2 cells (0.83-0.92).
  - Estimate/actual: 4.8e5-7.2e5. Actual error was within the tolerance in 72/72.
- **Certified Chebyshev's machinery** (residual propagation by (k+1) e^{k eta}) is cheap relative to an adjoint-type
  tail bound. The Laguerre adjoint bound was 7.35x to 7.6e13x tighter than the R4 majorant (L-0039), but needed
  per-column envelopes (L-0042: setup 2.3e4 to 7.5e6 Ir).
- No code of this node exists.

## Predictions

- **Stage 0.** T1, T2, T4 and T5 are feasible. T3 is the risk: it is likely provable, but loose.
  - Probability FEASIBLE: about 60%.
  - Otherwise HOLD(proof: T3 or T1'), and EstimateOnly stays.
- **Stage 1.**
  - Items 1 and 2 are expected to hold.
  - Item 3 is expected to FAIL. Even uncertified Leja saved only 8-17% of products in the niche. Certification adds
    node-polynomial and tail-maximum work, and T1's factorial bound needs a higher degree than the PP10 estimate stop.
- **Overall: Leja stays EstimateOnly**, through HOLD or a FAIL on item 3 (about 85%).

## Results

Appended after the recorded run. Nothing above this heading changes.
