# V04 — RODAS5P step의 phase별 비용 해부 (loop L1, wave 2)

작성 2026-11-17. 노드 `V04_STAGE_COST_ANATOMY`, 브랜치 `research/production-loop-20261116-w2-v04`. 이 문서는 `build_report.py`가 `RESULTS.json`·`AMDAHL.json`에서 만든다. 본문의 수치는 렌더링할 때 두 파일에서 읽어 넣으며, 사람이 입력한 측정값은 없다.

## 0. 요약

- 격자 60개 cell 중 59개에 wall protocol을 실행했다. 라벨: `authoritative` 29개, `diagnostic_only` 30개(그중 n < 128이라 instruction 수가 1차 값인 것 9개), 미측정 1개. 보호 driver와의 bit-동일성은 526쌍 전부에서 성립했다.

- **8단계 사슬(8 RHS + 8 solve)이 병목인 영역**: 1D banded(사슬 70.5–75.1 %, 그 대부분이 solve 65.4–69.7 %), RHS가 비싼 경우(dense와 1D banded의 배율 100에서 RHS 84.4–94.0 %, 배율 10에서 35.1–61.7 %), matrix-free(8회 Krylov solve 구간 99.8–99.9 %). 소형 dense(n ≤ 32)는 instruction 기준 사슬이 41.6–50.3 %이다. n = 8에서는 나머지(벡터 연산·controller·출력)가 37.6–46.7 %로 사슬과 같은 규모이고, n = 32에서는 10.9–24.7 %다.

- **사슬이 병목이 아닌 영역**: dense n ≥ 128은 factorisation이 지배한다(in-place LU 58.2–97.9 %, faer LU 61.3–86.3 %). 2D/3D banded 직접법도 factorisation 72.4–94.0 %이다. FD Jacobian을 쓰면 dense n ≥ 128에서 Jacobian이 46.3–93.8 %를 차지한다.

- **V07 stage b 착수 조건**((J+LU)/사슬 ∈ [0.15, 0.92]이고 수락 step당 wall ≥ 500 µs)을 만족하는 regime은 7개이고, 그중 `authoritative` 행은 3개다(표 T7). 착수 조건은 충족된다. 다만 이 regime들의 상한은 1.14–1.20(Jacobian만 겹치면 1.01–1.19)로, stage b의 통과 기준 1.15와 거의 같다.

- **lever별 결론**(4절): Jacobian 병렬 조립(c)은 FD Jacobian 영역에서, LU lever(d, e)는 dense n ≥ 128과 2D/3D banded에서, RHS 병렬화(g)는 RHS 배율 10 이상에서만 상한이 의미 있다. level 구조 tableau(a)와 투기적 J/LU(b)는 사슬이 큰 1D banded·고비용 RHS에서만 여지가 있다. matrix-free의 shared basis(f)는 상한이 가장 크지만, 그 대부분은 보호 driver의 GMRES가 stage solve마다 restart 주기(40열)를 끝까지 돌고 그 열들을 2-pass MGS로 직교화하는 데서 나온다.

- **측정 중 드러난 사실**(3.6절): (1) 보호 dense driver의 faer 경로는 n = 128, 512에서 같은 궤적의 client arm보다 wall이 느렸다. instruction 수는 같고, 원인은 factorisation마다의 할당과 page fault다(6.2절). (2) matrix-free GMRES의 반복 수는 restart 주기(40열) 단위로만 늘고, 주기 안에서는 일찍 멈추지 않는다. (3) FD JVP는 보호 선형 허용오차 1e-10에서 쓸 수 없다.

## 1. 방법

### 1.1 phase 정의

attempt(수락·거절을 모두 포함한 step 시도) 하나의 평균 비용을 다음 phase로 나눴다.

| phase | 직접법 (dense, banded) | matrix-free |
|---|---|---|
| Jacobian | `TimedStageClient`의 `linearize_ns`. FD Jacobian이면 그 안의 RHS 호출을 포함 | 없음 (JVP callback) |
| factor | `factor_ns`: W = I/(hγ) − J 조립과 LU | 없음 |
| solve | `solve_ns`: attempt당 8회 삼각 solve | stage solve 구간: 앞선 RHS 호출이 끝난 시각부터 다음 RHS 호출 시작(마지막 stage는 attempt observer)까지. JVP, Krylov 직교화, 최소제곱, stage 벡터 연산을 포함 |
| RHS | driver가 부르는 RHS callback 둘레의 wall clock (새 상태의 f(t, y) 1회 + stage 7회) | 같음 |
| 나머지 | total − (J + factor + solve + RHS): stage 벡터 연산, 오차 norm, controller, 출력 | total − (RHS + solve 구간): controller, 출력. 과제 문장의 정의(total − RHS − JVP)도 함께 기록 |

- **plain arm**은 보호 driver 그대로다: dense `integrate_rodas5p_fast_observed_with_atol`, banded `integrate_rodas5p_fast_banded_observed_with_atol`(Indexed kernel), matrix-free `integrate_rodas5p_mf_fast_observed_gmres_into` + `RoutingOptions::default().matrix_free`(GMRES, restart 40, rtol 1e-10, 전처리 없음, Arnoldi 예산 200).
- **instrumented arm**은 `integrate_rodas5p_fast_client_observed_with_atol` + `TimedStageClient(DenseLuStageClient | BandedStageLu)` + RHS clock이다. matrix-free는 `integrate_rodas5p_mf_fast_observed_traced`에 RHS·JVP clock과 attempt observer를 붙였다. crate 소스는 바꾸지 않았다.
- dense LU mode는 보호 정책(`DenseLuMode::protected_choice_at`)이 고르는 것만 쟀다: n ≤ 64 또는 nonzero ≤ 10 %이면 in-place zero-skipping, 아니면 faer. 정책이 고르지 않는 mode는 보호 대응 run이 없어서 재지 않았다.
- RHS 배율 k는 같은 RHS를 k번 계산하고 마지막 결과만 남긴다. 두 arm과 FD Jacobian에 똑같이 적용된다.
- FD Jacobian은 Jacobian callback 안의 전진 차분이다: dense n + 1회, banded는 band의 Curtis–Powell–Reid 묶음으로 lower + upper + 2회. 이 RHS 호출은 Jacobian phase에 들어가고 `fd_rhs_calls`로 따로 센다.

### 1.2 문제

| family | 내용 | 쓰인 곳 |
|---|---|---|
| `hires` | HIRES (n = 8). U11에서 소진된 구성이므로 개발용으로만 쓰고 holdout이라 부르지 않는다 | dense n = 8 |
| `bruss1d` | 1D Brusselator MOL, 종 interleave, 확산계수 (cells+1)²/50, band 2/2 | dense 저장 n = 32, 128, 512, 2048 (in-place LU); banded n = 1e3, 1e4, 1e5 |
| `radnet` | 고리 위의 복사·전도 집중 열망: 모든 노드 쌍의 T⁴ 교환(Jacobian 전 원소 nonzero), 열용량이 4자릿수에 걸침. 물리 모형이고 매개변수는 식으로 생성 | dense n = 8, 32 (in-place), 128, 512, 2048 (faer) |
| `ac2d`, `ac3d` | Allen–Cahn, cell-centred FV, zero-flux 경계, D = 0.01 (2D), 0.04 (3D) | banded 2D 32², 64², 128²; banded 3D 8³, 16³; matrix-free 2D 64², 128², 256²; 3D 16³, 32³ |
| `bruss2d` | 2D Brusselator (2종), α = 0.02, zero-flux 경계, band 2m/2m | banded 32², 64², 128²; matrix-free 128² |

정의는 `runner/src/problems.rs`에 있다. `runner selftest`가 해석 Jacobian·band·JVP·FD가 서로 맞는지 확인한다(36개 검사).

### 1.3 프로토콜

1. `PREDECLARED.json`을 pilot과 모든 측정보다 먼저 commit했다(격자, 규칙, 라벨, Amdahl 식).
2. **pilot**: cell마다 plain arm을 attempt 12회로 한 번 실행해 attempt당 시간 c를 얻었다(phase는 재지 않음).
3. **sizing**(`SIZING.json`): 20c ≤ 60 s이면 full protocol — attempt 예산 A = min(1000, max(20, ⌊1.5 s / c⌋)), 2 warm-up + 7 반복. 그렇지 않고 8c ≤ 150 s이면 reduced protocol(A = 8, 1 + 3). run은 tf에 닿거나 예산을 다 쓰면 끝난다(줄인 horizon). 예산은 두 arm에 같다.
4. **wall protocol**: `taskset -c 10`, 한 프로세스 안에서 반복마다 plain 1회와 instrumented 1회, 순서는 반복마다 교대. cell 전후의 load average를 기록.
5. n < 128은 timer 비용이 지배하므로 callgrind로 phase별 instruction 수를 셌다. clock이 없는 marker 심볼(`v04_phase_*`)의 inclusive Ir이다.

### 1.4 허용 기준(frozen)과 라벨

n ≥ 128인 행은 다음을 모두 만족할 때만 `authoritative`다. (i) instrumented/plain wall(중앙값의 비) ≤ 1.02. (ii) 잰 phase의 합이 instrumented total의 0.90–1.00. (iii) 모든 쌍에서 attempt·수락·거절 수, 모든 work counter, 종료 시각과 종료 상태의 모든 비트가 보호 driver와 같음. 그리고 full protocol(고정 core, arm 교차, 2 + 7, load 기록). 그 밖은 `diagnostic_only`다. reduced protocol 행은 세 기준을 계산해 적되 라벨은 `diagnostic_only`다. n < 128의 wall은 `diagnostic_only`이고 instruction 수가 1차 값이다.

matrix-free 행의 (ii)는 과제 문장대로 RHS + JVP callback 시간만 더해 판정했다. solve 구간까지 더한 합은 확장 값으로 따로 적었고 라벨에는 쓰지 않았다(표 T2의 마지막 열).

## 2. 결과 표

표는 `build_amdahl.py`가 생성한다(`REPORT_TABLES.md`와 같은 내용). `AUTH` = authoritative, `diag` = diagnostic_only.

<!-- BEGIN GENERATED TABLES -->
<!-- generated by build_amdahl.py from RESULTS.json and AMDAHL.json; do not edit -->

### T1. Wall anatomy, direct drivers (per attempt, medians over the timed repetitions)

