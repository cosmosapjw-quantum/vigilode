# V07 투기적 Jacobian, 단계 a (순차 모사) 결과 보고

- 노드: `V07_SPECULATIVE_JACOBIAN`, 단계 a만 수행 (thread 없음, wall time 주장 없음). 단계 b는 만들지 않았다.
- 브랜치 `research/production-loop-20261116-w2-v07`. 사전선언 `PREDECLARED.json`은 첫 commit(`16cd264d`)이고, 측정 로그는 코드 commit `94960932`에서 만들었다(로그 머리말에 기록).
- 이 보고서의 수치는 전부 `RESULTS.json`에서 `build_report.py`가 찍은 것이다. `RESULTS.json`은 `build_results.py`가 raw 로그(`logs/`)와 `reference/reference.json`만 읽어 만든다.

## 1. 결론

| 모드 | step 비 (중앙값 ≤ 1.02, 최대 ≤ 1.05) | 오차 비 ([0.67, 1.5]) | 관측 차수 (≥ 4.8) | KILL (g7 step 비 > 1.10) | 단계 a 판정 |
|---|---|---|---|---|---|
| G8 | PASS (중앙값 1.000, 최대 1.039) | FAIL (33셀 중 PASS 29, FAIL 3, 판정 불가 1) | PASS (판별 문제의 최소 기울기 4.862) | 해당 없음 | **FAIL** |
| G7 | PASS (중앙값 1.000, 최대 1.044) | FAIL (33셀 중 PASS 26, FAIL 6, 판정 불가 1) | PASS (판별 문제의 최소 기울기 4.842) | 미발동 | **FAIL** |

요점은 네 가지다.

1. **step 수와 차수는 유지된다.** 33개 셀(11문제 × rtol 3개)에서 시도 수 비의 중앙값은 두 모드 모두 1.000이고 최대는 G8 1.039(e5, rtol 1e-04), G7 1.044(robertson, rtol 1e-06)이다. 1.10을 넘는 셀은 없다(KILL 미발동). 판별 가능한 2개 문제에서 관측 차수는 정확 Jacobian과 G8 0.001, G7 0.021 이내로 같다.
2. **오차 비 기준은 사전선언한 엄격 해석(모든 셀이 [0.67, 1.5] 안)으로 두 모드 모두 FAIL이다.** G8은 3셀(전부 e5), G7은 6셀이 띠 밖이다. 사전선언 규칙대로 판정을 FAIL로 적는다.
3. **그러나 같은 규칙은 보호 방법 자신도 통과하지 못한다.** 방법은 그대로 두고 rtol과 atol만 1.02배 또는 1/1.02배로 바꾼 대조 실행(탐색적 E4)의 오차 비도 6셀과 5셀에서 띠를 벗어난다. 즉 이 문제 집합에서 끝점 오차 비의 셀별 잡음이 띠 폭보다 크고, 이 기준은 투기적 Jacobian의 효과와 step 열이 조금 달라진 효과를 구분하지 못한다. e5에서는 보호 방법의 끝점 오차 자체가 허용오차의 4.3e+02배에서 3.2e+04배다.
4. **한 step 지연된 Jacobian(탐색적 대조)은 이론 S4대로 무너진다.** 시도 수 비 중앙값 3.024, 최대 11.320, 관측 차수 3.024(정확 4.863)와 3.631(정확 4.976). g7·g8에서 평가한 Jacobian과는 질적으로 다르다.

판정을 바꿀 권한은 소유자에게 있다. 오차 비 기준을 셀별 엄격 해석으로 유지하면 두 모드 모두 FAIL이고 단계 b는 시작하지 않는다. 대조군 잡음을 근거로 기준의 집계 방식(예: 중앙값, 대조군 대비 분포, 허용오차를 못 맞춘 문제의 처리)을 정하는 것은 이 노드가 사후에 할 수 없는 결정이다(9절).

## 2. 만든 것

`crates/rodas5p-integrators/src/stage_clients/speculative.rs` (보호 파일 무수정, trait 그대로 사용).

- `StageArgumentRecorder`와 `wrap_rhs`: 우변 callback을 감싸 모든 호출을 그대로 전달하고, `factor` 이후 처음 7번 호출의 시간·상태 인자를 복사한다. `OdeProblem`의 callback이 private이라 문제를 만들기 전에 우변에 씌운다.
- `SpeculativeJacobianClient<C>`: 임의의 `StageLinearSystem` client를 감싼다. `factor`와 `solve_in_place`는 그대로 넘기고, `linearize(t, y)`만 모드의 점에서 호출한다. 모드: `Exact`(대조), `G8`, `G7`, `LaggedOneStep`(탐색적 대조).
- trait만으로 표현됐으므로 별도 연구 driver는 만들지 않았다. driver의 stage, controller, 출력은 보호 driver 그대로다.

### 2.1 어느 호출이 어느 stage인가 (가정하지 않고 테스트로 증명)

fast driver의 fresh attempt는 `linearize` → `f(t, y)` → (비자율이면 시간 미분) → `factor` → stage 2..8의 우변 평가 7회 순서다. `factor`와 attempt 끝 사이에 다른 우변 호출은 없다. `tests/stage_clients_speculative_contracts.rs`가 이를 두 방법으로 확인한다.

- 변환형(U-form) 거울: 매 accepted step을 공개 kernel로 driver와 같은 연산 순서로 다시 계산한다. 거울의 새 상태가 driver의 accepted 상태와 bit 단위로 같고, `factor` 뒤 k번째 기록 호출이 stage k+1의 인자와 bit 단위로 같으며 시간 인자는 `t + c_{k+1} h`다. HIRES, Robertson, 비자율 PR(해석적 f_t / 차분 f_t), Brusselator N=100(거절 뒤 수락된 step 포함)에서 확인.
- 독립 형식(k-form): 변환하지 않은 Rosenbrock 식으로 같은 step을 계산해 기록값과 반올림 수준으로 일치함을 확인하고, g7, g8, 새 상태 사이 간격이 그 불일치보다 천 배 이상 큼을 단언한다(수치는 테스트 출력 `SPECJ_KFORM_JSON`).
- 이론 S5의 항등식 `g7 = g6 + U6`, `g8 = g7 + U7`, `y_new = g8 + U8`이 부동소수 합으로 bit 단위 성립함도 확인했다. 즉 g8은 embedded 해이고 `y_new - g8`이 오차 추정 벡터다.

### 2.2 예외 규칙 (정확 Jacobian을 쓰는 경우)

1. 실행의 첫 linearization (이전 attempt가 없음).
2. 직전 linearization과 같은 시각의 linearization. driver는 error형 실패(`LinearSolve`, `NonlinearSolve`, `NonFinite`) 뒤의 재시도에서만 그렇게 한다. 실패 원인이 투기점일 수 있고, threaded driver도 그 시점에는 accepted 상태를 이미 갖고 있어 겹칠 것이 없다. stage 평가 실패, factorization 전 `f(t, y)` 실패, 투기점에서의 Jacobian 실패 세 경우를 테스트했다.
3. recorder에 그 시각에서 끝나는 완결된 attempt가 없을 때 (wrapper를 거친 여분의 우변 호출, `reset` 없이 재사용된 client 등).

