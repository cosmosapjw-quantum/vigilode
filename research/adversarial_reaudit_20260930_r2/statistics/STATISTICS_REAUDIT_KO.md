# VigilODE 통계·benchmark 재감사 — 2026-09-30

## 판정과 고정 범위

고정 source는 commit `7708ef90554fc3986478d4602de6a01c7266b14f`, 직접 검토한 benchmark 수정은 `882c8b0fbee3b0b291a0d7076a6c35aa40b4d778`이다. 생산 코드는 수정하지 않았다. 이전 B-01의 **명시적으로 동일한 session ID를 붙인 경로**는 고쳐졌고, B-02의 vector-unit Pareto consumer가 연결되었으며, B-03의 근거 없는 CLI median 승격은 폐쇄되었다. 그러나 session identity가 누락된 기본 측정 경로와 A/A 연결, 약화된 bootstrap 설정의 authority, legacy ledger 혼합에 잔여 문제가 있다. 일반 성능 우위 주장은 여전히 HOLD다. 이 판단은 실제 timing 우열을 뜻하지 않는다.

GPT-6 Astra 연구 하네스의 `PROJECT_INSTRUCTIONS.md`, 코딩 하네스 `AGENTS.md`와 `SCIENTIFIC_CONTRACT.md`를 읽고 정의→source→native 재현→독립 계산→개선 후보 순으로 수행했다. 하네스의 빈 계약 템플릿을 결과 증거로 사용하지 않았다. 본 문서는 통계 작업자의 결과이며 최종 독립 승격 판정은 상위 통합 검토자의 몫이다.

| 이전 finding | 재감사 판정 | 근거와 한계 |
|---|---|---|
| B-01, case 간 process 중복 | PARTIALLY_CLOSED | ID 77을 명시하면 실제 1 block/Inconclusive. 하지만 public `measure_paired_case`의 기본 빈 label이 case별 독립 session으로 인정됨. 아래 R2-STAT-01. |
| B-02, vector counter 미소비 | CLOSED_FOR_ORIGINAL_REPRODUCER | 실제 `IntegratorRunRecord::cost(OperatorStateVectors)`가 sequential 16, block 24를 반환. legacy 단독은 None. 혼합 ledger에는 별도 R2-STAT-02가 남음. |
| B-03, median-only CLI 승격 | CLOSED_FOR_ORIGINAL_REPRODUCER | CLI의 실제 gate 테스트에서 3회/1warmup/ratio0.8은 NotEvaluated. paired runner가 아직 없는 것은 정확한 보류이고 새 결함으로 세지 않는다. |

## 실행·독립성

`probe/`는 원 저장소 core/fair-ab crate를 path dependency로 링크한다. 기존 source를 복제하여 알고리즘을 바꾼 mirror가 아니다. 측정 경로 실험은 원본 `measure_paired_case`를 실제 호출하되 clock과 작업 비용은 명시적으로 synthetic이다. 한 실제 PID에서 여섯 case를 호출했다는 사실만 process identity 반례의 독립 reference로 사용한다. 합성 1.3 배를 실제 solver speedup으로 주장하지 않는다.

- `probe.log` / `probe_results.json`: 기본 측정 경로, 명시적 session 수정, disjoint A/A sessions, 원본 Pareto cost, serde legacy 누락과 checked accumulation, bootstrap B=1, 국소 coding candidate 실행.
- `cli_gate.log`: 원본 CLI `tier_l_wall_decisions_come_only_from_a_paired_assessment` 실제 실행.
- `paired_contracts.log`: 원본 timing contract 전체 실제 실행.
- `exact_bootstrap_oracle.py/.json/.log`: Rust 구현/PRNG를 공유하지 않는 독립 Python 계산. 6-session bootstrap의 6^6=46,656개 순서 표본을 전부 열거한다. coverage의 바깥 표본만 Monte Carlo다.
- 정확한 실행 결과와 exit는 `RESULT.json`의 `executions`에 기록한다. 이 보고서의 개별 검사 통과를 전체 workspace 통과로 합산하지 않는다.

