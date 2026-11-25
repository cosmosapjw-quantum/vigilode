# V29 조립 클라이언트의 기본 첫 사용 검사: 채색 경로는 다중 채색 검사(MC)

이 보고서는 `parse_results.py`가 `logs/`의 원시 로그(기록 A와 B, unit.log, checks.log), V08 동일성 로그, V26 코퍼스 로그와 L-0338 리뷰 probe 로그만 읽어 `RESULTS.json`과 함께 생성했다. 숫자를 손으로 적지 않았다. 기준 커밋 `e36fc6f4`, 생성 시점 HEAD `fbcd44de`.

**정정 고지 (2026-11-25, 독립 리뷰 `REVIEW.json`, 판정 accept_with_corrections).** 리뷰어가 원시 로그에서 네 판별 기준(비트 불변, 경로별 기본값, 코퍼스 검출, 테스트 묶음)을 다시 계산해 모두 PASS로 같게 얻었습니다. 아래 일곱 가지를 바로잡습니다. 아래 수치는 리뷰의 확인입니다.

1. clippy는 `stage_clients_assembly_*` 대상 10개(11개가 아님)에 대해 실행되었습니다(경고 0).
2. V26 기준 arm의 대응(seq는 MC@seq, t6과 t3은 MC@t6, 조밀 경로는 S8@seq와 S8@t3)은 실행 뒤 이 파서(`fbcd44de`)에서 처음 고정되었습니다. NOTES 1항목은 'V26의 MC와 S8 결정'만 적었습니다. B의 채색 경로 결정은 MC@seq와도 MC@t6과도 9689/9689 같으므로 결과에 영향이 없습니다. PROTOCOL.json은 쓰지 않았습니다.
3. '기존 테스트는 모두 검사를 명시한다'는 문장은 틀렸습니다. `stage_clients_assembly_identity.rs:1034-1060`은 검사를 명시하지 않고 클라이언트를 만듭니다. 검사나 그 횟수를 단언하지 않으므로 고칠 필요가 없었습니다. 맞는 문장: 검사나 그 횟수를 단언하는 기존 테스트는 모두 검사를 명시합니다.
4. 넓은 선언 밴드에서는 MC의 그룹 수(우변 평가 수)가 `n`을 넘을 수 있습니다. 리뷰의 무작위 밴드 패턴 75개(반폭 0.2n~0.45n) 중 59개에서 넘었고 최대 1.85n이었습니다(주 세션이 리뷰 probe로 재현). 이때는 전체 검사 `All`이 더 싸고 반올림 범위에서 정확합니다. 병합 뒤 문서 커밋에서 rustdoc에 이 사실을 적었습니다. 이 경우 기본값을 `All`로 바꿀지는 소유자 결정입니다.
5. CI에서 도는 테스트 `the_default_decides_as_the_multi_colouring_check_on_part_of_the_v26_corpus`는 4846개 중 527개 사례만 다룹니다. 전체 코퍼스 대조는 `run.sh`로 재현할 수 있는 1회 기록(무시된 프로그램 `v29_record`와 이 파서)입니다. CI의 ignored-tests job은 `v29_record`를 `V29_MODE` 없이 실행하므로 사용법만 출력합니다.
6. '유일한 미검출은 알려진 반올림 한계 사례'는 verify 경로에 대한 말입니다. 첫 사용 경로에서는 `controls/impure16-dense`도 seq에서 통과합니다. 순차 조밀 경로는 첫 사용 검사를 하지 않기 때문이고, 이 동작은 바뀌지 않았습니다(`RESULTS.json`의 `corpus.first_use_path_misses_B`, 3건).
7. 이 파서는 prot 행의 `bits_equal`을 무조건 true로 적었습니다. 불일치는 여전히 `bits_ok`에 들어가고 리뷰의 전수 비교에서 prot 차이는 없었으므로 보고된 결과는 틀리지 않았습니다. 파서는 이제 그 행의 불일치 수로 계산합니다. 커밋된 `RESULTS.json`은 다시 만들지 않았습니다. 전체 재실행은 `head_at_parse`와 실행 시점의 루프 검사기 필드를 바꾸기 때문입니다. 이 보고서는 `parse_results.py --report-only`로 커밋된 `RESULTS.json`에서 다시 렌더링했습니다.

