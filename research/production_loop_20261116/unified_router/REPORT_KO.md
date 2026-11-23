# V03 part B: 통합 router가 실제로 한 일을 보고하도록 고침

이 문서는 `build_report.py`가 `RECEIPT.json`에서 만들었다. 숫자는 모두 실행한 명령의 로그(`logs/*.log`)와 `git`에서 `build_receipt.py`가 읽었다. 손으로 적은 숫자는 없다.

**정정 고지 (2026-11-23, 독립 리뷰 `REVIEW.json`, 판정: 정정 조건부 수용).** 판정 9개는 리뷰가 원자료에서 다시 계산해 모두 PASS로 확인했다. 정정할 서술: (1) 강성 screen은 '한쪽 방향'만이 아니다. probe 점에서 fatal callback 오류가 나면 어떤 leg도 돌기 전에 solve가 그 오류로 끝난다(3.2절에 반영). (2) 판별식 A1의 '각 테스트를 실패하는 테스트로 먼저 썼다'는 base API 테스트 20개에만 해당한다. 나머지 16개는 구현 뒤에 추가했다. (3) SwitchingSolver·ProductionSolver 기록이 'byte 동일'이라고 한 것은 JSON 길이와 FNV-1a 64 비교였다(77/77). byte 단위 비교는 리뷰어가 416개 기록으로 했고 모두 같았다. (4) 정지한 실행이 driver의 원래 이유를 돌려준다는 서술은 `stop`에는 맞고 `status`에는 항상 맞지는 않는다(9절). (5) screen이 보지 못하는 강성 문제의 비용은 explicit trial이다. 리뷰어 측정에서 최악은 직접 RODAS5P 대비 RHS 호출 약 15배(Jacobian 없는 Robertson, 6233 대 415)였다. (6) 읽히지 않는 option과 `HighPerformance`를 남긴 것, 그리고 `SelectedTier` 값 3개 등 공개 항목을 없앤 것은 저장소 밖 호출자에게는 source 호환이 깨지는 변경이다. 리뷰어 수치는 `REVIEW.json`에 있고 `RECEIPT.json`에는 없다.

- base: `587a139501729bbe886bb2b8c49b0ef893705d08`
- 검증을 실행한 코드 commit: `4503b86e16a576685a756abb4d51d3d306228db6`, `ec0ee46dbc6ff74a7cef43dc0af2f941ece6b950`
- 사전등록: `PREDECLARED.json`, commit `5192520c81497e362c5aef235015da567ee63204`. 첫 로그의 commit(`8dbbfcd5ae36fe0c6f8f8b78db731b496db4b6eb`)보다 먼저 commit했는가: 예
- 시간 측정은 하지 않았다. 로그의 경과 시간은 `diagnostic_only`다.
- PREREGISTRATION.md에는 part B의 동결 행이 없다. 과제문의 수용 기준 네 개(A1-A4)를 기준으로 삼았고, 그 밖의 선택(문제, 허용오차, 경계값)은 첫 실행 전에 `PREDECLARED.json`에 적었다.

## 1. 결론

| 수용 기준 | 측정 | 판정 |
|---|---|---|
| A1 계약 테스트 통과, 각 테스트는 먼저 실패로 확인 | 통과 36, 실패 0; part 1의 20개 중 20개가 base에서 실패하고 지금 통과 | PASS |
| A2 기본 경로 golden 불변 | exit 0, golden 파일 9개 동일, base 대비 diff 0줄 | PASS |
| A3 기존 facade 테스트 전부 통과 | exit 0, 통과 533, 실패 0, 무시 0, 테스트 바이너리 56개와 doc-test 1회 | PASS |
| A4 다른 frontend의 동작 불변 | `SwitchingSolver`와 `ProductionSolver`의 기록 digest 77 / 77줄 동일, golden 9개 동일, 보호 경로 diff 0줄 | PASS |

`VigilODE::solve`는 이제 세 진입점(`Dopri5::solve`, `Rodas5p::solve`, `SwitchingSolver::solve`) 중 하나를 고르고, 실행한 driver의 식별자를 그 driver의 실행 기록에서 읽어 보고한다. 중간에 멈춘 실행은 받아들인 구간, 모든 구간과 사전 점검의 작업량, driver가 밝힌 정지 이유를 그대로 돌려준다.

## 2. 이전 router가 내세운 것과 실제

