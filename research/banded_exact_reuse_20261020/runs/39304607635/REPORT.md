# Banded exact-reuse improvement measurements

This report is generated from retained new-run observations. No historical wall times are pooled.

## Scope and checks

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

```json
{
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
}
```

## Integration cost and actual factor counts

|Case|Grid|Arm|ms|Attempts|LU|Exact reuse|Jacobian|E_empirical|
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

## Parent/candidate paired ratios

Ratios >1 mean shorter candidate time. Conditional bootstrap intervals include a Bonferroni allocation for36registered comparisons, but shared-runner noise and12blocks limit coverage claims.

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

## Interpretation limits

Exact reuse does not reduce Jacobian evaluations. On changing nonlinear Jacobians it may add comparisons, copies and storage without any hit. No default kernel was changed. All old/new admitted observations are bit-identical, so this is implementation-cost comparison at identical observed accuracy, not evidence that the solver meets a continuous-time tolerance theorem.
The declared bandwidth2 specialization falls back to the existing Slices arithmetic for other bandwidths. Exact cache matching includes the rounded shift and all stored Jacobian entry bits; signed-zero changes conservatively miss. A failed factorization clears the key. The four pre-existing nonfinite/size regression failures are tested separately from speed.
No new Diffsol/DVODE binaries are timed in this node. The previously observed spatial performance gap is not claimed closed by ratios to a different host.
Whole-workspace test status is in WORKSPACE_STATUS.json and must not be inferred from the scoped audit.
