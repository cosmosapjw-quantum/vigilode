# V25: 행렬 없는 경로의 적응 단계 풀이 P1 — 새 문제 네 가지에서의 확인 시험

이 보고서는 `make_report.py`가 `RESULTS.json`과 `PROTOCOL.json`에서 숫자를 읽어 만든 것입니다. 판정 규칙은 `PREDECLARED.json`(부록 A7, 동결)에 있고, 그 파일이 열어 둔 선택은 P1의 첫 실행 전에 `PROTOCOL.json`에 고정했습니다.

## 1. 무엇을 시험했나

RODAS5P의 행렬 없는(matrix-free) 경로는 단계마다 선형계 `(I - h gamma J) U = rhs`를 재시작 GMRES로 풉니다. 두 정책의 차이는 '언제 멈출지를 얼마나 자주 확인하느냐' 하나뿐입니다.

- **S (현재 기본값)**: 재시작 주기마다 Arnoldi 열 40개를 모두 만든 다음에야 참 잔차를 한 번 확인합니다. 필요한 열이 5개뿐이어도 40개를 만듭니다.
- **P1 (적응 정책)**: 열을 하나 만들 때마다 Givens 회전으로 이미 알고 있는 투영 잔차를 확인하고, 단계 임계값을 처음 만족하는 열에서 주기를 끝냅니다. 그 다음 참 잔차를 **같은 임계값**으로 다시 확인하고, 통과하지 못하면 풀이를 계속합니다. 주기의 상한은 원래의 재시작 길이 40 그대로이므로 **새 하이퍼파라미터가 없습니다**.
- 단계 임계값, Krylov 방법·예산(200)·전처리, 제어기 상수, 허용오차는 바꾸지 않았습니다. P2(첫 주기 10열, 두 배씩 증가)는 비교용으로만 보고합니다.

## 2. 문제족 (이전 노드가 쓰지 않은 네 가지)

| 문제족 | n | 시간 구간 | 성격 | 기준해 불확실성 u_ref |
|---|---|---|---|---|
| ac3d | 4096 | [0, 1] | 3차원 Allen-Cahn (반응-확산) | 2.98e-13 |
| robertson_rd1d | 1200 | [0, 320] | Robertson 반응 + 확산 (강한 강성) | 9.18e-11 |
| schnakenberg2d | 2048 | [0, 1] | Schnakenberg 2차원 (Turing 패턴, 오차 증폭 큼) | 3.35e-09 |
| porous2d | 2304 | [0, 0.1] | 다공성 매질 u_t = lap(u^2) (비선형 확산) | 5.95e-13 |

구간은 동결 규칙의 크기 맞춤 시험(S 단독, rtol 1e-7)으로 정했습니다: robertson_rd1d는 1초 미만이어서 세 번 두 배로 늘려 [0, 320]이 되었고, 나머지는 잠정 구간 그대로입니다. 기준해는 V17 규칙(SciPy DOP853 rtol 1e-13, 교차 검증 두 개)으로 독립 numpy 구현에서 계산했고, Rust와 numpy의 우변·JVP는 20개 상태에서 모든 성분이 비트 단위로 같았습니다.

## 3. 판정표

| 판별자 | 뜻 | 결과 |
|---|---|---|
| D0 | S와 P1을 명시적으로 고른 실행이 편집 뒤에도 비트 단위로 같음 | 통과 |
| V | P1이 끝낸 모든 단계 풀이의 참 잔차가 임계값 이하 (풀이 로그로 재계산) | 통과 (P1 위반 0, 완료 풀이 9287, 최대 잔차/임계값 0.99991) |
| B | 예산을 다 쓴 풀이가 어느 셀에서도 S보다 많지 않음 | 실패 |
| A | 같은 허용오차에서 P1 오차가 S의 3배 이하 | 통과 |
| S | 시도 횟수 비: 문제족 기하평균 1.05 이하, 셀 1.15 이하 | 통과 |
| E | 같은 달성 오차에서 명령어 수가 문제족마다 S보다 적고(1.00 미만) 어느 셀에서도 많지 않음 | 통과 |
| W | 권위 있는 벽시계 셀 중 P1이 S보다 느린 셀이 없음 | 통과 |
| G | (전환 시) 골든 기록 차이가 한계 이내 | 미실행 |
| R | (전환 시) 테스트 묶음·clippy·rustfmt 통과 | 미실행 |