| cell | n | LU | attempts | accepted | plain us/attempt | instr us/attempt | J % | LU % | solve % | RHS % | rest % | phase sum | instr/plain | identical | load before/after | label |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dense.hires.n8.analytic.x1 | 8 | in_place_zero_skipping | 210 | 204 | 1.19 | 1.99 | 1.4 | 6.9 | 38.3 | 9.6 | 43.8 | 0.562 | 1.680 | yes | 2.5/2.5 | diag |
| dense.hires.n8.fd.x1 | 8 | in_place_zero_skipping | 210 | 204 | 1.28 | 2.09 | 6.3 | 6.6 | 36.8 | 9.2 | 40.9 | 0.591 | 1.636 | yes | 2.5/2.5 | diag |
| dense.bruss1d.n32.analytic.x1 | 32 | in_place_zero_skipping | 93 | 82 | 5.23 | 6.02 | 1.0 | 20.6 | 48.7 | 6.6 | 22.8 | 0.772 | 1.151 | yes | 2.5/2.5 | diag |
| dense.bruss1d.n32.fd.x1 | 32 | in_place_zero_skipping | 93 | 82 | 7.01 | 7.83 | 23.4 | 16.0 | 37.4 | 5.1 | 17.8 | 0.822 | 1.118 | yes | 2.5/2.5 | diag |
| dense.bruss1d.n32.analytic.x10 | 32 | in_place_zero_skipping | 93 | 82 | 7.74 | 8.52 | 0.7 | 14.9 | 34.4 | 33.7 | 16.2 | 0.838 | 1.101 | yes | 2.5/2.5 | diag |
| dense.bruss1d.n32.analytic.x100 | 32 | in_place_zero_skipping | 93 | 82 | 32 | 32.7 | 0.2 | 3.9 | 9.0 | 82.7 | 4.3 | 0.957 | 1.024 | yes | 2.5/2.5 | diag |
| dense.bruss1d.n128.analytic.x1 | 128 | in_place_zero_skipping | 92 | 82 | 37.5 | 38.6 | 0.5 | 58.2 | 29.7 | 3.1 | 8.5 | 0.915 | 1.030 | yes | 2.5/2.5 | diag |
| dense.bruss1d.n128.fd.x1 | 128 | in_place_zero_skipping | 92 | 82 | 70.7 | 71.7 | 46.3 | 31.4 | 16.1 | 1.7 | 4.5 | 0.955 | 1.015 | yes | 2.5/2.5 | AUTH |
| dense.bruss1d.n512.analytic.x1 | 512 | in_place_zero_skipping | 92 | 82 | 738 | 733 | 1.2 | 90.3 | 6.3 | 0.6 | 1.7 | 0.983 | 0.994 | yes | 2.5/2.5 | AUTH |
| dense.bruss1d.n512.fd.x1 | 512 | in_place_zero_skipping | 92 | 82 | 2019 | 2010 | 63.5 | 33.3 | 2.3 | 0.2 | 0.7 | 0.993 | 0.995 | yes | 2.5/2.7 | AUTH |
| dense.bruss1d.n2048.analytic.x1 | 2048 | in_place_zero_skipping | 77 | 68 | 17522 | 17466 | 0.3 | 97.9 | 1.1 | 0.1 | 0.6 | 0.994 | 0.997 | yes | 2.7/2.7 | AUTH |
| dense.bruss1d.n2048.fd.x1 | 2048 | in_place_zero_skipping | 31 | 30 | 48069 | 47998 | 59.4 | 39.6 | 0.4 | 0.0 | 0.5 | 0.995 | 0.999 | yes | 2.7/2.8 | AUTH |
| dense.radnet.n8.analytic.x1 | 8 | in_place_zero_skipping | 79 | 79 | 1.87 | 2.77 | 2.8 | 8.0 | 36.4 | 20.0 | 32.7 | 0.673 | 1.479 | yes | 2.8/2.8 | diag |
| dense.radnet.n32.analytic.x1 | 32 | in_place_zero_skipping | 84 | 84 | 13.7 | 14.3 | 2.2 | 24.1 | 46.8 | 16.8 | 10.1 | 0.899 | 1.038 | yes | 2.8/2.8 | diag |
| dense.radnet.n32.fd.x1 | 32 | in_place_zero_skipping | 84 | 84 | 23.1 | 24 | 41.6 | 14.6 | 27.8 | 9.8 | 6.0 | 0.940 | 1.041 | yes | 2.8/2.8 | diag |
| dense.radnet.n128.analytic.x1 | 128 | faer | 85 | 85 | 201 | 170 | 2.0 | 61.3 | 19.6 | 14.4 | 2.6 | 0.974 | 0.845 | yes | 2.8/2.8 | AUTH |
| dense.radnet.n128.analytic.x10 | 128 | faer | 85 | 85 | 418 | 393 | 0.9 | 27.1 | 8.8 | 61.7 | 1.2 | 0.988 | 0.939 | yes | 2.8/2.8 | AUTH |
| dense.radnet.n128.analytic.x100 | 128 | faer | 85 | 85 | 2590 | 2574 | 0.1 | 4.1 | 1.4 | 94.0 | 0.2 | 0.998 | 0.994 | yes | 2.8/2.9 | AUTH |
| dense.radnet.n128.fd.x1 | 128 | faer | 85 | 85 | 607 | 578 | 70.9 | 18.1 | 5.9 | 4.3 | 0.8 | 0.992 | 0.952 | yes | 2.9/2.9 | AUTH |
| dense.radnet.n128.fd.x10 | 128 | faer | 85 | 85 | 4324 | 4324 | 90.4 | 3.2 | 0.8 | 5.6 | 0.1 | 0.999 | 1.000 | yes | 2.9/2.7 | AUTH |
| dense.radnet.n128.fd.x100 | 128 | faer | 35 | 35 | 41505 | 41440 | 93.8 | 0.3 | 0.1 | 5.8 | 0.0 | 1.000 | 0.998 | yes | 2.7/2.5 | AUTH |
| dense.radnet.n512.analytic.x1 | 512 | faer | 86 | 86 | 4806 | 4038 | 1.4 | 78.6 | 10.0 | 9.6 | 0.4 | 0.996 | 0.840 | yes | 2.5/2.5 | AUTH |
| dense.radnet.n512.fd.x1 | 512 | faer | 48 | 48 | 30980 | 31841 | 82.1 | 15.1 | 1.4 | 1.3 | 0.1 | 0.999 | 1.028 | yes | 2.5/3.2 | diag |
| dense.radnet.n512.analytic.x10 | 512 | faer | 86 | 86 | 8515 | 7540 | 0.8 | 43.0 | 5.5 | 50.5 | 0.2 | 0.998 | 0.885 | yes | 3.2/3.2 | AUTH |
| dense.radnet.n512.analytic.x100 | 512 | faer | 34 | 34 | 42389 | 42323 | 0.3 | 8.6 | 1.0 | 90.1 | 0.0 | 1.000 | 0.998 | yes | 3.2/3.6 | AUTH |
| dense.radnet.n2048.analytic.x1 | 2048 | faer | 20 | 20 | 217739 | 217755 | 1.7 | 86.3 | 6.1 | 5.5 | 0.3 | 0.997 | 1.000 | yes | 3.6/3.3 | AUTH |
| banded_1d.bruss1d.n1000.analytic.x1 | 1000 | banded_indexed | 92 | 82 | 150 | 151 | 0.7 | 11.3 | 69.7 | 5.4 | 12.9 | 0.871 | 1.006 | yes | 3.3/3.3 | diag |
| banded_1d.bruss1d.n1000.fd.x1 | 1000 | banded_indexed | 92 | 82 | 163 | 164 | 8.3 | 10.5 | 64.2 | 4.9 | 12.0 | 0.880 | 1.004 | yes | 3.3/3.3 | diag |
| banded_1d.bruss1d.n10000.analytic.x1 | 10000 | banded_indexed | 92 | 82 | 1566 | 1566 | 0.8 | 11.7 | 67.8 | 5.3 | 14.7 | 0.853 | 1.000 | yes | 3.3/3.2 | diag |
| banded_1d.bruss1d.n10000.fd.x1 | 10000 | banded_indexed | 92 | 82 | 1706 | 1705 | 8.6 | 10.6 | 62.6 | 4.7 | 13.6 | 0.864 | 0.999 | yes | 3.2/3.2 | diag |
| banded_1d.bruss1d.n100000.analytic.x1 | 100000 | banded_indexed | 88 | 79 | 16204 | 16285 | 2.0 | 12.3 | 65.4 | 5.1 | 15.4 | 0.846 | 1.005 | yes | 3.2/3.4 | diag |
| banded_1d.bruss1d.n100000.fd.x1 | 100000 | banded_indexed | 79 | 70 | 17549 | 17509 | 9.3 | 11.3 | 60.8 | 4.7 | 13.9 | 0.861 | 0.998 | yes | 3.4/3.2 | diag |
| banded_1d.bruss1d.n10000.analytic.x10 | 10000 | banded_indexed | 92 | 82 | 2266 | 2266 | 0.6 | 7.9 | 46.9 | 35.1 | 9.4 | 0.906 | 1.000 | yes | 3.2/3.2 | AUTH |
| banded_1d.bruss1d.n10000.analytic.x100 | 10000 | banded_indexed | 92 | 82 | 9441 | 9453 | 0.1 | 1.9 | 11.3 | 84.4 | 2.3 | 0.977 | 1.001 | yes | 3.2/3.0 | AUTH |
| banded_1d.bruss1d.n10000.fd.x10 | 10000 | banded_indexed | 92 | 82 | 2884 | 2881 | 21.9 | 6.3 | 36.8 | 27.7 | 7.5 | 0.925 | 0.999 | yes | 3.0/3.0 | AUTH |
| banded_1d.bruss1d.n10000.fd.x100 | 10000 | banded_indexed | 92 | 82 | 15016 | 14979 | 36.8 | 1.2 | 7.1 | 53.4 | 1.5 | 0.985 | 0.998 | yes | 3.0/4.4 | AUTH |
| banded_2d.ac2d.g32.analytic.x1 | 1024 | banded_indexed | 39 | 38 | 2442 | 2436 | 0.4 | 72.4 | 25.3 | 0.8 | 1.1 | 0.989 | 0.998 | yes | 4.4/4.2 | AUTH |
| banded_2d.ac2d.g64.analytic.x1 | 4096 | banded_indexed | 39 | 38 | 36804 | 34260 | 0.6 | 83.6 | 15.3 | 0.2 | 0.3 | 0.997 | 0.931 | yes | 4.2/7.2 | AUTH |
| banded_2d.ac2d.g64.fd.x1 | 4096 | banded_indexed | 39 | 38 | 40304 | 41805 | 6.8 | 78.1 | 14.5 | 0.2 | 0.4 | 0.996 | 1.037 | yes | 7.2/14.4 | diag |
| banded_2d.ac2d.g64.analytic.x10 | 4096 | banded_indexed | 39 | 38 | 61463 | 61534 | 0.4 | 83.3 | 13.9 | 2.2 | 0.3 | 0.997 | 1.001 | yes | 14.4/16.1 | AUTH |
| banded_2d.ac2d.g64.analytic.x100 | 4096 | banded_indexed | 37 | 36 | 40404 | 40236 | 0.5 | 69.3 | 12.4 | 17.5 | 0.3 | 0.997 | 0.996 | yes | 16.1/11.6 | AUTH |
| banded_2d.ac2d.g128.analytic.x1 | 16384 | banded_indexed | 20 | 19 | 500415 | 501770 | 0.5 | 87.5 | 11.8 | 0.1 | 0.2 | 0.998 | 1.003 | yes | 11.6/4.2 | AUTH |
| banded_2d.bruss2d.g32.analytic.x1 | 2048 | banded_indexed | 91 | 86 | 16242 | 16163 | 0.6 | 83.8 | 15.1 | 0.1 | 0.3 | 0.997 | 0.995 | yes | 4.2/4.2 | AUTH |
| banded_2d.bruss2d.g64.analytic.x1 | 8192 | banded_indexed | 20 | 16 | 245638 | 242570 | 0.3 | 89.5 | 10.1 | 0.0 | 0.1 | 0.999 | 0.988 | yes | 4.2/4.1 | AUTH |
| banded_3d.ac3d.g8.analytic.x1 | 512 | banded_indexed | 41 | 38 | 3451 | 3452 | 0.2 | 82.9 | 16.1 | 0.3 | 0.4 | 0.996 | 1.000 | yes | 4.1/4.1 | AUTH |
| banded_3d.ac3d.g16.analytic.x1 | 4096 | banded_indexed | 20 | 20 | 424949 | 425655 | 0.2 | 94.0 | 5.8 | 0.0 | 0.1 | 0.999 | 1.002 | yes | 4.1/4.7 | AUTH |
| banded_3d.ac3d.g16.fd.x1 | 4096 | banded_indexed | 20 | 20 | 438320 | 441587 | 3.6 | 90.7 | 5.6 | 0.0 | 0.1 | 0.999 | 1.007 | yes | 4.7/4.3 | AUTH |
| dense.radnet.n2048.fd.x1 | 2048 | faer | 20 | 20 | 2701175 | 2986490 | 92.5 | 6.5 | 0.5 | 0.4 | 0.0 | 1.000 | 1.106 | yes | 2.5/5.1 | diag |
| banded_2d.bruss2d.g128.analytic.x1 | 32768 | banded_indexed | 8 | 8 | 3776364 | 3800889 | 0.4 | 92.7 | 6.8 | 0.0 | 0.1 | 0.999 | 1.006 | yes | 5.1/4.0 | diag (reduced) |

### T2. Wall anatomy, matrix-free driver (per attempt, medians)

| cell | n | attempts | accepted | plain us/attempt | RHS % | JVP % | solve intervals % | of which orthogonalisation + vector work % | other % | sum (RHS+JVP) | sum (RHS+intervals) | instr/plain | identical | JVP/attempt | Krylov iterations/solve | label | label with intervals |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| matrix_free_2d.ac2d.g64.jvp.x1 | 4096 | 28 | 27 | 53269 | 0.1 | 6.3 | 99.9 | 93.6 | 0.0 | 0.064 | 1.000 | 1.003 | yes | 336.0 | 40.0 | diag | authoritative |
| matrix_free_2d.ac2d.g128.jvp.x1 | 16384 | 20 | 19 | 271190 | 0.1 | 6.1 | 99.9 | 93.8 | 0.0 | 0.062 | 1.000 | 0.992 | yes | 422.1 | 50.5 | diag | authoritative |
| matrix_free_2d.ac2d.g128.jvp.x10 | 16384 | 20 | 19 | 293216 | 1.1 | 6.3 | 98.9 | 92.6 | 0.0 | 0.074 | 1.000 | 1.003 | yes | 422.1 | 50.5 | diag | authoritative |
| matrix_free_2d.ac2d.g128.jvp.x100 | 16384 | 20 | 19 | 298406 | 9.3 | 5.5 | 90.7 | 85.2 | 0.0 | 0.148 | 1.000 | 1.001 | yes | 422.1 | 50.5 | diag | authoritative |
| matrix_free_2d.ac2d.g256.jvp.x1 | 65536 | 20 | 19 | 1904250 | 0.1 | 5.8 | 99.9 | 94.1 | 0.0 | 0.059 | 1.000 | 1.005 | yes | 717.3 | 86.5 | diag | authoritative |
| matrix_free_2d.bruss2d.g128.jvp.x1 | 32768 | 20 | 15 | 421859 | 0.1 | 4.1 | 99.9 | 95.7 | 0.0 | 0.042 | 1.000 | 1.002 | yes | 336.0 | 40.0 | diag | authoritative |
| matrix_free_3d.ac3d.g16.jvp.x1 | 4096 | 28 | 25 | 52940 | 0.2 | 7.2 | 99.8 | 92.6 | 0.0 | 0.073 | 1.000 | 1.001 | yes | 336.0 | 40.0 | diag | authoritative |
| matrix_free_3d.ac3d.g32.jvp.x1 | 32768 | 20 | 20 | 432002 | 0.2 | 7.4 | 99.8 | 92.4 | 0.0 | 0.076 | 1.000 | 1.004 | yes | 336.0 | 40.0 | diag | authoritative |
| matrix_free_3d.ac3d.g32.jvp.x10 | 32768 | 20 | 20 | 448823 | 1.6 | 7.3 | 98.4 | 91.0 | 0.0 | 0.090 | 1.000 | 0.984 | yes | 336.0 | 40.0 | diag | authoritative |
| matrix_free_3d.ac3d.g32.jvp.x100 | 32768 | 20 | 20 | 504636 | 14.2 | 6.4 | 85.8 | 79.5 | 0.0 | 0.205 | 1.000 | 0.996 | yes | 336.0 | 40.0 | diag | authoritative |