## R2-STAT-01 — P2: 누락된 session identity를 독립성으로 승격하고 A/A session 연결을 확인하지 않는다

상태: **implementation-verified / STATISTICAL_METHOD + EVIDENCE_AUTHORITY**. 이전 B-01의 미폐쇄 경로이며 A/A 연결 부재는 이번에 추가 식별했다.

위치: `paired_timing.rs:224–228`은 빈 `process_blocks`를 `OwnCase(case_index)`로 바꾼다. `measure_paired_case` 자신은 `362–369`에서 빈 `process_blocks`를 반환한다. `452–458`은 그 합집합 크기를 독립 session 수로 센다. 실제 하나의 Rust process에서 이 public 측정 함수를 여섯 번 호출하면 원본 assessment는 `independent_blocks=6`, `timing_authoritative=true`, `gate_decision=Promote`를 반환한다. 후보와 A/A가 모두 그렇게 측정된다. 같은 입력에 실제 공통 session ID 77만 붙이면 `independent_blocks=1`, `Inconclusive`로 바뀐다.

문서가 빈 label을 ‘case 자체의 session’이라고 정의한 것은 byte/schema semantics일 뿐, 측정 과정에 독립 process를 만들지 않는다. 따라서 명시적으로 ID를 붙이는 caller에게는 이번 수정이 유효하지만, 기본 producer와 consumer 사이의 계약은 여전히 위험하다. 새 producer가 default로 생성한 데이터를 쓰기만 해도 허위 독립성이 생긴다는 점에서 악의적으로 ID를 위조해야 하는 사례와 다르다.

두 번째 native 반례에서 후보의 IDs는 0…5, A/A는 100…105이고 교집합이 없다. 원본 `assess_paired_timing`은 `693–700`에서 A/A를 별도로 평가할 뿐 session 연결을 확인하지 않아 다시 Promote한다. 현재 module 설명은 ‘same session’ A/A를 요구한다. 좋은 다른 세션의 A/A로 나쁜 측정 세션을 인증할 수 없어야 한다. 단, 이 합성 반례는 실제 host drift를 측정한 실험이 아니다.

**수정:** missing identity는 Unknown으로 두고 authoritative 판단을 닫는다. 실제 runner가 campaign UUID, process-launch UUID, session 범위와 raw pair에 대한 영수증을 만들고 동일한 실행을 공유하는 모든 case에 같은 ID를 전달해야 한다. A/A를 session별로 candidate/reference campaign에 묶고 policy상 필요한 coverage를 검사한다. session labels는 자가 선언만으로 실재를 입증하지 않으므로 trusted runner receipt와 assessment의 역할을 구분한다.

**Acceptance:** 한 실제 process에서 1/6/60 case를 default 생성해도 authority가 생기지 않는다. 명시적 6개의 독립 process와 session-matched A/A는 최소조건을 만족할 수 있다. A/A session 누락/불일치/타 campaign은 Inconclusive 또는 typed error다. 단순 PID 재사용을 독립성으로 오인하지 않는다.

## R2-STAT-02 — P2: legacy와 새 work ledger를 합치면 Unknown이 작은 확정 비용으로 바뀐다

상태: **implementation-verified / IMPLEMENTATION + measurement semantics**. 새 vector metric 자체는 바르게 연결되었다. 이 finding은 후속 aggregation 문제다.

위치: `work.rs:113–120`은 ‘calls>0이고 vectors=0’만 Unknown으로 판정한다. `checked_accumulate:191–250`과 `accumulate`는 field만 더한다. `global_error.rs:941–943`의 실제 Pareto consumer는 합계가 Some이면 그대로 사용한다.

