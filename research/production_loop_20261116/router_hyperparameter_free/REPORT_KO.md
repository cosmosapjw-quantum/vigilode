# V27 phase 2 결과 보고서: forward break-even pilots (FBP) Auto 경로의 사전 등록 평가

- 노드: V27_ROUTER_HYPERPARAMETER_FREE (연구 루프 L1, wave 3, group 8)
- 브랜치 `research/production-loop-20261116-w3-v27`; phase 1 head `c998963d`; base(arm T) `1ffbda20`
- 사전 등록: `PREDECLARED.json` (sha256 `325c9d67bccae6e0…`), `AMENDMENT_A8_2.json`, 정오표 A8.1·A8.3; 계산 방법: `PHASE2_NOTES.txt` (첫 결정 실행 전 커밋)
- 첫 결정 실행(UTC): 2026-11-24T14:49:52Z; 실행 760개 프로세스, 케이스 76개 x 팔(arm) 4개 x 3회
- 이 보고서와 RESULTS.json은 `parse_results.py`가 원시 runner 출력과 기준해(references)만 읽어 생성합니다. 벽시계 시간은 측정하지 않았습니다(카운터만).

## 1. 결정

**동결된 결정 규칙**: the Auto route becomes N iff E1 to E10 all pass; otherwise today's route stays (the implementation is kept on its branch and is not merged). E3, E4, E5, E6 or E10 failing refutes the unconditional properties (design or implementation defect); E1 failing refutes the economic claim; E2 failing blocks adoption.

**결과**: 유지: 오늘의 경로(T)가 유지되고, 구현은 브랜치에 남으며 병합되지 않습니다. 실패한 판별자: E1, E3, E7.

- 통과: E2, E4, E5, E6, E8, E9, E10
- 실패: E1, E3, E7

실패 원인과 결정에 쓰인 수:

- E1: 기하평균 N/T는 콜백 0.5258, 연산 0.4434 (1 미만 조건: 콜백 예, 연산 예). 최선 단일법 대비 최대비 조건 N < T: 콜백 아니오, 연산 아니오; N의 최대 522.8 (robertson_40/A/regime=nonstiff), T의 최대 522.8 (robertson_40/A/regime=nonstiff), 연산 N 286 대 T 286. 두 최대가 같은 케이스 robertson_40/A/regime=nonstiff에서 같은 값으로 나오므로 엄격한 부등식이 성립하지 않습니다. 이 케이스의 분석 근거는 declared-regime(선언된 사실)이며, N과 T는 E3 (b)에서 비트 단위로 같습니다(robertson_40/A/regime=nonstiff). 선언 없는 62개 케이스만 보면(기술적 정보, 기준 아님) 최대비는 콜백 N 1.978 (P_scalar_fd/S/default) 대 T 54.15 (vdp_1000/L/default), 연산 N 1.875 (P_scalar_fd/S/default) 대 T 54.28 (lorenz96_40/L/default)입니다.
- E3: (a) 실패 N2/A/default, (b) 실패 없음. (a)는 치명적 pilot(D1)으로 끝난 실행에서만 실패했습니다. D1과 E8은 N2가 첫 pilot에서 끝나기를 요구하고 Dopri5::solve는 N2를 끝까지 풀기 때문에, 문자 그대로의 E3 (a)와 E8은 N2에서 동시에 성립할 수 없습니다(R1). 동결 규칙은 문자 그대로 적용했으며, 이 모순은 소유자 판단이 필요한 열린 문제입니다. 그 밖의 handover 없는 실행은 모두 (a)를 만족합니다.
- E7: P_scalar_fd/A/default: 가중 RMS 오차 err(N) = 0.004119 > 3 x max(err(D), err(R)) = 0.000244 (err(D) -, err(R) 8.133e-05, 비 50.65). 최대 절대오차 N 2.23e-09, R 4.403e-11. 가중 노름에서 1이 허용오차 수준이므로 N의 오차는 허용오차 안이지만, 기준은 상대 비교이며 하한이 없습니다.
- E7: P_scalar_fd/L/default: 가중 RMS 오차 err(N) = 2.874e-07 > 3 x max(err(D), err(R)) = 2.44e-07 (err(D) -, err(R) 8.133e-08, 비 3.534). 최대 절대오차 N 1.556e-10, R 4.403e-11. 가중 노름에서 1이 허용오차 수준이므로 N의 오차는 허용오차 안이지만, 기준은 상대 비교이며 하한이 없습니다.
- E5 (정오 E5-i, 결정 실행 뒤 기록): 판정 PASS는 동결 문구 '미과금 항목 = 0'(과금되지 않은 콜백 항목)으로 계산했습니다. PHASE2_NOTES 6에 처음 적은 공식(항목 - 과금 카운터 = 0, 모든 종류)으로는 5개 실행(P_scalar_fd/A/default, kt_vanishing/A/default, P_scalar_fd/L/default, P_scalar_fd/S/default, P_mass/A/default)이 실패하고 판정은 FAIL입니다. 원인은 ft_calls가 시간미분 콜백이 없는 계산(유한차분 시간미분, P_mass)도 센다는 점이며, 같은 차이가 R과 T에도 있습니다 (과금 카운터가 항목보다 큰 것이지 과금되지 않은 항목이 아닙니다). 동결 결정은 어느 계산이든 같습니다.

