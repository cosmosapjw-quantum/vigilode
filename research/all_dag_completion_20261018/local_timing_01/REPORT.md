# A07 collective matched-accuracy report

Rows retained: 450. Schema valid: True. Source binding: True.
Speed promotion: **HOLD**. The reference and uncertainty are empirical; timings are instrumented diagnostics.

| Modes | rtol | dense / structured rows | Absolute-error ratio | Matched achieved accuracy | Paired median time ratio | Independent process units |
|---:|---:|---:|---:|:---:|---:|---:|
| 8 | 1e-10 | 25 / 25 | 1.0019924280713577 | True | 0.4406952074522642 | 5 |
| 8 | 1e-08 | 25 / 25 | 1.0000291675648143 | True | 0.4369304946096755 | 5 |
| 8 | 1e-06 | 25 / 25 | 1.0000037095037084 | True | 0.49593963474139136 | 5 |
| 32 | 1e-10 | 25 / 25 | 1.0061629062301256 | True | 1.5511242627785058 | 5 |
| 32 | 1e-08 | 25 / 25 | 1.0000706070686485 | True | 1.5583132775160164 | 5 |
| 32 | 1e-06 | 25 / 25 | 1.0000055690537912 | True | 1.5691709418328124 | 5 |
| 128 | 1e-10 | 25 / 25 | 1.0069330064733597 | True | 9.031626525231076 | 5 |
| 128 | 1e-08 | 25 / 25 | 1.000059696730729 | True | 8.917564352011924 | 5 |
| 128 | 1e-06 | 25 / 25 | 1.0000070297396977 | True | 9.185087290232849 | 5 |

Physical tolerances and output grid are fixed; no failed arm is removed. Reference calculation is excluded from every timed arm. Solver/adapter setup, integration and output are included; coefficient tables are prewarmed. New campaigns record complete process wall time and build time separately; process wall time includes startup and reference calculation, with no isolated startup measurement.

A source-bound matched pair is an admissible diagnostic comparison, not a production or speed promotion. The uncertainty interval uses independent process medians; one process is explicitly insufficient. Detailed failures, allocator calibration, raw measurements and intervals are in VALIDATED.json.
