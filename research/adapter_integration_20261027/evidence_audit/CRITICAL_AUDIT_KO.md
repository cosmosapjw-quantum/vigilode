# 공개된 최신 결과의 독립 감사 — 2026-10-27

이 문서는 이번 통합 수정의 **입력 증거**를 감사한다. 후속 통합 코드의 실행 결과와 구분하며, 기존의 성공한 실험을 다시 실행하지 않았다. GitHub 브랜치 215개와 최근 갱신 PR 20개를 열람하고, 아래 고정 소스의 코드·보고서·전체 segment 분석을 검토했다. 원자료 864개 파일과 실행 바이너리는 사용자 로컬 archive에 있어 이 작업에서 직접 재실행하거나 바이트별 재검증하지 않았다.

## 1. 어느 코드에서 이어갈 것인가

최신 고유 solver 개발은 `research/adapter-switcher-perf-20261026`, 공개 tip `e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824`이다. 이 tip의 tree는 `41f6894f726ae8a2e375b919dfad0a78e79b08ca`이며 실제 최신 측정 소스는 `596dcdfae8b95d24f27b57975d9f4626267b8149`이다.

공동 제어 연구 `research/joint-controller-20261025`의 tip `9b062e812b821a20dfcb40dfa0340a9df41e835e`와는 단순 선후관계가 아니다. 공통 조상 `4fe67602502440d76aa9dd245c6b1d67558c14bf`에서 각각 갈라졌으며 native 쪽이 5 commits 앞, 2 commits 뒤다. 따라서 최신 native를 기준으로 공동 제어의 동시간 구간 비교·적응형 탐색·실행 식별 계약을 선택적으로 통합해야 한다. 오래된 joint 소스로 덮어쓰면 이미 측정된 개선을 잃는다.

별도 응용 연구 `research/program-expansion-20261025`의 tip은 `72836e0290b66d33f4ca1771284b1c8070669a74`이다. BASS/REI 결과를 범용 ODE solver 검증으로 대신하지 않는다. 전체 브랜치와 PR snapshot은 [SOURCE_INVENTORY.json](SOURCE_INVENTORY.json)에 있다. 최신 native 변경은 새 PR 없이 branch commit으로 공개되었으므로 PR 목록만 보면 놓친다.

## 2. 실제로 이미 개선된 것

다음 값은 최신 공개 비교표의 결정론적 작업량이다. `before` 소스는 공통 조상, `after` 소스는 `596dcdfa`이다.

| 실험 | 총 attempts | RHS 호출 | 추가 분해 |
|---|---:|---:|---|
| rotation evidence | 1046 → 892 | 6597 → 5371 | 미채택 implicit pilot attempts 169 → 15 |
| stiff-forced evidence | 137 → 66 | 1002 → 456 | 생산적 implicit batches 8 → 3, attempts 81 → 21 |
| case5 switching | 232 → 115 | 1633 → 748 | implicit batches 16 → 4, attempts 97 → 26 |
| gyromotion, rtol=1e-9 | 4271 → 4159 | 26018 → 25102 | 미채택 implicit pilot attempts 128 → 16 |
| rotation pressure-only | 701 → 666 | 5430 → 5144 | 비싼 implicit 경로 선택 자체는 남음 |

적용된 변화는 (i) 통과 가능성이 없어진 trial의 callback 상한, (ii) 실패 탐색의 간격 증가와 작업률 변화에 따른 재탐색, (iii) 다음 recheck까지 implicit 창을 하나의 native run으로 합침, (iv) 이후 run의 평균 수락 스텝 seed, (v) 배치별 clone·문자열 할당 감소다. 64 explicit batches의 측정 할당 횟수는 568 → 376이며 최종 계약 상한은 `6 × batches + 32`이다. 이 작업을 새로운 연구 결과처럼 중복 개발할 필요가 없다.

