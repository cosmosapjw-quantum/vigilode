# R3 측정 권위·작업량 감사 및 통계 연구

대상은 `cc2cd041737e7ff543624d1b59893a3b4397369f`, tree `182fa306067b84ee10ec1f1b1e5fbad9e769aae1`이다. 이전 `1ba914d` 보고서에서 발견한 세 문제의 원 재현을 현재 public API로 다시 구성하고, 원자료 검증이 실제 측정 프로토콜을 검증하는지 구분했다. 모든 clock·duration fixture는 합성 입력이다. 성능 개선을 측정했다는 주장은 없다.

## 이전 문제의 재판정

| 항목 | 현재 설계 및 판정 범위 |
|---|---|
| R2-STAT-01 | 라벨 없는 여섯 case는 독립 session 0개로 기록한다. 동일 label 77은 1개다. candidate 0…5와 A/A 100…105는 세션 집합 불일치로 authority를 받지 않는다. 여섯 명시적 session과 일치하는 A/A의 범위에서만 native raw verification을 통과시킨다. |
| R2-STAT-02 | `unknown_vector_calls`를 merge 이전에 계산해서 보존한다. legacy 16 calls + modern 1 call/1 vector의 합은 17 calls, vector cost `None`이다. 순서를 바꾸거나 JSON 왕복 후 다시 합쳐도 Unknown이다. Pareto 비용 public consumer에서도 `None`이다. |
| R2-STAT-03 | bootstrap B=1의 assessment는 preview로 보존되지만 gate는 Inconclusive이고 confirmatory consumer는 거부한다. B=10000인 원 fixture의 CI는 독립 exact bootstrap의 `[0.848528137423857, 1.449137674618944]`와 일치한다. |

새 native probe의 exit와 각 assertion은 `EXECUTION_RECEIPT.json`과 `native_probe.jsonl`에 기록한다. private front/attainment 함수, CLI gate의 기존 변경 테스트는 root runtime campaign 결과를 별도로 인용해야 하며 이 문서의 소스 독해만으로 실행 PASS를 주장하지 않는다.

## R3-STAT-01 — 원자료를 재계산해도 프로토콜 위반을 승인할 수 있음

심각도 P2, 층 `EVIDENCE_AUTHORITY / IMPLEMENTATION`. `PairedTimingCase::validate` (`paired_timing.rs:288–323`)는 duration 양수·유한성, pair 수와 session label 길이를 확인하지만 `order` 자체, warmup 유한성/부호, 실제 batch 값과 calibration 결과의 일치를 검사하지 않는다. `assess_paired_timing`과 `verify_against_raw`는 같은 누락된 검증을 공유한다.

public `from_samples`로 ratio 1.3, 30 pairs, 명시된 여섯 session을 구성하고 동일 session의 ratio 1 A/A를 둔다. 이후 원자료의 한 필드만 다음과 같이 바꾼다.

1. `order=[]`로 만든다.
2. 모든 pair를 Candidate→Reference로 바꾸어 seeded ABBA를 위반한다.
3. warmup이 지시하는 batch=2 대신 batch=1로 둔다.
4. warmup duration을 모두 −1초로 둔다.
5. in-memory warmup을 NaN으로 둔다.

이 fixture는 원자료를 변경한 **후에** assessment를 만들며, 이미 만들어진 assessment만 바꾸는 공격과 다르다. SHA-256 및 재계산 일치는 새로 만들어진 잘못된 자료와의 일관성만 확인한다. 첫 네 fixture는 JSON으로도 표현할 수 있다. NaN fixture는 Rust API 범위의 추가 재현이다. JSON 출력에서는 NaN이 `null`로 표시되므로 이것을 유효한 JSON duration replay라고 해석하면 안 된다.

현재 소스의 raw verification은 위 프로토콜 위반을 검사하지 못한다. 따라서 실제 paired runner가 receipt를 공급하기 전에 이 검증을 보강해야 한다. 이것은 일반적인 “caller label을 신뢰한다”라는 이미 문서화된 한계보다 좁고 직접 검사 가능한 일관성 오류다. 반대로 현재 CLI에는 그 runner가 없고 wall criterion이 NotEvaluated이므로 **현재 CLI가 실제 성능을 오승격했다는 재현은 아니다**. `measure_paired_case` 자체가 잘못된 ABBA를 생산한다는 주장도 아니다.

최소 수정은 raw admission 시 `calibrate_batch_iterations(warmup_seconds, protocol)`를 다시 호출하여 성공과 batch 동일성을 요구하고, `order == abba_pair_order(pair_count, seed)` 및 pair 수를 검사하는 것이다. 미래 runner가 세션별 별도 seed를 허용하면 그 seed를 receipt schema에 명시해야 한다. `probe/src/main.rs::candidate_receipt_check`는 이 두 검사를 독립 후보로 구현했다. 변경하지 않은 정상 fixture는 허용하고 위 다섯 malformed fixture는 거부하도록 검증한다. 아직 production patch는 아니다.

## 통계 연구 — 유한 bootstrap Monte Carlo 오차를 gate에 전달하기