## 2. 무엇이 바뀌었나 (arm N)

- 오늘의 Auto 경로(arm T)는 t0에서 강성 스크린(유한차분 열 1개)과 차원 한계(n > 10이면 RODAS5P), 1000회 DOPRI5 시험 예산을 썼습니다. 이 이름들과 규칙은 제거되었습니다(소유자 결정 D5).
- 새 경로(arm N, FBP): 선언이 없는 요청은 DOPRI5로 시작하고, DOPRI5 시도 수가 b·2^k에 이를 때마다 RODAS5P 한 번의 시도(pilot, h0 = H = DOPRI5가 마지막 b번 시도에서 전진한 길이, max_steps = 1)를 DOPRI5의 수락된 상태에서 시험합니다. pilot이 수락되면 RODAS5P로 넘어가고(handover, 단방향, D3), 거부되면 DOPRI5가 비트 단위로 그대로 계속합니다. b = ceil((2n^2 + 54n + 384)/156) 은 정수 산술로 n에서만 정해집니다.
- pilot 안의 치명적 콜백은 해를 CallbackFailed와 수락된 접두부로 끝냅니다(D1). 공급된 Jacobian/JVP/band는 RODAS5P로 시작합니다(D2). 선언 규칙은 그대로입니다.

## 3. 판별자 E1-E10

| 판별자 | 판정 | 결정에 쓰인 수 |
|---|---|---|
| E1 경제성 | FAIL | 케이스 71개: 기하평균 N/T 콜백 0.5258, 연산 0.4434; 최선 단일법 대비 최대비 콜백 N 522.8 (robertson_40/A/regime=nonstiff) vs T 522.8 (robertson_40/A/regime=nonstiff), 연산 N 286 (robertson_40/A/regime=nonstiff) vs T 286 (robertson_40/A/regime=nonstiff) |
| E2 새 실패 없음 | PASS | T 성공 72, N 성공 73; T 성공·N 실패: 없음; N만 성공: F2/A/default; 둘 다 실패: N2/A/default, P_fatal/A/default, P_blowup/A/default |
| E3 중립성 | FAIL | (a) handover 없는 N 실행 40개 중 실패 1개: N2/A/default; (b) 선언된 사실로 정해진 실행 10개 중 T와 다른 것: 없음 |
| E4 예산 | PASS | N 실행 76개, 위반 없음; 가장 빠듯한 경우 robertson_40/A/max_steps=50 36/50, robertson_40/A/regime=nonstiff 36162/100000, damped_osc/S/default 13580/100000 |
| E5 콜백 | PASS | N 실행 76개(DOPRI5로 시작 68개): 미과금 항목·재생 분해 실패 없음 |
| E6 결정성 | PASS | (케이스, 팔) 304개, 3회 실행 불일치 없음 |
| E7 정확도 | FAIL | 케이스 72개, 위반 P_scalar_fd/A/default, P_scalar_fd/L/default; 최악 err(N)/max(err(D), err(R)) = 50.65 (P_scalar_fd/A/default) |
| E8 콜백 케이스 | PASS | F2 성공 예; P_fatal CallbackFailed{Rhs, 'sensor offline'} 예; N2 첫 pilot에서 CallbackFailed 예 |
| E9 V03b 작업 한계 | PASS | P_heat_fd/A/default: 과금 rhs 945 ≤ 5890, handover 예; P_scalar_fd/A/default: 과금 rhs 145 ≤ 181818, handover 예 |
| E10 pilot 회계 | PASS | pilot 있는 N 실행 62개(없는 실행 14개), 위반 없음; pilot 지출/DOPRI5 지출 최대 콜백 0.5342, 연산 0.9641 |

