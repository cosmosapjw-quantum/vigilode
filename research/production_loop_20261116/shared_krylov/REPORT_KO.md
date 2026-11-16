# V06 공유 Krylov basis: 실측 보고

- node `V06_SHARED_KRYLOV_BASIS`, loop L1 wave 2, 2026-11-17 (KST)
- 사전등록: `PREDECLARED.json` (sha256 `29b17919f670c3a0947605ef647e0d1fe76813500858c4ae3677abf849b80b93`), 첫 측정 전에 commit
- stage a 측정 commit `552b0b60dec8381fd349013e43708b20f43f1a8b` (추적 파일 변경 0건), binary sha256 `7ed0b9bf43331c35b78dda02086bfa5a731caf51e3d1ef729dcbf545182d725f`
- 환경: AMD Ryzen 9 5900X 12-Core Processor, rustc 1.94.1 (e408947bf 2026-03-25), perf version 7.0.14, taskset 0-9,12-21
- 이 문서의 수치는 전부 `make_report.py`가 `RESULTS.json`에서 읽어 채웠다. `RESULTS.json`은 `parse_results.py`가 `logs/`의 실행 출력만으로 만든다.
- 탐색 실행: commit `2ea52c2c40bac7006ae5648efb90334f78c5bbb3`, binary sha256 `7b93186bf179bfbc98a05d71296f4efc058887ccb8f2937e050f37936e5dc785`. 일부 그룹 시작 시점에 추적 파일 변경이 0/2건 있었다(`parse_results.py`와 계약 테스트 파일로, 측정 binary에 들어가지 않는다; binary 해시가 그룹 간에 같다).

## 1. 결론

- **Stage a: FAIL.** frozen 기준(JVP 비 ≥ 1.5, step 비 ≤ 1.05)을 두 tolerance 모두에서 만족한 family는 3개 중 1개다. 필요한 수는 2개.
- family별 JVP 비(두 tolerance 중 작은 값)의 중앙값은 1.454. 1.2 미만이면 v2를 취소한다는 조항은 발동하지 않았다.
- stage a를 통과하지 못했으므로 **v2 kernel(`shared_basis_v2.rs`)은 만들지 않았다.** 최종 판별식은 **NOT_RUN**이다.
- 원 잔차 감사: 선언 실행 8,704건의 stage solve 중 실패 0건 (탐색 실행 포함 42,022건 중 0건). 잔차/허용치의 최댓값은 0.999997.
- instructions: 병합 kernel을 쓰는 C는 B의 64~376배다. 병합 kernel이 basis 벡터를 하나 더할 때마다 n×k 행렬을 처음부터 다시 분해하기 때문이다.
- 판정에 쓰지 않는 탐색 결과 가운데 결정에 중요한 것은 8.3절(선형 허용치)과 8.4절(병합 hook)이다. 그 자료에서 JVP 수를 좌우하는 것은 basis 공유가 아니라 선형 stopping test다.
- 사전등록 수정: 0건.

## 2. 무엇을 쟀나

세 arm이 같은 선형 stopping test를 쓴다. 보호된 matrix-free driver의 규칙이다: scaled 계 `(I - h gamma J) U = rhs`에서 **true residual**(새 JVP 한 번으로 계산)의 2-norm이 `max(gamma*atol, rtol*||rhs||_2)` 이하일 때만 해를 받는다. 값은 routed production 경로의 `rtol = 1e-10`, `atol = 1e-14`, restart 40, stage solve당 Arnoldi 벡터 200개 한도.

- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2785 (operator I - h gamma J of the scaled U form)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2787-2791 (linear_atol = raw_absolute_residual_budget(config.atol, gamma) = gamma * atol)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:2924-2929 (GmresConfig: restart, max_arnoldi = max(maxiter, restart), rtol, atol = linear_atol)`
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:3016-3031 (solve_gmres_into_with_accounting, residual_scale = None, i.e. the 2-norm)`
- `crates/rodas5p-krylov/src/gmres_into.rs:268-269 (threshold from the right-hand side norm)`
- `crates/rodas5p-krylov/src/common.rs:40-41 (threshold = max(atol, rtol * ||rhs||))`
- `crates/rodas5p-krylov/src/gmres_into.rs:278-306 (true residual at the top of every restart pass; the loop leaves only when it meets the threshold)`
- `crates/rodas5p-krylov/src/gmres_into.rs:307-312 (budget exhaustion is CoreError::LinearSolve)`
- `crates/rodas5p-krylov/src/gmres_into.rs:467-474 (a restart pass runs all its columns; only a happy breakdown ends it early)`
- `crates/rodas5p-krylov/src/gmres_into.rs:372-390 (final diagnostic true residual under the default accounting)`
- `crates/rodas5p-integrators/src/routing.rs:233-251 (RoutingOptions::default().matrix_free, the configuration of the routed production matrix-free path, called at routing.rs:501-506)`