재현은 완전한 원본 `WorkCounters` JSON에서 `linear_matvec_vectors`를 제거하여 legacy를 실제 deserialize한 것이다. legacy에는 16 calls가 있어 비용 None이다. 여기에 정상적인 새 ledger 1 call/1 state-vector를 `checked_accumulate`하면 17 calls/1 state-vector가 되고 실제 `IntegratorRunRecord::cost(OperatorStateVectors)`는 **Some(1.0)** 이다. 합산 전 Unknown이 합산 후 정밀한 작은 수가 된다. legacy calls가 전부 single-state-vector였다고 낙관적으로 가정해도 실제 최소는 17이다.

본 실행은 공개 ledger API와 Pareto consumer까지 연결했다. 현재 production campaign이 실제로 과거 JSON과 새 데이터를 섞어 이 결과를 발표했다는 증거는 없다. 따라서 이미 발표된 성능 순위가 틀렸다고 단정하지 않는다.

**수정:** provenance를 숫자 0에 인코딩하지 말고 schema/version 또는 coverage 상태를 보존한다. source ledger를 읽을 때 `Known(n)` 또는 `Unknown`으로 해석하고, 합산의 의미를 `Unknown + x = Unknown`으로 고정한다. Known+Known도 overflow 시 error/unknown이다. 원래 raw fields는 남겨 진단 가능하게 한다. 합계만 보고 사라진 coverage를 복원할 수는 없다.

**Acceptance:** legacy+known, known+legacy, checked/nonchecked accumulation, segment 순서 변경, serde roundtrip에서 Unknown이 보존된다. 완전히 새 ledger에서는 16+24=40을 유지한다. 새로운 numeric default로 과거 자료를 소급 인증하지 않는다.

## R2-STAT-03 — P2: bootstrap 한 번만으로도 authoritative 95% 구간과 Promote를 발행한다

상태: **implementation-verified / STATISTICAL_METHOD**. 기본 `authoritative()` 설정의 B=10,000은 이 반례를 피한다. public protocol을 변경하거나 deserialize하는 경로의 authority 제한이 불충분하다.

`paired_timing.rs:90–98`은 bootstrap 횟수가 0이 아니고 confidence가 (0,1)이면 허용한다. six-session fixture의 session별 ratio를 `[0.8,0.9,1.0,1.1,1.4,1.5]`로 하고 30 pairs를 균형 배치한다. `bootstrap_resamples=1, seed=1`에서 실제 point는 **1.048808848**, 그런데 lower=upper=**1.240967365**이 되어 Promote한다. 정상적인 같은-session A/A도 주었다. 데이터 point가 threshold 1.15 미만인데 한 개의 우연한 resample이 확정 구간처럼 쓰였다.

같은 자료·seed·95% 수준에서 기본 B=10,000으로 바꾸면 interval **[0.8485281374,1.4491376746]**, Inconclusive다. 독립 Python이 모든 46,656개 resample을 열거한 exact *empirical bootstrap distribution*의 구간도 위 두 endpoint와 일치한다. ‘exact empirical bootstrap’은 실제 모집단에 대한 정확 95% coverage라는 뜻이 아니다.

**수정:** descriptive exploratory bootstrap과 authoritative policy를 분리한다. 저비용 preview가 B=1을 허용해도 authority는 발급하지 않는다. confidence 수준, resample 수, required effect, policy/schema version이 소비자의 기준과 일치해야 한다. 특히 CLI는 현시점 None을 넣어 안전하지만, 향후 runner를 연결할 때 assessment의 required_speedup이 CLI의 1.15와 같은지도 검증해야 한다. B=10,000은 Monte Carlo quantile 안정성의 설정이지 유한 표본 coverage 증명이 아니다.

**Acceptance:** 위 fixture의 B=1은 typed nonauthoritative이고, 같은 fixture의 B=10,000은 Inconclusive다. 낮은 confidence나 다른 threshold를 넣은 assessment를 고정된 95%/1.15 gate로 소비할 수 없다. seed 탐색으로 유리한 결과를 고른 것을 confirmatory evidence로 쓰지 않는다.