건전성(arms.sanity): 두 checkout으로 빌드한 D와 R의 비트 일치 152건 비교, 불일치 0건 → PASS

### E3 (a) 실패 내역

| 케이스 | 종료 방식 | 실패한 검사 | 콜백 N / D | 연산(x3) N / D |
|---|---|---|---|---|
| N2/A/default | fatal pilot (D1) | final_time_bits_equal_D, final_state_bits_equal_D, dopri5_leg_counters_equal_D, dopri5_leg_attempts_equal_D, rhs_ft_equal_D_plus_pilots, attempts_equal_D_plus_pilots, op3_equal_D_plus_pilots | 33 / 2161 | 2352 / 112320 |

handover 없는 실행 중 치명적 pilot(D1)로 끝난 실행을 빼면 (a)가 모두 성립하는가(기술적 정보, 판정 기준 아님): 예

## 4. 비용 표 (첫 번째 실행; 콜백 = 과금된 rhs + jvp + ft + Jacobian 콜백, 연산 = 사전 등록 공식; N/T 비는 둘 다 성공한 케이스만)

| 케이스 | T 콜백 | N 콜백 | D 콜백 | R 콜백 | N/T 콜백 | T 연산 | N 연산 | D 연산 | R 연산 | N/T 연산 | N pilot (수락/손실/치명) | handover t (DOPRI5 시도) | 상태 N / T / D / R |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| P_scalar_fd/A/default | 80 | 154 | 600001 | 78 | 1.93 | 880 | 1632 | 5.2e+06 | 880 | 1.85 | 1 (1/0/0) | 1.819e-05 (6) | ok / ok / DOPRI5 attempt budget exhaus / ok |
| lin_2000/A/default | 4023 | 687 | 4021 | 434 | 0.171 | 34840 | 6605.33 | 34840 | 6449.33 | 0.19 | 6 (1/5/0) | 0.07541 (96) | ok / ok / ok / ok |
| lin_3200/A/default | 6043 | 687 | 6259 | 434 | 0.114 | 52586.7 | 6605.33 | 54236 | 6449.33 | 0.126 | 6 (1/5/0) | 0.04711 (96) | ok / ok / ok / ok |
| lin_3400/A/default | 436 | 687 | 6631 | 434 | 1.58 | 6449.33 | 6605.33 | 57460 | 6449.33 | 1.02 | 6 (1/5/0) | 0.04434 (96) | ok / ok / ok / ok |
| P_heat_fd/A/default | 723 | 945 | 58411 | 714 | 1.31 | 105728 | 86869.3 | 4.04976e+06 | 105728 | 0.822 | 5 (1/4/0) | 1.146 (112) | ok / ok / ok / ok |
| robertson_40/A/default | 6233 | 385 | 216973 | 415 | 0.0618 | 166716 | 17040 | 5.64127e+06 | 19722 | 0.102 | 2 (1/1/0) | 0.00179 (8) | ok / ok / ok / ok |
| robertson_1e5/A/default | 2063 | 2041 | 600001 | 2059 | 0.989 | 96990 | 94872 | 1.56e+07 | 96990 | 0.978 | 2 (1/1/0) | 0.00179 (8) | ok / ok / DOPRI5 attempt budget exhaus / ok |
| hires/A/default | 2651 | 2444 | 64507 | 2642 | 0.922 | 415573 | 329131 | 4.47242e+06 | 415573 | 0.792 | 5 (1/4/0) | 2.651 (112) | ok / ok / ok / ok |
| orego/A/default | 10825 | 10870 | 600001 | 10821 | 1 | 514446 | 512670 | 1.56e+07 | 514446 | 0.997 | 4 (1/3/0) | 0.5831 (32) | ok / ok / DOPRI5 attempt budget exhaus / ok |
| vdp_1000/A/default | 5698 | 292 | 5695 | 223 | 0.0512 | 98696 | 6328 | 98696 | 6984 | 0.0641 | 4 (1/3/0) | 0.01497 (32) | ok / ok / ok / ok |
| flame_1e-2/A/default | 585 | 633 | 583 | 685 | 1.08 | 5044 | 5777.33 | 5044 | 10256.7 | 1.15 | 5 (0/5/0) | - | ok / ok / ok / ok |
| flame_1e-3/A/default | 2457 | 1263 | 2455 | 1008 | 0.514 | 21268 | 11597.3 | 21268 | 15372 | 0.545 | 7 (1/6/0) | 1307 (192) | ok / ok / ok / ok |
| flame_1e-4/A/default | 6123 | 1283 | 19495 | 1375 | 0.21 | 53760 | 11890.7 | 168948 | 21636.7 | 0.221 | 7 (1/6/0) | 1.023e+04 (192) | ok / ok / ok / ok |
| heat_64_fd/A/default | 2993 | 3328 | 314119 | 2993 | 1.11 | 1.0524e+07 | 1.02769e+07 | 1.74231e+08 | 1.0524e+07 | 0.977 | 1 (1/0/0) | 0.01644 (80) | ok / ok / ok / ok |
| kt_vanishing/A/default | 1580 | 1538 | 1435 | 1578 | 0.973 | 18460 | 17480.7 | 12428 | 18460 | 0.947 | 3 (1/2/0) | 0.0005244 (12) | ok / ok / ok / ok |
| P_scalar_fd/L/default | 80 | 148 | 600001 | 78 | 1.85 | 880 | 1580 | 5.2e+06 | 880 | 1.8 | 1 (1/0/0) | 4e-05 (5) | ok / ok / DOPRI5 attempt budget exhaus / ok |
| lin_3200/L/default | 6033 | 173 | 6049 | 154 | 0.0287 | 52440 | 2090.67 | 52416 | 2342.67 | 0.0399 | 3 (1/2/0) | 0.003143 (12) | ok / ok / ok / ok |
| P_heat_fd/L/default | 281 | 259 | 59095 | 272 | 0.922 | 40277.3 | 33952 | 4.09718e+06 | 40277.3 | 0.843 | 1 (1/0/0) | 0.1792 (9) | ok / ok / ok / ok |
| robertson_40/L/default | 6149 | 181 | 300019 | 199 | 0.0294 | 162768 | 7956 | 7.80047e+06 | 9570 | 0.0489 | 1 (1/0/0) | 0.002858 (4) | ok / ok / ok / ok |
| vdp_1000/L/default | 5740 | 97 | 5737 | 106 | 0.0169 | 99424 | 2520 | 99424 | 3325.33 | 0.0253 | 1 (1/0/0) | 0.001669 (5) | ok / ok / ok / ok |
| P_scalar_fd/S/default | 93 | 180 | 600001 | 91 | 1.94 | 1026.67 | 1925.33 | 5.2e+06 | 1026.67 | 1.88 | 1 (1/0/0) | 1.256e-05 (6) | ok / ok / DOPRI5 attempt budget exhaus / ok |
| lin_3200/S/default | 6053 | 1273 | 6739 | 964 | 0.21 | 52733.3 | 11744 | 58396 | 14222.7 | 0.223 | 7 (1/6/0) | 0.06491 (192) | ok / ok / ok / ok |
| P_heat_fd/S/default | 1590 | 1566 | 58435 | 1581 | 0.985 | 234112 | 125909 | 4.05142e+06 | 234112 | 0.538 | 6 (1/5/0) | 2.257 (224) | ok / ok / ok / ok |
| robertson_40/S/default | 6833 | 1107 | 214045 | 1230 | 0.162 | 194916 | 48384 | 5.56514e+06 | 59112 | 0.248 | 4 (1/3/0) | 0.009727 (32) | ok / ok / ok / ok |
| vdp_1000/S/default | 5842 | 495 | 5839 | 432 | 0.0847 | 101192 | 9989.33 | 101192 | 13317.3 | 0.0987 | 5 (1/4/0) | 0.02213 (64) | ok / ok / ok / ok |
| osc_1/A/default | 58 | 55 | 55 | 77 | 0.948 | 936 | 936 | 936 | 2333.33 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| osc_10/A/default | 382 | 423 | 379 | 467 | 1.11 | 6552 | 7885.33 | 6552 | 14944 | 1.2 | 4 (0/4/0) | - | ok / ok / ok / ok |
| osc_1000/A/default | 41406 | 35582 | 35461 | 42580 | 0.859 | 1.25264e+06 | 618307 | 614640 | 1.38155e+06 | 0.494 | 11 (0/11/0) | - | ok / ok / ok / ok |
| lotka_volterra/A/default | 6180 | 6143 | 6055 | 18431 | 0.994 | 109333 | 107603 | 104936 | 582629 | 0.984 | 8 (0/8/0) | - | ok / ok / ok / ok |
| lorenz/A/default | 2129 | 2209 | 2125 | 5342 | 1.04 | 55224 | 59172 | 55224 | 251508 | 1.07 | 7 (0/7/0) | - | ok / ok / ok / ok |
| arenstorf/A/default | 4343 | 1369 | 1291 | 4338 | 0.315 | 301536 | 49776 | 44720 | 301536 | 0.165 | 6 (0/6/0) | - | ok / ok / ok / ok |
| brusselator_2/A/default | 982 | 1045 | 979 | 2450 | 1.06 | 16952 | 18952 | 16952 | 77752 | 1.12 | 6 (0/6/0) | - | ok / ok / ok / ok |
| vdp_1/A/default | 1438 | 1501 | 1435 | 3472 | 1.04 | 24856 | 26856 | 24856 | 112797 | 1.08 | 6 (0/6/0) | - | ok / ok / ok / ok |
| damped_osc/A/default | 42575 | 35582 | 35461 | 42572 | 0.836 | 1.38154e+06 | 618307 | 614640 | 1.38154e+06 | 0.448 | 11 (0/11/0) | - | ok / ok / ok / ok |
| nonnormal/A/default | 487 | 423 | 379 | 484 | 0.869 | 14666.7 | 7885.33 | 6552 | 14666.7 | 0.538 | 4 (0/4/0) | - | ok / ok / ok / ok |
| lorenz96_40/A/default | 33467 | 3233 | 3037 | 33467 | 0.0966 | 5.7117e+07 | 1.35883e+06 | 1.05248e+06 | 5.7117e+07 | 0.0238 | 4 (0/4/0) | - | ok / ok / ok / ok |
| pleiades/A/default | 10758 | 1577 | 1429 | 10758 | 0.147 | 1.13228e+07 | 475851 | 346528 | 1.13228e+07 | 0.042 | 4 (0/4/0) | - | ok / ok / ok / ok |
| osc_1/L/default | 40 | 37 | 37 | 66 | 0.925 | 624 | 624 | 624 | 2000 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| osc_10/L/default | 94 | 113 | 91 | 143 | 1.2 | 1560 | 2226.67 | 1560 | 4333.33 | 1.43 | 2 (0/2/0) | - | ok / ok / ok / ok |
| osc_1000/L/default | 6378 | 6328 | 6229 | 9031 | 0.992 | 115333 | 110952 | 107952 | 273667 | 0.962 | 9 (0/9/0) | - | ok / ok / ok / ok |
| lotka_volterra/L/default | 1930 | 2004 | 1927 | 6369 | 1.04 | 33384 | 35717.3 | 33384 | 211680 | 1.07 | 7 (0/7/0) | - | ok / ok / ok / ok |
| lorenz/L/default | 719 | 775 | 715 | 1485 | 1.08 | 18564 | 21384 | 18564 | 75654 | 1.15 | 5 (0/5/0) | - | ok / ok / ok / ok |
| arenstorf/L/default | 1558 | 431 | 379 | 1553 | 0.277 | 114229 | 16474.7 | 13104 | 114229 | 0.144 | 4 (0/4/0) | - | ok / ok / ok / ok |
| brusselator_2/L/default | 424 | 476 | 421 | 861 | 1.12 | 7280 | 8946.67 | 7280 | 28808 | 1.23 | 5 (0/5/0) | - | ok / ok / ok / ok |
| vdp_1/L/default | 460 | 512 | 457 | 1035 | 1.11 | 7904 | 9570.67 | 7904 | 34760 | 1.21 | 5 (0/5/0) | - | ok / ok / ok / ok |
| damped_osc/L/default | 9001 | 6310 | 6211 | 8998 | 0.701 | 272667 | 110640 | 107640 | 272667 | 0.406 | 9 (0/9/0) | - | ok / ok / ok / ok |
| nonnormal/L/default | 157 | 160 | 127 | 154 | 1.02 | 4666.67 | 3184 | 2184 | 4666.67 | 0.682 | 3 (0/3/0) | - | ok / ok / ok / ok |
| lorenz96_40/L/default | 8764 | 963 | 865 | 8764 | 0.11 | 1.62583e+07 | 452693 | 299520 | 1.62583e+07 | 0.0278 | 2 (0/2/0) | - | ok / ok / ok / ok |
| pleiades/L/default | 4293 | 621 | 547 | 4293 | 0.145 | 5.05747e+06 | 197157 | 132496 | 5.05747e+06 | 0.039 | 2 (0/2/0) | - | ok / ok / ok / ok |
| osc_1/S/default | 100 | 119 | 97 | 132 | 1.19 | 1664 | 2330.67 | 1664 | 4000 | 1.4 | 2 (0/2/0) | - | ok / ok / ok / ok |
| osc_10/S/default | 838 | 901 | 835 | 1042 | 1.08 | 14456 | 16456 | 14456 | 32594.7 | 1.14 | 6 (0/6/0) | - | ok / ok / ok / ok |
| osc_1000/S/default | 99929 | 81319 | 81187 | 101391 | 0.814 | 3.05867e+06 | 1.41122e+06 | 1.40722e+06 | 3.1894e+06 | 0.461 | 12 (0/12/0) | - | ok / ok / ok / ok |
| lotka_volterra/S/default | 30045 | 14073 | 13963 | 42169 | 0.468 | 840440 | 245341 | 242008 | 1.292e+06 | 0.292 | 10 (0/10/0) | - | ok / ok / ok / ok |
| lorenz/S/default | 4895 | 4987 | 4891 | 13636 | 1.02 | 127140 | 131652 | 127140 | 641760 | 1.04 | 8 (0/8/0) | - | ok / ok / ok / ok |
| arenstorf/S/default | 10134 | 2912 | 2821 | 10129 | 0.287 | 667275 | 103659 | 97760 | 667275 | 0.155 | 7 (0/7/0) | - | ok / ok / ok / ok |
| brusselator_2/S/default | 2026 | 2100 | 2023 | 5526 | 1.04 | 35048 | 37381.3 | 35048 | 169832 | 1.07 | 7 (0/7/0) | - | ok / ok / ok / ok |
| vdp_1/S/default | 2980 | 3054 | 2977 | 7909 | 1.02 | 51584 | 53917.3 | 51584 | 248384 | 1.05 | 7 (0/7/0) | - | ok / ok / ok / ok |
| damped_osc/S/default | 101098 | 81541 | 81409 | 101095 | 0.807 | 3.17636e+06 | 1.41507e+06 | 1.41107e+06 | 3.17636e+06 | 0.446 | 12 (0/12/0) | - | ok / ok / ok / ok |
| nonnormal/S/default | 1147 | 955 | 889 | 1144 | 0.833 | 34666.7 | 17392 | 15392 | 34666.7 | 0.502 | 6 (0/6/0) | - | ok / ok / ok / ok |
| lorenz96_40/S/default | 90020 | 8755 | 8461 | 90020 | 0.0973 | 1.49943e+08 | 3.39232e+06 | 2.9328e+06 | 1.49943e+08 | 0.0226 | 6 (0/6/0) | - | ok / ok / ok / ok |
| pleiades/S/default | 25628 | 3276 | 3091 | 25628 | 0.128 | 2.36507e+07 | 911493 | 749840 | 2.36507e+07 | 0.0385 | 5 (0/5/0) | - | ok / ok / ok / ok |
| F2/A/default | 2 | 49 | 49 | 2 | - | 0 | 416 | 416 | 130 | - | 0 (0/0/0) | - | ok / callback-failed / ok / callback-failed |
| N2/A/default | 2 | 33 | 2161 | 2 | - | 0 | 784 | 37440 | 264 | - | 1 (0/0/1) | - | callback-failed / callback-failed / ok / callback-failed |
| P_fatal/A/default | 30 | 28 | 28 | 44 | - | 260 | 260 | 260 | 719.333 | - | 0 (0/0/0) | - | callback-failed / callback-failed / invalid numerical input: sen / callback-failed |
| P_blowup/A/default | 2754 | 2754 | 2443 | 2754 | - | 44880 | 44880 | 21164 | 44880 | - | 0 (0/0/0) | - | step-size-too-small / step-size-too-small / DOPRI5 cannot advance under  / step-size-too-small |
| P_mass/A/default | 190 | 190 | - | 190 | 1 | 10716 | 10716 | - | 10716 | 1 | 0 (0/0/0) | - | ok / ok / error: unsupported explicit- / ok |
| P_heat_dense/A/default | 369 | 369 | 5845 | 369 | 1 | 103211 | 103211 | 405184 | 103211 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| P_scalar_dense/A/default | 60 | 60 | 600001 | 60 | 1 | 880 | 880 | 5.2e+06 | 880 | 1 | 0 (0/0/0) | - | ok / ok / DOPRI5 attempt budget exhaus / ok |
| P_heat_jvp/A/default | 340 | 340 | 43 | 340 | 1 | 10240 | 10240 | 2912 | 10240 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| P_heat_banded/A/default | 45 | 45 | 43 | 45 | 1 | 12586.7 | 12586.7 | 2912 | 12586.7 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| robertson_40/A/regime=stiff | 415 | 415 | 216973 | 415 | 1 | 19722 | 19722 | 5.64127e+06 | 19722 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| robertson_40/A/regime=nonstiff | 216973 | 216973 | 216973 | 415 | 1 | 5.64127e+06 | 5.64127e+06 | 5.64127e+06 | 19722 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| osc_10/A/regime=stiff | 467 | 467 | 379 | 467 | 1 | 14944 | 14944 | 6552 | 14944 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| osc_10/A/regime=nonstiff | 379 | 379 | 379 | 467 | 1 | 6552 | 6552 | 6552 | 14944 | 1 | 0 (0/0/0) | - | ok / ok / ok / ok |
| robertson_40/A/max_steps=500 | 1769 | 385 | 3001 | 415 | 0.218 | 51408 | 17040 | 78000 | 19722 | 0.331 | 2 (1/1/0) | 0.00179 (8) | ok / ok / DOPRI5 attempt budget exhaus / ok |
| robertson_40/A/max_steps=50 | 443 | 385 | 301 | 415 | 0.869 | 17436 | 17040 | 7800 | 19722 | 0.977 | 2 (1/1/0) | 0.00179 (8) | ok / ok / DOPRI5 attempt budget exhaus / ok |