| arm | 내용 |
|---|---|
| A | 보호 driver 그대로 (`integrate_rodas5p_mf_fast_observed_gmres_into`). 잔차 검사는 40열 restart pass가 끝날 때만 한다. |
| B0 | 대조용. client driver + per-stage GMRES, A와 같은 검사 주기. client 경로가 A를 재현하는지 본다. |
| B | per-stage GMRES client (frozen 기준의 기준선). 매 Arnoldi 열마다 projected residual을 보고, 기준에 닿으면 true residual로 확인한다. |
| C | shared-basis client. attempt마다 basis 하나(최대 200개)를 `rodas5p_krylov::shared_basis::SharedBasis`로 8개 solve가 공유한다. kernel이 수렴을 보고하면 true residual로 확인하고, 못 끝낸 solve는 B의 GMRES로 처음부터 푼다(병합 hook과 같은 fallback). |

- 확인용 true residual JVP(solve당 1회)는 B와 C 모두 arm의 JVP에 넣었다. 감사용 JVP는 별도 counter에 있고 arm 합계에 넣지 않았다.
- JVP 비 = JVP(B)/JVP(C), step 비 = attempts(C)/attempts(B). family는 두 tolerance 모두 통과해야 통과다 (`PREDECLARED.json`).
- 병합 kernel의 수렴 판정식은 `h_gamma*||r||_w <= theta` 하나뿐이라, client가 균일 weight `sqrt(n)/(h_gamma*tau)`와 `theta = 1`로 보호 규칙의 임계값을 넣었다. kernel 파일은 수정하지 않았다.

| family | n | 구간 | 문제 | 기준해 불확도 (max abs) | DOP853 RHS 평가 수 |
|---|---|---|---|---|---|
| `bruss1d` | 1024 | [0, 10] | 1D Brusselator, method of lines | 8.73e-13 | 10,625 |
| `grayscott2d` | 2048 | [0, 20] | 2D Gray-Scott on the periodic unit square as built by rodas5p_integrators | 4.25e-11 | 905 |
| `adr2d` | 1024 | [0, 0.1] | 2D advection-diffusion-reaction, scalar, nonnormal Jacobian | 2.20e-15 | 1,673 |

- 기준해: 각 family를 numpy로 다시 구현해 scipy DOP853(rtol 1e-13)으로 적분했다. 불확도는 scipy Radau(rtol 1e-10) 결과와의 성분별 최대 차이다. Rust 코드와 선형 solver를 공유하지 않는다.
- 최종 시험용으로 선언한 네 번째 family `robertson_rd1d`는 기준해만 만들어 두었고 실행하지 않았다(최종 시험이 NOT_RUN).
- 실행 수: 선언 실행 54개, 탐색 실행 80개. 실행 하나가 프로세스 하나이고 `perf stat -e instructions:u` 아래에서 돌았다. counter는 결정적이므로 반복은 1회, wall time은 `diagnostic_only`.
- 모든 실행이 끝까지 적분했다.

## 3. frozen 판별식: stage a

| family | rtol | attempts B | attempts C | JVP B | JVP C | JVP 비 | step 비 | 감사 통과 B / C | 판정 |
|---|---|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | 64 | 64 | 16,412 | 14,631 | **1.122** | 1.000 | 예 / 예 | 미달 |
| `bruss1d` | 1e-6 | 110 | 110 | 17,472 | 13,698 | **1.276** | 1.000 | 예 / 예 | 미달 |
| `grayscott2d` | 1e-4 | 14 | 14 | 1,719 | 964 | **1.783** | 1.000 | 예 / 예 | 통과 |
| `grayscott2d` | 1e-6 | 18 | 18 | 2,065 | 1,215 | **1.700** | 1.000 | 예 / 예 | 통과 |
| `adr2d` | 1e-4 | 21 | 21 | 2,461 | 1,693 | **1.454** | 1.000 | 예 / 예 | 미달 |
| `adr2d` | 1e-6 | 45 | 45 | 3,851 | 2,574 | **1.496** | 1.000 | 예 / 예 | 미달 |

| family | JVP 비 (1e-4) | JVP 비 (1e-6) | family 값 (작은 쪽) | 판정 |
|---|---|---|---|---|
| `bruss1d` | 1.122 | 1.276 | 1.122 | 미달 |
| `grayscott2d` | 1.783 | 1.700 | 1.700 | 통과 |
| `adr2d` | 1.454 | 1.496 | 1.454 | 미달 |

- 통과 family 1개 < 2개: **FAIL**.
- 중앙값 1.454 (6개 cell의 중앙값은 1.475, 참고용).
- 여섯 cell 모두 step 비는 정확히 1이고, C와 B의 달성 오차 비는 0.999997~1.000005이다. 두 arm은 같은 궤적을 따른다.

### 계수 방식에 대한 민감도 (판정에 쓰지 않음)

사전등록한 JVP에는 solve당 1회의 확인용 true residual이 들어 있다(두 arm 모두). 이를 빼고 Krylov 벡터만 세면:

| family | rtol | JVP 비 (사전등록) | Krylov 벡터 비 (탐색) |
|---|---|---|---|
| `bruss1d` | 1e-4 | 1.122 | 1.121 |
| `bruss1d` | 1e-6 | 1.276 | 1.293 |
| `grayscott2d` | 1e-4 | 1.783 | 1.868 |
| `grayscott2d` | 1e-6 | 1.700 | 1.786 |
| `adr2d` | 1e-4 | 1.454 | 1.504 |
| `adr2d` | 1e-6 | 1.496 | 1.577 |

