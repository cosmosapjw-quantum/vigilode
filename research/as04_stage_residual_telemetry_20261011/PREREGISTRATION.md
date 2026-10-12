# Preregistration: residual and budget telemetry of every accepted stage solve (AS04)

Node AS04 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(finding F103 of `docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`). Executed on branch
`audit/rvj-reaudit-remaining-20261011`, registered on top of `0ce7c32`. It depends on:
- AS01, the overflow-safe production fallback (`ProductionFallbackRule`, merged in `0283c1f`);
- AS03, the fail-closed evidence validator `tools/evidence_schema_v2.py` (merged in `d0c5722`), used by the checker.

**Claim boundary.**
- Telemetry only. It is **not an error certificate**: no record carries a verified bound, and a met residual target
  is not a forward-error, step-contamination or global-accuracy statement (MATHEMATICS_PORTING_KO.md section 2).
- Every change is opt-in and default-off. No default coupled acceptance, no change of any accept/reject decision, no
  production default promotion (DAG global invariant).
- No wall-time or instruction claim. Work is counted (WorkCounters, Krylov columns, true residuals) only.
- ALG06's record (L-0096, corrected by L-0097: "G3 never acted") is not re-scored and the G3 experiment is not
  repeated. ALG06 rows are rerun only as a bitwise parity anchor for the new telemetry.

## Question

The staged stage solver accepts iterates in four ways: `Converged`, `StallAccepted`, `FloorAccepted` and
`FallbackAccepted`. The re-audit found three accepted residuals above the requested target among nine small
configurations (`NATIVE.json` `/implementation_adversaries/stage_acceptance`). The driver's G3 bookkeeping debits only
`FallbackAccepted`. The coupled target's roundoff raise (`16 eps ||D b||`) and the nonnormality tightening also change
the threshold silently.

Can every staged stage solve and every attempt record, without changing any trajectory or hiding any work:
- the requested target, the raised target, the tightened target and the actual residual;
- the acceptance reason, the heuristic gain and an (always unavailable) verified bound;
- the metric and the operator epoch;
- the work of every attempt, including rejected and failed attempts, with a contamination debit for committed attempts
  only?

## Change

1. **Solver telemetry** (`crates/rodas5p-krylov/src/gmres_staged.rs`).
   - New `solve_staged_gmres_observed(..., requested_target: f64, telemetry: &mut StagedSolveTelemetry)`.
     `solve_staged_gmres` becomes this function with the telemetry discarded and must stay bitwise unchanged.
     `StagedGmresReport` and `StagedGmresOutcome` keep their fields and variants, because pre-existing tests
     compare whole reports.
   - **Deviation from the DAG (disclosed).** The DAG lists `gmres_staged.rs::StagedGmresReport` as an edit target.
     AS04 deliberately leaves `StagedGmresReport` unchanged and puts the new fields in the separate
     `StagedSolveTelemetry`. Reason: pre-existing tests and the recorded `NATIVE.json` S1 reports compare whole reports
     bitwise (items 1 and 4), so a new report field would modify pre-existing test files or break the S1 anchor.
   - `StagedSolveTelemetry` is a `Copy` struct of scalars, with no vector field. It holds:
     - `reason: StagedAcceptanceReason {Converged, StallAccepted, FloorAccepted, FallbackAccepted, Failed}`;
     - `requested_target` (caller input), `raised_target` (= `report.threshold`), `tightened_target`
       (= `final_threshold`) and `attainable_floor` (the floor value that admitted a stall or floor acceptance;
       N/A otherwise);
     - `actual_residual` (true residual 2-norm of the returned iterate) and `actual_residual_inf` (infinity norm of
       the same residual vector, read in place from the workspace);
     - `attainment: TargetAttainment {Met, BudgetExceeded, Unverified}`:
       - `Met` iff `actual_residual <= requested_target`;
       - `BudgetExceeded` iff both are finite and `actual_residual > requested_target`;
       - `Unverified` iff either is non-finite or was not computed.
     - `heuristic_gain: HeuristicGain {nu_max, ...}`, labelled heuristic, and
       `verified_bound: VerifiedBound::Unavailable` (the only variant AS04 may produce);
     - work: `arnoldi_jvps` (columns), `residual_jvps` (true residuals), `fallback_evaluations`,
       `acceptance_vector_copies` (length-n copies made by the fallback predicate), `nu_flops`, and
       `uncounted_shadow_matvecs` (the scratch-counter work of the shadow classification). The last one is reported
       and never added to WorkCounters.
