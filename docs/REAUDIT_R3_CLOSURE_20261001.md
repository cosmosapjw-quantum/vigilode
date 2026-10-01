# Re-audit R3 closure (2026-10-01)

This document covers the external R3 re-audit package `research/adversarial_reaudit_20261001_r3/`
(source `cc2cd041737e7ff543624d1b59893a3b4397369f`, 21-node `NEXT_DEVELOPMENT_DAG.json`). It lists what the stacked
branch `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3` implements for each node, which contract tests check it, the
research results with their preregistrations and ledger rows, and what stays open. Each node's claim ceiling is the one
the DAG states. Nothing here is a production readiness or speed promotion.

## Status legend

| Status | Meaning |
|---|---|
| **Closed** | Every acceptance item has a contract test on this branch. |
| **Closed (research)** | The node is implemented and tested. Its claim is limited to the research module or the declared domain. |
| **Measured: FAIL** | The preregistered experiment ran. Its numeric gate failed, and the result is kept as a negative ledger row. |
| **Measured: PASS** | The preregistered gate was met. The claim stays limited to the measured population. |

## Matrix

| Node | Status | Implementation | Contract tests / evidence |
|---|---|---|---|
| TIME-01 | Closed | `output.rs`: `step_to` integrates `h_eff = t_end - t` on the represented interval, with a TwoSum clock check, a `max_step` cap with one-resolution slack and `split_clock` for step doubling. Adaptive drivers stop on a typed time-resolution failure. | `r3_represented_clock_contracts.rs`: constant flow at `t0 = +-1e12` on the 8-ULP span, origin and unit shifts, hard stops, below-ULP steps, BDF across power-of-two boundaries. |
| TIME-02 | Closed | Strict `validate_span`. `OutputSchedule::uniform` and `CommonOutputGrid::uniform` reject indivisible spans (tolerance `min(8 eps max(|t0|,|tf|,span), spacing/1024)`) and a lone `[tf]`. | Same file: `uniform(1e12, nextafter^4(1e12), 1)` rejected, explicit `[tf]` rejected, adjacent endpoints and subnormal spans kept distinct, the valid control integrates. |
| TIME-03 | Closed | `FixedGrid` (`t0 + k h`, no final micro-step). The fixed BDF uses variable coefficients on unequal spacing. Sealed research replays keep `RESEARCH_REPLAY_CLOCK_POLICY = "nominal-accumulated-v0"`. Production uses `PRODUCTION_CLOCK_POLICY = "represented-indexed-v1"`. | 1000 steps of 0.01 on (0, 10) give exactly 1000 grid points. BDF startup and order history are checked. |
| ARITH-01 | Closed | `PhiTransformStatus`. A lost or subnormal weight makes the report non-converged (`TRANSFORM_ERROR_UNBOUNDED`). `dense_fused_phi_action` returns a typed error. The report variant carries the status. | `r3_phi_transform_authority_contracts.rs`: nilpotent witnesses at positive and negative `h`, PHI-R1/R2 fixtures within 1e-12, `A = 0` bounded acceptance. |
| ARITH-02 | Closed (research) | `rodas5p_core::transform_bound`: `ExpBound` mantissa and exponent bounds, `bound_transform_error` for nilpotent and dissipative classes, `Unbounded` otherwise. Exact weights give `delta = 0`. | `transform_bound_contracts.rs`: the `+-1e-8` / `1e308` fixture bound `1.6666666666666680e-27`, subnormal and overflow branches. |
| ARITH-03 | Closed | `is_reference_authoritative` and `weight_underflows` on dense references. The G3 gate holds a phi row whose reference is not authoritative (`phi_reference_not_evaluated`). | `r3_phi_transform_authority_contracts.rs`: the `diag(700, -700)` witness is flagged, the amplitude sweep keeps authority, an injected flagged row holds the gate. |
| ARITH-04 | Closed (research) | `certified_budget.rs`: `step_power_enclosure` and `OutputBudgetPolicy::certified_budget`/`certified_decide` with the statuses Exact, Enclosed, ExactZero, UnderflowLowerZero and OverflowNonBinding. | `r3_certified_budget_contracts.rs`: 18 exact rational fixtures (`fixtures/r3_certified_budget_fixtures.json`), including one-ULP boundaries. |
| STAT-DEV-01 | Closed | `PairedTimingCase::admit` (`INVALID_RAW_TIMING_PROTOCOL`) serves producer, assessment and replay. Per-session ABBA segments are allowed only for complete session runs. | `r3_raw_protocol_admission_contracts.rs`: five malformed variants, A/A cases, round-trip, singleton-session gaming. |
| STAT-DEV-02 | Closed | `paired_receipt.rs`: session provenance, recorded failures, arm and executable identity. `paired-timing-campaign` runs one process per session. The CLI wall criterion checks arm and identity. | `paired_timing_campaign_cli_contracts.rs`: a real 6-process campaign. No receipt gives NotEvaluated. Corruption is rejected. |
| STAT-DEV-03 | Closed | Hoeffding Monte-Carlo gate on `K = #{T* < ln 1.15}` with a predeclared `delta = 0.01`. The decision needs agreement between endpoints and gate, otherwise it is Inconclusive (schema v3). Resamples are never extended after the data are seen. `percentile_endpoint_decision` is kept as a diagnostic. | `r3_monte_carlo_gate_contracts.rs`: the exact 6^6 probability 32884/46656 lies inside the gate interval, the atom at the threshold stays unresolved, all-above and all-below cases, no promotion below B = 4239. |
| STAT-DEV-04 | **Measured: FAIL** | `docs/TIMING_DESIGN_CONTRACT.md` fixes the estimand, cells, missing-cell policy and frozen recipe. `timing_design.rs` and `rodas5p paired-timing-coverage-study` implement the study. The median is computed by selection, with the same bits. | `r3_timing_design_contracts.rs`: pair multiplication changes no bit, session duplication adds no session. Preregistered studies `research/r3_timing_coverage_study_20261001/` (`L-0007` FAIL) and the corrected `..._v2_20261001/` (`L-0010` FAIL). In both, single-case designs undercover (0.78 to 0.93) and five-case designs cover 0.99 or more. |
| HOM-01 | Closed | `stage_target.rs`: `STAGE_TARGET_SEQUENTIAL` and `STAGE_TARGET_STRICT_LOWER_PROJECTION` carry coefficient bits. `native_coefficient_leakage` counts alpha 28 / 5.58e-16 and L 34 / 3.77e-16. `block_sequential_allowance` gives the allowance, and `strictly_lower_nilpotent` checks the target's own structure. | `coefficient_structural_contracts.rs` and `fixtures/stage_target_semantics.json`. |
| HOM-02 | Closed (research) | `outward_certificate.rs::certify_stage_target`: residual enclosure, witness, upward recurrence, output and embedded projections. `CertificateKind::StageTargetBound` is not an ODE error bound. | `outward_certificate_contracts.rs`: all 24 recorded fixtures are enclosed on the matching target and match the Python decisions (output accept 12, combined 8). Fail-closed cases are covered. |
| HOM-03 | Closed (research) | `InverseWitness::{diagonal, small, approximate}` with a `WitnessIdentity` (h, gamma, Jacobian SHA-256, structure, tolerance) and `WitnessWork`. | Same file: wrong-identity witnesses are rejected, residual arithmetic is enclosed, costs are recorded. |
| HOM-04 | Closed (research) | `doubling_certificate`: path sum `(I+H^4)(I+H^2)(I+H) a` with a radius from `PastStepData`, which has no reference field. Failed radii are recorded, and at most `max_attempts` are tried. | Same file: 22 radii close, closure failures reject, enclosures are compared with the serial certificate. Worker execution is measured. |
| HOM-05 | Closed | `Q2Admission::NativeTargetCertificate` in `transactional_q1_q2.rs`. The eighth batch is not spent. The certificate binds `(y, h)`, the candidate digest and the sequential target. The budget comes from the certified embedded lower bound. Certificate work is ledgered separately. | `q2_diagnostic_replacement_contracts.rs`: q1 and q2 candidates, the accepted step and its stages are bit-identical, with 7 W batches instead of 8. Five wrong or unverified certificates fall back to the unchanged sequential path. |
| HOM-06 | **Measured: FAIL** | `rodas5p r3-campaign --study hom06` and `r3-campaign-verify`: five quadratic-family ODEs, certified transactional q1/q2 at 1, 2, 4 and 8 threads against the sequential JF-GMRES baseline. | `research/r3_matched_accuracy_hom06_20261001/`, ledger `L-0008` FAIL (`SPEEDUP_UNPROVEN`). Accuracy and per-step admission pass. Every arm is Blocked: speedup point 0.17 at 1 thread. |
| POLY-01 | Closed (research) | `rodas5p_core::polynomial_action`: joint Chebyshev and Laguerre actions for given `w_k`, a symmetric nonpositive domain with a Gershgorin-verified or declared enclosure, separate distinct-vector and same-vector recurrences, `poly_*` WorkCounters, and a coefficient cache keyed to operator, enclosure, `h`, degree and scale. | `r3_polynomial_action_contracts.rs`: 20 joint actions and 100 columns against the independent oracle (`fixtures/r3_polynomial_oracle_fixtures.json`), `h = 0`, `A = 0`, scalar matrices, nonnormal and range rejections, cache keys. |
| POLY-02 | Closed (research) | Coefficient enclosures from positive series with directed rounding. The degree is chosen by rigorous tail bounds. `TotalErrorStatus::Certified` applies to Chebyshev with a verified enclosure. `EstimateOnly` (`TOTAL_ERROR_NOT_CERTIFIED`) applies to Laguerre and to declared enclosures. | Same file: coefficient enclosures contain the 40-digit quadrature values, certified bounds enclose the observed errors, the one-ULP budget boundary never understates, range failures are explicit. |
| POLY-03 | **Measured: PASS** (measured population only) | `rodas5p r3-campaign --study poly03`: Chebyshev and Laguerre joint phi, cold and warm coefficients, against the Arnoldi fused phi on five symmetric operators. | `research/r3_matched_accuracy_poly03_20261001/`, ledger `L-0009` PASS. Warm Chebyshev is Promoted (point 3.17, interval 1.17 to 13.4). Cold Chebyshev is Inconclusive and cold Laguerre is Blocked. |
| PROCESS-01 | Closed | Every experiment on this branch has its `PREREGISTRATION.md` committed and pushed before its first output. The coverage study is `c8539fe`, its corrected v2 is `0799a05`, and HOM-06 and POLY-03 are `380f565` with a rerun addendum in `0799a05`. Each discloses its pilot runs and incidents. Ledger rows from L-0007 are appended. The existing prefix L-0001 to L-0006 is unchanged (`check-research-node.py --base` passes). | `git log` chronology. Each claim separates the process verdict, the numeric evidence and production readiness. |

