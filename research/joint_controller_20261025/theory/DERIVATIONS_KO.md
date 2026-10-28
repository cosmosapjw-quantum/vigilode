# 적응 제어와 구조 기반 방법 선택을 연결하는 수학

## 연구 대상과 결론

VigilODE의 고유한 계산 경로를 하나의 판단 체계로 묶는 핵심은 **현재 단계의 원래 방정식에 대한 잔차, 원래 상태에서의 오차 증폭, 앞으로 지불할 계산비용**이다. 구조를 아는 경우에는 다항식·회전·보존량으로 계산을 줄이고, 일반적인 경우에는 같은 기준으로 선택적 JVP 경로를 비교한다. 작은 스텝 하나를 물리적 강성으로 분류하는 대신, 정확도 조건을 만족하는 후보 중 실제로 더 적은 비용으로 전진하는 후보를 선택한다.

아래 결과는 명시한 가정에서 유도한 구성적 결과다. 삼각계의 전진 대입, 다항식 역원, Grönwall 추정 자체에 대한 문헌 신규성을 주장하지 않는다. 이번 기여는 이들을 현재 코드의 **단계별 오차 예산·실제 스텝 크기·비용 증거 유효기간·방법 및 실행 선택**으로 연결하는 데 있다. 과거의 유효한 Fourier·quotient·causal 연구는 다시 계산하지 않고 코드와 근거를 재사용한다.

기준 코드는 원격 `4fe67602502440d76aa9dd245c6b1d67558c14bf`, tree `8bab954cffcf7abd9cdd0ca528567ece47e43bc7`이다. 일반 유한차원 ODE를 대상으로 하며, $h>0$, 상태 차원 $n$, 단계 수 $s$, 고정된 물리 노름을 사용한다. 벡터장·시간·노름의 단위가 다를 때 아래 계수에 해당 단위를 유지한다.

## 1. 수축 조건 없이 작동하는 인과적 단계 오차 상계

### 가정

원래 단계 방정식이

\[
 W_i u_i=b_i(u_0,\ldots,u_{i-1}),\qquad i=0,\ldots,s-1
\]

이고 각 $W_i$는 가역이라고 하자. 지정한 노름에서

\[
 \|W_i^{-1}\|\le M_i,
 \qquad
 \|b_i(v)-b_i(w)\|\le\sum_{j<i}L_{ij}\|v_j-w_j\|
\]

가 후보와 정확한 단계값을 포함하는 영역에서 성립한다. 이 영역의 포함은 별도의 구조적 가정이다. 후보만 포함하는 작은 표본영역으로 대체할 수 없다. 원래 목표에서 계산한 잔차와 계산 오차의 합을

\[
 \rho_i\ge\|W_i\widehat u_i-b_i(\widehat u_{<i})\|
\]

로 상계한다. $d_i=M_i\rho_i$, $B_{ij}=M_iL_{ij}\ge0;(j<i)$, 그 외 $B_{ij}=0$로 정의한다.

### 정리 1 — 유한 삼각 오차 전파

다음 스칼라 전진 대입은 모든 단계 오차를 상계한다.

\[
 E_0=d_0,\qquad E_i=d_i+\sum_{j<i}B_{ij}E_j,
 \qquad\|\widehat u_i-u_i\|\le E_i.
\]

동등하게

\[
 E=(I-B)^{-1}d=\sum_{k=0}^{s-1}B^kd.
\]

**증명.** 두 단계식을 빼고 $W_i^{-1}$를 적용하면

\[
 \|\widehat u_i-u_i\|\le d_i+\sum_{j<i}B_{ij}\|\widehat u_j-u_j\|.
\]

첫 단계에서 결론이 성립하고, 이전 단계의 상계를 대입하는 귀납법으로 모든 단계에서 성립한다. 엄격한 하삼각 행렬은 $B^s=0$이므로 역행렬의 유한 급수 표현도 성립한다. □

여기에는 $\|B\|<1$이라는 조건이 없다. $B_{i,i-1}=2$, $d_i=10^{-8}$, $s=8$이면 마지막 단계 상계는 $255\times10^{-8}$이다. 전체 수축률을 2로 잡는 무한기하급수는 사용할 수 없지만, 유한 인과계의 상계는 유효하다. 동시에 단순히 최대 잔차 $10^{-8}$만 읽으면 마지막 단계 증폭을 놓친다.

