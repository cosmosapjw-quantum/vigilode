# Paper II에서 구면 격자 수치기법으로 이어지는 구성

2026-10-23. 입력: `ANGULAR_CALCULUS_PAPER_II_v13_OVERLEAF.zip`, SHA256 `3aa88e68135a6bbb24121d36e7dea4ff91895ab4fe287f300e63e9a6d2a7bd61`. Paper I의 Poisson 연산자와 관측량 정보 예산도 함께 확인했다. 아래에서 기존 초안의 정리는 재사용하며, 새 결과는 **유한 양의 구적 격자에서의 알고리즘 및 조건부 비용 보장**이다.

## 1. 전제와 이산화

물리적인 에너지는 $E=E_*x$, $E_*=k_{\rm B}T_*>0$, $z=mc^2/E_*$이다. $v=\sqrt{1-z^2/x^2}$이고 구면 적분은 $d\omega=d\Omega/(4\pi)$로 정규화한다. $Z_{\ell m}=\sqrt{4\pi}Y_{\ell m}$는 $d\omega$에 대해 정규직교한다. $c,k_{\rm B},\hbar$를 1로 둔다는 뜻이 아니다. 전체 측도 계수 $\mathcal C_*$는 number-density 단위를 가진다.

양의 구적점 $i=(r,q)$에서 $w_i>0$를 사용한다. 유한 에너지 구간과 구적 오차는 별도 근사로 기록한다. 자연 score 벡터는 $t_i=(s_j,x_rs_j)_j$, $s_j=v_r^{\ell_j}Z_j(\hat e_q)$, 자연변수는 $\theta=(a_j,-b_j)_j$이다. 그러면

\[
\psi_i=\theta\cdot t_i,\quad
f_i=h_\xi^{*\prime}(\psi_i),\quad
\mathcal A_h(\theta)=\sum_iw_i h_\xi^*(\psi_i),\quad
r=\nabla\mathcal A_h-m,\quad
H=\sum_iw_i W_i t_it_i^T,
\]

여기서 $W_i=f_i(1+\xi f_i)>0$. MB에서는 $f=e^\psi$, FD에서는 logistic $f=(1+e^{-\psi})^{-1}$, BE에서는 $\psi<0$이며 $f=(e^{-\psi}-1)^{-1}$이다. $H$는 **dual potential의 Hessian**이다. 일반적인 $H_{\rm kinetic}[f_\theta]$의 좌표 Hessian과 혼동하지 않는다.

구적이 $\mu=\cos\vartheta$에 대한 Gauss–Legendre $n_\mu\ge L+1$과 균일 방위각 $n_\phi\ge2L+1$의 곱이면, $\ell\le L$의 harmonic Gram 행렬은 정확하다. 방위각 합이 서로 다른 Fourier 주파수를 제거하고, 남은 같은 $m$의 associated-Legendre 곱은 차수 $\le2L$의 다항식이므로 GL 정확성이 적용된다. 방사 방향은 서로 다른 두 개 이상의 유효 에너지점이 있어야 두 score가 독립이다. $v=0$ apex 하나만으로 $\ell>0$를 구성할 수 없다.

## 2. Fisher whitening: 상관과 거대 rank별 스케일을 동시에 제거

등방 기준 $f_0(x)$에 대해 각 $(\ell,m)$의 $(\delta b,\delta\eta)$ 순서 block은

\[
G_\ell=\begin{pmatrix}M_{\ell,2}&-M_{\ell,1}\\-M_{\ell,1}&M_{\ell,0}\end{pmatrix},
\quad M_{\ell,k}=\sum_rw_r^{\rm rad}W_{0,r}v_r^{2\ell}x_r^k.
\]

방사 구적 가중치에 이미 $\mathcal C_*x^2v$가 포함되었다. $\mu_\ell=M_{\ell,1}/M_{\ell,0}$와
$\sigma_\ell^2=\sum_r\widehat w_{\ell,r}(x_r-\mu_\ell)^2>0$를 쓰면

\[
(\delta b,\delta\eta)G_\ell(\delta b,\delta\eta)^T
=M_{\ell,0}\{(\delta\eta-\mu_\ell\delta b)^2+\sigma_\ell^2\delta b^2\}.
\]

