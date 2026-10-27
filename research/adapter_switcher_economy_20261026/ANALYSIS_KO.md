# Adapter/switcher 병목 — 감사 패킷 비판적 독해, 적대적 분석, 수정과 측정 (2026-10-26)

대상: `external_reports/adapter_local_followup_20261025/` (브랜치 `research/adapter-local-followup-20261025`, head `4fe67602`, 측정 소스 `d914b4c8`). 작업 브랜치 `research/adapter-switcher-perf-20261026`, 실행 commit `636cdc7a`. 사전등록·게이트·증거는 같은 디렉터리의 `PREREGISTRATION.md`와 `evidence/`에 있다. 모든 벽시계 시간은 `diagnostic_only`이며 판정은 결정론적 `WorkCounters`로만 한다.

## 1. 요약

패킷은 선택 경로(switching adapter)가 같은 오차 상한의 직접 대안보다 rotation 8.0배, gyromotion 3.1배, case5 14.6배, stiff-forced 2.0배 느리다고 측정하고 원인을 "반복 pilot과 명시법을 먼저 택한 비용"으로 적은 뒤 N05(선택 정책)를 HOLD, N03(RODAS 세션 유지)을 부분 완료로 남겼다. 이 분석은 그 판정을 재현한 뒤 비용을 disposition별로 분해해, 벽시계 격차의 대부분이 **RHS 횟수가 아니라 버려진 trial의 attempt당 단가**(matrix-free RODAS pilot 1회 ≈ DOPRI attempt의 30배)와 **창마다 h0에서 다시 시작하는 RODAS tenure**에서 나온다는 것을 보였다. 등록된 임계값·허용오차·hysteresis를 하나도 바꾸지 않고 (1) 결정을 보존하는 work ceiling, (2) 기하급수 back-off + 추세 re-arm, (3) recheck 사이 창을 하나의 native run으로 합치는 implicit tenure와 평균 스텝 seed, (4) 배치 루프의 할당 정리를 넣었다. 결과(결정론적 카운트, 같은 호스트·같은 binary 프로토콜):

| 셀 | attempts 전→후 | RHS 전→후 | 버린 pilot attempts | 버린 recheck attempts | RODAS batch 수 | 오차 (cap) |
|---|---|---|---|---|---|---|
| rotation `evidence` | 1046→892 | 6597→5371 | 169→15 | 0 | 0 | 9.3e-7 (1e-5) |
| stiff-forced `evidence` | 137→66 | 1002→456 | 8→8 (통과 pilot) | 16→5 | 8→3 | 7.3e-10 (1e-5) |
| case5 suite `switching` | 232→115 | 1633→748 | 23→6 | 48→19 | 16→4 | 1.3e-8 (1e-4) |
| gyromotion tight (rtol 1e-9) | 4271→4159 | 26018→25102 | 128→16 | 0 | 0 | 3.9e-5 (1e-4) |
| rotation `pressure-only` (legacy 정책) | 701→666 | 5430→5144 | 0 | 16→16 | 8→3 | 8.4e-8 (1e-5) |

직접 `Dopri5::solve`/`Rodas5p::solve` 9개 셀은 카운터와 최종 상태가 비트 단위로 동일하다(G1). 진단용 벽시계 비율은 rotation 9.0배→2.2배, case5(대 dense RODAS) 17.5배→4.6배, stiff-forced 2.3배→1.5배, gyromotion 3.1배→2.2배였으나, 등록된 drift 규칙(before 캠페인 두 번 사이 직접 셀 1.5배 초과: `bench:rotation:dopri` 0.225→0.412 ms)에 걸려 **시간표는 신뢰 불가로 표기**한다. 카운터 게이트와 무관하다.

## 2. 패킷의 비판적 독해

맞는 것. (a) 다섯 cohort의 선택/직접 비율과 "같은 오차 상한 내 후보만 비교"하는 원칙. (b) case5/KS의 DOPRI h가 음의 실축 선형 안정 한계의 0.9997배라는 계측(안정성 제한). (c) 고정 임계값을 낮춰 늦은 강성 전이를 다시 놓치게 하는 변경을 거부한 결정. (d) chirp 이득이 선언된 chart에 한정된다는 점.

