# Causal candidate-reconsideration measurements

All ratios are baseline time divided by adaptive time; values above one favour the adaptive path. The fastest protected method is selected from the successfully completed in-repository controls at the common sampled-error limit. Failed/incomplete groups are excluded from ratios and retained in JSON.

| Case | Fixed/adaptive | Best protected | Best protected/adaptive | Adaptive candidate attempts/skips |
|---|---:|---|---:|---:|
| few-mode | 0.974 | protected-band | 0.146 | 29/1 |
| expensive-few-mode | 0.998 | protected-band | 0.459 | 29/1 |
| broad | 0.881 | protected-band | 0.001 | 4/57 |
| riccati | 0.910 | protected-band | 0.148 | 18/1 |
| nonnormal | 0.870 | protected-band | 0.208 | 17/1 |
| oscillatory | 0.972 | protected-band | 0.309 | 90/2 |
| changing-rate | 0.731 | protected-dense | 0.004 | 4/43 |

| Case | Method | Passes/samples | Median ms [min, max] | Maximum sampled absolute error |
|---|---|---:|---:|---:|
| few-mode | fixed-causal | 5/5 | 0.978 [0.954, 1.039] | 6.596e-10 |
| few-mode | switching-causal | 5/5 | 1.004 [1.000, 1.110] | 6.596e-10 |
| few-mode | protected-raw | 5/5 | 2.310 [2.248, 2.615] | 6.596e-10 |
| few-mode | protected-mf | 5/5 | 1.865 [1.820, 1.949] | 6.596e-10 |
| few-mode | protected-dense | 5/5 | 0.589 [0.582, 0.622] | 6.596e-10 |
| few-mode | protected-band | 5/5 | 0.146 [0.145, 0.147] | 6.596e-10 |
| expensive-few-mode | fixed-causal | 5/5 | 16.073 [15.176, 16.131] | 6.596e-10 |
| expensive-few-mode | switching-causal | 5/5 | 16.100 [15.457, 17.217] | 6.596e-10 |
| expensive-few-mode | protected-raw | 5/5 | 16.983 [16.687, 18.065] | 6.596e-10 |
| expensive-few-mode | protected-mf | 5/5 | 9.041 [8.985, 9.098] | 6.596e-10 |
| expensive-few-mode | protected-dense | 5/5 | 7.920 [7.679, 8.147] | 6.596e-10 |
| expensive-few-mode | protected-band | 5/5 | 7.386 [7.307, 7.641] | 6.596e-10 |
| broad | fixed-causal | 5/5 | 477.980 [448.769, 484.334] | 1.757e-10 |
| broad | switching-causal | 5/5 | 542.273 [539.536, 671.421] | 1.757e-10 |
| broad | protected-raw | 5/5 | 553.409 [530.072, 626.856] | 1.757e-10 |
| broad | protected-mf | 5/5 | 535.012 [530.292, 548.372] | 1.757e-10 |
| broad | protected-dense | 5/5 | 1.107 [0.991, 1.218] | 1.757e-10 |
| broad | protected-band | 5/5 | 0.511 [0.494, 0.569] | 1.757e-10 |
| riccati | fixed-causal | 5/5 | 0.402 [0.400, 0.410] | 1.746e-10 |
| riccati | switching-causal | 5/5 | 0.441 [0.430, 0.490] | 1.746e-10 |
| riccati | protected-raw | 5/5 | 1.096 [1.028, 1.170] | 1.746e-10 |
| riccati | protected-mf | 5/5 | 0.864 [0.845, 0.929] | 1.746e-10 |
| riccati | protected-dense | 5/5 | 0.138 [0.138, 0.185] | 1.746e-10 |
| riccati | protected-band | 5/5 | 0.066 [0.065, 0.069] | 1.746e-10 |
| nonnormal | fixed-causal | 5/5 | 0.322 [0.320, 0.340] | 1.370e-10 |
| nonnormal | switching-causal | 5/5 | 0.370 [0.348, 0.383] | 1.370e-10 |
| nonnormal | protected-raw | 5/5 | 0.786 [0.767, 1.186] | 1.370e-10 |
| nonnormal | protected-mf | 5/5 | 0.616 [0.594, 0.735] | 1.370e-10 |
| nonnormal | protected-dense | 5/5 | 0.136 [0.133, 0.176] | 1.370e-10 |
| nonnormal | protected-band | 5/5 | 0.077 [0.076, 0.080] | 1.370e-10 |
| oscillatory | fixed-causal | 5/5 | 1.584 [1.551, 1.672] | 1.705e-07 |
| oscillatory | switching-causal | 5/5 | 1.630 [1.609, 1.759] | 1.705e-07 |
| oscillatory | protected-raw | 5/5 | 3.769 [3.724, 4.177] | 1.705e-07 |
| oscillatory | protected-mf | 5/5 | 3.016 [2.972, 3.744] | 1.705e-07 |
| oscillatory | protected-dense | 5/5 | 0.610 [0.605, 0.679] | 1.705e-07 |
| oscillatory | protected-band | 5/5 | 0.504 [0.489, 0.519] | 1.705e-07 |
| changing-rate | fixed-causal | 5/5 | 117.558 [114.697, 140.224] | 7.191e-09 |
| changing-rate | switching-causal | 5/5 | 160.823 [154.735, 161.394] | 7.198e-09 |
| changing-rate | protected-raw | 5/5 | 160.060 [156.533, 173.913] | 7.198e-09 |
| changing-rate | protected-mf | 5/5 | 157.396 [151.386, 171.238] | 7.198e-09 |
| changing-rate | protected-dense | 5/5 | 0.683 [0.681, 0.902] | 7.198e-09 |

## Interpretation limits

- Within-process descriptive medians, five cyclic repetitions; no independent-process confidence interval.
- Exact reference sampled at 17 physical observations; no continuous-time error certificate.
- Expensive-RHS case deliberately injects work, not an application measurement.
- All comparators here are in-repository; no new VODE/DVODE, diffsol or Julia results.
- A win over unconditional candidate does not establish a win over protected dense/banded/matrix-free methods.
