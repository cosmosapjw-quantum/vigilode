# V22: facade banded 경로의 커널 선택 (소유자 결정 OD-L1-9)

- 사전등록: `PREDECLARED.json`, 수정 조항 `AMENDMENT_1.json` (확인 셀 반복 실행). 수치는 전부 `RESULTS.json`에서 읽었다.
- 두 커널(Indexed, Slices)은 같은 부동소수점 연산을 같은 순서로 한다. 바뀌는 것은 속도와 경로 라벨뿐이다.

> 정정 공지 (2026-11-25, 독립 리뷰 `REVIEW.json` 이후, 판정은 그대로): W = 6은 이 host(Ryzen 9 5900X, 측정 core 9)에서 대칭 strip 두 계열(n 약 8192)로 정한 값이다. 폭 6에서 cycles 기준 0.95까지 남은 여유는 0.025(acstrip, 두 번째 측정에서 권위)와 0.038(brstrip)이다. instructions 기준만 쓰면 W는 3이고, 폭 3에서 5까지는 cycles가 정한다. small-n 확인은 n = 64에서 띠 8만 쟀다. n이 작을 때의 띠 6과 7, 양쪽이 모두 6 이상인 비대칭 띠, 다른 CPU는 재지 않았다. 쓰인 binary 목록은 `BINARIES.json`의 `corrections_after_review`에 runner-a와 runner-b를 더해 바로잡았다. index 2 측정 명령은 `REVIEW.json`의 처리 기록에 있다.

## 1. 결론

- 폭 W = 6. 아래 띠와 위 띠가 모두 6 이상이면 Slices, 아니면 지금처럼 Indexed.
- 한쪽 띠만 좁은 경우는 바꾸지 않았다(Indexed 유지).
- small-n 확인(ac2d 8, 200 integrations per process (ac2dr)): instructions 0.638, cycles 0.884 (authoritative). 통과.
- facade 수준(D4, Allen-Cahn 2D 64): 직접 Indexed 호출 대비 instructions 0.225 (기준 0.6), cycles 0.394 (authoritative). 횟수와 상태 hash 동일: True.

## 2. 폭 scan (Slices / Indexed, attempt당)

유효성(기록된 모든 실행에서 두 커널의 횟수, 연산 수, 상태 hash 동일): True. 측정 로그 39개, 비교한 (로그, attempt 예산) 쌍 75개, 차이 0건.

| 반띠폭 | Allen-Cahn strip: instr | cycles | 라벨 | 측정 | Brusselator strip: instr | cycles | 라벨 | 측정 | 자격 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 1.165 | 1.014 | 권위 | 2회 중 `drive_s2_acstrip1.jsonl` | - | - | - | - | 아니오 |
| 2 | 0.982 | 1.137 | 권위 | 2회 중 `drive_s2_acstrip2.jsonl` | 0.981 | 1.146 | 권위 | 2회 중 `drive_s2_brstrip1.jsonl` | 아니오 |
| 3 | 0.933 | 1.109 | 권위 | 1회 중 `drive_s1_acstrip3.jsonl` | - | - | - | - | 아니오 |
| 4 | 0.795 | 0.942 | 권위 | 1회 중 `drive_s1_acstrip4.jsonl` | 0.791 | 1.042 | 권위 | 1회 중 `drive_s1_brstrip2.jsonl` | 아니오 |
| 5 | 0.774 | 1.032 | 권위 | 3회 중 `drive_s3_acstrip5.jsonl` | - | - | - | - | 아니오 |
| 6 | 0.672 | 0.925 | 권위 | 2회 중 `drive_s2_acstrip6.jsonl` | 0.668 | 0.912 | 권위 | 1회 중 `drive_s1_brstrip3.jsonl` | 예 |
| 8 | 0.586 | 0.845 | 권위 | 2회 중 `drive_s2_acstrip8.jsonl` | 0.583 | 0.857 | 권위 | 1회 중 `drive_s1_brstrip4.jsonl` | 예 |
| 10 | 0.524 | 0.793 | 권위 | 1회 중 `drive_s1_acstrip10.jsonl` | - | - | - | - | 예 |
| 12 | 0.477 | 0.745 | 권위 | 1회 중 `drive_s1_acstrip12.jsonl` | 0.474 | 0.742 | 권위 | 3회 중 `drive_s3_brstrip6.jsonl` | 예 |
| 16 | 0.410 | 0.664 | 권위 | 1회 중 `drive_s1_acstrip16.jsonl` | 0.408 | 0.658 | 권위 | 1회 중 `drive_s1_brstrip8.jsonl` | 예 |
| 24 | 0.332 | 0.562 | 권위 | 2회 중 `drive_s2_acstrip24.jsonl` | 0.331 | 0.561 | 권위 | 1회 중 `drive_s1_brstrip12.jsonl` | 예 |
| 32 | 0.290 | 0.470 | 권위 | 2회 중 `drive_s2_acstrip32.jsonl` | 0.289 | 0.470 | 권위 | 3회 중 `drive_s3_brstrip16.jsonl` | 예 |
| 64 | 0.219 | 0.381 | 권위 | 3회 중 `drive_s3_acstrip64.jsonl` | 0.219 | 0.386 | 권위 | 3회 중 `drive_s3_brstrip32.jsonl` | 예 |

