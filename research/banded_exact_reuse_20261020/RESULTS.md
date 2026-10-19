# Banded improvement: executed results

Run `39304607635`; candidate `f8e89b70528da8490fbe1febf0b0ede9ff84a01e`; parent numerical library `b56bb19ce85bc28a021138ee29689701a81bc4f0`.

The baseline uses the common new study driver copied into the historical numerical-library worktree. No older-host times are pooled. Default Indexed/Slices selection and Jacobian/RHS callback frequency are unchanged.

## Bounded results

```json
{
  "accuracy_rows": 432,
  "comparisons": 36,
  "complete_comparisons": 36,
  "default_kernel_changed": false,
  "jacobian_calls_skipped": false,
  "parity_pairs": 324,
  "parity_pass": 324,
  "reference_certified": false,
  "state_compressed_bytes": 7959517,
  "state_objects": 87,
  "status": "COMPLETED_BOUNDED",
  "timed_integrations": 3744,
  "timing_authority": "diagnostic_shared_runner",
  "timing_processes": 576,
  "valid_timing_processes": 576
}
```

## Tests and remaining whole-workspace limit

```json
{
  "regressions": {
    "completed_targets": 1,
    "passed_in_completed_targets": 9,
    "failed_in_completed_targets": 0,
    "ignored_in_completed_targets": 0,
    "failure_lines": [],
    "last_lines": [
      "test failed_banded_jacobian_rebuilds_snapshot ... ok",
      "test failed_banded_rhs_rebuilds_snapshot ... ok",
      "test failed_dense_jacobian_rebuilds_snapshot ... ok",
      "test failed_dense_rhs_rebuilds_snapshot ... ok",
      "test finite_inputs_with_overflowing_solution_are_rejected ... ok",
      "test nonfinite_rhs_is_rejected ... ok",
      "test nonfinite_upper_entry_is_not_a_successful_solution ... ok",
      "test size_arithmetic_overflow_is_a_typed_error_not_a_panic ... ok",
      "test tiny_but_representable_solution_is_not_over_rejected ... ok",
      "",
      "test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
      ""
    ]
  },
  "internal": {
    "completed_targets": 1,
    "passed_in_completed_targets": 17,
    "failed_in_completed_targets": 0,
    "ignored_in_completed_targets": 0,
    "failure_lines": [],
    "last_lines": [
      "test rodas5p_fast::exact_band_cache::tests::failed_record_invalidates ... ok",
      "test rodas5p_fast::exact_band_cache::tests::identical_matrix_hits ... ok",
      "test rodas5p_fast::exact_band_cache::tests::nonfinite_shift_misses ... ok",
      "test rodas5p_fast::exact_band_cache::tests::shape_change_misses ... ok",
      "test rodas5p_fast::exact_band_cache::tests::signed_zero_is_conservative ... ok",
      "test rodas5p_fast::tests::in_place_lu_solves_pivoted_and_banded_systems ... ok",
      "test rodas5p_fast_small::tests::every_small_option_set_has_its_own_driver_id ... ok",
      "test rodas5p_fast_small::tests::the_construction_check_refuses_a_table_off_by_one_bit ... ok",
      "test rodas5p_fast_small::tests::the_indexed_solve_equals_the_slice_solve_bitwise ... ok",
      "",
      "test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.02s",
      ""
    ]
  },
  "workspace": {
    "completed_targets": 14,
    "passed_in_completed_targets": 142,
    "failed_in_completed_targets": 0,
    "ignored_in_completed_targets": 33,
    "failure_lines": [
      "error: invalid value 'unknown-arm' for '--arm <ARM>'",
      "error: invalid value 'unknown-family' for '--family <FAMILY>'",
      "error: unexpected argument '--profile' found"
    ],
    "last_lines": [
      "test stiff_benchmark::spd01::spd03_band_fill_equals_dense_fill ... ok",
      "test stiff_benchmark::spd01::spd03_identity_export ... ignored, identity export of research/spd03_banded_arm_instructions_20261005; release build; set SPD03_IDENTITY",
      "test stiff_benchmark::spd06::the_static_arms_are_known_and_equal_the_small_arm ... ok",
      "test stiff_benchmark::spd09::colext64_equals_the_legacy_driver_on_both_sides_of_the_threshold ... ok",
      "test stiff_benchmark::spd09::spd09_identity_export ... ignored, identity export of research/spd09_colext_threshold_20261007; release build; set SPD09_IDENTITY",
      "test stiff_benchmark::tests::analytic_jacobians_match_finite_differences ... ok",
      "test stiff_benchmark::tests::every_arm_completes_a_loose_hires_run ... ok",
      "test stiff_benchmark::tests::in_place_jacobians_equal_the_explicit_ones ... ok",
      "test triage_list_parses_with_unique_ids ... ok",
      "test usage_errors_exit_with_code_2 ... ok",
      "test wired_arms_have_distinct_component_sets ... ok",
      "test work_counter_fields_include_the_skipped_ones ... ok"
    ]
  },
  "workspace_status": {
    "exit_code": 124,
    "timeout_seconds": 660,
    "command": "cargo +1.94.1 test --workspace --no-fail-fast --locked -- --test-threads=2"
  },
  "workspace_reuse": {
    "source_run": 39301232025,
    "numerical_source": "b51c051b045e977c038e5eec468de0004840872b",
    "relation": "BYTE_IDENTITY of crates and root Cargo files",
    "status": "TIMEOUT124_PRESERVED_NOT_RERUN"
  },
  "readback": {
    "status": "PASS_SCOPED_READBACK",
    "checked": {
      "accuracy": 432,
      "recomputed_metric": 1008,
      "timing": 576,
      "timing_raw_link": 576,
      "same_output_parity": 324,
      "paired_recomputed": 36
    },
    "verified_state_objects": 87,
    "independence": "different metric implementation; same agent authorship, not independent personnel",
    "uncertified": "continuous time error, covariance of shared-host noise, general performance ranking"
  },
  "verified_files": 122,
  "scope": "scoped validation; full workspace timeout remains NOT_PASS"
}
```

