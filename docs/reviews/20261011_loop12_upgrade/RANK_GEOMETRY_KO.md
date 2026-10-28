# Rank-r·다중 군집 packet: 정확한 memory 축약, factorial 수렴, 동일 맵의 작업량 감소

작성: 2026-10-11. 근거 상태: **derived**. 작은 새 Fraction 항등식 검산은 별도 `RANK_EXACT_CHECKS.json`에 보존한다. 후보 생성자 자신의 PROMOTE 판정은 없다. 이전 scalar Loop12 campaign과 기존 승인 정리는 다시 실행하지 않았다. 이 문서는 새 model-specific 이론이며 현재 VigilODE native 실행·wall-clock 결과가 아니다.

입력은 첨부 `RESEARCH_NOTE_KO.md` §§3–9의 scalar moment packet과 기존 `POLYNOMIAL_KO.md`, `HOMOTOPY_KO.md`, `CERTIFICATE_ATLAS_KO.md`다. 하네스 `PROJECT_INSTRUCTIONS.md`, `docs/MODEL_ROUTING.md`, template `state/RESEARCH_STATE.md`를 읽었다. 후자의 NOT_RUN을 실행 결과로 상속하지 않는다. 이전에 선택된 사용자 지정 Astra v4.0.0을 따른다. 실제 모델 신원 UNKNOWN, 모델 성능 NOT_EVALUATED다.

## 1. 정의와 계약

실수 또는 복소 상태 $y(t)\in\mathbb C^n$, 관측량 $m(t)\in\mathbb C^r$에 대해

\[
 y'=Dy+U F(V^*y),\qquad y(0)=y_0,\qquad m=V^*y,                 \tag{1}
\]

를 고정한다. $D=\operatorname{diag}(d_j)$는 **모델이 직접 제공하는 알려진 대각 연산자**이고 $U\in\mathbb C^{n\times r}$, $V^*\in\mathbb C^{r\times n}$도 고정된 exact target data다. $V^*$는 표기상 adjoint이며 여기서는 공급된 선형 관측 사상 자체를 뜻한다. 모델에서 이 구조를 발굴하는 비용·일반 dense diagonalization 비용을 0으로 놓지 않는다. $r$은 descriptor가 실제 사용하는 채널 수다. 작은 singular value만 보고 numerical rank를 exact rank로 바꾸지 않는다. 불필요한 채널을 제거하려면 $U F(V^*y)$의 항등식을 다시 증명해야 한다.

고정 물리 척도로 무차원화한 subordinate vector/operator norm을 사용한다. $F$는 상태 변화율의 단위를 가지며, $D,c_a,\delta_j$는 시간 역수다. 따라서 $\delta_jt$, $BLT$는 무차원이다. $U,V^*$에 물리 단위를 포함한다면 아래 $B L$의 단위가 시간 역수인 동일한 scale convention을 써야 한다. $c,\hbar,k_B$를 1로 놓는 가정은 없다.

각 $j$가 한 군집 $C_a$에 속하고

\[
 d_j=c_a+i\delta_j,\quad c_a=-\kappa_a+i\omega_a,\quad
 \kappa_a\ge0,\quad\delta_j\in\mathbb R,\quad |\delta_j|\le w_a
                                                               \tag{2}
\]

라고 하자. 군집 수는 $c$, 군집별 차수는 $p_a\ge0$, $P_a=p_a+1$, $J=\sum_aP_a$다. 폭이 커질수록 필요한 $p_a$가 커지며, 군집 수와 rank가 커지는 비용도 보존한다. 다른 damping rate를 가진 행은 이 계약에서는 별도 군집 또는 별도 오차 처리 대상이다. “모든 고유값이 왼쪽에 있으니 동일 damping”이라고 묶지 않는다.

각 row $u_j=e_j^*U$, column $v_j=V^*e_j$를 정의한다. $u_jF$는 scalar이고 $v_ju_j$는 $r\times r$ rank-one 행렬이다. 다음 nonnegative 상계를 보유한다.

\[
 B_a=\sum_{j\in C_a}\|v_j\|\|u_j\|,\quad
 A_a=\sum_{j\in C_a}\|v_j\||y_{0j}|,\quad B=\sum_aB_a.       \tag{3}
\]

이는 보수적이지만 cancellation이나 미관측 방향을 임의로 제거하지 않는 full-channel 상계다. 더 작은 검증된 kernel norm 상계가 있으면 대체할 수 있다.