출력이 $y^+=y+\sum_i\beta_i u_i$이면

\[
 \|\widehat y^+-y^+\|\le\sum_i|\beta_i|E_i+\epsilon_{\rm reconstruction}.
\]

이 값은 **부정확하게 풀린 단계 때문에 추가된 오차**다. 원래 시간 이산화의 국소 오차와 합산하여 사용한다. embedded 차분이 작다는 사실만으로 전체 ODE 오차에 대한 엄밀한 상계가 되는 것은 아니다.

### 현재 raw-U RODAS와 RK에 대한 대응

현재 인과적 raw-U 식의 우변은

\[
 b_i(U_{<i})=h\gamma f\!\left(y+\sum_{j<i}a_{ij}U_j\right)
       +\gamma\sum_{j<i}c_{ij}U_j.
\]

따라서 유효 영역에서 벡터장의 Lipschitz 상수가 $L_f$라면

\[
 L_{ij}=|h\gamma|L_f|a_{ij}|+|\gamma c_{ij}|
\]

를 사용할 수 있다. explicit RK에서는 해당 단계 변수의 정의에 따라 $W_i=I$, $L_{ij}=|h a_{ij}|L_f$가 된다. 계수가 구조적으로 알려져 있으면 이 상계 자체는 수치 미분이나 AD를 요구하지 않는다.

`vigilode-coupled/src/solver.rs::full_check`는 이미 원래 공간의 단계 잔차를 계산하고 `full_residual_l2`를 보존한다. 새 상계 소비자는 이 스칼라들을 재사용할 수 있다. `rodas5p-integrators/src/causal_majorant.rs`에는 위 유한 경로합의 upward-rounded 구현과 radius-box closure가 이미 있다. 이번 구현의 연결점은 **할당 없는 호출자 버퍼, 출력 방향 adjoint, 단계별 예산, 정확한 identity를 갖춘 소비자**다. 다만 현재 coupled 경로는 frozen basis를 만들 때 JVP를 사용한다. **현재 전체 coupled 구현을 완전 derivative-free라고 부르는 것은 정확하지 않다.** 이번 상계는 Newton-free 인과 스케줄과 fully derivative-free 구조 제공자 양쪽에 적용되는 공통 인터페이스다.

### 출력 방향 예산과 계산량 배분

비음수 출력 가중치 $w_i=|\beta_i|$를 고정하고 역방향 삼각 대입으로

\[
 \sigma=(I-B^T)^{-1}w
\]

를 계산하면 $w^TE=\sigma^Td$이다. 따라서 $\theta_i\ge0$, $\sum_i\theta_i\le1$에 대해

\[
 d_i\le\frac{\theta_i\tau}{\sigma_i}\quad(\sigma_i>0)
\]

이면 출력 추가 오차가 $\tau$ 이하이다. 원래 잔차 $\rho_i$의 허용치는 여기에 $M_i$를 추가로 나누어 얻는다. $\sigma_i=0$은 **이 출력 상계**에 대한 영향이 0이라는 뜻이며, 다른 출력·embedded 추정량·연속 경로의 영향까지 0이라고 뜻하지 않는다.

실제 소스가 내보내는 principal 출력과 embedded 차분을 함께 보호하려면 각 가중치를 따로 사용하거나, $w_i=|\beta_i|+|\beta_i-\widehat\beta_i|$로 두 오차의 합을 묶을 수 있다. 물리 스케일은 두 후보에 공통으로 고정해야 한다. 후보에 따라 달라지는 WRMS 분모를 사용하면 그 분모의 하한을 별도로 적용한다.

이번 pilot은 고정 절대 노름에서 위 **sum 가중치**와 두 reconstruction roundoff의 합을 사용한다. 관측 embedded 크기와 이 상계를 더하여, embedded 추정량 자체의 오차와 principal 출력의 추가 오차를 같은 예산에 넣는다. max 가중치는 두 오차를 각각 상계할 수 있지만 그 합의 예산에 그대로 대입할 수 없으므로 사용하지 않는다. `causal/README_KO.md`의 실제 코드 대응과 단계를 구분한다.

정밀도 개선의 국소 비용모델이 $c_i(d_i)=a_i\log(D_i/d_i)$, $a_i>0$, $\tau>0$이고 saturation이 없는 구간이라 하자. $\sum_i\sigma_i d_i\le\tau$ 아래 비용 합을 최소화하는 양의 예산은 (이 식에서는 모든 $\sigma_i>0$인 활성 단계만 포함한다)

