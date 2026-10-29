# Structural quadratic raw reaggregation

Source `0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b`. 2460 IVP solves:60 correctness,160 warmup,2240 timed;19 native tests PASS,2 ignored;88 exact Fraction checks over6 accepted/2 refused kernel records.

| Case | n | GMRES median µs | Proposal median µs | GMRES/proposal median [min,max] | Faster blocks |
|---|---:|---:|---:|---:|---:|
| rotation | 16 | 247.424 | 76.969 | 3.425328 [2.675085,3.903079] | 7/7 |
| rotation-large | 4096 | 22745.464 | 11577.926 | 1.972945 [1.930496,2.161135] | 7/7 |
| damped-rotation | 16 | 241.880 | 76.558 | 3.174434 [2.930166,3.558505] | 7/7 |
| time-jvp | 16 | 2560.370 | 2227.634 | 1.186286 [1.010904,1.395022] | 7/7 |
| badcoeff | 16 | 240.494 | 300.547 | 0.803978 [0.746615,0.920267] | 0/7 |

Durations are medians of14 child medians per arm; paired ratios use the two child-median sums within each ABBA/BAAB block. Ranges are not confidence intervals.

| Case | JVP old→new | MGS inner products old→new | MGS updates old→new | Arnoldi columns old→new | QR/LS cycles old→new | Proposals accept/refuse | Outer attempts old→new | Grid max error old→new |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| rotation | 699→412 | 858→0 | 1001→0 | 286→0 | 143→0 | 144/0 | 18→18 | 8.50431e-14→8.01581e-14 |
| rotation-large | 702→414 | 864→0 | 1008→0 | 288→0 | 144→0 | 144/0 | 18→18 | 8.21565e-14→7.9492e-14 |
| damped-rotation | 699→412 | 858→0 | 1001→0 | 286→0 | 143→0 | 144/0 | 18→18 | 9.9809e-14→9.82547e-14 |
| time-jvp | 2718→2790 | 39168→34272 | 39312→34398 | 2304→2016 | 144→126 | 18/126 | 18→18 | 1.83409e-13→1.85407e-13 |
| badcoeff | 699→1104 | 858→852 | 1001→994 | 286→284 | 143→142 | 2/142 | 18→18 | 8.50431e-14→8.52651e-14 |

Work values are one complete previous-start probe solve; all three repeats agree, and every warm/timed sample has the same counters. QR/LS count is derived from reported GMRES cycles for this ordinary GMRES-into path; there is no separate hardware/function QR counter. New O(n) loops are included in wall time but not counted as zero FLOPs.

All-grid diagnostic maximum: 1.86517468137e-13; campaign scientific wall: 602.900460228/900s including the retained earlier failed checks. Production HOLD.
