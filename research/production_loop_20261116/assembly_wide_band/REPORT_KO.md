# 노드 V31 보고서: 넓은 선언 띠에서 조립 클라이언트의 기본 첫 사용 검사 (OD-L1-27)

이 문서는 `parse_results.py`가 `logs/`의 원시 로그에서 계산해 만든다. 손으로 적은 수치는 없다. 기준 커밋 `d4e2772f`, 파싱 시점 HEAD `06485d86`.

**정정 고지 (2026-11-25, 독립 리뷰 `REVIEW.json`, 판정 accept_with_corrections).** 리뷰어가 자기 파서로 모든 수치를 다시 얻었다. 아래 세 가지를 바로잡는다. 이 고지는 `parse_results.py --report-only`로 커밋된 `RESULTS.json`에서 렌더링했다.

1. 'A가 잡은 잘못된 선언을 B가 놓친 경우 0'과 rustdoc의 '같은 비용이면 정확하다'는 시험한 집합에서만 성립한다. 일반적으로 새 기본값 `All`은 반올림 때문에, V29의 기본값 `MultiColouring`이 잡는 잘못된 선언을 놓칠 수 있다. 리뷰어가 만든 예(1e6 항 옆의 약 5.6e-9 결합: 그 열만 움직이면 반올림으로 사라지고, 그룹이 선언된 열과 함께 움직이면 비트가 바뀐다)에서 commit A의 기본값은 160/160을 잡고 HEAD의 기본값은 160/160을 놓쳤다. 실무 영향은 무시할 만하다. 병합 때 rustdoc에 '반올림 범위에서 정확'이 MC의 상위집합을 뜻하지 않는다는 문장을 넣었다.
2. NOTES.txt의 commit 시각 몇 개가 git과 다르다(예: bb09dfb4는 09:42가 아니라 09:36:58). 순서는 맞고, git 시각이 기준이다.
3. rustdoc의 '(넓은 선언 띠)'는 불완전했다. 불규칙 패턴(추가 채색이 단일 열이라 두 검사가 같은 상태를 움직임)과 작은 대각·빈 패턴에서도 `All`이 된다. 병합 때 rustdoc를 고쳤다.

## 1. 결론

- **identity unchanged**: 통과. 36 runs; field mismatches A/B: 0; cell record mismatches: 0; total_groups of the coloured cells: [18, 59, 150]
- **rule per route implemented**: 통과. rule mismatches: identity 0, corpus 0, survey 0, supplement 0; unit tests passed 11, failed 0
- **corpus detection not worse than A**: 통과. corpus: detected by A not by B 0, misses A 5, B 5, correct rejected by B 0; supplement: detected by A not by B 0, misses B 0, correct rejected by B 0
- **suites**: 통과. stage client tests and sentinels: exit 0, 157 passed, 0 failed in 34 binaries; clippy: exit 0; fmt: exit 0; loop checker: exit 1; loop checker complaints: ['implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly.rs']

채색(coloured) 경로의 기본 첫 사용 검사는 이제 선언 패턴의 `CheckColourings::total_groups`가 차수 n 이상이면 `CheckColumns::All`(모든 열을 하나씩, 반올림 범위에서 정확, 우변 n회), 그보다 작으면 이전처럼 `MultiColouring`이다. 조밀(dense) 경로는 `Sample(8)` 그대로다. 판정에는 선언 패턴만 쓰인다(Curtis-Powell-Reid 채색, 띠 폭, 차수). 상태나 우변 평가는 들어가지 않으므로 옵션을 만들 때 이미 정해진다.

비용: 조사의 378개 무작위 띠 패턴에서 기본 검사의 우변 합계가 A 70871에서 B 62792로 줄었다(n의 합 80892). 보충 집합에서 total_groups > n인 올바른 패턴의 첫 사용 검사 우변: random / correct / total_groups > n seq: A 2354, B 1608; random / correct / total_groups > n t6: A 2368, B 1622. V08 식별성 셀과 V26 말뭉치의 비용은 바뀌지 않았다(링 6개는 n = total_groups라 같은 비용).

## 2. 구현

- `AssemblyOptions::default_first_use_check(Coloured(pattern))`: `CheckColourings::total_groups_of(pattern) >= n`이면 `All`, 아니면 `MultiColouring`. `coloured(pattern)`이 이 값을 쓰고 `threaded()`, `with_fingerprints()`는 유지하며 `with_first_use_check()`는 바꾼다. 모든 검사는 계속 선택할 수 있다.
- `CheckColourings::total_groups_of`(새 공개 함수)는 채색을 만들지 않고 두 구성(띠, 세분)의 합계 중 작은 값을 계산한다. 합계 계산은 `from_parts`와 같은 보조 함수(`band_plan`, `refinement_plan`)를 공유하므로 산술이 같다. 단위 테스트가 168개 무작위 패턴과 고정 패턴에서 `CheckColourings::of(..).total_groups()`와 같음을 확인한다.
- 기본값이 `All`인 순차 클라이언트는 `new()`에서 추가 채색을 만들지 않으며(필요 없음), 전체 검사는 클라이언트 자신의 작업 공간을 쓰므로 첫 선형화와 이후 선형화에서 할당이 없다(할당 테스트, 6절).
- 조립된 야코비안의 산술은 바뀌지 않았다.

