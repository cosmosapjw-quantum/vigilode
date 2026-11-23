# V08 Jacobian 조립: 정확한 thread 병렬·색칠 유한차분 조립 (opt-in stage client)

- node: `V08_JACOBIAN_ASSEMBLY` (loop L1, wave 2). 이 문서는 `build_report.py`가 `RESULTS.json`에서 만든다. 숫자는 모두 실행 로그(`logs/`)에서 `build_results.py`가 읽은 값이다.
- 사전 선언: `PREDECLARED.json`, commit `1312388740c0e7217aff08e465e0a2973a1d39a1` (어떤 측정보다 먼저 commit). 결과 생성 시점 HEAD: `d64effda39f145cf62c3ea6f25aca6dc8e533f16`.
- code: `crates/rodas5p-integrators/src/stage_clients/assembly.rs`, `assembly/engine.rs`, `assembly/pattern.rs`. test: `tests/stage_clients_assembly_{identity,alloc,measure,problems}.rs`.

## 1. 판정 요약

| 동결 판별식 | 판정 | 근거 |
|---|---|---|
| Jacobian과 궤적이 순차 조립과 bit 동일 | PASS | 선언 문제 5개의 30개 run(arm seq, t1, t2, t4, t6)이 보호 driver + 기준 유한차분 run과 모든 항목에서 같다 (3.1절) |
| 색칠 경로의 RHS 호출 = 색 수 + 1 | PASS | client run 516개 전부에서 조립 호출 = build 수 × (색 수 + 1) (dense 경로는 n + 1). 위반 0건 (3.2절) |
| 틀린 pattern은 닫힌 채 실패 | PASS | 음성 통제 7개 모두 `InvalidInput` + typed error, 수락 step 0 (3.3절) |
| wall ≥ 1.5x (n ≥ 64, RHS ≥ 50 µs인 FD 문제); KILL < 1.1x | 판정 불가 | 자격 문제별: `nlkpp-dense-n400` 판정 불가, `nlkpp-band-n4096-r24` 판정 불가, `bruss1d-n131072` 판정 불가. 사전 선언한 엄격한 규칙(자격 문제 전부 PASS여야 node PASS)에 따른 값 (3.4절) |

판정 규칙은 `PREDECLARED.json`의 `verdict_rules` 그대로다. wall 판정은 전체 풀이 시간(client 생성 + driver 호출)으로 하고, Jacobian phase 시간은 옆에 적기만 한다.

## 2. 만든 것

`AssemblyStageClient`는 선형화마다 문제의 RHS만으로 Jacobian을 만들고, 그 값을 kit의 기존 client(바꾸지 않음)에 넘긴다. 문제의 Jacobian·JVP callback은 부르지 않는다. 기본 경로에는 아무 영향이 없다(opt-in).

- **산술**: facade의 유한차분 `crates/vigilode/src/fd.rs`와 같다. 증분은 `(y_j + sqrt(eps) max(|y_j|, 1)) - y_j`(`fd.rs:443-455`), 원소는 `(f(t, y + shift)[i] - f(t, y)[i]) / delta_j`(`fd.rs:499`, `fd.rs:611`).
- **dense 경로**: 열마다 RHS 1회 + 기준 평가 1회 = build당 n + 1회. 순차 mode는 `fd.rs:476-509`의 loop를 문장 단위로 옮겼다.
- **색칠 경로**: 선언한 sparsity pattern의 열을 Curtis–Powell–Reid(자연 순서 first-fit)로 묶고 묶음마다 RHS 1회 = build당 색 수 + 1회. loop는 `fd.rs:581-622`. 꽉 찬 band에서는 facade와 같은 묶음(`j mod (lower + upper + 1)`)이 된다.
- **실행**: 순차(호출 thread, pool 없음) 또는 thread(client가 소유한 rayon pool, 정확히 P개 thread, 전역 pool 미사용). thread mode는 순도 계약을 명시적으로 수락해야 켜진다(5절).
- **선형대수 back end**: `DenseLuStageClient`(보호 dense driver의 LU), `BandedStageLu`(보호 banded kernel, Indexed), `BandedTightStageLu`(V21, 보호 kernel과 bit 동일). 조립한 값은 back end의 배치(dense row-major 또는 band)로 한 번 복사된다.

이 crate에는 상태 Jacobian의 유한차분이 없다(`OdeProblem::new`는 Jacobian이나 JVP를 요구한다). 보호 driver의 유한차분 경로는 facade(`vigilode`)가 Jacobian callback으로 끼워 넣는 `fd.rs`이고, integrators crate는 facade에 의존할 수 없으므로 산술을 옮겨 적었다. 옮겨 적은 것이 실제 facade code와 같은지는 6절(facade 대조)에서 `vigilode::Rodas5p::solve`로 확인했다.

## 3. 판별식별 결과

### 3.1 Jacobian과 궤적의 bit 동일성