2. **Driver telemetry** (`crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`).
   - `StageSolveStatistics` keeps its fields and serialization: they are pinned by recorded exports.
   - A new opt-in `StageResidualTelemetry` (switch `Rodas5pMfFastWorkspace::set_stage_telemetry`, and a new entry
     point `integrate_rodas5p_mf_fast_observed_with_stage_telemetry`) holds two kinds of record.
   - **`StageResidualRecord`, one per staged solve:**
     - attempt and stage index, and the solver telemetry;
     - `metric: ResidualMetric {ScaledWrms {atol_bits, rtol_bits, state_digest}, UnscaledL2}`;
     - `operator_epoch: {model_epoch: Option<u64>, generation: u64}`, where `generation` increments at every
       re-linearization;
     - `h` and `gamma` bits;
     - for coupled policies, `tau_e,i` and the heuristic debit `tau_e,i ||r_i||_WRMS`.
   - **`AttemptResidualRecord`, one per attempt:**
     - `t`, `h`, and an outcome `{Committed, RejectedError, FailedLinear, FailedOther}`;
     - per-reason stage counts, the count of `BudgetExceeded` stages, and the maximum `actual/requested` ratio;
     - `debit_all`: the heuristic debit summed over every accepted stage that is not `Met`, in stage order;
     - `debit_fallback_only`: the existing G3 charge;
     - the attempt's work deltas: `jvp_calls`, Krylov columns, residual JVPs, shadow matvecs and vector copies.
   - **Ledgers.** `committed_debit` sums `debit_all` over `Committed` attempts only. `work_all_attempts` sums the work
     of every attempt.
   - The requested target is `eps_i sqrt(n)` for coupled policies (scaled L2), and `max(atol_lin, rtol_lin ||b||)`
     for the other staged policies.
   - **Unchanged.** No decision reads the telemetry, and G3's applied charge is unchanged.
   - **State digest.** The digest is computed once per attempt, and only with telemetry on.

## Cells

- **S1 (solver, recorded).** The nine `stage_acceptance` configurations of `NATIVE.json`: identity2, triangular2 and
  triangular4, each strict, stall and floor, with `requested_target` = the recorded `threshold`. `gmres_staged.rs`
  is unchanged since `95d589e`, so the recorded reports are the reference.
- **S2 (fallback).**
  - The two `fallback_seam` configurations (the overflowing one must now fail; the finite control fails).
  - A finite cyclic 4x4 with `restart = budget = 1` and a production rule loose enough to accept (`FallbackAccepted`
    above target).
  - The same with a homogeneous-branch right-hand side (components 1.5e308, AS01).
- **S3 (guard and shadow).** The contraction-abort fixture of
  `crates/rodas5p-krylov/tests/alg04_alg06_staged_contracts.rs`, with classification of accepted aborts on. A
  counting operator wrapper gives an independent count of the shadow operator applications.
- **S4 (raised target).** A diagonal 2x2 with tiny `atol` and `rtol` large enough that `Converged` is reached against
  the raised threshold above `requested_target`.
- **S5 (failures).** `NonFinite`, `Breakdown`, and `BudgetExhausted` without a fallback.
- **D1 (driver, transactional failure).** The AS01 direct-driver boundary fixture of
  `tests/residual_acceptance_contract.rs` (item 3), which yields at least one `FailedLinear` attempt. The run may
  end in failure; its records must still be complete.
- **D2 (driver, epoch).** A 2x2 linear problem with `OdeProblem::with_model_epoch`, whose epoch changes after the
  third attempt.
