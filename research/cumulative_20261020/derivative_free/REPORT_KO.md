# VigilODE: JVP를 요구하지 않는 causal 격자·packet·adaptive 연구

기준일: 2026-10-20 KST. 작업 종류: 기존 연구를 재사용한 발산적 구성과 대표 예제의 직접 유도·경량 검산. 생산 Rust 변경·새 benchmark 속도 승격·GitHub mutation은 수행하지 않았다.

## 1. 이번 연구의 판단

이번 요구는 단순히 SP02의 JVP callback을 다른 이름으로 감싸는 것이 아니다. 제안하는 중심 구조는 **생성형 보존 flux 모델 → 국소 반응의 대수적 소거/물리적 선형 전파 → 저차원 비선형 memory 또는 interface waveform → 연속 defect 인증 → causal 시간·격자 transaction**이다. 기존 supplied-JVP RODAS5P는 별도의 보호 경로로 유지하되, 그 정확한 stage target을 미분정보 없이 그대로 실행한다고 주장하지 않는다.

실제 계산으로 세 긍정적 연결을 확인했다. 첫째, 알려진 물리적 band stencil과 정확히 저랭크인 비선형 피드백은 모든 초기 공간모드를 보존하면서 비선형 계산을 scalar로 축소할 수 있었다. 둘째, 보존식으로 국소 반응의 비선형 방정식을 quadratic으로 소거해 JVP/Newton 없이 물리적 root를 선택할 수 있었다. 셋째, 다항식 합성·Bernstein 계수·두 점 dissipativity만으로 한 scalar 문제의 adaptive trajectory와 모든 연속 출력의 오차를 연결할 수 있었고, 실제 반환 binary64의 오차도 포함했다.

동시에 다음 주장은 채택하지 않았다. 일반 물질 흡수항이 항상 저랭크라는 주장, 근사 Schur로 반복이 언제나 3~5회라는 주장, limiter의 계수를 동결해도 원래 고차 ODE를 푼다는 주장, 보존형 remap이면 즉시 5차가 회복된다는 주장, 한 배열의 capacity 확보로 전체 가변격자 경로가 zero-allocation이라는 주장이다. 각 판단은 아래 식이나 실행 대조로 구분한다.

## 2. 원래 연구에서 실제로 가져온 것

주 기준 source는 `c03ca90f19a9e64ba7751c069325a48b54b39c2e`이다. PR70의 metadata head는 `826fa05cc1fdaf7b1a0cf7ac45116596b4ddff65`로 별도 확인했다. PR75는 `6c945b6c6e0e024d8f733c21da8d35fbcc3c5f2a`의 draft다. 새로운 이론을 기존 PR에서 이미 실행한 것처럼 표시하지 않는다.

| 기존 source | 재사용하는 실질적 내용 | 이번 확장 |
|---|---|---|
| `docs/reviews/20261011_promotion_theory/REPORT_KO.md` | 몫대수·소멸다항식, finite jet, stage/시간 오차 전달, 조건부 최적 residual 배분 | 물리적 stencil의 작용대수와 국소 constitutive graph에 적용 |
| `docs/reviews/20261011_loop12_upgrade/{REPORT_KO,RANK_GEOMETRY_KO}.md` | 모든 초기모드 보존, exact rank-r Volterra 축약, causal factorial 수렴, 감쇠 분리 | 대각 D에서 알려진 band L 및 물질 waveform에 조건부인 propagator로 확장 |
| `CERTIFICATE_ATLAS_KO.md` | parameter cell·Bernstein/SOS identity, 정확한 model binding, 검증 비용 amortization | runtime JVP 대신 두 상태의 차이 부등식·flux positivity·limiter branch 인증 |
| PR70 및 chart 연구 | 물리좌표 복원·공통 tube·cofactor forcing, 일반 RVJ 전이의 실제 실패 보존 | 국소 반응 invariant ideal과 regular chart atlas, 원래 좌표 defect로 재검증 |
| `packet_window.rs`, `packet_trajectory.rs` | causal waveform, 상속 반경, rejected trial의 원자적 rollback | 수송·반응·remap까지 동일한 반경/epoch transaction으로 연결 |
| `bound_pr101.rs` | 봉인된 model/provider, 연속 residual, one-sided gain, 실제 반환 중심의 오차 | 생성형 FVM의 국소 대수적 proof provider 설계 |
| PR75 | JVP count만으로 작은 projected solve 비용을 예측하지 못한 반례 | 총 준비·후보·검증·fallback·remap 비용을 포함한 선택 목적함수 |