기준은 보호 driver(dense: `integrate_rodas5p_fast_observed_with_atol`, banded: `integrate_rodas5p_fast_banded_observed_with_atol`, kernel Indexed)에 `fd.rs`를 두 번째로 옮겨 적은 순차 유한차분을 Jacobian callback으로 준 run이다. 비교 항목: `success`, `stop_reason`, `attempts`, `accepted_steps`, `rejected_steps`, `jacobian_reuses`, `t_end_bits`, `final_fingerprint`, `state_fingerprint`, `jacobian_fingerprint`, `builds`, `jacobian_builds`, `direct_factorizations`, `direct_solve_calls`. `state_fingerprint`는 모든 선형화 시점의 (t, y) bit, `jacobian_fingerprint`는 조립한 Jacobian의 모든 저장 원소 bit를 이어서 hash한 값이다.

| 문제 | back end | 기준 driver | 시도 / 수락 / 거절 | Jacobian build | t_end | Jacobian fingerprint (기준) | 기준과 같은 arm |
|---|---|---|---|---|---|---|---|
| `bruss1d-n128-coloured` | banded_tight | protected banded driver (kernel Indexed) | 48 / 48 / 0 | 48 | 5.18423 | `798ab0df41f545fb` | 5 / 5 |
| `bruss1d-n128-dense` | dense_lu(in-place) | protected dense driver | 48 / 48 / 0 | 48 | 5.18423 | `1bdfd9898cbad42b` | 5 / 5 |
| `bruss1d-n131072` | banded_tight | protected banded driver (kernel Indexed) | 48 / 48 / 0 | 48 | 5.16045 | `054538965049b1a2` | 5 / 5 |
| `nlkpp-band-n4096-r24` | banded_tight | protected banded driver (kernel Indexed) | 38 / 37 / 1 | 37 | 10 | `ac7e0be050e14f1b` | 5 / 5 |
| `nlkpp-band-n4096-r24` | banded_protected | protected banded driver (kernel Indexed) | 38 / 37 / 1 | 37 | 10 | `ac7e0be050e14f1b` | 5 / 5 |
| `nlkpp-dense-n400` | dense_lu(faer) | protected dense driver | 34 / 34 / 0 | 34 | 10 | `194829b4db8581e9` | 5 / 5 |

crate test(`stage_clients_assembly_identity.rs`, 13개)는 작은 문제에서 같은 것을 더 넓게 본다: 모든 mode(dense·색칠, 순차·thread 1–4개, dense·band 저장)의 Jacobian이 기준과 원소 단위로 bit 동일, rtol 1e-4·1e-6·1e-8에서 보호 dense driver(in-place LU와 faer LU)·보호 banded driver와 상태·통계·counter까지 동일.

색칠 경로의 Jacobian이 dense 경로와 bit 동일하려면 RHS의 성분 i가 선언한 열의 값만 읽어야 한다(pattern = 읽기 의존성). 수학적으로 0이지만 부동소수점에서 상쇄되지 않는 항은 의존성으로 선언해야 하고, 선언하지 않으면 3.3절의 검사가 거부한다.

### 3.2 RHS 호출 수

| 문제 | back end | arm | 색 수 | build당 RHS | build | 조립 RHS | 검사 RHS | stage RHS (기준 run) | run counter | callback이 센 값 |
|---|---|---|---|---|---|---|---|---|---|---|
| `bruss1d-n128-coloured` | banded_tight | seq | 5 | 6 | 48 | 288 | 8 | 384 | 680 | 680 |
| `bruss1d-n128-coloured` | banded_tight | t6 | 5 | 6 | 48 | 288 | 9 | 384 | 681 | 681 |
| `bruss1d-n128-dense` | dense_lu(in-place) | seq | - | 129 | 48 | 6192 | 0 | 384 | 6576 | 6576 |
| `bruss1d-n128-dense` | dense_lu(in-place) | t6 | - | 129 | 48 | 6192 | 9 | 384 | 6585 | 6585 |
| `bruss1d-n131072` | banded_tight | seq | 5 | 6 | 48 | 288 | 8 | 384 | 680 | 680 |
| `bruss1d-n131072` | banded_tight | t6 | 5 | 6 | 48 | 288 | 9 | 384 | 681 | 681 |
| `nlkpp-band-n4096-r24` | banded_tight | seq | 49 | 50 | 37 | 1850 | 8 | 303 | 2161 | 2161 |
| `nlkpp-band-n4096-r24` | banded_tight | t6 | 49 | 50 | 37 | 1850 | 9 | 303 | 2162 | 2162 |
| `nlkpp-band-n4096-r24` | banded_protected | seq | 49 | 50 | 37 | 1850 | 8 | 303 | 2161 | 2161 |
| `nlkpp-band-n4096-r24` | banded_protected | t6 | 49 | 50 | 37 | 1850 | 9 | 303 | 2162 | 2162 |
| `nlkpp-dense-n400` | dense_lu(faer) | seq | - | 401 | 34 | 13634 | 0 | 272 | 13906 | 13906 |
| `nlkpp-dense-n400` | dense_lu(faer) | t6 | - | 401 | 34 | 13634 | 9 | 272 | 13915 | 13915 |