## Research results

Every result below was preregistered before its first output. Process deviations are disclosed in the node, and the
verdicts are the preregistered gates'. The host is a shared 4-vCPU cloud container and is not an authoritative timing
host.

| Ledger | Node | Verdict | Numbers |
|---|---|---|---|
| L-0007 | STAT-DEV-04 coverage study v1 | FAIL | Single-case coverage 0.789 to 0.926 against a 0.9374 threshold. One false-promote rate is 0.045. Five-case coverage is 0.9925 to 1.0. |
| L-0008 | HOM-06 matched-accuracy transactional campaign | FAIL (`SPEEDUP_UNPROVEN`) | All lanes are accurate, with 0 admission mismatches. Every arm is Blocked: points 0.17, 0.07, 0.05 and 0.04 at P = 1, 2, 4 and 8. The candidate makes about 5x the baseline's RHS evaluations. |
| L-0009 | POLY-03 matched-accuracy polynomial campaign | PASS (measured population) | Errors are at most 2.2e-13. Warm Chebyshev is Promoted at 3.17 (1.17 to 13.4). Cold Chebyshev is Inconclusive (0.57), cold Laguerre is Blocked (0.12) and warm Laguerre is Inconclusive (1.18). |
| L-0010 | STAT-DEV-04 coverage study v2 (corrected) | FAIL | Single-case coverage 0.7825 to 0.9255. Five-case coverage 0.9935 to 1.0, with promotion at most 0.0015. |