## 1. 결론

- 소유자 결정 OD-L1-24(권고사항 수용): 선택형 `AssemblyStageClient`의 기본 첫 사용 검사는 MC가 적용되는 경로(채색 경로: 조밀 LU 뒷단과 두 밴드 뒷단, 순차와 스레드 실행 모두)에서 `CheckColumns::MultiColouring`이 되었다. 조밀 경로는 MC가 적용되지 않으므로(선언된 패턴 없음) `Sample(8)`을 유지한다. FA(`All`), FA_P(`AllPooled`), MC_P(`MultiColouringPooled`)와 나머지 검사는 그대로 선택할 수 있다.
- 구현: `AssemblyOptions::default_first_use_check(route)`가 경로별 기본값을 정하고 `dense()`와 `coloured()`가 이를 쓴다. MC가 첫 사용 검사인 클라이언트는 추가 채색을 `AssemblyStageClient::new`에서 만든다(첫 선형화에서 할당 없음). 조립 야코비안의 산술은 한 줄도 바뀌지 않았다.
- 판별 기준: bits unchanged PASS, default per route PASS, corpus detection of the default PASS, suites PASS.

| 판별 기준 | 판정 | 측정 |
|---|---|---|
| bits unchanged | PASS | 36 runs (5 cells, both band back ends of nlkpp-band-n4096-r24, arms prot, seq, t1, t2, t4, t6): 36 equal to commit A in Jacobian, linearization-point, output and final-state fingerprints, steps and every counter but the check's; 0 mismatches; coloured cells change only verification RHS (8 -> total_groups, +1 threaded), verified columns (8 -> n) and rhs_calls by the same amount; dense cells unchanged; commit A equals the V08 identity logs in 36 runs (0 mismatches) |
| default per route | PASS | identity cells: bruss1d-n128-coloured/banded_tight ['Sample(8)'] -> ['MultiColouring']; bruss1d-n128-dense/dense_lu(in-place) ['Sample(8)'] -> ['Sample(8)']; bruss1d-n131072/banded_tight ['Sample(8)'] -> ['MultiColouring']; nlkpp-band-n4096-r24/banded_protected ['Sample(8)'] -> ['MultiColouring']; nlkpp-band-n4096-r24/banded_tight ['Sample(8)'] -> ['MultiColouring']; nlkpp-dense-n400/dense_lu(faer) ['Sample(8)'] -> ['Sample(8)']; corpus: {"coloured@seq": ["MultiColouring"], "coloured@t3": ["MultiColouring"], "coloured@t6": ["MultiColouring"], "dense@seq": ["Sample(8)"], "dense@t3": ["Sample(8)"], "dense@t6": ["Sample(8)"]}; tests the_default_first_use_check_follows_the_route ok, the_default_check_runs_once_at_the_first_linearization_on_every_route ok |
| corpus detection of the default | PASS | 4846 V26 cases, same ids and fields: True; decisions of B equal to the V26 reference arm in 9693 of 9693 (MC@seq/MC@t6 coloured, S8@seq/S8@t3 dense); misses of B 2 (case, execution) = the V26 MC miss ['omit1024/e3072-r768-c769'], rounding-limited; misses of A (Sample(8)) 1152 (576 seq), of which 1150 detected by B; detected by A and not by B: 0; correct patterns rejected: 0 |
| suites | PASS | stage client tests: exit 0, 91 passed / 0 failed / 14 ignored in 29 binaries; sentinels: exit 0, 61 passed / 0 failed / 0 ignored in 2 binaries; clippy: exit 0; fmt: exit 0; loop checker: exit 1; edited lines without reason: 0 |

## 2. 경로별 기본값

