> **검토 반영 (2026-11-23, 독립 검토 `REVIEW.json`, 판정: 수정 조건부 수용).** 판정(유효성 PASS, cadence PASS 4/4, 정책 PASS 4/4, KILL 미발동)은 검토자가 raw log에서 다시 계산해 일치했다. 아래는 표현을 바로잡는 내용이다.
>
> 1. cadence 이득은 출하 설정(restart 40) 대비다. 보호 driver를 restart 10으로만 바꿔도 명령어가 restart 40의 0.26-0.37배가 된다(탐색). 같은 restart 10에서 열마다 residual을 검사하는 효과만 보면 명령어 0.64-0.78배다(탐색, 허용오차 1개).
> 2. 정책(C)의 시도 횟수 조건(1.10 이하)은 사전선언한 통계(기하평균)와 같은 허용오차끼리의 비교에서 성립한다. 최악 격자점에서는 cusp1d 1.44, bruss1d 1.23으로 1.10을 넘는다.
> 3. "모든 solve에서 residual check 통과"는 완료된 solve 기준이다. cusp1d에서 Krylov 예산을 다 쓴 solve가 A 149, B 149, C 33회 있었고 이들은 검사 대상 밖이다.
> 4. KILL은 "평가했고 발동하지 않음"이다. 측정 binary는 3개다. timing 부하 범위는 try 1이 14.5-41.7, try 2와 3이 1.9-3.9다.
> 5. cusp1d는 jump 구간의 오차가 불규칙해, matched-error 진술에서는 noise 한계가 있는 계열로 본다.
> 6. timing try 2와 3, parser, 보고서 생성, checker는 builder agent가 사용 한도로 끊긴 뒤 orchestrator가 실행했다(`NOTES.txt` 마지막 항목).

# V17 matrix-free stage solve: 잔차 검사 주기와 선형 허용치 정책 (실측 보고)

- node `V17_MATRIX_FREE_STAGE_SOLVE`, loop L1 wave 2 group 2, 생성 시각 2026-11-23T19:44:08+0900
- 사전등록: `PREDECLARED.json` (sha256 `9bb43f9b1d98c22efebca1d55516f3793ba0cf21e1ddb917362bd5f4f4c2c012`). 이 파일을 바꾼 commit (최신순): `2681f19a 2026-11-17T14:51:29+09:00 research(loop): V17 amendment A1 (final families and spans) before any run at a declared size`; `a6c66e9b 2026-11-17T14:21:29+09:00 research(loop): V17 pre-declaration (arms, policy formula, families, matched-error rule, timing rule)`. 선언 크기에서의 첫 실행은 마지막 수정 commit 뒤에 했다.
- 측정 시점의 commit: `2bd2dd65fdbac41d378243b123312f1606c4367f dirty_tracked=0` (140회), `98ea8a37904321e2f47736bcc72b3a435d8e9abd dirty_tracked=2` (72회), `df6dfa6fede330dfa66d3d22d7c0c3cc0dfb3acd dirty_tracked=1` (6회)
- 측정 binary sha256: `363a1a04e538bbeb...` (72회), `3fc4994c65c15fbe...` (140회), `533d04044ca00e39...` (6회)
- 이 문서의 수치는 전부 `make_report.py`가 `RESULTS.json`에서 읽어 채웠다. `RESULTS.json`은 `parse_results.py`가 `logs/`의 실행 출력과 `reference/*.json`만으로 만든다.
- 로그 수: sizing 15, reference 6, setup 24, runs 140, timing 75, exploratory 82, v06_regression 14

## 1. 결론

- **Validity (잔차 검사)**: 세 arm의 모든 stage solve를 원래 stage 계의 잔차로 다시 검사했다. A: 37,606건 중 실패 0건 (잔차/허용치 최대 0.997), B: 37,679건 중 실패 0건 (잔차/허용치 최대 1.000), C: 36,881건 중 실패 0건 (잔차/허용치 최대 1.000). 모든 solve 통과: 예.
- **Validity (attempt 수)**: B의 attempt 수가 모든 family, 모든 tolerance에서 A의 1/1.05~1.05배 안에 있는가: 예.
- **Cadence (B) 판정: PASS.** family별: `fhn3d` PASS, `cusp1d` PASS, `burgers2d` PASS, `bruss1d` PASS. instructions 조건(matched error에서 A의 0.67배 이하)은 4개 family가 충족, 0개 미달, 0개 판정 불가. wall 조건(1.3배 이상, authoritative 측정)은 4개 충족, 0개 미달, 0개 판정 불가.
- **Policy (C) 판정: PASS.** family별: `fhn3d` PASS, `cusp1d` PASS, `burgers2d` PASS, `bruss1d` PASS. instructions 조건(A의 0.5배 이하)은 4개 충족, 0개 미달; attempt 조건(A의 1.1배 이하)은 4개 충족, 0개 미달.
- **KILL 조항: 발동 안 됨.** B의 잔차 검사 실패 0건. 잔차 실패가 없고 authoritative wall이 1.10배 이상인 family가 있다.

family별 이득의 크기 (matched achieved error, 기하평균; 괄호는 비교 구간 안의 최소~최대):

| family | n | B instructions / A | B wall 배속 | B on-CPU 배속 (탐색) | B JVP / A | C instructions / A | C wall 배속 | C on-CPU 배속 (탐색) | C JVP / A | C attempts / A |
|---|---|---|---|---|---|---|---|---|---|---|
| `fhn3d` | 2000 | 0.359 (0.197~0.531) | 3.03x (1.95~5.97) [authoritative] | 3.03x (1.95~5.98) | 0.510 (0.343~0.644) | 0.129 (0.100~0.146) | 9.96x (8.51~13.64) [authoritative] | 9.97x (8.52~13.64) | 0.248 (0.206~0.275) | 1.009 (1.001~1.027) |
| `cusp1d` | 1200 | 0.382 (0.324~0.527) | 2.61x (1.85~3.08) [authoritative] | 2.61x (1.85~3.08) | 0.508 (0.460~0.640) | 0.118 (0.115~0.137) | 9.04x (8.11~9.40) [authoritative] | 9.04x (8.11~9.41) | 0.210 (0.177~0.275) | 1.005 (0.744~1.439) |
| `burgers2d` | 1568 | 0.216 (0.106~0.519) | 5.30x (1.97~12.08) [authoritative] | 5.32x (1.97~12.13) | 0.382 (0.238~0.649) | 0.100 (0.072~0.159) | 13.02x (7.44~19.42) [authoritative] | 12.93x (7.35~19.40) | 0.221 (0.173~0.298) | 1.000 (0.984~1.005) |
| `bruss1d` | 1400 | 0.430 (0.308~0.635) | 2.36x (1.55~3.41) [authoritative] | 2.36x (1.55~3.41) | 0.579 (0.476~0.738) | 0.146 (0.126~0.184) | 7.64x (5.78~9.29) [authoritative] | 7.65x (5.79~9.29) | 0.279 (0.266~0.295) | 1.003 (0.969~1.226) |

- 'on-CPU 배속'은 timing 실행에서 solver thread가 실제로 CPU 위에 있던 시간(`/proc/thread-self/schedstat`)의 비다. run queue에서 기다린 시간이 빠지므로 host 부하의 영향을 wall보다 덜 받는다. 탐색 수치이고 frozen 기준의 wall 조건을 대신하지 않는다.

## 2. 무엇을 쟀나

보호된 규칙: scaled 계 `(I - h gamma J) U = rhs`에서 true residual(새 JVP 한 번으로 계산)의 2-norm이 `max(gamma*atol_lin, rtol_lin*||rhs||_2)` 이하일 때만 해를 받는다. `rtol_lin = 1e-10`, `atol_lin = 1e-14`, restart 40, stage solve당 Arnoldi 벡터 200개 한도. 보호 kernel은 이 검사를 40열 pass가 끝날 때만 한다.

- `crates/rodas5p-integrators/src/routing.rs:233-251 (RoutingOptions::default().matrix_free: GMRES, rtol 1e-10, atol 1e-14, no preconditioner, zero start, maxiter 200)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2785 (ShiftedOperator I - h gamma J of the scaled U form)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2787-2791 (linear_atol = raw_absolute_residual_budget(config.atol, gamma) = gamma * atol, rounded down)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2865-2881 (stage right-hand side rhs = h gamma f_i + gamma sum_j C_ij U_j)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2924-2929 (GmresConfig: restart, max_arnoldi = max(maxiter, restart), rtol, atol = linear_atol)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:3016-3031 (solve_gmres_into_with_accounting, residual_scale = None: the 2-norm)`
- `crates/rodas5p-krylov/src/gmres_into.rs:268-269 (threshold from the right-hand side norm)`
- `crates/rodas5p-krylov/src/common.rs:39-47 (residual_threshold = max(atol, rtol * ||rhs||))`
- `crates/rodas5p-krylov/src/gmres_into.rs:276-312 (true residual at the top of every restart pass; the loop leaves only when it meets the threshold; budget exhaustion is CoreError::LinearSolve)`
- `crates/rodas5p-krylov/src/gmres_into.rs:455-476 (a restart pass runs all its columns; only a happy breakdown ends it early)`
- `crates/rodas5p-krylov/src/gmres_into.rs:372-390 (final diagnostic true residual under the default accounting)`