## 수학 연구 루프 — estimand와 유한 session 수를 고정해야 한다

case c, session s, pair k의 무차원 log ratio를 \(X_{csk}=\log(t^{ref}_{csk}/t^{cand}_{csk})\)라 두자. 현재 point estimate는

\[
\widehat\theta=\operatorname{median}_{c}\left[\operatorname{median}_{s,k}X_{csk}\right].
\]

각 case의 pair를 먼저 합치므로 session별 pair 수가 다르면 더 많이 측정한 session이 더 큰 비중을 차지한다. fixed corpus의 같은 알고리즘을 여러 process에서 반복한 것인지, problem population에서 case들을 독립 추출한 것인지에 따라 불확실성의 대상이 달라진다. 전자에서는 고정 case 전체 vector를 session 단위로 재표집하고, 후자에서는 case sampling 모델을 별도로 정해야 한다. 현재 two-way bootstrap은 case와 session을 둘 다 재표집하며, sparse design에서는 선택된 session에 관측이 없는 case를 replicate에서 빠뜨린다. 이는 단순 구현 문제가 아니라 대상 추정량과 missingness 가정의 선택이다.

Owen의 pigeonhole bootstrap과 Owen–Eckles의 다중 factor bootstrap은 이러한 crossed effects를 고려하는 원전이다. 다만 그들의 mean/variance consistency 및 충분조건 아래 보수성 결과를 **6 session에서 median-of-medians의 percentile 95% 구간에 대한 정확한 보장**으로 옮길 수 없다. 이번 열람은 저자 arXiv 초록 범위이며 정리 전체를 검증했다고 주장하지 않는다.

이를 직접 판별했다. 모든 case가 한 session의 공통 효과를 공유하고 pair 내부 noise는 없으며, session 효과 6개는 서로 iid인 균형 설계를 사용했다. 이는 case를 재표집해도 변화가 없는 현재 bootstrap의 특수 경우다. 매 dataset에서 가능한 46,656개 bootstrap samples를 전부 열거하여 inner Monte Carlo 오차를 제거했다. 참 log median은 0이다.

| session 분포 | 독립 outer datasets | 명목 coverage | 관측 coverage | outer 표준오차 |
|---|---:|---:|---:|---:|
| Uniform[-0.3,0.3] | 20,000 | 0.95 | 0.93845 | 0.001699 |
| Normal(0,0.15²) | 20,000 | 0.95 | 0.93575 | 0.001734 |

이는 원본 Rust의 추가 오류라고 별도로 세지 않는다. percentile bootstrap의 작은 표본 근사 한계이며, ‘case도 resample하므로 보수적’이라는 일반 문구를 제한하는 **numerically checked** 결과다. 두 분포 이외의 coverage를 증명하지 않는다.

추가 발전은 두 경로를 명시적으로 택하는 것이다.

1. 기존 \(\theta\)를 유지하면 설계와 regime를 고정하고 session 수, skew, heavy tails, 상관, 불균형, 결측에 대한 coverage/power study를 먼저 한다. 이때 bootstrap 횟수 증가와 독립 session 수 증가는 서로 대체하지 않는다.
2. confirmatory 최소 pilot에서 session별 corpus score \(T_s\)의 모집단 median을 target으로 택할 수도 있다. 이는 기존 median 순서를 바꾸는 **다른 estimand**이며 별도 계약이 필요하다. iid 연속 \(T_s\) 6개에서 \([T_{(1)},T_{(6)}]\)가 참 median을 포함할 확률은 \(1-2(1/2)^6=0.96875\)다. 이는 직접 유도한 finite-sample 분포무관 구간이다. 더 좁은 구간을 원하면 session 수를 늘리거나 추가 모형 가정을 정당화해야 한다. 기존 목표를 몰래 이 목표로 바꾸면 안 된다.

