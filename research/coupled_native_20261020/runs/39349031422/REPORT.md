# Native coupled-step benchmark

## Measured source

`1e4a993874d6930ea2bb41bb0ff7392f21c33df7`

Fixed-h complete-step root latency. All candidate/basis/corrector/full-residual work is timed; pool setup, references, JSON and result destruction are outside the timer. This is not a full-trajectory benchmark.

```json
{
  "accuracy_admitted": 77,
  "accuracy_rows": 80,
  "admitted_timing_processes": 770,
  "by_arm": {
    "coupled-1": {
      "reference_admitted": 15,
      "root_completed": 15
    },
    "coupled-2": {
      "reference_admitted": 15,
      "root_completed": 15
    },
    "coupled-4": {
      "reference_admitted": 15,
      "root_completed": 15
    },
    "sequential-mf": {
      "reference_admitted": 16,
      "root_completed": 16
    },
    "transactional-4": {
      "reference_admitted": 16,
      "root_completed": 16
    }
  },
  "complete_pairs": 61,
  "no_certified_error_claim": true,
  "no_trajectory_speed_claim": true,
  "pairs": 64,
  "state_compressed_bytes": 931878,
  "state_objects": 82,
  "status": "COMPLETE_BOUNDED",
  "timed_integrations": 3700,
  "timing_processes": 800
}
```

