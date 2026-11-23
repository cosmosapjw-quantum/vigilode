# V16 레벨 구조 Rosenbrock-Wanner 타블로 탐색 (루프 L1, 2026-11-17)

- 노드: `V16_LEVEL_TABLEAU_SEARCH` (사전등록 부록 A1). Python 프로토타입만 사용, Rust 코드와 보호 경로는 변경 없음.
- 이 문서는 `render_report.py`가 `RESULTS.json`에서 생성한다. `RESULTS.json`은 `parse_results.py`가 `logs/*.jsonl`만 읽어 만든다.
- 브랜치 `research/production-loop-20261116-w2-v16`, 생성 시점 HEAD `3777768a`.
- 사전선언: `PREDECLARED.json` (첫 측정 전 커밋). 수정 기록 3건: A-1, A-2, A-3. 내용은 2절.

## 1. 결론

도구 검증: 생성된 차수 조건 17개(차수 5 이하)가 보호된 RODAS5P 계수에서 최대 8.94e-15 (binary64), 5.52e-15 (60자리)로 소멸한다. 기준 1.00e-12: 통과.

| 레이아웃 | 판정 | 1순위 후보 | gamma | 차수 5 잔차 (binary64 / 정확 산술) | 관측 차수 (3문제) | 시도 횟수 비 = 후보 / RODAS5P (정합 오차) | 동결 기준 |
|---|---|---|---|---|---|---|---|
| 깊이 5 (8단, 레벨 1-2-2-2-1) | 발견(대수 조건 충족), pay-off 스크린 미통과 (KILL 수준은 아님) | D5F_r01 | 0.278054 | 1.18e-16 / 1.37e-40 | 5.237, 5.037, 4.985 | ROBER 3.368, VDPOL 2.423, BURGERS 1.29 | 통과 <= 1.25 (3문제 중 2), KILL >= 1.6 (3문제 모두) |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | 발견(대수 조건 충족), pay-off 스크린 미통과 (KILL 수준은 아님) | D6L_r04 | 0.334142 | 3.61e-16 / 1.49e-39 | 6.095, 5.281, 4.979 | ROBER 1.246, VDPOL 1.482, BURGERS 1.289 | 통과 <= 1.1 (3문제 중 2), KILL >= 1.33 (3문제 모두) |

노드 판정 (동결 행의 규칙 적용): **FAIL (통과한 레이아웃 없음, 모든 발견 레이아웃이 KILL 수준인 것은 아님)**.

탐색적 관찰 (판정에 쓰이지 않음): 개발 문제 순위에서 1순위가 아니었던 `D6T_r07` (깊이 6 (10단, 레벨 1-2-2-2-2-1), gamma 0.19)를 같은 스크린에 함께 돌렸다. 시도 횟수 비는 ROBER 1.377, VDPOL 1.011, BURGERS 0.8487. 관측 차수는 BRUSS_ODE 4.799 (FAIL), VDP_MU2 4.999 (PASS), PENDULUM_FORCED 4.984 (PASS). 코드형 계수 max|C| = 1555. 이 후보는 사전선언된 선택 규칙이 고른 후보가 아니므로 레이아웃의 판정을 바꾸지 않는다. 단서로만 취급한다(12절).

구조적 설명 (탐색적 분석, 8절).

- 깊이 L의 강성 정확 타블로의 안정성 함수는 P(z)/(1 - gamma z)^L이고 gamma만으로 정해진다. RODAS5P는 극 7개로 선형 차수 6(z^7 결함 -1.02e-05)을 갖지만, 깊이 5는 선형 차수 5(z^6 결함 5.30e-04)에 묶이고 깊이 6은 gamma = 0.33414에서만 선형 차수 6이 되며 그때의 z^7 결함은 -3.41e-04로 RODAS5P의 33.6배다(8.1절).
- Prothero-Robinson 인자 W_2(z)를 항등적으로 0으로 만들면(RODAS5P가 가진 성질) 안정성 함수의 극이 하나 줄어든다(8.2b절의 항등식). 그래서 깊이 5에서는 불가능하고 깊이 6에서는 gamma가 깊이 5의 값 0.27805로 고정된다. 여기의 후보들은 이 조건을 정확히 만족하지 못하며 강성 Prothero-Robinson 문제에서 쌍별 차수가 1 근처로 떨어진다(8.3절).
- 깊이 5에서는 임베디드 방법을 강성 정확하게 만들 수 없다(3절).

읽는 법.

- "발견"은 동결 행의 대수 조건 전부(차수 5의 17개 조건, 강성 정확성과 R(무한대)=0, A-안정성, 임베디드 차수 4, sum b_i gamma_i = 0, 계수 크기 100 이하)를 독립 검증 스크립트가 계수 파일에서 다시 확인했다는 뜻이다.
- 시도 횟수 비는 같은 프로토타입, 같은 제어기 공식으로 적분했을 때 같은 달성 오차에서의 (수락+거부) 스텝 수의 비다. 1보다 크면 후보가 RODAS5P보다 스텝을 더 쓴다.
- 깊이 이득은 체인만의 상한이다: 8/5 = 1.6, 8/6 = 1.33. 시도 횟수 비가 이 값 이상이면 두 프로세서를 써도 체인에서 얻는 것이 없다.
- "찾지 못함"은 선언한 예산 안의 결과이며 불가능의 증명이 아니다. "발견"된 타블로는 보호 경로 밖의 새 방법이고, 라우팅과 Rust 이식은 이 노드의 범위가 아니다.

## 2. 절차와 사전선언 수정

1. `PREDECLARED.json`을 생성기, 탐색, 적분기의 어떤 실행보다 먼저 커밋했다. 동결 행이 정하지 않은 선택(레이아웃별 조건 목록, 미지수, 최적화기, 시작점 수와 시드, 시험 문제와 허용오차, 기준해, 정합 오차 보간 규칙, 제어기 공식, 계수 한계)을 담았다.
2. 선언한 시드 범위 밖의 스모크 실행과 RODAS5P 단독 실행에서 드러난 문제 때문에 선언을 3번 수정했다. 각 수정은 해당 실행 전에 커밋했고 동결 임계값은 건드리지 않았다. 리뷰어가 수정의 정당성을 판단해야 한다.

- **A-1** (2026-11-17, committed before any declared start (start indices 0..1999) was run). 범위: internals of the search optimiser only.
  - 요약: 탐색 최적화기의 내부만 바꿈: 1단계 함수 평가 150회, 경성 조건 다양체 위에서의 연마(시작점마다 60회, 상위 40개는 600회 추가), gamma 연속법의 풀이 방식, REG 가중치 1e-5, 깊이 6의 gamma 탐색 구간 [0.19, 0.33].
  - 근거: logs/smoke_search_D6T.jsonl and logs/smoke_search_D5F.jsonl: 8 starts each with start indices 1000000..1000007 (seeds outside the declared range), run with the optimiser as first declared; and an interactive test of the polish on the smoke starts 1000002 and 1000004, quoted in NOTES.txt entry 2
  - 변경: stage 1 of every start: least_squares as declared, but max_nfev = 150 (was 400)
  - 변경: after stage 1, a polish on the manifold of the hard conditions replaces the plain projection: projected Levenberg-Marquardt on the weighted soft terms, every iterate projected back onto the hard conditions by minimum-norm Gauss-Newton steps; 60 iterations per start (phase A)
  - 변경: phase B: the 40 best passing starts of a formulation by score get up to 600 further polish iterations
  - 변경: continuation in gamma (depth 6): from the 5 best phase-B results; each grid solve fixes gamma, projects and runs up to 200 polish iterations (instead of a full least_squares run)
  - 변경: weight of REG is 1e-5 (was 1e-3)
  - 변경: gamma of depth 6 is searched in [0.19, 0.33], inside the declared window, to keep a margin from the ends of the A-stability window
  - 변경: the refinement pool is the 8 best by score among the phase-B and continuation results (distinctness rule unchanged)
  - 유지: hard conditions, 2000 starts per formulation, seeds, start distribution, score definition, every FOUND check, candidate selection rule, order check, pay-off screen, all thresholds
- **A-2** (2026-11-17, committed before any candidate existed and before any order run of a candidate). 범위: round-off guard of the order check.
  - 요약: 차수 검사의 반올림 가드를 1e-11에서 1e-12로 낮춤(후보가 생기기 전, RODAS5P 단독 실행을 근거로).
  - 근거: logs/amend_A2_order_RODAS5P_floor1e-11.jsonl: the cross-check run of RODAS5P alone. On PENDULUM_FORCED its errors are 1.4e-7, 5.0e-9, 1.7e-10, 5.6e-12, 1.8e-13, 9.9e-15, so only two consecutive pairs lie above the declared guard 1e-11 and the declared rule (at least 3 pairs) would return NOT_DECIDABLE for the reference method itself. The reference uncertainty of the three problems is at most 3e-14 and the error at 320 steps (9.9e-15) shows the round-off level.
  - 변경: round-off guard of the order check: 1e-12 (was 1e-11)
  - 유지: problems, step numbers, error measure, reference, median rule, the minimum of 3 pairs, the threshold 4.8
