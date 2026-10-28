# 공통 감쇠를 정확히 보존하는 moment packet과 다중 창 전역 인증

2026-10-11. 근거 상태: **derived**; 아래 작은 분수 항등식은 **exact-arithmetic checked**. 후보 생성자의 문서이며 최종 PROMOTE 판정이 아니다. 첨부 Loop12의 기존 campaign을 재실행하지 않았고, native 코드를 변경하지 않았다. 문헌상 신규성은 주장하지 않는다.

## 1. 입력과 목표

직접 읽은 입력은 첨부 `RESEARCH_NOTE_KO.md`의 4–9, 12절 및 감쇠 한계 명시, `rvj_packet/packet.py`, `rvj_packet/convergence.py`, 이전 공개 `GLOBAL_ACCURACY_KO.md`의 G2–G3이다. 기존 Python reference는 `Model`에 κ가 없고 `compile_packet`, `conv`, `candidate`, `certificate`, `reconstruct`가 κ=0 다항식 계약을 사용한다. 따라서 아래 감쇠 client나 restart가 이미 구현되었다는 뜻이 아니다. 기존 global ledger의 local-to-global 합성 원리는 재사용하고, 여기서는 명시된 collective 모델의 훨씬 싼 closed-form transport를 구한다.

대상은 유한 n과 고정 양의 가중치 π_j, Σπ_j=1인

\[
 z_j'=(-\kappa+i\Omega+i\Delta\xi_j)z_j+i\chi|m_z|^2m_z,
 \quad m_z=\sum_j\pi_jz_j,
 \quad |\xi_j|\le1,\ \kappa\ge0,\ \chi\in\mathbb R.                 \tag{DW1}
\]

κ, Ω, Δ의 단위는 시간⁻¹, χ의 단위는 진폭⁻² 시간⁻¹이다. 각 초기 복소 진폭과 위상은 모두 보존한다. 공통 감쇠/주파수를 원 식에서 확인했다는 구조 계약이 필요하다. 임의 서로 다른 κ_j, 임의 local nonlinearity, 추정된 black-box eigensystem은 이 정리의 대상이 아니다. 사용자의 homotopy 아이디어는 아래 비선형 계수 λ의 **최종 목표 λ=1**을 계산하는 장치다. 목표를 작은 λ의 다른 모델로 바꾸지 않는다.

결론은 다음 세 가지다. (i) κT를 해상하는 시간 Taylor를 피하면서 정확한 감쇠와 φ 적분을 유지한다. (ii) 명시적 작은 비선형 regime에서 κ에 무관한 detuning 차수와 nonlinear homotopy 차수를 증명한다. 이것은 스칼라 초월함수의 bit cost까지 κ에 무관하다는 주장이 아니다. (iii) 이전 창 오차를 버리지 않는 O(n) 전역 전파 계약과 같은 이산 계산의 엄밀한 작업량 감소 조건을 얻는다.

## 2. DW-A: 감쇠 인자를 보존한 Duhamel 표현

한 창을 s∈[0,T]로 쓰고 처음 물리 상태를 그대로 `a_j(0)=z_j(t_n)`로 둔다. 국소 carrier를 `z_j(t_n+s)=e^{iΩs}a_j(s)`로 제거한다. 다시

\[
 a_j(s)=e^{-\kappa s}b_j(s),\quad u=\sum_j\pi_jb_j,
 \quad w_\kappa(s)=e^{-2\kappa s}
\]

라 두면 직접 미분과 cubic homogeneity로

\[
 b_j'=i\Delta\xi_jb_j+w_\kappa(s)\,g(u),\qquad
 g(u)=i\chi u^2\bar u.                                           \tag{DW2}
\]

따라서 정확히

\[
 u(t)=F(t)+\int_0^tK(t-s)w_\kappa(s)g(u(s))\,ds,                  \tag{DW3}
\]
\[
 K(t)=\sum_j\pi_je^{i\Delta\xi_jt},\quad
 F(t)=\sum_j\pi_ja_j(0)e^{i\Delta\xi_jt}.
\]

이 변환은 초기 amplitude를 제거하거나 나누어 정규화하지 않는다. 원래 energy identity는