| arm | 내용 |
|---|---|
| A | 보호 driver 그대로 (`integrate_rodas5p_mf_fast_observed_gmres_into`, `RoutingOptions::default().matrix_free`). |
| B | per-stage GMRES client (`KrylovStageClient::per_stage`, `PerStageGmresOptions::baseline()`): 같은 임계값을 매 Arnoldi 열마다 projected residual로 보고, 닿으면 true residual(JVP 1회, arm에 계상)로 확인한 뒤에만 받는다. |
| C | B와 같되 임계값이 `max(tau_B, kappa*sqrt(n)*min_i(atol + rtol*|y_i|)/(h*gamma))`, `kappa = 1e-3`. `rtol`, `atol`은 step 허용치, `y`는 그 attempt를 선형화한 상태. 실행 전에 `PREDECLARED.json`에 적었고 바꾸지 않았다. |
| B0 | 대조용(탐색): client driver + per-stage GMRES에 A와 같은 검사 주기. 판정에 쓰지 않는다. |

- B의 임계값이 보호 규칙과 같다는 증명: 계약 테스트 `the_client_threshold_is_the_protected_threshold`(두 식이 무작위 2,000개 계에서 8 ulp 이내), `arm_b_and_the_protected_kernel_stop_at_the_same_residual`(첫 열 잔차를 임계값의 0.99배와 1.01배로 맞춘 2x2 계에서 보호 kernel과 B가 같은 열 수에서 멈춘다; 상대항과 절대항 둘 다), `the_routed_configuration_has_the_constants_of_arm_b`.
- 잔차 검사: B, C는 `ResidualAuditClient`가 solve 전의 b를 저장했다가 자기 JVP로 `b - W x`를 다시 계산한다(C는 감사가 스스로 계산한 `min_i(atol + rtol*|y_i|)`로 C의 임계값을 쓴다). A는 보호 driver가 GMRES에 넘기는 벡터를 내놓지 않으므로, attempt observer에서 stage 벡터 `U_i`를 읽어 stage 방정식으로 우변을 다시 만들고(`rhs_i = h*gamma*f(Y_i) + gamma*sum_j C_ij U_j`, 자체 RHS 평가) 자체 JVP로 잔차를 계산한다. 검사용 JVP와 RHS는 별도 counter에 있고 arm의 일에 넣지 않았다. 감사를 켠 실행과 끈 실행이 counter와 최종 상태의 모든 비트에서 같은지 parser가 확인한다.
- matched error: 달성 오차 `E = max_i |y_i(t1) - ref_i| / (1 + |ref_i|)`. arm별 (E, 일) 점을 tolerance 순서로 log-log 선분으로 잇고, 두 arm의 오차 구간이 겹치는 범위(기준해 불확도의 10배 이상)에서 201점 격자의 일 비율을 기하평균한 값으로 family를 판정한다. 곡선이 단조가 아니면 A는 작은 값, 후보는 큰 값을 쓴다.
- instructions: `perf stat -e instructions:u`로 프로세스 전체를 재고 같은 family의 SETUP 실행(문제와 시작 상태만 만드는 실행) 3회의 중앙값을 뺐다. wall: `flock ... taskset -c 6-11` 아래 한 프로세스에서 A, B, C를 round마다 순서를 돌려 가며 9 round(앞 2개는 warm-up).

| family | n | 구간 | 구분 | 강한 stiff | 문제 | 기준해 불확도 u_ref | Rust/numpy RHS 상대차 |
|---|---|---|---|---|---|---|---|
| `fhn3d` | 2000 | [0, 40] | 새 family |  | 3D FitzHugh-Nagumo system with a heterogeneous drive on the unit cube, zero-flux boundaries | 6.42e-12 | 6.17e-15 |
| `cusp1d` | 1200 | [0, 0.7] | 새 family | 예 | CUSP after Hairer and Wanner (Solving ODEs II, section IV.10) with N cells on a periodic ring | 1.19e-10 | 0.00e+00 |
| `burgers2d` | 1568 | [0, 0.5] | 새 family |  | 2D viscous Burgers system on the periodic unit square | 8.24e-12 | 1.29e-14 |
| `bruss1d` | 1400 | [0, 10] | V06 family, 새 크기 |  | 1D Brusselator, method of lines, as V06 (generator bruss1d of tests/stage_clients_krylov_problems.rs) | 2.39e-11 | 0.00e+00 |

- 기준해: 각 family를 numpy로 다시 구현해 scipy DOP853(rtol = atol = 1e-13)으로 적분했다. 불확도는 DOP853(1e-11) 및 Radau(1e-11, 유한차분 sparse Jacobian) 결과와의 `|차| / (1 + |ref|)` 최댓값 중 큰 쪽이다. Rust 코드와 선형 solver를 공유하지 않는다.

| family | 대조 적분 | 기준해와의 차 (scaled max) | 대조 RHS 평가 수 | 기준해 RHS 평가 수 |
|---|---|---|---|---|
| `fhn3d` | DOP853 1e-11 | 6.31e-12 | 10,373 | 12,317 |
| `fhn3d` | Radau 1e-11 | 6.42e-12 | 17,757 | 12,317 |
| `cusp1d` | DOP853 1e-11 | 1.19e-10 | 92,657 | 152,669 |
| `cusp1d` | Radau 1e-11 | 3.61e-12 | 83,742 | 152,669 |
| `burgers2d` | DOP853 1e-11 | 8.24e-12 | 965 | 1,697 |
| `burgers2d` | Radau 1e-11 | 9.20e-14 | 11,575 | 1,697 |
| `bruss1d` | DOP853 1e-11 | 2.39e-11 | 14,057 | 18,689 |
| `bruss1d` | Radau 1e-11 | 2.86e-13 | 17,550 | 18,689 |

- step tolerance: rtol = 1e-3, 1e-4, 1e-5, 1e-6, 1e-7, atol = rtol. 초기 step 1e-3(`AdaptiveStepConfig::default()`), 나머지 controller 설정은 기본값.

## 3. frozen 판별식

### 3.1 validity

| family | arm | 검사한 solve | 실패 | 잔차/허용치 최대 | 모든 solve 검사 + 감사 on/off 동일 |
|---|---|---|---|---|---|
| `fhn3d` | A | 3,832 | 0 | 0.948 | 예 |
| `fhn3d` | B | 3,832 | 0 | 1.000 | 예 |
| `fhn3d` | C | 3,848 | 0 | 1.000 | 예 |
| `cusp1d` | A | 28,662 | 0 | 0.997 | 예 |
| `cusp1d` | B | 28,735 | 0 | 1.000 | 예 |
| `cusp1d` | C | 27,897 | 0 | 1.000 | 예 |
| `burgers2d` | A | 1,528 | 0 | 0.829 | 예 |
| `burgers2d` | B | 1,528 | 0 | 1.000 | 예 |
| `burgers2d` | C | 1,528 | 0 | 0.999 | 예 |
| `bruss1d` | A | 3,584 | 0 | 0.994 | 예 |
| `bruss1d` | B | 3,584 | 0 | 1.000 | 예 |
| `bruss1d` | C | 3,608 | 0 | 1.000 | 예 |

attempts(B) / attempts(A), 같은 tolerance:

| family | rtol 1e-3 | rtol 1e-4 | rtol 1e-5 | rtol 1e-6 | rtol 1e-7 | 1.05 이내 |
|---|---|---|---|---|---|---|
| `fhn3d` | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 예 |
| `cusp1d` | 0.976 | 1.026 | 1.006 | 1.000 | 1.000 | 예 |
| `burgers2d` | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 예 |
| `bruss1d` | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 예 |

attempts(C) / attempts(A), 같은 tolerance:

| family | rtol 1e-3 | rtol 1e-4 | rtol 1e-5 | rtol 1e-6 | rtol 1e-7 | 1.10 이하 |
|---|---|---|---|---|---|---|
| `fhn3d` | 1.000 | 1.000 | 1.000 | 1.017 | 1.000 | 예 |
| `cusp1d` | 0.794 | 0.928 | 0.907 | 0.986 | 0.999 | 예 |
| `burgers2d` | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 예 |
| `bruss1d` | 1.023 | 1.000 | 1.000 | 1.019 | 1.000 | 예 |

### 3.2 Cadence (B)

| family | validity | instructions B/A (기하평균) | <= 0.67 | 최악 격자점 기준 | wall 배속 (기하평균) | wall 표시 | >= 1.3x | 비교한 오차 구간 | family |
|---|---|---|---|---|---|---|---|---|---|
| `fhn3d` | 예 | 0.359 | PASS | PASS | 3.03x | authoritative | PASS | 4.8e-07~7.8e-04 | PASS |
| `cusp1d` | 예 | 0.382 | PASS | PASS | 2.61x | authoritative | PASS | 6.7e-07~1.1e-02 | PASS |
| `burgers2d` | 예 | 0.216 | PASS | PASS | 5.30x | authoritative | PASS | 5.1e-08~4.7e-04 | PASS |
| `bruss1d` | 예 | 0.430 | PASS | PASS | 2.36x | authoritative | PASS | 2.9e-07~3.8e-05 | PASS |

- 판정 규칙(`PREDECLARED.json`): 3개 이상 family가 PASS면 PASS, 판정된 부분에서 2개 이상이 FAIL이면 FAIL, 그 밖은 NOT_DECIDABLE. 결과: PASS 4, FAIL 0, 판정 불가 0 -> **PASS**.
- instructions 조건은 기하평균으로 읽어도 최악 격자점으로 읽어도 같은 판정이다.

### 3.3 Policy (C)

| family | validity | instructions C/A (기하평균) | <= 0.5 | 최악 격자점 기준 | attempts C/A (matched error, 기하평균) | attempts C/A (matched error, 격자 최대) | attempts C/A (같은 tolerance, 최대) | <= 1.1 | attempts: 최악 격자점 기준 | family |
|---|---|---|---|---|---|---|---|---|---|---|
| `fhn3d` | 예 | 0.129 | PASS | PASS | 1.009 | 1.027 | 1.017 | PASS | PASS | PASS |
| `cusp1d` | 예 | 0.118 | PASS | PASS | 1.005 | 1.439 | 0.999 | PASS | FAIL | PASS |
| `burgers2d` | 예 | 0.100 | PASS | PASS | 1.000 | 1.005 | 1.000 | PASS | PASS | PASS |
| `bruss1d` | 예 | 0.146 | PASS | PASS | 1.003 | 1.226 | 1.023 | PASS | FAIL | PASS |