|Case|Arm|Root complete|Reference gate|ms|Basis|Peak concurrent stage tasks|
|---|---|---|---|---:|---:|---:|
|few-mode_n96_eps1_h0.01|coupled-1|True|True|0.1279135|3|1|
|few-mode_n96_eps1_h0.01|coupled-2|True|True|0.1711135|3|2|
|few-mode_n96_eps1_h0.01|coupled-4|True|True|0.137097|3|4|
|few-mode_n96_eps1_h0.01|sequential-mf|True|True|0.0883085|||
|few-mode_n96_eps1_h0.01|transactional-4|True|True|0.137908|||
|few-mode_n96_eps10_h0.01|coupled-1|True|True|0.085229|2|1|
|few-mode_n96_eps10_h0.01|coupled-2|True|True|0.1224895|2|2|
|few-mode_n96_eps10_h0.01|coupled-4|True|True|0.1022845|2|4|
|few-mode_n96_eps10_h0.01|sequential-mf|True|True|0.074788|||
|few-mode_n96_eps10_h0.01|transactional-4|True|True|0.09927|||
|few-mode_n384_eps1_h0.01|coupled-1|True|True|0.438222|3|1|
|few-mode_n384_eps1_h0.01|coupled-2|True|True|0.391721|3|2|
|few-mode_n384_eps1_h0.01|coupled-4|True|True|0.3225665|3|4|
|few-mode_n384_eps1_h0.01|sequential-mf|True|True|0.215896|||
|few-mode_n384_eps1_h0.01|transactional-4|True|True|0.3198025|||
|few-mode_n384_eps10_h0.01|coupled-1|True|True|0.2832225|2|1|
|few-mode_n384_eps10_h0.01|coupled-2|True|True|0.2678445|2|2|
|few-mode_n384_eps10_h0.01|coupled-4|True|True|0.2209435|2|4|
|few-mode_n384_eps10_h0.01|sequential-mf|True|True|0.183172|||
|few-mode_n384_eps10_h0.01|transactional-4|True|True|0.218845|||
|few-mode_n12288_eps1_h0.01|coupled-1|True|True|16.449693|3|1|
|few-mode_n12288_eps1_h0.01|coupled-2|True|True|8.6955295|3|2|
|few-mode_n12288_eps1_h0.01|coupled-4|True|True|7.8599935|3|4|
|few-mode_n12288_eps1_h0.01|sequential-mf|True|True|5.5550245|||
|few-mode_n12288_eps1_h0.01|transactional-4|True|True|7.8190515|||
|few-mode_n12288_eps10_h0.01|coupled-1|True|True|11.7429295|2|1|
|few-mode_n12288_eps10_h0.01|coupled-2|True|True|5.8427735|2|2|
|few-mode_n12288_eps10_h0.01|coupled-4|True|True|5.983341|2|4|
|few-mode_n12288_eps10_h0.01|sequential-mf|True|True|4.785588|||
|few-mode_n12288_eps10_h0.01|transactional-4|True|True|5.9057445|||
|few-mode_n96_eps1_h0.001|coupled-1|True|True|0.094713|3|1|
|few-mode_n96_eps1_h0.001|coupled-2|True|True|0.1325295|3|2|
|few-mode_n96_eps1_h0.001|coupled-4|True|True|0.104152|3|4|
|few-mode_n96_eps1_h0.001|sequential-mf|True|True|0.088063|||
|few-mode_n96_eps1_h0.001|transactional-4|True|True|0.109465|||
|few-mode_n384_eps1_h0.001|coupled-1|True|True|0.322617|3|1|
|few-mode_n384_eps1_h0.001|coupled-2|True|True|0.29456|3|2|
|few-mode_n384_eps1_h0.001|coupled-4|True|True|0.2448295|3|4|
|few-mode_n384_eps1_h0.001|sequential-mf|True|True|0.21505|||
|few-mode_n384_eps1_h0.001|transactional-4|True|True|0.245801|||
|few-mode_n12288_eps1_h0.001|coupled-1|True|True|12.8465455|3|1|
|few-mode_n12288_eps1_h0.001|coupled-2|True|True|6.587397|3|2|
|few-mode_n12288_eps1_h0.001|coupled-4|True|True|6.172241|3|4|
|few-mode_n12288_eps1_h0.001|sequential-mf|True|True|5.53202|||
|few-mode_n12288_eps1_h0.001|transactional-4|True|True|6.199362|||
|broad_n64_eps0_h0.02|coupled-1|False|False|3.7107555||1|
|broad_n64_eps0_h0.02|coupled-2|False|False|3.1990835||2|
|broad_n64_eps0_h0.02|coupled-4|False|False|3.105232||4|
|broad_n64_eps0_h0.02|sequential-mf|True|True|3.3014025|||
|broad_n64_eps0_h0.02|transactional-4|True|True|6.616972|||
|chain_n96_eps1_h0.01|coupled-1|True|True|0.5388235|8|1|
|chain_n96_eps1_h0.01|coupled-2|True|True|0.498833|8|2|
|chain_n96_eps1_h0.01|coupled-4|True|True|0.408512|8|4|
|chain_n96_eps1_h0.01|sequential-mf|True|True|3.882986|||
|chain_n96_eps1_h0.01|transactional-4|True|True|0.4154065|||
|chain_n384_eps1_h0.01|coupled-1|True|True|1.8822415|8|1|
|chain_n384_eps1_h0.01|coupled-2|True|True|1.286772|8|2|
|chain_n384_eps1_h0.01|coupled-4|True|True|0.9840205|8|4|
|chain_n384_eps1_h0.01|sequential-mf|True|True|6.710258|||
|chain_n384_eps1_h0.01|transactional-4|True|True|0.9889175|||
|diffusion_n96_eps1_h0.001|coupled-1|True|True|1.2842785|12|1|
|diffusion_n96_eps1_h0.001|coupled-2|True|True|1.09034|12|2|
|diffusion_n96_eps1_h0.001|coupled-4|True|True|0.9723725|12|4|
|diffusion_n96_eps1_h0.001|sequential-mf|True|True|3.926156|||
|diffusion_n96_eps1_h0.001|transactional-4|True|True|0.9870995|||
|diffusion_n96_eps1_h0.01|coupled-1|True|True|3.9974035|16|1|
|diffusion_n96_eps1_h0.01|coupled-2|True|True|3.312625|16|2|
|diffusion_n96_eps1_h0.01|coupled-4|True|True|2.8876835|16|4|
|diffusion_n96_eps1_h0.01|sequential-mf|True|True|3.917633|||
|diffusion_n96_eps1_h0.01|transactional-4|True|True|2.9040335|||
|diffusion_n384_eps1_h0.001|coupled-1|True|True|4.23341|12|1|
|diffusion_n384_eps1_h0.001|coupled-2|True|True|2.923898|12|2|
|diffusion_n384_eps1_h0.001|coupled-4|True|True|2.293847|12|4|
|diffusion_n384_eps1_h0.001|sequential-mf|True|True|6.8210255|||
|diffusion_n384_eps1_h0.001|transactional-4|True|True|2.2899415|||
|diffusion_n384_eps1_h0.01|coupled-1|True|True|12.964167|16|1|
|diffusion_n384_eps1_h0.01|coupled-2|True|True|8.638934|16|2|
|diffusion_n384_eps1_h0.01|coupled-4|True|True|6.5277925|16|4|
|diffusion_n384_eps1_h0.01|sequential-mf|True|True|6.841987|||
|diffusion_n384_eps1_h0.01|transactional-4|True|True|6.5144225|||