## 3. 녹화의 출처

- A: 변경 전 코드(V29 기본값), 빌드 커밋 1952c87c; B: 변경 후, 빌드 커밋 63170db9. 로그 수 A 14, B 14.
- 같은 프로그램 소스(A와 B): True; 깨끗한 작업 트리: True; 종료 코드 0: True; 클라이언트 소스 해시가 A와 B 사이에서 바뀜: True; 바이너리가 다름: True.
- V29 녹화 프로그램(`stage_clients_assembly_default_record.rs`)은 수정하지 않고 재사용했다. 이 노드의 프로그램(`stage_clients_assembly_wide_band_record.rs`)은 조사(survey)와 보충 집합을 녹화한다.
- 커밋 A는 V29의 B 녹화와 같다: 식별성 36회 실행 중 불일치 0, 말뭉치 4846개 경우 중 불일치 0.

## 4. V08 식별성 셀: A와 B

36회 실행(셀 5개, nlkpp-band-n4096-r24는 두 띠 백엔드, 팔 prot, seq, t1, t2, t4, t6)의 모든 기록 필드를 비교했다. 필드 불일치 0, 셀 기록 불일치 0. 채색 셀의 total_groups는 모두 n보다 작아 규칙이 `MultiColouring`을 유지하므로 검사 카운터도 같다.


| 셀 | 백엔드 | n | total_groups | 기본값 A | 기본값 B |
|---|---|---|---|---|---|
| nlkpp-dense-n400 | dense_lu(faer) | 400 | - | Sample(8) | Sample(8) |
| bruss1d-n128-dense | dense_lu(in-place) | 128 | - | Sample(8) | Sample(8) |
| nlkpp-band-n4096-r24 | banded_tight | 4096 | 150 | MultiColouring | MultiColouring |
| nlkpp-band-n4096-r24 | banded_protected | 4096 | 150 | MultiColouring | MultiColouring |
| bruss1d-n131072 | banded_tight | 131072 | 59 | MultiColouring | MultiColouring |
| bruss1d-n128-coloured | banded_tight | 128 | 18 | MultiColouring | MultiColouring |


| 셀 | 백엔드 | 팔 | 검사(B) | 검사 우변 A / B | 우변 합계 A / B | 야코비안 지문 | 최종 상태 지문 | 모든 필드 같음 |
|---|---|---|---|---|---|---|---|---|
| nlkpp-dense-n400 | dense_lu(faer) | seq | Sample(8) | 0 / 0 | 13906 / 13906 | 194829b4db8581e9 | 3420708b661a5d39 | 예 |
| nlkpp-dense-n400 | dense_lu(faer) | t1 | Sample(8) | 9 / 9 | 13915 / 13915 | 194829b4db8581e9 | 3420708b661a5d39 | 예 |
| nlkpp-dense-n400 | dense_lu(faer) | t2 | Sample(8) | 9 / 9 | 13915 / 13915 | 194829b4db8581e9 | 3420708b661a5d39 | 예 |
| nlkpp-dense-n400 | dense_lu(faer) | t4 | Sample(8) | 9 / 9 | 13915 / 13915 | 194829b4db8581e9 | 3420708b661a5d39 | 예 |
| nlkpp-dense-n400 | dense_lu(faer) | t6 | Sample(8) | 9 / 9 | 13915 / 13915 | 194829b4db8581e9 | 3420708b661a5d39 | 예 |
| bruss1d-n128-dense | dense_lu(in-place) | seq | Sample(8) | 0 / 0 | 6576 / 6576 | 1bdfd9898cbad42b | f56aca057fe92bc6 | 예 |
| bruss1d-n128-dense | dense_lu(in-place) | t1 | Sample(8) | 9 / 9 | 6585 / 6585 | 1bdfd9898cbad42b | f56aca057fe92bc6 | 예 |
| bruss1d-n128-dense | dense_lu(in-place) | t2 | Sample(8) | 9 / 9 | 6585 / 6585 | 1bdfd9898cbad42b | f56aca057fe92bc6 | 예 |
| bruss1d-n128-dense | dense_lu(in-place) | t4 | Sample(8) | 9 / 9 | 6585 / 6585 | 1bdfd9898cbad42b | f56aca057fe92bc6 | 예 |
| bruss1d-n128-dense | dense_lu(in-place) | t6 | Sample(8) | 9 / 9 | 6585 / 6585 | 1bdfd9898cbad42b | f56aca057fe92bc6 | 예 |
| nlkpp-band-n4096-r24 | banded_protected | seq | MultiColouring | 150 / 150 | 2303 / 2303 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_protected | t1 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_protected | t2 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_protected | t4 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_protected | t6 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_tight | seq | MultiColouring | 150 / 150 | 2303 / 2303 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_tight | t1 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_tight | t2 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_tight | t4 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| nlkpp-band-n4096-r24 | banded_tight | t6 | MultiColouring | 151 / 151 | 2304 / 2304 | ac7e0be050e14f1b | 6db303b1d8c4fa03 | 예 |
| bruss1d-n131072 | banded_tight | seq | MultiColouring | 59 / 59 | 731 / 731 | 054538965049b1a2 | 57ec697f9e76eac6 | 예 |
| bruss1d-n131072 | banded_tight | t1 | MultiColouring | 60 / 60 | 732 / 732 | 054538965049b1a2 | 57ec697f9e76eac6 | 예 |
| bruss1d-n131072 | banded_tight | t2 | MultiColouring | 60 / 60 | 732 / 732 | 054538965049b1a2 | 57ec697f9e76eac6 | 예 |
| bruss1d-n131072 | banded_tight | t4 | MultiColouring | 60 / 60 | 732 / 732 | 054538965049b1a2 | 57ec697f9e76eac6 | 예 |
| bruss1d-n131072 | banded_tight | t6 | MultiColouring | 60 / 60 | 732 / 732 | 054538965049b1a2 | 57ec697f9e76eac6 | 예 |
| bruss1d-n128-coloured | banded_tight | seq | MultiColouring | 18 / 18 | 690 / 690 | 798ab0df41f545fb | f56aca057fe92bc6 | 예 |
| bruss1d-n128-coloured | banded_tight | t1 | MultiColouring | 19 / 19 | 691 / 691 | 798ab0df41f545fb | f56aca057fe92bc6 | 예 |
| bruss1d-n128-coloured | banded_tight | t2 | MultiColouring | 19 / 19 | 691 / 691 | 798ab0df41f545fb | f56aca057fe92bc6 | 예 |
| bruss1d-n128-coloured | banded_tight | t4 | MultiColouring | 19 / 19 | 691 / 691 | 798ab0df41f545fb | f56aca057fe92bc6 | 예 |
| bruss1d-n128-coloured | banded_tight | t6 | MultiColouring | 19 / 19 | 691 / 691 | 798ab0df41f545fb | f56aca057fe92bc6 | 예 |

