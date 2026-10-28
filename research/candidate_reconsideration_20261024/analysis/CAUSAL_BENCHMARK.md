# Causal candidate-reconsideration measurements

All ratios are baseline time divided by adaptive time; values above one favour the adaptive path. The fastest protected method is selected from the successfully completed in-repository controls at the common sampled-error limit. Failed/incomplete groups are excluded from ratios and retained in JSON.

| Case | Fixed/adaptive | Legacy switching/adaptive | Policy changes work? | Best protected | Best protected/adaptive | Adaptive candidate attempts/skips |
|---|---:|---:|---|---|---:|---:|
| broad | 0.932 | 1.054 | True | protected-band | 0.001 | 30/31 |
| changing-rate | 0.921 | 1.271 | True | protected-dense | 0.005 | 28/19 |
| expensive-few-mode | 0.998 | 1.025 | False | protected-band | 0.483 | 29/1 |
| few-mode-basis1 | 1.115 | 0.852 | True | protected-band | 0.050 | 15/15 |
| few-mode | 0.892 | 0.972 | False | protected-band | 0.131 | 29/1 |
| nonnormal | 0.790 | 0.959 | False | protected-band | 0.190 | 17/1 |
| oscillatory | 0.907 | 0.964 | False | protected-band | 0.296 | 90/2 |
| riccati | 1.082 | 1.177 | False | protected-band | 0.145 | 18/1 |

