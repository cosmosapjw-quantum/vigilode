# Delta evidence and measurement audit — 2026-10-10

Source: `cfed140bf127f5aa8fcf4fe4c16689d7de062b87`. Read-only review of the Oct-5/7 speed program, Oct-8 pilot integration, and Oct-10 ALG04–ALG06 checkers, registrations, recorded JSON and corrections. No experiments or published checkers were rerun. Static JSON extraction only inspected the committed artifacts. This reviewer did not design or implement any existing candidate.

## Decision

The newest recorded dispositions should be retained: ALG04 FAIL, ALG05 FAIL, ALG06 registered-rule PASS with accuracy/promotion HOLD. The latest authors already disclosed the consequential ALG06 failure of robustness; it is not a newly discovered numerical result in this review. The same applies to the G3 branch not acting, the raw stage target's nonnormal gap, and the ALG05 controls not firing on endpoint-only grids. Repeating their campaigns would not add information.

New static findings concern checker fail-closed behavior and durable evidence. They do not show that the committed solver runs or their reported ratios are false. Every committed ALG04/06 twin reproduction flag is true; all 599 ALG04, 828 ALG05 and 144 ALG06 exported state vectors inspected have no nonfinite bit patterns. These counts are artifact inventory, not scientific reruns.

## New findings

### EVD-01 — P2, finite/dimension checks absent from endpoint metric

`tools/alg04_coupled_target_v2_check.py:96-105` and `tools/alg06_guard_v2_check.py:88-89` use `max(... for a,b in zip(y,ref))` with no equal-length or finiteness check. A shorter successful state silently excludes trailing coordinates; a NaN in a non-first coordinate can be ignored by Python's `max` comparison order. Empty vectors raise instead of accepting, but nonempty truncation is not rejected. `alg05_controller_v2_check.py:73-76` asserts length but still does not explicitly reject NaN, and `assert` disappears under `python -O`.

Impact: malformed future exports can receive an endpoint error that omits invalid/missing components. Existing committed states inspected do not contain this defect; do not rewrite their results. A future common checker schema should reject empty/mismatched/nonfinite states and references before computing metrics, and reject unexpected bool/string values in numeric fields. Use explicit exceptions or INVALID rather than assertions. A focused synthetic malformed-artifact regression is sufficient; no solver campaign is needed.

### EVD-02 — P2, identity diagnostics do not participate in gate

ALG04 constructs `item1["pass"]` at `tools/alg04_coupled_target_v2_check.py:250-256`, then appends `dup_fix_other_counters_equal` and `twins_and_lu_reproduced` at lines 257-260. Those values cannot affect that pass. ALG06 records twin equality at lines 168-169, but uses it only under `reported.twins_reproduced` at lines 337-340; `verdict` at 269-272 ignores it. Accuracy denominators are taken from RUNS twins rather than the immutable BASE twins.

Impact: a future changed twin/reference implementation can silently change accuracy thresholds while the checker still labels its identity gate PASS. In ALG04 the registered identity item literally mentions Legacy and DupFix, not a separate twin condition, so this is a promotion-grade authority gap rather than a demonstrated violation of the historical registration. Current artifacts report all these identities true. Remedy in a new version: bind twin/reference identity as an artifact-validity precondition or compute the denominator directly from the source-bound BASE, then use complete schemas before numeric gates. Do not retrospectively change ALG04/06 verdicts.

### EVD-03 — P3, duplicate cell rows collapse silently

ALG04 dictionary comprehensions at lines 209-217 and ALG06 at lines 146-152 collapse repeated cell keys. Their set/count tests inspect the resulting dictionaries, not raw-row uniqueness. Identical extra rows are accepted; contradictory duplicate rows are last-wins. ALG05 additionally checks raw row count and arm-set cardinality, which prevents the corresponding easy duplication path on RUNS. For ALG04/06, require exact unique keys, exact raw row count, and exact prescribed cell/arm sets before evaluation.

No duplicated committed rows were identified from the declared inventory (65 ALG04 adaptive rows; 18 ladders; 24 ALG06 rows). This is a reusable evidence validator hardening item, not evidence to invalidate published counts.

### EVD-04 — P3, reviewer sensitivity data exist only in narrative

