# VigilODE R3 적대적 재감사 및 수학·코딩 연구 보고서

작성일: 2026-10-01 KST / 2026-09-30 UTC. 감사 대상은 `cc2cd041737e7ff543624d1b59893a3b4397369f`, tree `182fa306067b84ee10ec1f1b1e5fbad9e769aae1`이다. 최신 remote tip의 committer date와 R2 수정 이력을 확인하여 기존 `claude/jolly-wozniak-7wl15h-wu22-reaudit-r2` 브랜치를 선택했다. R2 보고서 게시 commit `1ba914dc234e25c532a9598b1c30ab156c75218d` 이후 8 commits, 30 files, +2216/−209를 검토했다. 새 브랜치를 만들지 않았으며 production solver를 수정하지 않았다.

## 1. 판정과 이번 업데이트의 성과

**판정은 REWORK다.** 기존 일곱 지적의 원 재현은 새 source에서 닫혔다. 그러나 이미 문서에 인정된 P1 시간 간격 불일치를 여섯 driver 종류에서 다시 관측했고, 추가 P2 세 건을 native public API에서 확인했다. 이 셋 모두 이번 업데이트가 도입한 회귀라고 단정하지 않는다. 실제 기능 개선과 남은 정확성 범위를 구분한다.

수정의 방향은 좋다. 절대 epoch에 비례한 시간 허용 오차를 줄이고 인접 출력점을 구분했으며, 곱의 exponent를 분리하여 φ 가중치와 budget의 기존 overflow/underflow 반례를 해결했다. session identity를 명시하고 Unknown work coverage를 absorbing하게 만든 점, 낮은 bootstrap 반복 수를 confirmatory gate에서 차단한 점도 실제 재현으로 확인했다. 다음 개발은 새 backend를 넓히기 전에 **시각–상태 계약, 원 입력–변환 입력의 오차 연결, raw timing admission**을 닫는 것이 가장 가치가 크다.

| R2 지적 | 이번 독립 실행 | 판정 범위 |
|---|---|---|
| R2-OUT-01 | 큰 ±epoch의 짧은 span, 1-ULP span, dense/clipped 인접 출력 | 원 재현 닫힘; 실제 소비한 h와 clock displacement 문제는 별개로 남음 |
| PHI-R1 | 극단적인 h⁴b₄ 여섯 dense/fused 결과, exact Fraction 참조 | 최대 상대오차 8.50e−16 |
| PHI-R2 | 진폭 1e−200…1e300의 dense/fused 20행, Decimal 참조 | 최대 상대오차 3.93e−16 |
| R2-POL-01 | public policy, exact represented-input budget | 예산 0.9999888671826829; WRMS 5 거절 |
| R2-STAT-01 | 무label, 중복 label, 불일치 A/A session | confirmatory 권한 제한 확인; label은 아직 OS 관측값이 아님 |
| R2-STAT-02 | legacy/modern work 양방향 merge·serde·cost consumer | Unknown 유지; unknown 비용을 평가된 값으로 승격하지 않음 |
| R2-STAT-03 | B=1 preview 거절, B=10000 대 exact 6⁶ bootstrap | 원 gate 닫힘; 모집단 coverage 증명과는 다름 |

이 표는 원 반례 closure다. 모든 가능한 입력에 대한 correctness 증명이나 이전 전체 suite의 PASS 상속이 아니다. 원자료·수치·실행 범위는 `REPORT_STATUS.json`, 각 lane receipt 및 runtime 부록을 함께 읽어야 한다.

## 2. 우선순위별 남은 결함

### P1 R3-TIME-01: 성공한 적분의 물리 간격과 표현 시각이 다르다

이것은 `docs/REAUDIT_R2_CLOSURE_20260930.md`의 한계 1에 이미 인정된 잔여 문제다. 새로운 발견으로 세지 않는다. `t₀=10¹²`, 종료점은 8 ULP 뒤, 제안 step은 `10⁻⁴`로 설정한다. 이 epoch의 한 ULP는 `0.0001220703125`이고 전체 span은 `0.0009765625`다. `y′=1/span, y₀=0`의 정확한 종료값은 1이다. native 결과는 성공하면서 약 **0.8192**, 즉 **18.08% 오차**다.