## 코딩 연구 루프와 다음 개발 단계

`probe/src/candidate.rs`에 생산 코드와 분리된 작은 후보를 실제 작성·실행했다. (i) 명시적 session IDs, 최소 CI protocol, 후보/A/A의 같은 complete balanced session set을 요구하는 admission과 (ii) Unknown이 흡수되는 typed ledger addition이다. 정상 6-session 자료 허용, 빈 session 차단, disjoint A/A 차단, B=1 차단, legacy+modern Unknown, Known16+Known24=40을 실제 assertion으로 확인했다. 이것은 완전한 runner나 임의 불균형 설계의 통계 해법이 아니다. 원본 사용자 API를 수정하지 않았다.

| 다음 work unit | 선행조건 | 구현과 산출물 | 종료 조건 |
|---|---|---|---|
| STAT-R1 identity admission | 현재 정확 source 고정 | missing=Unknown, campaign/session receipt와 A/A join; 위 counterexample regression | 기본 API에서 허위 독립성·disjoint A/A 승격 0 |
| STAT-R2 metric coverage | STAT-R1과 독립 가능 | ledger provenance enum/schema migration, 흡수적 Unknown aggregation, 실제 Pareto JSON | legacy 혼합이 numeric cost로 승격되지 않음 |
| STAT-R3 confirmatory policy | 실험 estimand 결정 | descriptive/authoritative 분리, threshold/confidence/B policy 검증 | 약화된 설정으로 고정 gate를 통과할 수 없음 |
| STAT-R4 sampling design | STAT-R1, R3 | fixed-corpus 또는 case-population 명시, raw complete crossed design pilot, coverage/power study | 사전 고정된 효과·오류율·session 예산 충족; coverage 실패를 보존 |
| STAT-R5 actual CLI runner | STAT-R1–R4, solver accuracy gate | 독립 process launch, A/A, warmup/ABBA, host/build identity, raw timing과 최종 assessment를 동일 JSON에 결속 | 원본 CLI에서 missing/invalid=NotEvaluated, 유효 campaign만 해당 정책으로 평가 |
| STAT-R6 actual performance | 정확도/실패 비용 포함 | 실제 sequential/block/polynomial/homotopy를 같은 accuracy·setup policy·자원 조건으로 비교 | timing과 scientific accuracy 모두 통과해야 특정 regime 우위 주장 가능 |

실제 성능 캠페인을 시작하기 전에 하나의 session에서 많은 pair만 늘리지 않는다. q1/q2의 critical-path 단축과 state-vector work 증가는 서로 다른 관측량이므로 wall time, vector count, factorization/setup, diagnostic/fallback 비용을 모두 기록한다. 성공한 fast path만 집계하면 homotopy의 fallback 위험을 숨기게 된다.

## 원전과 읽은 범위

- Art B. Owen (2007), *The pigeonhole bootstrap*, Annals of Applied Statistics 1(2),386–411. DOI 10.1214/07-AOAS122, https://arxiv.org/abs/0712.1111 . 원저자 arXiv 초록 확인. crossed row/column resampling의 정당화 범위를 확인하는 문헌이며 이 구현의 median CI 보증이 아니다.
- Art B. Owen & Dean Eckles (2012), *Bootstrapping data arrays of arbitrary order*, Annals of Applied Statistics 6(3),895–927. DOI 10.1214/12-AOAS547, https://arxiv.org/abs/1106.2125 . 원저자 arXiv v3 초록 확인. mean variance와 충분조건 아래 mildly conservative라는 범위를 보존했다.
- Tomas Kalibera & Richard E. Jones (2013), *Rigorous Benchmarking in Reasonable Time*, DOI10.1145/2464157.2464160, https://kar.kent.ac.uk/33611/ . 저자 기관 초록 및 corrected-version notice 확인. build/process/iteration별 반복 설계와 effect-size CI의 근거이며 VigilODE의 속도 증거가 아니다.