Interpretation:

- **Timing statistics.** Paired timing decisions on single-case corpora are not authoritative. Both coverage studies
  fail for that reason, and the defects in v1 did not cause its failure. Five-case designs are conservative under the
  declared model, but the preregistered gate covers all designs, so every timing decision on this branch, the POLY-03
  Promote included, carries `STATISTICAL_AUTHORITY_HOLD`.
- **HOM-06.** The certified transactional path is not faster on this corpus. It does several times the RHS and JVP work
  of the sequential path, and building the thread pool for each step makes more threads slower.
- **POLY-03.** The only speedup observed is coefficient reuse on a frozen operator. Paying coefficient setup on every
  call removes it. This is evidence for a frozen-operator regime on small dense symmetric problems, not a general claim.
- No production readiness or speed promotion follows from any row.

## Independent review and execution incidents

A read-only adversarial review of the branch diff (2026-10-01) reported the findings below. Each is resolved in `d198558`
unless stated otherwise.

| Finding | Resolution |
|---|---|
| HOM-05: the model check (`f(y)`, one JVP) passes a cubic ODE declared as quadratic, and a `q` hidden where `y_a = 0`. | The model must also match the ODE at every candidate stage state. The cubic case is a regression test and now falls back. Agreement remains a consistency check, not a proof. |
| The coverage study's data and bootstrap shared one random stream, and its seeds depended on grid position. | Separate seeds, keyed by scenario id. The committed v1 result is kept with an appended note. The corrected v2 study is preregistered and run separately. |
| The Monte-Carlo gate documentation claimed it bounds simulation error only. | Documented as conservative beyond the resampling error: Promote needs `K/B < 0.0087` at B = 10000. The gate is unchanged, as preregistered. A variance-aware bound is future work. |
| The HOM-06 "admission mismatch" item was a count difference that could not fail. | Counted per step (`certificate_mismatches`). |
| POLY-03 built the reference operator inside the timed call. | Both operators are built outside. |
| `r3-campaign` headlined the unverified decision and aborted all arms on one failure. | The verified decision is headlined, receipt failures are counted, and a failing arm is recorded. |
| Unbounded polynomial mode differed in the sign of zero. | Sums start at +0.0. The test compares bits. |
| `critical_path_depth` and the reused gate fields in certificate mode. | Documented. The certificate work is `certificate_operations`. |

