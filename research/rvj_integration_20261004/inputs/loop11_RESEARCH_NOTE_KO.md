# RVJ 독립 연구 루프 11
## Homotopy–involution에 의한 다중모드 연산 공유: 퇴화점, local algebra, FFT predictor

날짜: 2026-10-04. 모든 연구는 exploratory다. Vigilode 저장소·PR·현재 local 검증을 읽거나 변경하지 않았다. 이번 결과는 직접 유도, 연결 Wolfram 항등식 검산, exact rational reference, f64 numerical probe, 부모 원방정식 certificate를 구분한다. 독립 최종 reviewer가 없으므로 방법 승격은 HOLD다.

## 0. 질문에 대한 판정

사용자의 직관을 가장 생산적으로 정식화하면 ‘여러 독립 모드를 값 하나로 없애고 복원한다’가 아니라 **‘여러 점을 중복도와 방향 정보를 가진 한 비축약점으로 모아, 그 local algebra 안에서 공통 연산을 수행하고 각 모드에 평가한다’**가 된다. 한 점의 support와 그 점의 scheme length 또는 jet 차원은 다르다.

이 차이는 부정적인 말장난이 아니다. 이번에는 이를 실제 세 가지 구성으로 만들었다.

1. 포물선의 normal-foot cubic에서 세 점이 유한한 cusp에 합쳐지는 monic family와 길이 3인 quotient algebra를 만들었다. Sign involution은 even/odd 계수를 재사용하되 odd 정보를 삭제하지 않는다.
2. 현재 dissipative J의 여러 가까운 shifted inverse를 공통 중심의 jet으로 계산했다. 65개 shifts에 하나의 factorization을 공유하면서 강직도에 무관한 truncation bound와 current-target residual을 제공했다.
3. 부모 Loop10의 실제 비선형 Fourier–Volterra 후보 생성에 f64 FFT를 붙이고, 결과는 exact binary rational로 고정한 뒤 **변경하지 않은 부모 certificate**로 수락했다. FFT roundtrip을 정확성 근거로 사용하지 않았다.

이 셋을 하나의 production integrator로 통합한 것은 아니다. 기하학적 witness, 연산자의 eigenmode, Fourier coefficient는 서로 다른 객체이며, 그 사이의 명시적 표현 사상과 dynamics의 결속이 필요하다.

## 1. 원문이 실제로 지지하는 구조

선택한 원본은 curved-mirror **v12 source ZIP**과 algebraic_geometry_stage6_wolfram_release ZIP이다. Library에서 v13 metadata도 검색되었지만 v12를 조용히 대체하지 않았다. 원 archive의 hash/크기는 SOURCE_BINDING.json, 읽은 원문 범위는 SOURCE_MAP_KO.md다. 전체 source suite는 재실행하지 않았고 authentic missing-input blocker를 유지한다.

v12의 `05_compact_source.tex`는 m(t)=(t,t²/2), p_λ=(λ/2,5/2)에 대해 **normal-foot 조건** (p−m)·m'=0에서 t³−3t−λ=0을 얻고, 각 witness의 reflected target을 2m−p로 정의한다. 그러므로 cubic은 고정 중심에 대한 점대칭이 다가 함수로 변한 것이 아니라 **가능한 normal foot의 다중성**이다. v12는 witness 데이터와 target support를 구분한다.

`07_algebraic_track_c.tex`의 (m,d)↦(m,−d)는 outer endpoint 교환이다. `08_boundary_audit.tex`의 2P−I는 선형 idempotent P에서 얻은 involution이다. Stage6 `04_residual.tex`의 길이 반감 정리는 fixed-point-free witness swap에 대한 것으로, 임의의 고정점을 가진 sign action에 그대로 적용할 수 없다. 같은 원문은 nonreduced length를 보존한다.

Stage6 `05_midpoints.tex`의 cubic Taylor 분해는 F(m±λn)=F(m)±λL+λ²H/2±λ³T/6이다. λ≠0에서 even/odd 부분을 분리해 조건을 얻는다. 이는 퇴화 한계에서 derivative·multiplicity 정보를 버리지 않아야 한다는 직접적인 계산 맥락이다. 원문 자체가 아래 적분기 비용 감소나 모드 압축 정리를 증명하는 것은 아니다.

