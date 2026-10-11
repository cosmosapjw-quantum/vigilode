# 유한 몫대수로 polynomial action을 압축하는 긍정 정리

작성: 2026-10-11. 검토 대상 checkout: `8ce9bda` (`audit/rvj-reaudit-remaining-20261011`).
상태: **derived**, 신규 exact-rational 예제 검산 포함. production 구현·실행 시간·ODE 전역 정확도는 이 문서의 주장 범위가 아니다. 수학적 신규성은 주장하지 않는다. 기초는 matrix-function Hermite functional calculus이며, 기여 목표는 현재 VigilODE의 total-certificate / work-accounting 계약에 맞는 구체적 이식 설계다.

## 1. 기존 결과를 재사용하고 실제로 바꾸는 것

L0039의 signed Laguerre adjoint identity와 L0047의 symmetric total admission은 이미 공개됐다. PP09의 이전 재구성만으로 `exp(L/2)`를 확정 병목이라고 부르면 안 된다. PP08에서 Laguerre가 Chebyshev를 이겼다고 주장할 근거도 없다. `py01_laguerre_component_export_20261011/PREREGISTRATION.md`는 새 exporter의 계획이며, 이 읽기 범위에서 그 실행 결과를 확인했다고 주장하지 않는다. 이 문서는 과거 campaign을 반복하지 않았다.

이번 아이디어는 degree-m recurrence의 bound만 더 작게 만드는 일이 아니다. **현재 연산자가 만족하는 낮은 차수의 다항 관계를 인증할 수 있으면, recurrence 자체를 작은 quotient algebra에서 먼저 계산하고 실제 n차원 vector recurrence를 degree d−1로 바꾼다.** 그러면 과거 degree-m vector recurrence의 local defect와 adjoint amplification은 새 알고리즘에서 발생하지 않는다. 새 scalar coefficient 계산과 짧은 vector evaluation의 오차를 별도로 감싸야 한다.

일반 matrix-free callback에 낮은 차수 관계가 존재하거나 쉽게 발견된다고 가정하지 않는다. 이 경로는 projection, 적은 수의 spectral class, 선언된 nilpotent block / finite jet 구조, 또는 엄밀한 near-annihilator witness가 있는 경우를 위한 것이다.

## 2. 정리 P1 — near-annihilator에 의한 certified phi degree 감소

### 가정과 표기

1. X는 실대칭 또는 복소 Hermitian이고 `spec(X) ⊂ [-1,1]`이다. 모든 norm은 spectral 2-norm 및 대응 vector norm이다.
2. `A = c I + s X`, `s > 0`, `c+s ≤ 0`, `h ≥ 0`이다. c,s,h와 X가 가리키는 exact-real target을 고정한다.
3. `q(x)=∏_{j=1}^d (x−ξ_j)`는 monic degree d≥1이고, 모든 ξ_j가 [-1,1]에 있다. 반복 node는 허용한다. **현재 X 전체에 대해** `||q(X)|| ≤ δ`가 인증돼 있다.
4. `φ_0(z)=exp(z)`, `φ_k(z)=Σ_{j≥0} z^j/(j+k)!`이다. `r_k`는 `f_k(x)=φ_k(h(c+s x))`의 node ξ_j에서 multiplicity를 포함하는 degree<d Hermite interpolant이다.

그러면 k≥0에 대해

\[
\boxed{\|\varphi_k(hA)-r_k(X)\|\le
\delta\frac{(hs)^d}{(d+k)!}.}\tag{P1}
\]

특히 δ=0이면 `φ_k(hA)=r_k(X)`가 정확하다. n이나 h||A||가 커도 이 정확한 algebraic degree가 d이면 **d−1 operator products로 하나의 action을 계산**할 수 있다. 이는 연속 spectral interval 전체에서 작은 uniform approximation error를 얻기 위해 필요한 degree와 다른 정보다.

### 증명

k≥1에는 entire-function identity

\[
\varphi_k(z)=\frac1{(k-1)!}\int_0^1 e^{tz}(1-t)^{k-1}\,dt
\]

가 성립한다. exponential series를 적분하면 beta integral로 각 계수가 1/(j+k)!임을 직접 확인할 수 있다. 따라서 x∈[-1,1]에 대해 h(c+sx)≤0이므로

\[
|f_k^{(d)}(x)|\le
\frac{(hs)^d}{(k-1)!}\int_0^1t^d(1-t)^{k-1}dt
=(hs)^d\frac{d!}{(d+k)!}.
\]

