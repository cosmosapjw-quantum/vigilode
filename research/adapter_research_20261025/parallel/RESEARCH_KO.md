# 관측 비용에 따른 병렬 실행 선택

## 조사 대상과 확인된 상태

`crates/vigilode/examples/local_bottleneck_support/models.rs::Model::apply`는
`parallel` 플래그가 참이면 출력 성분에 `par_iter_mut`를 적용한다. 고정된
성분 수, callback 비용, 최근 측정치에 따른 선택은 없다. `local_bottleneck_suite`
는 사용자가 선택한 serial/data/ensemble 경로와 스레드 수를 그대로 실행한다.
`rodas5p-integrators::ParallelExecution`은 순서 보존과 지역 pool을 제공하지만
작업별 손익 판단이나 nested 실행 억제를 수행하지 않는다.

이 때문에 기존 sweep은 병렬화 가능한 경로를 비교하는 실험이었으며, 실제
사용자가 호출하는 자동 실행 정책의 정확도를 측정한 실험은 아니었다.

기존 N5 nonlinear all-pair phase-coupling fixture에서 1→2 스레드에 따른
같은 경로 속도비는 fixed-causal 1.310, fixed-Picard 1.449,
fixed-residual-first 1.742였다. 그러나 두 스레드 fixed-residual-first도
직렬 dense보다 약 9.46배 오래 걸렸다. 병렬화 자체의 이득과 최적 방법을
선택한 결과는 별도의 질문이다. 자료: `research/local_frontier_20261024/analysis/N5_PARALLEL_CALLBACKS.json`.

최신 `stop_audit_20261025/evidence/state/TIMEOUT_FAMILY_TRIAGE.json`에서
199개 timeout의 stdout은 모두 header-only이고 accepted-step 기록이 없다.
그 중 126개가 case14/23/24에 집중된다. 이 자료만으로 lock 정체, 작은 스텝,
비싼 RHS를 분리할 수 없다. 다른 완결된 case12 data/8 실행에서 시스템 CPU
84.51초와 약 1,541만 회 비자발적 문맥 전환이 관찰되지만, 이는 scheduler 비용의
단서이며 병렬화가 timeout의 원인이라는 증명은 아니다.

## 계산식과 조건부 보장

같은 문제·방법·허용오차·작업량에 대해 직렬 비용을 S, 병렬 비용을 P,
보정 실험과 pool 시작의 추가 비용을 H, 남은 반복 수를 m이라 하자.
두 경로가 같은 유효한 결과를 반환하고 비용이 반복 동안 고정되면

    T_serial − T_adaptive = m(S − P) − H.

따라서 S>P이고 m>H/(S−P)이면 보정 비용을 포함해 adaptive 병렬 실행이
직렬 실행보다 짧다. 이는 실제 미래 비용에 대한 조건부 대수 명제이다.
파일럿은 세 쌍의 측정에서 S_lo=min(S_i), P_hi=max(P_i)를 구성하고
`H/m < S_lo−P_hi`를 비용 회수 진단값으로 기록한다. 이후 비용이 S≥S_lo, P≤P_hi 범위에 머문다면
같은 부등식이 보장을 준다. 표본의 최솟값/최댓값은 미래 비용의 통계적
신뢰구간이 아니므로 실제 성능 보장은 별도 held-out 측정으로 평가한다.

## 구현

`crates/vigilode/src/execution_policy.rs`에 opt-in 정책과 실제 dispatcher를
추가했다. 기존 solver의 기본 동작은 변경하지 않는다.

- 키는 model epoch, method tag, 차원, batch 크기, ensemble/data/stage 종류,
  환경 epoch의 정확한 조합이다. 방법 전환·모델 변경·affinity 변경 시 키를 바꾼다.
- 세 쌍의 실제 비용과 결과 동등성 확인 전에는 직렬 실행한다. 병렬 진입은
  기본 15% 여유를 요구한다. 이미 지출한 보정 비용의 회수 여부는 별도로 기록한다.
- 현재 병렬 경로에는 2% 이탈 여유를 두며, 보정이 128회 선택 이후 낡으면
  직렬로 돌아가 재측정을 요청한다. 새로운 key로 비용을 무단 일반화하지 않는다.
- `map_ordered`는 선택한 backend로 실제 실행하며 결과 순서를 보존한다.
  정상 호출은 입력을 중복 평가하지 않는다. 작업 callback 안의 thread-local
  guard는 이 wrapper의 nested 병렬 실행을 직렬로 바꾼다. 다른 라이브러리가
  수행하는 외부 병렬화는 `external_parallel_context=true`로 알린다.
- `calibrate_replay_safe`만 명시적으로 순수/재시작 가능한 작업을 재실행한다.
  `Fn + Sync` 자체가 부작용 없음의 증거는 아니다. IO, RNG, event journal,
  외부 capture를 가진 callback은 효과를 격리하거나 calibration 대상에서 제외한다.
- PR87의 수락 후 효과 전달과 PR88의 callback full-write 계약은 서로 다른
  안전 성질이다. 계산 결과 동등성과 효과 동등성 모두 만족해야 replay할 수 있다.