- **R (real-driver cells).**
  - Arm `B3` of ALG06 (`CoupledGuarded2`, budget 2,000, stagnation guard, production fallback, G1 + G2 + G3, both
    classifications) on:
    - robertson-4e10 at rtol 1e-5 and 1e-9;
    - stosc-w1e4 at 1e-4;
    - van-der-pol-mu1000 at 1e-6;
    - brusselator-1d-50 at 1e-6, a control with no approximate acceptance in ALG06.
  - Arm `B3b` (G3 off) on robertson-4e10 at 1e-5.
  - `ProjL2` with the production fallback on brusselator-1d-50 at 1e-6, for the unscaled-L2 metric path. It has no
    ALG06 anchor.
  - Every R and D run is made with telemetry on and with telemetry off: 2 x (7 + 2) runs.

## Commands

    cargo test --offline --locked -p rodas5p-krylov --test staged_solve_telemetry
    cargo test --offline --locked -p rodas5p-integrators --test stage_residual_telemetry
    AS04_RUNS=research/as04_stage_residual_telemetry_20261011/RUNS.json cargo test --offline --locked --release -p rodas5p-integrators --test stage_residual_telemetry -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/as04_stage_telemetry_check.py --runs research/as04_stage_residual_telemetry_20261011/RUNS.json --native research/reaudit_accuracy_speed_20261010/NATIVE.json --alg06 research/alg06_guard_v2_20261010/RUNS.json --output research/as04_stage_residual_telemetry_20261011/RESULTS.json

