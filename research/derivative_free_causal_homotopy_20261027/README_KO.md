# VigilODE: 도함수 없는 causal/homotopy 연구와 증명에서 유도한 상수

기준은 [PR90](https://github.com/cosmosapjw-quantum/vigilode/pull/90)의
`68a28357d960c4c2743c299770f0a33ef63245b2`다. 이전 보고서를 비판적으로
읽고, adapter와 개별 적분 모드를 분리하여 이론·코딩 연구 루프를 실행했다.
사용자가 추가로 요구한 **임의의 hyperparameter 대신 수학적으로 유도한
상수**는 별도 확장 프로토콜로 반영했다. 새로운 수치 결과를 기존 사전
고정 실험의 결과로 바꿔 쓰지 않았다.

이번 결과는 다음 네 경로의 실제 코드와 실험이다.

| 경로 | 구현한 개선 | 도함수·행렬의 실제 범위 |
|---|---|---|
| Causal Picard | 같은 projected input의 RHS 재평가 제거, 같은 RHS prefix의 삼각 풀이 재사용 | 기존 frozen JVP와 작은 projected LU를 사용한다. 완전 derivative-free라고 부르지 않는다. |
| 정확한 구조적 raw-U | 두 스칼라 tableau를 계산하고 모든 상태 성분을 병렬 복원 | 선언된 물리 모델에서 RHS 이외 derivative callback·상태 행렬·Newton이 없다. |
| RHS homotopy/RK | 실제 잔차로 검증한 depth-one extrapolation, accepted stage 재사용, 증명으로 정한 정확도 계획 | direct map 평가·벡터 연산만 사용한다. Cubic consumer는 outward residual 검증까지 수행한다. |
| Adapter | 진입 전 checkpoint proposal 재사용, 같은 구간의 비용을 checked integer로 비교 | RHS-only 경로를 기준으로 실제 비용 우세가 확인된 pilot을 선택한다. JVP는 supplied provider만 사용한다. |

실행한 병렬 agent, R1–R10/C1–C10 기록은 `RESEARCH_LOOPS.md`, 실제 명령은
`REPRODUCE.md`, 각 측정의 소스·바이너리 해시는 execution receipt에 있다.
과거의 완료된 V6/V8 물리 캠페인을 재실행하거나 그 완료 판정을 변경하지 않았다.

## 1. 이전 PR에서 가져온 아이디어와 교정한 해석

PR65의 common-W 실패는 동일 순차 구조를 반복하면 풀이 수만 늘 수 있다는
구체적인 반례다. 따라서 이번 causal 개선은 반복 횟수를 줄였다고 주장하지
않고, 바뀌지 않은 입력과 이미 확정된 prefix의 실제 계산을 제거했다.
PR74의 coupled raw-U 목표식·stage 병렬화·계수 convention을 그대로 사용했고,
PR75의 residual-first와 speculative work 전량 과금 원칙을 유지했다.
PR77의 구조적 물리 연산과 collective/transverse 분해는 원래 목표식과
동일함을 증명할 수 있는 sealed affine 모델로 옮겼다.

PR61/62 및 후속 재감사에서 강조한 estimate/bound, linearized correction/
실제 오차의 구분을 유지했다. 특히 projected residual만 작다고 full-state
오차나 시간 적분 정확도가 증명된 것으로 취급하지 않았다. PR90의 비용
하한 중단·checkpoint 보존을 adapter 진입 경로로 이어 갔다. 관련 PR 원문
snapshot은 `evidence/PR_SOURCE_SNAPSHOTS.json`에 보존했다.

기초 문헌으로 van der Houwen–Sommeijer의 parallel iterated RK,
Tranquilli–Sandu의 Rosenbrock–Krylov, Pollock–Rebholz와 Walker–Ni의
Anderson 연구를 확인했다. 표준 Anderson 식이나 기존 causal 정리를 새로운
발견으로 주장하지 않는다. 이번 기여는 목표식·오차·비용 계약을 실제 코드와
분별적인 실험에 연결한 구성이다.

## 2. 임의 상수를 없앤 범위

정확도 요구, 물리 모델 계수, 자원 한도는 문제의 입력이다. 증명은 그
입력에서 허용 step과 residual을 결정한다. 반면 과거 pressure/backoff 같은
정책 값을 보편적인 최적 상수로 증명한 것은 아니다.

| 요소 | 사용한 값 또는 식 | 근거와 범위 |
|---|---|---|
| 가속 후보 잔차 수용 | eta=1 | 실제 후보 잔차가 이미 계산한 Picard 후보보다 크지 않으면 수축 잔차 부등식을 계승한다. |
| Cubic 구간 Lipschitz | L=k+3gY² | 알려진 다항식의 도함수 극값에서 정확히 도출한다. |
| Cubic 역오차 계수 | D=1+hk/2 | 전역 강한 단조성으로 root error≤residual/D. |
| 유한 step 절단오차 | C=L² Phi(Y)/8 | interpolation/quadrature remainder에서 유도한 상계다. 최적 sharp 상수라는 주장은 하지 않는다. |
| 전체 오차 예산 분배 | (epsilon−E_trunc)/N | 임의 epsilon/2 분할 없이 증명된 절단오차를 제외한 실제 잔여분을 사용한다. |
| Structural 잔차 허용치 | delta=epsilon_alg/C_N | 모든 step/stage/component residual에서 최종 출력으로 가는 정확한 induced norm. |
| 같은 구간 entry 선택 | I<E | 선언한 정수 작업량 목적함수의 strict 감소 조건. 동률은 incumbent 유지. |
| Callback 중단 cap | E−1 | 과금한 callback 하한 L이 E 이상이면 더 이상 strict win 불가능. |

`ResidualDominance`에는 임의 gain margin, coefficient magnitude cap,
느린 수렴비 임계값, rejection-count cap이 적용되지 않는다. 유한성·정의역
검사와 실제 잔차 평가는 유지하며, 후보의 추가 비용도 모두 센다.
이 수렴 조건이 자동으로 작업량 우세를 의미하지는 않는다.

### Cubic midpoint: 잔차에서 전체 물리 오차까지

\[
\Phi(y)=ky+gy^3,\quad k>0,\quad g\ge0,\quad y(0)=Y\ge0.
\]

Stage increment K의 목표식은

\[
F(K)=K+h\Phi(y+K/2)=0.
\]

모든 실수에서 F'(K)≥D=1+hk/2이므로
\(|K-K_*|\le |F(K)|/D\)이다. 정확한 midpoint step
\(\Psi_h(y)=2m-y\), \(m+(h/2)\Phi(m)=y\)는

\[
\Psi_h'(y)=\frac{1-(h/2)\Phi'(m)}{1+(h/2)\Phi'(m)},\qquad
|\Psi_h'(y)|\le1
\]

을 만족한다. 따라서 단계별 algebraic error를 전체 궤적으로 더해 전달할
수 있다. 정확해는 [0,Y]에 머물고, 두 양의 quadrature defect의 차이에
대해

\[
2\Phi'(Y)^2-\Phi''(Y)\Phi(Y)
=2k^2+6kgY^2+12g^2Y^4\ge0
\]

를 사용하면 유한 step residual≤Ch³, C=L²Phi(Y)/8을 얻는다.
물리적 local truncation error는 Ch³/D 이하이다.

코드는 표현 가능한 dyadic 격자 중

\[
E_{\rm trunc}=NCh^3/D<\epsilon,\qquad
q_B=\frac h2\{k+3g(Y+\epsilon)^2\}<1
\]

을 만족하는 가장 작은 N을 선택한다. Dyadic이라는 조건은 binary64 시간
격자의 정확한 표현을 위한 명시적 제약이고 모든 정수 N에서의 최적성을
주장하지 않는다. 나머지 예산으로

\[
\alpha_{\rm step}=(\epsilon-E_{\rm trunc})/N,\qquad
\tau_F=D\alpha_{\rm step}
\]

를 정한다. 원래 F의 outward interval, 실제 상태 덧셈의 반올림 변위,
시간 label의 변위를 검증하여 각 반환점의 물리 오차 상한을 만든다.
표현 정밀도의 바닥보다 작은 요구는 성공으로 위장하지 않고 거절한다.
Directed binary64/FMA·gradual underflow·compiler association 전제는
`homotopy/EXACT_CONSTANTS.md`에 명시했다.

### Structural raw-U: 정확한 누적 증폭 상수

물리 모델은 \(f(y)=b-dy+\kappa\bar y\mathbf1\), d≥κ≥0이다.
P=11ᵀ/n, Q=I−P에 대해 J=−(d−κ)P−dQ이므로 원래 8-stage 목표가
두 스칼라 점화식으로 분해된다. Q 성분을 버리는 것이 아니므로 임의의
초기 상태와 forcing의 모든 transverse 성분이 남는다.

원래 residual에 대한 두 스칼라 adjoint weights를 w_t,w_c,
두 channel의 discrete stability 값을 R_t,R_c라 하고

\[
\nu_n(t,c)=\left|t+\frac{c-t}{n}\right|
+(n-1)\left|\frac{c-t}{n}\right|
\]

로 두면 정확한 누적 norm은

\[
C_N=\sum_{j=0}^{N-1}\sum_i
\nu_n(R_t^j w_{t,i},R_c^j w_{c,i})
\]

이다. 각 residual 성분을 한 출력 행의 부호에 맞추면 등호가 성립하므로
sharp하다. 수축 가정 없이 delta=epsilon_alg/C_N을 사용할 수 있다.
실제 n=17,d=20,κ=19,h=1/64에서는 C_64≈947.2963123944904,
epsilon_alg=1e-8에서 delta≈1.0556359049601808e-11이다.
이는 이산 목표식의 algebraic error에 관한 정리다. 현재 structural 구현의
f64 상수와 sampled residual은 outward 물리 인증이 아니며 rounding과
시간 절단오차는 별도 항이다.

### Causal DAG: 전역 수축 없이 가능한 유한 오차 전달

Strictly lower triangular 비음수 block Lipschitz 행렬 L에 대해

\[
e\le (I-L)^{-1}\rho
=\sum_{j=0}^{s-1}L^j\rho.
\]

L^s=0이므로 ||L||<1이 필요하지 않다. 다만 큰 비정규 증폭은 남는다.
정확 검산 예에서 ||L||∞=13이며 첫 stage residual1e-6이 마지막 stage
error119e-6으로 전달된다. 작은 잔차를 곧바로 작은 출력 오차로 읽는
해석은 이 상수를 빠뜨린 것이다.

## 3. 실제 성능과 정확도

### Causal cache

108/108 후보 셀에서 원래 Picard와 재사용 경로의 stage/output/error bits가
같았다. 강한 비선형 단일 목표에서 RHS58→37, block solve56→35로 줄었다.
같은 목표의 Forward 경로는 RHS16, solve8로 더 저렴했다. 재사용 경로가
항상 최저 작업량이라고 주장하지 않는다.

정확 Riccati 해로 검증한 전체 궤적에서는 RHS1868→1702(**8.89%**),
block solve1456→1290(**11.40%**)였고 JVP726은 같았다.
33개 공통 관측 상태와 모든 시도의 time/step/decision bits가 같았다.
최대 물리 오차는2.68565e-11, 고정 cap은3e-9이다.

별도 root-attempt timing의 큰 4-worker 셀 중앙값은1.395배였다.
그러나 그 h=.1 후보의 embedded WRMS가 약4.62e4이므로 실제 외부
적분기는 거절한다. 이 값을 accepted-trajectory 가속률로 보고하지 않는다.
작은 n96,2-worker 셀은 중앙값.873배로 느려졌다.

### 완전 derivative-free structural 궤적

비교 기준도 같은 물리적 resolvent를 사용하는 강한 derivative-free 순차
구현이다. 두 경로 모두 원래 공간에서 8-stage residual을 검사한다.
64 step 전체의 RHS는1024→576(**43.75%**)이고 shifted resolvent는512→0이다.
n2048의 causal component 누적은14680064→7340032이며 lift1048576이 추가된다.

| 차원 / worker | 매 step 새 plan의 paired 중앙값 | 5개 block의 관측 범위 |
|---|---:|---:|
| 64 / 1 | 1.705배 | 1.571–1.840 |
| 2048 / 1 | 1.883배 | 1.472–1.943 |
| 2048 / 2 | 1.571배 | .866–2.218 |

비율은 같은 worker 수의 sequential time/quotient time이다. 범위는
신뢰구간이 아니다. n2048에서 최대 공통 grid 오차2.39760e-10,
old/new 차이8.88178e-16으로 동일한 cap을 통과했다. 할당·plan 생성·
전체 residual 검사를 모두 시간에 포함했다. 공유 tableau loader의 고정
8×8 역행렬 계산은 프로세스당 한 번 수행되며, 사전 diagnostics가 cache를
초기화했다. **상태 행렬/매 step factorization이0**이라는 범위와 구분했다.

유도된 residual cap을 사용하는 별도 n17 궤적도 성공했다. RHS1024→576,
최대 물리 오차2.42381e-10, old/new 차이5.55112e-16이다. 기존 타이밍을
새 threshold로 다시 측정했다고 주장하지 않는다.

### RHS homotopy와 정확도 계획

거의 비수축인 scalar linear stage에서 Picard2819회가 screened4회로
줄었다. 이는 같은 거친 RK stage의 root 비용이지 물리 오차를 맞춘 전체
적분 가속률은 아니다. Noncontractive Picard의 iteration-limit 실패도
보존했다. 명시적 RK4 목표에서는 Picard20회, screened36회, 직접 forward4회로
가속 후보의 추가 평가가 손해였다. 쉬운 비선형 문제의 중간 continuation
station도 직접 풀이12회를28회로 늘렸다.

초기 고정-grid nonlinear midpoint 실험은2차 수렴을 유지하며 64-step
RHS424→296(**30.19%**)였다. Warm Picard377, zero-seed Screened384라는
ablation을 함께 보존했으므로 합친 개선을 전부 Anderson에 귀속하지 않는다.

추가된 proof-budget 소비자는 k=1,g=1/2,Y=T=1이므로 C=**75/64**다.
동일 계획에서 zero/warm과 Picard/ResidualDominance의 네 arm을 비교했다.

| 요구 물리 오차 | 유도 N | 유도 stage residual | Zero/Picard RHS+interval | Warm/Dominance RHS+interval |
|---:|---:|---:|---:|---:|
| 1e-3 | 64 | 1.12767219543e-5 | 192+64=256 | 134+64=198 |
| 1e-4 | 128 | 2.25508213043e-7 | 430+128=558 | 334+128=462 |
| 1e-5 | 512 | 1.08191743493e-8 | 1580+512=2092 | 1147+512=1659 |

12/12 arm이 모든 반환점에서 outward bound와 요구 오차를 만족했다.
가장 엄격한 warm/Dominance의 최대 실제 오차7.08647e-7, 최종 bound6.09514e-6이다.
전체 polynomial 평가 수 감소는22.66%,17.20%,20.70%다. 같은 seed의 두
정책은 RHS 수가 같으므로 여기의 절감은 accepted stage 초기값 재사용
효과다. Ordinary RHS와 interval 평가의 단위 시간 비용이 같다는 가정이나
wall-clock 가속 주장은 하지 않는다. 증명 상계가 보수적이면 더 많은
시간 step을 요구할 수 있으며, 이 계획이 모든 adaptive 방법보다 적은
step을 쓴다는 결과도 아니다.

### Adapter: cold seed와 임의 20% margin을 각각 분리

처음에는 explicit session이 이미 h≈.003662를 제안하는 checkpoint에서도
implicit pilot이 전역 h0=.0001로 다시 시작했다. 기록된 **batch 시작 전**
proposal을 사용하도록 수정하자 같은 구간의 pilot이160work로 완료됐다.
하지만 explicit192work 대비5/6은 기존.8 기준을 통과하지 못했다.

별도 진단 후 exact strict entry를 구현했다. 목적함수
W=N_RHS+N_JVP+N_ft+nN_J+N_solve에 대해 실제 완료된 같은 구간의
W_pilot<W_explicit를 checked u128로 판정한다. 동률·2^53 이상1단위 차이·
overflow·같은 endpoint qualification을 시험했다.

| rate | RHS-only 전체 work | 기존 selective | seed만 교정 | seed+정수 strict |
|---:|---:|---:|---:|---:|
| 1 | 79 | 79 | 79 | 79 |
| 1000 | 2377 | 2958 | 2890 | 1691 |
| 10000 | 19261 | 20190 | 20094 | 1629 |

어려운 두 셀의 전체 비용 감소는**28.86%,91.54%**다. 초기에 실패한
241work pilot과 return probe도 포함했다. 쉬운 셀은 JVP0이다.
12/12 arm이 동일한1e-6 정확도 cap을 만족했고 native counter와 callback
atomics가 일치했다. Strict arm 최대 오차는 각각3.37183e-9,1.06542e-9,
1.31464e-9다. Strict path는 RHS-only보다 오차가 클 수 있지만 같은
계약을 통과하며, 이 비교는 실제 초 단위 시간이 아니라 선언한 work다.

새 두 flag는 기본 false다. Strict entry는 pilot budget>0인 competitive
entry에 적용된다. 기존 budget0 direct entry와 pressure/backoff/return
heuristic을 이 연구에서 모두 대체하지 않았다. 관측한 구간의 최선 선택이
미래 구간의 전역 최적성까지 증명하지 않는다.

## 4. 이론 연구와 코드 최적화를 구분한 판정

**증명과 이식이 끝난 부분**은 exact cache equivalence, causal finite residual
transport, sealed affine quotient, monotone cubic root bound, midpoint
비팽창성·유한-step 정확도 계획, actual-residual dominance, structural
누적 sharp norm, 같은 구간 정수 비용 판정이다.

**더 필요한 이론**은 일반 비선형 모델의 approximate closure와 closure
defect 전달, frozen derivative를 물리 carrier로 바꾸었을 때의 실제
고차 order conditions, 일반 noncontractive map의 branch/domain 제어,
비정규 증폭을 반영한 output error, 미래 구간의 비용 및 exploration 경계다.
이 조건들이 없으면 특정 구조 모델의 성공을 범용 solver 정리로 확장할
수 없다. 현재 구성을 반증하는 전역 no-go를 주장하지도 않는다.

**구현·측정으로 개선할 부분**은 full-space residual screen의 O(s²n)
비용, 작은 상태의 dispatch overhead, 비용 모델의 실제 시간 가중치,
반복 adapter 탐색, structured API의 실패 시 partial-work 반환이다.
현재 structural CoreResult는 성공한 작업량만 반환하므로 adaptive
fallback에 붙이기 전 실패 비용 계약이 필요하다. 이 제한을 성공한
standalone 궤적의 성능 측정과 혼동하지 않았다.

## 5. 검증과 증거의 신뢰도

선택된 로그에서 **서로 다른 native 계약59개 PASS**, 변경한 두 crate의
**all-target locked/offline compile PASS**다. exact Fraction 검산은
기존32개 quotient cell/832성분 등식과 새12개 structural 상수 cell,
900개 root secant,900개 비팽창 identity,180개 bracket 조건,
36개 truncation polynomial,25개 정규화 AA,6개 dyadic budget 검사를 포함한다.
검산은 일반 증명을 대신하지 않고 대수·인덱스·계수 convention을 독립적으로
확인한다. Native full-state·정확해·interval 검증도 별도로 수행했다.

마지막 감사에서 초기 debug 취소 receipt가 최종 로그와 다름을 발견했다.
짧은 취소 로그는 완성된 로그의 byte prefix였고, 완성 로그에는6+6테스트
성공이 있었다. 원 receipt와 두 로그를 지우지 않고 correction receipt로
명시적으로 교정했다. Measurement-profile 실행을 대표 증거로 삼고 같은
테스트를 중복 계산하지 않았다.

보고서는 실제 `revtex4-2`로 작성했다. 전체 증명, 음성 대조군, 과금 범위,
floating-point 전제와 추가 과제를 포함한다. `report/`의 그림은 모두
보존한 native 측정에서 생성했다. `MANIFEST.json`과 원자료 archive의 해시로
코드·결과·보고서의 대응을 확인할 수 있다.

## 6. 주요 파일

- `report/derivative_free_causal_homotopy.tex`: 통합 REVTeX 원문.
- `theory/THEORY.md`, `theory/theorems.tex`: 정리·가정·증명.
- `causal/REPORT_KO.md`: causal work와 bit parity, timing scope.
- `homotopy/HOMOTOPY_LOOP.md`, `homotopy/EXACT_CONSTANTS.md`: root 소비자와 정확도 계획.
- `adapter/EXACT_ENTRY_WORK_POLICY.md`: 정수 비용 정리, 실제 성능, 남은 heuristic.
- `RESULTS.json`, `evidence/VALIDATION_SUMMARY.json`: 집계 및 중복 없는 검증 목록.
- `REPRODUCE.md`, `RESEARCH_LOOPS.md`: 재현 절차와 실행한 full loop.

문헌 원문:
[parallel iterated RK](https://ir.cwi.nl/pub/1589/1589D.pdf),
[Rosenbrock–Krylov](https://arxiv.org/abs/1305.5481),
[Anderson convergence](https://doi.org/10.1093/imanum/draa095).
