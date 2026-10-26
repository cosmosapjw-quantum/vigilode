# Before/after: adapter work (authority) and wall time (diagnostic only)

before: commit `4fe67602502440d76aa9dd245c6b1d67558c14bf` binaries `{'evidence_switch_bench': 'ab812631e6f4a32e532958388081d10b03498034e279e598569953e3af9ca912', 'local_bottleneck_suite': '12e66ce27610a274f9c83ca8c95ee75cd5f864d815ab2d99d47b97b45ccaeff8'}`

after: commit `636cdc7a49011af3c10f8cf474cc4bb46e249700` binaries `{'evidence_switch_bench': '68ef8abd78008244d59b25dfa73011a8165801c56ef8c99583379af709c974c6', 'local_bottleneck_suite': '386c105c9b8cebd3f53bd15207d565d2aea8c7ed53252201402e72de3f69d0c5'}`

## G1 — untouched direct paths are bit-identical (counters and final state)

| direct cell | attempts | RHS | JVP | solves | final state bitwise | wall ms before→after (drift) |
|---|---|---|---|---|---|---|
| bench:rotation:dopri | 875/875 | 5251/5251 | 0/0 | 0/0 | yes | 0.225→0.219 (0.97x) |
| bench:rotation:rodas | 640/640 | 5022/5022 | 20453/20453 | 5120/5120 | yes | 6.592→6.444 (0.98x) |
| bench:stiff-forced:dopri | 3111/3111 | 18667/18667 | 0/0 | 0/0 | yes | 0.872→0.856 (0.98x) |
| bench:stiff-forced:rodas | 14/14 | 112/112 | 0/0 | 112/112 | yes | 0.092→0.092 (0.99x) |
| suite:c5:rodas5p-mf | 39/39 | 307/307 | 1206/1206 | 312/312 | yes | 0.635→0.989 (1.56x) |
| suite:c5:rodas5p-dense | 39/39 | 307/307 | 0/0 | 312/312 | yes | 0.137→0.183 (1.33x) |
| suite:c10t:dopri5 | 4067/4067 | 24403/24403 | 0/0 | 0/0 | yes | 2.530→2.543 (1.01x) |
| suite:c10t:rodas5p-mf | 9454/9454 | 75314/75314 | 378160/378160 | 75632/75632 | yes | 186.106→189.373 (1.02x) |
| suite:c10:dopri5 | 1105/1105 | 6631/6631 | 0/0 | 0/0 | yes | 0.713→0.718 (1.01x) |

**G1 = True**

## G2/G3 — switching cells: deterministic work never higher, accuracy caps kept

| cell | attempts b→a | RHS b→a | pilots b→a (attempts) | probes b→a (attempts) | RODAS batches b→a (attempts) | error b→a (cap) | success/pass | wall ms b→a | best direct ms b→a | ratio b→a |
|---|---|---|---|---|---|---|---|---|---|---|
| bench:rotation:evidence | 1046→892 | 6597→5371 | 6(169)→3(15) | 0(0)→0(0) | 0(0)→0(0) | 9.26e-07→9.26e-07 (1e-05) | True/True | 2.024→0.477 | 0.225→0.219 | 8.98→2.18 |
| bench:rotation:screened-reset | 1274→1105 | 8069→6710 | 8(204)→7(35) | 0(0)→0(0) | 0(0)→0(0) | 9.06e-07→9.06e-07 (1e-05) | True/True | 2.446→0.722 | 0.225→0.219 | 10.86→3.30 |
| bench:rotation:pressure-only | 701→666 | 5430→5144 | 0(0)→0(0) | 1(16)→1(16) | 8(653)→3(618) | 8.32e-08→8.37e-08 (1e-05) | True/True | 6.589→6.384 | 0.225→0.219 | 29.24→29.20 |
| bench:stiff-forced:evidence | 137→66 | 1002→456 | 1(8)→1(8) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | True/True | 0.212→0.134 | 0.092→0.092 | 2.30→1.47 |
| bench:stiff-forced:screened-reset | 137→66 | 1002→456 | 1(8)→1(8) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | True/True | 0.206→0.132 | 0.092→0.092 | 2.24→1.45 |
| bench:stiff-forced:pressure-only | 129→58 | 938→392 | 0(0)→0(0) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | True/True | 0.238→0.123 | 0.092→0.092 | 2.58→1.34 |
| suite:c5:switching | 232→115 | 1633→748 | 2(23)→2(6) | 3(48)→2(19) | 16(97)→4(26) | 9.86e-10→1.25e-08 (1e-04) | True/True | 2.405→0.833 | 0.137→0.183 | 17.49→4.55 |
| suite:c10t:switching | 4271→4159 | 26018→25102 | 4(128)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.89e-05→3.89e-05 (1e-04) | True/True | 7.812→5.523 | 2.530→2.543 | 3.09→2.17 |
| suite:c10:switching | 1321→1209 | 8221→7309 | 4(128)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.09e-02→3.09e-02 (1e-04) | True/False | 4.291→1.986 | 0.713→0.718 | 6.02→2.77 |

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