따라서 $q_0=\sqrt{M_{\ell,0}}(\delta\eta-\mu_\ell\delta b)$,
$q_1=\sqrt{M_{\ell,0}}\sigma_\ell\delta b$는 정확한 orthonormal 좌표이다. 이는 초안의 energy-variance determinant를 곧바로 전처리기로 바꾼다. 구현에서는 작은 차이 $M_2/M_0-(M_1/M_0)^2$ 대신 중심화된 양의 variance 합을 계산하고, $M_0$ underflow에는 log-scaled weight 또는 명시적인 rank 제외 판정을 쓴다. 작은 $M_0$를 임의의 epsilon으로 바꾸면 다른 metric을 풀게 된다.

### 정리 1: 각도 차원과 무관한 조건수 상한

동일 격자·score에서 $H_0=\sum_iw_iW_{0,i}t_it_i^T\succ0$라 하자. 모든 점에서 $0<a\le W_i/W_{0,i}\le b<\infty$이면

\[
aH_0\preceq H\preceq bH_0,\qquad
\kappa_2(H_0^{-1/2}HH_0^{-1/2})\le b/a.
\]

증명: 임의 $u$에 대해 $u^THu=\sum_iw_iW_i(t_i^Tu)^2$에 점별 상·하한을 적용한다. $u=H_0^{-1/2}y$를 대입하면 Rayleigh quotient가 $[a,b]$에 있다. 이 증명에는 작고 등방적인 실제 상태를 가정하지 않으며, 비교 기준의 등방성은 $H_0$를 독립적인 $2\times2$ block으로 싸게 적용하기 위해 사용한다.

MB와 FD에서 $|\psi_i-\psi_{0,i}|\le\delta$이면 $a=e^{-\delta},b=e^\delta$를 쓸 수 있다. MB는 $\log W=\psi$, FD는 $d\log W/d\psi=1-2f\in[-1,1]$이므로 평균값 정리로 따른다. BE에서는 두 점 사이의 자연변수 선분이 $\psi\le-\gamma<0$를 지킬 때 $|d\log W/d\psi|=\coth(-\psi/2)\le\coth(\gamma/2)$이므로 $a=e^{-\delta\coth(\gamma/2)},b=e^{\delta\coth(\gamma/2)}$가 된다. 실제 점별 ratio의 min/max가 대개 더 날카롭다.

이 보장은 유한 active grid에 대한 것이다. 무한 에너지 구간에서 비등방 inverse-temperature 변화는 $\Delta\psi\sim x\Delta b$를 만들므로, 단순한 유한 $\delta$ 증명을 그대로 적용할 수 없다. 전체 에너지 문제에는 tail majorant 또는 에너지별 전처리가 더 필요하다.

### 계산량에 대한 직접적인 귀결

$d=2(L+1)^2$, $N=N_EN_\Omega$일 때 dense Hessian 구성·factorization은 각각 $O(Nd^2)$, $O(d^3)$이다. matrix-free 곱
$Hu=T^T[wW\odot(Tu)]$는 일반 dense score 평가에서도 $O(Nd)$이며 $H_0^{-1}$ 적용은 $O(d)$이다. 전처리 CG는 에너지 norm 상대 오차가

\[
\frac{\|e_k\|_H}{\|e_0\|_H}\le2\left(\frac{\sqrt\kappa-1}{\sqrt\kappa+1}\right)^k,
\quad\kappa\le b/a
\]

를 만족한다. 따라서 고정 ratio bound 아래 inner iteration 수의 상한은 $L$에 의존하지 않는다. 이 결과는 **entropy moment inversion**의 비용 감소 보장이다. ODE 전체나 streaming/collision Jacobian의 조건수가 자동으로 개선된다는 주장은 이 정리에서 나오지 않는다. 실제 speedup에는 transform, quadrature, memory 비용을 포함한 측정이 필요하다.