## 5. 선언된 예상 손실과 측정값

사전 등록 문구: not failures: P_scalar_fd (model 145 vs 74 charged right-hand sides), P_heat_fd (945 vs 723), lin_3400 (687 vs 436), heat_64_fd (3328 vs 2993), short non-stiff runs +0.3 to +26 percent in callbacks (up to +46 percent in operations at rtol 1e-3)

| 케이스 | 모델 N | 모델 T | 측정 N 콜백 | 측정 T 콜백 | 측정 N 연산(x3) | 측정 T 연산(x3) |
|---|---|---|---|---|---|---|
| P_scalar_fd/A/default | 145 | 74 | 154 | 80 | 4896 | 2640 |
| P_heat_fd/A/default | 945 | 723 | 945 | 723 | 260608 | 317184 |
| lin_3400/A/default | 687 | 436 | 687 | 436 | 19816 | 19348 |
| heat_64_fd/A/default | 3328 | 2993 | 3328 | 2993 | 30830592 | 31571968 |

| 허용오차 | 비강성 케이스 | N이 T보다 콜백이 많은 케이스 | 콜백 증가 최소-최대 | 연산 증가 케이스 | 연산 증가 최대 |
|---|---|---|---|---|---|
| A | 12 | 4 | +3.8% ~ +10.7% | 4 | +20.4% |
| L | 12 | 6 | +1.9% ~ +20.2% | 5 | +42.7% |
| S | 12 | 5 | +1.9% ~ +19.0% | 5 | +40.1% |