### T3. Phase time per attempt in microseconds (median / minimum)

| cell | Jacobian | factor | solves | RHS | rest | JVP (inside solves) |
|---|---|---|---|---|---|---|
| dense.hires.n8.analytic.x1 | 0.0283 / 0.0279 | 0.136 / 0.134 | 0.765 / 0.759 | 0.192 / 0.19 | 0.874 / 0.857 | - |
| dense.hires.n8.fd.x1 | 0.133 / 0.131 | 0.137 / 0.136 | 0.763 / 0.758 | 0.193 / 0.19 | 0.858 / 0.848 | - |
| dense.bruss1d.n32.analytic.x1 | 0.0582 / 0.0569 | 1.24 / 1.23 | 2.92 / 2.91 | 0.397 / 0.394 | 1.37 / 1.36 | - |
| dense.bruss1d.n32.fd.x1 | 1.84 / 1.81 | 1.25 / 1.23 | 2.93 / 2.92 | 0.398 / 0.397 | 1.39 / 1.38 | - |
| dense.bruss1d.n32.analytic.x10 | 0.0583 / 0.0573 | 1.26 / 1.23 | 2.93 / 2.91 | 2.85 / 2.83 | 1.38 / 1.36 | - |
| dense.bruss1d.n32.analytic.x100 | 0.0586 / 0.0576 | 1.26 / 1.26 | 2.95 / 2.94 | 27.1 / 27 | 1.41 / 1.39 | - |
| dense.bruss1d.n128.analytic.x1 | 0.185 / 0.182 | 22.5 / 22.3 | 11.5 / 11.5 | 1.21 / 1.2 | 3.29 / 3.21 | - |
| dense.bruss1d.n128.fd.x1 | 33.2 / 33.1 | 22.6 / 22.5 | 11.6 / 11.5 | 1.2 / 1.19 | 3.21 / 3.2 | - |
| dense.bruss1d.n512.analytic.x1 | 8.47 / 8.41 | 662 / 659 | 46 / 45.8 | 4.31 / 4.26 | 12.1 / 11.8 | - |
| dense.bruss1d.n512.fd.x1 | 1275 / 1273 | 670 / 660 | 45.9 / 45.7 | 4.54 / 4.37 | 13.2 / 12.4 | - |
| dense.bruss1d.n2048.analytic.x1 | 54 / 51.1 | 17094 / 16928 | 200 / 197 | 17.1 / 16.9 | 105 / 101 | - |
| dense.bruss1d.n2048.fd.x1 | 28505 / 28465 | 19068 / 18891 | 200 / 199 | 17.6 / 17.1 | 222 / 216 | - |
| dense.radnet.n8.analytic.x1 | 0.0775 / 0.0709 | 0.223 / 0.221 | 1.01 / 0.997 | 0.553 / 0.55 | 0.903 / 0.881 | - |
| dense.radnet.n32.analytic.x1 | 0.31 / 0.308 | 3.44 / 3.4 | 6.67 / 6.64 | 2.39 / 2.38 | 1.44 / 1.41 | - |
| dense.radnet.n32.fd.x1 | 10 / 9.92 | 3.5 / 3.42 | 6.68 / 6.67 | 2.35 / 2.34 | 1.43 / 1.41 | - |
| dense.radnet.n128.analytic.x1 | 3.37 / 3.32 | 104 / 104 | 33.5 / 33.1 | 24.6 / 24.4 | 4.49 / 4.42 | - |
| dense.radnet.n128.analytic.x10 | 3.65 / 3.4 | 106 / 105 | 34.2 / 34 | 241 / 240 | 5.09 / 4.39 | - |
| dense.radnet.n128.analytic.x100 | 3.73 / 3.55 | 106 / 105 | 35.6 / 34.1 | 2402 / 2399 | 5.01 / 4.68 | - |
| dense.radnet.n128.fd.x1 | 410 / 409 | 105 / 104 | 34 / 33.4 | 24.8 / 24.7 | 4.5 / 4.44 | - |
| dense.radnet.n128.fd.x10 | 3909 / 3903 | 138 / 107 | 33.6 / 32.8 | 241 / 241 | 4.94 / 4.72 | - |
| dense.radnet.n128.fd.x100 | 38877 / 38665 | 126 / 114 | 37.8 / 35.4 | 2399 / 2384 | 8.39 / 5.95 | - |
| dense.radnet.n512.analytic.x1 | 55 / 54.3 | 3168 / 3153 | 402 / 386 | 388 / 384 | 17.2 / 15.9 | - |
| dense.radnet.n512.fd.x1 | 25982 / 25444 | 4956 / 4454 | 442 / 412 | 406 / 381 | 45.3 / 34.3 | - |
| dense.radnet.n512.analytic.x10 | 64 / 60.2 | 3242 / 3215 | 414 / 411 | 3816 / 3801 | 16.1 / 15.5 | - |
| dense.radnet.n512.analytic.x100 | 108 / 92.2 | 3626 / 3489 | 436 / 415 | 38155 / 38044 | 20.5 / 19.2 | - |
| dense.radnet.n2048.analytic.x1 | 3712 / 3582 | 187444 / 186138 | 13124 / 12649 | 12088 / 11668 | 730 / 716 | - |
| banded_1d.bruss1d.n1000.analytic.x1 | 1.11 / 1.1 | 17 / 16.9 | 105 / 105 | 8.07 / 8.02 | 19.5 / 19.3 | - |
| banded_1d.bruss1d.n1000.fd.x1 | 13.7 / 13.5 | 17.2 / 17.1 | 105 / 105 | 8.04 / 8.02 | 19.6 / 19.5 | - |
| banded_1d.bruss1d.n10000.analytic.x1 | 13.3 / 12.9 | 184 / 179 | 1064 / 1060 | 82.8 / 80.2 | 230 / 221 | - |
| banded_1d.bruss1d.n10000.fd.x1 | 148 / 145 | 180 / 179 | 1067 / 1064 | 80.5 / 80.3 | 231 / 220 | - |
| banded_1d.bruss1d.n100000.analytic.x1 | 317 / 305 | 1999 / 1985 | 10648 / 10641 | 823 / 816 | 2507 / 2433 | - |
| banded_1d.bruss1d.n100000.fd.x1 | 1623 / 1609 | 1987 / 1986 | 10641 / 10628 | 819 / 813 | 2443 / 2398 | - |
| banded_1d.bruss1d.n10000.analytic.x10 | 13.5 / 13.4 | 179 / 178 | 1063 / 1059 | 796 / 794 | 214 / 209 | - |
| banded_1d.bruss1d.n10000.analytic.x100 | 13.7 / 13.5 | 180 / 179 | 1067 / 1063 | 7980 / 7957 | 215 / 211 | - |
| banded_1d.bruss1d.n10000.fd.x10 | 630 / 626 | 180 / 178 | 1062 / 1061 | 797 / 796 | 215 / 212 | - |
| banded_1d.bruss1d.n10000.fd.x100 | 5507 / 5483 | 181 / 179 | 1068 / 1063 | 7993 / 7951 | 219 / 214 | - |
| banded_2d.ac2d.g32.analytic.x1 | 9.96 / 9.72 | 1765 / 1754 | 619 / 613 | 19.2 / 18.8 | 26.4 / 25.6 | - |
| banded_2d.ac2d.g64.analytic.x1 | 211 / 206 | 28642 / 27764 | 5219 / 4964 | 75.5 / 71.4 | 122 / 108 | - |
| banded_2d.ac2d.g64.fd.x1 | 2791 / 2689 | 32525 / 31281 | 6065 / 5733 | 86.1 / 81.6 | 152 / 147 | - |
| banded_2d.ac2d.g64.analytic.x10 | 219 / 208 | 51236 / 27691 | 8354 / 4996 | 1349 / 699 | 180 / 105 | - |
| banded_2d.ac2d.g64.analytic.x100 | 209 / 207 | 27836 / 27682 | 4999 / 4974 | 7042 / 6958 | 110 / 106 | - |
| banded_2d.ac2d.g128.analytic.x1 | 2296 / 2256 | 438678 / 436460 | 59160 / 57556 | 283 / 279 | 1017 / 995 | - |
| banded_2d.bruss2d.g32.analytic.x1 | 101 / 99.2 | 13545 / 13519 | 2441 / 2429 | 23.5 / 23.2 | 54.2 / 52.5 | - |
| banded_2d.bruss2d.g64.analytic.x1 | 629 / 614 | 216789 / 215033 | 24548 / 24270 | 93.5 / 92.2 | 340 / 334 | - |
| banded_3d.ac3d.g8.analytic.x1 | 8.58 / 8.41 | 2856 / 2846 | 555 / 552 | 12 / 11.5 | 14.7 / 14.2 | - |
| banded_3d.ac3d.g16.analytic.x1 | 783 / 771 | 399752 / 397681 | 24756 / 23818 | 87.4 / 86.1 | 224 / 215 | - |
| banded_3d.ac3d.g16.fd.x1 | 15867 / 15836 | 400465 / 398899 | 24793 / 23865 | 88.5 / 87.5 | 224 / 217 | - |
| dense.radnet.n2048.fd.x1 | 2764204 / 2416163 | 192275 / 187949 | 14136 / 12951 | 12469 / 11668 | 733 / 714 | - |
| banded_2d.bruss2d.g128.analytic.x1 | 14241 / 14216 | 3518224 / 3491754 | 258225 / 254357 | 381 / 376 | 5579 / 5386 | - |
| matrix_free_2d.ac2d.g64.jvp.x1 | - | - | 53347 / 53096 | 70 / 69.2 | 6.3 / 5.19 | 3344 / 3302 |
| matrix_free_2d.ac2d.g128.jvp.x1 | - | - | 268815 / 265402 | 279 / 272 | 29.9 / 27.6 | 16384 / 16137 |
| matrix_free_2d.ac2d.g128.jvp.x10 | - | - | 290858 / 268345 | 3145 / 2730 | 30.2 / 27.6 | 18541 / 16169 |
| matrix_free_2d.ac2d.g128.jvp.x100 | - | - | 270955 / 269743 | 27797 / 27631 | 31.4 / 28 | 16445 / 16157 |
| matrix_free_2d.ac2d.g256.jvp.x1 | - | - | 1912405 / 1855311 | 1129 / 1095 | 104 / 92.9 | 110810 / 108490 |
| matrix_free_2d.bruss2d.g128.jvp.x1 | - | - | 422104 / 418809 | 362 / 359 | 117 / 106 | 17581 / 17247 |
| matrix_free_3d.ac3d.g16.jvp.x1 | - | - | 52904 / 52702 | 83.4 / 82.3 | 6.46 / 6.14 | 3838 / 3774 |
| matrix_free_3d.ac3d.g32.jvp.x1 | - | - | 432904 / 431203 | 703 / 699 | 58 / 54.7 | 32296 / 32021 |
| matrix_free_3d.ac3d.g32.jvp.x10 | - | - | 434405 / 432266 | 7126 / 6990 | 60.8 / 54.9 | 32481 / 31923 |
| matrix_free_3d.ac3d.g32.jvp.x100 | - | - | 431537 / 430170 | 71032 / 70779 | 59.8 / 55.3 | 32040 / 31579 |

### T4. Calls per attempt and per accepted step

