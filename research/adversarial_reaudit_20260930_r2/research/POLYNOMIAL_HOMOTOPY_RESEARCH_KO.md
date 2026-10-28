# 비-Arnoldi 다항식 및 homotopy 병렬화: 2차 연구 루프

검토 대상: VigilODE commit `7708ef90554fc3986478d4602de6a01c7266b14f`. 연구일: 2026-09-30. 이 문서는 앞선 감사의 `polynomial_research.md`, `homotopy_parallel.md`를 이어 실제 미완료 계산 두 가지를 수행했다. 생산 소스는 변경하지 않았다. GPT-6 Astra v4.0.0 연구·코딩 하네스의 원문과 상태 템플릿을 읽었으며, 템플릿의 `NOT_RUN`을 이전 연구 실행으로 취급하지 않았다.

**결론:** Laguerre의 동일 24차 입력에서 exp와 φ1…φ4를 한 기저로 계산했고, 앞선 실패를 같은 입력으로 재현한 뒤 스케일 제한으로 정확도를 회복했다. 그러나 Chebyshev의 exp 기준 대비 JVP 비용은 크다. Homotopy에는 비선형 stage enclosure를 순차적으로 구성하는 오차 상계를 실제 구현했다. 비정규 문제에서는 단일 norm 상계가 사실상 쓸 수 없을 만큼 보수적이었지만, 성분별 구조를 보존하면 같은 q2 후보를 합리적인 상계로 판정할 수 있었다. 이는 후속 구현 방향을 바꾸는 실질적 결과다. 생산 인증·일반적 속도 우위·stiff-uniform 차수는 여전히 미확정이다.

## 1. 실행 계약과 전회 대비 진전

- 질문 P: 전회에 실패한 **동일한 24차 행렬·입력·h**에서 Laguerre 스케일 제한이 해결하는가? φ1에서 φ1…φ4의 공동 action으로 확장되는가?
- 질문 H: observed correction ratio 대신 사용할 **구성 가능한 nonlinear output bound**가 있는가? 실제 비정규 구조를 버린 norm 상계와 성분별 상계의 차이는 어느 정도인가?
- 보존: RODAS fixture/tableau, sequential stage target, 입력과 tolerance; 기존 생산 분기를 수정하지 않음.
- 수렴 기준: 같은 입력 비교, 독립 scalar oracle, 정확 유리수 예제, 현 fixture 수치 검산, 비용과 미실행 항목을 담은 개발 DAG. 계산 목표를 충족하여 추가 광범위 sweep은 하지 않았다.
- 실행: Python 3.12.14, NumPy 2.3.5, SciPy 1.17.0; `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1`. 실제 두 프로그램 exit 0. `mpmath` import 시도는 `ModuleNotFoundError`였고, 필요한 scalar oracle은 표준 라이브러리 Decimal 110자리로 구현했다. Python/NumPy 실행 경로는 정상이다.
- 근거: `polynomial_joint_results.json`, `homotopy_majorant_results.json`; 명령·소스 해시는 `RESEARCH_EXECUTION_RECEIPT.json`. 주요 source와 논문의 전회 확인 범위는 원 보고서에 보존되어 있다.

## 2. Laguerre 공동 exp–φ action

### 정의와 조건

고정 선형 연산자 A는 시간 역수, h>0은 시간이다. β>0도 시간 역수로 두고 B=−A/β, a=hβ, q=a/(1+a)로 정하면 모두 무차원이다. 다음 truncation 보장은 **A=Aᵀ≤0, spec(B)⊂[0,L]**에 한정한다. arbitrary matrix-free callback에서 eigenvalue 추측만으로 이 조건을 선언하면 안 된다.

Laguerre 생성함수와 φ 적분 정의로

$$e^{hA}v=(1-q)\sum_{n\ge0}q^nL_n(B)v,$$
$$\varphi_k(hA)v=\sum_{n\ge0}c_{n,k}(a)L_n(B)v,$$
$$c_{n,k}(a)=\frac1{(k-1)!}\int_0^1\frac{(au)^n}{(1+au)^{n+1}}(1-u)^{k-1}\,du,\qquad k\ge1.$$

계수는 비음수다. 한 번의 3항 점화식

$$v_0=v,\quad v_1=(I-B)v,\quad
v_{n+1}=\frac{((2n+1)I-B)v_n-nv_{n-1}}{n+1}$$