| Case | Method | Passes/samples | Median ms [min, max] | Maximum sampled absolute error |
|---|---|---:|---:|---:|
| broad | fixed-causal | 5/5 | 520.106 [459.613, 550.394] | 1.757e-10 |
| broad | switching-causal | 5/5 | 557.802 [503.523, 579.800] | 1.757e-10 |
| broad | legacy-switching-causal | 5/5 | 587.709 [519.527, 593.810] | 1.757e-10 |
| broad | protected-raw | 5/5 | 582.665 [566.063, 592.512] | 1.757e-10 |
| broad | protected-mf | 5/5 | 589.156 [565.941, 597.171] | 1.757e-10 |
| broad | protected-dense | 5/5 | 1.113 [1.069, 1.386] | 1.757e-10 |
| broad | protected-band | 5/5 | 0.580 [0.496, 0.643] | 1.757e-10 |
| changing-rate | fixed-causal | 5/5 | 131.817 [126.261, 140.460] | 7.191e-09 |
| changing-rate | switching-causal | 5/5 | 143.140 [140.535, 163.220] | 7.198e-09 |
| changing-rate | legacy-switching-causal | 5/5 | 181.878 [173.786, 196.595] | 7.198e-09 |
| changing-rate | protected-raw | 5/5 | 183.732 [176.936, 199.306] | 7.198e-09 |
| changing-rate | protected-mf | 5/5 | 173.638 [167.297, 185.384] | 7.198e-09 |
| changing-rate | protected-dense | 5/5 | 0.771 [0.697, 0.811] | 7.198e-09 |
| expensive-few-mode | fixed-causal | 5/5 | 15.424 [15.202, 15.839] | 6.596e-10 |
| expensive-few-mode | switching-causal | 5/5 | 15.454 [15.192, 15.623] | 6.596e-10 |
| expensive-few-mode | legacy-switching-causal | 5/5 | 15.836 [15.191, 16.404] | 6.596e-10 |
| expensive-few-mode | protected-raw | 5/5 | 17.264 [16.806, 17.934] | 6.596e-10 |
| expensive-few-mode | protected-mf | 5/5 | 9.191 [8.918, 9.333] | 6.596e-10 |
| expensive-few-mode | protected-dense | 5/5 | 7.983 [7.904, 8.143] | 6.596e-10 |
| expensive-few-mode | protected-band | 5/5 | 7.458 [7.344, 7.549] | 6.596e-10 |
| few-mode-basis1 | fixed-causal | 5/5 | 3.238 [3.129, 3.340] | 6.596e-10 |
| few-mode-basis1 | switching-causal | 5/5 | 2.904 [2.764, 3.057] | 6.596e-10 |
| few-mode-basis1 | legacy-switching-causal | 5/5 | 2.475 [2.333, 2.667] | 6.596e-10 |
| few-mode-basis1 | protected-raw | 5/5 | 2.339 [2.210, 2.593] | 6.596e-10 |
| few-mode-basis1 | protected-mf | 5/5 | 1.879 [1.783, 2.008] | 6.596e-10 |
| few-mode-basis1 | protected-dense | 5/5 | 0.582 [0.570, 0.684] | 6.596e-10 |
| few-mode-basis1 | protected-band | 5/5 | 0.145 [0.141, 0.163] | 6.596e-10 |
| few-mode | fixed-causal | 5/5 | 1.005 [0.984, 1.073] | 6.596e-10 |
| few-mode | switching-causal | 5/5 | 1.127 [1.008, 1.220] | 6.596e-10 |
| few-mode | legacy-switching-causal | 5/5 | 1.095 [0.984, 1.370] | 6.596e-10 |
| few-mode | protected-raw | 5/5 | 2.362 [2.342, 2.547] | 6.596e-10 |
| few-mode | protected-mf | 5/5 | 1.828 [1.811, 2.010] | 6.596e-10 |
| few-mode | protected-dense | 5/5 | 0.662 [0.616, 0.851] | 6.596e-10 |
| few-mode | protected-band | 5/5 | 0.148 [0.141, 0.167] | 6.596e-10 |
| nonnormal | fixed-causal | 5/5 | 0.340 [0.319, 0.378] | 1.370e-10 |
| nonnormal | switching-causal | 5/5 | 0.430 [0.359, 0.538] | 1.370e-10 |
| nonnormal | legacy-switching-causal | 5/5 | 0.413 [0.351, 2.834] | 1.370e-10 |
| nonnormal | protected-raw | 5/5 | 0.886 [0.777, 1.019] | 1.370e-10 |
| nonnormal | protected-mf | 5/5 | 0.875 [0.619, 2.054] | 1.370e-10 |
| nonnormal | protected-dense | 5/5 | 0.141 [0.138, 0.256] | 1.370e-10 |
| nonnormal | protected-band | 5/5 | 0.082 [0.077, 0.106] | 1.370e-10 |
| oscillatory | fixed-causal | 5/5 | 1.739 [1.615, 1.904] | 1.705e-07 |
| oscillatory | switching-causal | 5/5 | 1.918 [1.661, 2.036] | 1.705e-07 |
| oscillatory | legacy-switching-causal | 5/5 | 1.848 [1.684, 1.975] | 1.705e-07 |
| oscillatory | protected-raw | 5/5 | 4.237 [3.934, 4.421] | 1.705e-07 |
| oscillatory | protected-mf | 5/5 | 3.373 [3.249, 3.672] | 1.705e-07 |
| oscillatory | protected-dense | 5/5 | 0.617 [0.598, 0.668] | 1.705e-07 |
| oscillatory | protected-band | 5/5 | 0.568 [0.490, 0.732] | 1.705e-07 |
| riccati | fixed-causal | 5/5 | 0.498 [0.396, 0.560] | 1.746e-10 |
| riccati | switching-causal | 5/5 | 0.460 [0.436, 0.838] | 1.746e-10 |
| riccati | legacy-switching-causal | 5/5 | 0.542 [0.475, 0.564] | 1.746e-10 |
| riccati | protected-raw | 5/5 | 1.236 [1.109, 1.454] | 1.746e-10 |
| riccati | protected-mf | 5/5 | 0.988 [0.894, 1.168] | 1.746e-10 |
| riccati | protected-dense | 5/5 | 0.241 [0.141, 0.516] | 1.746e-10 |
| riccati | protected-band | 5/5 | 0.067 [0.066, 0.212] | 1.746e-10 |

## Interpretation limits

- Within-process descriptive medians, five cyclic repetitions; no independent-process confidence interval.
- Exact reference sampled at 17 physical observations; no continuous-time error certificate.
- Expensive-RHS case deliberately injects work, not an application measurement.
- All comparators here are in-repository; no new VODE/DVODE, diffsol or Julia results.
- A win over unconditional candidate does not establish a win over protected dense/banded/matrix-free methods.