\[
 d_i^*=\frac{\tau a_i}{\sigma_i\sum_j a_j}.
\]

**증명.** 목적함수의 Hessian은 양의 대각 $a_i/d_i^2$이므로 엄격히 볼록하다. KKT 조건 $-a_i/d_i+\lambda\sigma_i=0$와 활성 예산식으로 $\lambda=\sum_j a_j/\tau$를 얻는다. 따라서 위 해는 유일한 전역 최솟값이다. □

이 범위에서 비용 가중 예산은 균등 예산보다 모델상 총비용이 크지 않다. 실제 반복 수가 정수이고 최소/최대 잔차가 있으므로 구현 확장은 clamped water-filling 또는 작은 정수 후보 비교를 사용한다. 초기 pilot은 균등 출력 예산을 사용하고, 비용 가중 배분은 추가 개발 항목으로 남긴다.

모든 상계 연산은 양의 항을 upward rounding하고, 허용 오차를 나눌 때 downward rounding해야 한다. 입력 $M,L,\rho$가 단순 관측치이면 결과도 관측 기반 진단이다. 원래 실수값의 유효한 상계와 연산자/노름 identity가 제공되었을 때만 인증된 단계 오차로 취급한다. overflow·NaN·유효영역 closure 실패는 사용할 수 없는 후보로 반환한다.

### 전역 오차로의 연결

$L\ge0$일 때 이상적인 단계법의 전진 사상이 $\|\Phi_h(v)-\Phi_h(w)\|\le(1+Lh)\|v-w\|$를 만족하고, 이상적인 국소 결함이 $Ch^{p+1}$, 위의 추가 출력 오차가 $Dh^{p+1}$ 이하라 하자. 고정 구간에서 반복하면

\[
 e_{n+1}\le(1+Lh_n)e_n+(C+D)h_n^{p+1},
 \qquad
 e_N\le e^{LT}\left(e_0+(C+D)T h_{\max}^{p}\right).
\]

첫 부등식은 삼각부등식, 두 번째는 반복 전개와 $\prod(1+Lh_n)\le e^{LT}$에서 얻는다. 따라서 단계 잔차를 출력 방향으로 $O(h^{p+1})$에 묶으면 원래 방법의 차수 $p$를 유지한다. 높은 강성에서 $L,C,D$가 강성에 독립적인지는 별도 안정성·정칙성 문제이며 이 식이 자동으로 해결하지 않는다.

## 2. 인과적 병렬 반복과 homotopy의 역할

### 정리 2 — 유한 의존 깊이

엄격한 인과 관계를 갖는 단계 사상 $T_i(U)=W_i^{-1}b_i(U_{<i})$에 대해 동시 업데이트 $U^{k+1}=T(U^k)$를 하자. 정확한 산술에서 첫 $k$개 단계는 $k$번 업데이트 뒤 정확한 단계값과 같다. 따라서 $s$번 이내에 전체 단계가 일치한다.

**증명.** 첫 단계는 입력에 의존하지 않아 첫 업데이트에 확정된다. 앞의 $k$개가 정확하면 다음 업데이트의 $k$번째 단계는 정확한 선행 단계만 사용하므로 정확하다. 이후 업데이트도 그 값을 보존한다. □

이는 기존 `causal.rs`의 finite-propagation Picard 근거를 재사용한다. 더 일반적으로 의존 그래프의 최장 경로 깊이를 $d$라 하면 $d+1$개 동시 레벨 업데이트로 충분하다. 행렬 역원이 원래 인과 구조를 보존하는지 확인해야 한다. 모든 단계의 비선형 인수가 현재 단계에도 의존하는 일반 implicit RK로 이 결론을 확장하지 않는다.

단계당 RHS 비용을 $F$, 사용할 작업자를 $P$, 실제 필요한 업데이트 수를 $q\le s$, 각 배치의 동기화·작은 선형 대입 비용을 $A$라 하면 대략

\[
 C_{\rm parallel}\simeq q\lceil(s-1)/P\rceil F+qA+C_{\rm certificate},
\]

를 동일한 원래 단계의 순차 비용과 비교해야 한다. $q=s$이면 일을 반복하므로 단순한 레벨 병렬화가 이기지 못할 수 있다. 재사용 predictor·quotient·homotopy가 $q$를 낮추는 구간이 유용한 대상이다. 선택 알고리즘은 필요한 업데이트 수와 검증비용을 실제로 기록한다.