**국소 오차 거절 시**: driver는 linearization을 유지한다. 재시도 전에 `linearize`가 아니라 더 작은 h로 `factor`만 부른다. 따라서 재시도는 거절된 attempt가 쓰던 Jacobian(직전 accepted attempt의 g7/g8에서 평가한 것, 또는 예외 규칙의 정확 Jacobian)을 그대로 쓴다. 거절된 attempt의 stage 인자는 재시도의 것으로 덮이고 한 번도 쓰이지 않는다. 재시도가 수락되면 다음 linearization은 그 재시도의 g7/g8을 쓴다.

**시간 인자**: 투기적 Jacobian은 기록된 stage 평가의 시간 인자 `t_n + c_i h`에서 평가한다. stage 7, 8은 `c_i = 1`이고 driver가 같은 합으로 시계를 전진시키므로 accepted 시각 `t_{n+1}`과 bit 단위로 같다(테스트로 확인, 다르면 예외 규칙 3).

**시간 미분 f_t**: client의 일이 아니다. driver가 fresh attempt마다 accepted `(t_{n+1}, u_{n+1})`에서 직접 구한다(`partial_t` callback, 없으면 t에 대한 중심차분으로 우변 2회). 따라서 f_t와 `f(t_{n+1}, u_{n+1})`은 모든 모드에서 정확하고 Jacobian만 투기적이다. 테스트가 f_t 평가점이 전부 accepted 상태임을 bit 단위로 확인한다.

### 2.3 테스트 (`cargo test`, release)

- `stage_clients_speculative_contracts` 17개: Exact 모드와 보호 dense driver의 bit 동일성(in-place LU: Robertson, HIRES, Brusselator N=32/100, 비자율 PR 2종; faer LU: mean-field Brusselator n=200; 각 rtol 3개; counter 전 항목, 출력, linearization 점), wrapper 결과 중립성(호출 단위와 보호 driver 전체 실행), 호출-stage 대응, 모드별 linearization 점, 실패 뒤 정확 재시도, f_t 평가점, 오용 거부와 `reset`, 모델 검증(해석적 Jacobian 대 차분, 닫힌 해, 저장소의 기존 정의와 bit 일치).
- `stage_clients_speculative_alloc` 1개(전용 binary): 5문제 × 4모드 × rtol 3개에서 warm-up 뒤 accepted step 사이 할당 0, 실행 전체 할당 수가 step 수와 무관.
- `stage_clients_speculative_measure`: 측정 프로그램 3개(`#[ignore]`).

## 3. 측정 프로토콜 요약 (`PREDECLARED.json`)

- arm: `protected`(보호 dense driver, wrapper 없는 문제), `exact`(client, Exact 모드; 매 셀에서 protected와 bit 동일해야 함), `g8`, `g7`, 탐색적 `lagged`와 tolerance 교란 대조 2개.
- 적응 집합 11문제 × rtol {1e-4, 1e-6, 1e-8}: robertson, vdpol-eps1e-6, oregonator, hires, pollution, e5, brusselator-1d-32/100, pr-nonlinear(비자율, 상태 의존 stiff rate), combustion-100, nldiff-100(준선형 확산). atol = rtol × 문제별 scale(e5는 1.7e-24 고정), initial_step 1e-6, 출력은 양 끝점만.
- step 비 = 시도 수(accepted + rejected) 비, 정수 산술로 비교. 끝점 오차 = controller 가중치의 WRMS(허용오차 단위). 오차 비 구간은 참조 불확실성 U로 계산.
- 참조: SciPy Radau rtol 1e-13(값), 불확실성 `u = |ref − A| + max(|ref − B|, |ref − C|)`, A = SciPy Radau rtol 1e-11, B = Julia RadauIIA5, C = Julia Rodas5P (둘 다 reltol 1e-13). pr-nonlinear와 차수 문제는 닫힌 해.
- 차수 집합 5문제, 고정 step h = 1.5/N (N 16개, 전부 이진 유리수). 고정 step은 `initial_step = max_step = h`, `rtol = atol = 1e10`으로 강제하고 실행마다 검증. 맞춤 구간은 exact arm만으로 정하는 규칙(큰 h부터 내려가 exact 오차가 1e-11 아래로 떨어지기 전까지, 그중 가장 작은 h 6개).

프로토콜 점검 (`RESULTS.json`의 `protocol_checks`): 셀 목록이 사전선언과 같음 예, 차수 문제가 같음 예, N 목록이 같음 예, 두 로그 완결 예/예, 모든 exact arm이 protected와 bit 동일 예, 유효 셀 33/33, Rust와 Python 모델 정의 일치 예 (probe 48개, 우변 최대 상대차 0.0e+00).

## 4. 결과 (동결 판별 기준)

### 4.1 step 비와 Jacobian 수

