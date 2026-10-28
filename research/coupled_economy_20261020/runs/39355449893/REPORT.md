# Residual-first coupled-step economy

Measured source `f08d152a35fb2db748692570969cb1723c287c26`.

This is fixed-h complete-root attempt cost. Original raw-U/full-space and embedded tests are unchanged. Root success is not physical timestep acceptance. All failed work remains visible.

```json
{
  "accuracy_rows": 80,
  "admitted_timing_processes": 608,
  "comparison_count": 50,
  "complete_comparisons": 50,
  "continuous_error_certificate": false,
  "exact_output_parity_pairs": 27,
  "new_adaptive_integrator": false,
  "reference_admitted": 76,
  "root_and_embedded_admitted": 48,
  "state_compressed_bytes": 751457,
  "state_objects": 71,
  "status": "COMPLETE_BOUNDED",
  "timed_attempts": 1920,
  "timing_processes": 640,
  "unchanged_default_eager_api": true,
  "valid_timed_processes": 640
}
```

|Case|Arm|Reference gate|Embedded test|ms|JVP|Avoided JVP|Derivative batches|Fallback|
|---|---|---|---|---:|---:|---:|---:|---|
|few-mode_n96_eps1_h0.001|eager-1|True|True|0.1449175|59|0|0|False|
|few-mode_n96_eps1_h0.001|eager-4|True|True|0.166981|59|0|0|False|
|few-mode_n96_eps1_h0.001|lazy-1|True|True|0.119875|35|24|1|False|
|few-mode_n96_eps1_h0.001|lazy-4|True|True|0.153661|35|24|1|False|
|few-mode_n96_eps1_h0.001|sequential-mf|True|True|0.1194445||||False|
|few-mode_n96_eps1_h0.001|txn|True|True|0.1531|35|24|1|False|
|few-mode_n96_eps1_h0.001|cap64|True|True|0.15302|35|24|1|False|
|few-mode_n96_eps1_h0.001|cap256|True|True|0.154092|35|24|1|False|
|few-mode_n384_eps1_h0.001|eager-1|True|True|0.49593|59|0|0|False|
|few-mode_n384_eps1_h0.001|eager-4|True|True|0.352815|59|0|0|False|
|few-mode_n384_eps1_h0.001|lazy-1|True|True|0.3943825|35|24|1|False|
|few-mode_n384_eps1_h0.001|lazy-4|True|True|0.341368|35|24|1|False|
|few-mode_n384_eps1_h0.001|sequential-mf|True|True|0.2877675||||False|
|few-mode_n384_eps1_h0.001|txn|True|True|0.3435965|35|24|1|False|
|few-mode_n384_eps1_h0.001|cap64|True|True|0.337252|35|24|1|False|
|few-mode_n384_eps1_h0.001|cap256|True|True|0.3353845|35|24|1|False|
|few-mode_n12288_eps1_h0.001|eager-1|True|True|18.789624|59|0|0|False|
|few-mode_n12288_eps1_h0.001|eager-4|True|True|8.997115|59|0|0|False|
|few-mode_n12288_eps1_h0.001|lazy-1|True|True|13.2861555|35|24|1|False|
|few-mode_n12288_eps1_h0.001|lazy-4|True|True|7.7376035|35|24|1|False|
|few-mode_n12288_eps1_h0.001|sequential-mf|True|True|7.709782||||False|
|few-mode_n12288_eps1_h0.001|txn|True|True|7.786738|35|24|1|False|
|few-mode_n12288_eps1_h0.001|cap64|True|True|7.7751155|35|24|1|False|
|few-mode_n12288_eps1_h0.001|cap256|True|True|7.805336|35|24|1|False|
|few-mode_n12288_eps10_h0.01|eager-1|True|False|16.609805|58|0|0|False|
|few-mode_n12288_eps10_h0.01|eager-4|True|False|8.383815|58|0|0|False|
|few-mode_n12288_eps10_h0.01|lazy-1|True|False|13.0988865|42|16|2|False|
|few-mode_n12288_eps10_h0.01|lazy-4|True|False|7.4953945|42|16|2|False|
|few-mode_n12288_eps10_h0.01|sequential-mf|True|False|6.489954||||False|
|few-mode_n12288_eps10_h0.01|txn|True|False|7.457658|42|16|2|False|
|few-mode_n12288_eps10_h0.01|cap64|True|False|7.489039|42|16|2|False|
|few-mode_n12288_eps10_h0.01|cap256|True|False|7.5502075|42|16|2|False|
|chain_n96_eps1_h0.001|eager-1|True|True|0.6212385|144|0|0|False|
|chain_n96_eps1_h0.001|eager-4|True|True|0.450667|144|0|0|False|
|chain_n96_eps1_h0.001|lazy-1|True|True|0.451909|80|64|1|False|
|chain_n96_eps1_h0.001|lazy-4|True|True|0.442575|80|64|1|False|
|chain_n96_eps1_h0.001|sequential-mf|True|True|5.886292||||False|
|chain_n96_eps1_h0.001|txn|True|True|0.433782|80|64|1|False|
|chain_n96_eps1_h0.001|cap64|True|True|6.198298|8|0|0|True|
|chain_n96_eps1_h0.001|cap256|True|True|0.4270615|80|64|1|False|
|chain_n384_eps1_h0.001|eager-1|True|True|2.2082135|144|0|0|False|
|chain_n384_eps1_h0.001|eager-4|True|True|1.1579655|144|0|0|False|
|chain_n384_eps1_h0.001|lazy-1|True|True|1.519959|80|64|1|False|
|chain_n384_eps1_h0.001|lazy-4|True|True|0.993368|80|64|1|False|
|chain_n384_eps1_h0.001|sequential-mf|True|True|10.279259||||False|
|chain_n384_eps1_h0.001|txn|True|True|1.0023865|80|64|1|False|
|chain_n384_eps1_h0.001|cap64|True|True|10.754173|8|0|0|True|
|chain_n384_eps1_h0.001|cap256|True|True|0.995802|80|64|1|False|
|diffusion_n96_eps1_h0.001|eager-1|True|True|2.109009|348|0|0|False|
|diffusion_n96_eps1_h0.001|eager-4|True|True|1.506539|348|0|0|False|
|diffusion_n96_eps1_h0.001|lazy-1|True|True|1.595137|188|160|2|False|
|diffusion_n96_eps1_h0.001|lazy-4|True|True|1.4111705|188|160|2|False|
|diffusion_n96_eps1_h0.001|sequential-mf|True|True|5.9093115||||False|
|diffusion_n96_eps1_h0.001|txn|True|True|1.425858|188|160|2|False|
|diffusion_n96_eps1_h0.001|cap64|True|True|6.1814405|8|0|0|True|
|diffusion_n96_eps1_h0.001|cap256|True|True|1.422187|188|160|2|False|
|diffusion_n384_eps1_h0.001|eager-1|True|False|7.081587|348|0|0|False|
|diffusion_n384_eps1_h0.001|eager-4|True|False|3.672809|348|0|0|False|
|diffusion_n384_eps1_h0.001|lazy-1|True|False|4.818066|188|160|2|False|
|diffusion_n384_eps1_h0.001|lazy-4|True|False|3.1194375|188|160|2|False|
|diffusion_n384_eps1_h0.001|sequential-mf|True|False|10.3975045||||False|
|diffusion_n384_eps1_h0.001|txn|True|False|3.123615|188|160|2|False|
|diffusion_n384_eps1_h0.001|cap64|True|False|10.9256905|8|0|0|True|
|diffusion_n384_eps1_h0.001|cap256|True|False|3.123624|188|160|2|False|
|diffusion_n96_eps1_h0.01|eager-1|True|False|6.5959665|904|0|0|False|
|diffusion_n96_eps1_h0.01|eager-4|True|False|4.4781705|904|0|0|False|
|diffusion_n96_eps1_h0.01|lazy-1|True|False|5.2860395|616|288|6|False|
|diffusion_n96_eps1_h0.01|lazy-4|True|False|4.4072835|616|288|6|False|
|diffusion_n96_eps1_h0.01|sequential-mf|True|False|5.931308||||False|
|diffusion_n96_eps1_h0.01|txn|True|False|4.3957965|616|288|6|False|
|diffusion_n96_eps1_h0.01|cap64|True|False|6.191736|8|0|0|True|
|diffusion_n96_eps1_h0.01|cap256|True|False|7.731703|244|64|3|True|
|broad_n64_eps0_h0.02|eager-1|False||6.0976645|704|0|0|False|
|broad_n64_eps0_h0.02|eager-4|False||4.6743695|704|0|0|False|
|broad_n64_eps0_h0.02|lazy-1|False||3.3662035|64|640|0|False|
|broad_n64_eps0_h0.02|lazy-4|False||3.767347|64|640|0|False|
|broad_n64_eps0_h0.02|sequential-mf|True|False|4.8740705||||False|
|broad_n64_eps0_h0.02|txn|True|False|8.861882|64|640|0|True|
|broad_n64_eps0_h0.02|cap64|True|False|8.831947|64|640|0|True|
|broad_n64_eps0_h0.02|cap256|True|False|8.8704195|64|640|0|True|

