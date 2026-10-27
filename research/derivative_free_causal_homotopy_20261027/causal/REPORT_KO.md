# Causal Picard의 표현값 재사용: 이론·구현 연구 루프

기준 코드: `68a28357d960c4c2743c299770f0a33ef63245b2` (PR90).
사용한 규약: 사용자가 제공한 `physmath-research-loop` 1.0.0.

## 결과

새 opt-in `CausalSchedule::BatchedPicardReuse`는 기존의 represented Picard
반복을 유지하면서, 같은 입력의 nonlinear remainder와 변하지 않은 삼각계
우변 prefix의 풀이를 재사용한다. 세부 연산 순서를 유지해 동일 binary에서
모든 stage, endpoint, embedded-error vector의 bitwise 일치를 확인했다.

완료된 Riccati 적분에서는 RHS **1868→1702 (8.8865% 감소)**, 작은 block solve
**1456→1290 (11.4011% 감소)**를 얻었다. 33개 관측 시각의 모든 물리 상태와
66개 attempt의 시각·step·수용 여부가 동일하다. frozen JVP 726회는 유지된다.
이는 derivative-free 주 경로가 불리한 영역에서 사용할 수 있는 기존
JVP 기반 보호 경로의 개선이다. 이 경로에는 frozen JVP와 작은 projected
행렬이 존재하며, 새 structural consumer의 zero-JVP 계약과 구별한다.

## 이전 아이디어에서 가져온 것

PR74의 all-stage raw-U target, 공유 basis와 full-space residual 검사를
유지했다. PR75의 residual-first 원칙을 nonlinear remainder 재평가에도
확장했다. 이후 `causal.rs`의 Newton-free affine-preconditioned Picard와
유한 causal propagation을 사용한다. PR65가 기록한 반복 보정의 연산량
증가를 고려해 forward elimination도 같은 입력의 대조군으로 포함했다.
새 이름을 기존 단계 병렬화의 발명으로 제시하지 않는다. 새 기여는 기존
반복의 입력 identity를 이용한 native 작업 생략, 그 동등성 증명과 실제
전체 적분의 계산량 절감이다.

## 수학적 구성과 코드 계약

기저가 고정된 한 attempt에서 `z=(z_0,...,z_7)`,

\[
x_i(z)=\sum_{j<i}a_{ij}z_j,\qquad
N_i(z)=V^T\{f(t,y+Vx_i)-f(t,y)-J_0Vx_i\}
\]

로 두고, 고정된 block-lower-triangular affine 행렬 `A`와 우변 `b`를
사용한다. 각 반복은

\[
 r^{(k)}=b+h\gamma N(z^{(k)}),\qquad
 z^{(k+1)}=A^{-1}r^{(k)}
\]

이다. `A`의 대각 block은 같은 `I-h gamma V^T J_0 V`이고 한 번의 LU를
공유한다. stage 0의 remainder는 기존 경로와 동일하게 영벡터를 사용한다.
길이·시간 차원의 정규화와 원래 raw-U output coefficients는 변경하지
않았다.

### 정리 C1: 표현값 cache의 경로 동등성

고정된 `t,y,J0,V,A,LU`, 동일한 floating-point 환경과 결정적인 callback을
가정한다. `x_i`의 전체 binary64 비트열이 앞선 평가와 같으면, 기존 코드의
동일 순서로 계산한 `Vx_i`, `J0Vx_i`, `y+Vx_i`, `f` 및 projected remainder는
각각 같은 비트열이다. 따라서 저장한 remainder로 치환해도 represented
Picard recurrence는 같다.

증명은 각 결정적 연산의 입력 identity를 순서대로 적용한다. 물리 state만
key로 쓰면 별도로 계산하는 `J0Vx_i`의 identity가 따라오지 않으므로,
구현은 더 강한 전체 projected displacement를 key로 쓴다. `to_bits()`
비교는 `+0`과 `-0`도 구별한다. NaN/Inf는 cache admission 전에 거부한다.
cache는 함수 호출 내부에만 존재하며 step이나 basis expansion을 넘지
않는다. 모델 epoch를 dispatch 전후와 cache 사용 전후에 확인한다.

