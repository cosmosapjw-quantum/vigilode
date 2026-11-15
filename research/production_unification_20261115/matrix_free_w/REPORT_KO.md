# U04 PPSW1 matrix-free common-W — 2026-11-15

판정: **U04 PARTIAL**. 구현·계약·잔차 예산·측정은 했지만, 측정 범위에서 **crossover가 없었다**(matrix-free가 dense보다 빠른 n 없음). production **HOLD**, 기본 라우팅·G0·허용오차·계수 불변. 사전 계약은 `CHANGE_CONTRACT.json`(M1–M4), 근거는 `RESULTS.json`과 `logs/`다.

## 구현 (기존 부품 재사용)
- `ppsw1::linear_backend` (`src/ppsw1_linear_backend.rs`를 `ppsw1.rs`에서 `#[path]`로 선언, `lib.rs` 무변경).
  - W = `ShiftedOperator::new_counted_jvp(None, problem.linearize_matrix_free(t_m, y_m), h, γ)`. J는 step 시작에서 고정되고 stage 값에서 다시 선형화하지 않는다.
  - transfer 항 `h T Σ Γ_ij Y_prev,j`는 stage마다 JVP 1회로 계산하고 계수한다.
  - 풀이: 독립 GMRES 6개(production `solve_gmres`, stage 병렬, 모든 job join 후 계수) 또는 block GMRES 1개(기존 rank deflation). preconditioner는 identity뿐이다.
  - 모든 stage의 원 잔차 `||b_i − W x_i||`를 W 1회 추가 적용으로 다시 계산한다. `slack·max(atol, rtol||b||)`를 넘으면 typed `LinearSolve`다.
- `advance_with_backend`, `Session::with_linear_backend`(기본 Dense, 기존 동작·counter 불변), `last_linear_report`.
- `start_with_policy`: dense 또는 Newton을 금지하면 callback 전에 InvalidInput을 반환한다. matrix-free starter는 없다.
- M4 `LINEAR_BUDGET_RATIO = 1e-2`: `try_step`이 선형 rtol > 1e-2·step rtol을 callback 전에 거부한다. 근거와 유도는 `BUDGET_KO.md`.

## 계약 테스트 (`ppsw1_linear_backend_contracts.rs`, 5 통과 + 1 timing ignored)
- c1 parity: 대각 stiff n=30, 비정규 bidiagonal n=40, rank-1 RHS n=40에서 dense 대비 최대 상대 차이 ≤4.3e-14, 원 잔차 기록, Jacobian build/LU 0회. block은 rank-1 RHS에서 initial rank 1을 검출했다. stage 병렬(6 thread)은 순차와 bit·counter가 동일하다.
- c2 실패: GMRES 고갈 → `Err(LinearSolve)`, state 보존, JVP 작업 계수(순차 4, 병렬 24 — 병렬은 6 job 모두 실행). NaN JVP → NonFinite → Rejected, state 보존.
- c3 권한: JVP 없는 문제는 Session/advance에서 callback 전에 거부(counter 0). starter dense/Newton 금지는 callback 전에 거부. 기본 정책은 `start`와 동일하다.
- c4 예산: `BUDGET_KO.md` 표. c6 가드: 비율 1, 0.1 거부(counter·state 무변경), 0.01 이하 허용, Dense는 가드 대상이 아니다.
- 회귀: library 123(+1 ignored), Peer 37, U03 qualification 18, 병렬 5, 경계 6 = 189 통과. fmt 통과, 진단 clippy 경고는 기존 8개뿐(변경 파일 0).

## 측정된 crossover (c5, Fisher MOL, tol 1e-6, 선형 rtol 1e-9)
2 warmup + 7 반복을 arm 순서를 바꿔 가며 interleave했고 median을 썼다. AMD Ryzen 9 5900X(24 thread), 측정 시작 시 load average 5.5–6.6(공유 호스트)이다. wall time은 이 조건의 측정값이고 일반화하지 않는다. RSS는 NOT_RUN이며 저장량은 모델값(dense 3n², MF 6·61·n f64)만 기록했다.

| n | dense median s | 최선 admissible MF arm | dense 대비 속도 |
|---|---|---|---|
| 25 | 0.0077 | block | 0.27 |
| 50 | 0.019 | 독립 GMRES restart 10, 6 thread | 0.33 |
| 100 | 0.061 | 독립 restart 10, 6 thread | 0.58 |
| 200 | 0.29 | 독립 restart 10, 6 thread | 0.85 |
| 400 | 1.60 | 독립 restart 10, 6 thread | 0.64 |
| 800 | 11.9 | 없음: 모든 MF arm이 `LinearSolve`(GMRES 600 Arnoldi 고갈)로 t≈0.05–0.48에서 실패 | – |

결론: **n ≤ 400에서 crossover 없음**. n=800에서는 unpreconditioned Krylov가 실패한다. 실패한 run의 wall time(>1배로 보이는 값)은 속도 주장이 아니다.

## 새로 발견한 점
- **B1 예산 일반화 실패**: restart 10은 n=400에서 ρ=1e-3(가드 통과)인데도 step 수가 670 → 9647, 거절 257회로 바뀌었고 끝점이 Dense와 0.088 tol-단위 달랐다. n=200도 569 → 894. n=40 근거의 ρ=1e-2 가드만으로는 controller 보존이 보장되지 않는다. 원인(truncated GMRES 잔차의 구조가 추정기에 미치는 영향)은 미해결이다.
- **B2 kernel 성질**: production `solve_gmres`는 restart cycle 사이에서만 수렴을 검사하므로 restart ≥ n이면 rtol과 무관하게 n회 반복한다. restart 60은 n≤40에서 사실상 직접 풀이다.
- **B3 preconditioner 부재**: Fisher의 W 조건수는 ~γh·4/dx²로 커진다. identity preconditioner로는 n=800에서 실패한다. preconditioner 실패 계약은 preconditioner가 없어 NOT_APPLICABLE이다.

## capability 판정
| 성질 | per-step (MF backend) | startup | 전체 실행 |
|---|---|---|---|
| matrix-free | 예(J/W 행렬 없음) | 아니오(5n×5n dense LU, h0 규칙의 dense J) | 아니오 |
| Newton-free | 예 | 아니오(simplified Newton) | 아니오 |
| Jacobian-free | JVP 사용(모델 제공 도함수) | 아니오 | 아니오 |
| derivative-free | 아니오 | 아니오 | 아니오 |

fallback은 없다. 실패는 typed error 또는 Rejected다.

## 남은 의무
SPD/대각 preconditioner와 그 실패 계약, B1 원인 분석과 n·restart 의존 예산, CPU time·isolated-process RSS, 다른 문제(비정규·비선형 stiff), 독립 matched-error 참조(현재는 Dense와의 차이만), matrix-free starter(별도 qualification 필요). 범위 제한 결과를 속도·production 주장으로 승격하지 않는다.