현재 packet 경로의 `roundoff_enclosed=false`와 production-audit HOLD는 유지한다. 기존 code에 adaptive window가 있다는 사실을 무시해 새 기능처럼 다시 선언하지 않는다. 아래 scalar proof-of-concept의 인증은 현재 일반 packet/FVM 구현의 권한을 올리는 근거가 아니다. 외부 Exa/SciSpace 검색은 인용된 CVODE/AMR 주장의 범위를 확인하는 보조 수단으로만 사용했다.

## 3. 무엇을 Jacobian-free라고 부를 것인가

세 계약을 구분한다.

1. **Assembly-free**: 전체 Jacobian을 저장하지 않지만 supplied JVP를 사용한다. 현재 SP02/기존 coupled의 계약이다.
2. **Derivative-approximation based**: FD-Jv, complex-step, AD, 학습한 tangent를 쓴다. 저장공간은 줄일 수 있어도 이번 강한 요구의 완성이 아니다.
3. **Runtime nonlinear-derivative-oracle-free**: 후보와 검증에서 상태 Jacobian·JVP·FD/AD를 요청하지 않는다. 대신 직접 제공된 물리적 flux 계수, 반응식, 보존식, 저차 polynomial/rational graph와 그 domain 증명을 사용한다.

이 연구의 새 후보는 3번이다. 알려진 선형 수송 연산자를 적용하거나 banded linear balance를 푸는 것까지 금지한다는 뜻은 아니다. 선형부분의 물리 연산자가 수학적으로 그 부분의 derivative와 같을 수 있어도, 임의의 비선형 RHS를 미분해 그 행렬을 얻는 알고리즘과는 구분한다. 모델 구조를 얻는 비용과 가정을 숨기지 않는다. 일반 black-box RHS의 유한한 샘플만으로 whole-domain 증명이나 낮은 rank를 선언하지 않는다.

기존 RODAS5P target의 W는 J를 포함한다. J를 0 또는 frozen coefficient로 대체하면 별도 방법/target이다. 그 경로를 구성할 수는 있지만 기존 차수·안정성·인증을 자동 상속하지 않는다. 여기서는 RHS/flux로 정의된 integral target과 생성형 모델을 별도 API로 제안한다.

## 4. 후보 A: banded physical propagator와 낮은 비선형 memory

고정 mesh G의 cell-integrated 상태 q, 작은 물질 상태 z에 대해

\[
\dot q=Lq+UF(V^Tq,z)+b(t),\qquad \dot z=G(z,V^Tq)
\]

가 정확한 모델 식이라 하자. L은 공간 수송·선형 산란에서 직접 주어지는 연산자다. 적절히 물리 단위를 정규화한 norm을 쓰며 L의 단위는 시간 역수, 지수함수의 L(t-s)는 무차원이다. U,V에는 필요한 변환 단위가 포함된다.

m=V^Tq와 \(\Phi(t,s)=e^{(t-s)L}\)를 쓰면

\[
q(t)=\Phi(t,t_n)q_n+\int_{t_n}^t\Phi(t,s)[UF(m(s),z(s))+b(s)]\,ds,
\]
\[
m(t)=V^T\Phi(t,t_n)q_n+\int_{t_n}^tV^T\Phi(t,s)[UF(m(s),z(s))+b(s)]\,ds.
\]

이는 변분상수법의 정확한 소거다. 새로운 것은 이 표준 항등식 자체가 아니라, 사용자의 Loop12에서 D에 대해 구성한 packet/오차/비용 계약을 **물리적 band L**에 옮기는 설계다. 일반적으로 m은 memory를 가진 상태이지 r차원 Markov ODE가 아니다. \(V^TL=A_rV^T\)가 성립할 때만 선형부분이 관측량에 닫힌다. q_n의 transverse 초기정보는 자유 profile과 전체 복원에 남겨야 한다.

### 실제 1,600변수 대조

주기 stencil의 제조 모델

\[
\dot q=\nu L_\Delta q-\lambda q-\alpha\bar q^2\mathbf1,
\quad \nu=0.01,\ \lambda=0.7,\ \alpha=0.4
\]