clock은 매번 한 ULP 전진하지만 stage는 nominal h=0.0001만 적분한다. 상수 flow에서 embedded error는 공유된 clock 오류를 감지하지 못한다. fixed/adaptive RODAS, fixed Radau/BDF, dense RODAS, adaptive fused exponential에서 총 7개 관측으로 재현했다. 관련 위치는 `output.rs:26–57`, `integrate.rs:92–103`, `dense_output_v2.rs:434–450`, `bdf.rs:608–619`, `radau.rs:651–655`, `adaptive_exponential.rs:201–212`다.

**수정 계약:** 한 step plan에 `t_start,t_end,h_eff`를 결속하고 stage, dense extension, controller/history, BDF 계수와 receipt 모두 같은 간격을 사용해야 한다. 국소 Sterbenz 조건에서는 `h_eff=t_end−t_start`를 사용한 standalone public-step 후보가 세 origin에서 최대 오차 3.11e−15로 통과했다. 이 결과를 모든 opposite-sign time interval, 내부 half-step, BDF history에 대한 해법으로 확대하면 안 된다. 실제 h_eff가 hard max_step을 넘으면 조용히 확대하지 말고 typed time-resolution failure를 반환해야 한다.

### P2 R3-TIME-02: singleton 출력표가 y₀를 y(tf)로 반환한다

`uniform(10¹²,10¹²+4ULP,1.0)`이 `[tf]`를 만들고 span 검증을 통과한다. `y′=2048, y₀=0`을 내부 두 step으로 실제 적분했는데도 dense와 clipped 반환값은 **성공, 표본 1개, y(tf)=0**이다. 정확한 값은 1이다. explicit `[tf]`도 같다. `OutputCollector`가 첫 요청점에 y₀를 저장하고 이미 모든 요청을 처리했다고 생각하기 때문이다. `output.rs:173–203,361–378,408–417`에 원인이 연결된다.

**수정 계약:** nonzero span에는 적어도 두 개의 실제 endpoint를 요구한다. zero로 반올림된 uniform interval count를 epoch-scaled tolerance로 승인하지 않는다. endpoint equality를 엄격히 검사하거나, alias를 허용하는 별도 API에서 실제 시각과 요청 label을 구분해서 반환한다. standalone strict admission은 invalid/alias 세 입력을 거절하고 정확한 두 endpoint 대조군을 허용했다. 이 검사는 적분 시작 전에 실패해야 한다.

### P2 R3-ARITH-01: 부분 underflow 뒤 convergence가 원 문제에 적용된다

유한 입력

\[
A=\begin{pmatrix}0&10^{308}\\0&0\end{pmatrix},\quad h=\pm10^{-8},\quad
b_0=(10^{-300},0)^T,\quad b_2=(0,10^{-310})^T
\]

에서 `A²=0`이므로 참조식을 정확하게 종료시킬 수 있다.

\[
e^{hA}b_0+h^2\varphi_2(hA)b_2
=\left(b_{0,1}+h^3A_{12}b_{2,2}/6,\;h^2b_{2,2}/2\right)^T.
\]

첫 성분은 약 ±1.66667e−27로 충분히 표현 가능하다. native fused/prefix는 약 1e−300을 내면서 `converged=true`, `invariant-subspace`, `error_estimate=0`을 보고했다. `phi_weight_underflows=1`은 기록된다. 핵심은 `h²b₂≈1e−326`을 먼저 0으로 만든 뒤 증폭을 잃었다는 점이다. 이후 정확한 불변 부분공간을 찾았더라도 이미 바뀐 문제에 대한 정확성이다. dense wrapper의 별도 큰 비정규 오차에 기대지 않아도 이 반례는 성립한다.

