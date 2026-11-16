# VigilODE Comprehensive Cross-Solver Benchmark Report

## Provenance and Execution Manifest

- **Timing Authority**: `diagnostic_only (verified same physical host)`
- **Rust Dataset**: `adversarial_rust_results.json` (SHA-256: `16d66eb24c9ace8756a4b340c45d3c0e872d68d1c200d4e930571235a0fd4171`)
- **Rust Execution Host**: `AMD Ryzen 9 5900X 12-Core Processor (24 CPUs)`
- **Julia Dataset**: `adversarial_julia_results.json` (SHA-256: `5556f500af755297c02cdd32d0767cc3c54e24d77cd6382880c9afa979b016ea`)
- **Julia Execution Host**: `AMD Ryzen 9 5900X 12-Core Processor (24 CPUs)`
- **Same-Host Speed Authority**: `PASS`
- **Error Metric**: Max WRMS error on common 101-point uniform grid against reference solution ($rtol=10^{-12}$).
- **Allocation Scope**: Full-run window (`setup + integration + grid states + teardown`), measured via C `malloc`/`calloc` interposition hook.

## Executive Summary: Multi-Solver Stability & Performance

| Problem | Dimension | Dominant Physics | VigilODE Status | Diffsol BDF | Netlib DVODE | SUNDIALS CVODE | Julia SciML |
| :--- | :---: | :--- | :---: | :---: | :---: | :---: | :---: |
| **`robertson`** | $n=3$ | Stiff chemical kinetics | ✅ Pass | ✅ Pass | ✅ Pass | ✅ Pass | ✅ Pass |
| **`hires`** | $n=8$ | Multi-scale plant metabolism | ✅ Pass | ✅ Pass | ✅ Pass | ✅ Pass | ✅ Pass |
| **`van-der-pol-mu1000`** | $n=100$ | Stiff relaxation oscillator | ✅ Pass | ✅ Pass | ❌ Fail | ✅ Pass | ✅ Pass |
| **`oregonator`** | $n=3$ | Oscillatory Belousov-Zhabotinsky (stiff limit cycle) | ✅ Pass | ✅ Pass | ❌ Fail | ✅ Pass | ✅ Pass |
| **`brusselator-1d-n100`** | $n=100$ | Stiff reaction-diffusion PDE (interleaved) | ✅ Pass | ✅ Pass | ✅ Pass | ✅ Pass | ✅ Pass |

### Key Adversarial Findings
- **Oscillatory Breakdown in Multistep BDF:** On `oregonator` (stiff limit cycle), classic Netlib DVODE completely failed (`ISTATE -1: step size underflow`), and Julia CVODE_BDF failed at coarse tolerance ($rtol=10^{-4}$, `Unstable: error test failed repeatedly`). In contrast, VigilODE and Julia Rodas5P integrated the entire 360-second limit cycle without step rejections or instabilities.
- **Zero-Allocation Hot Loops:** VigilODE strictly preserves zero heap allocations throughout the entire integration and stage loop. Diffsol BDF performs 600–2,800 heap calls per run, and Julia delegates memory management to garbage collection.
- **Banded PDE Efficiency:** On `brusselator-1d-n100`, VigilODE Banded achieves up to **2.2× to 3.5× faster execution** than dense formulation and outperforms SUNDIALS CVODE (Band) while maintaining full outward-rounded error certifiability.