현재 코드가 새로 기록하는 percentile endpoint의 ±2 binomial 표준편차 band는 유용한 진단이다. 그것은 전체 bootstrap CI의 모집단 coverage 보장도 아니고, 일정 수준으로 보장된 Monte Carlo 승격 gate도 아니다. 이를 기존 CI 결함으로 중복 계수하지 않고 다음 단계의 구체적인 연구 후보를 만들었다.

고정된 경험자료와 bootstrap resampling law 아래 log-speedup 통계량을 $T^*$, 필요한 speedup의 로그를 $\tau=\log(1.15)$라고 하자. $B$개 iid resample에 대해

\[
X_b=\mathbf1\{T_b^*<\tau\},\quad p_*=P(T^*<\tau),\quad
\widehat p=B^{-1}\sum_{b=1}^B X_b.
\]

Bernoulli 변수의 centered log-MGF $g(\lambda)=\log E\exp[\lambda(X-p_*)]$는 $g(0)=g'(0)=0$, $g''(\lambda)=\operatorname{Var}_{\lambda}(X)\le1/4$를 만족하므로 $g(\lambda)\le\lambda^2/8$이다. 독립 표본의 exponential Markov inequality를 적용하고 $\lambda=4\epsilon$에서 최소화하면

\[
P(\widehat p-p_*\ge\epsilon)\le e^{-2B\epsilon^2},\qquad
P(p_*-\widehat p\ge\epsilon)\le e^{-2B\epsilon^2}.
\]

따라서 두 방향에 합계 $\delta$를 배분하면 확률 적어도 $1-\delta$로

\[
p_*\in[L,U],\qquad
[L,U]=[\max(0,\widehat p-\epsilon),\min(1,\widehat p+\epsilon)],\quad
\epsilon=\sqrt{\frac{\log(2/\delta)}{2B}}.
\]

원 bootstrap percentile tail이 $a=0.025$일 때 $U<a$이면 lower percentile이 $\tau$ 이상이라는 보수적인 Promote 조건이다. $L>1-a$이면 upper percentile이 $\tau$보다 작다는 Block 조건이다. 나머지는 Inconclusive다. 엄격 부등식을 써서 discrete atom의 경계 문제를 피한다. 이 유도는 확률분포의 차원을 바꾸지 않으며, 기존 median-of-case-medians estimand를 다른 평균이나 세션 median으로 대체하지 않는다.

`bootstrap_mc_candidate.py`에서 기존 six-session fixture의 모든 $6^6=46,656$ resample을 열거했다. 독립 Python 구현의 seed=1/B=10000 percentile은 exact endpoint와 1e−12 이내에서 일치한다. B=10000, δ=.01이면 ε=0.0162762363이다. 기존 mixed fixture는 Inconclusive, ratio 1.3으로 고정된 fixture는 Promote, ratio .9 fixture는 Block이다. 별도 threshold 주변의 탐색 fixture도 Inconclusive이다. 데이터에서 경계 fixture를 만들었으므로 독립 holdout이 아니다.

보장 범위는 **경험자료에 조건부인 유한 resampling 오차**뿐이다. 원래 empirical bootstrap이 모집단 speedup CI로서 95% coverage를 갖는지는 별도 문제다. 이 구분을 무시하여 “99% 통계적 성능 승격”이라고 부르면 안 된다. 또한 이 정리는 iid 이상적 resample에 대한 것이며, 실제 고정 seed의 PRNG를 확률적으로 인증하지 않는다. B와 threshold는 사전 고정이다. 데이터에 따라 B를 계속 늘리면서 같은 δ를 재사용하는 optional stopping은 이 보장에 포함하지 않는다. 더 좁은 band가 필요하면 동일 Bernoulli 문제의 binomial tail inversion을 구현할 수 있으나 이번에는 구현하지 않았다.

## 다음 개발 단계

1. **Raw admission을 먼저 완성한다.** warmup, batch, ABBA, duration 및 session schema를 하나의 검증 함수로 묶고 public producer, replay reader, `verify_against_raw` 모두에서 호출한다. malformed 원자료로 assessment 자체를 새로 만드는 회귀를 포함해야 한다.
2. **Runner가 원자료를 내도록 한다.** campaign UUID·세션 생성 방식·실행 identity·실측 순서·batch 단위 duration을 남기고 endpoint artifact를 원자료 digest와 연결한다. 기존 CLI의 NotEvaluated를 실제 authoritative로 바꾸는 것은 그 후다. 최소 threshold만 통과한 JSON을 수동 주입하는 방식은 피한다.
3. **MC gate를 preview에서 검증한다.** 고정 B·δ·threshold를 기록하고 exact enumerable fixtures의 true conditional tail probability와 비교한다. 더 작은 CI width를 위해 B를 바꾼다면 그 정책을 사전 고정하거나 time-uniform confidence sequence를 별도로 도입한다.
4. **통계적 성능 주장을 실제 설계에 결속한다.** cases×sessions의 incidence table과 각 pair 수를 기록하고, 불균형·missing-cell design에서 estimand가 어떻게 가중되는지 먼저 정한다. 현재 연구 후보를 모집단 coverage의 대체물로 사용하지 않는다.

`RESULT.json`은 finding/closure/claim scope, `NEXT_STEPS.json`은 입력·수정 위치·수락 조건·산출물 계약을 담는다. 해시 검사는 source identity이며 과학적 승인과 별개다.