| 셀 / 뒷단 | 경로 | 기본 검사 (변경 전, A) | 기본 검사 (변경 후, B) |
|---|---|---|---|
| bruss1d-n128-coloured/banded_tight | coloured | Sample(8) | MultiColouring |
| bruss1d-n128-dense/dense_lu(in-place) | dense | Sample(8) | Sample(8) |
| bruss1d-n131072/banded_tight | coloured | Sample(8) | MultiColouring |
| nlkpp-band-n4096-r24/banded_protected | coloured | Sample(8) | MultiColouring |
| nlkpp-band-n4096-r24/banded_tight | coloured | Sample(8) | MultiColouring |
| nlkpp-dense-n400/dense_lu(faer) | dense | Sample(8) | Sample(8) |

코퍼스(경로@실행별 기본 검사, B): `coloured@seq` MultiColouring, `coloured@t3` MultiColouring, `coloured@t6` MultiColouring, `dense@seq` Sample(8), `dense@t3` Sample(8), `dense@t6` Sample(8). 순차 조밀 경로는 예전과 같이 첫 사용 검사를 하지 않는다(비교할 두 번째 평가가 없음).

## 3. 비트 불변 (V08 동일성 셀)

같은 기록 프로그램(소스 sha256 `43f68080`)을 변경 전(A)과 변경 후(B)에 실행했다. 모든 클라이언트는 검사를 지정하지 않은 기본 옵션으로 만들었다. 야코비안 지문(모든 조립 야코비안), 선형화 지점 지문, 출력과 최종 상태 지문, 걸음 수, 첫 사용 검사 이외의 모든 카운터가 A와 같다. 바뀐 것은 첫 사용 검사의 카운터뿐이며, 그 양은 문서화한 양(채색 경로: 검사 RHS 8 → MC 그룹 합, 스레드 실행은 +1; 검사한 열 8 → n; `rhs_calls`도 같은 양)과 정확히 같다.