The workspace timeout receipt is from run39301232025 and reused only after proving byte identity of all crates and root Cargo files. It is NOT a full-suite PASS.

## Per-call time and actual factor counts

|Case|Grid|Arm|ms|Attempts|LU|Reuse|Jacobian|E_empirical|
|---|---|---|---:|---:|---:|---:|---:|---:|
|brusselator-50|endpoint|parent-slices|1.99945|92|92|0|82|8.747002069411406e-07|
|brusselator-50|endpoint|patched-slices|1.9542305|92|92|0|82|8.747002069411406e-07|
|brusselator-50|endpoint|fixed2|1.89271|92|92|0|82|8.747002069411406e-07|
|brusselator-50|endpoint|exact-cache|1.950679|92|92|0|82|8.747002069411406e-07|
|brusselator-50|uniform101|parent-slices|3.0101355|136|136|0|133|4.759857861603312e-07|
|brusselator-50|uniform101|patched-slices|2.9541455|136|136|0|133|4.759857861603312e-07|
|brusselator-50|uniform101|fixed2|2.858728|136|136|0|133|4.759857861603312e-07|
|brusselator-50|uniform101|exact-cache|2.9239945|136|136|0|133|4.759857861603312e-07|
|brusselator-127|endpoint|parent-slices|5.011705|92|92|0|82|8.679480848155537e-07|
|brusselator-127|endpoint|patched-slices|4.9330735|92|92|0|82|8.679480848155537e-07|
|brusselator-127|endpoint|fixed2|4.762138|92|92|0|82|8.679480848155537e-07|
|brusselator-127|endpoint|exact-cache|4.8694285|92|92|0|82|8.679480848155537e-07|
|brusselator-127|uniform101|parent-slices|7.4842545|136|136|0|133|4.988546517543623e-07|
|brusselator-127|uniform101|patched-slices|7.3719845|136|136|0|133|4.988546517543623e-07|
|brusselator-127|uniform101|fixed2|7.1171325|136|136|0|133|4.988546517543623e-07|
|brusselator-127|uniform101|exact-cache|7.3031305|136|136|0|133|4.988546517543623e-07|
|brusselator-200|endpoint|parent-slices|7.851086|92|92|0|82|8.650727777356477e-07|
|brusselator-200|endpoint|patched-slices|7.718258|92|92|0|82|8.650727777356477e-07|
|brusselator-200|endpoint|fixed2|7.485703|92|92|0|82|8.650727777356477e-07|
|brusselator-200|endpoint|exact-cache|7.6511665|92|92|0|82|8.650727777356477e-07|
|brusselator-200|uniform101|parent-slices|11.8616235|137|137|0|134|5.059981556792367e-07|
|brusselator-200|uniform101|patched-slices|11.7034365|137|137|0|134|5.059981556792367e-07|
|brusselator-200|uniform101|fixed2|11.302102|137|137|0|134|5.059981556792367e-07|
|brusselator-200|uniform101|exact-cache|11.5675985|137|137|0|134|5.059981556792367e-07|
|linear-diffusion-50|endpoint|parent-slices|0.21201875|10|10|0|10|1.1938648337315047e-09|
|linear-diffusion-50|endpoint|patched-slices|0.2092365|10|10|0|10|1.1938648337315047e-09|
|linear-diffusion-50|endpoint|fixed2|0.20297225|10|10|0|10|1.1938648337315047e-09|
|linear-diffusion-50|endpoint|exact-cache|0.20804175|10|10|0|10|1.1938648337315047e-09|
|linear-diffusion-50|dyadic129|parent-slices|2.875371|134|134|0|134|1.5749470388193078e-13|
|linear-diffusion-50|dyadic129|patched-slices|2.83365275|134|134|0|134|1.5749470388193078e-13|
|linear-diffusion-50|dyadic129|fixed2|2.74884425|134|134|0|134|1.5749470388193078e-13|
|linear-diffusion-50|dyadic129|exact-cache|2.5710845|134|8|126|134|1.5749470388193078e-13|
|linear-diffusion-127|endpoint|parent-slices|0.53539625|10|10|0|10|1.1941170666674514e-09|
|linear-diffusion-127|endpoint|patched-slices|0.528674|10|10|0|10|1.1941170666674514e-09|
|linear-diffusion-127|endpoint|fixed2|0.5121755|10|10|0|10|1.1941170666674514e-09|
|linear-diffusion-127|endpoint|exact-cache|0.52407275|10|10|0|10|1.1941170666674514e-09|
|linear-diffusion-127|dyadic129|parent-slices|7.20775275|134|134|0|134|1.581382097667683e-13|
|linear-diffusion-127|dyadic129|patched-slices|7.0923695|134|134|0|134|1.581382097667683e-13|
|linear-diffusion-127|dyadic129|fixed2|6.88841275|134|134|0|134|1.581382097667683e-13|
|linear-diffusion-127|dyadic129|exact-cache|6.43920825|134|8|126|134|1.581382097667683e-13|
|linear-diffusion-200|endpoint|parent-slices|0.84341525|10|10|0|10|1.19324161873543e-09|
|linear-diffusion-200|endpoint|patched-slices|0.82893825|10|10|0|10|1.19324161873543e-09|
|linear-diffusion-200|endpoint|fixed2|0.80473075|10|10|0|10|1.19324161873543e-09|
|linear-diffusion-200|endpoint|exact-cache|0.82548675|10|10|0|10|1.19324161873543e-09|
|linear-diffusion-200|dyadic129|parent-slices|11.34740675|134|134|0|134|1.5829050411646541e-13|
|linear-diffusion-200|dyadic129|patched-slices|11.2135955|134|134|0|134|1.5829050411646541e-13|
|linear-diffusion-200|dyadic129|fixed2|10.8461625|134|134|0|134|1.5829050411646541e-13|
|linear-diffusion-200|dyadic129|exact-cache|10.16744325|134|8|126|134|1.5829050411646541e-13|