## 2. R1 — 정확한 rank-r Volterra 축약과 reconstruction

\[
 f_0(t)=V^*e^{Dt}y_0,\qquad K(t)=V^*e^{Dt}U.                  \tag{4}
\]

그러면 원식 (1)의 해와 다음 memory equation의 해는 일대일 대응한다.

\[
 m(t)=f_0(t)+\int_0^tK(t-s)F(m(s))\,ds.                    \tag{5}
\]

Memory 해가 주어지면 모든 독립 초기 데이터를 보존하여

\[
 y_j(t)=e^{d_jt}y_{0j}+\int_0^t e^{d_j(t-s)}u_jF(m(s))\,ds. \tag{6}
\]

**증명.** 변분상수법으로 (6)을 얻고 $V^*$를 곱하면 (5)다. 역으로 (5)의 연속 해로 (6)을 정의하면 $V^*y=m$이고, 미분하면 원 ODE와 초기조건을 만족한다. 따라서 선택한 유일성 영역 안에서는 정확한 대응이다. $y_0\mapsto V^*y_0$만 저장한 것은 아니며 free profile $f_0$와 (6)이 transverse 초기 정보를 보존한다. □

이는 $n$차원 비선형 tangent problem을 만들지 않고 $r$차원 비선형 함수를 평가하는 정리다. 선형 memory가 남기 때문에 $r$차원 Markovian ODE라고 부르지 않는다. 예외적으로 $V^*D=A_rV^*$라는 **정확한** intertwining이 있으면

\[
 m'=A_rm+V^*U F(m),                                      \tag{7}
\]

로 닫힌다. 이 조건은 관측 사상의 kernel이 $D$에 의해 보존된다는 뜻이다. 행렬 $V^*$가 full row rank이면 선형항이 관측량만으로 닫히기 위한 필요충분조건이기도 하다. 필요성은 같은 $m$을 갖는 두 상태의 차이 $z\in\ker V^*$에 대해 $V^*Dz=0$이어야 함에서 따른다. 충분성은 quotient vector space 위에 $A_r$를 정의하면 된다.

## 3. R2 — 군집별 matrix moment packet과 factorial tail

원데이터에서

\[
 M_{ak}=\sum_{j\in C_a}v_ju_j\delta_j^k,\qquad
 \nu_{ak}=\sum_{j\in C_a}v_jy_{0j}\delta_j^k,
 \qquad 0\le k\le p_a                                    \tag{8}
\]

를 한 번 컴파일한다. Approximate kernel과 free profile은

\[
 K_p(t)=\sum_a e^{c_at}\sum_{k=0}^{p_a}M_{ak}\frac{(it)^k}{k!},\qquad
 f_p(t)=\sum_a e^{c_at}\sum_{k=0}^{p_a}\nu_{ak}\frac{(it)^k}{k!}. \tag{9}
\]

$\tau_a(t)=(w_at)^{p_a+1}/(p_a+1)!$라 두면 모든 실수 $0\le t\le T$에 대해

\[
 \|K(t)-K_p(t)\|\le\sum_a B_ae^{-\kappa_at}\tau_a(t),\qquad
 \|f_0(t)-f_p(t)\|\le\sum_a A_ae^{-\kappa_at}\tau_a(t).       \tag{10}
\]

특히

\[
 \sup_t\|f_0-f_p\|\le\varepsilon_f:=\sum_a A_a\tau_a(T),\qquad
 \int_0^T\|K-K_p\|(s)ds\le
 \varepsilon_K:=\sum_a B_a\frac{T\tau_a(T)}{p_a+2}.         \tag{11}
\]

**증명.** 실수 δ에 대한 integral remainder는

\[
 e^{i\delta t}-\sum_{k=0}^p\frac{(i\delta t)^k}{k!}
 =\frac{(i\delta)^{p+1}}{p!}\int_0^t(t-s)^p e^{i\delta s}ds.
\]

따라서 절댓값은 $(|\delta|t)^{p+1}/(p+1)!$ 이하이며 여기에는 $e^{w_at}$가 필요 없다. 이를 각 $v_ju_j$, $v_jy_{0j}$에 곱해 삼각부등식과 (3)을 적용하면 (10)이다. $e^{-\kappa_at}\le1$를 사용하고 $t^{p_a+1}$을 적분하면 (11)이다. □

