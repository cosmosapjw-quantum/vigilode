# Root-schedule and parallel-cost experiment

Exact-arithmetic finite propagation does not imply elapsed-time acceleration. Compare against serial causal forward and serial dense as well as same-method parallel speedup.

Five cyclic repetitions in one process per case/thread setting; amortized caller-owned reused pool, pool construction excluded; no one-shot end-to-end parallel-speed guarantee or independent-process CI.

expensive-few-mode injects 2048 sin evaluations into every RHS callback; its crossover is not a measured application speedup.

| Case | Method | 1-thread / 2-thread | Serial forward / 2-thread method | Serial dense / 2-thread method |
|---|---|---:|---:|---:|
| expensive-few-mode | fixed-causal | 1.018 | 1.018 | 0.492 |
| expensive-few-mode | fixed-picard | 1.237 | 1.250 | 0.604 |
| expensive-few-mode | fixed-residual-first | 1.436 | 1.318 | 0.637 |
| expensive-few-mode | protected-dense | 0.989 | 2.047 | 0.989 |
| expensive-few-mode | protected-raw | 1.035 | 0.973 | 0.470 |
| expensive-few-mode | switching-causal | 0.992 | 1.021 | 0.493 |
| expensive-few-mode | switching-picard | 1.313 | 1.328 | 0.642 |
| expensive-few-mode | switching-residual-first | 1.390 | 1.287 | 0.622 |
| few-mode | fixed-causal | 0.478 | 0.478 | 0.286 |
| few-mode | fixed-picard | 0.420 | 0.325 | 0.195 |
| few-mode | fixed-residual-first | 0.454 | 0.314 | 0.188 |
| few-mode | protected-dense | 0.957 | 1.599 | 0.957 |
| few-mode | protected-raw | 1.023 | 0.362 | 0.217 |
| few-mode | switching-causal | 0.489 | 0.474 | 0.284 |
| few-mode | switching-picard | 0.533 | 0.360 | 0.215 |
| few-mode | switching-residual-first | 0.509 | 0.313 | 0.187 |
| riccati | fixed-causal | 0.420 | 0.420 | 0.138 |
| riccati | fixed-picard | 0.268 | 0.125 | 0.041 |
| riccati | fixed-residual-first | 0.322 | 0.112 | 0.037 |
| riccati | protected-dense | 0.963 | 2.934 | 0.963 |
| riccati | protected-raw | 0.950 | 0.368 | 0.121 |
| riccati | switching-causal | 0.418 | 0.413 | 0.136 |
| riccati | switching-picard | 0.329 | 0.150 | 0.049 |
| riccati | switching-residual-first | 0.428 | 0.145 | 0.047 |

| Case | Threads | Candidate | Fixed / switching |
|---|---:|---|---:|
| expensive-few-mode | 1 | causal | 1.029 |
| expensive-few-mode | 1 | picard | 1.001 |
| expensive-few-mode | 1 | residual-first | 1.009 |
| expensive-few-mode | 2 | causal | 1.003 |
| expensive-few-mode | 2 | picard | 1.063 |
| expensive-few-mode | 2 | residual-first | 0.977 |
| few-mode | 1 | causal | 0.969 |
| few-mode | 1 | picard | 0.872 |
| few-mode | 1 | residual-first | 0.888 |
| few-mode | 2 | causal | 0.991 |
| few-mode | 2 | picard | 1.106 |
| few-mode | 2 | residual-first | 0.996 |
| riccati | 1 | causal | 0.989 |
| riccati | 1 | picard | 0.978 |
| riccati | 1 | residual-first | 0.974 |
| riccati | 2 | causal | 0.983 |
| riccati | 2 | picard | 1.203 |
| riccati | 2 | residual-first | 1.295 |

