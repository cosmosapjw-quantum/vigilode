# Paper I에서 구면 격자 수치기법으로 이어지는 구성적 결과

입력은 `ANGULAR_CALCULUS_PAPER_I_v13_OVERLEAF.zip` (SHA-256 `25c67a4f0fcc3a44eee9d8bd8fad17aa38e01d84d4967182baca160bc27b5585`)이다. 이 문서는 드래프트의 정리와 이번에 유도한 수치해석 명제를 구분한다. 드래프트 전체의 신규성이나 모든 정리를 재심사한 결과는 아니다.

## 1. 재사용할 구조와 적용 영역

Paper I의 `appendices/A_radial_proofs.tex`, `eq:rho-beta-chain`, `eq:angular-power-functional`은 질량 없는 one-field 열적 section에서

\[
\rho=3a_\xi C\beta^{-4},\qquad F(\rho)=-\kappa_\xi\rho^{3/4},\qquad
\kappa_\xi=\frac{4(a_\xi C)^{1/4}}{3^{3/4}}
\]

를 준다. \(\rho\)는 드래프트의 무차원 방향 에너지이고, 물리 에너지는 \(k_BT_{\rm ref}\rho\), 물리 엔트로피는 무차원 정보량에 \(k_B\)를 곱하여 얻는다. \(F''(\rho)=3\kappa_\xi\rho^{-5/4}/16>0\)이다. 동일한 고정 관측자와 동일한 정규화를 유지한다.

`eq:divergence-preserving-lift`는 선택된 radial lift 사이의 kinetic Bregman divergence와 이 \(F\)의 angular divergence가 같음을 증명한다. `eq:exact-two-axis-budget`는 spectral 정보 손실과 angular 정보 손실을 분리한다. 따라서 아래의 angular 계산은 **이미 선택한 열적 에너지 section의 공간 해상도 제어**에 직접 적용된다. 비열적 \(f(E,\hat e)\)의 radial closure 오차는 별도의 항이다.

드래프트의 normalized KL이 아닌 kinetic Bregman Hessian을 사용한다. `eq:kinetic-fisher-metric`은 \(W=f(1+\xi f)\)를, `eq:angular-bregman-local-expansion`은 \(\beta\) 좌표의 질량 없는 metric을 준다. `sec:entropy-budget-observable`의 정보 반경은 선언된 추가 입력이다. 보유한 에너지 모멘트만으로 미해상도 정보 반경을 추정하는 단계는 허용되지 않는다.

## 2. 보존형 셀 평균의 정확한 정보 손실

구면 위의 양의 quadrature 값 \(\rho_i>0\), 가중치 \(w_i>0\)와 셀 partition \(\mathcal P\)를 고정한다. \(w_i\)는 \(d\Omega\) 또는 \(d\omega=d\Omega/(4\pi)\) 중 하나에 대한 것이며, 모든 식에서 한 convention을 일관되게 사용한다. 셀 \(C\)에 대해

\[
W_C=\sum_{i\in C}w_i,\quad \bar\rho_C=W_C^{-1}\sum_{i\in C}w_i\rho_i,
\quad \eta_C=\sum_{i\in C}w_i F(\rho_i)-W_CF(\bar\rho_C)
\]

로 둔다. 셀 평균은 양의 상태와 셀 에너지를 정확히 보존한다. \(\sum_iw_i(\rho_i-\bar\rho_C)=0\)이므로

\[
\eta_C=\sum_{i\in C}w_iD_F(\rho_i\Vert\bar\rho_C)\ge0.
\tag{T1}
\]

이것은 주어진 이산 데이터에 대한 정확한 등식이다. 임의의 coarse cell을 child \(C_j\)로 나누면

\[
\eta_C=\sum_j\eta_{C_j}+\Delta_C,\qquad
\Delta_C=\sum_jW_{C_j}F(\bar\rho_{C_j})-W_CF(\bar\rho_C)\ge0.
\tag{T2}
\]

따라서 refinement의 실제 정보 이득 \(\Delta_C\)를 선형 시간에 계산할 수 있다. child 수를 \(r\)라 하면 \(\Delta_C/(r-1)\)를 추가 자유도당 정보 이득으로 사용할 수 있다. 이 점수의 greedy 사용이 전역 최적 partition을 보장한다는 주장은 하지 않는다.