- Krylov 벡터만 세면 1.5 이상을 두 tolerance에서 넘는 family가 2개가 된다. `adr2d`는 사전등록 계수로 1.454, 1.496이고 Krylov 벡터만 세면 1.504, 1.577이다. 판정은 사전등록대로 FAIL이지만, `adr2d`의 통과 여부는 이 계수 선택에 달려 있다는 점을 밝힌다.
- 어느 계수로도 최종 판별식의 JVP 조건(B의 0.5배 이하, 즉 비 2.0 이상)에 닿는 family는 없다. v2는 벡터 하나의 비용을 줄이는 것이지 벡터 수를 줄이지 않으므로(기각 후 재시도 제외, 8.2절), stage a를 통과로 읽더라도 최종 PASS는 이 자료로는 기대할 수 없다. 이것은 측정이 아니라 추론이다.
- 최종 판별식의 instructions 조건(B의 0.8배 이하)도 같은 방향이다. V05의 비용 모형(`theory/THEORY.md` S7)에서 basis 하나의 직교화 작업은 basis 여덟 개의 `8*theta^2`배이고, `theta`는 C와 B의 Krylov 벡터 수의 비다. 측정된 `theta`는 0.535~0.892로 모두 `1/sqrt(8) = 0.354`보다 크므로 모형상 직교화 작업은 2.3~6.4배가 된다. 이 family들은 JVP가 stencil 한 번이라 싸다. 모형에 측정값을 넣은 추론이며 v2의 instructions를 잰 것이 아니다.

## 4. B는 A의 공정한 대역인가

| family | rtol | attempts A / B0 / B | JVP A / B0 / B | B0 counter = A | 상태 차 B0-A | JVP B/A | 달성 오차 A / B / C (허용치 단위) |
|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | 64 / 64 / 64 | 27,285 / 27,285 / 16,412 | 예 | 4.1e-14 | 0.602 | 0.3430 / 0.3430 / 0.3430 |
| `bruss1d` | 1e-6 | 110 / 110 / 110 | 37,616 / 37,616 / 17,472 | 예 | 4.5e-14 | 0.464 | 3.0324 / 3.0324 / 3.0324 |
| `grayscott2d` | 1e-4 | 14 / 14 / 14 | 4,540 / 4,540 / 1,719 | 예 | 3.0e-14 | 0.379 | 0.1079 / 0.1079 / 0.1079 |
| `grayscott2d` | 1e-6 | 18 / 18 / 18 | 5,597 / 5,597 / 2,065 | 예 | 2.6e-15 | 0.369 | 0.3693 / 0.3693 / 0.3693 |
| `adr2d` | 1e-4 | 21 / 21 / 21 | 6,933 / 6,933 / 2,461 | 예 | 1.1e-15 | 0.355 | 0.1891 / 0.1891 / 0.1891 |
| `adr2d` | 1e-6 | 45 / 45 / 45 | 14,997 / 14,997 / 3,851 | 예 | 6.9e-16 | 0.257 | 0.3186 / 0.3186 / 0.3186 |

- B0(같은 검사 주기)는 여섯 cell 중 6개에서 A와 JVP 수, Arnoldi 반복 수, matvec 수, 진단 matvec 수, RHS 호출 수가 모두 같다. 최종 상태는 반올림 수준으로만 다르다(두 driver가 stage 우변을 scaled/unscaled 형태로 다르게 만든다).
- B는 여섯 cell 중 6개에서 A와 attempt 수가 같고, 달성 오차 비 B/A는 0.999997~1.000006이다. JVP가 적은 것은 pass 중간에 멈출 수 있어서다. A를 기준선으로 쓰면 C의 이득에 이 효과가 섞이므로 frozen 비교의 기준선은 B다.
- 달성 오차는 기준해 대비 `max_i |y_i - ref_i| / (atol + rtol |ref_i|)`.

## 5. Krylov 차원, basis 메모리, instructions

| family | rtol | B: solve당 벡터 평균 / 최대 | C: attempt 끝 basis 차원 평균 / 최대 | C fallback solve | 저장 MB (B / C) | instructions B | instructions C | C/B |
|---|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | 30.8 / 83 | 156.4 / 200 | 73 | 0.38 / 4.96 | 5.08e+09 | 1.91e+12 | 376 |
| `bruss1d` | 1e-6 | 18.8 / 53 | 113.9 / 200 | 6 | 0.38 / 4.96 | 4.38e+09 | 1.44e+12 | 329 |
| `grayscott2d` | 1e-4 | 14.4 / 47 | 61.6 / 184 | 0 | 0.75 / 9.91 | 1.16e+09 | 2.18e+11 | 189 |
| `grayscott2d` | 1e-6 | 13.4 / 34 | 60.1 / 125 | 0 | 0.75 / 9.91 | 1.20e+09 | 1.53e+11 | 128 |
| `adr2d` | 1e-4 | 13.7 / 25 | 72.7 / 123 | 0 | 0.38 / 4.96 | 6.12e+08 | 9.50e+10 | 155 |
| `adr2d` | 1e-6 | 9.7 / 14 | 49.2 / 72 | 0 | 0.38 / 4.96 | 7.83e+08 | 5.02e+10 | 64 |

