# VigilODE Comprehensive Cross-Solver Benchmark Report

> **Timing Authority**: `diagnostic_only` (Measured on same physical host with median repetitions).
> **Error Metric**: Max WRMS error on common 101-point uniform grid against reference solution ($rtol=10^{-12}$).
> **Allocations**: Hot-loop allocations verified by `vigilode-bench` allocation interposition hook.

## Problem: `robertson`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.671e-07 | 0.084 | 902 | 111 | 111/2 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 1.156e-06 | 0.078 | 342 | 36 | 36/2 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 1.697e-04 | 0.040 | 155 | 3 | 109/3 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 9.432e-05 | 0.086 | 169 | 0 | 92/1 | 669 calls |
| **Julia FBDF** | ✅ Success | 7.965e-05 | 0.140 | 236 | 2 | 96/4 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.863e-04 | 0.161 | 207 | 4 | 138/8 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 2.863e-04 | 0.150 | 207 | 4 | 130/8 | GC managed |
| **Hairer RODAS** | ✅ Success | 2.191e-05 | 0.018 | 192 | 32 | 32/0 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 1.511e-05 | 0.019 | 187 | 24 | 28/1 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 5.188e-08 | 0.094 | 1004 | 122 | 122/4 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 5.407e-08 | 0.085 | 612 | 66 | 66/2 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 3.366e-06 | 0.082 | 337 | 5 | 237/19 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 1.497e-06 | 0.114 | 260 | 0 | 168/0 | 871 calls |
| **Julia FBDF** | ✅ Success | 8.666e-07 | 0.297 | 436 | 2 | 189/18 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.029e-06 | 0.236 | 304 | 4 | 231/7 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 1.985e-06 | 0.224 | 304 | 5 | 224/7 | GC managed |
| **Hairer RODAS** | ✅ Success | 3.982e-07 | 0.038 | 485 | 80 | 80/1 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 8.013e-07 | 0.030 | 338 | 40 | 47/2 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.598e-09 | 0.121 | 1347 | 164 | 164/5 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 1.147e-09 | 0.264 | 1484 | 162 | 162/3 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 5.523e-08 | 0.122 | 535 | 7 | 403/22 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 2.862e-08 | 0.192 | 511 | 0 | 331/11 | 1450 calls |
| **Julia FBDF** | ✅ Success | 6.755e-09 | 0.911 | 702 | 2 | 374/14 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 5.151e-08 | 0.418 | 554 | 8 | 403/29 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 5.150e-08 | 0.332 | 554 | 8 | 374/29 | GC managed |
| **Hairer RODAS** | ✅ Success | 9.252e-09 | 0.101 | 1445 | 240 | 240/1 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 4.445e-08 | 0.048 | 639 | 68 | 90/2 | 109 calls |

## Problem: `hires`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.274e-05 | 0.188 | 1119 | 132 | 132/9 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 4.850e-04 | 0.239 | 819 | 81 | 81/11 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 7.920e-04 | 0.201 | 555 | 10 | 356/27 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 4.061e-04 | 0.244 | 488 | 0 | 189/11 | 1322 calls |
| **Julia FBDF** | ✅ Success | 1.431e-04 | 0.598 | 618 | 8 | 181/16 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.061e-04 | 0.414 | 382 | 9 | 238/13 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 3.038e-04 | 0.312 | 383 | 9 | 225/13 | GC managed |
| **Hairer RODAS** | ✅ Success | 5.157e-05 | 0.090 | 562 | 92 | 92/2 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 3.158e-05 | 0.101 | 622 | 37 | 61/0 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.435e-07 | 0.363 | 2141 | 265 | 265/3 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 2.329e-06 | 0.727 | 2524 | 258 | 258/25 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 8.976e-06 | 0.293 | 827 | 11 | 496/45 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 5.554e-06 | 0.430 | 884 | 0 | 363/15 | 2071 calls |
| **Julia FBDF** | ✅ Success | 3.609e-05 | 1.300 | 1440 | 6 | 395/20 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 8.504e-06 | 0.954 | 825 | 12 | 513/46 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 8.504e-06 | 0.651 | 826 | 12 | 467/46 | GC managed |
| **Hairer RODAS** | ✅ Success | 7.625e-07 | 0.337 | 2219 | 369 | 369/1 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 6.856e-06 | 0.180 | 1140 | 73 | 117/2 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.180e-09 | 1.068 | 6633 | 823 | 823/7 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 5.293e-09 | 2.093 | 8091 | 873 | 873/29 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 1.668e-07 | 0.484 | 1347 | 17 | 958/62 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 8.148e-08 | 0.617 | 1396 | 0 | 725/7 | 2827 calls |
| **Julia FBDF** | ✅ Success | 1.009e-07 | 2.497 | 2398 | 7 | 898/43 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 1.655e-07 | 1.739 | 1512 | 19 | 1103/62 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 1.638e-07 | 1.070 | 1512 | 19 | 1041/62 | GC managed |
| **Hairer RODAS** | ✅ Success | 8.460e-09 | 1.863 | 12617 | 2102 | 2102/1 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 1.008e-06 | 0.308 | 2050 | 134 | 235/1 | 109 calls |