\[
 \sum_j\pi_j|a_j(t)|^2=e^{-2\kappa t}\sum_j\pi_j|a_j(0)|^2,
 \qquad \sum_j\pi_j|b_j(t)|^2=\sum_j\pi_j|a_j(0)|^2.              \tag{DW4}
\]

증명은 π 가중 실수 내적에서 imaginary diagonal term과 `Re(\bar m iχ|m|²m)`이 0임을 쓰면 된다. 따라서 유한 시간 blow-up이 없고 모든 초기 상태의 해가 존재한다. 변환된 평균의 정확한 bound는 `|u|≤(Σπ|a_j(0)|²)^{1/2}`다. floating code가 e^{κt}를 곱해 작은 a를 복원하도록 요구하지 않는다. b/u를 직접 대표하고 최종에 e^{-κt}만 적용한다.

모멘트 μ_k=Σπξ_j^k, ν_k=Σπξ_j^ka_j(0)에서

\[
 K_p(t)=\sum_{k=0}^p\mu_k(i\Delta t)^k/k!,\quad
 F_p(t)=\sum_{k=0}^p\nu_k(i\Delta t)^k/k!
\]

를 쓰되 **w_κ는 전개하지 않는다**. `r=|Δ|T`, `A_0=Σπ|a_j(0)|`이면 `||K_p||_∞≤e^r`, `||F_p||_∞≤A_0e^r`이고

\[
 \theta_\kappa(t):=\int_0^t w_\kappa(s)ds
 =t\varphi_1(-2\kappa t)
 =\begin{cases}(1-e^{-2\kappa t})/(2\kappa),&\kappa>0,\\t,&\kappa=0.\end{cases}   \tag{DW5}
\]

따라서 `θ≤min(t,1/(2κ))`이며 큰 감쇠는 비선형 memory operator의 norm을 감소시킨다. κ→0 극한은 연속적이다.

## 3. DW-B: κ를 시간 다항식으로 해상하지 않는 구성적 수렴 정리

η>1을 **homotopy coefficient의 가중 반경**으로 고른다. 시간 구간에서 연속인 함수 계수 `U_ℓ(t)`의 공간에

\[
 \|U\|_\eta=\sum_{\ell\ge0}\eta^\ell\|U_\ell\|_{C[0,T]},
 \qquad U(\lambda,t)=\sum_{\ell\ge0}\lambda^\ell U_\ell(t)
\]

를 준다. 이는 실수 Banach algebra이고 계수별 복소켤레 `U^*`는 등거리다. 복소 시간에서 `|u|²u`가 holomorphic이라고 가정하지 않는다. λ는 formal/real homotopy 변수이며 coefficient norm의 수렴만 사용한다.

\[
 \mathcal T_\lambda(U)=F_p+\lambda\int_0^tK_p(t-s)w_\kappa(s)i\chi U(s)^2U^*(s)ds.
\]

어떤 B>0에 대해

\[
 M:=A_0e^r+\eta|\chi|e^r\theta_\kappa(T)B^3 < B,
 \qquad q_\eta:=3\eta|\chi|e^r\theta_\kappa(T)B^2<1               \tag{DW6}
\]

이면 닫힌 B-ball에서 유일한 고정점 U가 있다. `||U||_η≤M`. 물리값 λ=1에서 `u_p=U(1,.)`는 truncated-kernel memory equation의 해다. Q차 후보

\[
 u_Q(t)=\sum_{\ell=0}^Q U_\ell(t)
\]

는

\[
 \|u_p-u_Q\|_\infty\le M\eta^{-Q-1},\qquad
 \|u_Q-F_p-K_p*(w_\kappa g(u_Q))\|_\infty
 \le(1+q_1)M\eta^{-Q-1},\quad q_1=q_\eta/\eta.                  \tag{DW7}
\]