**증명.** 셀 평균의 선형 항 상쇄와 convexity로 T1을 얻는다. \(\sum_jW_{C_j}F(\bar\rho_{C_j})\)를 더하고 빼면 T2의 등식이 나온다. child 평균의 가중 평균이 parent 평균이므로 Jensen 부등식이 \(\Delta_C\ge0\)을 준다.

이 construction은 드래프트의 finite harmonic max-entropy projection과 서로 다른 projection이다. 닮은 Bregman telescope는 직접 위와 같이 유도한 것이며, 격자 셀 평균을 드래프트의 \(\beta\)-harmonic projection이라고 부르지 않는다.

## 3. 관측량별 유한 오차 보증

\(a_C\le\rho_i\le b_C\)라 두고

\[
m_C=\frac{3\kappa_\xi}{16}b_C^{-5/4},\qquad
M_C=\frac{3\kappa_\xi}{16}a_C^{-5/4}
\]

라 하자. Taylor의 적분 나머지로

\[
\frac{m_C}{2}\sum_{i\in C}w_i(\rho_i-\bar\rho_C)^2
\le\eta_C\le
\frac{M_C}{2}\sum_{i\in C}w_i(\rho_i-\bar\rho_C)^2.
\tag{T3}
\]

목표 선형 관측량 \(O_K=\sum_iw_iK_i\rho_i\)와 셀 평균 대체 후 관측량 \(\bar O_K=\sum_C\bar\rho_C\sum_{i\in C}w_iK_i\)를 비교한다. \(\bar K_C=W_C^{-1}\sum_Cw_iK_i\), \(V_{K,C}=\sum_Cw_i(K_i-\bar K_C)^2\)라 하면

\[
|O_K-\bar O_K|\le
\sum_C\sqrt{\frac{2\eta_C V_{K,C}}{m_C}}.
\tag{T4}
\]

**증명.** 셀별 오차에서 \(\bar K_C\sum_iw_i(\rho_i-\bar\rho_C)=0\)을 빼고, weighted Cauchy–Schwarz와 T3의 왼쪽 부등식을 순서대로 사용한다. 이것은 small-radius 근사가 아닌 유한 이산 정보 손실의 bound이다. \(K\)가 각 셀에서 상수이면 해당 관측량 오차는 정확히 0이다.

T4의 각 셀 항은 energy flux, quadrupole, 특정 angular detector response마다 다르다. 다수의 목표 \(K^{(q)}\)가 있으면 각각의 합을 계산하고 정해진 \(\epsilon_q\)와 비교한다. 가장 큰 상태 gradient만 보고 분할하는 것보다 실제 관측량 허용오차를 판정에 직접 연결한다. 비교할 baseline과 방법이 같은 fine-grid data 및 같은 \(\epsilon_q\)를 사용해야 한다.

이 보증은 **fine-grid reference에 대한 보증**이다. 미샘플 영역에서 놓친 구조까지 포함하는 continuum 또는 exact-PDE 해의 보증에는 추가 regularity/잔차 추정이 필요하다. 예를 들어 continuum에서 geodesic diameter \(h_C\), \(\|\nabla_{S^2}\rho\|_{L^\infty(C)}\le G_C\), \(0<a_C\le\rho\le b_C\)가 독립적으로 알려지면

\[
\eta_C\le\frac{M_C}{2}|C|h_C^2G_C^2
\]

가 따라온다. 이를 T4와 continuum kernel variance에 대입할 수 있다. 단순한 pilot sample의 최대 기울기를 엄밀한 \(G_C\)로 바꾸어 쓰면 안 된다.

## 4. 정보 metric이 예측하는 격자 자유도 절감

\(\rho\in C^2(S^2)\), 양의 uniform lower bound, 작은 shape-regular approximately isotropic 셀을 가정한다. 셀 공분산이 동일한 형상 상수에 수렴하는 범위에서 leading entropy loss는

\[
I\simeq c_{\rm shape}\int_{S^2}A(x)a(x)\,d\Omega,
\quad A(x)=F''(\rho(x))|\nabla_{S^2}\rho(x)|^2,
\quad N\simeq\int_{S^2}\frac{d\Omega}{a(x)}
\]

이다. 여기서 \(a(x)\)는 local cell area이다. Cauchy–Schwarz로

\[
\left(\int\sqrt A\,d\Omega\right)^2
\le\left(\int Aa\,d\Omega\right)\left(\int a^{-1}\,d\Omega\right).
\]

