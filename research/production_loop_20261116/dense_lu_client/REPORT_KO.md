# V18 조밀 LU 클라이언트: 인수분해 저장소 재사용과 선택형 스레드 인수분해

- 노드: `V18_DENSE_LU_CLIENT` (사전등록 부록 A2), 루프 L1 wave 2
- 이 문서는 `make_report.py`가 `RESULTS.json`에서 만든다. `RESULTS.json`은 `parse_results.py`가 `logs/`의 원시 출력만으로 만든다.
- 사전 선언: `PREDECLARED.json` (첫 측정 실행 전에 커밋). 동결 임계값은 바꾸지 않았다.

**정정 고지 (2026-11-23, 독립 리뷰 `REVIEW.json`, 판정: 정정 조건부 수용).** 동결 판별식의 판정(동일성·할당·page fault PASS, 재사용 wall 판정 불가, faer `Par` 줄 NOT_RUN, client 쪽 스레드 arm 판정 불가)은 리뷰가 원자료에서 다시 계산해 확인했다. 병합 뒤 orchestrator가 내린 판정 두 개와 정정할 서술:
1. **판정 1, 스레드 arm**: client 쪽 스레드 arm이 사전등록 부록 A2의 스레드 arm이다. 동결 행은 worker 수(2, 4, 6), opt-in, 그리고 '보호 manifest를 바꿔야 하면 NOT_RUN'만 정한다. 이 arm은 manifest를 바꾸지 않고, worker 수를 client 인수분해의 인자로 받고, 전역 설정을 하지 않으며, 상수는 timing 전에 고정했다. 'faer의 `Par` 인자' 줄은 그 기제에 대한 NOT_RUN으로 남는다.
2. **스레드 판정 읽는 법 (리뷰 R5)**: KILL은 발동하지 않았다(n = 2048의 권위 측정 두 개가 2.69, 2.73). PASS 줄은 '보류'가 아니다. n = 512의 6-worker 값은 측정 15개 전부 1.21-1.28이고(품질 규칙을 만족한 2회차 시도 두 개 1.262, 1.217 포함) 기준 1.5에 못 미치므로, 판정 가능해지면 FAIL이다. n = 2048만 보면 모든 측정이 1.5 이상이다(2.49-2.77).
3. **재사용 wall의 조건 (리뷰 R1, major)**: 1.10을 넘는 여유는 보호 driver가 page fault 상태일 때만 있다. 선언한 2-arm cell에서 보호 arm은 step당 456-840회 fault를 내고 비율은 1.27-1.62다. 같은 cell이 5-arm 실행 안에 있으면 보호 arm의 fault가 0이고 비율은 1.075-1.303이다. 품질 규칙을 만족한 시도 하나가 1.075로 동결 기준 1.10보다 작다. user-mode 명령어 수는 보호 driver의 0.960-0.966배다.
4. **판정 2, 분산 규칙**: 품질 규칙은 자료를 본 뒤에 바꾸지 않는다. 보호 arm의 page fault 순서는 결정적이어서 skewnet cell 세 개는 어떤 load에서도 분산 10 % 규칙을 만족하지 못한다(측정 15개 전부 0.102 이상). 따라서 사전 선언한 결합 규칙(여섯 실행 전부 권위 측정)으로는 재사용 wall 기준을 판정할 수 없고, 이 node에서는 판정 불가로 남긴다. 2회차 측정과 리뷰어 측정은 탐색 자료다. 판정 가능한 시험은 새 사전등록이 필요하다(예: 조용한 host의 meanfield cell은 분산 규칙을 6회 중 4회 만족하고 값은 1.32-1.56).
5. **page fault (리뷰 R3)**: 재사용 arm의 최악 비율은 60회 실행 기준 step당 0.57이다(clippy 수정 뒤 다시 돌려 덮어쓴 첫 로그 묶음 포함. 그 묶음은 commit 9e1eadef에 있다). 한 step의 최대는 13회다. 사전 선언한 비율 통계(1 이하)의 판정은 그대로 PASS다.
6. **builder 최종 보고의 낡은 숫자 (리뷰 R4)**: n = 2048 동일성 test는 251.93 s, 엄격 JSON 파일은 687개, 변경 경로는 167개가 맞다. `RESULTS.json`과 이 문서는 로그에서 그대로 재생성되며 영향이 없다.
7. **사전 선언 전에 돌린 것 (리뷰 R6)**: 사전 선언 commit 전에 보호 driver의 크기 조정 pilot 10회와 n = 160 smoke 실행이 있었다(`PREDECLARED.json`에 공개, smoke 출력은 남기지 않음). client 소스는 그 앞 commit 이후 바뀌지 않았다.

## 1. 동결 판별식별 판정

| 판별식 | 판정 |
|---|---|
| 재사용 arm: 비트 동일성 (클라이언트 키트 집합 + faer 경로 n = 128, 512, 2048) | PASS |
| 재사용 arm: 워밍업 후 시도당 힙 할당 0 | PASS |
| 재사용 arm: 워밍업 후 시도당 minor page fault 1 이하 | PASS |
| 재사용 arm: n = 512 벽시계 1.10배 이상, 세 힙 배치 변형 모두 | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 재사용 arm PASS 기준 (위 네 항목 모두) | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 재사용 arm KILL 기준 (비트 동일성 불가, 또는 세 변형 모두 1.03배 미만) | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 스레드 arm, 계약 문구 그대로 (faer 인수분해 호출의 병렬 인자) | NOT_RUN |
| 스레드 arm, 클라이언트 측 분할 구현에 동결 임계값 적용: PASS 기준 (6 워커, n >= 512, 1.5배 이상) | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 같은 구현: KILL 기준 (n = 2048, 6 워커, 1.15배 미만) | NOT_DECIDABLE (타이밍 품질 규칙 미달) |