이 bound는 군집 중심의 큰 ω에 직접 의존하지 않는다. 그러나 $e^{c_at}$의 평가·phase reduction·outward enclosure 비용이 무료라는 결론은 아니다. 실제 binary64 frequency가 physical frequency를 불확실하게 표현하면 그 차이도 target-data uncertainty로 추가해야 한다.

작은 폭에는 작은 차수만 필요하다. $B_a,A_a,w_a,T$와 허용 오차가 $n$에 대해 균일하게 bounded인 family에서는 충분한 $p_a=O(w_aT+\log(1/\epsilon))$를 고를 수 있다. 예컨대 $k=p_a+1\ge2e w_aT$이면 $k!\ge(k/e)^k$에서 $\tau_a(T)\le2^{-k}$다. 계수 precision, nonlinear waveform degree, iteration 횟수는 이 kernel-tail 문장만으로 고정되지 않는다.

### Nilpotent jet realization과 합체 극한

$X_{ak}\in\mathbb C^r$, $X_{a,p_a+1}=0$를 두고

\[
 X'_{ak}=c_aX_{ak}+iX_{a,k+1}+M_{ak}F(m_p),\quad
 X_{ak}(0)=\nu_{ak},\quad m_p=\sum_aX_{a0}                 \tag{12}
\]

를 풀면 $m_p=f_p+K_p*F(m_p)$다. 선형 block shift의 지수함수를 전개하면 첫 row에 $(it)^k/k!$가 나오므로 직접 확인된다. 상태 수는 $rJ$다. 이 shift nilpotence는 **근사 packet의 표현**이고 원 diagonal $D$가 nilpotent라는 주장이 아니다. 기존 symmetric-negative `joint_phi_action` 인증을 skew-complex $D$ 또는 이 비정규 jet block에 그대로 적용할 수 없다.

$w_a=0$인 exact 군집에는 $p_a=0$, tail 0이다. 그러면

\[
 X'_{a0}=c_aX_{a0}+M_{a0}F(\sum_bX_{b0})                  \tag{13}
\]

는 원래 observable의 정확한 $cr$차원 ODE다. 서로 다른 중심들이 합쳐져도 식에 $(c_a-c_b)^{-1}$나 Vandermonde inverse가 없으므로 이 표현 자체는 연속이다. 같은 중심의 moment를 더하면 더 작은 exact packet이 된다. 출력의 초기값은 여전히 모두 보존한다.

## 4. R3 — original-target 정확도와 모든 모드의 재구성 오차

원해 $m$과 후보 $\widehat m$가 공통 convex domain에 있고 그 domain에서 $F$가 $L$-Lipschitz이며 $\|F(\widehat m(t))\|\le G$라고 하자. 복소 비정칙 $F$는 실수 $2r$차원 함수로 취급한다. 후보의 **연속 시간** residual이

\[
 r_p=\widehat m-f_p-K_p*F(\widehat m),\qquad
 \|r_p\|_\infty\le\mathcal D                             \tag{14}
\]

로 검증되었다면

\[
 E_m:=\|m-\widehat m\|_\infty
 \le e^{BLT}\,[\mathcal D+\varepsilon_f+G\varepsilon_K].   \tag{15}
\]

**증명.** $e=m-\widehat m$에 대해

\[
 e=(f_0-f_p)+(K-K_p)*F(\widehat m)
       +K*[F(m)-F(\widehat m)]-r_p.
\]

한편 “원” kernel에는 $\|K(t)\|\le B$가 성립한다. 따라서 $\|e(t)\|\le\mathcal D+\varepsilon_f+G\varepsilon_K+BL\int_0^t\|e(s)\|ds$이고 Gronwall 부등식이 (15)를 준다. 근사 kernel의 불필요하게 큰 norm으로 오차 전달을 평가할 필요가 없다. □

각 $j\in C_a$의 근사 propagator를 $E_{j,p}(t)=e^{c_at}\sum_{k=0}^{p_a}(i\delta_jt)^k/k!$라 하고

\[
 \widehat y_j(t)=E_{j,p}(t)y_{0j}
       +\int_0^tE_{j,p}(t-s)u_jF(\widehat m(s))ds          \tag{16}
\]

로 재구성하면

\[
 |y_j(t)-\widehat y_j(t)|
 \le \tau_a(T)\left[|y_{0j}|+\|u_j\|G\frac{T}{p_a+2}\right]
       +\|u_j\|LT E_m.                                  \tag{17}
\]