| 셀 | 뒷단 | 팔 | 비트 같음 | 검사 (A → B) | 검사 RHS (A → B) | 검사 열 (A → B) | rhs_calls (A → B) | 야코비안 지문 |
|---|---|---|---|---|---|---|---|---|
| nlkpp-dense-n400 | dense_lu(faer) | prot | 예 | - | - | - | - | `194829b4db8581e9` |
| nlkpp-dense-n400 | dense_lu(faer) | seq | 예 | Sample(8) → Sample(8) | 0 → 0 | 0 → 0 | 13,906 → 13,906 | `194829b4db8581e9` |
| nlkpp-dense-n400 | dense_lu(faer) | t1 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 13,915 → 13,915 | `194829b4db8581e9` |
| nlkpp-dense-n400 | dense_lu(faer) | t2 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 13,915 → 13,915 | `194829b4db8581e9` |
| nlkpp-dense-n400 | dense_lu(faer) | t4 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 13,915 → 13,915 | `194829b4db8581e9` |
| nlkpp-dense-n400 | dense_lu(faer) | t6 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 13,915 → 13,915 | `194829b4db8581e9` |
| bruss1d-n128-dense | dense_lu(in-place) | prot | 예 | - | - | - | - | `1bdfd9898cbad42b` |
| bruss1d-n128-dense | dense_lu(in-place) | seq | 예 | Sample(8) → Sample(8) | 0 → 0 | 0 → 0 | 6,576 → 6,576 | `1bdfd9898cbad42b` |
| bruss1d-n128-dense | dense_lu(in-place) | t1 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 6,585 → 6,585 | `1bdfd9898cbad42b` |
| bruss1d-n128-dense | dense_lu(in-place) | t2 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 6,585 → 6,585 | `1bdfd9898cbad42b` |
| bruss1d-n128-dense | dense_lu(in-place) | t4 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 6,585 → 6,585 | `1bdfd9898cbad42b` |
| bruss1d-n128-dense | dense_lu(in-place) | t6 | 예 | Sample(8) → Sample(8) | 9 → 9 | 8 → 8 | 6,585 → 6,585 | `1bdfd9898cbad42b` |
| nlkpp-band-n4096-r24 | banded_protected | prot | 예 | - | - | - | - | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_protected | seq | 예 | Sample(8) → MultiColouring | 8 → 150 | 8 → 4096 | 2,161 → 2,303 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_protected | t1 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_protected | t2 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_protected | t4 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_protected | t6 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_tight | prot | 예 | - | - | - | - | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_tight | seq | 예 | Sample(8) → MultiColouring | 8 → 150 | 8 → 4096 | 2,161 → 2,303 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_tight | t1 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_tight | t2 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_tight | t4 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| nlkpp-band-n4096-r24 | banded_tight | t6 | 예 | Sample(8) → MultiColouring | 9 → 151 | 8 → 4096 | 2,162 → 2,304 | `ac7e0be050e14f1b` |
| bruss1d-n131072 | banded_tight | prot | 예 | - | - | - | - | `054538965049b1a2` |
| bruss1d-n131072 | banded_tight | seq | 예 | Sample(8) → MultiColouring | 8 → 59 | 8 → 131072 | 680 → 731 | `054538965049b1a2` |
| bruss1d-n131072 | banded_tight | t1 | 예 | Sample(8) → MultiColouring | 9 → 60 | 8 → 131072 | 681 → 732 | `054538965049b1a2` |
| bruss1d-n131072 | banded_tight | t2 | 예 | Sample(8) → MultiColouring | 9 → 60 | 8 → 131072 | 681 → 732 | `054538965049b1a2` |
| bruss1d-n131072 | banded_tight | t4 | 예 | Sample(8) → MultiColouring | 9 → 60 | 8 → 131072 | 681 → 732 | `054538965049b1a2` |
| bruss1d-n131072 | banded_tight | t6 | 예 | Sample(8) → MultiColouring | 9 → 60 | 8 → 131072 | 681 → 732 | `054538965049b1a2` |
| bruss1d-n128-coloured | banded_tight | prot | 예 | - | - | - | - | `798ab0df41f545fb` |
| bruss1d-n128-coloured | banded_tight | seq | 예 | Sample(8) → MultiColouring | 8 → 18 | 8 → 128 | 680 → 690 | `798ab0df41f545fb` |
| bruss1d-n128-coloured | banded_tight | t1 | 예 | Sample(8) → MultiColouring | 9 → 19 | 8 → 128 | 681 → 691 | `798ab0df41f545fb` |
| bruss1d-n128-coloured | banded_tight | t2 | 예 | Sample(8) → MultiColouring | 9 → 19 | 8 → 128 | 681 → 691 | `798ab0df41f545fb` |
| bruss1d-n128-coloured | banded_tight | t4 | 예 | Sample(8) → MultiColouring | 9 → 19 | 8 → 128 | 681 → 691 | `798ab0df41f545fb` |
| bruss1d-n128-coloured | banded_tight | t6 | 예 | Sample(8) → MultiColouring | 9 → 19 | 8 → 128 | 681 → 691 | `798ab0df41f545fb` |

- A와 B의 불일치: 0건. A는 V08 동일성 로그와 36개 실행에서 비교했고 불일치 0건(오늘 코드의 Sample(8) 기본값이 V08 측정과 같은 비트와 카운터를 냄).
- 프로그램 단위 테스트 `a_run_with_the_default_check_keeps_every_bit`: 여섯 경로(링 패턴과 밴드의 조밀 LU, 보호 밴드 커널, 타이트 밴드 커널, 조밀 경로 둘), 순차와 2 작업자에서 기본 검사로 돈 전체 실행이 검사 없는 실행과 모든 출력 비트, 선형화 지점, 야코비안이 같고 카운터는 검사 RHS만큼만 다르다.

## 4. V26 코퍼스에서 기본값의 탐지