스레드 arm에는 두 줄이 있다. faer 자체의 스레드 인수분해는 보호 대상 매니페스트를 바꾸지 않고는 쓸 수 없어서 NOT_RUN이다 (6절). 매니페스트 변경이 필요 없는 클라이언트 측 구현은 만들어서 동결 임계값 그대로 재었다. 둘째 줄을 부록 A2의 판정으로 볼지 탐색 결과로 볼지는 오케스트레이터가 정한다. 이 노드는 정하지 않는다.

타이밍 시도 36개 가운데 품질 규칙을 만족한 것은 2개다 (spread 한도를 넘은 시도 33개, load 한도만 넘은 시도 1개; 시작 시 1분 load 범위 [5.88, 14.63]). 호스트에서 같은 그룹의 다른 에이전트가 빌드와 테스트를 돌리고 있었다. NOT_DECIDABLE인 줄은 동결 규칙대로 판정하지 않은 것이다. 아래 범위는 라벨과 상관없이 실행된 모든 시도의 값이며 진단용이다 (판정에 쓰지 않음):

| 속도비 (보호 중앙값 / arm 중앙값) | 시도 수 | 최소 | 최대 |
|---|---|---|---|
| 재사용 / 보호, meanfield.n512 | 9 | 1.408 | 1.623 |
| 재사용 / 보호, skewnet.n512 | 9 | 1.329 | 1.483 |
| threaded2 / 보호, skewnet.n2048 | 9 | 1.941 | 2.097 |
| threaded4 / 보호, skewnet.n2048 | 9 | 2.456 | 2.722 |
| threaded6 / 보호, skewnet.n2048 | 9 | 2.533 | 2.757 |
| reuse / 보호, skewnet.n512 | 9 | 1.105 | 1.303 |
| threaded2 / 보호, skewnet.n512 | 9 | 1.090 | 1.258 |
| threaded4 / 보호, skewnet.n512 | 9 | 1.166 | 1.309 |
| threaded6 / 보호, skewnet.n512 | 9 | 1.214 | 1.272 |

## 2. 무엇을 만들었나, 사용자가 무엇을 켜야 하나

- 코드: `crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse.rs`, `dense_lu_reuse/threaded.rs`. 기본 경로는 건드리지 않았다. 아무것도 기본으로 선택되지 않는다.
- 재사용 arm을 쓰려면: `DenseLuReuseStageClient::for_protected_run(&problem, t0, &y0)?` (또는 `new(n, DenseLuMode::Faer)`)로 클라이언트를 만들어 `integrate_rodas5p_fast_client_observed_with_atol`에 넘긴다. 보호 조밀 드라이버와 같은 정책으로 LU 모드를 고르고, faer 모드에서는 인수분해가 클라이언트 소유 저장소에 쓴다.
- 스레드 arm을 쓰려면: 위 클라이언트에 `.with_factor_workers(k)?` (k = 2 이상)를 붙인다. 풀은 클라이언트가 소유하고, 전역 설정은 건드리지 않는다. k = 1은 재사용 arm과 같다.
- 보호 경로가 한 번의 인수분해에서 하는 일 (`rodas5p_core::LuFactorization::new` -> faer 0.24.4): 열 우선 행렬 새로 할당, 그 사본 할당, 치환 배열 2개, 스크래치 1개, `lu_in_place`(순차), U용 세 번째 행렬 할당과 삼각 복사. 재사용 arm은 같은 `lu_in_place`를 같은 인자로, 같은 배치의 자기 소유 `Mat`에 대해 부른다. W는 열 우선 행렬에 바로 조립하고 L/U 분리는 하지 않는다 (faer의 삼각 풀이는 반대쪽 삼각을 읽지 않는다).

## 3. 측정 1: 비트 동일성

| 출처 | 행 수 | 비트 동일 | 비고 |
|---|---|---|---|
| 테스트: 클라이언트 키트 집합 (Robertson, HIRES, Brusselator n = 64, 200, 평균장 n = 200; rtol 3개) | 15 | 15 | 비교한 수락 상태 2216개 |
| 테스트: faer 경로, 재사용 arm | 18 | 18 | n = [128, 512, 2048] |
| 러너: 재사용 arm | 18 | 18 | n = [128, 512, 2048], 수락 상태 1712개 |
| 러너: 키트 faer 클라이언트 | 18 | 18 |  |
| 러너: 스레드 arm (2, 4, 6 워커; 보고만) | 54 | 54 | 끝 상태 최대 차이 0.000e+00 |

- 테스트 바이너리: `test_identity.log` test result: ok. 7 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 13.46s; `test_identity_n2048.log` test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 251.93s.
- 비교한 양: 시도 수, 수락/기각 수, 야코비안 재사용 수, `WorkCounters`의 모든 필드, 수락 상태의 개수와 지문(t와 y의 비트), 끝 상태 지문. 테스트에서는 출력 시각과 상태 전부도 비교한다.
- 커널 수준: `factors_permutation_and_solves_equal_the_protected_kernel` 테스트가 행 교환이 일어나는 행렬에서 결합 인자, 치환, 풀이를 `LuFactorization`과 비트 단위로 비교한다 (상태: ok).
- 행 교환이 실제로 일어난 재사용 행(러너): 8개, 마지막 인수분해에서 옮겨진 행의 최대 수 1393. 평균장 Brusselator는 행 교환이 없고 skewnet은 있다.
- 스레드 인수분해의 인자가 순차 인자와 비트 동일한가 (테스트, n = 65..512, 2/4/6 워커): 18 / 18 행.

n = 2048 (러너, 재사용 arm):

| 셀 | rtol | 완주 | 시도 | 수락 상태 | 비트 동일 | 옮겨진 행 |
|---|---|---|---|---|---|---|
| meanfield.n2048 | 1e-04 | 아니오 | 40 | 32 | 예 | 0 |
| meanfield.n2048 | 1e-06 | 아니오 | 40 | 39 | 예 | 0 |
| meanfield.n2048 | 1e-08 | 아니오 | 40 | 40 | 예 | 0 |
| skewnet.n2048 | 1e-04 | 예 | 17 | 17 | 예 | 1393 |
| skewnet.n2048 | 1e-05 | 예 | 26 | 26 | 예 | 1272 |
| skewnet.n2048 | 1e-06 | 예 | 45 | 45 | 예 | 1125 |

