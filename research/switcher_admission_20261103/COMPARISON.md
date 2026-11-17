# Before/after: adapter work (authority) and wall time (diagnostic only)

before: commit `596dcdfae8b95d24f27b57975d9f4626267b8149` binaries `{'evidence_switch_bench': '0b6c5e117091828e9a53a6eca4fcefea4682fa9400a5619ad7b672779a581cb5', 'local_bottleneck_suite': '8d8d61895e290344d64a92032c0275dbf70a502a65071dfdb5b64cff8d8d6d7f'}`

after: commit `a4b08c5970247ab4fc435bb6ef269518fc1d3c3d` binaries `{'evidence_switch_bench': 'f46bb4455401190475fcce53fb7688c96f9934d5001b849281e49e31aa5181f4', 'local_bottleneck_suite': 'f4ca617911334439f98a5d8969a2867a69343a95ed109c873b2d6113f3c6d4eb'}`

## G1 — untouched direct paths are bit-identical (counters and final state)

| direct cell | attempts | RHS | JVP | solves | final state bitwise | wall ms before→after (drift) |
|---|---|---|---|---|---|---|
| bench:rotation:dopri | 875/875 | 5251/5251 | 0/0 | 0/0 | yes | 0.223→0.240 (1.07x) |
| bench:rotation:rodas | 640/640 | 5022/5022 | 20453/20453 | 5120/5120 | yes | 6.566→7.703 (1.17x) |
| bench:stiff-forced:dopri | 3111/3111 | 18667/18667 | 0/0 | 0/0 | yes | 0.866→1.203 (1.39x) |
| bench:stiff-forced:rodas | 14/14 | 112/112 | 0/0 | 112/112 | yes | 0.080→0.095 (1.19x) |
| suite:c5:rodas5p-mf | 39/39 | 307/307 | 1206/1206 | 312/312 | yes | 0.594→0.403 (0.68x) |
| suite:c5:rodas5p-dense | 39/39 | 307/307 | 0/0 | 312/312 | yes | 0.122→0.116 (0.95x) |
| suite:c10t:dopri5 | 4067/4067 | 24403/24403 | 0/0 | 0/0 | yes | 4.613→2.454 (0.53x) |
| suite:c10t:rodas5p-mf | 9454/9454 | 75314/75314 | 378160/378160 | 75632/75632 | yes | 194.169→154.309 (0.79x) |
| suite:c10:dopri5 | 1105/1105 | 6631/6631 | 0/0 | 0/0 | yes | 0.770→0.689 (0.89x) |

**G1 = True**

## G2/G3 — switching cells: deterministic work never higher, accuracy caps kept

| cell | attempts b→a | RHS b→a | pilots b→a (attempts) | probes b→a (attempts) | RODAS batches b→a (attempts) | error b→a (cap) | success/pass | wall ms b→a | best direct ms b→a | ratio b→a |
|---|---|---|---|---|---|---|---|---|---|---|
| bench:rotation:evidence | 892→892 | 5371→5371 | 3(15)→3(15) | 0(0)→0(0) | 0(0)→0(0) | 9.26e-07→9.26e-07 (1e-05) | True/True | 0.863→0.501 | 0.223→0.240 | 3.86→2.09 |
| bench:rotation:screened-reset | 1105→1105 | 6710→6710 | 7(35)→7(35) | 0(0)→0(0) | 0(0)→0(0) | 9.06e-07→9.06e-07 (1e-05) | True/True | 0.701→1.064 | 0.223→0.240 | 3.14→4.43 |
| bench:rotation:pressure-only | 666→666 | 5144→5144 | 0(0)→0(0) | 1(16)→1(16) | 3(618)→3(618) | 8.37e-08→8.37e-08 (1e-05) | True/True | 6.290→6.287 | 0.223→0.240 | 28.16→26.19 |
| bench:stiff-forced:evidence | 66→66 | 456→456 | 1(8)→1(8) | 1(5)→1(5) | 3(21)→3(21) | 7.33e-10→7.33e-10 (1e-05) | True/True | 0.212→0.138 | 0.080→0.095 | 2.65→1.45 |
| bench:stiff-forced:screened-reset | 66→66 | 456→456 | 1(8)→1(8) | 1(5)→1(5) | 3(21)→3(21) | 7.33e-10→7.33e-10 (1e-05) | True/True | 0.140→0.127 | 0.080→0.095 | 1.75→1.33 |
| bench:stiff-forced:pressure-only | 58→58 | 392→392 | 0(0)→0(0) | 1(5)→1(5) | 3(21)→3(21) | 7.33e-10→7.33e-10 (1e-05) | True/True | 0.121→0.150 | 0.080→0.095 | 1.51→1.57 |
| suite:c5:switching | 115→115 | 748→748 | 2(6)→2(6) | 2(19)→2(19) | 4(26)→4(26) | 1.25e-08→1.25e-08 (1e-04) | True/True | 0.820→0.614 | 0.122→0.116 | 6.73→5.30 |
| suite:c10t:switching | 4159→4159 | 25102→25102 | 4(16)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.89e-05→3.89e-05 (1e-04) | True/True | 9.216→5.022 | 4.613→2.454 | 2.00→2.05 |
| suite:c10:switching | 1209→1209 | 7309→7309 | 4(16)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.09e-02→3.09e-02 (1e-04) | True/False | 2.114→1.872 | 0.770→0.689 | 2.74→2.72 |

