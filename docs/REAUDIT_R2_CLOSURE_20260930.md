# Re-audit R2 (2026-09-30): fix-closure matrix

Source audited: `7708ef9` (package `research/adversarial_reaudit_20260930_r2/`, 7 findings, verdict REWORK).
Fixes: branch `claude/jolly-wozniak-7wl15h-wu22-reaudit-r2`, commits `df2da78`, `5639a33`, `e1bc29d`, `90df209`, `e4c96be`.
This is DEV-08 of the audit's development DAG: the seven fixtures as regressions, one validation run on one source identity, and an independent review. DEV-09 to DEV-14 (calibration, polynomial backends, homotopy majorant, parallel path, measurement campaign, release gate) were not started.

The audit's own `run_reproductions.py` refuses to run on any source whose audited files differ from `7708ef9` (by design), so it cannot re-execute the probes on the fixed tree. Each fixture was ported into a Rust regression test instead, with the audit's numbers as expected values.

## Closure

| Finding | DAG criterion | Status | Regression |
|---|---|---|---|
| R2-OUT-01 (P1) | 8-ULP span at t0 = +-1e12 computes y = 1 or fails; no zero-attempt success | Closed for the fixture (h = span/2); see limit 1 | `r2_output_time_identity_contracts::a_short_representable_span_at_a_large_epoch_is_integrated`, `radau_fixed_steps_cover_a_short_span_at_a_large_epoch`, `a_step_below_half_an_ulp_is_a_typed_failure` |
| | `next_up(endpoint)` is not consumed at the endpoint | Closed | `a_request_one_ulp_after_an_endpoint_is_interpolated_not_snapped` |
| | adjacent representable requests are not duplicates | Closed | `adjacent_representable_output_times_are_distinct_requests` |
| | hard stop, partial prefix, failure ledger and BDF history keep their meaning | Closed on the tested drivers | `fixed_steps_that_accumulate_below_the_end_land_on_it`, `a_uniform_output_grid_adds_no_micro_steps` (BDF2 keeps its order and error), `grids_through_zero_from_negative_times_add_no_micro_steps`, `an_adaptive_step_across_zero_reaches_the_end`; all existing hard-stop and BDF tests unchanged and passing |
| R2-POL-01 | 1e-320 / 1e-160 / p = 2 budget ~ 0.999988867182683, WRMS 5 rejected | Closed | `r2_output_budget_contracts::the_step_term_is_evaluated_without_intermediate_overflow` |
| | epsilon_ref = 0 does not hide NaN in `min` | Closed | same test |
| | real homotopy consumer, both sides, step vs embedded term | Closed | `the_homotopy_step_consumes_the_binding_step_budget_on_both_sides` (step term 2^1030-scaled and embedded term, factor 1.01 accepts, 0.99 rejects) |
| PHI-R1 | h = +-1e-100, b4 = 1e300 gives 4.166666666666667e-102 | Closed (fused, dense, prefix) | `r2_phi_range_contracts::representable_weights_survive_the_transform` |
| | h = 1e100, b4 = 1e-300 stays finite | Closed | same test |
| | existing contracts not relaxed | Closed: no existing phi test was edited | full matrix |
| | transform loss not disguised | Closed: total loss is an error; partial loss is counted in `WorkCounters::phi_weight_underflows` | `the_weight_transform_reports_loss_instead_of_hiding_it` |
| PHI-R2 | A = -1, h = 0.1, w1 = c, c = 1e-200 .. 1e300, relative error <= 1e-12 | Closed | `the_dense_oracle_is_invariant_to_input_amplitude` (closed-form phi1 oracle) |
| | mixed range and cancellation labelled | Closed in `dense_phi_combination_report`; see limit 4 | `mixed_range_and_small_output_are_labelled` |
| | independent oracle | Scalar closed forms only; see limit 4 | same tests |
| R2-STAT-01 | one PID with six cases is not six sessions | Closed | `r2_timing_authority_contracts::a_producer_without_a_session_is_never_authoritative` |
| | candidate 0..5 with A/A 100..105 gets no authority | Closed | `an_aa_control_in_other_sessions_confers_no_authority` |
| | matched explicit sessions only, linked to the raw receipt | Closed in the library (`raw_cases_sha256`, `verify_against_raw`); see limit 2 | `a_preview_protocol_or_an_inconsistent_record_never_gates` |
| R2-STAT-02 | 16 legacy + 1 current call gives 17 calls, vectors unknown | Closed | `work_unit_contracts::unknown_vector_coverage_survives_aggregation` |
| | known + known exact, order- and grouping-invariant | Closed | same test |
| | unknown is not evaluated in the final comparison | Closed | `r2_work_coverage_consumer_contracts`, `global_error::r2_not_evaluated_tests` (front and attainment list the run) |
| R2-STAT-03 | B = 1 cannot promote | Closed | `a_preview_protocol_or_an_inconsistent_record_never_gates` |
| | confidence, threshold, schema tampering rejected | Closed for self-consistency; numeric edits need the raw cases (limit 2) | same test (9 tamperings, duplicate sessions, forged interval) |
| | exact 6^6 endpoints kept; simulation uncertainty reported | Closed for the audit fixture | `the_resampled_interval_matches_the_exact_six_session_bootstrap` |