- 결과: PASS 4, FAIL 0, 판정 불가 0 -> **PASS**.
- 사전등록한 판정 통계는 비교 구간의 기하평균이다. 최악 격자점으로 읽으면 판정이 달라지는 family: `cusp1d`, `bruss1d`. 이 family들에는 tolerance를 조여도 달성 오차가 거의 줄지 않거나 오히려 커지는 구간이 있다(4절의 달성 오차 열). 그런 구간에서는 작은 오차 차이가 큰 일 차이로 환산되고, 곡선이 단조가 아닌 곳에서는 사전등록 규칙이 후보 arm에 큰 쪽 일을, A에 작은 쪽 일을 배정한다. 그래서 일부 격자점에서 matched-error attempt 비가 커진다. 같은 tolerance에서의 attempt 비는 표의 '같은 tolerance, 최대' 열이다.

### 3.4 KILL

- B의 잔차 검사 실패: 0건.
- B의 wall 배속(matched error): `fhn3d` 3.03x [authoritative], `cusp1d` 2.61x [authoritative], `burgers2d` 5.30x [authoritative], `bruss1d` 2.36x [authoritative].
- 판정: **발동 안 됨** (no residual-check failure in B and at least one authoritative family at or above 1.10x).

## 4. cell별 자료

### `fhn3d` (n = 2000)

| rtol | attempts A / B / C | 기각 A / B / C | JVP A / B / C | instructions (net) A / B / C | instr B/A | instr C/A | 달성 오차 E A / B / C | E / rtol A / B / C |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 50 / 50 / 50 | 16 / 16 / 16 | 19,793 / 12,751 / 5,010 | 1.71e+10 / 9.07e+09 / 2.42e+09 | 0.531 | 0.141 | 7.77e-04 / 7.77e-04 / 7.84e-04 | 0.777 / 0.777 / 0.784 |
| 1e-4 | 63 / 63 / 63 | 16 / 16 / 16 | 21,701 / 13,501 / 5,945 | 1.87e+10 / 9.10e+09 / 2.73e+09 | 0.486 | 0.146 | 5.13e-05 / 5.13e-05 / 5.38e-05 | 0.513 / 0.513 / 0.538 |
| 1e-5 | 78 / 78 / 78 | 10 / 10 / 10 | 26,208 / 13,457 / 6,665 | 2.26e+10 / 8.07e+09 / 2.96e+09 | 0.357 | 0.131 | 1.78e-05 / 1.78e-05 / 1.83e-05 | 1.781 / 1.781 / 1.827 |
| 1e-6 | 116 / 116 / 118 | 13 / 13 / 15 | 38,976 / 16,472 / 9,162 | 3.36e+10 / 8.92e+09 / 3.94e+09 | 0.265 | 0.117 | 2.89e-06 / 2.89e-06 / 3.02e-06 | 2.886 / 2.886 / 3.023 |
| 1e-7 | 172 / 172 / 172 | 13 / 13 / 13 | 57,792 / 19,829 / 11,894 | 4.98e+10 / 9.83e+09 / 4.97e+09 | 0.197 | 0.100 | 4.79e-07 / 4.79e-07 / 4.83e-07 | 4.791 / 4.791 / 4.825 |

| rtol | wall 중앙값 s A / B / C | spread A / B / C | 배속 B (같은 tol) | 배속 C (같은 tol) | on-CPU 중앙값 s A / B / C (탐색) | on-CPU 배속 B / C (탐색) | 1분 load 전 / 후 | 측정 횟수 | 표시 | 규칙 미달 사유 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1e-3 | 1.5205 / 0.7809 / 0.1753 | 5.1% / 5.0% / 3.4% | 1.95x | 8.67x | 1.5199 / 0.7809 / 0.1750 | 1.95x / 8.69x | 2.600 / 2.820 | 2 (사용: 2회차) | authoritative |  |
| 1e-4 | 1.6473 / 0.7726 / 0.1932 | 0.8% / 1.3% / 2.5% | 2.13x | 8.53x | 1.6468 / 0.7719 / 0.1930 | 2.13x / 8.53x | 2.820 / 2.490 | 2 (사용: 2회차) | authoritative |  |
| 1e-5 | 1.9927 / 0.6535 / 0.2024 | 0.7% / 0.8% / 0.8% | 3.05x | 9.85x | 1.9927 / 0.6540 / 0.2020 | 3.05x / 9.87x | 2.490 / 2.070 | 2 (사용: 2회차) | authoritative |  |
| 1e-6 | 2.9829 / 0.6932 / 0.2632 | 0.8% / 1.1% / 1.1% | 4.30x | 11.33x | 2.9827 / 0.6930 / 0.2630 | 4.30x / 11.34x | 2.070 / 1.880 | 2 (사용: 2회차) | authoritative |  |
| 1e-7 | 4.4123 / 0.7387 / 0.3231 | 1.8% / 1.9% / 2.2% | 5.97x | 13.66x | 4.4116 / 0.7379 / 0.3230 | 5.98x / 13.66x | 1.880 / 2.430 | 2 (사용: 2회차) | authoritative |  |

### `cusp1d` (n = 1200)

| rtol | attempts A / B / C | 기각 A / B / C | JVP A / B / C | instructions (net) A / B / C | instr B/A | instr C/A | 달성 오차 E A / B / C | E / rtol A / B / C |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 412 / 402 / 327 | 176 / 172 / 131 | 194,017 / 124,414 / 36,033 | 8.77e+10 / 4.63e+10 / 1.09e+10 | 0.528 | 0.124 | 1.16e-02 / 1.15e-02 / 8.25e-03 | 11.6 / 11.5 / 8.247 |
| 1e-4 | 544 / 558 / 505 | 217 / 224 / 198 | 232,683 / 129,497 / 47,379 | 1.05e+11 / 4.50e+10 / 1.36e+10 | 0.429 | 0.129 | 1.81e-03 / 1.44e-04 / 3.78e-04 | 18.1 / 1.441 / 3.779 |
| 1e-5 | 689 / 693 / 625 | 268 / 271 / 232 | 276,424 / 139,829 / 51,860 | 1.25e+11 / 4.65e+10 / 1.33e+10 | 0.373 | 0.106 | 1.55e-04 / 1.51e-04 / 1.24e-03 | 15.5 / 15.1 / 124 |
| 1e-6 | 853 / 853 / 841 | 306 / 306 / 301 | 336,478 / 154,696 / 68,055 | 1.52e+11 / 4.92e+10 / 1.64e+10 | 0.324 | 0.108 | 6.70e-07 / 6.71e-07 / 3.93e-06 | 0.670 / 0.671 / 3.928 |
| 1e-7 | 1,226 / 1,226 / 1,225 | 421 / 421 / 422 | 442,879 / 175,497 / 92,430 | 1.99e+11 / 5.06e+10 / 2.07e+10 | 0.254 | 0.104 | 9.47e-07 / 9.48e-07 / 7.00e-07 | 9.474 / 9.481 / 7.002 |

| rtol | wall 중앙값 s A / B / C | spread A / B / C | 배속 B (같은 tol) | 배속 C (같은 tol) | on-CPU 중앙값 s A / B / C (탐색) | on-CPU 배속 B / C (탐색) | 1분 load 전 / 후 | 측정 횟수 | 표시 | 규칙 미달 사유 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1e-3 | 8.5859 / 4.6453 / 1.0565 | 4.1% / 4.1% / 3.1% | 1.85x | 8.13x | 8.5850 / 4.6446 / 1.0557 | 1.85x / 8.13x | 3.290 / 3.700 | 2 (사용: 2회차) | authoritative |  |
| 1e-4 | 10.4 / 4.4601 / 1.2919 | 1.7% / 3.1% / 2.9% | 2.33x | 8.04x | 10.4 / 4.4606 / 1.2919 | 2.33x / 8.04x | 3.700 / 3.320 | 2 (사용: 2회차) | authoritative |  |
| 1e-5 | 12.3 / 4.5779 / 1.2230 | 1.3% / 1.1% / 1.6% | 2.69x | 10.07x | 12.3 / 4.5778 / 1.2211 | 2.69x / 10.08x | 3.320 / 3.540 | 2 (사용: 2회차) | authoritative |  |
| 1e-6 | 14.7 / 4.7682 / 1.4792 | 2.8% / 2.7% / 3.5% | 3.08x | 9.94x | 14.7 / 4.7674 / 1.4788 | 3.08x / 9.94x | 3.540 / 3.120 | 2 (사용: 2회차, round별 9회 호출) | authoritative |  |
| 1e-7 | 19.4 / 4.7546 / 1.8105 | 1.6% / 0.7% / 2.0% | 4.07x | 10.69x | 19.4 / 4.7539 / 1.8103 | 4.07x / 10.69x | 3.120 / 2.380 | 2 (사용: 2회차, round별 9회 호출) | authoritative |  |

### `burgers2d` (n = 1568)