## 4. 문제족별 숫자

| 문제족 | E: 같은 오차에서 명령어 비 (P1/S) | E: 가장 나쁜 셀 | A: 최대 오차 비 | S: 시도 비 기하평균 / 최대 | B: 예산 소진 S / P1 | W: 벽시계 속도 향상 | W: 권위 셀 |
|---|---|---|---|---|---|---|---|
| ac3d | 0.198 | 0.350 | 1.000 | 1.000 / 1.000 | 0 / 0 | 6.466 | 5 / 5 |
| robertson_rd1d | 0.791 | 0.905 | 1.000 | 1.034 / 1.059 | 38 / 41 | 1.254 | 5 / 5 |
| schnakenberg2d | 0.516 | 0.677 | 1.000 | 1.000 / 1.000 | 0 / 0 | 2.053 | 5 / 5 |
| porous2d | 0.470 | 0.671 | 1.000 | 1.000 / 1.000 | 0 / 0 | 2.252 | 5 / 5 |

명령어 비는 V17의 같은 달성 오차 통계(오차 구간 201점의 기하평균)입니다. 1보다 작을수록 P1이 싸다는 뜻입니다. 벽시계 속도 향상은 S 시간 / P1 시간이며, 고정 코어·교차 실행·준비 2회와 반복 7회·부하 기록 규칙을 만족한 셀만 판정에 씁니다.

### 셀별 (허용오차마다)

**ac3d**

| rtol | 오차 S | 오차 P1 | 오차 비 | 명령어 셀 수치 (종류) | 시도 S / P1 | 열/풀이 S / P1 | 예산 소진 S / P1 | 벽시계 비 P1/S (권위) |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 1.76e-06 | 1.76e-06 | 1.000 | 0.350 (같은 허용오차) | 19 / 19 | 40.0 / 19.5 | 0 / 0 | 0.314 (예) |
| 1e-4 | 2.94e-07 | 2.94e-07 | 1.000 | 0.269 (같은 오차) | 24 / 24 | 40.0 / 16.3 | 0 / 0 | 0.223 (예) |
| 1e-5 | 3.40e-08 | 3.40e-08 | 1.000 | 0.210 (같은 오차) | 28 / 28 | 40.0 / 13.5 | 0 / 0 | 0.167 (예) |
| 1e-6 | 3.11e-09 | 3.11e-09 | 1.000 | 0.165 (같은 오차) | 35 / 35 | 40.0 / 11.0 | 0 / 0 | 0.122 (예) |
| 1e-7 | 1.19e-10 | 1.19e-10 | 1.000 | 0.127 (같은 오차) | 55 / 55 | 40.0 / 8.7 | 0 / 0 | 0.091 (예) |

**robertson_rd1d**

| rtol | 오차 S | 오차 P1 | 오차 비 | 명령어 셀 수치 (종류) | 시도 S / P1 | 열/풀이 S / P1 | 예산 소진 S / P1 | 벽시계 비 P1/S (권위) |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 1.55e-05 | 1.45e-05 | 0.938 | 0.905 (같은 허용오차) | 35 / 37 | 116.2 / 104.8 | 11 / 12 | 0.911 (예) |
| 1e-4 | 7.18e-06 | 5.98e-06 | 0.834 | 0.805 (같은 오차) | 34 / 36 | 113.8 / 96.9 | 9 / 10 | 0.815 (예) |
| 1e-5 | 2.39e-06 | 2.39e-06 | 1.000 | 0.789 (같은 오차) | 40 / 40 | 107.3 / 93.9 | 10 / 10 | 0.789 (예) |
| 1e-6 | 7.38e-07 | 7.38e-07 | 1.000 | 0.768 (같은 오차) | 43 / 43 | 95.2 / 81.3 | 8 / 8 | 0.779 (예) |
| 1e-7 | 3.75e-07 | 3.51e-07 | 0.935 | 0.772 (같은 오차) | 36 / 38 | 75.0 / 60.2 | 0 / 1 | 0.773 (예) |