## Limits that remain

1. **Step size vs represented time.** At t0 = 1e12 a step of h = 1e-4 advances t by one ULP (1.22e-4) while the stage equations use h. With y' = 1/span over an 8-ULP span this gives y = 0.8192 after 8 steps. The DAG fixture (h = span/2) passes. Fixing it means computing each step's h from the represented times (`t_new - t`), which changes every fixed-step trajectory; not done.
2. **Session identity is declared, not observed.** Labels come from the caller (`measure_paired_case_in_session`); nothing binds a label to an OS process or a campaign. The CLI consumer checks record consistency (`verified_gate_decision`) but has no raw cases to recompute from, because no paired-timing runner feeds it yet; every CLI wall criterion is still NotEvaluated.
3. **Long fixed-step runs.** Accumulated time drift larger than the landing residue (64 eps max(|t|, |t+h|, |target|), capped at h/1024) still ends with one extra micro-step, e.g. 1000 steps of 0.01 on (0, 10). This predates the audit; index-based times would remove it.
4. **Oracle scope.** The fused Krylov and dense paths now share `weight_phi_vectors`; their independent check is the closed-form scalar fixtures. The G3 gate uses `dense_fused_phi_action`, which does not return the amplitude labels.
5. **Research drivers.** The g4 regime-atlas, unified-gate and homotopy-experiment research drivers keep the old `10 eps max(|tf|, 1)` end slack, so sealed research replays (v3.7 V4) are unchanged.
6. **Budget certification.** The policy evaluation is safe (no overflow, no hidden NaN); a directed-rounding `error_upper <= budget_lower` path (DEV-02 step 4) was not implemented.
7. **Timing records.** Records of schema v1 still deserialize but are rejected by the consumer (NotEvaluated).

## Independent review

Two independent review passes (AI reviewer, bit-exact Python emulation of `step_to`, `land`, the collectors and the drivers; no shared code with the implementation):

- Pass 1 (on the first patch) found 10 defects, including two P1 regressions: Radau fixed drivers keeping the old slack, and ordinary fixed-step runs whose time accumulates to 0.9999999999999999. All are fixed (`df2da78` to `90df209`), and each has a regression test.
- Pass 2 (on `e1bc29d`) confirmed the fixes and found three further step-size issues: a residue that collapsed near zero, the h/8 extension cap, and the `lands` flag. These are fixed in `90df209`. Its remaining observations are limits 1 to 5 above.

No human reviewer has approved the claim scope yet.

## Validation

Filled in from the final matrix run on the head commit; see the PR description.