| cell | Jacobian builds | factorisations | stage solves | RHS calls | RHS calls inside FD | JVP calls | Krylov iterations | preconditioner applications |
|---|---|---|---|---|---|---|---|---|
| dense.hires.n8.analytic.x1 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.24 | 7.97 / 8.21 | 0.00 / 0.00 | - | - | - |
| dense.hires.n8.fd.x1 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.24 | 7.97 / 8.21 | 8.74 / 9.00 | - | - | - |
| dense.bruss1d.n32.analytic.x1 | 0.88 / 1.00 | 1.00 / 1.13 | 8.00 / 9.07 | 7.88 / 8.94 | 0.00 / 0.00 | - | - | - |
| dense.bruss1d.n32.fd.x1 | 0.88 / 1.00 | 1.00 / 1.13 | 8.00 / 9.07 | 7.88 / 8.94 | 29.10 / 33.00 | - | - | - |
| dense.bruss1d.n32.analytic.x10 | 0.88 / 1.00 | 1.00 / 1.13 | 8.00 / 9.07 | 7.88 / 8.94 | 0.00 / 0.00 | - | - | - |
| dense.bruss1d.n32.analytic.x100 | 0.88 / 1.00 | 1.00 / 1.13 | 8.00 / 9.07 | 7.88 / 8.94 | 0.00 / 0.00 | - | - | - |
| dense.bruss1d.n128.analytic.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 0.00 / 0.00 | - | - | - |
| dense.bruss1d.n128.fd.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 114.98 / 129.00 | - | - | - |
| dense.bruss1d.n512.analytic.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 0.00 / 0.00 | - | - | - |
| dense.bruss1d.n512.fd.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 457.24 / 513.00 | - | - | - |
| dense.bruss1d.n2048.analytic.x1 | 0.88 / 1.00 | 1.00 / 1.13 | 8.00 / 9.06 | 7.88 / 8.93 | 0.00 / 0.00 | - | - | - |
| dense.bruss1d.n2048.fd.x1 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.27 | 7.97 / 8.23 | 1982.90 / 2049.00 | - | - | - |
| dense.radnet.n8.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n32.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n32.fd.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 33.00 / 33.00 | - | - | - |
| dense.radnet.n128.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n128.analytic.x10 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n128.analytic.x100 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n128.fd.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 129.00 / 129.00 | - | - | - |
| dense.radnet.n128.fd.x10 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 129.00 / 129.00 | - | - | - |
| dense.radnet.n128.fd.x100 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 129.00 / 129.00 | - | - | - |
| dense.radnet.n512.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n512.fd.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 513.00 / 513.00 | - | - | - |
| dense.radnet.n512.analytic.x10 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n512.analytic.x100 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| dense.radnet.n2048.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| banded_1d.bruss1d.n1000.analytic.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 0.00 / 0.00 | - | - | - |
| banded_1d.bruss1d.n1000.fd.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 5.35 / 6.00 | - | - | - |
| banded_1d.bruss1d.n10000.analytic.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 0.00 / 0.00 | - | - | - |
| banded_1d.bruss1d.n10000.fd.x1 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 5.35 / 6.00 | - | - | - |
| banded_1d.bruss1d.n100000.analytic.x1 | 0.90 / 1.00 | 1.00 / 1.11 | 8.00 / 8.91 | 7.90 / 8.80 | 0.00 / 0.00 | - | - | - |
| banded_1d.bruss1d.n100000.fd.x1 | 0.89 / 1.00 | 1.00 / 1.13 | 8.00 / 9.03 | 7.89 / 8.90 | 5.32 / 6.00 | - | - | - |
| banded_1d.bruss1d.n10000.analytic.x10 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 0.00 / 0.00 | - | - | - |
| banded_1d.bruss1d.n10000.analytic.x100 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 0.00 / 0.00 | - | - | - |
| banded_1d.bruss1d.n10000.fd.x10 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 5.35 / 6.00 | - | - | - |
| banded_1d.bruss1d.n10000.fd.x100 | 0.89 / 1.00 | 1.00 / 1.12 | 8.00 / 8.98 | 7.89 / 8.85 | 5.35 / 6.00 | - | - | - |
| banded_2d.ac2d.g32.analytic.x1 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.21 | 7.97 / 8.18 | 0.00 / 0.00 | - | - | - |
| banded_2d.ac2d.g64.analytic.x1 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.21 | 7.97 / 8.18 | 0.00 / 0.00 | - | - | - |
| banded_2d.ac2d.g64.fd.x1 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.21 | 7.97 / 8.18 | 126.67 / 130.00 | - | - | - |
| banded_2d.ac2d.g64.analytic.x10 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.21 | 7.97 / 8.18 | 0.00 / 0.00 | - | - | - |
| banded_2d.ac2d.g64.analytic.x100 | 0.97 / 1.00 | 1.00 / 1.03 | 8.00 / 8.22 | 7.97 / 8.19 | 0.00 / 0.00 | - | - | - |
| banded_2d.ac2d.g128.analytic.x1 | 1.00 / 1.05 | 1.00 / 1.05 | 8.00 / 8.42 | 8.00 / 8.42 | 0.00 / 0.00 | - | - | - |
| banded_2d.bruss2d.g32.analytic.x1 | 0.95 / 1.00 | 1.00 / 1.06 | 8.00 / 8.47 | 7.95 / 8.41 | 0.00 / 0.00 | - | - | - |
| banded_2d.bruss2d.g64.analytic.x1 | 0.80 / 1.00 | 1.00 / 1.25 | 8.00 / 10.00 | 7.80 / 9.75 | 0.00 / 0.00 | - | - | - |
| banded_3d.ac3d.g8.analytic.x1 | 0.93 / 1.00 | 1.00 / 1.08 | 8.00 / 8.63 | 7.93 / 8.55 | 0.00 / 0.00 | - | - | - |
| banded_3d.ac3d.g16.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| banded_3d.ac3d.g16.fd.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 514.00 / 514.00 | - | - | - |
| dense.radnet.n2048.fd.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 2049.00 / 2049.00 | - | - | - |
| banded_2d.bruss2d.g128.analytic.x1 | 1.00 / 1.00 | 1.00 / 1.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | - | - | - |
| matrix_free_2d.ac2d.g64.jvp.x1 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.30 | 7.96 / 8.26 | 0.00 / 0.00 | 336.0 / 348.4 | 320.0 / 331.9 | 328.0 |
| matrix_free_2d.ac2d.g128.jvp.x1 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.42 | 8.00 / 8.42 | 0.00 / 0.00 | 422.1 / 444.3 | 404.0 / 425.3 | 414.1 |
| matrix_free_2d.ac2d.g128.jvp.x10 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.42 | 8.00 / 8.42 | 0.00 / 0.00 | 422.1 / 444.3 | 404.0 / 425.3 | 414.1 |
| matrix_free_2d.ac2d.g128.jvp.x100 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.42 | 8.00 / 8.42 | 0.00 / 0.00 | 422.1 / 444.3 | 404.0 / 425.3 | 414.1 |
| matrix_free_2d.ac2d.g256.jvp.x1 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.42 | 8.00 / 8.42 | 0.00 / 0.00 | 717.3 / 755.1 | 692.0 / 728.4 | 709.3 |
| matrix_free_2d.bruss2d.g128.jvp.x1 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 10.67 | 7.75 / 10.33 | 0.00 / 0.00 | 336.0 / 448.0 | 320.0 / 426.7 | 328.0 |
| matrix_free_3d.ac3d.g16.jvp.x1 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.96 | 7.93 / 8.88 | 0.00 / 0.00 | 336.0 / 376.3 | 320.0 / 358.4 | 328.0 |
| matrix_free_3d.ac3d.g32.jvp.x1 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | 336.0 / 336.0 | 320.0 / 320.0 | 328.0 |
| matrix_free_3d.ac3d.g32.jvp.x10 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | 336.0 / 336.0 | 320.0 / 320.0 | 328.0 |
| matrix_free_3d.ac3d.g32.jvp.x100 | 0.00 / 0.00 | 0.00 / 0.00 | 8.00 / 8.00 | 8.00 / 8.00 | 0.00 / 0.00 | 336.0 / 336.0 | 320.0 / 320.0 | 328.0 |

### T5. Instruction counts (callgrind, inclusive Ir of the marker symbols)

| cell | n | role | attempts | Ir/attempt | J % | LU % | solve % | RHS % | JVP % | rest % | marked/plain Ir | identical |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dense.hires.n8.analytic.x1 | 8 | required_n_below_128 | 210 | 24784 | 0.9 | 10.8 | 37.9 | 3.7 | - | 46.7 | 1.0327 | yes |
| dense.hires.n8.fd.x1 | 8 | required_n_below_128 | 210 | 27297 | 10.0 | 9.8 | 34.4 | 3.4 | - | 42.4 | 1.0293 | yes |
| dense.bruss1d.n32.analytic.x1 | 32 | required_n_below_128 | 93 | 98625 | 1.0 | 26.2 | 40.6 | 7.4 | - | 24.7 | 1.0085 | yes |
| dense.bruss1d.n32.fd.x1 | 32 | required_n_below_128 | 93 | 144057 | 32.2 | 17.9 | 27.8 | 5.1 | - | 16.9 | 1.0059 | yes |
| dense.bruss1d.n32.analytic.x10 | 32 | required_n_below_128 | 93 | 165211 | 0.6 | 15.6 | 24.3 | 44.7 | - | 14.7 | 1.0051 | yes |
| dense.bruss1d.n32.analytic.x100 | 32 | required_n_below_128 | 93 | 827039 | 0.1 | 3.1 | 4.8 | 89.0 | - | 2.9 | 1.0010 | yes |
| dense.bruss1d.n128.analytic.x1 | 128 | exploratory | 20 | 616339 | 0.7 | 53.1 | 25.9 | 4.7 | - | 15.6 | 0.9993 | yes |
| dense.radnet.n8.analytic.x1 | 8 | required_n_below_128 | 79 | 41402 | 2.9 | 11.0 | 27.0 | 21.6 | - | 37.6 | 1.0197 | yes |
| dense.radnet.n32.analytic.x1 | 32 | required_n_below_128 | 84 | 229779 | 3.0 | 35.8 | 29.1 | 21.2 | - | 10.9 | 1.0037 | yes |
| dense.radnet.n32.fd.x1 | 32 | required_n_below_128 | 84 | 445460 | 50.0 | 18.5 | 15.0 | 10.9 | - | 5.6 | 1.0019 | yes |
| dense.radnet.n128.analytic.x1 | 128 | exploratory | 20 | 2247930 | 3.0 | 50.2 | 23.1 | 19.5 | - | 4.3 | 0.9972 | yes |
| banded_1d.bruss1d.n1000.analytic.x1 | 1000 | exploratory | 20 | 2518106 | 2.4 | 19.8 | 51.4 | 8.9 | - | 17.6 | 1.0004 | yes |
| banded_2d.ac2d.g32.analytic.x1 | 1024 | exploratory | 20 | 63527011 | 0.9 | 76.3 | 21.3 | 0.8 | - | 0.7 | 1.0000 | yes |
| matrix_free_2d.ac2d.g64.jvp.x1 | 4096 | exploratory | 20 | 560533034 | - | - | - | 0.3 | 15.9 | 83.7 | 1.0000 | yes |

### T6. Upper bounds of the whole-step speed-up per lever (fractions of T1, T2, T5)