- **A-3** (2026-11-17, committed while the declared D6T run was in progress and before any of its results was inspected; before any D6L start). 범위: one more formulation for depth 6; nothing is removed.
  - 요약: 깊이 6에 정식화 D6L을 추가: gamma를 선형 차수 6이 되는 근 0.33414...(아래로 반올림)에 고정. 깊이 6의 후보 풀은 D6T 상위 8개 + D6L 상위 8개.
  - 근거: logs/dev_smoke_payoff.jsonl: development runs on HIRES and OREGO (development problems) with the smoke candidates candidates/smoke_D6T_r01.json and candidates/smoke_D5F_r01.json (seeds outside the declared range). Attempts at matched error were 2 to 3 times those of RODAS5P, growing as the tolerance is tightened. Explanation by the stability function, which for a depth-L stiffly accurate tableau is P(z)/(1 - gamma z)^L with P fixed by gamma: RODAS5P has linear order 6 with defect 1.0e-5 at z^7; depth 5 has linear order 5 with defect 5.3e-4 at z^6 (theory S9); depth 6 has a z^6 defect that vanishes only at the root gamma = 0.3341423670680504... of 720 g^6 - 4320 g^5 + 5400 g^4 - 2400 g^3 + 450 g^2 - 36 g + 1 (then the z^7 defect is 3.4e-4). That root is the upper end of the declared window; amendment A-1 had cut the searched range at 0.33. No pay-off problem (ROBER, VDPOL, BURGERS) was run with any candidate.
  - 변경: additional formulation D6L for depth 6: as D6T, but gamma is fixed at that root rounded down to 40 decimals (0.3341423670680504359540300929339359425765), so that the value lies on the A-stable side; 73 unknowns; hard conditions O5, E4, W, C as D6T (27); seed code 63; 2000 starts; phases A and B of amendment A-1; no continuation
  - 변경: depth-6 pool for refinement and selection: the 8 best of D6T (phase B and continuation) and the 8 best of D6L (phase B); the selection rule (development problems) is unchanged and picks one primary candidate for the layout
  - 유지: D6T and D5F runs as declared, all FOUND checks, selection rule, order check, pay-off screen, all thresholds

3. 후보 선택은 pay-off 문제(ROBER, VDPOL, BURGERS)를 보지 않고 했다: 정밀화된 후보를 개발 문제(HIRES, OREGO; U11에서 이미 소비된 설정이며 여기서는 개발용으로만 사용)에서 돌려 시도 횟수 비의 기하평균이 가장 낮은 후보를 레이아웃의 1순위로 정했다. 판정은 1순위 후보만으로 한다.

## 3. 도구 검증

- 생성기 `rowtrees.py`: 뿌리 나무를 차수 6까지 열거하고(차수별 1, 1, 2, 4, 9개, 차수 6은 20개), 가지가 하나인 꼭짓점에는 beta(대각 gamma 포함), 둘 이상인 꼭짓점에는 alpha를 쓰는 재귀로 Phi를 만든다. 조건은 `sum_j b_j Phi_j(t) = 1/gamma(t)`. 출처는 Hairer-Wanner II, IV.7절이며 기억에 의한 인용이다(책을 확인하지 않음). 공식은 다시 유도했고 RODAS5P에서 수치로 검증했다.
- 동결 검증: 17개 조건의 RODAS5P 잔차 최대 8.94e-15 (binary64). 기준 1.00e-12 이하이므로 통과.
- 생성기를 쓰지 않는 수치 교차 확인: 프로토타입 적분기에서 RODAS5P 자체의 고정 스텝 관측 차수 (6절 표의 RODAS5P 행).
- 독립 검증기 `verify_candidate.py`는 17개 조건을 손으로 풀어 쓴 식으로 계산하며 생성기와 코드를 공유하지 않는다. 후보 형식으로 옮겨 쓴 RODAS5P에 적용한 자체 시험(스크래치, `NOTES.txt` 3번 항목)에서 생성기와 같은 잔차를 냈다.
- 지수-1 확장(동결 검증 대상 아님): 두 색 나무 60개를 생성했고 그중 58개가 RODAS5P 주 가중치에서 1e-10 이하로 소멸한다. 소멸하지 않는 것: {o,{o,{o,[o]}}} (차수 5, 잔차 0.0469); {o,{o,{o,{o,o}}}} (차수 5, 잔차 0.281). 소멸하는 조건만 탐색의 연성 목표로 썼다.
- 깊이 5의 구조적 사실: gamma는 0.278053841136...로 고정된다. 이론 S9의 44-미지수 변형(임베디드 가중치 = beta의 7행)은 불가능하다: 그 임베디드 방법은 네 레벨만 쓰고 R_hat(무한대)=0이므로 차수 4에 `24 g^4 - 96 g^3 + 72 g^2 - 16 g + 1 = 0`이 필요한데, 고정된 gamma에서 이 값은 0.1974이다. 그래서 깊이 5에서는 임베디드 가중치를 자유 벡터로 두었고 임베디드 방법은 강성 정확이 아니다.

## 4. 탐색

| 정식화 | 레이아웃 | 미지수 | 경성 조건 | 시작점 | 완료 | binary64 통과 | gamma 창 밖 | 점수 최소 (A) | 점수 중앙 (A) | B단계 수 | 점수 최소 (B) | 점수 중앙 (B) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| D5F | 깊이 5 (8단, 레벨 1-2-2-2-1) | 58 | 28 | 2000 | 예 | 368 | 0 | 0.0118 | 0.0347 | 40 | 0.0117 | 0.0118 |
| D6L | 깊이 6 (10단, 레벨 1-2-2-2-2-1) | 73 | 27 | 2000 | 예 | 1090 | 0 | 0.00112 | 0.00411 | 40 | 0.00101 | 0.00113 |
| D6T | 깊이 6 (10단, 레벨 1-2-2-2-2-1) | 74 | 27 | 2000 | 예 | 551 | 499 | 3.35e-04 | 0.00244 | 40 | 1.93e-04 | 3.36e-04 |

- 통과 기준(binary64): 사영 뒤 경성 조건 잔차 최대 1e-12 이하, gamma가 탐색 구간 안, 계수 크기 100 이하. 실패한 시작점도 로그에 남아 있다.
- 점수 = sqrt(|D|^2 + |Dhat|^2 + |PR|^2 + 0.09 |PRhat|^2 + 0.01 |E6|^2): 지수-1 조건, Prothero-Robinson 항, 차수 6 오차 계수의 가중 노름. 낮을수록 좋다. FOUND 판정에는 쓰이지 않는다.
- 깊이 6의 gamma 연속법(D6T, B단계 상위 5개에서 출발, gamma 고정 격자): 71회 풀이 중 70회 통과. gamma별 최저 점수: 0.2: 1.41e-04, 0.21: 1.66e-04, 0.22: 1.54e-04, 0.23: 1.45e-04, 0.24: 1.58e-04, 0.25: 1.83e-04, 0.26: 2.14e-04, 0.27: 2.66e-04, 0.28: 3.48e-04, 0.29: 4.76e-04, 0.3: 5.84e-04, 0.31: 6.86e-04, 0.32: 7.85e-04, 0.33: 9.41e-04.

## 5. 후보의 대수 검증 (계수 파일에서, 독립 스크립트)