초기 propagator tail, forcing propagator tail, exact propagator를 통한 $F(m)-F(\widehat m)$ 전달을 각각 bound하면 된다. 원단위 norm이나 WRMS로 바꾸려면 (17)을 해당 scale로 나누어 합성한다. 모든 mode의 amplitude bound가 동일하다고 임의 가정하지 않는다. Reconstruction의 평균은 $V^*\widehat y=f_p+K_p*F(\widehat m)=\widehat m-r_p$이므로 후보 observable과 정확히 같을 필요가 없다.

Moment compiler error, coefficient rounding, nonlinear evaluation error, quadrature remainder, phase error, 재구성 rounding은 (14) 또는 (10) 또는 (17)의 대응 항에 **정확히 한 번씩** 더한다. Collocation node에서만 작은 residual을 관측한 것으로 (14)를 승인하지 않는다. Initial state uncertainty는 free-profile/reconstruction term으로 전파한다. 군집 membership·model epoch·norm이 바뀌면 옛 certificate를 그대로 쓰지 않는다.

여기서 원해와 후보의 domain 가정은 실질적이다. Polynomial $F$에 global Lipschitz를 가정할 수 없다. Model invariant, a priori growth bound, 또는 검증된 continuation tube로 해당 domain을 공급해야 한다. “오차가 작다고 가정해서 같은 domain이라고 하고, 같은 domain이므로 오차가 작다”는 순환은 허용하지 않는다.

## 5. R4 — 시간 인과성이 주는 factorial 수렴과 no-fold homotopy

이 절은 continuous Volterra operator에 대한 양의 정리다. Degree projection을 넣은 algebraic system은 별도다.

먼저 $F$가 전체 실수 $2r$ 공간에서 $L$-Lipschitz라고 하자. $K$는 연속이고 $\sup_{[0,T]}\|K\|\le B$, $f_0$는 연속이다. $\Phi(m)=f_0+K*F(m)$, $m^{(0)}=f_0$, $m^{(s+1)}=\Phi(m^{(s)})$, $D_0=\|m^{(1)}-m^{(0)}\|_\infty$, $a=BL$라 두자. 그러면 모든 finite $T$에서 unique fixed point가 존재하고

\[
 \|m^{(s+1)}(t)-m^{(s)}(t)\|
       \le D_0\frac{(at)^s}{s!},\qquad
 \|m-m^{(s)}\|_\infty
       \le D_0e^{aT}\frac{(aT)^s}{s!}.                   \tag{18}
\]

**증명.** 차분 부등식 $d_{s+1}(t)\le a\int_0^td_s(u)du$를 반복하면 단체적분 부피 $t^s/s!$가 나온다. $\sum_sD_0(aT)^s/s!$가 수렴하므로 반복열은 균등 Cauchy이고, 연속성과 적분의 극한 교환으로 fixed point다. Tail은 $\sum_{k=s}^\infty x^k/k!\le x^s e^x/s!$를 쓴다. 두 fixed point 차이는 Gronwall 또는 반복적 단체적분으로 0이다. $aT<1$은 필요 없다. $a=0$이면 한 번에 닫힌다. □

이는 실패할 수 있는 순수 Banach 상계 $aT$ 대신 causal Volterra powers의 factorial을 사용한다. 고정 $aT$에서 충분한 횟수 $s=O(\log(1/\epsilon))$를 얻고, 큰 $aT$에는 적어도 그 크기를 반영해야 한다. 단순 충분조건은

\[
 s\ge\max\left(2e aT,
       \log_2(D_0e^{aT}/\epsilon),0\right)               \tag{19}
\]

의 정수 상향 반올림이다. 실제 단체적분 덕분에 더 좋은 차수를 얻을 수도 있으나 이를 optimality 주장으로 바꾸지 않는다.

Finite precision·projection에서 각 sweep의 함수 오차가 $\|\eta_s\|_\infty\le\eta$이고 실제 recurrence $\widehat m^{(s+1)}=\Phi(\widehat m^{(s)})+\eta_s$를 따른다고 하자. 동일한 causal majorant 반복으로 동일 초기값의 exact iteration과의 차이는

\[
 \|\widehat m^{(s)}-m^{(s)}\|_\infty
       \le\eta\sum_{k=0}^{s-1}\frac{(aT)^k}{k!}
       \le\eta e^{aT}.                                 \tag{20}
\]