예컨대 $H_\lambda(U)=(1-\lambda)G(U)+\lambda F(U)$를 따라 predictor를 만들더라도 최종 소비자는 $F(U)=H_1(U)$의 잔차를 평가한다. $H_{\lambda<1}$에서의 수렴은 다른 방정식의 수렴이다. continuation은 좋은 후보를 싸게 만들기 위한 내부 경로이며, 성공의 기준은 정리 1의 **원래 목표 출력 오차**다. 이 구분을 유지하면 유용한 homotopy 후보를 정확도 예외 없이 경쟁시킬 수 있다.

## 3. 다항식 몫대수로 만드는 구조적 역원

### 정리 3 — 근사 소멸다항식에 의한 유한 역원

선형 작용 $B$에 대해 차수 $d$의 다항식 $q$, $q(1)\ne0$가 주어졌다고 하자. 다음은 차수 $d-1$의 다항식이다.

\[
 p(x)=\frac{q(1)-q(x)}{q(1)(1-x)}.
\]

따라서

\[
 (I-B)p(B)=I-D,\qquad D=q(B)/q(1).
\]

특히 $q(B)=0$이면 $p(B)=(I-B)^{-1}$이다. $\|D\|\le\delta<1$이면

\[
 (I-B)^{-1}=p(B)\sum_{j=0}^{\infty}D^j,
 \qquad
 \|(I-B)^{-1}\|\le\frac{\|p(B)\|}{1-\delta}.
\]

**증명.** $q(1)-q(x)$는 $1-x$로 나누어떨어지므로 $p$는 다항식이다. 다항식 항등식에 $B$를 대입하고, $D$가 $B,p(B)$와 가환함을 이용한다. 마지막 식은 수렴하는 Neumann 급수로부터 따른다. □

이는 $\mathbb R[x]/(q)$에서 $1-x$의 역원을 계산하는 구체적 대수기하·대수적 구성이다. $B$의 전체 행렬을 만들지 않고 작용만 계산할 수 있으면 Horner 방식으로 $d-1$회의 작용을 사용한다. $d\ll n$인 구조가 유리하며, 구조의 참/거짓은 표본 벡터 몇 개에서 잔차가 작다는 이유만으로 판정하지 않는다.

현재 `polynomial_resolvent.rs`, `conditioned_polynomial.rs`는 이 아이디어와 원래 연산자 잔차 확인을 이미 구현한다. 정확한 정규화, 불변 operator identity, 저장된 binary64 계수의 결함과 작용 오차를 유지해야 한다. 이는 선택 시 구조적 신뢰도를 별도 입력으로 요구할 이유다.

### 스텝 크기와 직접 연결되는 특수화

선형 성분 $A=-\alpha I+E$, $\alpha\ge0$, $\|E\|\le e$, $\gamma>0$를 알고 있고 $W=I-h\gamma A$를 풀고자 하자. $P=(1+h\gamma\alpha)^{-1}I$를 쓰면

\[
 D=I-PW=\frac{h\gamma E}{1+h\gamma\alpha},\qquad
 \eta(h)=\frac{h\gamma e}{1+h\gamma\alpha}.
\]

목표 수축률 $0<\eta_*<1$에 대한 충분조건은

\[
 e\le\eta_*\alpha \Rightarrow \text{모든 }h>0\text{에서 적합},
\]

\[
 e>\eta_*\alpha \Rightarrow
 h\le\frac{\eta_*}{\gamma(e-\eta_*\alpha)}.
\]

양의 분모를 곱하여 정리하면 바로 얻는다. $x_0=0$, $x_{k+1}=x_k+P(b-Wx_k)$라 하면

\[
 \|x-x_k\|\le\frac{\eta(h)^k}{1-\eta(h)}\|Pb\|.
\]

따라서 오차 예산에서 필요한 반복 수를 **스텝을 실행하기 전에** 정할 수 있고, 예상 비용이 보호된 직접/선형 풀이보다 클 때 그 후보를 피할 수 있다. 이것은 시간 정확도에 관한 스텝 제한이 아니라 **구조적 내부 풀이의 적합 범위**다. $E$의 작용이 모델의 선형 구조에서 제공되면 Jacobian·JVP·AD가 모두 필요 없다.

## 4. 물리 노름과 chart 증거를 적응 제어에 공유하기