| cell | basis | label | chain % | (a) L=4/5/6 | r=(J+LU)/chain | (b) J+LU | (b) J only | (c) P=2/4/6 | (d) k=2/4 | (e) P=2/4/6 | (f) | (g) P=2/4/6 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dense.hires.n8.analytic.x1 | Ir | diag | 41.6 | 1.26 / 1.18 / 1.12 | 0.282 | 1.12 | 1.01 | 1.00 / 1.01 / 1.01 | 1.06 / 1.09 | 1.06 / 1.09 / 1.10 | - | 1.02 / 1.03 / 1.03 |
| dense.hires.n8.fd.x1 | Ir | diag | 37.8 | 1.23 / 1.16 / 1.10 | 0.525 | 1.10 | 1.10 | 1.05 / 1.08 / 1.09 | 1.05 / 1.08 | 1.05 / 1.08 / 1.09 | - | 1.02 / 1.03 / 1.03 |
| dense.bruss1d.n32.analytic.x1 | Ir | diag | 48.1 | 1.32 / 1.22 / 1.14 | 0.566 | 1.14 | 1.01 | 1.01 / 1.01 / 1.01 | 1.15 / 1.24 | 1.15 / 1.24 / 1.28 | - | 1.04 / 1.06 / 1.07 |
| dense.bruss1d.n32.fd.x1 | Ir | diag | 32.9 | 1.20 / 1.14 / 1.09 | 1.524 | 1.09 | 1.09 | 1.19 / 1.32 / 1.37 | 1.10 / 1.16 | 1.10 / 1.16 / 1.18 | - | 1.03 / 1.04 / 1.04 |
| dense.bruss1d.n32.analytic.x10 | Ir | diag | 69.0 | 1.53 / 1.35 / 1.21 | 0.235 | 1.19 | 1.01 | 1.00 / 1.00 / 1.01 | 1.08 / 1.13 | 1.08 / 1.13 / 1.15 | - | 1.29 / 1.51 / 1.59 |
| dense.bruss1d.n32.analytic.x100 | Ir | diag | 93.8 | 1.88 / 1.54 / 1.31 | 0.035 | 1.03 | 1.00 | 1.00 / 1.00 / 1.00 | 1.02 / 1.02 | 1.02 / 1.02 / 1.03 | - | 1.80 / 3.00 / 3.87 |
| dense.bruss1d.n128.analytic.x1 | wall | diag | 32.9 | 1.20 / 1.14 / 1.09 | 1.785 | 1.09 | 1.00 | 1.00 / 1.00 / 1.00 | 1.41 / 1.77 | 1.41 / 1.77 / 1.94 | - | 1.02 / 1.02 / 1.03 |
| dense.bruss1d.n128.fd.x1 | wall | AUTH | 17.8 | 1.10 / 1.07 / 1.05 | 4.371 | 1.05 | 1.05 | 1.30 / 1.53 / 1.63 | 1.19 / 1.31 | 1.19 / 1.31 / 1.35 | - | 1.01 / 1.01 / 1.01 |
| dense.bruss1d.n512.analytic.x1 | wall | AUTH | 6.8 | 1.04 / 1.03 / 1.02 | 13.366 | 1.02 | 1.01 | 1.01 / 1.01 / 1.01 | 1.82 / 3.10 | 1.82 / 3.10 / 4.05 | - | 1.00 / 1.00 / 1.00 |
| dense.bruss1d.n512.fd.x1 | wall | AUTH | 2.5 | 1.01 / 1.01 / 1.01 | 38.642 | 1.01 | 1.01 | 1.47 / 1.91 / 2.12 | 1.20 / 1.33 | 1.20 / 1.33 / 1.38 | - | 1.00 / 1.00 / 1.00 |
| dense.bruss1d.n2048.analytic.x1 | wall | AUTH | 1.2 | 1.01 / 1.00 / 1.00 | 79.772 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.96 / 3.76 | 1.96 / 3.76 / 5.42 | - | 1.00 / 1.00 / 1.00 |
| dense.bruss1d.n2048.fd.x1 | wall | AUTH | 0.5 | 1.00 / 1.00 / 1.00 | 218.632 | 1.00 | 1.00 | 1.42 / 1.80 / 1.98 | 1.25 / 1.42 | 1.25 / 1.42 / 1.49 | - | 1.00 / 1.00 / 1.00 |
| dense.radnet.n8.analytic.x1 | Ir | diag | 48.6 | 1.32 / 1.22 / 1.14 | 0.286 | 1.14 | 1.03 | 1.01 / 1.02 / 1.02 | 1.06 / 1.09 | 1.06 / 1.09 / 1.10 | - | 1.12 / 1.19 / 1.22 |
| dense.radnet.n32.analytic.x1 | Ir | diag | 50.3 | 1.34 / 1.23 / 1.14 | 0.771 | 1.14 | 1.03 | 1.02 / 1.02 / 1.03 | 1.22 / 1.37 | 1.22 / 1.37 / 1.43 | - | 1.12 / 1.19 / 1.21 |
| dense.radnet.n32.fd.x1 | Ir | diag | 25.9 | 1.15 / 1.11 / 1.07 | 2.637 | 1.07 | 1.07 | 1.33 / 1.60 / 1.71 | 1.10 / 1.16 | 1.10 / 1.16 / 1.18 | - | 1.06 / 1.09 / 1.10 |
| dense.radnet.n128.analytic.x1 | wall | AUTH | 34.0 | 1.21 / 1.15 / 1.09 | 1.859 | 1.09 | 1.02 | 1.01 / 1.02 / 1.02 | 1.44 / 1.85 | 1.44 / 1.85 / 2.04 | - | 1.08 / 1.12 / 1.14 |
| dense.radnet.n128.analytic.x10 | wall | AUTH | 70.5 | 1.54 / 1.36 / 1.21 | 0.397 | 1.21 | 1.01 | 1.00 / 1.01 / 1.01 | 1.16 / 1.25 | 1.16 / 1.25 / 1.29 | - | 1.45 / 1.86 / 2.06 |
| dense.radnet.n128.analytic.x100 | wall | AUTH | 95.4 | 1.91 / 1.56 / 1.31 | 0.045 | 1.04 | 1.00 | 1.00 / 1.00 / 1.00 | 1.02 / 1.03 | 1.02 / 1.03 / 1.04 | - | 1.89 / 3.39 / 4.61 |
| dense.radnet.n128.fd.x1 | wall | AUTH | 10.2 | 1.05 / 1.04 / 1.03 | 8.739 | 1.03 | 1.03 | 1.55 / 2.14 / 2.44 | 1.10 / 1.16 | 1.10 / 1.16 / 1.18 | - | 1.02 / 1.03 / 1.04 |
| dense.radnet.n128.fd.x10 | wall | AUTH | 6.4 | 1.03 / 1.02 / 1.02 | 14.730 | 1.02 | 1.02 | 1.82 / 3.10 / 4.05 | 1.02 / 1.02 | 1.02 / 1.02 / 1.03 | - | 1.03 / 1.04 / 1.05 |
| dense.radnet.n128.fd.x100 | wall | AUTH | 5.9 | 1.03 / 1.02 / 1.01 | 15.990 | 1.01 | 1.01 | 1.88 / 3.37 / 4.58 | 1.00 / 1.00 | 1.00 / 1.00 / 1.00 | - | 1.03 / 1.05 / 1.05 |
| dense.radnet.n512.analytic.x1 | wall | AUTH | 19.7 | 1.11 / 1.08 / 1.05 | 4.066 | 1.05 | 1.01 | 1.01 / 1.01 / 1.01 | 1.65 / 2.44 | 1.65 / 2.44 / 2.90 | - | 1.05 / 1.08 / 1.09 |
| dense.radnet.n512.fd.x1 | wall | diag | 2.7 | 1.01 / 1.01 / 1.01 | 36.543 | 1.01 | 1.01 | 1.70 / 2.60 / 3.16 | 1.08 / 1.13 | 1.08 / 1.13 / 1.14 | - | 1.01 / 1.01 / 1.01 |
| dense.radnet.n512.analytic.x10 | wall | AUTH | 56.0 | 1.39 / 1.27 / 1.16 | 0.783 | 1.16 | 1.01 | 1.00 / 1.01 / 1.01 | 1.27 / 1.48 | 1.27 / 1.48 / 1.56 | - | 1.34 / 1.61 / 1.73 |
| dense.radnet.n512.analytic.x100 | wall | AUTH | 91.1 | 1.84 / 1.52 / 1.30 | 0.097 | 1.10 | 1.00 | 1.00 / 1.00 / 1.00 | 1.04 / 1.07 | 1.04 / 1.07 / 1.08 | - | 1.82 / 3.08 / 4.01 |
| dense.radnet.n2048.analytic.x1 | wall | AUTH | 11.6 | 1.06 / 1.05 / 1.03 | 7.598 | 1.03 | 1.02 | 1.01 / 1.01 / 1.01 | 1.76 / 2.84 | 1.76 / 2.84 / 3.56 | - | 1.03 / 1.04 / 1.05 |
| banded_1d.bruss1d.n1000.analytic.x1 | wall | diag | 75.1 | 1.60 / 1.39 / 1.23 | 0.160 | 1.14 | 1.01 | 1.00 / 1.01 / 1.01 | 1.06 / 1.09 | - | - | 1.03 / 1.04 / 1.05 |
| banded_1d.bruss1d.n1000.fd.x1 | wall | diag | 69.2 | 1.53 / 1.35 / 1.21 | 0.272 | 1.21 | 1.09 | 1.04 / 1.07 / 1.07 | 1.06 / 1.09 | - | - | 1.03 / 1.04 / 1.04 |
| banded_1d.bruss1d.n10000.analytic.x1 | wall | diag | 73.1 | 1.58 / 1.38 / 1.22 | 0.172 | 1.14 | 1.01 | 1.00 / 1.01 / 1.01 | 1.06 / 1.10 | - | - | 1.03 / 1.04 / 1.05 |
| banded_1d.bruss1d.n10000.fd.x1 | wall | diag | 67.3 | 1.51 / 1.34 / 1.20 | 0.286 | 1.20 | 1.09 | 1.05 / 1.07 / 1.08 | 1.06 / 1.09 | - | - | 1.02 / 1.04 / 1.04 |
| banded_1d.bruss1d.n100000.analytic.x1 | wall | diag | 70.5 | 1.54 / 1.36 / 1.21 | 0.202 | 1.17 | 1.02 | 1.01 / 1.01 / 1.02 | 1.07 / 1.10 | - | - | 1.03 / 1.04 / 1.04 |
| banded_1d.bruss1d.n100000.fd.x1 | wall | diag | 65.4 | 1.49 / 1.33 / 1.20 | 0.315 | 1.20 | 1.10 | 1.05 / 1.07 / 1.08 | 1.06 / 1.09 | - | - | 1.02 / 1.04 / 1.04 |
| banded_1d.bruss1d.n10000.analytic.x10 | wall | AUTH | 82.1 | 1.70 / 1.44 / 1.26 | 0.104 | 1.09 | 1.01 | 1.00 / 1.00 / 1.01 | 1.04 / 1.06 | - | - | 1.21 / 1.36 / 1.41 |
| banded_1d.bruss1d.n10000.analytic.x100 | wall | AUTH | 95.7 | 1.92 / 1.56 / 1.31 | 0.021 | 1.02 | 1.00 | 1.00 / 1.00 / 1.00 | 1.01 / 1.01 | - | - | 1.73 / 2.73 / 3.37 |
| banded_1d.bruss1d.n10000.fd.x10 | wall | AUTH | 64.5 | 1.48 / 1.32 / 1.19 | 0.436 | 1.19 | 1.19 | 1.12 / 1.20 / 1.22 | 1.03 / 1.05 | - | - | 1.16 / 1.26 / 1.30 |
| banded_1d.bruss1d.n10000.fd.x100 | wall | AUTH | 60.6 | 1.43 / 1.29 / 1.18 | 0.627 | 1.18 | 1.18 | 1.23 / 1.38 / 1.44 | 1.01 / 1.01 | - | - | 1.36 / 1.67 / 1.80 |
| banded_2d.ac2d.g32.analytic.x1 | wall | AUTH | 26.1 | 1.15 / 1.11 / 1.07 | 2.795 | 1.07 | 1.00 | 1.00 / 1.00 / 1.00 | 1.57 / 2.19 | - | - | 1.00 / 1.01 / 1.01 |
| banded_2d.ac2d.g64.analytic.x1 | wall | AUTH | 15.5 | 1.08 / 1.06 / 1.04 | 5.429 | 1.04 | 1.01 | 1.00 / 1.00 / 1.01 | 1.72 / 2.68 | - | - | 1.00 / 1.00 / 1.00 |
| banded_2d.ac2d.g64.fd.x1 | wall | diag | 14.7 | 1.08 / 1.06 / 1.04 | 5.799 | 1.04 | 1.04 | 1.04 / 1.05 / 1.06 | 1.64 / 2.42 | - | - | 1.00 / 1.00 / 1.00 |
| banded_2d.ac2d.g64.analytic.x10 | wall | AUTH | 16.1 | 1.09 / 1.06 / 1.04 | 5.202 | 1.04 | 1.00 | 1.00 / 1.00 / 1.00 | 1.71 / 2.66 | - | - | 1.01 / 1.02 / 1.02 |
| banded_2d.ac2d.g64.analytic.x100 | wall | AUTH | 29.9 | 1.18 / 1.13 / 1.08 | 2.330 | 1.08 | 1.01 | 1.00 / 1.00 / 1.00 | 1.53 / 2.08 | - | - | 1.10 / 1.15 / 1.17 |
| banded_2d.ac2d.g128.analytic.x1 | wall | AUTH | 11.9 | 1.06 / 1.05 / 1.03 | 7.396 | 1.03 | 1.00 | 1.00 / 1.00 / 1.00 | 1.78 / 2.91 | - | - | 1.00 / 1.00 / 1.00 |
| banded_2d.bruss2d.g32.analytic.x1 | wall | AUTH | 15.2 | 1.08 / 1.06 / 1.04 | 5.548 | 1.04 | 1.01 | 1.00 / 1.00 / 1.01 | 1.72 / 2.69 | - | - | 1.00 / 1.00 / 1.00 |
| banded_2d.bruss2d.g64.analytic.x1 | wall | AUTH | 10.1 | 1.05 / 1.04 / 1.03 | 8.844 | 1.03 | 1.00 | 1.00 / 1.00 / 1.00 | 1.81 / 3.04 | - | - | 1.00 / 1.00 / 1.00 |
| banded_3d.ac3d.g8.analytic.x1 | wall | AUTH | 16.4 | 1.09 / 1.07 / 1.04 | 5.063 | 1.04 | 1.00 | 1.00 / 1.00 / 1.00 | 1.71 / 2.64 | - | - | 1.00 / 1.00 / 1.00 |
| banded_3d.ac3d.g16.analytic.x1 | wall | AUTH | 5.8 | 1.03 / 1.02 / 1.01 | 16.241 | 1.01 | 1.00 | 1.00 / 1.00 / 1.00 | 1.89 / 3.39 | - | - | 1.00 / 1.00 / 1.00 |
| banded_3d.ac3d.g16.fd.x1 | wall | AUTH | 5.7 | 1.03 / 1.02 / 1.01 | 16.680 | 1.01 | 1.01 | 1.02 / 1.03 / 1.03 | 1.83 / 3.13 | - | - | 1.00 / 1.00 / 1.00 |
| matrix_free_2d.ac2d.g64.jvp.x1 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 7.93 | 1.00 / 1.00 / 1.00 |
| matrix_free_2d.ac2d.g128.jvp.x1 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 9.46 | 1.00 / 1.00 / 1.00 |
| matrix_free_2d.ac2d.g128.jvp.x10 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 8.80 | 1.01 / 1.01 / 1.01 |
| matrix_free_2d.ac2d.g128.jvp.x100 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 5.32 | 1.05 / 1.08 / 1.08 |
| matrix_free_2d.ac2d.g256.jvp.x1 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 9.28 | 1.00 / 1.00 / 1.00 |
| matrix_free_2d.bruss2d.g128.jvp.x1 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 7.92 | 1.00 / 1.00 / 1.00 |
| matrix_free_3d.ac3d.g16.jvp.x1 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 7.91 | 1.00 / 1.00 / 1.00 |
| matrix_free_3d.ac3d.g32.jvp.x1 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 7.87 | 1.00 / 1.00 / 1.00 |
| matrix_free_3d.ac3d.g32.jvp.x10 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 7.17 | 1.01 / 1.01 / 1.01 |
| matrix_free_3d.ac3d.g32.jvp.x100 | wall | diag | 100.0 | 2.00 / 1.60 / 1.33 | 0.000 | 1.00 | 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 | - | 4.01 | 1.08 / 1.12 / 1.13 |
| dense.radnet.n2048.fd.x1 | wall | diag | 0.9 | 1.00 / 1.00 / 1.00 | 106.392 | 1.00 | 1.00 | 1.86 / 3.27 / 4.37 | 1.03 / 1.05 | 1.03 / 1.05 / 1.06 | - | 1.00 / 1.00 / 1.00 |
| banded_2d.bruss2d.g128.analytic.x1 | wall | diag | 6.8 | 1.04 / 1.03 / 1.02 | 13.677 | 1.02 | 1.00 | 1.00 / 1.00 / 1.00 | 1.86 / 3.28 | - | - | 1.00 / 1.00 / 1.00 |