## 2. 직선화와 유한 점의 합체는 다른 퇴화다

### 2.1 직선으로 평탄화하면 두 근은 무한대로 갈 수 있다

포물선 m_ε(t)=(t,εt²), source p=(X,Y)에 대해 normal 조건은

    2ε²t³+(1−2εY)t−X=0.

ε=0의 affine equation은 t=X다. 그러나 degree 3으로 homogenize하면

    2ε²T³+(1−2εY)TU²−XU³
       → U²(T−XU).

즉 projective special fibre에는 **유한점 하나와 무한대의 중복점**이 남는다. 일반적으로 세 점이 동일한 유한점으로 모인 것이 아니다. 복소근과 중복도를 포함한 진술이며, 실근 수는 그 전에 discriminant를 건너 바뀔 수 있다. 평탄화 homotopy의 전체 곡선 family가 smooth isotopy라는 주장도 아니다.

또한 임의의 이차곡선의 normal 식이 항상 cubic인 것은 아니다. 타원 (a cosθ,b sinθ), a≠b에서 t=tan(θ/2)를 사용하면

    2(a²−b²)t(1−t²)−2aXt(1+t²)+bY(1−t⁴)=0

이라는 generic quartic이 나온다. Circle specialization에는 표현상 분모 인자가 생기므로 chart를 구분해야 한다. 이 계산은 general conic degree의 보편적 분류를 주장하기 위한 것이 아니라 포물선 cubic의 적용 범위를 명시하기 위한 것이다.

### 2.2 같은 원문 포물선에서 실제 유한 합체를 구성할 수 있다

m(t)=(t,t²/2)를 고정하고 source를

    p_(s,μ)=(μ/2,1+s²/2)

로 두면 normal-foot equation은

    f_(s,μ)(t)=t³−s²t−μ=0,
    Disc_t f=4s⁶−27μ².

μ=0에서는 {0,+s,−s}가 s→0에 한 점으로 합쳐진다. 이는 사용자의 직관을 살리는 명시적 cusp family다. 다만 거울을 직선으로 만드는 경로 대신 source를 cusp로 이동시킨 **다른, 명시된 homotopy**다.

Sign involution t↦−t는 μ=0 fibre에서만 보존된다. 일반적으로

    f_(s,μ)(−t)+f_(s,μ)(t)=−2μ,

이므로 μ≠0이면 같은 fibre의 대칭이 아니라 μ↦−μ를 포함하는 family 대칭이다. Homotopy 전체가 involution과 호환되는지 검사하지 않으면 ‘짝 모드 절반 계산’이 정당화되지 않는다.

## 3. 한 점에 무엇을 남겨야 하는가: flat local algebra

### 정리 A: support 합체는 algebra의 차원 감소가 아니다

    A_s=C[ξ]/(ξ³−s²ξ)

를 생각하자. 모든 s에서 {1,ξ,ξ²}가 basis다. Monic polynomial division이 유일한 degree<3 remainder를 주므로 C[s] 위 family도 free rank 3이다. 이것이 여기서 사용하는 finite flat degeneration의 직접적인 증명이다.

s≠0일 때 평가 사상

    ev_s: A_s → C³,
    c0+c1ξ+c2ξ² ↦ (c0,c0+s c1+s²c2,c0−s c1+s²c2)

은 대수 동형이다. 행렬은

    V_s=[[1,0,0],[1,s,s²],[1,−s,s²]], det(V_s)=2s³.

s=0에서는 A_0=C[ξ]/ξ³이다. Support는 한 점이지만 ξ와 ξ²는 0이 아니고 ξ³만 0이다. 따라서 **중심값, 1차 방향, 2차 방향**에 해당하는 세 coefficient가 남는다. 이를 reduced algebra C로 바꾸면 두 방향의 정보가 사라진다.

역평가는

    c0=y0,
    c1=(y+−y−)/(2s),
    c2=(y++y−−2y0)/(2s²)