고정 증거: [최신 비교표](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/research/adapter_switcher_economy_20261026/evidence/COMPARISON.md), [실제 측정 소스](https://github.com/cosmosapjw-quantum/vigilode/blob/596dcdfae8b95d24f27b57975d9f4626267b8149/crates/vigilode/src/switching.rs).

## 3. 기존 해석을 바로잡아야 하는 부분

### 시간표는 방향 탐색에만 사용할 수 있다

등록된 drift 대조에서 같은 baseline direct rotation의 시간이 0.225 → 0.412 ms, 약 1.83배 변했다. 보고서도 이를 `diagnostic_only`로 처리한다. 결정론적 작업량 감소는 유효하지만 벽시계 가속률을 인증된 성능 향상으로 제시할 수 없다.

더구나 `ANALYSIS_KO.md` 1·5절에는 run 1의 소스와 일부 시간값이 남아 있다. 최신 판독에는 run 2 비교표를 우선해야 한다. 최신 진단값은 rotation evidence 0.863 ms / direct DOPRI 0.223 ms = 3.86배, case5 0.820 ms / direct dense RODAS 0.122 ms = 6.73배다. 오래된 본문의 2.2배·4.6배를 최신 결과라고 인용하면 안 된다.

고정 증거: [호스트·drift 기록](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/research/adapter_switcher_economy_20261026/evidence/HOST.json), [기존 해석 문서](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/research/adapter_switcher_economy_20261026/ANALYSIS_KO.md).

### case5에는 selector 비용과 선형대수 경로 비용이 섞여 있다

case5 switching은 matrix-free JVP 경로이고 비교 최저값은 dense 경로다. 최신 direct MF는 0.594 ms, direct dense는 0.122 ms이며 두 경로의 RHS는 모두 307이다. switching의 6.73배 격차 전부를 adapter overhead로 읽을 수 없다. 같은 backend 대비 비용과, backend 선택 자체의 비용을 나눠야 한다. 작은 차원에서 dense가 유리할 수 있지만 이것만으로 미분 없는 기본 철학을 폐기하거나 큰 차원까지 dense를 채택할 근거는 없다.

### 성공한 advisory pilot도 “discarded”로 묶인다

`analyze.py::seg_rows`는 RODAS segment가 `committed=false`이면 모두 `implicit_pilot_discarded`로 분류한다. 그러나 진입 여부를 결정하는 데 성공한 advisory pilot도 explicit prefix를 유지하므로 미채택 상태다. stiff-forced 및 case5에서 성공 pilot과 실제 실패 pilot이 섞인다. rotation에서는 성공 진입이 없어 169 → 15라는 핵심 감소값은 맞다. 후속 분석은 성공한 advisory pilot, 상한 초과, 불완전 trial, 비경제적인 완료 trial을 별도로 집계해야 한다.

고정 증거: [분석 코드](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/research/adapter_switcher_economy_20261026/tools/analyze.py).

### “모두 정확도 통과”로 요약하면 안 된다

기본 허용오차 gyromotion의 endpoint error는 약 0.0309로 외부 오차 상한 1e-4를 넘는다. 이 셀은 `accuracy_report_only_cells`에 들어가므로 집계 `G3=true`는 이 셀의 정확도 통과를 의미하지 않는다. tight 설정(rtol=1e-9)은 약 3.89e-5로 상한을 만족했다. 이는 tolerance·위상 오차 관찰을 포함한 실제 정확도 관리가 아직 필요함을 보여준다.

또한 compare.py는 direct의 전체 WorkCounters가 아니라 8개 요약 필드를 비교한다. 실제 기록에는 18셀과 direct 9셀이 존재하고 상태 bit 일치가 확인되지만, “모든 counter 일치”라는 설명보다 검사의 범위가 좁다. 교집합만 비교하고 direct 후보가 없으면 해당 switching 셀을 건너뛰므로 미래 불완전 입력의 누락도 통과로 오인할 수 있다.

고정 증거: [판정 JSON](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/research/adapter_switcher_economy_20261026/evidence/GATES.json), [비교 코드](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/research/adapter_switcher_economy_20261026/tools/compare.py).

### coalescing은 복귀 비교의 기준도 바꾼다

최신 native는 직전 implicit 단일 창 대신 합쳐진 여러 창의 평균 작업률을 복귀 시험의 기준으로 쓴다. 이전의 비싼 구간과 현재의 싼 구간을 혼합하면 현재 구간에서 더 비싼 explicit 후보를 채택할 수 있다. 임계 비율이 같아도 의사결정의 측정 대상은 달라졌다. 기존 문서 6절은 이를 인정하지만 “결정 불변”이라는 앞부분 표현은 그대로다. joint branch의 같은 체크포인트·같은 구간 비교는 이 문제에 직접 대응한다. 이미 계산한 더 나은 implicit 비교 결과를 생산적으로 채택하면 중복 계산을 줄일 수 있다.

### 일반 사용성 및 검토 범위

e857의 `AutomaticOde::solve`는 미분 허용 시 implicit provider를 선행 검증한다. 선택적 challenger가 없다는 이유로 RHS-only로 풀 수 있는 요청까지 거절될 수 있다. challenger capability 확인과 전체 문제 유효성 검사를 분리할 필요가 있다. `carry_explicit_step=false`에서 잔여 구간이 1e-14 미만일 때 초기 스텝 검증이 실패하는 기존 한계도 공개되어 있다.

기존 리뷰 24건 중 별도 verifier가 완료한 것은 3건이며 나머지 21건은 저자가 코드로 재검토했다. 전부 독립적으로 재검증되었다고 기술하면 안 된다.

고정 증거: [선택 facade](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/crates/vigilode/src/automatic.rs), [리뷰 기록](https://github.com/cosmosapjw-quantum/vigilode/blob/e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824/external_reports/adapter_switcher_economy_20261026/review/REVIEW_FINDINGS.json).

## 4. 후보 선택과 다음 실험

| 후보 | 판단 | 다음 실험 |
|---|---|---|
| callback 상한·backoff·implicit coalescing·할당 정리 | 유지 | 통합 후 기존 계약의 회귀 확인 |
| 동시간 구간 복귀 비교 | 통합·측정 | 과거 평균률이 오도하는 전이, 더 나은 implicit 상태 재사용, 전체 비용 청구 |
| 선택적 implicit provider 가용성 | 우선 수정 | RHS-only 허용, 실제 잘못된 요청은 여전히 거부 |
| 작은 차원 backend 선택 | 조건부 연구 | 동일 backend 기준과 최선 backend 기준 분리, 사용자의 미분 정책 준수 |
| 총 탐색 예산 | 연구 | 생산적 작업 대비 탐색 비율을 별도 추적, 늦은 전이 감지 유지 |
| 완전한 persistent implicit session | 잔여 병목 측정 후 | 이미 구현된 coalescing 대비 추가 이득 확인 |
| pressure-only 자동 기본값 | 제외 | rotation의 비싼 MF 경로 오선택 때문에 일반 기본값에 부적합 |
| 선언된 chart·quotient·구조화 회전 | 특수 adapter 유지 | 구조 가정 및 원상태 오차를 검증한 경우에만 선택 |
| causal/stage/data 병렬 | 특수·추가 연구 | 비싼 callback에서 calibration 포함 이익, 작은 문제는 직렬 |
| 기본 허용오차 gyro 정확도 | 미충족 | 궤적·위상 오차와 tolerance/구간 길이의 관계를 명시 |

## 5. 생산 사용 가능성

현재 공개 증거는 제한된 문제군에서 switching의 낭비 작업이 크게 줄었다는 결론을 지지한다. 그러나 **범용 production ODE solver로의 승격 근거는 아직 부족하다**. 정확도 실패 셀, 선택적 provider 처리, 바뀌는 구간 사이의 비교 타당성, backend 혼입, 검증된 벽시계 자료 부재가 남는다. 직접 solver와 명시적 조건을 만족하는 specialized adapter를 독립적으로 평가하면서, 이번 통합은 이 구체적 결함을 해결하는 방향으로 제한하여 검증하는 것이 타당하다.