Execution incident: the first preregistered HOM-06/POLY-03 run, at `86750eb`/`380f565`, aborted in the shared session
harness on the A/A batch defect (`INVALID_RAW_TIMING_PROTOCOL`) before any receipt was written. The defect is fixed in
`c8aa0dc` with a regression test. The outputs are kept in `aborted-run-380f565/`. The rerun addenda were committed before
the rerun. The one unverified decision that was printed, HOM-06 p1 `Block`, is disclosed there.

## Known limitations kept open

- Timing receipts are unauthenticated JSON. A fabricated record with fresh provenance cannot be excluded
  cryptographically. Only its internal consistency and the executable hash are bound.
- `check_clock` is effectively vacuous after `step_to`, because the represented step is exact by construction. It is kept
  as a guard.
- A fixed `h` between 0.5 and 1 ULP of `t` becomes 1-ULP steps. `same_step` in the fixed BDF has an absolute floor.
- G3 rows serialized before R3 deserialize with `authoritative: false`, which is conservative.
- The research replay drivers (G2, G4 atlas, unified gates, homotopy experiments) keep the sealed legacy clock under
  `RESEARCH_REPLAY_CLOCK_POLICY`. Their outputs are unchanged and are not new evidence.
- The HOM-05 certificate covers autonomous identity-mass problems of the quadratic family. It checks agreement with
  the integrated ODE only numerically, at `f(y)` and one JVP.
- The POLY-01/02 module is dense and research only. The Laguerre recurrence rounding has no propagation bound, and a
  declared enclosure is recorded but not verified.
- The STAT-DEV-04 gate failed, so paired timing decisions carry `STATISTICAL_AUTHORITY_HOLD`. A redesigned interval for
  few independent units needs a new preregistered study.