**증명.** 함수 곱의 sup norm과 coefficient convolution의 ℓ¹ submultiplicativity에서 `||U²U*||≤||U||³`, cubic difference는 B-ball에서 `≤3B²||U−V||`다. λ 곱은 norm을 η배, 적분은 `e^r θ`배 이하로 증가시킨다. (DW6)은 self-map 및 contraction이다. Banach 고정점과 계수별 수렴으로 존재·유일성을 얻는다. 고정점의 norm을 다시 map bound에 넣으면 M이다. 고정점 tail에 `η^{Q+1}`을 적용하면 첫 오차식. u_p와 u_Q가 모두 B-ball에 있으므로 residual 차이를 identity 부분과 map 부분으로 분리하면 둘째 식이다. □

**유한한 실제 계산.** 계수는 이전 차수만으로 결정된다.

\[
 U_0=F_p,\qquad
 U_\ell=i\chi\sum_{a+b+c=\ell-1}
 \int_0^tK_p(t-s)e^{-2\kappa s}U_a(s)U_b(s)U_c^*(s)ds.            \tag{DW8}
\]

Q개 차수의 순차 의존성은 남는다. 같은 차수의 coefficient product, convolution, output evaluation은 병렬화할 수 있으나 Q 전체를 한 라운드라고 하지 않는다. 모든 U_ℓ는 다음 유한 공간에 속한다.

\[
 U_\ell(t)=\sum_{j=0}^\ell e^{-2j\kappa t}P_{\ell j}(t),\qquad
 \deg P_{\ell j}\le D_\ell=(3\ell+1)p+\ell.                     \tag{DW9}
\]

κ=0에서는 exponential들을 합쳐 하나의 다항식으로 처리한다. 증명은 induction이다. 세 자식의 차수 합이 ℓ−1이므로 polynomial degree 합은 `3ℓp+ℓ−1`, exponential index 합은 ≤ℓ−1이다. w를 곱하면 index≤ℓ이고, degree≤p인 kernel과 적분하면 degree≤`(3ℓ+1)p+ℓ`. exponential-polynomial 적분은 같은 exponential rate와 rate 0의 다항식 합으로 닫힌다. κ→0에서 이 함수는 연속이지만 개별 expanded coefficient는 큰 역κ 거듭제곱을 가질 수 있으므로 stable confluent 표현이 필요하다.

이 결과는 detuning 시간 차수 p와 nonlinear homotopy 차수 Q가 각각 `O(log(1/ε))`가 되는 충분조건을 제공한다. θ≤T로 (DW6)을 검사하면 **모든 κ≥0에 공통된 p,Q**를 사용할 수 있다. 저장 basis의 시간 polynomial degree는 O(pQ)이고 exponent index는 Q 이하이다. κT를 다항식 차수로 해상하지 않는다. 해당 표현의 interval bit precision과 scalar exponential/φ evaluation 비용이 κ에 무관하다는 결론은 포함하지 않는다.

### 명시적 비어 있지 않은 regime

첨부와 같은 `T≤1/2, |Δ|≤1, |χ|≤1/8, A0≤9/32, max|a_j(0)|≤11/32`를 쓰고 B=3/4, η=4로 둔다. 모든 κ≥0에서 `e^{1/2}<5/3`, θ≤1/2를 사용하면

\[
 M\le165/256<3/4,\qquad q_\eta\le45/64<1,\quad q_1\le45/256.
\]

representation rounding이 쓰지 않은 여유는 `B−M≥27/256`이다. 아래 전체 mode error 정리까지 합하면 **p=8, Q=10**, exact descriptor/representation/phase 산술에서 κ와 n에 공통으로

\[
 \max_j|a_j(T)-\widehat a_j(T)|
 \le20832297221/892274301783244800 < 1/(4\cdot10^7)
\]

를 보장한다. 이는 사전 상계이며 실제 출력 오차나 실측 speed가 아니다. 이 scheme이 기존 κ=0 polynomial candidate보다 적은 coefficient work를 쓴다는 결론도 없다. `D_10=258`이라는 보수적인 basis bound를 숨기지 않는다. 큰 감쇠를 정확한 지수 인자에 남기는 **가능성·복잡도 정리**다.

## 4. DW-C: exact φ 적분과 안정한 계산 계약

필요한 일반 항은

\[
 I_{r,d}(a,t)=\int_0^t(t-s)^r s^d e^{-as}ds,\quad a\ge0.
\]

`φ_k(z)=(k−1)!^{-1}∫_0^1e^{(1−v)z}v^{k−1}dv`로 정의하면