| 문제 | rtol | protected 시도 (거절) | G8 시도 / 비 | G7 시도 / 비 | Jacobian 수 prot / G8 / G7 | 대조 rtol×1.02 비 | 대조 rtol/1.02 비 | lagged 비 (탐색적) |
|---|---|---|---|---|---|---|---|---|
| robertson | 1e-04 | 26 (1) | 26 / 1.000 | 26 / 1.000 | 25 / 25 / 25 | 1.000 | 1.000 | 1.538 |
| robertson | 1e-06 | 45 (2) | 45 / 1.000 | 47 / 1.044 | 43 / 43 / 43 | 1.000 | 1.000 | 3.133 |
| robertson | 1e-08 | 103 (4) | 103 / 1.000 | 103 / 1.000 | 99 / 99 / 99 | 1.000 | 1.010 | 11.320 |
| vdpol-eps1e-6 | 1e-04 | 341 (132) | 344 / 1.009 | 341 / 1.000 | 209 / 209 / 208 | 1.003 | 0.994 | 1.469 |
| vdpol-eps1e-6 | 1e-06 | 634 (50) | 635 / 1.002 | 634 / 1.000 | 584 / 584 / 583 | 1.003 | 1.008 | 3.849 |
| vdpol-eps1e-6 | 1e-08 | 1965 (23) | 1966 / 1.001 | 1965 / 1.000 | 1942 / 1942 / 1942 | 0.993 | 1.005 | 9.348 |
| oregonator | 1e-04 | 389 (128) | 389 / 1.000 | 396 / 1.018 | 261 / 261 / 263 | 1.008 | 1.003 | 1.553 |
| oregonator | 1e-06 | 766 (33) | 764 / 0.997 | 766 / 1.000 | 733 / 733 / 733 | 0.996 | 1.005 | 4.061 |
| oregonator | 1e-08 | 2324 (7) | 2324 / 1.000 | 2325 / 1.000 | 2317 / 2317 / 2317 | 0.995 | 1.006 | 7.926 |
| hires | 1e-04 | 82 (24) | 83 / 1.012 | 83 / 1.012 | 58 / 58 / 58 | 1.024 | 1.024 | 1.963 |
| hires | 1e-06 | 210 (6) | 212 / 1.010 | 210 / 1.000 | 204 / 204 / 204 | 1.000 | 1.010 | 5.143 |
| hires | 1e-08 | 784 (6) | 781 / 0.996 | 784 / 1.000 | 778 / 777 / 778 | 0.992 | 1.005 | 9.838 |
| pollution | 1e-04 | 35 (0) | 35 / 1.000 | 35 / 1.000 | 35 / 35 / 35 | 1.000 | 1.000 | 1.600 |
| pollution | 1e-06 | 85 (0) | 85 / 1.000 | 85 / 1.000 | 85 / 85 / 85 | 0.988 | 1.000 | 3.024 |
| pollution | 1e-08 | 254 (2) | 254 / 1.000 | 254 / 1.000 | 252 / 252 / 252 | 0.996 | 1.004 | 6.969 |
| e5 | 1e-04 | 155 (4) | 161 / 1.039 | 158 / 1.019 | 151 / 157 / 155 | 0.974 | 0.981 | 4.129 |
| e5 | 1e-06 | 408 (6) | 406 / 0.995 | 401 / 0.983 | 402 / 400 / 395 | 0.988 | 0.998 | 8.968 |
| e5 | 1e-08 | 1881 (5) | 1882 / 1.001 | 1883 / 1.001 | 1876 / 1877 / 1876 | 0.997 | 1.005 | 7.884 |
| brusselator-1d-32 | 1e-04 | 51 (9) | 52 / 1.020 | 52 / 1.020 | 42 / 42 / 42 | 1.020 | 1.000 | 1.353 |
| brusselator-1d-32 | 1e-06 | 93 (11) | 93 / 1.000 | 93 / 1.000 | 82 / 82 / 82 | 1.000 | 1.000 | 2.570 |
| brusselator-1d-32 | 1e-08 | 192 (3) | 192 / 1.000 | 192 / 1.000 | 189 / 189 / 189 | 1.000 | 1.005 | 5.375 |
| brusselator-1d-100 | 1e-04 | 52 (10) | 52 / 1.000 | 52 / 1.000 | 42 / 42 / 42 | 1.000 | 0.981 | 1.308 |
| brusselator-1d-100 | 1e-06 | 92 (10) | 92 / 1.000 | 92 / 1.000 | 82 / 82 / 82 | 1.000 | 1.000 | 2.587 |
| brusselator-1d-100 | 1e-08 | 192 (3) | 192 / 1.000 | 192 / 1.000 | 189 / 189 / 189 | 1.005 | 1.010 | 5.354 |
| pr-nonlinear | 1e-04 | 106 (27) | 106 / 1.000 | 107 / 1.009 | 79 / 79 / 78 | 1.028 | 1.019 | 1.745 |
| pr-nonlinear | 1e-06 | 245 (40) | 245 / 1.000 | 245 / 1.000 | 205 / 205 / 205 | 1.000 | 1.020 | 1.739 |
| pr-nonlinear | 1e-08 | 673 (45) | 673 / 1.000 | 671 / 0.997 | 628 / 628 / 628 | 1.006 | 1.018 | 1.865 |
| combustion-100 | 1e-04 | 73 (21) | 73 / 1.000 | 72 / 0.986 | 52 / 52 / 51 | 0.973 | 0.973 | 0.945 |
| combustion-100 | 1e-06 | 106 (24) | 106 / 1.000 | 106 / 1.000 | 82 / 82 / 82 | 1.000 | 1.000 | 1.934 |
| combustion-100 | 1e-08 | 234 (37) | 234 / 1.000 | 230 / 0.983 | 197 / 197 / 197 | 1.000 | 0.996 | 3.923 |
| nldiff-100 | 1e-04 | 88 (13) | 88 / 1.000 | 87 / 0.989 | 75 / 75 / 75 | 1.000 | 0.977 | 1.432 |
| nldiff-100 | 1e-06 | 174 (39) | 174 / 1.000 | 165 / 0.948 | 135 / 135 / 134 | 1.017 | 1.023 | 2.006 |
| nldiff-100 | 1e-08 | 409 (23) | 409 / 1.000 | 411 / 1.005 | 386 / 386 / 388 | 0.993 | 1.007 | 4.098 |

- G8: 중앙값 1.000, 최소 0.995, 최대 1.039. 비 = 1인 셀 22, < 1인 셀 3, > 1인 셀 8, > 1.02인 셀 1, > 1.05인 셀 0, > 1.10인 셀 0. 전체 시도 수 합의 비 1.0007, Jacobian 수 합의 비 1.0003. linearization 12523회 중 투기점 12490, 첫 step 정확 33, 실패 뒤 정확 0, 점 없음 0. → **PASS**
- G7: 중앙값 1.000, 최소 0.948, 최대 1.044. 비 = 1인 셀 18, < 1인 셀 6, > 1인 셀 9, > 1.02인 셀 1, > 1.05인 셀 0, > 1.10인 셀 0. 전체 시도 수 합의 비 0.9997, Jacobian 수 합의 비 0.9997. linearization 12515회 중 투기점 12482, 첫 step 정확 33, 실패 뒤 정확 0, 점 없음 0. → **PASS**
- KILL (g7의 step 비 > 1.10): 해당 셀 없음, 미발동.
- 참고(탐색적): tolerance 교란 대조의 step 비 최대는 1.028와 1.024다. 투기 모드의 최대(1.039, 1.044)와 같은 크기다.

### 4.2 끝점 오차 비

`e`는 허용오차 단위의 끝점 오차(WRMS), `U`는 같은 단위의 참조 불확실성이다. 비 옆 글자는 셀 판정(P = PASS, F = FAIL, N = 판정 불가)이다.