**schnakenberg2d**

| rtol | 오차 S | 오차 P1 | 오차 비 | 명령어 셀 수치 (종류) | 시도 S / P1 | 열/풀이 S / P1 | 예산 소진 S / P1 | 벽시계 비 P1/S (권위) |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 1.864 | 1.864 | 1.000 | 0.677 (같은 오차) | 76 / 76 | 73.7 / 55.0 | 0 / 0 | 0.671 (예) |
| 1e-4 | 0.790 | 0.790 | 1.000 | 0.653 (같은 오차) | 94 / 94 | 59.3 / 43.0 | 0 / 0 | 0.640 (예) |
| 1e-5 | 0.793 | 0.793 | 1.000 | 0.579 (같은 오차) | 124 / 124 | 50.6 / 34.6 | 0 / 0 | 0.559 (예) |
| 1e-6 | 0.011 | 0.011 | 1.000 | 0.508 (같은 오차) | 167 / 167 | 42.2 / 27.2 | 0 / 0 | 0.477 (예) |
| 1e-7 | 0.001 | 0.001 | 1.000 | 0.349 (같은 오차) | 247 / 247 | 40.0 / 20.4 | 0 / 0 | 0.311 (예) |

**porous2d**

| rtol | 오차 S | 오차 P1 | 오차 비 | 명령어 셀 수치 (종류) | 시도 S / P1 | 열/풀이 S / P1 | 예산 소진 S / P1 | 벽시계 비 P1/S (권위) |
|---|---|---|---|---|---|---|---|---|
| 1e-3 | 3.74e-06 | 3.74e-06 | 1.000 | 0.671 (같은 오차) | 8 / 8 | 66.2 / 50.1 | 0 / 0 | 0.666 (예) |
| 1e-4 | 6.66e-07 | 6.66e-07 | 1.000 | 0.582 (같은 오차) | 11 / 11 | 59.1 / 40.0 | 0 / 0 | 0.568 (예) |
| 1e-5 | 1.99e-07 | 1.99e-07 | 1.000 | 0.510 (같은 오차) | 19 / 19 | 45.0 / 28.6 | 0 / 0 | 0.486 (예) |
| 1e-6 | 2.88e-08 | 2.89e-08 | 1.000 | 0.387 (같은 오차) | 29 / 29 | 40.7 / 21.8 | 0 / 0 | 0.356 (예) |
| 1e-7 | 6.88e-09 | 6.88e-09 | 1.000 | 0.263 (같은 허용오차) | 44 / 44 | 40.0 / 16.4 | 0 / 0 | 0.229 (예) |

## 5. 판정

- 6단계 전 동결 규칙의 판정: **KEEP** — V holds and ['B'] fail
- 최종 판정: **KEEP** (G 미실행, R 미실행)

- B 실패 셀: robertson_rd1d rtol 1e-3 (S 11, P1 12), robertson_rd1d rtol 1e-4 (S 9, P1 10), robertson_rd1d rtol 1e-7 (S 0, P1 1). 동결 규칙은 셀 하나라도 P1의 예산 소진 풀이가 S보다 많으면 B를 실패로 봅니다.
- 같은 달성 오차에서 P1의 명령어 수 / S: ac3d 0.198, robertson_rd1d 0.791, schnakenberg2d 0.516, porous2d 0.470 (모두 1.00 미만).
- V: P1이 끝낸 단계 풀이 9287개를 풀이 로그로 다시 세었고 위반 0개, 가장 큰 잔차/임계값 0.999909.
- 탐색용 재풀이 (robertson_rd1d P1의 같은 단계 계 1488개): S가 풀고 P1이 예산을 다 쓴 계 0개, S가 예산을 다 쓰고 P1이 푼 계 0개; 예산 소진 수 [S, P1, P2] = [41, 41, 66].
- 탐색용 재풀이 (robertson_rd1d S의 같은 단계 계 1443개): S가 풀고 P1이 예산을 다 쓴 계 0개, S가 예산을 다 쓰고 P1이 푼 계 0개; 예산 소진 수 [S, P1, P2] = [38, 38, 52].