| 후보 | gamma | 점수 | 차수5 잔차 b64 | 차수5 잔차 정확 | 임베디드 차수4 잔차 | 임베디드 차수5 잔차(0이 아니어야 함) | sum b gamma_i | R(무한대) | A-안정 | 최대 계수 | 대수 통과 | HIRES 비 | OREGO 비 | 개발 기하평균 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| D5F_r01 | 0.278054 | 0.0117 | 1.18e-16 | 1.37e-40 | 1.26e-40 | 0.00899 | 6.39e-41 | 0 | 예 | 1.439 | 예 | 2.476 | 1.832 | 2.13 |
| D5F_r02 | 0.278054 | 0.0117 | 3.33e-16 | 1.71e-39 | 1.82e-39 | 0.00461 | -1.21e-39 | 0 | 예 | 6.051 | 예 | 2.731 | 1.856 | 2.251 |
| D5F_r03 | 0.278054 | 0.0117 | 1.67e-16 | 1.89e-40 | 1.74e-40 | 0.00276 | 1.86e-40 | 0 | 예 | 2.514 | 예 | 2.473 | 1.903 | 2.169 |
| D5F_r04 | 0.278054 | 0.0117 | 1.67e-16 | 3.58e-40 | 3.39e-40 | 0.0121 | -5.31e-41 | 0 | 예 | 1.4 | 예 | 2.864 | 1.835 | 2.292 |
| D5F_r05 | 0.278054 | 0.0117 | 3.33e-16 | 2.39e-40 | 2.61e-40 | 0.00128 | 3.21e-40 | 0 | 예 | 2.886 | 예 | 2.605 | 1.924 | 2.239 |
| D5F_r06 | 0.278054 | 0.0117 | 5.55e-17 | 7.55e-40 | 5.38e-40 | 0.00288 | -2.34e-40 | 0 | 예 | 3.057 | 예 | 2.506 | 1.927 | 2.197 |
| D5F_r07 | 0.278054 | 0.0117 | 7.63e-17 | 3.07e-40 | 2.63e-40 | 0.00382 | 2.30e-40 | 0 | 예 | 2.352 | 예 | 2.718 | 1.725 | 2.165 |
| D5F_r08 | 0.278054 | 0.0117 | 3.05e-16 | 6.85e-40 | 5.50e-40 | 0.0082 | 1.63e-40 | 0 | 예 | 2.381 | 예 | 2.529 | 1.891 | 2.187 |
| D6L_r01 | 0.334142 | 0.00101 | 3.96e-16 | 5.59e-40 | 3.04e-40 | 0.0342 | -1.14e-40 | 0 | 예 | 3.122 | 예 | 1.98 | 1.114 | 1.485 |
| D6L_r02 | 0.334142 | 0.00101 | 9.44e-16 | 4.16e-40 | 8.17e-40 | 0.106 | -4.21e-40 | 0 | 예 | 2.448 | 예 | 1.939 | 1.209 | 1.531 |
| D6L_r03 | 0.334142 | 0.00103 | 2.22e-16 | 9.03e-40 | 3.16e-40 | 0.0338 | 3.19e-40 | 0 | 예 | 3.248 | 예 | 2.113 | 1.243 | 1.62 |
| D6L_r04 | 0.334142 | 0.00104 | 3.61e-16 | 1.49e-39 | 2.65e-40 | 0.0539 | 7.46e-40 | 0 | 예 | 2.444 | 예 | 1.784 | 1.063 | 1.377 |
| D6L_r05 | 0.334142 | 0.00104 | 5.00e-16 | 7.10e-40 | 1.61e-39 | 0.0958 | -3.95e-40 | 0 | 예 | 2.059 | 예 | 1.927 | 1.098 | 1.455 |
| D6L_r06 | 0.334142 | 0.00106 | 5.27e-16 | 1.72e-39 | 1.29e-39 | 0.0353 | -7.82e-40 | 0 | 예 | 2.152 | 예 | 2.229 | 1.231 | 1.657 |
| D6L_r07 | 0.334142 | 0.00107 | 3.33e-16 | 6.02e-40 | 6.75e-40 | 0.0295 | 9.22e-40 | 0 | 예 | 8.201 | 예 | 1.739 | 1.64 | 1.689 |
| D6L_r08 | 0.334142 | 0.00107 | 2.43e-16 | 6.66e-40 | 1.21e-39 | 0.0628 | -2.42e-40 | 0 | 예 | 2.06 | 예 | 2.291 | 1.347 | 1.757 |
| D6T_r01 | 0.2 | 1.41e-04 | 5.55e-17 | 2.67e-40 | 8.00e-41 | 0.0334 | -7.69e-41 | 0 | 예 | 4.463 | 예 | 1.905 | 1.497 | 1.689 |
| D6T_r02 | 0.23 | 1.45e-04 | 2.22e-16 | 6.17e-40 | 4.67e-40 | 0.0651 | 6.59e-42 | 0 | 예 | 3.974 | 예 | 2.076 | 2.213 | 2.143 |
| D6T_r03 | 0.22 | 1.54e-04 | 1.11e-16 | 5.20e-40 | 1.20e-40 | 0.0505 | 2.30e-40 | 0 | 예 | 4.032 | 예 | 2.154 | 1.987 | 2.069 |
| D6T_r04 | 0.24 | 1.58e-04 | 8.33e-17 | 7.97e-40 | 4.43e-41 | 0.0886 | 4.21e-40 | 0 | 예 | 3.887 | 예 | 1.918 | 2.28 | 2.092 |
| D6T_r05 | 0.21 | 1.66e-04 | 2.22e-16 | 5.78e-40 | 1.90e-40 | 0.0371 | 2.65e-40 | 0 | 예 | 4.074 | 예 | 2.466 | 1.803 | 2.108 |
| D6T_r06 | 0.25 | 1.83e-04 | 1.11e-16 | 5.70e-40 | 1.15e-40 | 0.131 | -3.29e-40 | 0 | 예 | 3.767 | 예 | 1.792 | 2.559 | 2.142 |
| D6T_r07 | 0.19 | 1.93e-04 | 2.22e-16 | 6.10e-40 | 8.10e-40 | 0.0265 | 1.03e-39 | 0 | 예 | 3.745 | 예 | 1.472 | 1.373 | 1.422 |
| D6T_r08 | 0.20165 | 1.94e-04 | 1.11e-16 | 5.50e-40 | 1.10e-40 | 0.0251 | -2.43e-40 | 0 | 예 | 4.148 | 예 | 2.039 | 1.553 | 1.779 |

- 계수 파일: `candidates/<후보>.json`. 독립 계수는 유효숫자 40자리 십진 문자열이고, 종속 계수(묶인 행, b, bhat)는 그 문자열의 정확한 십진 합이므로 구조적 항등식(b = beta의 마지막 행 등)은 파일 안에서 정확히 성립한다.
- 정확 산술 = 십진 문자열을 `fractions.Fraction`으로 읽은 유리수 계산. A-안정성 = 설계 안정성 함수의 E-다항식에 대한 Sturm 열(양의 근 0개) + 타블로 분자와 설계 분자의 차이 1e-25 이하 + 허수축 격자에서 |R(iy)| - 1 <= 1e-20.
- `candidates/smoke_*.json`은 선언 범위 밖 시드의 파이프라인 시험용 파일이며 판정에 쓰이지 않는다.

- 깊이 5 (8단, 레벨 1-2-2-2-1)의 1순위: `D5F_r01` (lowest geometric mean of the development attempt ratios). 개발 순위: D5F_r01 2.13, D5F_r07 2.165, D5F_r03 2.169, D5F_r08 2.187, D5F_r06 2.197, D5F_r05 2.239, D5F_r02 2.251, D5F_r04 2.292.
- 깊이 6 (10단, 레벨 1-2-2-2-2-1)의 1순위: `D6L_r04` (lowest geometric mean of the development attempt ratios). 개발 순위: D6L_r04 1.377, D6T_r07 1.422, D6L_r05 1.455, D6L_r01 1.485, D6L_r02 1.531, D6L_r03 1.62, D6L_r06 1.657, D6T_r01 1.689, D6L_r07 1.689, D6L_r08 1.757, D6T_r08 1.779, D6T_r03 2.069, D6T_r04 2.092, D6T_r05 2.108, D6T_r06 2.142, D6T_r02 2.143.

## 6. 고정 스텝 관측 차수 (정확한 야코비안)

| 방법 | 문제 | 오차 (스텝 수 10, 20, 40, 80, 160, 320) | 쌍별 차수 (오차 >= 1e-12인 쌍) | 관측 차수 (중앙값) | 판정 (>= 4.8) |
|---|---|---|---|---|---|
| D5F_r01 | BRUSS_ODE | 3.13e-04 4.87e-06 1.10e-07 2.93e-09 8.41e-11 2.52e-12 | 6.004 5.463 5.237 5.121 5.061 | 5.237 | PASS |
| D5F_r01 | VDP_MU2 | 8.71e-06 2.65e-07 7.98e-09 2.43e-10 7.50e-12 2.32e-13 | 5.039 5.053 5.035 5.02 | 5.037 | PASS |
| D5F_r01 | PENDULUM_FORCED | 2.85e-07 9.10e-09 2.87e-10 9.02e-12 2.83e-13 1.04e-14 | 4.969 4.985 4.992 | 4.985 | PASS |
| D6L_r04 | BRUSS_ODE | 1.05e-04 1.56e-06 1.90e-08 2.74e-10 5.16e-12 1.22e-13 | 6.073 6.361 6.116 5.73 | 6.095 | PASS |
| D6L_r04 | VDP_MU2 | 3.38e-05 9.13e-07 2.24e-08 5.57e-10 1.48e-11 4.16e-13 | 5.208 5.352 5.326 5.237 | 5.281 | PASS |
| D6L_r04 | PENDULUM_FORCED | 1.83e-06 5.96e-08 1.90e-09 5.99e-11 1.88e-12 5.99e-14 | 4.939 4.972 4.987 4.993 | 4.979 | PASS |
| D6T_r07 | BRUSS_ODE | 1.05e-05 1.09e-07 3.48e-09 2.39e-10 9.69e-12 3.19e-13 | 6.591 4.973 3.864 4.625 | 4.799 | FAIL |
| D6T_r07 | VDP_MU2 | 8.94e-06 2.92e-07 9.14e-09 2.85e-10 8.89e-12 2.93e-13 | 4.938 4.996 5.004 5.001 | 4.999 | PASS |
| D6T_r07 | PENDULUM_FORCED | 2.78e-07 8.91e-09 2.81e-10 8.87e-12 3.02e-13 3.41e-14 | 4.964 4.984 4.988 | 4.984 | PASS |
| RODAS5P | BRUSS_ODE | 6.22e-04 8.49e-06 1.89e-07 5.15e-09 1.52e-10 4.64e-12 | 6.196 5.493 5.195 5.082 5.034 | 5.195 | PASS |
| RODAS5P | VDP_MU2 | 2.24e-05 8.29e-07 2.68e-08 8.40e-10 2.62e-11 8.12e-13 | 4.755 4.953 4.994 5.001 | 4.973 | PASS |
| RODAS5P | PENDULUM_FORCED | 1.43e-07 4.97e-09 1.72e-10 5.65e-12 1.82e-13 9.88e-15 | 4.848 4.852 4.93 | 4.852 | PASS |