평균장 n = 2048은 시도 40회에서 멈춘 실행을 마지막 수락 상태에서 비교한다 (사전 선언).

## 4. 측정 2: 힙 할당과 page fault

할당 (계수 할당자를 쓰는 전용 테스트 바이너리 `test_alloc.log`, n = 128, 512; 판정 PASS):

| arm | 행 | 센 스텝 | 센 스텝의 할당 합 | 시도당 할당 | 스텝당 최대 | 시도당 바이트 (n별) |
|---|---|---|---|---|---|---|
| kit | 8 | 437 | 2904 | 6..6 | 12 | n=128: [396288], n=512: [6303744] |
| protected | 8 | 437 | 2904 | 6..6 | 12 | n=128: [396288], n=512: [6303744] |
| reuse | 8 | 437 | 0 | 0..0 | 0 | n=128: [0], n=512: [0] |
| threaded2 | 8 | 437 | 4 | 0..0 | 1 | n=128: [0], n=512: [0] |
| threaded6 | 8 | 437 | 5 | 0..0 | 1 | n=128: [0], n=512: [0] |

스레드 arm의 0이 아닌 값은 rayon 풀의 작업 큐 블록이다 (인수분해 수십 회에 한 번).

Minor page fault (러너 `faults` 모드, 프로세스당 arm 하나, 3회 연속 실행, 워밍업 = 처음 2 수락 스텝; 판정 PASS). 값은 수락 스텝당이다. 수락 스텝 하나는 시도 1회와 그 기각 시도를 담으므로 시도당 값의 상한이다.

| 셀 | 경로 길이 | arm | 실행 수 | 스텝당 minor fault (최소..최대) |
|---|---|---|---|---|
| meanfield.n128 | 93 | kit | 3 | 35.74..35.74 |
| meanfield.n128 | 93 | protected | 3 | 35.74..35.74 |
| meanfield.n128 | 93 | reuse | 3 | 0.00..0.00 |
| meanfield.n2048 | 93 | kit | 3 | 25261.83..25263.11 |
| meanfield.n2048 | 93 | protected | 3 | 25261.75..25262.64 |
| meanfield.n2048 | 93 | reuse | 3 | 0.00..0.03 |
| meanfield.n512 | 93 | kit | 3 | 555.09..839.90 |
| meanfield.n512 | 93 | protected | 3 | 32.42..839.90 |
| meanfield.n512 | 93 | reuse | 3 | 0.00..0.29 |
| meanfield.n512 | 93 | threaded6 | 3 | 0.04..2.94 |
| meanfield.n512 | 108 | kit | 3 | 839.90..839.90 |
| meanfield.n512 | 108 | protected | 3 | 839.90..839.90 |
| meanfield.n512 | 108 | reuse | 3 | 0.00..0.00 |
| meanfield.n512 | 108 | threaded6 | 3 | 1.27..1.70 |
| meanfield.n512 | 124 | kit | 3 | 32.42..839.96 |
| meanfield.n512 | 124 | protected | 3 | 32.42..839.91 |
| meanfield.n512 | 124 | reuse | 3 | 0.00..0.00 |
| meanfield.n512 | 124 | threaded6 | 3 | 0.00..1.25 |
| skewnet.n128 | 93 | kit | 3 | 13.37..33.22 |
| skewnet.n128 | 93 | protected | 3 | 33.22..33.22 |
| skewnet.n128 | 93 | reuse | 3 | 0.00..0.00 |
| skewnet.n2048 | 93 | kit | 3 | 24579.00..24579.00 |
| skewnet.n2048 | 93 | protected | 3 | 24579.00..24579.00 |
| skewnet.n2048 | 93 | reuse | 3 | 0.00..0.00 |
| skewnet.n512 | 93 | kit | 3 | 30.88..410.51 |
| skewnet.n512 | 93 | protected | 3 | 779.85..779.85 |
| skewnet.n512 | 93 | reuse | 3 | 0.00..0.00 |
| skewnet.n512 | 93 | threaded6 | 3 | 0.00..4.78 |
| skewnet.n512 | 108 | kit | 3 | 779.85..779.85 |
| skewnet.n512 | 108 | protected | 3 | 30.81..514.38 |
| skewnet.n512 | 108 | reuse | 3 | 0.00..0.00 |
| skewnet.n512 | 108 | threaded6 | 3 | 1.19..2.99 |
| skewnet.n512 | 124 | kit | 3 | 30.81..779.85 |
| skewnet.n512 | 124 | protected | 3 | 30.81..514.37 |
| skewnet.n512 | 124 | reuse | 3 | 0.00..0.00 |
| skewnet.n512 | 124 | threaded6 | 3 | 1.20..2.79 |

재사용 arm의 최악값: 0.286 (실행 30개).

## 5. 측정 3: 재사용 arm의 벽시계, n = 512, 세 힙 배치 변형

프로토콜: 코어 6-11 고정 (타이밍 락), arm 교대, 워밍업 2 + 반복 7, 실행 전후 load average 기록. 권위 조건: 전후 1분 load 8 이하, arm별 (최대-최소)/중앙값 0.10 이하. 속도비 = 보호 드라이버 중앙값 / 재사용 arm 중앙값.