V26 코퍼스 4846건을 이 노드의 기록 프로그램에 옮겨 적었고, 사례 id, 차수, 선언 항목 수, 누락 항목, 클라이언트 색 수가 V26 로그와 같다(불일치 0건). 각 사례의 결정은 새 클라이언트에서 두 경로로 기록했다: 첫 `linearize`(드라이버 실행이 하는 일)와 `verify(t0, y0, 기본 검사)`(V26의 결정 규약). 두 경로는 순차 조밀 경로(검사 없음)를 빼고 모든 사례에서 같다(불일치 0건).

| 그룹 | 사례 | 잘못된 선언 | A(Sample(8)) 놓침, seq | B(기본) 놓침, seq | B 놓침, t6 | V26 MC 놓침 | V26 S8 놓침 | B = V26 기준 팔 |
|---|---|---|---|---|---|---|---|---|
| controls | 4 | 4 | 0 | 0 | 0 | 0 | 0 | 11 / 11 |
| ring | 13 | 13 | 6 | 0 | 0 | 0 | 6 | 26 / 26 |
| omit128 | 508 | 508 | 48 | 0 | 0 | 0 | 48 | 1016 / 1016 |
| omit1024 | 4092 | 4092 | 497 | 1 | 1 | 1 | 497 | 8184 / 8184 |
| omit131072 | 200 | 200 | 25 | 0 | 0 | 0 | 25 | 400 / 400 |
| narrow | 6 | 6 | 0 | 0 | 0 | 0 | 0 | 12 / 12 |
| correct | 23 | 0 | 0 | 0 | 0 | 0 | 0 | 44 / 44 |

- 기준 팔: 채색 경로 B는 V26 MC@seq(seq), MC@t6(t6, t3), 조밀 경로는 S8@seq, S8@t3. B의 결정은 비교한 9693건에서 모두 같다(불일치 0건; A도 S8 기준과 같음).
- B(새 기본값)가 놓친 경우: 2건(사례, 실행) = `omit1024/e3072-r768-c769` (seq), `omit1024/e3072-r768-c769` (t6). V26에서 FA와 MC가 함께 놓친 한 사례이며, B 로그의 증거에서도 MC가 평가한 모든 상태에서 행 비트가 같다(반올림 한계).
- A(옛 기본값 Sample(8))가 놓친 경우: 1152건(seq 576, t6 576), 그중 B가 잡은 것 1150건. A가 잡고 B가 놓친 사례: 0건. 정답 패턴을 거부한 경우: 0건.
- 조밀 경로 사례는 A와 B의 결정이 모든 실행에서 같다(불일치 0건): 조밀 경로의 기본값은 바뀌지 않았다.
- 첫 선형화 경로로 본 B의 통과(잘못된 선언): 3건 = `controls/impure16-dense` (seq), `omit1024/e3072-r768-c769` (seq), `omit1024/e3072-r768-c769` (t6). 위 반올림 한계 사례 외에는 순차 조밀 경로의 비순수 대조군뿐이다. 순차 조밀 경로는 비교할 두 번째 평가가 없어 첫 사용 검사를 하지 않으며(V08 설계, 이 노드에서 바뀌지 않음), A에서도 같다.

## 5. MC가 놓치는 것 (L-0338): 기본값도 그대로 물려받는다

러스트독(`CheckColumns`, `AssemblyOptions::default_first_use_check`, 모듈 문서)에 다음을 적었다. MC는 동결 V26 코퍼스(행마다 선언되지 않은 의존이 하나 이하)에서 FA가 잡는 잘못된 선언을 모두 잡지만 일반적으로 정확하지 않다. (a) 한 행의 선언되지 않은 결합들이 점검 상태에서 모든 채색의 모든 그룹에서 쌍으로 상쇄되면 통과한다. (b) 항목 붕괴: 행 비트는 바뀌지만 기준값을 빼고 증분으로 나눈 비교 항목이 같은 수로 반올림되면 통과한다. FA(`All`)나 FA_P(`AllPooled`)가 반올림 한계까지 정확한 선택이다(n번의 우변). 재현: `research/production_loop_20261116/assembly_check/review_probe`의 `cancel`, `collapse`.