초기 함수 오차가 $\eta_0$이면 추가 $\eta_0(aT)^s/s!$다. 따라서 실제 scheme이 (20)의 **연속 함수 오차**를 제공할 경우 작은 $aT$ 없이도 명시적인 accuracy budget이 생긴다. 그러나 고정 polynomial projection이 항상 작은 η를 준다는 결론은 아니다. 그 오차는 analytic tail 또는 certified approximation으로 별도 계산해야 한다. Loop12의 coefficient analytic-ball 정리는 그 한 방법이며, 그 정리의 smallness 조건을 이 절로 자동 삭제하지 않는다.

### Homotopy branch의 no-fold와 독립 λ endpoints

$\lambda\in[0,1]$에 따라 알려진 $d_j(\lambda)=c_a+i\lambda\delta_j$를 사용하거나 feedback amplitude를 $\lambda F$로 둔다. $K_\lambda,f_\lambda$와 $F$가 해당 변수에서 $C^1$, $F$의 상태 미분 norm이 전체에서 $L$ 이하라고 하자. Uniform $B$가 존재하면 각 λ에서 위 existence/uniqueness가 성립한다. 해 $m_\lambda$의 선형화는

\[
 (\mathcal A_\lambda h)(t)
  =\int_0^tK_\lambda(t-s)DF(m_\lambda(s))h(s)ds,
 \qquad \|\mathcal A_\lambda^k\|\le (BLT)^k/k! .          \tag{21}
\]

따라서

\[
 (I-\mathcal A_\lambda)^{-1}
       =\sum_{k=0}^\infty\mathcal A_\lambda^k,
 \qquad\|(I-\mathcal A_\lambda)^{-1}\|\le e^{BLT}.         \tag{22}
\]

증명은 (18)의 simplex 적분과 같으며 absolute operator convergence로 양쪽 inverse임을 확인한다. Banach 공간 implicit-function theorem을 적용하면 unique $C^1$ branch가 생기고, λ projection에 대한 fold가 없다. 강한 미분 가능성이 없으면 unique continuous branch까지만 주장한다. $F$가 비정칙 cubic이면 이를 실 Banach 공간에 적용한다. $C^1$만으로 complex-λ Taylor radius 또는 exponential spectral convergence를 주장하지 않는다.

따라서 실제 필요한 여러 λ target을 각각 동일 initial-value 계약에서 독립적으로 풀 수 있다. 이전 λ의 corrected root를 반드시 받아야 한다는 branch-selection 의존성은 제거된다. 그러나 한 λ 안의 Picard sweeps는 순차이며, 여러 physical time windows는 이전 window의 초기조건이 없으면 독립적이지 않다. 이 정리는 원래 RVJ represented K-form의 유한 stage nilpotence 또는 finite-round theorem이 아니다. 그 source에는 별도 H5 leakage 조건이 남는다.

다수 λ 출력이 실제로 요청되고 $P$ workers에 job을 배정했다면 parallel span은 worker별 누적 λ job 비용의 최대값에 공통 setup·certificate·dispatch를 더한 값 이하이다. 그것이 동일 출력의 sequential comparator보다 작다는 전체 비용 gate가 성립할 때만 속도 모델상의 승격이 가능하다. 하나의 endpoint만 필요한데 불필요한 λ들을 계산해 speedup이라고 부르지 않는다.

Polynomial $F$ 등 local-Lipschitz 경우에는 invariant compact domain 또는 공통 continuation tube와 모든 branch 해의 존재를 별도로 제공하면 (21)–(22)는 그 tube 안에서 그대로 성립한다. Derivative inverse만으로 finite-time blowup 부재를 증명한 것은 아니다.

## 6. R5 — 동일 waveform 맵의 정확한 집계와 strict work gate

### R5a. Projection을 포함한 same-map 항등식

어떤 고정 선형 시간 표현/투영 $\Pi$를 모든 mode에 동일하게 적용한다고 하자. Full iteration은

\[
 y_j^{(s+1)}=\Pi\left[E_{j,p}y_{0j}
                    +E_{j,p}*(u_jF(m^{(s)}))\right],
 \qquad m^{(s)}=V^*y^{(s)}.                              \tag{23}
\]

Reduced iteration은

\[
 \widetilde m^{(s+1)}=\Pi\left[f_p+K_p*F(\widetilde m^{(s)})\right]. \tag{24}
\]

초기 observable이 같으면 모든 $s$에서 $m^{(s)}=\widetilde m^{(s)}$다. **증명:** $V^*$의 행별 합, causal convolution, scalar 선형 투영은 서로 교환된다. (8)의 정확한 moment identity를 사용하면 full 합은 (24)다. 같은 observable에서는 $F$도 같으므로 귀납법으로 끝난다. $F$는 비선형이어도 무방하며 양쪽 모두 각 sweep에서 동일한 $F$를 한 번만 평가한다. □