| 셀 | 경로 길이 | 라벨 | 쓴 시도 | 보호 ms | 재사용 ms | 속도비 | spread 보호/재사용 | 스텝당 fault 보호/재사용 | load 전/후 | 시도 수 |
|---|---|---|---|---|---|---|---|---|---|---|
| meanfield.n512 | 93 | diagnostic_only | 3회 중 round 1 try 3 | 500.71 | 315.88 | 1.585 | 0.209 / 0.172 | 839.9 / 0.0 | 11.61 / 11.48 | 89 |
| meanfield.n512 | 108 | diagnostic_only | 3회 중 round 1 try 3 | 469.83 | 307.68 | 1.527 | 0.134 / 0.082 | 839.9 / 0.0 | 11.48 / 11.31 | 89 |
| meanfield.n512 | 124 | diagnostic_only | 3회 중 round 1 try 3 | 441.92 | 307.17 | 1.439 | 0.137 / 0.095 | 557.7 / 0.0 | 11.31 / 11.36 | 89 |
| skewnet.n512 | 93 | diagnostic_only | 3회 중 round 1 try 3 | 506.07 | 350.15 | 1.445 | 0.163 / 0.042 | 741.9 / 0.0 | 14.14 / 12.43 | 88 |
| skewnet.n512 | 108 | diagnostic_only | 3회 중 round 1 try 3 | 464.97 | 349.97 | 1.329 | 0.282 / 0.087 | 456.1 / 0.0 | 12.43 / 12.08 | 88 |
| skewnet.n512 | 124 | diagnostic_only | 3회 중 round 1 try 3 | 476.03 | 354.82 | 1.342 | 0.482 / 0.079 | 526.7 / 0.0 | 12.08 / 11.61 | 88 |

- 셀별 판정: meanfield.n512: NOT_DECIDABLE (타이밍 품질 규칙 미달), skewnet.n512: NOT_DECIDABLE (타이밍 품질 규칙 미달). 합친 판정: NOT_DECIDABLE (타이밍 품질 규칙 미달).
- 모든 시도(재측정 포함)는 `RESULTS.json`의 `reuse_wall.cells.*.variants[].invocation.tries`에 있다.

## 6. 측정 4: 스레드 arm

### 6.1 faer의 병렬 인자: NOT_RUN

- 작업공간 매니페스트(`Cargo.toml` 24행)는 faer 0.24.4를 기능 ['linalg', 'sparse', 'sparse-linalg', 'std']로 빌드한다 (`logs/threaded_faer_feature_state.log`). `rayon` 기능이 없다.
- 이 빌드에서 `faer::Par`에는 `Seq`뿐이다. 컴파일 프로브: `error[E0599]: no variant or associated item named `Rayon` found for enum `faer::Par` in the current scope` (종료 코드 101, `logs/threaded_faer_par_compile_probe.log`).
- 켜려면 보호 대상인 `Cargo.toml`과 `Cargo.lock`(faer의 의존성에 rayon, spindle 추가)을 바꿔야 한다. 동결 행의 NOT_RUN 조건에 해당한다.

### 6.2 클라이언트 측 분할 구현

faer의 재귀를 클라이언트 안에서 돌린다: 64열 이하 패널은 faer의 순차 `lu_in_place`, 그 위 수준의 삼각 풀이, 행렬 곱, 행 교환을 워커 수만큼의 서로 겹치지 않는 블록으로 나눠 각 블록에서 faer의 순차 커널을 부른다 (`rayon::join`, 클라이언트 소유 풀). 상수(패널 64열, 조각당 최소 2e6 flop, 교환 65536개)는 타이밍 전에 정했고 조정하지 않았다.

**skewnet.n2048** (적분 구간 [0.0, 1.0]):

| rtol | 라벨 | 쓴 시도 | 시도 수 | 보호 arm 달성 오차 | 중앙값 ms: 보호 / 재사용 / 2 / 4 / 6 워커 | 최악 spread | load 전/후 | 보호 ms/시도 |
|---|---|---|---|---|---|---|---|---|
| 1e-04 | diagnostic_only | 3회 중 round 1 try 3 | 17 | 3.765e-06 | 3679.70 / - / 1784.25 / 1363.79 / 1383.94 | 0.110 | 10.55 / 6.77 | 216.45 |
| 1e-05 | authoritative | 3회 중 round 1 try 3 | 26 | 1.151e-06 | 5593.03 / - / 2667.35 / 2146.85 / 2046.36 | 0.065 | 6.55 / 5.88 | 215.12 |
| 1e-06 | authoritative | 3회 중 round 1 try 3 | 45 | 2.006e-07 | 9645.73 / - / 4611.96 / 3543.79 / 3580.84 | 0.084 | 5.88 / 3.22 | 214.35 |

| arm | rtol | 같은 달성 오차에서 속도비 | 같은 허용오차에서 속도비 | 시도 arm/보호 | 오차비 | 궤적 비트 동일 | 라벨 |
|---|---|---|---|---|---|---|---|
| threaded2 | 1e-04 | 2.062 | 2.062 | 17 / 17 | 1.000000 | 예 | diagnostic_only |
| threaded2 | 1e-05 | 2.097 | 2.097 | 26 / 26 | 1.000000 | 예 | authoritative |
| threaded2 | 1e-06 | 2.091 | 2.091 | 45 / 45 | 1.000000 | 예 | authoritative |
| threaded4 | 1e-04 | 2.698 | 2.698 | 17 / 17 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-05 | 2.605 | 2.605 | 26 / 26 | 1.000000 | 예 | authoritative |
| threaded4 | 1e-06 | 2.722 | 2.722 | 45 / 45 | 1.000000 | 예 | authoritative |
| threaded6 | 1e-04 | 2.659 | 2.659 | 17 / 17 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-05 | 2.733 | 2.733 | 26 / 26 | 1.000000 | 예 | authoritative |
| threaded6 | 1e-06 | 2.694 | 2.694 | 45 / 45 | 1.000000 | 예 | authoritative |

**skewnet.n512** (적분 구간 [0.0, 2.0]):