\[
 I_{r,d}(a,t)=t^{r+d+1}\sum_{v=0}^d(-1)^v{d\choose v}(r+v)!\,
 \varphi_{r+v+1}(-at).                                           \tag{DW10}
\]

증명은 `s^d=[t−(t−s)]^d`를 전개하고 적분변수를 바꾸면 된다. 동치인 **양의** 표현은

\[
 I_{r,d}(a,t)=t^{r+d+1}\int_0^1(1-v)^r v^d e^{-atv}dv.
\]

이는 `exp(tz)`의 repeated-node divided difference
`r!d! exp(t·)[0 repeated r+1, −a repeated d+1]`이기도 하다. a=0에서는 정확히 `r!d! t^{r+d+1}/(r+d+1)!`다. 계수 교대합을 작은 a에서 무비판적으로 floating 계산하지 않는다.

재현 가능한 scalar client의 최소 계약은 `(r,d,a,t,requested absolute radius) → enclosing interval, charged work`다. 구현 경로의 한 예는 다음과 같다.

* `at≤1`: 양의 적분의 `exp(−atv)`를 작은 argument에서 전개하고 절대 remainder `≤(at)^{M+1}/(M+1)!`에 beta 적분 상계를 곱한다. Taylor degree가 큰 κT를 해상하지 않는다.
* `at>1`: `(t−s)^r`를 전개하고 `H_m(a,t)=m!a^{−m−1}[1−e^{−at}Σ_{j=0}^m(at)^j/j!]`를 outward arithmetic으로 평가한다. e^{-at}는 검증된 scalar exponent evaluator를 쓴다. cancellation이 radius를 키우면 precision을 올리고 작업량을 charge한다. positive integral enclosure를 결합하여 부호를 보호한다.
* 원하는 radius가 work cap 내에서 닫히지 않으면 해당 후보를 인증하지 않는다. 일반 φ가 존재한다는 identity를 현재 native API의 기능으로 간주하지 않는다.

`scalar_phi_enclosure`의 기존 낮은 차수/유한 argument 지원만으로 (DW10)의 임의 차수를 지원한다고 주장할 수 없다. 필요한 arbitrary-degree mixed integral API는 **새 이식 과제**다. 위 식은 구현 가능한 유한 절차와 오차항을 고정하지만 이 문서에서 그 evaluator를 구현·벤치마크하지 않았다.

## 5. DW-D: inherited error와 kernel/roundoff 오차를 포함하는 모든 mode 상계

창 시작 computed 상태를 `\hat a_j^0`, 진짜 상태와의 상계를 `|a_j^0−\hat a_j^0|≤δ_j`라 하자. descriptor는 **computed 초기값에 바인딩**되며 진짜 초기값으로 가장하지 않는다. 다음을 둔다.

\[
 E_0=\sum_j\pi_jδ_j,\quad \widehat A_0=\sum_j\pi_j|\widehat a_j^0|,
 \quad \tau_p(t)=\frac{(|\Delta|t)^{p+1}}{(p+1)!},
\]
\[
 \beta_{\kappa,p}(t)=|\Delta|^{p+1}t^{p+2}\varphi_{p+2}(-2\kappa t)
 =\frac{|\Delta|^{p+1}}{(p+1)!}\int_0^t(t-s)^{p+1}e^{-2\kappa s}ds.
\]

β≤`t τ_p/(p+2)`이며 β≤θτ_p. 후보 `\hat u(t)`가 전 구간에서 `|\hat u|≤B`이고 진짜 변환된 mean도 ≤B라 하자. 후자는 `(Σπ|\hat a_j^0|²)^{1/2}+(Σπδ_j²)^{1/2}≤B`로 충분히 확인한다. 후보의 **exact descriptor에 대한** 변환된 Volterra residual을

\[
 \widehat u-F_p-K_p*(w_\kappa g(\widehat u))=r_u,\qquad
 \sup_{0\le s\le t}|r_u(s)|\le D(t)
\]

로 감싸고 D(t)는 단조 상계라 하자. `C=3|χ|B²`, `G=|χ|B³` 및