실제 오차를 기준으로 종료하려면 $Hx_*=c$와 현재 residual $r=c-Hx$에 대해
\[
\frac{\|x-x_*\|_H}{\|x_*\|_H}
\le\sqrt{b/a}\,\frac{\|r\|_{H_0^{-1}}}{\|c\|_{H_0^{-1}}}
\]
를 쓸 수 있다. 이는 $b^{-1}H_0^{-1}\preceq H^{-1}\preceq a^{-1}H_0^{-1}$를 분자·분모에 적용한 결과이며 block inverse만으로 계산된다. 현재 작은 CG/PCG 비교는 동일한 자연좌표 Euclidean residual을 사용하므로, 해당 숫자를 곧바로 이 energy-norm 종료 보장과 동일시하지 않는다.

### Newton-free 변형의 조건부 수렴

$\widetilde F(z)=\mathcal A_h(\theta_0+H_0^{-1/2}z)-m^T(\theta_0+H_0^{-1/2}z)$를 쓰자. 해 $z_*$를 중심으로 하는 닫힌 구 $B_R(z_*)$가 admissible domain 안에 있고, 그 구에서 $aI\preceq\nabla^2\widetilde F\preceq bI$라 하자. 그러면

\[
z_{k+1}=z_k-\frac2{a+b}\nabla\widetilde F(z_k)
\]

는 $q=(b-a)/(b+a)<1$로 수축한다. 실제로 $z_k$와 $z_*$ 사이에서 Hessian을 적분하면 오차의 선형 변환 고유값이 $[-q,q]$에 있다. 따라서 구가 보존되고 $\|z_k-z_*\|\le q^k\|z_0-z_*\|$이다. 각 반복은 moment residual과 block inverse만 필요하며 전체 Hessian, Newton factorization, Krylov basis가 필요 없다. 시작점이 이 구 안에 있다는 조건과 구 전체의 susceptibility ratio certificate는 별도의 수용 계약이다. damping/line search만으로 이 특정 반복 횟수 보장을 대체하지 않는다.

## 3. 보존량을 지키는 entropy repair

각도 remap 또는 작은 시간 스텝이 만든 양의 prior $g_i$에 대해 정확히 유지할 moment를 $Af=m$이라 하자. 유한 positive grid에서

\[
\min_{f\in\mathcal D_\xi}\sum_iw_i d_{h_\xi}(f_i\Vert g_i)
\quad\text{s.t.}\quad\sum_iw_it_if_i=m
\]

를 푼다. feasible interior와 충분한 constraint rank를 가정하면 strict convexity로 해가 유일하다. Lagrange stationarity는

\[
h_\xi'(f_i)=h_\xi'(g_i)+\lambda\cdot t_i
\]

를 준다. MB는 multiplicative repair $f_i=g_i e^{\lambda\cdot t_i}$, FD는 logit shift, BE는 음의 entropy score를 유지하는 shift이다. 이 표현은 positivity/FD upper bound를 구조적으로 보존한다. BE는 line search에서 모든 score $<0$를 검사한다. 대상 moment가 격자의 실현 가능 영역 바깥이면 clipping으로 성공 처리하지 않고 정확한 실패를 반환해야 한다.

제약이 전역 number·energy 두 개이면 매 반복은 $O(N)$의 residual와 $2\times2$ susceptibility 계산이다. 유한 에너지 band에서 두 양의 에너지 bin에 가중치를 옮기는 임의의 선형 repair보다 relative entropy 최적성을 갖는다. 다만 prior가 음수인 경우에는 $h'(g)$ 자체가 정의되지 않으므로 이 방법의 입력을 만족하지 않는다. 고차 적분기의 음수 trial을 직접 덮어써서 통과시키는 대신 step rejection/positivity-preserving predictor 뒤에 적용한다.

정확한 constraint 해 $f_*$와 임의 feasible $f$에 대해

\[
D_H(f\Vert g)=D_H(f\Vert f_*)+D_H(f_*\Vert g)
\]

이다. 세 점 항은 $\lambda\cdot A(f-f_*)=0$으로 소거된다. 이 등식은 repair가 moment fiber 위에서 최소 추가 entropy distance를 갖는다는 정확한 certificate이다. 다른 stage의 physical conservation을 자동 보장하지는 않는다.

## 4. 에너지 shell별 각도 해상도 줄이기