### T7. Regimes that open V07 stage b: r in [0.15, 0.92] and plain wall per accepted step >= 500 us

| cell | n | r | plain us/accepted step | (b) bound | (b) J only | basis | label |
|---|---|---|---|---|---|---|---|
| dense.radnet.n512.analytic.x10 | 512 | 0.783 | 8515 | 1.16 | 1.01 | wall | authoritative |
| banded_1d.bruss1d.n10000.analytic.x1 | 10000 | 0.172 | 1757 | 1.14 | 1.01 | wall | diagnostic_only |
| banded_1d.bruss1d.n10000.fd.x1 | 10000 | 0.286 | 1914 | 1.20 | 1.09 | wall | diagnostic_only |
| banded_1d.bruss1d.n100000.analytic.x1 | 100000 | 0.202 | 18050 | 1.17 | 1.02 | wall | diagnostic_only |
| banded_1d.bruss1d.n100000.fd.x1 | 100000 | 0.315 | 19805 | 1.20 | 1.10 | wall | diagnostic_only |
| banded_1d.bruss1d.n10000.fd.x10 | 10000 | 0.436 | 3236 | 1.19 | 1.19 | wall | authoritative |
| banded_1d.bruss1d.n10000.fd.x100 | 10000 | 0.627 | 16847 | 1.18 | 1.18 | wall | authoritative |

### T8. Regimes with r in [0.15, 0.92] whose accepted step is shorter than 500 us (not in the gate list)

| cell | r | r in band | plain us/accepted step | >= 500 us | label |
|---|---|---|---|---|---|
| dense.hires.n8.analytic.x1 | 0.282 | yes | 1.22 | no | diagnostic_only |
| dense.hires.n8.fd.x1 | 0.525 | yes | 1.32 | no | diagnostic_only |
| dense.bruss1d.n32.analytic.x1 | 0.566 | yes | 5.93 | no | diagnostic_only |
| dense.bruss1d.n32.analytic.x10 | 0.235 | yes | 8.78 | no | diagnostic_only |
| dense.radnet.n8.analytic.x1 | 0.286 | yes | 1.87 | no | diagnostic_only |
| dense.radnet.n32.analytic.x1 | 0.771 | yes | 13.7 | no | diagnostic_only |
| dense.radnet.n128.analytic.x10 | 0.397 | yes | 418 | no | authoritative |
| banded_1d.bruss1d.n1000.analytic.x1 | 0.160 | yes | 168 | no | diagnostic_only |
| banded_1d.bruss1d.n1000.fd.x1 | 0.272 | yes | 183 | no | diagnostic_only |

### T9. Cells without a wall measurement

| cell | protocol | reason | pilot attempts | pilot accepted | pilot rejected | pilot s/attempt |
|---|---|---|---|---|---|---|
| matrix_free_2d.ac2d.g128.jvpfd.x1 | counts_only | exploratory cell | 12 | 6 | 6 | 0.2449 |
<!-- END GENERATED TABLES -->

## 3. 해석

### 3.1 소형 dense (n = 8, 32): instruction 기준

- HIRES(n = 8)는 attempt당 instruction의 46.7 %가 나머지(stage 벡터 연산, 오차 norm, controller, 출력)이고, solve 37.9 %, factor 10.8 %, RHS 3.7 %, Jacobian 0.9 %다. attempt 하나가 1.19 µs이다. 어떤 lever도 나머지에는 닿지 않고, thread를 쓸 만한 크기의 일감도 없다.
- n = 32에서는 solve가 29.1–40.6 %, factor가 26.2–35.8 %로 올라온다. RHS가 n²인 `radnet`은 RHS가 21.2 %다.
- 이 크기에서 instrumented/plain wall 비는 1.024–1.680이다. timer 비용이 phase 자체와 같은 규모라는 뜻이고, wall 분해는 쓸 수 없다. client driver 자체의 instruction 오버헤드(marked/plain)는 n = 8에서 1.033배다.

### 3.2 dense n ≥ 128

- **in-place LU**(band 구조 문제를 dense로 저장한 `bruss1d`): factor가 58.2–97.9 %, 사슬이 1.2–32.9 %다. n = 2048에서 attempt당 17.5 ms인데, 같은 문제를 band로 선언한 n = 1e4 cell은 1.57 ms다. 코드상 이 경로의 factor는 n × n 크기의 W 조립과, Legacy 정책의 pivot 탐색(열 전체 주사)을 포함한다. band LU 자체보다 이 n² 부분이 지배하는 것으로 보인다(따로 재지는 않았다). 이 영역의 처방은 LU 병렬화가 아니라 band 선언(라우팅)이다.
- **faer LU**(`radnet`): factor 61.3–86.3 %, solve 6.1–19.6 %, RHS 5.5–14.4 %. n이 커질수록 factor 몫이 커진다(n = 2048에서 86.3 %).
- **FD Jacobian**: Jacobian phase가 46.3–93.8 %다. RHS 호출이 attempt당 수락 상태마다 n + 1회 더 들어가기 때문이다(표 T4).

### 3.3 1D banded (n = 1e3, 1e4, 1e5)

- solve가 65.4–69.7 %, factor 11.3–12.3 %, RHS 5.1–5.4 %, 나머지 12.9–15.4 %다. 세 크기에서 비율이 거의 같다(n에 선형). 사슬(RHS + solve)이 70.5–75.1 %이고, 나머지의 대부분도 stage 사이의 벡터 연산이라 사실상 step 전체가 순차 사슬이다.
- solve가 factor보다 훨씬 큰 것은 band 폭이 2라서 factor가 싸고 solve는 attempt당 8번 반복되기 때문이다. Indexed kernel의 후진 대입은 행마다 나눗셈을 한 번 하는데, n = 1000의 instruction 기준 solve 몫은 51.4 %로 wall 몫 69.7 %보다 작다(느린 instruction이 solve에 몰려 있다는 뜻). 보호 `_with_atol` 진입점은 Indexed kernel만 쓴다(Slices·FixedWidth2는 opt-in이라 재지 않았다).
- FD(묶음 5개 + 1회)는 Jacobian을 8.3–9.3 %로 올릴 뿐이다.
- 이 행들은 phase 합이 0.846–0.871라서 기준 (ii)에 걸려 `diagnostic_only`다. timer 문제는 아니다(instrumented/plain 1.000–1.006, bit-동일). 재지 못한 것은 driver 안의 벡터 연산이다.

### 3.4 2D/3D banded 직접법

- factor가 72.4–94.0 %, solve가 5.8–25.3 %, RHS는 0.0–0.8 %다. band 폭이 m(2D) 또는 m²(3D)라서 factor가 n·b², solve가 n·b로 커진다.
- FD Jacobian(band 묶음 2b + 1개)은 Jacobian을 3.6–6.8 %로 만든다.
- RHS 배율 100을 걸어도 2D banded 64²에서는 RHS가 17.5 %에 그친다. stencil RHS가 band LU에 비해 그만큼 싸다.
- 3D 32³의 banded 직접법은 사전등록대로 제외했다(factor 저장 805 MB, factorisation당 곱셈-뺄셈 약 6.9e10회). 표 T9와 6절 참조.

### 3.5 matrix-free

- stage solve 구간이 99.8–99.9 %를 차지하고, 그 안에서 JVP callback은 4.1–7.4 %뿐이다. 나머지 92.4–95.7 %가 Krylov 직교화(2-pass MGS)와 벡터 연산이다. RHS는 0.1–0.2 %다.
- 보호 driver의 GMRES는 solve당 Krylov 반복이 평균 40.0–86.5회, attempt당 JVP가 336–717회다(cell별 solve당 반복: `ac2d.g64.jvp.x1` 40.0, `ac2d.g128.jvp.x1` 50.5, `ac2d.g256.jvp.x1` 86.5, `bruss2d.g128.jvp.x1` 40.0, `ac3d.g16.jvp.x1` 40.0, `ac3d.g32.jvp.x1` 40.0). `gmres_into.rs`의 주기(`run_cycle`)가 최소제곱 문제를 주기 끝에서 한 번만 풀기 때문에 반복 수는 restart 길이 40열 단위로만 늘어난다. 한 주기 안에서는 허용오차에 일찍 닿아도 멈추지 않는다. 작은 격자에서는 모든 solve가 정확히 40회(JVP 42회)이고, 격자가 커지면 뒤쪽 stage부터 둘째 주기가 필요해진다(T2의 `JVP/attempt`).
- RHS 배율 100에서도 RHS는 9.3–14.2 %에 그친다(JVP가 해석식이라 RHS 비용이 JVP로 전파되지 않음).
- 이 행들은 기준 (ii)(RHS + JVP 합)에 걸려 `diagnostic_only`다. driver에 Krylov 직교화를 감싸는 hook이 없어서 callback clock으로는 잴 수 없다. solve 구간으로 세면 합은 0.99 이상이다(T2의 `sum (RHS+intervals)` 열).

### 3.6 측정 중 드러난 사실

1. **faer 경로의 arm 간 차이.** `radnet` n = 128, 512(해석 Jacobian, 배율 1)에서 instrumented/plain wall이 0.840–0.845였다. timer를 단 쪽이 더 빠르다. 반복 측정(6.3절)에서도 n = 128은 0.847, n = 512는 0.748로 다시 나타났다. 두 arm은 bit-동일하고 callgrind의 instruction 수도 같으므로(T5의 marked/plain), 차이는 사용자 instruction이 아닌 곳에서 온다. 보호 driver의 faer 경로는 factorisation마다 n × n 행렬을 새로 할당하고 앞의 것을 버린다(`LuFactorization::new`). arm을 하나씩 별도 프로세스로 실행한 perf stat(6.1절)에서도 instructions:u는 같고, page fault 수는 arm과 구성에 따라 크게 갈린다. 그래서 wall protocol과 같은 교차 실행에서 run별 page fault를 직접 셌다(6.2절): n = 512에서 plain arm은 run당 42656회(attempt당 496회), instrumented arm은 480회(attempt당 6회)의 minor fault를 냈다. 즉 보호 driver의 faer 경로가 factorisation마다 새 page를 건드리는 배치에 놓였고, client arm은 같은 블록을 재사용하는 배치에 놓였다. n = 2048에서는 두 arm이 같은 수의 page fault를 내고 wall 비가 1.000이다. 기준 (i)은 상한만 두므로 이 행들은 `authoritative`지만, 분율은 instrumented run의 것이고 이 측정에서 보호 driver는 그보다 18–34 % 더 썼다는 점을 감안해야 한다. factor 저장을 재사용하면 없어질 비용이므로, 격자에 없던 lever 후보로 4절에 적었다.

2. **GMRES 반복 수가 40열 단위**(3.5절). 한 주기로 충분한 solve도 40열을 다 만들고 2-pass MGS로 직교화한다. matrix-free wall의 대부분이 여기서 나오며, shared basis(f)의 상한이 큰 이유이기도 하다.

3. **FD JVP 불가.** 탐색 cell `matrix_free_2d.ac2d.g128.jvpfd.x1`(pilot, attempt 12회)은 6회가 거절됐다. 전진 차분 JVP의 잡음(약 1e-8)이 보호 선형 허용오차 1e-10보다 크다. 그래서 timing 격자에서 뺐다.

## 4. lever별 상한과 권고

상한은 표 T6의 값이다. 모두 `오버헤드 0, 완전한 확장성, step 수 불변`을 가정한 위쪽 한계이며, `diagnostic_only` 행에서 나온 값은 진단값이다. 식은 `AMDAHL.json`의 `formulas`에 있다.