V29 프로그램의 메타 필드(띠 (1, 1), 차수 4의 기본값)는 A ['MultiColouring'], B ['All']이다. 이 패턴은 total_groups 4 = n이라 규칙대로 바뀌었다(경우가 아닌 메타 필드).

## 5. V26 말뭉치: A와 B

경우 4846개(A), 4846개(B), 같은 식별자: True. 경로별 결정 9696건(경우 x 실행).

- 규칙이 `All`을 고르는 경우는 6개이며 모두 올바른 패턴이고 total_groups = n(같음)이다: correct/ring-n12-true, correct/ring-n13-true, correct/ring-n14-true, correct/ring-n15-true, correct/ring-n16-true, correct/ring-n32-true.
- 나머지 4840개 경우는 A와 B의 기록이 모든 필드에서 같다(불일치 0).
- A가 찾고 B가 놓친 잘못된 선언: 0. 놓친 수 A 5, B 5(같은 목록: True). 놓친 경우: controls/impure16-dense seq first_use (Sample(8), 반올림 한계: None); omit1024/e3072-r768-c769 seq first_use (MultiColouring, 반올림 한계: True); omit1024/e3072-r768-c769 seq verify (MultiColouring, 반올림 한계: True); omit1024/e3072-r768-c769 t6 first_use (MultiColouring, 반올림 한계: True); omit1024/e3072-r768-c769 t6 verify (MultiColouring, 반올림 한계: True). 순차 조밀 경로의 불순 대조군은 첫 사용 검사를 하지 않으므로(변경 없음) 첫 사용 경로에서 통과하고, e3072-r768-c769는 V26에서 전체 검사와 다중 채색 검사가 모두 놓친 반올림 한계 경우다.
- 거부된 올바른 패턴: A 0, B 0.


| 경우 | n | total_groups | 실행 | 검사 A / B | 첫 사용 결정 A / B | 검사 우변 A / B |
|---|---|---|---|---|---|---|
| correct/ring-n12-true | 12 | 12 | seq | MultiColouring / All | pass / pass | 12 / 12 |
| correct/ring-n12-true | 12 | 12 | t6 | MultiColouring / All | pass / pass | 13 / 13 |
| correct/ring-n13-true | 13 | 13 | seq | MultiColouring / All | pass / pass | 13 / 13 |
| correct/ring-n13-true | 13 | 13 | t6 | MultiColouring / All | pass / pass | 14 / 14 |
| correct/ring-n14-true | 14 | 14 | seq | MultiColouring / All | pass / pass | 14 / 14 |
| correct/ring-n14-true | 14 | 14 | t6 | MultiColouring / All | pass / pass | 15 / 15 |
| correct/ring-n15-true | 15 | 15 | seq | MultiColouring / All | pass / pass | 15 / 15 |
| correct/ring-n15-true | 15 | 15 | t6 | MultiColouring / All | pass / pass | 16 / 16 |
| correct/ring-n16-true | 16 | 16 | seq | MultiColouring / All | pass / pass | 16 / 16 |
| correct/ring-n16-true | 16 | 16 | t6 | MultiColouring / All | pass / pass | 17 / 17 |
| correct/ring-n32-true | 32 | 32 | seq | MultiColouring / All | pass / pass | 32 / 32 |
| correct/ring-n32-true | 32 | 32 | t6 | MultiColouring / All | pass / pass | 33 / 33 |

