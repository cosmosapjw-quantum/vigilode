# Preregistration: plan first, execute one polynomial backend (PY03)

Node PY03 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`REVIEW_KO.md` §5, `MATHEMATICS_PORTING_KO.md` §6.2). Registered on branch `audit/rvj-reaudit-remaining-20261011`
(base `0ce7c32`). It depends on AS03, the fail-closed evidence validator (merged).

**Claim boundary.**
- Counted work only: vector products, coefficient-series terms and certification operations. No wall-time claim.
- The router is opt-in and research-only. The PP08 `route_joint_phi` stays as it is, and nothing changes in any
  default dispatch.
- PP08's zero Laguerre selections (L-0071: Chebyshev 76, Laguerre 0, fallback 3) give no current Laguerre routing
  benefit. This node tests that benefit only on a declared niche, against Chebyshev alone, and does not assume it.
- A plan is not a certificate: every result is admitted only by `route_admission` on the executed report.

## Question

PP08's router executes both bases in full and charges both. Can it instead do the following, under the same joint phi
target `sum_k phi_k(hA) w_k`, the same absolute total budget and the same truncation fraction (1/4)?
1. Separate degree and coefficient planning from execution.
2. Execute one backend.
3. Run the other only when the executed result is not admitted.

Is the result certified, with every preflight, cache state and failed execution charged? And on the declared niche,
is its complete cost below Chebyshev alone?

## Change

In `crates/rodas5p-core/src/polynomial_action.rs` and `polynomial_action/router.rs`.

