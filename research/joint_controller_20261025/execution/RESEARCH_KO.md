# 수치 상태와 실행 비용을 함께 구분하는 병렬 실행 선택

이번 변경은 실제 `AdaptiveBatchExecutor`에 적용된다. 동일한 실행 크기라도 좌표 변환, 격자, 물리 오차 노름, 허용오차 또는 callback 제공자가 바뀌면 이전 비용 관측을 재사용하지 않는다. 선택한 경로의 실제 호출 비용이 연속으로 기존 관측 범위를 벗어나면 다음 선택부터 관측을 만료한다. 새 관측을 얻기 위한 callback 재실행은 사용자가 명시한 replay-safe 경계에서만 이루어진다.

## 기존 결과에서 이어지는 문제

직전 코드는 세 쌍의 실제 시간에서 직렬 최소값과 병렬 최대값을 비교했다. 따라서 분산을 무시한 평균값 비교라는 진단은 맞지 않는다. `research/adapter_research_20261025/parallel/PILOT_SUMMARY_REPAIRED.json`의 배치 1/64/256 결과와 sunk-cost 수정은 유지한다. 새 로컬 후속 보고서 `external_reports/adapter_local_followup_20261025/REPORT_KO.md`는 큰 격자 callback의 독립 속도와 통합 solver의 출력·시간이 아직 연결되지 않았다고 기록한다. 이 자료를 실제 PDE solver의 병렬 가속 결과로 올려 읽지 않는다.

남아 있던 구체적인 연결 결함은 다음 두 가지다.

1. `WorkloadKey`에는 모델·방법·차원·배치·실행 계층·환경이 있었지만, chart/mesh/physical norm/tolerance/provider 및 비교 구간의 버전은 별도로 표현되지 않았다.
2. 세 쌍의 초기 관측이 128회 선택 동안 살아 있었으며, 그 사이 실제 callback 비용이 크게 변해도 선택기는 시간 변화를 관측하지 않았다.

## 공통 비교식

`selection_evidence::SelectionIdentity`는 원래 물리 문제, 제공자, 좌표 변환, 격자, 오차 노름, 허용오차, 물리 checkpoint/출력 구간, 실행 환경의 버전을 정확히 비교한다. `window_epoch`는 호출자가 관리하는 비교 구간의 버전이다. 문자열 해시나 같은 길이의 배열만으로 동일 문제를 추정하지 않는다. 모든 값이 0인 초기 버전도 정확한 하나의 버전이며 wildcard가 아니다.

같은 구간과 허용오차를 만족하는 incumbent와 candidate의 비용을 각각

\[
C_i\in[L_i,U_i],\qquad C_c\in[L_c,U_c]
\]

로 두고, 앞으로 필요한 전환 비용을 \(T\), 앞으로 비교 범위가 유지될 호출 수를 \(N\ge1\), 요구하는 상대 이득을 \(0\le\eta<1\)로 둔다. 코드는

\[
U_c+T/N\le(1-\eta)L_i
\]

인 경우에만 후보를 선택한다. 구간이 실제 미래 비용을 포함한다는 가정 아래

\[
NC_c+T\le NU_c+T\le(1-\eta)NL_i\le(1-\eta)NC_i
\]

가 성립한다. 이미 지불한 calibration 비용은 \(T\)에 다시 넣지 않는다. 그 비용은 전체 campaign 합계와 회수에 필요한 호출 수에 남는다.

이 정리는 비용 구간의 유효성을 전제로 한다. 세 번의 시간 측정에서 얻은 최솟값·최댓값은 `EmpiricalTiming`으로 보존하며 미래 비용의 수학적 포함 구간으로 승격하지 않는다. 같은 checkpoint의 실제 작업 점수는 `ObservedWork`이다. 사용자가 증명으로 공급하는 포함 구간은 `DeclaredBound`이며, 이 helper는 그 증명을 대신하지 않는다. `CostUnit`은 작업 점수와 초의 혼합을 차단한다. accuracy/output qualification은 비용 비교보다 먼저 충족되어야 한다.

계산식에는 보수적인 `next_up`/`next_down`을 적용한다. 큰 정수 \(N\)을 binary64로 바꿀 때 분모를 아래로 두고, 전환 비용 나눗셈·후보 합을 위로, incumbent 임계값을 아래로 둔다. 비유한 값, 음수, 역전된 구간, 0 호출 수, 단위/identity 불일치는 비교 거부로 반환된다. 경계의 정확한 동률은 보수적으로 incumbent를 유지할 수 있다.

## 실제 adapter 연결