규칙 경우에서 A와 B 기록이 다른 필드: correct/ring-n12-true: .check_colourings, .executions.seq.first_use_check, .executions.t6.first_use_check; correct/ring-n13-true: .check_colourings, .executions.seq.first_use_check, .executions.t6.first_use_check; correct/ring-n14-true: .check_colourings, .executions.seq.first_use_check, .executions.t6.first_use_check; correct/ring-n15-true: .check_colourings, .executions.seq.first_use_check, .executions.t6.first_use_check; correct/ring-n16-true: .check_colourings, .executions.seq.first_use_check, .executions.t6.first_use_check; correct/ring-n32-true: .check_colourings, .executions.seq.first_use_check, .executions.t6.first_use_check.

### 첫 사용 검사 비용(우변 수), 경우 분류별

검사 우변은 통과한 경우의 `verification_rhs_calls` 합이다(실패한 검사는 일찍 멈추므로 비용으로 세지 않는다). 우변 합계는 첫 사용 경로의 모든 우변이다.


| 분류 | 실행 | 경우 | 검사 A | 검사 B | 통과 A / B | 검사 우변 A / B | 우변 합계 A / B |
|---|---|---|---|---|---|---|---|
| controls / coloured | seq | 3 | MultiColouring 3 | MultiColouring 3 | 0 / 0 | 0 / 0 | 11 / 11 |
| controls / coloured | t3 | 3 | MultiColouring 3 | MultiColouring 3 | 0 / 0 | 0 / 0 | 15 / 15 |
| controls / coloured | t6 | 3 | MultiColouring 3 | MultiColouring 3 | 0 / 0 | 0 / 0 | 15 / 15 |
| controls / dense | seq | 1 | Sample(8) 1 | Sample(8) 1 | 1 / 1 | 0 / 0 | 17 / 17 |
| controls / dense | t3 | 1 | Sample(8) 1 | Sample(8) 1 | 0 / 0 | 0 / 0 | 18 / 18 |
| controls / dense | t6 | 1 | Sample(8) 1 | Sample(8) 1 | 0 / 0 | 0 / 0 | 18 / 18 |
| correct / coloured | seq | 21 | MultiColouring 21 | All 6, MultiColouring 15 | 21 / 21 | 830 / 830 | 982 / 982 |
| correct / coloured | t6 | 21 | MultiColouring 21 | All 6, MultiColouring 15 | 21 / 21 | 851 / 851 | 1003 / 1003 |
| correct / dense | seq | 2 | Sample(8) 2 | Sample(8) 2 | 2 / 2 | 0 / 0 | 530 / 530 |
| correct / dense | t6 | 2 | Sample(8) 2 | Sample(8) 2 | 2 / 2 | 18 / 18 | 548 / 548 |
| narrow / coloured | seq | 6 | MultiColouring 6 | MultiColouring 6 | 0 / 0 | 0 / 0 | 56 / 56 |
| narrow / coloured | t6 | 6 | MultiColouring 6 | MultiColouring 6 | 0 / 0 | 0 / 0 | 118 / 118 |
| omit1024 / coloured | seq | 4092 | MultiColouring 4092 | MultiColouring 4092 | 1 / 1 | 29 / 29 | 16755 / 16755 |
| omit1024 / coloured | t6 | 4092 | MultiColouring 4092 | MultiColouring 4092 | 1 / 1 | 30 / 30 | 22636 / 22636 |
| omit128 / coloured | seq | 508 | MultiColouring 508 | MultiColouring 508 | 0 / 0 | 0 / 0 | 2076 / 2076 |
| omit128 / coloured | t6 | 508 | MultiColouring 508 | MultiColouring 508 | 0 / 0 | 0 / 0 | 2806 / 2806 |
| omit131072 / coloured | seq | 200 | MultiColouring 200 | MultiColouring 200 | 0 / 0 | 0 / 0 | 816 / 816 |
| omit131072 / coloured | t6 | 200 | MultiColouring 200 | MultiColouring 200 | 0 / 0 | 0 / 0 | 1108 / 1108 |
| ring / coloured | seq | 13 | MultiColouring 13 | MultiColouring 13 | 0 / 0 | 0 / 0 | 94 / 94 |
| ring / coloured | t6 | 13 | MultiColouring 13 | MultiColouring 13 | 0 / 0 | 0 / 0 | 110 / 110 |

## 6. 보충 집합(규칙이 `All`을 고르는 잘못된 선언)

V26 말뭉치에는 규칙이 `All`을 고르는 잘못된 선언이 없어서, 같은 프로그램으로 A와 B를 녹화한 보충 집합을 더했다: 무작위 띠 패턴(차수 60, 96, 128, 200, 반폭 0.2/0.3/0.45 n, 밀도 0.1/0.3; 올바른 패턴, 띠 전체 상위 집합, 한 항목 누락 16개), 넓은 띠의 Fisher-KPP(차수 60, 128; 올바른 띠와 하나 좁은 띠), total_groups = n인 링 6개의 모든 한 항목 누락, 그리고 규칙이 `MultiColouring`을 유지하는 좁은 대조군.