| 문제 | rtol | e(protected) | U | G8 비 | G7 비 | 대조 rtol×1.02 | 대조 rtol/1.02 | lagged (탐색적) |
|---|---|---|---|---|---|---|---|---|
| robertson | 1e-04 | 9.1e-02 | 5.1e-08 | 0.997 P | 0.999 P | 1.011 P | 0.995 P | 1.256 P |
| robertson | 1e-06 | 5.8e-01 | 5.1e-06 | 1.001 P | 0.982 P | 1.039 P | 0.973 P | 0.094 F |
| robertson | 1e-08 | 3.8e-01 | 5.1e-04 | 1.000 P | 1.042 P | 1.058 P | 1.017 P | 0.022 F |
| vdpol-eps1e-6 | 1e-04 | 1.1e-01 | 1.8e-07 | 1.387 P | 1.121 P | 0.861 P | 0.950 P | 7.157 F |
| vdpol-eps1e-6 | 1e-06 | 3.3e-01 | 1.8e-05 | 0.998 P | 0.855 P | 0.844 P | 0.919 P | 1.611 F |
| vdpol-eps1e-6 | 1e-08 | 5.1e-01 | 1.8e-03 | 1.020 P | 1.022 P | 1.024 P | 0.991 P | 3.333 F |
| oregonator | 1e-04 | 9.4e-02 | 1.6e-05 | 1.067 P | 0.974 P | 0.974 P | 1.282 P | 0.704 P |
| oregonator | 1e-06 | 4.3e-01 | 1.6e-03 | 1.006 P | 0.985 P | 1.012 P | 0.966 P | 0.249 F |
| oregonator | 1e-08 | 1.7e-01 | 1.6e-01 | 1.000 N | 0.996 N | 1.021 N | 0.978 N | 0.480 N |
| hires | 1e-04 | 4.2e-01 | 1.4e-07 | 0.944 P | 0.973 P | 0.974 P | 0.962 P | 0.826 P |
| hires | 1e-06 | 1.3e-01 | 1.4e-05 | 1.461 P | 1.050 P | 1.562 F | 1.280 P | 5.288 F |
| hires | 1e-08 | 5.3e-02 | 1.4e-03 | 0.879 P | 0.880 P | 1.033 P | 0.710 N | 5.542 F |
| pollution | 1e-04 | 2.6e-03 | 1.6e-08 | 0.995 P | 1.373 P | 0.882 P | 3.116 F | 47.810 F |
| pollution | 1e-06 | 9.2e-03 | 1.6e-06 | 0.948 P | 0.712 P | 1.787 F | 0.727 P | 26.004 F |
| pollution | 1e-08 | 2.8e-02 | 1.6e-04 | 0.988 P | 0.939 P | 1.532 F | 1.109 P | 7.592 F |
| e5 | 1e-04 | 3.2e+04 | 6.0e-01 | 0.106 F | 0.239 F | 1.276 P | 1.869 F | 0.084 F |
| e5 | 1e-06 | 3.4e+03 | 6.1e-01 | 1.848 F | 3.895 F | 2.959 F | 2.248 F | 0.267 F |
| e5 | 1e-08 | 4.3e+02 | 6.1e-01 | 0.135 F | 0.544 F | 2.126 F | 1.613 F | 0.174 F |
| brusselator-1d-32 | 1e-04 | 1.7e-01 | 1.7e-07 | 1.035 P | 0.977 P | 1.040 P | 1.022 P | 3.299 F |
| brusselator-1d-32 | 1e-06 | 8.8e-01 | 1.7e-05 | 1.002 P | 1.018 P | 1.029 P | 0.967 P | 0.474 F |
| brusselator-1d-32 | 1e-08 | 8.8e-01 | 1.7e-03 | 1.000 P | 0.997 P | 1.024 P | 0.981 P | 0.411 F |
| brusselator-1d-100 | 1e-04 | 1.8e-01 | 1.7e-07 | 0.994 P | 0.920 P | 0.991 P | 0.971 P | 3.156 F |
| brusselator-1d-100 | 1e-06 | 8.7e-01 | 1.7e-05 | 1.002 P | 1.017 P | 1.029 P | 0.967 P | 0.480 F |
| brusselator-1d-100 | 1e-08 | 8.8e-01 | 1.7e-03 | 1.000 P | 0.996 P | 1.023 P | 0.981 P | 0.411 F |
| pr-nonlinear | 1e-04 | 4.9e-02 | 7.0e-13 | 1.037 P | 0.667 F | 0.534 F | 0.578 F | 0.971 P |
| pr-nonlinear | 1e-06 | 2.8e-02 | 7.0e-11 | 1.000 P | 0.895 P | 0.945 P | 0.988 P | 2.505 F |
| pr-nonlinear | 1e-08 | 1.2e-02 | 7.0e-09 | 1.000 P | 0.661 F | 0.694 P | 0.746 P | 18.150 F |
| combustion-100 | 1e-04 | 1.8e+01 | 2.3e-06 | 0.899 P | 0.934 P | 0.961 P | 1.040 P | 3.534 F |
| combustion-100 | 1e-06 | 7.7e+01 | 2.3e-04 | 0.999 P | 0.979 P | 1.006 P | 0.994 P | 1.772 F |
| combustion-100 | 1e-08 | 1.4e+02 | 2.3e-02 | 1.000 P | 1.047 P | 1.024 P | 1.037 P | 0.600 F |
| nldiff-100 | 1e-04 | 2.1e-02 | 3.3e-09 | 1.032 P | 1.196 P | 1.178 P | 0.923 P | 4.876 F |
| nldiff-100 | 1e-06 | 1.8e-01 | 3.3e-07 | 0.983 P | 1.514 F | 1.178 P | 0.893 P | 1.315 P |
| nldiff-100 | 1e-08 | 1.0e-01 | 3.3e-05 | 0.953 P | 0.888 P | 0.994 P | 1.137 P | 2.304 F |

- G8: PASS 29, FAIL 3, 판정 불가 1. 비의 중앙값 1.000, 최소 0.106, 최대 1.848. PASS가 아닌 셀: oregonator rtol 1e-08 (1.000, 판정 불가), e5 rtol 1e-04 (0.106, FAIL), e5 rtol 1e-06 (1.848, FAIL), e5 rtol 1e-08 (0.135, FAIL). → **FAIL** (엄격 해석)
- G7: PASS 26, FAIL 6, 판정 불가 1. 비의 중앙값 0.982, 최소 0.239, 최대 3.895. PASS가 아닌 셀: oregonator rtol 1e-08 (0.996, 판정 불가), e5 rtol 1e-04 (0.239, FAIL), e5 rtol 1e-06 (3.895, FAIL), e5 rtol 1e-08 (0.544, FAIL), pr-nonlinear rtol 1e-04 (0.667, FAIL), pr-nonlinear rtol 1e-08 (0.661, FAIL), nldiff-100 rtol 1e-06 (1.514, FAIL). → **FAIL** (엄격 해석)

해석에 필요한 사실 (탐색적, 판정에 쓰지 않음):

| arm | 셀 PASS / FAIL / 판정 불가 | 점 추정이 띠 밖인 셀 | |ln 비| 중앙값 | |ln 비| 90% 분위 | |ln 비| 최대 |
|---|---|---|---|---|---|
| G8 | 29 / 3 / 1 | 3 | 0.012 | 0.379 | 2.249 |
| G7 | 26 / 6 / 1 | 6 | 0.063 | 0.415 | 1.432 |
| 대조 rtol×1.02 | 26 / 6 / 1 | 6 | 0.040 | 0.580 | 1.085 |
| 대조 rtol/1.02 | 26 / 5 / 2 | 5 | 0.039 | 0.548 | 1.137 |

| 문제 | e(protected) 범위 (허용오차 단위) | G8 FAIL 셀 | G7 FAIL 셀 | 대조 ×1.02 FAIL 셀 | 대조 /1.02 FAIL 셀 |
|---|---|---|---|---|---|
| robertson | 9.1e-02 – 5.8e-01 | 0 | 0 | 0 | 0 |
| vdpol-eps1e-6 | 1.1e-01 – 5.1e-01 | 0 | 0 | 0 | 0 |
| oregonator | 9.4e-02 – 4.3e-01 | 0 | 0 | 0 | 0 |
| hires | 5.3e-02 – 4.2e-01 | 0 | 0 | 1 | 0 |
| pollution | 2.6e-03 – 2.8e-02 | 0 | 0 | 2 | 1 |
| e5 | 4.3e+02 – 3.2e+04 | 3 | 3 | 2 | 3 |
| brusselator-1d-32 | 1.7e-01 – 8.8e-01 | 0 | 0 | 0 | 0 |
| brusselator-1d-100 | 1.8e-01 – 8.8e-01 | 0 | 0 | 0 | 0 |
| pr-nonlinear | 1.2e-02 – 4.9e-02 | 0 | 2 | 1 | 1 |
| combustion-100 | 1.8e+01 – 1.4e+02 | 0 | 0 | 0 | 0 |
| nldiff-100 | 2.1e-02 – 1.8e-01 | 0 | 1 | 0 | 0 |