k=0에는 `f_0^{(d)}=(hs)^d exp(h(c+sx))`이므로 같은 식의 k=0 경우가 성립한다. 실수 Hermite remainder theorem은

\[
f_k(x)-r_k(x)=\frac{f_k^{(d)}(\zeta_x)}{d!}q(x),
\qquad \zeta_x\in[-1,1]
\]

을 준다. node에서의 식은 양변 0이고, 다른 x에는 generalized Rolle theorem으로 얻는다. 반복 node의 도함수 조건을 포함해 총 d개 interpolation 조건을 쓴다. 그러므로 scalar absolute error는 `(hs)^d |q(x)|/(d+k)!` 이하이다. X의 unitary diagonalization에서 각 eigenvalue에 이 scalar 부등식을 적용하면 `max_{λ∈spec X}|q(λ)|=||q(X)||`이므로 P1이 성립한다. □

### joint phi와 finite input rank

`F=Σ_{k=0}^4 φ_k(hA)w_k`이면

\[
E_{\rm alg}=\delta (hs)^d\sum_{k=0}^4
\frac{\|w_k\|}{(d+k)!}
\]

가 degree-reduction error 상계다. 입력이 정확히 `w_k=Σ_{ℓ=1}^r a_{ℓk}v_ℓ`이면 작은 scalar polynomial

\[
p_\ell(x)=\sum_{k=0}^4 a_{\ell k}r_k(x)
\]

를 먼저 만들고 `Σ_ℓ p_ℓ(X)v_ℓ`을 평가한다. 총 X 적용은 r(d−1)회다. 기존 same-vector joint input은 r=1인 직접 적용점이다. 부동소수점 low-rank 추정만으로 r을 줄이면 안 된다. 실제 factorization residual `e_k=w_k−Σ_ℓa_{ℓk}v_ℓ`가 있으면

\[
E_{\rm input}\le\sum_{k=0}^4\frac{\|e_k\|}{k!}
\]

를 추가한다. 이는 symmetric A≤0에서 `||φ_k(hA)||≤1/k!`이기 때문이다. 축약 입력을 쓰는 경우 E_alg의 w_k는 **축약한 입력**으로 계산하고 E_input을 더한다. 원래 w_k와 축약 w_k를 혼용하지 않는다.

### finite-precision 계약

`p_ℓ(x)=Σ_{i=0}^{d−1}p_{ℓi}x^i`의 정확한 scalar coefficients가 outward intervals 안에 들어 있고, 선택한 binary64 coefficient `ĉ_{ℓi}`에 대해 `|ĉ_{ℓi}−p_{ℓi}|≤ε_{ℓi}`가 검증돼 있다고 하자. Horner evaluation의 각 실제 local defect를

\[
\widehat y_i=X\widehat y_{i+1}+\widehat c_i v+e_i,
\quad\|e_i\|\le\eta_i
\]

로 감싼다. 시작은 `ŷ_d=0`이다. 첫 단계는 zero에 대한 X callback을 생략할 수 있으므로 products는 d−1회다. local defect에는 callback approximation, multiplication/AXPY rounding, c,s로 X를 형성하는 rounding, underflow 손실을 전부 포함한다. 그러면

\[
\|\widehat y_0-p(X)v\|\le
\|v\|\sum_i\epsilon_i+\sum_i\eta_i
\]

이다. 이유는 coefficient error polynomial에서 `||X^i||≤1`이고, exact telescoping으로 evaluation error가 `Σ_i X^i e_i`이기 때문이다. 최종 column sum의 rounding 상계 E_sum을 더하면

\[
\boxed{E_{\rm total}=E_{\rm alg}+E_{\rm input}
+\sum_\ell(\|v_\ell\|\sum_i\epsilon_{\ell i}+\sum_i\eta_{\ell i})
+E_{\rm sum}.}\tag{P1-FP}
\]

모든 합·곱은 위쪽으로 반올림한다. overflow, nonfinite, uncertified callback error 또는 coefficient enclosure failure이면 admission은 거절한다. “||X||≤1”은 intermediate vectors가 overflow하지 않는다는 약속이 아니다. 정확 scalar polynomial의 monomial coefficients가 클 수 있으므로 충분한 precision 또는 다른 basis가 필요할 수 있다.

일반 weighted norm에 적용하려면 `SAS^{-1}`가 Hermitian이라는 현재 operator의 정확한 binding, metric transport와 S/S^{-1} 적용 오차까지 포함한다. eigenvalues만 실수라는 이유로 P1을 nonnormal A에 적용할 수 없다.