| rtol | attempts A / B / C | 기각 A / B / C | JVP A / B / C | instructions (net) A / B / C | instr B/A | instr C/A | 달성 오차 E A / B / C | E / rtol A / B / C |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 13 / 13 / 13 | 0 / 0 / 0 | 4,368 / 2,834 / 1,324 | 2.81e+09 / 1.46e+09 / 4.52e+08 | 0.519 | 0.161 | 4.70e-04 / 4.70e-04 / 4.26e-04 | 0.470 / 0.470 / 0.426 |
| 1e-4 | 20 / 20 / 20 | 0 / 0 / 0 | 6,720 / 3,154 / 1,681 | 4.32e+09 / 1.27e+09 / 5.11e+08 | 0.295 | 0.118 | 3.22e-05 / 3.22e-05 / 3.23e-05 | 0.322 / 0.322 / 0.323 |
| 1e-5 | 31 / 31 / 31 | 0 / 0 / 0 | 10,416 / 3,717 / 2,177 | 6.69e+09 / 1.27e+09 / 6.11e+08 | 0.189 | 0.091 | 3.08e-06 / 3.08e-06 / 3.14e-06 | 0.308 / 0.308 / 0.314 |
| 1e-6 | 49 / 49 / 49 | 0 / 0 / 0 | 16,464 / 4,695 / 3,057 | 1.06e+10 / 1.44e+09 / 8.27e+08 | 0.136 | 0.078 | 3.79e-07 / 3.79e-07 / 3.85e-07 | 0.379 / 0.379 / 0.385 |
| 1e-7 | 78 / 78 / 78 | 0 / 0 / 0 | 26,208 / 6,240 / 4,512 | 1.68e+10 / 1.79e+09 / 1.20e+09 | 0.106 | 0.071 | 5.10e-08 / 5.10e-08 / 5.18e-08 | 0.510 / 0.510 / 0.518 |

| rtol | wall 중앙값 s A / B / C | spread A / B / C | 배속 B (같은 tol) | 배속 C (같은 tol) | on-CPU 중앙값 s A / B / C (탐색) | on-CPU 배속 B / C (탐색) | 1분 load 전 / 후 | 측정 횟수 | 표시 | 규칙 미달 사유 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1e-3 | 0.2677 / 0.1360 / 0.0365 | 4.6% / 4.2% / 2.5% | 1.97x | 7.34x | 0.2680 / 0.1360 / 0.0370 | 1.97x / 7.24x | 2.720 / 2.720 | 2 (사용: 2회차) | authoritative |  |
| 1e-4 | 0.3957 / 0.1071 / 0.0376 | 1.5% / 0.3% / 0.6% | 3.69x | 10.53x | 0.3950 / 0.1070 / 0.0380 | 3.69x / 10.39x | 2.430 / 2.320 | 3 (사용: 3회차) | authoritative |  |
| 1e-5 | 0.6347 / 0.1027 / 0.0436 | 2.7% / 7.3% / 6.1% | 6.18x | 14.55x | 0.6340 / 0.1020 / 0.0440 | 6.22x / 14.41x | 2.760 / 2.860 | 2 (사용: 2회차) | authoritative |  |
| 1e-6 | 1.0069 / 0.1125 / 0.0573 | 3.6% / 4.1% / 3.0% | 8.95x | 17.57x | 1.0069 / 0.1120 / 0.0570 | 8.99x / 17.66x | 2.860 / 3.430 | 2 (사용: 2회차) | authoritative |  |
| 1e-7 | 1.5766 / 0.1305 / 0.0809 | 3.8% / 2.9% / 3.5% | 12.08x | 19.49x | 1.5769 / 0.1300 / 0.0810 | 12.13x / 19.47x | 3.480 / 3.290 | 2 (사용: 2회차) | authoritative |  |

### `bruss1d` (n = 1400)

| rtol | attempts A / B / C | 기각 A / B / C | JVP A / B / C | instructions (net) A / B / C | instr B/A | instr C/A | 달성 오차 E A / B / C | E / rtol A / B / C |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 44 / 44 / 45 | 10 / 10 / 11 | 26,756 / 19,754 / 7,082 | 1.43e+10 / 9.08e+09 / 2.49e+09 | 0.635 | 0.174 | 3.76e-05 / 3.76e-05 / 4.80e-05 | 0.038 / 0.038 / 0.048 |
| 1e-4 | 61 / 61 / 61 | 15 / 15 / 15 | 31,361 / 21,957 / 8,517 | 1.67e+10 / 9.77e+09 / 2.73e+09 | 0.584 | 0.163 | 3.33e-05 / 3.33e-05 / 3.19e-05 | 0.333 / 0.333 / 0.319 |
| 1e-5 | 81 / 81 / 81 | 17 / 17 / 17 | 36,523 / 23,642 / 10,316 | 1.95e+10 / 1.01e+10 / 3.08e+09 | 0.521 | 0.158 | 1.08e-05 / 1.08e-05 / 1.05e-05 | 1.077 / 1.077 / 1.047 |
| 1e-6 | 106 / 106 / 108 | 11 / 11 / 13 | 40,003 / 22,698 / 11,525 | 2.13e+10 / 8.90e+09 / 3.10e+09 | 0.418 | 0.146 | 3.14e-06 / 3.14e-06 / 3.01e-06 | 3.136 / 3.136 / 3.012 |
| 1e-7 | 156 / 156 / 156 | 10 / 10 / 10 | 54,179 / 25,763 / 14,599 | 2.88e+10 / 8.86e+09 / 3.63e+09 | 0.308 | 0.126 | 2.93e-07 / 2.93e-07 / 2.90e-07 | 2.927 / 2.927 / 2.898 |

| rtol | wall 중앙값 s A / B / C | spread A / B / C | 배속 B (같은 tol) | 배속 C (같은 tol) | on-CPU 중앙값 s A / B / C (탐색) | on-CPU 배속 B / C (탐색) | 1분 load 전 / 후 | 측정 횟수 | 표시 | 규칙 미달 사유 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1e-3 | 1.3849 / 0.8940 / 0.2314 | 0.8% / 0.5% / 2.2% | 1.55x | 5.98x | 1.3849 / 0.8939 / 0.2310 | 1.55x / 6.00x | 3.850 / 3.460 | 2 (사용: 2회차) | authoritative |  |
| 1e-4 | 1.6047 / 0.9478 / 0.2450 | 1.8% / 3.5% / 2.6% | 1.69x | 6.55x | 1.6049 / 0.9480 / 0.2450 | 1.69x / 6.55x | 3.460 / 3.120 | 2 (사용: 2회차) | authoritative |  |
| 1e-5 | 1.8556 / 0.9674 / 0.2698 | 1.1% / 3.1% / 2.2% | 1.92x | 6.88x | 1.8549 / 0.9669 / 0.2700 | 1.92x / 6.87x | 3.120 / 2.610 | 2 (사용: 2회차) | authoritative |  |
| 1e-6 | 2.0229 / 0.8397 / 0.2643 | 1.1% / 4.5% / 2.9% | 2.41x | 7.66x | 2.0229 / 0.8399 / 0.2640 | 2.41x / 7.66x | 2.610 / 2.370 | 2 (사용: 2회차) | authoritative |  |
| 1e-7 | 2.8217 / 0.8264 / 0.3039 | 2.7% / 2.2% / 4.0% | 3.41x | 9.28x | 2.8218 / 0.8269 / 0.3040 | 3.41x / 9.28x | 2.370 / 2.720 | 2 (사용: 2회차) | authoritative |  |

## 5. 탐색 (어느 판정에도 넣지 않음)

### 5.1 이득은 어디서 오는가

family별로 JVP 1회와 RHS 1회의 instructions를 보정 실행(각 20,000회 호출, SETUP 차감)으로 재고, 각 실행의 net instructions를 JVP 몫, RHS 몫, 나머지(직교화, 벡터 연산, driver)로 나눴다. JVP 보정 loop에는 방향 벡터를 다시 정규화하는 O(n) 연산이 들어 있어 JVP 몫은 약간 과대평가다.

| family | instructions / JVP | instructions / RHS | SETUP instructions (중앙값) |
|---|---|---|---|
| `fhn3d` | 2.10e+05 | 2.13e+05 | 980,391 |
| `cusp1d` | 56,089 | 43,357 | 909,837 |
| `burgers2d` | 1.29e+05 | 95,101 | 1,058,816 |
| `bruss1d` | 73,249 | 49,103 | 927,822 |

