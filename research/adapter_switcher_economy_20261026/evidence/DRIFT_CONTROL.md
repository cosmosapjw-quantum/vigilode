# Before/after: adapter work (authority) and wall time (diagnostic only)

before: commit `4fe67602502440d76aa9dd245c6b1d67558c14bf` binaries `{'evidence_switch_bench': 'ab812631e6f4a32e532958388081d10b03498034e279e598569953e3af9ca912', 'local_bottleneck_suite': '12e66ce27610a274f9c83ca8c95ee75cd5f864d815ab2d99d47b97b45ccaeff8'}`

after: commit `4fe67602502440d76aa9dd245c6b1d67558c14bf` binaries `{'evidence_switch_bench': 'ab812631e6f4a32e532958388081d10b03498034e279e598569953e3af9ca912', 'local_bottleneck_suite': '12e66ce27610a274f9c83ca8c95ee75cd5f864d815ab2d99d47b97b45ccaeff8'}`

## G1 — untouched direct paths are bit-identical (counters and final state)

| direct cell | attempts | RHS | JVP | solves | final state bitwise | wall ms before→after (drift) |
|---|---|---|---|---|---|---|
| bench:rotation:dopri | 875/875 | 5251/5251 | 0/0 | 0/0 | yes | 0.225→0.412 (1.83x) |
| bench:rotation:rodas | 640/640 | 5022/5022 | 20453/20453 | 5120/5120 | yes | 6.592→6.705 (1.02x) |
| bench:stiff-forced:dopri | 3111/3111 | 18667/18667 | 0/0 | 0/0 | yes | 0.872→1.237 (1.42x) |
| bench:stiff-forced:rodas | 14/14 | 112/112 | 0/0 | 112/112 | yes | 0.092→0.097 (1.05x) |
| suite:c5:rodas5p-mf | 39/39 | 307/307 | 1206/1206 | 312/312 | yes | 0.635→0.895 (1.41x) |
| suite:c5:rodas5p-dense | 39/39 | 307/307 | 0/0 | 312/312 | yes | 0.137→0.176 (1.28x) |
| suite:c10t:dopri5 | 4067/4067 | 24403/24403 | 0/0 | 0/0 | yes | 2.530→2.488 (0.98x) |
| suite:c10t:rodas5p-mf | 9454/9454 | 75314/75314 | 378160/378160 | 75632/75632 | yes | 186.106→188.939 (1.02x) |
| suite:c10:dopri5 | 1105/1105 | 6631/6631 | 0/0 | 0/0 | yes | 0.713→0.729 (1.02x) |

**G1 = True**

## G2/G3 — switching cells: deterministic work never higher, accuracy caps kept

| cell | attempts b→a | RHS b→a | pilots b→a (attempts) | probes b→a (attempts) | RODAS batches b→a (attempts) | error b→a (cap) | success/pass | wall ms b→a | best direct ms b→a | ratio b→a |
|---|---|---|---|---|---|---|---|---|---|---|
| bench:rotation:evidence | 1046→1046 | 6597→6597 | 6(169)→6(169) | 0(0)→0(0) | 0(0)→0(0) | 9.26e-07→9.26e-07 (1e-05) | True/True | 2.024→2.050 | 0.225→0.412 | 8.98→4.97 |
| bench:rotation:screened-reset | 1274→1274 | 8069→8069 | 8(204)→8(204) | 0(0)→0(0) | 0(0)→0(0) | 9.06e-07→9.06e-07 (1e-05) | True/True | 2.446→2.522 | 0.225→0.412 | 10.86→6.12 |
| bench:rotation:pressure-only | 701→701 | 5430→5430 | 0(0)→0(0) | 1(16)→1(16) | 8(653)→8(653) | 8.32e-08→8.32e-08 (1e-05) | True/True | 6.589→6.535 | 0.225→0.412 | 29.24→15.85 |
| bench:stiff-forced:evidence | 137→137 | 1002→1002 | 1(8)→1(8) | 1(16)→1(16) | 8(81)→8(81) | 1.57e-10→1.57e-10 (1e-05) | True/True | 0.212→0.225 | 0.092→0.097 | 2.30→2.32 |
| bench:stiff-forced:screened-reset | 137→137 | 1002→1002 | 1(8)→1(8) | 1(16)→1(16) | 8(81)→8(81) | 1.57e-10→1.57e-10 (1e-05) | True/True | 0.206→0.207 | 0.092→0.097 | 2.24→2.13 |
| bench:stiff-forced:pressure-only | 129→129 | 938→938 | 0(0)→0(0) | 1(16)→1(16) | 8(81)→8(81) | 1.57e-10→1.57e-10 (1e-05) | True/True | 0.238→0.191 | 0.092→0.097 | 2.58→1.97 |
| suite:c5:switching | 232→232 | 1633→1633 | 2(23)→2(23) | 3(48)→3(48) | 16(97)→16(97) | 9.86e-10→9.86e-10 (1e-04) | True/True | 2.405→3.394 | 0.137→0.176 | 17.49→19.30 |
| suite:c10t:switching | 4271→4271 | 26018→26018 | 4(128)→4(128) | 0(0)→0(0) | 0(0)→0(0) | 3.89e-05→3.89e-05 (1e-04) | True/True | 7.812→8.237 | 2.530→2.488 | 3.09→3.31 |
| suite:c10:switching | 1321→1321 | 8221→8221 | 4(128)→4(128) | 0(0)→0(0) | 0(0)→0(0) | 3.09e-02→3.09e-02 (1e-04) | True/False | 4.291→4.386 | 0.713→0.729 | 6.02→6.02 |