\[
 H(t)=E_0+\widehat A_0\tau_p(t)+G\beta_{\kappa,p}(t)+D(t)
\]

를 정의하면

\[
 |u(t)-\widehat u(t)|\le e^{C\theta_\kappa(t)}H(t).               \tag{DW11}
\]

각 mode를

\[
 \widehat a_j(t)=e^{-\kappa t}\left[
 E_p(i\Delta\xi_jt)\widehat a_j^0+
 \int_0^tE_p(i\Delta\xi_j(t-s))w_\kappa(s)g(\widehat u(s))ds
 \right]
\]

로 재구성하면

\[
 |a_j(t)-\widehat a_j(t)|\le e^{-\kappa t}\left[
 δ_j+|\widehat a_j^0|\tau_p(t)|\xi_j|^{p+1}
 +G\beta_{\kappa,p}(t)|\xi_j|^{p+1}
 +(e^{C\theta_\kappa(t)}-1)H(t)\right].                         \tag{DW12}
\]

**증명.** real-frequency exponential Taylor integral remainder는 `|e^{ix}−E_p(ix)|≤|x|^{p+1}/(p+1)!`다. (DW3)와 candidate 식을 뺀 뒤 initial mismatch, free-profile tail, kernel tail, cubic Lipschitz difference, residual로 나눈다. 그러면 transformed mean error e(t)는

`e(t)≤H(t)+C∫_0^t e^{-2κs} e(s)ds`를 만족한다. H는 단조이므로 weighted Gronwall이 (DW11)을 준다. 각 mode Duhamel 식에도 같은 분해를 적용한다. nonlinear difference 부분은 `C∫w e≤H(t)∫Cw exp(Cθ)=H(t)(exp(Cθ(t))−1)`이고 나머지는 첫 세 항이다. □

이 proof는 candidate가 tube 밖인 경우를 포함하지 않는다. (DW6)으로 만든 exact candidate는 `||u_Q||≤M<B`; represented coefficient uncertainty `ε_u≤B−M`를 검사하면 tube가 닫힌다. 다른 후보를 쓰면 전체 구간 sup enclosure를 따로 증명해야 한다. endpoint 표본 검사만으로 이 조건을 대체하지 않는다.

### arithmetic와 descriptor error

새 descriptor의 각 moment error가 `|\tildeν_k−ν_k|≤ε_{ν,k}`, `|\tildeμ_k−μ_k|≤ε_{μ,k}`라면 exact-descriptor residual에 다음을 추가한다.

\[
 E_\nu(t)=\sum_{k=0}^p\epsilon_{\nu,k}(|\Delta|t)^k/k!,
\]
\[
 E_\mu(t)=G\sum_{k=0}^p\epsilon_{\mu,k}|\Delta|^k t^{k+1}
                   \varphi_{k+1}(-2\kappa t).
\]

유한 representation과 exact U_Q 사이의 uniform enclosure가 ε_u이면, 둘 다 B-ball에서 residual의 추가량은 `(1+q_1)ε_u` 이하이다. scalar φ/exponential integration, nonlinear coefficient product, residual summation의 interval radius도 각각 D에 charge한다. nominal model의 χ, Δ, κ, Ω가 uncertain할 때는 parameter discrepancy를 별도로 envelope에 추가하거나 동일 parameter를 포함하는 uniform witness를 새로 구성한다. 단순히 moment rounding으로 숨기지 않는다.

최종 물리 output에서 carrier/damping 곱의 오류는 mode별 arithmetic `ε_out,j`에 포함한다. 예컨대 carrier enclosure `|\hat c−e^{iΩT}|≤ε_phase`와 이미 재구성한 damped 값의 bound `|\hat a_j|≤A_j`를 사용하면 `A_j ε_phase`와 마지막 multiplication radius를 더한다. e^{-κT}의 underflow는 자동으로 0이라는 exact identity가 아니라 absolute enclosure의 일부다.

## 6. DW-E: restart를 포함하는 전역 정확도와 O(n) 전파