1. **Planning.** `plan_joint_phi(op, h, input, basis, truncation_budget, caches, cost) -> CoreResult<ActionPlan>` runs
   `choose_transform` (degree, Laguerre scale, beta, L'), counts the tail evaluations, and predicts the vector products
   (degree x block width).
   - The coefficient table is built, or read from `CoefficientCache`, only when the plan is executed or when the
     Laguerre preflight below needs it.
   - The plan is not a certificate.
2. **Execution.** `execute_joint_phi(plan, op, h, input, cache, work) -> CoreResult<JointPhiReport>`.
   - Parity: for the same arguments its report must be bitwise identical to `joint_phi_action` (fused, columns,
     components, totals, counters).
3. **Router.** `route_joint_phi_single(op, h, input, total_budget, policy, caches, work) -> SingleRoutedPhi`, with
   policy `ChebyshevFirstPreflight` (the default of this entry point).
   - **Plan** both bases.
   - **Preflight.** Laguerre is preselected only if all of the following hold; otherwise the preflight abstains to
     Chebyshev:
     - the Laguerre plan needs strictly fewer vector products (ties go to Chebyshev);
     - its degree is <= `LAGUERRE_ADJOINT_DEGREE_LIMIT`;
     - its vector-independent terms are <= total_budget. These are the truncation term plus the coefficient term
       built from the coefficient radii, which needs the Laguerre table (built and charged).
   - **Execute** the preselected backend and admit it with `route_admission(report, total_budget)`.
   - **On rejection,** execute the other backend, charged, and admit it. If neither is admitted, the result is
     `Fallback` with both reasons.
   - `RoutePolicy::ForceLaguerreFirst` exists only for contract tests of a failed preflight.
4. **Cost record.** `RouteCost` is router-local; `WorkCounters` is not extended. It counts:
   - plan tail evaluations;
   - coefficient setups, cache hits and series terms;
   - executed backends and failed executions;
   - vector and block products;
   - certification operations: Chebyshev propagation-loop terms, Laguerre envelope interval operations (through the
     counted envelope path) and adjoint-sum terms.
5. **Caches.** In the cold state every case starts with empty `CoefficientCache` and `LaguerreEnvelopeCache`. In the
   warm state the same call is repeated with the caches left by the cold call. Chebyshev alone uses the same cache
   state.

## Cases (fresh; disjoint from PP08, R-NEXT-04, PY02)

**Grid** (96 cases):
- n in {8, 24};
- two operator families:
  - **diagonal**, with spectrum evenly spaced in [-rho, 0] including both ends; Gershgorin verification is then
    exact and lambda = 0, the end least unfavourable to Laguerre, since Laguerre ignores lambda;
  - **rotated**, PP08-style: one sweep of seeded adjacent Givens rotations with angles in [-0.3, 0.3], spectrum
    [-rho, -0.1 rho] with both ends present, seeds starting at 7001;
- rho in {4, 64};
- h in {0.005, 0.05, 0.12}, so h rho in {0.02, 0.2, 0.48, 0.32, 3.2, 7.68}: the range where PP08's Laguerre totals
  admitted;
- distinct and same-vector inputs (scales [1, -0.5, 0.25, 2, -1]);
- total budget in {1e-6, 1e-10}.

**Boundary cases:**
- a declared enclosure;
- the unbounded-timing execution;
- Laguerre degree > 128;
- total budget 0;
- the scalar branch (-3I);
- one `ForceLaguerreFirst` case per operator family whose Laguerre total is rejected (a failed preflight, charged).

**Arms per case and cache state:** `single` (the new router), `cheb` (Chebyshev alone) and `pp08` (`route_joint_phi`,
reported only).

**Declared niche N:** the grid cases (boundary cases excluded) in which the cold preflight of `single` preselects
Laguerre. N is fixed by the preflight before execution, and it is recorded.

## Commands

    cargo test --offline --locked -p rodas5p-core --test polynomial_single_backend_router
    PY03_CASES=research/py03_single_backend_router_20261011/cases.json cargo test --offline --locked --release -p rodas5p-core --test polynomial_single_backend_router -- --ignored --nocapture --test-threads=1 export_cases
    python3 tools/py03_single_backend_router_check.py --cases research/py03_single_backend_router_20261011/cases.json --output research/py03_single_backend_router_20261011/RESULTS.json

**Contract tests.** The first command is the DAG command. Its contract tests cover plan/execute parity, the charged
failed preflight, cache keys, and the boundary refusals.

**Checker.** The checker calls `tools/evidence_schema_v2.py` first (INVALID exit 2, FAIL exit 1, PASS exit 0). It
computes the reference with mpmath 1.3.0: a 50-digit eigendecomposition from the exact binary inputs, as in PP08,
checked at 70 digits. The checker is committed before the recorded run, and the export runs once.

## Gate

**Validity.** INVALID if any of the following holds:
- The AS03 rules fail.
- The case or arm set is incomplete, or a (case, arm, cache state) record is duplicated.
- The 50- and 70-digit references differ by more than 1e-40 ||F||.
- The export is internally inconsistent in its arithmetic: a cost total it records does not equal the field-by-field
  sum of the plan and execution records it records for the same (case, arm, cache state). This covers only the
  export's own arithmetic. Whether `RouteCost` matches the caller's `WorkCounters` is gate item 3, a FAIL, never
  INVALID.
- The checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.
1. **Certification.** Every admitted `single` result has bound <= total_budget and bound >= its 50-digit error. None of
   the following is ever admitted: a declared enclosure, the timing execution, a Laguerre degree > 128, an
   estimate-only total. An unmet budget ends in `Fallback`.
2. **Parity.** `execute_joint_phi` equals `joint_phi_action` bitwise wherever a backend executes. Wherever `single`
   selects Chebyshev, its fused bits and bound equal `cheb`'s.
3. **Accounting.** In every record, `RouteCost` and the caller's `WorkCounters` equal the plan work plus every
   execution. In the `ForceLaguerreFirst` cases the failed Laguerre execution is charged and Chebyshev then executes.
4. **One execution.** Wherever the first executed backend is admitted, exactly one recurrence runs. `single` never
   needs more vector products than `pp08`, and needs strictly fewer wherever `pp08` executed both bases with nonzero
   products.
5. **Niche benefit (the DAG gate).** |N| >= 6, a design choice. In every case of N, under both cold and warm caches:
   - the selected Laguerre result is admitted;
   - its cost is below `cheb` in this sense: vector products strictly lower, and coefficient-series terms and
     certification operations not higher.

Everything else is **FAIL**, with every number preserved. If |N| < 6, item 5 fails as "niche empty", and the node is
FAIL.

**Reported, not gated.**
- The cost ratios `single` / `cheb` and `single` / `pp08` per case and cache state, and the plan overhead (tail
  evaluations; the Laguerre table when the preflight builds it).
- The cases admitted by one basis only.
- The preflight abstentions and their reasons.

## Kill and hold

- **No Laguerre benefit is claimed** from zero selections. No dispatch link is made, and no basis enters any default.
- **No re-scoring.** A FAIL of item 5 is not re-scored by widening the grid or the niche after the run.

## Coordination with sibling nodes

PY01, PY02, PY03 and PY05 all edit `crates/rodas5p-core/src/polynomial_action.rs`. Their changes are merged in the
fixed order PY01 -> PY03 -> PY02 -> PY05. PY05 stays out of the shared router of this node. PY02's v2 envelope key
must not change any v1 digest, and this node's cache-key contract tests must pass unchanged after PY02 is merged.

## Prior information (disclosed)

- **PP08 data.** In all 79 PP08 fixtures the Laguerre degree is >= the Chebyshev degree. Where both admit, the
  vector-product ratio Laguerre / Chebyshev is 1.0-1.57; there are 7 ties.
- **Plan scan (before registration).** A float emulation of the two degree rules found no point where the Laguerre
  degree is below the Chebyshev degree. Rules: Chebyshev tail with t = asinh(r/b) and eta = 0; Laguerre tail
  e^{L/2} q^{m+1} over the default scales. Grid: h rho in [1e-4, 1e3], lambda / rho in {0, 0.1, 0.5, 0.9, 0.99}, tail
  targets 1e-4 to 1e-14. It chose the grid's emphasis (lambda = 0, moderate h rho) and did not use native code.
- **Nothing run.** No code of this node exists, and no fixture of this grid has been run.

## Predictions

- Items 1-4 hold.
- **Item 5:** |N| = 0, so the node FAILS on item 5 ("niche empty"). This is consistent with the DAG kill rule.
- **Costs.**
  - `single` = `cheb` in vector products in every grid case.
  - Overhead: the Laguerre plan's tail evaluations, and no Laguerre table, since the preflight stops at the product
    comparison.
  - Against `pp08`: about 0.25-0.50x the vector products. This is Chebyshev / (Chebyshev + Laguerre), with the
    Laguerre / Chebyshev product ratio between 1.0 and about 3 (PP08: 1.0-1.57 where both admit, up to 2.9 at
    h rho = 40).