에서 서로 다른 계수로 exp, φ1, φ2, φ3, φ4를 누적한다. degree m에 **m JVP**, 두 recurrence 벡터와 다섯 output accumulator가 필요하다. 다섯 개의 서로 다른 입력 벡터를 한 번의 JVP로 처리했다는 뜻은 아니다. 현재 실험은 동일 v에 대한 다섯 함수다. fused Σφk(A)vk는 block action 또는 별도 입력 recurrence 계약이 필요하다.

### 새 truncation 유도

DLMF의 α=0 상계 |Ln(x)|≤e^(x/2)와 spectral theorem을 사용한다. φk의 각 u에서 기하급수 tail을 합하면

$$\left\|\varphi_k(hA)v-\sum_{n=0}^{m}c_{n,k}L_n(B)v\right\|_2
\le \frac{e^{L/2}\|v\|_2}{(k-1)!}\int_0^1(1-u)^{k-1}
\left(\frac{au}{1+au}\right)^{m+1}\,du
\le \frac{e^{L/2}q^{m+1}}{k!}\|v\|_2.$$

따라서 exp용 tail budget은 k=1…4에도 충분하다. 이 결론은 **exact arithmetic의 truncation만** 제어한다. 128/256점 Gauss–Legendre 계수 적분과 Kahan summation을 사용했지만, 이것으로 quadrature 또는 recurrence roundoff 상계를 증명한 것은 아니다. h=0 또는 A=0에서 φk(0)=1/k!를 직접 반환하는 생산 API 분기는 이번 planner의 검증 범위 밖이며 후속 acceptance에 포함했다.

### 같은 입력에서 확인한 결과

A=−diag(geomspace(0.1,100,24)), vj=cos(0.37j)+0.3를 L2=1로 정규화했다. 입력 byte identity는 JSON에 있다. 모든 스케일 비교에서 동일 A,v,h를 유지했다. scalar reference는 Decimal 110자리 exp 및 φ recurrence이며, final vector는 binary64로 변환했다. φ 계산은 현재 Laguerre recurrence 또는 quadrature를 공유하지 않는다.

| hρ | L 제한 | 선택 L | degree/JVP | exp L2 오차 | φ1 L2 오차 | φ2 L2 오차 | φ3 L2 오차 | φ4 L2 오차 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.1 | 16 | 16 | 6 | 1.71e−14 | 2.16e−15 | 2.41e−16 | 4.07e−17 | 2.24e−17 |
| 1 | 16 | 16 | 10 | 1.04e−12 | 9.18e−14 | 7.39e−15 | 5.45e−16 | 4.33e−17 |
| 10 | 16 | 16 | 32 | 2.87e−13 | 1.36e−14 | 5.86e−16 | 5.20e−17 | 2.31e−17 |
| 100 | 64 | 64 | 111 | **1.89e−8** | **1.02e−8** | 8.43e−11 | 5.23e−12 | 1.57e−11 |
| 100 | 16 | 16 | 209 | **2.27e−13** | **6.62e−15** | 8.12e−16 | 2.23e−16 | 6.82e−17 |

절대 L2 target은 1e−10이다. cap16의 4×5 출력은 모두 통과했다. hρ=100에서 cap64의 실패는 Kahan으로 제거되지 않는다. 가장 큰 성분의 절대항 합에 ε를 곱한 **진단량**은 cap64에서 1.55e−7, cap16에서 4.58e−15다. 이는 error certificate가 아니지만 degree-only planner가 큰 중간항을 허용한다는 원인을 지지한다. 128→256 quadrature 변경에 따른 φ1 출력 차이도 cap64에서 1.35e−8, cap16에서는 1.78e−15다. 이를 순수 quadrature 오차로 해석해서는 안 된다. 계수의 작은 변화가 큰 basis cancellation에 의해 증폭되는 효과가 함께 들어 있다.

같은 hρ=100에서 Chebyshev exp action은 **50 JVP, 약 3.05e−12**였다. 따라서 이번 결과는 Laguerre가 Chebyshev보다 빠르다는 근거가 아니다. 동일 basis로 여러 φ를 공동 계산하는 것, global reduction이 없는 것, frozen A에서 계수·기저 재사용이 가능한 조건이 Laguerre의 후속 실험 동기다. 실제 wall time은 측정하지 않았다. cap16은 전회 탐색에서 고른 정책이며 새로운 독립 holdout으로 정당화한 보편 상수가 아니다.

### 후보 선택