### 정리 C2: 삼각계 prefix 재사용

같은 block-lower-triangular 행렬과 같은 대각 factor를 사용하는 두 풀이에서,
앞의 `p`개 우변 block이 비트 단위로 같으면 그 `p`개 solution block도 같은
순서의 deterministic solve에서 일치한다. 이 prefix를 보존하고 `p`부터
기존 compensated dot 및 LU solve를 수행하면 전체 결과가 기존 전방
대입과 bitwise 동일하다.

첫 block에는 같은 우변과 factor가 들어간다. 그 다음 block에는 같은
우변, 행 및 이미 같은 앞쪽 solution이 들어간다. 이 귀납을 `p`까지
적용한 뒤, suffix에서도 같은 계산을 적용하면 증명이 끝난다.

`affine_solve_reuse`는 최대 일치 prefix만 재사용한다. 앞선 solution이
바뀌었는데 후방 우변 block이 우연히 같다는 이유로 그 solve를 생략하지
않는다. prefix 이후의 block 계산 순서는 baseline과 같다.

### 유한 causal 전파와 비용

strict causal graph에서 stage `i`의 비선형 입력은 앞선 stage에만 의존한다.
고정점 반복의 안정된 prefix가 늘어나면 cache 가능한 부분도 늘어난다.
이는 전역 contraction bound를 추가하는 작업과 독립적이다. 같은 target을
더 적은 callbacks로 계산하는 정리이며, Picard iteration 수나 임계경로
round 수가 자동으로 줄어든다는 정리는 아니다.

정확 산술에서 stage 0의 affine seed가 고정된 8-stage 완전 lower graph에는
7,6,...,1개의 새 remainder 평가가 남아 최대 28개가 된다. 실제 허용오차에서 더 일찍 멈추는
경로와 표현값 roundoff에 의한 residual floor는 실측으로 구별한다.
cache를 사용해 고정점 반복 자체가 더 강한 물리적 오차 인증을 얻지는
않는다. 기존 full-space screen과 embedded 물리-step 판단을 유지한다.

### Callback와 자원 계약

수치 callback 결과는 고정 `(epoch,t,state)`에서 결정적이어야 한다.
진행 epoch가 없는 모델에도 이 고정 모델 전제는 적용된다. callback-owned
counter 등 부수 관측은 줄어든 호출 횟수를 관측한다. 생략한 재호출에서
발생할 미래의 외부 오류를 예측하는 계약은 제공하지 않는다.

각 batch는 실제 활성 stage 수만 예약하고 기록한다. 모든 worker를 join하고
attempted/completed 횟수를 합산한 후 첫 오류를 반환한다. epoch 변화는
attempt를 거부한다. full-space residual 검사는 계속 실행하며, 작은 기저가
투영 밖의 성분을 잃는 사례는 기존처럼 거부한다. callback 비용을 덜 쓰므로
더 작은 budget에서 새 경로만 완료될 수 있다. 비트 동등성 비교에는 두
경로가 모두 완료할 공통 budget을 사용했다.

## 실제 검증

`native_tests.log`: 기존 causal 계약 6개와 새 계약 6개, **12 PASS**.
새 검증은 27개 affine/nonlinear fixture의 bitwise parity, four-worker 결과와
callback accounting, 더 작은 callback cap, 실패 batch의 완전 계수,
epoch 변경, 불완전 기저의 full-space 거부를 다룬다.

`WORK_RESULTS.json`: n=6,96,6144; epsilon=0,1,10; h=.01,.1; worker=1,4;
legacy/reuse/forward의 **108/108 raw-target 후보가 정확도 cap을 통과**했다.
모든 legacy/reuse pair의 모든 stage/output/error 비트가 동일했다. 별도의
scalar raw-U 전방 소거 oracle과 최대 절대차는 **4.73649e-14**였다
(등록 cap 2e-10).

강한 비선형 fixture `epsilon=10,h=.1`에서는 각 차원에서 다음을 관측했다.

| 경로 | RHS | frozen JVP | block solves | Picard updates |
|---|---:|---:|---:|---:|
| Legacy Picard | 58 | 10 | 56 | 6 |
| Reused Picard | 37 | 10 | 35 | 6 |
| Causal forward | 16 | 10 | 8 | 0 |