| rtol | 라벨 | 쓴 시도 | 시도 수 | 보호 arm 달성 오차 | 중앙값 ms: 보호 / 재사용 / 2 / 4 / 6 워커 | 최악 spread | load 전/후 | 보호 ms/시도 |
|---|---|---|---|---|---|---|---|---|
| 1e-04 | diagnostic_only | 3회 중 round 1 try 3 | 29 | 2.468e-05 | 154.79 / 118.77 / 123.02 / 118.29 / 121.69 | 0.472 | 11.36 / 11.33 | 5.34 |
| 1e-06 | diagnostic_only | 3회 중 round 1 try 3 | 88 | 6.235e-08 | 405.84 / 363.58 / 372.38 / 340.49 / 334.19 | 0.215 | 11.33 / 12.62 | 4.61 |
| 1e-08 | diagnostic_only | 3회 중 round 1 try 3 | 340 | 4.193e-09 | 1536.34 / 1363.79 / 1321.69 / 1263.27 / 1257.33 | 0.299 | 12.62 / 10.55 | 4.52 |

| arm | rtol | 같은 달성 오차에서 속도비 | 같은 허용오차에서 속도비 | 시도 arm/보호 | 오차비 | 궤적 비트 동일 | 라벨 |
|---|---|---|---|---|---|---|---|
| threaded2 | 1e-04 | 1.258 | 1.258 | 29 / 29 | 1.000000 | 예 | diagnostic_only |
| threaded2 | 1e-06 | 1.090 | 1.090 | 88 / 88 | 1.000000 | 예 | diagnostic_only |
| threaded2 | 1e-08 | 1.162 | 1.162 | 340 / 340 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-04 | 1.309 | 1.309 | 29 / 29 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-06 | 1.192 | 1.192 | 88 / 88 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-08 | 1.216 | 1.216 | 340 / 340 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-04 | 1.272 | 1.272 | 29 / 29 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-06 | 1.214 | 1.214 | 88 / 88 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-08 | 1.222 | 1.222 | 340 / 340 | 1.000000 | 예 | diagnostic_only |
| reuse | 1e-04 | 1.303 | 1.303 | 29 / 29 | 1.000000 | 예 | diagnostic_only |
| reuse | 1e-06 | 1.116 | 1.116 | 88 / 88 | 1.000000 | 예 | diagnostic_only |
| reuse | 1e-08 | 1.127 | 1.127 | 340 / 340 | 1.000000 | 예 | diagnostic_only |

- 6 워커 판독: skewnet.n2048: 상태 NOT_DECIDABLE, 속도비 최소 2.659 / 최대 2.733, PASS 임계 충족 -; skewnet.n512: 상태 NOT_DECIDABLE, 속도비 최소 1.214 / 최대 1.272, PASS 임계 충족 -
- PASS 기준: NOT_DECIDABLE (타이밍 품질 규칙 미달). KILL 기준: NOT_DECIDABLE (타이밍 품질 규칙 미달).
- 참고 (판정 줄은 바꾸지 않음): n = 2048에서 권위 라벨을 얻은 허용오차는 2개이고 6 워커 속도비는 rtol 1e-05: 2.733, rtol 1e-06: 2.694다. KILL 조건은 세 허용오차 모두에서 1.15 미만이어야 하므로, 그중 2개가 이미 그 이상인 이상 나머지 한 칸의 값과 무관하게 성립할 수 없다. 그래도 사전 선언한 규칙(세 호출 모두 권위)에 따라 판정 줄은 NOT_DECIDABLE로 둔다.
- 기준 해: skewnet의 정확해 phi(t_end)를 파서가 mpmath 40자리로 다시 계산해 끝 상태(비트 패턴으로 기록)와 비교한다. skewnet.n2048: 러너 값과의 최대 차이 1.225e-16; skewnet.n512: 러너 값과의 최대 차이 1.100e-16. 기준 해 불확도 상한 1e-12 (사전 선언).

## 7. 탐색 측정 (판정 없음)

같은 허용오차에서 보호 드라이버 대비 속도비 (기준 해 없음):

| 실행 | 라벨 | 속도비 | 보호 중앙값 ms | load 전/후 |
|---|---|---|---|---|
| extra.meanfield.n512.rtol1e-6.threads | authoritative | threaded2: 1.174, threaded4: 1.247, threaded6: 1.280 | 321.81 | 3.22 / 3.63 |
| extra.meanfield.n2048.rtol1e-6.threads | diagnostic_only | threaded2: 2.322, threaded4: 3.178, threaded6: 3.373 | 8003.50 | 3.63 / 3.85 |
| extra.skewnet.n2048.rtol1e-4.reuse | authoritative | reuse: 1.444 | 3755.46 | 3.85 / 3.86 |

사용자 모드 명령 수 (`perf stat -e instructions:u`, 설정 비용 차감, 결정적 보조 지표). 커널이 page fault에서 하는 일은 들어 있지 않다:

| 셀 | arm | 시도 합 | 시도당 instructions:u | 보호 대비 비 | 시도당 minor fault (perf) |
|---|---|---|---|---|---|
| meanfield.n512 | kit | 267 | 3.9508e+07 | 0.9999 | 765.4 |
| meanfield.n512 | protected | 267 | 3.9514e+07 | - | 68.7 |
| meanfield.n512 | reuse | 267 | 3.7950e+07 | 0.9604 | 4.5 |
| skewnet.n512 | kit | 264 | 4.6157e+07 | 0.9999 | 279.3 |
| skewnet.n512 | protected | 264 | 4.6163e+07 | - | 540.4 |
| skewnet.n512 | reuse | 264 | 4.4599e+07 | 0.9661 | 4.5 |

단계별 시간 (`v18-dense-lu-phases`, 클라이언트 키트의 `TimedStageClient`로 감싼 각 arm; 시도당 ms, 반복 5회의 중앙값). `kit`은 보호 드라이버의 faer 경로와 같은 선형대수, 같은 할당, 같은 비트다. 이 측정은 사전 선언에 없고, 첫 타이밍 결과를 본 뒤에 추가한 탐색 측정이다:

| 셀 | arm | 인수분해 | 풀이 8회 | 선형화 | 나머지 | 합계 | 인수분해 비중 | 인수분해 속도비 (재사용 대비) | 실행 속도비 (kit 대비) | 라벨 |
|---|---|---|---|---|---|---|---|---|---|---|
| meanfield.n512 | kit | 3.624 | 0.487 | 0.069 | 0.054 | 4.234 | 0.856 | 0.82 | 1.00 | diagnostic_only |
| meanfield.n512 | reuse | 2.965 | 0.442 | 0.050 | 0.033 | 3.490 | 0.850 | 1.00 | 1.21 | diagnostic_only |
| meanfield.n512 | threaded2 | 2.649 | 0.514 | 0.046 | 0.057 | 3.266 | 0.811 | 1.12 | 1.30 | diagnostic_only |
| meanfield.n512 | threaded4 | 2.521 | 0.512 | 0.044 | 0.038 | 3.115 | 0.809 | 1.18 | 1.36 | diagnostic_only |
| meanfield.n512 | threaded6 | 2.508 | 0.518 | 0.048 | 0.070 | 3.144 | 0.798 | 1.18 | 1.35 | diagnostic_only |
| skewnet.n1024 | kit | 24.699 | 1.891 | 0.703 | 2.655 | 29.948 | 0.825 | 0.75 | 1.00 | diagnostic_only |
| skewnet.n1024 | reuse | 18.501 | 1.795 | 0.583 | 2.736 | 23.615 | 0.783 | 1.00 | 1.27 | diagnostic_only |
| skewnet.n1024 | threaded2 | 14.493 | 1.999 | 0.658 | 3.044 | 20.194 | 0.718 | 1.28 | 1.48 | diagnostic_only |
| skewnet.n1024 | threaded4 | 11.188 | 2.022 | 0.641 | 2.826 | 16.676 | 0.671 | 1.65 | 1.80 | diagnostic_only |
| skewnet.n1024 | threaded6 | 10.687 | 1.891 | 0.666 | 2.761 | 16.005 | 0.668 | 1.73 | 1.87 | diagnostic_only |
| skewnet.n2048 | kit | 205.643 | 18.537 | 5.189 | 18.470 | 247.839 | 0.830 | 0.63 | 1.00 | diagnostic_only |
| skewnet.n2048 | reuse | 128.786 | 15.232 | 4.863 | 17.362 | 166.243 | 0.775 | 1.00 | 1.49 | diagnostic_only |
| skewnet.n2048 | threaded2 | 86.475 | 16.575 | 5.167 | 17.412 | 125.629 | 0.688 | 1.49 | 1.97 | diagnostic_only |
| skewnet.n2048 | threaded4 | 62.508 | 17.472 | 5.058 | 18.490 | 103.528 | 0.604 | 2.06 | 2.39 | diagnostic_only |
| skewnet.n2048 | threaded6 | 55.988 | 17.858 | 4.858 | 19.024 | 97.727 | 0.573 | 2.30 | 2.54 | diagnostic_only |
| skewnet.n512 | kit | 3.728 | 0.487 | 0.096 | 0.626 | 4.937 | 0.755 | 0.82 | 1.00 | diagnostic_only |
| skewnet.n512 | reuse | 3.047 | 0.437 | 0.079 | 0.553 | 4.116 | 0.740 | 1.00 | 1.20 | diagnostic_only |
| skewnet.n512 | threaded2 | 2.907 | 0.503 | 0.072 | 0.618 | 4.100 | 0.709 | 1.05 | 1.20 | diagnostic_only |
| skewnet.n512 | threaded4 | 2.738 | 0.506 | 0.076 | 0.555 | 3.875 | 0.707 | 1.11 | 1.27 | diagnostic_only |
| skewnet.n512 | threaded6 | 2.692 | 0.509 | 0.086 | 0.610 | 3.896 | 0.691 | 1.13 | 1.27 | diagnostic_only |

### 조용한 호스트에서의 반복 측정 (round 2; 이 노드의 판정이 아님)

재측정 규칙은 호출마다 측정 1회와 재측정 2회까지만 판정에 쓰게 한다. 세 번째 시도가 끝날 무렵 호스트가 조용해져서, 같은 호출을 같은 프로토콜로 한 번 더 돌려 탐색 자료로 남겼다 (로그 접미사 `.round2`). 아래는 같은 파서 규칙을 그 자료에 적용한 판독이다. 판정으로 채택할지는 오케스트레이터가 정한다.

| 판독 (판정 아님) | 값 |
|---|---|
| 재사용 벽시계, 합친 판독 | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 재사용 벽시계, 셀별 | meanfield.n512: NOT_DECIDABLE (타이밍 품질 규칙 미달), skewnet.n512: NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 스레드 PASS 기준 | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 스레드 KILL 기준 | NOT_DECIDABLE (타이밍 품질 규칙 미달) |
| 스레드 6 워커, 크기별 | skewnet.n2048: 상태 NOT_DECIDABLE, 최소 2.494, 최대 2.707, 임계 충족 -; skewnet.n512: 상태 NOT_DECIDABLE, 최소 1.217, 최대 1.262, 임계 충족 - |

| 셀 | 경로 길이 | 라벨 | 쓴 시도 | 보호 ms | 재사용 ms | 속도비 | spread 보호/재사용 | 스텝당 fault 보호/재사용 | load 전/후 | 시도 수 |
|---|---|---|---|---|---|---|---|---|---|---|
| meanfield.n512 | 93 | authoritative | 1회 중 round 2 try 1 | 424.08 | 291.69 | 1.454 | 0.022 / 0.012 | 839.9 / 0.0 | 2.46 / 2.34 | 89 |
| meanfield.n512 | 108 | authoritative | 1회 중 round 2 try 1 | 441.59 | 290.66 | 1.519 | 0.091 / 0.012 | 839.9 / 0.0 | 2.34 / 2.13 | 89 |
| meanfield.n512 | 124 | diagnostic_only | 1회 중 round 2 try 1 | 393.62 | 295.20 | 1.333 | 0.118 / 0.034 | 557.7 / 0.0 | 2.13 / 2.28 | 89 |
| skewnet.n512 | 93 | diagnostic_only | 1회 중 round 2 try 1 | 485.15 | 344.32 | 1.409 | 0.143 / 0.018 | 741.9 / 0.0 | 3.68 / 3.7 | 88 |
| skewnet.n512 | 108 | diagnostic_only | 1회 중 round 2 try 1 | 438.91 | 344.63 | 1.274 | 0.255 / 0.011 | 456.1 / 0.0 | 3.98 / 3.89 | 88 |
| skewnet.n512 | 124 | diagnostic_only | 1회 중 round 2 try 1 | 454.28 | 343.86 | 1.321 | 0.246 / 0.012 | 526.7 / 0.0 | 2.58 / 2.46 | 88 |

