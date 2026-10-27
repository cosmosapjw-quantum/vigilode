# P2 — Deterministic decomposition of adapter work from driver records

Worktree `/home/cosmosapjw/vigilode-adapter-perf` at packet head `4fe67602`; prebuilt measurement-profile examples (no build). Binaries: evidence_switch_bench `ab812631e6f4…`, local_bottleneck_suite `12e66ce27610…`. Host AMD Ryzen 9 5900X 12-Core Processor, kernel 7.0.0-38-generic, `taskset -c 0`; protocol: warm-up 1 run + 5 timed runs per config; counters are deterministic WorkCounters (authority); wall time diagnostic_only.

Bench options: rtol 1e-7, atol 1e-10, h0 1e-7, span [0,1]. Suite options: rtol 1e-6, atol 1e-9, h0 min(1e-4,end/10), max_steps 100000, size standard, SwitchingConfig{ProvidedOnly, window_count 16, max_dense 256, pilot budget 32, pilot limit 4}. Bench evidence/screened-reset use `SwitchingConfig::evidence_driven` (pilot budget 64, limit 8, cooldown 4); pressure-only uses `SwitchingConfig::default()`.

Bucket definitions: **explicit committed** = dopri5 & committed; **explicit probe discarded** = dopri5 & !committed; **implicit committed (RODAS window)** = rodas5p & committed; **implicit pilot discarded** = rodas5p & !committed. Every switching run reconciles (sum of segment attempts/RHS/JVP/linear solves == stats totals) and every config's WorkCounters are bit-identical across the 6 runs.

## 1. Headline: switch vs direct (WorkCounters authority; time diagnostic_only)

| run | switch attempts | switch RHS | switch JVP | best direct | direct attempts | direct RHS | RHS ratio | switch median ms | switch min ms | direct median ms | direct min ms | time ratio (median) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rotation-evidence | 1046 | 6597 | 5246 | bench:rotation:dopri | 875 | 5251 | 1.26 | 2.179 | 2.045 | 0.225 | 0.221 | 9.67 |
| stiff-evidence | 137 | 1002 | 0 | bench:stiff-forced:rodas | 14 | 112 | 8.95 | 0.214 | 0.210 | 0.097 | 0.091 | 2.20 |
| case5-switching | 232 | 1633 | 3802 | suite:5:rodas5p-mf | 39 | 307 | 5.32 | 2.193 | 2.156 | 0.573 | 0.566 | 3.83 |
| case10-switching | 1321 | 8221 | 5120 | suite:10:dopri5 | 1105 | 6631 | 1.24 | 4.176 | 4.120 | 1.239 | 0.702 | 3.37 |
| rotation-screened-reset | 1274 | 8069 | 6312 | bench:rotation:dopri | 875 | 5251 | 1.54 | 2.434 | 2.410 | 0.225 | 0.221 | 10.80 |
| rotation-pressure-only | 701 | 5430 | 20677 | bench:rotation:dopri | 875 | 5251 | 1.03 | 6.702 | 6.482 | 0.225 | 0.221 | 29.74 |
| stiff-screened-reset | 137 | 1002 | 0 | bench:stiff-forced:rodas | 14 | 112 | 8.95 | 0.306 | 0.204 | 0.097 | 0.091 | 3.14 |
| stiff-pressure-only | 129 | 938 | 0 | bench:stiff-forced:rodas | 14 | 112 | 8.38 | 0.300 | 0.289 | 0.097 | 0.091 | 3.08 |

Direct candidates (all, incl. non-best):

| run | role | success | attempts | accepted | rejected | RHS | JVP | Jacobians | lin. solves | route | median ms | min ms | timed samples (ms) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bench:rotation:dopri | direct | True (t_final=1.0000) | 875 | 784 | 91 | 5251 | 0 | 0 | 0 | dopri5-5(4)-fsal | 0.225 | 0.221 | 0.231, 0.225, 0.260, 0.221, 0.225 |
| bench:rotation:rodas | direct | True (t_final=1.0000) | 640 | 542 | 98 | 5022 | 20453 | 0 | 5120 | rodas5p-mf-fast-transformed-v1 | 6.545 | 6.317 | 6.545, 9.274, 8.952, 6.400, 6.317 |
| bench:stiff-forced:dopri | direct | True (t_final=1.0000) | 3111 | 3109 | 2 | 18667 | 0 | 0 | 0 | dopri5-5(4)-fsal | 0.890 | 0.872 | 1.419, 0.876, 0.890, 0.872, 0.907 |
| bench:stiff-forced:rodas | direct | True (t_final=1.0000) | 14 | 14 | 0 | 112 | 0 | 14 | 112 | rodas5p-fast-transformed-v2 | 0.097 | 0.091 | 0.092, 0.105, 0.091, 0.097, 0.097 |
| suite:5:rodas5p-mf | direct | True (t_final=2.0000) | 39 | 34 | 5 | 307 | 1206 | 0 | 312 | rodas5p-mf-fast-transformed-v1 | 0.573 | 0.566 | 0.566, 0.573, 0.680, 0.604, 0.571 |
| suite:5:dopri5 | direct | False (t_final=0.3227) | 100000 | 97551 | 2449 | 600001 | 0 | 0 | 0 | dopri5-5(4)-fsal | 43.022 | 41.743 | 42.004, 53.215, 43.022, 43.980, 41.743 |
| suite:10:rodas5p-mf | direct | True (t_final=1.0000) | 2452 | 2230 | 222 | 19394 | 98080 | 0 | 19616 | rodas5p-mf-fast-transformed-v1 | 44.973 | 44.641 | 44.641, 44.973, 44.777, 45.132, 46.074 |
| suite:10:dopri5 | direct | True (t_final=1.0000) | 1105 | 1052 | 53 | 6631 | 0 | 0 | 0 | dopri5-5(4)-fsal | 1.239 | 0.702 | 0.702, 1.236, 1.257, 1.249, 1.239 |