| family | rtol | A: JVP / RHS / 나머지 | B: JVP / RHS / 나머지 | C: JVP / RHS / 나머지 | A->B 절감의 출처 (JVP / 나머지) | A->C 절감의 출처 (JVP / 나머지) | 직교화 내적 수 B/A | 직교화 내적 수 C/A | 나머지 / 직교화 원소 연산 A / B0 / B / C |
|---|---|---|---|---|---|---|---|---|---|
| `fhn3d` | 1e-3 | 24% / 0.5% / 75% | 29% / 0.9% / 70% | 43% / 3.4% / 53% | 18% / 82% | 21% / 79% | 0.508 | 0.090 | 4.142 / 3.917 / 4.007 / 4.577 |
| `fhn3d` | 1e-4 | 24% / 0.6% / 75% | 31% / 1.1% / 68% | 46% / 3.8% / 51% | 18% / 82% | 21% / 79% | 0.449 | 0.086 | 4.146 / 3.921 / 4.050 / 4.718 |
| `fhn3d` | 1e-5 | 24% / 0.6% / 75% | 35% / 1.6% / 63% | 47% / 4.4% / 48% | 18% / 82% | 21% / 79% | 0.300 | 0.072 | 4.147 / 3.921 / 4.159 / 4.878 |
| `fhn3d` | 1e-6 | 24% / 0.6% / 75% | 39% / 2.2% / 59% | 49% / 5.0% / 46% | 19% / 81% | 21% / 79% | 0.202 | 0.059 | 4.146 / 3.921 / 4.293 / 5.054 |
| `fhn3d` | 1e-7 | 24% / 0.6% / 75% | 42% / 3.0% / 55% | 50% / 5.8% / 44% | 20% / 80% | 21% / 79% | 0.133 | 0.046 | 4.146 / 3.921 / 4.471 / 5.286 |
| `cusp1d` | 1e-3 | 12% / 0.1% / 87% | 15% / 0.3% / 85% | 19% / 1.0% / 81% | 9% / 91% | 12% / 88% | 0.536 | 0.115 | 4.180 / 3.923 / 3.986 / 4.173 |
| `cusp1d` | 1e-4 | 12% / 0.2% / 87% | 16% / 0.4% / 83% | 20% / 1.2% / 79% | 10% / 90% | 11% / 89% | 0.425 | 0.115 | 4.183 / 3.925 / 4.029 / 4.242 |
| `cusp1d` | 1e-5 | 12% / 0.2% / 87% | 17% / 0.5% / 83% | 22% / 1.6% / 77% | 10% / 90% | 11% / 89% | 0.364 | 0.089 | 4.185 / 3.927 / 4.060 / 4.383 |
| `cusp1d` | 1e-6 | 12% / 0.2% / 87% | 18% / 0.6% / 82% | 23% / 1.7% / 75% | 10% / 90% | 11% / 89% | 0.310 | 0.087 | 4.187 / 3.928 / 4.095 / 4.466 |
| `cusp1d` | 1e-7 | 12% / 0.2% / 87% | 19% / 0.8% / 80% | 25% / 2.0% / 73% | 10% / 90% | 11% / 89% | 0.232 | 0.079 | 4.189 / 3.931 / 4.183 / 4.593 |
| `burgers2d` | 1e-3 | 20% / 0.4% / 79% | 25% / 0.7% / 74% | 38% / 2.2% / 60% | 15% / 85% | 17% / 83% | 0.501 | 0.110 | 4.169 / 3.929 / 4.034 / 4.600 |
| `burgers2d` | 1e-4 | 20% / 0.4% / 79% | 32% / 1.2% / 67% | 43% / 3.0% / 54% | 15% / 85% | 17% / 83% | 0.243 | 0.068 | 4.167 / 3.928 / 4.248 / 4.963 |
| `burgers2d` | 1e-5 | 20% / 0.4% / 79% | 38% / 1.9% / 60% | 46% / 3.9% / 50% | 16% / 84% | 18% / 82% | 0.132 | 0.044 | 4.167 / 3.927 / 4.524 / 5.405 |
| `burgers2d` | 1e-6 | 20% / 0.4% / 79% | 42% / 2.6% / 55% | 48% / 4.5% / 48% | 17% / 83% | 18% / 82% | 0.081 | 0.034 | 4.166 / 3.927 / 4.833 / 5.741 |
| `burgers2d` | 1e-7 | 20% / 0.4% / 79% | 45% / 3.3% / 52% | 49% / 4.9% / 46% | 17% / 83% | 18% / 82% | 0.056 | 0.029 | 4.165 / 3.926 / 5.153 / 5.957 |
| `bruss1d` | 1e-3 | 14% / 0.1% / 86% | 16% / 0.2% / 84% | 21% / 0.7% / 78% | 10% / 90% | 12% / 88% | 0.650 | 0.159 | 4.162 / 3.917 / 3.960 / 4.160 |
| `bruss1d` | 1e-4 | 14% / 0.1% / 86% | 16% / 0.2% / 83% | 23% / 0.9% / 76% | 10% / 90% | 12% / 88% | 0.592 | 0.142 | 4.165 / 3.920 / 3.979 / 4.247 |
| `bruss1d` | 1e-5 | 14% / 0.2% / 86% | 17% / 0.3% / 83% | 25% / 1.0% / 74% | 10% / 90% | 12% / 88% | 0.521 | 0.132 | 4.168 / 3.922 / 4.002 / 4.330 |
| `bruss1d` | 1e-6 | 14% / 0.2% / 86% | 19% / 0.5% / 81% | 27% / 1.3% / 71% | 10% / 90% | 11% / 89% | 0.404 | 0.113 | 4.172 / 3.926 / 4.062 / 4.494 |
| `bruss1d` | 1e-7 | 14% / 0.2% / 86% | 21% / 0.7% / 78% | 29% / 1.7% / 69% | 10% / 90% | 12% / 88% | 0.280 | 0.090 | 4.175 / 3.928 / 4.166 / 4.664 |

- 마지막 열은 '나머지' instructions를 직교화의 원소 연산 수((내적 수 + 벡터 갱신 수) x n)로 나눈 값이다. 이 값이 family와 tolerance에 걸쳐 거의 일정하면 '나머지'가 직교화 일에 비례한다는 뜻이다.
- '나머지'에는 Arnoldi 직교화(two-pass modified Gram-Schmidt), 벡터 갱신, driver의 일이 들어 있다. 열 j의 직교화 비용은 j에 비례하므로, pass를 일찍 끝내면 JVP 수보다 직교화 일이 더 크게 줄 수 있다. 표의 '직교화 내적 수' 비가 그 크기다.

### 5.2 대조 arm B0 (client driver에 보호 kernel의 검사 주기)

| family | rtol | attempts A / B0 | JVP A / B0 | attempt, JVP, Arnoldi 열 수 동일 | instructions B0/A | 오차 B0/A |
|---|---|---|---|---|---|---|
| `fhn3d` | 1e-3 | 50 / 50 | 19,793 / 19,793 | 예 | 0.959 | 1.000 |
| `fhn3d` | 1e-4 | 63 / 63 | 21,701 / 21,701 | 예 | 0.959 | 1.000 |
| `fhn3d` | 1e-5 | 78 / 78 | 26,208 / 26,208 | 예 | 0.959 | 1.000 |
| `fhn3d` | 1e-6 | 116 / 116 | 38,976 / 38,976 | 예 | 0.959 | 1.000 |
| `fhn3d` | 1e-7 | 172 / 172 | 57,792 / 57,792 | 예 | 0.959 | 1.000 |
| `cusp1d` | 1e-3 | 412 / 421 | 194,017 / 198,303 | 아니오 | 0.967 | 0.423 |
| `cusp1d` | 1e-4 | 544 / 556 | 232,683 / 236,307 | 아니오 | 0.961 | 0.064 |
| `cusp1d` | 1e-5 | 689 / 688 | 276,424 / 276,006 | 아니오 | 0.944 | 0.899 |
| `cusp1d` | 1e-6 | 853 / 853 | 336,478 / 336,478 | 예 | 0.946 | 1.000 |
| `cusp1d` | 1e-7 | 1,226 / 1,226 | 442,879 / 442,879 | 예 | 0.946 | 1.000 |
| `burgers2d` | 1e-3 | 13 / 13 | 4,368 / 4,368 | 예 | 0.954 | 1.000 |
| `burgers2d` | 1e-4 | 20 / 20 | 6,720 / 6,720 | 예 | 0.954 | 1.000 |
| `burgers2d` | 1e-5 | 31 / 31 | 10,416 / 10,416 | 예 | 0.954 | 1.000 |
| `burgers2d` | 1e-6 | 49 / 49 | 16,464 / 16,464 | 예 | 0.954 | 1.000 |
| `burgers2d` | 1e-7 | 78 / 78 | 26,208 / 26,208 | 예 | 0.954 | 1.000 |
| `bruss1d` | 1e-3 | 44 / 44 | 26,756 / 26,756 | 예 | 0.949 | 1.000 |
| `bruss1d` | 1e-4 | 61 / 61 | 31,361 / 31,361 | 예 | 0.949 | 1.000 |
| `bruss1d` | 1e-5 | 81 / 81 | 36,523 / 36,523 | 예 | 0.949 | 1.000 |
| `bruss1d` | 1e-6 | 106 / 106 | 40,003 / 40,003 | 예 | 0.949 | 1.000 |
| `bruss1d` | 1e-7 | 156 / 156 | 54,179 / 54,179 | 예 | 0.949 | 1.000 |

- B0는 검사 주기가 A와 같으므로, instructions B0/A는 두 driver와 두 GMRES 구현의 차이만 보여 준다. B/A에서 이 값을 뺀 나머지가 검사 주기의 효과다.

### 5.3 solve당 Krylov 차원