- 경우 A 799, B 799(같은 식별자: True); 규칙이 `All`을 고르는 경우 612, 그중 잘못된 선언 574.

| 계열 | 규칙 | 경우 |
|---|---|---|
| band | rule keeps MultiColouring | 6 |
| band | rule picks All | 18 |
| narrow | rule keeps MultiColouring | 37 |
| random | rule keeps MultiColouring | 144 |
| random | rule picks All | 288 |
| ring | rule picks All | 306 |

- A가 찾고 B가 놓친 잘못된 선언: 0; 놓친 수 A 0, B 0; 거부된 올바른 패턴 A 0, B 0.
- 기본값과 무관한 명시적 전체 검사와 명시적 다중 채색 검사의 결정은 A와 B에서 같다(불일치 0). 규칙이 `MultiColouring`을 유지하는 경우의 기록도 같다(불일치 0).
- 규칙이 `All`을 고르는 잘못된 선언을 누가 찾았나(첫 사용 경로):

| 실행 | 찾은 곳 | 경우 |
|---|---|---|
| seq | build (UncoveredRowChanged) | 436 |
| seq | the check | 138 |
| t6 | build (UncoveredRowChanged) | 436 |
| t6 | the check | 138 |


| 실행 | 오류 종류 A | 오류 종류 B | 경우 |
|---|---|---|---|
| seq | MultiColouringEntryDiffers | AliasedEntry | 58 |
| seq | MultiColouringEntryDiffers | UndeclaredEntry | 9 |
| seq | MultiColouringUncoveredRow | AliasedEntry | 6 |
| seq | MultiColouringUncoveredRow | UndeclaredEntry | 65 |
| seq | UncoveredRowChanged | UncoveredRowChanged | 436 |
| t6 | MultiColouringEntryDiffers | AliasedEntry | 58 |
| t6 | MultiColouringEntryDiffers | UndeclaredEntry | 9 |
| t6 | MultiColouringUncoveredRow | AliasedEntry | 6 |
| t6 | MultiColouringUncoveredRow | UndeclaredEntry | 65 |
| t6 | UncoveredRowChanged | UncoveredRowChanged | 436 |

### 첫 사용 검사 비용(우변 수), 보충 집합의 분류별


| 분류 | 실행 | 경우 | 검사 A | 검사 B | 통과 A / B | 검사 우변 A / B | 우변 합계 A / B |
|---|---|---|---|---|---|---|---|
| band / band one too narrow / total_groups < n | seq | 4 | MultiColouring 4 | MultiColouring 4 | 0 / 0 | 0 / 0 | 194 / 194 |
| band / band one too narrow / total_groups < n | t6 | 4 | MultiColouring 4 | MultiColouring 4 | 0 / 0 | 0 / 0 | 380 / 380 |
| band / band one too narrow / total_groups = n | seq | 12 | MultiColouring 12 | All 12 | 0 / 0 | 0 / 0 | 230 / 230 |
| band / band one too narrow / total_groups = n | t6 | 12 | MultiColouring 12 | All 12 | 0 / 0 | 0 / 0 | 764 / 764 |
| band / correct / total_groups < n | seq | 2 | MultiColouring 2 | MultiColouring 2 | 2 / 2 | 0 / 0 | 190 / 190 |
| band / correct / total_groups < n | t6 | 2 | MultiColouring 2 | MultiColouring 2 | 2 / 2 | 2 / 2 | 192 / 192 |
| band / correct / total_groups = n | seq | 6 | MultiColouring 6 | All 6 | 6 / 6 | 564 / 564 | 952 / 952 |
| band / correct / total_groups = n | t6 | 6 | MultiColouring 6 | All 6 | 6 / 6 | 570 / 570 | 958 / 958 |
| narrow / band one too narrow / total_groups < n | seq | 2 | MultiColouring 2 | MultiColouring 2 | 0 / 0 | 0 / 0 | 12 / 12 |
| narrow / band one too narrow / total_groups < n | t6 | 2 | MultiColouring 2 | MultiColouring 2 | 0 / 0 | 0 / 0 | 34 / 34 |
| narrow / correct / total_groups < n | seq | 3 | MultiColouring 3 | MultiColouring 3 | 3 / 3 | 101 / 101 | 144 / 144 |
| narrow / correct / total_groups < n | t6 | 3 | MultiColouring 3 | MultiColouring 3 | 3 / 3 | 104 / 104 | 147 / 147 |
| narrow / omission / total_groups < n | seq | 32 | MultiColouring 32 | MultiColouring 32 | 0 / 0 | 0 / 0 | 344 / 344 |
| narrow / omission / total_groups < n | t6 | 32 | MultiColouring 32 | MultiColouring 32 | 0 / 0 | 0 / 0 | 468 / 468 |
| random / correct / total_groups < n | seq | 8 | MultiColouring 8 | MultiColouring 8 | 8 / 8 | 800 / 800 | 1026 / 1026 |
| random / correct / total_groups < n | t6 | 8 | MultiColouring 8 | MultiColouring 8 | 8 / 8 | 808 / 808 | 1034 / 1034 |
| random / correct / total_groups = n | seq | 2 | MultiColouring 2 | All 2 | 2 / 2 | 328 / 328 | 589 / 589 |
| random / correct / total_groups = n | t6 | 2 | MultiColouring 2 | All 2 | 2 / 2 | 330 / 330 | 591 / 591 |
| random / correct / total_groups > n | seq | 14 | MultiColouring 14 | All 14 | 14 / 14 | 2354 / 1608 | 2945 / 2199 |
| random / correct / total_groups > n | t6 | 14 | MultiColouring 14 | All 14 | 14 / 14 | 2368 / 1622 | 2959 / 2213 |
| random / correct superset / total_groups < n | seq | 8 | MultiColouring 8 | MultiColouring 8 | 8 / 8 | 800 / 800 | 1204 / 1204 |
| random / correct superset / total_groups < n | t6 | 8 | MultiColouring 8 | MultiColouring 8 | 8 / 8 | 808 / 808 | 1212 / 1212 |
| random / correct superset / total_groups = n | seq | 16 | MultiColouring 16 | All 16 | 16 / 16 | 1936 / 1936 | 3420 / 3420 |
| random / correct superset / total_groups = n | t6 | 16 | MultiColouring 16 | All 16 | 16 / 16 | 1952 / 1952 | 3436 / 3436 |
| random / omission / total_groups < n | seq | 128 | MultiColouring 128 | MultiColouring 128 | 0 / 0 | 0 / 0 | 2668 / 2668 |
| random / omission / total_groups < n | t6 | 128 | MultiColouring 128 | MultiColouring 128 | 0 / 0 | 0 / 0 | 4185 / 4185 |
| random / omission / total_groups = n | seq | 32 | MultiColouring 32 | All 32 | 0 / 0 | 0 / 0 | 2219 / 2184 |
| random / omission / total_groups = n | t6 | 32 | MultiColouring 32 | All 32 | 0 / 0 | 0 / 0 | 4252 / 4217 |
| random / omission / total_groups > n | seq | 224 | MultiColouring 224 | All 224 | 0 / 0 | 0 / 0 | 6823 / 7782 |
| random / omission / total_groups > n | t6 | 224 | MultiColouring 224 | All 224 | 0 / 0 | 0 / 0 | 10924 / 11883 |
| ring / omission / total_groups = n | seq | 306 | MultiColouring 306 | All 306 | 0 / 0 | 0 / 0 | 1730 / 1730 |
| ring / omission / total_groups = n | t6 | 306 | MultiColouring 306 | All 306 | 0 / 0 | 0 / 0 | 2275 / 2275 |

