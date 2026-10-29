# Root-schedule and parallel-cost experiment

Exact-arithmetic finite propagation does not imply elapsed-time acceleration. Compare against serial causal forward and serial dense as well as same-method parallel speedup.

Five cyclic repetitions in one process per case/thread setting; amortized caller-owned reused pool, pool construction excluded; no one-shot end-to-end parallel-speed guarantee or independent-process CI.

expensive-few-mode injects 2048 sin evaluations into every RHS callback; its crossover is not a measured application speedup.

| Case | Method | 1-thread / 2-thread | Serial forward / 2-thread method | Serial dense / 2-thread method |
|---|---|---:|---:|---:|
| expensive-few-mode | fixed-causal | 1.151 | 1.151 | 0.574 |
| expensive-few-mode | fixed-picard | 1.250 | 1.281 | 0.639 |
| expensive-few-mode | fixed-residual-first | 1.527 | 1.430 | 0.714 |
| expensive-few-mode | protected-dense | 0.997 | 1.998 | 0.997 |
| expensive-few-mode | protected-raw | 1.021 | 0.944 | 0.471 |
| expensive-few-mode | switching-causal | 1.117 | 1.122 | 0.560 |
| expensive-few-mode | switching-picard | 1.312 | 1.312 | 0.655 |
| expensive-few-mode | switching-residual-first | 1.440 | 1.374 | 0.686 |
| few-mode | fixed-causal | 0.523 | 0.523 | 0.320 |
| few-mode | fixed-picard | 0.435 | 0.371 | 0.228 |
| few-mode | fixed-residual-first | 0.497 | 0.336 | 0.206 |
| few-mode | protected-dense | 0.901 | 1.469 | 0.901 |
| few-mode | protected-raw | 0.989 | 0.399 | 0.244 |
| few-mode | switching-causal | 0.542 | 0.508 | 0.312 |
| few-mode | switching-picard | 0.625 | 0.373 | 0.229 |
| few-mode | switching-residual-first | 0.521 | 0.352 | 0.216 |
| riccati | fixed-causal | 0.407 | 0.407 | 0.138 |
| riccati | fixed-picard | 0.328 | 0.164 | 0.056 |
| riccati | fixed-residual-first | 0.339 | 0.114 | 0.039 |
| riccati | protected-dense | 0.994 | 2.926 | 0.994 |
| riccati | protected-raw | 0.954 | 0.379 | 0.129 |
| riccati | switching-causal | 0.383 | 0.367 | 0.125 |
| riccati | switching-picard | 0.356 | 0.163 | 0.055 |
| riccati | switching-residual-first | 0.433 | 0.157 | 0.053 |

| Case | Threads | Candidate | Fixed / switching |
|---|---:|---|---:|
| expensive-few-mode | 1 | causal | 1.005 |
| expensive-few-mode | 1 | picard | 0.976 |
| expensive-few-mode | 1 | residual-first | 1.019 |
| expensive-few-mode | 2 | causal | 0.975 |
| expensive-few-mode | 2 | picard | 1.024 |
| expensive-few-mode | 2 | residual-first | 0.961 |
| few-mode | 1 | causal | 0.938 |
| few-mode | 1 | picard | 0.699 |
| few-mode | 1 | residual-first | 0.998 |
| few-mode | 2 | causal | 0.973 |
| few-mode | 2 | picard | 1.004 |
| few-mode | 2 | residual-first | 1.047 |
| riccati | 1 | causal | 0.959 |
| riccati | 1 | picard | 0.915 |
| riccati | 1 | residual-first | 1.075 |
| riccati | 2 | causal | 0.901 |
| riccati | 2 | picard | 0.991 |
| riccati | 2 | residual-first | 1.374 |