- B의 저장은 GMRES basis 41개 벡터와 작업 벡터, C의 저장은 병합 kernel의 Q, Z, 분해용 행렬(각 n×200)이다. C는 fallback용 B 저장도 함께 갖는다.
- instructions는 감사를 끈 실행의 프로세스 전체 값이다. 문제 구성만 하는 SETUP 실행을 빼도 비는 같다(`RESULTS.json`).
- `bruss1d`에서는 basis가 한도 200에 닿아 fallback이 생겼다(8.2절의 C300 참조).
- 감사를 켠 실행과 끈 실행은 여섯 cell 중 6개에서 B, C 모두 counter, 통계, 최종 상태가 비트 단위로 같다 (같은 arm을 두 번 돌린 것이기도 하다).
- 할당 검사(`stage_clients_krylov_alloc`, 단독 binary): client 구성 9가지, 실행 54개, warm-up 뒤 accepted step 1,728개에서 step당 할당 최대 0회(합계 0). fallback이 일어난 실행 6개 포함. 작은 개발용 문제(n = 64~128)에서 잰 것이다.

## 6. 이득이 작은 이유: stage별 분해

attempt당 새 Krylov 벡터 수를 solve 순서(stage 1~8)별로 평균한 값이다.

| family | rtol | arm | s1 | s2 | s3 | s4 | s5 | s6 | s7 | s8 |
|---|---|---|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | B | 27.7 | 30.6 | 28.1 | 29.1 | 27.2 | 34.9 | 33.4 | 35.2 |
| `bruss1d` | 1e-4 | C | 27.4 | 29.4 | 20.6 | 24.4 | 21.8 | 28.8 | 32.4 | 34.8 |
| `bruss1d` | 1e-6 | B | 16.3 | 18.0 | 16.3 | 17.0 | 15.9 | 21.5 | 22.0 | 23.8 |
| `bruss1d` | 1e-6 | C | 16.4 | 17.6 | 10.9 | 12.7 | 8.8 | 12.3 | 17.1 | 20.7 |
| `grayscott2d` | 1e-4 | B | 14.3 | 14.6 | 14.1 | 14.4 | 13.8 | 15.1 | 14.6 | 14.1 |
| `grayscott2d` | 1e-4 | C | 14.2 | 10.6 | 6.6 | 6.6 | 4.7 | 5.0 | 6.6 | 7.2 |
| `grayscott2d` | 1e-6 | B | 13.1 | 13.3 | 12.7 | 13.1 | 12.4 | 14.3 | 14.3 | 14.2 |
| `grayscott2d` | 1e-6 | C | 12.9 | 10.2 | 6.3 | 6.4 | 4.4 | 4.9 | 7.5 | 7.3 |
| `adr2d` | 1e-4 | B | 13.3 | 13.4 | 13.3 | 13.4 | 13.2 | 14.6 | 14.0 | 14.0 |
| `adr2d` | 1e-4 | C | 13.5 | 11.5 | 8.2 | 7.7 | 6.0 | 7.7 | 9.0 | 9.1 |
| `adr2d` | 1e-6 | B | 9.3 | 9.3 | 9.3 | 9.3 | 9.3 | 10.2 | 10.5 | 10.5 |
| `adr2d` | 1e-6 | C | 9.3 | 7.4 | 4.9 | 5.0 | 3.6 | 5.2 | 6.9 | 7.0 |

- stage 1은 C도 B와 같은 수의 벡터가 필요하다(basis가 비어 있다).
- stage 2~8에서 C가 새로 만드는 벡터는 B의 33~99%다(위 표의 stage별 C/B). stage 우변에는 `f(y + sum a_ij U_j)`의 비선형 증분이 들어 있고, 이것은 앞 stage들의 Krylov 공간 밖에 있다. 허용치가 1e-10이라 공간 밖 성분이 작아도 거의 처음부터 줄여야 한다.
- 우변이 앞 우변과 그 해의 선형결합이면(선형 문제의 다음 stage가 그렇다) 공유가 잘 된다: 계약 테스트 `single_solves_meet_the_rule_and_the_shared_basis_is_reused`에서 그런 우변은 새 벡터가 첫 solve의 1/4 이하다. 선언한 세 family는 모두 비선형이다. n >= 1e3의 선형 문제는 재지 않았다.

## 7. 보조 측정: 같은 preconditioner (right Jacobi)

| family | rtol | JVP B | JVP C | JVP 비 | step 비 | 감사 통과 B / C |
|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | 17,661 | 16,465 | 1.073 | 1.000 | 예 / 예 |
| `bruss1d` | 1e-6 | 19,533 | 16,709 | 1.169 | 1.000 | 예 / 예 |
| `grayscott2d` | 1e-4 | 1,734 | 1,073 | 1.616 | 1.000 | 예 / 예 |
| `grayscott2d` | 1e-6 | 2,176 | 1,362 | 1.598 | 1.000 | 예 / 예 |
| `adr2d` | 1e-4 | 3,343 | 2,306 | 1.450 | 1.000 | 예 / 예 |
| `adr2d` | 1e-6 | 4,923 | 3,640 | 1.352 | 1.000 | 예 / 예 |