Switch timing samples:

| run | warm-up ms | timed samples (ms) | median | min |
|---|---|---|---|---|
| rotation-evidence | 4.741 | 2.045, 2.049, 2.370, 2.179, 2.307 | 2.179 | 2.045 |
| stiff-evidence | 0.234 | 0.219, 0.265, 0.214, 0.210, 0.212 | 0.214 | 0.210 |
| case5-switching | 3.591 | 2.193, 2.185, 2.246, 2.303, 2.156 | 2.193 | 2.156 |
| case10-switching | 4.252 | 4.162, 4.250, 4.120, 4.202, 4.176 | 4.176 | 4.120 |
| rotation-screened-reset | 2.980 | 2.614, 2.523, 2.434, 2.411, 2.410 | 2.434 | 2.410 |
| rotation-pressure-only | 6.505 | 6.482, 6.941, 6.601, 6.722, 6.702 | 6.702 | 6.482 |
| stiff-screened-reset | 0.211 | 0.205, 0.204, 0.419, 0.313, 0.306 | 0.306 | 0.204 |
| stiff-pressure-only | 0.294 | 0.300, 0.300, 0.289, 0.299, 0.303 | 0.300 | 0.289 |

## 2. Attempt / RHS decomposition by disposition bucket

### rotation-evidence — 1046 attempts (940 acc / 106 rej), 6597 RHS, 5246 JVP, 0 Jac, 1352 solves, switches=0, segments=38, success=True, error=9.258e-07

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 32 | 877 | 789 | 88 | 5263 | 0 | 0 | 0 | 5263 | 1.0000 | 83.8 | 79.8 |
| explicit_probe_discarded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |
| implicit_committed | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |
| implicit_pilot_discarded | 6 | 169 | 151 | 18 | 1334 | 5246 | 0 | 1352 | 7932 | 0.2133 | 16.2 | 20.2 |

### stiff-evidence — 137 attempts (133 acc / 4 rej), 1002 RHS, 0 JVP, 89 Jac, 712 solves, switches=1, segments=11, success=True, error=1.575e-10

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 1 | 32 | 30 | 2 | 193 | 0 | 0 | 0 | 193 | 0.0082 | 23.4 | 19.3 |
| explicit_probe_discarded | 1 | 16 | 14 | 2 | 97 | 0 | 0 | 0 | 97 | 0.0030 | 11.7 | 9.7 |
| implicit_committed | 8 | 81 | 81 | 0 | 648 | 0 | 81 | 648 | 1458 | 0.9918 | 59.1 | 64.7 |
| implicit_pilot_discarded | 1 | 8 | 8 | 0 | 64 | 0 | 8 | 64 | 144 | 0.0082 | 5.8 | 6.4 |

### case5-switching — 232 attempts (205 acc / 27 rej), 1633 RHS, 3802 JVP, 0 Jac, 960 solves, switches=1, segments=23, success=True, error=n/a

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 2 | 64 | 53 | 11 | 386 | 0 | 0 | 0 | 386 | 0.0001 | 27.6 | 23.6 |
| explicit_probe_discarded | 3 | 48 | 36 | 12 | 291 | 0 | 0 | 0 | 291 | 0.0002 | 20.7 | 17.8 |
| implicit_committed | 16 | 97 | 97 | 0 | 776 | 3104 | 0 | 776 | 4753 | 1.9999 | 41.8 | 47.5 |
| implicit_pilot_discarded | 2 | 23 | 19 | 4 | 180 | 698 | 0 | 184 | 1081 | 0.0001 | 9.9 | 11.0 |

### case10-switching — 1321 attempts (1242 acc / 79 rej), 8221 RHS, 5120 JVP, 0 Jac, 1024 solves, switches=0, segments=52, success=True, error=3.085e-02

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 48 | 1193 | 1123 | 70 | 7206 | 0 | 0 | 0 | 7206 | 1.0000 | 90.3 | 87.7 |
| explicit_probe_discarded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |
| implicit_committed | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |
| implicit_pilot_discarded | 4 | 128 | 119 | 9 | 1015 | 5120 | 0 | 1024 | 7159 | 0.0523 | 9.7 | 12.3 |

### rotation-screened-reset — 1274 attempts (1180 acc / 94 rej), 8069 RHS, 6312 JVP, 0 Jac, 1632 solves, switches=0, segments=46, success=True, error=9.061e-07

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 38 | 1070 | 997 | 73 | 6458 | 0 | 0 | 0 | 6458 | 1.0000 | 84.0 | 80.0 |
| explicit_probe_discarded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |
| implicit_committed | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |
| implicit_pilot_discarded | 8 | 204 | 183 | 21 | 1611 | 6312 | 0 | 1632 | 9555 | 0.2489 | 16.0 | 20.0 |

### rotation-pressure-only — 701 attempts (614 acc / 87 rej), 5430 RHS, 20677 JVP, 0 Jac, 5224 solves, switches=1, segments=10, success=True, error=8.324e-08

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 1 | 32 | 30 | 2 | 193 | 0 | 0 | 0 | 193 | 0.0313 | 4.6 | 3.6 |
| explicit_probe_discarded | 1 | 16 | 15 | 1 | 97 | 0 | 0 | 0 | 97 | 0.0113 | 2.3 | 1.8 |
| implicit_committed | 8 | 653 | 569 | 84 | 5140 | 20677 | 0 | 5224 | 31041 | 0.9687 | 93.2 | 94.7 |
| implicit_pilot_discarded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |

### stiff-screened-reset — 137 attempts (133 acc / 4 rej), 1002 RHS, 0 JVP, 89 Jac, 712 solves, switches=1, segments=11, success=True, error=1.575e-10

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 1 | 32 | 30 | 2 | 193 | 0 | 0 | 0 | 193 | 0.0082 | 23.4 | 19.3 |
| explicit_probe_discarded | 1 | 16 | 14 | 2 | 97 | 0 | 0 | 0 | 97 | 0.0030 | 11.7 | 9.7 |
| implicit_committed | 8 | 81 | 81 | 0 | 648 | 0 | 81 | 648 | 1458 | 0.9918 | 59.1 | 64.7 |
| implicit_pilot_discarded | 1 | 8 | 8 | 0 | 64 | 0 | 8 | 64 | 144 | 0.0082 | 5.8 | 6.4 |

### stiff-pressure-only — 129 attempts (125 acc / 4 rej), 938 RHS, 0 JVP, 81 Jac, 648 solves, switches=1, segments=10, success=True, error=1.575e-10

| bucket | batches | attempts | accepted | rejected | RHS | JVP | Jac | lin. solves | work_units | span covered | attempts % | RHS % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| explicit_committed | 1 | 32 | 30 | 2 | 193 | 0 | 0 | 0 | 193 | 0.0082 | 24.8 | 20.6 |
| explicit_probe_discarded | 1 | 16 | 14 | 2 | 97 | 0 | 0 | 0 | 97 | 0.0030 | 12.4 | 10.3 |
| implicit_committed | 8 | 81 | 81 | 0 | 648 | 0 | 81 | 648 | 1458 | 0.9918 | 62.8 | 69.1 |
| implicit_pilot_discarded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.0 | 0.0 |

## 3. RODAS windows (committed implicit batches) and restart overhead vs direct RODAS