Reuse는 RHS 36.2069%, block solve 37.5%를 줄였다. Forward가 이 fixture의
최소 작업량 경로이며, Picard reuse는 독립 stage callback의 병렬 실행을
허용한다. candidate 전체(full-space validation 포함)의 실제 peak concurrency
4도 관측했다. 현재 peak counter는 두 phase를 나누지 않는다.

전체 Riccati 적분은 epsilon=.5, [0,.2], 33개 물리 관측 시각,
atol=1e-11, rtol=1e-7로 실행했다.

| 차원 | attempts/accepted | RHS old→new | solves old→new | 최대 물리 오차 |
|---|---:|---:|---:|---:|
| 6 | 66/65 | 1868→1702 | 1456→1290 | 2.6856392e-11 |
| 96 | 66/65 | 1868→1702 | 1456→1290 | 2.6856479e-11 |

독립적인 정확해는
`0.1 exp(a t) / [1+(.05/a)(1-exp(a t))]`이다. 두 차원 모두 3e-9 cap을
통과했고 fallback은 0회였다. accepted/rejected pattern과 모든 observation
비트가 같다. 관측 grid의 검증이며 연속 시간 supremum의 수치 인증은
별도 문제다.

## 시간 측정과 적용 범위

`PROTOCOL.md`를 실행 전에 작성하고, same-binary 5개 ABBA/BAAB block,
segment당 5회로 한 번 측정했다. affinity 0–3, thread-pool 구성·oracle·JSON은
timer 밖이다. 내부 timer에는 기저 생성, factorization, candidate solve,
full-space 검증을 모두 포함한다. 실행 시간 3.798초였다.

| n | workers | old/new paired median | 5 block 범위 |
|---:|---:|---:|---:|
| 96 | 1 | 1.2914 | 1.2733–1.6989 |
| 96 | 2 | 0.8727 | 0.2628–1.5385 |
| 96 | 4 | 1.0019 | 0.4338–1.4228 |
| 6144 | 1 | 1.2820 | 1.1502–1.7341 |
| 6144 | 2 | 1.2794 | 1.0209–1.7976 |
| 6144 | 4 | 1.3950 | 1.2296–2.1593 |

이 timed fixture의 embedded WRMS는 약46210이므로 물리 integrator는
해당 h=.1 step을 거부한다. 표는 **동일하게 정확한 raw-U target attempt의
비용**이다. 정확도 cap을 통과한 전체 적분의 감소는 앞 표의 callback 및
solve 횟수로 확인했고, 전체 적분 시간은 측정하지 않았다. 작은 n에서
worker scheduling 비용과 큰 산포가 나타났다. paired 범위는 신뢰구간이
아니다. worker별 old/new 비율은 worker1 대비 병렬 speedup과도 다르다.

## 실제 R1–R10 / C1–C10

| 단계 | 실행한 내용과 근거 |
|---|---|
| R1 / C1 | PR90 baseline·원래 raw-U target·고정 accuracy/counter 목표를 확인했다. |
| R2 / C2 | PR74/75와 기존 causal implementation을 식과 callback 경로까지 읽고 재사용했다. |
| R3 / C3 | frozen JVP 경로와 완전 derivative-free 소비자를 구분하고 고정 epoch/결정적 입력 계약을 정렬했다. |
| R4 / C4 | 물리 state만의 cache와 전체 projected input cache를 비교해 후자를 선택했다. 별도 native agents와 compile lock으로 실행했다. |
| R5 / C5 | C1/C2를 유도하고 실제 opt-in enum, cache, suffix solve 및 native 실험을 구현했다. |
| R6 / C6 | theory agent가 식과 코드를 독립적으로 검토했다. 부모 검토의 callback 계약 명료화 지적을 public API에 반영했다. 12 native 계약을 실행했다. |
| R7 / C7 | 108개 same-target 후보와 독립 scalar oracle, 두 전체 Riccati 적분을 실제 실행했다. |
| R8 / C8 | raw-target work 절감과 whole-trajectory work 절감, timing의 outer rejection 및 작은-n 회귀를 구분했다. raw JSON·명령·binary hash를 보존했다. |
| R9 / C9 | proof-to-code 계약을 본 보고서와 public API에 기록하고 parent/theory code review를 처리했다. |
| R10 / C10 | 새 기능·회귀·부정적 timing 셀·재현 명령을 함께 인계한다. 기존 경로와 defaults는 보존된다. |