All commands run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`.

**RUNS.json** stores, as hex bits:
- every record of every run;
- per fallback evaluation, `{branch, r_le_atol}`: the predicate branch taken (scaled-literal, scaled-homogeneous,
  unscaled-literal, unscaled-homogeneous) and whether `||r|| <= atol` held, so that item 2's copy rule is checkable;
- the full state at every output time, the counters and `StageSolveStatistics`;
- the charge log;
- the source commit and tree status.

**Checker.** `tools/as04_stage_telemetry_check.py` is committed with `tools/test_as04_stage_telemetry_check.py`
(mutation tests) before any recorded run. It calls `tools/evidence_schema_v2.py` first. Malformed or unbound evidence
gives **INVALID** (exit 2), a gate failure **FAIL** (exit 1), a pass **PASS** (exit 0). Each recorded sum is
recomputed in Python binary64, in the registered order.

## Gate

**Validity.** The result is INVALID if any of the following holds:
- the AS03 validation fails;
- a registered case, run or twin is missing or duplicated;
- a telemetry-off twin is missing;
- the source commit is not the committed implementation, or the tree is dirty;
- the checker postdates the run;
- `NATIVE.json` or ALG06 `RUNS.json` differs from its committed SHA-256;
- the checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold. Every item is exact (integer or bitwise). No numeric tolerance is used, by
design.

1. **No behaviour change.**
   - **1a. Telemetry on/off bitwise (the kill rule applies).** In every R and D run, telemetry on and off give
     bitwise identical results:
     - states at every output time;
     - attempts, accepted and rejected steps;
     - every WorkCounters field;
     - serialized `StageSolveStatistics`;
     - the charge log.

     Every pre-existing test passes, and no pre-existing test file is modified.
   - **1b. Telemetry off equals the ALG06 RUNS rows.** The telemetry-off `B3`/`B3b` rows equal ALG06 `RUNS.json`'s
     rows bitwise in the same fields. A 1b failure is a FAIL reported as anchor drift; the kill rule is not applied.
2. **No hidden JVP or copy.**
   - Per solve, `arnoldi_jvps + residual_jvps` equals that solve's Krylov matvec delta.
   - Per run, the attempt records' `jvp_calls` plus a run-level `jvp_outside_attempts` field sum exactly to the
     run's `jvp_calls`, with zero unattributed.
   - `uncounted_shadow_matvecs` equals the wrapper's independent count in S3, and is 0 whenever classification is off.
   - `acceptance_vector_copies` equals the sum, over the recorded fallback evaluations, of the documented copies of
     each evaluation's `{branch, r_le_atol}`: scaled-literal 1; scaled-homogeneous 2 (1 when `r_le_atol`);
     unscaled-literal 0; unscaled-homogeneous 1. The number of recorded evaluations equals `fallback_evaluations`.
   - The record types are `Copy`, a compile-time assertion.
3. **Accepted residual above target is visible.**
   - Every record with `reason != Failed` and `actual_residual > requested_target` has `attainment = BudgetExceeded`.
   - Every committed attempt containing one is counted in the attempt's `BudgetExceeded` stages.
   - In S1, exactly `triangular2_stall`, `triangular2_floor` and `triangular4_floor` are `BudgetExceeded`, with
     `actual_residual` bitwise equal to the recorded `residual_norm`. The other six are `Met`.
   - In S4 the reason is `Converged` and the attainment is `BudgetExceeded`.
4. **Complete and consistent records.**
   - In every run, the records equal `StageSolveStatistics.solves`, and the per-reason counts equal its `converged`,
     `stall_accepted`, `floor_accepted`, `fallback_accepted` and `failed`.
   - There is one attempt record per attempt.
   - `requested <= raised`, and `tightened <= raised`.
   - `Converged` implies `actual <= tightened`. `StallAccepted`/`FloorAccepted` imply
     `tightened < actual <= attainable_floor`. `FallbackAccepted` implies a guard abort or budget exhaustion.
   - S1 reports equal the recorded `NATIVE.json` reports bitwise.
   - `verified_bound` is `Unavailable` in every record.
5. **Ledgers.**
   - `committed_debit` equals the sum of `debit_all` over `Committed` attempts only.
   - `work_all_attempts` includes every `RejectedError` and `FailedLinear` attempt. D1 has at least one such attempt
     with nonzero JVPs.
   - `debit_fallback_only` equals the charge-log `charge` bitwise for every charged attempt, and
     `debit_all >= debit_fallback_only` in every attempt.
6. **Epoch and metric.**
   - In D2, `generation` increments exactly at the re-linearizations, and `model_epoch` changes at the fourth
     attempt.
   - `metric` is `ScaledWrms` for coupled runs and `UnscaledL2` for `ProjL2`.

Everything else is **FAIL**, with every number preserved.

**Kill and hold rules.**
- If item 1a fails, the telemetry is not adopted, and no telemetry number is used downstream. A 1b failure alone does
  not trigger this rule.
- If completeness (items 3-4) would need a decision change, HOLD: no default coupled acceptance follows. In the ledger
  this HOLD is recorded as verdict FAIL with claim "decision HOLD".
- A PASS gives AS06 an accounting input only. It never certifies an error or a budget.

**Reported, not gated:**
- per-reason and per-attainment counts;
- `actual/requested` distributions;
- `committed_debit` against the G3 charge;
- attempts whose `norm + debit_all` would cross 1;
- the telemetry's memory per run.

## Coordination with sibling nodes

AS04, SP02, SP04 and PC01 all edit `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`. Their changes are
merged in the fixed order AS04 -> SP02 -> SP04 -> PC01, each as orthogonal opt-in fields, default off. No node adds a
new entry point where a field on the existing options or workspace suffices; AS04's switch is the workspace field
`set_stage_telemetry`, and its observed entry point exists only to return the records. Each later node's parity tests
include the earlier nodes' options-off identity, so AS04's telemetry-off identity (item 1a) is re-checked by SP02,
SP04 and PC01.

## Prior information (disclosed)

- `NATIVE.json` S1 reports: `StallAccepted` 2.7756e-17 and `FloorAccepted` 7.8505e-17 / 6.6844e-16, against thresholds
  3.18e-31 and 1.61e-30.
- **ALG06 `B3` stage statistics:**

  | Cell | Stall | Floor | Fallback | Charged attempts | Roundoff raises |
  |---|---|---|---|---|---|
  | robertson-4e10, 1e-5 | 3 | 241 | 58 | 42 (max charge 3.4e-7) | 443 |
  | robertson-4e10, 1e-9 | 14 | 2,138 | 856 | 320 | 3,921 |
  | stosc-w1e4, 1e-4 | 0 | 29 | 0 | 0 | 43 |
  | van-der-pol-mu1000, 1e-6 | 0 | 15 | 0 | 0 | 404 |
  | brusselator-1d-50, 1e-6 | 0 | 0 | 0 | 0 | 38 |

  The cells were chosen knowing these counts.
- `gmres_staged.rs` has SHA-256 `5e18bdb6...` now, the same as in ALG06's L-0096 inputs. ALG06 `RUNS.json` is
  `26aaa444...`. The driver changed since ALG06 by AS01 (same decisions for finite inputs) and by SP01 (default
  accounting unchanged; staged path untouched), so the item 1b anchor is expected to reproduce.
- No code of this node exists before this commit.

## Predictions

- Items 1-6 hold. They are bookkeeping that is identical by construction.
- The most likely FAIL source is item 2's attempt-level JVP closure, through a JVP that is neither inside an attempt
  nor recorded in `jvp_outside_attempts`.
- Robertson 1e-5 `B3`: 302 non-`Converged` acceptances. Some `Converged` stages are `BudgetExceeded` through the
  roundoff raise; the count is unknown, between 1 and 443.
- `debit_all` exceeds the G3 charge on Robertson, but no attempt's `norm + debit_all` crosses 1 at 1e-5 (uncertain at
  1e-9).

## Results (appended after the recorded run; the registered text above is unchanged)

**Verdict: FAIL** (item 5). Items 1a, 1b, 2, 3, 4 and 6 pass. The kill rule (item 1a) is not triggered, the ALG06
anchor reproduces (item 1b), and the HOLD rule (items 3-4) is not triggered.

**Recorded commits** (branch `node/as04`, from `8ce9bda`).

| Commit | Content |
|---|---|
| `dd9f9ff` | Implementation and new tests: `solve_staged_gmres_observed` / `StagedSolveTelemetry` (gmres_staged.rs), `ProductionFallbackRule::evaluate`, `set_stage_telemetry`, `StageResidualRecord` / `AttemptResidualRecord`, `integrate_rodas5p_mf_fast_observed_with_stage_telemetry` (rodas5p_matrix_free_fast.rs); `crates/rodas5p-krylov/tests/staged_solve_telemetry.rs`, `crates/rodas5p-integrators/tests/stage_residual_telemetry.rs` (contracts and the ignored `export_runs`) |
| `bf92e67` | Checker `tools/as04_stage_telemetry_check.py` and `tools/test_as04_stage_telemetry_check.py` (35 synthetic/mutation tests), before any recorded run |
| `2fb627e` | `RUNS.json` (produced on `bf92e67`, clean tree, recorded tree status empty before and after) and `RESULTS.json` |

| File | Bytes | SHA-256 |
|---|---|---|
| `RUNS.json` | 14,500,815 | `e42c7be23ea01e2869bd8bbe1c6ad069a5b1f83b9b076ad63799c6306ea2c218` |
| `RESULTS.json` | 23,601 | `8227be57005df82a21379c4e936a442d0b7a00d7b750426d02be78bc2efab3bf` |
| `tools/as04_stage_telemetry_check.py` (blob `b48d3f9`) | | `94bf6a975c00c8f6a3a34dcd818a70217bd51fa65d49e018a78942edcbc7c9a3` |
| NATIVE.json (input, pinned) | | `b9404714af837dfb099db564550e88127ff52a0c9341758862529261b85bd364` |
| ALG06 RUNS.json (input, pinned) | | `26aaa444f06149335e904558805797067c0fdcc53c976b92ea1ee3fe56e67432` |

RUNS.json is below 20 MB and is not compressed.

**Commands.** The registered ones, with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 CARGO_INCREMENTAL=0`. The export
took 4.9 s. The checker took 28 min, almost all of it the pre-existing test suites of item 1a, which it runs itself
(`cargo test --offline --locked -p rodas5p-krylov -p rodas5p-integrators --all-targets`: exit 0, 701 passed, 0
failed). `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` are clean.