- B와 C에 같은 right Jacobi(`diag(W)`)를 걸었다. A는 preconditioner를 받지 않으므로 A 대조는 없다. 코드의 `JacobiPreconditioner`는 dense 행렬에서만 만들 수 있어서, client가 문제의 해석적 Jacobian 대각으로 같은 대각 scaling을 한다.
- 이 family들에서 Jacobi는 B를 돕지 못한다: B의 JVP가 무전처리 대비 1.009~1.358배다. 공유 이득도 무전처리보다 작다. right preconditioning에서는 stage 해 `U_j = D^-1 u_j`가 `W D^-1`의 Krylov 공간 밖으로 나가기 때문이다(계약 테스트 `single_solves_meet_the_rule_and_the_shared_basis_is_reused`).

## 8. 탐색 실행 (어느 판정에도 넣지 않음)

### 8.1 나중 binary로 선언 실행 재현

- `a_adr2d_1e-4_B_none_audit1` 대 `x_recheck_adr2d_1e-4_B_none_audit1`: counter, 통계, 최종 상태 일치 = 예
- `a_adr2d_1e-4_C_none_audit1` 대 `x_recheck_adr2d_1e-4_C_none_audit1`: counter, 통계, 최종 상태 일치 = 예
- 탐색 옵션을 넣은 뒤의 binary가 선언 arm의 동작을 바꾸지 않았음을 보인다.

### 8.2 basis 크기와 기각 후 유지

| family | rtol | 변형 | 상태 | JVP | JVP 비 (B 대비) | step 비 | basis 최대 | fallback | 유지 횟수 | 감사 실패 |
|---|---|---|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | C60 (병합 기본값 60) | OK | 16,946 | 0.968 | 1.000 | 60 | 343 | 0 | 0 |
| `bruss1d` | 1e-4 | C300 (한도 300) | OK | 13,668 | 1.201 | 1.000 | 300 | 18 | 0 | 0 |
| `bruss1d` | 1e-4 | Ckeep (기각 후 유지) | OK | 14,352 | 1.144 | 1.000 | 200 | 132 | 14 | 0 |
| `bruss1d` | 1e-6 | C60 (병합 기본값 60) | OK | 17,221 | 1.015 | 1.000 | 60 | 402 | 0 | 0 |
| `bruss1d` | 1e-6 | C300 (한도 300) | OK | 13,499 | 1.294 | 1.000 | 227 | 0 | 0 | 0 |
| `bruss1d` | 1e-6 | Ckeep (기각 후 유지) | OK | 13,257 | 1.318 | 1.000 | 200 | 18 | 11 | 0 |
| `grayscott2d` | 1e-4 | C60 (병합 기본값 60) | OK | 1,586 | 1.084 | 1.000 | 60 | 31 | 0 | 0 |
| `grayscott2d` | 1e-4 | Ckeep (기각 후 유지) | OK | 964 | 1.783 | 1.000 | 184 | 0 | 0 | 0 |
| `grayscott2d` | 1e-6 | C60 (병합 기본값 60) | OK | 1,826 | 1.131 | 1.000 | 60 | 41 | 0 | 0 |
| `grayscott2d` | 1e-6 | Ckeep (기각 후 유지) | OK | 1,215 | 1.700 | 1.000 | 125 | 0 | 0 | 0 |
| `adr2d` | 1e-4 | C60 (병합 기본값 60) | OK | 2,171 | 1.134 | 1.000 | 60 | 51 | 0 | 0 |
| `adr2d` | 1e-4 | Ckeep (기각 후 유지) | OK | 1,693 | 1.454 | 1.000 | 123 | 0 | 0 | 0 |
| `adr2d` | 1e-6 | C60 (병합 기본값 60) | OK | 2,724 | 1.414 | 1.000 | 60 | 19 | 0 | 0 |
| `adr2d` | 1e-6 | Ckeep (기각 후 유지) | OK | 2,574 | 1.496 | 1.000 | 72 | 0 | 0 | 0 |

- C60: 병합 hook의 기본 크기 60으로는 보호 stopping test 아래에서 basis가 금방 차서 fallback이 많다. JVP 비는 0.968~1.414로, 이득이 거의 없거나 손해다.
- C300: 한도를 300으로 올려도 `bruss1d`의 JVP 비는 1.201, 1.294로 1.5에 못 미친다. 선언한 한도 200이 `bruss1d`의 판정을 바꾼 것은 아니다.
- Ckeep: local-error 기각 뒤의 재시도에서 Jacobian은 그대로이고 shift만 바뀌므로 Q와 Z = JQ를 그대로 쓴다(재사용에 JVP 0). 기각이 있는 `bruss1d`에서 선언한 C보다 JVP가 1.9%, 3.2% 적다. 기각이 없는 `grayscott2d`, `adr2d`에서는 C와 같다.

### 8.3 선형 허용치를 바꾸면 (세 arm에 같은 값)

보호 규칙의 `rtol = 1e-10`만 바꾸고 나머지는 그대로 둔 실행이다. 마지막 행(1e-10)은 선언 실행이다.