창 n의 computed 시작 상태 `\hat z_n`와 true 상태 사이의 componentwise error vector를 `d_n≥0`로 보존한다. 각 창에서 computed 시작 상태를 정확한 초기값으로 둔 local reference flow와 candidate 사이의 local endpoint bounds `η_{n,j}`를 (DW12)의 **δ=0** 버전으로 얻는다. 여기에 moment/representation/phase/output arithmetic을 모두 포함한다. 같은 inherited error를 local η와 아래 propagation에 두 번 넣지 않는다.

두 exact flows의 initial weighted norm이 모두 R_n 이하라고 인증하자. 예를 들어

\[
 R_n\ge\|\widehat z_n\|_\pi+\|d_n\|_\pi,
 \quad \|x\|_\pi^2=\sum_j\pi_j|x_j|^2
\]

이면 충분하다. (DW4)로 모든 exact means의 진폭은 `R_n e^{-κs}` 이하이다. 따라서 `C_n=3|χ|R_n²`, `Θ_n=θκ(h_n)`, `P=1π^T`에 대해

\[
 \boxed{d_{n+1}\le M_n d_n+\eta_n},\qquad
 M_n=e^{-\kappa h_n}\left[I+(e^{C_n\Theta_n}-1)P\right].         \tag{DW13}
\]

**증명.** exact-flow 차이에 (DW11)–(DW12)의 kernel/residual=0 식을 적용하면 mode j의 차이는 `e^{-κh}[d_j+(exp(CΘ)−1)Σπd]` 이하이다. local approximation η를 삼각부등식으로 더하면 된다. C_n의 근거는 에너지 불변식과 두 시작값의 enclosure이므로 unknown numerical-step Jacobian을 추정할 필요가 없다. □

P²=P이고 π가 모든 창에서 같으므로 여러 창의 transport는 정확히

\[
 M_{b-1}\cdots M_a=e^{-\kappa\sum_{n=a}^{b-1}h_n}
 \left[I+\left(\exp\sum_{n=a}^{b-1}C_n\Theta_n-1\right)P\right]. \tag{DW14}
\]

이는 δ 벡터의 **상계 전파 행렬** 항등식이며 원래 nonlinear flow의 Jacobian identity라고 해석하지 않는다. (DW13)은 한 번의 π 가중합과 n개 scalar update, 즉 O(n) work/O(n) storage다. dense n×n derivative나 행렬 곱은 만들지 않는다. 전체 trajectory bound는

\[
 d_N\le(M_{N-1}\cdots M_0)d_0+
 \sum_{j=0}^{N-1}(M_{N-1}\cdots M_{j+1})\eta_j.                 \tag{DW15}
\]

각 창은 exact physical initial vector를 새로 안다고 가정하지 않고 이전 d를 소비한다. weights/frequencies/model epoch가 바뀌면 기존 P, moments, proof identity를 자동 재사용하지 않는다. adaptive grid도 승인된 실제 h_n마다 조건과 누적 bound가 닫히면 허용된다. 미래 오차 증폭을 사후 관측치로 미리 사용하지 않는다.

**감쇠 regime의 유용한 충분조건.** `κ≥3|χ|R_n²`이면 `C_nΘ_n≤κh_n`이므로 weighted ℓ² 또는 ℓ∞에서 `||M_n||≤1`이다. 따라서 `||d_0||+Σ||η_n||≤ε_total`이면 full trajectory endpoint가 budget 안이다. 종료 시간 T_total을 미리 고정하고 `||η_n||≤ε_local h_n/T_total`로 배분하면 step 수를 미리 몰라도 합은 ε_local 이하이다. κ=0 또는 이 조건이 없는 경우는 (DW15)의 실제 증폭을 유지한다. E'≤0인 한 trajectory의 안정성과 두 trajectory의 nonexpansivity를 혼동하지 않는다.

**restart 비용.** μ는 고정 π,ξ에서 재사용할 수 있다. ν는 새로운 computed 복소 상태와 모든 mode error에 바인딩하여 다시 만든다. 전 상태 출력을 매 창 요구하면 moments/reconstruction의 O(n(p+1)) 및 O(n) 저장을 charge한다. 이를 O(p)짜리 초기화로 숨기지 않는다. 국소 carrier는 각 창의 실제 시작 복소 상태에서 시작하므로 원래 독립 위상은 지워지지 않는다.

