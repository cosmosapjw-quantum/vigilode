# 단일 스레드 switching 비교: 실행 결과

소스 commit: `548526add83a38a996875b3bd2f3cd3357465ed6`. 동일한 measurement binary, CPU affinity 0, Rayon/BLAS/OMP thread 수 1. 9개 문제·8개 방법·3개 tolerance, 한 warmup+세 timed 반복으로 **864회** 실행했다. 총 campaign wall time은 **11.159 s**, source와 binary identity는 PLAN/EXECUTION에 있다. 공유 호스트 시간은 진단용이다.

**864개 결과의 독립 재채점이 일치했다. 828회가 끝까지 도달했고 36회는 Prothero–Robinson에서 DOPRI의 25,000회 attempt budget을 소진했다.** 실패는 3개 DOPRI 출력/재시작 정책×3 tolerances×4 repeats다. 모든 raw 기록을 `RAW_RUNS.jsonl.gz`에 보존했다. 그래프에는 성공한 구성만 점으로 나타내며, 실패 목록은 SUMMARY에 있다.

Robertson의 기준은 SciPy 1.17.0 VODE BDF+analytic Jacobian, rtol1e-12/atol1e-15다. rtol1e-11/atol1e-14와의 최대 차이는 **4.437e-12**였고, 실제 switching 중간점까지 포함한 20개 시각을 계산했다. 나머지는 Python에서 해석해를 독립 계산해 상태·위상·진폭을 재채점했다. 수치 reference refinement는 엄밀한 enclosure와 구별한다.

## 같은 달성 오차에서의 비교

아래는 저장된 모든 시각/성분의 `max |error|/max(1,|initial|,|reference|)`가 **1e-6 이하**인 설정들 중 방법별 최소 median 시간이다. 실제 tolerance는 방법마다 다를 수 있다. switching이 추가로 공개한 accepted partial endpoint도 오차에 포함했으므로 switching에 더 엄격한 표본 조건이 걸릴 수 있다.

| 문제 | 가장 빠른 측정 방법 | 해당 시간 ms | switch-FD ms | switch-JVP ms | 가장 빠른 설정 오차 |
|---|---|---:|---:|---:|---:|
| damped | dopri5 | 0.4834 | 1.2146 | 7.1050 | 2.056e-08 |
| decay | dopri-whole | 0.0386 | 0.0903 | 0.0943 | 1.198e-07 |
| logistic | dopri-whole | 0.0480 | 0.0955 | 0.0985 | 1.110e-07 |
| nonnormal | dopri-whole | 0.0519 | 0.1008 | 0.1005 | 1.668e-07 |
| prothero | rodas-whole-fd | 0.3302 | 0.4584 | 1.5936 | 2.298e-07 |
| rapid | dopri5 | 4.3123 | 8.8492 | 143.1801 | 9.456e-07 |
| robertson | rodas-whole-fd | 0.2230 | 0.4251 | 2.3193 | 4.103e-08 |
| transition | rodas-fd | 0.4485 | 0.5974 | 1.8297 | 5.554e-07 |
| vector | dopri-whole-dense | 3.2835 | 3.6896 | 9.4312 | 1.709e-07 |

`dopri5`, `rodas-fd`, `rodas-jvp`, `switch-fd`, `switch-jvp`는 같은 16구간 재시작 계약을 쓴다. 추가된 whole 방법은 한 호출의 workspace를 유지한다. `dopri-whole`은 관측 시각 착지, `dopri-whole-dense`는 quartic 표본, `rodas-whole-fd`는 WFilteredEnforce를 적용한 continuous extension이다. 전체 생성·재시작·적분 비용을 포함하며, benchmark의 기준해 계산과 JSON 직렬화는 timer 밖이다.

![달성 오차와 실행 시간](../results/switching_campaign/WORK_PRECISION.png)

## 판정과 구체적인 후속 교정

1. **기능 계약:** bounded derivative fallback은 stiff Prothero 구간을 완주한다. DOPRI, JVP, FD의 비용과 거절된 probe는 분리되어 추적된다. 기본 RHS-only 정책을 JVP/FD와 혼동하지 않는다.
2. **반대 결과:** rapid에서 switch-JVP는 같은 1e-6 상한의 whole DOPRI보다 약 **32.5배 느리다**. 단순히 explicit attempt/rejection이 많다는 이유만으로 JVP RODAS가 유리하다고 결정할 수 없다. switch-FD도 이 조건에서 약 2배 느리다. damped 문제도 JVP 경로가 불리하다.
3. **재시작 비용:** cheap nonstiff 세 문제에서는 switching이 window-DOPRI와 비슷하지만 whole-DOPRI보다 대략 2배 느리다. window baseline만 제시하면 숨겨질 비용이며, persistent per-method workspace와 FSAL 유지가 다음 개발 대상이다.
4. **유리한 제한적 결과:** smooth nonstiff–stiff–nonstiff 전환에서 switch-FD는 whole-DOPRI보다 **1.512배 빠르지만**, whole-RODAS-FD보다 약 **1.28배 느리다**. 어느 후보가 빠를지는 실제 비용 비교로 결정해야 한다.
5. **다음 구현:** RODAS fallback 자체를 짧고 유한한 work budget으로 시험하고, 이전 explicit 진행량 대비 유효 시간당 비용을 비교한다. 불리한 candidate에는 재시도 간격을 길게 하는 hysteresis를 적용한다. 단일 thread에서는 analytic low-rank/quotient flow가 존재할 때 해당 action을 먼저 사용한다. 구조가 없는 RHS에서는 제한된 selective derivative fallback을 유지한다.

이번 결과로 자동 기본 dispatcher를 범용 production으로 승격하지 않는다. 선택적 adapter의 보존·실패 처리 기능을 채택하고, 성능 정책은 위의 반대 결과를 반영해 후속 교정한다. 현 상태의 작은 시스템에서는 사용자가 알고 있는 구조와 검사된 고정 방법을 선택할 수 있어야 한다.