리뷰 probe 로그(L-0338)의 결정 (탐지 = 예):

| probe | 사례 | S8@seq | MC@seq | FA@seq | FA_P@t6 |
|---|---|---|---|---|---|
| cancel | cancel-B-tridiagonal-n60 | 아니오 | 아니오 | 예 | 예 |
| cancel | cancel-G-ring-n84 | 아니오 | 아니오 | 예 | 예 |
| cancel | control-B-tridiagonal-n60-unpaired | 아니오 | 예 | 예 | 예 |
| collapse | collapse trial 10 | 예 | 아니오 | 예 | 예 |
| collapse | collapse trial 11 | 예 | 아니오 | 예 | 예 |
| collapse | collapse trial 18 | 예 | 아니오 | 예 | 예 |

- 소유자에게 알릴 점: 코퍼스에서는 새 기본값이 옛 기본값이 잡는 것을 모두 잡지만, 일반적으로는 상위 집합이 아니다. 위 항목 붕괴 세 사례는 Sample(8)이 잡고(41열이 표본 열) MC는 놓친다. 이 노드의 단위 테스트 `the_limits_stated_for_the_default_are_real`가 상쇄 두 구성(B, G)과 붕괴 첫 사례를 기본값으로 다시 확인한다(기본값 통과, FA와 FA_P 탐지).

## 6. 테스트

기존 테스트에서 고친 줄 (기본 검사나 그 카운트를 고정하는 줄만, 이유와 함께):

| 파일 | 줄 | 변경 전 | 변경 후 | 이유 |
|---|---|---|---|---|
| `stage_clients_assembly_identity.rs` | 608 | `// The default first-use check runs once, at the first linearization.` | `// The first-use check (spec() names Sample(8)) runs once, at the first linearization.` | Sample(8)을 기본 첫 사용 검사라고 부르던 주석. 이 블록은 spec()으로 Sample(8)을 명시적으로 지정하고 단언 (1, 8)은 그대로다. Sample(8)은 더 이상 채색 경로의 기본값이 아니다 |

기존 테스트는 모두 검사를 명시적으로 지정하므로(공통 도우미 `ClientSpec::options`가 항상 `with_first_use_check`를 부름) 단언문은 하나도 바꿀 필요가 없었다.

새 테스트:

- `stage_clients_assembly_default.rs`: `the_default_first_use_check_follows_the_route`, `the_default_check_runs_once_at_the_first_linearization_on_every_route`, `a_run_with_the_default_check_keeps_every_bit`, `the_default_decides_as_the_multi_colouring_check_on_part_of_the_v26_corpus`, `the_limits_stated_for_the_default_are_real`
- `stage_clients_assembly_default_alloc.rs`: `the_default_check_allocates_nothing_at_the_first_linearization`
- `stage_clients_assembly_default_record.rs`: `v29_record`

`logs/unit.log`: 통과 6, 실패 0. 할당(첫 선형화, 순차 채색 클라이언트): ring pattern, dense LU: 기본 검사 0, 검사 없음 0, 나중에 요청한 MC 12; band (2, 2), dense LU: 기본 검사 0, 검사 없음 0, 나중에 요청한 MC 12; band (2, 2), protected kernel: 기본 검사 0, 검사 없음 0, 나중에 요청한 MC 12; band (4, 4), tight kernel: 기본 검사 0, 검사 없음 0, 나중에 요청한 MC 12.

## 7. 점검 (logs/checks.log)

| 점검 | 종료 코드 | 테스트 (통과/실패/무시, 바이너리) | 마지막 줄 |
|---|---|---|---|
| stage client tests | 0 | 91/0/14, 29 | test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s |
| sentinels | 0 | 61/0/0, 2 | test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s |
| clippy | 0 | - | Finished `release` profile [optimized] target(s) in 18.13s |
| fmt | 0 | - |  |
| loop checker | 1 | - | ValueError: implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly.rs |