자격: 그 폭에서 측정한 모든 계열이 instructions와 cycles 모두 0.95 이하이고 cycles가 권위 측정일 때. W는 그 폭 이상 전부가 자격을 갖는 가장 작은 폭.

측정 칸: 그 셀을 잰 횟수(최대 3회)와 판정에 쓴 로그. 품질 규칙(1분 load 8 이하, 커널별 cycles 분산 0.10 이하)을 처음 만족한 측정이 판정한다. 만족한 측정이 없으면 마지막 측정을 진단용으로 보인다.
- 중단된 측정 `logs/drive_s1_acstrip1.jsonl`: 기록된 실행 2개, 두 커널을 비교할 수 있었던 예산 [], 짝이 없는 예산 [1, 60]. 그 셀의 측정 1회로 센다. 중단 이유는 host 상태였고 결과를 보고 멈춘 것이 아니다(PROGRESS.md 22:48, 23:00).
- 중단된 측정 `logs/drive_s1_acstrip2.jsonl`: 기록된 실행 3개, 두 커널을 비교할 수 있었던 예산 [1], 짝이 없는 예산 [60]. 그 셀의 측정 1회로 센다. 중단 이유는 host 상태였고 결과를 보고 멈춘 것이 아니다(PROGRESS.md 22:48, 23:00).

## 3. 구현 검증

### D1 커널 인자를 받는 진입점 = 기존 진입점 (bit 단위)

- 로그: `logs/d1_identity.log`
  - `test the_dense_output_twins_refuse_the_other_kernels ... ok`
  - `V22_D1_EXCHANGE largest_step=7.204669554265517e-1 diagonal=7.549045336734997e0 below=1e3`
  - `test the_exchange_chain_exchanges_rows ... ok`
  - `test a_run_stopped_by_its_budget_keeps_its_partial_record ... ok`
  - `V22_D1_ENDPOINT case=allen-cahn-strip n=384 lower=16 upper=16 attempts_scalar=16 attempts_vector=16 rejected_scalar=0 factor_operations=6189040 solve_operations=4597760`
  - `V22_D1_DENSE case=allen-cahn-strip n=384 interior=Off segments=16 attempts=16 rejected=0 interior_samples=0`
  - `V22_D1_ENDPOINT case=brusselator-strip n=256 lower=8 upper=8 attempts_scalar=25 attempts_vector=25 rejected_scalar=2 factor_operations=1726525 solve_operations=2440000`
  - `V22_D1_ENDPOINT case=exchange-chain n=120 lower=2 upper=2 attempts_scalar=41 attempts_vector=39 rejected_scalar=0 factor_operations=111315 solve_operations=503152`
  - `test endpoint_twins_equal_the_existing_entry_points ... ok`
  - `V22_D1_DENSE case=allen-cahn-strip n=384 interior=WFilteredEnforce segments=16 attempts=16 rejected=0 interior_samples=16`
  - `V22_D1_DENSE case=brusselator-strip n=256 interior=Off segments=22 attempts=23 rejected=1 interior_samples=0`
  - `V22_D1_DENSE case=brusselator-strip n=256 interior=WFilteredEnforce segments=22 attempts=23 rejected=1 interior_samples=22`
  - `V22_D1_DENSE case=exchange-chain n=120 interior=Off segments=40 attempts=40 rejected=0 interior_samples=0`
  - `V22_D1_DENSE case=exchange-chain n=120 interior=WFilteredEnforce segments=40 attempts=40 rejected=0 interior_samples=40`
  - `test dense_output_twins_equal_the_existing_entry_points ... ok`
  - `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s`