과소·미분해. (1) 패킷은 RHS 횟수와 벽시계를 함께 적었지만 **disposition별로 나누지 않았다**. rotation은 RHS가 1.26배인데 시간은 9배다: 6개의 버린 MF-RODAS pilot이 전체 attempt의 16%(169/1046)이면서 시간의 약 85%를 차지한다(attempt당 ≈10 µs 대 DOPRI 0.3 µs). "반복 pilot 비용"이라는 문장은 맞지만, 그 크기가 격차의 거의 전부라는 사실은 보고서에서 읽을 수 없다. (2) case5의 14.6배는 N03("RODAS tenure 재구성")이 주원인이다. 16개 창마다 `Rodas5p::solve`를 h0=1e-4(suite)에서 다시 시작해 97 attempts를 쓰는데 직접 실행은 39이고, 4창마다 explicit recheck 16 attempts(총 48)를 버린다. 패킷은 이를 "남은 작업"으로만 적었다. (3) gyromotion의 4개 pilot(128 attempts)은 모두 `implicit-pilot-incomplete`, 즉 예산(32)을 다 쓰고도 비교 prefix 끝에 못 간 경우다: 비교 상대보다 9배 비싼 pilot을 끝까지 돌리는 것은 정책 결함이지 정보가 아니다. (4) 패킷의 직접 대안 선택: case5의 직접 대안은 `rodas5p-dense`(0.137 ms)이고 선택 경로는 JVP(matrix-free) 경로를 탄다. 직접 `rodas5p-mf`는 0.635 ms이므로 "14.6배"의 상당 부분은 선택 정책이 아니라 **경로(MF 대 dense)** 차이다. 패킷은 이를 구분하지 않았다. (5) 패킷의 gyro cohort(26018 RHS)는 rtol 1e-9 설정이었고 suite 기본 tolerance에서는 모든 방법이 상한(1e-4)을 넘는다(0.031). 이번 측정은 두 설정을 모두 돌리고 기본 tolerance 셀은 보고만 한다.

과대. "persistent explicit switching: SCOPED_CONTRACT_PASS"는 맞지만, carry가 켜진 `evidence` 설정에서도 pilot이 explicit 작업량의 6~9배를 매번 소모했으므로 "지속 세션"의 효과(screened-reset 대비 1274→1046 attempts)는 pilot 낭비에 묻혀 있었다.

## 3. 병목 순위 (측정 근거)

1. **버린 implicit pilot의 attempt 단가와 반복** — rotation 169, gyro 128 attempts; MF-RODAS attempt당 ≈8 RHS + ≈32 JVP + 8 linear solve. 원인: pressure 판정(`attempts ≥ 24` 또는 batch 예산 소진)은 많은 스텝이 필요한 모든 문제에서 참이고, pilot은 자신이 이미 비교 상한을 넘겼음을 모른 채 예산 끝까지 돈다. 수정: entry ceiling(정확; 통과 가능성이 사라지는 순간 정지) + 두 번째 discard부터 cooldown 배증 + 작업률 2배 점프 시 즉시 re-arm.
2. **RODAS tenure의 창별 재시작** — case5 97 대 39, stiff-forced 81 대 14. 원인: 창마다 facade를 다시 만들고 h0에서 ramp. 수정: 다음 recheck까지의 창을 한 native run으로(각 창 끝에 landing), 다음 run은 이전 run의 평균 수락 스텝으로 시작. 결과 26(4 batch)/21(3 batch).
3. **지속적으로 강성인 문제의 explicit recheck** — case5 48 attempts(3회). 수정: return ceiling(정확) + 두 번째 discard부터 간격 배증(최대 2회) + implicit 작업률 하락 시 re-arm. 결과 19(2회).
4. **explicit 배치 루프의 할당** — 배치마다 `ExplicitStats` 3회 복제, `Options` 복제, pilot용 상태 복제, `String` 결정 문자열. 수정: 복사 가능한 session snapshot에서 기록을 한 번만 구성, tenure당 요청 복사 1회, `&'static str` 결정. 계약: 배치당 12회 이하(`explicit_batch_loop_allocation_is_bounded`).

남는 것(주장하지 않음): 명시법을 먼저 시도하는 등록된 비용(case5 explicit 2 batch 64 attempts), 창 landing 비용(창당 1 attempt), oscillatory 문제의 batch 경계 비용(rotation 892 대 875), MF 대 dense 경로 차이. 큰 격자(N06)와 stage/data 병렬은 이 노드의 범위 밖이다.

## 4. 수정이 지키는 것

- 결정 보존: pilot은 `work_units ≤ entry_cost_ratio × W_explicit`일 때만 통과한다. work_units ≥ 청구된 callback 단위이므로 callback 단위가 상한을 넘은 pilot은 어떤 경우에도 통과할 수 없다. recheck도 `rate ≤ return_cost_ratio × implicit_rate`에서 RHS 단위로 같은 논리다. 계약 `discarded_pilot_work_is_bounded_by_the_entry_ceiling`, `economical_pilot_is_never_stopped_by_the_ceiling`.
- 늦은 전이: `discarded_pilots_back_off_geometrically_and_rearm_on_a_work_rate_jump`는 ω=400 강제항 뒤 t=0.6에서 λ=1e6으로 바뀌는 문제에서 cooldown이 16 batch까지 자란 뒤에도 전이 후 6e-4 안에 RODAS에 진입함을 고정한다. 반대 방향(`return_to_explicit_still_happens_after_a_stiff_to_smooth_transition`)도 고정한다.
- 거래 원자성: 버린 trial은 수락 상태·시간·스텝 제안·세션을 바꾸지 않는다(기존 36개 switching/session 계약 전부 통과, 변경 없음).
- 직접 경로 비트 동일(G1), 모든 attempt/RHS/JVP/solve 청구 보존(각 셀 reconciliation, 6회 실행 결정론).

