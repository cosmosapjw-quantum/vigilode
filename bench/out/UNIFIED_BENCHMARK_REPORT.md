# VigilODE Comprehensive Cross-Solver Benchmark Report

## Provenance and Execution Manifest

- **Timing Authority**: `diagnostic_only (verified same physical host)`
- **Rust Dataset**: `rust_bench_results.json` (SHA-256: `3eb05f0dea0a233cfdad65d1e7742069e9f5ce0554509a93f8ae0fb5313fe793`)
- **Rust Execution Host**: `AMD Ryzen 9 5900X 12-Core Processor (24 CPUs)`
- **Julia Dataset**: `julia_bench_results.json` (SHA-256: `580e8b9153e1d4b73e3e382638161469e3e76f71b1dc92d796ee608fbe1d5d50`)
- **Julia Execution Host**: `AMD Ryzen 9 5900X 12-Core Processor (24 CPUs)`
- **Same-Host Speed Authority**: `PASS`
- **Error Metric**: Max WRMS error on common 101-point uniform grid against reference solution ($rtol=10^{-12}$).
- **Allocation Scope**: Full-run window (`setup + integration + grid states + teardown`), measured via C `malloc`/`calloc` interposition hook.

## Problem: `robertson`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.671e-07 | 0.083 | 902 | 111 | 111/2 | 265 calls (22.5 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.049 | 342 | 36 | 36/2 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 1.697e-04 | 0.039 | 155 | 3 | 109/3 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 9.432e-05 | 0.074 | 169 | 0 | 92/1 | 669 calls (64.6 KB) |
| **Julia FBDF** | ✅ Success | — | 0.201 | 236 | 2 | 96/4 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.863e-04 | 0.159 | 207 | 4 | 138/8 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.106 | 207 | 4 | 130/8 | GC managed |
| **Hairer RODAS** | ✅ Success | 2.191e-05 | 0.017 | 192 | 32 | 32/0 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 1.511e-05 | 0.018 | 187 | 24 | 28/1 | 109 calls (8.1 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 5.188e-08 | 0.091 | 1004 | 122 | 122/4 | 276 calls (23.2 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.079 | 612 | 66 | 66/2 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 3.366e-06 | 0.077 | 337 | 5 | 237/19 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 1.497e-06 | 0.105 | 260 | 0 | 168/0 | 871 calls (84.0 KB) |
| **Julia FBDF** | ✅ Success | — | 0.347 | 436 | 2 | 189/18 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.029e-06 | 0.226 | 304 | 4 | 231/7 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.144 | 304 | 5 | 224/7 | GC managed |
| **Hairer RODAS** | ✅ Success | 3.982e-07 | 0.037 | 485 | 80 | 80/1 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 8.013e-07 | 0.028 | 338 | 40 | 47/2 | 109 calls (8.1 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.598e-09 | 0.120 | 1347 | 164 | 164/5 | 318 calls (26.2 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.174 | 1484 | 162 | 162/3 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 5.523e-08 | 0.120 | 535 | 7 | 403/22 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.862e-08 | 0.249 | 511 | 0 | 331/11 | 1450 calls (138.4 KB) |
| **Julia FBDF** | ✅ Success | — | 0.621 | 702 | 2 | 374/14 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 5.151e-08 | 0.417 | 554 | 8 | 403/29 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.231 | 554 | 8 | 374/29 | GC managed |
| **Hairer RODAS** | ✅ Success | 9.252e-09 | 0.100 | 1445 | 240 | 240/1 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 4.445e-08 | 0.047 | 639 | 68 | 90/2 | 109 calls (8.1 KB) |

## Problem: `hires`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.274e-05 | 0.188 | 1119 | 132 | 132/9 | 286 calls (86.2 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.170 | 819 | 81 | 81/11 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 7.920e-04 | 0.194 | 555 | 10 | 356/27 | 108 calls (17.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 4.061e-04 | 0.237 | 488 | 0 | 189/11 | 1322 calls (167.3 KB) |
| **Julia FBDF** | ✅ Success | — | 0.409 | 618 | 8 | 181/16 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.061e-04 | 0.409 | 382 | 9 | 238/13 | 200 calls (29.1 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.236 | 383 | 9 | 225/13 | GC managed |
| **Hairer RODAS** | ✅ Success | 5.157e-05 | 0.087 | 562 | 92 | 92/2 | 108 calls (17.3 KB) |
| **Hairer RADAU5** | ✅ Success | 3.158e-05 | 0.097 | 622 | 37 | 61/0 | 109 calls (18.3 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.435e-07 | 0.344 | 2141 | 265 | 265/3 | 419 calls (152.7 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.517 | 2524 | 258 | 258/25 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 8.976e-06 | 0.277 | 827 | 11 | 496/45 | 108 calls (17.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 5.554e-06 | 0.397 | 884 | 0 | 363/15 | 2071 calls (251.1 KB) |
| **Julia FBDF** | ✅ Success | — | 0.867 | 1440 | 6 | 395/20 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 8.504e-06 | 0.904 | 825 | 12 | 513/46 | 200 calls (29.1 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.409 | 826 | 12 | 467/46 | GC managed |
| **Hairer RODAS** | ✅ Success | 7.625e-07 | 0.325 | 2219 | 369 | 369/1 | 108 calls (17.3 KB) |
| **Hairer RADAU5** | ✅ Success | 6.856e-06 | 0.169 | 1140 | 73 | 117/2 | 109 calls (18.3 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.180e-09 | 1.051 | 6633 | 823 | 823/7 | 977 calls (431.7 KB) |
| **Julia Rodas5P** | ✅ Success | — | 1.620 | 8091 | 873 | 873/29 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 1.668e-07 | 0.465 | 1347 | 17 | 958/62 | 108 calls (17.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 8.148e-08 | 0.602 | 1396 | 0 | 725/7 | 2827 calls (280.5 KB) |
| **Julia FBDF** | ✅ Success | — | 1.816 | 2398 | 7 | 898/43 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 1.655e-07 | 1.706 | 1512 | 19 | 1103/62 | 200 calls (29.1 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.713 | 1512 | 19 | 1041/62 | GC managed |
| **Hairer RODAS** | ✅ Success | 8.460e-09 | 1.813 | 12617 | 2102 | 2102/1 | 108 calls (17.3 KB) |
| **Hairer RADAU5** | ✅ Success | 1.008e-06 | 0.296 | 2050 | 134 | 235/1 | 109 calls (18.3 KB) |

## Problem: `van-der-pol-mu1000`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.442e-05 | 0.181 | 2446 | 234 | 234/82 | 388 calls (21.0 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.206 | 2109 | 163 | 163/80 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 3.710e-03 | 0.139 | 808 | 22 | 495/58 | 108 calls (6.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.858e-04 | 0.338 | 1029 | 0 | 428/67 | 3272 calls (275.2 KB) |
| **Julia FBDF** | ✅ Success | — | 0.782 | 1953 | 31 | 449/113 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.718e-03 | 0.448 | 780 | 19 | 487/55 | 200 calls (17.7 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.277 | 779 | 19 | 432/55 | GC managed |
| **Hairer RODAS** | ✅ Success | 1.331e-04 | 0.072 | 1350 | 210 | 210/18 | 108 calls (6.1 KB) |
| **Hairer RADAU5** | ✅ Success | 9.541e-05 | 0.085 | 1739 | 139 | 188/8 | 109 calls (6.2 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 4.598e-07 | 0.283 | 3904 | 411 | 411/88 | 565 calls (26.5 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.451 | 4759 | 381 | 381/166 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 2.547e-04 | 0.240 | 1352 | 22 | 925/89 | 108 calls (6.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 3.977e-05 | 0.547 | 1777 | 0 | 817/115 | 4803 calls (425.3 KB) |
| **Julia FBDF** | ✅ Success | — | 1.580 | 3342 | 13 | 936/153 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.539e-04 | 0.852 | 1354 | 23 | 925/90 | 200 calls (17.7 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.463 | 1354 | 23 | 835/90 | GC managed |
| **Hairer RODAS** | ✅ Success | 6.608e-06 | 0.187 | 3708 | 608 | 608/12 | 108 calls (6.1 KB) |
| **Hairer RADAU5** | ✅ Success | 2.470e-06 | 0.141 | 3008 | 247 | 350/11 | 109 calls (6.2 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.237e-08 | 0.570 | 7960 | 960 | 960/40 | 1114 calls (43.7 KB) |
| **Julia Rodas5P** | ✅ Success | — | 0.969 | 10569 | 975 | 975/224 | GC managed |
| **Netlib DVODE (MF=21)** | ❌ DVODE ISTATE -1 | — | 0.220 | 1209 | 16 | 818/80 | 48 calls (3.8 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.271e-06 | 0.940 | 3275 | 0 | 1776/105 | 7716 calls (671.3 KB) |
| **Julia FBDF** | ✅ Success | — | 2.956 | 5796 | 11 | 1944/185 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.502e-06 | 1.878 | 2860 | 37 | 2023/169 | 200 calls (17.7 KB) |
| **Julia CVODE_BDF** | ✅ Success | — | 0.970 | 2858 | 37 | 1854/169 | GC managed |
| **Hairer RODAS** | ✅ Success | 5.991e-08 | 0.583 | 11916 | 1981 | 1981/6 | 108 calls (6.1 KB) |
| **Hairer RADAU5** | ✅ Success | 1.348e-07 | 0.258 | 5539 | 360 | 715/7 | 109 calls (6.2 KB) |

## Problem: `brusselator-1d-n100`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 2.213e-05 | 1.681 | 847 | 105 | 105/1 | 155 calls (114.3 KB) |
| **VigilODE (Dense)** | ✅ Success | 2.213e-05 | 2.809 | 847 | 105 | 105/1 | 259 calls (8.3 MB) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 1.492e-03 | 0.458 | 148 | 3 | 122/3 | 108 calls (178.9 KB) |
| **Diffsol BDF (Sparse)** | ✅ Success | 1.768e-03 | 0.732 | 191 | 0 | 97/6 | 1499 calls (1.4 MB) |
| **Diffsol BDF (Dense)** | ✅ Success | 1.768e-03 | 2.688 | 191 | 0 | 97/6 | 666 calls (3.9 MB) |
| **SUNDIALS CVODE (Band)** | ✅ Success | 1.567e-03 | 1.211 | 142 | 2 | 119/2 | 200 calls (200.1 KB) |
| **Julia FBDF** | ✅ Success | — | 2.077 | 324 | 2 | 112/5 | GC managed |
| **Julia Rodas5P** | ✅ Success | — | 2.428 | 440 | 46 | 46/3 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | — | 0.829 | 142 | 3 | 117/2 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 2.105e-04 | 0.600 | 319 | 49 | 49/5 | 108 calls (182.7 KB) |
| **Hairer RADAU5-Band** | ✅ Success | 6.664e-05 | 0.612 | 297 | 33 | 39/3 | 109 calls (193.7 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 4.544e-07 | 2.140 | 1053 | 129 | 129/3 | 155 calls (114.3 KB) |
| **VigilODE (Dense)** | ✅ Success | 4.544e-07 | 3.579 | 1053 | 129 | 129/3 | 283 calls (10.1 MB) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 1.182e-05 | 0.735 | 236 | 4 | 196/6 | 108 calls (178.9 KB) |
| **Diffsol BDF (Sparse)** | ✅ Success | 4.310e-05 | 1.034 | 292 | 0 | 203/8 | 1849 calls (1.7 MB) |
| **Diffsol BDF (Dense)** | ✅ Success | 4.310e-05 | 3.524 | 292 | 0 | 203/8 | 900 calls (4.8 MB) |
| **SUNDIALS CVODE (Band)** | ✅ Success | 1.183e-05 | 2.139 | 236 | 4 | 196/6 | 200 calls (200.1 KB) |
| **Julia FBDF** | ✅ Success | — | 2.787 | 491 | 1 | 215/8 | GC managed |
| **Julia Rodas5P** | ✅ Success | — | 5.397 | 974 | 100 | 100/9 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | — | 1.335 | 236 | 4 | 190/6 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 1.003e-06 | 1.435 | 789 | 129 | 129/3 | 108 calls (182.7 KB) |
| **Hairer RADAU5-Band** | ✅ Success | 1.683e-06 | 1.028 | 506 | 65 | 71/2 | 109 calls (193.7 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 6.258e-08 | 3.612 | 1859 | 228 | 228/5 | 155 calls (114.3 KB) |
| **VigilODE (Dense)** | ✅ Success | 6.258e-08 | 6.113 | 1859 | 228 | 228/5 | 382 calls (17.6 MB) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 6.182e-07 | 1.279 | 428 | 7 | 380/6 | 108 calls (178.9 KB) |
| **Diffsol BDF (Sparse)** | ✅ Success | 4.659e-07 | 1.572 | 530 | 0 | 434/4 | 2239 calls (2.0 MB) |
| **Diffsol BDF (Dense)** | ✅ Success | 4.659e-07 | 4.524 | 530 | 0 | 434/4 | 1352 calls (4.8 MB) |
| **SUNDIALS CVODE (Band)** | ✅ Success | 6.182e-07 | 3.819 | 428 | 7 | 380/6 | 200 calls (200.1 KB) |
| **Julia FBDF** | ✅ Success | — | 4.382 | 854 | 1 | 493/9 | GC managed |
| **Julia Rodas5P** | ✅ Success | — | 11.835 | 2134 | 228 | 228/10 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | — | 2.237 | 428 | 7 | 374/6 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 9.897e-09 | 4.104 | 2322 | 387 | 387/0 | 108 calls (182.7 KB) |
| **Hairer RADAU5-Band** | ✅ Success | 7.408e-08 | 1.901 | 985 | 121 | 142/1 | 109 calls (193.7 KB) |
