# RVJ 독립 연구 루프 12: causal derivative-free moment packet

날짜: 2026-10-11. 범위: 독립 이론 및 exact-rational reference 연구. Vigilode 저장소·PR·원격 백업을 읽거나 변경하지 않았다. 새 후보는 `CAUSAL_MOMENT_PACKET`이며 기존 RVJ5와 동일한 tableau가 아니다. 사용자 목표는 no-go 추가가 아니라 JVP를 실질적으로 제거하고 정확성과 연산량 감소를 동시에 증명하는 것이다.

## 0. 이번 결론과 권위

현재 모델군에서 online Jacobian, JVP, VJP, finite-difference directional derivative, Krylov, matrix factorization 호출은 0이다. 이는 해석적 미분을 몰래 반복한 결과가 아니라, 알려진 구조로부터 scalar Volterra kernel과 초기 모멘트를 컴파일한 결과다. 임의 독립 초기상태는 모두 저장하고 재구성한다.

이번에 증명한 개선은 (A) 공통 선형계수의 exact collective/transverse 분리, (B) 서로 다른 주파수에서의 모멘트 packet 및 원래 n-mode 문제의 오차경계, (C) 동일한 다항식 waveform 맵을 보존하면서 kernel contraction 작업량을 n배 줄이는 항등식, (D) 명시된 analytic ball에서 실제 후보 수렴과 자원 상계다. General black-box ODE, 불명확한 spectral basis, 임의 비정규 연산자에서 같은 개선을 주장하지 않는다.

`derived / connected-CAS-checked / exact-reference-verified / numerically-checked`를 구분한다. 최종 independent decision reviewer는 없으므로 PROMOTE는 HOLD다. 원래 generic RVJ, approximate-W, embedded blindness 반례는 그대로 유지한다.

## 1. 원본과 하네스

사용자가 지정한 Dropbox 경로의 마지막 폴더명이 두 번 반복된 경로는 실제 조회에서 NOT_FOUND였다. 검색 후 동일 이름의 실제 폴더 `/bianchi/htt_base/harness/physmath-research-gpt6-astra`를 발견했고 START_HERE, PROJECT_INSTRUCTIONS, MODEL_ROUTING, state/RESEARCH_STATE, integrated prompt를 직접 읽었다. Version은4.0.0이고 remote state는 NOT_RUN 템플릿이다. 템플릿을 현재 연구의 실행 상태로 상속하지 않는다.

Loop1–11의 primary notes를 원 ZIP에서 읽고 보존했다. `inputs/HISTORY_SOURCE_INDEX.json`이 source archive identity를 기록한다. 전체 과거 numerical campaign을 재실행하지 않았다. 하네스의 이론·수치·구현·환경 실패 분류, 원래 승인 범위 내 계속 실행, independent reviewer 없는 HOLD 규칙을 유지한다.

## 2. 전체 후보의 재정렬

- Ordinary Rosenbrock-W의 depth 장벽: 고차 derivative 또는 같은 step 정보를 더 넣지 않고 일반 W5의 깊이를 줄이는 경로는 여전히 닫혀 있다. 이번에는 다른 표현으로 nonlinear 작업 자체의 차원을 줄인다.
- RJ5/RVJ5: 제한된 차수·안정성 결과는 보존하지만 generic stiff-uniform 정확성을 주장하지 않는다. 여기서는 scalar mean 후보를 명시적으로 certificate에 넣는다.
- Chart, invariant, witness, homotopy: 해를 보존하는 projectability와 초기 transverse data를 확인할 때 실제 계산 축약을 제공한다. Point support가 하나인 것만으로 정보를 지우지 않는다.
- Fourier/FFT: nonlinear polynomial product의 빠른 후보 생성에 유용하지만 alias/tail 및 물리 검증은 별개다. 이번에는 FFT를 추가하지 않고 직접 exact coefficient convolution을 사용한다.
- Loop11의 shifted inverse 공유: factorization1개, jetdepth18의 결과는 유효하지만 JV와 inverse action이 남았다. 아래 11절은 명시적 JV 제거 항등식도 주지만 inverse oracle의 내부비용은 남는다.
- 이번 중심: linear flow + low-rank collective feedback의 구조를 확인하여 큰 tangent problem을 생성하지 않는다. Scalar state만 저장하는 것이 아니라 scalar nonlinear computation과 n개의 고정 초기데이터를 함께 쓴다.