- **Chebyshev:** 대칭 음의 실수축 operator에서 가장 먼저 비교할 기준 backend. φ1…φ4 공동 계수와 총 오차 budget까지 동등하게 맞춰야 한다.
- **Laguerre:** exp–φ 공통 recurrence를 구현한 조건부 후보. degree만 최소화하지 말고 coefficient sensitivity/roundoff budget을 함께 고려해야 한다.
- **Leja:** 이번 loop에서도 새 실행 구현은 하지 않았다. 기존 backward-error 분석에 맞춘 node/scaling/degree planner와 φ augmentation 또는 직접 φ divided difference를 구현하는 후속 후보다. 간단 Newton interpolation을 넣고 기존 Leja 논문의 robustness를 획득했다고 말해서는 안 된다.
- **Arnoldi:** 일반 비정규 도메인의 fallback 후보로 유지한다. 그 자체의 heuristic residual이 rigorous forward bound라는 뜻은 아니다. 수학적으로 위 다항식들도 span{v,Av,…}에 속하며, 대체 대상은 Arnoldi 직교화·투영 과정이다.

## 3. Homotopy: 구성 가능한 비선형 enclosure

### 최신 소스가 제공하는 것

`audit2_stage_certificate_research.rs`는 `SyntheticSchemaConsistencyOnly` authority를 명시한다. caller가 제공한 `strict_lower`가 비음수·strict-lower인지 검사하고, inverse witness 및 directed arithmetic으로 synthetic schema의 상계를 처리한다. 이것만으로 nonlinear stage coupling의 실제 Lipschitz premise가 구성되는 것은 아니다. 또한 그 모듈의 q_upper는 ||approximate_solution||+κ||residual||이므로 아래의 stage-error majorant와 의미가 다르다. 합리적인 다음 단계는 이 모듈의 기존 q에 새 수식을 무비판적으로 끼워 넣는 것이 아니라 **별도 StageErrorEnclosure 계약**을 만드는 것이다.

현재 transactional 경로의 q1 후보는 common-W batch 깊이 6, q2 diagnostic까지는 8이다. 함수 이름이나 λ=1 도달 여부만으로 numerical certification 또는 가속을 인정하지 않는다.

### 정의와 직접 유도

동일 stage target을

$$WK_i=g_i+hJ\sum_{j<i}L_{ij}K_j+hq\left(\sum_{j<i}\alpha_{ij}K_j\right)^{\odot2}$$

로 둔다. 여기서는 h≥0(실험은 h>0)이고, W=I−hγJ는 가역 공통 operator, α,L은 strict-lower이며 q는 scalar quadratic remainder 계수다. K와 δ는 state 단위, J는 시간 역수, q는 1/(state×time) 단위다. signed h로 확장할 때 모든 상계의 h를 |h|로 바꿔야 한다. 곱셈·제곱은 성분별이다. 후보 K̂의 잔차 ri는 위 식의 왼쪽에서 오른쪽을 뺀 값이며, δ̂i=Σj<i αijK̂j다. 이 quadratic toy는 전체 Rn에서 정의되어 domain enclosure가 닫힌다.

이미 j<i에서 성분별 |K̂j−Kj|≤Ej를 알고 있다고 하자. 다음을 순서대로 구성한다.

$$d_i=\sum_{j<i}|\alpha_{ij}|E_j,$$
$$E_i=U\left[|r_i|+h|J|\sum_{j<i}|L_{ij}|E_j
+h|q|\left(2|\widehat\delta_i|\odot d_i+d_i^{\odot2}\right)\right],\qquad U\ge|W^{-1}|.$$

여기서 부등호는 성분별이고 U는 검증된 비음수 상계다. quadratic difference 항은

$$|(\widehat\delta_i+\Delta\delta_i)^{\odot2}-\widehat\delta_i^{\odot2}|
\le2|\widehat\delta_i|\odot d_i+d_i^{\odot2}$$

로 제어된다. 처음 stage에는 이전 오차가 없으므로 Ei=U|ri|다. 유한 귀납으로 모든 stage enclosure와 endpoint bound

$$|\widehat y-y_{seq}|\le\sum_i|b_i|E_i$$

를 얻는다. 이 귀납은 **global norm contraction κ<1을 요구하지 않는다**. 하지만 큰 h에서 Ei가 급격히 커져 판정불능이 되는 것은 가능하다. 이는 false accept를 만드는 대신 fallback을 유도한다.

Scalar infinity-norm version은 wi=||W−1||∞, J∞=||J||∞, di=Σ|αij|ej를 써서