**수정 계약:** `δwₖ=hᵏbₖ−stored(wₖ)`를 표현 가능한 exponent/interval 형태로 보존하고 `Σφₖ(hA)δwₖ`의 bound를 출력 budget에 더해야 한다. bound가 없으면 원 입력 tolerance 달성을 승인하지 않는다. 우선 partial-loss 입력을 명시적으로 거절하는 후보가 가능하나 `A=0`의 무해한 소실도 거절하므로 보수적이다. nilpotent 구조에서 곱 순서를 바꾼 독립 후보는 양·음 h의 상대오차 6.53e−17이었다. 일반 matrix-free φ 해법을 구현한 결과는 아니다. 세부 source 위치와 exact oracle은 `arithmetic/REVIEW_KO.md`에 있다.

### P2 R3-STAT-01: raw 재검증이 측정 프로토콜 위반을 수락한다

ratio=1.3의 30 pairs, 명시된 6 sessions, matched A/A 대조군을 사용한다. 원자료를 assessment **이전**에 변조하여 빈 order, ABBA가 아닌 order, calibration과 다른 batch, 음수 warmup, 메모리상의 NaN warmup을 각각 넣었다. 다섯 경우 모두 `assess_paired_timing`이 Promote이고 `verify_against_raw`도 `Ok(Promote)`였다. 정상 JSON roundtrip 대조군도 통과했다.

원자료와 digest가 일치한다는 사실은 원자료가 허용된 프로토콜로 생성되었다는 사실을 보장하지 않는다. `paired_timing.rs:288–323,798–814`의 admission에 유한·비음수 warmup, 재계산된 integer batch, seed 기반 ABBA order와 pair 길이 검사가 필요하다. 독립 admission 후보는 정상 사례를 허용하고 위 다섯 사례를 모두 거절했다. **현재 CLI에서 실측 성능이 잘못 승격된 사례는 아니다.** CLI wall gate는 여전히 NotEvaluated이며 정상 producer가 malformed receipt를 생성한다고 주장하지 않는다.

## 3. 수학·코딩 연구 루프 A: Laguerre와 Chebyshev 공동 φ

사용자의 non-Krylov 요청을 Arnoldi/Lanczos 직교화가 없는 polynomial action으로 구체화했다. 다항식 `p(A)v` 자체는 대수적으로 Krylov span에 속하므로 “polynomial이면 Krylov 공간 밖”이라는 구분은 하지 않는다. 서로 다른 다섯 입력 벡터의 `Σₖ₌₀⁴ φₖ(hA)wₖ`를 동일 조건에서 비교했다. `hᵏbₖ` 가중치 변환은 이 연구의 입력 이전 단계이며 위 arithmetic finding과 별도로 처리해야 한다.

가정은 실수 대칭 `A≤0`, 알려진 spectrum `[−ρ,−λ]`, `h≥0`이다. `X=(A+(ρ+λ)I/2)/((ρ−λ)/2)`, `a=−h(ρ+λ)/2`, `b=h(ρ−λ)/2`에 대해 Bessel expansion과 φ 적분을 결합하면

\[
c_{n,k}=\frac{2-\delta_{n0}}{(k-1)!}\int_0^1e^{ua}I_n(ub)(1-u)^{k-1}du\quad(k\ge1)
\]

를 얻는다. `||Tₙ(X)||₂≤1`, Skellam Chernoff bound에서 `r=m+1`일 때 공동 truncation 상계는

\[
C_m(b)\sum_{k=0}^4\frac{\|w_k\|_2}{k!},\qquad
C_m(b)=2\exp\left(\sqrt{b^2+r^2}-b-r\operatorname{arsinh}(r/b)\right).
\]

Wolfram으로 최적점·볼록성·φ₁…φ₄ 적분식을 exact 검산했고, 독립 reviewer는 다른 dyadic 2×2 입력을 Decimal160 Taylor oracle로 검사했다. 유도는 **exact-arithmetic truncation**이며 coefficient quadrature, Bessel 함수, recurrence/summation rounding까지 합친 인증은 아니다.

| 24차 diagonal hρ | Laguerre degree | Chebyshev degree | Laguerre fused 오차 | Chebyshev fused 오차 |
|---:|---:|---:|---:|---:|
|0.1|7|6|1.48e−16|1.83e−15|
|1|12|10|4.25e−15|5.50e−15|
|10|38|21|3.62e−16|6.78e−15|
|100|245|56|5.82e−15|1.07e−14|