**Gate.**

| Item | Result | Numbers |
|---|---|---|
| 1a. On/off bitwise | PASS | 9/9 R and D twins identical (all output states, steps, every counter, StageSolveStatistics, charge log). No pre-existing test file modified (2 test files added; `#[cfg(test)]` tail of gmres_staged.rs unchanged). Touched crates' suites: 701 passed, 0 failed |
| 1b. Off = ALG06 | PASS | All 6 B3/B3b rows reproduce ALG06 RUNS bitwise (states, steps, counters, stage statistics, charges) |
| 2. No hidden JVP/copy | PASS | Every solve: Arnoldi + residual applications = Krylov matvec delta. Every run: attempt JVPs + 0 outside = run jvp_calls. S3 shadow: 99 uncounted matvecs = wrapper count; 0 without classification. Copies equal the documented table on every evaluation |
| 3. Excess visible | PASS | Every accepted record above its requested target is `BudgetExceeded`; per-attempt counts match. S1: exactly triangular2_stall, triangular2_floor, triangular4_floor `BudgetExceeded` with NATIVE's residual bits; six `Met`. S4: `Converged` + `BudgetExceeded` |
| 4. Complete records | PASS | Records = StageSolveStatistics in all 9 on-runs; one attempt record per attempt; target order and reason implications hold; S1 reports = NATIVE bitwise; `verified_bound` always `Unavailable`; registered S2/S3/S5 outcomes |
| 5. Ledgers | **FAIL** | committed_debit, debit, debit_all, debit_fallback_only and work ledgers recompute bitwise; D1 has 1 FailedLinear attempt with 2 JVPs; debit_fallback_only = charge-log charge on every charged attempt. **`debit_all >= debit_fallback_only` fails on 8 of 784 attempts of Robertson-4e10 1e-9 `B3`** (attempts 715, 734, 737, 739, 757, 760, 764, 769) |
| 6. Epoch and metric | PASS | D2: 39 attempts, generation increments exactly at the re-linearizations (rhs_calls = 7 + relinearized in every attempt), model_epoch 1,1,1,2,... (changes at the fourth attempt, which re-linearizes although the second and third attempts reused the state). ScaledWrms in all coupled runs, UnscaledL2 in all 736 ProjL2 records |