## 7. 조사(survey): 무작위 띠 선언 패턴의 total_groups

패턴 378개(차수 60, 96, 128, 200, 300, 500; 반폭 0.05-0.45 n; 밀도 0.05, 0.1, 0.3, 1.0(띠 전체); 시드 1, 2). 우변 평가 없음. total_groups >= n: 184개(> n 133, = n 51). 최악 비율 total_groups/n = 1.85(n 60, 띠 [27, 27], total_groups 111, Band { lower: 27, upper: 27, moduli: [55, 56] }). V29 검토 F4와 같은 설정(밀도 0.1, 반폭 0.2-0.45 n) 72개 중 > n은 60개. 기본 검사 우변 합계 A 70871, B 62792(n의 합 80892). A와 B의 기록은 기본 검사 필드를 빼면 같다(불일치 0).


| 반폭 비율 | 패턴 | 규칙 All | > n | = n | 기본 검사 우변 A | 기본 검사 우변 B | n 합계 |
|---|---|---|---|---|---|---|---|
| 0.05 | 42 | 0 | 0 | 0 | 1981 | 1981 | 8988 |
| 0.1 | 42 | 0 | 0 | 0 | 3736 | 3736 | 8988 |
| 0.15 | 42 | 0 | 0 | 0 | 5500 | 5500 | 8988 |
| 0.2 | 42 | 0 | 0 | 0 | 7322 | 7322 | 8988 |
| 0.25 | 42 | 33 | 27 | 6 | 8645 | 8565 | 8988 |
| 0.3 | 42 | 34 | 25 | 9 | 9749 | 8815 | 8988 |
| 0.35 | 42 | 38 | 27 | 11 | 10566 | 8922 | 8988 |
| 0.4 | 42 | 39 | 27 | 12 | 11293 | 8970 | 8988 |
| 0.45 | 42 | 40 | 27 | 13 | 12079 | 8981 | 8988 |