## Problem: `van-der-pol-mu1000`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.442e-05 | 0.185 | 2446 | 234 | 234/82 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 7.105e-05 | 0.326 | 2109 | 163 | 163/80 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 3.710e-03 | 0.143 | 808 | 22 | 495/58 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 2.858e-04 | 0.355 | 1029 | 0 | 428/67 | 3272 calls |
| **Julia FBDF** | ✅ Success | 5.172e-03 | 1.116 | 1953 | 31 | 449/113 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.718e-03 | 0.455 | 780 | 19 | 487/55 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 3.718e-03 | 0.412 | 779 | 19 | 432/55 | GC managed |
| **Hairer RODAS** | ✅ Success | 1.331e-04 | 0.073 | 1350 | 210 | 210/18 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 9.541e-05 | 0.088 | 1739 | 139 | 188/8 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 4.598e-07 | 0.286 | 3904 | 411 | 411/88 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 1.754e-06 | 0.531 | 4759 | 381 | 381/166 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 2.547e-04 | 0.250 | 1352 | 22 | 925/89 | 108 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 3.977e-05 | 0.563 | 1777 | 0 | 817/115 | 4803 calls |
| **Julia FBDF** | ✅ Success | 6.097e-05 | 2.150 | 3342 | 13 | 936/153 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.539e-04 | 0.840 | 1354 | 23 | 925/90 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 2.539e-04 | 0.759 | 1354 | 23 | 835/90 | GC managed |
| **Hairer RODAS** | ✅ Success | 6.608e-06 | 0.191 | 3708 | 608 | 608/12 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 2.470e-06 | 0.149 | 3008 | 247 | 350/11 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.237e-08 | 0.569 | 7960 | 960 | 960/40 | 0 (Zero-Alloc) |
| **Julia Rodas5P** | ✅ Success | 1.724e-08 | 1.418 | 10569 | 975 | 975/224 | GC managed |
| **Netlib DVODE (MF=21)** | ❌ DVODE ISTATE -1 | — | 0.222 | 1209 | 16 | 818/80 | 48 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 2.271e-06 | 0.944 | 3275 | 0 | 1776/105 | 7716 calls |
| **Julia FBDF** | ✅ Success | 1.815e-06 | 4.270 | 5796 | 11 | 1944/185 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.502e-06 | 1.845 | 2860 | 37 | 2023/169 | 200 calls |
| **Julia CVODE_BDF** | ✅ Success | 3.502e-06 | 1.433 | 2858 | 37 | 1854/169 | GC managed |
| **Hairer RODAS** | ✅ Success | 5.991e-08 | 0.595 | 11916 | 1981 | 1981/6 | 108 calls |
| **Hairer RADAU5** | ✅ Success | 1.348e-07 | 0.265 | 5539 | 360 | 715/7 | 109 calls |