**Why item 5 fails.** On Robertson-4e10 1e-9, 8 of the 856 `FallbackAccepted` stages have
`tightened < actual <= requested`: the nonnormality guard (nu_max about 1.37) tightened the threshold below the
requested target `eps_i sqrt(n)`, the stagnation guard aborted (contraction), and the production fallback accepted an
iterate that meets the requested target. Such a stage is `Met`, so the registered `debit_all` (accepted stages that
are not `Met`) leaves it out, while ALG06's G3 charge (`debit_fallback_only`) includes every fallback-accepted stage.
The registered inequality `debit_all >= debit_fallback_only` therefore does not hold by construction once a
fallback-accepted stage is `Met`. The telemetry itself is consistent and complete (items 2-4); the failure is a
conflict between the two registered debit definitions, not a hidden residual. No decision change is involved.

**Reported, not gated.**

| Run | Stages (Conv / Stall / Floor / Fallback / Failed) | Met / BudgetExceeded | Conv. BudgetExceeded | committed_debit | G3 charge sum | crossings | memory |
|---|---|---|---|---|---|---|---|
| Robertson 1e-5 B3 | 850 / 3 / 241 / 58 / 0 | 639 / 513 | 211 | 1.08e-5 | 3.90e-6 | 0 | 320 KB |
| Robertson 1e-9 B3 | 3,264 / 14 / 2,138 / 856 / 0 | 1,701 / 4,571 | 1,574 | 8.04e-2 | 6.99e-2 | 0 | 1.74 MB |
| stosc 1e-4 B3 | 59 / 0 / 29 / 0 / 0 | 41 / 47 | 18 | 6.0e-7 | 0 | 0 | 24 KB |
| van der Pol 1e-6 B3 | 3,737 / 0 / 15 / 0 / 0 | 3,678 / 74 | 59 | 1.4e-8 | 0 | 0 | 1.04 MB |
| Brusselator-50 1e-6 B3 | 736 / 0 / 0 / 0 / 0 | 702 / 34 | 34 | 1.0e-10 | 0 | 0 | 205 KB |
| Robertson 1e-5 B3b | 866 / 4 / 231 / 51 / 0 | 650 / 502 | 216 | 9.22e-6 | 3.55e-6 | 0 | 320 KB |
| Brusselator-50 1e-6 ProjL2 | 736 / 0 / 0 / 0 / 0 | 736 / 0 | 0 | 0 | 0 | 0 | 205 KB |
| D1 | 0 / 0 / 0 / 0 / 1 | 0 / 1 (failed) | 0 | 0 | 0 | 0 | 0.4 KB |
| D2 | 312 / 0 / 0 / 0 / 0 | 285 / 27 | 27 | 1.5e-10 | 0 | 0 | 87 KB |

