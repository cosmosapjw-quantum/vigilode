# Local adaptive trajectory results

All configurations, including failures, are retained in the campaign receipts. Timings use one local sandbox and cannot be extrapolated to NCP.

| Case | Path | Workers | rtol | All repeats passed | Max absolute error | Median seconds | Fallback attempts |
|---|---|---:|---:|---|---:|---:|---:|
| few-mode | sequential | 1 | 1e-05 | True | 8.170043165221449e-10 | 0.00951309 | n/a |
| few-mode | coupled | 1 | 1e-05 | True | 8.17004304595921e-10 | 0.0225187 | 0 |
| few-mode | coupled | 2 | 1e-05 | True | 8.17004304595921e-10 | 0.11232 | 0 |
| few-mode | sequential | 1 | 1e-07 | True | 1.272700900830509e-11 | 0.0293223 | n/a |
| few-mode | coupled | 1 | 1e-07 | True | 1.2727016597720298e-11 | 0.0868787 | 0 |
| few-mode | coupled | 2 | 1e-07 | True | 1.2727016597720298e-11 | 0.817446 | 0 |
| riccati | sequential | 1 | 1e-05 | True | 1.3622328178669596e-08 | 0.00212098 | n/a |
| riccati | coupled | 1 | 1e-05 | True | 1.3622328327855815e-08 | 0.00457354 | 0 |
| riccati | coupled | 2 | 1e-05 | True | 1.3622328327855815e-08 | 0.0302578 | 0 |
| riccati | sequential | 1 | 1e-07 | True | 5.388120713489819e-10 | 0.0143095 | n/a |
| riccati | coupled | 1 | 1e-07 | True | 5.388121684934966e-10 | 0.0111855 | 0 |
| riccati | coupled | 2 | 1e-07 | True | 5.388121684934966e-10 | 0.147014 | 0 |
| stiff | sequential | 1 | 1e-05 | True | 4.409272145317056e-09 | 0.0142856 | n/a |
| stiff | coupled | 1 | 1e-05 | True | 4.409271782759849e-09 | 0.0818442 | 0 |
| stiff | coupled | 2 | 1e-05 | True | 4.409271782759849e-09 | 0.0965968 | 0 |
| stiff | sequential | 1 | 1e-07 | True | 3.368002751691357e-11 | 0.0196368 | n/a |
| stiff | coupled | 1 | 1e-07 | True | 3.368005180304223e-11 | 0.0184451 | 0 |
| stiff | coupled | 2 | 1e-07 | True | 3.368005180304223e-11 | 0.20889 | 0 |
| nonnormal | sequential | 1 | 1e-05 | True | 5.659099605592743e-09 | 0.0157824 | n/a |
| nonnormal | coupled | 1 | 1e-05 | True | 5.659099494570441e-09 | 0.00146643 | 0 |
| nonnormal | coupled | 2 | 1e-05 | True | 5.659099494570441e-09 | 0.0559461 | 0 |
| nonnormal | sequential | 1 | 1e-07 | True | 5.243178113900626e-11 | 0.0162252 | n/a |
| nonnormal | coupled | 1 | 1e-07 | True | 5.2431253783069565e-11 | 0.00718801 | 0 |
| nonnormal | coupled | 2 | 1e-07 | True | 5.2431253783069565e-11 | 0.0321995 | 0 |
| oscillatory | sequential | 1 | 1e-05 | True | 1.9130162374203508e-06 | 0.00647246 | n/a |
| oscillatory | coupled | 1 | 1e-05 | True | 1.913016217658381e-06 | 0.00363505 | 0 |
| oscillatory | coupled | 2 | 1e-05 | True | 1.913016217658381e-06 | 0.0498865 | 0 |
| oscillatory | sequential | 1 | 1e-07 | True | 7.421301684296111e-09 | 0.0510275 | n/a |
| oscillatory | coupled | 1 | 1e-07 | True | 7.421283476638507e-09 | 0.0110121 | 0 |
| oscillatory | coupled | 2 | 1e-07 | True | 7.421283476638507e-09 | 0.596896 | 0 |
| broad | sequential | 1 | 1e-05 | True | 1.1674085081871816e-09 | 6.08638 | n/a |
| broad | coupled | 1 | 1e-05 | True | 1.167355824526796e-09 | 5.32359 | 37 |
| broad | coupled | 2 | 1e-05 | True | 1.167355824526796e-09 | 2.72053 | 37 |
| broad | sequential | 1 | 1e-07 | True | 2.2450785913501192e-11 | 9.38085 | n/a |
| broad | coupled | 1 | 1e-07 | True | 2.2451722637073165e-11 | 11.2041 | 79 |
| broad | coupled | 2 | 1e-07 | True | 2.2451722637073165e-11 | 7.74992 | 79 |
| few-mode-large | sequential | 1 | 1e-05 | True | 8.170042933202185e-10 | 0.224653 | n/a |
| few-mode-large | coupled | 1 | 1e-05 | True | 8.170042825866169e-10 | 0.213481 | 0 |
| few-mode-large | coupled | 2 | 1e-05 | True | 8.170042825866169e-10 | 0.294277 | 0 |
| few-mode-large | sequential | 1 | 1e-07 | True | 1.2727015079837256e-11 | 0.449778 | n/a |
| few-mode-large | coupled | 1 | 1e-07 | True | 1.2727075795158915e-11 | 0.462548 | 0 |
| few-mode-large | coupled | 2 | 1e-07 | True | 1.2727075795158915e-11 | 0.596877 | 0 |
| riccati | forced-fallback | 1 | 1e-07 | True | 5.388120713489819e-10 | 0.00459926 | 16 |

The machine-readable comparison uses common maximum-absolute-error thresholds of 1e-6 and 1e-8. It selects the fastest measured configuration satisfying each threshold for both methods. This establishes a shared accuracy requirement, not exactly equal errors. Failed and ineligible configurations are excluded from timing ratios but remain in the table.

All 51 warm-up solves and 153 timed solves returned success. This is completion evidence; it does not mean every configuration satisfies either accuracy threshold. In particular, the loose oscillatory configurations have sampled maximum error about 1.91e-6 and are excluded from both 1e-6 and 1e-8 comparisons.

The broad-spectrum configurations use sequential fallback in 37/38 or 79/81 attempts (97.4% or 97.5%). Their elapsed-time ratios therefore do not establish success or speedup of the shared-subspace candidate. The differing timing samples are noisy: for example the loose broad one-worker coupled measurements span 2.13–6.05 seconds, while the bare sequential measurements span 6.06–14.57 seconds. Scheduling and run-order effects are uncontrolled. The measured medians are retained as observations, not a performance qualification.

Two-worker execution is slower than one-worker execution at the same settings in 12 of the 14 paired configurations; the two broad cases are the exceptions and are fallback-dominated. Cheap callbacks, task scheduling and temporary allocations can outweigh saved JVP work. The 6144-component few-mode case shows only a 1.052 median bare-sequential/coupled-one-worker ratio at the looser tolerance, with overlapping observed ranges. A cost-based selector and workspace reuse remain necessary.

SUMMARY.json preserves worker ratios, fallback fractions, repeat-to-repeat work agreement and callback-counter reconciliation. The separately listed fallback callback diagnostics are not added again to total work. Error comparisons sample five observation times (two for refinement), not a continuous-time maximum.

![Measured work-precision](WORK_PRECISION.png)