Paper I의 $\mathcal P_vZ_{\ell m}=v^\ell Z_{\ell m}$는 양의 Markov 평균이다. 이를 entropy preconditioner나 massive dressed score 평가에 그대로 사용하면, 느린 에너지 shell은 낮은 각도 cutoff로 계산할 수 있다. $A\in L^2(d\omega)$와 $v<1$에 대해

\[
\|\mathcal P_vA-\Pi_L\mathcal P_vA\|_2
\le v^{L+1}\|A\|_2.
\]

Parseval에서 각 omitted eigenvalue를 $v^{L+1}$로 상계하면 증명된다. 따라서 주어진 $K\ge\|A\|_2$와 오차 $\varepsilon$에 대해

\[
L+1\ge\frac{\log(K/\varepsilon)}{-\log v}
\]

는 $0<v<1$에서 충분조건이다($K\le\varepsilon$이면 monopole만으로 충분할 수 있다). $v=0$에서는 Poisson 연산자가 정확히 monopole projection이므로 로그 공식을 호출하지 않는다. $v=1$에서는 이 rank suppression이 사라진다. massless photon에 massive particle의 개선율을 대입하지 않는다.

positivity를 위해 필요한 균등 오차도 계산할 수 있다. $s=v^2$, $n=L+1$이면 addition theorem와 Cauchy–Schwarz로

\[
\|\mathcal P_vA-\Pi_L\mathcal P_vA\|_\infty
\le\|A_{>L}\|_2\,s^{n/2}
\sqrt{\frac{2n+1}{1-s}+\frac{2s}{(1-s)^2}}.
\]

여기서 $\sum_{\ell=n}^\infty(2\ell+1)s^\ell=s^n[(2n+1)/(1-s)+2s/(1-s)^2]$를 사용했다. 이를 thermal field $B$의 positive margin보다 작게 요구하면 truncated shell field의 양성을 보증한다. 즉 coefficient를 작다는 이유만으로 삭제하지 않고 같은 전제에서 실제 cutoff를 선택할 수 있다.

기존 uniform cutoff $L_{\max}$의 평가 항 수가 $N_E(L_{\max}+1)^2$이면 shell별 cutoff의 항 수는 $\sum_r(L_r+1)^2$이다. 위 조건으로 선택한 $L_r$에 대해서는 지정한 field 오차를 지키면서 이 항 수 감소가 보장된다. 이것은 score/basis evaluation count이며 전체 PDE wall time 비율은 아니다. fine tail norm을 얻는 비용은 update 시점의 별도 비용으로 포함한다.

초안 II App. F의 finite-cutoff entropy certificate를 사용하면 이 field bound를 number-density 단위의 $D_H$ 상한으로 바꿀 수 있다. 그 정리는 MB/FD, 동일한 dressed moment map, 자연변수 선분의 $B\ge b_*>0$, $A\le A_*$와 알려진 omitted tail을 요구한다. 고정 $z>0$의 $\exp[-c\ell^{1/3}]$ asymptotic만으로 유한 cutoff의 오차를 인증하지 않는다. 실제 finite kernel bound를 계산한다.

## 5. Laguerre와 미해결 score의 사용 위치

massless MB 기준 $f_0=e^{-x}$에서는 radial score measure가 $x^2e^{-x}dx$이다. 따라서

\[
P_n(x)=(-1)^n\sqrt{\frac{n!}{(n+2)!}}L_n^{(2)}(x)
\]

는 orthonormal이며 $P_0=1/\sqrt2$, $P_1=(x-3)/\sqrt6$이다. thermochemical 두 score는 정확히 이 첫 두 polynomial이 이루는 span이다. 고차 $P_n$은 unresolved rate response를 평가하거나 추가 closure의 후보를 선택하는 analysis basis로 쓸 수 있다. $n\ge2$ polynomial을 그대로 무한 에너지 natural exponent에 추가하면 양의 최고차 계수가 tail integrability를 깨뜨릴 수 있으므로, tangent analysis와 실제 positive spectral family 확대는 별개로 구현한다. massive/quantum reference에서는 초안 App. C의 양의 weight에 대한 Stieltjes recurrence를 사용한다.