- 기준해: mpmath.odefun 40자리. DOP853(rtol 1e-13)과의 차이: BRUSS_ODE 3.00e-14, PENDULUM_FORCED 5.64e-15, VDP_MU2 1.44e-15.
- RODAS5P 행은 프로토타입의 검증이자 생성기와 무관한 교차 확인이다.

## 7. pay-off 스크린 (정합 달성 오차에서의 시도 횟수 비)

| 레이아웃 | 후보 | 역할 | 문제 | 비 (기하평균) | 목표별 최소 ~ 최대 | 쓸 수 있는 목표 수 | 기준해 불확도 u_ref | 실패 실행 | 통과 임계 | KILL 수준 |
|---|---|---|---|---|---|---|---|---|---|---|
| 깊이 5 (8단, 레벨 1-2-2-2-1) | D5F_r01 | 1순위 (판정에 사용) | ROBER | 3.368 | 2.745 ~ 4.034 | 10 | 6.13e-12 | 0 | 1.25 | 1.6 |
| 깊이 5 (8단, 레벨 1-2-2-2-1) | D5F_r01 | 1순위 (판정에 사용) | VDPOL | 2.423 | 1.334 ~ 3.904 | 8 | 6.66e-12 | 0 | 1.25 | 1.6 |
| 깊이 5 (8단, 레벨 1-2-2-2-1) | D5F_r01 | 1순위 (판정에 사용) | BURGERS | 1.29 | 1.192 ~ 1.356 | 6 | 1.83e-12 | 0 | 1.25 | 1.6 |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | D6L_r04 | 1순위 (판정에 사용) | ROBER | 1.246 | 1.014 ~ 2.25 | 10 | 6.13e-12 | 0 | 1.1 | 1.33 |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | D6L_r04 | 1순위 (판정에 사용) | VDPOL | 1.482 | 1.163 ~ 2.395 | 9 | 6.66e-12 | 0 | 1.1 | 1.33 |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | D6L_r04 | 1순위 (판정에 사용) | BURGERS | 1.289 | 1.177 ~ 1.374 | 8 | 1.83e-12 | 0 | 1.1 | 1.33 |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | D6T_r07 | 탐색적 (판정에 쓰이지 않음) | ROBER | 1.377 | 1.13 ~ 2.497 | 10 | 6.13e-12 | 0 | 1.1 | 1.33 |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | D6T_r07 | 탐색적 (판정에 쓰이지 않음) | VDPOL | 1.011 | 0.7659 ~ 1.353 | 9 | 6.66e-12 | 0 | 1.1 | 1.33 |
| 깊이 6 (10단, 레벨 1-2-2-2-2-1) | D6T_r07 | 탐색적 (판정에 쓰이지 않음) | BURGERS | 0.8487 | 0.7943 ~ 0.9873 | 8 | 1.83e-12 | 0 | 1.1 | 1.33 |

- 깊이 5 (8단, 레벨 1-2-2-2-1): 통과 임계 이하 0문제, 깊이 이득 이상 2문제 -> pay-off FAIL.
- 깊이 6 (10단, 레벨 1-2-2-2-2-1): 통과 임계 이하 0문제, 깊이 이득 이상 1문제 -> pay-off FAIL.
- 규칙: 허용오차 사다리 rtol = 10^(-k/2), k = 6..18, atol = a_scale * rtol. 각 방법의 (오차, 시도) 점에서 파레토 전선을 만들고 ln-ln 선형 보간으로 목표 오차 10^(-m/2)에서의 시도 횟수를 구한다. 두 방법의 전선 범위 안에 있고 100 u_ref 이상인 목표만 쓴다. 문제의 비는 쓸 수 있는 목표들의 기하평균이며 목표가 3개 미만이면 판정 불가.
- 기준해: scipy Radau (rtol 1e-13). u_ref는 Radau(rtol 1e-11)과의 차이와 RODAS5P 프로토타입(rtol 1e-11)과의 차이 중 큰 값.
- 벽시계 시간은 재지 않았다. 판정에 쓰인 수는 모두 결정적 계수기(스텝 시도 횟수)다.

### 깊이 5 (8단, 레벨 1-2-2-2-1), 후보 `D5F_r01` (1순위 (판정에 사용)): 허용오차별 원자료

ROBER:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 44+1 | 5.66e-05 | 34+0 | 0.00181 |
| 3.16e-04 | 50+0 | 2.61e-05 | 39+1 | 0.00113 |
| 1.00e-04 | 58+0 | 1.24e-05 | 46+0 | 9.09e-04 |
| 3.16e-05 | 68+0 | 5.30e-06 | 63+0 | 3.01e-04 |
| 1.00e-05 | 81+1 | 2.31e-06 | 89+0 | 1.11e-04 |
| 3.16e-06 | 97+4 | 9.14e-07 | 131+1 | 3.34e-05 |
| 1.00e-06 | 116+5 | 3.48e-07 | 202+1 | 5.60e-06 |
| 3.16e-07 | 139+4 | 1.28e-07 | 324+1 | 1.16e-06 |
| 1.00e-07 | 170+4 | 4.76e-08 | 537+1 | 1.32e-07 |
| 3.16e-08 | 228+2 | 1.68e-08 | 911+1 | 7.64e-09 |
| 1.00e-08 | 311+3 | 5.43e-09 | 1568+1 | 1.68e-09 |
| 3.16e-09 | 434+4 | 1.74e-09 | 2710+2 | 6.38e-10 |
| 1.00e-09 | 621+3 | 5.67e-10 | 4661+3 | 1.49e-10 |

VDPOL:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 141+92 | 2.49e-05 | 114+108 | 0.00184 |
| 3.16e-04 | 167+98 | 1.24e-04 | 130+65 | 7.68e-04 |
| 1.00e-04 | 205+132 | 1.02e-05 | 160+84 | 2.76e-04 |
| 3.16e-05 | 253+120 | 3.87e-06 | 197+110 | 3.75e-05 |
| 1.00e-05 | 324+108 | 3.46e-06 | 243+110 | 3.11e-05 |
| 3.16e-06 | 432+84 | 5.91e-08 | 379+102 | 7.37e-06 |
| 1.00e-06 | 575+51 | 3.09e-07 | 570+110 | 2.42e-06 |
| 3.16e-07 | 772+22 | 1.61e-07 | 858+53 | 7.30e-07 |
| 1.00e-07 | 1044+21 | 5.93e-08 | 1323+51 | 2.45e-07 |
| 3.16e-08 | 1415+22 | 1.92e-08 | 2070+28 | 8.54e-08 |
| 1.00e-08 | 1923+24 | 6.11e-09 | 3278+18 | 2.82e-08 |
| 3.16e-09 | 2624+24 | 1.96e-09 | 5189+19 | 8.22e-09 |
| 1.00e-09 | 3595+22 | 6.01e-10 | 8225+15 | 2.79e-09 |

BURGERS:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 14+0 | 7.97e-06 | 12+0 | 3.33e-04 |
| 3.16e-04 | 16+0 | 4.73e-06 | 13+0 | 1.12e-04 |
| 1.00e-04 | 18+0 | 2.32e-06 | 14+0 | 1.58e-04 |
| 3.16e-05 | 20+0 | 1.78e-06 | 16+0 | 2.48e-05 |
| 1.00e-05 | 24+0 | 4.00e-07 | 18+0 | 1.88e-05 |
| 3.16e-06 | 29+0 | 3.23e-07 | 21+0 | 4.95e-06 |
| 1.00e-06 | 35+0 | 1.17e-07 | 25+0 | 1.75e-06 |
| 3.16e-07 | 42+0 | 3.35e-08 | 30+0 | 8.92e-07 |
| 1.00e-07 | 52+0 | 1.16e-08 | 36+0 | 2.43e-07 |
| 3.16e-08 | 65+0 | 3.62e-09 | 44+0 | 9.41e-08 |
| 1.00e-08 | 81+0 | 1.25e-09 | 55+0 | 4.12e-08 |
| 3.16e-09 | 101+0 | 5.45e-10 | 68+0 | 1.17e-08 |
| 1.00e-09 | 127+0 | 1.46e-10 | 85+0 | 4.18e-09 |

### 깊이 6 (10단, 레벨 1-2-2-2-2-1), 후보 `D6L_r04` (1순위 (판정에 사용)): 허용오차별 원자료

ROBER:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 44+1 | 5.66e-05 | 46+1 | 1.17e-04 |
| 3.16e-04 | 50+0 | 2.61e-05 | 53+1 | 3.77e-05 |
| 1.00e-04 | 58+0 | 1.24e-05 | 63+1 | 1.19e-05 |
| 3.16e-05 | 68+0 | 5.30e-06 | 76+1 | 4.02e-06 |
| 1.00e-05 | 81+1 | 2.31e-06 | 94+1 | 1.33e-06 |
| 3.16e-06 | 97+4 | 9.14e-07 | 117+2 | 4.18e-07 |
| 1.00e-06 | 116+5 | 3.48e-07 | 150+1 | 1.33e-07 |
| 3.16e-07 | 139+4 | 1.28e-07 | 206+2 | 4.35e-08 |
| 1.00e-07 | 170+4 | 4.76e-08 | 307+3 | 1.36e-08 |
| 3.16e-08 | 228+2 | 1.68e-08 | 522+3 | 4.23e-09 |
| 1.00e-08 | 311+3 | 5.43e-09 | 998+2 | 1.30e-09 |
| 3.16e-09 | 434+4 | 1.74e-09 | 2008+3 | 4.11e-10 |
| 1.00e-09 | 621+3 | 5.67e-10 | 4003+3 | 1.29e-10 |