에서 \(\bar q'= -\lambda\bar q-\alpha\bar q^2\)이고 transverse는 알려진 선형 Fourier flow다. 초기값은 \(0.6+0.08\cos(2\pi i/n)+0.03\sin(6\pi i/n)\)이며 모든 n개 값을 보존했다. n=32,128,1600, t∈[0,0.3]의 51개 출력에서 실행했다.

n1600의 물리적 stencil spectral radius는102400.7, 비선형 상태 차원은1, nonlinear J/JVP 요청은0이었다. 복원한 mean의 항등식 오차는1.11e-16이다. n32에 한해 독립 tight DOP853 reference와 최대 절대차1.78835e-12를 확인했다. n1600을 별도 generic stiff solver로 독립 비교한 것은 아니며 시간가속을 측정하지 않았다. FFT가 특히 유리한 주기·고정계수 toy이지 비균일 E-theta FVM 전체가 해결됐다는 결과는 아니다.

### 정확한 저랭크와 단순 평균을 구분

상태 의존 흡수항 \(-\kappa(z)D_\sigma q\)는 z의 수가 작아도 q에 대한 rank가 N일 수 있다. sigma=(1,3), q=(1,0), p=(0,1)는 같은 평균0.5를 가지지만 평균 변화율은 각각-0.5,-1.5다. 평균만 남기는 근사는 이 두 원래 상태를 구별하지 못한다.

따라서 일반 광흡수 모델에는 다음 더 넓은 구성을 제안한다. **물질의 trial waveform z(·)가 주어졌을 때만** 알려진 선형 방사 연산자 L[z](t)의 causal propagator를 계산한다.

\[
q[z](t)=\Phi_z(t,t_n)q_n+\int_{t_n}^t\Phi_z(t,s)b[z](s)ds,
\quad
\mathcal S[z](t)=z(t)-z_n-\int_{t_n}^tG(z(s),V^Tq[z](s))ds.
\]

이 nonlinear Schur memory residual을 z와 필요한 face/moment waveform에 대해 푼다. 이는 시간 동안 z를 영구 동결하는 것이 아니다. 반복되는 trial을 갱신하고 마지막에는 원래 결합 방정식의 defect를 검사한다. Trial radiation flow의 비용은 그대로 남으며, 물질 상태가 공간별로 많으면 작은 r 가정도 재검토해야 한다. 서로 다른 시각의 L이 commute하지 않으면 exp(∫L)로 바꾸지 않는다. Commutator를 정확히 소거하거나 그 모델오차를 따로 감싸야 한다.

## 5. 후보 B: 국소 반응 ideal과 비선형 Schur/flux packet

적절히 같은 반응량 단위로 정규화한 toy reaction을

\[
\dot u=-k u(1-x),\quad \dot x=k u(1-x),\quad u+x=Q
\]

로 둔다. a=hk이고 implicit local balance를 풀면

\[
x-x_0-a(Q-x)(1-x)=0,
\]
\[
a x^2-[1+a(Q+1)]x+x_0+aQ=0.
\]

이는 ideal \(\langle u+x-Q,\;x-x_0-a u(1-x)\rangle\)에서 u를 소거한 식이다. \(u_0\ge0,0\le x_0\le1,a\ge0\)이면

\[
D=1+2a(Q+1-2x_0)+a^2(Q-1)^2\ge1
\]

이고 물리 root는

\[
x_+=\frac{2(x_0+aQ)}{1+a(Q+1)+\sqrt D},\qquad u_+=Q-x_+.
\]

작은 root를 뺄셈 cancellation 없이 표현한 것이다. g(x)=x-x0-a(Q-x)(1-x)에 대해

\[
g(x)-g(y)=(x-y)[1+a(Q+1-x-y)]
\]

이며 물리구간 [x0,min(Q,1)]에서는 증가한다. Endpoint sign과 함께 이 구간 root의 존재·유일성을 직접 확인할 수 있다. 따라서 online state Jacobian이나 Newton iterate가 필요 없다. Wolfram으로 conservation, discriminant, elimination identity와 q=.8,x0=.2,a1000의 root0.79705798725856…를 확인했다.

4개 초기조건×6개a(0부터1e6까지),총24개 bounded 검산에서 보존·물리구간 조건과 정규화 balance residual<1e-9를 확인했다. 관측 최대 residual은약6.99e-12다. 임의로 큰 binary64 입력의 overflow-safe 구현이나 forward error certificate는 아직 제공하지 않는다.

**이 local map은 backward Euler resolvent이지5차 적분기가 아니다.** 이를 고차 waveform/collocation residual의 nonlinear preconditioner 또는 예측자로 사용하거나, exact reaction flow를 이용한 별도 고차 결합을 연구해야 한다. 반응 분할을 한두 번 했다는 이유로 기존 RODAS5P 차수를 상속하지 않는다.

PR70의 chart/cofactor 원칙을 재사용할 때도 마찬가지다. 변환 x→z에서 algebraic branch와 denominator의 whole-tube 조건을 확인하며, 보존 manifold에서 벗어난 오차를 cofactor forcing으로 전달한다. 국소 chart가 singular해지면 다른 chart나 원래 residual로 돌아간다. 기존 no-chart RVJ의 실패를 무시하고 범용 교체로 승격하지 않는다.

Groebner/resultant/SOS compilation은 국소 반응·면 연결·작은 물질 block에서 수행하는 후보다. 전체1600변수에 거대한 elimination을 적용하는 설계는 아니다. Rational 식의 denominator를 곱해 없앨 때 생기는 가짜 root는 denominator≠0 및 물리적 부등식으로 배제해야 한다.

## 6. 후보 C: 대수적 물리 band solve, quotient action, causal scan

물리적 선형 balance가

\[
\begin{pmatrix}A&B\\C&D\end{pmatrix}
\binom qz=\binom{b_q}{b_z}
\]

이고 A의 물리적 band 풀이가 있다면 S=D-CA^{-1}B를 작은 border에서 구성할 수 있다. 여기서 전체 block이 nonlinear Jacobian을 미분해 얻은 것이라면 이번의 강한 derivative-free 계약은 아니다. 이 절은 원 모델이 제공한 선형 flux balance, 또는 조건부 waveform을 고정한 물리적 부분문제에 적용한다.

정확한 lower block 전처리기 P=[[A,0],[C,S]]에 대해서는

\[
P^{-1}W=\begin{pmatrix}I&A^{-1}B\\0&I\end{pmatrix},
\qquad (P^{-1}W-I)^2=0.
\]

따라서 정확한 연산에서 GMRES는 최대2회로 닫힌다. 직접 block elimination도 가능하므로 이것을 최적 speedup이라고 부르지 않는다. 실제 작은 시험에서 band 차원32/128,물질차원2에 대해 diagonal preconditioner는34/130회, exact band-Schur는2/2회였고 후자의 상대 residual은3.13e-15,7.98e-15였다. 근사 S·inexact A solve에서는 nilpotence가 깨져3~5회 보장은 사라진다. 바뀌는 preconditioner나 비선형 subsolve는 일반 고정 선형 GMRES의 계약과도 구분해야 한다.

### 전체 Jacobian 없이 대역 구조를 쓰는 세 방식

- **알려진 tensor 연산자의 작용**: L=L_E⊗I+I⊗L_theta가 정확히 성립하고 commute하면 분리된 flow가 가능하다. 시간·상태 의존 coefficient와 boundary가 이 구조를 깨는 경우 별도 defect가 필요하다.
- **one-way flux의 associative scan**: x_i=α_i x_{i-1}+β_i의 두 map 합성은 (α2α1,β2+α2β1)다. upwind/bidiagonal 물리 balance에서 전파를 tree-scan으로 구성할 수 있다. 실수 연산의 associativity와 부동소수점의 bit equality는 다르므로 scan 자체의 rounding radius를 전달한다. 공간 O(log N)span은8stage 순차 깊이와 다른 축이다.
- **몫대수/finite jet**: q(L)U=0가 인증되면 강제응답의 작용대수 차수가 줄어든다. q(L)q0까지 작지 않으면 초기 자유응답을 함께 줄여서는 안 된다. Generic band diffusion은 최소다항식 차수가 N일 수 있어, banded라는 사실만으로 작은 quotient를 보장하지 않는다.

수송 방향성은 물리적으로 주어진 flux graph의 partial order에서 가져온다. 주기 방향이나 bidirectional 산란은 strongly connected block으로 묶어 풀고, 전역 moment coupling은 작은 border로 남긴다. 서로 독립인 cell/face/mode packet은 병렬이지만, accepted time-window의 초기값 의존성은 사라지지 않는다. 공간적으로 비국소인 implicit diffusion이 시간 인과성을 깨는 것은 아니다. Runtime causal은 미래 참해·사후 reference·다음 관측값을 사용하지 않는다는 뜻까지 포함한다.

## 7. Jacobian/JVP 없는 연속 오차 인증

물리 상태의 candidate curve \(\widehat y(t)\)에 대해 원 방정식의 연속 defect

\[
d(t)=\dot{\widehat y}(t)-F(t,\widehat y(t))
\]

를 직접 계산한다. Candidate 다항식의 시간미분은 nonlinear state Jacobian이 아니다. F가 생성형 polynomial/rational graph라면 합성, Bernstein enclosure와 denominator/domain 검증으로 시간 전체의 defect를 감쌀 수 있다. 몇 개 시각 샘플만으로 sup defect를 인증하지 않는다.

정규화된 고정 metric M에서 모든 x,y가 유효 tube 안에 있을 때

\[
\langle x-y,F(t,x)-F(t,y)\rangle_M
\le\mu(t)\|x-y\|_M^2
\]

를 두 상태의 polynomial identity/부등식으로 인증하면, Jacobian norm을 runtime에 계산하지 않고

\[
r(t)\le e^{\int_{t_n}^t\mu}r_n+
\int_{t_n}^t e^{\int_s^t\mu}\,\delta(s)ds,
\qquad \|d(s)\|_M\le\delta(s)
\]

가 따른다. 원래 PR의 one-sided/lognorm certificate를 **두 점의 식에 대한 인증**으로 옮기는 것이다. 이 관계의 수학 자체는 새로운 정리가 아니라 이번 모델 구조에 쓰는 constructive extension이다.

### 격자 diffusion에서 stiffness를 양의 gain으로 바꾸지 않기

Incidence B,양의 edge conductance K,cell measure M가 주어지고

\[
F_{\mathrm{diff}}(y)=-M^{-1}BK B^Ty
\]

이면 e=x-y에 대해

\[
e^TM[F_{\mathrm{diff}}(x)-F_{\mathrm{diff}}(y)]
=-(B^Te)^TK(B^Te)\le0.
\]

이 exact sparse quadratic/SOS identity는 full Jacobian 없이 확인된다. 각 edge의 monotone nonlinear flux φ에 대해서도 -(B^Te)^T[φ(B^Tx)-φ(B^Ty)]≤0이면 동일한 역할을 한다. 전자의 매우 큰 음의 diffusion eigenvalue를 |L|로 바꿔 e^{h||L||}에 넣어 인증을 무용하게 만드는 것을 피한다. 실제 radiation/reaction 전체가 같은 metric에서 dissipative하다는 증명은 별도로 필요하며, positivity/conservation만으로 임의의 Euclidean contractivity를 단정하지 않는다.

### 인증 셀과 limiter branch

기존 atlas를 이용해 낮은 차원의 parameter/branch cell마다 flux bounds,monotonicity,반응 branch,denominator 양성,physical reconstruction gain을 offline 증명한다. Runtime은 셀 membership과 실제 coefficient uncertainty·mesh epoch를 확인한다. State 차원 N 전체를 tensor Bernstein parameter차원으로 놓으면 비용이 폭증하므로 local factorization/분리 구조가 필요하다.

Minmod 등의 비매끄러운 표현에서도 **선택된 branch를 증명된 tube 안에서 유지하되 live state로 그 branch 수식을 평가**하면 원래 함수를 그대로 계산한다. 반대로 slope/weight의 숫자 자체를 step 시작값에 고정하면 일반적으로 다른 방정식을 푼다. Branch-crossing을 감지하면 window를 줄이거나 piecewise interval을 나누고, 필요하면 전체 nonsmooth flux bound로 안전하게 전환한다. Rational smooth limiter도 원 target/양성/TVD 성질과 regularization 단위를 따로 확인한다.

## 8. 실제 adaptive + dense-output 인증 예제

`adaptive_scalar_certified.py`는 다음 한 모델을 직접 적분한다.

\[
y'=-2y-y^2,\quad y(0)=1/2,\quad t\in[0,1].
\]

각 accepted center에서 차수5 다항식을 만든다. 계수는 state derivative 호출 대신 정확한 rational polynomial 합성 recurrence

\[
a_{k+1}=\frac{-2a_k-\sum_{j=0}^k a_j a_{k-j}}{k+1}
\]

로 계산한다. \(d=q'+2q+q^2\)는 차수10이며 Bernstein 계수의 절댓값 최대가 시간창 전체의 δ 상계다. 동시에 q가[0,1]에 있음을 coefficient convex hull로 검사한다.

두 비음수 상태 u,v에 대한 exact identity

\[
(u-v)[F(u)-F(v)]=-(2+u+v)(u-v)^2\le-2(u-v)^2
\]

와 참해의 비음수 불변성에서

\[
r(t)\le e^{-2(t-t_n)}r_n+
\tfrac12\delta[1-e^{-2(t-t_n)}]
\]

가 따른다. Exponential은 이 예제의 h≤1/4범위에서 정확한 rational alternating Taylor 하한/상한으로 감싼다. 이 비교함수는 창 안에서 단조이므로 양 끝 반경의 최대가 전창 상계다.

Polynomial계수·Bernstein·proof arithmetic은 Python Fraction의 정확한 정수/유리수다. 반환 중심은 binary64로 변환하며 그 exact rational displacement를 radius에 더한다. [0,1]에서 uniformly safe한2^-53 conversion allowance를 dense-output 전창 admission에 포함한다. 다음 창의 radius는 이전 radius를 전파한 값이지0이 아니다. Query별 비교와 전창 상계는 둘 다 유효하므로 두 값의 최소를 사용한다.

결과:

|항목|실행값|
|---|---:|
|Accepted windows|28|
|Rejected trials|5|
|연속출력 query 추가검사|101|
|nonlinear state Jacobian/JVP/FD/AD calls|0|
|전체 absolute tolerance|1e-6|
|최대 반환 반경|8.933370631013405e-7|
|90자리 exact solution과의 최대 관측오차|1.5435012476975697e-7|

실패 trial은 accepted time/center/radius를 변경하지 않았다. 모든 창에서 polynomial positivity,전체 반경상계,다음창 초기반경 연결을 exact arithmetic으로 검사했다. 101개 샘플은 부가 검산이며 인증의 논리는 연속 Bernstein bound와 비교부등식에 있다.

이는 **이 명시된 scalar 모델과 정확한 rational 연산·반환 변환에 한한 계산적 인증**이다. 범용 runtime ODE certificate,기존 Rust packet certificate,1600변수 FVM의 인증,정식 proof-assistant 검증으로 확대하지 않는다. Fraction 비용이 크고 zero-allocation/속도 이득을 주장하지 않는다. 단순 계획만 제시하지 않고 요청한 root/time/dense/inherited/roundoff 연결이 비어 있지 않음을 실제 산출했다는 의미다.

## 9. Root·시간·dense 오차를 결합할 때 이중 계산하지 않기

동일한 실제 반환 curve의 full continuous defect를 인증하면 불완전한 root,시간차수의 절단,다항식 projection이 그 defect에 이미 반영된다. 동일 효과에 별도 embedded estimate나 stage residual bound를 다시 무조건 더하면 중복 부과다. 분리 예산을 사용할 때는 proof상 비중복 항으로 정의한다.

반대로 full model이 아니라 근사 opacity·표·limiter·frozen propagator로 defect를 계산했다면

\[
\delta_{\mathrm{total}}
\ge\|\widehat y'-F_{\mathrm{true}}(\widehat y)\|
\]

를 위해 모델 불일치·계수불확실성·출력 산술 항을 추가해야 한다. 시간적분의 semidiscrete ODE certificate와 원 PDE의 공간절단오차도 구분한다. 물리적 scale가 다른 에너지/이온화율/온도를 raw Euclidean norm에 그대로 합치지 않고 metric과 단위를 고정한다. Metric이 ALE/시간에 따라 변하면 그 시간 변화에 따른 비교항도 포함한다.

## 10. AMR·ALE·제한자에 관한 조언의 교정

### 계수 동결의 정확한 반례

y'=y²,y0=1에서 매 step 한 factor를 y_n으로 고정한 y'=y_n y를 정확히 풀면 y_{n+1}=y_n exp(h y_n)이다. 원 해1/(1-h)와의 local 차이는 -h²/2+O(h³)다. t=.4에 대해10,20,…,320steps에서 실제 global order는0.954→0.997로1차에 접근했다. Frozen문제 자체를 완벽히 푸는 것만으로 원 문제의 고차 정확도는 보장되지 않는다.

### 보존 remap의 정확도 반례

Smooth density1+.2sin(2πx)의 cell content를 정확히 적분한 뒤 두 cell을 합치고 같은 평균으로 다시 나누면 총량은 보존된다. n32/64/128/256에서 합산 absolute content error는.0125403,.0062550,.00312563,.00156258이며 관측 차수는약1이다. 따라서 conservative라는 한 조건만으로5차가 유지되지 않는다.

### 반경을 포함한 remap 계약

고정된 old/new cell-content 공간 사이 선형 projection P에 대해

\[
q^+=P\widehat q^-,\qquad
r^+\le |P|r^-+\eta_{\mathrm{map}}+\eta_{\mathrm{round}}.
\]

여기서 η_map은 old-represented 상태를 new-target 상태로 바꾸는 물리/공간 재구성 차이의 상계다. P≥0,각 column합1이면 content l1에서 nonexpansive이지만 고차 signed remap에서는 그 성질을 가정하지 않는다. 물질 피드백의 moments가 바뀌지 않아야 한다면 \(V_+^TP\widehat q^--V_-^T\widehat q^-\)도 검증한다. 단순 질량보존은 photon energy/heating의 서로 다른 moments를 모두 보존한다는 뜻이 아니다.

새 mesh에서 같은 operator/closure로 계속 간다고 주장하려면 \(L_+P-PL_-\)의 commutation defect 또는 그에 상응하는 spatial consistency항도 필요하다. Mesh topology,metric,model,limiter branch가 바뀌면 Krylov/basis/factor/certificate epoch를 갱신한다. Local proof template은 그대로 재사용할 수 있어도 그 입력 binding까지 그대로 재사용할 수는 없다.

### ALE는 차원만 고정하면 끝나지 않음

Cell content Q_i=V_i f_i에 대해 moving-mesh flux는

\[
\frac d{dt}(V_i\bar f_i)=-\sum_{f\in\partial i}A_f[\mathcal F_f-\bar f_f v_{g,f}]\cdot n_f+V_i S_i.
\]

균일상태를 유지하려면 같은 시간/면 적분에서 \(dV_i/dt=\sum_f A_f v_{g,f}\cdot n_f\)라는 geometric conservation law를 만족해야 한다. FixedN은 이 의무를 없애지 않는다. E² dE dΩ 같은 가중 phase-space measure를 쓰면 V와 flux의 정의 및 좌표변환 Jacobian에 그 가중치를 일관되게 반영해야 한다. Energy의 log/comoving 좌표도 상태가 점별 f인지 cell-integrated measure인지 먼저 정의해야 한다. 공변 좌표가 적색편이 advective drift를 제거할 수 있어도 threshold/source coefficient의 시간 의존성은 남는다.

### BDF·크기·할당에 대한 범위

CVodeReInit이 실제로 history를 지우고1차에서 다시 시작한다는 부분은 공식 자료로 확인했다. 해당 reinitialization은 같은 문제크기를 전제한다. 그러나 BDF가 언제나 과거5개step을 고정적으로 요구하거나 remesh 후 반드시 극소 h에 갇힌다는 주장은 성립 근거가 없다. One-step은 history 이식 부담을 줄이지만 remap오차,새domain,stiff order reduction,limiter 변경 때문에 다음 h의 안전성은 별도로 판단해야 한다.

고정 half-bandwidth b의 band LU는 O(N b²),storage O(N b)규모다. Energy-major E-theta 배치에서 b가 N_theta 또는 angular block크기와 함께 늘 수 있어 N만 늘리는 선형비용 주장과 구분해야 한다. 산란도 항상 theta인접항은 아니고 dense integral/low-rank angular kernel 또는 energy비국소 coupling일 수 있다. 실제 graph로 판별한다. 현재 사용 RODAS5P stage count는8로, 제시한 조언의6과 다르다.

Nmax행 벡터를 미리 확보하는 것은 필요할 수 있지만, Krylov basis Nmax×(mmax+1),block RHS,preconditioner fill/symbolic state,packet degree/rank,thread scratch,remap buffer,certificate/output archive까지 hot loop에서 재할당되지 않는지 별도 확인해야 한다. Dimension변경 시 active length뿐 아니라 stride·capacity·basis·operator identity도 reset한다. Zero-allocation은 추측하지 않고 allocator계측으로만 보고한다.

## 11. Causal 자동 선택의 목적함수

선택변수 a=(method,h,mesh,degree,rank,workers,certificate-route)에 대해 최소화할 것은

\[
\mathcal J(a)=
\frac{\mathbb E[C_{\mathrm{setup}}+C_{\mathrm{trial}}+C_{\mathrm{certificate}}
+C_{\mathrm{reject}}+C_{\mathrm{fallback}}+C_{\mathrm{remap}}]}{\mathbb E[\Delta t_{\mathrm{accepted}}]}
\]

이고,물리적 domain/invariant와 요구한 certified output budget을 제약으로 둔다. 후보 root solve의 latency나 JVP수만의 최소화가 아니다. 모든 정보는 현재 accepted state와 허용된 trial/probe에서 얻으며 미래 참해·사후 reference는 선택에 사용하지 않는다.

기존 G4/G5의 residual allocation은 검증된 contraction·영향도·cost모델에 한해 활용한다. 선형화된 비용모델의 최적화,온라인 학습의 경험적 성과,모든미래문제에서 전역최적이라는 주장은 구분한다. Fixedcaps64/256의 실패를 바탕으로 projected dimension³,graph evaluation,proof boundary crossings,expected remaining rounds와 cutoff시점의 sunk work까지 계측한다.

AMR의 모델링된 이익도 남은 계산에서 절약할 비용이 remap+recompile+revalidate+history-reset+error-budget 비용보다 클 때다. RODAS5P와 BDF를 비교한다면 같은 물리출력오차·격자변경 schedule와 실제 필요 history전략을 비교해야 한다. One-step의 장점을 증명하기 위해 BDF에 불필요한 재시작을 강요하지 않는다.

## 12. 남겨둘 경쟁 후보와 다음 최소 구현

이번 발산 탐색에서는 다음을 동시에 유지한다.

- **정확 저랭크/quotient family:** L과 U,V의 구조가 증명되면 가장 직접적인 derivative-free 후보. 큰원상태를 보존하면서 nonlinear dimension을 줄인다.
- **조건부 transport+matter waveform:** 흡수항이 고랭크이면 이 경로. L[z]처리비용과 시간순서 보존이 승부처다.
- **local reaction ideal+face waveform:** scalar저랭크가 없는 격자에서도 local algebra와 graph를 이용한다. 고차target과 국소resolvent 차수는 구분한다.
- **certificate-first polynomial packet:** 후보를 만드는 알고리즘과 무관하게 원래 continuous curve의 defect를 닫는다. 이번 scalar구현은 구체적인 시작점이다.
- **보호된 현재 JVP RODAS5P/structured solver:** strong derivative-free 조건을 만족하지 않는 모델의 별도 fallback으로 유지한다. 이것을 연구목표의 달성으로 재명명하지 않는다.

다음 한 native 구현 단위는 `GeneratedFluxModel`(flux/reaction/invariant/domain)과 `CertifiedWindow`(represented curve,defect,tube,inherited radius,output evaluation)를 연결하는 고정 비균일격자 소형모델이다. Callback계약에서 J/JVP/AD/FD요청을 제거하고 물리적 stencil apply와 local-resolvent,coefficient/proof work를 별도로 센다. 먼저 고정격자에서 conservation·positivity·continuous-output를 닫고, 그다음 accepted-window간 remap반경/epoch를 연결한다. ALE는 GCL과 time-dependent metric시험을 추가한 별도 가지로 두는 것이 원인분리에 유리하다.

이번에 실제 실행한 것은6개bounded실험 묶음과 별도adaptive scalar certification이다. 결과는 `RESULTS.json`, `ADAPTIVE_SCALAR_RESULT.json`,코드는 두 Python파일에 있다. 새로운 native 속도 측정이나 실제 FVM전체 인증을 완료했다고 주장하지 않는다. 기존 PR/production 기본값은 변경하지 않았다.

## Sources and exact identities

The source map records inspected paths and their actual scope. Main code pin:
`https://github.com/cosmosapjw-quantum/vigilode/tree/c03ca90f19a9e64ba7751c069325a48b54b39c2e`.

Ancillary verification, not the design's authority:
- https://computing.llnl.gov/projects/sundials/usage-notes
- https://sundials.readthedocs.io/en/v7.2.1/cvode/Usage/index.html

Wolfram symbolic input and returned identities are stored in SYMBOLIC_CHECK.wl and SYMBOLIC_RESULT.txt. Search results did not supply or validate the new FVM/packet construction. No academic novelty claim or native-subagent/independent-personnel review claim is made.