| lever | 가장 큰 상한 (authoritative 행) | 가장 큰 상한 (전체 행) | 권고 |
|---|---|---|---|
| (a) level 구조 tableau, L = 5 | `banded_1d.bruss1d.n10000.analytic.x100` 1.56, `dense.radnet.n128.analytic.x100` 1.56, `dense.radnet.n512.analytic.x100` 1.52 | `matrix_free_2d.ac2d.g256.jvp.x1` 1.60, `matrix_free_2d.ac2d.g128.jvp.x100` 1.60, `matrix_free_2d.ac2d.g128.jvp.x10` 1.60 | 사슬이 큰 곳(1D banded, 배율 100, matrix-free)에서만 1.36–1.60. 새 tableau는 새 방법이라 별도 자격 심사가 필요하고 L = 4는 이론상 불가(S9). dense n ≥ 128(배율 1)에서는 1.00–1.15 |
| (b) 투기적 J + LU | `dense.radnet.n128.analytic.x10` 1.21, `banded_1d.bruss1d.n10000.fd.x10` 1.19, `banded_1d.bruss1d.n10000.fd.x100` 1.18 | `dense.radnet.n128.analytic.x10` 1.21, `banded_1d.bruss1d.n1000.fd.x1` 1.21, `banded_1d.bruss1d.n10000.fd.x1` 1.20 | 상한 1.25. T7의 regime에서만 시도. step 크기 예측이 없으면 J만 겹칠 수 있다(`J only` 열) |
| (c) Jacobian 병렬·색칠 조립, P = 6 | `dense.radnet.n128.fd.x100` 4.58, `dense.radnet.n128.fd.x10` 4.05, `dense.radnet.n128.fd.x1` 2.44 | `dense.radnet.n128.fd.x100` 4.58, `dense.radnet.n2048.fd.x1` 4.37, `dense.radnet.n128.fd.x10` 4.05 | FD Jacobian 영역에서 만들 가치가 있다. 해석 Jacobian에서는 1.0x |
| (d) lagged LU, k = 4 | `dense.bruss1d.n2048.analytic.x1` 3.76, `banded_3d.ac3d.g16.analytic.x1` 3.39, `banded_3d.ac3d.g16.fd.x1` 3.13 | `dense.bruss1d.n2048.analytic.x1` 3.76, `banded_3d.ac3d.g16.analytic.x1` 3.39, `banded_2d.bruss2d.g128.analytic.x1` 3.28 | dense n ≥ 128과 2D/3D banded에서 상한이 크다. 단 Jacobian 지연은 차수를 3으로 낮추므로(S4) step 수 증가를 V09에서 재야 한다 |
| (e) dense LU 병렬화, P = 6 | `dense.bruss1d.n2048.analytic.x1` 5.42, `dense.bruss1d.n512.analytic.x1` 4.05, `dense.radnet.n2048.analytic.x1` 3.56 | `dense.bruss1d.n2048.analytic.x1` 5.42, `dense.bruss1d.n512.analytic.x1` 4.05, `dense.radnet.n2048.analytic.x1` 3.56 | faer 경로(n ≥ 512)에서 가치가 있다. in-place LU 행의 상한은 band 선언으로 대체해야 한다(3.2절) |
| (f) shared Krylov basis | 없음 | `matrix_free_2d.ac2d.g128.jvp.x1` 9.46, `matrix_free_2d.ac2d.g256.jvp.x1` 9.28, `matrix_free_2d.ac2d.g128.jvp.x10` 8.80 | 상한이 가장 크다. 다만 먼저 GMRES 주기의 조기 종료 부재를 분리해서 봐야 한다(같은 stopping test의 per-stage GMRES가 V06의 대조군) |
| (g) RHS 공간 병렬화, P = 6 | `dense.radnet.n128.analytic.x100` 4.61, `dense.radnet.n512.analytic.x100` 4.01, `banded_1d.bruss1d.n10000.analytic.x100` 3.37 | `dense.radnet.n128.analytic.x100` 4.61, `dense.radnet.n512.analytic.x100` 4.01, `dense.bruss1d.n32.analytic.x100` 3.87 | RHS 배율 10 이상에서 의미가 있다. 배율 1에서는 P = 6이어도 최대 1.22 |

### 4.0 격자 밖 후보: faer factor 저장의 재사용

3.6절의 차이는 lever 목록에 없던 여지를 가리킨다. `radnet` n = 128, 512에서 보호 driver는 같은 일을 하는 client arm보다 18–34 % 더 걸렸고(첫 측정과 반복 측정의 범위), 그 원인은 factorisation마다의 할당과 page fault다(6.2절). n = 2048에서는 두 arm이 모두 이 비용을 낸다. factor 저장과 작업 행렬을 재사용하면 방법과 궤적을 바꾸지 않고 없앨 수 있는 비용이다. 보호 파일(`rodas5p_fast.rs`, `matrix.rs`)의 일이라 이 노드에서는 측정만 했다.

### 4.1 V07 stage b gate

조건을 만족하는 regime(표 T7):

- `dense.radnet.n512.analytic.x10`: r = 0.783, 수락 step당 8515 µs, 상한 1.16 (J만 1.01), authoritative
- `banded_1d.bruss1d.n10000.analytic.x1`: r = 0.172, 수락 step당 1757 µs, 상한 1.14 (J만 1.01), diagnostic_only
- `banded_1d.bruss1d.n10000.fd.x1`: r = 0.286, 수락 step당 1914 µs, 상한 1.20 (J만 1.09), diagnostic_only
- `banded_1d.bruss1d.n100000.analytic.x1`: r = 0.202, 수락 step당 18050 µs, 상한 1.17 (J만 1.02), diagnostic_only
- `banded_1d.bruss1d.n100000.fd.x1`: r = 0.315, 수락 step당 19805 µs, 상한 1.20 (J만 1.10), diagnostic_only
- `banded_1d.bruss1d.n10000.fd.x10`: r = 0.436, 수락 step당 3236 µs, 상한 1.19 (J만 1.19), authoritative
- `banded_1d.bruss1d.n10000.fd.x100`: r = 0.627, 수락 step당 16847 µs, 상한 1.18 (J만 1.18), authoritative

이 regime들의 상한은 1.14–1.20이고, Jacobian만 겹칠 때는 1.01–1.19다. stage b의 통과 기준이 wall 1.15배이므로 여유가 거의 없다. 해석 Jacobian regime에서는 LU까지 겹쳐야 하므로 step 크기 예측이 필요하다(S6). FD Jacobian regime에서만 Jacobian 단독으로 상한이 1.15를 넘는다. 착수 조건은 충족하지만, 얻을 수 있는 최댓값이 통과 기준과 거의 같다는 점을 V07에서 먼저 따져야 한다.

## 5. 허용 기준을 만족하지 못한 측정