| family | rtol | A: 평균 / 최대 열 | A: pass 수 분포 (성공한 solve) | B: 평균 / 최대 열 | C: 평균 / 최대 열 | C: 정책 임계값이 쓰인 solve | C: 0 반복으로 받은 solve |
|---|---|---|---|---|---|---|---|
| `fhn3d` | 1e-3 | 47.3 / 80 | 1 pass: 327, 2 pass: 73 | 30.7 / 64 | 11.5 / 26 | 100% | 4 |
| `fhn3d` | 1e-4 | 41.0 / 80 | 1 pass: 491, 2 pass: 13 | 25.8 / 51 | 10.8 / 23 | 100% | 4 |
| `fhn3d` | 1e-5 | 40.0 / 40 | 1 pass: 624 | 20.6 / 38 | 9.686 / 20 | 100% | 3 |
| `fhn3d` | 1e-6 | 40.0 / 40 | 1 pass: 928 | 16.8 / 30 | 8.709 / 17 | 100% | 3 |
| `fhn3d` | 1e-7 | 40.0 / 40 | 1 pass: 1376 | 13.4 / 26 | 7.646 / 15 | 100% | 3 |
| `cusp1d` | 1e-3 | 61.0 / 200 | 1 pass: 1926, 2 pass: 542, 3 pass: 299, 4 pass: 60, 5 pass: 48 | 42.0 / 200 | 13.2 / 200 | 92% | 1 |
| `cusp1d` | 1e-4 | 53.7 / 200 | 1 pass: 3100, 2 pass: 483, 3 pass: 357, 4 pass: 14, 5 pass: 31 | 29.8 / 200 | 10.9 / 200 | 98% | 1 |
| `cusp1d` | 1e-5 | 49.1 / 200 | 1 pass: 4418, 2 pass: 533, 3 pass: 259, 4 pass: 17, 5 pass: 23 | 25.1 / 200 | 9.395 / 200 | 100% | 1 |
| `cusp1d` | 1e-6 | 47.3 / 200 | 1 pass: 5943, 2 pass: 617, 3 pass: 59, 4 pass: 52, 5 pass: 85 | 21.7 / 200 | 9.088 / 200 | 100% | 0 |
| `cusp1d` | 1e-7 | 43.1 / 200 | 1 pass: 9408, 2 pass: 201, 3 pass: 64, 4 pass: 69, 5 pass: 54 | 16.8 / 200 | 8.407 / 200 | 100% | 0 |
| `burgers2d` | 1e-3 | 40.0 / 40 | 1 pass: 104 | 26.2 / 41 | 11.8 / 21 | 100% | 2 |
| `burgers2d` | 1e-4 | 40.0 / 40 | 1 pass: 160 | 18.7 / 29 | 9.512 / 16 | 100% | 1 |
| `burgers2d` | 1e-5 | 40.0 / 40 | 1 pass: 248 | 14.0 / 20 | 7.782 / 12 | 100% | 1 |
| `burgers2d` | 1e-6 | 40.0 / 40 | 1 pass: 392 | 11.0 / 14 | 6.801 / 9 | 100% | 1 |
| `burgers2d` | 1e-7 | 40.0 / 40 | 1 pass: 624 | 9.000 / 12 | 6.236 / 8 | 100% | 3 |
| `bruss1d` | 1e-3 | 73.2 / 160 | 1 pass: 135, 2 pass: 153, 3 pass: 53, 4 pass: 11 | 54.3 / 149 | 18.6 / 57 | 100% | 3 |
| `bruss1d` | 1e-4 | 61.7 / 120 | 1 pass: 253, 2 pass: 205, 3 pass: 30 | 43.5 / 116 | 16.5 / 49 | 100% | 4 |
| `bruss1d` | 1e-5 | 54.0 / 120 | 1 pass: 426, 2 pass: 217, 3 pass: 5 | 35.1 / 87 | 14.9 / 39 | 100% | 2 |
| `bruss1d` | 1e-6 | 45.0 / 80 | 1 pass: 741, 2 pass: 107 | 25.6 / 68 | 12.3 / 37 | 100% | 1 |
| `bruss1d` | 1e-7 | 41.4 / 80 | 1 pass: 1205, 2 pass: 43 | 19.6 / 53 | 10.7 / 29 | 100% | 1 |

stage별 solve당 평균 열 수 (rtol 1e-5; stage 1~8 순):

| family | arm | stage 1 | stage 2 | stage 3 | stage 4 | stage 5 | stage 6 | stage 7 | stage 8 |
|---|---|---|---|---|---|---|---|---|---|
| `fhn3d` | B | 18.5 | 19.7 | 18.1 | 18.7 | 17.5 | 23.7 | 23.4 | 24.9 |
| `fhn3d` | C | 9.5 | 11.0 | 9.6 | 12.4 | 11.7 | 10.1 | 7.3 | 5.9 |
| `cusp1d` | B | 30.6 | 26.1 | 22.9 | 23.5 | 22.6 | 26.0 | 24.4 | 24.0 |
| `cusp1d` | C | 9.6 | 12.2 | 9.3 | 11.8 | 11.0 | 9.0 | 6.7 | 5.6 |
| `burgers2d` | B | 13.3 | 13.4 | 13.3 | 13.3 | 13.3 | 14.5 | 15.5 | 15.3 |
| `burgers2d` | C | 8.0 | 8.5 | 8.2 | 9.5 | 9.6 | 7.5 | 6.0 | 5.0 |
| `bruss1d` | B | 30.5 | 34.5 | 31.2 | 32.5 | 30.1 | 40.1 | 39.7 | 42.5 |
| `bruss1d` | C | 14.5 | 17.4 | 15.1 | 19.3 | 18.2 | 15.4 | 11.2 | 8.2 |

- B의 임계값은 우변 크기에 상대적이고 C의 정책 임계값은 step 허용치에 묶인 절대값이다. 우변이 작은 stage에서 두 규칙이 요구하는 열 수가 어떻게 갈리는지는 위 표의 stage별 값에서 볼 수 있다.

### 5.4 restart 길이 (rtol 1e-5)

| family | arm | restart 10 | restart 20 | restart 40 (선언 실행) | restart 80 |
|---|---|---|---|---|---|
| `fhn3d` | A | 78 att, 18,950 JVP, 8.30e+09 instr, 실패 0 | 78 att, 19,419 JVP, 1.12e+10 instr, 실패 0 | 78 att, 26,208 JVP, 2.26e+10 instr, 실패 0 | 78 att, 51,168 JVP, 7.40e+10 instr, 실패 0 |
| `fhn3d` | B | 78 att, 15,991 JVP, 6.36e+09 instr, 실패 0 | 78 att, 14,097 JVP, 7.19e+09 instr, 실패 0 | 78 att, 13,457 JVP, 8.07e+09 instr, 실패 0 | 78 att, 13,457 JVP, 8.07e+09 instr, 실패 0 |
| `cusp1d` | A | 764 att, 218,490 JVP, 4.19e+10 instr, 실패 70 | 742 att, 245,507 JVP, 6.82e+10 instr, 실패 65 | 689 att, 276,424 JVP, 1.25e+11 instr, 실패 34 | 672 att, 450,553 JVP, 3.62e+11 instr, 실패 30 |
| `cusp1d` | B | 752 att, 190,679 JVP, 3.18e+10 instr, 실패 71 | 747 att, 187,947 JVP, 4.46e+10 instr, 실패 65 | 693 att, 139,829 JVP, 4.65e+10 instr, 실패 34 | 685 att, 136,810 JVP, 6.46e+10 instr, 실패 30 |
| `burgers2d` | A | 31 att, 5,528 JVP, 1.70e+09 instr, 실패 0 | 31 att, 5,456 JVP, 2.28e+09 instr, 실패 0 | 31 att, 10,416 JVP, 6.69e+09 instr, 실패 0 | 31 att, 20,336 JVP, 2.24e+10 instr, 실패 0 |
| `burgers2d` | B | 31 att, 4,010 JVP, 1.08e+09 instr, 실패 0 | 31 att, 3,717 JVP, 1.27e+09 instr, 실패 0 | 31 att, 3,717 JVP, 1.27e+09 instr, 실패 0 | 31 att, 3,717 JVP, 1.27e+09 instr, 실패 0 |
| `bruss1d` | A | 81 att, 31,250 JVP, 7.21e+09 instr, 실패 0 | 81 att, 30,972 JVP, 1.02e+10 instr, 실패 0 | 81 att, 36,523 JVP, 1.95e+10 instr, 실패 0 | 81 att, 53,217 JVP, 5.02e+10 instr, 실패 0 |
| `bruss1d` | B | 81 att, 27,666 JVP, 5.59e+09 instr, 실패 0 | 81 att, 24,677 JVP, 7.15e+09 instr, 실패 0 | 81 att, 23,642 JVP, 1.01e+10 instr, 실패 0 | 81 att, 23,199 JVP, 1.24e+10 instr, 실패 0 |

같은 자료를 A(restart 40, 선언 실행) 대비 비로 본 것 (instructions 비 / 달성 오차 비):

| family | A r10 | A r20 | A r40 | A r80 | B r10 | B r20 | B r40 | B r80 |
|---|---|---|---|---|---|---|---|---|
| `fhn3d` | 0.367 / 1.000 | 0.497 / 1.000 | 1.000 / 1.000 | 3.274 / 1.000 | 0.281 / 1.000 | 0.318 / 1.000 | 0.357 / 1.000 | 0.357 / 1.000 |
| `cusp1d` | 0.336 / 0.974 | 0.547 / 0.888 | 1.000 / 1.000 | 2.903 / 1.321 | 0.255 / 1.136 | 0.357 / 0.886 | 0.373 / 0.976 | 0.518 / 0.983 |
| `burgers2d` | 0.255 / 1.000 | 0.341 / 1.000 | 1.000 / 1.000 | 3.353 / 1.000 | 0.162 / 1.000 | 0.189 / 1.000 | 0.189 / 1.000 | 0.189 / 1.000 |
| `bruss1d` | 0.370 / 1.000 | 0.527 / 1.000 | 1.000 / 1.000 | 2.581 / 1.000 | 0.288 / 1.000 | 0.368 / 1.000 | 0.521 / 1.000 | 0.640 / 1.000 |

- restart 길이는 Krylov 한도(200 벡터)를 그대로 두고 pass당 열 수만 바꾼 것이다. A는 pass가 끝날 때만 잔차를 보므로 pass가 짧을수록 헛도는 열이 줄고, 두 arm 모두 pass가 짧을수록 열당 직교화 일이 준다. 대신 restart가 짧으면 수렴이 느려져 Krylov 한도에 걸리는 solve가 늘 수 있다(표의 '실패' 수).
- 이 자료(rtol 1e-5 한 점)에서 보호 driver를 restart 10으로만 바꿔도 instructions가 선언 설정(restart 40)보다 적은 family: `fhn3d`, `cusp1d`, `burgers2d`, `bruss1d`. restart는 `LinearSolverConfig`의 설정값이라 보호 파일을 고치지 않고 호출자가 줄 수 있다. 다만 routed 기본값(40)을 바꾸는 것은 default routing 변경이므로 이 node의 범위 밖이고, 한 tolerance에서 본 탐색 결과일 뿐이다.