| 내세운 것 (base) | 실제 (base) | 지금 |
|---|---|---|
| tier 6개 (`Tier2MatrixFreeKrylov`, `Tier3Exponential`, `Tier4StagedRodas5p` 포함) | 넷은 같은 `Rodas5p::solve`를 불렀고 문자열만 달랐다. `Tier3Exponential`은 만들어진 적이 없다 | 진입점이 있는 값 3개만 남겼다 |
| route history의 tier 이름 | 실행 전에 적은 label | 실행 뒤 그 구간의 실행 기록에서 읽은 driver 식별자 |
| 로그 노름 μ2로 강성을 미리 감지 | μ2 ≤ 0(수축)이고 구간 ≤ 100이면 비강성으로 보고 DOPRI5로 보냈다. 열 방정식, `y' = -1e6 (y - cos t)`가 여기에 걸린다 | μ2를 계산하지 않는다. 선언된 사실로 고르고, 아무 선언이 없는 작은 문제에만 과금되는 강성 screen을 쓴다 |
| fail-closed fallback, 모든 `WorkCounters` 보존 | fallback 구간이 `max_steps`를 새로 받았고, `Stats`의 시도 수는 fallback 구간만 셌으며, fallback이 거절되면 앞 구간과 작업량을 버렸다. 치명적 callback 오류 뒤에도 다른 방법으로 다시 불렀다 | 모든 구간이 `Options::max_steps` 하나를 나눠 쓴다. 앞 구간과 작업량을 유지한다. 치명적 callback 뒤에는 다음 구간을 시작하지 않는다 |
| 실패 상태 | switching 실패는 모두 `MaxStepsReached`, 사전 점검의 callback 오류는 `InvalidInput` | driver가 밝힌 이유를 `VigilodeSolution::stop`(`UnifiedStop`)에 담고, `status`는 그에 해당하는 `FailureReason` |
| VG-1 zero-allocation workspace | `8n`, `(m+1)n`, `n*n` buffer를 모든 문제에 할당했고 어느 driver에도 넘기지 않았다. `step_into`는 Heun 단계다 | `n`짜리 vector 4개만 둔다. dense 행렬은 그것이 필요한 driver 안에서만 생긴다 |
| 사전 점검 비용 | RHS 1회(dense Jacobian이면 Jacobian 1회와 고유값 계산)를 세지 않았다 | 선언으로 정해지는 경로는 callback이 0회다. screen의 RHS 호출은 counter에 들어간다 |

## 3. 지금 router가 하는 일

### 3.1 경로 결정 (`PreflightAnalyzer::analyze`)

위에서부터 처음 맞는 규칙이 결정한다. 1-5번은 callback을 부르지 않는다.

1. `SolverStrategy::RobustStiff`는 RODAS5P, `AdaptiveReentrant`는 switching adapter.
2. 선언된 regime(새 옵션 `VigilodeOptions::regime`, 기본 `Unknown`): `Stiff`는 RODAS5P, `Nonstiff`는 시도 예산 전부로 DOPRI5를 돌리고 두 번째 방법은 없다.
3. registry가 선언된 사실에 대해 DOPRI5를 받아들이지 않으면(질량 행렬, dense output, event) RODAS5P.
4. Jacobian, JVP, band 중 하나라도 주어졌으면 RODAS5P.
5. 차원이 10을 넘거나(base의 한계를 유지) 예산이 trial을 허용하지 않으면 RODAS5P.
6. 그 밖에는 강성 screen. 증거가 있으면 RODAS5P, 없으면 DOPRI5 trial (`min(1000, max_steps / 2)`회) 뒤에, 끝나지 않으면 마지막으로 받아들인 상태에서 RODAS5P.

regime에 맞는 방법은 `MethodRegistry::general_method`, 승인 여부와 계획 경로는 `MethodRegistry::route(MethodChoice::Explicit(id))`에서 받는다. DOPRI5와 RODAS5P 구간은 `MethodRegistry::solve`로 실행한다. 어느 RODAS5P driver 계열이 도는지에 대한 규칙은 analyzer에 남기지 않았다.

### 3.2 강성 screen

