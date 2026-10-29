# A07 collective matched-accuracy report

Rows retained: 30. Schema valid: True. Source binding: False.
Speed promotion: **HOLD**. The reference and uncertainty are empirical; timings are instrumented diagnostics.

| Modes | rtol | dense / structured rows | Absolute-error ratio | Matched achieved accuracy | Paired median time ratio | Independent process units |
|---:|---:|---:|---:|:---:|---:|---:|
| 32 | 1e-10 | 5 / 5 | 1.0061629062301256 | True | 1.6006735749735896 | 1 |
| 32 | 1e-08 | 5 / 5 | 1.0000706070686485 | True | 1.5734829878137 | 1 |
| 32 | 1e-06 | 5 / 5 | 1.0000055690537912 | True | 1.6000715468055564 | 1 |

Physical tolerances and output grid are fixed; no failed arm is removed. Reference calculation is excluded from every timed arm. Solver/adapter setup, integration and output are included; coefficient tables are prewarmed. New campaigns record complete process wall time and build time separately; process wall time includes startup and reference calculation, with no isolated startup measurement.

A source-bound matched pair is an admissible diagnostic comparison, not a production or speed promotion. The uncertainty interval uses independent process medians; one process is explicitly insufficient. Detailed failures, allocator calibration, raw measurements and intervals are in VALIDATED.json.