| Case | Threads | Method | Passed / samples | Median ms | Maximum sampled error |
|---|---:|---|---:|---:|---:|
| expensive-few-mode | 1 | fixed-causal | 5/5 | 16.817 | 6.596e-10 |
| expensive-few-mode | 1 | switching-causal | 5/5 | 16.740 | 6.596e-10 |
| expensive-few-mode | 1 | fixed-picard | 5/5 | 16.413 | 6.596e-10 |
| expensive-few-mode | 1 | switching-picard | 5/5 | 16.819 | 6.596e-10 |
| expensive-few-mode | 1 | fixed-residual-first | 5/5 | 17.959 | 6.596e-10 |
| expensive-few-mode | 1 | switching-residual-first | 5/5 | 17.619 | 6.596e-10 |
| expensive-few-mode | 1 | protected-raw | 5/5 | 18.193 | 6.596e-10 |
| expensive-few-mode | 1 | protected-dense | 5/5 | 8.392 | 6.596e-10 |
| expensive-few-mode | 2 | fixed-causal | 5/5 | 14.608 | 6.596e-10 |
| expensive-few-mode | 2 | switching-causal | 5/5 | 14.984 | 6.596e-10 |
| expensive-few-mode | 2 | fixed-picard | 5/5 | 13.132 | 6.596e-10 |
| expensive-few-mode | 2 | switching-picard | 5/5 | 12.821 | 6.596e-10 |
| expensive-few-mode | 2 | fixed-residual-first | 5/5 | 11.762 | 6.596e-10 |
| expensive-few-mode | 2 | switching-residual-first | 5/5 | 12.239 | 6.596e-10 |
| expensive-few-mode | 2 | protected-raw | 5/5 | 17.820 | 6.596e-10 |
| expensive-few-mode | 2 | protected-dense | 5/5 | 8.417 | 6.596e-10 |
| few-mode | 1 | fixed-causal | 5/5 | 1.030 | 6.596e-10 |
| few-mode | 1 | switching-causal | 5/5 | 1.098 | 6.596e-10 |
| few-mode | 1 | fixed-picard | 5/5 | 1.206 | 6.596e-10 |
| few-mode | 1 | switching-picard | 5/5 | 1.725 | 6.596e-10 |
| few-mode | 1 | fixed-residual-first | 5/5 | 1.523 | 6.596e-10 |
| few-mode | 1 | switching-residual-first | 5/5 | 1.526 | 6.596e-10 |
| few-mode | 1 | protected-raw | 5/5 | 2.556 | 6.596e-10 |
| few-mode | 1 | protected-dense | 5/5 | 0.631 | 6.596e-10 |
| few-mode | 2 | fixed-causal | 5/5 | 1.970 | 6.596e-10 |
| few-mode | 2 | switching-causal | 5/5 | 2.025 | 6.596e-10 |
| few-mode | 2 | fixed-picard | 5/5 | 2.772 | 6.596e-10 |
| few-mode | 2 | switching-picard | 5/5 | 2.760 | 6.596e-10 |
| few-mode | 2 | fixed-residual-first | 5/5 | 3.064 | 6.596e-10 |
| few-mode | 2 | switching-residual-first | 5/5 | 2.926 | 6.596e-10 |
| few-mode | 2 | protected-raw | 5/5 | 2.584 | 6.596e-10 |
| few-mode | 2 | protected-dense | 5/5 | 0.701 | 6.596e-10 |
| riccati | 1 | fixed-causal | 5/5 | 0.410 | 1.746e-10 |
| riccati | 1 | switching-causal | 5/5 | 0.428 | 1.746e-10 |
| riccati | 1 | fixed-picard | 5/5 | 0.821 | 1.746e-10 |
| riccati | 1 | switching-picard | 5/5 | 0.897 | 1.746e-10 |
| riccati | 1 | fixed-residual-first | 5/5 | 1.219 | 1.746e-10 |
| riccati | 1 | switching-residual-first | 5/5 | 1.134 | 1.746e-10 |
| riccati | 1 | protected-raw | 5/5 | 1.031 | 1.746e-10 |
| riccati | 1 | protected-dense | 5/5 | 0.139 | 1.746e-10 |
| riccati | 2 | fixed-causal | 5/5 | 1.006 | 1.746e-10 |
| riccati | 2 | switching-causal | 5/5 | 1.117 | 1.746e-10 |
| riccati | 2 | fixed-picard | 5/5 | 2.500 | 1.746e-10 |
| riccati | 2 | switching-picard | 5/5 | 2.523 | 1.746e-10 |
| riccati | 2 | fixed-residual-first | 5/5 | 3.598 | 1.746e-10 |
| riccati | 2 | switching-residual-first | 5/5 | 2.618 | 1.746e-10 |
| riccati | 2 | protected-raw | 5/5 | 1.081 | 1.746e-10 |
| riccati | 2 | protected-dense | 5/5 | 0.140 | 1.746e-10 |