`f0 = f(t0, y0)`와 좌표마다 `δ_j = sqrt(eps) max(|y0_j|, 1)`로 `L = max_j |f(t0, y0 + δ_j e_j) - f0| / δ_j`를 만든다. `L`은 `f(t0, ·)`의 Lipschitz 상수의 하계다. `L (t1 - t0) > 3.3 B`(`B`는 trial의 시도 예산, 3.3은 DOPRI5의 실수축 안정 한계)이면 trial이 그 예산으로 구간을 끝낼 수 없으므로 RODAS5P로 보낸다. RHS `n + 1`회, 저장은 `O(n)`이고 호출은 모두 counter에 들어간다.

screen은 trial을 RODAS5P로 바꿀 수는 있어도 그 반대는 없다. 증거가 없다는 것을 비강성의 증거로 쓰지 않으므로 trial에는 예산과 fallback이 그대로 붙는다. `t0` 이후에 생기는 강성(Robertson)은 screen이 보지 못한다.

세 번째 결과가 있다(독립 리뷰 F2). probe 점 `y0 + δ_j e_j`는 궤적 위의 점이 아니다. 그 점에서 RHS가 fatal 오류를 내면 어떤 leg도 돌기 전에 solve가 그 오류로 끝난다. DOPRI5만으로는 끝까지 적분되는 문제도 그렇다(`Rodas5p::solve`의 유한차분 Jacobian도 같은 RHS에서 실패한다). 궤적이 지나지 않는 점의 fatal 오류를 'screen이 아무것도 보이지 않은 것'으로 볼지는 소유자 결정 사항이다.

| 문제 | `L` | 구간 | 기준 `3.3 B` | 증거 | RHS 호출 |
|---|---|---|---|---|---|
| `y' = -1e6 (y - cos t)` | 1e+06 | 1 | 3300 | 예 | 2 |
| 열 방정식 n=8 | 198.409 | 100 | 3300 | 예 | 9 |
| 열 방정식 n=8 | 198.409 | 1 | 3300 | 아니오 | 9 |
| Robertson | 0.632203 | 10 | 3300 | 아니오 | 4 |
| 조화 진동자 | 1 | 1 | 3300 | 아니오 | 3 |

### 3.3 수축성 강성 문제 (COD-07)

경계값은 실행 전에 정했다: DOPRI5가 안정 한계에서 할 수 있는 최소 작업(`6 ρ (t1 - t0) / 3.3`회의 RHS)의 10분의 1.

| 문제 | 실행한 driver (base → 지금) | 과금된 RHS 호출 (base → 지금) | 경계값 |
|---|---|---|---|
| 열 방정식 n=64, dense Jacobian, 구간 (0, 10) | `Tier1ExplicitDopri5` → `rodas5p-fast-transformed-v2` | 314119 → 328 | 30727 |
| `y' = -1e6 (y - cos t)`, Jacobian 있음, 구간 (0, 1) | `Tier1ExplicitDopri5`, `ProtectedRODAS5P` → `rodas5p-fast-transformed-v2` | 600049 → 48 | 181818 |
| 열 방정식 n=8, Jacobian 없음, 구간 (0, 100) | `Tier1ExplicitDopri5` → `rodas5p-fast-transformed-v2` | 58411 → 723 | 5890 |
| `y' = -1e6 (y - cos t)`, Jacobian 없음, 구간 (0, 1) | `Tier1ExplicitDopri5`, `ProtectedRODAS5P` → `rodas5p-fast-transformed-v2` | 600073 → 74 | 181818 |

### 3.4 실패 보고 (COD-06)

| 경우 | base의 `status` | 지금의 `status` |
|---|---|---|
| switching 실행 중 RHS가 치명적 오류 | `max-steps-reached` (max_steps=100000) | `callback-failed` (callback=rhs, message=sensor offline) |
| switching 실행 중 Jacobian이 치명적 오류 | `max-steps-reached` (max_steps=100000) | `callback-failed` (callback=jacobian, message=jacobian table missing) |
| switching 실행 중 해가 발산 (`y' = y^2`), 시도 예산은 남음 | `max-steps-reached` (max_steps=100000) | `step-size-too-small` (min_step=1e-14) |
| 사전 점검에서 RHS가 치명적 오류 | `Err: InvalidInput("initial RHS evaluation failed: fatal callback error: model not loaded")` | `Ok: {"status":"failed","kind":"callback-failed","callback":"rhs","t":0.0,"message":"model not loaded"}` |

`Auto`에서 DOPRI5 구간의 RHS가 치명적 오류를 낸 경우, 치명적 오류를 돌려준 RHS 호출 수(다른 방법이 다시 부르면 2): 2 → 1.