## 7. DW-F: 정확도와 엄밀한 작업량 감소가 함께 성립하는 조건

(DW8)의 각 forcing coefficient를 먼저 생성하고, 동일 p와 같은 representation, 같은 exact scalar integration rule을 적용하는 n-mode homologous 계산을 comparator로 둔다. 두 방법 모두 scalar mean의 nonlinear forcing을 한 번만 생성한다. comparator가 이미 aggregation을 hoist했다면 새 이득을 중복 주장할 수 없다.

**동일 계산 정리.** 각 homotopy 차수에서 n개 mode의 π 가중합은 compiled F_p,K_p를 사용한 U_ℓ와 정확히 같다. ℓ=0은 moment 정의이고, inductive forcing가 같으면 π 가중합이 causal convolution/linear representation과 commute한다. 공통 κ가 있어도 같은 항등식이다. floating 반올림 순서의 bit identity는 포함하지 않고, 각 경로의 arithmetic error를 DW-D로 감싼다.

각 차수의 convolution에 사용되는 forcing coefficient slot 총수를 M_f라 하면 dense fixed schedule의 kernel coefficient product 수는

\[
 n(p+1)M_f\ \longrightarrow\ (p+1)M_f.
\]

zero skipping을 하는 comparator는 실제 nonzero slot count로 다시 비교해야 한다. φ table은 가능한 한 **양쪽 모두 공유**하도록 charge하며 n회 중복 계산을 강요한 baseline을 만들지 않는다. 최종 모든 mode는

\[
 \widehat a_j(T)=e^{-\kappa T}\{B_p(\xi_j)\widehat a_j^0+C_p(\xi_j)\},
\]
\[
 B_p(x)=\sum_{r=0}^p(i\Delta T)^r x^r/r!,\quad
 C_p(x)=\sum_{r=0}^p\frac{(i\Delta)^r x^r}{r!}
      \int_0^T(T-s)^r e^{-2\kappa s}g(u_Q(s))ds
\]

로 O(n(p+1)) Horner 재구성이 된다. C_p setup은 n과 독립이며 mixed φ evaluation 비용을 포함한다.

**strict total-work 충분조건.** 같은 charged arithmetic 단위에서 각 mode convolution 총비용을 C_conv>0, packet의 추가 per-mode input/reconstruction/certificate 비용 상계를 C_io, 추가 n-independent setup/representation/φ/certificate/failure/fallback 비용을 C_fixed로 실제 기록하자. 두 방법에 공통된 비용은 공통으로 취소하되, packet에 불리한 추가 작업을 빼지 않는다. 그러면

\[
 C_{conv}>C_{io},\qquad
 n>\frac{C_{conv}+C_{fixed}}{C_{conv}-C_{io}}                    \tag{DW16}
\]

이면 packet total charged work가 comparator보다 엄밀히 작다. 증명은 절약 `(n−1)C_conv`가 추가 `nC_io+C_fixed`보다 큼을 정리하면 된다. 예컨대 실제 counter upper/lower가 `C_conv=12,C_io=6,C_fixed≤120`인 동일 단위 regime에서 n≥23이면 strict reduction이다. 이 수치는 측정값이 아니라 (DW16)의 **비어 있지 않은 충분조건 예**다. 수학적 정확도는 DW-B/D를, 엄밀한 작업 감소는 별도로 DW16을 모두 충족해야 한다.

**구성적 upper complexity.** 모든 U_ℓ의 exponent-polynomial slots는 `V=O((p+1)(Q+1)^3)`개 이하이다. U²와 U²U*의 coefficient arrays를 차수 순서로 축적하면 dense naive products는 O(V²)로 상계된다. kernel integration의 finite expanded algebra를 포함한 한 보수적 상계는

\[
 W_{packet}=O\big(n(p+1)+(p+1)^2(Q+1)^6+(p+1)^3(Q+1)^4
 +R_{out}[n(p+1)+(p+1)(Q+1)(D+1)^2]\big)
 +C_{transcendental/interval,total},\quad D=(3Q+1)p+Q.
\]