**G2 = True, G3 = True**

## Decision sequences (after)

- **bench:rotation:evidence** (switches=0): `0:Rpilot![26] 1:D[32] 2:D[32] 3:D[32] 4:D[16] 5:D[32] 6:Rpilot![29] 7:D[32] 8:D[32] 9:D[12] 10:D[32] 11:D[32] 12:Rpilot![29] 13:D[32] 14:D[14] 15:D[32] 16:D[32] 17:D[32] 18:D[13] 19:Rpilot![28] 20:D[32] 21:D[32] 22:D[32] 23:D[14] 24:D[32] 25:Rpilot![28] 26:D[32] 27:D[32] 28:D[13] 29:D[32] 30:D[32] 31:Rpilot![29] 32:D[32] 33:D[14] 34:D[32] 35:D[32] 36:D[32] 37:D[13]`
- **bench:rotation:screened-reset** (switches=0): `0:Rpilot![26] 1:D[32] 2:D[32] 3:D[32] 4:D[32] 5:D[7] 6:Rpilot![25] 7:D[32] 8:D[32] 9:D[32] 10:D[31] 11:D[32] 12:Rpilot![24] 13:D[32] 14:D[32] 15:D[31] 16:D[32] 17:D[32] 18:Rpilot![26] 19:D[32] 20:D[32] 21:D[7] 22:D[32] 23:D[32] 24:Rpilot![24] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:Rpilot![26] 31:D[32] 32:D[32] 33:D[8] 34:D[32] 35:D[32] 36:Rpilot![27] 37:D[32] 38:D[32] 39:D[7] 40:D[32] 41:D[32] 42:Rpilot![26] 43:D[32] 44:D[32] 45:D[9]`
- **bench:rotation:pressure-only** (switches=1): `0:D[32] 1:R[64] 2:R[83] 3:R[83] 4:R[83] 5:Dprobe![16] 6:R[86] 7:R[84] 8:R[84] 9:R[86]`
- **bench:stiff-forced:evidence** (switches=1): `0:Rpilot![8] 1:D[32] 2:R[11] 3:R[10] 4:R[10] 5:R[10] 6:Dprobe![16] 7:R[10] 8:R[10] 9:R[10] 10:R[10]`
- **bench:stiff-forced:screened-reset** (switches=1): `0:Rpilot![8] 1:D[32] 2:R[11] 3:R[10] 4:R[10] 5:R[10] 6:Dprobe![16] 7:R[10] 8:R[10] 9:R[10] 10:R[10]`
- **bench:stiff-forced:pressure-only** (switches=1): `0:D[32] 1:R[11] 2:R[10] 3:R[10] 4:R[10] 5:Dprobe![16] 6:R[10] 7:R[10] 8:R[10] 9:R[10]`
- **suite:c5:switching** (switches=1): `0:Rpilot![22] 1:D[32] 2:Rpilot![1] 3:D[32] 4:R[7] 5:R[6] 6:R[6] 7:R[6] 8:Dprobe![16] 9:R[6] 10:R[6] 11:R[6] 12:R[6] 13:Dprobe![16] 14:R[6] 15:R[6] 16:R[6] 17:R[6] 18:Dprobe![16] 19:R[6] 20:R[6] 21:R[6] 22:R[6]`
- **suite:c10t:switching** (switches=0): `0:Rpilot![32] 1:D[32] 2:Rpilot![32] 3:D[32] 4:Rpilot![32] 5:D[32] 6:Rpilot![32] 7:D[32] 8:D[32] 9:D[32] 10:D[32] 11:D[32] 12:D[6] 13:D[32] 14:D[32] 15:D[32] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[32] 21:D[3] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[32] 27:D[32] 28:D[32] 29:D[32] 30:D[6] 31:D[32] 32:D[32] 33:D[32] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[32] 39:D[4] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[32] 45:D[32] 46:D[32] 47:D[32] 48:D[5] 49:D[32] 50:D[32] 51:D[32] 52:D[32] 53:D[32] 54:D[32] 55:D[32] 56:D[31] 57:D[32] 58:D[32] 59:D[32] 60:D[32] 61:D[32] 62:D[32] 63:D[32] 64:D[32] 65:D[2] 66:D[32] 67:D[32] 68:D[32] 69:D[32] 70:D[32] 71:D[32] 72:D[32] 73:D[32] 74:D[3] 75:D[32] 76:D[32] 77:D[32] 78:D[32] 79:D[32] 80:D[32] 81:D[32] 82:D[32] 83:D[32] 84:D[32] 85:D[32] 86:D[32] 87:D[32] 88:D[32] 89:D[32] 90:D[32] 91:D[3] 92:D[32] 93:D[32] 94:D[32] 95:D[32] 96:D[32] 97:D[32] 98:D[32] 99:D[32] 100:D[6] 101:D[32] 102:D[32] 103:D[32] 104:D[32] 105:D[32] 106:D[32] 107:D[32] 108:D[32] 109:D[1] 110:D[32] 111:D[32] 112:D[32] 113:D[32] 114:D[32] 115:D[32] 116:D[32] 117:D[32] 118:D[4] 119:D[32] 120:D[32] 121:D[32] 122:D[32] 123:D[32] 124:D[32] 125:D[32] 126:D[32] 127:D[32] 128:D[32] 129:D[32] 130:D[32] 131:D[32] 132:D[32] 133:D[32] 134:D[32] 135:D[2] 136:D[32] 137:D[32] 138:D[32] 139:D[32] 140:D[32] 141:D[32] 142:D[32] 143:D[32] 144:D[3]`
- **suite:c10:switching** (switches=0): `0:Rpilot![32] 1:D[32] 2:Rpilot![32] 3:D[32] 4:D[8] 5:Rpilot![32] 6:D[32] 7:Rpilot![32] 8:D[32] 9:D[10] 10:D[32] 11:D[32] 12:D[9] 13:D[32] 14:D[32] 15:D[10] 16:D[32] 17:D[32] 18:D[14] 19:D[32] 20:D[32] 21:D[8] 22:D[32] 23:D[32] 24:D[13] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:D[11] 31:D[32] 32:D[32] 33:D[12] 34:D[32] 35:D[32] 36:D[13] 37:D[32] 38:D[32] 39:D[10] 40:D[32] 41:D[32] 42:D[9] 43:D[32] 44:D[32] 45:D[10] 46:D[32] 47:D[32] 48:D[10] 49:D[32] 50:D[32] 51:D[12]`