검사한 client run 516개(그중 30개는 RHS callback 안의 counter로도 확인) 전부에서 조립 RHS = build × (색 수 + 1), dense 경로는 build × (n + 1)이다. 검사 RHS는 첫 선형화 한 번뿐이다: 검사 열 8개, thread mode는 기준 평가 1회 추가. 순차 dense 경로는 검사가 없다. run counter = stage RHS + 조립 RHS + 검사 RHS.

### 3.3 틀린 pattern과 순수하지 않은 RHS: 닫힌 채 실패

검사는 두 겹이다.

1. **색칠 build마다(추가 RHS 없음)**: 어떤 색의 열도 선언하지 않은 행은 그 색의 평가에서 기준 평가와 bit가 같아야 한다. 다르면 선언하지 않은 의존성이다.
2. **첫 선형화, 그리고 요청할 때**: 검사 열(기본 8개, 고르게 배치)을 호출 thread에서 한 열씩 평가한다. 색칠 경로는 선언한 원소가 조립한 원소와 bit 동일해야 하고 선언하지 않은 행은 변하지 않아야 한다. thread mode는 기준 평가도 호출 thread에서 다시 해서 pool이 낸 값과 비교한다.

실패하면 `AssemblyError`(typed)가 report handle에 남고 driver에는 `CoreError::InvalidInput`이 간다 (치명적 종류: run이 `Err`로 끝나고 재시도하지 않는다).

| 통제 | 문제 | arm | 첫 검사 | 기대 | driver 결과 | typed error | 수락 step | 직접 전체 검사 (`verify`, 모든 열) |
|---|---|---|---|---|---|---|---|---|
| 1 omitted coupling reaches a row its colour does not cover | `ring-n12` | seq | Sample(8) | 실패해야 함 | error InvalidInput | `UncoveredRowChanged { row: 11, colour: 0 }` | 0 | `UncoveredRowChanged { row: 11, colour: 0 }` |
| 1 omitted coupling reaches a row its colour does not cover | `ring-n12` | t3 | Sample(8) | 실패해야 함 | error InvalidInput | `UncoveredRowChanged { row: 11, colour: 0 }` | 0 | `UncoveredRowChanged { row: 11, colour: 0 }` |
| 2 omitted coupling aliased into a declared entry | `lower-coupled-chain-n24` | seq | Sample(8) | 실패해야 함 | error InvalidInput | `UndeclaredEntry { row: 2, column: 1 }` | 0 | `UndeclaredEntry { row: 1, column: 0 }` |
| 2 omitted coupling aliased into a declared entry | `lower-coupled-chain-n24` | t3 | Sample(8) | 실패해야 함 | error InvalidInput | `UndeclaredEntry { row: 2, column: 1 }` | 0 | `UndeclaredEntry { row: 1, column: 0 }` |
| 3 right-hand side that is not a function of its input | `impure-n16` | t3 | Sample(8) | 실패해야 함 | error InvalidInput | `NotReproducible { row: 0, column: None }` | 0 | `NotReproducible { row: 0, column: None }` |
| 3 right-hand side that is not a function of its input | `impure-n16` | t3 | Sample(8) | 실패해야 함 | error InvalidInput | `UncoveredRowChanged { row: 15, colour: 1 }` | 0 | `UncoveredRowChanged { row: 15, colour: 1 }` |
| 4 full check requested with the aliased pattern | `lower-coupled-chain-n24` | seq | All | 실패해야 함 | error InvalidInput | `UndeclaredEntry { row: 1, column: 0 }` | 0 | `UndeclaredEntry { row: 1, column: 0 }` |
| counter-example: the aliased pattern without check columns is not noticed | `lower-coupled-chain-n24` | seq | None | 통과해도 됨 | ran | - | 36 | `UndeclaredEntry { row: 1, column: 0 }` |
| positive control: the true ring pattern with a full check | `ring-n12` | t3 | All | 통과해도 됨 | ran | - | 28 | `Ok(12)` |

**검사가 보지 못하는 것**(숨기지 않는다): (a) 검사한 모든 상태에서 유한차분이 정확히 0인 미선언 원소. (b) 검사 열을 표본으로 뽑을 때, 표본에 들지 않은 열에서만 같은 색의 선언 원소에 섞여 들어가는 미선언 원소. 위 표의 반례 행이 (b)다: 검사 열을 끄면 섞인 원소는 build마다의 검사만으로는 드러나지 않는다. `CheckColumns::All`(RHS n회)이 (b)를 닫는다. 체계적인 오류(모든 행에서 같은 결합을 빠뜨림)는 표본 8열로 잡힌다.

### 3.4 wall