원래 상태가 $y=\Psi(t,z)$이고 reduced 벡터장이 $z'=g(t,z)$라고 하자. lift의 결함은

\[
 \xi(t,z)=\partial_t\Psi(t,z)+D_z\Psi(t,z)g(t,z)-f(t,\Psi(t,z)).
\]

연속 reduced 후보가 $z'=g(t,z)+r_z(t)$를 만족하면 lift한 경로의 원래 ODE 잔차는

\[
 r_y(t)=\xi(t,z(t))+D_z\Psi(t,z(t))r_z(t).
\]

따라서 $\|D_z\Psi\|\le K(t)$인 영역에서

\[
 \|r_y(t)\|\le\|\xi(t,z(t))\|+K(t)\|r_z(t)\|.
\]

원래 벡터장이 물리 Euclidean 노름에서 one-sided Lipschitz 상수 $\mu(t)$를 갖고, 후보와 참해가 해당 영역 안에 남으면

\[
 E(t)\le e^{\int_{t_0}^t\mu}\,E(t_0)
 +\int_{t_0}^t e^{\int_s^t\mu}\,[\|\xi(s)\|+K(s)\|r_z(s)\|]ds.
\]

**증명.** chain rule로 첫 잔차식을 얻는다. 참해와 lift한 후보의 차에 대한 미분방정식에 one-sided Lipschitz 조건을 적용하면 $D^+E\le\mu E+\|r_y\|$이다. 적분인자를 곱하고 적분한다. □

이 식은 `ExponentialChirpChart`, cofactor, affine lift, Fourier 위상 경로에 공통으로 적용된다. 시간 의존 chart에서 $\partial_t\Psi$를 빠뜨리면 step-size 제어와 method 선택이 동시에 잘못된 문제를 보게 된다. phase 오차·reconstruction 반올림·리매핑 오차도 원래 노름으로 더해야 한다. 함수의 analytic 구조가 알려져 있으면 이 유도에서 $D_z\Psi$를 사용하더라도 실행 중 AD/JVP를 부를 필요는 없다.

재사용 가능한 구조 증거에는 `model/provider/chart/mesh` epoch, 물리 노름 및 tolerance-vector identity, 적용 영역, 실제 $h$, 출력 계약이 필요하다. norm을 바꾸고 이전의 작은 residual 숫자를 그대로 재사용하지 않는다. 비용 증거와 수학적 구조 증거는 같은 epoch 키를 공유하되, **경험적 비용의 유효기간과 수학적 식의 유효기간은 별도**다.

## 5. 잘못된 방법 전환을 줄이는 비용 구간 판정

정확도 조건을 충족하는 두 후보를 같은 accepted checkpoint와 같은 물리 시간 구간에 비교한다. 후보 $a$와 현행 $b$의 다음 $N$개 구간별 전체 비용을 각각

\[
 C_a^{(j)}\in[L_a,U_a],\qquad C_b^{(j)}\in[L_b,U_b]
\]

로 상계하고, 앞으로 필요한 전환 비용을 $T\le\overline T$라 하자. 비용에는 후보 내부의 실패·재시도·검증·출력·배치 동기화가 들어간다. 두 후보의 오차 수용 조건이 다른 경우에는 비교쌍으로 사용하지 않는다.

### 정리 4 — 유효한 구간 아래의 손해 없는 전환

$0\le\eta<1$에 대해

\[
 \overline T+NU_a\le(1-\eta)NL_b
\]

이면 해당 $N$개 구간에서 전환한 비용은 현행 방법을 유지한 비용의 $1-\eta$배 이하이다.

**증명.** $T+\sum_jC_a^{(j)}\le\overline T+NU_a\le(1-\eta)NL_b\le(1-\eta)\sum_jC_b^{(j)}$. □

이 보장은 지정한 범위에서 실제 비용 구간이 참일 때 성립한다. 세 번의 측정 최솟값·최댓값은 유용한 경험적 envelope지만, 미래 실행시간을 엄밀하게 묶는 증거는 아니다. 현재 코드의 경험적 선택은 이 점을 표시한 채, holdout에서 잘못 선택한 비율과 총비용을 측정한다. 통계적 보장을 붙이려면 적응적 재관측까지 포함하는 coverage 조건이 추가로 필요하다.

구간의 중심 오차가 $u_a,u_b$ 이하이고 실제 고정 비용이 $C_a,C_b$라면,