독립 검토는 같은 대화 계보의 서로 다른 agent가 수행한 상보적 한 차례
검토다. 별도 모델의 독립 검증으로 표시하지 않는다. 새로운 strong no-go
정리를 제시하지 않았으므로 NG1–NG7의 채택 대상은 없다.

## 재현과 실행 이력

공통 환경: 제공된 Rust1.94.1, 기존 Cargo.lock과 offline dependencies,
`CARGO_BUILD_JOBS=2`, `--profile measurement`. 명령과 시간은
`TEST_RECEIPT.json`, `BUILD_RECEIPT.json`, `WORK_RECEIPT.json`,
`TIMING_RECEIPT.json`에 있다. binary SHA256은
`a5a18c798298e42aab78c2ee82b44b4d142ebdaa1183fc774fbccea73f0d6cca`.

첫 default-debug 실행에 대해서는 원래 receipt가222.935초와 exit0을
기록했지만, 동시에 `status=cancelled`, `tests_executed=false`로 해석했다.
최종 provenance 감사에서 이 해석을 교정했다. 짧은
`debug_profile_cancelled.log`는 빌드 중 출력만 포함하고, 보존된 더 긴
`debug_profile_aborted.log`의 앞2521byte와 정확히 같다. 더 긴 로그에는
`Finished test profile [unoptimized + debuginfo]` 및3m42s가 기록되어 있고,
`causal_contracts`와 `causal_reuse_contracts` 두 suite가 각각6 PASS로
완료되어 있다. 따라서 이 보고서의 이전 “실제 test 출력이 없다”는
서술과 receipt의 `tests_executed=false`는 전체 보존 증거와 모순된다.

중단은 요청된 것으로 기록하되, 실제 SIGINT 전달 시점이나 그 효과는
보존 파일만으로 확정할 수 없다. 로그 이름의 cancelled/aborted도 종료
상태의 증거로 쓰지 않는다. 파일 mtime은 artifact metadata이며 실제
명령 시작·종료 timestamp를 대신하지 않는다. 원래222.935초 기록은
긴 로그의 build/test 완료와 양립하지만 독립적으로 재계측한 값은 아니다.

원래 `DEBUG_PROFILE_CANCELLED_RECEIPT.json` 및 두 로그를 byte 그대로
보존했다. 새 `DEBUG_PROFILE_OBSERVATIONAL_CORRECTION.json`이 원래
receipt를 해석상 superseded metadata로 지정하고, 두 로그의 전체SHA256,
파일 시각, byte-prefix 관계, 실제 test 이름/결과와 provenance 한계를
기록한다. 선택한 검증 근거는 measurement profile의
`TEST_RECEIPT.json`과 `native_tests.log`이며, 같은 두 suite의 debug 통과를
별도의 추가 계약12개로 중복 집계하지 않는다. measurement profile에서
실제 계약12개가11.413초, example build가11.522초,
work/trajectory 실행이0.289초에 완료됐다. 이 감사 교정에는 재실행이나
코드 변경이 없다.

```bash
cargo test --profile measurement --locked --offline -p vigilode-coupled \
  --test causal_reuse_contracts --test causal_contracts
cargo build --profile measurement --locked --offline -p vigilode-coupled \
  --example causal_reuse_bench
taskset -c 0-3 "$CARGO_TARGET_DIR/measurement/examples/causal_reuse_bench"
taskset -c 0-3 "$CARGO_TARGET_DIR/measurement/examples/causal_reuse_bench" timing
```

선택적 적용은 caller가 새 schedule을 지정하는 방식으로 제공한다. 전체
trajectory에서 state/counter 계약을 보존한 비용 절감이 확인됐으므로 이를
채택할 수 있다. 입력별 최적 route를 결정하는 학습/선택 정책과, derivative-free
structural 소비자의 적용 범위는 통합 부모 연구의 별도 과제다.