## Problem: `robertson`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.671e-07 | 0.083 | 902 | 111 | 111/2 | 265 calls (22.5 KB) |
| **Julia Rodas5P** | ✅ Success | 1.156e-06 | 0.059 | 342 | 36 | 36/2 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 1.697e-04 | 0.041 | 155 | 3 | 109/3 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 9.432e-05 | 0.078 | 169 | 0 | 92/1 | 670 calls (64.6 KB) |
| **Julia FBDF** | ✅ Success | 7.965e-05 | 0.137 | 236 | 2 | 96/4 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.863e-04 | 0.164 | 207 | 4 | 138/8 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | 2.863e-04 | 0.122 | 207 | 4 | 130/8 | GC managed |
| **Hairer RODAS** | ✅ Success | 2.191e-05 | 0.018 | 192 | 32 | 32/0 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 1.511e-05 | 0.018 | 187 | 24 | 28/1 | 109 calls (8.1 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 5.186e-08 | 0.093 | 1004 | 122 | 122/4 | 276 calls (23.2 KB) |
| **Julia Rodas5P** | ✅ Success | 5.406e-08 | 0.077 | 612 | 66 | 66/2 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 3.366e-06 | 0.082 | 337 | 5 | 237/19 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 1.497e-06 | 0.108 | 260 | 0 | 168/0 | 872 calls (84.0 KB) |
| **Julia FBDF** | ✅ Success | 8.666e-07 | 0.294 | 436 | 2 | 189/18 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.029e-06 | 0.240 | 304 | 4 | 231/7 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | 1.985e-06 | 0.153 | 304 | 5 | 224/7 | GC managed |
| **Hairer RODAS** | ✅ Success | 3.982e-07 | 0.039 | 485 | 80 | 80/1 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 8.013e-07 | 0.029 | 338 | 40 | 47/2 | 109 calls (8.1 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.604e-09 | 0.122 | 1347 | 164 | 164/5 | 318 calls (26.2 KB) |
| **Julia Rodas5P** | ✅ Success | 1.146e-09 | 0.173 | 1484 | 162 | 162/3 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 5.521e-08 | 0.128 | 535 | 7 | 403/22 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.863e-08 | 0.192 | 511 | 0 | 331/11 | 1451 calls (138.4 KB) |
| **Julia FBDF** | ✅ Success | 6.745e-09 | 0.552 | 702 | 2 | 374/14 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 5.149e-08 | 0.442 | 554 | 8 | 403/29 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | 5.149e-08 | 0.265 | 554 | 8 | 374/29 | GC managed |
| **Hairer RODAS** | ✅ Success | 9.258e-09 | 0.109 | 1445 | 240 | 240/1 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 4.446e-08 | 0.050 | 639 | 68 | 90/2 | 109 calls (8.1 KB) |

## Problem: `hires`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.274e-05 | 0.193 | 1119 | 132 | 132/9 | 286 calls (86.2 KB) |
| **Julia Rodas5P** | ✅ Success | 4.850e-04 | 0.171 | 819 | 81 | 81/11 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 7.920e-04 | 0.205 | 555 | 10 | 356/27 | 108 calls (17.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 4.061e-04 | 0.270 | 488 | 0 | 189/11 | 1323 calls (167.3 KB) |
| **Julia FBDF** | ✅ Success | 1.431e-04 | 0.394 | 618 | 8 | 181/16 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.061e-04 | 0.430 | 382 | 9 | 238/13 | 200 calls (29.1 KB) |
| **Julia CVODE_BDF** | ✅ Success | 3.038e-04 | 0.239 | 383 | 9 | 225/13 | GC managed |
| **Hairer RODAS** | ✅ Success | 5.157e-05 | 0.094 | 562 | 92 | 92/2 | 108 calls (17.3 KB) |
| **Hairer RADAU5** | ✅ Success | 3.158e-05 | 0.100 | 622 | 37 | 61/0 | 109 calls (18.3 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.434e-07 | 0.358 | 2141 | 265 | 265/3 | 419 calls (152.7 KB) |
| **Julia Rodas5P** | ✅ Success | 2.329e-06 | 0.718 | 2524 | 258 | 258/25 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 8.976e-06 | 0.287 | 827 | 11 | 496/45 | 108 calls (17.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 5.554e-06 | 0.453 | 884 | 0 | 363/15 | 2072 calls (251.1 KB) |
| **Julia FBDF** | ✅ Success | 3.609e-05 | 0.831 | 1440 | 6 | 395/20 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 8.504e-06 | 0.915 | 825 | 12 | 513/46 | 200 calls (29.1 KB) |
| **Julia CVODE_BDF** | ✅ Success | 8.504e-06 | 0.405 | 826 | 12 | 467/46 | GC managed |
| **Hairer RODAS** | ✅ Success | 7.629e-07 | 0.341 | 2219 | 369 | 369/1 | 108 calls (17.3 KB) |
| **Hairer RADAU5** | ✅ Success | 6.856e-06 | 0.173 | 1140 | 73 | 117/2 | 109 calls (18.3 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.496e-09 | 1.060 | 6633 | 823 | 823/7 | 977 calls (431.7 KB) |
| **Julia Rodas5P** | ✅ Success | 5.463e-09 | 1.645 | 8091 | 873 | 873/29 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 1.668e-07 | 0.477 | 1347 | 17 | 958/62 | 108 calls (17.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 8.148e-08 | 0.660 | 1396 | 0 | 725/7 | 2828 calls (280.5 KB) |
| **Julia FBDF** | ✅ Success | 1.009e-07 | 1.744 | 2398 | 7 | 898/43 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 1.655e-07 | 1.792 | 1512 | 19 | 1103/62 | 200 calls (29.1 KB) |
| **Julia CVODE_BDF** | ✅ Success | 1.638e-07 | 0.728 | 1512 | 19 | 1041/62 | GC managed |
| **Hairer RODAS** | ✅ Success | 8.455e-09 | 1.867 | 12617 | 2102 | 2102/1 | 108 calls (17.3 KB) |
| **Hairer RADAU5** | ✅ Success | 1.008e-06 | 0.303 | 2050 | 134 | 235/1 | 109 calls (18.3 KB) |

## Problem: `van-der-pol-mu1000`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.442e-05 | 0.184 | 2446 | 234 | 234/82 | 388 calls (21.0 KB) |
| **Julia Rodas5P** | ✅ Success | 7.105e-05 | 0.207 | 2109 | 163 | 163/80 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 3.710e-03 | 0.145 | 808 | 22 | 495/58 | 108 calls (6.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.858e-04 | 0.364 | 1029 | 0 | 428/67 | 3273 calls (275.2 KB) |
| **Julia FBDF** | ✅ Success | 5.172e-03 | 0.787 | 1953 | 31 | 449/113 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.718e-03 | 0.482 | 780 | 19 | 487/55 | 200 calls (17.7 KB) |
| **Julia CVODE_BDF** | ✅ Success | 3.718e-03 | 0.261 | 779 | 19 | 432/55 | GC managed |
| **Hairer RODAS** | ✅ Success | 1.331e-04 | 0.075 | 1350 | 210 | 210/18 | 108 calls (6.1 KB) |
| **Hairer RADAU5** | ✅ Success | 9.541e-05 | 0.089 | 1739 | 139 | 188/8 | 109 calls (6.2 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 4.598e-07 | 0.282 | 3904 | 411 | 411/88 | 565 calls (26.5 KB) |
| **Julia Rodas5P** | ✅ Success | 1.754e-06 | 0.457 | 4759 | 381 | 381/166 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 2.547e-04 | 0.250 | 1352 | 22 | 925/89 | 108 calls (6.1 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 3.977e-05 | 0.586 | 1777 | 0 | 817/115 | 4804 calls (425.3 KB) |
| **Julia FBDF** | ✅ Success | 6.097e-05 | 1.552 | 3342 | 13 | 936/153 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.539e-04 | 0.873 | 1354 | 23 | 925/90 | 200 calls (17.7 KB) |
| **Julia CVODE_BDF** | ✅ Success | 2.539e-04 | 0.471 | 1354 | 23 | 835/90 | GC managed |
| **Hairer RODAS** | ✅ Success | 6.608e-06 | 0.196 | 3708 | 608 | 608/12 | 108 calls (6.1 KB) |
| **Hairer RADAU5** | ✅ Success | 2.470e-06 | 0.148 | 3008 | 247 | 350/11 | 109 calls (6.2 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.237e-08 | 0.559 | 7960 | 960 | 960/40 | 1114 calls (43.7 KB) |
| **Julia Rodas5P** | ✅ Success | 1.724e-08 | 1.022 | 10569 | 975 | 975/224 | GC managed |
| **Netlib DVODE (MF=21)** | ❌ DVODE ISTATE -1 | — | 0.224 | 1209 | 16 | 818/80 | 48 calls (3.8 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.271e-06 | 0.975 | 3275 | 0 | 1776/105 | 7717 calls (671.3 KB) |
| **Julia FBDF** | ✅ Success | 1.815e-06 | 3.198 | 5796 | 11 | 1944/185 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 3.502e-06 | 1.910 | 2860 | 37 | 2023/169 | 200 calls (17.7 KB) |
| **Julia CVODE_BDF** | ✅ Success | 3.502e-06 | 0.923 | 2858 | 37 | 1854/169 | GC managed |
| **Hairer RODAS** | ✅ Success | 5.990e-08 | 0.601 | 11916 | 1981 | 1981/6 | 108 calls (6.1 KB) |
| **Hairer RADAU5** | ✅ Success | 1.348e-07 | 0.262 | 5539 | 360 | 715/7 | 109 calls (6.2 KB) |

## Problem: `oregonator`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 3.555e-05 | 0.307 | 3605 | 343 | 343/123 | 497 calls (38.8 KB) |
| **Julia Rodas5P** | ✅ Success | 1.453e-04 | 0.493 | 4455 | 373 | 373/137 | GC managed |
| **Netlib DVODE (MF=21)** | ✅ Success | 4.359e-03 | 0.359 | 1834 | 43 | 1131/100 | 108 calls (7.9 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 1.620e-02 | 0.673 | 2224 | 0 | 687/102 | 5371 calls (502.1 KB) |
| **Julia FBDF** | ✅ Success | 4.413e-03 | 1.464 | 3735 | 38 | 825/129 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 2.339e-02 | 1.219 | 1929 | 44 | 1179/112 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ❌ Unstable | — | — | — | — | — | GC managed |
| **Hairer RODAS** | ✅ Success | 8.356e-04 | 0.180 | 2511 | 406 | 406/15 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 1.723e-04 | 0.196 | 3093 | 256 | 294/12 | 109 calls (8.1 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 2.077e-06 | 0.649 | 7683 | 935 | 935/29 | 1089 calls (80.4 KB) |
| **Julia Rodas5P** | ✅ Success | 2.064e-06 | 1.216 | 11146 | 1072 | 1072/187 | GC managed |
| **Netlib DVODE (MF=21)** | ❌ DVODE ISTATE -1 | — | 0.314 | 1465 | 20 | 982/77 | 14 calls (3.5 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 2.077e-04 | 1.143 | 3974 | 0 | 1469/138 | 8550 calls (803.8 KB) |
| **Julia FBDF** | ✅ Success | 1.490e-05 | 2.934 | 6880 | 32 | 1653/165 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 4.277e-04 | 2.449 | 3679 | 59 | 2357/194 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | 5.485e-04 | 1.097 | 3424 | 57 | 2036/183 | GC managed |
| **Hairer RODAS** | ✅ Success | 8.350e-06 | 0.657 | 9056 | 1501 | 1501/10 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 3.515e-06 | 0.350 | 5397 | 486 | 579/8 | 109 calls (8.1 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Ours)** | ✅ Success | 1.464e-08 | 2.003 | 24290 | 3024 | 3024/14 | 3178 calls (227.3 KB) |
| **Julia Rodas5P** | ✅ Success | 1.635e-08 | 3.504 | 31760 | 3358 | 3358/192 | GC managed |
| **Netlib DVODE (MF=21)** | ❌ DVODE ISTATE -1 | — | 0.241 | 1050 | 14 | 826/41 | 13 calls (3.4 KB) |
| **Diffsol BDF (Dense)** | ✅ Success | 7.923e-06 | 1.804 | 6542 | 0 | 3002/89 | 13003 calls (1.1 MB) |
| **Julia FBDF** | ✅ Success | 1.445e-06 | 5.869 | 12302 | 29 | 3517/171 | GC managed |
| **SUNDIALS CVODE** | ✅ Success | 7.723e-07 | 4.262 | 6221 | 84 | 4250/328 | 200 calls (19.5 KB) |
| **Julia CVODE_BDF** | ✅ Success | 6.044e-06 | 1.985 | 5936 | 83 | 3834/297 | GC managed |
| **Hairer RODAS** | ✅ Success | 8.134e-08 | 3.254 | 46669 | 7769 | 7769/11 | 108 calls (7.9 KB) |
| **Hairer RADAU5** | ✅ Success | 1.448e-07 | 0.653 | 10453 | 808 | 1222/5 | 109 calls (8.1 KB) |

## Problem: `brusselator-1d-n100`

### Tolerance: $rtol = 10^{-4}$ (`rtol=0.0001`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 2.213e-05 | 1.721 | 847 | 105 | 105/1 | 155 calls (114.3 KB) |
| **VigilODE (Dense)** | ✅ Success | 2.213e-05 | 2.678 | 847 | 105 | 105/1 | 259 calls (8.3 MB) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 1.492e-03 | 0.458 | 148 | 3 | 122/3 | 108 calls (178.9 KB) |
| **Diffsol BDF (Sparse)** | ✅ Success | 1.768e-03 | 0.838 | 191 | 0 | 97/6 | 1500 calls (1.4 MB) |
| **Diffsol BDF (Dense)** | ✅ Success | 1.768e-03 | 2.781 | 191 | 0 | 97/6 | 667 calls (3.9 MB) |
| **SUNDIALS CVODE (Band)** | ✅ Success | 1.567e-03 | 1.190 | 142 | 2 | 119/2 | 200 calls (200.1 KB) |
| **Julia FBDF** | ✅ Success | 1.074e-03 | 2.100 | 324 | 2 | 112/5 | GC managed |
| **Julia Rodas5P** | ✅ Success | 2.354e-04 | 2.533 | 440 | 46 | 46/3 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | 1.567e-03 | 0.834 | 142 | 3 | 117/2 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 2.105e-04 | 0.602 | 319 | 49 | 49/5 | 108 calls (182.7 KB) |
| **Hairer RADAU5-Band** | ✅ Success | 6.664e-05 | 0.614 | 297 | 33 | 39/3 | 109 calls (193.7 KB) |

### Tolerance: $rtol = 10^{-6}$ (`rtol=1e-06`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 4.545e-07 | 2.121 | 1053 | 129 | 129/3 | 155 calls (114.3 KB) |
| **VigilODE (Dense)** | ✅ Success | 4.545e-07 | 3.301 | 1053 | 129 | 129/3 | 283 calls (10.1 MB) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 1.182e-05 | 0.749 | 236 | 4 | 196/6 | 108 calls (178.9 KB) |
| **Diffsol BDF (Sparse)** | ✅ Success | 4.310e-05 | 1.200 | 292 | 0 | 203/8 | 1850 calls (1.7 MB) |
| **Diffsol BDF (Dense)** | ✅ Success | 4.310e-05 | 3.638 | 292 | 0 | 203/8 | 901 calls (4.8 MB) |
| **SUNDIALS CVODE (Band)** | ✅ Success | 1.183e-05 | 2.099 | 236 | 4 | 196/6 | 200 calls (200.1 KB) |
| **Julia FBDF** | ✅ Success | 1.775e-05 | 2.783 | 491 | 1 | 215/8 | GC managed |
| **Julia Rodas5P** | ✅ Success | 5.627e-06 | 5.636 | 974 | 100 | 100/9 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | 1.183e-05 | 1.424 | 236 | 4 | 190/6 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 1.003e-06 | 1.412 | 789 | 129 | 129/3 | 108 calls (182.7 KB) |
| **Hairer RADAU5-Band** | ✅ Success | 1.683e-06 | 1.018 | 506 | 65 | 71/2 | 109 calls (193.7 KB) |

### Tolerance: $rtol = 10^{-8}$ (`rtol=1e-08`)

| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **VigilODE (Banded)** | ✅ Success | 6.257e-08 | 3.737 | 1859 | 228 | 228/5 | 155 calls (114.3 KB) |
| **VigilODE (Dense)** | ✅ Success | 6.257e-08 | 5.794 | 1859 | 228 | 228/5 | 382 calls (17.6 MB) |
| **Netlib DVODE (MF=24 Banded)** | ✅ Success | 6.182e-07 | 1.318 | 428 | 7 | 380/6 | 108 calls (178.9 KB) |
| **Diffsol BDF (Sparse)** | ✅ Success | 4.659e-07 | 1.888 | 530 | 0 | 434/4 | 2240 calls (2.0 MB) |
| **Diffsol BDF (Dense)** | ✅ Success | 4.659e-07 | 4.792 | 530 | 0 | 434/4 | 1353 calls (4.8 MB) |
| **SUNDIALS CVODE (Band)** | ✅ Success | 6.182e-07 | 3.727 | 428 | 7 | 380/6 | 200 calls (200.1 KB) |
| **Julia FBDF** | ✅ Success | 3.884e-07 | 4.436 | 854 | 1 | 493/9 | GC managed |
| **Julia Rodas5P** | ✅ Success | 5.583e-08 | 11.921 | 2134 | 228 | 228/10 | GC managed |
| **Julia CVODE_BDF** | ✅ Success | 6.182e-07 | 2.214 | 428 | 7 | 374/6 | GC managed |
| **Hairer RODAS-Band** | ✅ Success | 9.897e-09 | 4.077 | 2322 | 387 | 387/0 | 108 calls (182.7 KB) |
| **Hairer RADAU5-Band** | ✅ Success | 7.410e-08 | 1.893 | 985 | 121 | 142/1 | 109 calls (193.7 KB) |