각 cell은 한 process에서 arm을 돌아가며(round r은 arm r mod A부터) warm-up 2회 + 측정 7회. timing lock + `taskset -c 6-11`. run 길이는 선언대로 48 시도 또는 구간 끝. 속도비 = seq arm 중앙값 / tP arm 중앙값. `(d)`는 품질 규칙(1분 load ≤ 8, arm 내 (max − min)/중앙값 ≤ 10 %)을 못 맞춰 `diagnostic_only`인 값이다. 표의 숫자는 판정에 쓴 try(없으면 마지막 try)의 값이다.

| 문제 | 자격 | back end | 색 수 | RHS 비용 µs | 시도/수락 | seq wall ms | seq의 Jacobian 비중 | 전체 풀이 속도비 P=1/2/4/6 | Jacobian phase 속도비 P=1/2/4/6 | cell 판정 |
|---|---|---|---|---|---|---|---|---|---|---|
| `nlkpp-dense-n400` | 예 | dense_lu(faer) | n=400 | 103.3 | 34/34 | 1564.78 | 91.9 % | 0.97 (d) / 1.79 (d) / 3.05 (d) / 3.89 (d) | 0.98 (d) / 1.95 (d) / 3.81 (d) / 5.09 (d) | 판정 불가 |
| `nlkpp-band-n4096-r24` (보조: 보호 kernel) | 예 | banded_protected | 49 | 92.7 | 38/37 | 478.72 | 40.9 % | - / - / - / 1.37 | - / - / - / 3.23 (d) | (판정 불가) |
| `nlkpp-band-n4096-r24` | 예 | banded_tight | 49 | 95.1 | 38/37 | 374.01 | 53.4 % | 0.87 (d) / 1.21 (d) / 1.46 (d) / 1.58 (d) | 0.84 (d) / 1.61 (d) / 2.77 (d) / 3.38 (d) | 판정 불가 |
| `bruss1d-n131072` | 예 | banded_tight | 5 | 164.0 | 48/48 | 940.59 | 19.2 % | 0.88 (d) / 0.91 (d) / 0.92 (d) / 0.94 (d) | 0.70 (d) / 0.88 (d) / 0.95 (d) / 0.99 (d) | 판정 불가 |
| `bruss1d-n128-dense` | 아니오(통제) | dense_lu(in-place) | n=128 | 0.2 | 48/48 | 5.37 | 65.0 % | 0.72 (d) / 0.83 (d) / 0.89 (d) / 0.85 (d) | 0.65 (d) / 0.79 (d) / 0.89 (d) / 0.84 (d) | 판정 불가 |
| `bruss1d-n128-coloured` | 아니오(통제) | banded_tight | 5 | 0.2 | 48/48 | 0.88 | 13.7 % | 0.75 (d) / 0.64 (d) / 0.59 (d) / 0.48 (d) | 0.27 (d) / 0.20 (d) / 0.17 (d) / 0.13 (d) | 판정 불가 |

node 판정(엄격한 규칙): **판정 불가**.

| 문제 | back end | seq Jacobian ms | seq LU ms | seq stage solve ms | client 생성 ms | Jacobian 비중 | Jacobian phase 중 RHS (추정) | wall 중 조립 RHS (추정) | Amdahl 상한 P=2/4/6 | seq client / 보호 driver wall |
|---|---|---|---|---|---|---|---|---|---|---|
| `nlkpp-dense-n400` | dense_lu(faer) | 1437.31 | 90.88 | 9.50 | 0.18 | 91.9 % | 98.0 % | 90.0 % | 1.85 / 3.21 / 4.26 | 1.02 |
| `nlkpp-band-n4096-r24` | banded_protected | 196.00 | 155.42 | 79.90 | 14.56 | 40.9 % | 87.5 % | 35.8 % | - / - / 1.52 | - |
| `nlkpp-band-n4096-r24` | banded_tight | 199.63 | 52.32 | 75.14 | 13.15 | 53.4 % | 88.1 % | 47.0 % | 1.36 / 1.67 / 1.80 | 0.79 |
| `bruss1d-n131072` | banded_tight | 180.14 | 70.18 | 431.43 | 16.53 | 19.2 % | 26.2 % | 5.0 % | 1.11 / 1.17 / 1.19 | 0.77 |
| `bruss1d-n128-dense` | dense_lu(in-place) | 3.49 | 1.03 | 0.56 | 0.05 | 65.0 % | 30.4 % | 19.8 % | 1.48 / 1.95 / 2.18 | 1.12 |
| `bruss1d-n128-coloured` | banded_tight | 0.12 | 0.06 | 0.42 | 0.02 | 13.7 % | 43.1 % | 5.9 % | 1.07 / 1.11 / 1.13 | 0.77 |

Amdahl 상한은 측정한 seq arm의 Jacobian 비중 f로 계산한 `1 / (1 - f (1 - 1/P))`다. 'Jacobian phase 중 RHS'는 build 수 × build당 RHS 수 × 보정한 RHS 비용을 seq arm의 Jacobian phase 시간으로 나눈 추정값이다: 나머지는 원소 계산·검사·back end로의 복사이며, RHS만큼 잘 병렬화되지 않는다. 마지막 열은 순차 client의 wall을 같은 유한차분 callback을 단 보호 driver의 wall로 나눈 값(진단)이다.