"crossings" counts attempts with error norm <= 1 and norm + debit_all > 1. No `Unverified` record occurs in any
driver run. actual/requested over accepted stages: median 0.55 and p90 6.4e3 on Robertson 1e-5; median 153 and p90
9.8e5 on Robertson 1e-9 (the roundoff raise `16 eps ||D b||` dominates the tiny coupled targets). jvp_outside_attempts
is 0 in every run.

**Predictions.**
- Items 1-6 hold: **not confirmed** (item 5 fails; items 1-4 and 6 hold).
- Most likely FAIL source item 2's JVP closure: not confirmed (item 2 passes; the failure is item 5).
- Robertson 1e-5 B3: 302 non-`Converged` acceptances: **confirmed exactly** (3 + 241 + 58). `Converged` stages
  `BudgetExceeded` through the roundoff raise, predicted 1-443: 211.
- `debit_all` exceeds the G3 charge on Robertson: confirmed at both rtols (committed_debit 1.08e-5 vs 3.90e-6; 8.04e-2
  vs 6.99e-2). No attempt crosses 1 at 1e-5 (confirmed) nor at 1e-9 (was uncertain).

**Interpretations and deviations** (fixed in the checker docstring at `bf92e67`, before the run).
- Requested targets: coupled `eps_i sqrt(n)`; other staged policies and the solver cells (except S4)
  `max(atol, rtol ||b||_2)` of the solve's configuration (S1: the recorded threshold, bitwise); S4 `atol`.
- Copies table applied literally to evaluations that compared the residual norm (unscaled-homogeneous counts 1 even
  when `r_le_atol`, where the code makes 0; no registered cell reaches it). Exits before the comparison and
  unrepresentable thresholds are documented from the code (candidate rejected 0; residual rejected or unrepresentable
  1 scaled, 0 unscaled). The record also carries the evaluation's exit.
- S2 uses `ProductionFallbackRule` (overflowing 1e308 and control 1e100 at production rtol 1e-10; loose rtol 1.0 at
  scale 1 and 1e308). S3's fallback is the unscaled production rule with rtol 2. The registered solver-cell outcomes
  are gated under item 4.
- NATIVE's `"NaN"` matches any NaN. ALG06 floats compared as binary64 bits; counters on ALG06's key set.
- ProjL2 + fallback uses budget 200 (ALG04's); B3/B3b budget 2,000 (ALG06's).
- The per-attempt `BudgetExceeded` count is checked on every attempt (registered: committed ones).
- `debit_fallback_only` is the attempt's G3 charge (0 for non-coupled policies); charged attempts follow the ALG06
  charge-log rule (a fallback-accepted stage and a computed error norm).
- D2: `CoupledGuarded2`, budget 200, `y' = A y`, `A = [[-1, 1], [0, -1000]]`, `y0 = [1, 1]`, T = 1, atol 1e-8,
  rtol 1e-6, h0 0.5; the epoch closure returns 1 for its first three reads and 2 afterwards (read once per attempt;
  each twin has its own closure).
- The solver telemetry also carries the report's columns, matvecs, guard abort and budget flag; the stage record
  carries the per-solve `jvp_calls` delta; the attempt record carries `relinearized`, `error_norm` and `rhs_calls`.
- A schema smoke export (`export_fixture_smoke`, S and D cells only, no R cells) was run before the recorded run to
  exercise the checker on real-format data; it is not a recorded run and was not committed.
- No deviation from the registered change, cells, commands or gate.

**Claim boundary.** Telemetry only: no record is an error certificate, and a `Met` target is no forward-error
statement. Default off; no decision reads it. No wall-time claim.