| cell | 라벨 | 걸린 기준 |
|---|---|---|
| `dense.hires.n8.analytic.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.hires.n8.fd.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.bruss1d.n32.analytic.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.bruss1d.n32.fd.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.bruss1d.n32.analytic.x10` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.bruss1d.n32.analytic.x100` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02 |
| `dense.bruss1d.n128.analytic.x1` | diagnostic_only | (i) instrumented/plain > 1.02 |
| `dense.radnet.n8.analytic.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.radnet.n32.analytic.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02; (ii) phase 합이 0.90–1.00 밖 |
| `dense.radnet.n32.fd.x1` | diagnostic_only | n < 128 (instruction 수가 1차 값); (i) instrumented/plain > 1.02 |
| `dense.radnet.n512.fd.x1` | diagnostic_only | (i) instrumented/plain > 1.02 |
| `banded_1d.bruss1d.n1000.analytic.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `banded_1d.bruss1d.n1000.fd.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `banded_1d.bruss1d.n10000.analytic.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `banded_1d.bruss1d.n10000.fd.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `banded_1d.bruss1d.n100000.analytic.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `banded_1d.bruss1d.n100000.fd.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `banded_2d.ac2d.g64.fd.x1` | diagnostic_only | (i) instrumented/plain > 1.02 |
| `matrix_free_2d.ac2d.g64.jvp.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_2d.ac2d.g128.jvp.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_2d.ac2d.g128.jvp.x10` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_2d.ac2d.g128.jvp.x100` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_2d.ac2d.g256.jvp.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_2d.bruss2d.g128.jvp.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_3d.ac3d.g16.jvp.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_3d.ac3d.g32.jvp.x1` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_3d.ac3d.g32.jvp.x10` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `matrix_free_3d.ac3d.g32.jvp.x100` | diagnostic_only | (ii) phase 합이 0.90–1.00 밖 |
| `dense.radnet.n2048.fd.x1` | diagnostic_only | (i) instrumented/plain > 1.02 |
| `banded_2d.bruss2d.g128.analytic.x1` | diagnostic_only | reduced protocol (1 + 3, attempt 8회); full protocol 아님 |
| `matrix_free_2d.ac2d.g128.jvpfd.x1` | 미측정 | exploratory cell |

이유를 종류별로 정리하면 다음과 같다.

- **(ii) phase 합 < 0.90, 직접법**: 1D banded 행. driver 안의 stage 벡터 연산·오차 norm·controller가 10 %를 넘는다. client 인터페이스 밖이라 보호 소스를 고치지 않고는 잴 수 없다.
- **(ii), matrix-free**: Krylov 직교화가 wall의 대부분인데 callback clock 밖이다. solve 구간 계측(확장)으로는 합이 0.99 이상이다.
- **(i) > 1.02**: timer 비용이 attempt에 비해 큰 작은 문제(n = 128 in-place 행은 반복 측정에서도 같았다), 그리고 FD Jacobian이 지배하는 cell 세 개. 뒤의 세 cell은 측정 중 host load가 변했거나 내 callgrind 작업과 겹쳤고, 반복한 두 cell은 기준을 통과했다(6.3절). 즉 run이 길고 반복 수가 적은 cell에서는 (i)의 판정이 host 상태에 민감하다.
- **reduced protocol**: 단일 run이 60 s를 넘는 cell.
- **n < 128**: 규칙상 wall은 진단값이고 instruction 수(T5)를 쓴다.

## 6. 한계와 사전등록 대비 편차

- **host 부하.** cell별 1분 load average의 최댓값은 2.5–16.1였다(다른 builder가 core 0-9, 12-21을 쓰는 중). load가 8을 넘은 cell: `banded_2d.ac2d.g64.fd.x1`, `banded_2d.ac2d.g64.analytic.x10`, `banded_2d.ac2d.g64.analytic.x100`, `banded_2d.ac2d.g128.analytic.x1`, `matrix_free_2d.ac2d.g128.jvp.x10`, `matrix_free_2d.ac2d.g128.jvp.x100`. arm 교차와 중앙값이 비율을 보호하지만, 절대 시간은 그만큼 흔들릴 수 있다.

- **편차 1 (내 작업의 겹침).** callgrind pass를 wall protocol과 동시에 다른 core complex(0-5, 12-17; L3를 공유하지 않음)에서 돌렸다. 겹친 cell: `dense.radnet.n128.fd.x100`, `dense.radnet.n512.fd.x1`, `dense.radnet.n512.analytic.x10`, `dense.radnet.n512.analytic.x100`, `dense.radnet.n2048.analytic.x1`. 사전등록은 겹침을 금하지 않았지만 의도한 바가 아니었고, 이후의 비-timing 작업(identity sweep, perf stat)은 wall protocol이 끝난 뒤에 실행했다. `RESULTS.json`의 `own_non_timing_jobs_running_during_this_cell`에 cell별로 기록돼 있다.

- **편차 2 (비-timing core 집합).** 사전등록의 `taskset -c 0-9,12-21` 대신 그 부분집합 `0-5,12-17`을 썼다(timing core와 L3를 공유하지 않는 쪽).

- **편차 4 (측정 뒤의 runner 변경).** 로그 묶음별 runner binary(SHA-256 앞 12자리)는 `6c8024535a7f`: callgrind, identity, measure, measure_repeat, perfstat, pilot; `d9d81dc869c4`: exploratory_faults이다. wall protocol이 끝난 뒤 run별 page fault counter를 넣어 다시 빌드한 binary는 `exploratory_faults`(6.2절)에만 썼다. counter는 timed 구간 밖에서 읽는다.

- **편차 3 (보고서 작성 방식).** 이 세션의 Write 도구가 subagent의 보고서 파일 작성을 거부해서, 보고서는 `build_report.py`가 결과 JSON에서 렌더링한다. 내용상의 차이는 없다.

- **줄인 horizon.** run은 t0에서 시작해 attempt 예산에서 끝난다. 직접법의 phase 비용은 h에 거의 무관하지만, 궤적의 초반(작은 step, 드문 거절)만 본 것이다. matrix-free는 step이 커질수록 주기 수가 늘어나므로, 여기서 본 것(solve당 1–3주기)보다 뒤쪽 구간의 solve가 더 비쌀 수 있다. 직교화가 지배한다는 분율은 주기 수와 무관하게 유지될 것으로 보이지만 재지 않았다.
- **허용오차 한 점.** 문제마다 rtol 1e-6 한 가지만 썼다. 직접법의 분율은 허용오차에 둔감하지만 거절 비율(Jacobian 재사용 빈도)은 달라진다.
- **in-place LU 행의 해석.** `bruss1d` dense 행의 factor는 대부분 조립과 열 주사다. LU lever 상한(d, e)을 그대로 읽으면 안 된다.
- **FD 증분.** 전진 차분, 증분 √ε·max(|y_j|, 1). 다른 FD 구현은 호출 수가 같아도 조립 비용이 다를 수 있다.
- **callgrind의 instruction 수**는 사용자 공간 instruction이다. page fault 같은 kernel 비용은 들어가지 않는다.

### 6.1 perf stat (탐색)

arm별 단일 run을 별도 프로세스로 실행하고 set-up만 하는 run(`none`)을 뺀 값이다. attempt 예산은 min(A, 20).

이 pass는 고정 core가 아니고 identity sweep과 같이 돌았으므로 cycles는 해석하지 않는다(`RESULTS.json`에는 있다). n < 128 행의 instruction 비가 큰 것은 clock 호출의 instruction이 attempt 자체와 같은 규모이기 때문이다.

| cell | attempts | instructions:u instr/plain | page faults plain | page faults instr |
|---|---|---|---|---|
| `dense.hires.n8.analytic.x1` | 20 | 1.1501 | 18 | 18 |
| `dense.hires.n8.fd.x1` | 20 | 1.1411 | 20 | 20 |
| `dense.bruss1d.n32.analytic.x1` | 20 | 1.0604 | 19 | 20 |
| `dense.bruss1d.n32.fd.x1` | 20 | 1.0433 | 19 | 17 |
| `dense.bruss1d.n32.analytic.x10` | 20 | 1.0380 | 21 | 18 |
| `dense.bruss1d.n32.analytic.x100` | 20 | 1.0083 | 21 | 21 |
| `dense.bruss1d.n128.analytic.x1` | 20 | 1.0079 | 82 | 85 |
| `dense.bruss1d.n128.fd.x1` | 20 | 1.0035 | 79 | 80 |
| `dense.bruss1d.n512.analytic.x1` | 20 | 0.9961 | 1055 | 1060 |
| `dense.bruss1d.n512.fd.x1` | 20 | 0.9987 | 1055 | 1061 |
| `dense.bruss1d.n2048.analytic.x1` | 20 | 0.9943 | 16482 | 16500 |
| `dense.bruss1d.n2048.fd.x1` | 20 | 0.9983 | 16488 | 16494 |
| `dense.radnet.n8.analytic.x1` | 20 | 1.1173 | 17 | 18 |
| `dense.radnet.n32.analytic.x1` | 20 | 1.0288 | 19 | 18 |
| `dense.radnet.n32.fd.x1` | 20 | 1.0154 | 17 | 14 |
| `dense.radnet.n128.analytic.x1` | 20 | 1.0021 | 889 | 864 |
| `dense.radnet.n128.analytic.x10` | 20 | 1.0008 | 888 | 864 |
| `dense.radnet.n128.analytic.x100` | 20 | 1.0001 | 886 | 863 |
| `dense.radnet.n128.fd.x1` | 20 | 1.0005 | 890 | 864 |
| `dense.radnet.n128.fd.x10` | 20 | 1.0001 | 889 | 865 |
| `dense.radnet.n128.fd.x100` | 20 | 1.0000 | 889 | 866 |
| `dense.radnet.n512.analytic.x1` | 20 | 0.9995 | 6780 | 17800 |
| `dense.radnet.n512.fd.x1` | 20 | 0.9999 | 6778 | 6764 |
| `dense.radnet.n512.analytic.x10` | 20 | 0.9998 | 6782 | 17803 |
| `dense.radnet.n512.analytic.x100` | 20 | 1.0000 | 17832 | 17801 |
| `dense.radnet.n2048.analytic.x1` | 20 | 0.9997 | 509058 | 508857 |
| `banded_1d.bruss1d.n1000.analytic.x1` | 20 | 1.0025 | 81 | 82 |
| `banded_1d.bruss1d.n1000.fd.x1` | 20 | 1.0020 | 82 | 81 |
| `banded_1d.bruss1d.n10000.analytic.x1` | 20 | 1.0003 | 676 | 660 |
| `banded_1d.bruss1d.n10000.fd.x1` | 20 | 1.0002 | 689 | 670 |
| `banded_1d.bruss1d.n100000.analytic.x1` | 20 | 1.0000 | 6276 | 6276 |
| `banded_1d.bruss1d.n100000.fd.x1` | 20 | 1.0000 | 7056 | 7058 |
| `banded_1d.bruss1d.n10000.analytic.x10` | 20 | 1.0001 | 679 | 663 |
| `banded_1d.bruss1d.n10000.analytic.x100` | 20 | 1.0000 | 681 | 664 |
| `banded_1d.bruss1d.n10000.fd.x10` | 20 | 1.0001 | 689 | 673 |
| `banded_1d.bruss1d.n10000.fd.x100` | 20 | 1.0000 | 691 | 669 |
| `banded_2d.ac2d.g32.analytic.x1` | 20 | 1.0001 | 385 | 383 |
| `banded_2d.ac2d.g64.analytic.x1` | 20 | 1.0000 | 2770 | 2763 |
| `banded_2d.ac2d.g64.fd.x1` | 20 | 1.0000 | 2770 | 2766 |
| `banded_2d.ac2d.g64.analytic.x10` | 20 | 1.0000 | 2770 | 2763 |
| `banded_2d.ac2d.g64.analytic.x100` | 20 | 1.0000 | 2768 | 2762 |
| `banded_2d.ac2d.g128.analytic.x1` | 20 | 1.0000 | 21221 | 21218 |
| `banded_2d.bruss2d.g32.analytic.x1` | 20 | 1.0000 | 1393 | 1395 |
| `banded_2d.bruss2d.g64.analytic.x1` | 20 | 1.0000 | 10628 | 10642 |
| `banded_3d.ac3d.g8.analytic.x1` | 20 | 1.0001 | 360 | 361 |
| `banded_3d.ac3d.g16.analytic.x1` | 20 | 1.0000 | 10450 | 10445 |
| `banded_3d.ac3d.g16.fd.x1` | 20 | 1.0000 | 10451 | 10445 |
| `matrix_free_2d.ac2d.g64.jvp.x1` | 20 | 1.0002 | 605 | 603 |
| `matrix_free_2d.ac2d.g128.jvp.x1` | 20 | 1.0001 | 2325 | 2397 |
| `matrix_free_2d.ac2d.g128.jvp.x10` | 20 | 1.0001 | 2327 | 2325 |
| `matrix_free_2d.ac2d.g128.jvp.x100` | 20 | 1.0000 | 2326 | 2327 |
| `matrix_free_2d.ac2d.g256.jvp.x1` | 20 | 1.0000 | 8994 | 8992 |
| `matrix_free_2d.bruss2d.g128.jvp.x1` | 20 | 1.0000 | 4540 | 4541 |
| `matrix_free_3d.ac3d.g16.jvp.x1` | 20 | 1.0002 | 606 | 607 |
| `matrix_free_3d.ac3d.g32.jvp.x1` | 20 | 1.0000 | 4546 | 4536 |
| `matrix_free_3d.ac3d.g32.jvp.x10` | 20 | 1.0000 | 4535 | 4535 |
| `matrix_free_3d.ac3d.g32.jvp.x100` | 20 | 1.0000 | 4541 | 4531 |
| `dense.radnet.n2048.fd.x1` | 20 | 1.0000 | 508870 | 508936 |
| `banded_2d.bruss2d.g128.analytic.x1` | 8 | 1.0000 | 83394 | 83389 |

### 6.2 page fault 진단 (탐색)

wall protocol이 끝난 뒤 runner에 run별 page fault 수(`/proc/self/stat`, timed 구간 밖에서 읽음)를 추가해 다시 빌드하고, 같은 프로토콜을 다섯 cell에 실행했다. 측정에 쓴 binary와 다른 binary다(로그 metadata의 digest 참조). 값은 timed 반복의 중앙값이다.

| cell | attempts | plain: minor fault/run | attempt당 | instrumented: minor fault/run | attempt당 | instr/plain wall | load 전/후 |
|---|---|---|---|---|---|---|---|
| `dense.bruss1d.n512.analytic.x1` | 92 | 0 | 0 | 0 | 0 | 0.975 | 8.7/8.7 |
| `dense.radnet.n128.analytic.x1` | 85 | 2773 | 33 | 1184 | 14 | 0.897 | 8.7/8.7 |
| `dense.radnet.n512.analytic.x1` | 86 | 42656 | 496 | 480 | 6 | 0.839 | 8.7/8.6 |
| `dense.radnet.n512.analytic.x100` | 34 | 27616 | 812 | 27104 | 797 | 1.027 | 8.6/9.5 |
| `dense.radnet.n2048.analytic.x1` | 20 | 508039 | 25402 | 508030 | 25402 | 0.985 | 9.5/7.1 |

읽는 법: faer 경로(`radnet` n ≥ 128)는 factorisation마다 새 메모리를 건드린다. n = 2048에서는 두 arm 모두 attempt당 25402번의 page fault를 낸다(4 KiB page로 99 MiB). n = 512에서는 heap 배치에 따라 한 arm만 낸다: 배율 1 구성에서 plain은 attempt당 496번(n × n 행렬 하나가 512 page), instrumented는 6번이고, page fault를 내는 arm이 느리다. 배율 100 구성에서는 두 arm이 같이 낸다. in-place LU(`bruss1d`)는 step 중 page fault가 없다. 따라서 3.6절의 arm 간 차이는 timer나 client driver가 아니라 faer 경로의 factorisation별 할당에서 온다. 어느 arm이 내는지는 프로세스의 heap 상태에 달려 있으므로, 사용자 프로그램에서도 같은 비용이 나타날 수 있다.

### 6.3 반복 측정 (탐색)

내 callgrind 작업과 겹쳤거나 host load가 높았던 cell을, wall protocol이 끝난 뒤 내 다른 작업이 없는 상태에서 같은 프로토콜로 한 번 더 쟀다. cell의 결과(2절의 표와 라벨)는 첫 측정이고, 아래는 비교용이다.

| cell | 첫 측정 instr/plain | phase 합 | plain µs/attempt | load 전/후 | 반복 instr/plain | phase 합 | plain µs/attempt | load 전/후 | 반복에서 (i) (ii) (iii) | 분율 차이 최대 (%p) |
|---|---|---|---|---|---|---|---|---|---|---|
| `dense.bruss1d.n128.analytic.x1` | 1.030 | 0.915 | 37 | 2.5/2.5 | 1.043 | 0.915 | 38 | 3.6/3.6 | X O O | 0.8 |
| `dense.radnet.n128.analytic.x1` | 0.845 | 0.974 | 201 | 2.8/2.8 | 0.847 | 0.973 | 203 | 3.6/3.6 | O O O | 0.3 |
| `dense.radnet.n128.fd.x100` | 0.998 | 1.000 | 41505 | 2.7/2.5 | 0.995 | 1.000 | 41724 | 3.6/3.6 | O O O | 0.0 |
| `dense.radnet.n512.analytic.x1` | 0.840 | 0.996 | 4806 | 2.5/2.5 | 0.748 | 0.996 | 5489 | 3.6/3.7 | O O O | 0.1 |
| `dense.radnet.n512.fd.x1` | 1.028 | 0.999 | 30980 | 2.5/3.2 | 1.012 | 0.999 | 31360 | 3.7/3.9 | O O O | 0.2 |
| `dense.radnet.n512.analytic.x10` | 0.885 | 0.998 | 8515 | 3.2/3.2 | 0.897 | 0.998 | 8558 | 3.9/3.9 | O O O | 0.1 |
| `dense.radnet.n512.analytic.x100` | 0.998 | 1.000 | 42389 | 3.2/3.6 | 1.006 | 0.999 | 42379 | 3.9/4.0 | O O O | 0.6 |
| `dense.radnet.n2048.analytic.x1` | 1.000 | 0.997 | 217739 | 3.6/3.3 | 0.999 | 0.997 | 218857 | 4.0/3.5 | O O O | 0.3 |
| `banded_2d.ac2d.g64.fd.x1` | 1.037 | 0.996 | 40304 | 7.2/14.4 | 0.995 | 0.997 | 35654 | 3.5/3.4 | O O O | 0.3 |
| `banded_2d.ac2d.g64.analytic.x10` | 1.001 | 0.997 | 61463 | 14.4/16.1 | 1.007 | 0.996 | 34024 | 3.4/3.5 | O O O | 1.3 |
| `banded_2d.ac2d.g64.analytic.x100` | 0.996 | 0.997 | 40404 | 16.1/11.6 | 1.000 | 0.997 | 40092 | 3.5/3.5 | O O O | 0.1 |
| `banded_2d.ac2d.g128.analytic.x1` | 1.003 | 0.998 | 500415 | 11.6/4.2 | 1.052 | 0.998 | 609431 | 3.5/11.3 | X O O | 2.4 |
| `matrix_free_2d.ac2d.g128.jvp.x10` | 1.003 | 0.074 | 293216 | 6.7/11.0 | 1.008 | 0.081 | 290234 | 11.3/11.7 | O X O | 0.7 |
| `matrix_free_2d.ac2d.g128.jvp.x100` | 1.001 | 0.148 | 298406 | 11.0/5.6 | 1.002 | 0.148 | 292170 | 11.7/4.7 | O X O | 0.1 |

두 측정 사이에서 phase 분율은 최대 2.4 %p 달랐다. 분율과 그로부터 나온 상한은 이 정도의 재현성을 가진다.

## 7. 검사

- wall protocol의 identity: 59개 cell, 526쌍(warm-up 포함) — 전부 bit-동일.
- identity sweep(세 arm, attempt 12회): 60/60개 cell에서 instrumented·marked가 plain과 bit-동일.
- callgrind run의 marked/plain 동일성: 14/14개 cell.
- clock의 호출 수와 driver counter의 일치(`clock_calls_equal_driver_counters`): 59/59개 cell.

## 8. 재현

```
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v04
cd research/production_loop_20261116/cost_anatomy/runner
taskset -c 0-9,12-21 cargo +1.94.1 build --locked --offline -j 4 --release
cd ..
python3 -I campaign.py environment
python3 -I campaign.py pilot        # taskset -c 10
python3 -I campaign.py size         # SIZING.json
python3 -I campaign.py measure      # taskset -c 10, logs/measure/
python3 -I campaign.py callgrind    # logs/callgrind/
python3 -I campaign.py identity     # logs/identity/
python3 -I campaign.py perfstat     # logs/perfstat/ (탐색)
python3 -I campaign.py repeat --cell <id> ...   # logs/measure_repeat/ (탐색, 6.3절)
python3 -I campaign.py faults --cell <id> ...   # logs/exploratory_faults/ (탐색, 6.2절)
python3 -I build_results.py         # RESULTS.json
python3 -I build_amdahl.py          # AMDAHL.json, REPORT_TABLES.md
python3 -I build_report.py          # 이 문서
```

파일: `PREDECLARED.json`(사전등록), `SIZING.json`(attempt 예산), `RESULTS.json`(cell별 결과), `AMDAHL.json`(lever별 상한), `logs/`(raw 출력: pilot, measure, callgrind, identity, perfstat, environment), `runner/`(측정 프로그램), `campaign.py`·`build_results.py`·`build_amdahl.py`·`build_report.py`.