## 3. 정리 A: exact collective quotient와 transverse reconstruction

상수 선형 A, 고정 B, 양의 pi_j(sum pi_j=1)에 대해

    y_j' = A y_j + B g(m),   m=sum_j pi_j y_j

를 고려한다. 국소유일해가 존재하는 영역에서

    m'=A m+B g(m),
    y_j(t)=m(t)+exp(tA)[y_j(0)-m(0)]

가 정확하다. 증명: 가중합을 취하면 첫 식이고, 원식에서 평균식을 빼면 각 차이는 homogeneous linear ODE를 만족한다.

n개의 독립 초기값은 삭제하지 않는다. 평균만 online nonlinear solve로 계산하고 각 fibre의 초기좌표가 나머지 해를 결정한다. P=1 pi^T는 P²=P이고 J=2P-I는 involution이다. H=diag(pi_j)에서 P는 자기수반이며 collective/transverse orthogonal decomposition을 제공한다. 이것을 물리값의 동일시나 임의 차원축소로 해석하지 않는다.

미분위상 관점에서 평균 map은 submersion이다. 예를 들어 n3의 좌표 (m,y1-y3,y2-y3)는 sum pi=1일 때 determinant1이다. Involution은 두 sector를 보존한다. 합체 전후 reconstruction이 정확하기 때문에 사용자의 mode-fusion 직관이 안전한 형태로 구현된다.

대수기하 관점의 충분조건: polynomial constraints g_a(y)=0의 ideal I에 대해 Dg_a F=sum_b B_ab g_b이고 Dpi F-f_red(pi) in I이면 constraint의 불변성과 quotient dynamics가 성립한다. 이는 유한 polynomial identity로 offline 확인할 수 있다. Groebner basis 탐색·구조 발견 자체의 최악 복잡도가 작다는 주장은 아니다. Offline symbolic derivatives를 허용하는 것과 online JVP를 제거하는 것을 구분한다.

공통 quasi-linear ensemble의 이런 exact reconstruction에는 선행연구가 있다. Augustsson–Martens–Cestnik의 원전 SectionsII–IV를 읽었고 초기 parameter를 보존해 모든 microscopic 상태를 재구성한다는 점을 대조했다. Exact reduction 자체를 처음 발견했다고 주장하지 않는다.

### 더 일반적인 finite-rank Volterra closure

    y'=A y+B g(Cy),  m=Cy

이면 정확히

    m(t)=C exp(tA)y0 + integral_0^t C exp((t-s)A) B g(m(s)) ds.

이는 CA=Ar C라는 Markovian lumping 조건 없이도 성립한다. 대가로 r=rank(C)차원 비선형 memory equation이 생긴다. A의 propagator를 싸게 계산할 구조가 있어야 실제 비용 이득이다. 본 구현은 그 구조를 알려진 diagonal-frequency packet에서 명시적으로 공급한다.

## 4. 모델 B: 서로 다른 주파수를 가진 실제 결합계

무차원 benchmark convention:

    z_j'=(-kappa+i(Omega+Delta xi_j))z_j + i chi |m|² m,
    m=sum_j pi_j z_j,
    pi_j>0, sum pi_j=1, |xi_j|<=1, chi real, kappa>=0.

Omega,Delta,kappa는 물리단위로 inverse time이고 chi는 |z|²와 time scale에 맞는 단위를 갖는다. Physical state unit과 tolerance는 변환 후에도 유지한다. 임의의 주파수를 black-box J로부터 무료로 추출했다고 가정하지 않는다.

    E=sum pi_j |z_j|², E'=-2 kappa E.