다. 각 입력값 오차가 ε_sample 이하이면 c1 오차는 ε_sample/|s|, c2 오차는 2ε_sample/|s|²까지 커질 수 있다. 이것은 특히 **jet coordinate를 값의 차분으로 복구하는 conditioning**이다. 물리 모드값 자체의 오차와 같은 양이라고 해석하면 안 된다.

### 독립 자유도 압축의 반례

s=0에서 (1,0,0)과 (1,2,3)은 같은 세 evaluated value (1,1,1)을 주지만, algebra product는 다르다. 단일 point value로는 두 경우를 구분하지 못한다. 더 일반적으로 열린 n차원 상태공간을 smooth encoder E:U→R^r, r<n, decoder D로 D∘E=id가 되도록 압축할 수 없다. Chain rule에서 identity의 rank n이 r 이하가 되어 모순이다. 이는 arbitrary set-theoretic encoding 같은 비연속 장치를 논하는 주장이 아니다.

**Homotopy inversion에는 최종 점의 값만 아니라 family, multiplicity structure, branch 또는 amplitude의 결속 데이터가 필요하다.** 하나의 유한 jet 역시 임의의 analytic family 전체를 결정하지 않는다. f와 f+ξ³는 같은 2-jet을 갖지만 떨어진 점에서 다르다.

### 일반 m모드 확장

고정된 서로 다른 λ_j에 대해 P_s(ξ)=∏_(j=1)^m(ξ−sλ_j)는 monic degree m이다. C[s,ξ]/P_s는 free rank m이며 special fibre는 C[ξ]/ξ^m이다. 중앙 support는 하나지만 길이는 m이다. ± symmetry는 {λ_j}가 부호반전에 대해 닫힐 때만 algebra involution을 제공한다. Arbitrary spectrum 또는 arbitrary mode amplitudes에 같은 대칭이 자동으로 존재하지 않는다.

## 4. Involution은 두 sector를 보존하며 계산을 공유한다

A_s에서 J(c0,c1,c2)=(c0,−c1,c2)로 두면

    J²=I,
    J(ab)=J(a)J(b),
    Π_even=(I+J)/2,
    Π_odd=(I−J)/2.

Even sector의 차원은 2, odd sector는 1이다. Odd sector를 0으로 만들면 projection이지 역가능 대칭 활용이 아니다. 여기에는 fixed central root가 있으므로 Stage6의 free witness quotient에서 나온 길이/2 공식을 적용할 수 없다.

ξ의 multiplication matrix는

    M_s=[[0,0,0],[1,0,s²],[0,1,0]],
    M_s³=s²M_s,
    S M_s S=−M_s, S=diag(1,−1,1).

s=0에서 rank(M_0)=2, rank(M_0²)=1이며 nonzero nilpotent다. 반면 D_s=diag(0,s,−s)는 D_0=0이다. s≠0에서 V_sM_s=D_sV_s지만 V_s가 퇴화한다. **같은 합쳐진 eigenvalues를 갖는다는 사실만으로 semisimple physical limit와 nilpotent algebra-coordinate limit를 동일시할 수 없다.**

### Analytic transfer의 정확한 묶음 표현

    exp(a+bξ)=exp(a)[1+sinh(bs)/s ξ+(cosh(bs)−1)/s² ξ²].

이는 M_s에 대한 미분방정식 및 초기값 또는 세 점 평가로 증명할 수 있다. s=0의 removable limit는 exp(a)(1+bξ+b²ξ²/2)다.

직접 차분 대신 sinhc(z)=sinh(z)/z, sinhc(0)=1을 사용하면

    c1=exp(a)b sinhc(bs),
    c2=exp(a)b²/2 sinhc(bs/2)²

로 near-confluence cancellation을 피한다. 초월함수 f64 구현은 directed certificate가 아닌 numerical probe다.

7개 s값의 80자리 비교에서 stable coefficient error는 최대 1.15e−16이었다. Naive second difference는 s=1e−8에서 약0.222 오차가 나왔다. 이는 중심 coefficient의 계산 문제이며, naive modal output 전체가 같은 상대오차를 갖는다는 진술은 아니다.

### ‘한 번의 비선형 계산’의 정확한 의미