## Paired cost ratios

Baseline/candidate; >1 means lower candidate time. Worker-count comparisons have checked identical output bits. Sequential comparisons pass the same raw-reference threshold but the native-K and raw-U targets are not declared identical. Neither guarantees matched continuous-time accuracy.

|Case|Baseline|Candidate|Ratio|Family-adjusted interval|
|---|---|---|---:|---|
|few-mode_n96_eps1_h0.01|coupled-1|coupled-2|0.7471|0.7390, 0.7631|
|few-mode_n96_eps1_h0.01|coupled-1|coupled-4|0.9343|0.8132, 0.9689|
|few-mode_n96_eps1_h0.01|sequential-mf|coupled-4|0.6467|0.5632, 0.6735|
|few-mode_n96_eps1_h0.01|sequential-mf|transactional-4|0.6446|0.6046, 0.6643|
|few-mode_n96_eps10_h0.01|coupled-1|coupled-2|0.6941|0.6719, 0.7088|
|few-mode_n96_eps10_h0.01|coupled-1|coupled-4|0.8300|0.7901, 0.8769|
|few-mode_n96_eps10_h0.01|sequential-mf|coupled-4|0.7250|0.7038, 0.7704|
|few-mode_n96_eps10_h0.01|sequential-mf|transactional-4|0.7597|0.7066, 0.7904|
|few-mode_n384_eps1_h0.01|coupled-1|coupled-2|1.1250|1.1030, 1.1446|
|few-mode_n384_eps1_h0.01|coupled-1|coupled-4|1.3589|1.2773, 1.3819|
|few-mode_n384_eps1_h0.01|sequential-mf|coupled-4|0.6729|0.6308, 0.6854|
|few-mode_n384_eps1_h0.01|sequential-mf|transactional-4|0.6758|0.6620, 0.6863|
|few-mode_n384_eps10_h0.01|coupled-1|coupled-2|1.0536|0.7574, 1.0757|
|few-mode_n384_eps10_h0.01|coupled-1|coupled-4|1.2722|1.2459, 1.3286|
|few-mode_n384_eps10_h0.01|sequential-mf|coupled-4|0.8323|0.8169, 1.2096|
|few-mode_n384_eps10_h0.01|sequential-mf|transactional-4|0.8374|0.7816, 0.8539|
|few-mode_n12288_eps1_h0.01|coupled-1|coupled-2|1.9017|1.7080, 2.0277|
|few-mode_n12288_eps1_h0.01|coupled-1|coupled-4|2.0895|2.0497, 2.2425|
|few-mode_n12288_eps1_h0.01|sequential-mf|coupled-4|0.7024|0.6894, 0.7340|
|few-mode_n12288_eps1_h0.01|sequential-mf|transactional-4|0.7120|0.6933, 0.7254|
|few-mode_n12288_eps10_h0.01|coupled-1|coupled-2|2.0192|1.9505, 2.0536|
|few-mode_n12288_eps10_h0.01|coupled-1|coupled-4|1.9819|1.9296, 1.9949|
|few-mode_n12288_eps10_h0.01|sequential-mf|coupled-4|0.8023|0.7888, 0.8187|
|few-mode_n12288_eps10_h0.01|sequential-mf|transactional-4|0.8098|0.7922, 0.8311|
|few-mode_n96_eps1_h0.001|coupled-1|coupled-2|0.7106|0.6949, 0.7363|
|few-mode_n96_eps1_h0.001|coupled-1|coupled-4|0.9055|0.7519, 0.9596|
|few-mode_n96_eps1_h0.001|sequential-mf|coupled-4|0.8479|0.6963, 0.8956|
|few-mode_n96_eps1_h0.001|sequential-mf|transactional-4|0.8005|0.6101, 0.8650|
|few-mode_n384_eps1_h0.001|coupled-1|coupled-2|1.1077|1.0730, 1.1262|
|few-mode_n384_eps1_h0.001|coupled-1|coupled-4|1.3190|1.2858, 1.3393|
|few-mode_n384_eps1_h0.001|sequential-mf|coupled-4|0.8768|0.8521, 0.8855|
|few-mode_n384_eps1_h0.001|sequential-mf|transactional-4|0.8792|0.8302, 0.9038|
|few-mode_n12288_eps1_h0.001|coupled-1|coupled-2|1.9514|1.4882, 2.0044|
|few-mode_n12288_eps1_h0.001|coupled-1|coupled-4|2.0765|2.0202, 2.1531|
|few-mode_n12288_eps1_h0.001|sequential-mf|coupled-4|0.8966|0.8771, 0.9204|
|few-mode_n12288_eps1_h0.001|sequential-mf|transactional-4|0.8907|0.8713, 0.9177|
|broad_n64_eps0_h0.02|sequential-mf|transactional-4|0.5031|0.4940, 0.5081|
|chain_n96_eps1_h0.01|coupled-1|coupled-2|1.0708|0.9274, 1.0956|
|chain_n96_eps1_h0.01|coupled-1|coupled-4|1.3176|1.0229, 1.3429|
|chain_n96_eps1_h0.01|sequential-mf|coupled-4|9.4370|8.0531, 10.1586|
|chain_n96_eps1_h0.01|sequential-mf|transactional-4|9.4199|8.9725, 10.9316|
|chain_n384_eps1_h0.01|coupled-1|coupled-2|1.4479|0.9817, 1.4838|
|chain_n384_eps1_h0.01|coupled-1|coupled-4|1.9135|1.8683, 2.4887|
|chain_n384_eps1_h0.01|sequential-mf|coupled-4|6.8196|6.7017, 6.8792|
|chain_n384_eps1_h0.01|sequential-mf|transactional-4|6.8154|6.5861, 6.8853|
|diffusion_n96_eps1_h0.001|coupled-1|coupled-2|1.1877|1.1421, 1.5525|
|diffusion_n96_eps1_h0.001|coupled-1|coupled-4|1.3106|1.1819, 1.3468|
|diffusion_n96_eps1_h0.001|sequential-mf|coupled-4|3.9874|3.1984, 4.0927|
|diffusion_n96_eps1_h0.001|sequential-mf|transactional-4|3.9897|3.9194, 4.3144|
|diffusion_n96_eps1_h0.01|coupled-1|coupled-2|1.2072|1.1764, 1.2155|
|diffusion_n96_eps1_h0.01|coupled-1|coupled-4|1.3820|1.3564, 1.4009|
|diffusion_n96_eps1_h0.01|sequential-mf|coupled-4|1.3570|1.3397, 1.3754|
|diffusion_n96_eps1_h0.01|sequential-mf|transactional-4|1.3591|1.3242, 1.3733|
|diffusion_n384_eps1_h0.001|coupled-1|coupled-2|1.4522|1.4269, 1.4705|
|diffusion_n384_eps1_h0.001|coupled-1|coupled-4|1.8545|1.8146, 1.8754|
|diffusion_n384_eps1_h0.001|sequential-mf|coupled-4|2.9741|2.9317, 3.0215|
|diffusion_n384_eps1_h0.001|sequential-mf|transactional-4|2.9770|2.9494, 3.0145|
|diffusion_n384_eps1_h0.01|coupled-1|coupled-2|1.5170|1.4445, 1.5341|
|diffusion_n384_eps1_h0.01|coupled-1|coupled-4|1.9928|1.9744, 2.0255|
|diffusion_n384_eps1_h0.01|sequential-mf|coupled-4|1.0462|1.0387, 1.0544|
|diffusion_n384_eps1_h0.01|sequential-mf|transactional-4|1.0479|1.0393, 1.0550|

## Independent computational readback

```json
{
  "status": "PASS_SCOPED_RAW_READBACK",
  "checks": {
    "accuracy": 80,
    "recomputed_metrics": 847,
    "work_accounting": 528,
    "declined_preserved": 33,
    "timing": 800,
    "timing_calibration_identity": 770,
    "worker_count_bit_parity": 30,
    "paired_medians": 61
  },
  "verified_state_objects": 82,
  "accuracy_declines_by_kind": {
    "projection-leakage": 3
  },
  "independence": "different metric implementation; same author, NOT a CodeRabbit or independent personnel review",
  "not_verified": [
    "global trajectory error",
    "certified residual-to-output transport",
    "adaptive switching",
    "cross-platform speed",
    "bootstrap coverage under correlated host noise"
  ]
}
```

Small residual is not a nonlinear output certificate. References here solve the frozen step target, not the full physical trajectory. Matrix-free state operators can still lead to expensive reduced dense systems and repeated stage JVP work. The negative broad/stencil cases and failed speculative work are retained rather than excluded from cost accounting.
Tests and review availability are separate: a successful numerical readback is not a CodeRabbit review, and the standalone companion is not covered by a root-workspace test unless explicitly invoked.