## 5. 재현

`tools/run_campaign.sh BIN RAW`, `tools/analyze.py RAW OUT.json BIN COMMIT`, `tools/compare.py BEFORE AFTER OUT.md GATES.json`. before = 패킷 binary(`BINARY_SHA256_FINAL.json`과 동일 해시), after = `636cdc7a` measurement 빌드, before-2 = 패킷 binary 재실행(drift 대조). 원자료 648개 파일의 정렬 해시 요약은 `evidence/HOST.json`에 있고 저장소 밖(세션 scratch)에 남겼다. NCP·네트워크 미사용.

## 6. 적대적 리뷰(독립 에이전트 3개 lens)와 수정 루프 (2026-10-27)

리뷰는 24건을 냈고 그중 3건만 독립 검증까지 끝났다(세션 한도). 나머지 21건은 코드로 직접 재검토했다. 확정·수정(단일 수정 루프, 임계값·게이트 변경 없음):

- **MAJOR (확정)** `Jacobian::Dense` + 선언된 band 경로에서 entry ceiling이 route 검증(band fill + dense 참조, 2n 단위) 안에서 걸리면 `StructureRefused`로 올라와 pilot이 아니라 **adapter 전체 solve가 중단**되고 accounting이 깨진다. 수정: ceiling은 driver 작업만 제한하도록 검증 비용(2n) 위에서 arm. 결정 보존은 유지된다(상한 ≥ cap이면 통과 pilot은 절대 정지되지 않음). 회귀 계약 `bounded_pilot_on_a_declared_band_is_discarded_not_refused`.
- **MINOR (확정)** 합쳐진 implicit batch의 창 endpoint가 시계 분해능 아래에서 같아지면 facade가 run을 거부. 수정: landing을 strictly increasing으로 dedupe.
- **MINOR (확정)** `Recoverable` 오류를 낸 callback은 driver가 세지만 ceiling에 청구되지 않았다. 수정: 청구(여전히 ≤ work_units, 결정 보존).
- **MINOR (확정)** return ceiling의 곱(`ratio·rate·span`)과 return 판정의 나눗셈(`work/span ≤ ratio·rate`) 사이 반올림 비대칭. 수정: ceiling에 1 단위 slack.
- **MINOR (확정)** recheck hysteresis 기준이 "마지막 창의 rate"에서 "합쳐진 batch 평균 rate"로 바뀌었다 — 문서화(아래 amendment). recheck re-arm은 "즉시" 발동이 아니라 **back-off를 해제**하는 것이며 실제 recheck는 진행 중인 implicit run이 끝나는 등록된 지점에서 일어난다 — 문서 수정.
- **MINOR (확정)** 테스트 공백: 늦은 전이 테스트에 re-arm 비활성 음성 대조(진입 지연 > 1e-3) 추가; recheck ceiling이 실제로 발동했는지 단언; 전이 후 복귀 상한을 span 안으로(3.5 < 4.0) 조임; 할당 상한을 측정값으로(새 376 대 기존 568 / 64 batch) 6/batch로 조여 기존 루프를 거부하게 함.
- **MINOR (확정)** 비교기 `compare.py`가 측정 후 수정됨(등록된 report-only 셀 규칙 구현). EXEC_SHA 시점 도구로는 G3가 그 셀에서 False. 결과 섹션에 공개.
- **MINOR (확정)** 커밋된 분석 JSON은 `segments` 행을 제거한 압축본(원본은 archive에 SHA와 함께 보존). 공개.
- **반박/범위 밖**: "carry=false recheck도 session 경로"(의도된 변경, 공개); "pilot 예산 16으로는 창당 16 attempt 초과 문제는 복귀 불가"(기존 설계); "ceiling의 verdict-identity 대조 실험 없음"(수학적 논증만, 명시); "implicit 셀 오차 증가"(1.6e-10→7.3e-10, 9.9e-10→1.3e-8: seed된 큰 스텝의 결과, cap 대비 4자리 여유).
- 새 테스트 작성 중 발견한 **기존** 한계: `carry_explicit_step=false`에서 남은 span이 driver 최소 스텝(1e-14) 아래면 `local.h0.min(remaining)`이 거부되어 solve가 실패로 끝난다. 이번 범위 밖, 후속 과제.