- 대조 실행은 방법을 바꾸지 않는다. 그런데도 6셀과 5셀이 띠를 벗어난다. 끝점 오차는 step 열의 작은 변화에 민감해서, 같은 방법의 두 실행 사이에서도 비가 0.534에서 3.116까지 움직인다.
- G8의 FAIL은 전부 e5다. e5에서는 보호 방법의 끝점 오차가 허용오차의 4.3e+02–3.2e+04배다. 참조 끝값의 크기는 8.9e-23(초기값은 1.76e-3)이고 보호 방법의 끝 상태는 자릿수부터 다르다. 원인은 조사하지 않았다. 대조 실행도 e5 6셀 중 5셀이 FAIL이다. 이 문제의 오차 비는 방법 비교의 근거가 되지 못한다.
- G7의 e5 밖 FAIL 셀: pr-nonlinear rtol 1e-04 (오차 비 0.667, step 비 1.009); pr-nonlinear rtol 1e-08 (오차 비 0.661, step 비 0.997); nldiff-100 rtol 1e-06 (오차 비 1.514, step 비 0.948). 비가 0.67보다 작은 셀은 투기 모드의 오차가 더 **작은** 경우다.
- oregonator rtol 1e-8의 판정 불가는 참조 불확실성 때문이다. 사전선언 규칙이 두 Julia 실행 중 큰 차이를 쓰는데, RadauIIA5(B)가 요청 허용오차보다 훨씬 부정확하다(7절). B를 빼면 이 셀은 PASS다(탐색적 E6: G8 PASS 30 / FAIL 3, G7 PASS 27 / FAIL 6). 다른 셀의 판정은 바뀌지 않는다.

### 4.3 관측 차수 (고정 step)

| 문제 | 분류 | 맞춤에 쓴 N (6점) | exact | G8 | G7 | lagged (탐색적) | 판별 문제 (exact ≥ 4.8) | G8 / G7 |
|---|---|---|---|---|---|---|---|---|
| kaps-eps1 | non-stiff | 12–64 | 4.863 | 4.862 | 4.842 | 3.024 | 예 | PASS / PASS |
| logistic-pair | non-stiff | 16–96 | 4.976 | 4.977 | 4.964 | 3.631 | 예 | PASS / PASS |
| kaps-eps1e-6 | stiff | 24–128 | 2.021 | 2.021 | 2.021 | 1.026 | 아니오 | 비판별 / 비판별 |
| pr-linear | stiff, Prothero-Robinson, non-autonomous, constant Jacobian | 8–48 | 2.943 | 2.943 | 2.943 | 2.943 | 아니오 | 비판별 / 비판별 |
| pr-nonlinear | stiff, Prothero-Robinson type, non-autonomous, state-dependent Jacobian | 48–256 | 3.282 | 3.281 | 3.252 | 2.888 | 아니오 | 비판별 / 비판별 |

- 고정 step 실행 80개(문제 × N)의 다섯 arm이 모두 사전선언한 고정 step 검증을 통과했고(accepted = N, 거절 0, 최소·최대 accepted step = h bit 단위), exact arm은 전부 protected와 bit 동일하다: True.
- 판별 문제는 kaps-eps1, logistic-pair (비stiff). G8의 기울기는 exact와 0.001 이내, G7은 0.021 이내로 같고 모두 4.8 이상이다. → G8 **PASS**, G7 **PASS**.
- stiff 문제 kaps-eps1e-6, pr-linear, pr-nonlinear (Prothero–Robinson형 두 개 포함)는 exact arm 자체의 관측 차수가 2.02–3.28이라 사전선언대로 비판별이다(stiff 영역의 차수 저하). 이 문제들에서도 G8·G7의 기울기는 exact와 같다(차이 G8 0.001, G7 0.030 이내). pr-linear는 Jacobian이 상수라 모든 arm이 같은 값을 낸다.
- 판별 문제의 맞춤 구간은 h = 0.125–0.016로, 약간 점근 전 영역이다. 더 작은 h에서의 국소 차수는 `RESULTS.json`의 `order.problems[].local_orders`에 있다.

## 5. 탐색적 결과 (판정에 쓰지 않음)

### 5.1 한 step 지연된 Jacobian (E1)

- 적응 실행: 시도 수 비 중앙값 3.024, 최대 11.320(robertson, rtol 1e-08), > 1.10인 셀 32/33. rtol이 작을수록 나빠진다(오차 추정이 지연 항 O(h³)에 지배됨). 오차 비 PASS 5, FAIL 27.
- 고정 step 관측 차수: kaps-eps1 3.024, logistic-pair 3.631, kaps-eps1e-6 1.026, pr-linear 2.943, pr-nonlinear 2.888. 이론 S4의 예측(고전 차수 3)과 맞는다.

### 5.2 Jacobian 차이의 크기 (E2, E3)

고정 step, 차수 맞춤과 같은 6점에서 step별 최대 Frobenius 노름의 h에 대한 기울기 (예측: g7 4, g8 5, 지연 1; 괄호는 점 자체의 거리 `max|g − u_{n+1}|`의 기울기):

| 문제 | J(g7) − J(u) | J(g8) − J(u) | J(u_n) − J(u_{n+1}) |
|---|---|---|---|
| kaps-eps1 | 3.78 (3.78) | 4.77 (4.72) | 0.97 (0.94) |
| logistic-pair | 4.05 (4.05) | 5.31 (5.31) | 1.00 (1.00) |
| kaps-eps1e-6 | 4.70 (2.97) | 5.01 (1.96) | 0.99 (0.97) |
| pr-linear | - (2.99) | - (2.99) | - (0.99) |
| pr-nonlinear | 2.97 (2.78) | 3.73 (3.62) | 1.00 (1.00) |

- 비stiff 문제에서는 예측(4, 5, 1)에 가깝다. stiff 문제에서는 stage 인자 자체의 차수가 떨어진다(pr-nonlinear의 거리 기울기: g7 2.78, g8 3.62). pr-linear는 Jacobian이 상수라 차이가 0이다.
- 적응 실행 한 개(brusselator-1d-32, rtol 1e-06, 82 step)의 상대 크기 `‖J(점) − J(u)‖_F / ‖J(u)‖_F`: g8 중앙값 1.0e-07 최대 3.0e-07, g7 중앙값 2.3e-06 최대 1.2e-05, 지연 중앙값 6.3e-03 최대 9.4e-03.

적응 실행의 셀별 요약 (중앙값 / 최대). 거리는 허용오차 단위 WRMS다.