## 3. 정리 P2 — Laguerre recurrence의 quotient compilation

X와 q는 P1과 같은 정규화된 연산자와 monic polynomial이라고 하자. `Y=αI+βX`이며 기존 Laguerre polynomial이

\[
P_m(x)=\sum_{j=0}^m c_jL_j(\alpha+\beta x)
\]

라고 하자. 여기서 c_j는 **기존 계산이 실제 사용하는 stored coefficients를 exact-real로 해석한 값**이다. 표준 recurrence를

\[
a_j(x)=\frac{2j+1-\alpha-\beta x}{j+1},\quad
b_j=\frac j{j+1},\quad
L_{j+1}(\alpha+\beta x)=a_j(x)L_j(\alpha+\beta x)-b_jL_{j-1}(\alpha+\beta x)
\]

로 쓴다. `R_{−1}=0, R_0=1`에서

\[
R_{j+1}=\operatorname{rem}(a_jR_j-b_jR_{j-1},q)
\]

를 계산하고 `R=Σ_jc_jR_j`를 누적한다. 그러면 `deg R<d`이며 다음을 만족한다.

1. **정확성:** q(X)=0이면 `P_m(X)=R(X)`이다.
2. **scalar work:** 각 recurrence에서 degree<d polynomial을 affine polynomial과 곱하므로 degree≤d이다. monic q에 대한 나눗셈은 leading coefficient 하나를 제거하면 된다. 따라서 총 O(md) scalar arithmetic operations이다.
3. **memory:** 이전 두 remainder와 누적 R만 유지하므로 추가 scalar workspace O(d)이다. coefficients 저장 또는 기존 coefficient generator의 비용은 별도로 센다.
4. **vector work:** R(X)v는 d−1 products와 O(nd) vector arithmetic으로 평가한다. 기존 degree-m forward Laguerre evaluation은 m products를 사용한다. `d−1<m`이면 이 action의 product count를 정확히 `m−d+1`만큼 줄인다.

### 증명

다항식 remainder map은 quotient ring `K[x]/(q)`에서 덧셈과 곱셈을 보존한다. 초기값이 일치하므로 recurrence에 대한 귀납법으로 `R_j ≡ L_j(α+βx) mod q`이다. linear combination에도 보존되므로 `P_m−R=qT`인 polynomial T가 존재한다. X에 대입하면 `P_m(X)−R(X)=q(X)T(X)=0`이다. degree, work 및 memory 진술은 위의 한 번짜리 monic reduction과 streaming 구현에서 바로 따른다. □

### near-annihilator의 constructive 확장

각 step에서 정확히

\[
a_jR_j-b_jR_{j-1}=R_{j+1}+\tau_j q
\]

라고 쓰자. degree≤d이므로 τ_j는 scalar다. `T_{−1}=T_0=0`과

\[
T_{j+1}=a_jT_j-b_jT_{j-1}+\tau_j
\]

를 정의하면 `L_j(α+βx)−R_j=qT_j`다. `M_{−1}=M_0=0`에서

\[
M_{j+1}=A_jM_j+|b_j|M_{j-1}+|\tau_j|,
\quad A_j=\max_{x\in[-1,1]}|a_j(x)|
\]

를 위쪽으로 계산하면 `||T_j(X)||≤M_j`이다. 따라서

\[
\boxed{\|P_m(X)-R(X)\|\le
\delta\sum_{j=0}^m|c_j|M_j.}\tag{P2-near}
\]

scalar reduction이 finite precision이면 R 및 τ_j를 outward coefficient intervals로 포함하고, M에는 τ_j magnitude upper bound를 쓰며, R의 coefficient uncertainty는 P1-FP 식으로 처리한다. α,β의 interval 또는 representation error도 같은 방식으로 target에 맞게 포함한다. 이 보수적 M이 클 때는 P1의 직접 Hermite interpolant가 더 유리할 수 있다. 둘 중 작은 bound를 선택하더라도 setup/실패 비용을 누락하지 않는다.

### 기존 total certificate와의 연결

기존 알고리즘에서 `||φ_k(hA)−P_m(X)||≤E_spec`가 scalar truncation 및 coefficient uncertainty만으로 검증돼 있다면 새 algorithm은

`E_spec + E_quotient + E_short_evaluation + E_input + E_output_sum`