`VigilodeSolution::stop`은 멈춘 driver가 밝힌 값이다: RODAS5P는 `FailureReason`, DOPRI5는 `Dopri5StopReason`과 callback 실패와 메시지, switching adapter는 자신의 메시지와 멈춘 segment의 driver 정지 기록, 그리고 callback 경계에서 본 첫 치명적 오류. adapter는 치명적 callback의 메시지를 어느 기록에도 남기지 않으므로(`switching.rs`가 `BatchStop::Fatal`의 내용을 버린다), router는 callback을 그대로 전달하면서 첫 치명적 오류만 적어 두는 포장을 문제에 씌워 adapter에 넘긴다. adapter의 결과는 이 포장과 무관하게 bit 단위로 같다(5절).

`status`는 `stop`을 facade의 `FailureReason`으로 옮긴 것이다. 정확하지 않은 경우가 있다(`UnifiedStop::is_exact`가 구분한다): switching adapter의 마지막 segment가 DOPRI5이고 정지 이유가 callback도 시도 예산도 아닐 때. adapter가 DOPRI5의 정지 이유를 기록하지 않기 때문이며, 이때 `status`는 `StepSizeTooSmall`이고 정확한 값은 `stop`의 메시지뿐이다.

### 3.5 작업량 회계와 시도 예산

Jacobian 없는 Robertson, 구간 (0, 10), `max_steps = 500`:

| 항목 | base → 지금 |
|---|---|
| 실행한 driver | `Tier1ExplicitDopri5`, `ProtectedRODAS5P` → `dopri5-5(4)-fsal`, `rodas5p-fast-transformed-v2` |
| 모든 구간의 시도 수 (예산 500) | 516 → 266 |
| `Stats::attempts` | 16 → 266 |
| callback 경계에서 센 RHS 진입 | 3189 → 1697 |
| 과금된 RHS 호출 | 3188 → 1697 |

구간별 기록(`VigilodeSolution::legs`):

| 구간 | driver | 경로 | 시작 → 끝 | 예산 | 시도 | RHS | 정지 |
|---|---|---|---|---|---|---|---|
| 1 | `dopri5-5(4)-fsal` | `dopri5/rhs-only` | 0 → 0.345748 | 250 | 250 | 1501 | `{"callback": null, "message": "DOPRI5 attempt budget exhausted", "reason": "attempt-budget", "source": "dopri5"}` |
| 2 | `rodas5p-fast-transformed-v2` | `rodas5p/dense-fast` | 0.345748 → 10 | 250 | 16 | 192 | 없음 |

사전 점검 4회 + 구간 합 1693회 = 전체 1697회, callback 경계에서 센 진입 1697회.

저장 공간:

- `VigilodeWorkspace::new(2000, 30)`이 쥐는 byte: 32704000 → 64000
- matrix-free 경로(n = 3000) 한 번의 solve에서 가장 큰 단일 할당(byte): 72000000 → 4194304 (`n*n*8` = 72000000)

## 4. 남은 tier, 제거한 것, 아직 이름뿐인 것

**남은 tier (`SelectedTier`)**

| 값 | 진입점 | 실행 기록의 driver 식별자 |
|---|---|---|
| `Tier1Explicit` | `Dopri5::solve` | `dopri5-5(4)-fsal` |
| `TierProtectedReference` | `Rodas5p::solve` | facade가 고른 계열: `rodas5p-fast-transformed-v2`(dense), `rodas5p-fast-banded-v1`, `rodas5p-mf-fast-transformed-v1`(matrix-free), `rodas5p-fast-dense-v1`, `rodas5p-fast-banded-dense-v1`, `rodas5p-sequential-dense-v2` |
| `Tier5AdaptiveSwitching` | `SwitchingSolver::solve` | `vigilode.switching.v1`; segment별 driver는 `UnifiedLeg::segment_drivers` |

**제거한 것**