VDPOL:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 141+92 | 2.49e-05 | 159+122 | 4.96e-04 |
| 3.16e-04 | 167+98 | 1.24e-04 | 200+114 | 8.22e-05 |
| 1.00e-04 | 205+132 | 1.02e-05 | 256+145 | 1.63e-05 |
| 3.16e-05 | 253+120 | 3.87e-06 | 335+162 | 5.53e-06 |
| 1.00e-05 | 324+108 | 3.46e-06 | 444+92 | 1.46e-06 |
| 3.16e-06 | 432+84 | 5.91e-08 | 616+37 | 4.09e-07 |
| 1.00e-06 | 575+51 | 3.09e-07 | 894+35 | 1.04e-07 |
| 3.16e-07 | 772+22 | 1.61e-07 | 1328+29 | 3.11e-08 |
| 1.00e-07 | 1044+21 | 5.93e-08 | 2024+27 | 9.10e-09 |
| 3.16e-08 | 1415+22 | 1.92e-08 | 3156+26 | 4.02e-09 |
| 1.00e-08 | 1923+24 | 6.11e-09 | 5025+25 | 1.97e-09 |
| 3.16e-09 | 2624+24 | 1.96e-09 | 8138+24 | 8.83e-10 |
| 1.00e-09 | 3595+22 | 6.01e-10 | 13359+25 | 6.74e-10 |

BURGERS:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 14+0 | 7.97e-06 | 14+0 | 9.30e-05 |
| 3.16e-04 | 16+0 | 4.73e-06 | 16+0 | 1.63e-05 |
| 1.00e-04 | 18+0 | 2.32e-06 | 18+0 | 2.28e-05 |
| 3.16e-05 | 20+0 | 1.78e-06 | 21+0 | 8.45e-06 |
| 1.00e-05 | 24+0 | 4.00e-07 | 25+0 | 1.84e-06 |
| 3.16e-06 | 29+0 | 3.23e-07 | 30+0 | 8.20e-07 |
| 1.00e-06 | 35+0 | 1.17e-07 | 37+0 | 1.82e-07 |
| 3.16e-07 | 42+0 | 3.35e-08 | 45+0 | 1.13e-07 |
| 1.00e-07 | 52+0 | 1.16e-08 | 56+0 | 2.32e-08 |
| 3.16e-08 | 65+0 | 3.62e-09 | 70+0 | 1.02e-08 |
| 1.00e-08 | 81+0 | 1.25e-09 | 87+0 | 2.94e-09 |
| 3.16e-09 | 101+0 | 5.45e-10 | 109+0 | 1.16e-09 |
| 1.00e-09 | 127+0 | 1.46e-10 | 136+0 | 2.95e-10 |

### 깊이 6 (10단, 레벨 1-2-2-2-2-1), 후보 `D6T_r07` (탐색적 (판정에 쓰이지 않음)): 허용오차별 원자료

ROBER:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 44+1 | 5.66e-05 | 40+0 | 5.39e-04 |
| 3.16e-04 | 50+0 | 2.61e-05 | 47+0 | 2.01e-04 |
| 1.00e-04 | 58+0 | 1.24e-05 | 55+0 | 6.14e-05 |
| 3.16e-05 | 68+0 | 5.30e-06 | 65+0 | 1.98e-05 |
| 1.00e-05 | 81+1 | 2.31e-06 | 79+1 | 6.67e-06 |
| 3.16e-06 | 97+4 | 9.14e-07 | 96+1 | 2.14e-06 |
| 1.00e-06 | 116+5 | 3.48e-07 | 119+1 | 6.88e-07 |
| 3.16e-07 | 139+4 | 1.28e-07 | 150+1 | 2.16e-07 |
| 1.00e-07 | 170+4 | 4.76e-08 | 201+1 | 7.02e-08 |
| 3.16e-08 | 228+2 | 1.68e-08 | 286+1 | 2.17e-08 |
| 1.00e-08 | 311+3 | 5.43e-09 | 433+6 | 6.82e-09 |
| 3.16e-09 | 434+4 | 1.74e-09 | 802+16 | 2.15e-09 |
| 1.00e-09 | 621+3 | 5.67e-10 | 1637+19 | 6.73e-10 |

VDPOL:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 141+92 | 2.49e-05 | 134+77 | 9.64e-05 |
| 3.16e-04 | 167+98 | 1.24e-04 | 164+94 | 7.37e-05 |
| 1.00e-04 | 205+132 | 1.02e-05 | 201+116 | 1.08e-05 |
| 3.16e-05 | 253+120 | 3.87e-06 | 246+143 | 4.77e-06 |
| 1.00e-05 | 324+108 | 3.46e-06 | 305+167 | 1.97e-06 |
| 3.16e-06 | 432+84 | 5.91e-08 | 389+202 | 6.85e-07 |
| 1.00e-06 | 575+51 | 3.09e-07 | 498+87 | 2.28e-07 |
| 3.16e-07 | 772+22 | 1.61e-07 | 694+50 | 6.30e-08 |
| 1.00e-07 | 1044+21 | 5.93e-08 | 1000+72 | 2.05e-08 |
| 3.16e-08 | 1415+22 | 1.92e-08 | 1507+78 | 5.00e-09 |
| 1.00e-08 | 1923+24 | 6.11e-09 | 2361+77 | 1.56e-09 |
| 3.16e-09 | 2624+24 | 1.96e-09 | 3790+117 | 2.41e-09 |
| 1.00e-09 | 3595+22 | 6.01e-10 | 6185+185 | 3.93e-10 |

BURGERS:

| rtol | RODAS5P 수락+거부 | RODAS5P 오차 | 후보 수락+거부 | 후보 오차 |
|---|---|---|---|---|
| 0.001 | 14+0 | 7.97e-06 | 13+0 | 3.79e-05 |
| 3.16e-04 | 16+0 | 4.73e-06 | 14+0 | 4.20e-05 |
| 1.00e-04 | 18+0 | 2.32e-06 | 16+0 | 4.63e-06 |
| 3.16e-05 | 20+0 | 1.78e-06 | 18+0 | 2.01e-06 |
| 1.00e-05 | 24+0 | 4.00e-07 | 21+0 | 7.37e-07 |
| 3.16e-06 | 29+0 | 3.23e-07 | 24+0 | 4.61e-07 |
| 1.00e-06 | 35+0 | 1.17e-07 | 29+0 | 8.39e-08 |
| 3.16e-07 | 42+0 | 3.35e-08 | 35+0 | 2.88e-08 |
| 1.00e-07 | 52+0 | 1.16e-08 | 43+0 | 1.29e-08 |
| 3.16e-08 | 65+0 | 3.62e-09 | 52+0 | 3.50e-09 |
| 1.00e-08 | 81+0 | 1.25e-09 | 64+0 | 1.37e-09 |
| 3.16e-09 | 101+0 | 5.45e-10 | 80+0 | 4.74e-10 |
| 1.00e-09 | 127+0 | 1.46e-10 | 99+0 | 1.61e-10 |

## 8. 탐색적 분석 (판정에 쓰이지 않음)

### 8.1 안정성 함수가 정하는 선형 오차

깊이 L의 강성 정확 타블로에서 N = beta - gamma I는 N^L = 0이므로 R(z) = P(z)/(1 - gamma z)^L이고 deg P <= L - 1이다. 차수 조건이 P를 gamma만의 함수로 고정한다. 따라서 선형 문제에서의 오차는 나머지 계수와 무관하게 (L, gamma)로 정해진다.

| 깊이 | gamma | 비고 | eps=0.001 | eps=1.00e-04 | eps=1.00e-05 | eps=1.00e-06 | eps=1.00e-07 | eps=1.00e-08 | eps=1.00e-09 | eps=1.00e-10 |
|---|---|---|---|---|---|---|---|---|---|---|
| 5 | 0.27805 | depth 5, pinned gamma | 2.23 | 2.56 | 3.13 | 2.68 | 2.5 | 2.51 | 2.57 | 2.66 |
| 6 | 0.19 | depth 6 | 1.12 | 1.65 | 2.14 | 1.87 | 1.77 | 1.79 | 1.84 | 1.92 |
| 6 | 0.22 | depth 6 | 1.93 | 2.25 | 2.77 | 2.37 | 2.22 | 2.23 | 2.29 | 2.37 |
| 6 | 0.25 | depth 6 | 2.17 | 2.49 | 3.05 | 2.61 | 2.43 | 2.44 | 2.5 | 2.59 |
| 6 | 0.28 | depth 6 | 2.23 | 2.56 | 3.13 | 2.68 | 2.5 | 2.51 | 2.57 | 2.66 |
| 6 | 0.3 | depth 6 | 2.19 | 2.51 | 3.07 | 2.62 | 2.45 | 2.45 | 2.51 | 2.6 |
| 6 | 0.32 | depth 6 | 2.07 | 2.36 | 2.86 | 2.42 | 2.25 | 2.24 | 2.29 | 2.37 |
| 6 | 0.33 | depth 6 | 1.97 | 2.21 | 2.64 | 2.2 | 2.01 | 1.98 | 2 | 2.04 |
| 6 | 0.33414 | depth 6, linear order 6 | 1.91 | 2.13 | 2.52 | 2.06 | 1.84 | 1.77 | 1.73 | 1.7 |