$$e_i=w_i\left[\|r_i\|_\infty+h\|J\|_\infty\sum_{j<i}|L_{ij}|e_j
+h|q|(2\|\widehat\delta_i\|_\infty d_i+d_i^2)\right]$$

다. 두 version은 같은 정리를 구현하지만 공간 성분 구조를 버리는 비용이 크게 다르다.

일반 smooth N에서는 이전 stage enclosure로 얻는 ball/box 안에서 |N′|의 검증 상계를 계산해야 한다. enclosure 밖에서 얻은 Lipschitz 상수나 관측된 correction ratio는 이 역할을 대신하지 못한다. 시간·상태별 varying mass matrix 및 W singularity는 별도 계약이다.

### 실행: 정확 유리수와 실제 tableau를 분리

1. **정확 유리수 8-stage fixture:** 단순 rational strict-lower α,L, γ=1/4, J=−1; h=1/10,1,3; affine/quadratic; q1/q2를 Fraction 연산으로 계산했다. 12개 candidate 모두 stage·state enclosure·endpoint 부등식이 **정확한 유리수 비교로 참**이다. target 1e−8에서 4개가 인증 수락되고 모두 실제 오차 기준도 충족했다. float safety epsilon을 사용하지 않았다. 이 fixture는 현재 RODAS tableau와 다른 합성 문제이며 integrator order 검증이 아니다.
2. **최신 8-stage RODAS fixture:** affine normal, affine nonnormal, quadratic nonlinear; h=0.01,0.1,1,10; q1/q2 총 24개. 비교 oracle은 동일 h의 직접 forward-substitution stage solution이다. source 계수는 공유하며, 원 ODE의 정확해를 독립 검증한 것이 아니다. binary64에서는 잔차·inverse·bound 평가의 directed rounding을 수행하지 않았으므로 **수치 검산**이다. roundoff 수준의 일부 bound underestimate가 존재하며 JSON에는 `arithmetic_check_tolerance`를 기록했다. 이를 생산 certificate PASS로 승격할 수 없다.

새로운 중요한 관찰은 J=[[-1,100],[0,−1]]인 비정규 예제다.

| h | 후보 | 실제 endpoint infinity 오차 | scalar norm bound | 성분별 bound |
|---:|---|---:|---:|---:|
| 0.1 | q1 | 2.63e−9 | 8.13e−9 | 2.65e−9 |
| 1 | q1 | 6.69e−3 | 30.49 | 7.10e−3 |
| 1 | q2 | 7.82e−14 | **30.36** | **7.25e−13** |
| 10 | q1 | 34.75 | 4.42e6 | 55.80 |
| 10 | q2 | 9.28e−13 | **4.42e6** | **5.96e−12** |

Scalar norm은 transient와 방향성을 stage마다 최악 방향으로 다시 조합한다. 실제 upper-triangular 공간 구조를 보존한 성분별 bound는 그 과대평가를 크게 줄인다. target 1e−8에서 scalar 방식은 24개 중 14개, 성분별 방식은 16개를 수치적으로 수락했고, reference 기준도 16개가 충족했다. 이 작은 표본의 일치는 일반 sharpness theorem 또는 production false-accept rate가 아니다.

성분별 U를 일반 matrix-free 문제에서 얻는 일은 공짜가 아니다. 실제 inverse를 dense 구성하는 toy를 대규모 backend에 그대로 적용하면 원래 목표를 훼손한다. 가능한 경로는 구조적 positive comparison operator, verified sparse/block approximate inverse, 또는 저차원 observables에 대한 adjoint residual bound다. approximate inverse V만 있을 때 |I−VW|≤R, ρ(R)<1을 검증하면

$$|W^{-1}|\le(I-R)^{-1}|V|$$

라는 충분조건이 있다. 이 조건과 (I−R)−1의 계산 비용을 실제 certificate work에 포함해야 한다. 검증되지 않은 |V|를 |W−1|의 상계로 사용해서는 안 된다.

### WRMS, embedded error와 차수

현재 실험은 infinity norm이다. positive scale σa를 고정한 WRMS로 이전하려면, componentwise endpoint bound Ea^out에 대해

$$\|\widehat y-y_{seq}\|_{WRMS}\le\left[\frac1n\sum_a(E_a^{out}/\sigma_a)^2\right]^{1/2}$$

를 directed arithmetic으로 평가해야 한다. embedded weights에 대한 별도 bound E^emb도 구성하고, endpoint accuracy budget과 estimator contamination을 구별해야 한다. sequential discretization error와 stage algebraic error는 서로 다른 항이다. 원 sequential method가 p차이고 안정성이 제어된다는 별도 전제 아래 output stage perturbation O(h^(p+1))가 충분한 차수 보존 조건이다. 이번 toy 결과는 stiff-uniform 5차 증명이 아니다.