측정 이력(모든 try를 남긴다):

| 문제 | back end | try | 1분 load 전/후 | arm별 wall 퍼짐 % | 모든 run의 끝 상태 동일 | 권위 없는 이유 | log |
|---|---|---|---|---|---|---|---|
| `nlkpp-dense-n400` | dense_lu(faer) | 1 | 13.39 / 12.97 | t2 6.1, t4 10.1, t6 25.8, prot 8.7, seq 7.0, t1 3.4 | 예 | 1-minute load 13.39 before, 12.97 after (limit 8); t4: wall spread 0.101 > 0.10; t6: wall spread 0.258 > 0.10 | `logs/timing_nlkpp-dense-n400_try1.log` |
| `nlkpp-dense-n400` | dense_lu(faer) | 2 | 8.11 / 10.86 | t2 3.2, t4 10.5, t6 40.2, prot 4.0, seq 4.2, t1 9.0 | 예 | 1-minute load 8.11 before, 10.86 after (limit 8); t4: wall spread 0.105 > 0.10; t6: wall spread 0.402 > 0.10 | `logs/timing_nlkpp-dense-n400_try2.log` |
| `nlkpp-band-n4096-r24` | banded_protected | 1 | 19.65 / 18.25 | seq 11.0, t6 22.5 | 예 | 1-minute load 19.65 before, 18.25 after (limit 8); seq: wall spread 0.110 > 0.10; t6: wall spread 0.225 > 0.10 | `logs/timing_nlkpp-band-n4096-r24_protected_try1.log` |
| `nlkpp-band-n4096-r24` | banded_protected | 2 | 6.38 / 6.67 | seq 8.6, t6 9.6 | 예 | 모든 비율이 규칙 충족 | `logs/timing_nlkpp-band-n4096-r24_protected_try2.log` |
| `nlkpp-band-n4096-r24` | banded_tight | 1 | 12.97 / 15.0 | t2 26.9, t4 35.3, t6 46.5, prot 15.3, seq 12.2, t1 18.7 | 예 | 1-minute load 12.97 before, 15.0 after (limit 8); seq: wall spread 0.122 > 0.10; t1: wall spread 0.187 > 0.10; t2: wall spread 0.269 > 0.10; t4: wall spread 0.353 > 0.10; t6: wall spread 0.465 > 0.10 | `logs/timing_nlkpp-band-n4096-r24_try1.log` |
| `nlkpp-band-n4096-r24` | banded_tight | 2 | 8.21 / 8.14 | t2 8.5, t4 19.0, t6 7.7, prot 1.6, seq 5.3, t1 7.1 | 예 | 1-minute load 8.21 before, 8.14 after (limit 8); t4: wall spread 0.190 > 0.10 | `logs/timing_nlkpp-band-n4096-r24_try2.log` |
| `bruss1d-n131072` | banded_tight | 1 | 15.0 / 19.65 | t2 16.8, t4 17.4, t6 19.2, prot 16.7, seq 13.9, t1 47.4 | 예 | 1-minute load 15.0 before, 19.65 after (limit 8); seq: wall spread 0.139 > 0.10; t1: wall spread 0.474 > 0.10; t2: wall spread 0.168 > 0.10; t4: wall spread 0.174 > 0.10; t6: wall spread 0.192 > 0.10 | `logs/timing_bruss1d-n131072_try1.log` |
| `bruss1d-n131072` | banded_tight | 2 | 9.34 / 7.31 | t2 33.4, t4 16.5, t6 10.5, prot 6.9, seq 5.0, t1 7.8 | 예 | 1-minute load 9.34 before, 7.31 after (limit 8); t2: wall spread 0.334 > 0.10; t4: wall spread 0.165 > 0.10; t6: wall spread 0.105 > 0.10 | `logs/timing_bruss1d-n131072_try2.log` |
| `bruss1d-n128-dense` | dense_lu(in-place) | 1 | 19.65 / 19.65 | t2 9.2, t4 25.4, t6 20.2, prot 7.3, seq 43.8, t1 7.1 | 예 | 1-minute load 19.65 before, 19.65 after (limit 8); seq: wall spread 0.438 > 0.10; t4: wall spread 0.254 > 0.10; t6: wall spread 0.202 > 0.10 | `logs/timing_bruss1d-n128-dense_try1.log` |
| `bruss1d-n128-dense` | dense_lu(in-place) | 2 | 6.55 / 6.55 | t2 4.7, t4 5.1, t6 13.0, prot 3.0, seq 29.2, t1 5.2 | 예 | seq: wall spread 0.292 > 0.10; t6: wall spread 0.130 > 0.10 | `logs/timing_bruss1d-n128-dense_try2.log` |
| `bruss1d-n128-coloured` | banded_tight | 1 | 19.65 / 19.65 | t2 20.3, t4 21.2, t6 16.1, prot 23.8, seq 32.8, t1 20.1 | 예 | 1-minute load 19.65 before, 19.65 after (limit 8); seq: wall spread 0.328 > 0.10; t1: wall spread 0.201 > 0.10; t2: wall spread 0.203 > 0.10; t4: wall spread 0.212 > 0.10; t6: wall spread 0.161 > 0.10 | `logs/timing_bruss1d-n128-coloured_try1.log` |
| `bruss1d-n128-coloured` | banded_tight | 2 | 17.72 / 17.72 | t2 22.1, t4 14.3, t6 13.0, prot 7.4, seq 12.8, t1 6.4 | 예 | 1-minute load 17.72 before, 17.72 after (limit 8); seq: wall spread 0.128 > 0.10; t2: wall spread 0.221 > 0.10; t4: wall spread 0.143 > 0.10; t6: wall spread 0.130 > 0.10 | `logs/timing_bruss1d-n128-coloured_try2.log` |