세 matrix family의 20 fused actions와 100 function-column 비교가 모두 관측 오차 목표 1e−10 이하, 최대 fused L2 오차 3.50e−14였다. degree m은 m block products 또는 5m vector-equivalent products다. **245/56≈4.38은 이 사례의 recurrence 단계 비이고 실측 speedup이 아니다.** 계수 setup, allocation, memory, fallback을 포함한 timing은 수행하지 않았다. 6개 zero/scalar 경계도 검사했고, 비정규 Jordan 입력은 명시적으로 거절했다.

권고는 대칭 dissipative domain에서 Chebyshev 공동 φ를 첫 Rust 후보로, Laguerre를 비교·재사용 후보로 유지하는 것이다. Leja는 일반 nonnormal domain의 추가 연구 후보이며 이번 구현은 NOT_RUN이다. 실제 backend 승격에는 검증 가능한 spectral enclosure, total-error 또는 정직한 EstimateOnly 상태, distinct-vector work accounting, 동일 정확도 실측 campaign이 필요하다. 유도·코드·원자료·문헌 확인 범위는 `polynomial/`에 있다.

## 4. 수학·코딩 연구 루프 B: homotopy 후보와 인증 계산의 병렬화

native RODAS 계수 bit pattern을 직접 추출했다. 첫 native target은 strict-lower 전제를 만족하지 않아 **실패를 보존했다**. α의 상삼각+대각 28개, L의 34개가 0이 아니며 최대 크기는 각각 5.58e−16, 3.77e−16이다. 따라서 H⁸=0을 native full-block에 그대로 적용할 수 없다. 이후 실험은 명시적인 `strict-lower-projection` target으로 분리했다. 작은 계수의 존재만으로 큰 production 오류를 재현했다고 주장하지 않는다.

1/2차원 quadratic stage target과 실제 lower coefficient에서 후보 residual을 interval로 재평가하고, exact Fraction inverse witness를 위쪽으로 감싸 성분별 오차를 계산했다. 각 단계의 `Eᵢ≥|K̂ᵢ−Kᵢ|`는 이전 단계 오차와 quadratic remainder의 Lipschitz 항을 전파한다. output `bᵀK`, embedded `b̃ᵀK`의 rounding까지 따로 합쳤다. 24 q1/q2 행 모두 exact stage/output/embedded/residual/WRMS checks를 통과했다. output WRMS≤0.1은 12/24, output+embedded proxy≤1은 8/24였다. **embedded proxy가 ODE truncation error의 엄밀 상계라는 정리는 별도로 필요하다.**

인증 자체의 stage 직렬성을 줄이기 위해 exact root를 보지 않고 D=1e−4를 고정하고 비음수 strict-lower H를 구성했다.

\[
E=(I-H)^{-1}a=(I+H)(I+H^2)(I+H^4)a,\qquad H^8=0.
\]

3 doubling levels 뒤 `|α|E≤D`를 outward 계산으로 확인한다. 22/24에서 closure와 exact enclosure가 통과했고 2개는 거절했다. Python 구현은 dense 연산을 **직렬 실행**했다. 3-level dependency depth는 가능한 병렬 알고리즘의 성질이고 실측 가속은 아니다. 큰 문제에서 dense inverse witness와 block fill 비용이 이점을 없앨 수 있으므로 diagonal/banded/block 구조부터 확장하는 것이 현실적이다.

현재 q2의 8번째 W batch는 상태에 반영하지 않는 q=0 diagnostic이다. 동일 후보에 대한 검증된 certificate가 이 역할을 대체할 때 7번째 batch 뒤 결정하는 설계를 시험할 수 있다. 충분한 worker와 고정 W 비용이라는 단순 모형에서 q1 수락 확률 p₁, 최종 fallback 확률 p_f, 인증비 c₁,c₂, inverse witness amortization a, 기타비 r이면