을 쓴다. **이전 m-step vector recurrence의 adjoint bound를 새 알고리즘에 더하지 않는다.** 해당 m-step vector recurrence를 실행하지 않았기 때문이다. 그 대신 실제 새 scalar compilation 오차와 d−1-step evaluation 오차를 전부 포함한다. exact rational compilation도 공짜가 아니며 operand bit length, allocation, coefficient-rounding cost를 setup에 센다. O(md)는 scalar arithmetic count이지 bounded-bit machine time의 보장이 아니다.

이 정리는 Laguerre뿐 아니라 affine three-term recurrence를 가진 Chebyshev / Legendre, Newton 형식의 Leja, Taylor polynomial에도 같은 quotient engine을 적용하게 한다. 따라서 “Laguerre가 이긴다”가 아니라 **연산자 algebra를 이용하면 basis-independent degree reduction을 할 수 있다**는 정리다. 기존 Chebyshev 전용 경로와 direct structured formula도 비교 대상이다.

## 4. 정리 P3 — nonnormal finite jets의 종료하는 resolvent

`A=−κI+N`, `N^d=0`, `a∈R`, `b=1+aκ≠0`라고 하자. 그러면

\[
\boxed{(I-aA)^{-1}=
\frac1b\sum_{j=0}^{d-1}\left(\frac abN\right)^j.}\tag{P3}
\]

이 결과는 `|a| ||N|| / |b| < 1`을 요구하지 않는다. nilpotence 때문에 Neumann series가 유한하게 끝난다. 실제 bound ν≥||N||가 있으면 full-space gain은

\[
G=\frac1{|b|}\sum_{j=0}^{d-1}
\left(\frac{|a|\nu}{|b|}\right)^j
\]

이하이다. 정확 해는 d−1 applications of N으로 계산할 수 있고, computed candidate의 현재 true residual ρ와 residual evaluation/assembly error ζ가 있으면 `||x*−xhat||≤G(ρ+ζ)`이다. G와 residual은 outward arithmetic으로 계산한다.

### 증명과 near-nilpotent 확장

`T=(a/b)N`, `R=b^{-1}Σ_{j<d}T^j`이면 직접 곱셈으로

`(I−aA)R=I−T^d`이다. N^d=0이면 P3가 성립한다. 일반적으로 `||N^d||≤δ_N`이고 `η=(|a|/|b|)^d δ_N<1`이면 `I−T^d`가 Neumann lemma로 가역이어서

\[
(I-aA)^{-1}=R(I-T^d)^{-1},\quad
\|(I-aA)^{-1}\|\le\frac{\|R\|}{1-\eta},
\]

\[
\|(I-aA)^{-1}-R\|\le
\frac{\|R\|\eta}{1-\eta}.
\]

따라서 이 상계와 short-evaluation rounding을 합해 requested budget 이하면 반복 Krylov solve 없이 한 번의 polynomial action으로 인증된 stage solve를 끝낼 수 있다. η가 작다는 것은 ν가 작다는 것과 다르다. 예를 들어 매우 nonnormal인 nilpotent N도 exact δ_N=0이면 허용된다. 그러나 큰 ν는 G와 floating-point error amplification을 키울 수 있으므로 representability / useful error budget은 별도다. □

구조 witness를 `N=N_0+E`, `N_0^d=0`, `||N_0||≤ν_0`, `||E||≤ε`로 주면 noncommuting telescoping으로

\[
\|N^d\|\le\epsilon\sum_{j=0}^{d-1}
(\nu_0+\epsilon)^{d-1-j}\nu_0^j
\]

을 얻는다. ε>0이면 이는 `(ν_0+ε)^d−ν_0^d`와 같지만 subtraction cancellation을 피하려고 **양의 항의 합**으로 평가한다. exact nilpotent declaration이 없는 floating matrix에서 몇 개 vector에 N^d를 적용해 작았다는 결과로 δ_N을 대신할 수 없다.

## 5. 대수기하 관점에서 실제 도움이 되는 부분

대수는 `K[x]/(q)`이다. `q=∏_i(x−λ_i)^{m_i}`가 coprime primary factors로 주어지면 중국인의 나머지 정리에 따라

\[
K[x]/(q)\cong\prod_i K[\epsilon_i]/(\epsilon_i^{m_i}).
\]

오른쪽은 eigenvalue별 **finite jet algebra**다. 각 좌표에서 analytic f는 `Σ_{j<m_i} f^{(j)}(λ_i) ε_i^j/j!`로 끝난다. 서로 다른 local factors의 scalar jets 계산은 독립적이며, reconstruction polynomial을 계산한 뒤 vector action을 수행한다. radical ideal은 값만, nonradical ideal은 미분 jet까지 요구한다. 따라서 nonnormal Jordan multiplicity를 eigenvalue 목록만으로 삭제하지 않는 구조적 장점이 있다.