Direct comparator = direct RODAS run of the same problem (bench `rodas` = Rodas5p facade; suite `rodas5p-mf`). Rate = attempts per unit time over the span actually covered. Excess = window attempts − direct_rate × covered span. `ramp_est` = accepted steps needed to grow the window's initial_step to the direct run's mean accepted step at max_factor 5 (estimate of the re-ramp cost from h0; every RODAS window restarts at the user's h0 because `proposed_next_step` is not exported for the implicit driver).

### rotation-evidence

Direct RODAS: 640 attempts, 542 accepted over span 1.0 → 640.00 attempts/unit time, mean accepted step 1.8450e-03, 5022 RHS.

- Committed RODAS windows: **0**, attempts 0 (mean n/a/window), accepted 0, RHS 0, span covered 0.0000, rate n/a attempts/unit time, mean accepted step n/a; rate ratio vs direct **n/a**, direct attempts over the same covered span ≈ n/a, excess attempts ≈ **n/a**; RHS/attempt switch n/a vs direct n/a.
- Discarded implicit pilots: **6**, attempts 169 (mean 28.17/pilot), accepted 151, RHS 1334, span covered 0.2133, rate 792.15, rate ratio vs direct 1.24, excess attempts ≈ 32.5.

Discarded implicit pilots:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | h0 | ramp_est | evidence | comparator_work_ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0.031328 | 0.031328 | 0.03133 | True | 26 | 23 | 3 | 205 | 805 | 0 | 208 | 1.362e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.311 |
| 6 | 0.1614 | 0.19937 | 0.19937 | 0.03797 | True | 29 | 26 | 3 | 229 | 901 | 0 | 232 | 1.460e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 7.094 |
| 12 | 0.32265 | 0.35902 | 0.35902 | 0.03637 | True | 29 | 26 | 3 | 229 | 901 | 0 | 232 | 1.399e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 7.094 |
| 19 | 0.5 | 0.53429 | 0.53429 | 0.03429 | True | 28 | 25 | 3 | 221 | 869 | 0 | 224 | 1.372e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.844 |
| 25 | 0.66064 | 0.6959 | 0.6959 | 0.03527 | True | 28 | 25 | 3 | 221 | 869 | 0 | 224 | 1.411e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.844 |
| 31 | 0.82252 | 0.86064 | 0.86064 | 0.03812 | True | 29 | 26 | 3 | 229 | 901 | 0 | 232 | 1.466e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 7.094 |

Explicit committed vs direct DOPRI: 32 batches, 877 attempts over span 1.0000 (rate 877.0) vs direct 875 attempts over span 1.0 (rate 875.0); rate ratio 1.002, excess attempts ≈ 2.0; RHS/attempt 6.00 vs 6.00.

### stiff-evidence

Direct RODAS: 14 attempts, 14 accepted over span 1.0 → 14.00 attempts/unit time, mean accepted step 7.1429e-02, 112 RHS.

- Committed RODAS windows: **8**, attempts 81 (mean 10.12/window), accepted 81, RHS 648, span covered 0.9918, rate 81.67 attempts/unit time, mean accepted step 1.2244e-02; rate ratio vs direct **5.83**, direct attempts over the same covered span ≈ 13.9, excess attempts ≈ **67.1**; RHS/attempt switch 8.00 vs direct 8.00.
- Discarded implicit pilots: **1**, attempts 8 (mean 8.00/pilot), accepted 8, RHS 64, span covered 0.0082, rate 975.85, rate ratio vs direct 69.70, excess attempts ≈ 7.9.

| win idx | start | target | reached | span | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | attempts/unit t | direct att. same span | excess | h0 | ramp_est | clipped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2 | 0.008198 | 0.125 | 0.125 | 0.1168 | 11 | 11 | 0 | 88 | 0 | 11 | 88 | 1.062e-02 | 94.2 | 1.6 | 9.4 | 1.00e-07 | 9 | 0 |
| 3 | 0.125 | 0.25 | 0.25 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 4 | 0.25 | 0.375 | 0.375 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 5 | 0.375 | 0.5 | 0.5 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 7 | 0.5 | 0.625 | 0.625 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 8 | 0.625 | 0.75 | 0.75 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 9 | 0.75 | 0.875 | 0.875 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 10 | 0.875 | 1 | 1 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |

Discarded implicit pilots:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | h0 | ramp_est | evidence | comparator_work_ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0.008198 | 0.008198 | 0.008198 | True | 8 | 8 | 0 | 64 | 0 | 8 | 64 | 1.025e-03 | 1.00e-07 | 9 | implicit-cheaper-on-prefix | 0.746 |

Discarded explicit probes:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | h0 | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 6 | 0.5 | 0.625 | 0.50304 | 0.003039 | False | 16 | 14 | 2 | 97 | 1.00e-07 | explicit-return-not-supported |

Explicit committed vs direct DOPRI: 1 batches, 32 attempts over span 0.0082 (rate 3903.4) vs direct 3111 attempts over span 1.0 (rate 3111.0); rate ratio 1.255, excess attempts ≈ 6.5; RHS/attempt 6.03 vs 6.00.

### case5-switching

Direct RODAS: 39 attempts, 34 accepted over span 2.0 → 19.50 attempts/unit time, mean accepted step 5.8824e-02, 307 RHS.

- Committed RODAS windows: **16**, attempts 97 (mean 6.06/window), accepted 97, RHS 776, span covered 1.9999, rate 48.50 attempts/unit time, mean accepted step 2.0617e-02; rate ratio vs direct **2.49**, direct attempts over the same covered span ≈ 39.0, excess attempts ≈ **58.0**; RHS/attempt switch 8.00 vs direct 7.87.
- Discarded implicit pilots: **2**, attempts 23 (mean 11.50/pilot), accepted 19, RHS 180, span covered 0.0001, rate 209089.16, rate ratio vs direct 10722.52, excess attempts ≈ 23.0.

| win idx | start | target | reached | span | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | attempts/unit t | direct att. same span | excess | h0 | ramp_est | clipped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 4 | 0.00011 | 0.125 | 0.125 | 0.12489 | 7 | 7 | 0 | 56 | 224 | 0 | 56 | 1.784e-02 | 56.0 | 2.4 | 4.6 | 1.00e-04 | 4 | 0 |
| 5 | 0.125 | 0.25 | 0.25 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 6 | 0.25 | 0.375 | 0.375 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 7 | 0.375 | 0.5 | 0.5 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 9 | 0.5 | 0.625 | 0.625 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 10 | 0.625 | 0.75 | 0.75 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 11 | 0.75 | 0.875 | 0.875 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 12 | 0.875 | 1 | 1 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 14 | 1 | 1.125 | 1.125 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 15 | 1.125 | 1.25 | 1.25 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 16 | 1.25 | 1.375 | 1.375 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 17 | 1.375 | 1.5 | 1.5 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 19 | 1.5 | 1.625 | 1.625 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 20 | 1.625 | 1.75 | 1.75 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 21 | 1.75 | 1.875 | 1.875 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |
| 22 | 1.875 | 2 | 2 | 0.125 | 6 | 6 | 0 | 48 | 192 | 0 | 48 | 2.083e-02 | 48.0 | 2.4 | 3.6 | 1.00e-04 | 4 | 0 |

Discarded implicit pilots:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | h0 | ramp_est | evidence | comparator_work_ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 2.4358e-05 | 2.4358e-05 | 2.436e-05 | True | 22 | 18 | 4 | 172 | 666 | 0 | 176 | 1.353e-06 | 2.44e-05 | 5 | implicit-not-cheaper-on-prefix | 5.347 |
| 2 | 2.4358e-05 | 0.00011 | 0.00011 | 8.564e-05 | True | 1 | 1 | 0 | 8 | 32 | 0 | 8 | 8.564e-05 | 8.56e-05 | 5 | implicit-cheaper-on-prefix | 0.254 |

Discarded explicit probes:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | h0 | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 8 | 0.5 | 0.625 | 0.50005 | 4.955e-05 | False | 16 | 12 | 4 | 97 | 1.00e-04 | explicit-return-not-supported |
| 13 | 1 | 1.125 | 1.0001 | 5.411e-05 | False | 16 | 12 | 4 | 97 | 1.00e-04 | explicit-return-not-supported |
| 18 | 1.5 | 1.625 | 1.5001 | 5.687e-05 | False | 16 | 12 | 4 | 97 | 1.00e-04 | explicit-return-not-supported |

Explicit committed vs direct DOPRI: direct DOPRI did not complete the span.

### case10-switching

Direct RODAS: 2452 attempts, 2230 accepted over span 1.0 → 2452.00 attempts/unit time, mean accepted step 4.4843e-04, 19394 RHS.

- Committed RODAS windows: **0**, attempts 0 (mean n/a/window), accepted 0, RHS 0, span covered 0.0000, rate n/a attempts/unit time, mean accepted step n/a; rate ratio vs direct **n/a**, direct attempts over the same covered span ≈ n/a, excess attempts ≈ **n/a**; RHS/attempt switch n/a vs direct n/a.
- Discarded implicit pilots: **4**, attempts 128 (mean 32.00/pilot), accepted 119, RHS 1015, span covered 0.0523, rate 2446.96, rate ratio vs direct 1.00, excess attempts ≈ -0.3.

Discarded implicit pilots:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | h0 | ramp_est | evidence | comparator_work_ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0.028227 | 0.013185 | 0.01319 | False | 32 | 30 | 2 | 254 | 1280 | 0 | 256 | 4.395e-04 | 1.00e-04 | 1 | implicit-pilot-incomplete | 9.275 |
| 2 | 0.028227 | 0.05751 | 0.041482 | 0.01325 | False | 32 | 30 | 2 | 254 | 1280 | 0 | 256 | 4.418e-04 | 1.00e-04 | 1 | implicit-pilot-incomplete | 9.275 |
| 5 | 0.0625 | 0.090624 | 0.075563 | 0.01306 | False | 32 | 30 | 2 | 254 | 1280 | 0 | 256 | 4.354e-04 | 1.00e-04 | 1 | implicit-pilot-incomplete | 9.275 |
| 7 | 0.090624 | 0.11906 | 0.10343 | 0.01281 | False | 32 | 29 | 3 | 253 | 1280 | 0 | 256 | 4.416e-04 | 1.00e-04 | 1 | implicit-pilot-incomplete | 9.269 |

Explicit committed vs direct DOPRI: 48 batches, 1193 attempts over span 1.0000 (rate 1193.0) vs direct 1105 attempts over span 1.0 (rate 1105.0); rate ratio 1.080, excess attempts ≈ 88.0; RHS/attempt 6.04 vs 6.00.

### rotation-screened-reset

Direct RODAS: 640 attempts, 542 accepted over span 1.0 → 640.00 attempts/unit time, mean accepted step 1.8450e-03, 5022 RHS.

- Committed RODAS windows: **0**, attempts 0 (mean n/a/window), accepted 0, RHS 0, span covered 0.0000, rate n/a attempts/unit time, mean accepted step n/a; rate ratio vs direct **n/a**, direct attempts over the same covered span ≈ n/a, excess attempts ≈ **n/a**; RHS/attempt switch n/a vs direct n/a.
- Discarded implicit pilots: **8**, attempts 204 (mean 25.50/pilot), accepted 183, RHS 1611, span covered 0.2489, rate 819.50, rate ratio vs direct 1.28, excess attempts ≈ 44.7.

Discarded implicit pilots:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | h0 | ramp_est | evidence | comparator_work_ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0.031328 | 0.031328 | 0.03133 | True | 26 | 23 | 3 | 205 | 805 | 0 | 208 | 1.362e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.311 |
| 6 | 0.125 | 0.15614 | 0.15614 | 0.03114 | True | 25 | 23 | 2 | 198 | 773 | 0 | 200 | 1.354e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.067 |
| 12 | 0.2815 | 0.31297 | 0.31297 | 0.03147 | True | 24 | 23 | 1 | 191 | 741 | 0 | 192 | 1.368e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 5.824 |
| 18 | 0.43578 | 0.46707 | 0.46707 | 0.03129 | True | 26 | 23 | 3 | 205 | 805 | 0 | 208 | 1.361e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.311 |
| 24 | 0.56263 | 0.59229 | 0.59229 | 0.02965 | True | 24 | 22 | 2 | 190 | 741 | 0 | 192 | 1.348e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 5.819 |
| 30 | 0.68408 | 0.71542 | 0.71542 | 0.03134 | True | 26 | 23 | 3 | 205 | 805 | 0 | 208 | 1.363e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.311 |
| 36 | 0.81259 | 0.84391 | 0.84391 | 0.03132 | True | 27 | 23 | 4 | 212 | 837 | 0 | 216 | 1.362e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.554 |
| 42 | 0.93251 | 0.9639 | 0.9639 | 0.03139 | True | 26 | 23 | 3 | 205 | 805 | 0 | 208 | 1.365e-03 | 1.00e-07 | 7 | implicit-not-cheaper-on-prefix | 6.311 |

Explicit committed vs direct DOPRI: 38 batches, 1070 attempts over span 1.0000 (rate 1070.0) vs direct 875 attempts over span 1.0 (rate 875.0); rate ratio 1.223, excess attempts ≈ 195.0; RHS/attempt 6.04 vs 6.00.

### rotation-pressure-only

Direct RODAS: 640 attempts, 542 accepted over span 1.0 → 640.00 attempts/unit time, mean accepted step 1.8450e-03, 5022 RHS.

- Committed RODAS windows: **8**, attempts 653 (mean 81.62/window), accepted 569, RHS 5140, span covered 0.9687, rate 674.12 attempts/unit time, mean accepted step 1.7024e-03; rate ratio vs direct **1.05**, direct attempts over the same covered span ≈ 619.9, excess attempts ≈ **33.1**; RHS/attempt switch 7.87 vs direct 7.85.
- Discarded implicit pilots: **0**, attempts 0 (mean n/a/pilot), accepted 0, RHS 0, span covered 0.0000, rate n/a, rate ratio vs direct n/a, excess attempts ≈ n/a.

| win idx | start | target | reached | span | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | attempts/unit t | direct att. same span | excess | h0 | ramp_est | clipped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.031328 | 0.125 | 0.125 | 0.093672 | 64 | 56 | 8 | 504 | 2021 | 0 | 512 | 1.673e-03 | 683.2 | 59.9 | 4.1 | 1.00e-07 | 7 | 0 |
| 2 | 0.125 | 0.25 | 0.25 | 0.125 | 83 | 73 | 10 | 654 | 2629 | 0 | 664 | 1.712e-03 | 664.0 | 80.0 | 3.0 | 1.00e-07 | 7 | 0 |
| 3 | 0.25 | 0.375 | 0.375 | 0.125 | 83 | 73 | 10 | 654 | 2629 | 0 | 664 | 1.712e-03 | 664.0 | 80.0 | 3.0 | 1.00e-07 | 7 | 0 |
| 4 | 0.375 | 0.5 | 0.5 | 0.125 | 83 | 73 | 10 | 654 | 2629 | 0 | 664 | 1.712e-03 | 664.0 | 80.0 | 3.0 | 1.00e-07 | 7 | 0 |
| 6 | 0.5 | 0.625 | 0.625 | 0.125 | 86 | 74 | 12 | 676 | 2722 | 0 | 688 | 1.689e-03 | 688.0 | 80.0 | 6.0 | 1.00e-07 | 7 | 0 |
| 7 | 0.625 | 0.75 | 0.75 | 0.125 | 84 | 73 | 11 | 661 | 2661 | 0 | 672 | 1.712e-03 | 672.0 | 80.0 | 4.0 | 1.00e-07 | 7 | 0 |
| 8 | 0.75 | 0.875 | 0.875 | 0.125 | 84 | 73 | 11 | 661 | 2661 | 0 | 672 | 1.712e-03 | 672.0 | 80.0 | 4.0 | 1.00e-07 | 7 | 0 |
| 9 | 0.875 | 1 | 1 | 0.125 | 86 | 74 | 12 | 676 | 2725 | 0 | 688 | 1.689e-03 | 688.0 | 80.0 | 6.0 | 1.00e-07 | 7 | 0 |

Discarded explicit probes:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | h0 | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 5 | 0.5 | 0.625 | 0.51135 | 0.01135 | False | 16 | 15 | 1 | 97 | 1.00e-07 | explicit-return-not-supported |

Explicit committed vs direct DOPRI: 1 batches, 32 attempts over span 0.0313 (rate 1021.4) vs direct 875 attempts over span 1.0 (rate 875.0); rate ratio 1.167, excess attempts ≈ 4.6; RHS/attempt 6.03 vs 6.00.

### stiff-screened-reset

Direct RODAS: 14 attempts, 14 accepted over span 1.0 → 14.00 attempts/unit time, mean accepted step 7.1429e-02, 112 RHS.

- Committed RODAS windows: **8**, attempts 81 (mean 10.12/window), accepted 81, RHS 648, span covered 0.9918, rate 81.67 attempts/unit time, mean accepted step 1.2244e-02; rate ratio vs direct **5.83**, direct attempts over the same covered span ≈ 13.9, excess attempts ≈ **67.1**; RHS/attempt switch 8.00 vs direct 8.00.
- Discarded implicit pilots: **1**, attempts 8 (mean 8.00/pilot), accepted 8, RHS 64, span covered 0.0082, rate 975.85, rate ratio vs direct 69.70, excess attempts ≈ 7.9.

| win idx | start | target | reached | span | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | attempts/unit t | direct att. same span | excess | h0 | ramp_est | clipped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2 | 0.008198 | 0.125 | 0.125 | 0.1168 | 11 | 11 | 0 | 88 | 0 | 11 | 88 | 1.062e-02 | 94.2 | 1.6 | 9.4 | 1.00e-07 | 9 | 0 |
| 3 | 0.125 | 0.25 | 0.25 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 4 | 0.25 | 0.375 | 0.375 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 5 | 0.375 | 0.5 | 0.5 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 7 | 0.5 | 0.625 | 0.625 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 8 | 0.625 | 0.75 | 0.75 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 9 | 0.75 | 0.875 | 0.875 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 10 | 0.875 | 1 | 1 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |

Discarded implicit pilots:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | h0 | ramp_est | evidence | comparator_work_ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0.008198 | 0.008198 | 0.008198 | True | 8 | 8 | 0 | 64 | 0 | 8 | 64 | 1.025e-03 | 1.00e-07 | 9 | implicit-cheaper-on-prefix | 0.746 |

Discarded explicit probes:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | h0 | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 6 | 0.5 | 0.625 | 0.50304 | 0.003039 | False | 16 | 14 | 2 | 97 | 1.00e-07 | explicit-return-not-supported |

Explicit committed vs direct DOPRI: 1 batches, 32 attempts over span 0.0082 (rate 3903.4) vs direct 3111 attempts over span 1.0 (rate 3111.0); rate ratio 1.255, excess attempts ≈ 6.5; RHS/attempt 6.03 vs 6.00.

### stiff-pressure-only

Direct RODAS: 14 attempts, 14 accepted over span 1.0 → 14.00 attempts/unit time, mean accepted step 7.1429e-02, 112 RHS.

- Committed RODAS windows: **8**, attempts 81 (mean 10.12/window), accepted 81, RHS 648, span covered 0.9918, rate 81.67 attempts/unit time, mean accepted step 1.2244e-02; rate ratio vs direct **5.83**, direct attempts over the same covered span ≈ 13.9, excess attempts ≈ **67.1**; RHS/attempt switch 8.00 vs direct 8.00.
- Discarded implicit pilots: **0**, attempts 0 (mean n/a/pilot), accepted 0, RHS 0, span covered 0.0000, rate n/a, rate ratio vs direct n/a, excess attempts ≈ n/a.

| win idx | start | target | reached | span | attempts | acc | rej | RHS | JVP | Jac | solves | mean acc step | attempts/unit t | direct att. same span | excess | h0 | ramp_est | clipped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.008198 | 0.125 | 0.125 | 0.1168 | 11 | 11 | 0 | 88 | 0 | 11 | 88 | 1.062e-02 | 94.2 | 1.6 | 9.4 | 1.00e-07 | 9 | 0 |
| 2 | 0.125 | 0.25 | 0.25 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 3 | 0.25 | 0.375 | 0.375 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 4 | 0.375 | 0.5 | 0.5 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 6 | 0.5 | 0.625 | 0.625 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 7 | 0.625 | 0.75 | 0.75 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 8 | 0.75 | 0.875 | 0.875 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |
| 9 | 0.875 | 1 | 1 | 0.125 | 10 | 10 | 0 | 80 | 0 | 10 | 80 | 1.250e-02 | 80.0 | 1.8 | 8.2 | 1.00e-07 | 9 | 0 |

Discarded explicit probes:

| idx | start | target | reached | span | success | attempts | acc | rej | RHS | h0 | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 5 | 0.5 | 0.625 | 0.50304 | 0.003039 | False | 16 | 14 | 2 | 97 | 1.00e-07 | explicit-return-not-supported |

Explicit committed vs direct DOPRI: 1 batches, 32 attempts over span 0.0082 (rate 3903.4) vs direct 3111 attempts over span 1.0 (rate 3111.0); rate ratio 1.255, excess attempts ≈ 6.5; RHS/attempt 6.03 vs 6.00.

## 4. Decision sequences

Legend: `D[n]` explicit committed batch with n attempts; `Dprobe![n]` discarded explicit probe; `R[n]` committed RODAS window; `Rpilot![n]` discarded implicit pilot. Segment order is disposition order (an entry pilot precedes the explicit batch it evaluated, although the explicit batch executed first).

**rotation-evidence** (switches=0): `0:Rpilot![26] 1:D[32] 2:D[32] 3:D[32] 4:D[16] 5:D[32] 6:Rpilot![29] 7:D[32] 8:D[32] 9:D[12] 10:D[32] 11:D[32] 12:Rpilot![29] 13:D[32] 14:D[14] 15:D[32] 16:D[32] 17:D[32] 18:D[13] 19:Rpilot![28] 20:D[32] 21:D[32] 22:D[32] 23:D[14] 24:D[32] 25:Rpilot![28] 26:D[32] 27:D[32] 28:D[13] 29:D[32] 30:D[32] 31:Rpilot![29] 32:D[32] 33:D[14] 34:D[32] 35:D[32] 36:D[32] 37:D[13]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| dopri5 | explicit_committed | explicit-work-pressure | implicit entry not supported by bounded pilot evidence; retain explicit route | 24 |
| dopri5 | explicit_committed | uncompared | explicit window completed below work pressure | 8 |
| rodas5p | implicit_pilot_discarded | implicit-not-cheaper-on-prefix | same-checkpoint implicit pilot discarded: incomplete or uneconomical | 6 |

**stiff-evidence** (switches=1): `0:Rpilot![8] 1:D[32] 2:R[11] 3:R[10] 4:R[10] 5:R[10] 6:Dprobe![16] 7:R[10] 8:R[10] 9:R[10] 10:R[10]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| rodas5p | implicit_committed | implicit-window-completed | implicit window completed; retain route until bounded recheck | 8 |
| rodas5p | implicit_pilot_discarded | implicit-cheaper-on-prefix | same-checkpoint implicit pilot passed entry work ratio | 1 |
| dopri5 | explicit_committed | explicit-work-pressure | observed explicit pressure and economical replay; enter authorized RODAS route | 1 |
| dopri5 | explicit_probe_discarded | explicit-return-not-supported | bounded explicit probe discarded; retain implicit route | 1 |

**case5-switching** (switches=1): `0:Rpilot![22] 1:D[32] 2:Rpilot![1] 3:D[32] 4:R[7] 5:R[6] 6:R[6] 7:R[6] 8:Dprobe![16] 9:R[6] 10:R[6] 11:R[6] 12:R[6] 13:Dprobe![16] 14:R[6] 15:R[6] 16:R[6] 17:R[6] 18:Dprobe![16] 19:R[6] 20:R[6] 21:R[6] 22:R[6]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| rodas5p | implicit_committed | implicit-window-completed | implicit window completed; retain route until bounded recheck | 16 |
| dopri5 | explicit_probe_discarded | explicit-return-not-supported | bounded explicit probe discarded; retain implicit route | 3 |
| rodas5p | implicit_pilot_discarded | implicit-not-cheaper-on-prefix | same-checkpoint implicit pilot discarded: incomplete or uneconomical | 1 |
| dopri5 | explicit_committed | explicit-work-pressure | implicit entry not supported by bounded pilot evidence; retain explicit route | 1 |
| rodas5p | implicit_pilot_discarded | implicit-cheaper-on-prefix | same-checkpoint implicit pilot passed entry work ratio | 1 |
| dopri5 | explicit_committed | explicit-work-pressure | observed explicit pressure and economical replay; enter authorized RODAS route | 1 |

**case10-switching** (switches=0): `0:Rpilot![32] 1:D[32] 2:Rpilot![32] 3:D[32] 4:D[8] 5:Rpilot![32] 6:D[32] 7:Rpilot![32] 8:D[32] 9:D[10] 10:D[32] 11:D[32] 12:D[9] 13:D[32] 14:D[32] 15:D[10] 16:D[32] 17:D[32] 18:D[14] 19:D[32] 20:D[32] 21:D[8] 22:D[32] 23:D[32] 24:D[13] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:D[11] 31:D[32] 32:D[32] 33:D[12] 34:D[32] 35:D[32] 36:D[13] 37:D[32] 38:D[32] 39:D[10] 40:D[32] 41:D[32] 42:D[9] 43:D[32] 44:D[32] 45:D[10] 46:D[32] 47:D[32] 48:D[10] 49:D[32] 50:D[32] 51:D[12]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| dopri5 | explicit_committed | explicit-work-pressure | implicit entry not supported by bounded pilot evidence; retain explicit route | 32 |
| dopri5 | explicit_committed | uncompared | explicit window completed below work pressure | 16 |
| rodas5p | implicit_pilot_discarded | implicit-pilot-incomplete | same-checkpoint implicit pilot discarded: incomplete or uneconomical | 4 |

**rotation-screened-reset** (switches=0): `0:Rpilot![26] 1:D[32] 2:D[32] 3:D[32] 4:D[32] 5:D[7] 6:Rpilot![25] 7:D[32] 8:D[32] 9:D[32] 10:D[31] 11:D[32] 12:Rpilot![24] 13:D[32] 14:D[32] 15:D[31] 16:D[32] 17:D[32] 18:Rpilot![26] 19:D[32] 20:D[32] 21:D[7] 22:D[32] 23:D[32] 24:Rpilot![24] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:Rpilot![26] 31:D[32] 32:D[32] 33:D[8] 34:D[32] 35:D[32] 36:Rpilot![27] 37:D[32] 38:D[32] 39:D[7] 40:D[32] 41:D[32] 42:Rpilot![26] 43:D[32] 44:D[32] 45:D[9]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| dopri5 | explicit_committed | explicit-work-pressure | implicit entry not supported by bounded pilot evidence; retain explicit route | 32 |
| rodas5p | implicit_pilot_discarded | implicit-not-cheaper-on-prefix | same-checkpoint implicit pilot discarded: incomplete or uneconomical | 8 |
| dopri5 | explicit_committed | uncompared | explicit window completed below work pressure | 6 |

**rotation-pressure-only** (switches=1): `0:D[32] 1:R[64] 2:R[83] 3:R[83] 4:R[83] 5:Dprobe![16] 6:R[86] 7:R[84] 8:R[84] 9:R[86]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| rodas5p | implicit_committed | implicit-window-completed | implicit window completed; retain route until bounded recheck | 8 |
| dopri5 | explicit_committed | explicit-work-pressure | observed explicit work pressure; enter authorized RODAS route | 1 |
| dopri5 | explicit_probe_discarded | explicit-return-not-supported | bounded explicit probe discarded; retain implicit route | 1 |

**stiff-screened-reset** (switches=1): `0:Rpilot![8] 1:D[32] 2:R[11] 3:R[10] 4:R[10] 5:R[10] 6:Dprobe![16] 7:R[10] 8:R[10] 9:R[10] 10:R[10]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| rodas5p | implicit_committed | implicit-window-completed | implicit window completed; retain route until bounded recheck | 8 |
| rodas5p | implicit_pilot_discarded | implicit-cheaper-on-prefix | same-checkpoint implicit pilot passed entry work ratio | 1 |
| dopri5 | explicit_committed | explicit-work-pressure | observed explicit pressure and economical replay; enter authorized RODAS route | 1 |
| dopri5 | explicit_probe_discarded | explicit-return-not-supported | bounded explicit probe discarded; retain implicit route | 1 |

**stiff-pressure-only** (switches=1): `0:D[32] 1:R[11] 2:R[10] 3:R[10] 4:R[10] 5:Dprobe![16] 6:R[10] 7:R[10] 8:R[10] 9:R[10]`

| method | disposition | evidence | decision | count |
|---|---|---|---|---|
| rodas5p | implicit_committed | implicit-window-completed | implicit window completed; retain route until bounded recheck | 8 |
| dopri5 | explicit_committed | explicit-work-pressure | observed explicit work pressure; enter authorized RODAS route | 1 |
| dopri5 | explicit_probe_discarded | explicit-return-not-supported | bounded explicit probe discarded; retain implicit route | 1 |

## 5. Notes / caveats

- WorkCounters (attempts, RHS, JVP, Jacobian builds, linear solves) are the authority; wall-clock is `diagnostic_only` (single process, core 0, warm-up 1 + 5 timed).
- The direct bench RODAS record exposes only the span endpoints in `record.t`, so restart overhead is quantified with whole-span direct rates applied to each window's covered span, not with a per-window matched step density.
- `ramp_est` is an estimate from `adaptive.max_factor` (5.0) and the direct run's mean accepted step; it is not a measured controller trace.
- suite case 5 `dopri5` exhausts the 100000-attempt budget at t≈0.3227 (expected; not a valid direct comparator); best direct for case 5 is `rodas5p-mf`.
- Packet headline numbers (2026-10-25) differ from this binary's counters (e.g. rotation evidence 6597 RHS here vs 6395 in the packet; case10 switching 8221 RHS vs 26018; stiff-forced direct RODAS 112 RHS vs 80); this report uses only the prebuilt binaries at packet head 4fe67602 and does not reconcile the earlier packet.