\[
T/t_W=7-p_1+8p_f+c_1+(1-p_1)c_2+a+r.
\]

8회의 순차 단일-vector W solve baseline보다 빠르려면 `1+p₁>8p_f+c₁+(1−p₁)c₂+a+r`가 필요하다. 이는 예측식이며 현재 fixture 비율을 운영 확률로 사용하지 않는다. q1의 6 batches도 총 48 vector solves이므로 8-vector sequential 대비 work 증가를 숨기면 안 된다. 1/2/4/8 worker, 실패 시도와 fallback, witness, RHS/JVP를 모두 포함한 실제 측정이 최종 gate다.

## 5. 통계 연구: Monte Carlo 불확실성을 gate에 전달하기

고정 empirical bootstrap law와 고정 threshold에 대해 `X=1(T*<log1.15)`를 두면 conditional p의 estimate는 K/B다. Hoeffding으로 `ε=sqrt(log(2/δ)/(2B))`, `[L,U]=[K/B−ε,K/B+ε]∩[0,1]`을 구성한다. U<0.025면 Promote, L>0.975면 Block, 나머지는 Inconclusive로 둔다. B=10000, δ=0.01이면 ε≈0.0162762다.

46,656개 exact bootstrap outcomes와 기존 B=10000 fixture의 percentile endpoints를 비교했고, 일정 ratio 및 threshold 근접 사례에서 보수적 gate를 실행했다. 이것은 **고정 empirical law에 조건부인 Monte Carlo 오류**다. 모집단 coverage, ideal iid를 구현한다는 PRNG 증명, 관측 후 B를 늘리는 optional stopping 권한을 주지 않는다. 먼저 preview diagnostic으로 도입하고, 실제 성능 estimand와 session×case design은 별도 사전 고정 campaign으로 검증할 것을 권한다.

## 6. 실행 신뢰도와 감사 자체의 제한

Rust 1.94.1과 repo의 locked dependency를 현재 환경에서 다시 준비했다. 118/118 locked registry package checksum을 확인했다. Python은 3.12.14, NumPy 2.3.5, SciPy 1.17.0이다. native focused probes는 time 29행, statistics 10행, arithmetic 529행·legacy 104행을 실행했다. 이 행 수는 독립 unit-test 수가 아니며 workspace suite와 합산하지 않는다.

초기 rustc library 추출의 잘린 파일과 공유 build cache의 0-byte objects 때문에 build가 실패한 기록이 있다. 해당 bytes/cache를 복구하고 incremental을 끈 뒤 native 실행을 얻었다. 전체 campaign에는 codegen-units=1의 환경 완화 설정을 기록했다. 이 환경 실패를 numerical bug나 test assertion 실패로 분류하지 않는다. 전체 build의 최종 완료 event가 누락되어 cached 회수를 시도했지만 제한 시간 내 완료되지 않았다. 따라서 전체 build 성공을 주장하지 않고, 존재하는 native harness를 fingerprint와 실제 실행으로 별도 확인했다. 변경 7개 target의 37개 테스트는 모두 통과했다. 전체 682개 이름 있는 test 중 625 PASS, 0 assertion FAIL, 26 timeout 미확정, 28 시간 상한 미실행, 3 ignored 미실행이다. 전체 workspace campaign 결과와 bounded timeouts는 `RUNTIME_SUMMARY_KO.md` 및 `evidence/runtime/`에 있다. doctest, 실측 성능 campaign, 모든 feature 조합의 개별 matrix는 별도 미수행 범위다.

수학·코딩 GPT6 Astra v4.0.0 하네스는 사용자가 이전에 선택한 계약의 연속으로 적용했다. 이는 실제 runtime model identity에 대한 주장이 아니다. 생성 담당과 독립 decision reviewer를 분리했고 독립 exact oracle 및 선택 재실행을 남겼다. `decision/INDEPENDENT_DECISION.json`이 최종 주장별 결정의 근거다. 단일 bounded closeout 뒤 검토를 종료한다.