### 5.5 preconditioner 한 가지 (client의 right Jacobi, arm B, rtol 1e-5)

| family | attempts B / B+Jacobi | JVP B / B+Jacobi | instructions B / B+Jacobi | solve당 열 B / B+Jacobi | 오차 E B / B+Jacobi | Jacobi 실행의 잔차 검사 / 실패 |
|---|---|---|---|---|---|---|
| `fhn3d` | 78 / 78 | 13,457 / 18,788 | 8.07e+09 / 1.36e+10 | 20.6 / 29.1 | 1.78e-05 / 1.78e-05 | 624 / 0 |
| `cusp1d` | 693 / 619 | 139,829 / 56,049 | 4.65e+10 / 1.29e+10 | 25.1 / 10.3 | 1.51e-04 / 2.59e-04 | 4,944 / 0 |
| `burgers2d` | 31 / 31 | 3,717 / 3,731 | 1.27e+09 / 1.29e+09 | 14.0 / 14.0 | 3.08e-06 / 3.08e-06 | 248 / 0 |
| `bruss1d` | 81 / 81 | 23,642 / 25,458 | 1.01e+10 / 1.12e+10 | 35.1 / 37.9 | 1.08e-05 / 1.08e-05 | 648 / 0 |

- 보호 driver는 preconditioner를 받지 않으므로(`rodas5p_matrix_free_fast.rs:1355-1366`) A에는 대응 실행이 없다.

### 5.6 kappa 민감도 (arm C; 선언값은 1e-3, 판정에는 선언값만 썼다)

| family | rtol | kappa 1e-4: instr / 오차 / attempts (A 대비) | kappa 1e-3 (선언) | kappa 1e-2 |
|---|---|---|---|---|
| `fhn3d` | 1e-3 | 0.188 / 1.001 / 1.000 | 0.141 / 1.008 / 1.000 | 0.105 / 2.503 / 0.980 |
| `fhn3d` | 1e-4 | 0.187 / 1.005 / 1.000 | 0.146 / 1.047 / 1.000 | 0.117 / 1.183 / 1.048 |
| `fhn3d` | 1e-5 | 0.165 / 1.000 / 1.000 | 0.131 / 1.025 / 1.000 | 0.104 / 1.046 / 1.013 |
| `fhn3d` | 1e-6 | 0.143 / 1.000 / 1.000 | 0.117 / 1.048 / 1.017 | 0.094 / 1.050 / 1.009 |
| `fhn3d` | 1e-7 | 0.122 / 1.000 / 1.000 | 0.100 / 1.007 / 1.000 | 0.083 / 0.972 / 0.988 |
| `cusp1d` | 1e-3 | 0.173 / 0.967 / 0.799 | 0.124 / 0.711 / 0.794 | 0.094 / 0.617 / 0.740 |
| `cusp1d` | 1e-4 | 0.160 / 0.176 / 0.928 | 0.129 / 0.209 / 0.928 | 0.091 / 0.307 / 0.860 |
| `cusp1d` | 1e-5 | 0.158 / 3.559 / 0.935 | 0.106 / 7.995 / 0.907 | 0.078 / 1.362 / 0.894 |
| `cusp1d` | 1e-6 | 0.144 / 3.802 / 0.998 | 0.108 / 5.861 / 0.986 | 0.080 / 1.015 / 0.971 |
| `cusp1d` | 1e-7 | 0.133 / 0.729 / 0.994 | 0.104 / 0.739 / 0.999 | 0.082 / 0.742 / 1.005 |
| `burgers2d` | 1e-3 | 0.210 / 0.998 / 1.000 | 0.161 / 0.905 / 1.000 | 0.118 / 1.242 / 1.000 |
| `burgers2d` | 1e-4 | 0.148 / 0.998 / 1.000 | 0.118 / 1.005 / 1.000 | 0.095 / 1.413 / 1.050 |
| `burgers2d` | 1e-5 | 0.109 / 1.001 / 1.000 | 0.091 / 1.021 / 1.000 | 0.075 / 1.136 / 1.000 |
| `burgers2d` | 1e-6 | 0.092 / 1.001 / 1.000 | 0.078 / 1.017 / 1.000 | 0.065 / 1.267 / 1.000 |
| `burgers2d` | 1e-7 | 0.082 / 1.000 / 1.000 | 0.071 / 1.016 / 1.000 | 0.061 / 1.863 / 1.000 |
| `bruss1d` | 1e-3 | 0.234 / 1.017 / 1.000 | 0.174 / 1.276 / 1.023 | 0.121 / 2.704 / 1.045 |
| `bruss1d` | 1e-4 | 0.230 / 1.006 / 1.000 | 0.163 / 0.958 / 1.000 | 0.113 / 0.659 / 0.967 |
| `bruss1d` | 1e-5 | 0.225 / 0.999 / 1.000 | 0.158 / 0.972 / 1.000 | 0.113 / 0.710 / 1.000 |
| `bruss1d` | 1e-6 | 0.201 / 1.001 / 1.000 | 0.146 / 0.961 / 1.019 | 0.109 / 0.888 / 1.028 |
| `bruss1d` | 1e-7 | 0.173 / 1.000 / 1.000 | 0.126 / 0.990 / 1.000 | 0.095 / 0.898 / 1.000 |

### 5.7 Krylov 한도에 걸리는 경우 (표준 Brusselator, 길이 1, 600 cell, n = 1200, rtol 1e-4)

| arm | 상태 | attempts | 기각 | 선형 solve 실패 | JVP | instructions (net) | 오차 E | 잔차 검사 / 실패 |
|---|---|---|---|---|---|---|---|---|
| A | 완료 | 954 | 475 | 475 | 730,216 | 3.36e+11 | 1.16e-09 | 5,582 / 0 |
| B | 완료 | 954 | 475 | 475 | 619,413 | 2.59e+11 | 1.15e-09 | 5,576 / 0 |
| C | 완료 | 184 | 82 | 78 | 83,132 | 3.42e+10 | 1.88e-05 | 946 / 0 |

- 이 영역에서는 무전처리 GMRES(40)이 200벡터 한도 때문에 step을 줄인다. 선언 family가 아니고 timing 규약도 적용하지 않았다.
- tolerance가 하나뿐이라 matched error 비교가 아니다. 달성 오차는 A 1.16e-09, C 1.88e-05 (E/rtol: A 1.16e-05, C 0.188). A와 B는 선형 solve 실패로 step이 줄어 요청한 tolerance보다 훨씬 정확한 해를 내고 그만큼 일을 더 한다. C는 느슨한 임계값 덕에 한도 안에서 수렴하는 solve가 늘어 attempt 수 자체가 준다.
- 이 cell의 기준해: Radau rtol 1e-12 (`reference/bruss1d_stiff600_radau.json`, 불확도 2.87e-13). DOP853(1e-13)은 13분 안에 끝나지 않아 중단했고(`logs/reference/bruss1d_stiff600.log`), 사전등록의 대체 규칙(Radau 1e-12, 대조 Radau 1e-10)을 썼다.

## 6. 이득을 얻으려면 사용자가 켜야 하는 것

기본 경로는 그대로다. 보호 driver와 그 허용치는 수정하지 않았고, 아래는 전부 opt-in이다.

```rust
use rodas5p_integrators::stage_clients::krylov::{
    KrylovPreconditioner, KrylovStageClient, PerStageGmresOptions, StageStoppingRule, StepTiedTolerance,
};
use rodas5p_integrators::integrate_rodas5p_fast_client_observed_with_atol;

// arm B: 보호 규칙의 임계값, 매 Arnoldi 열마다 검사
let client = KrylovStageClient::per_stage(
    n, StageStoppingRule::protected_production()?, PerStageGmresOptions::baseline(), KrylovPreconditioner::None)?;
// arm C: 추가로 step 허용치에 묶인 임계값 (kappa = 1e-3)
let client = client.with_step_tied_tolerance(StepTiedTolerance::declared(config.rtol, config.atol)?);
let (run, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
    &problem, Box::new(client), t_span, &y0, &config, &output, &vec![config.atol; n])?;
```

- 문제는 JVP를 제공해야 한다(행렬 없음). preconditioner는 `KrylovPreconditioner::RightJacobi`(Jacobian 대각 callback)만 있다.
- `ResidualAuditClient::new(client, rule).with_step_tied_tolerance(policy)`로 감싸면 모든 solve의 원 잔차를 별도 JVP로 검사한다(실행 결과는 비트 단위로 같다).
- facade(`vigilode`)의 routing은 건드리지 않았다. facade에서 이 client를 고르는 길은 아직 없다.

## 7. default가 되기 전에 남은 것