## 6. 해석 규칙과 열린 문제

- R1 (PHASE2_NOTES.txt 7): E3 (a)는 handover 없는 모든 N 실행에 문자 그대로 적용했습니다. 치명적 pilot으로 끝난 실행(D1)도 포함되므로, N2처럼 D1이 요구하는 종료는 D의 최종 상태 비트를 가질 수 없습니다. 이 경우를 따로 표시했지만 대체 기준으로 쓰지 않았습니다.
- R2: E5 (ii)/(iii)은 DOPRI5로 시작하는 실행에만 적용했습니다(선언된 규칙으로 RODAS5P로 시작하는 실행은 E3 (b)가 다룹니다). (i) 미과금 항목 0은 모든 실행에 적용했습니다.
- R3: E1의 최대비는 D 또는 R 중 하나라도 성공한 케이스에서만 계산했습니다. R4: E7은 N의 오차와 기준해가 있는 케이스에서만 계산했습니다. R5: 추가 허용오차 쌍은 기본 옵션의 해당 문제에만 적용했습니다.
- R6 (addendum 0b): E10은 pilot이 하나 이상인 N 실행에 적용했습니다(pilot이 없으면 셀 것이 없고, A_D = 0이면 공식이 정의되지 않습니다).
- E5 (iii)은 runner의 재생 분해(e5_split)로 판정했습니다: N의 콜백 항목이 비트 단위로 Dopri5::solve 항목의 접두부, 각 pilot의 재생(Dopri5::solve가 같은 시도 수 뒤 수락한 점에서 Rodas5p::solve, h0 = H, max_steps = 1), handover 뒤 RODAS5P 재생으로 정확히 나뉘어야 합니다. Solution은 dense output 없이 t0와 t1만 기록하므로 수락점 목록 대신 이 방법을 썼습니다.
- runner의 분류기는 비자율 문제에서 RODAS5P 시도의 첫 단계 f(t, y)를 옆의 유한차분 시간미분 호출과 같은 상태라는 이유로 시간미분으로 표시합니다. 종류별 수는 측정값으로만 보고하며 어떤 판정에도 쓰지 않았습니다.
- 탐색용 팔 N_prod와 N_cont는 AMENDMENT_A8_2에 따라 실행하지 않았습니다.
- 예제 examples/unified_production_solver.rs와 contract_4의 주석은 break-even 경로를 설명하도록 고쳤습니다(코드와 단언은 그대로).