동일한 deterministic forcing approximation 연산자 $\mathcal F$를 양쪽 $F$ 대신 적용해도 same-map 증명은 같다. 이 경우 R3의 residual은 원래 $F$로 계산하여 forcing projection error를 포함해야 한다.

Finite rounding은 이 exact identity의 전제가 아니다. Rounding된 두 프로그램의 bit identity를 주장하지 않고 각각 R3의 physical certificate를 사용한다. Full output을 마지막 forcing profile에서 공유 reconstruction으로 얻을 때 **free profile과 shared forcing integral에 (23)과 같은 Π를 적용한 출력만** 그 same-sweep full profile과 정확히 일치한다. R3의 unprojected original-target reconstruction과 이 projected same-map 출력을 구분한다. 마지막 입력 observable과 출력 observable 사이의 차이는 해당 sweep residual이므로 residual 검사를 삭제하지 않는다.

### R5b. Rank와 cluster 폭을 실제 비용에 남기는 계약

Forcing waveform은 공통 scalar basis에서 $F(\widehat m(t))=\sum_{\ell=0}^{q-1}g_\ell b_\ell(t)$로 표현한다. Endpoint reconstruction을 위한 shared vector integrals는

\[
 Z_{ak}(t)=\int_0^t e^{c_a(t-s)}\frac{[i(t-s)]^k}{k!}
                    F(\widehat m(s))ds\in\mathbb C^r.    \tag{25}
\]

그러면

\[
 \widehat y_j(t)=E_{j,p}(t)y_{0j}
                         +u_j\sum_{k=0}^{p_a}\delta_j^kZ_{ak}(t). \tag{26}
\]

식 (26)은 R3의 unprojected 출력이다. Projected same-map 출력을 요청하면 이를
\[
 y_{j,\Pi}(t)=y_{0j}(\Pi E_{j,p})(t)
       +u_j\sum_{k=0}^{p_a}\delta_j^k(\Pi Z_{ak})(t)
\]
로 바꾼다. Π의 비용은 아래 basis/projection 비용에 포함한다. 예컨대 Π가 degree-0이고 forcing이 constant이면 (23)은 초기상수만 남기므로, unprojected 선형-in-time 출력과 같다고 주장할 수 없다.

원데이터 moment setup은 dense arbitrary $U,V^*$에서 $O(nr^2P_{\max})$, initial moment는 $O(nrP_{\max})$다. Nonlinear sweep의 matrix-vector coefficient contraction은 $O(r^2Jq)$이고, basis integration·projection 비용 $C_{\mathrm{basis}}(c,p_a,q)$가 추가된다. Exponential basis의 안정적 계산과 large-ω phase evaluation을 여기서 숨기지 않는다. 각 출력 시각마다 (25)의 shared setup에 $O(rJq)+C_{\mathrm{basis,out}}$, 모든 mode 평가에 $O(nrP_{\max})$가 든다. 즉 $R_{\mathrm{out}}$개 출력에는 **shared setup도 매번** 부과한다.

\[
\begin{split}
W_{\rm packet}\le C_{\rm compile}+C_{\rm proof}
 &+S\,[C_F+C_{\rm contraction}+C_{\rm basis}+C_{\rm residual}]\\
 &+R_{\rm out}\,[C_{\rm shared,out}+C_{\rm all\ modes,out}]
 +C_{\rm rejected}+C_{\rm fallback}.                       \tag{27}
\end{split}
\]

모든 $C$는 해당 실제 알고리즘의 상계 또는 실제 누적 비용이다. Matrix moment가 이미 작더라도 초기 데이터와 전체 출력의 Ω$n$ 비용은 사라지지 않는다. $r,c,p_a$가 $n$과 함께 커지거나 $F$ 내부가 full $n$ model을 다시 평가하면 작은-r 이득을 주장할 수 없다.

정확히 같은 target·Π·차수·sweep·출력·오차 예산에 대해 full comparator의 비용 하계 $W_{\rm full,low}$가 있고 (27)의 상계가 그보다 작으면 strict total arithmetic work reduction이다. 양쪽에서 동일하게 실행한 nonlinear coefficient product, common scalar weight setup, physical certificate가 있으면 같은 공통 비용으로 정확히 상쇄할 수 있다. 그렇지 않으면 새 경로에 추가한다. Best available comparator가 이미 이 moment aggregation을 한다면 이를 새로운 speedup으로 세지 않는다.