### 작성 허용 목록 밖에서 바뀌어야 하는 줄 (오케스트레이터/소유자 결정 필요)

- `crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs:182` — pins the enum default to the shipped policy
- `crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs:250` — pins RoutingOptions::default().matrix_free_policy to the shipped policy
- `crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs:260` — pins the routed default's driver identifier to v1 and (next lines) its bits to the shipped entry
- `crates/rodas5p-integrators/tests/mf_adaptive_contracts.rs:411` — pins the candidate identifier of P1, which a switch replaces by rodas5p-mf-fast-transformed-v3
- `crates/vigilode/tests/capabilities_contracts.rs:116` — pins the identifier of the default matrix-free route
- `crates/vigilode/tests/facade_contracts.rs:14` — pins the facade's matrix-free driver identifier (and, near these lines, its bits to the shipped entry)
- `crates/vigilode/tests/facade_contracts.rs:281` — pins the facade's matrix-free driver identifier (and, near these lines, its bits to the shipped entry)
- `crates/vigilode/tests/facade_contracts.rs:1038` — pins the facade's matrix-free driver identifier (and, near these lines, its bits to the shipped entry)
- `crates/rodas5p-integrators/src/routing.rs:258` — the default of the route itself
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:104` — the enum default of MfStageSolvePolicy
- `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs:148` — the identifier of P1

## 6. P2 (보고만 함)

| 문제족 | 같은 오차 명령어 비 P2/S | 예산 소진 S / P2 | 최대 오차 비 | 감사 위반 |
|---|---|---|---|---|
| ac3d | 0.181 | 0 / 0 | 1.000 | 0 |
| robertson_rd1d | 0.851 | 38 / 58 | 0.984 | 0 |
| schnakenberg2d | 0.374 | 0 / 0 | 1.000 | 0 |
| porous2d | 0.357 | 0 / 0 | 1.000 | 0 |

## 7. 사용자에게 무엇이 바뀌는가

- 행렬 없는 경로의 기본값은 바뀌지 않습니다: 라우터와 파사드는 계속 S(rodas5p-mf-fast-transformed-v1)를 씁니다. 골든 기록과 기존 결과도 그대로입니다.
- P1은 계속 선택 사항(opt-in)입니다: RoutingOptions { matrix_free_policy: MfStageSolvePolicy::ColumnTested, .. } 또는 integrate_rodas5p_mf_fast_observed_with_stage_solve_policy(.., MfStageSolvePolicy::ColumnTested, ..)로 고르면, 이 시험의 네 문제족에서 같은 달성 오차의 명령어 수가 S의 0.20, 0.79, 0.52, 0.47배였습니다 (식별자 rodas5p-mf-fast-transformed-v2-candidate-p1).
- 기본값을 P1로 바꿀지는 소유자 결정입니다. 바꾸려면 동결 규칙 B를 넘어서는 결정이 필요하고, 이 노드의 작성 허용 목록 밖에 있는 V24 테스트 줄 4개(OUT_OF_LIST_EDITS.json: mf_adaptive_shipped_bits.rs, mf_adaptive_contracts.rs)도 함께 바뀌어야 합니다.

## 8. 증거 파일

- 동결 설계: `PREDECLARED.json`; 프로토콜: `PROTOCOL.json`; 결과: `RESULTS.json`; 기록 A: `RECORDING_A.json`
- 원시 로그: `logs/sizing`, `logs/consistency`, `logs/reference`, `logs/setup`, `logs/record_a` (S와 P1, 단계 풀이 로그 `*.solves.txt.gz` 포함), `logs/p2`, `logs/timing`
- 진행 기록: `NOTES.txt`