| family | ODE rtol | 선형 rtol | attempts A / B / C | JVP A | JVP B | JVP C | JVP 비 B/C | B: solve당 벡터 | 달성 오차 A / B / C |
|---|---|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | 1e-4 | 64 / 65 / 64 | 21,217 | 6,005 | 3,318 | 1.810 | 10.5 | 0.343 / 0.745 / 5.918 |
| `bruss1d` | 1e-4 | 1e-6 | 64 / 64 / 64 | 21,381 | 8,541 | 5,585 | 1.529 | 15.7 | 0.343 / 0.342 / 0.365 |
| `bruss1d` | 1e-4 | 1e-8 | 64 / 64 / 64 | 23,021 | 12,046 | 9,480 | 1.271 | 22.4 | 0.343 / 0.343 / 0.343 |
| `bruss1d` | 1e-4 | 1e-10 | 64 / 64 / 64 | 27,285 | 16,412 | 14,631 | 1.122 | 30.8 | 0.343 / 0.343 / 0.343 |
| `bruss1d` | 1e-6 | 1e-4 | 110 / 180 / 393 | 36,673 | 9,905 | 8,177 | 1.211 | 5.9 | 3.032 / 19.227 / 148.691 |
| `bruss1d` | 1e-6 | 1e-6 | 110 / 112 / 112 | 36,673 | 9,760 | 6,162 | 1.584 | 9.9 | 3.032 / 2.999 / 7.071 |
| `bruss1d` | 1e-6 | 1e-8 | 110 / 110 / 110 | 36,673 | 12,990 | 9,176 | 1.416 | 13.8 | 3.032 / 3.031 / 3.036 |
| `bruss1d` | 1e-6 | 1e-10 | 110 / 110 / 110 | 37,616 | 17,472 | 13,698 | 1.276 | 18.8 | 3.032 / 3.032 / 3.032 |
| `grayscott2d` | 1e-4 | 1e-4 | 14 / 14 / 14 | 4,253 | 756 | 362 | 2.088 | 5.8 | 0.108 / 0.098 / 0.095 |
| `grayscott2d` | 1e-4 | 1e-6 | 14 / 14 / 14 | 4,253 | 1,048 | 525 | 1.996 | 8.4 | 0.108 / 0.108 / 0.109 |
| `grayscott2d` | 1e-4 | 1e-8 | 14 / 14 / 14 | 4,253 | 1,361 | 723 | 1.882 | 11.2 | 0.108 / 0.108 / 0.108 |
| `grayscott2d` | 1e-4 | 1e-10 | 14 / 14 / 14 | 4,540 | 1,719 | 964 | 1.783 | 14.4 | 0.108 / 0.108 / 0.108 |
| `grayscott2d` | 1e-6 | 1e-4 | 18 / 19 / 22 | 5,597 | 975 | 545 | 1.789 | 5.4 | 0.369 / 1.769 / 1.719 |
| `grayscott2d` | 1e-6 | 1e-6 | 18 / 18 / 18 | 5,597 | 1,286 | 648 | 1.985 | 7.9 | 0.369 / 0.365 / 0.403 |
| `grayscott2d` | 1e-6 | 1e-8 | 18 / 18 / 18 | 5,597 | 1,664 | 896 | 1.857 | 10.6 | 0.369 / 0.369 / 0.370 |
| `grayscott2d` | 1e-6 | 1e-10 | 18 / 18 / 18 | 5,597 | 2,065 | 1,215 | 1.700 | 13.4 | 0.369 / 0.369 / 0.369 |
| `adr2d` | 1e-4 | 1e-4 | 21 / 21 / 21 | 6,933 | 1,152 | 618 | 1.864 | 5.9 | 0.189 / 0.222 / 0.566 |
| `adr2d` | 1e-4 | 1e-6 | 21 / 21 / 21 | 6,933 | 1,565 | 881 | 1.776 | 8.3 | 0.189 / 0.190 / 0.189 |
| `adr2d` | 1e-4 | 1e-8 | 21 / 21 / 21 | 6,933 | 1,990 | 1,253 | 1.588 | 10.9 | 0.189 / 0.189 / 0.189 |
| `adr2d` | 1e-4 | 1e-10 | 21 / 21 / 21 | 6,933 | 2,461 | 1,693 | 1.454 | 13.7 | 0.189 / 0.189 / 0.189 |
| `adr2d` | 1e-6 | 1e-4 | 45 / 47 / 78 | 14,997 | 1,780 | 1,353 | 1.316 | 3.7 | 0.319 / 0.536 / 21.780 |
| `adr2d` | 1e-6 | 1e-6 | 45 / 45 / 45 | 14,997 | 2,386 | 1,331 | 1.793 | 5.6 | 0.319 / 0.316 / 0.599 |
| `adr2d` | 1e-6 | 1e-8 | 45 / 45 / 45 | 14,997 | 3,070 | 1,870 | 1.642 | 7.5 | 0.319 / 0.318 / 0.320 |
| `adr2d` | 1e-6 | 1e-10 | 45 / 45 / 45 | 14,997 | 3,851 | 2,574 | 1.496 | 9.7 | 0.319 / 0.319 / 0.319 |