여러 군집에서 고차 exponential-polynomial basis 연산이 싸다고 가정해 $C_{\rm basis}$를 지우지 않는다. 따라서 일반 cluster 정리는 (27)의 비용 gate이며, 다음 순수 polynomial 하위 모델이 비어 있지 않은 strict improvement를 구체적으로 증명한다.

### R5c. 양의 엄격 여유를 가진 exact 연산량 예

$D=0$, $r=2$, $p=0$, $q=4$, $S=16$, $R_{\rm out}=1$이며 $U,V^*$는 full rank 2인 알려진 dense matrices라 하자. Forcing은 각 sweep에 동일하게 공급된 degree 3 polynomial이고, Π는 적분 결과의 degree 4를 모두 유지하는 같은 선형 표현이다. 이 예는 generic cubic nonlinearity의 degree가 항상 3이라는 주장이 아니다. 두 비교 경로가 동일하게 생성한 $q=4$ forcing representation을 고정한다. 생성오차가 있으면 공통 residual에 포함한다.

비용 단위는 exact scalar multiplication 한 번이다. Scalar integration weight의 사전 준비, free-profile 상수 입력, nonlinear coefficient generation, 같은 residual proof 비용을 양쪽 공통 비용으로 둔다. Packet의 나머지 비용 상계는 다음과 같다.

| 항목 | multiplication 상계 |
|---|---:|
| $M=V^*U$ 직접 컴파일 | $nr^2$ |
| 초기 $\nu=V^*y_0$ | $nr$ |
| $S$회 $M g_\ell$와 scalar integral weights | $S(r^2q+rq)$ |
| 마지막 forcing의 shared endpoint integral | $rq$ |
| 모든 endpoint $U Z$ | $nr$ |

따라서 $n=1024$에서 $W_{\rm packet,extra}\le8584$다. Full comparator는 매 sweep 각 mode의 $u_jg_\ell$를 만든 후 scalar coefficient integration을 개별 수행하는 명시된 full-profile algorithm이다. 계수 1의 곱을 생략해도, 적분 계수 $1/2,1/3,1/4$의 개별 적용만으로 $Sn(q-1)=49152$의 하계가 있다. 그러므로 나머지 추가 proof·실패·fallback 비용의 **차액**이 40568 미만이면 total scalar-multiplication count가 엄밀히 감소한다. 전체 산술 work 또는 시간은 추가·메모리·integer bitlength까지 포함하는 별도 ledger를 사용해야 한다. 이 숫자를 “전체 5.7배 속도”로 읽으면 안 된다.

덧셈까지 포함한 같은 unit-cost scalar arithmetic 모형에서도 양의 여유가 남는다. Packet의 추가 덧셈 상계는 compiler의 $(n-1)r^2+(n-1)r$, sweep의 $Sqr(r-1)$, shared endpoint의 $r(q-1)$, 모든 mode의 $n(r-1)+n$으로 총 8320이다. 따라서 전체 추가 곱셈·덧셈은 $8584+8320=16904<49152$다. 공통 비용을 상쇄한 나머지 setup·certificate·실패 비용이 32248 scalar operations 미만이면 **곱셈만이 아니라 총 scalar arithmetic work의 엄격한 감소**가 보장된다. Complex multiplication과 addition에 다른 비용을 부여하거나 arbitrary-precision bit complexity를 비교하려면 그 비용모형으로 (27)을 다시 평가한다.

이 비교자는 일부러 $F$를 $n$번 호출하지 않는다. 이미 $V^*U$로 계산하는 reduced comparator와는 이 차이가 없다. $D=0$이라는 admissible family에서 exact quotient가 일으키는 절감이므로, 더 강한 known-structure baseline에는 tie가 합리적인 결과다. 비자명한 rank-2 kernel과 arbitrary independent $y_0$를 허용하며, rank 1 scalar loop의 단순 재실행이 아니다.

## 7. R6 — 몫대수·불변 다양체와 memory reduction의 정확한 구분

Monic $q_D(z)=z^d+\sum_{k=0}^{d-1}a_kz^k$가 full $D$에 대해 $q_D(D)=0$이라 하자. $x_k=V^*D^ky$, $N_k=V^*D^kU$, $0\le k<d$로 두면