| 문제 | rtol | 거리 g8 | 거리 g7 | 상대 J차 g8 | 상대 J차 g7 | 상대 J차 지연 |
|---|---|---|---|---|---|---|
| robertson | 1e-04 | 1.1e-01 / 3.5e-01 | 3.6e+00 / 4.4e+00 | 2.1e-05 / 2.8e-04 | 1.8e-04 / 3.1e-03 | 1.0e-01 / 1.0e+00 |
| robertson | 1e-06 | 2.2e-01 / 9.0e-01 | 2.3e+01 / 5.8e+01 | 7.9e-07 / 9.4e-06 | 2.0e-05 / 3.6e-04 | 5.5e-02 / 1.0e+00 |
| robertson | 1e-08 | 4.2e-01 / 7.2e-01 | 5.5e+01 / 1.0e+03 | 1.9e-08 / 1.3e-07 | 1.1e-06 / 8.0e-05 | 2.0e-02 / 1.0e+00 |
| vdpol-eps1e-6 | 1e-04 | 5.1e-01 / 2.5e+00 | 2.9e+00 / 1.3e+02 | 7.7e-05 / 5.4e-04 | 5.9e-04 / 1.8e-02 | 1.6e-01 / 4.4e+00 |
| vdpol-eps1e-6 | 1e-06 | 7.2e-01 / 1.3e+00 | 9.3e+00 / 3.9e+02 | 1.1e-06 / 1.3e-05 | 1.6e-05 / 5.5e-04 | 4.8e-02 / 2.7e+00 |
| vdpol-eps1e-6 | 1e-08 | 6.2e-01 / 1.1e+00 | 8.2e+00 / 1.5e+03 | 1.0e-08 / 8.6e-06 | 1.4e-07 / 2.1e-05 | 1.0e-02 / 5.0e+01 |
| oregonator | 1e-04 | 5.3e-01 / 2.2e+00 | 2.7e+00 / 1.1e+02 | 5.1e-05 / 4.0e-04 | 3.6e-04 / 1.9e-02 | 2.0e-01 / 1.7e+00 |
| oregonator | 1e-06 | 7.1e-01 / 1.3e+00 | 4.9e+00 / 1.5e+02 | 8.6e-07 / 3.1e-06 | 8.3e-06 / 2.8e-04 | 6.2e-02 / 3.5e-01 |
| oregonator | 1e-08 | 6.2e-01 / 1.1e+00 | 5.2e+00 / 3.5e+02 | 6.4e-09 / 2.3e-08 | 9.4e-08 / 6.4e-06 | 1.5e-02 / 1.5e-01 |
| hires | 1e-04 | 1.8e-01 / 1.0e+00 | 2.9e+00 / 3.6e+01 | 5.6e-07 / 9.6e-05 | 1.1e-05 / 3.4e-04 | 1.5e-01 / 2.7e-01 |
| hires | 1e-06 | 5.5e-01 / 1.3e+00 | 2.8e+00 / 6.8e+02 | 1.8e-08 / 4.2e-07 | 1.4e-07 / 7.5e-05 | 2.7e-02 / 2.0e-01 |
| hires | 1e-08 | 5.9e-01 / 9.8e-01 | 3.6e+00 / 2.4e+02 | 7.2e-11 / 1.6e-09 | 5.4e-10 / 9.5e-08 | 6.6e-03 / 4.8e-02 |
| pollution | 1e-04 | 1.2e-01 / 2.6e-01 | 9.9e-01 / 3.2e+00 | 6.6e-16 / 3.1e-14 | 1.3e-14 / 2.5e-13 | 1.4e-10 / 4.4e-10 |
| pollution | 1e-06 | 3.4e-01 / 7.8e-01 | 2.8e+00 / 9.1e+00 | 3.5e-17 / 3.6e-16 | 5.3e-16 / 3.2e-15 | 9.3e-11 / 1.2e-10 |
| pollution | 1e-08 | 5.1e-01 / 8.8e-01 | 4.2e+00 / 1.2e+01 | 1.6e-19 / 3.5e-18 | 3.5e-18 / 3.2e-17 | 3.0e-11 / 4.7e-11 |
| e5 | 1e-04 | 3.6e-01 / 1.2e+00 | 3.0e+00 / 3.3e+01 | 9.2e-13 / 1.4e-05 | 9.8e-12 / 7.3e-05 | 1.3e-08 / 5.4e-01 |
| e5 | 1e-06 | 4.9e-01 / 9.5e-01 | 4.6e+00 / 2.5e+02 | 5.5e-14 / 2.7e-07 | 1.8e-12 / 1.6e-06 | 3.4e-08 / 2.0e-01 |
| e5 | 1e-08 | 5.7e-01 / 9.9e-01 | 3.0e+00 / 2.3e+03 | 5.0e-16 / 1.7e-09 | 2.5e-14 / 2.7e-08 | 9.8e-08 / 4.1e-02 |
| brusselator-1d-32 | 1e-04 | 2.7e-01 / 1.1e+00 | 2.9e+00 / 8.9e+00 | 5.1e-06 / 2.6e-05 | 5.3e-05 / 2.3e-04 | 1.2e-02 / 2.1e-02 |
| brusselator-1d-32 | 1e-06 | 4.3e-01 / 9.5e-01 | 1.2e+01 / 4.1e+01 | 1.0e-07 / 3.0e-07 | 2.3e-06 / 1.2e-05 | 6.4e-03 / 9.4e-03 |
| brusselator-1d-32 | 1e-08 | 5.3e-01 / 9.7e-01 | 3.8e+01 / 1.2e+02 | 1.3e-09 / 2.7e-09 | 7.1e-08 / 3.2e-07 | 2.5e-03 / 4.2e-03 |
| brusselator-1d-100 | 1e-04 | 2.7e-01 / 8.1e-01 | 2.7e+00 / 9.1e+00 | 5.4e-07 / 2.0e-06 | 5.5e-06 / 2.4e-05 | 1.3e-03 / 2.2e-03 |
| brusselator-1d-100 | 1e-06 | 4.3e-01 / 9.5e-01 | 1.2e+01 / 4.0e+01 | 1.1e-08 / 3.2e-08 | 2.5e-07 / 1.2e-06 | 6.7e-04 / 9.8e-04 |
| brusselator-1d-100 | 1e-08 | 5.3e-01 / 9.7e-01 | 3.7e+01 / 1.1e+02 | 1.4e-10 / 2.8e-10 | 7.5e-09 / 3.4e-08 | 2.7e-04 / 4.4e-04 |
| pr-nonlinear | 1e-04 | 3.7e-01 / 1.1e+00 | 3.0e+00 / 2.4e+01 | 7.6e-05 / 2.9e-04 | 6.4e-04 / 2.4e-03 | 1.1e-01 / 2.7e-01 |
| pr-nonlinear | 1e-06 | 5.6e-01 / 1.1e+00 | 1.2e+01 / 3.9e+01 | 1.2e-06 / 3.3e-06 | 2.7e-05 / 8.9e-05 | 4.3e-02 / 1.1e-01 |
| pr-nonlinear | 1e-08 | 5.9e-01 / 9.8e-01 | 4.0e+01 / 1.6e+02 | 1.4e-08 / 3.3e-08 | 9.5e-07 / 5.5e-06 | 1.4e-02 / 3.9e-02 |
| combustion-100 | 1e-04 | 4.9e-02 / 5.6e-01 | 8.4e-01 / 2.7e+00 | 5.3e-07 / 2.6e-04 | 1.4e-05 / 1.3e-03 | 4.5e-03 / 2.7e-02 |
| combustion-100 | 1e-06 | 5.4e-01 / 9.1e-01 | 1.1e+01 / 3.9e+01 | 6.6e-07 / 2.3e-06 | 3.4e-05 / 6.7e-05 | 6.8e-03 / 1.4e-02 |
| combustion-100 | 1e-08 | 5.8e-01 / 1.0e+00 | 1.9e+01 / 8.6e+01 | 1.6e-08 / 3.7e-08 | 5.2e-07 / 3.7e-06 | 2.5e-03 / 7.1e-03 |
| nldiff-100 | 1e-04 | 3.7e-01 / 8.7e-01 | 1.3e+00 / 3.1e+00 | 7.1e-05 / 2.0e-03 | 2.8e-04 / 1.2e-02 | 4.5e-02 / 9.8e-01 |
| nldiff-100 | 1e-06 | 3.4e-01 / 1.0e+00 | 1.1e+01 / 3.6e+01 | 7.3e-07 / 4.6e-05 | 2.2e-05 / 1.2e-03 | 2.5e-02 / 6.9e-01 |
| nldiff-100 | 1e-08 | 5.5e-01 / 9.9e-01 | 1.4e+01 / 1.9e+02 | 1.0e-08 / 4.1e-07 | 3.1e-07 / 9.4e-05 | 8.3e-03 / 4.3e-01 |