**정합 오차**: 한 cell의 모든 arm은 같은 수락 상태를 bit 단위로 낸다(timing run마다 끝 상태 fingerprint를 확인, 위 표). 따라서 달성 오차는 arm마다 같은 수이고, 속도비는 정확히 같은 오차에서의 비교다. 달성 오차 수준 자체는 7절에 적었다.

## 4. thread 병렬이 이득인 영역과 손해인 영역

병렬화되는 것은 Jacobian 조립의 RHS뿐이다. 8개 stage의 RHS와 LU, stage solve는 순차로 남는다. 그래서 전체 풀이의 이득은 Jacobian 비중 f가 정한다(3.4절 둘째 표).

- **이득**: dense 유한차분(build당 n + 1 RHS, stage는 8 RHS)에서 RHS가 비쌀 때. f가 1에 가깝다.
- **조건부**: 색칠 경로. build당 RHS가 색 수 + 1뿐이므로 f ≤ (색 수 + 1)/(색 수 + 9)이고, 선형대수 시간이 더해지면 더 작다. 색 수가 많고(넓은 band) LU가 싸야 이득이 난다. back end의 kernel 속도가 f를 바꾼다(보조 cell 참조).
- **이득 없음**: 좁은 band의 고전적 MOL(색 5개). 구조상 Jacobian이 step의 작은 부분이다. V04의 측정(FD band에서 Jacobian phase 3.6–9.3 %)과 같은 결론이다.
- **손해**: RHS가 싼 문제(통제 cell). pool에 일을 넘기고 깨우는 비용이 RHS보다 크다. P = 1 열은 병렬 이득 없이 넘기는 비용만 보여 준다.

기준 평가도 pool의 작업 하나다. 작업 수(색 수 + 1, dense는 n + 1)가 P보다 적으면 남는 worker는 할 일이 없다. 원소 계산과 행별 모으기도 pool에서 하지만, 이 부분과 back end로의 복사는 memory 대역폭에 묶여 RHS만큼 빨라지지 않는다. 그래서 Jacobian phase의 속도비는 P보다 작고, RHS가 Jacobian phase에서 차지하는 몫(3.4절 둘째 표)이 작을수록 더 작다.

측정값으로 본 순서: `nlkpp-dense-n400`: 조립 RHS가 wall의 90.0 %, P = 2·4·6 중 가장 큰 전체 속도비 3.89; `nlkpp-band-n4096-r24`: 조립 RHS가 wall의 47.0 %, P = 2·4·6 중 가장 큰 전체 속도비 1.58; `bruss1d-n131072`: 조립 RHS가 wall의 5.0 %, P = 2·4·6 중 가장 큰 전체 속도비 0.94; `bruss1d-n128-dense`: 조립 RHS가 wall의 19.8 %, P = 2·4·6 중 가장 큰 전체 속도비 0.89; `bruss1d-n128-coloured`: 조립 RHS가 wall의 5.9 %, P = 2·4·6 중 가장 큰 전체 속도비 0.64.

## 5. 순도 계약 (thread mode를 켜는 사람이 수락하는 것)

thread build에서는 RHS가 여러 thread에서 동시에, 서로 다른 이동된 상태로, 정해지지 않은 순서로 불린다. callback 형이 요구하는 `Send + Sync`는 memory 안전만 보장한다. `RhsPurity::pure_and_reentrant()`를 부르는 사람은 다음을 보증한다.

1. 결과는 `(t, y)`만의 함수다: 호출 사이에 넘어가는 상태가 없다(직전 호출을 key로 하는 cache, 결과에 들어가는 counter, 난수, 적응 table).
2. 동시 호출이 서로에게 영향을 주지 않는다: 공유 scratch가 없거나, 있어도 결과를 바꿀 수 없다.
3. 결과가 실행 thread에 의존하지 않는다: thread-local 상태가 없고, 모든 thread의 부동소수점 환경(반올림 mode, flush-to-zero)이 같다.

