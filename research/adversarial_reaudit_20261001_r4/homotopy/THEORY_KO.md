# R4 homotopy: 인증의 수학적 범위와 병렬화 후속 설계

Source `1c54194123ee6abc6daa512e8574922f510b4e2c`. 이 문서는 source를 읽어 유도한 결과다. 실제 probe 결과는 RESULTS.json과 REVIEW_KO.md가 별도로 기록한다.

## 1. 정확한 대상과 삼각 귀납

표현된 binary64 계수를 정확한 실수로 보고, `StageTarget::sequential`이 읽는 엄밀한 하삼각 계수만 사용한다. \(W=I-h\gamma J\), \(L^*_{ij}=\alpha_{ij}+\Gamma_{ij}\)이며 이 합은 저장된 rounded L과 구별한다. 선언된 autonomous identity-mass quadratic 문제의 단계는

\[
WK_i^*=h(Jy-q\odot y^2)+hJ\sum_{j<i}L^*_{ij}K_j^*+h q\odot(\sum_{j<i}\alpha_{ij}K_j^*)^2.
\]

\(W\)가 가역이면 각 우변은 이전 단계에만 의존하므로 정확한 단계근은 유일하다. 이것은 전체 block consumer의 upper leakage를 무시해 얻은 명목상 nilpotency가 아니라, 명시된 sequential target의 구조에 관한 주장이다. ODE local/global truncation error에 관한 주장은 아니다.

후보 \(\widehat K_i\), 잔차 \(r_i\), 후보 increment \(\widehat d_i=\sum_{j<i}\alpha_{ij}\widehat K_j\)를 둔다. 실제 오차 \(e_i=|\widehat K_i-K_i^*|\)와 \(d_i=\sum_{j<i}|\alpha_{ij}|e_j\)는, **검증된** \(U\ge|W^{-1}|\)에 대하여

\[
e_i\le U\{ |r_i|+h[|J|\sum_{j<i}|L^*_{ij}|e_j+|q|\odot(2|\widehat d_i|\odot d_i+d_i^2)]\}.
\]

이는 \(|a^2-b^2|\le2|a||a-b|+|a-b|^2\)와 삼각부등식으로 직접 도출된다. Native serial recurrence는 이 관계를 단계별 outward rounding으로 계산한다. 정상 생성 witness와 정확한 target라는 전제하에 이론 구조는 타당하다. 입력 위조·오손에 대한 API 검증은 별도 문제다.

## 2. 반지름 인증과 유한 경로합

현재 후보 또는 정확근의 oracle에서 가져오지 않은 \(D\ge0\)를 먼저 제안하고 \(d_i\le D\)를 가정하여 \(\ell_i=|q|(2|\widehat d_i|+D)\)를 만든다. 그러면 양의 엄밀 하삼각 block matrix H와 a에 대해 \(e\le a+He\). \(H^s=0\)이므로

\[
\bar E=(I-H)^{-1}a=\sum_{j=0}^{s-1}H^j a.
\]

outward 결과 E가 \(E\ge\bar E\)를 보장하고 \(\sum_{j<i}|\alpha_{ij}|E_j\le D\)이면, i에 대한 귀납으로 실제 d도 D에 머물고 실제 e가 \(\bar E\) 이하임을 보일 수 있다. Rounded E 자체가 \(E\ge a+HE\)를 만족한다고 추가 가정할 필요가 없다. 이 구별은 rounding이 섞인 proof를 간결하고 정확하게 만든다.

\[
S_L=\prod_{\ell=0}^{L-1}(I+H^{2^\ell})=\sum_{j=0}^{2^L-1}H^j,
\qquad L=\lceil\log_2s\rceil.
\]

s=8일 때 L=3이 정확하다. 가변 s를 받는 공개 API라면 이 구조에서 L을 계산하거나 s≤8을 명시적으로 검사해야 한다. 상수 3을 일반 s에 그대로 사용하면 빠진 경로가 생긴다. Fraction candidate는 단계수별 exact inverse identity까지 확인하도록 사전등록했다.

## 3. diagonal 문제에서 인증 자체의 병렬화

J와 U가 diagonal이고 q가 componentwise이면 H의 서로 다른 물리성분을 잇는 원소가 0이다. stage-major index \((i,u)\)를 component-major \((u,i)\)로 바꾸는 permutation P에 대해

\[
PHP^{-1}=\bigoplus_{u=1}^n H_u,\qquad H_u\in\mathbb R^{s\times s}.
\]