이것은 알려진 matrix-function functional calculus를 finite algebra로 표현한 것이다. 현대 대수기하라는 명칭만으로 새로운 최적 알고리즘이 되지는 않는다. 가까운 roots에서는 CRT reconstruction 조건수가 나빠질 수 있다. 그때 isolated-root CRT보다 하나의 repeated-root cluster jet 또는 verified coefficient solve가 적절하며, 임의로 실제 distinct roots를 합치면 모델을 바꾸므로 approximation bound가 추가돼야 한다.

P3는 이 원리를 한 개의 nonreduced point에서 사용한 것이다. 일반 sparse upper-triangular matrix라면 direct triangular solve가 더 쌀 수 있다. 이 경우 router는 direct solve를 택해야 하며 quotient route의 허수 baseline을 만들면 안 된다.

## 6. exact concrete example — projector algebra

`P=I−11ᵀ/4`, `A=−16P`, `h=1/4`, `v=(1,2,3,4)ᵀ`를 택한다. P는 symmetric projector이고

\[
Pv=(-3/2,-1/2,1/2,3/2)^T,\quad
e^{hA}v=\frac52\mathbf1+e^{-4}Pv.
\]

X=I−2P라 두면 `A=−8I+8X`, `X²=I`, `q(x)=x²−1`, δ=0이다. P1의 d=2 조건을 만족하므로 exponential과 모든 φ_k action이 degree 1에서 정확하다.

\[
\varphi_k(-4P)v=\frac1{k!}(I-P)v+\varphi_k(-4)Pv.
\]

joint input들이 v의 배수이면 단 한 번의 P application을 모든 k에 공유한다. m-step generic polynomial recurrence가 사용되는 비교군에 비해 products는 m→1이다. 하지만 가장 강한 projector direct formula와 비교하면 이것은 같은 방법이며 그보다 빠르다고 주장하지 않는다. projector의 구조 자체를 활용하는 route를 라이브러리에 추가한다는 의미다.

동봉 `POLYNOMIAL_EXACT_CHECK.py`는 Fraction으로 near-annihilator P1의 k=0..4 다섯 φ-action bound와 projector 예제의 실제 binary64 candidate / scalar coefficient / output assembly bound를 검산한다. e^-4는 30/31차 alternating bounds for e^-1를 4제곱한 rational interval로 감싼다. 모두 6개 확인이 PASS다. 실행 결과는 `POLYNOMIAL_EXACT_CHECK.json`에 있다. 별도로 root의 `research/promotion_theory_20261011/check_exact.py`와 `EXACT_RESULTS.json`에 degree-64 Laguerre polynomial의 degree-1 quotient action identity 및 nonnormal nilpotent inverse의 정확 유리수 확인이 있다. 이들 예제는 기존 repository campaign을 재실행하지 않는다. 일반 정리의 증명은 수식으로 제시한 proof이며 예제 통과로 대체하지 않는다.

## 7. 복잡도 감소를 실제로 보장하는 비용 gate

같은 exact target·input·accuracy budget·current operator epoch에서 기존 알고리즘이 독립 입력 채널 r개 각각 m회의 operator application을 수행한다고 하자. 한 application 비용이 C_X이고, 새 route가 d−1회씩 수행하며 다음 비용 상계가 사전 확보돼 있다고 하자.

| 항목 | 새 경로가 charge할 비용 |
|---|---|
| operator relation / model binding | S_alg (epoch당 한 번; 실제 M회 재사용 때만 M으로 나눔) |
| scalar coefficients / quotient compilation | S_coeff(h), interval 또는 exact-rational 비용 포함 |
| short vector arithmetic / certificate / normalization | V_new |
| dispatch / rejected candidates / fallback | F_new |

그러면

\[
\boxed{\frac{S_{alg}}M+S_{coeff}(h)+V_{new}+F_{new}
<r(m-d+1)C_X}\tag{P-cost}
\]

이면 새 경로의 전체 modeled work가 **기존 경로의 operator work만으로 된 lower bound**보다 작다. 따라서 기존의 추가 coefficient / orthogonalization / certification work를 모두 포함한 전체 modeled work보다도 작다. 이 부등식은 충분조건이며 positivity를 직접 검사할 수 있다. M은 예상 횟수가 아니라 보장되거나 이미 달성한 amortization 횟수이어야 한다. cold-call 비교에는 M=1이다.