표의 수는 스칼라 선형 문제 y' = lambda y (lambda < 0)에서 스텝당 오차 |R(z) - exp(z)|가 eps를 처음 넘는 |z|의 비, 곧 RODAS5P 대비 필요한 스텝 수의 비다(모형). 깊이 이득 1.6(깊이 5), 1.33(깊이 6)과 비교한다.

| 방법 | gamma | z^6 결함 | z^7 결함 | R_hat(무한대) |
|---|---|---|---|---|
| D5F_r01 | 0.278054 | 5.30e-04 | 8.34e-04 | 0.2 |
| D6L_r04 | 0.334142 | 2.68e-40 | -3.41e-04 | 2.58e-40 |
| D6T_r07 | 0.19 | 7.65e-05 | 1.13e-04 | 7.11e-40 |
| RODAS5P | 0.211938 | -5.29e-17 | -1.02e-05 | -2.20e-41 |

RODAS5P는 극이 7개인 R로 선형 차수 6을 갖는다. 깊이 5는 선형 차수 5에 묶이고, 깊이 6은 gamma = 0.33414에서만 선형 차수 6이 되며 그때도 z^7 결함이 RODAS5P보다 훨씬 크다(위 표). 스칼라 선형 모형에서는 이 차이가 깊이 이득을 넘는다. 실제 문제에서는 비선형 오차 항과 섞이므로 7절의 비는 이 모형값보다 작다.

### 8.2 선도 오차 항

| 방법 | |E6| (차수 6 오차 계수 노름) | RODAS5P 대비 | |ehat5| (추정기 계수 노름) | RODAS5P 대비 | 지수-1 잔차 노름 (주) | 지수-1 잔차 노름 (임베디드) | PR 항 노름 q=2 / 3 / 4 | c_i 범위 | 코드형 max|A|, max|C| |
|---|---|---|---|---|---|---|---|---|---|
| D5F_r01 | 0.00474 | 0.412 | 0.00346 | 0.236 | 0.00962 | 0.00648 | 0.00109 / 2.29e-04 / 1.28e-04 | [-0.424, 1] | 11.79, 60.39 |
| D6L_r04 | 0.00281 | 0.244 | 0.0199 | 1.36 | 2.17e-06 | 1.08e-06 | 4.35e-04 / 1.40e-04 / 7.72e-05 | [-0.236, 1] | 4.411, 41.1 |
| D6T_r07 | 0.00158 | 0.137 | 0.00839 | 0.572 | 2.87e-06 | 3.30e-07 | 6.70e-05 / 2.74e-05 / 4.18e-05 | [-0.168, 1] | 71.17, 1555 |
| RODAS5P | 0.0115 | 1 | 0.0147 | 1 | 5.09e-14 | 1.05e-14 | 4.83e-16 / 4.52e-16 / 1.03e-04 | [0, 1] | 11.63, 165.4 |

- E6는 비강성 점근 영역의 주 방법 오차 계수, ehat5는 임베디드 방법의 차수 5 계수(오차 추정의 크기)다. RODAS5P의 지수-1 잔차와 PR q=2, 3 항은 설계상 0이다.
- c_i가 [0, 1]을 벗어나는 단은 스텝 구간 밖에서 우변을 평가한다. 선언한 목적함수는 이를 제어하지 않았다.

### 8.2b Prothero-Robinson 조건과 극의 수 (구조적 사실)

N = beta - gamma I, u_2 = beta^2 1 - c^2/2라 하면 강성 정확 레벨 타블로에서 항등식 `b^T N^(L-1) u_2 = gamma^2 * b^T N^(L-1) 1`이 성립한다(N^(L-1)은 레벨 1에서 레벨 L로 가는 경로만 남기고 (u_2)_1 = gamma^2이기 때문). Prothero-Robinson 국소 오차 인자 W_2(z) = sum_k z^(k+1) b^T beta^k u_2가 항등적으로 0이려면 좌변이 0이어야 하고, 그러면 mu_(L-1) = b^T N^(L-1) 1 = 0, 곧 R(z) = P(z)/(1 - gamma z)^(L-1)로 극이 하나 줄어든다. 유도는 `NOTES.txt` 5번 항목, 아래는 수치 확인이다.

| 방법 | 레벨 수 L | mu_(L-1) | b^T N^(L-1) u_2 | gamma^2 mu_(L-1) | 항등식 잔차 |
|---|---|---|---|---|---|
| D5F_r01 | 5 | 0.00229 | 1.77e-04 | 1.77e-04 | 3.26e-55 |
| D6L_r04 | 6 | -0.00119 | -1.33e-04 | -1.33e-04 | -3.26e-55 |
| D6T_r07 | 6 | 1.69e-04 | 6.11e-06 | 6.11e-06 | -1.02e-56 |
| RODAS5P | 8 | -6.11e-22 | -2.74e-23 | -2.74e-23 | -4.68e-57 |

- RODAS5P는 8단이지만 mu_7 = 0(설계)이라 극이 7개이고 W_2 = W_3 = 0이다(`logs/validation.jsonl`의 pr_terms). 같은 기제다.
- 두 프로세서, 차수 5, R(무한대)=0에서의 귀결: 깊이 5에서 W_2 = 0을 정확히 요구하면 극이 4개가 되어 불가능하다(이론 S9의 L = 4 불가). 깊이 6에서는 극이 5개가 되어 gamma가 깊이 5의 값(0.27805)으로 고정되고 선형 차수 5(z^6 결함 5.30e-04)에 묶인다. 주 방법과 임베디드 방법 모두에 정확한 W_2 = 0을 주려면 레벨이 7개 이상 필요하다(깊이 이득 8/7). 이 문단은 유도와 위 수치 확인에 근거하며 기계 증명은 아니다.
- 스모크 시험(선언 범위 밖 시드, `logs/smoke3_search_D6L_hardPR.jsonl`): gamma = 0.33414에서 PR 조건(q = 2, 3)을 경성으로 넣은 변형은 4개 시작점 중 0개만 실현 가능했다(경성 잔차 0.015, 0.0207, 0.065, 1.09). 위 항등식과 일치하며 이 변형은 선언 실행 전에 버렸다.

### 8.3 강성 Prothero-Robinson 문제의 고정 스텝 차수

| 방법 | lambda | 오차 (스텝 수 10..320) | 쌍별 차수 |
|---|---|---|---|
| D5F_r01 | -100.0 | 7.15e-05 1.09e-05 1.01e-06 6.38e-08 3.06e-09 1.22e-10 | 2.72 3.43 3.98 4.38 4.65 |
| D5F_r01 | -10000.0 | 2.98e-06 1.52e-06 7.57e-07 3.64e-07 1.65e-07 6.59e-08 | 0.966 1.01 1.06 1.14 1.32 |
| D5F_r01 | -1000000.0 | 3.01e-08 1.56e-08 7.92e-09 3.99e-09 2.00e-09 1.00e-09 | 0.95 0.977 0.989 0.996 1 |
| D6L_r04 | -100.0 | 6.55e-05 9.98e-06 8.81e-07 4.99e-08 2.06e-09 7.13e-11 | 2.71 3.5 4.14 4.6 4.85 |
| D6L_r04 | -10000.0 | 2.74e-06 1.41e-06 7.00e-07 3.35e-07 1.51e-07 6.03e-08 | 0.959 1.01 1.06 1.15 1.33 |
| D6L_r04 | -1000000.0 | 2.77e-08 1.44e-08 7.36e-09 3.71e-09 1.86e-09 9.31e-10 | 0.941 0.973 0.988 0.995 1 |
| D6T_r07 | -100.0 | 3.47e-05 2.98e-06 1.21e-07 2.53e-09 1.30e-11 1.37e-12 | 3.54 4.63 5.57 7.61 3.25 |
| D6T_r07 | -10000.0 | 4.59e-06 2.30e-06 1.08e-06 4.70e-07 1.80e-07 5.67e-08 | 0.997 1.09 1.2 1.38 1.67 |
| D6T_r07 | -1000000.0 | 4.79e-08 2.50e-08 1.28e-08 6.43e-09 3.22e-09 1.60e-09 | 0.936 0.972 0.988 0.998 1.01 |
| RODAS5P | -100.0 | 1.16e-08 1.18e-09 6.05e-11 2.14e-12 6.48e-14 5.77e-15 | 3.29 4.29 4.82 5.04 3.49 |
| RODAS5P | -10000.0 | 5.06e-10 6.09e-11 6.85e-12 6.86e-13 5.42e-14 2.22e-15 | 3.05 3.15 3.32 3.66 4.61 |
| RODAS5P | -1000000.0 | 5.35e-12 6.78e-13 8.30e-14 7.99e-15 1.33e-15 3.55e-15 | 2.98 3.03 3.38 2.58 -1.42 |

y' = lambda (y - phi(t)) + phi'(t), phi = sin t + 2, t in [0, 1]. 오차가 1e-14 근처면 반올림 수준이다.

### 8.4 두 프로세서 스케줄