### D2 facade 기록: 라벨 두 필드만 변경

- 로그: `logs/d2_facade.log`
  - `test the_kernel_changes_at_the_declared_width ... ok`
  - `V22_D2_LABELS width=6 runs_on_slices=28 runs_on_indexed=36`
  - `test the_route_labels_follow_the_width_rule ... ok`
  - `test banded_facade_runs_keep_their_record ... ok`
  - `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s`
  - `test format_helpers_are_bit_faithful ... ok`
  - `test golden_directory_is_exactly_the_listed_files ... ok`
  - `test rodas5p_contracts ... ok`
  - `test dopri5 ... ok`
  - `test production_automatic ... ok`
  - `test rodas5p_robertson ... ok`
  - `test rodas5p_vanderpol ... ok`
  - `test rodas5p_hires ... ok`
  - `test decision_grid ... ok`
  - `test general_ode ... ok`
  - `test rodas5p_brusselator ... ok`
  - `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s`
  - `D2_GOLDEN records 64 against commit 66c558a0 (recorded before the facade edit): changed field sets {'none': 36, 'driver_id + entry_point': 28}`
  - `D2_GOLDEN width W = 6; bands whose records changed (labels only): [(6, 6), (8, 8), (12, 12), (16, 16), (32, 32)]`
  - `D2_GOLDEN bands whose records are unchanged: [(1, 8), (1, 16), (2, 2), (3, 3), (4, 4), (8, 1), (16, 1)]`
  - `D2_GOLDEN holds: True`

### D3 다른 곳 불변 (golden, sentinel, suite, lint)

- 로그: `logs/d3_summary.log`
  - `fmt exit=0`
  - `clippy_facade exit=0`
  - `clippy_integrators exit=0`
  - `integrators exit=0`
  - `facade exit=0`
  - `D3_TOTAL integrators: test binaries 38, passed 342, failed 0, ignored 14`
  - `D3_TOTAL facade: test binaries 59, passed 536, failed 0, ignored 0`
  - `D3_GOLDEN default-path golden files changed against commit 66c558a0: 0`

## 4. 바뀐 것과 안 바뀐 것

- 바뀐 것: `Rodas5p::solve`의 banded 경로 네 개(끝점/dense output, scalar/vector atol)가 넓은 띠에서 Slices를 쓴다. 그때 경로 기록의 진입점 이름과 driver 식별자가 실제로 실행된 것으로 바뀐다(`..._with_kernel`, `-slices`).
- 안 바뀐 것: 결과 bit, 횟수, counter. 좁은 띠의 경로와 라벨. 커널 자체. 기존 진입점과 그 호출자(CLI, 예제, 벤치, `BandedStageLu`).
- 다루지 않은 것: dense Jacobian에 띠를 선언해 router(규칙 1)로 가는 경로는 Indexed 그대로다. 띠 2 전용 fixed-width 커널은 쓰지 않았다.

## 5. 탐색 측정 (판정에 쓰지 않음)

host가 바쁠 때 잰 값이다. instructions는 결정적이라 참고가 되고, cycles는 진단용이다.

| 계열 | 반띠폭 | instr | cycles |
|---|---|---|---|
| acstrip | 1 | 1.165 | 1.113 |
| acstrip | 2 | 0.982 | 1.027 |
| acstrip | 3 | 0.933 | 1.097 |
| acstrip | 4 | 0.795 | 0.989 |
| acstrip | 5 | 0.774 | 1.009 |
| acstrip | 6 | 0.672 | 0.911 |
| acstrip | 8 | 0.586 | 0.840 |
| acstrip | 10 | 0.524 | 0.776 |
| acstrip | 12 | 0.477 | 0.794 |
| acstrip | 16 | 0.410 | 0.678 |
| acstrip | 24 | 0.332 | 0.568 |
| acstrip | 32 | 0.290 | 0.463 |
| acstrip | 64 | 0.219 | 0.369 |
| brstrip | 2 | 0.981 | 1.148 |
| brstrip | 4 | 0.791 | 1.042 |
| brstrip | 6 | 0.668 | 0.943 |
| brstrip | 8 | 0.583 | 0.843 |
| brstrip | 12 | 0.474 | 0.746 |
| brstrip | 16 | 0.408 | 0.680 |
| brstrip | 24 | 0.331 | 0.585 |
| brstrip | 32 | 0.289 | 0.488 |
| brstrip | 64 | 0.219 | 0.393 |

