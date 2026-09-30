# R3: 서로 다른 입력의 공동 exp–φ polynomial action

Source `cc2cd041737e7ff543624d1b59893a3b4397369f`. 근거: derived / numerically checked / implementation-verified, 제한된 독립 Python prototype. 생산 backend·전체 floating-point certificate·실측 wall speedup은 미확립이다.

## 질문과 이번 진전

R2는 같은 v에 대한 Laguerre exp·φ1…φ4와 Chebyshev의 exp만 비교했다. 이번에는 **서로 다른 w0,…,w4**에 대해

\[
F(h,A,W)=\sum_{k=0}^4\varphi_k(hA)w_k,\qquad \varphi_0(z)=e^z
\]

를 두 방법 모두로 계산했다. 즉 공동 φ 구현과 different-vector 의미를 맞춘 비교다. scaled convention의 `w_k=h^k b_k` 변환은 별도 arithmetic lane의 책임이며 이 prototype의 입력은 이미 주어진 w_k다. `p(A)v` 자체는 대수적으로 Krylov 공간에 속한다. 여기서 교체하는 것은 Arnoldi/Lanczos 직교화·projection 알고리즘이며, polynomial이라는 이름만으로 Krylov span 바깥 방법이 되는 것은 아니다.

## 수학 계약과 직접 유도

A=Aᵀ≤0, spec(A)⊂[−ρ,−λ], 0≤λ≤ρ, h≥0을 가정한다. A,ρ,λ의 단위는 시간⁻¹, h는 시간이다. ρ>λ일 때

\[
X=\frac{A+(\rho+\lambda)I/2}{(\rho-\lambda)/2},\quad
a=-h(\rho+\lambda)/2,\quad b=h(\rho-\lambda)/2,
\]

이므로 spec(X)⊂[−1,1], hA=aI+bX이다. spectral theorem으로 ||T_n(X)||₂≤1이다. 이 전제는 eigenvalue 위치만 아는 비정규 행렬에 성립하지 않는다. Prototype은 비대칭 입력을 거절하지만 임의 matrix-free callback의 spectral enclosure를 인증하지 않는다.

Modified Bessel 생성식에서

\[
e^{u(aI+bX)}=e^{ua}\sum_{n=0}^{\infty}(2-\delta_{n0})I_n(ub)T_n(X).
\]

φ 적분 표현을 적용하면 k≥1에 대해

\[
c_{n,k}=\frac{2-\delta_{n0}}{(k-1)!}\int_0^1
e^{ua}I_n(ub)(1-u)^{k-1}\,du\ge0,
\qquad \varphi_k(hA)=\sum_{n\ge0}c_{n,k}T_n(X).
\]

직접 exp 계수는 c_{n,0}=(2−δn0)eᵃI_n(b)다. 구현은 overflow를 피하려고 `exp(-h*lambda*u)*ive(n,b*u)`를 사용한다. 이 항등식은 u,b≥0에 한정한다.

### 공동 truncation bound

독립 Poisson(b/2) 두 변수의 차이 S는 P(S=n)=e⁻ᵇI_|n|(b)이고, E exp(θS)=exp[b(coshθ−1)]이다. r=m+1, θ>0이면 Chernoff와 대칭성으로

\[
\sum_{n>m}2e^aI_n(b)=e^{a+b}P(|S|\ge r)
\le2\exp\{b(\cosh\theta-1)-r\theta\},
\]

여기서 a+b=−hλ≤0을 사용했다. u∈[0,1]에도 같은 θ를 쓰면 ub≤b 및 e^{u(a+b)}≤1 때문에 같은 상계가 적용된다. φ의 적분 가중치를 적분하면

\[
\left\|\varphi_k(hA)w_k-\sum_{n=0}^m c_{n,k}T_n(X)w_k\right\|_2
\le \frac{C_m(b)}{k!}\|w_k\|_2,\quad k=0,\ldots,4,
\]

\[
C_m(b)=2\exp\!\left[\sqrt{b^2+r^2}-b-r\operatorname{arsinh}(r/b)\right].
\]

따라서 fused truncation은 C_m(b)Σ||w_k||₂/k! 이하이다. 최적 θ=arsinh(r/b)는 지수의 1차 미분 0, 2차 미분 b coshθ>0으로 얻는다. Wolfram이 최적점·볼록성과 φ1…φ4 적분 항등식을 exact 검산했다(`WOLFRAM_CHECK.json`). b=0은 이 식의 singular limit를 수치 계산하지 않고 scalar-matrix branch로 처리한다. h=0 및 A=0은 φ_k(0)=1/k!를 직접 적용한다.

이는 **exact arithmetic의 truncation theorem**이다. 선택된 degree의 계산, quadrature, `ive`, recurrence와 summation의 rounding은 인증되지 않았다. 128/256 Gauss–Legendre 차이는 진단이며 엄밀 오차 상계가 아니다. 전체 인증으로 승격하려면 이 항들을 enclosure로 합쳐야 한다.

### Laguerre와 작업량 의미

Laguerre는 R2의 exact-arithmetic 상계 e^{L/2}q^{m+1}Σ||w_k||₂/k!를 유지하고 L∈{1,2,4,8,16}에서 degree를 선택한다. cap16은 이전 탐색에 기반한 정책으로 독립 holdout에서 최적성을 입증한 것이 아니다.