**G2 = True, G3 = True**

## Decision sequences (after)

- **bench:rotation:evidence** (switches=0): `0:Rpilot![5] 1:D[32] 2:D[32] 3:D[32] 4:D[16] 5:D[32] 6:Rpilot![5] 7:D[32] 8:D[32] 9:D[12] 10:D[32] 11:D[32] 12:D[32] 13:D[14] 14:D[32] 15:D[32] 16:Rpilot![5] 17:D[32] 18:D[13] 19:D[32] 20:D[32] 21:D[32] 22:D[14] 23:D[32] 24:D[32] 25:D[32] 26:D[13] 27:D[32] 28:D[32] 29:D[32] 30:D[14] 31:D[32] 32:D[32] 33:D[32] 34:D[13]`
- **bench:rotation:screened-reset** (switches=0): `0:Rpilot![5] 1:D[32] 2:D[32] 3:D[32] 4:D[32] 5:D[7] 6:Rpilot![5] 7:D[32] 8:D[32] 9:D[32] 10:D[31] 11:D[32] 12:Rpilot![5] 13:D[32] 14:D[32] 15:D[31] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[7] 21:Rpilot![5] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[10] 27:Rpilot![5] 28:D[32] 29:D[32] 30:D[32] 31:D[32] 32:D[8] 33:Rpilot![5] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[7] 39:Rpilot![5] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[9]`
- **bench:rotation:pressure-only** (switches=1): `0:D[32] 1:R[141] 2:R[156] 3:Dprobe![16] 4:R[321]`
- **bench:stiff-forced:evidence** (switches=1): `0:Rpilot![8] 1:D[32] 2:R[12] 3:R[4] 4:Dprobe![5] 5:R[5]`
- **bench:stiff-forced:screened-reset** (switches=1): `0:Rpilot![8] 1:D[32] 2:R[12] 3:R[4] 4:Dprobe![5] 5:R[5]`
- **bench:stiff-forced:pressure-only** (switches=1): `0:D[32] 1:R[12] 2:R[4] 3:Dprobe![5] 4:R[5]`
- **suite:c5:switching** (switches=1): `0:Rpilot![5] 1:D[32] 2:Rpilot![1] 3:D[32] 4:R[9] 5:R[3] 6:Dprobe![10] 7:R[5] 8:Dprobe![9] 9:R[9]`
- **suite:c10t:switching** (switches=0): `0:Rpilot![4] 1:D[32] 2:Rpilot![4] 3:D[32] 4:D[32] 5:D[32] 6:Rpilot![4] 7:D[32] 8:D[32] 9:D[32] 10:D[32] 11:D[6] 12:Rpilot![4] 13:D[32] 14:D[32] 15:D[32] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[32] 21:D[3] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[32] 27:D[32] 28:D[32] 29:D[32] 30:D[6] 31:D[32] 32:D[32] 33:D[32] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[32] 39:D[4] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[32] 45:D[32] 46:D[32] 47:D[32] 48:D[5] 49:D[32] 50:D[32] 51:D[32] 52:D[32] 53:D[32] 54:D[32] 55:D[32] 56:D[31] 57:D[32] 58:D[32] 59:D[32] 60:D[32] 61:D[32] 62:D[32] 63:D[32] 64:D[32] 65:D[2] 66:D[32] 67:D[32] 68:D[32] 69:D[32] 70:D[32] 71:D[32] 72:D[32] 73:D[32] 74:D[3] 75:D[32] 76:D[32] 77:D[32] 78:D[32] 79:D[32] 80:D[32] 81:D[32] 82:D[32] 83:D[32] 84:D[32] 85:D[32] 86:D[32] 87:D[32] 88:D[32] 89:D[32] 90:D[32] 91:D[3] 92:D[32] 93:D[32] 94:D[32] 95:D[32] 96:D[32] 97:D[32] 98:D[32] 99:D[32] 100:D[6] 101:D[32] 102:D[32] 103:D[32] 104:D[32] 105:D[32] 106:D[32] 107:D[32] 108:D[32] 109:D[1] 110:D[32] 111:D[32] 112:D[32] 113:D[32] 114:D[32] 115:D[32] 116:D[32] 117:D[32] 118:D[4] 119:D[32] 120:D[32] 121:D[32] 122:D[32] 123:D[32] 124:D[32] 125:D[32] 126:D[32] 127:D[32] 128:D[32] 129:D[32] 130:D[32] 131:D[32] 132:D[32] 133:D[32] 134:D[32] 135:D[2] 136:D[32] 137:D[32] 138:D[32] 139:D[32] 140:D[32] 141:D[32] 142:D[32] 143:D[32] 144:D[3]`
- **suite:c10:switching** (switches=0): `0:Rpilot![4] 1:D[32] 2:Rpilot![4] 3:D[32] 4:D[8] 5:D[32] 6:Rpilot![4] 7:D[32] 8:D[10] 9:D[32] 10:D[32] 11:D[9] 12:Rpilot![4] 13:D[32] 14:D[32] 15:D[10] 16:D[32] 17:D[32] 18:D[14] 19:D[32] 20:D[32] 21:D[8] 22:D[32] 23:D[32] 24:D[13] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:D[11] 31:D[32] 32:D[32] 33:D[12] 34:D[32] 35:D[32] 36:D[13] 37:D[32] 38:D[32] 39:D[10] 40:D[32] 41:D[32] 42:D[9] 43:D[32] 44:D[32] 45:D[10] 46:D[32] 47:D[32] 48:D[10] 49:D[32] 50:D[32] 51:D[12]`