- `SelectedTier::Tier2MatrixFreeKrylov`, `Tier3Exponential`, `Tier4StagedRodas5p`. 저장소에서 이 이름을 쓰는 곳은 `src/unified` 안뿐이었다.
- `PreflightAnalysis`의 `is_stiff`, `estimated_mu2`, `has_jvp`, `has_band`, `has_mass`, `requires_dae_handling`. 근거(`basis`), registry의 선언 사실(`facts`), 계획 경로, screen 기록으로 바꿨다.
- `VigilodeWorkspace`의 `stage_vectors`, `krylov_basis`, `linear_scratch`, `error_scratch`, `scale_scratch`.
- `FallbackEngine::execute_fallback`. `FallbackEngine`은 fallback 규칙을 담은 순수 함수 세 개가 됐다.
- JVP 문제에서 같은 `Rodas5p::solve`를 한 번 더 부르던 'fallback'.
- 모듈 문서의 nilpotent homotopy sweep, GCRO-DR, exponential action, zero-allocation hot loop 문장.

**아직 이름뿐이거나 읽히지 않는 것** (문서에 그렇게 적었다)

- `VigilodeOptions::profile`(`PerformanceProfile`), `max_krylov_dim`, `max_krylov_restarts`, `invariant_drift_tol`: 저장만 되고 아무것도 읽지 않는다. 기존 테스트, 예제, prelude가 쓰고 있어 남겼다.
- `SolverStrategy::HighPerformance`: `Auto`와 똑같이 경로를 고른다.
- `VigilodeWorkspace::new`의 둘째 인자와 `step_into`(고정 간격 Heun 단계): 어떤 solver도 쓰지 않는다.
- 마지막 구간이 RODAS5P가 아닐 때 `VigilodeSolution::stats`의 driver 전용 필드(`driver_work`, `driver_counters`, `adaptive` 등)는 형식상의 빈 값이다. `Stats`가 RODAS5P 기록 형식이기 때문이다. 그 driver의 기록은 `explicit_solution`, `switching_solution`에 있다.
- `PexprbWindowDriver`의 segment 기록은 `SwitchingMethod::Rodas5p`로 적힌다. `SwitchingMethod`에 지수 적분 값이 없고 `switching.rs`는 보호 대상이다. 문서에 적었다.

## 5. 테스트

**Part 1 (base의 공개 API만 사용, 20개).** 소스를 고치기 전에 base 소스에서 먼저 실행했다 (`logs/failing_first_base.log`): 통과 0, 실패 20. 지금은 20개가 모두 통과한다.

| 테스트 | base | 지금 |
|---|---|---|
| `p1_failing_switching_solve_reports_the_fatal_jacobian_callback` | FAILED | ok |
| `p1_failing_switching_solve_reports_the_fatal_rhs_callback` | FAILED | ok |
| `p1_failing_switching_solve_with_attempts_left_is_not_reported_as_max_steps` | FAILED | ok |
| `p1_fallback_shares_one_attempt_budget_and_keeps_the_work_of_the_failed_leg` | FAILED | ok |
| `p1_fatal_callback_on_the_explicit_leg_is_not_retried_by_another_method` | FAILED | ok |
| `p1_label_equals_executed_driver_on_every_rodas5p_driver_family` | FAILED | ok |
| `p1_label_equals_executed_driver_on_the_explicit_route` | FAILED | ok |
| `p1_label_equals_executed_driver_on_the_switching_adapter` | FAILED | ok |
| `p1_matrix_free_solve_allocates_no_dense_matrix` | FAILED | ok |
| `p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input` | FAILED | ok |
| `p1_preflight_callbacks_are_charged_with_a_dense_jacobian` | FAILED | ok |
| `p1_preflight_callbacks_are_charged_with_a_jvp` | FAILED | ok |
| `p1_preflight_callbacks_are_charged_without_a_jacobian` | FAILED | ok |
| `p1_rodas5p_leg_is_the_direct_call_bit_for_bit` | FAILED | ok |
| `p1_stiff_contractive_heat_equation_with_dense_jacobian_is_not_explicit` | FAILED | ok |
| `p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit` | FAILED | ok |
| `p1_stiff_contractive_scalar_with_dense_jacobian_is_not_explicit` | FAILED | ok |
| `p1_stiff_contractive_scalar_without_jacobian_is_not_explicit` | FAILED | ok |
| `p1_switching_leg_keeps_the_full_work_of_its_segments` | FAILED | ok |
| `p1_workspace_storage_is_linear_in_the_dimension` | FAILED | ok |

label과 driver의 일치: 사례 11개 중 base에서 불일치 11개, 지금 0개. 기준 값은 같은 문제에 대해 driver를 직접 불러 얻은 식별자다.