`research/alg06_guard_v2_20261010/PREREGISTRATION.md` correction explicitly states that the 24-run h0 perturbation experiment was unregistered and not committed. L-0097 binds the original RUNS/RESULTS hashes, not raw sensitivity rows or the review program. The qualitative warning is appropriate and should be preserved. The reported extrema, median 2.39 and 15/24 failures have narrative provenance, not independently inspectable raw-artifact provenance in this repository.

Do not rerun merely to replace this historical narrative. If original raw logs still exist, archive them with source/command/hash; otherwise mark them `PUBLISHED_REVIEW_NARRATIVE_ONLY`. A necessary new robustness experiment should preregister and persist every cell/arm/trace, so it produces new evidence rather than pretending to reproduce the lost review experiment.

### EVD-05 — P3, unit-test presence is not a machine-readable test pass

ALG05 item 6 scans for two `#[test] fn` names (`tools/alg05_controller_v2_check.py:257-263`, `376-383`). It does not consume an execution receipt, and accepts those names even if a test body were vacuous. This is candidly disclosed in the script and registration, and the project records a successful Cargo run in prose, so it is not a concealed defect or a reason to rerun ALG05. For the next node, preserve a test execution receipt bound to source commit, command, return code and exact test names, and record `unit_contract_pass` separately from `closed_loop_mechanism_activated`. The existing grid has no interior output points, and both new controller fixes were inactive on all 276 cells.

## Published blockers to carry forward once, without revalidation

| Published finding | Current evidence | Required disposition |
|---|---|---|
| ALG04 CoupledGuarded2 tight `vig1b-k20` accuracy | L-0094 / RESULTS item 2: 4.18x dense twin; Legacy itself 1.73x | No general nonnormal accuracy promotion; need a different mathematical mechanism, not repeated exhaustion |
| ALG06 fragile long Robertson endpoint | L-0097; 15/24 reviewer perturbations over 1.5x, median 2.39x | Registered PASS retained; robust accuracy claim HOLD |
| G3 cannot address per-unit-step target gap | ALG06 correction: charges <=3.4e-7 at rtol1e-5, <=2.7e-3 at1e-9, no acceptance crossing | Stop re-testing this additive local-error charge unchanged; target contamination budget itself |
| ALG05 v2 equals v1 on all 276 cells | ALG05 RESULTS / correction | Separate numerical work benefit of predictive rule from the two unactivated edge fixes |
| ALG05 single-cell absolute bound failure | L-0098, PREDcap2 err/rtol65.17 vs I65.04 | Preserve FAIL; new relative/distributional question requires its own rule and fresh corpus |
| Stage-target savings contain small-n bookkeeping effect | ALG04 DupFix 0.63–0.89x Legacy JVPs on small n, own target GM0.738 vs ProjL2 on Brusselators | Compare algorithms against DupFix/ProjL2, not only historical Legacy |
| SVD costs missing from WorkCounters | ALG04 reported-only correction | JVP reduction is not total-work or speed reduction; track orthogonalization, SVD, projection, fallback and failed work |
| Wall-time authority remains HOLD | TIMING_DESIGN_CONTRACT; speed status | Instructions/allocations/iterations have their own units; no speedup factor from them |
| SPD08 N=2 lanes FAIL kill | L-0089, 0.924/0.937 Ir ratios, gate0.8, kill0.9 | Closed as currently designed; no repeat without different bottleneck |
| SPD07 previous-step warm start FAIL | L-0084 | Prior zero-start advantage is a hypothesis for an adoption gate, not a default change |

## Closures since the October-4 audit (do not report stale open findings)