각 imaginary diagonal term은 norm을 바꾸지 않고, 공통 nonlinear항의 에너지 기여는 2Re(conj(m) i chi |m|²m)=0이다. E(0)<=1이면 |m|<=1이다. g(m)=i chi |m|²m은 unit disk에서 G=|chi|와 L=3|chi|를 만족한다. 삼각부등식으로 직접 얻을 수 있으며 runtime derivative가 필요하지 않다.

Carrier z_j=exp(iOmega t)a_j는 정확하다. Delta와 서로 다른 xi_j는 남는다. 현재 code는 kappa=0이며 damping은 아래 정리에는 포함하지만 damped kernel client를 구현·검증한 것은 아니다.

## 5. 정리 B: heterogeneous packet의 exact scalar memory와 truncated kernel

    K(t)=exp(-kappa t) sum pi_j exp(iDelta xi_j t),
    F0(t)=exp(-kappa t) sum pi_j a_j0 exp(iDelta xi_j t)

이면

    m(t)=F0(t)+integral_0^t K(t-s)g(m(s))ds.

데이터 모멘트

    mu_k=sum pi_j xi_j^k,
    nu_k=sum pi_j xi_j^k a_j0

를 컴파일한다. Degree-p kernel은

    Kp(t)=exp(-kappa t) sum_(k=0)^p mu_k(iDelta t)^k/k!,
    Fp(t)=exp(-kappa t) sum_(k=0)^p nu_k(iDelta t)^k/k!.

Moment packet은 n개의 amplitude를 지우지 않는다. mu와nu는 공통 nonlinear 계산이 요구하는 집계량이며 최종 reconstruction에는 원 a_j0를 계속 사용한다.

### Homotopy와 local algebra의 실제 연결

Delta->0은 공통 주파수의 exact quotient로 간다. Delta!=0에서는 각 주파수의 분리 정보를 모멘트에 보존하고 remainder를 지불한다. z^j 또는 xi^j를 높은 차수에서 무조건0으로 놓고 원모델과 같다고 하지 않는다.

유한 kernel realization은 N^(p+1)=0인 shift matrix N에 대해

    M'=(-kappa I+iDelta N)M+mu g(M0), M(0)=nu

이며 M0가 Fp+Kp*g(M0)를 만족한다. 이것은 p+1 coefficient를 가진 nilpotent packet이며 generic n개의 eigenmode를 한 scalar로 압축하는 정리가 아니다. Top moment의 생략은 아래 tail이 제어한다.

대칭 xi distribution이면 mu_odd=0이다. 그러나 독립 a_j0 때문에 nu_odd는 일반적으로0이 아니다. 이번 exact regression은 mu1=0,nu1!=0을 확인한다. +/- target 재구성에서 even/odd Horner 결과를 공유할 수 있지만 odd 초기정보를 버리지 않는다.

## 6. 정리 C: 원래 모든 모드의 오차를 n-mode residual 없이 인증

실수 theta의 exponential Taylor remainder로

    tau_p(T)=(|Delta|T)^(p+1)/(p+1)!

를 정의한다. exp(-kappa t)<=1이므로

    |F0-Fp|<=A0 tau_p,
    integral_0^T |K-Kp| ds <= T tau_p/(p+2),
    A0=sum pi_j |a_j0|.

후보 mhat가 [0,T]에서 unit disk에 있고 scalar Volterra defect

    r=mhat-Fp-Kp*g(mhat),   sup|r|<=D

를 만족한다고 하자. 그러면

    Em <= exp(LT)[D+tau_p(A0+GT/(p+2))].                   (C1)

증명: exact mean과 후보를 빼서

    m-mhat=(F0-Fp)+(K-Kp)*g(mhat)+K*[g(m)-g(mhat)]-r

