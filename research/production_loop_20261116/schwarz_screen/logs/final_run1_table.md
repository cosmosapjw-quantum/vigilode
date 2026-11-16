| problem | N | tol | sigma med (min-max) | ell cells med | P | H | m | halo cells med | halo/H | work (1+2d/H)^2 | g0 sweeps med/max | g1 sweeps med/max | g1 endpoint/tol after 0, 1, 2 sweeps | g1 contraction/sweep | steps with halo cells <= 4 ell | verdict g1 | g1 strict cells | g0 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| P1_heat | 64 | 0.0001 | 234 (230-235) | 7.05 | 2 | 32 | 1 | 8 | 0.25 | 2.25 | 5 / 5 | 5 / 5 | 3.4e+02, 75, 12 | 0.14 | 20/20 | KILL | KILL | KILL |
| P1_heat | 64 | 0.0001 | 234 (230-235) | 7.05 | 2 | 32 | 2 | 15 | 0.469 | 3.75 | 3 / 3 | 3 / 3 | 3.4e+02, 31, 1.5 | 0.036 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 0.0001 | 234 (230-235) | 7.05 | 2 | 32 | 4 | 29 | 0.906 | 7.91 | 2 / 2 | 2 / 2 | 3.4e+02, 1.8, 0.001 | 0.00066 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 0.0001 | 234 (230-235) | 7.05 | 4 | 16 | 1 | 8 | 0.5 | 4 | 6 / 7 | 6 / 7 | 3.4e+02, 99, 19 | 0.21 | 20/20 | KILL | KILL | KILL |
| P1_heat | 64 | 0.0001 | 234 (230-235) | 7.05 | 4 | 16 | 2 | 15 | 0.938 | 8.27 | 4 / 4 | 4 / 4 | 3.4e+02, 47, 4.7 | 0.065 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 0.0001 | 234 (230-235) | 7.05 | 4 | 16 | 4 | 29 | 1.81 | 21.4 | 2 / 2 | 2 / 2 | 3.4e+02, 6.6, 0.023 | 0.0034 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 1e-06 | 71.3 (71.1-71.6) | 3.89 | 2 | 32 | 1 | 4 | 0.125 | 1.56 | 6 / 6 | 5 / 6 | 3.5e+03, 7.2e+02, 1.1e+02 | 0.11 | 20/20 | KILL | KILL | KILL |
| P1_heat | 64 | 1e-06 | 71.3 (71.1-71.6) | 3.89 | 2 | 32 | 2 | 8 | 0.25 | 2.25 | 4 / 4 | 4 / 4 | 3.5e+03, 3.7e+02, 9.7 | 0.015 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 1e-06 | 71.3 (71.1-71.6) | 3.89 | 2 | 32 | 4 | 16 | 0.5 | 4 | 3 / 3 | 3 / 3 | 3.5e+03, 60, 0.12 | 0.00096 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 1e-06 | 71.3 (71.1-71.6) | 3.89 | 4 | 16 | 1 | 4 | 0.25 | 2.25 | 7 / 8 | 7 / 7 | 3.5e+03, 1.1e+03, 3.2e+02 | 0.15 | 20/20 | KILL | KILL | KILL |
| P1_heat | 64 | 1e-06 | 71.3 (71.1-71.6) | 3.89 | 4 | 16 | 2 | 8 | 0.5 | 4 | 4 / 4 | 4 / 4 | 3.5e+03, 5.5e+02, 31 | 0.025 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 64 | 1e-06 | 71.3 (71.1-71.6) | 3.89 | 4 | 16 | 4 | 16 | 1 | 9 | 3 / 3 | 3 / 3 | 3.5e+03, 99, 1 | 0.0033 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 0.0001 | 94.2 (18.2-238) | 4.46 | 2 | 32 | 1 | 5 | 0.156 | 1.72 | 6 / 7 | 5 / 6 | 2.5e+02, 89, 22 | 0.17 | 20/20 | KILL | KILL | KILL |
| P2_brusselator | 64 | 0.0001 | 94.2 (18.2-238) | 4.46 | 2 | 32 | 2 | 9.5 | 0.297 | 2.54 | 4 / 4 | 3 / 4 | 2.5e+02, 48, 3.1 | 0.035 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 0.0001 | 94.2 (18.2-238) | 4.46 | 2 | 32 | 4 | 18 | 0.562 | 4.52 | 1 / 3 | 1 / 2 | 2.5e+02, 7.4e-12, 0.021 | 0.0024 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 0.0001 | 94.2 (18.2-238) | 4.46 | 4 | 16 | 1 | 5 | 0.312 | 2.64 | 7 / 9 | 6 / 8 | 2.5e+02, 1.4e+02, 49 | 0.19 | 20/20 | KILL | KILL | KILL |
| P2_brusselator | 64 | 0.0001 | 94.2 (18.2-238) | 4.46 | 4 | 16 | 2 | 9.5 | 0.594 | 4.79 | 4 / 5 | 4 / 5 | 2.5e+02, 76, 11 | 0.066 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 0.0001 | 94.2 (18.2-238) | 4.46 | 4 | 16 | 4 | 18 | 1.12 | 10.6 | 3 / 3 | 2.5 / 3 | 2.5e+02, 11, 0.12 | 0.0057 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 1e-06 | 51.4 (9.37-104) | 3.3 | 2 | 32 | 1 | 4 | 0.125 | 1.56 | 7 / 8 | 6 / 7 | 7e+03, 2e+03, 3.7e+02 | 0.099 | 20/20 | KILL | KILL | KILL |
| P2_brusselator | 64 | 1e-06 | 51.4 (9.37-104) | 3.3 | 2 | 32 | 2 | 7 | 0.219 | 2.07 | 4 / 5 | 4 / 4 | 7e+03, 1.1e+03, 52 | 0.018 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 1e-06 | 51.4 (9.37-104) | 3.3 | 2 | 32 | 4 | 13.5 | 0.422 | 3.4 | 3 / 3 | 3 / 3 | 7e+03, 1.9e+02, 0.49 | 0.0014 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P2_brusselator | 64 | 1e-06 | 51.4 (9.37-104) | 3.3 | 4 | 16 | 1 | 4 | 0.25 | 2.25 | 9 / >10 | 6 / 9 | 7e+03, 3.1e+03, 7.2e+02 | 0.12 | 20/20 | KILL | KILL | KILL |
| P2_brusselator | 64 | 1e-06 | 51.4 (9.37-104) | 3.3 | 4 | 16 | 2 | 7 | 0.438 | 3.52 | 6 / 7 | 4 / 5 | 7e+03, 1.7e+03, 1.2e+02 | 0.03 | 20/20 | FAIL | FAIL | KILL |
| P2_brusselator | 64 | 1e-06 | 51.4 (9.37-104) | 3.3 | 4 | 16 | 4 | 13.5 | 0.844 | 7.23 | 3 / 4 | 3 / 3 | 7e+03, 3.2e+02, 2.2 | 0.0032 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 0.0001 | 4.35 (3.81-21.4) | 0.96 | 2 | 32 | 1 | 1 | 0.0312 | 1.13 | 3 / 3 | 2 / 3 | 31, 1.1, 0.053 | 0.05 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 64 | 0.0001 | 4.35 (3.81-21.4) | 0.96 | 2 | 32 | 2 | 2 | 0.0625 | 1.27 | 2 / 3 | 2 / 2 | 31, 0.45, 0.0055 | 0.0096 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 64 | 0.0001 | 4.35 (3.81-21.4) | 0.96 | 2 | 32 | 4 | 4 | 0.125 | 1.56 | 2 / 2 | 2 / 2 | 31, 0.12, 6.9e-05 | 0.00039 | 0/20 | PASS | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 0.0001 | 4.35 (3.81-21.4) | 0.96 | 4 | 16 | 1 | 1 | 0.0625 | 1.27 | 3 / 4 | 3 / 3 | 31, 2.5, 0.17 | 0.067 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 0.0001 | 4.35 (3.81-21.4) | 0.96 | 4 | 16 | 2 | 2 | 0.125 | 1.56 | 2 / 3 | 2 / 2 | 31, 0.99, 0.014 | 0.013 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 64 | 0.0001 | 4.35 (3.81-21.4) | 0.96 | 4 | 16 | 4 | 4 | 0.25 | 2.25 | 2 / 2 | 2 / 2 | 31, 0.15, 0.00011 | 0.00059 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 1e-06 | 1.8 (1.73-1.91) | 0.618 | 2 | 32 | 1 | 1 | 0.0312 | 1.13 | 3 / 4 | 3 / 3 | 6.7e+02, 9.3, 0.15 | 0.015 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 1e-06 | 1.8 (1.73-1.91) | 0.618 | 2 | 32 | 2 | 2 | 0.0625 | 1.27 | 2 / 3 | 2 / 2 | 6.7e+02, 2.4, 0.0027 | 0.0015 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 64 | 1e-06 | 1.8 (1.73-1.91) | 0.618 | 2 | 32 | 4 | 3 | 0.0938 | 1.41 | 2 / 2 | 2 / 2 | 6.7e+02, 0.54, 0.00013 | 0.00025 | 0/20 | PASS | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 1e-06 | 1.8 (1.73-1.91) | 0.618 | 4 | 16 | 1 | 1 | 0.0625 | 1.27 | 4 / 4 | 3 / 3 | 6.7e+02, 29, 0.82 | 0.022 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 64 | 1e-06 | 1.8 (1.73-1.91) | 0.618 | 4 | 16 | 2 | 2 | 0.125 | 1.56 | 3 / 3 | 2 / 2 | 6.7e+02, 7.2, 0.023 | 0.0032 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 64 | 1e-06 | 1.8 (1.73-1.91) | 0.618 | 4 | 16 | 4 | 3 | 0.188 | 1.89 | 2 / 2 | 2 / 2 | 6.7e+02, 1.6, 0.00066 | 0.0004 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 0.0001 | 925 (910-929) | 14 | 2 | 64 | 1 | 15 | 0.234 | 2.16 | 5 / 6 | 5 / 5 | 3.4e+02, 81, 15 | 0.17 | 20/20 | KILL | KILL | KILL |
| P1_heat | 128 | 0.0001 | 925 (910-929) | 14 | 2 | 64 | 2 | 29 | 0.453 | 3.63 | 3 / 4 | 3 / 4 | 3.4e+02, 34, 2 | 0.04 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 0.0001 | 925 (910-929) | 14 | 2 | 64 | 4 | 57 | 0.891 | 7.74 | 2 / 2 | 2 / 2 | 3.4e+02, 2.2, 0.0018 | 0.00089 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 0.0001 | 925 (910-929) | 14 | 4 | 32 | 1 | 15 | 0.469 | 3.75 | 7 / 7 | 7 / 7 | 3.4e+02, 1.1e+02, 24 | 0.24 | 20/20 | KILL | KILL | KILL |
| P1_heat | 128 | 0.0001 | 925 (910-929) | 14 | 4 | 32 | 2 | 29 | 0.906 | 7.91 | 4 / 4 | 4 / 4 | 3.4e+02, 50, 4.9 | 0.069 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 0.0001 | 925 (910-929) | 14 | 4 | 32 | 4 | 57 | 1.78 | 20.8 | 2 / 2 | 2 / 2 | 3.4e+02, 7.5, 0.028 | 0.0042 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 1e-06 | 281 (281-283) | 7.72 | 2 | 64 | 1 | 8 | 0.125 | 1.56 | 6 / 6 | 6 / 6 | 3.5e+03, 7.4e+02, 1.3e+02 | 0.12 | 20/20 | KILL | KILL | KILL |
| P1_heat | 128 | 1e-06 | 281 (281-283) | 7.72 | 2 | 64 | 2 | 16 | 0.25 | 2.25 | 4 / 4 | 4 / 4 | 3.5e+03, 3.8e+02, 11 | 0.017 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 1e-06 | 281 (281-283) | 7.72 | 2 | 64 | 4 | 31 | 0.484 | 3.88 | 3 / 3 | 3 / 3 | 3.5e+03, 69, 0.2 | 0.00089 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 1e-06 | 281 (281-283) | 7.72 | 4 | 32 | 1 | 8 | 0.25 | 2.25 | 8 / 8 | 7 / 7 | 3.5e+03, 1.2e+03, 3.6e+02 | 0.18 | 20/20 | KILL | KILL | KILL |
| P1_heat | 128 | 1e-06 | 281 (281-283) | 7.72 | 4 | 32 | 2 | 16 | 0.5 | 4 | 4 / 5 | 4 / 4 | 3.5e+03, 5.7e+02, 35 | 0.027 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P1_heat | 128 | 1e-06 | 281 (281-283) | 7.72 | 4 | 32 | 4 | 31 | 0.969 | 8.63 | 3 / 3 | 3 / 3 | 3.5e+03, 1.1e+02, 1.3 | 0.0024 | 0/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 128 | 0.0001 | 4.69 (4.24-5.05) | 0.997 | 2 | 64 | 1 | 1 | 0.0156 | 1.06 | 3 / 3 | 2 / 2 | 23, 0.33, 0.017 | 0.053 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 128 | 0.0001 | 4.69 (4.24-5.05) | 0.997 | 2 | 64 | 2 | 2 | 0.0312 | 1.13 | 2 / 2 | 2 / 2 | 23, 0.17, 0.0017 | 0.0088 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 128 | 0.0001 | 4.69 (4.24-5.05) | 0.997 | 2 | 64 | 4 | 4 | 0.0625 | 1.27 | 2 / 2 | 1 / 1 | 23, 0.02, n/a | n/a | 0/20 | PASS | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 128 | 0.0001 | 4.69 (4.24-5.05) | 0.997 | 4 | 32 | 1 | 1 | 0.0312 | 1.13 | 3 / 3 | 2 / 3 | 23, 0.73, 0.017 | 0.053 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 128 | 0.0001 | 4.69 (4.24-5.05) | 0.997 | 4 | 32 | 2 | 2 | 0.0625 | 1.27 | 2 / 2 | 2 / 2 | 23, 0.28, 0.0016 | 0.0097 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 128 | 0.0001 | 4.69 (4.24-5.05) | 0.997 | 4 | 32 | 4 | 4 | 0.125 | 1.56 | 2 / 2 | 1 / 2 | 23, 0.04, 7.7e-05 | 0.00076 | 0/20 | PASS | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 128 | 1e-06 | 2.07 (1.97-2.18) | 0.663 | 2 | 64 | 1 | 1 | 0.0156 | 1.06 | 3 / 4 | 3 / 3 | 4.5e+02, 5.2, 0.12 | 0.019 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 128 | 1e-06 | 2.07 (1.97-2.18) | 0.663 | 2 | 64 | 2 | 2 | 0.0312 | 1.13 | 2 / 3 | 2 / 2 | 4.5e+02, 1.5, 0.0027 | 0.0021 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 128 | 1e-06 | 2.07 (1.97-2.18) | 0.663 | 2 | 64 | 4 | 3 | 0.0469 | 1.2 | 2 / 2 | 2 / 2 | 4.5e+02, 0.43, 0.00016 | 0.0004 | 0/20 | PASS | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 128 | 1e-06 | 2.07 (1.97-2.18) | 0.663 | 4 | 32 | 1 | 1 | 0.0312 | 1.13 | 3 / 4 | 3 / 3 | 4.5e+02, 6.6, 0.16 | 0.019 | 20/20 | FAIL | FAIL | REFERENCE_ONLY |
| P3_gray_scott | 128 | 1e-06 | 2.07 (1.97-2.18) | 0.663 | 4 | 32 | 2 | 2 | 0.0625 | 1.27 | 2 / 3 | 2 / 2 | 4.5e+02, 2.2, 0.0039 | 0.0021 | 20/20 | PASS | PASS | REFERENCE_ONLY |
| P3_gray_scott | 128 | 1e-06 | 2.07 (1.97-2.18) | 0.663 | 4 | 32 | 4 | 3 | 0.0938 | 1.41 | 2 / 2 | 2 / 2 | 4.5e+02, 0.56, 0.00022 | 0.0004 | 0/20 | PASS | FAIL | REFERENCE_ONLY |