- `D5F_r01`: 8단, 5라운드. 라운드별 (프로세서 1, 프로세서 2): (1, -), (2, 3), (4, 5), (6, 7), (8, -). 한 라운드의 두 단은 서로 독립이고 같은 LU를 쓴다. 체인 깊이 이득 8/5 = 1.6, 총 풀이 수는 RODAS5P의 1배.
- `D6L_r04`: 10단, 6라운드. 라운드별 (프로세서 1, 프로세서 2): (1, -), (2, 3), (4, 5), (6, 7), (8, 9), (10, -). 한 라운드의 두 단은 서로 독립이고 같은 LU를 쓴다. 체인 깊이 이득 8/6 = 1.33, 총 풀이 수는 RODAS5P의 1.25배.

### 8.5 스텝 전체 속도 모형

모형: `S = 1 / ((1 - f_chain + f_chain * L / 8) * attempt_ratio)`. f_chain은 `cost_anatomy/AMDAHL.json`의 영역별 체인 비중(59개 영역), 표시는 `REVIEW_OVERLAY.json`. 오버헤드 0, 완전한 2-프로세서 확장, 단당 비용 동일을 가정한 상한이며 측정이 아니다.

| 후보 | 역할 | 문제 | 시도 비 | 체인만의 상한 (8/L)/비 | 권위 영역: 최소 / 중앙 / 최대 | 1 초과 영역 수 (권위) | 전체 영역: 최소 / 중앙 / 최대 | 1 초과 영역 수 (전체) |
|---|---|---|---|---|---|---|---|---|
| D5F_r01 | 1순위 (판정에 사용) | ROBER | 3.368 | 0.475 | 0.297 / 0.316 / 0.463 | 0 / 16 | 0.297 / 0.352 / 0.475 | 0 / 59 |
| D5F_r01 | 1순위 (판정에 사용) | VDPOL | 2.423 | 0.66 | 0.413 / 0.44 / 0.644 | 0 / 16 | 0.413 / 0.489 / 0.66 | 0 / 59 |
| D5F_r01 | 1순위 (판정에 사용) | BURGERS | 1.29 | 1.24 | 0.776 / 0.826 / 1.21 | 5 / 16 | 0.776 / 0.918 / 1.24 | 25 / 59 |
| D6L_r04 | 1순위 (판정에 사용) | ROBER | 1.246 | 1.07 | 0.804 / 0.837 / 1.06 | 3 / 16 | 0.804 / 0.896 / 1.07 | 15 / 59 |
| D6L_r04 | 1순위 (판정에 사용) | VDPOL | 1.482 | 0.9 | 0.676 / 0.704 / 0.887 | 0 / 16 | 0.676 / 0.753 / 0.9 | 0 / 59 |
| D6L_r04 | 1순위 (판정에 사용) | BURGERS | 1.289 | 1.03 | 0.776 / 0.809 / 1.02 | 2 / 16 | 0.776 / 0.865 / 1.03 | 14 / 59 |
| D6T_r07 | 탐색적 (판정에 쓰이지 않음) | ROBER | 1.377 | 0.968 | 0.727 / 0.757 / 0.954 | 0 / 16 | 0.727 / 0.81 / 0.968 | 0 / 59 |
| D6T_r07 | 탐색적 (판정에 쓰이지 않음) | VDPOL | 1.011 | 1.32 | 0.99 / 1.03 / 1.3 | 13 / 16 | 0.99 / 1.1 / 1.32 | 54 / 59 |
| D6T_r07 | 탐색적 (판정에 쓰이지 않음) | BURGERS | 0.8487 | 1.57 | 1.18 / 1.23 / 1.55 | 16 / 16 | 1.18 / 1.31 / 1.57 | 59 / 59 |

1보다 작은 값은 두 프로세서를 써도 RODAS5P 순차 스텝보다 느리다는 뜻이다.

## 9. 문헌 확인 (1차 출처, 신규성 주장 없음)

방법: Owner's PDF copies in the main checkout (read-only, text extracted with pdftotext into the scratch directory) and web search with an automated page reader. Publisher pages of Elsevier, zbMATH and global-sci returned HTTP 403 for full records; what was not readable is tagged.

- B. A. Schmitt, R. Weiner, Parallel two-step W-methods with peer variables, SIAM J. Numer. Anal. 42(1), 265-282 (2004), doi:10.1137/S0036142902411057. 접근: read: owner's PDF copy, abstract and pages 265-268, 280-282. 분류: two-step peer W-methods (PPSW).
  - s peer stage values per step; the s linear stage equations are independent and may be solved in parallel, i.e. depth 1 per step on s processors
  - convergence of order s-1 is proved, conditions for order s are given, methods up to order 7 (up to eight stages) are identified by tests
  - one matrix decomposition of I - gamma h T can be used for all stages
  - sequential comparison with the code RODAS: 'RODAS is clearly superior for weak tolerances', attributed to its faster step-size increase; several methods are called competitive with RODAS
- R. Weiner, B. A. Schmitt, H. Podhaisky, Parallel 'Peer' two-step W-methods and their application to MOL-systems, Appl. Numer. Math. 48, 425-439 (2004), doi:10.1016/j.apnum.2003.10.005. 접근: read: owner's PDF copy, abstract and introduction. 분류: two-step peer W-methods.
  - order and stage order s-1 or s, optimal damping at infinity, robust under step-size changes
  - tests on a shared-memory computer with Krylov techniques for MOL systems
- B. A. Schmitt, R. Weiner, H. Podhaisky, Multi-implicit peer two-step W-methods for parallel time integration, BIT Numer. Math. 45, 197-217 (2005), doi:10.1007/s10543-005-2635-y. 접근: read: owner's PDF copy, abstract and introduction. 분류: two-step peer W-methods with a different system parameter per stage.
  - order s-1; optimal damping for stiff problems by different system parameters in different stages (s different matrices per step)
  - uniform stability for realistic step-size sequences, a superconvergence property, matrix-free Krylov tests on a shared-memory computer
- H. Podhaisky, Parallele Zweischritt-W-Methoden, Dissertation, Martin-Luther-Universitaet Halle-Wittenberg (2002), urn:nbn:de:gbv:3-000002903. 접근: read: owner's PDF copy, title page and table of contents only. 분류: parallel two-step W-methods (PTSW).
  - chapters on stiffly accurate methods with a nilpotent stability matrix at infinity and on optimised two-, three- and four-stage L(alpha)- and A(alpha)-stable parameter sets
- X.-N. Cao, S.-F. Li, D.-G. Liu, Modified parallel Rosenbrock methods for stiff differential equations, J. Comput. Math. 20(1), 23-34 (2002), doi:10.4208/jcm.v20.n1.p23. 접근: publisher abstract page through an automated reader (short quotes); full text not read. 분류: parallel Rosenbrock methods (MPROW), building on the PRMs of Chen and Liu (1995).
  - 'practically optimal 2-stage 3rd-order and 3-stage 4th-order MPROWs', 'all A-stable and have small error constants'
  - how the stages are distributed over processors is not stated in the abstract: UNVERIFIED
- L.-R. Chen, D.-G. Liu, Parallel Rosenbrock methods for solving stiff systems in real-time simulation, J. Comput. Math. (2000). 접근: search-result snippet only; volume, pages and orders UNVERIFIED. 분류: parallel Rosenbrock methods (PRM).
  - construction, convergence and stability of PRMs for real-time simulation on parallel computers
- D. A. Voss, Fourth-order parallel Rosenbrock formulae for stiff systems, Math. Comput. Modelling 40(11-12), 1193-1198 (2004), doi:10.1016/j.mcm.2005.01.013. 접근: bibliographic records only (MaRDI portal, OpenAlex); abstract and text not reachable (HTTP 403). 분류: parallel Rosenbrock formulae.
  - order 4 (from the title); keywords: stability, parallel computation, Robertson problem, variable step size; number of stages and processors UNVERIFIED
- D. A. Voss, A. Q. M. Khaliq, Parallel Rosenbrock methods for chemical systems, Computers and Chemistry 25, 101-107 (2001). 접근: search-result snippet of the abstract only; authors, venue and pages are from memory: UNVERIFIED. 분류: parallel Rosenbrock methods.
  - 'Parallelism across the method allows for the solution of the linear algebraic systems in essentially Backward Euler-like solves on concurrent processors'; order UNVERIFIED
- K. Ponnammal, R. Ponalagusamy, A parallel fourth order Rosenbrock method: construction, analysis and numerical comparison (2014). 접근: search-result snippet only: UNVERIFIED. 분류: parallel Rosenbrock method.
  - four stages, order 4, intended for four processors
- V. T. Luan, A. Ostermann, Parallel exponential Rosenbrock methods (2016). 접근: search-result snippet only: UNVERIFIED. 분류: exponential Rosenbrock integrators (a different class: matrix functions instead of linear solves).
  - an order-4 parallel integrator at the cost of a two-stage method
- E. Hairer, G. Wanner, Solving Ordinary Differential Equations II, 2nd ed., Springer (1996), sections IV.7 and VI.4. 접근: cited from memory, the book was not consulted; the formulas used were re-derived and validated numerically on RODAS5P. 분류: order conditions of Rosenbrock methods; index-1 trees.
- G. Steinebach, Construction of Rosenbrock-Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package, BIT Numer. Math. 63, 27 (2023), doi:10.1007/s10543-023-00967-x. 접근: as quoted in theory/THEORY.md section S4 (short quotes through an automated reader); not re-read in this node. 분류: sequential ROW method of order 5(4), 8 stages.
  - order conditions of Tables 2-4 including index-1 and Prothero-Robinson conditions