- g8은 accepted 상태에서 오차 추정 벡터만큼 떨어져 있다(수락 조건이 이 벡터의 노름 ≤ 1). 거리의 셀별 중앙값은 g8 4.9e-02–7.2e-01, g7 8.4e-01–5.5e+01 허용오차 단위이고, 셀별 최대값 중 가장 큰 것은 g8 2.5e+00, g7 2.3e+03다.
- Jacobian의 상대 변화(셀별 최대값 중 가장 큰 것): g8 2.0e-03, g7 1.9e-02, 지연 5.0e+01. 셀별 중앙값 중 가장 큰 것: g8 7.7e-05, g7 6.4e-04, 지연 2.0e-01.

### 5.3 matched error에서의 step 비 (E5)

| arm | 평가된 셀 | 중앙값 | 최소 | 최대 | 평가 불가 문제 (오차가 rtol에 단조가 아님) |
|---|---|---|---|---|---|
| G8 | 30 | 1.000 | 0.969 (hires, rtol 1e-08) | 1.096 (hires, rtol 1e-06) | e5 |
| G7 | 30 | 1.000 | 0.927 (pollution, rtol 1e-06) | 1.073 (pollution, rtol 1e-04) | e5 |
| lagged | 33 | 2.335 | 0.078 (e5, rtol 1e-04) | 18.864 (vdpol-eps1e-6, rtol 1e-08) | 없음 |

- 문제별 세 점 (오차, 시도 수)을 log-log 보간해, protected가 달성한 오차에서 그 arm이 필요로 하는 시도 수를 추정한 값이다. 오차와 step 수를 한 번에 본 비용 비교다. e5의 오차는 4.2절에서 본 대로 잡음이므로 e5가 포함된 arm의 최소·최대는 의미가 없다.

## 6. 참조해

| 문제 | 참조값 | A: Radau 1e-11와의 최대 상대차 | B: Julia RadauIIA5 | C: Julia Rodas5P |
|---|---|---|---|---|
| robertson | SciPy Radau 1e-13 | 1.6e-14 | 1.3e-11 | 5.0e-13 |
| vdpol-eps1e-6 | SciPy Radau 1e-13 | 3.0e-14 | 4.6e-11 | 1.2e-12 |
| oregonator | SciPy Radau 1e-13 | 1.1e-13 | 2.7e-09 | 8.3e-13 |
| hires | SciPy Radau 1e-13 | 1.9e-13 | 2.1e-11 | 3.5e-12 |
| pollution | SciPy Radau 1e-13 | 2.4e-13 | 4.6e-12 | 3.0e-13 |
| e5 | SciPy Radau 1e-13 | 1.0e+00 | 3.2e+06 | 1.0e+00 |
| brusselator-1d-32 | SciPy Radau 1e-13 | 1.8e-13 | 7.6e-11 | 4.7e-14 |
| brusselator-1d-100 | SciPy Radau 1e-13 | 1.2e-13 | 7.8e-11 | 4.9e-14 |
| pr-nonlinear | 닫힌 해 | - | - | - |
| combustion-100 | SciPy Radau 1e-13 | 4.2e-11 | 1.5e-09 | 3.5e-11 |
| nldiff-100 | SciPy Radau 1e-13 | 1.1e-13 | 4.3e-12 | 2.7e-14 |

- 모든 참조 실행이 성공했다. e5를 뺀 문제에서 SciPy 두 허용오차와 Julia Rodas5P의 최대 상대차는 1.6e-14–4.2e-11다. Julia RadauIIA5(B)의 차이는 Rodas5P(C)의 6배에서 3308배이고 불확실성 `u`를 지배한다(사전선언 규칙: 둘 중 큰 값).
- e5의 상대차 열은 0에 가까운 성분(참조값 1e-70 이하) 때문에 의미가 없다. e5의 U는 허용오차 단위로 6.1e-01이고 측정된 오차(4.3e+02–3.2e+04 단위)보다 훨씬 작다.
- Rust와 Python 정의는 probe 48개에서 우변의 최대 상대차가 0.0e+00다(`logs/check_models.log`). Julia 정의는 세 번째로 옮겨 적은 것이며, SciPy 참조와의 일치(C 열)가 그 검증이다.

## 7. 사전선언 대비 편차와 주의

1. 고정 step 검증에서 protected arm에는 accept control이 없어 `AcceptedStepView::h_used`를 읽을 수 없다. protected의 accepted step은 controller telemetry trace(`integrate_rodas5p_fast_observed_with_telemetry`, scalar atol 경로)에서 읽었다. exact arm이 80개 실행 전부에서 protected와 bit 동일하다.
2. E2의 고정 step 기울기를 어느 h 구간에서 맞출지 사전선언에 없었다. 차수 맞춤과 같은 6점을 썼다.
3. E5(matched error)의 보간 세부(오차를 셀의 허용오차 단위 크기로 환산, 끝 구간 외삽, 오차가 rtol에 단조가 아니면 제외)는 측정 뒤에 정했다. 탐색적 항목이다.
4. 4.2절의 대조군 대비 분포 표, 문제별 FAIL 표, B를 뺀 불확실성(E6)은 사전선언에 없던 사후 분석이다. 판정에 쓰지 않았다.
5. 계약 테스트는 사전선언 commit 뒤, 측정 실행 전에 돌렸다. 이 테스트는 HIRES, pr-nonlinear, Brusselator N=100의 rtol 1e-6에서 G8·G7·lagged의 시도 수를 출력한다. 그 뒤 프로토콜은 바꾸지 않았다.
6. 사전선언 전에 실행한 것은 `PREDECLARED.json`의 `status_at_commit`에 적은 두 가지뿐이다(SciPy만 쓰는 PDE 특성 파악, Julia 환경 smoke 실행).
7. 참조 스크립트는 `python3 -I`(시스템 SciPy)로 돌렸다. 버전은 `reference/reference.json`에 있다. 두 참조 스크립트는 실행 당시 commit 전이었고 그 뒤 수정 없이 commit했다(Julia 스크립트의 SHA-256은 로그 머리말에 있다).

## 8. 단계 b(threaded overlap)에 추가로 필요한 것

단계 a가 답한 것은 "J를 g7/g8에서 평가해도 방법이 유지되는가"뿐이다. 실제로 겹쳐 돌리려면 다음이 더 필요하다.