**skewnet.n2048** (적분 구간 [0.0, 1.0]):

| rtol | 라벨 | 쓴 시도 | 시도 수 | 보호 arm 달성 오차 | 중앙값 ms: 보호 / 재사용 / 2 / 4 / 6 워커 | 최악 spread | load 전/후 | 보호 ms/시도 |
|---|---|---|---|---|---|---|---|---|
| 1e-04 | diagnostic_only | 1회 중 round 2 try 1 | 17 | 3.765e-06 | 3791.89 / - / 1913.10 / 1513.69 / 1400.67 | 0.306 | 3.82 / 6.15 | 223.05 |
| 1e-05 | authoritative | 1회 중 round 2 try 1 | 26 | 1.151e-06 | 5663.21 / - / 2720.66 / 2125.33 / 2118.42 | 0.077 | 6.15 / 3.75 | 217.82 |
| 1e-06 | diagnostic_only | 1회 중 round 2 try 1 | 45 | 2.006e-07 | 10506.22 / - / 5571.00 / 4461.40 / 4213.06 | 0.298 | 3.75 / 9.01 | 233.47 |

| arm | rtol | 같은 달성 오차에서 속도비 | 같은 허용오차에서 속도비 | 시도 arm/보호 | 오차비 | 궤적 비트 동일 | 라벨 |
|---|---|---|---|---|---|---|---|
| threaded2 | 1e-04 | 1.982 | 1.982 | 17 / 17 | 1.000000 | 예 | diagnostic_only |
| threaded2 | 1e-05 | 2.082 | 2.082 | 26 / 26 | 1.000000 | 예 | authoritative |
| threaded2 | 1e-06 | 1.886 | 1.886 | 45 / 45 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-04 | 2.505 | 2.505 | 17 / 17 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-05 | 2.665 | 2.665 | 26 / 26 | 1.000000 | 예 | authoritative |
| threaded4 | 1e-06 | 2.355 | 2.355 | 45 / 45 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-04 | 2.707 | 2.707 | 17 / 17 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-05 | 2.673 | 2.673 | 26 / 26 | 1.000000 | 예 | authoritative |
| threaded6 | 1e-06 | 2.494 | 2.494 | 45 / 45 | 1.000000 | 예 | diagnostic_only |

**skewnet.n512** (적분 구간 [0.0, 2.0]):

| rtol | 라벨 | 쓴 시도 | 시도 수 | 보호 arm 달성 오차 | 중앙값 ms: 보호 / 재사용 / 2 / 4 / 6 워커 | 최악 spread | load 전/후 | 보호 ms/시도 |
|---|---|---|---|---|---|---|---|---|
| 1e-04 | authoritative | 1회 중 round 2 try 1 | 29 | 2.468e-05 | 128.12 / 113.60 / 114.21 / 106.08 / 101.54 | 0.099 | 2.28 / 2.34 | 4.42 |
| 1e-06 | authoritative | 1회 중 round 2 try 1 | 88 | 6.235e-08 | 370.33 / 344.50 / 337.99 / 318.55 / 304.29 | 0.057 | 2.34 / 2.26 | 4.21 |
| 1e-08 | diagnostic_only | 1회 중 round 2 try 1 | 340 | 4.193e-09 | 1486.68 / 1319.02 / 1306.08 / 1229.19 / 1217.27 | 0.105 | 2.26 / 3.82 | 4.37 |

| arm | rtol | 같은 달성 오차에서 속도비 | 같은 허용오차에서 속도비 | 시도 arm/보호 | 오차비 | 궤적 비트 동일 | 라벨 |
|---|---|---|---|---|---|---|---|
| threaded2 | 1e-04 | 1.122 | 1.122 | 29 / 29 | 1.000000 | 예 | authoritative |
| threaded2 | 1e-06 | 1.096 | 1.096 | 88 / 88 | 1.000000 | 예 | authoritative |
| threaded2 | 1e-08 | 1.138 | 1.138 | 340 / 340 | 1.000000 | 예 | diagnostic_only |
| threaded4 | 1e-04 | 1.208 | 1.208 | 29 / 29 | 1.000000 | 예 | authoritative |
| threaded4 | 1e-06 | 1.163 | 1.163 | 88 / 88 | 1.000000 | 예 | authoritative |
| threaded4 | 1e-08 | 1.209 | 1.209 | 340 / 340 | 1.000000 | 예 | diagnostic_only |
| threaded6 | 1e-04 | 1.262 | 1.262 | 29 / 29 | 1.000000 | 예 | authoritative |
| threaded6 | 1e-06 | 1.217 | 1.217 | 88 / 88 | 1.000000 | 예 | authoritative |
| threaded6 | 1e-08 | 1.221 | 1.221 | 340 / 340 | 1.000000 | 예 | diagnostic_only |
| reuse | 1e-04 | 1.128 | 1.128 | 29 / 29 | 1.000000 | 예 | authoritative |
| reuse | 1e-06 | 1.075 | 1.075 | 88 / 88 | 1.000000 | 예 | authoritative |
| reuse | 1e-08 | 1.127 | 1.127 | 340 / 340 | 1.000000 | 예 | diagnostic_only |