\[
 C_b-C_a>\eta C_b+\overline T/N+2u_a+2(1-\eta)u_b
\]

는 위 판정이 유익한 후보를 탐지하는 충분조건이다. 실제 $U_a\le C_a+2u_a$, $L_b\ge C_b-2u_b$를 대입하면 얻는다. 따라서 noise와 변동이 클수록 작은 이익에 대해 **전환을 보류하는 영역**이 커진다. 이는 오탐 감소와 놓치는 미세 이득 사이의 정량적 절충이다.

스텝 크기나 상태가 바뀌면 비용도 바뀐다. $C(x)$에 대한 유효한 변동 상계 $L_C\|x-x_0\|$가 있으면 관측 구간을 그만큼 넓힐 수 있다. 그런 상계가 없는 경험적 설정에서는 h/state locality, epoch, age와 관측 drift로 증거를 만료시키고 다시 제한된 비교를 수행한다. 작은 h만으로 stiff label을 붙일 필요는 없다.

실제 공통 함수는 `crates/vigilode/src/selection_evidence.rs`의 `compare_qualified_costs`다. `SelectionIdentity`와 단위를 정확히 비교하고, `CostEvidence`의 수용 여부·증거 종류와 비용 구간을 입력받는다. `execution/RESEARCH_KO.md`에 실제 API와 실행기 소비 경로가 있다. 직접 관측한 작업량 point interval과 경험적 timing envelope는 미래 비용 상계와 구별하여 기록한다.

## 6. 탐색비용의 누적 상한

추가 탐색에 사용할 credit을

\[
 B=B_0+\varepsilon C_{\rm productive}-C_{\rm probe}
\]

로 정의한다. $B_0\ge0$는 초기 탐색비, $\varepsilon\ge0$는 허용 비율이다. 생산적 비용과 탐색비는 중복 없이 기록한다. 탐색 전에 그 탐색의 최대 비용 $P_{\max}\le B$를 예약하고 실제 비용 $P\le P_{\max}$를 지불한다. 실패한 탐색도 동일하게 지불한다.

### 정리 5 — 모든 접두 구간의 탐색비용 상한

위 절차에서는 실행의 모든 시점에서

\[
 C_{\rm probe}\le B_0+\varepsilon C_{\rm productive}
\]

가 유지된다.

**증명.** 초기 credit은 비음수가 아니다. 생산적 작업은 credit을 증가시키고, 탐색은 기존 credit 이하로 예약되어 credit을 음수로 만들지 않는다. 귀납적으로 $B\ge0$이며 식을 재배열하면 결론이다. □

callback 하나가 예약비를 무한히 넘을 수 있는 wall-clock 설정에서는 이 보장을 그대로 얻을 수 없다. 카운트로 제한되는 RHS/JVP/단계 작업에는 결정적 작업량 상한을 사용할 수 있고, 시간 한계에는 반환 가능한 callback 또는 별도의 실행 중단 계약이 필요하다.

이미 쓴 calibration 비용은 전체 보고서에는 남지만, **다음 실행을 어느 방법으로 할지** 결정할 때 다시 회수 비용으로 부과하지 않는다. 앞으로 새로 지불할 setup/transition만 정리 4의 $T$에 들어간다. 그렇지 않으면 과거 비용 때문에 매번 더 비싼 방법을 계속 선택하게 된다.

### 현재 pilot에 적용하는 음성 비교의 재사용

추가한 entry-probe 정책은 경쟁 후보가 계속 지는 동안 대기 간격을 $\min(2^k b,M)$로 늘린다. $b$는 기본 간격, $M$은 최대 만료 간격, $k\ge1$은 연속으로 유익하지 않았던 비교의 수다. 첫 비교가 첫 explicit batch에서 이루어지고, 변화 감지 없이 항상 같은 결과가 나오는 경우 $p$번째 비교가 가능한 가장 이른 batch는

\[
 n_p=1+\sum_{k=1}^{p-1}\min(2^kb,M).
\]

cap에 닿기 전에는 $n_p=1+2b(2^{p-1}-1)$이므로 총 explicit batch $N$개에서 비교 횟수는

\[
 P(N)\le1+\left\lfloor\log_2\!\left(1+\frac{N-1}{2b}\right)\right\rfloor.
\]