- bench:rotation:evidence: before 2.013, 2.024, 1.980, 2.090, 2.189; after 0.554, 0.470, 0.504, 0.472, 0.477
- bench:rotation:screened-reset: before 2.436, 2.586, 2.465, 2.446, 2.442; after 0.703, 0.838, 0.722, 0.794, 0.706
- bench:rotation:pressure-only: before 6.618, 6.501, 6.589, 6.758, 6.532; after 6.606, 6.645, 6.253, 6.286, 6.384
- bench:stiff-forced:evidence: before 0.225, 0.212, 0.210, 0.212, 0.213; after 0.127, 0.135, 0.126, 0.137, 0.134
- bench:stiff-forced:screened-reset: before 0.212, 0.271, 0.206, 0.203, 0.203; after 0.132, 0.131, 0.132, 0.199, 0.139
- bench:stiff-forced:pressure-only: before 0.245, 0.243, 0.194, 0.238, 0.201; after 0.123, 0.121, 0.125, 0.160, 0.118
- suite:c5:switching: before 2.603, 2.406, 2.372, 2.369, 2.405; after 0.939, 0.833, 0.826, 0.849, 0.788
- suite:c10t:switching: before 7.516, 7.760, 7.812, 8.097, 8.186; after 6.243, 5.523, 5.572, 5.249, 5.485
- suite:c10:switching: before 4.300, 4.291, 4.631, 4.146, 4.158; after 1.987, 1.974, 1.985, 2.091, 1.986
- bench:rotation:dopri: before 0.224, 0.223, 0.225, 0.226, 0.280; after 0.297, 0.219, 0.207, 0.217, 0.259
- bench:rotation:rodas: before 6.611, 6.480, 6.799, 6.592, 6.370; after 6.364, 6.213, 6.445, 6.472, 6.444
- bench:stiff-forced:dopri: before 0.872, 0.907, 0.862, 0.865, 0.905; after 0.847, 0.899, 0.856, 0.853, 0.876
- bench:stiff-forced:rodas: before 0.095, 0.086, 0.104, 0.090, 0.092; after 0.080, 0.077, 0.108, 0.092, 0.092
- suite:c5:rodas5p-mf: before 0.635, 0.604, 0.664, 0.615, 0.645; after 0.994, 0.989, 0.986, 0.971, 0.991
- suite:c5:rodas5p-dense: before 0.138, 0.130, 0.132, 0.137, 0.162; after 0.183, 0.183, 0.181, 0.185, 0.122
- suite:c10t:dopri5: before 2.540, 2.448, 2.744, 2.530, 2.435; after 2.756, 2.657, 2.543, 2.483, 2.498
- suite:c10t:rodas5p-mf: before 185.921, 182.496, 186.211, 186.106, 187.694; after 189.373, 188.884, 190.424, 185.878, 197.859
- suite:c10:dopri5: before 0.697, 0.703, 0.946, 1.094, 0.713; after 0.716, 0.719, 0.736, 0.712, 0.718