여섯 cell의 범위로 요약하면:

| 선형 rtol | JVP B / JVP B(1e-10) | 오차 B / 오차 B(1e-10) | JVP 비 B/C | 오차 C / 오차 B | step 비 C/B |
|---|---|---|---|---|---|
| 1e-4 | 0.37 ~ 0.57 | 0.91 ~ 6.34 | 1.21 ~ 2.09 | 0.97 ~ 40.60 | 0.98 ~ 2.18 |
| 1e-6 | 0.52 ~ 0.64 | 0.99 ~ 1.00 | 1.53 ~ 2.00 | 1.00 ~ 2.36 | 1.00 ~ 1.00 |
| 1e-8 | 0.73 ~ 0.81 | 1.00 ~ 1.00 | 1.27 ~ 1.88 | 1.00 ~ 1.00 | 1.00 ~ 1.00 |
| 1e-10 | 1 (기준) | 1 (기준) | 1.12 ~ 1.78 | 1.00 ~ 1.00 | 1.00 ~ 1.00 |

- B는 선형 허용치를 1e-8, 1e-6으로 풀어도 달성 오차가 거의 그대로이고 JVP는 표의 비율만큼 준다. 1e-4에서는 오차와 attempt 수가 변하기 시작한다. A는 pass를 끝까지 돌기 때문에 허용치를 풀어도 JVP가 거의 줄지 않는다.
- 공유의 JVP 비는 선형 허용치를 1e-10에서 1e-6으로 풀수록 커진다. 그러나 1e-6부터는 **같은 잔차 허용치인데도 C의 달성 오차가 B보다 나빠지는 cell이 생기고**(표의 오차 C/B), 1e-4에서는 attempt도 늘어 JVP 비가 다시 작아지는 cell이 있다. 공유 basis에서는 여덟 stage의 잔차가 모두 같은 부분공간에 직교하므로 stage 오차가 서로 상관되어 더해지는 것으로 보인다(확인하지 않은 설명이다). 즉 느슨한 허용치에서의 큰 JVP 비는 matched error가 아니다.
- 이 절은 탐색이며 frozen 기준(보호 규칙 1e-10)을 대신하지 않는다.

### 8.4 병합 hook을 그 자체의 stopping test로 돌리면

보호 driver에 `set_shared_basis(Some(max_dim 60, theta 1e-3))`을 켠 실행이다. hook은 driver의 GMRES 규칙이 아니라 `h*gamma*||r||_WRMS <= 1e-3`(step controller의 weight)으로 stage 해를 받는다.

| family | rtol | attempts A / hook | JVP A | JVP hook | JVP A/hook | JVP B/hook | 달성 오차 A / hook |
|---|---|---|---|---|---|---|---|
| `bruss1d` | 1e-4 | 64 / 63 | 27,285 | 7,075 | 3.9 | 2.3 | 0.343 / 0.341 |
| `bruss1d` | 1e-6 | 110 / 112 | 37,616 | 6,745 | 5.6 | 2.6 | 3.032 / 2.986 |
| `grayscott2d` | 1e-4 | 14 / 14 | 4,540 | 147 | 30.9 | 11.7 | 0.108 / 0.112 |
| `grayscott2d` | 1e-6 | 18 / 18 | 5,597 | 290 | 19.3 | 7.1 | 0.369 / 0.458 |
| `adr2d` | 1e-4 | 21 / 21 | 6,933 | 419 | 16.5 | 5.9 | 0.189 / 0.185 |
| `adr2d` | 1e-6 | 45 / 45 | 14,997 | 761 | 19.7 | 5.1 | 0.319 / 0.338 |

- hook은 A보다 JVP가 훨씬 적고 달성 오차는 비슷하다. 이 차이의 대부분은 공유가 아니라 **느슨한 stopping test**에서 온다: 같은 계에서 hook이 받는 잔차는 보호 임계값의 수만 배다(계약 테스트 `the_merged_hook_accepts_residuals_far_above_the_protected_threshold`, 실패하도록 쓴 `merged_hook_solution_meets_the_protected_threshold`의 출력은 `logs/merged_hook_failing_test.log`).
- 따라서 hook을 켠 실행과 끈 실행의 JVP 차이는 공유의 효과를 재지 못한다. 병합 계열의 계약 테스트 `shared_basis_on_heat_uses_fewer_jvps_at_matched_steps`가 하는 비교가 이것이다. 공유의 효과를 보려면 같은 규칙의 per-stage GMRES와 비교해야 하며, 8.3절이 2-norm 상대 규칙으로 그 비교를 보여 준다. hook의 WRMS 규칙을 쓰는 per-stage GMRES는 만들지 않았다.

### 8.5 강하게 stiff한 경우 (표준 Brusselator, 길이 1, 512 cell)

| arm | 상태 | attempts | 기각 | 선형 solve 실패 | JVP | 달성 오차 |
|---|---|---|---|---|---|---|
| A | OK | 865 | 428 | 428 | 690,281 | 0.002 |
| B | OK | 865 | 428 | 428 | 594,230 | 0.002 |
| C60 | OK | 865 | 428 | 428 | 627,267 | 0.002 |