| Case | Threads | Method | Passed / samples | Median ms | Maximum sampled error |
|---|---:|---|---:|---:|---:|
| expensive-few-mode | 1 | fixed-causal | 5/5 | 17.309 | 6.596e-10 |
| expensive-few-mode | 1 | switching-causal | 5/5 | 16.822 | 6.596e-10 |
| expensive-few-mode | 1 | fixed-picard | 5/5 | 17.129 | 6.596e-10 |
| expensive-few-mode | 1 | switching-picard | 5/5 | 17.110 | 6.596e-10 |
| expensive-few-mode | 1 | fixed-residual-first | 5/5 | 18.858 | 6.596e-10 |
| expensive-few-mode | 1 | switching-residual-first | 5/5 | 18.686 | 6.596e-10 |
| expensive-few-mode | 1 | protected-raw | 5/5 | 18.415 | 6.596e-10 |
| expensive-few-mode | 1 | protected-dense | 5/5 | 8.362 | 6.596e-10 |
| expensive-few-mode | 2 | fixed-causal | 5/5 | 17.000 | 6.596e-10 |
| expensive-few-mode | 2 | switching-causal | 5/5 | 16.954 | 6.596e-10 |
| expensive-few-mode | 2 | fixed-picard | 5/5 | 13.849 | 6.596e-10 |
| expensive-few-mode | 2 | switching-picard | 5/5 | 13.029 | 6.596e-10 |
| expensive-few-mode | 2 | fixed-residual-first | 5/5 | 13.135 | 6.596e-10 |
| expensive-few-mode | 2 | switching-residual-first | 5/5 | 13.446 | 6.596e-10 |
| expensive-few-mode | 2 | protected-raw | 5/5 | 17.794 | 6.596e-10 |
| expensive-few-mode | 2 | protected-dense | 5/5 | 8.454 | 6.596e-10 |
| few-mode | 1 | fixed-causal | 5/5 | 1.020 | 6.596e-10 |
| few-mode | 1 | switching-causal | 5/5 | 1.053 | 6.596e-10 |
| few-mode | 1 | fixed-picard | 5/5 | 1.317 | 6.596e-10 |
| few-mode | 1 | switching-picard | 5/5 | 1.510 | 6.596e-10 |
| few-mode | 1 | fixed-residual-first | 5/5 | 1.472 | 6.596e-10 |
| few-mode | 1 | switching-residual-first | 5/5 | 1.658 | 6.596e-10 |
| few-mode | 1 | protected-raw | 5/5 | 2.878 | 6.596e-10 |
| few-mode | 1 | protected-dense | 5/5 | 0.610 | 6.596e-10 |
| few-mode | 2 | fixed-causal | 5/5 | 2.134 | 6.596e-10 |
| few-mode | 2 | switching-causal | 5/5 | 2.153 | 6.596e-10 |
| few-mode | 2 | fixed-picard | 5/5 | 3.135 | 6.596e-10 |
| few-mode | 2 | switching-picard | 5/5 | 2.835 | 6.596e-10 |
| few-mode | 2 | fixed-residual-first | 5/5 | 3.246 | 6.596e-10 |
| few-mode | 2 | switching-residual-first | 5/5 | 3.259 | 6.596e-10 |
| few-mode | 2 | protected-raw | 5/5 | 2.813 | 6.596e-10 |
| few-mode | 2 | protected-dense | 5/5 | 0.638 | 6.596e-10 |
| riccati | 1 | fixed-causal | 5/5 | 0.424 | 1.746e-10 |
| riccati | 1 | switching-causal | 5/5 | 0.429 | 1.746e-10 |
| riccati | 1 | fixed-picard | 5/5 | 0.909 | 1.746e-10 |
| riccati | 1 | switching-picard | 5/5 | 0.929 | 1.746e-10 |
| riccati | 1 | fixed-residual-first | 5/5 | 1.223 | 1.746e-10 |
| riccati | 1 | switching-residual-first | 5/5 | 1.255 | 1.746e-10 |
| riccati | 1 | protected-raw | 5/5 | 1.095 | 1.746e-10 |
| riccati | 1 | protected-dense | 5/5 | 0.139 | 1.746e-10 |
| riccati | 2 | fixed-causal | 5/5 | 1.009 | 1.746e-10 |
| riccati | 2 | switching-causal | 5/5 | 1.026 | 1.746e-10 |
| riccati | 2 | fixed-picard | 5/5 | 3.395 | 1.746e-10 |
| riccati | 2 | switching-picard | 5/5 | 2.822 | 1.746e-10 |
| riccati | 2 | fixed-residual-first | 5/5 | 3.799 | 1.746e-10 |
| riccati | 2 | switching-residual-first | 5/5 | 2.935 | 1.746e-10 |
| riccati | 2 | protected-raw | 5/5 | 1.152 | 1.746e-10 |
| riccati | 2 | protected-dense | 5/5 | 0.145 | 1.746e-10 |