계약을 어기면 Jacobian이 scheduling에 의존한다. client는 계약을 증명할 수 없다. 대신 첫 선형화에서 기준 평가와 검사 열을 호출 thread에서 다시 계산해 pool의 결과와 bit 비교하고, 다르면 `AssemblyError::NotReproducible`로 run을 끝낸다(3.3절 통제 3). 이 검사는 표본이다: 통과가 순도의 증명은 아니다. 순차 mode에는 이 계약이 필요 없다(보호 경로와 같은 호출 순서).

실패 시의 차이 하나: RHS가 `Err`를 내거나 원소가 넘치면 thread build도 순차 build가 만났을 첫 실패(작업 순서, 기준 평가 먼저)를 그대로 돌려주므로 driver의 반응과 궤적은 같다. 다만 그때까지 build의 모든 RHS가 이미 호출되었으므로 실패한 build의 `rhs_calls`는 순차보다 크다.

## 6. facade 대조와 facade의 색칠 경로 (별도 측정)

**인용 확인**: `vigilode::Rodas5p::solve`(`Jacobian::FiniteDifference`, vector atol)와, 같은 설정으로 client driver + 조립 client를 돌린 결과. facade의 `fd.rs` code가 실제로 실행된 쪽과 비교한다.

| 경우 | facade가 부른 entry | 시도 / 수락 | facade FD RHS | client | client 조립 RHS | 끝 상태에서 bit가 다른 성분 | step 수 동일 |
|---|---|---|---|---|---|---|---|
| dense, nonlocal KPP n = 96 | `integrate_rodas5p_fast_observed_with_atol` | 22 / 22 | 2134 | seq + dense_lu(Faer) | 2134 | 0 | 예 |
| dense, nonlocal KPP n = 96 | `integrate_rodas5p_fast_observed_with_atol` | 22 / 22 | 2134 | t3 + dense_lu(Faer) | 2134 | 0 | 예 |
| dense, Brusselator 32 cells | `integrate_rodas5p_fast_observed_with_atol` | 101 / 92 | 5980 | seq + dense_lu(InPlace) | 5980 | 0 | 예 |
| dense, Brusselator 32 cells | `integrate_rodas5p_fast_observed_with_atol` | 101 / 92 | 5980 | t3 + dense_lu(InPlace) | 5980 | 0 | 예 |
| banded (2, 2), Brusselator 64 cells | `integrate_rodas5p_fast_banded_observed_with_atol` | 101 / 92 | 552 | seq + banded_protected | 552 | 0 | 예 |
| banded (2, 2), Brusselator 64 cells | `integrate_rodas5p_fast_banded_observed_with_atol` | 101 / 92 | 552 | t3 + banded_protected | 552 | 0 | 예 |

**facade의 색칠 경로**(`Jacobian::sparse_jvp` + `crates/vigilode/src/coloring.rs`). 무효 처리된 ledger 행 L-0214(simulation에서 나온 수)를 실행한 수로 대신한다. V08의 동결 판별식은 아니다.

- 문제: 1D Brusselator, 64 cells (n = 128), analytic JVP; not a holdout configuration. pattern은 해석 Jacobian의 실제 0 아닌 원소 508개, band (2, 2).
- 색 수: **4** (꽉 찬 band로 묶으면 5). colouring 유효: 예.
- Jacobian build당 JVP 호출: **4.0** (callback이 센 368회 / build 92회; 시도 101, 수락 92, entry `integrate_rodas5p_fast_banded_observed_with_atol`).
- 복원한 값 대 해석 dense Jacobian: y0에서 508개 원소 중 bit가 다른 것 0개(최대 차 0), 끝 상태에서 0개(최대 차 0).
- 끝 상태 대 해석 band Jacobian(`Jacobian::banded`) 풀이: bit가 다른 성분 0개, 최대 차 0.
- 참고: 같은 band의 facade 유한차분은 build당 RHS 6.0회이고, 끝 상태는 색칠 JVP 풀이와 128개 성분에서 다르다(최대 차 5.75e-12; 유한차분 Jacobian이므로 당연하다).
- **발견**: facade `Stats`의 `counters.jvp_calls`는 0, `driver_counters.jvp_calls`는 0이다. band 채우기 안에서 부른 JVP 368회가 facade의 work 기록에 잡히지 않는다.

## 7. 그 밖의 측정