**Part 2, 3 (이 node가 추가한 API, 16개).** 구현 뒤에 썼다. base에는 없는 API를 쓰므로 base 소스 사본에서는 compile이 실패한다(`logs/failing_first_new_api.log`, exit 101, 오류 코드별 개수: `E0432` 2, `E0609` 76, `E0599` 23, `E0282` 1, `E0614` 1, `E0004` 1). 실행으로 실패를 확인한 것은 part 1뿐이다.

**재현 격자.** 요청 336개(공급자 7종 × 전략 4종 × regime 3종 × dense output 여부 × 차원 2개)에 대해, solution이 보고한 구간을 그 구간이 이름 붙인 진입점의 직접 호출로 다시 실행했다.

- 실행 312개, callback 전에 거절 24개, 불일치 0개.
- 다시 실행한 최종 상태는 bit 단위로 같고, callback 진입 횟수(RHS, Jacobian, JVP)는 사전 점검 호출을 더하면 같다.
- tier별 건수: `Tier1Explicit` 30, `Tier5AdaptiveSwitching` 78, `TierProtectedReference` 204
- driver별 구간 수: `dopri5-5(4)-fsal` 30, `rodas5p-fast-banded-dense-v1` 72, `rodas5p-fast-banded-v1` 42, `rodas5p-fast-dense-v1` 72, `rodas5p-fast-transformed-v2` 26, `rodas5p-mf-fast-transformed-v1` 28, `vigilode.switching.v1` 42
- 경로별 구간 수: `dopri5/rhs-only` 30, `rodas5p/banded-fast` 42, `rodas5p/dense-fast` 26, `rodas5p/fast-banded-dense` 72, `rodas5p/fast-dense` 72, `rodas5p/matrix-free` 28, `switching/dopri5` 39, `switching/dopri5+rodas5p` 3

**switching adapter의 불변성.** base API만 쓰는 모듈을 base 소스 사본과 이 node에서 각각 실행해 `SwitchingSolver::solve`와 `ProductionSolver::solve`의 전체 기록(JSON)의 길이와 FNV-1a 64 값을 비교했다: 77 / 77줄 동일 (문제 11개 × 설정 7개; 치명적 callback, 발산, 예산 소진, driver 거절 경로 포함).

## 6. `registry.rs` (COD-06의 registry 항목)

- `WindowDriver::run`은 `Result<BatchOutcome, WindowDriverError>`를 돌려준다. facade, DOPRI5, core의 오류가 형식을 유지한다. `Display`와 `String` 변환은 base가 내던 문자열 그대로이므로 오류를 글로 읽는 switching adapter는 같은 메시지를 받는다.
- `BatchOutcome::failure`가 driver의 정지 이유를 담는다(RODAS5P의 `FailureReason`, DOPRI5의 정지 이유와 callback 실패).
- `BatchStop::Fatal`의 문자열은 고정 문구 `callback failure` 대신 callback의 메시지다.
- `BatchStop`의 값 5개는 그대로다. `switching.rs`가 wildcard 없이 match하므로 값을 추가하면 보호 파일이 compile되지 않는다. 그래서 최소 간격에서 멈춘 RODAS5P 실행은 여전히 `BatchStop::Completed`로 분류되고(문서에 적었다), 정지 이유는 `failure`에 있다.

고치지 않은 것: `work_units`가 분해 횟수를 빼는 것(고치면 `SwitchingSolver`의 결정이 바뀐다), `PexprbWindowDriver`가 recoverable 오류도 치명적으로 처리하는 것, 호출자가 없는 `SolverState`/`next_solver_state`.

## 7. 기존 파일 편집

**`crates/vigilode/tests/unified_production_solver_contracts.rs`** (+23 / -11줄)

- contract_3: `assert_eq!(first_tier, "Tier1ExplicitDopri5")`는 label 문자열을 고정했다. 이제 history 항목을 DOPRI5 구간의 `ExplicitStats::method`와 비교한다.
- contract_3: `contains("Protected") || contains("Staged")`도 label 문자열이었다. 이제 `Stats::route.driver_id`와 비교한다.
- contract_3: `assert_eq!(heat_tier, "Tier2MatrixFreeKrylov")`는 driver가 없던 tier의 label이었다. 이제 `Stats::route.driver_id`와 비교하고 `DriverWork::MatrixFree`를 확인한다.
- contract_4: `to_tier == "ProtectedRODAS5P"`는 label 문자열이었다. 이제 DOPRI5 구간 뒤에 자기 실행 기록의 이름을 가진 RODAS5P 구간이 오는지 본다.
- 이 단정문들 옆의 주석 두 개가 제거된 label을 이름으로 들고 있었다.