Algebra-valued U'=U²는 pointwise evaluation 아래 세 scalar equation을 동시에 표현한다. U(t)=U0(1−tU0)^−1인 영역에서 exact rational solution을 계산했다. s=0에서

    c0'=c0²,
    c1'=2c0c1,
    c2'=2c0c2+c1².

초기 (a,b,c)에 대해

    c0=a/(1−at),
    c1=b/(1−at)²,
    c2=c/(1−at)²+t b²/(1−at)³.

s=0,10^−6,0.1,1에서 algebra solution과 세 scalar solutions가 정확히 일치했다. 한 algebra-valued evaluation이지만 여전히 세 coefficient를 계산한다. General coupled Fourier modes의 convolution을 독립 scalar dynamics로 바꾼 결과가 아니다. 그런 coupling은 quotient 표현에서도 별도의 선형·비선형 연산자로 남는다.

## 5. 실제 비용 절감 정리: homotopy 중심 하나의 shifted resolvent jet

### 가정과 단위

현재 J가 고정 positive Hermitian metric H에서 dissipative라고 하자.

    J*H+HJ <= 0.

h>=0, real γ0>0이다. J는 inverse time, hJ와 γ0는 dimensionless다. 일반 physical state의 H scaling은 명시적으로 보존해야 한다. 아래 구현은 Euclidean norm과 real positive target shifts에 한정한다. 고유값 부호만으로 dissipativity를 대신하지 않는다.

    P=I−γ0hJ, R0=P^−1, K=hR0J.

### 정리 B: stiffness-independent jet radius

    ||R0||_H <= 1,    ||K||_H <= 1/γ0.

증명: 모든 v에 대해

    ||Pv||_H²=||v||_H²−2γ0h Re<v,Jv>_H+γ0²h²||Jv||_H²
              >=||v||_H²+γ0²h²||Jv||_H².

첫 경계는 바로 나오고, v=R0u를 대입한 뒤 JR0=R0J를 사용하면 두 번째가 나온다. J의 normality나 diagonalizability는 필요 없다.

γ=γ0+δ, |δ|<γ0이면

    (I−γhJ)^−1B=(I−δK)^−1R0B
               =Σ_(k>=0) δ^k V_k,
    V0=R0B, V_(k+1)=R0(hJV_k).

따라서 degree p 근사에 대해 ρ=|δ|/γ0<1이면

    ||error||_H <= [ρ^(p+1)/(1−ρ)] ||R0B||_H.

이는 large stiffness를 작은 값으로 바꾸거나 target operator를 stale W로 대체한 정리가 아니다. Homotopy center γ0에서 계산한 자료로 **현재 모든 γ target**을 근사한다.

정확한 산술의 target residual은

    B−(I−(γ0+δ)hJ)U_p = δ^(p+1)hJ K^p R0B.

실제 positive real target γ에 대해서는 다시 inverse contractivity로 ||error||<=||target residual||이다. Floating implementation에서 rigorous certificate를 얻으려면 실제 residual의 rounding/action error enclosure가 필요하다. 현재 f64 probe의 residual은 수치 diagnostic이며 interval certificate가 아니다.

### Involution을 이용하는 평가

    E_δ=Σ δ^(2j)V_(2j), O_δ=Σ δ^(2j+1)V_(2j+1)

를 공유하면 U_(+δ)=E_δ+O_δ, U_(−δ)=E_δ−O_δ다. 서로 다른 두 inverse를 새로 factorize하지 않는다. RHS가 다른 경우 공통 RHS basis B와 계수들을 유지해야 한다. 임의의 독립 RHS를 하나로 대체하면 안 된다.

### 비용 및 실제 실행

Exact fixture는

    J0=[[-1,-80,0],[80,-4,-30],[0,30,-9]],
    J0+J0^T=diag(-2,-8,-18).

이는 비정규 dissipative 행렬이다. γ0=1/2, δ_j=γ0*j/160, j=−32,...,32를 써서 65 targets와 ρ_max=1/5를 만들었다. p=17이면 coefficient tail factor가 1e−12보다 작다.