를 얻는다. |K|<=1, |g(mhat)|<=G, LipschitzL을 이용한 Gronwall이 결론을 준다. 실제 코드에서는 LT<1에서 exp(LT)<=1/(1-LT)를 사용하고 모든 상계를 Fraction으로 계산한다.

각 모드를

    ahat_j=Ep(iDelta xi_j t) a_j0
           +integral_0^t Ep(iDelta xi_j(t-s))g(mhat(s))ds

로 재구성하면(kappa0 코드; 이론에서는 공통 exp(-kappa t)를 유지)

    Ej <= tau_p[|a_j0|+GT/(p+2)]+LT Em.                 (C2)

증명: initial kernel error, forced kernel error, exact kernel에 전달된 g(m)-g(mhat) 세 부분으로 나누면 된다. 임의 initial amplitude에 대해 각 Ej를 검사한다. Mode 평균 출력이 mhat와 완전히 같을 필요는 없고, 그 차이에는 mhat의 Volterra residual이 들어 있다. 이 점을 exact test로 확인한다.

모든 fine-scale JVP가 없어지는 핵심은 C1–C2가 원래 모델과 packet 사이의 오차를 구조적으로 포함한다는 것이다. 매 보정마다 n-mode Jacobian 또는 n-mode residual을 생성할 필요가 없다.

## 7. 정리 D: 동일한 이산 맵에서 보장되는 연산량 감소

kappa0에서 tau=t/T, Kp(tau)=sum a_j tau^j, f(tau)=sum b_k tau^k라 하면

    integral_0^tau (tau-s)^j s^k ds
        =j!k! tau^(j+k+1)/(j+k+1)!.

시간 길이T는 convolution 앞에 곱한다. 이 식은 causal convolution이지 circularFFT나 pointwiseproduct가 아니다. 알려진 polynomial basis의 적분만 수행하며 vector field를 미분하지 않는다.

### 동일 맵 정리

각 개별 mode에 동일한 degree projection Pi_D와 정확 산술을 적용하는 waveform Picard와, weighted sum을 먼저 하고 Fp,Kp를 사용하는 scalar Picard의 평균은 모든 sweep에서 정확히 같다.

증명: 초기 free profile에서 선형성으로 성립한다. 같은 평균이면 nonlinear forcing g도 같고, causal convolution·가중합·선형 degree projection이 commute하므로 다음 sweep에도 성립한다. 귀납법으로 모든 sweep에 성립한다.

주의: 서로 다른 위치에서 각 mode를 rounding한 뒤 합하면 bit identity까지 같다는 보장은 없다. Exact 비교는 rounding을 껐고, 실제 rounded scalar candidate는 C1–C2로 다시 검증한다.

동일 kernel order p, forcing degree3D에서 한 sweep의 convolution coefficient product 수는 개별 계산의 n(p+1)(3D+1)에서 (p+1)(3D+1)로 **정확히 n배** 줄어든다. 양쪽 모두 nonlinear mean을 한 번 계산하도록 하여 불필요하게 n번 nonlinear evaluation을 강요한 baseline을 만들지 않았다. 기존 최적 solver가 이미 같은 선형성 재배치를 한다면 이 n배 이득을 다시 주장할 수 없다.

### 모든 개별 출력도 shared polynomial로 복원

최종 forcing f(tau)=sum f_k tau^k에서

    Bp(xi)=sum_(r=0)^p (iDelta T)^r xi^r/r!,
    Cp(xi)=T sum_(r=0)^p (iDelta T)^r xi^r
                       sum_k f_k k!/(r+k+1)!,
    ahat_j(T)=Bp(xi_j)a_j0+Cp(xi_j).

Cp coefficient setup은 O(pD), 모든 mode output은 O(np) Horner다. 입력a_j0는 그대로 사용한다. 출력R회, sweepS회이면(직접 다항식곱)

    W_packet=O(np+S[D²+pD]+R[np+pD]),
    storage=O(n+p+D).

반면 개별 profile을 갱신하는 동일맵은

    W_full=O(np+S[D²+npD]), storage=O(nD)