1. **G8만으로는 단계 b 통과 기준에 닿지 못한다.** g8은 stage 7의 solve가 끝나야 알 수 있으므로 겹칠 수 있는 구간은 stage 8 하나(사슬의 1/8)다. 이론 S6의 모델에서 그 상한은 9/8 = 1.125로, 동결된 단계 b 기준 1.15보다 작다. 기준에 닿을 수 있는 것은 두 stage를 겹치는 G7뿐이다(상한 1.25, r ∈ [0.15, 0.917]에서 ≥ 1.15).
2. **factorization을 겹치려면 step 크기 예측이 필요하다.** `W = I/(hγ) − J`이고 다음 h는 stage 8과 controller가 끝나야 정해진다(오차 추정이 U8). 예측 없이 겹칠 수 있는 것은 Jacobian 평가뿐이며, 그 경우 S6의 r은 `T_J / T_chain`이다. LU까지 겹치려면 (a) 보호 controller와 양립하는 h 예측기와 (b) 예측이 빗나갈 때의 재factorization 비용, (c) 적중률 측정이 필요하다. 적중률은 이번에 재지 않았다. controller 상수는 보호 대상이라 "예측한 h를 그대로 쓴다"는 선택은 할 수 없다.
3. **stage 인자가 준비된 시점을 알리는 통로.** 현재 trait에는 그런 호출이 없다. 단계 a는 우변 wrapper가 `factor` 뒤 호출 순번으로 stage를 식별했다. 단계 b는 같은 wrapper가 6번째 호출(stage 7의 인자 g7)에서 worker를 깨우고, 다음 `linearize`에서 결과를 받으면 된다. 보호 파일을 고치지 않고 client로 만들 수 있다.
4. **버리는 작업의 처리.** 거절된 attempt와 error로 실패한 attempt에서 시작한 투기 계산은 쓰지 않는다(단계 a의 규칙과 같다). 이번 측정에서 protected의 거절 비율은 셀에 따라 0%–39%다(4.1절 표). worker 취소 또는 결과 무시가 필요하고, 그 시간은 이득에서 빠진다.
5. **이중 버퍼.** stage 7–8이 W_n의 인자를 쓰는 동안 worker가 J_{n+1}을 쓰므로 Jacobian 행렬이 둘 필요하다(LU까지 겹치면 인자 저장도 둘).
6. **callback의 동시 호출 가능성.** worker의 Jacobian callback과 main thread의 우변 callback이 동시에 돈다. 타입은 `Send + Sync`지만 사용자 모델의 내부 가변 상태가 있으면 계약으로 금지하거나 epoch로 막아야 한다.
7. **결정성 검증.** threaded G7의 결과는 이번 순차 G7 모사와 bit 단위로 같아야 한다. 이번 client가 그 oracle이다.
8. **착수 조건(동결).** V04가 (J+LU)/chain ∈ [0.15, 0.92]이고 step당 wall ≥ 500 µs인 영역을 보여야 한다. 2번 때문에 LU 예측이 없으면 조건은 사실상 J/chain에 대한 것이 된다. wall 측정은 고정 core, arm 교차, warm-up 2 + 반복 7, load 기록이 필요하다.

## 9. 한계와 소유자 결정이 필요한 사항

1. **오차 비 기준의 해석.** 동결 행은 "error ratio in [0.67, 1.5]"라고만 적고 집계 방식을 정하지 않았다. 이 노드는 측정 전에 "모든 셀"로 사전선언했고 그 결과는 두 모드 모두 FAIL이다. 대조군이 같은 규칙에서 FAIL이므로, 이 FAIL이 투기적 Jacobian 때문이라는 증거는 없다. 반대로 "문제없다"는 것도 이 기준으로는 증명되지 않았다. 집계 방식을 다시 정할지(예: 대조군 분포와의 비교, 보호 방법이 허용오차를 맞춘 셀로 제한)는 소유자 결정이다.
2. **e5의 끝점.** 보호 RODAS5P는 이 설정(t_end = 1e13, atol 1.7e-24, 배정밀도)에서 e5의 끝값을 맞추지 못한다(오차가 허용오차의 4.3e+02–3.2e+04배). 이것은 V07과 무관한 보호 방법의 관찰이며 원인은 조사하지 않았다. 별도 기록이 필요할 수 있다.
   단서 하나(탐색적 E7): 이 정식화(f3 = f2 − f4)에서 `y3 − y2 + y4`는 값이 0인 선형 불변량이고, 정확한 산술에서는 Rosenbrock step이 이를 보존한다(Jacobian의 행도 같은 관계를 만족한다). protected 끝 상태의 잔차는 1.1e-19 (rtol 1e-04), -1.2e-20 (rtol 1e-06), -1.5e-21 (rtol 1e-08)이고, 참조 끝값의 크기는 8.9e-23, 참조의 잔차는 4.8e-25다. 즉 끝점 오차는 이 불변량의 부동소수 표류와 같은 크기다. 조건수가 큰 W의 solve에서 증폭된 반올림으로 추정하지만 확인하지 않았다.
3. **차수 기준의 근거 폭.** 판별 문제가 비stiff 2개뿐이다. stiff 문제에서는 보호 방법의 관측 차수가 2.02–3.28이라 4.8 기준을 적용할 수 없었다(사전선언대로 비판별). stiff 영역의 근거는 적응 실행의 step 비와 고정 step 기울기의 일치다.
4. **문제 크기.** n ≤ 200, dense 저장, 해석적 Jacobian만 썼다. banded·sparse client나 차분 Jacobian은 다루지 않았다(wrapper는 inner client와 무관하게 동작한다).
5. **참조 불확실성.** B(Julia RadauIIA5)가 부정확해 한 셀이 판정 불가가 됐다. `u`는 추정이지 증명된 상계가 아니다.

## 10. 재현

저장소 루트에서, `export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v07`와 `taskset -c 0-9,12-21`을 앞에 붙여 실행했다.

```
cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
  --test stage_clients_speculative_contracts --test stage_clients_speculative_alloc \
  --test stage_clients_speculative_problems --test stage_clients_speculative_measure
D=research/production_loop_20261116/speculative_jacobian
bash $D/run_measurements.sh                 # logs/measure_{model_probes,adaptive,fixed_step}.log
python3 -I $D/reference/check_models.py $D/logs/measure_model_probes.log > $D/logs/check_models.log
(cd $D && python3 -I reference/reference.py > logs/reference_scipy.log 2>&1)
JULIA_PKG_OFFLINE=true julia --startup-file=no --project=tools/benchmarks/component_julia \
  $D/reference/reference.jl > $D/logs/reference_julia.log 2>&1
python3 -I $D/reference/build_reference.py  # reference/reference.json
python3 -I $D/build_results.py              # RESULTS.json
python3 -I $D/build_report.py               # REPORT_KO.md
```

입력 로그의 SHA-256과 각 로그의 commit, 명령, exit code는 `RESULTS.json`의 `provenance`에 있다. wall time은 재지 않았고 수치는 전부 결정적 counter와 부동소수 상태다. 검증 명령(테스트, sentinel, clippy, rustfmt, loop checker, 엄격 JSON, 허용 경로, 측정 재실행의 byte 동일성)과 그 exit code는 `run_checks.sh`가 쓴 `logs/checks.log`에 있다.