**감사 프로세스 위반을 공개한다.** `PREREGISTRATION.md`는 probe 전에 작성했으나 실행 전에 commit하지 않았고 모든 exact command와 holdout을 미리 고정하지 않았다. 따라서 repo의 사전등록 규칙을 충족한 confirmatory run으로 분류하지 않는다. 새 연구 노드는 탐색적 외부 리뷰이며 ledger verdict는 INCONCLUSIVE다. 과거를 사전등록으로 재구성하거나 exemption/검증기를 변경하지 않았다. 증명된 제한된 명제와 실제 반례의 증거를 버릴 이유는 없지만, 향후 생산·성능 승격은 먼저 commit한 새 protocol과 독립 holdout을 거쳐야 한다.

이미 알려진 dense mixed-range label, caller-supplied session identity, 0→10의 h=.01에서 나타나는 1001번째 micro-step, 일부 sealed research driver의 구 clock policy, directed budget lower bound의 부재는 한계로 보존했다. 기본 G3 campaign의 오염, production homotopy 동치, 일반 비정규 안정성, 실측 speedup을 새로 입증했다고 주장하지 않는다.

## 7. 다음 개발 순서와 수락 기준

상세 실행 계약은 `NEXT_DEVELOPMENT_DAG.json`에 있다. 각 node는 dependency, target, 구현 단계, acceptance, 실패 상태와 산출물을 포함한다. 다음 순서를 권한다.

1. **즉시 correctness:** TIME-01/02, ARITH-01, STAT-DEV-01을 독립 변경으로 구현한다. 기존 일곱 closure와 새 반례 모두 회귀에 넣고, 오류를 typed failure로 닫는 최소 구현부터 수락한다. tolerance를 늘리거나 underflow counter만 기록하여 성공을 유지하는 수정은 실패다.
2. **공유 계약:** 실제 h를 BDF/history까지 결속하고, weight-loss bound와 dense oracle status를 consumer까지 보낸다. 이 층이 다음 backend의 correctness 경계가 된다. Unknown 상태를 threshold 0이나 성공으로 바꾸지 않는다.
3. **제한된 연구 backend:** Chebyshev joint φ Rust prototype, outward homotopy certificate의 작은 구조부터 구현한다. full-block/strict-lower target identity를 먼저 고정하고, certificate를 통과하지 못하면 모든 비용을 기록한 순차 fallback으로 돌아간다.
4. **측정 권한:** validated raw protocol을 읽는 실제 paired runner와 OS/campaign provenance를 붙인다. synthetic clocks는 regression용이다. 모집단·case weights·session independence·A/A·실패 처리와 B를 먼저 commit한다.
5. **최종 비교:** 동일 output schedule과 accuracy에서 baseline, Laguerre, Chebyshev, 제한된 homotopy를 측정한다. setup/amortization, coefficient generation, allocation, rejected attempts, fallback, workers를 모두 포함한다. 수치 우수성과 speed gate를 분리한다.

개발이 완료됐다는 판정은 다음 commit에서 새 소스에 대해 내릴 수 있다. 이번 commit은 코드 수정의 대체물이 아니라 재현 가능한 결함 증거, 제한된 연구 결과와 실행 가능한 개발 계약을 제공한다.

## 8. 읽기·재현 안내

- 빠른 기계형 진입점: `REPORT_STATUS.json`, `FINDINGS.json`, `RESEARCH_CLAIMS.json`, `NEXT_DEVELOPMENT_DAG.json`.
- 전문 부록: `time/REVIEW_KO.md`, `arithmetic/REVIEW_KO.md`, `statistics/REVIEW_KO.md`, `polynomial/RESEARCH_KO.md`, `homotopy/HOMOTOPY_RESEARCH_KO.md`.
- 실행: `REPRODUCE.md`, `reproduce.py`. 새 output directory를 사용하며 게시된 원자료를 덮어쓰지 않는다.
- 무결성·일관성: `python3 validate_review_package.py`. 이 검사는 suite 실행이나 수학 증명 대체가 아니다.
- 독립 결정과 최초 실패는 보존한다. SHA-256 manifest는 package content를 고정하고, publication commit은 기존 branch의 parent chain에 연결한다.