새 `ExecutionIdentity`는 기존 `WorkloadKey`와 공통 `SelectionIdentity`를 묶는다. 기존 함수는 0 버전의 공통 identity를 사용해 동작하며, 세부 버전을 관리하는 호출자는 다음 API를 사용한다.

```rust
let identity = ExecutionIdentity { workload, comparison };
let mut executor = AdaptiveBatchExecutor::new(
    workers, ExecutionPolicyConfig::monitored()
)?;
executor.calibrate_replay_safe_for(identity, &jobs, operation, equivalent)?;
let (outputs, decision) = executor.map_ordered_for(
    identity, remaining_calls, outer_is_parallel, &jobs, operation
)?;
```

`begin_calibration_for`/`record_pair_for`/`decide_for`는 policy만 따로 사용할 때의 API다. 수치 계약이 한 번 바뀌면 이전 관측은 폐기된다. 뒤에 이전 epoch 번호로 돌아가도 폐기된 관측이 다시 살아나지 않는다. 외부 병렬 문맥 및 wrapper 내부의 중첩 호출은 직렬화하고, 바깥 workload의 관측이나 drift 연속 횟수를 바꾸지 않는다.

기본 실행 설정의 `drift=None`은 유지했다. 기존 호출에 새로운 시계 측정 비용을 강제하지 않는다. `monitored()`는 선택한 backend의 실제 dispatch만 측정한다. lazy pool 생성은 별도 `startup_seconds`에 기록한다. 각 dispatch가 해당 backend의 세 calibration 값의 최소/최대 구간을 factor 2 이상 벗어나는 경우를 세고, 두 번 연속이면 다음 결정부터 `ObservedCostDrift`와 `probe_due=true`를 반환한다. 구간 안의 관측은 연속 횟수를 0으로 돌린다. 이 과정은 완료된 물리 계산의 결과를 되돌리거나 callback을 재실행하지 않는다.

비용 변화가 발생한 첫 호출을 미리 알아내지는 않는다. 선택하지 않은 backend만 변하면 현재 경로 관측에서 그 변화가 보이지 않을 수 있으므로 기존 최대 관측 수명 128회를 유지한다. factor와 연속 횟수는 비용 변화 감지 설정이며 물리 오차 허용오차가 아니다. 오차 수용은 수치 적분기에서 그대로 수행한다.

## 검증 자료와 읽는 법

- `selection_evidence_contracts.rs`: 실제 비용 차이, 미래 전환 비용, 겹치는 구간, 서로 다른 단위·물리 구간·출력 qualification, 비유한/음수와 경계 반올림의 5개 계약 시험.
- `execution_identity_drift.rs`: 여덟 물리 epoch 각각의 폐기, 오래된 pair 거부, 연속 비용 변화, 정상 관측의 reset, 중첩 호출 격리, callback 정확히 한 번 실행 및 실제 시간 계측의 6개 계약 시험.
- 기존 `execution_policy_contracts.rs` 9개: 실제 ordered dispatch, nested 정책, no implicit replay, singleton no-pool/no-calibration 등은 그대로 유지한다.
- `joint_execution_pilot.rs`: 64개 실제 logistic ODE를 DOPRI로 풀고 세 단계에서 직렬·병렬·선택 wrapper를 동일 구간/출력 조건으로 비교한다. 각 단계는 순서를 회전한 5회 반복이며 모든 member의 해석적 endpoint 절대오차를 1e-7 미만으로 검사하고 출력의 bit 일치를 확인한다.

pilot의 첫 단계는 \([0,0.05]\)에서 calibration 후 평가한다. 둘째 단계는 \([0,32]\)로 문제를 바꾸면서 의도적으로 window epoch를 갱신하지 않는 비용 변화 stress case다. 이 단계는 caller 계약 위반이 검출되는 과정을 관찰하기 위한 것이며, 그 이전 calibration을 과학적으로 유효한 관측 재사용으로 취급하지 않는다. 셋째 단계는 실제 window/provider epoch를 갱신하고 명시적으로 재측정한다.

raw JSONL은 실제 backend 결정, 선택 이유, dispatch 후 관측 폐기, wrapper 전체 시간, 원래 상태 오차, 출력 parity를 남긴다. calibration 시간에는 warmup과 세 쌍의 측정이 이미 포함된다. pool startup은 cumulative이므로 단계별 증가량을 한 번만 더한다. 직접 병렬 comparator의 pool 시작 비용도 별도로 남긴다. 선택 wrapper와 고정 backend의 짧은 시간 비율은 해당 host의 진단이며, 관측 구간 수나 정확한 출력 계약이 다른 통계와 합치지 않는다. 테스트·실행의 실제 완료 상태와 source identity는 상위 `VALIDATION.json` 및 실행 영수증을 따른다.