## 선택 중심 검증

`execution_policy_pilot`은 실제 Dopri5 logistic IVP를 독립 ensemble로 풀고
매 구성원의 해석적 종료값 오차를 1e-7 이하로 검사한다. batch=1/64/256,
후보 workers=2를 사용한다. 한 쌍은 비용에 포함되는 warmup이고 이후 세 쌍은 calibration이며, 그 뒤 20회는 별도
held-out 관측이다. 직렬/병렬/adaptive 순서를 순환시켜 단순한 순서 편향을 줄인다.
진단 comparator 두 개를 남겨 선택 오류를 관측하지만 전 방법×전 스레드
Cartesian sweep은 수행하지 않는다.

핵심 출력은 선택 mode/reason, calibration+startup 포함 총비용, held-out 최선
경로 대비 시간비, 선택 불일치율이다. 이 짧은 실험은 이 실행기 API의 효과를
검사한다. 실제 사용자 호스트의 동적 stiff PDE 성능은 해당 원자료와 후속 실행으로
판정해야 한다. callback 데이터 병렬화와 coupled-stage batch에도 같은 정책을
연결할 수 있으나 이번 native pilot의 실제 연결 대상은 독립 ODE ensemble이다.

## 로컬 후속 작업

1. 기존 완결 raw receipt에서 동일 source/model/method/tolerance/thread scope를
   짝지어 선택기를 학습할 수 있는 비용자료를 추출한다. timeout은 검열값으로
   남기며 60초를 실제 종료 시간으로 취급하지 않는다.
2. ensemble/data/stage별로 직렬과 후보 2 worker만 짧게 보정하고, 이득이 있을 때만
   더 큰 후보를 인접 비교한다. 별도 cold/hot pool 비용과 CPU affinity를 보존한다.
3. 방법 switch가 일어나면 새 method tag로 보정을 무효화한다. h 변경만으로 항상
   재측정하지 않고 RHS/선형해법 작업량이 바뀐 regime epoch를 갱신한다.
4. 선택 정책의 calibration을 제외한 평가 구간에서 정확도와 종료 성공을 먼저
   확인하고, total wall time 및 최선 유효 comparator 대비 regret를 계산한다.
5. 최근 창의 비용이 calibration 범위를 벗어나면 짧은 paired probe를 예약한다.
   반복 전체 재실행 대신 선택 오류가 난 창의 상태·h·method·pool 정보를 보존한다.


## 첫 실제 실행과 한 차례 수정

첫 실행의 원본은 `initial/pilot.jsonl`, `initial/PILOT_SUMMARY.json`,
`initial/execution_policy.rs`에 보존했다. 세 구성 모두 직렬을 선택했다.
batch=1은 올바른 선택이지만 불필요한 pool 생성과 calibration까지 합쳐
총 0.5774ms를 썼고 직렬 비교 총시간은 0.2319ms였다. batch=64는 held-out
병렬 중앙시간이 0.5015ms, 직렬이 0.9835ms인데도 직렬을 선택했다.
batch=256도 병렬 1.8036ms, 직렬 2.7242ms인데 직렬을 선택했다.

batch=64의 정확한 선택 원인은 calibration S_lo=0.589923ms,
P_hi=0.509293ms, H=3.46805ms였다. 보정 비용 회수에는 관측 envelope로
43.01회가 필요하지만 horizon=20이라서 진입을 막았다. 그러나 보정이 이미
끝난 뒤에는 H가 공통의 매몰비용이다. 앞으로의 두 선택을 비교하면

    (H + mS) − (H + mP) = m(S − P).

따라서 보정 전에는 H를 회수할 수 있는지 평가해야 하지만, 보정 후 경로
할당에 H를 다시 장벽으로 쓰면 미래 비용이 더 큰 경로를 택할 수 있다.
수정본은 회수 횟수와 horizon 충족 여부를 진단 필드로 남기고 실제 선택은
미래 비용으로 수행한다. 진입 여유 15%는 변경하지 않았다.

batch=256은 calibration의 최대 병렬 비용이 12.3752ms로, held-out 중앙값
1.8036ms와 크게 달랐다. 당시 세 쌍의 개별 비용을 저장하지 않았으므로
cold start, 일시적인 scheduler 간섭, 지속적 변동을 구분할 수 없다.
수정본은 세 쌍의 원자료와 순서를 모두 출력하며, 재사용 pool에 대한 한 쌍의
warmup을 별도로 시행한다. warmup은 표본 envelope에서만 제외하고 총비용에는
전부 포함한다. 기존 extrema 기반 선택을 임의로 중앙값으로 바꾸지 않았다.

단일 구성원 또는 단일 worker에서는 pool 생성과 calibration 실행을 생략하는
실제 fast path를 추가했다. 검증은 callback 실행 횟수 0, startup 0을 확인한다.
첫 파일럿의 불리한 결과는 폐기하지 않으며, 수정 후 결과는 별도 파일로 기록한다.