## Problem: `brusselator-1d-n100`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 2.213e-05 | 1.678 | 847 | 105 | 105/1 | 0 (Zero-Alloc) |
| **VigilODE (Dense)** | ✅ Success | 2.213e-05 | 2.801 | 847 | 105 | 105/1 | 0 (Zero-Alloc) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 1.492e-03 | 0.454 | 148 | 3 | 122/3 | 108 calls |
| **Diffsol BDF (Sparse)** | ✅ Success | 1.768e-03 | 0.740 | 191 | 0 | 97/6 | 1499 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 1.768e-03 | 2.645 | 191 | 0 | 97/6 | 666 calls |
| **SUNDIALS CVODE (Band)** | ✅ Success | 1.567e-03 | 1.187 | 142 | 2 | 119/2 | 200 calls |
| **Julia FBDF** | ✅ Success | 1.074e-03 | 2.515 | 324 | 2 | 112/5 | GC managed |
| **Julia Rodas5P** | ✅ Success | 2.354e-04 | 3.046 | 440 | 46 | 46/3 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | 1.567e-03 | 1.011 | 142 | 3 | 117/2 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 2.105e-04 | 0.603 | 319 | 49 | 49/5 | 108 calls |
| **Hairer RADAU5-Band** | ✅ Success | 6.664e-05 | 0.614 | 297 | 33 | 39/3 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 4.544e-07 | 2.081 | 1053 | 129 | 129/3 | 0 (Zero-Alloc) |
| **VigilODE (Dense)** | ✅ Success | 4.544e-07 | 3.503 | 1053 | 129 | 129/3 | 0 (Zero-Alloc) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 1.182e-05 | 0.738 | 236 | 4 | 196/6 | 108 calls |
| **Diffsol BDF (Sparse)** | ✅ Success | 4.310e-05 | 1.047 | 292 | 0 | 203/8 | 1849 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 4.310e-05 | 3.654 | 292 | 0 | 203/8 | 900 calls |
| **SUNDIALS CVODE (Band)** | ✅ Success | 1.183e-05 | 2.127 | 236 | 4 | 196/6 | 200 calls |
| **Julia FBDF** | ✅ Success | 1.775e-05 | 3.432 | 491 | 1 | 215/8 | GC managed |
| **Julia Rodas5P** | ✅ Success | 5.627e-06 | 6.827 | 974 | 100 | 100/9 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | 1.183e-05 | 2.108 | 236 | 4 | 190/6 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 1.003e-06 | 1.449 | 789 | 129 | 129/3 | 108 calls |
| **Hairer RADAU5-Band** | ✅ Success | 1.683e-06 | 1.037 | 506 | 65 | 71/2 | 109 calls |

### Tolerance: $rtol = 10^{-2}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Hot-Loop Alloc |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 6.258e-08 | 3.671 | 1859 | 228 | 228/5 | 0 (Zero-Alloc) |
| **VigilODE (Dense)** | ✅ Success | 6.258e-08 | 6.160 | 1859 | 228 | 228/5 | 0 (Zero-Alloc) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 6.182e-07 | 1.291 | 428 | 7 | 380/6 | 108 calls |
| **Diffsol BDF (Sparse)** | ✅ Success | 4.659e-07 | 1.586 | 530 | 0 | 434/4 | 2239 calls |
| **Diffsol BDF (Dense)** | ✅ Success | 4.659e-07 | 4.563 | 530 | 0 | 434/4 | 1352 calls |
| **SUNDIALS CVODE (Band)** | ✅ Success | 6.182e-07 | 3.788 | 428 | 7 | 380/6 | 200 calls |
| **Julia FBDF** | ✅ Success | 3.884e-07 | 5.812 | 854 | 1 | 493/9 | GC managed |
| **Julia Rodas5P** | ✅ Success | 5.583e-08 | 14.917 | 2134 | 228 | 228/10 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | 6.182e-07 | 3.648 | 428 | 7 | 374/6 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 9.897e-09 | 4.126 | 2322 | 387 | 387/0 | 108 calls |
| **Hairer RADAU5-Band** | ✅ Success | 7.408e-08 | 1.913 | 985 | 121 | 142/1 | 109 calls |