등호를 주는 선택은 \(a(x)\propto A(x)^{-1/2}\), 즉 \(h(x)\propto A(x)^{-1/4}\)이다. 동일 leading-order 정보 오차 \(\epsilon\)에서

\[
N_{\rm adapt}\simeq
\frac{c_{\rm shape}}\epsilon\left(\int\sqrt A\,d\Omega\right)^2,
\quad
N_{\rm uniform}\simeq
\frac{c_{\rm shape}|S^2|}\epsilon\int A\,d\Omega,
\tag{T5}
\]

이므로 \(N_{\rm adapt}\le N_{\rm uniform}\)이다. \(A\)가 거의 상수면 이득이 없다. \(A\)가 면적 비율 \(f\)인 영역에 거의 일정하게 집중되면 leading ratio는 \(N_{\rm adapt}/N_{\rm uniform}\simeq f\)이다. 실제 mesh에서 최대/최소 셀 크기, 곡률, refinement balance 및 remap 비용이 이 이상화된 이득을 줄인다. \(A=0\)인 곳에는 최대 셀 크기를 부여해야 한다. 전체 시간적분기 wall-clock speedup은 T5만으로 정해지지 않으며 RHS와 grid-update 측정이 필요하다.

T5의 기여는 잘 정의된 해상도 모델 아래 **필요 자유도의 절감**이다. 메트릭을 재명명한 설명으로 끝내지 않고, 실제 refinement density를 제시한다. 적응형 선택의 효율을 검증할 때 원자료 전체를 훑는 비용도 포함해야 한다. fine reference를 매 RHS마다 모두 계산하면 축약 상태 수만 줄고 비용은 줄지 않을 수 있다.

## 5. Fisher 좌표와 오차 metric

scalar energy section의 Fisher line element는

\[
ds^2=F''(\rho)d\rho^2
=\frac{3\kappa_\xi}{16}\rho^{-5/4}d\rho^2=dz^2,
\quad z=\frac{2\sqrt{3\kappa_\xi}}{3}\rho^{3/8}.
\tag{T6}
\]

그러므로 interpolation/error representation을 \(z\)에서 수행하면 이 scalar local metric의 강한 amplitude 의존성이 사라진다. 다만 두 상태 사이의 **유한** Bregman divergence가 \((z_1-z_2)^2/2\)와 같다는 주장은 틀리다. 또한 \(\rho\)를 cell-average conservative variable로 보유하고 \(z\)를 reconstruction 또는 error-weight 계산에 쓰는 방식이 직접적이다. \(z\) 자체를 무조건 평균하면 energy conservation이 바뀐다.

전체 occupation의 scalar metric \(df^2/[f(1+\xi f)]\)도

\[
z_0(f)=2\sqrt f,\qquad
z_{+1}(f)=2\operatorname{arsinh}\sqrt f,\qquad
z_{-1}(f)=2\arcsin\sqrt f
\]

로 flatten된다. 이것은 occupation에 대한 local entropy geometry이다. ODE 좌표 변환 자체가 stiff eigenvalue 또는 nonnormal amplification을 제거한다는 결론은 나오지 않는다. positivity-aware embedded-error metric과 observer response weighting의 구현 후보로 사용한다.

## 6. 에너지와 입자수를 함께 보존하는 수정

one-field energy 평균은 일반적으로 \(n\propto\rho^{3/4}\)인 입자수를 보존하지 않는다. 따라서 실제 photon number conservation이 필요한 client에는 다음 두 경로를 구분한다.

이 변화의 부호와 크기도 계산할 수 있다. \(n=A_\xi\rho^{3/4}\)이면 concavity로 \(\Delta N_C=A_\xi[W_C\bar\rho_C^{3/4}-\sum_iw_i\rho_i^{3/4}]=(A_\xi/\kappa_\xi)\eta_C\ge0\)이다. zero-log-fugacity massless MB에서는 \(H=-4n\)이므로 특히 \(\Delta N_C=\eta_C/4\)이다. 즉 같은 energy를 유지한 채 coarse cell을 one-field thermal representative로 다시 만드는 과정이 입자수를 늘린다. 이 정확한 식은 joint 또는 full-occupation 구현이 필요한 경우를 판별하는 보존량 regression이다.