따라서 \(E_u=\sum_{j=0}^{s-1}H_u^j a_u\)를 서로 독립적으로 계산한다. 단일 (sn)² matrix의 저장량은 ns²로 줄고, dense multiplication의 형식적 곱셈 수는 (sn)³에서 ns³로 줄어든다. 각각 n배, n²배 감소다. 이것은 행렬 구조로부터 얻은 정확한 연산량 비교이며 wall-time 가속 측정이 아니다. sparse factorization/parallel scheduling 비용도 별도로 측정해야 한다.

현재 implementation은 block matrix를 dense vector-of-vectors로 구성한다. Diagonal witness도 n²개의 f64를 실제로 저장하면서 work.stored_values에는 n을 기록한다. 이 필드는 현재 물리적 allocation 수라기보다 비영 원소 수로 읽어야 한다. 후속 구현은 `Diagonal(Vec<f64>) / Dense / Banded` 등의 구조 표현을 분리하고 실제 allocation bytes와 nonzero count를 구별해야 한다.

D도 단일 scalar보다 (stage, component)별 D_i,u를 쓰면 특히 stiff/slow 성분의 규모 차이로 인한 certificate 과팽창을 줄일 수 있다. 단, predictor는 exact current root를 보지 않아야 하고 componentwise closure를 같은 outward 식으로 확인해야 한다.

## 4. 7-batch라는 수치와 실제 가속 조건

native certificate가 q=2 뒤의 diagnostic eighth W batch를 제거한 것은 실제 코드 변화다. 그러나 HOM-06 runtime은 `certify_stage_target`의 **serial** recurrence를 호출한다. 연구용 `doubling_certificate`가 그 경로에 통합됐다는 주장은 성립하지 않는다. q1도 여전히 operational gate이고 q2에만 target certificate가 붙는다.

평균 W-vector solve 비용을 c_W, s=8, 한 batch의 worker wave를 \(\lceil8/P\rceil\), q1/q2/fallback 비율을 p1,p2,pf라 하자. 모든 speculative attempt가 6 또는 7 batches를 수행하고 fallback에는 sequential 8 solves가 추가된다는 이상화 아래

\[
T_{trial}\approx(7-p_1)\lceil8/P\rceil c_W+8p_f c_W+T_{cert}+T_{rhs}+T_{pool}+T_{other}.
\]

동일한 solve-cost의 baseline \(T_{seq}\approx8c_W\)보다 빨라지려면 P≥8에서

\[
(T_{cert}+T_{rhs}+T_{pool}+T_{other})/c_W<1+p_1-8p_f.
\]

q1=0,pf=0이면 certificate와 추가 RHS, scheduling 모두에 허용되는 여유가 W solve 한 번뿐이다. pf=1인 coupled case에서는 이러한 이상화에서도 불가능하다. c_W와 단계별 iteration 수가 동일하다는 가정 때문에 이 식은 설계 예산식이며 측정 결과를 대체하지 않는다. Block matvec/GMRES의 stage batching, W/preconditioner reuse, 사전 reject 정책과 persistent pool이 함께 필요하다.

현재 common-W solver는 매 step pool을 만든다. 연구용 doubling도 각 matrix multiply마다 pool을 만든다. Pool을 outer integration/campaign lifetime으로 이동하는 것이 첫 구현 단계다. 단, 이것만으로 six-to-eight-fold vector-work 증가가 제거되는 것은 아니다. 연산량/정확도를 유지한 CPU batching과 matrix-free 적용의 비용 모델을 함께 검증해야 한다.

## 5. 모델과 ODE binding

stage-state RHS 점검 및 첫 stage direction의 JVP 일치는 유용한 consistency 검사다. 유한한 점 검사로 두 함수를 neighborhood 전체에서 같다고 증명할 수 없다. 현 코드도 그 한계를 reason 문자열과 trait 설명에 인정한다. quadratic corpus는 RHS/JVP와 certificate source가 같은 `(A,q)`를 공유하여 구조상 binding을 보완한다. 임의 callback에는 같은 강도의 주장을 확장하면 안 된다.

확장 경로는 두 가지다. (a) 동일한 typed quadratic model이 RHS/JVP/certificate 데이터를 모두 생성하게 하고 source identity를 구조적으로 묶는다. (b) 일반 ODE에서는 candidate tube에서의 model-defect bound rho_i와 remainder Lipschitz bound를 제공하여 잔차에 rho_i를 더한다. 모든 component의 도함수 bound나 인증된 AD/interval extension이 필요한데, 한 방향 JVP만으로 이를 확보할 수는 없다.