루프 점검기의 종료 코드 1은 위 마지막 줄의 불만 때문이다(그대로 옮김: `ValueError: implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly.rs`). 이 노드가 고치도록 허용된 클라이언트 파일의 구현 해시가 CHECKPOINT.json의 기록과 다르다는 뜻이며, CHECKPOINT.json은 병합 때 조정자가 갱신한다.

첫 불만에서 멈추지 않는 사본(`checker_all.py`, `logs/checker_all.log`, HEAD `fbcd44de`)의 불만 3건: `implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly.rs`; `implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly/engine.rs`; `implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs`. 모두 이 노드가 쓰기 허용 목록 안에서 바꾼 고정 구현 파일이다. 보호 파일, 증거 해시, 엄격 JSON, 허용 목록 점검은 통과했다.

이 노드가 바꾼 경로: `crates/rodas5p-integrators/src/stage_clients/assembly.rs`, `crates/rodas5p-integrators/src/stage_clients/assembly/engine.rs`, `crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs`, `crates/rodas5p-integrators/tests/stage_clients_assembly_default_alloc.rs`, `crates/rodas5p-integrators/tests/stage_clients_assembly_default_record.rs`, `crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs`, `research/production_loop_20261116/assembly_default/NOTES.txt`, `research/production_loop_20261116/assembly_default/checker_all.py`, `research/production_loop_20261116/assembly_default/parse_results.py`, `research/production_loop_20261116/assembly_default/run.sh` 및 로그 26개; 쓰기 허용 목록 밖: 0개.

참고: V08의 파사드 동일성 러너(`jacobian_assembly/runner/src/main.rs:133-142`, 이 노드의 쓰기 경로 밖)는 검사를 지정하지 않은 기본 옵션을 쓴다. 변경 후 그 밴드 사례는 첫 사용에서 MC를 돌리므로 `verification_rhs_calls`가 바뀐다. 파사드와의 비트 비교는 위 3절과 같은 이유로 바뀌지 않는다. 이 노드에서는 다시 돌리지 않았다.

## 8. 프로토콜 형식

- 이 노드는 소유자 결정의 구현이며 실험이나 수치 임계값이 없다. 판별 기준은 계약의 수락 점검(같음 비교)이다.
- 열려 있던 프로토콜 선택(셀, 팔, 코퍼스, 실행, 두 결정 경로, V26 기준 팔)은 첫 실행 전에 커밋했다: NOTES.txt 항목 1(`ec284167`, 2026-11-24 23:56 KST)과 기록 프로그램(`f1eb5118`, 2026-11-25 00:03 KST). 첫 실행은 00:03. 형식 편차: PROTOCOL.json이라는 파일은 쓰지 않았다(선택은 NOTES.txt와 프로그램에 있음).
- 기준 팔: A(모든 경로 Sample(8))는 V26의 S8@seq, S8@t3. B 채색 경로는 seq에 MC@seq, t6과 t3에 MC@t6. B 조밀 경로는 S8@seq, S8@t3. 조밀 경로 t6은 A와 B를 비교.
- 결정 경로: first_use = 기본 옵션의 새 클라이언트에서 첫 `linearize`; verify = 기본 옵션의 새 클라이언트에서 `AssemblyStageClient::verify(t0, y0, options.first_use_check)`(V26 규약).

## 9. 재현

```
R=research/production_loop_20261116/assembly_default
$R/run.sh build stage_clients_assembly_default_record
$R/run.sh identity <A|B> <cell>
$R/run.sh corpus <A|B> controls seq,t3,t6
$R/run.sh corpus <A|B> <ring|omit128|narrow|correct|omit1024|omit131072> seq,t6
$R/run.sh unit
$R/run.sh checks
python3 -I $R/parse_results.py
```

A는 커밋 `f1eb5118`(클라이언트 수정 전)에서, B는 `fc185e2a`(수정 후)에서 같은 프로그램으로 기록했다.