이 방법은 polynomial iteration의 이름만 바꾼 solver가 아니다. positive equilibrium tangent metric에 맞춰 radial/angle 좌표를 정규화하고, 실제 response와 residual의 크기를 계산하는 것이다.

### 작은 실제 검산

`src/spectral_tools_demo.py`를 실행했다. $\alpha=2$ Gauss–Laguerre 16점으로 첫 8개의 $P_n$ Gram 행렬을 계산한 결과 identity와의 최대 절대 차이는 $2.45\times10^{-15}$ 미만이었다. $P_0,P_1$의 닫힌 식도 확인했다. 이는 가중 다항식 적분의 작은 구현 검산이다.

완전히 알려진 $L_{\max}=24$의 유한 coefficient field에 대해 $v=(0,0.3,0.7,0.9,0.95,1)$, $L^2$ 오차 예산 $0.02$를 사용했다. 각 shell의 retained basis 수는 $(1,16,121,625,625,625)$였으며, 총 $2013/3750=0.5368$의 basis 항만 유지했다. 모든 실제 coefficient error가 유도한 상한과 오차 예산 아래였고 thermal field의 양성 margin도 유지했다. 이 **46.32%의 항 수 감소**는 알려진 유한 field와 이 여섯 shell에 대한 결과이다. $v=1$ shell은 전체 625항을 유지했으며, PDE 실행 시간은 측정하지 않았다. 입력 coefficient와 각 shell의 모든 오차·양성 bound는 `results/spectral_tools_results.json`에 보존했다.

## 6. 범위와 근거 상태

* **재사용한 초안:** number–energy inverse, Fisher variance block, nested Bregman equality, Poisson factorization, finite-cutoff bound. 여기서는 전면 재증명하지 않고 유한 격자 계약으로 필요한 부분을 옮겼다.
* **직접 유도:** susceptibility 비교에 따른 dimension-independent preconditioned condition bound, centered-score whitening, Newton-free local contraction, relative-entropy conservative repair, shellwise $L^2/L^\infty$ cutoff 계산과 항 수 감소.
* **구현 검증:** `src/fisher_demo.py`, `results/fisher_results.json`의 13개 finite-grid case를 독립적으로 소스·결과 검토했다. susceptibility ratio의 고유값 enclosure, 기준 whitening, 실제 자연좌표 residual 조건이 일치한다. `analysis/PAPER_II_CODE_REVIEW.json`에 검토 파일 해시를 기록했다. 실험은 assembled dense Hessian을 이용한 CG/PCG 비교이며, 위의 matrix-free 복잡도는 별도의 유도 결과이다. 실험 코드는 $d\Omega$-정규직교 $Y$를 사용하므로 이 문서의 $d\omega$/$Z$와 같은 Gram 항등식을 얻는다.
* **문헌 관계:** entropy Hessian을 Cholesky로 whitening하는 알고리즘 자체는 Alldredge, Hauck, O'Leary, Tits, *J. Comput. Phys.* 258 (2014) 489–508, DOI `10.1016/j.jcp.2013.10.049`, arXiv `1306.2881`에서 이미 개발되었다. 이번 연결은 초안의 해석적 $2\times2$ thermal–chemical block과 susceptibility-ratio certificate를 구면 discretization에 적용하는 것이다.

## 7. 구현 권장 계약

`build_isotropic_metric(radial_nodes, radial_weights, lmax, statistics, z)`는 각 rank의 log-weight, mean, positive centered variance를 반환한다. `whiten_scores`는 원래 moment와 multiplier의 dual transformation을 함께 보존한다. `susceptibility_ratio_certificate`는 실제 finite node의 min/max, theory bound, domain margin을 반환한다. `entropy_repair`는 prior domain, target realizability, constraint rank를 확인하고 residual·duality gap·positivity margin을 보고한다. `select_shell_lmax`는 tail norm의 출처와 desired field/entropy error를 입력으로 받는다. 이 함수들은 생산 solver에 들어가기 전에 reference projection, exact conservation, harmonic normalization, FD/BE boundary rejection을 확인해야 한다.