띠 전체(밀도 1.0) 패턴: n 60 반폭 3: 24 (MultiColouring); n 60 반폭 6: 27 (MultiColouring); n 60 반폭 9: 39 (MultiColouring); n 60 반폭 12: 51 (MultiColouring); n 60 반폭 15: 60 (All); n 60 반폭 18: 60 (All); n 60 반폭 21: 60 (All); n 60 반폭 24: 60 (All); n 60 반폭 27: 60 (All); n 96 반폭 5: 23 (MultiColouring); n 96 반폭 10: 43 (MultiColouring); n 96 반폭 14: 59 (MultiColouring); n 96 반폭 19: 79 (MultiColouring); n 96 반폭 24: 96 (All); n 96 반폭 29: 96 (All); n 96 반폭 34: 96 (All); n 96 반폭 38: 96 (All); n 96 반폭 43: 96 (All); n 128 반폭 6: 27 (MultiColouring); n 128 반폭 13: 55 (MultiColouring); n 128 반폭 19: 79 (MultiColouring); n 128 반폭 26: 107 (MultiColouring); n 128 반폭 32: 128 (All); n 128 반폭 38: 128 (All); n 128 반폭 45: 128 (All); n 128 반폭 51: 128 (All); n 128 반폭 58: 128 (All); n 200 반폭 10: 43 (MultiColouring); n 200 반폭 20: 83 (MultiColouring); n 200 반폭 30: 123 (MultiColouring); n 200 반폭 40: 163 (MultiColouring); n 200 반폭 50: 200 (All); n 200 반폭 60: 200 (All); n 200 반폭 70: 200 (All); n 200 반폭 80: 200 (All); n 200 반폭 90: 200 (All); n 300 반폭 15: 63 (MultiColouring); n 300 반폭 30: 123 (MultiColouring); n 300 반폭 45: 183 (MultiColouring); n 300 반폭 60: 243 (MultiColouring); n 300 반폭 75: 300 (All); n 300 반폭 90: 300 (All); n 300 반폭 105: 300 (All); n 300 반폭 120: 300 (All); n 300 반폭 135: 300 (All); n 500 반폭 25: 103 (MultiColouring); n 500 반폭 50: 203 (MultiColouring); n 500 반폭 75: 303 (MultiColouring); n 500 반폭 100: 403 (MultiColouring); n 500 반폭 125: 500 (All); n 500 반폭 150: 500 (All); n 500 반폭 175: 500 (All); n 500 반폭 200: 500 (All); n 500 반폭 225: 500 (All).

## 8. 테스트

`run.sh unit`(HEAD `cc875d68`, 추적 변경 0): 바이너리 4개, 통과 11, 실패 0, 종료 코드 [0].
- `the_limits_stated_for_the_default_are_real`: ok
- `the_default_first_use_check_follows_the_route`: ok
- `the_default_check_runs_once_at_the_first_linearization_on_every_route`: ok
- `a_run_with_the_default_check_keeps_every_bit`: ok
- `the_default_decides_as_the_multi_colouring_check_on_part_of_the_v26_corpus`: ok
- `the_default_check_allocates_nothing_at_the_first_linearization`: ok
- `every_check_stays_selectable_on_a_wide_pattern`: ok
- `a_client_whose_default_became_the_full_check_runs_it_once`: ok
- `the_full_default_detects_every_omission_of_a_wide_pattern`: ok
- `the_default_of_the_coloured_route_follows_total_groups`: ok
- `a_full_default_check_allocates_nothing_at_the_first_linearization`: ok

할당(순차 클라이언트, 선형화 3회의 할당 수):

| 경우 | 기본 검사 | total_groups / n | 기본 검사 | 검사 없음 | 다중 채색 요청 |
|---|---|---|---|---|---|
| wide random band (total_groups 111 > 60), dense LU | All | 111 / 60 | [0, 0, 0] | [0, 0, 0] | [12, 0, 0] |
| ring 30 (total_groups 30 = n), dense LU | All | 30 / 30 | [0, 0, 0] | [0, 0, 0] | [12, 0, 0] |
| band (15, 15) of order 60 (total_groups 60 = n), tight kernel | All | 60 / 60 | [0, 0, 0] | [0, 0, 0] | [14, 0, 0] |
| band (15, 15) of order 60 (total_groups 60 = n), protected kernel | All | 60 / 60 | [0, 0, 0] | [0, 0, 0] | [14, 0, 0] |
| control: ring 34 (total_groups 33 < n), dense LU | MultiColouring | 33 / 34 | [0, 0, 0] | [0, 0, 0] | [0, 0, 0] |

- full_check_default: 252, full_check_default_found_by_the_check_not_the_build: 102, omissions: 252, test: the_full_default_detects_every_omission_of_a_wide_pattern
- random_patterns_full_check: 97, random_patterns_multi_colouring: 71, test: the_default_of_the_coloured_route_follows_total_groups

기존 테스트에서 고친 줄(채색 경로 기본값을 고정하는 줄만):
- `crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs` 새 줄 153-153(옛 줄 153-153): 채색 경로 기본값의 고정(pin). 이 루프는 AssemblyOptions::coloured(ring_pattern(16))에 MultiColouring을 단언했다. ring_pattern(16)은 total_groups 16 = n이므로 OD-L1-27 규칙은 All을 고른다(새 테스트 stage_clients_assembly_wide_band.rs가 고정). total_groups 33 < 34(경계 n - 1)인 ring_pattern(34)로 바꿔, 루프가 고정하는 다중 채색 기본값을 띠가 아닌 패턴으로 그대로 검사한다
- `crates/rodas5p-integrators/tests/stage_clients_assembly_default_alloc.rs` 새 줄 114-122(옛 줄 114-114): 채색 경로 기본값의 고정(pin). 모든 경우에 MultiColouring을 기대했는데, 그중 ring_pattern(30)은 total_groups 30 = n이라 기본값이 All이 된다. 이 경우는 All을, 세 띠 경우는 MultiColouring을 기대하도록 바꿨다(한 문장, rustfmt가 9줄로 나눔). 할당 단언은 그대로이며 전체 검사에도 성립한다
- 이유 없는 수정: 0; 허용 경로 밖 변경: 0.