## 실제 실행 결과

`native/RECEIPT.json`과 `native/stdout.log`에 기록된 실행은 exit 0, wall 0.0640초로 완료되었고 실행 중 과학 소스가 유지되었다. 원자료 SHA-256은 `2ec7603c571274ea3845e2c4197c90e6c0ffc773d55eab70547db4bfa98890b6`이다. 실행 당시 작업트리 HEAD는 가져온 기준 tree를 복원한 로컬 commit이므로, 실제 실험 소스는 함께 저장한 `SOURCE_INPUTS.json`으로 식별한다. 이 HEAD를 게시된 최종 소스로 대체해서 기록하지 않는다.

여섯 identity/drift 시험, 다섯 공통 비용 비교 시험, 기존 아홉 실행 정책 시험은 모두 통과했다. 해당 20개는 `../validation/facade_final_r2/`의 facade 실행에 포함되며 여러 반복 실행을 별개의 시험 수로 합산하지 않았다. 최종 전체 facade 재검사 범위는 상위 `VALIDATION.json`을 따른다.

| 단계 | 실제 선택 | adapter 총비용(ms) | 직접 직렬(ms) | 직접 병렬(ms) | adapter/최소 직접 비용 |
|---|---|---:|---:|---:|---:|
| 짧은 구간, 초기 보정 | 병렬 5회 | 1.5202 | 0.9054 | 0.7016 | 2.167 |
| 알려주지 않은 비용 변화 | 병렬 2회 뒤 직렬 3회 | 5.7589 | 6.6279 | 4.3020 | 1.339 |
| epoch 갱신 후 재보정 | 직렬 5회 | 20.5445 | 7.4759 | 4.5386 | 4.527 |

각 행의 총비용은 그 단계의 다섯 평가 호출, 해당 단계의 calibration, pool 시작 비용의 증가량을 합산했다. calibration에는 warmup이 이미 들어 있다. 처음 단계의 adapter warm dispatch 중앙값은 0.0785ms였지만 초기 보정 0.9755ms와 pool 비용까지 포함하면 직접 backend보다 느렸다. 이는 짧은 잔여 실행에서 **보정을 시작하기 전의 경제성 검사**가 필요한 실제 사례다.

비용 변화 단계에서는 두 번째 변경된 호출 직후 stale 관측을 폐기했다. 이 기능은 설계대로 작동했지만 이후 직렬 fallback은 실제로 더 빠른 병렬 backend를 사용하지 못했다. epoch 갱신 뒤에는 병렬 관측 최대값 1.4022ms와 직렬 최소값 1.4253ms의 간격이 진입 조건을 충족하지 못해 병렬을 선택하지 않았다. 해당 holdout의 직접 병렬 중앙값은 0.7423ms, 직접 직렬은 1.3944ms였다. **관측 재사용의 강건성은 개선했으나, 이번 세 단계에서 보정까지 포함한 속도 이득은 얻지 못했다.** 보수적 구간 비교가 만드는 false-negative 비용과 짧은 실행의 보정 비용을 다음 최적화 대상으로 남긴다.

45개 평가 배치 모두 원래 상태 출력이 bitwise 일치했다. 각 배치의 64개 ODE에 대한 최대 endpoint 오차는 짧은 구간 1.1204e-12, 긴 구간 9.1318e-9 미만으로 공통 상한 1e-7을 만족했다. 일부 warm 시간 비율이 1보다 작은 것은 별도 시점 측정의 산포를 포함하며 wrapper 자체가 같은 backend보다 빠르다는 근거로 사용하지 않는다.

## 다음 개발 단계

1. 실제 large-grid callback 호출 경계에 `ExecutionIdentity`를 연결한다. 격자 재구성·active method·chart·physical norm·provider 변경 위치에서 epoch가 함께 바뀌는지 검사한다. primitive callback 성능만으로 통합 solver의 속도를 대신하지 않는다.
2. 기존 사용자 로컬 원자료에서 선택 경로와 같은 checkpoint의 대안 하나를 재구성한다. 선택 오류, 비용 관측의 수명 만료, 실측 변동으로 결론을 낼 수 없는 구간을 따로 센다.
3. 재측정 전에 예상 잔여 호출 수로 calibration 비용의 회수 가능성을 검사하는 정책을 추가한다. 이미 측정한 뒤의 선택식에는 sunk cost를 다시 넣지 않는다.
4. 오차 수용에 쓰이는 stage residual 또는 chart 오차 예산이 바뀌면 동일한 common identity를 방법 선택과 실행 선택에 동시에 전달한다. 병렬 backend 결정은 내부 Newton/JVP 사용 권한을 변경하지 않는다.