## Paired parent/candidate ratios

A ratio above1 means shorter candidate time. Family intervals allocate Bonferroni error across36comparisons;12blocks and shared-host autocorrelation still limit inference.

|Case|Grid|Candidate|Ratio|Family interval|
|---|---|---|---:|---|
|brusselator-50|endpoint|patched-slices|1.02175|[1.01781, 1.03877]|
|brusselator-50|endpoint|fixed2|1.05757|[1.05166, 1.07776]|
|brusselator-50|endpoint|exact-cache|1.02982|[1.01881, 1.04184]|
|brusselator-50|uniform101|patched-slices|1.01897|[1.01132, 1.02471]|
|brusselator-50|uniform101|fixed2|1.05508|[1.04954, 1.05834]|
|brusselator-50|uniform101|exact-cache|1.02920|[1.02013, 1.03393]|
|brusselator-127|endpoint|patched-slices|1.01706|[1.00834, 1.01936]|
|brusselator-127|endpoint|fixed2|1.05044|[1.04775, 1.05554]|
|brusselator-127|endpoint|exact-cache|1.02844|[1.01949, 1.03353]|
|brusselator-127|uniform101|patched-slices|1.01447|[1.00144, 1.02025]|
|brusselator-127|uniform101|fixed2|1.05246|[1.04769, 1.05468]|
|brusselator-127|uniform101|exact-cache|1.02424|[1.01248, 1.03057]|
|brusselator-200|endpoint|patched-slices|1.01757|[1.01290, 1.02053]|
|brusselator-200|endpoint|fixed2|1.05026|[1.04613, 1.05694]|
|brusselator-200|endpoint|exact-cache|1.02557|[1.02101, 1.03391]|
|brusselator-200|uniform101|patched-slices|1.01444|[1.00973, 1.01848]|
|brusselator-200|uniform101|fixed2|1.04978|[1.04311, 1.05219]|
|brusselator-200|uniform101|exact-cache|1.02562|[1.02101, 1.02847]|
|linear-diffusion-50|endpoint|patched-slices|1.01536|[1.00814, 1.02205]|
|linear-diffusion-50|endpoint|fixed2|1.04553|[1.04046, 1.05386]|
|linear-diffusion-50|endpoint|exact-cache|1.02190|[1.01707, 1.02655]|
|linear-diffusion-50|dyadic129|patched-slices|1.01328|[1.01126, 1.01865]|
|linear-diffusion-50|dyadic129|fixed2|1.04589|[1.04228, 1.05006]|
|linear-diffusion-50|dyadic129|exact-cache|1.11830|[1.11329, 1.12043]|
|linear-diffusion-127|endpoint|patched-slices|1.01388|[1.00390, 1.06001]|
|linear-diffusion-127|endpoint|fixed2|1.04566|[1.03643, 1.09280]|
|linear-diffusion-127|endpoint|exact-cache|1.01789|[1.01033, 1.02844]|
|linear-diffusion-127|dyadic129|patched-slices|1.01670|[1.01112, 1.02235]|
|linear-diffusion-127|dyadic129|fixed2|1.04612|[1.04251, 1.05154]|
|linear-diffusion-127|dyadic129|exact-cache|1.11808|[1.10632, 1.12377]|
|linear-diffusion-200|endpoint|patched-slices|1.01546|[1.01136, 1.02010]|
|linear-diffusion-200|endpoint|fixed2|1.04639|[1.04148, 1.05316]|
|linear-diffusion-200|endpoint|exact-cache|1.02043|[1.01790, 1.02500]|
|linear-diffusion-200|dyadic129|patched-slices|1.01553|[1.01013, 1.01762]|
|linear-diffusion-200|dyadic129|fixed2|1.04669|[1.04079, 1.05274]|
|linear-diffusion-200|dyadic129|exact-cache|1.11647|[1.11105, 1.12349]|

## Preserved negative evidence

The initial retry-invalidity suspicion was refuted by baseline tests and the complete Err branch; no recovery-policy patch was made.
Four standalone solve regressions were real: nonfinite upper matrix/RHS, unrepresentable solution overflow, and unchecked size arithmetic. These fail on the original and pass after the correction; a tiny but representable solve remains admitted.
The first432benchmark attempts failed before integration because the new driver omitted the derivative capability required by OdeProblem::new. Those are adapter-construction failures, not numerical divergence; the repaired common JVP adapter and preflight smoke were then used for both arms.
This exact cache does not freeze an approximate Jacobian. It requires bit-identical rounded shift and finite Jacobian entries, adds storage/copy checks, and may provide no benefit on changing nonlinear operators. Interpret actual cache hits and costs together.
The empirical reference radius is not a continuous-time certificate. Identical outputs establish no extra observed numerical error relative to the old implementation, not universal accuracy.
No new DVODE/Diffsol comparison, external-client compatibility proof, independent-personnel review or production-default promotion is claimed. BandedKernel gained enum variants, so external exhaustive matches may require updates.