## Paired ratios: baseline/candidate

Intervals allocate Bonferroni budget over50comparisons;8shared-host blocks do not establish universal performance or certified coverage. Failed-attempt cost is explicitly not solver speed. Different-target threshold comparisons are not exact matched-accuracy claims.

|Case|Baseline|Candidate|Ratio|Family interval|Relation|
|---|---|---|---:|---|---|
|few-mode_n96_eps1_h0.001|eager-1|lazy-1|1.2073|1.1959, 1.2576|exact_output_bits|
|few-mode_n96_eps1_h0.001|eager-4|lazy-4|1.0897|0.8788, 1.1770|exact_output_bits|
|few-mode_n96_eps1_h0.001|lazy-1|lazy-4|0.7872|0.6641, 0.8136|exact_output_bits|
|few-mode_n96_eps1_h0.001|txn|cap64|0.9894|0.9352, 1.1144|exact_output_bits|
|few-mode_n96_eps1_h0.001|txn|cap256|1.0118|0.7730, 1.1846|exact_output_bits|
|few-mode_n384_eps1_h0.001|eager-1|lazy-1|1.2606|1.1410, 1.3306|exact_output_bits|
|few-mode_n384_eps1_h0.001|eager-4|lazy-4|1.0354|0.9686, 1.0829|exact_output_bits|
|few-mode_n384_eps1_h0.001|lazy-1|lazy-4|1.1572|1.0717, 1.2677|exact_output_bits|
|few-mode_n384_eps1_h0.001|txn|cap64|1.0128|0.9494, 1.0535|exact_output_bits|
|few-mode_n384_eps1_h0.001|txn|cap256|1.0031|0.9737, 1.0626|exact_output_bits|
|few-mode_n12288_eps1_h0.001|eager-1|lazy-1|1.4117|1.3969, 1.4241|exact_output_bits|
|few-mode_n12288_eps1_h0.001|eager-4|lazy-4|1.1656|1.1187, 1.1921|exact_output_bits|
|few-mode_n12288_eps1_h0.001|lazy-1|lazy-4|1.7196|1.6546, 1.7496|exact_output_bits|
|few-mode_n12288_eps1_h0.001|txn|cap64|0.9983|0.9602, 1.0581|exact_output_bits|
|few-mode_n12288_eps1_h0.001|txn|cap256|0.9917|0.9677, 1.0441|exact_output_bits|
|few-mode_n12288_eps10_h0.01|eager-1|lazy-1|1.2649|1.2526, 1.2807|exact_output_bits|
|few-mode_n12288_eps10_h0.01|eager-4|lazy-4|1.1154|1.0967, 1.1395|exact_output_bits|
|few-mode_n12288_eps10_h0.01|lazy-1|lazy-4|1.7508|1.7006, 1.7646|exact_output_bits|
|few-mode_n12288_eps10_h0.01|txn|cap64|0.9929|0.9725, 1.0352|exact_output_bits|
|few-mode_n12288_eps10_h0.01|txn|cap256|0.9903|0.9603, 1.0273|exact_output_bits|
|chain_n96_eps1_h0.001|eager-1|lazy-1|1.3695|1.3265, 1.4091|exact_output_bits|
|chain_n96_eps1_h0.001|eager-4|lazy-4|1.0244|0.9797, 1.0583|exact_output_bits|
|chain_n96_eps1_h0.001|lazy-1|lazy-4|1.0255|0.9958, 1.0442|exact_output_bits|
|chain_n96_eps1_h0.001|txn|cap64|0.0702|0.0673, 0.0737|reference_threshold_only_different_targets|
|chain_n96_eps1_h0.001|txn|cap256|1.0236|0.9364, 1.0556|exact_output_bits|
|chain_n384_eps1_h0.001|eager-1|lazy-1|1.4524|1.4361, 1.4641|exact_output_bits|
|chain_n384_eps1_h0.001|eager-4|lazy-4|1.1669|1.1396, 1.1826|exact_output_bits|
|chain_n384_eps1_h0.001|lazy-1|lazy-4|1.5264|1.5139, 1.5591|exact_output_bits|
|chain_n384_eps1_h0.001|txn|cap64|0.0931|0.0913, 0.0942|reference_threshold_only_different_targets|
|chain_n384_eps1_h0.001|txn|cap256|1.0070|0.9811, 1.0119|exact_output_bits|
|diffusion_n96_eps1_h0.001|eager-1|lazy-1|1.3222|1.3084, 1.3425|exact_output_bits|
|diffusion_n96_eps1_h0.001|eager-4|lazy-4|1.0696|1.0334, 1.0831|exact_output_bits|
|diffusion_n96_eps1_h0.001|lazy-1|lazy-4|1.1315|1.0897, 1.1518|exact_output_bits|
|diffusion_n96_eps1_h0.001|txn|cap64|0.2306|0.2278, 0.2332|reference_threshold_only_different_targets|
|diffusion_n96_eps1_h0.001|txn|cap256|0.9986|0.9879, 1.0121|exact_output_bits|
|diffusion_n384_eps1_h0.001|eager-1|lazy-1|1.4682|1.4579, 1.4759|exact_output_bits|
|diffusion_n384_eps1_h0.001|eager-4|lazy-4|1.1763|1.1707, 1.1890|exact_output_bits|
|diffusion_n384_eps1_h0.001|lazy-1|lazy-4|1.5488|1.5341, 1.5610|exact_output_bits|
|diffusion_n384_eps1_h0.001|txn|cap64|0.2868|0.2813, 0.2902|reference_threshold_only_different_targets|
|diffusion_n384_eps1_h0.001|txn|cap256|1.0037|0.9865, 1.0168|exact_output_bits|
|diffusion_n96_eps1_h0.01|eager-1|lazy-1|1.2499|1.2394, 1.2553|exact_output_bits|
|diffusion_n96_eps1_h0.01|eager-4|lazy-4|1.0133|1.0017, 1.0566|exact_output_bits|
|diffusion_n96_eps1_h0.01|lazy-1|lazy-4|1.1992|1.1894, 1.2184|exact_output_bits|
|diffusion_n96_eps1_h0.01|txn|cap64|0.7087|0.6906, 0.7200|reference_threshold_only_different_targets|
|diffusion_n96_eps1_h0.01|txn|cap256|0.5679|0.5592, 0.5748|reference_threshold_only_different_targets|
|broad_n64_eps0_h0.02|eager-1|lazy-1|1.8108|1.8069, 1.8238|failed_attempt_cost_not_solver_speed|
|broad_n64_eps0_h0.02|eager-4|lazy-4|1.2401|1.2346, 1.2547|failed_attempt_cost_not_solver_speed|
|broad_n64_eps0_h0.02|lazy-1|lazy-4|0.8941|0.8848, 0.9016|failed_attempt_cost_not_solver_speed|
|broad_n64_eps0_h0.02|txn|cap64|1.0031|0.9969, 1.0135|exact_output_bits|
|broad_n64_eps0_h0.02|txn|cap256|0.9990|0.9818, 1.0101|exact_output_bits|

## Readback

```json
{
  "status": "PASS_SCOPED_READBACK",
  "checks": {
    "accuracy": 80,
    "metrics": 684,
    "work_ledgers": 630,
    "candidate_cap": 270,
    "failures_preserved": 36,
    "timing": 640,
    "timing_identity": 608,
    "exact_saved_work": 18,
    "full_stage_bit_parity": 27,
    "paired_medians": 50
  },
  "state_objects_verified": 71,
  "author_independence": "different metric path, same author; no independent-personnel claim",
  "unclaimed": [
    "continuous/global error certificate",
    "whole-workspace tests",
    "optimal budget or trajectory routing",
    "rigorous bootstrap coverage under shared-host dependence"
  ]
}
```

An explicit JVP cap limits only attempted candidate JVP work, not projected-LU time or total fallback time. It can refuse otherwise successful coupled candidates. The64/256values are predeclared sensitivity examples, not optimal thresholds.

Default eager API remains unchanged. Residual-first policy adds parallel dispatches, saves only unused derivatives and changes callback ordering. Callbacks must be pure at a frozen epoch. The existing prior19tests are rerun separately alongside new tests; no CodeRabbit or whole-repository PASS is inferred.