각 recurrence는 n×5 block W에 작용한다. degree m은 **m개의 block operator product, 5m개의 vector-equivalent product**다. 다섯 벡터를 한 벡터 JVP 비용으로 계산했다는 뜻이 아니다. 현재 수치는 NumPy dense matrix-block 곱이며 실제 Rust `WorkCounters` 측정이 아니다. 계수 계산·배치 메모리·cache·thread overhead를 포함한 시간은 비교하지 않았다.

## 실제 계산

`joint_polynomial_probe.py`를 Python3.12.14/NumPy2.3.5/SciPy1.17.0에서 실행했다. 두 방법은 같은 A,W,h와 공통 fused truncation budget10⁻¹², 관측 L2오차 목표10⁻¹⁰을 사용했다. λ 최소치를 사용한 spectral interval은 각 입력의 해석적 구성에서 알려진다.

- diagonal24: −diag(geomspace(0.1,100,24)), hρ=0.1,1,10,100.
- hadamard4: exact orthogonal Hadamard Q/2, eigenvalues −(0.125,1,8,64), hρ=6.4×10⁻¹¹,0.64,6.4,64.
- semidefinite4: 같은 Q, eigenvalues −(0,0.125,1,8), hρ=0.8,8.

독립 reference는 binary64 h·eigenvalue의 곱을 Decimal120에서 계산한 scalar exp/φ recurrence와 알려진 Q를 통한 exact Decimal 행렬 합이다. 기존 계산의 `h*eigenvalue`를 먼저 binary64로 반올림하던 초기 결과도 `RESULTS_INITIAL_ROUNDED_Z.json`에 보존했으며 최종 `RESULTS.json`은 이 oracle 표현을 강화한 결과다. h=0/A=0/scalar branch는 별도 150항 Decimal series와 비교했다. 일반 cancellation sensitivity는 `condition_proxy`로 표시하며 상대오차 전체 보증을 하지 않는다.

| diagonal24 hρ | Laguerre block products | Chebyshev block products | Laguerre fused L2오차 | Chebyshev fused L2오차 |
|---:|---:|---:|---:|---:|
|0.1|7|6|1.48e−16|1.83e−15|
|1|12|10|4.25e−15|5.50e−15|
|10|38|21|3.62e−16|6.78e−15|
|100|245|56|5.82e−15|1.07e−14|

20개 fused 계산과 100개 개별 function-column 비교가 모두 목표 이하였고 최대 fused L2오차는 **3.50×10⁻¹⁴**였다. 6개 scalar/zero 경계 비교가 10⁻¹⁵ 이내였으며 비정규 Jordan 입력은 두 backend 모두 domain error로 거절했다. 이 거절이 모든 비정규 문제를 풀었다는 뜻은 아니다.

마지막 행의 recurrence 비용 비는 245/56≈4.38이다. 오차가 둘 다 허용 범위인 이 입력에서 Chebyshev가 적은 operator product를 필요로 한다는 관측이다. **4.38배 속도 향상을 측정한 결과가 아니다.** R2의 209 degree와 비교할 때에는 이번 더 엄격한 truncation budget과 distinct-vector norm factor가 다름을 고려해야 한다.

## 건설적 결정과 다음 구현

대칭·비양정 연산자 구간에서는 **Chebyshev 공동 φ를 우선 backend 후보**, Laguerre를 독립 비교 및 장기 frozen-operator 재사용 후보로 둔다. 원래의 non-Krylov 탐색 동기는 보존되지만 Laguerre 우월성을 전제하지 않는다. Leja는 일반 nonnormal domain의 추가 후보이며 이번 구현·검증은 NOT_RUN이다.

1. Rust API는 `sum phi_k(hA) w_k`와 `sum h^k phi_k(hA)b_k`를 분리하고 weighting-loss status를 반드시 전달한다.
2. same-v output 묶음과 different-v block 묶음의 저장·operator-cost 계약을 구분한다. 직교화가 사라져도 recurrence의 degree 방향 의존성은 남는다.
3. 사용자 지정 spectral enclosure의 근거 또는 검증 가능한 symmetric operator capability를 받는다. 근거 없는 eigenvalue 추정만으로 일반 matrix-free 행렬을 수락하지 않는다.
4. quadrature·Bessel coefficient·recurrence roundoff·summation·fused cancellation 오차 budget을 별도로 설계한다. 총 상계가 없으면 `EstimateOnly`와 검증된 domain을 출력한다.
5. allocation, block matvec, coefficient setup/reuse, fallback을 포함한 work–accuracy 측정을 별도 paired protocol로 실행한다. 현재 Python small dense cost는 그 gate를 통과시키지 않는다.

## 문헌 확인 범위

NIST [DLMF10.35.1–2](https://dlmf.nist.gov/10.35)는 modified Bessel 생성식/Jacobi–Anger expansion의 직접 근거다. 위 공동 φ Chernoff bound는 그 항등식에서 이번에 직접 유도했다. [Caliari et al.](https://arxiv.org/abs/1506.08665)의 Leja backward-error 연구와 [Deka et al.](https://arxiv.org/abs/2211.08948)의 Leja/Krylov 비교는 operator·integrator별 비교가 필요함을 뒷받침하며 이번에는 abstract scope를 재확인했다. SciSpace discovery는 별도 JSON에 보존했고 이를 full-text 증명 검증으로 대체하지 않았다. 시간함수의 Laguerre series/shifted-solve 계열과 여기의 matrix-argument polynomial recurrence를 동일 알고리즘으로 취급하지 않는다.