4개의 (scale,h)=(1,1/1000),(1,1/8),(10^4,1/8),(10^8,1/8)에서 각 65 targets를 exact Fraction으로 계산했다. 각 target에 대해 series bound와 true residual norm 경계를 따로 검사해 총520개 exact inequalities가 성립했다. 최대 actual vector error는 두 번째 case의 약9.20e−14다.

Exact reference는 중심 inverse를 한 번 구해 사용한다. ‘18 solves’는 inverse를 이미 구한 뒤의 18 center-RHS action에 대응하는 비용 의미다. 별도의 18 Gauss–Jordan factorization을 했다는 뜻은 아니다.

48차원 f64 probe는 J=S−S^T−diag(d_i), d_i>0와 rank2 B를 사용했다. 65개 개별 LU baseline은 65 factorizations/130 scalar RHS solves이고, shared jet은 **1 factorization/18 block solves/36 scalar RHS solves**다. 최대 관측 Frobenius error는 2.0544e−14, target residual norm은1.2081e−12였다.

그러나 jet storage는1728 scalars, output는6240 scalars이며 recurrence depth는 **18**이다. 최적 multi-shift Krylov, Schur reuse, rational interpolation baseline보다 빠르다는 비교는 하지 않았다. 기존 RODAS가 이미 common γ를 쓰는 stage들에는 ‘새 factorization 절감’이 없을 수 있다. 미래 stage RHS가 준비되지 않았으면 이 기법으로 그 인과적 의존성을 없앨 수도 없다.

이 방법이 유리한 조건은 중심 재사용이 비싸게 만드는 factorization/setup을 여러 번 피하고, 대상 shifts가 좁게 모이며, 공통 RHS span이 작고, 요구 jet degree가 대상 수보다 충분히 작을 때다. 넓은 cluster, near pole, 불확실한 inverse bound, 큰 독립 RHS rank에서는 재분할하거나 다른 action 후보로 돌아가야 한다.

## 6. 진동 문제의 추가 조건: 공통 carrier와 cluster 폭을 분리한다

    exp[it(ω0+δ)] = exp(iω0t) Σ_(k>=0)(itδ)^k/k!.

많은 가까운 frequency에 공통 carrier evaluation과 coefficient recurrence를 공유할 수 있다. 그러나 오차는 |δ|t, 즉 **cluster width times horizon**에 의존한다. Real θ에 대한 integral Taylor remainder에서

    |exp(iθ)−Σ_(k=0)^p(iθ)^k/k!| <= |θ|^(p+1)/(p+1)!

를 얻는다. θ=100,p=8에서 actual error는 약2.4742e11이었다. ‘여러 mode를 중심에 모았다’는 이유로 phase separation을 무료로 복원할 수 없다. Carrier의 평가 정밀도와 independent modal amplitude도 남겨야 한다.

### Cauchy–FFT로 derivative-free central jet을 얻는 연결

f가 |z−σ|<=R의 근방에서 analytic이고 |f|<=M이라고 하자. 0<r<R, ζ_l=exp(2πil/N)에서

    a_hat_j=(1/(N r^j)) Σ_l f(σ+rζ_l) ζ_l^(−j), 0<=j<N

를 계산하면

    a_hat_j=Σ_(q>=0) a_(j+qN) r^(qN).

따라서 analytic alias는

    |a_hat_j−a_j| <= M/R^j * (r/R)^N/[1−(r/R)^N]

로 감싸진다. Sample error ε_s가 있으면 ε_s/r^j가 추가되고 FFT rounding도 별도로 필요하다. r를 무작정 작게 하면 alias는 줄어도 data error 증폭이 커진다.

이 방식은 N개의 function value와 FFT를 사용한다. ‘단 한 점의 한 scalar evaluation’이 아니다. f(z)=1/(2−z)의7개 coefficient를 시험했지만 native directed FFT 또는 일반 ODE derivative provider를 구현한 것은 아니다. 또한 complex conjugate를 포함한 physical RHS는 holomorphic f가 아니므로 naive Cauchy formula를 직접 적용할 수 없다. Analytic parameter family 또는 적절한 complexification과 그 domain을 별도로 마련해야 한다.