- **할당**(`stage_clients_assembly_alloc.rs`, 별도 binary): 순차 mode run 42개, 정상 구간 step 3158개에서 할당 합계 **0** (PASS). 대상: TimedStageClient(coloured route, BandedStageLu (protected kernel)); TimedStageClient(coloured route, BandedTightStageLu); TimedStageClient(coloured route, DenseLu(InPlace)); TimedStageClient(dense route, DenseLu(InPlace)); coloured route, BandedStageLu (protected kernel); coloured route, BandedTightStageLu; coloured route, DenseLu(InPlace); dense route, DenseLu(InPlace). thread mode는 보고만 한다: dense route, 2 workers, DenseLu(InPlace) rtol 0.0001: step 40개에 0회; dense route, 2 workers, DenseLu(InPlace) rtol 1e-06: step 80개에 1회; coloured route, 2 workers, BandedTightStageLu rtol 0.0001: step 40개에 1회; coloured route, 2 workers, BandedTightStageLu rtol 1e-06: step 80개에 1회.
- **instructions:u**(진단 전용, arm당 풀이 1회, process 전체): `bruss1d-n131072` seq: 9.590 G; `bruss1d-n131072` t1: 10.340 G; `bruss1d-n131072` t2: 10.342 G; `bruss1d-n131072` t4: 10.353 G; `bruss1d-n131072` t6: 10.365 G; `nlkpp-band-n4096-r24` seq: 5.970 G; `nlkpp-band-n4096-r24` t1: 6.112 G; `nlkpp-band-n4096-r24` t2: 6.113 G; `nlkpp-band-n4096-r24` t4: 6.119 G; `nlkpp-band-n4096-r24` t6: 6.127 G; `nlkpp-dense-n400` seq: 17.591 G; `nlkpp-dense-n400` t1: 17.634 G; `nlkpp-dense-n400` t2: 17.635 G; `nlkpp-dense-n400` t4: 17.639 G; `nlkpp-dense-n400` t6: 17.649 G. thread는 instruction을 줄이지 않는다(조금 늘린다). 이 node의 판별식이 아니다.
- **달성 오차** `nlkpp-dense-n400` (t = 10): 독립 기준(SciPy LSODA, rtol 1e-12, atol 1e-14, 해석 Jacobian, NumPy RHS) 대비 최대 절대 오차 3e-07, 허용오차 단위로 최대 0.122(rms 0.0418); 기준의 불확도(rtol 1e-10 풀이와의 최대 차) 8.85e-10. 탐색적 기록이며 판별식이 아니다.
- **달성 오차** `nlkpp-band-n4096-r24` (t = 10): 독립 기준(SciPy LSODA, rtol 1e-12, atol 1e-14, 해석 Jacobian, NumPy RHS) 대비 최대 절대 오차 6.62e-08, 허용오차 단위로 최대 0.0445(rms 0.00818); 기준의 불확도(rtol 1e-10 풀이와의 최대 차) 5.03e-10. 탐색적 기록이며 판별식이 아니다.
- **달성 오차** `bruss1d-n131072` (t = 5.16045): 독립 기준(SciPy LSODA, rtol 1e-12, atol 1e-14, 해석 Jacobian, NumPy RHS) 대비 최대 절대 오차 3.82e-06, 허용오차 단위로 최대 3.54(rms 1.6); 기준의 불확도(rtol 1e-10 풀이와의 최대 차) 3.15e-09. 탐색적 기록이며 판별식이 아니다.

## 8. 재현

```bash
cd research/production_loop_20261116/jacobian_assembly
./run_measurements.sh build && ./run_measurements.sh failclosed
./run_measurements.sh identity <cell>          # 5개 cell
./run_measurements.sh timing <cell> <try>      # timing lock, core 6-11
./run_measurements.sh runner-build && ./run_measurements.sh facade
python3 -I build_results.py && python3 -I build_report.py
```

| 검사 | 종료 code | 결과 |
|---|---|---|
| tests of this node (identity, alloc; measure is ignored) | 0 | 4 test binaries: 14 passed, 0 failed, 1 ignored |
| source-pinning sentinels | 0 | 2 test binaries: 61 passed, 0 failed, 0 ignored |
| stage client kit tests of earlier nodes (unchanged files) | 0 | 3 test binaries: 14 passed, 0 failed, 3 ignored |
| clippy -D warnings (lib and this node's test targets) | 0 |     Finished `release` profile [optimized] target(s) in 20.31s |
| rustfmt --check | 0 | no output |
| loop checker | 0 | Scope: checkpoint reproducibility. Production HOLD; no new numerical qualification. |
| strict JSON load of the JSON files of this node | 0 | RESULTS.json strict JSON ok |

## 9. 열린 문제

- 색칠 경로의 전체 풀이 이득은 문제 구조(색 수, 선형대수 비용)에 달려 있다. 넓은 band에서도 back end kernel에 따라 판정이 갈린다.
- 검사 열은 표본이다. 표본 밖 열의 섞인 원소와, 검사 시점에 유한차분이 0인 원소는 보지 못한다.
- dense back end의 값 복사(n²)와 thread mode의 작업별 상태 복사는 RHS가 싼 문제에서 손해다.
- facade `Jacobian::sparse_jvp`의 band 경로는 JVP 호출을 `Stats`에 기록하지 않는다(6절).
- client는 opt-in으로만 둔다. 기본 routing과 보호 source는 바꾸지 않았다.