**`crates/vigilode/examples/unified_production_solver.rs`** (+16 / -9줄)

- 머리말 목록: multi-tier routing, transactional fallback, stepping hot loop의 zero heap allocation(VG-1)을 내세웠다. 예제가 실제로 보여 주는 것으로 바꿨다.
- 문제 4의 주석: 'stiffness rejection을 만나 smoothly rolls back'은 없는 감지 기능을 말했다. trial 예산을 적었다.

편집 전 파일을 새 router에 돌리면 6개 중 2개만 실패했고(`contract_3_autonomous_tier_routing`, `contract_4_fail_closed_protected_fallback`), 둘 다 label 문자열 비교에서였다 (`logs/existing_unified_tests_before_edit.log`).

이 테스트 파일의 머리말과 `contract_2`의 주석에는 이전 용어(multi-tier, VG-1)가 남아 있다. 단정문이 아니어서 손대지 않았다.

## 8. 검증 명령

| 로그 | exit | 테스트 (통과/실패/무시) | 명령 |
|---|---|---|---|
| `logs/contracts.log` | 0 | 36 / 0 / 0 | `taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p vigilode --test unified_router_contracts -- --nocapture --test-threads 1` |
| `logs/existing_unified_and_registry_tests.log` | 0 | 86 / 0 / 0 | `taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p vigilode --test unified_production_solver_contracts --test unified_frontend_contracts --test registry_contracts --test switcher_v2_admission_contracts --test switching_economy_contracts --test production_hotloop_allocation_contracts --test sparse_jvp_contracts --test switching_v2_remediation_contracts` |
| `logs/default_path_golden.log` | 0 | 11 / 0 / 0 | `taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p vigilode --test default_path_golden -- --nocapture --test-threads 1` |
| `logs/full_suite.log` | 0 | 533 / 0 / 0 | `taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p vigilode --no-fail-fast` |
| `logs/clippy.log` | 0 |  | `taskset -c 0-5,12-17 cargo +1.94.1 clippy --locked -j 4 --release -p vigilode --all-targets --no-deps -- -D warnings` |
| `logs/fmt_check.log` | 0 |  | `taskset -c 0-5,12-17 cargo +1.94.1 fmt -p vigilode -- --check` |
| `logs/doc.log` | 0 |  | `env RUSTDOCFLAGS=-D warnings taskset -c 0-5,12-17 cargo +1.94.1 doc -j 4 -p vigilode --no-deps --locked` |
| `logs/example.log` | 0 |  | `taskset -c 0-5,12-17 cargo +1.94.1 run --locked -j 4 --release -p vigilode --example unified_production_solver` |
| `logs/check_checkpoint.log` | 1 |  | `python3 -I research/production_loop_20261116/check_checkpoint.py` |
| `logs/changed_paths.log` | 0 |  | `git diff --name-status 587a139501729bbe886bb2b8c49b0ef893705d08 HEAD` |
| `logs/golden_diff.log` | 0 |  | `git diff --stat 587a139501729bbe886bb2b8c49b0ef893705d08 HEAD -- crates/vigilode/tests/golden crates/vigilode/tests/default_path_golden.rs` |
| `logs/protected_diff.log` | 0 |  | `git diff --stat 587a139501729bbe886bb2b8c49b0ef893705d08 HEAD -- fixtures/rodas5p_coefficients_snapshot.json crates/rodas5p-core crates/rodas5p-krylov crates/rodas5p-integrators crates/vigilode/src/lib.rs crates/vigilode/src/automatic.rs crates/vigilode/src/switching.rs crates/vigilode/src/selection.rs crates/vigilode/src/selection crates/vigilode/src/solve.rs crates/vigilode/src/explicit.rs crates/vigilode/src/production.rs crates/vigilode/src/prelude.rs crates/vigilode/Cargo.toml Cargo.toml Cargo.lock rust-toolchain.toml research/LEDGER.jsonl docs/agent research/production_loop_20261116/PREREGISTRATION.md research/production_loop_20261116/PROGRESS.md research/production_loop_20261116/CHECKPOINT.json` |

loop checker 출력 (exit 1):