| Old finding | New disposition and binding |
|---|---|
| `INTAKE-NATIVE-03` GCRODR refresh not wired | SAFE-RECYCLE/L-0066 implemented explicit `RefreshAfterUpdate` and `Cold` driver policy; current `rodas5p_matrix_free_fast.rs:69-133`, `933-939`, `1440+`. Legacy remains default intentionally. The integration gap is closed for opt-in use; adoption is separate |
| `INTAKE-NATIVE-O2/O3` directed composition | SAFE-ENCLOSURE/L-0064 identified local counterexamples and repaired compositions; follow-up L-0065 fixed the exponential floor. Current chart uses product interval at `chart_transport.rs:243-246`; nonnormal decay uses interval time at `nonnormal_certificate.rs:515-524`, midpoint radius from actual rounded midpoint at531+ |
| Fourier certificate/native predictor missing | PP05 and PP06 native ports exist (L-0072/73), registered FAIL for certificate parity threshold/alias control expectations; no false certified bounds observed. Do not call these unimplemented |
| Shared-shift client not justified | PP07/L-0074 decision ABSTAIN: actual Fourier client performs no shifted solve, all hypothetical halving starts violate admissibility. Preserve the kill decision, do not manufacture many-shift workload |
| Complex-shift certificate missing | PP16/L-0070 native H=I gain |gamma|/Re(gamma) certificate exists, 480single-column+16partial-fraction published cases and36negative controls. Broader H-general implementation remains distinct |
| Laguerre total/router absent | PP08/L-0071 routes76/79 certified cases but zero to Laguerre; PP09/L-0077 bound decomposition cannot recover missing components and envelope tightening is insufficient. Native Laguerre implementation is not missing |
| Non-Krylov alternative absent | PP10/L-0080 Leja EstimateOnly and PP11/L-0078 total Taylor fused certificate implemented. Laguerre/Leja/Taylor/Chebyshev have different acceptance authorities; do not compare EstimateOnly counts to certified work as speed evidence |

## Concrete next decision nodes

1. **DUP-PROMOTE**: smallest useful coding adoption. Reuse ALG04 identity results. Audit `skip_final_residual` entry/exit semantics and residual telemetry contracts; run only new solver-mode/zero-RHS/nonfinite/budget/failure boundary tests. Keep verification residual required for acceptance; remove only redundant diagnostics. Baseline is current source, same binary and full solve/trajectory counters. Do not bundle target or guard changes. Success is exact solution/decision parity with expected diagnostic JVP removal; scientific speed is not asserted.
2. **BUDGET-CONTAMINATION**: accuracy priority. G3's charge is in the local error budget whereas target allowance is proportional to h/T. Design a separate cumulative stage contamination ledger (state and embedded output contributions, explicit weights and verified residual-to-error gain). Debit every approximate accepted stage, including floor/stall/fallback, before attempt commit. If the gain is unavailable or the remainder is insufficient, abstain/escalate/shorten; never label the projected-space inverse estimate a certificate. Record a bound, numerical estimate, or diagnostic with distinct types. Gate only a small new exact-linear/nonnormal model family and a fresh robustness corpus after the algebraic premise is checked. Frozen-J linear analysis alone is not a nonlinear/global error theorem.
3. **CTRL-GRID-ROBUSTNESS**: predictive controller with interior sample times, sliver and err=0-after-rejection cases. First demonstrate mechanism activation using focused contracts. Then fresh deterministic h0/grid perturbations against reference controller, at matched error; persist per-cell errors, work, failures and calibration fits. A corpus median/quantile is a descriptive rule over that declared corpus, not a statistical population claim. Do not infer independent random samples from three fixed h0 values. Keep worst/catastrophic cell and uncertainty exclusions visible; replacing the prior absolute limit must be explicit new registration.
4. **COST-ATTRIBUTION**: coupled-target work gate after accuracy premise holds. Candidate vs DupFix, ProjL2 and strongest structural rival (banded/direct when problem declares bands), charge JVP+orthogonalization+SVD+preconditioner+fallback+failed work. Callgrind instructions per complete trajectory at matched error can give a hardware-specific operation result; wall-time requires the timing authority gate. No extrapolation from n<=40 exhaustion to PDE-sized systems.
5. **VALIDATOR-V2**: use static common schema + short synthetic counterexamples for EVD-01/02/03 and source-bound execution receipts for EVD-05. Emit INVALID for malformed evidence; FAIL for valid experiments missing scientific gates; HOLD for promotion authority. Preserve immutable old outputs and ledger corrections rather than patching their verdicts in place.

## Scientific gate design notes

- Fixed rtol or JVP per accepted step is not generally matched accuracy per trajectory. Report completed trajectory error and every attempted stage cost together.
- Global power-law OLS over entire tolerance ladders is a model, not the literal measured Pareto frontier. Next promotion should include fit residuals, monotonicity/range checks, and the registered measured cheapest-run comparator rather than smoothing away isolated costly cells. Existing frontier results remain what their registration defined.
- Numerical reference disagreement at 1e-12 vs1e-13 is a useful empirical uncertainty filter, not a rigorous true-solution enclosure. Separate exact analytic or high-precision-independent references from same-method agreement.
- Existing failures provide design information. New runs are warranted only when a changed mathematical contract, new implementation boundary, or independently selected robustness cell resolves an explicit uncertainty.