## 8. 재사용 arm이 없애는 것, 스레드가 이득인 곳

### 8.1 재사용 arm이 없애는 것

- 힙 할당: 보호 faer 경로는 인수분해 1회에 6번 할당한다 (n x n 행렬 3개와 작은 배열 3개; n = 128에서 0.40 MB, n = 512에서 6.30 MB). 재사용 arm은 센 스텝 437개에서 0번.
- Minor page fault (수락 스텝당, 실행 전체의 최소..최대): n = 128: 보호 33.2..35.7, 재사용 0.00..0.00; n = 512: 보호 30.8..839.9, 재사용 0.00..0.29; n = 2048: 보호 24579.0..25262.6, 재사용 0.00..0.03. n = 512에서 보호 드라이버의 값은 힙 상태에 따라 실행마다, 한 호출의 반복 사이에서도 바뀐다. n = 2048의 값은 행렬 세 개(각 32 MB = 8192쪽, 합 24576쪽)를 인수분해마다 새로 건드리는 양에 해당한다.
- 사용자 모드 명령 수 (시도당, 재사용 / 보호): meanfield.n512: 0.9604, skewnet.n512: 0.9661. 사용자 모드에서 줄어드는 명령은 몇 퍼센트다. 벽시계 차이의 나머지는 이 계수에 잡히지 않는 일이며, page fault를 처리하는 커널 쪽 일이 거기에 들어간다 (얼마가 그 몫인지는 따로 재지 않았다).
- 인수분해 단계의 시간 (탐색, 시도당 ms, kit -> 재사용): meanfield.n512: 3.62 -> 2.97; skewnet.n1024: 24.70 -> 18.50; skewnet.n2048: 205.64 -> 128.79; skewnet.n512: 3.73 -> 3.05.
- 벽시계 (진단 범위, 1절): n = 512에서 보호 / 재사용 meanfield.n512 1.41..1.62, skewnet.n512 1.33..1.48. 다섯 arm을 한 프로세스에서 교대한 work-precision 호출(skewnet.n512)에서는 보호 드라이버의 워밍업 후 스텝당 fault가 0.0..0.0였고, 그때 재사용 / 보호는 1.11..1.30였다.

### 8.2 스레드가 이득인 곳

- 인수분해 단계만의 속도비 (탐색, 재사용 arm 대비, 2 / 4 / 6 워커): meanfield.n512: 1.12 / 1.18 / 1.18; skewnet.n1024: 1.28 / 1.65 / 1.73; skewnet.n2048: 1.49 / 2.06 / 2.30; skewnet.n512: 1.05 / 1.11 / 1.13.
- 재사용 arm에서 인수분해가 시도 시간에서 차지하는 비중 (탐색): meanfield.n512: 0.85; skewnet.n1024: 0.78; skewnet.n2048: 0.77; skewnet.n512: 0.74.
- 실행 전체의 속도비 (보호 대비, 6 워커, 모든 시도의 범위): skewnet.n2048: 2.53..2.76; skewnet.n512: 1.21..1.27.
- n = 512, 같은 호출들에서: 6 워커 1.21..1.27, 재사용 arm 1.11..1.30 (보호 대비). 두 범위가 겹친다: 이 구현에서 n = 512의 이득은 대부분 저장소 재사용에서 온다. 원인은 재지 않았다. 구현의 상수(패널 64열, 조각당 최소 2e6 flop)는 타이밍 전에 정한 값 그대로이며, 작은 n에 맞춘 조정은 하지 않았다 (사전 선언: 조정 없음).
- 스레드 arm의 궤적과 보호 드라이버의 비트 동일성: 러너 동일성 행 54 / 54, 타이밍 호출의 (시도, arm) 54 / 54에서 모든 반복이 동일. 잰 모든 행에서 같았다 (따라서 시도 수와 달성 오차도 같다). 구성상 보장되는 성질은 아니다.

## 9. 한계와 열린 항목

- 스레드 arm의 둘째 줄(6.2)이 부록 A2의 판정인지는 오케스트레이터의 결정 사항이다.
- 벽시계 판정은 타이밍 품질 규칙을 만족한 round 1의 시도만 쓴다. 라벨이 diagnostic_only인 칸의 수치는 판정에 쓰지 않았다.
- 보호 드라이버는 한 호출의 반복 사이에서 page fault 상태가 바뀌어 (max - min) / median이 10 %를 넘는 일이 잦다 (5절의 spread 열, 조용한 호스트의 round 2에서도 같다). 그래서 재사용 arm의 벽시계 기준은 루프의 spread 규칙 아래에서는 호스트가 조용해도 판정 불가로 남을 수 있다. 규칙을 어떻게 적용할지는 오케스트레이터의 결정 사항이다.
- 재사용 arm과 보호 드라이버의 동일성은 이 빌드(faer 0.24.4, `rayon` 기능 꺼짐, 전역 병렬도 `Seq`)에서 시험한 성질이다. faer 버전이나 기능이 바뀌면 다시 시험해야 한다.
- 스레드 arm의 풀은 실행마다 클라이언트와 함께 만들어지고 버려진다 (잰 구간 안). 풀이(solve)는 순차다.
- 종료 코드가 0이 아닌 호출: 0개.
- 타이밍 전 대기에 쓴 시간: 1200초 (노드 한도 1200초).

## 10. 재현

```
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v18
python3 -I research/production_loop_20261116/dense_lu_client/run_measurements.py build links reference identity tests faults
python3 -I research/production_loop_20261116/dense_lu_client/run_measurements.py wall wp extra perf
python3 -I research/production_loop_20261116/dense_lu_client/parse_results.py
python3 -I research/production_loop_20261116/dense_lu_client/make_report.py
```

러너 바이너리 sha256: fd1c9b5a6a28f80913276c927a184dba7c733b4e679528bebc28f55a9f8b1d8e. 원시 로그 146개의 sha256은 `RESULTS.json`의 `sources.logs`에 있다.