이는 기존 Schur–Parlett/근접 eigenvalue의 divided-difference 계산과 연결된다. Davies–Higham과 Higham–Liu의 원전 abstract는 가까운 spectral block에서 Taylor 또는 derivative-free higher-precision 계산이 쓰이며, blocking/precision tradeoff가 필요함을 명시한다. 그러므로 보편적으로 새로운 함수 계산법을 발견했다고 주장하지 않는다. 이번의 산출물은 사용자의 homotopy 직관을 multiplicity-preserving algebra와 current-target residual 계약으로 연결한 것이다.

## 7. 앞서 제안한 실제 다음 단계: FFT predictor를 원 certificate에 연결

부모 Loop10의 conservative two-complex-mode model과 invariant leaf(q=5/4)를 그대로 사용했다. Damping/general varying closure를 추가했다고 하지 않는다.

새 FFT predictor의 첫 축은 harmonic index k, 둘째 축은 polynomial degree j다. **둘째 축은 physical time이 아니다.** Polynomial coefficient convolution도 zero-padding으로 계산하는 것이다. Conjugation은 (k,j)↦(−k,j)와 coefficient conjugate를 먼저 적용한다. Complex evaluation grid에서 단순 conjugation하면 polynomial index까지 반전되어 다른 곱이 되므로 그렇게 하지 않는다.

입력 harmonic band K, polynomial degree p, phase band s/degree q이면 full cubic RHS support는 harmonic 3K+s, degree3p+q 이하다. 구현은 전체곱을 보존하려고

    N_k >= 2(3K+s)+1, N_j >=3p+q+1

이상의 길이를 사용한다. Retained band만 맞추는 것보다 강한 조건이다. Time primitive가 높은 polynomial degree를 낮은 coefficient에 전달할 수 있으므로 조기에 아무 항이나 삭제하지 않는다.

한 RHS 평가에 5 forward FFT2, 2 inverse FFT2가 사용된다. 모든 유한 f64 output을 exact binary Fraction으로 고정한 뒤 부모의 원래 noncyclic exact RHS와 Bernstein/full-target certificate를 호출한다. FFT rounding이 없다고 가정하지 않는다. 최종 저장 후보의 residual에 그 영향이 남고, 통과하지 못하면 거절된다. 이 검증은 일반 악의적 certificate에 대한 보안 인증이 아니라 trusted reference execution의 수학적 계약이다.

### 동일 조건 비교

Ω=40,10^4의 두 parameter settings에서 exact predictor와 FFT predictor를 각각 실행했다. T=1/2,tol1e−8, 같은 candidate registry(K2,p4,2sweeps → K3,p6,4sweeps), 같은 parent certificate와 physical reconstruction이다.

| Ω | predictor | accepted | RHS calls | 관측 최대 amplitude error |
|---|---|---:|---:|---:|
|40|exact coefficient|4|24|2.21752e−13|
|40|f64 FFT|4|24|2.21699e−13|
|10000|exact coefficient|4|24|1.81633e−13|
|10000|f64 FFT|4|24|1.81635e−13|

각 FFT run은8candidatebuilds/4rejections,120forward/48inverse FFT2,누적3168 padded-grid points,최대420 grid points를 기록했다. 이는 전체 allocation/FLOP/communication 비용을 대표하지 않는다. Evidence에는 single-run timing도 남겼지만 robust timing campaign이나 production speedup으로 사용하지 않았다.

총4runs/16acceptedsteps/48interiordiagnostics에서 DOP853 두 tolerance oracle와의 raw bound violations는0이다. Oracle는 원래 sqrt closure를 평가하지만 floating numerical check이지 formal validated oracle가 아니다. 마지막에는 모든16savedpath를 변경하지 않은 exact parent certificate로 다시 계산하여 residual,error,endpoint,physical bound가 정확히 일치함을 확인했다. 전체 candidate generation을 다시 재생성한 것과 구분한다.

원 계약의 ‘max parent full trajectories 2’는 두 물리 parameter settings라는 의도였으나 실제 saved outputs는 exact/FFT pair로4runs다. 이 구분을 결과 뒤에 숨기지 않고 명시하며, 모두 exploratory로 유지한다.