조건이 맞지 않아 비교를 건너뛰면 횟수는 더 줄어든다. 여기서 $N$은 pressure batch만이 아니라 **모든 explicit batch**의 수다. 시간이 충분히 지나고 pressure가 있는 비교 가능한 batch가 오면 비교를 갱신하므로 정적인 lifetime 횟수 제한 뒤의 영구적인 미탐도 피할 수 있다. 최대 간격 $M$은 다음 pressure가 있는 적격 batch에서 다시 비교할 시점을 정한다. 모든 physical-time 구간에서의 탐지 지연을 $M$으로 묶는 것은 아니다.

현행 구현의 조기 갱신 조건은 (a) 비용/전진시간이 커지고 proposed h가 함께 줄거나, (b) rejection이 둘 이상이고 거절 비율이 기준보다 증가한 경우다. 이는 물리적 강성 판정의 증명이 아니라 새 비교를 요청하는 관측 신호다. 실제 method 변경은 같은 checkpoint의 경쟁 계산을 통과해야 한다. 위 token-credit 정리의 전체 비용 보장은 **별도의 후속 구현**이며, 현재 exponential backoff와 혼동하지 않는다.

## 7. 하나의 실행 절차

1. accepted checkpoint에서 model·chart·operator·norm·output·resource identity를 고정한다.
2. 구조 제공자의 적용 범위에서 derivative-free 후보를 구성한다. 다항식 차수, causal 반복 수와 허용 $h$를 계산한다.
3. 원래 목표 잔차와 원래 상태 오차를 평가한다. 잘못된 chart, 부정확한 단계, 출력 예산 초과를 서로 다른 이유로 기록한다.
4. 현재 스텝을 줄일지, 내부 반복을 늘릴지, 다른 구조/방법을 쓸지 같은 오차 예산 아래 비교한다.
5. 허용된 탐색비 안에서 현재 선택과 가장 관련 있는 경쟁 후보 하나를 같은 checkpoint에서 비교한다. 외부 효과가 있는 callback은 transactional/replay-safe 계약을 요구한다.
6. 비용 envelope가 충분히 분리될 때만 방법·스케줄을 전환한다. 애매하면 현재 방법을 유지하고 새 정보가 있는 시점에 재평가한다.
7. 수락 뒤에만 상태·PI/FSAL 캐시·모델 epoch와 telemetry를 갱신한다. 방법 변경이나 상태 변경으로 무효화된 캐시는 버린다.

실제 속도 향상은 이 절차가 줄인 RHS/JVP/재시도와 추가한 검증·선택비를 함께 측정해서 판정한다. 현재 단계의 핵심은 특수 후보를 무조건 켜는 것이 아니라, **유효한 구조의 이득을 보존하면서 잘못 적용되는 구간과 비싼 탐색을 줄이는 것**이다.

## 문헌과 근거의 역할

- Petzold, 1983, DOI `10.1137/0904010`: stiff/nonstiff 방법을 계산 효율 관점에서 자동 선택하는 선행 근거. 이전 조사에서 확인한 자료를 재사용한다.
- Einkemmer, 2018, DOI `10.1016/j.apnum.2018.06.002`: iterative implicit 방법에서는 최대 허용 h보다 비용/전진량을 직접 최소화하는 전략이 유리할 수 있음을 다룬다. 이번에는 publisher의 introduction·section summaries를 확인했다. 여기의 특정 보장·구현은 위에서 별도로 유도했다.
- Ranocha–Giesselmann, 2024, DOI `10.1007/s44207-024-00001-0`, arXiv `2307.12677v3`: residual 기반 오차 제어와 step-size controller의 동역학을 구별하고 one-sided Lipschitz 기반 오차 전파를 제공한다. 본문 §2.1과 controller 안정성 논의를 직접 확인했다.
- Enright–Hayes, 2007, DOI `10.1145/1206040.1206041`: RK 연속 경로의 defect를 적은 추가 RHS로 감시하는 방법의 선행 근거. author manuscript를 확인했다.
- Hochbruck–Leibold–Ostermann, 2020, DOI `10.1007/s00211-020-01120-4`: Lawson 방법에서 full order를 얻는 정칙성·경계 조건을 구별한다. 이전 연구의 원문 확인을 재사용한다.

구체적인 파일과 주장 대응은 `SOURCE_LEDGER.json`, 정확한 유한대수 검산은 `check_derivations.py`와 `CHECK_RESULTS.json`에 있다. 검산은 위 증명의 유한 사례 검사이며 일반 명제의 증명을 대체하지 않는다.