## 4. 가속 가능성과 실행 우선순위

8-stage common-W ideal depth는 sequential8, q1=6, q2 diagnostic 포함8이다. q1 성공률 p1, q2 뒤 sequential fallback률 pf에 대해 전회와 같은 단순 모델은 E[depth]=8−2p1+8pf다. **p1>4pf**가 필요하다는 조건은 여전히 유효하지만, certificate 구성을 추가하면 비용 여유가 더 줄어든다. 이번 성분별 연구가 개선한 것은 정확도 admission이며 wall speed는 측정하지 않았다.

권장 실행 순서는 다음과 같다.

1. 최신 저장소의 correctness와 tolerance 소비자 문제를 먼저 닫고, 독립 precision reference 및 exact input contracts를 고정한다.
2. `MatrixFunctionAction` 결과에 method/domain/evidence/error components/work를 명시한다. Chebyshev와 Laguerre 공동 φ backend를 같은 호출 계약으로 비교한다. 타입에 `heuristic`, `truncation_only`, `certified_total`을 구분한다.
3. Laguerre planner에 truncation–quadrature–roundoff budget을 분리한다. L cap은 시작 정책이며 검증된 roundoff 모델로 대체한다. same-vector multi-φ와 different-vector fused-φ의 JVP 수를 별도 기록한다.
4. `StageErrorEnclosure`를 synthetic schema certificate와 별도 구현한다. 정확한 target residual, stage dependency, verified U/|J| 또는 local derivative bound, norm scale을 묶는다. 전역 scalar norm뿐 아니라 sparse/block componentwise majorant를 지원한다.
5. current q1/q2 소비자는 실제 upper bound를 얻지 못하면 `UNRESOLVED_BOUND`로 fallback한다. 각 실패의 모든 RHS/JVP/W solve/certificate work를 count한다. native Rust 결과를 Fraction/Decimal oracle와 교차검증한다.
6. correctness가 닫힌 후 실제 RHS/JVP batch callback과 pool reuse를 넣고, sequential도 같은 kernel/cache 최적화를 적용한다. 1/2/4/8 workers의 paired process benchmark를 전체 시도 비용으로 비교한다.
7. stage depth 여유가 작다는 사실을 받아들여, 장기 가속은 4–16 time-window의 Parareal/PFASST/ParaDiag를 별도 integrator/solver 실험으로 구분한다. λ-grid 확장만으로 parallel speedup을 기대하지 않는다.

각 작업의 prerequisites, observable acceptance, failure state, deliverables는 `RESEARCH_NEXT_DAG.json`에 담았다. 외부 독립 reviewer의 판단 전까지 이 문서의 후보는 **HOLD_FOR_PRODUCTION**이다. 결과 파일의 `implementation-verified`는 standalone probe 범위만 의미한다.

## 5. 문헌과 source의 근거 수준

이번 새 결과의 핵심은 직접 유도와 실행이다. 다음 문헌은 전회 감사에서 확인한 primary-source 범위를 상속하며, 새로운 문헌 전체를 읽었다고 주장하지 않는다.

- NIST DLMF §18.14 Eq.18.14.8, https://dlmf.nist.gov/18.14.E8 : Laguerre scalar bound. φ joint tail은 이번 직접 유도다.
- Caliari, Kandolf, Ostermann, Rainer, https://arxiv.org/abs/1506.08665 : Leja scaling/backward-error, nonnormal hump 관련 비교 근거. 새 Leja 구현은 NOT_RUN.
- Deka, Tokman, Einkemmer, https://arxiv.org/abs/2211.08948 : integrator/workload에 따른 action method 비교; VigilODE 성능으로 전이하지 않는다.
- Khoroshikh, Kurbatov, https://arxiv.org/abs/2312.07291 : 시간 Laguerre 함수의 rational shifted-solve 방법과 현재 matrix-polynomial recurrence를 구분한다.
- Speck, https://arxiv.org/abs/1703.08079 : across-the-method SDC 비교군; 이번 stage enclosure theorem과 다른 integrator다.

정확 source identity는 root source manifest를 따른다. 이 문서의 tableau bytes hash와 실행 원문은 JSON/로그에 보존했다. 수학 전제, 실행 성공, source identity, 독립 심사 상태는 서로 대체하지 않는다.
