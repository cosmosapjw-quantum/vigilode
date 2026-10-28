# A07 collective matched-accuracy report

Rows retained: 36. Schema valid: True. Source binding: True.
Speed promotion: **HOLD**. The reference and uncertainty are empirical; timings are instrumented diagnostics.

| Modes | rtol | dense / structured rows | Absolute-error ratio | Matched achieved accuracy | Paired median time ratio | Independent process units |
|---:|---:|---:|---:|:---:|---:|---:|
| 8 | 1e-10 | 6 / 6 | 1.0019924280713577 | True | 0.4212790390609712 | 3 |
| 8 | 1e-08 | 6 / 6 | 1.0000291675648143 | True | 0.43685227259685844 | 3 |
| 8 | 1e-06 | 6 / 6 | 1.0000037095037084 | True | 0.3921189342630531 | 3 |

Physical tolerances and output grid are fixed; no failed arm is removed. Reference calculation is excluded from every timed arm. Solver/adapter setup, integration and output are included; coefficient tables are prewarmed. New campaigns record complete process wall time and build time separately; process wall time includes startup and reference calculation, with no isolated startup measurement.

A source-bound matched pair is an admissible diagnostic comparison, not a production or speed promotion. The uncertainty interval uses independent process medians; one process is explicitly insufficient. Detailed failures, allocator calibration, raw measurements and intervals are in VALIDATED.json.