1. **비열적 전체 분포:** 각 energy bin에서 \(\bar f_{kC}=\sum_iw_if_{ki}/W_C\)로 평균한다. 그러면 각 bin의 population 및 모든 bin-wise linear energy weight가 보존된다. 양의 가중 평균은 MB/BE positivity와 FD \([0,1]\) 범위를 보존한다. \(\eta_C=\sum_k\nu_k[\sum_iw_ih_\xi(f_{ki})-W_Ch_\xi(\bar f_{kC})]\)의 Jensen telescope가 성립한다. \(\nu_k\)는 radial state-counting quadrature weight이다.
2. **질량 없는 MB thermochemical section:** 두 moment \((n,\rho)\)를 함께 보존한다. 이 section은 다음 닫힌식을 준다.

\[
n=2Ce^\alpha\beta^{-3},\quad\rho=6Ce^\alpha\beta^{-4},
\quad\beta=3n/\rho,
\quad\alpha=\ln\frac{27n^4}{2C\rho^3}.
\]

\(f=e^{\alpha-\beta u}\)를 MB entropy에 대입하면

\[
F_{NE}(n,\rho)=n\ln\frac{27n^4}{2C\rho^3}-4n,
\quad
\nabla F_{NE}=(\alpha,-\beta),
\quad
\nabla^2F_{NE}=
\begin{pmatrix}4/n&-3/\rho\\-3/\rho&3n/\rho^2\end{pmatrix}.
\tag{T7}
\]

Hessian determinant는 \(3/\rho^2>0\)이다. 따라서 \((\bar n_C,\bar\rho_C)\)의 weighted mean은 두 moment를 보존하고, \(F_{NE}\)에 대한 T1–T2와 동일한 Bregman telescope를 준다. dimensionless temperature \(\tau=\rho/(3n)\)를 사용하면

\[
ds^2=dn^2/n+3n(d\ln\tau)^2.
\tag{T8}
\]

**증명.** \(\delta\rho/\rho=\delta n/n+\delta\ln\tau\)를 T7의 quadratic form에 대입하면 cross term이 상쇄된다. 이는 number–temperature 좌표의 Fisher preconditioner를 diagonal로 만드는 정확한 식이다. thermochemical MB closure에서 유도했으며, BE/FD에 그대로 상수를 복사해서는 안 된다. 일반 통계에서는 Paper II의 radial susceptibility matrix를 사용해야 한다.

## 7. 구현상 핵심

셀에서 \(r=\rho/\bar\rho_C-1\)라 두면

\[
D_F(\rho\Vert\bar\rho_C)=\kappa_\xi\bar\rho_C^{3/4}
\left[1+\frac34r-(1+r)^{3/4}\right].
\]

작은 \(r\)에서는 `-expm1(0.75*log1p(r))+0.75*r`를 쓸 수 있고, 더 가까운 경우에는

\[
\frac{3}{32}r^2-\frac5{128}r^3+\frac{45}{2048}r^4-\frac{117}{8192}r^5+O(r^6)
\]

를 사용한다. 독립 검산 없이 작은 negative information loss를 무조건 0으로 clamp하면 잘못된 aggregate나 weight를 숨길 수 있다. 정보 손실·kernel variance·population sums를 저장하고 수치 허용오차와 실제 음수 실패를 구분한다.

현 prototype의 exact-discrete bound는 nonlinear remap 이후에도 **계산한 coarse field와 reference 사이**에만 적용된다. 장시간 PDE trajectory에 적용하려면 transport의 안정성 추정과 시간적분 오차, remap 누적을 결합한다. 그 단계에서 단일 remap budget을 전체 허용오차와 동일시하지 않는다.

## 8. 즉시 이식 순서

첫 구현은 보존변수 \(f_{ki}\) 또는 joint \((n_i,\rho_i)\), 양의 angular weights, cell topology, 실제 target kernels를 입력으로 받는 별도 refinement controller이다. accepted-step 시점에만 topology를 바꾸고, fine pilot/independent residual의 정보 손실과 T4형 target bound를 계산한다. Fisher 좌표는 reconstruction/error-weight 보조에 사용한다. 현재 scalar \(\rho\) prototype은 formula와 adaptive selection 검증용이다. 이 절차를 실제 radiative client의 에너지·입자수·polarization 제약으로 확장한 뒤 wall-clock 평가를 한다.

새 이론 주장 T1–T4, T6–T8은 위 가정하에서 derived이다. T5는 명시한 smooth shape-regular local-grid asymptotic model에서 derived이며 일반 비정형 격자의 비점근 복잡도 정리가 아니다. parent의 독립 코드 실험과 결과를 별도로 연결한다.