Verdict per configuration (predictor g1; `Pp/m` = P x P subdomains, overlap m*ell):

- P1_heat N=64 tol=0.0001: PASS: none; FAIL: P2/m2, P2/m4, P4/m2, P4/m4; KILL: P2/m1, P4/m1; g0 killed: P2/m1, P4/m1.
- P1_heat N=64 tol=1e-06: PASS: none; FAIL: P2/m2, P2/m4, P4/m2, P4/m4; KILL: P2/m1, P4/m1; g0 killed: P2/m1, P4/m1.
- P2_brusselator N=64 tol=0.0001: PASS: none; FAIL: P2/m2, P2/m4, P4/m2, P4/m4; KILL: P2/m1, P4/m1; g0 killed: P2/m1, P4/m1.
- P2_brusselator N=64 tol=1e-06: PASS: none; FAIL: P2/m2, P2/m4, P4/m2, P4/m4; KILL: P2/m1, P4/m1; g0 killed: P2/m1, P4/m1, P4/m2.
- P3_gray_scott N=64 tol=0.0001: PASS: P2/m1, P2/m2, P2/m4, P4/m2; FAIL: P4/m1, P4/m4; KILL: none; PASS only under the nominal overlap reading (integer halo > 4 ell): P2/m4; g0 killed: none.
- P3_gray_scott N=64 tol=1e-06: PASS: P2/m2, P2/m4, P4/m2; FAIL: P2/m1, P4/m1, P4/m4; KILL: none; PASS only under the nominal overlap reading (integer halo > 4 ell): P2/m4; g0 killed: none.
- P1_heat N=128 tol=0.0001: PASS: none; FAIL: P2/m2, P2/m4, P4/m2, P4/m4; KILL: P2/m1, P4/m1; g0 killed: P2/m1, P4/m1.
- P1_heat N=128 tol=1e-06: PASS: none; FAIL: P2/m2, P2/m4, P4/m2, P4/m4; KILL: P2/m1, P4/m1; g0 killed: P2/m1, P4/m1.
- P3_gray_scott N=128 tol=0.0001: PASS: P2/m1, P2/m2, P2/m4, P4/m1, P4/m2, P4/m4; FAIL: none; KILL: none; PASS only under the nominal overlap reading (integer halo > 4 ell): P2/m4, P4/m4; g0 killed: none.
- P3_gray_scott N=128 tol=1e-06: PASS: P2/m2, P2/m4, P4/m2, P4/m4; FAIL: P2/m1, P4/m1; KILL: none; PASS only under the nominal overlap reading (integer halo > 4 ell): P2/m4, P4/m4; g0 killed: none.

Totals over g1 configurations: {'PASS': 17, 'FAIL': 31, 'KILL': 12}; strict-cells reading: {'PASS': 11, 'FAIL': 37, 'KILL': 12}. Run status `complete`, wall 1067 s (diagnostic only).