## Timed samples (ms, diagnostic only)

- bench:rotation:evidence: before 2.013, 2.024, 1.980, 2.090, 2.189; after 2.135, 2.050, 1.982, 2.029, 2.810
- bench:rotation:screened-reset: before 2.436, 2.586, 2.465, 2.446, 2.442; after 2.401, 2.522, 3.200, 13.467, 2.431
- bench:rotation:pressure-only: before 6.618, 6.501, 6.589, 6.758, 6.532; after 6.441, 6.492, 7.373, 6.535, 6.616
- bench:stiff-forced:evidence: before 0.225, 0.212, 0.210, 0.212, 0.213; after 0.264, 0.220, 0.225, 0.239, 0.215
- bench:stiff-forced:screened-reset: before 0.212, 0.271, 0.206, 0.203, 0.203; after 0.217, 0.208, 0.207, 0.202, 0.207
- bench:stiff-forced:pressure-only: before 0.245, 0.243, 0.194, 0.238, 0.201; after 0.197, 0.191, 0.186, 0.195, 0.188
- suite:c5:switching: before 2.603, 2.406, 2.372, 2.369, 2.405; after 3.394, 3.411, 3.363, 3.326, 3.454
- suite:c10t:switching: before 7.516, 7.760, 7.812, 8.097, 8.186; after 11.708, 11.276, 8.237, 7.543, 7.602
- suite:c10:switching: before 4.300, 4.291, 4.631, 4.146, 4.158; after 4.347, 4.386, 4.471, 4.324, 4.398
- bench:rotation:dopri: before 0.224, 0.223, 0.225, 0.226, 0.280; after 0.363, 0.412, 0.402, 0.421, 0.435
- bench:rotation:rodas: before 6.611, 6.480, 6.799, 6.592, 6.370; after 7.717, 6.605, 6.783, 6.705, 6.424
- bench:stiff-forced:dopri: before 0.872, 0.907, 0.862, 0.865, 0.905; after 1.210, 1.237, 1.239, 1.231, 1.242
- bench:stiff-forced:rodas: before 0.095, 0.086, 0.104, 0.090, 0.092; after 0.120, 0.094, 0.097, 0.122, 0.092
- suite:c5:rodas5p-mf: before 0.635, 0.604, 0.664, 0.615, 0.645; after 0.867, 0.895, 0.885, 0.915, 0.918
- suite:c5:rodas5p-dense: before 0.138, 0.130, 0.132, 0.137, 0.162; after 0.184, 0.173, 0.172, 0.176, 0.179
- suite:c10t:dopri5: before 2.540, 2.448, 2.744, 2.530, 2.435; after 2.586, 2.427, 2.514, 2.433, 2.488
- suite:c10t:rodas5p-mf: before 185.921, 182.496, 186.211, 186.106, 187.694; after 182.082, 187.788, 196.972, 189.330, 188.939
- suite:c10:dopri5: before 0.697, 0.703, 0.946, 1.094, 0.713; after 0.729, 0.939, 0.729, 0.761, 0.724