에 더해 aggregation/output 비용이 있다. FFT 곱을 쓰면 coefficient multiplication의 점근복잡도를 더 개선할 수 있지만 이번 구현은 exact 직접곱이다.

이는 arithmetic-operation theorem이다. Fraction의 정수 bitlength, 메모리 locality, 언어, oracle 또는 output비용까지 같은 상수라고 가정한 wall-clock정리가 아니다. 모든 n outputs를 모든 step에서 요구하면 Omega(n*steps)의 출력 하한을 피할 수 없다. p가 n보다 크거나 군집이 넓으면 이 방식의 장점이 줄어든다.

## 8. 정리 E: 실제 후보 수렴과 n-independent 자원 상계

p,D,S를 고정했다고만 가정하면 accuracy를 달성하는 보장은 부족하다. 다음은 구체적인 충분조건이다.

Weighted analytic coefficient norm ||u||_R=sum |u_k|R^k, R>1에서 conjugation은 등거리이고 polynomial product는 Banach algebra다. Volterra convolution의 norm은 R ||K||_R||f||_R 이하이다. r=|Delta|T, B<=1에 대해

    A0 exp(rR)+|chi|TR exp(rR) B³ <= B,
    qR=3|chi|TR exp(rR)B²<1                           (E1)

이면 scalar truncated-kernel Volterra map과 그 degree-D projection은 이 analytic ball에서 self-map이고 contraction이다. 적절한 엄격 여유는 각 rounding의 ||.||_R 오차도 수용한다.

실제 무한차수 fixedpointu와 projected fixedpointu_D에 대해 q1=3|chi|T exp(r)B²<1이면

    ||u_D-u||_1 <= B R^(-D-1)/(1-q1).

증명: u의 analytic tail은 ||(I-Pi_D)u||_1<=B R^(-D-1)이다. 두 fixedpoint 식을 빼고 projection의 비팽창성과 q1을 사용해 이동시킨다.

S회 Picard와 각 회차 coefficient rounding 오차epsilon_b에 대해

    ||uhat_S-u||_1 <= 2B q1^S + B R^(-D-1)/(1-q1)
                                      +epsilon_b/(1-q1).       (E2)

따라서 실제 unprojected Volterra residual도 (1+q1) times RHS(E2) 이하이다. C1–C2와 합치면 요구 오차를 보장하는 p,D,S,b를 사전에 선택할 수 있다. Oracle나 Jacobian은 필요 없다.

구체적인 admitting family:

    |Delta|<=1, T<=1/2, |chi|<=1/8,
    A0<=9/32, max|a_j0|<=11/32,
    R=3/2, B=3/4.

정확 rational exponential bound exp(3/4)<17/8, exp(1/2)<5/3을 사용하면

    mapping_bound=0.68170166015625 < B,
    qR=0.336181640625 < 1,
    q1=0.17578125 < 1.

이 수들은 n과 공통Omega에 독립이다. Kernel tail의 factorial감쇠 및(E2)의 기하감쇠에서 충분한

    p,D,S=O(log(1/epsilon))

와 b=O(log(1/epsilon))의 coefficient precision을 얻는다(위 고정된 margin과군집범위). 예를 들어 k=p+1>=max(2e r,log2(1/eta))이면 r^k/k!<=2^-k<=eta이다. 따라서 직접 다항식 산술에서

    W=O(n log(1/epsilon)+log³(1/epsilon)
                        +R_out n log(1/epsilon))

라는 명시적인 arithmetic upper bound가 성립한다. n-independent라는 말은 nonlinear work의 차수가 n에 의존하지 않는다는 뜻이며 n 초기데이터와 output은 계속 존재한다.

Conservative preselector는 tol1e-7에 p18,D36,S18을 선택하고 individual upperbound7.536100496e-8을 보장했다. 실제 n17 실행의 사후 bound는6.302110928e-24였다. 이 간극 때문에 실무에서는 사전 theorem을 안전망으로 두고 더 싼 후보의 실제 residual을 먼저 검사하는 것이 합리적이다. 현재 primary candidate p8,D10,S6은 실제 certificate로 훨씬 일찍 통과한다. 사전수렴정리가 optimal cost를 증명한 것은 아니다.