- B는 보호 driver와 다른 driver(client driver)를 쓴다. 두 driver는 stage 우변을 scaled/unscaled 형태로 다르게 만들어 최종 상태가 반올림 수준에서 다르고(5.2절의 B0), 오류로 실패한 attempt 뒤에 RHS를 한 번 더 평가한다. default로 삼으려면 보호 kernel(`gmres_into.rs`)에 열 단위 검사를 넣거나 client driver를 matrix-free 기본 경로로 승격하는 결정이 필요하고, 어느 쪽이든 보호 파일 또는 default routing을 건드리므로 owner 결정 사항이다.
- default 경로의 golden(`default_path_golden.rs`)은 반복 수와 상태 비트가 달라지므로 다시 만들어야 한다. 그 전에 G0과 hosted CI에서의 재현을 확인해야 한다.
- C의 정책은 상수 하나(kappa = 1e-3)를 네 family에서 한 번 확인한 것이다. 성분 scale이 크게 다른 문제(atol << rtol*|y|인 성분과 0에 가까운 성분이 섞인 경우)는 이번 family에 없다: 그런 문제에서는 `min_i(atol + rtol*|y_i|)`가 가장 작은 성분에 맞춰져 보수적이 되지만, 2-norm 검사가 WRMS 검사를 대신할 수 있는지는 따로 확인해야 한다. 비자율 문제, mass matrix, DAE도 범위 밖이다.
- Krylov 한도에 걸리는 영역은 탐색 cell 하나(5.7절)만 봤다. 거기서는 검사 주기의 효과가 선언 family보다 작고 허용치 정책의 효과가 크지만, tolerance가 하나뿐이고 matched error 비교가 아니다. 이 영역에 대한 판정 자료는 없다. client의 preconditioner는 right Jacobi뿐이다(5.5절).
- restart 길이(5.4절)는 한 tolerance에서 본 탐색 자료다. 보호 driver의 설정값만 바꾸는 대안이므로, default 후보로 삼으려면 여러 tolerance와 family에서 matched error로 따로 사전등록해 확인해야 한다.
- facade에 opt-in 선택지를 내는 일(V03 계열)과 문서화가 남아 있다.

## 8. 사전등록 수정과 한계

- 사전등록 수정은 `PREDECLARED.json`의 `amendments`에 영어로 적혀 있다. 아래는 그 원문이다. 요지: 축소 instance에서 arm A만 돌려 본 결과 처음 선언한 family 두 개(ac3d, grayscott2d)는 끝 상태가 평형점이어서 달성 오차를 맞출 수 없었고, 나머지 둘은 구간 또는 시작 상태를 고쳤다. 선언 크기에서의 실행과 arm 비교가 있기 전에 commit했다.
- 수정 A1 (2026-11-17 (KST)): The family list and the time spans are replaced by 'families_final'. The original 'families' block is kept unchanged above for the record. ac3d is replaced by fhn3d (3D, new); grayscott2d (grid 40) is replaced by bruss1d with 700 cells (V06 family at a new size); the span of cusp1d is [0, 0.7] instead of [0, 0.1]; burgers2d starts with a mean flow of 4.0 in u instead of 0.5. The time_span_rule of the original block (scaling by powers of two towards a wall window) is withdrawn.
  - 시점: committed before any run at a declared size; no declared cell had been executed and no comparison of arms on a declared cell existed
  - 이유: ac3d: on the reduced instance (m = 8) the solution reaches the uniform equilibrium u = 1 before t = 0.7 (minimum and maximum of the state at t = 1 both equal 1.0; minimum 0.950 at t = 0.7), with 4 to 26 attempts of arm A at rtol 1e-5 for end times between 0.05 and 0.7. An achieved error at an equilibrium is damped to round-off whatever the tolerance and cannot be matched across tolerances.
  - 이유: grayscott2d: with the crate's start state the bump dies (on grid 16 the largest v is 0.033 at t = 10, 0.0072 at t = 20 and 1.5e-4 at t = 50) and arm A takes 9 to 13 attempts at rtol 1e-5 for end times 10 to 50: again an end state at the trivial equilibrium and so few attempts that one attempt more is 8 to 11 percent, which would decide the frozen attempts bounds (1.05, 1.10) by granularity.
  - 이유: cusp1d: on [0, 0.1] the solution only relaxes onto the cusp surface (47 attempts at N = 100, rtol 1e-5). The jumps that make CUSP a stiff test start near t = 0.65 (attempts at N = 100 for end times 0.6, 0.7, 0.8, 0.9, 1.1: 56, 158, 215, 273, 429). [0, 0.7] includes the start of that phase at a cost the timing protocol can carry.
  - 이유: burgers2d: with a mean flow of 0.5 arm A takes 7, 12 and 27 attempts (rtol 1e-3, 1e-5, 1e-7) on the reduced instance: too few for the attempts bounds. A mean flow of 4.0 (same equations, same grid, same viscosity) gives 12, 27 and 66.
  - 이유: The withdrawn span rule could only scale a span by the predicted wall and would have kept or lengthened spans that end in an equilibrium.
  - 적용 규칙: A family is admissible if on its reduced instance (a) the state at t1 is not an equilibrium (the solution is still changing), (b) arm A takes at least 12 attempts at the loosest tolerance, and (c) the wall of arm A predicted for the declared size (reduced wall times n_declared / n_reduced) at rtol 1e-5 is at most about 20 seconds. Only arm A was run for this choice; no arm was compared with another.
- 한계:
  - family는 네 개, 크기는 n = 1,200~2,000이다. 더 큰 n, 다른 이산화, 실제 사용자 문제로의 외삽은 이 자료가 보장하지 않는다.
  - `fhn3d`는 격자가 10^3이다. 이 해상도에서 확산이 stiff하려면 확산 길이가 영역 크기에 가까워야 하므로 해가 공간적으로 매끄럽다. 3D 문제로서 격자 해상도가 낮다.
  - `bruss1d`는 V06 family의 새 크기(700 cell)다. 완전히 새로운 family는 세 개다.
  - A의 잔차 검사는 stage 방정식으로 우변을 다시 만드는 방식이라 kernel이 받은 벡터와 반올림 수준에서 다를 수 있다. A의 검사 실패는 0건이다.
  - instructions는 사용자 모드 명령 수이고 캐시와 메모리 대역폭 효과를 반영하지 않는다. wall이 그 효과를 보여 주지만 이번에는 host 부하 때문에 authoritative 표시를 받지 못한 cell이 있다(4절).
  - 초기 step은 crate 기본값 1e-3이다. V06은 1e-6을 썼고 그 검토에서 초기 구간이 JVP 비에 영향을 준다고 지적했다. 다른 초기 step에서의 값은 재지 않았다.
- parser의 일관성 검사에서 나온 문제: 0건

## 9. 테스트, 회귀, 파일

- V06 회귀: V17 build로 V06 측정 target을 V06 cell에서 다시 돌려 `shared_krylov/logs`의 기록과 비교했다. 비교 14건 중 counter, 통계, 최종 상태 비트가 모두 같은 것 14건.

| V06 실행 | 동일 | 비교한 필드 수 | 최종 상태 비트 동일 | attempts | JVP | 값이 다른 필드 | V06 기록에 없던 필드 |
|---|---|---|---|---|---|---|---|
| `a_adr2d_1e-4_B0_none.log` | 예 | 22 | 예 | 21 | 6,933 |  | linear_rtol |
| `a_adr2d_1e-4_B_jacobi_audit1.log` | 예 | 23 | 예 | 21 | 3,343 |  | linear_rtol |
| `a_adr2d_1e-4_B_none_audit1.log` | 예 | 23 | 예 | 21 | 2,461 |  | linear_rtol |
| `a_adr2d_1e-4_C_none_audit1.log` | 예 | 23 | 예 | 21 | 1,693 |  | linear_rtol |
| `a_adr2d_1e-6_B0_none.log` | 예 | 22 | 예 | 45 | 14,997 |  | linear_rtol |
| `a_adr2d_1e-6_B_jacobi_audit1.log` | 예 | 23 | 예 | 45 | 4,923 |  | linear_rtol |
| `a_adr2d_1e-6_B_none_audit1.log` | 예 | 23 | 예 | 45 | 3,851 |  | linear_rtol |
| `a_adr2d_1e-6_C_none_audit1.log` | 예 | 23 | 예 | 45 | 2,574 |  | linear_rtol |
| `a_grayscott2d_1e-4_B0_none.log` | 예 | 22 | 예 | 14 | 4,540 |  | linear_rtol |
| `a_grayscott2d_1e-4_B_jacobi_audit1.log` | 예 | 23 | 예 | 14 | 1,734 |  | linear_rtol |
| `a_grayscott2d_1e-4_B_none_audit1.log` | 예 | 23 | 예 | 14 | 1,719 |  | linear_rtol |
| `a_grayscott2d_1e-6_B0_none.log` | 예 | 22 | 예 | 18 | 5,597 |  | linear_rtol |
| `a_grayscott2d_1e-6_B_jacobi_audit1.log` | 예 | 23 | 예 | 18 | 2,176 |  | linear_rtol |
| `a_grayscott2d_1e-6_B_none_audit1.log` | 예 | 23 | 예 | 18 | 2,065 |  | linear_rtol |

- 테스트 target (모두 `cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test <target>`): `stage_clients_krylov_v17_contracts`(11개), `stage_clients_krylov_v17_alloc`(1개, 전용 binary), V06의 `stage_clients_krylov_contracts`, `stage_clients_krylov_alloc`, sentinel `ppsw1_6b_contracts`, `ptsw2b_contracts`. 실행 결과는 `logs/checks/`에 있다.
- 파일:
  - `PREDECLARED.json`: 사전등록과 수정 A1
  - `NOTES.txt`: 작업 기록
  - `reference/make_reference.py, reference/*.json`: numpy 기준해
  - `run_measurements.py`: 실행 driver (setup, runs, timing, exploratory, regression)
  - `parse_results.py -> RESULTS.json`: 로그만으로 결과 생성
  - `make_report.py -> REPORT_KO.md`: 이 문서
  - `logs/`: 원자료 (sizing, reference, setup, runs, timing, exploratory, v06_regression, checks)
  - `crates/rodas5p-integrators/src/stage_clients/krylov.rs, krylov/step_tied.rs`: client 추가분 (opt-in)
  - `crates/rodas5p-integrators/tests/stage_clients_krylov_v17_{problems,contracts,alloc,measure}.rs`: family, 계약 테스트, 할당 테스트, 측정 프로그램