```
Traceback (most recent call last):
  File "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w2-v03b/research/production_loop_20261116/check_checkpoint.py", line 95, in <module>
    main()
  File "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w2-v03b/research/production_loop_20261116/check_checkpoint.py", line 89, in main
    require(any(fnmatch.fnmatchcase(path, pattern) for pattern in allowed),
  File "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w2-v03b/research/production_loop_20261116/check_checkpoint.py", line 28, in require
    raise ValueError(message)
ValueError: changed path outside active allowlist: crates/vigilode/examples/unified_production_solver.rs
```

checker는 프로그램 allowlist 밖의 첫 경로에서 멈춘다. 그 앞의 검사(엄격한 JSON, task/DAG 구조, 조상 관계, 보호 blob, 증거와 구현 소스의 동일성)는 통과했다. 마지막 반복문을 모든 경로에 대해 다시 돌리면 allowlist 밖의 경로는 12개이고, 모두 이 과제가 명시적으로 허용한 경로인가: 예.

- `crates/vigilode/examples/unified_production_solver.rs`
- `crates/vigilode/src/registry.rs`
- `crates/vigilode/src/unified/analyzer.rs`
- `crates/vigilode/src/unified/fallback.rs`
- `crates/vigilode/src/unified/legs.rs`
- `crates/vigilode/src/unified/mod.rs`
- `crates/vigilode/src/unified/options.rs`
- `crates/vigilode/src/unified/router.rs`
- `crates/vigilode/src/unified/solution.rs`
- `crates/vigilode/src/unified/workspace.rs`
- `crates/vigilode/tests/unified_production_solver_contracts.rs`
- `crates/vigilode/tests/unified_router_contracts.rs`

바꾼 경로 35개 가운데 과제의 허용 경로 밖: 없음. 보호 경로의 base 대비 diff: 0줄.

## 9. 열린 문제와 한계

1. **동작이 달라진 경우.** (a) Jacobian을 준 비강성 문제는 `Auto`에서 이제 RODAS5P로 간다(base는 μ2 ≤ 0이면 DOPRI5). DOPRI5를 원하면 `regime(Nonstiff)`를 선언한다. (b) 아무 선언이 없는 작은 문제의 DOPRI5 trial은 최대 1000회다(base는 `max_steps` 전부). 1000회를 넘는 긴 비강성 적분은 남은 구간을 RODAS5P(유한차분 Jacobian)로 끝낸다. 결과는 오차 제어를 받지만 더 비쌀 수 있다. 1000이라는 값과 차원 한계 10은 측정으로 정한 값이 아니다.
2. **screen의 범위.** `t0`의 Lipschitz 하계만 본다. 진동이 빠르거나 비정규성이 큰 비강성 문제를 RODAS5P로 보낼 수 있고, 나중에 생기는 강성은 trial 예산이 막는다.
3. **유한차분 Jacobian의 허가.** router는 `GeneralOdePolicy`와 달리 유한차분을 따로 허가받지 않고 쓴다(`Rodas5p::solve`가 Jacobian 없는 문제에 하는 것과 같다). switching adapter에도 `AllowFiniteDifference`를 준다. 허가 옵션을 추가하지 않았다.
4. **`status`의 근사.** 3.4절의 경우에는 `status`가 정확하지 않다. `switching.rs`와 `SwitchingSolution`을 고쳐야 없어진다.
5. **fallback 구간이 거절되는 경로**(앞 구간은 적분했는데 RODAS5P가 입력을 거절)는 구현했지만(`UnifiedStop::Refused`, 앞 구간 유지), 지금의 경로 규칙으로는 공개 API에서 이 상황을 만들 수 없어 테스트가 없다.
6. **screen 뒤의 거절.** screen이 RHS를 부른 뒤 두 진입점이 모두 적분 전에 오류를 돌려주면 `Err`가 반환되고 screen의 호출 수는 보고되지 않는다. screen 전에 DOPRI5의 입력 검증과 RODAS5P의 승인을 통과시키므로, 기록을 남기지 않는 driver 내부 오류일 때만 생길 수 있다. 테스트는 없다.
7. **registry를 거치는 비용.** 한 번의 solve에 method catalog를 여러 번 만든다(`MethodRegistry::route`, `solve`). 시간은 재지 않았다.
8. **V03 part A에서 넘어온 것은 그대로다.** 일반 sparse-JVP 문제의 sequential 경로 계획은 여전히 실행되지 않는다(`Rodas5p::capabilities`). 이 node의 격자에는 그 조합이 없다.