## 9. Causal, adaptive, hybrid 계약

현재 reference는 t0=0에서 시작하는 하나의 인증된 finite window다. 미래exactdata나 audit oracle 없이 model, initial data, horizon과 기존 moment packet에서 후보를 생성한다. 시간 Fourier의 periodic boundary를 도입하지 않는다. 여러 Picard sweep은 순차 의존성이 있고 무한병렬화라고 하지 않는다.

Kernel order4,8,12,16을 시험하는 selector는 max-order moments를 한 번만 만든 뒤 prefix를 재사용한다. Order를 바꿔도 xi와initial은 그대로 유지되므로 target dynamics를 homotopy로 바꿔치기하지 않는다. Trial별 cost를 누적하고 final target bound로 수락한다. 임의 여러 window를 연결하는 full adaptive integrator와 restart 초기오차 transport는 아직 구현하지 않았다.

Common carrier는 모든 outputs에 하나의 exp(iOmega T)를 공유한다. 정확 rational Taylor와 outward-rounded 반복제곱으로 복원한다. 이 단계의 cost는 angle과 precision에 의존한다. Physical error는 E_j+|ahat_j| epsilon_phase다. Frequency uncertainty와 detuning uncertainty의 일반 sensitivity는 별도다.

## 10. 실제 실행 결과

Primary model: uniform xi in[-1,1], weights1/n, a_j0=1/4+xi_j/16+i((j mod5)-2)/64, chi1/8,T1/2,kappa0. 모든 입력은 Fraction이다.

- Delta1,p8,D10,S6에서 n17/257/1025를 실행했다. 최대 mode error는 약1.68231e-9,1.68231e-9,1.68867e-9이고 최대 endpoint bound는 약2.19e-9다. Tol1e-7을 모두 통과했다.
- n17,Delta1,p0은 실제error0.154786으로거절, p4는8.12767e-5로거절했다. 작은 reduced residual만으로 잘못된 reducedmodel을 승인하지 않았다.
- n257,Delta4,p12는error4.08013e-7,bound~5.43e-7로tol1e-7을통과하지못했다. Adaptiveorder16은bound1.71924e-10로통과했고 별도oracle에서error1.14667e-10,ratio0.68933이었다.
- Delta0의 초정밀case는 binary64 DOP853의잡음~5.59e-17이 실제certificate~4e-21보다컸다. 이를방법실패라고해석하지않고 exactanalytical coalescent solution을100자리로검산했다. 관측error9.08485e-24,ratio0.00209048이었다. 다른case들의raw oracle violation은없었다.

Oracle는DOP853의두tolerance이며서로독립적인두알고리즘도formalvalidatedoracle도아니다. Certificate의근거는정리와exactcoeff arithmetic이다.

Exact unrounded same-map costs, p4,D6,S3:

| n | full Volterra terms | compiled terms | equality |
|---|---:|---:|---|
|7|1785|255|exact|
|31|7905|255|exact|
|127|32385|255|exact|

각각한번의runtime도보존했지만productionbenchmark나최적baseline대비speedup에사용하지않는다. Compiled쪽은momentsetup35/155/635항도기록했다. 두방식의nonlinear evaluations는각3회로같다.

## 11. Loop11에 바로 적용할 수 있는 별도 JVP 제거 항등식

이부분은mainpacket과다른,조건부구현개선이다. R0=(I-gamma0 hJ)^-1일때

    R0 hJ=(R0-I)/gamma0.

따라서oldjet V_(k+1)=R0 hJ V_k를

    V_(k+1)=(R0 V_k-V_k)/gamma0