정해진 실제 costs의 upper/lower bounds가 없으면 `products saved`만 주장하고 total speed는 주장하지 않는다. wall time은 별도의 환경·반복·동일정확도 비교가 필요하다. Paterson–Stockmeyer의 O(sqrt m) **matrix-matrix** product theorem을 JVP-only callback에 그대로 적용하는 것도 금지한다. 우리 정리는 m차 polynomial 자체를 modulo q로 줄이기 때문에 **matrix-vector** product 수가 줄어드는 서로 다른 주장이다.

streaming Horner는 채널별 작업 vector O(n), scalar coefficients O(d), scalar compilation workspace O(d)를 쓴다. r개 채널을 병렬 수행하면 O(rn)이다. 기존 basis를 모두 저장하는 full Arnoldi 구현의 O(mn) storage와 O(nm²) orthogonalization을 이 route는 수행하지 않는다. 하지만 short recurrence Chebyshev도 O(n) storage이므로 그 comparator에 대한 memory 개선은 주장하지 않는다.

## 8. 코드 적용 계약과 다음 node

제안 타입은 존재하는 API라는 뜻이 아니다.

```rust
struct BoundOperatorAlgebra {
    // Private authority: exact target + epoch + norm/metric + q + defect_upper.
    // Constructed from structural proof or complete verified operator bound.
}
struct CompiledAction {
    // Stored reduced coefficients + their outward uncertainty,
    // target phi weights, algebra identity, full scalar-work receipt.
}
struct CertifiedAction {
    // candidate + total absolute error + work; no global ODE authority.
}
```

1. `polynomial_action.rs` 위에 opt-in compilation entry point를 추가한다. 처음에는 `A=−κP` 및 `A=−κI+N, N^d=0`처럼 exact structural binding이 가능한 **두 family**만 구현한다. 일반 black-box minimal-polynomial discovery는 첫 node가 아니다.
2. P1 방식은 exact φ scalar interpolation, P2 방식은 기존 stored polynomial의 compilation이다. 두 target을 혼동하지 않고 기존 `TotalErrorStatus`와 same-vector semantics를 유지한다.
3. `operator fingerprint + algebra coefficients + structural proof version + metric + epoch`를 cache key에 넣는다. h 변경은 coefficient cache를 바꾸며, A 변경은 algebra witness를 무효화한다. JVP 몇 회가 같다는 sample은 structure authority가 아니다.
4. fresh projector family와 bounded-depth nilpotent family에서 n 증가, d 고정, hκ 변화, distinct/same input, extreme scales를 사전 등록한다. malformed operator binding과 stale epoch는 반드시 거절한다. 기존 PP08/PY02 campaign을 다시 돌리지 않는다.
5. comparison은 generic Chebyshev만이 아니라 analytic projector formula, direct block/triangular solve, 가장 싼 applicable structured backend를 포함한다. P-cost가 양수인 cell만 modeled-work promotion 후보로 넘긴다.
6. full finite-precision action certificate가 통과한 뒤에만 whole-stage / ODE-global proof owner에게 넘긴다. action의 algebraic exactness는 integrator의 order, controller 또는 final-time error certificate가 아니다.

## 9. 문헌 및 provenance

- Higham–Lin, *Matrix Functions: A Short Course*, author-hosted manuscript, <https://eprints.maths.manchester.ac.uk/2067/1/paper.pdf>. Web index에서 §3의 minimal-polynomial / interpolation passage를 확인했다. 직접 PDF fetch는 timeout이었으므로 전체 원문을 읽었다고 주장하지 않는다. 이 문서의 P1/P2/P3 증명은 위에 자족적으로 제시했다. 기초 functional calculus의 계보만 문헌에 귀속한다.
- Paterson–Stockmeyer, *On the Number of Nonscalar Multiplications Necessary to Evaluate Polynomials*, SIAM J. Comput. 2(1), 60–66 (1973), DOI <https://doi.org/10.1137/0202007>. Publisher abstract를 확인했다. 해당 nonscalar / matrix-matrix cost model을 matrix-vector work로 바꾸지 않는 한계 확인에만 사용한다.

본 draft의 P1/P2/P3는 **derived**이고 exact fixtures는 **numerically checked (exact rational arithmetic)**다. 독립 review 전에는 owner self-review이며, 생산 코드·전체 ODE 정확도·측정 speed promotion을 스스로 승인하지 않는다.
