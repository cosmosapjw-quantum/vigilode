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

## Results

**Verdict: FAIL** (gate item 5, "niche empty": |N| = 0 < 6). Items 1-4 PASS. This is the registered prediction and
the DAG kill rule: no Laguerre routing benefit is claimed, no dispatch link is made, and no basis enters any default.

### Record

| Step | Commit | Content |
|---|---|---|
| A | `ec42c61e81bbcc5315c849a20a78790a1f0114c0` | plan/execute split (`polynomial_action.rs`), router and `RouteCost` (`polynomial_action/router.rs`), contract tests and exporter (`tests/polynomial_single_backend_router.rs`) |
| B | `a5d5f2711ee30ec59b747b71a23e114a27d6b547` | checker `tools/py03_single_backend_router_check.py` and its synthetic tests, committed before the run |
| C | `358e0db161bcf44e329c03ab3448b9c2f4a3214d` | the recorded run: `cases.json` (export at B, clean tree) and `RESULTS.json` |

| File | SHA-256 | Size |
|---|---|---|
| `cases.json` (compact JSON, not gzipped) | `8a8d620e3af427b41798d97cd55a903643a7ea8c657557bf995054d6ef4a1add` | 5,505,784 B |
| `RESULTS.json` | `a6cc95b45aee7c0e66f72ccf82a2e6f220df55aa16aa79b49a0482cfcbd711cc` | 96,105 B |
| `tools/py03_single_backend_router_check.py` (blob at B) | `85079194df38c5a9be88ac166b432d48de8f9e6932ee6ab4687be1d371b593cb` | |

The export ran once (`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, release, 4.7 s): `source_commit` = B, recorded
`git status --porcelain` empty. The checker ran at B (`checker_head` = B, `--untracked-files=no` status empty;
python 3.11.15, mpmath 1.3.0, 50 and 70 digits, evidence schema `vigilode-evidence-schema-v2`), exit 1 (FAIL). Every
validity rule held (no INVALID): 103 cases (96 grid, 5 boundary, 2 ForceLaguerreFirst), 624 records, the 50/70-digit
references agree within 1e-40 ||F||, every recorded cost equals the sum of its plan, preflight and execution costs.

### Gate

| Item | Result | Numbers |
|---|---|---|
| 1 Certification | PASS | 208 admitted `single`/`force_laguerre_first` results, every bound <= total budget and >= the 50-digit error (bound / error 13.3 - 191.8, median 25.9); no declared enclosure, timing execution, Laguerre degree > 128 or estimate-only total admitted in any record of any arm; the declared and zero-budget cases end in Fallback in every record |
| 2 Parity | PASS | 418 executions (router arms and the two timing executions) bitwise equal to `joint_phi_action` / `joint_phi_action_unbounded` on a cache in the same state (report, counters, fused bits, candidate total); wherever `single` selects Chebyshev its fused bits and bound equal `cheb`'s |
| 3 Accounting | PASS | 624 records: caller counters = preflight work + every execution; cost products, block products, setups and cache hits = the counters (total and per execution); each execution's products = its plan's prediction; both ForceLaguerreFirst cases charge the rejected Laguerre recurrence, then execute Chebyshev |
| 4 One execution | PASS | every admitted first execution is the only recurrence; `single` <= `pp08` products everywhere and < in all 202 (case, cache state) pairs where both PP08 attempts made products |
| 5 Niche benefit | **FAIL** | N = {} (no grid case's cold preflight preselects Laguerre): niche empty |

### Reported (not gated)

- **Preflight.** On all 96 grid cases the Laguerre plan needs at least as many vector products as the Chebyshev plan
  (Laguerre / Chebyshev 1.00 - 1.76, 11 ties): 202 abstentions `PRODUCTS_NOT_FEWER` (grid and verified boundary
  records of `single`), 2 `LAGUERRE_PLAN_FAILED` (zero budget), 2 router refusals `ENCLOSURE_NOT_VERIFIED` (declared).
  The Laguerre table was never built by the preflight.
- **Costs.** `single` = `cheb` in vector products, coefficient-series terms and certification operations in every
  grid case and cache state; its only overhead is the Laguerre plan's tail evaluations (18 - 482 per call on the
  grid; up to 3439 on the boundary cases). `single` / `pp08` vector products 0.36 - 0.50 on the grid (total 4580 /
  10942 = 0.42).
- **Admitted by one basis only (PP08 attempts):** 3 cases, all Chebyshev only (the degree-above-limit case and the
  two ForceLaguerreFirst cases). PP08 admitted both bases on all 96 grid cases.
- **ForceLaguerreFirst selection.** No grid case had a rejected Laguerre total of degree <= 128, so both cases came
  from the registered fallback: n = 8, rho = 64, h = 0.12, distinct input, total budget 1e-13 (the first of
  1e-13, 1e-15, 1e-17).
- **Degree above 128:** h = 1 (Laguerre plan degree 151); the forced Laguerre execution is rejected, Chebyshev admits.

### Prediction check

| Prediction | Outcome |
|---|---|
| Items 1-4 hold | held |
| Item 5: \|N\| = 0, FAIL "niche empty" | held (\|N\| = 0) |
| `single` = `cheb` in vector products on every grid case | held |
| Overhead = Laguerre plan tail evaluations, no Laguerre table | held |
| `single` / `pp08` about 0.25 - 0.50 | held (0.36 - 0.50) |

### Interpretations and deviations

Fixed in the checker docstring at B, before the run (items 1-15 there). In short:
1. The plan/execute code sits in `polynomial_action.rs`, the router in `router.rs`; the action core gained one trailing
   `staging` parameter (as PY01 added `capture`), and series terms and tail evaluations are counted by a thread-local
   tally in the leaf loops. No value changes; PP08's `route_joint_phi` and every default path are bitwise unchanged
   (all pre-existing tests pass, PY01's `report_parity_on_fresh_fixtures` included).
2. `plan_joint_phi` takes `&PolynomialCaches` (lookup only), `execute_joint_phi` `&mut PolynomialCaches` (tables and
   envelopes); `route_joint_phi_single` returns `CoreResult` (invalid inputs are errors before any work, as PP08).
3. `cheb` is the same router with `RoutePolicy::ChebyshevOnly`.
4. On a declared enclosure the router executes nothing (`route_admission` could not admit any report) and returns
   Fallback; this is a refusal, never an admission.
5. ForceLaguerreFirst runs as an extra arm `force_laguerre_first` (also on the degree-above-limit case), never as
   `single`; item 1 gates both, item 4 compares `single` only.
6. A non-registered, ignored format check (`export_fixture_format`, fresh n = 5 fixtures) was run before B to align
   the export's key sets with the checker; it found that `WorkCounters` serialization skips zero fields, so the
   export writes every counter field. No registered case was executed before the recorded run.

### Coordination

Merge order PY01 -> PY03 -> PY02 -> PY05. The PY03 change to the shared core is one trailing parameter of
`joint_phi_action_impl_captured` / `joint_phi_action_core` (callers pass `None`) and counting lines at the
coefficient build, the Chebyshev propagation loop and the adjoint site (which, under staging, takes the envelopes
from `LaguerreEnvelopeCache` through `EnvelopeKey::new` with the v1 proof version). The cache-key contract test
(`cache_keys_separate_what_the_entries_depend_on`) uses only hits and misses, not key encodings.