로바꾸면명시적JVP를제거한다. Scaled Wk=gamma0^k Vk에서는 W_(k+1)=R0 W_k-W_k다. 기존dissipativity 가정에서||R0-I||<=1이고centeraction오차epsilon_k와subtractionrounding을추가하면errorrecurrence e_(k+1)<=e_k+epsilon_k이다. 모든target평가에서는rho^k 가중으로합친다.

하지만R0를내부JVP-Krylov로구하면전체JVP는0이아니다. 독립적인factorization/구조적inverse oracle가있는경우에만실제호출제거다. h작을때subtractioncancellation은absoluteerror로관리해야한다. Exact3x3예제에서5단계동일성을검사했으며fullnativeclient는구현하지않았다. Mainpacket은이inverse조차없으므로더강한derivative-free계약을갖는다.

## 12. 한계와 다음 코드 적용

이번문제는low-rank(mean)feedback이명시된구조화된oscillatoryensemble이다. Arbitrarylocalnonlinearity g_j(a_j), unknownnonormalbasis, localreactiondiffusionfullrank비선형에동일압축을자동적용하지않는다. 실제Vigilode의현재repo를읽지않았으므로현재코드에서이구조가존재한다고추정하지않는다.

Model compiler의전제는knownlinearflow+collectiveobservables+원방정식identity다. Polynomial모델에서는projectability/idealidentity를offline검사하고, symmetry에서는initialtransversedata와정칙chart를남긴다. Unknownstructure는기존genericfallback으로보내야한다. 이번에원래기하sourcefullsuite를다시실행하지않았고누락authorinput을추측복원하지않았다.

다음미완료과제: commonkappa>0 kernel의actualclient, rank-rfeedback과여러frequencyclusters, genuinevaryingimplicitclosure, bitcomplexity/nativef64outward산술, fulladaptivewindowrestart, independentreview와properbaselines. 새해석결과를일반UA5차또는기존모든solver보다빠르다는주장으로승격하지않는다.

### Analytic coefficient 공간의 추가 정확화

8절에서 coefficient conjugation은 p*(tau)=overline{p(overline{tau})}다. Complex time에서 g를 holomorphic state function이라고 가정하는 것이 아니라 p와p*의 real-Banach coefficient algebra 또는 별도 complexified 변수쌍에서 고정점을 구한다. 실수tau의 물리 경로에서는 실제 conjugate와 일치한다. 따라서 nonholomorphic cubic에 Cauchy differentiation을 무비판적으로 적용한 증명이 아니다.

### 과거 Vigilode 적용 리뷰와의 접점

V01 적용리뷰와V02 native실행·남은과제 문서도 원ZIP에서 읽었다. Raw-stage RHS JVP 제거, Laguerre adjoint certificate, homotopy radius, cached callback epoch 문제는 이미 진행된 역사적 작업이다. 이를 다시새미완료로등록하지않는다. 이번 packet은genericdriver의7개RHS JVP를줄이는patch와다른,model-specific lane이다. 제품소스에반영하려면 실제해당model이 collective structure를만족하는지현재source를읽고판정해야한다. V02의당시게시상태와이후pushreceipt를현재HEAD또는새testresult로상속하지않는다.

### Trusted compiled descriptor

`certificate`의 Packet은 `compile_packet`이 원래frequency/weight/initialdata에서 생성한 exact descriptor라는계약을전제로한다. 임의외부Packet의mu,nu를위조해전달한경우까지검증하는보안도구가아니다. 다른언어의compiler나approximateFFTpacket을연결하려면momentdata의identity와roundingerror를별도로인증해야한다. 현재지위는trustedexactreference다.

### 강직도와 수렴 복잡도의 범위

8절의analytic coefficient 수렴·복잡도 정리E는 현재구현과같이 **kappa=0** 창에대한것이다. 6절의physicalkernel-tail오차정리가commonkappa>=0에서도성립한다는사실만으로 정리E의analytic-radius조건이나runtime비용이kappa에도독립이라고말하지않는다. Dampedexp를basis에그대로남기는actualclient와그analyticbound는미완료다.