검토했지만 고치지 않은 줄:
- `crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs` 309-317: expected check counts of the coloured route are total_groups + repeated base; its only route whose default becomes All, ring_pattern(30), has total_groups 30 = n, so All spends the same count (n, and one more with threads) and verified_columns stays n
- `crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs` 573-576: reference check MultiColouring for the coloured cases: the one case whose default becomes All (ring 30, true pattern) passes with 30 columns under both checks, so the asserted equality of the decisions still holds
- `crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs` 1034-1060: default options in refused configurations and one accepted band (1, 1) of order 8 (total_groups 7 < 8); the refusals do not depend on the check

새 테스트 파일:
- `crates/rodas5p-integrators/tests/stage_clients_assembly_wide_band.rs`: the_default_of_the_coloured_route_follows_total_groups, a_client_whose_default_became_the_full_check_runs_it_once, the_full_default_detects_every_omission_of_a_wide_pattern, every_check_stays_selectable_on_a_wide_pattern
- `crates/rodas5p-integrators/tests/stage_clients_assembly_wide_band_alloc.rs`: a_full_default_check_allocates_nothing_at_the_first_linearization
- `crates/rodas5p-integrators/tests/stage_clients_assembly_wide_band_record.rs`: v31_record (ignored: v31_record)

## 9. 검사

`run.sh checks`(HEAD `cc875d68`, 추적 변경 0):
- stage client tests and sentinels: 종료 코드 0, 바이너리 34, 통과 157, 실패 0, 무시 15. 마지막 줄: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- clippy: 종료 코드 0. 마지막 줄: `Finished 'release' profile [optimized] target(s) in 19.80s`
- fmt: 종료 코드 0. 출력 없음
- loop checker: 종료 코드 1. 마지막 줄: `ValueError: implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly.rs`
- rustdoc(`run.sh doc`, 종료 코드 [0]): 경고 23개, 조립(assembly) 파일의 경고 0개(나머지는 다른 파일의 기존 경고: crates/rodas5p-integrators/src/actual_k_homotopy.rs 1, crates/rodas5p-integrators/src/collective_model.rs 1, crates/rodas5p-integrators/src/fixed_point_continuation.rs 1, crates/rodas5p-integrators/src/n01_mass_action_client.rs 1, crates/rodas5p-integrators/src/nonlinear_homotopy.rs 1, crates/rodas5p-integrators/src/output.rs 2, crates/rodas5p-integrators/src/polynomial_resolvent.rs 3, crates/rodas5p-integrators/src/rodas5p_fast.rs 4, crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs 3, crates/rodas5p-integrators/src/stage_target.rs 1, crates/rodas5p-integrators/src/stage_target_policy.rs 1).
- 루프 검사기의 모든 불만(checker_all.py): ['implementation changed; rerun affected tests/update checkpoint: crates/rodas5p-integrators/src/stage_clients/assembly.rs']. 이 노드가 바꿀 수 있는 고정 파일만: True.

## 10. 한계와 열린 문제

- 규칙은 비용 규칙이다. `All`은 반올림 범위에서 정확하지만, 다중 채색 검사가 여러 열을 함께 움직여 우연히 비트를 바꾸는 반올림 한계 경우를 `All`이 놓칠 가능성은 일반적으로 배제되지 않는다. 이 노드의 말뭉치와 보충 집합에서는 그런 경우가 없었다(A가 찾고 B가 놓친 경우 0).
- V26 말뭉치에서 규칙이 바꾸는 경우는 올바른 링 패턴 6개뿐이고 모두 같음(total_groups = n)이라 비용이 같다. 비용 절감은 total_groups > n인 패턴(조사와 보충 집합의 무작위 띠)에서만 생긴다. 조사의 띠 전체 패턴 54개 중 total_groups > n인 것: 0개, = n인 것: 30개.
- 실패하는(잘못된) 선언에서는 검사가 첫 실패에서 멈추므로, 전체 검사가 실패를 만나기 전까지 쓰는 우변이 다중 채색 검사보다 많을 수 있다(결정은 같다). 보충 집합에서 B의 첫 사용 경로 우변 합계가 A보다 큰 분류: random / omission / total_groups > n seq: A 6823, B 7782; random / omission / total_groups > n t6: A 10924, B 11883.
- 스레드 실행에서도 기본값은 호출 스레드의 `All`이다(V29가 `MultiColouring`을 호출 스레드에서 돌린 것과 같은 선택). `AllPooled`는 계속 선택할 수 있다.
- 계약은 독립 검토를 요구한다(completion_requires).