\[
 x_k'=x_{k+1}+N_kF(x_0)\;(k<d-1),\qquad
 x_{d-1}'=-\sum_{k=0}^{d-1}a_kx_k+N_{d-1}F(x_0).         \tag{28}
\]

초기값은 $x_k(0)=V^*D^ky_0$다. 이 정확한 $rd$차원 closure는 algebra $\mathbb C[z]/(q_D)$에서 곱셈 by z의 companion representation이다. 증명은 각 $x_k$를 미분하고 $D^d=-\sum a_kD^k$를 대입하면 된다. Reconstruction (6)이 arbitrary initial state를 복원한다. 이 정리는 기존 Laguerre quotient compilation과 같은 **full-space polynomial identity**를 재사용하지만 새로운 nonlinear memory client를 제공한다.

한편 $n$개의 distinct diagonal eigenvalue가 모두 남아 있으면 일반적으로 minimal polynomial degree는 $n$이므로 이 정리만으로 축약은 없다. Matrix kernel의 작은 realization을 찾더라도 initial free profile $V^*e^{Dt}y_0$까지 같은 realization에 들어간다는 조건을 따로 확인해야 한다. Kernel에 나타나지 않는 homogeneous observable 정보를 버리지 않는다. Low-rank Hankel fit 또는 finite samples는 exact annihilator의 authority가 아니다.

Polynomial constraint $h(y)=0$와 algebraic observation π를 사용하는 더 일반적 모델에서는

\[
 Dh(y)f(y)=B(y)h(y),\qquad
 D\pi(y)f(y)-f_{\rm red}(\pi(y))\in\langle h_1,\ldots,h_q\rangle \tag{29}
\]

가 constraint invariance와 quotient dynamics의 충분조건이다. 첫 identity는 해를 따라 $h'=B(y)h$, $h(0)=0$의 유일성으로 증명된다. 둘째 identity는 constraint 위에서 chain rule을 준다. 이 identity는 offline polynomial arithmetic으로 검사할 수 있으며 온라인 Jacobian 호출은 요구하지 않는다. 하지만 Gröbner basis·구조 발견의 작은 최악 복잡도, singular quotient의 global smooth chart, 또는 원상 reconstruction은 별도 문제다.

기존 H4의 affine invariant-manifold exact same-tableau reduction과 (28)은 역할이 다르다. H4는 초기값이 지정 불변 manifold 위에 있어 full stage가 reduced stage로 lift되는 경우다. R1–R3은 arbitrary initial fibre를 유지하며 다른 derivative-free memory discretization을 쓴다. 따라서 이 packet을 기존 RVJ5와 동일 tableau라고 등록하면 안 된다. 고등 대수·미분위상 언어의 효용은 정확한 identity와 regular branch를 제공하는 데 있으며, 이 명칭이 물리 accuracy 또는 native speed evidence를 대신하지 않는다.

## 8. 코드 이식으로 넘길 최소 계약과 종료 상태

제안 descriptor는 `D/U/V/initial identity`, exact cluster assignment·centers·widths, $r,p_a$, physical norm, $F$의 직접 rank-r evaluator, verified Lipschitz/domain witness, moment coefficient enclosures, 원 초기 데이터, model epoch, basis/projection identity를 포함해야 한다. Moment를 임의로 바꾼 JSON은 trusted compiler authority가 아니다. 초기 moment와 kernel moment의 서로 다른 provenance도 보존한다.

후보 생성에서는 online Jacobian/JVP/VJP/finite-difference derivative/Krylov/factorization을 요구하지 않는다. 그러나 $F$ evaluator 또는 propagator 내부가 그 작업을 호출한다면 이 선언은 거짓이므로 actual dispatch ledger에서 검사해야 한다. 충분조건의 $L,G,B$는 모델식에서 offline 증명하거나 verified provider가 제공해야 한다. 그것을 매 sweep hidden full-dimensional probe로 구하면 비용을 부과한다.

Admission은 R3의 original-target residual·tail·rounding·reconstruction budget을 통과해야 한다. 실패는 현재 protected solver로 보내되 비용에 기록한다. Generic arbitrary callback에 이 경로를 자동 선택하지 않는다. 새 방정식 구조의 identity, transcript가 아닌 source-bound descriptor, 실제 native outward implementation, 전체 비용 benchmark가 제공되기 전까지 **native production/global trajectory/measured speed는 미승격**이다. 여기서 완결한 것은 R1–R6의 조건부 양의 정리와 작은 새 exact 검산이다.