- 읽기: In the sources reached, one-step Rosenbrock-type formulae with stage parallelism have order 3 or 4 with two to four stages; orders 5 to 7 with full stage parallelism are obtained by two-step peer W-methods, which give up the one-step structure and whose stages are all independent within a step. The repository already contains methods of that family (PPSW1, PTSW2b).
- 읽기: No stiffly accurate order-5(4) one-step ROW tableau with a level structure for two processors was found in this search. The search was short and several full texts were not reachable; this sentence describes the search, it is not a novelty claim.

## 10. RODAS5P와 비교해 얻는 것과 잃는 것

- 깊이 5 (8단, 레벨 1-2-2-2-1), `D5F_r01`.
  - 얻는 것: 단 풀이의 임계 경로가 8에서 5로 줄어든다(두 프로세서, 같은 LU). 차수 6 오차 계수 노름은 RODAS5P의 0.412배.
  - 잃는 것: 선형 오차(z^6 결함 5.30e-04, z^7 결함 8.34e-04; RODAS5P는 z^7에서 -1.02e-05), 그리고 같은 오차에서의 시도 횟수 비 ROBER 3.368, VDPOL 2.423, BURGERS 1.29.
  - 깊이 5에서는 임베디드 방법이 강성 정확일 수 없다(R_hat(무한대) = 0.2로 정규화). gamma가 고정이라 안정성 함수를 조정할 자유도가 없다.
- 깊이 6 (10단, 레벨 1-2-2-2-2-1), `D6L_r04`.
  - 얻는 것: 단 풀이의 임계 경로가 8에서 6로 줄어든다(두 프로세서, 같은 LU). 차수 6 오차 계수 노름은 RODAS5P의 0.244배.
  - 잃는 것: 선형 오차(z^6 결함 2.68e-40, z^7 결함 -3.41e-04; RODAS5P는 z^7에서 -1.02e-05), 그리고 같은 오차에서의 시도 횟수 비 ROBER 1.246, VDPOL 1.482, BURGERS 1.289.
  - 깊이 6은 단이 10개라 총 작업량이 1.25배다. 두 번째 프로세서가 없으면 그만큼 느리다.
- 탐색적 후보 `D6T_r07` (판정에 쓰이지 않음): 차수 6 오차 계수 노름은 RODAS5P의 0.137배, z^6 결함 7.65e-05, 시도 횟수 비 ROBER 1.377, VDPOL 1.011, BURGERS 0.8487.

## 11. 이런 방법의 자격 검증에 필요한 것

이 노드의 스크린은 Python 프로토타입의 스텝 수 비교다. 채택 여부를 다루려면 적어도 다음이 필요하다.

1. 조건 집합의 완결: 지수-1 조건(주, 임베디드)과 Prothero-Robinson 조건을 연성 목표가 아니라 정확히 만족시키고, RODAS5P가 가진 조밀 출력 계수(H 행렬)와 그 차수 조건을 새로 구성한다. 여기의 후보에는 조밀 출력이 없다.
2. 선형 오차와 Prothero-Robinson 조건의 해결 또는 수용: 8.1절과 8.2b절의 한계는 레이아웃의 성질이다. RODAS5P와 같은 극 수(7개)와 정확한 W_2 = 0을 함께 가지려면 레벨이 8개 필요하므로 깊이 이득이 없다. 레벨 7개(깊이 이득 8/7, 12단)로는 극 6개에 정확한 W_2 = 0, 또는 극 7개에 부정확한 W_2 중 하나만 가능하다.
3. Rust 구현과 비트 수준 결정성: 레벨 안의 두 단을 두 스레드에서 풀 때 결과가 스레드 수와 무관해야 하고, 단당 비용이 작은 문제에서는 동기화 비용이 이득을 지운다(V04의 체인 비중과 단당 시간 참고).
4. 정합 달성 오차에서의 벽시계 측정: 고정 코어, 교차 배치, 2회 예열 + 7회 반복, 부하 기록. 이 보고서의 속도 수치는 모형이다.
5. 새 문제 집합에서의 확인(이미 소비된 HIRES, Oregonator, Brusselator N=32는 holdout으로 쓸 수 없다), 외부 비교자(CVODE, Radau, Julia Rodas5P), 부정확한 야코비안과 지연 야코비안에서의 거동(W-차수), 스텝 제어기와의 상호작용(추정기 보정).
6. 보호 경로와의 분리: 새 타블로는 새 방법 id를 갖고 기본 라우팅에 들어가지 않는다. G0와 허용오차, 제어기 상수는 그대로다.

## 12. 한계와 미해결

- 사전선언을 3번 수정했다(2절). 수정은 최적화기 내부, 차수 검사의 반올림 가드, 깊이 6의 추가 정식화에 한정되지만, 스모크 실행을 본 뒤의 수정이다.
- 탐색은 선언한 목적함수(가중치 고정)와 예산 안의 결과다. 다른 가중치, c_i 범위 제약, 더 긴 연마, 다른 임베디드 설계는 시험하지 않았다. 후보의 연성 조건(지수-1, PR)은 정확히 0이 아니다(8.2절).
- 계수 한계 100은 k-형 계수(gamma, alpha, gamma_ij, b, bhat)에 적용했다. 코드형 A, C의 크기는 8.2절에 참고로 적었다(보호된 RODAS5P의 max|C|도 100을 넘는다).
- pay-off 문제 3개와 개발 문제 2개는 작은 계다. 프로토타입은 조밀 LU를 쓰고, 출력 시점 처리와 예측형 제어기 종류는 재현하지 않았다.
- 기준해의 독립성: Radau(scipy)와 프로토타입은 다른 코드이지만 u_ref의 한 성분은 RODAS5P 프로토타입 자신과의 차이다.
- 문헌 확인은 짧았고 Voss, Chen-Liu의 본문은 읽지 못했다.
- 깊이 5의 임베디드 방법 정규화(R_hat(무한대) = 0.2)와 c_s = 1, 깊이 6의 c_9 = 1은 동결 행에 없는 설계 선택이다. 대체 정식화(D5G, D6F)는 주 정식화가 후보를 내지 못할 때만 실행하기로 선언했고, 두 레이아웃 모두 주 정식화에서 후보가 나왔으므로 실행하지 않았다.
- 선택 규칙의 민감도: 개발 문제(HIRES, OREGO)의 순위와 pay-off 문제의 순위가 다르다. 깊이 6에서 개발 2위였던 후보가 pay-off 문제 두 개에서 1순위 후보보다 낫다(7절의 탐색적 행). 이 차이는 문제 의존성이 크다는 뜻이며, 그 후보를 사후에 1순위로 바꾸지 않았다. 확인하려면 새 문제에서 사전등록된 확증 실행이 필요하다.
- 탐색적 후보 `D6T_r07`은 선언한 규칙으로 BRUSS_ODE의 관측 차수가 4.8에 못 미치고(6절: 쌍별 차수가 불규칙하고, 가드 아래로 빠진 마지막 쌍은 표에 없다), 코드형 계수가 크다(8.2절). gamma = 0.19는 A-안정 창의 아래 끝(0.1839)에 가깝고 임베디드 방법은 허수축에서 |R_hat| > 1인 구간이 있다(5절 로그의 embedded_grid_max_abs_R_minus_1).
- 프로토타입의 제어기는 추정기 차수 5를 두 방법에 똑같이 쓴다. 후보의 추정기 크기(8.2절의 ehat5)는 RODAS5P와 다르므로 같은 허용오차에서의 달성 오차가 다르다. 비교는 달성 오차를 맞춰서 했다.

## 13. 재현

```
cd research/production_loop_20261116/level_tableau     # 모든 명령은 taskset -c 0-5,12-17, OMP_NUM_THREADS=1
python3 -I run_validation.py > logs/validation.jsonl
python3 -I search.py D6T --starts 2000 --workers 4 > logs/search_D6T.jsonl      # D5F, D6L도 같은 방식
python3 -I search.py D6T --polish-top logs/search_D6T.jsonl > logs/polish_D6T.jsonl
python3 -I search.py D6T --continuation logs/polish_D6T.jsonl > logs/continuation_D6T.jsonl
python3 -I refine.py D6T logs/polish_D6T.jsonl logs/continuation_D6T.jsonl > logs/refine_D6T.jsonl
python3 -I verify_candidate.py candidates/D6T_r0*.json > logs/verify_D6T.jsonl
python3 -I run_payoff.py --label dev --problems HIRES,OREGO --arms RODAS5P candidates/... > logs/payoff_dev_....jsonl
python3 -I run_order.py candidates/<1순위>.json > logs/order_<1순위>.jsonl
python3 -I run_payoff.py --label screen --problems ROBER,VDPOL,BURGERS --arms RODAS5P candidates/<1순위>.json > logs/payoff_screen_....jsonl
python3 -I run_exploratory.py RODAS5P candidates/<1순위>.json > logs/exploratory.jsonl
python3 -I parse_results.py && python3 -I render_report.py
```

입력 로그의 SHA-256은 `RESULTS.json`의 `inputs_sha256`에 있다.