## Timed samples (ms, diagnostic only)

- bench:rotation:evidence: before 0.917, 0.863, 0.875, 0.474, 0.481; after 0.521, 0.443, 0.501, 0.631, 0.490
- bench:rotation:screened-reset: before 0.701, 0.698, 0.701, 1.138, 0.718; after 0.910, 1.131, 1.064, 1.027, 1.297
- bench:rotation:pressure-only: before 6.254, 6.290, 6.629, 6.223, 7.054; after 6.702, 6.515, 6.287, 4.415, 4.539
- bench:stiff-forced:evidence: before 0.213, 0.213, 0.203, 0.212, 0.208; after 0.138, 0.133, 0.143, 0.134, 0.159
- bench:stiff-forced:screened-reset: before 0.134, 0.140, 0.142, 0.163, 0.131; after 0.125, 0.125, 0.127, 0.154, 0.128
- bench:stiff-forced:pressure-only: before 0.121, 0.119, 0.121, 0.122, 0.126; after 0.129, 0.155, 0.121, 0.150, 0.153
- suite:c5:switching: before 0.820, 0.805, 0.800, 0.825, 0.825; after 0.614, 0.614, 0.615, 0.612, 0.630
- suite:c10t:switching: before 5.450, 9.149, 9.273, 9.386, 9.216; after 8.721, 6.225, 5.018, 5.022, 4.829
- suite:c10:switching: before 3.374, 2.114, 2.082, 2.060, 2.491; after 1.876, 2.013, 1.872, 1.855, 1.811
- bench:rotation:dopri: before 0.223, 0.227, 0.378, 0.223, 0.220; after 0.240, 0.280, 0.248, 0.230, 0.238
- bench:rotation:rodas: before 6.429, 6.566, 6.443, 11.888, 12.376; after 7.764, 7.703, 7.769, 4.690, 6.285
- bench:stiff-forced:dopri: before 0.866, 0.908, 0.853, 0.864, 0.953; after 0.925, 1.203, 1.195, 1.206, 1.290
- bench:stiff-forced:rodas: before 0.080, 0.080, 0.081, 0.078, 0.079; after 0.095, 0.095, 0.099, 0.101, 0.094
- suite:c5:rodas5p-mf: before 0.660, 0.586, 0.594, 0.590, 0.596; after 0.403, 0.508, 0.401, 0.405, 0.394
- suite:c5:rodas5p-dense: before 0.123, 0.123, 0.122, 0.122, 0.121; after 0.116, 0.109, 0.141, 0.122, 0.114
- suite:c10t:dopri5: before 4.595, 4.551, 4.731, 4.613, 4.725; after 2.436, 2.554, 2.491, 2.454, 2.454
- suite:c10t:rodas5p-mf: before 203.742, 190.713, 192.618, 194.169, 196.203; after 154.309, 153.555, 160.971, 190.738, 151.343
- suite:c10:dopri5: before 0.770, 0.724, 1.168, 0.915, 0.721; after 0.893, 0.675, 0.702, 0.685, 0.689