- JVP 비 B/C60 = 0.947, step 비 = 1.000.
- 이 영역에서는 출하 경로(무전처리 GMRES(40), 200벡터, 1e-10)가 Krylov 한도 때문에 step을 줄인다. 병합 kernel은 벡터당 O(n k^2)이라 수백 차원 basis로는 시간이 너무 걸려 기본 크기 60만 쟀다.

## 9. 한계

- stage a family 셋은 사전에 shift 수 `h*gamma*rho(J)`가 1~10 정도가 되게 잡았다. 실제로 약하게 stiff하다: explicit DOP853이 rtol 1e-13에서 RHS 905번(`grayscott2d`), 1,673번(`adr2d`), 10,625번(`bruss1d`)에 끝난다. 강하게 stiff한 문제에 대해서는 8.5절의 한 cell 말고는 말할 수 없다.
- `grayscott2d`(attempt 14, 18)와 `adr2d`(21, 45)는 attempt 수가 적고, 초기 step 1e-6에서 최대 5배씩 올라가는 구간이 그 안에 들어 있다. 비는 결정적이지만 적은 step의 합이다.
- JVP에는 확인용 true residual이 들어 있다(3절의 민감도 표).
- instructions와 메모리는 병합 kernel(벡터당 재분해, n×k 행렬 3개)의 값이다. v2가 있었다면 달랐을 것이다. v2는 만들지 않았다.
- preconditioner는 Jacobi 하나만 쟀고, 이 family들에서는 효과가 없다. 강한 preconditioner 아래의 공유는 재지 않았다.
- wall time은 공유 호스트에서 고정 core 없이 한 번 잰 값이라 `diagnostic_only`다.
- step 사이 basis 유지는 구현하지도 재지도 않았다(10절은 측정에서 나온 추론이다).

## 10. basis를 step 사이에도 유지해야 하는가

권고: **아니오.** 기각 뒤 재시도에서만 유지할 가치가 있다.

- 같은 attempt 안, 즉 Jacobian과 shift가 모두 같고 Z = JQ가 정확한 상황에서도 다음 stage는 per-stage GMRES가 쓰는 벡터의 33~99%를 다시 만든다(6절). step이 바뀌면 우변은 그보다 더 달라진다.
- step이 바뀌면 Jacobian이 바뀌므로 저장한 Z = JQ는 못 쓴다. 벡터 k개를 넘기려면 JVP k번으로 Z를 다시 만들어야 하고, stage 1이 처음부터 푸는 데 드는 벡터 수(5절의 B 평균)가 넘길 basis 크기(5절의 C 평균)보다 훨씬 작다. Z를 갱신하지 않으면 다른 Jacobian으로 푸는 것이 되어 방법이 바뀐다(차수 조건은 V05 S4).
- 기각 뒤 재시도는 Jacobian이 같고 shift만 다르므로 Q, Z를 JVP 없이 그대로 쓸 수 있다. 효과는 기각 비율만큼이다(8.2절 Ckeep).
- JVP를 줄이려는 것이 목적이면 8.3절과 8.4절이 가리키는 쪽, 즉 선형 stopping test를 ODE 허용치에 맞추는 것이 공유보다 훨씬 크다. 이것은 보호 driver의 허용치를 바꾸는 일이라 이 node의 범위 밖이며, 별도 사전등록이 필요하다.

## 11. 파일과 재현

- client: `crates/rodas5p-integrators/src/stage_clients/krylov.rs` (`KrylovStageClient::per_stage`, `KrylovStageClient::shared_basis`, `ResidualAuditClient`, `StageStoppingRule`)
- 테스트: `tests/stage_clients_krylov_contracts.rs`, `tests/stage_clients_krylov_alloc.rs` (단독 binary), 문제 정의와 arm 실행기 `tests/stage_clients_krylov_problems.rs`, 측정 프로그램 `tests/stage_clients_krylov_measure.rs`
- 증거: `PREDECLARED.json`, `reference/` (기준해와 생성 스크립트), `logs/` (실행별 `.log`, `.perf`, 환경 기록), `run_measurements.sh`, `parse_results.py`, `RESULTS.json`, `make_report.py`

검증 명령(결과는 node의 구조화 출력에 있다):

```
cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
  --test stage_clients_krylov_contracts --test stage_clients_krylov_alloc --test stage_clients_krylov_measure
cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test ppsw1_6b_contracts --test ptsw2b_contracts
cargo +1.94.1 clippy --locked -j 4 --release -p rodas5p-integrators --lib --test stage_clients_krylov_problems \
  --test stage_clients_krylov_contracts --test stage_clients_krylov_alloc --test stage_clients_krylov_measure --no-deps -- -D warnings
python3 -I research/production_loop_20261116/check_checkpoint.py
```

측정 재현:

```
cd research/production_loop_20261116/shared_krylov
python3 -I reference/make_reference.py --family bruss1d --out reference/bruss1d.json
./run_measurements.sh stage_a          # 이미 있는 log는 건너뛴다
./run_measurements.sh exploratory      # exploratory_tolerance, exploratory_hook, exploratory_capacity, exploratory_recheck
python3 -I parse_results.py && python3 -I make_report.py
```