여기서 **모든 출력시각의 공유 coefficient setup도 별도로 센다**. 최종 `g(u_Q)`의 exponential-polynomial representation은 한 번 만들 수 있지만, 그 적분값과 C_p(t)의 계수는 서로 다른 출력시각 t마다 달라진다. `g(u_Q)`는 exponent index ≤3Q, 시간 degree ≤3D이므로 각 r≤p에 대해 이 coefficient slots를 DW10으로 적분·합산한다. DW10의 d차 항에 최대 d+1개 scalar summand가 있어, 출력 하나의 ordinary scalar algebra를 `C_shared_alg=O((p+1)(Q+1)(D+1)^2)`로 보수적으로 감싼다. 이 값은 phi evaluator 호출 자체의 비용을 제외한 coefficient/power/합산 비용이며, 위 식에 `R_out C_shared_alg`로 명시했다. scalar phi, damping, carrier와 interval 정밀도 비용은 preprocessing 및 **모든 출력시각의 합**인 `C_transcendental/interval,total`로 추가한다. ordinary shared algebra를 그 항에 숨기지 않는다. p=0 또는 Q=0에서도 `(p+1),(Q+1),(D+1)`을 유지하므로 상수 forcing/output 비용이 사라지지 않는다.

이는 unit-cost exact scalar arithmetic model의 상계다. `p,Q=O(log(1/ε))`인 고정 (DW6) margin에서 n-dependent work는 `O((1+R_out)n log(1/ε))`; n-independent polynomial work는 위 식처럼 R_out 의존성을 가진다. 출력 횟수를 고정한 경우에만 nonlinear 전체를 n-independent polylogarithmic upper bound로 요약한다. 바깥의 `C_transcendental/interval,total`은 argument/precision/conditioning에 의존하며 공짜 oracle가 아니다. Fraction bit complexity, allocation, dense output 수가 늘어나는 경우, interval growth를 이 식에 숨기지 않는다. DW16을 여러 출력에 적용할 때 C_io는 모든 출력의 per-mode 비용을, C_fixed는 모든 출력의 공유 coefficient/evaluator/certificate 비용을 포함해야 한다. baseline보다 무조건 빠르다는 정리는 없지만, **동일 accuracy 계약과 실제 counted threshold 아래 strict gain을 보장하는 양의 정리**는 있다.

## 8. 구체적 이식 계약과 끝점

새 client는 다음 순서로 만들 수 있다. 현재 generic VigilODE driver에 이 모델 구조가 자동 존재한다고 주장하지 않는다.

1. `DampedCollectiveDescriptor`: π,ξ,κ,Δ,χ,Ω, 전체 초기 복소 상태, model epoch와 moments의 identity/roundoff를 보존한다. known collective structure를 선언·검증한 모델만 입장한다.
2. `ExpPolynomialHomotopy`: DW8, repeated exponent index, κ=0 confluent case, coefficient enclosure를 구현한다. `u`, `u*` coefficient 의미를 고정한다.
3. `MixedDuhamelIntegralEnclosure`: DW10/양의 적분/finite H_m 식의 작은/큰 argument 경로와 작업 cap을 구현한다. unknown range/차수는 decline한다.
4. `DampedPacketCertificate`: DW11–12, 전체 구간 candidate tube, descriptor error, local output arithmetic을 반환한다. candidate/model/initial-state/precision epoch를 하나의 immutable binding으로 묶는다.
5. `WindowErrorLedger`: DW13을 O(n)으로 업데이트하고, accepted endpoint와 error vector를 함께 다음 창에 전달한다. native pending error estimator를 global certificate로 대체했다고 가장하지 않는다.
6. 정확한 κ=0 boundary, κ>0 exponential-polynomial identity, inherited-error rejection, phase preservation, all-mode budgets, stable φ boundary를 새 단위/holdout 테스트로 검증한 뒤 counted-work 비교를 실시한다. 이 단계들은 아직 실행되지 않았다.

현재 닫힌 것은 새로운 유도와 작은 exact algebra이다. 현재 native damped client, global multiwindow execution, interval φ evaluator, measured speed는 **not implemented / not measured**다. 독립 reviewer가 위 가정을 승인한 후에도 해당 runtime 지위는 따로 남는다.