## 8. 무엇이 계산량 감소이고 무엇은 아닌가

1. Geometric roots를 동일 support로 보내는 것만으로 state dimension은 감소하지 않는다.
2. Local algebra는 모든 modal value를 한 자료구조의 algebra-valued 연산으로 **묶는다**. 유효 연산량 감소는 kernel reuse, 낮은 jet degree, symmetry, common RHS structure에 달려 있다.
3. Involution은 정확히 호환되는 ± family에서 even/odd evaluation을 재사용한다. 한 대표 branch로 일반 dynamics가 복원되려면 dynamics와 initial data가 orbit/symmetry 제약을 만족해야 한다. 일반 independent amplitudes는 그대로 저장한다.
4. Homotopy는 target을 바꿔 쉽게 계산한 뒤 정확히 돌아오는 마법이 아니다. 실제 퇴화점을 지날 때 ordinary inverse는 singular할 수 있다. Family algebra, higher jets, uniform remainder와 branch labels가 필요하다.
5. 이번 concrete shift method는 singular eigenvector basis를 만들지 않고 target resolvent family의 공통 중심을 쓴다. 이 경우 homotopy 전체가 inverse의 analytic domain 안에 있어 적절한 조건 아래 유용하다.
6. FFT predictor는 물리 자유도를 지우지 않고 convolution의 알고리즘 복잡도와 kernel 구현을 바꾼다. Exact residual oracle를 유지했으므로 기존 alias/roundtrip 반례를 덮어쓰지 않는다.

## 9. 실패, 검증 지위 및 미수행 사항

- Reduced point만 남기는 quotient baseline, singular inverse evaluation, odd sector 삭제에 대한3개 RED를 보존했다. 정확 algebra/유효 domain 처리 후 GREEN이다.
- FFT RHS를 잘못 대체한 negative baseline의1개 RED를 실제 full-product 계산으로 고쳤다. 이는 연구 reference의 시험이며 Vigilode defect가 아니다.
- 최초 trajectory 호출은 부모 modules의 sys.path 영향으로 loop9의 동명 run_studies를 import해 실패했다. Source identity를 explicit file import로 고정해 수정했다. 이 실패는 수학이나 FFT 알고리즘의 실패가 아니라 implementation import collision이다.
- 7개 near-confluence transcendental probe와48dim f64 shifted solve는 certified-rounding 계산이 아니다. Exact Fraction algebra/resolvent inequalities 및 parent exact target certificate와 구별한다.
- Cauchy–FFT 연결은 기본 계획 후 추가한 exploratory 소규모 확장이다. FFT function evaluations, analytic alias, sample/rounding error를 구분했다.
- Current block factorization은 현재 J에 바인딩되며 stale W로 대체하지 않았다. Large Krylov, native Rust/parallelization, unknown carriers, 일반 coupled mode fusion 및 multivariate closure는 미수행이다.
- 부모의 전체 과거 trajectorycampaign과 원 geometry 전체 suite를 재실행하지 않았다. 읽고 사용한 parentcode/excerpt와 archive hash를 보존하며, authentic missing-input blocker는 유지한다.

최종 local outputs는 scoped derived / exact algebra checked / reference verified다. 독립 reviewer와 formal proof assistant가 없으므로 최종 방법승격은 HOLD다.

## 10. 다음 연구 결정

계속할 중심은 ‘scalar collapse → miraculous inverse’가 아니라 **multiplicity-preserving jet packet → shared current operator action → involution-aware evaluation → unchanged original-target certificate**다.

다음 작업은 (i) 중심 jet degree/cluster 폭과 common RHS rank를 actual cost에 맞춰 선택하는 prospective policy, (ii) independent RHS와 실제 nonlinear Fourier coupling을 함께 갖는 모델에서 이 shared operator를 기존 FFT predictor와 결합, (iii) 같은 physical error의 multi-shift Krylov·Schur reuse·derivative-free UA와 비교, (iv) native directed residual과 full cost 측정이다. Already-completed FFT predictor 자체를 다시 미구현이라고 제안하지 않는다. 진동 cluster에서는 |Δω|h와 horizon phase error를 반드시 별도 계상한다.
