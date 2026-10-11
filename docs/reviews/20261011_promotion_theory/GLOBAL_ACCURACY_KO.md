# 전역 정확도와 비균일 잔차 예산의 구성적 정리

작성: 2026-10-11. 대상: VigilODE U-form Rosenbrock stage, 최신 source `8ce9bda`를 읽은 이론 연구. 상태: **DERIVED, 직접 대수 검산 포함; production 구현·실측 속도 승격 아님**. 이전 공개 campaign은 재실행하지 않았다. 이 문서는 후보 생성자의 유도이며 독립 decision reviewer의 판정을 대신하지 않는다.

## 1. 달성한 양의 결과

기존 L0048의 특정 모델 전역 인증을 재사용하고, 여기서는 선언된 모델 class의 일반 U-form에 대해 `linear residual → nonlinear full stage → step → ODE global error` 합성 정리를 명시적인 검증 가능 가정 아래 확장한다. 기존 전역 연구가 없었다는 뜻이 아니며, 현재 native whole-stage/global witness 연결의 남은 구현 경계를 구체화한다. 이어서 그 정리에서 나오는 **오차 영향 가중치**를 이용해, 동일한 전역오차 예산을 만족하는 선형 반복 횟수를 최소화한다. 이 조합의 문헌상 신규성은 확립하지 않았다. 결과는 단순한 불가능성 정리가 아니다.

* 고정된 stage DAG에서 전이행렬을 역행렬로 만들지 않고 backward sweep 한 번으로 모든 잔차의 최종오차 영향도를 구한다.
* 검증된 기하 잔차 감소를 가진 backend에서는 비균일 residual allocation의 닫힌 해를 얻는다.
* 명시한 정확 산술 예에서 같은 총오차 상계 `1/16`을 만족하는 횟수를 **68에서 36으로 줄인다**. 기준은 같은 backend의 최적 공통 residual threshold다.
* 영향도 불균형이 `2^L`인 m개 solve family에서는 scheduled iteration work를 `Θ(mL)`에서 `O(L+m log m)`로 줄이는 매개변수적 복잡도 정리를 얻는다. 현재 RODAS5P는 stage 수가 고정되어 있으므로 이 마지막 점근식을 곧바로 현재 코드의 dimension scaling이라고 부르지 않는다.

보장하는 것은 **구조·bounds가 검증된 regime의 정확도와 산술 작업량**이다. 같은 조건을 확인하지 않은 arbitrary GMRES, 임의 nonnormal ODE, 전체 runtime 또는 production default에 보장을 확장하지 않는다.

## 2. 표기, 실제 target, 유한정밀도

유한한 양의 scale `s_n`을 step n 동안 고정하고

\[
\|v\|_n=\|D_n v\|_2/\sqrt d,\qquad D_n=\operatorname{diag}(1/s_n)
\]

라 한다. 이에 대응하는 induced operator norm은 `||D_n A D_n^{-1}||_2`다. stage 수를 s, 공간 차원을 d로 구분한다.

정확 reference step의 입력은 **지정된 exact-real** `(t_n,h_n,y_n,J_n,f_{t,n},tableau)`다. 실제 코드의 U-form은 다음이다.

\[
W_n=I-h_n\gamma J_n,\quad
W_nU_i^*=b_i(U_{<i}^*),
\]
\[
b_i(U_{<i})=h_n\gamma f(t_n+c_i h_n,
 y_n+\sum_{j<i}a_{ij}U_j)
 +\gamma\sum_{j<i}C_{ij}U_j+h_n^2\gamma\gamma_i f_{t,n}.
\tag{1}
\]

출력은 `Ψ_n(y_n)=y_n+Σ b_i^out U_i*`다. 저장된 U-form 계수와 K-form 계수를 섞지 않는다. 현재 source의 stage rhs assembly를 확인했다. source는 `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`의 `rhs = h gamma f_i ...` 구간이며 output coefficient는 `b_code`다.

**세 종류의 mismatch를 별도로 기록한다.**

1. 실제 형성·callback 연산자 `Wtilde`와 reference W의 차이.
2. 실제 계산 rhs와 exact `b_i(Uhat_<i)`의 차이: state/time assembly, RHS·f_t 평가, 계수, h, γ 연산 반올림 포함.
3. 최종 output assembly 반올림.

실제 residual 검산이 `rtilde_i=btilde_i-Wtilde Uhat_i`의 norm을 `ρ_i`로 상계하고,

\[
\|b_i(\widehat U_{<i})-\widetilde b_i\|_n\le\xi_i,
\quad
\|(\widetilde W-W)\widehat U_i\|_n\le\omega_i
\]

이면 exact reference equation의 defect는

\[
\delta_i:=\|b_i(\widehat U_{<i})-W\widehat U_i\|_n
 \le \rho_i+\xi_i+\omega_i.
\tag{2}
\]

`ω_i ≤ ||D(Wtilde-W)D^-1|| ||Uhat_i||_n`로 계산할 수 있지만, candidate-specific bound가 더 작으면 그것을 쓴다. finite-difference JVP가 exact linear map을 정의하지 않아도, 최종 candidate에서 `computed_W(Uhat)-W Uhat`의 검증된 상계로 같은 정리가 성립한다. 단지 linear라고 가정해서는 안 된다. residual 자체의 계산 오차는 ρ에 포함한다.

floating time `fl(t+h)`가 exact accepted h의 합과 다르면, exact times를 별도 추적하고 callback time displacement를 ξ에 포함하거나 실제 grid 간격에 맞춰 h를 정의해야 한다. 최종 보고 시간이 다르면 `sup||f||·|Δt|`도 마지막 error에 추가한다. 이 계약은 반올림을 무시하는 exact-arithmetic 주장이 아니다.

## 3. 정리 G1: whole-stage contamination과 adjoint 영향도

**가정.** 각 stage i에 대해:

* W는 가역이고 `||W^-1||_n ≤ G_i`인 검증된 full-space 상계가 있다. 더 작은 restricted gain은 **candidate defect와 모든 nonlinear/linear coupling 차이의 합**이 속한다고 증명된 불변 부분공간에 대해서만 허용한다. 관측된 residual 한 방향에 대한 gain을 나머지 coupling 항에 재사용하지 않는다.
* stage-state tube `T_i`에서 f는 metric n에 대해 Lipschitz 상수 L_i를 갖는다.
* candidate state `y_n+Σa_ij Uhat_j`와 아래 recurrence가 감싸는 exact stage state 사이 선분이 T_i 안에 있다.
* (2)의 δ_i와 output assembly error `e_out`가 outward로 감싸져 있다.

strictly lower-triangular 비음수 행렬 A와 diagonal G를

\[
A_{ij}=G_i\bigl(|h_n\gamma|L_i|a_{ij}|+|\gamma C_{ij}|\bigr),\quad j<i,
\qquad G=\operatorname{diag}(G_i)
\tag{3}
\]

로 정의한다. `A_ij=0` for `j≥i`. 그러면

\[
q=(I-A)^{-1}G\delta
=\left(I+A+\cdots+A^{s-1}\right)G\delta
\tag{4}
\]

는 모든 stage error를 감싼다: `||Uhat_i-U_i*||_n ≤ q_i`. 따라서

\[
\|\widehat y_{n+1}-\Psi_n(y_n)\|_n
\le w_n^T\delta+e_{out,n},\quad
w_n=G(I-A)^{-T}|b^{out}|.
\tag{5}
\]

**증명.** reference equation에서 candidate equation을 빼고 W^-1를 곱한다. rhs 차이는 앞선 stage 차이에 대한 Lipschitz 항과 `γC`의 선형항, candidate defect의 합이다. 그 norm은 `G_i δ_i+Σ A_ij ||Uhat_j-U_j*||`로 상계된다. i=1부터 유도한다. A는 nilpotent이므로 geometric series는 유한한 정확 identity이며 `||A||<1` 가정이 필요 없다. output에 삼각부등식을 적용한 뒤 `(I-A)^-1`를 transpose하면 (5)다. □

**tube의 구성적 확인.** 먼저 후보 주변 T_i와 L_i를 제안한다. i 순서로 q_i를 구한다. exact state의 후보 중심 반경 `Σ_{j<i}|a_ij|q_j`가 T_i 반경 이내인지 확인한다. 이전 stage들이 이미 감싸졌으므로 이는 순환 가정이 아니라 인과 induction이다. polynomial/quadratic f라면 interval Jacobian 또는 Bernstein bound로 L_i를 구할 수 있다. tube check가 실패하면 그 step의 인증은 실패다.

**계산 비용.** `v=(I-A)^-T |b|`는 i=s,...,1의 backward substitution:

\[
v_i=|b_i^{out}|+\sum_{j>i}A_{ji}v_j,\quad w_i=G_i v_i
\tag{6}
\]

로 계산한다. stage dependency edge 수 e_s에 대해 `O(s+e_s)` scalar arithmetic, `O(s)` 추가 저장이다. dense s×s inverse나 O(s³) 작업은 필요 없다. current s가 작아도 componentwise tube cert와 residual budget의 연결이 명확해진다.

**outward rule.** A,G,δ를 상계로 교체하면 q,w도 단조 증가한다. 모든 비음수 연산을 upward로 수행하면 (4)–(6)의 floating implementation도 sound하다. coefficient/time/normalization 변환 오차는 이미 (2)에 포함해야 한다.

## 4. 정리 G2: estimator를 사용하지 않는 truncation 상계

embedded vector는 아래 정리에서 truncation authority로 쓰지 않는다. 두 가지 구성적인 방법 중 하나를 채택한다.

### 4.1 실제 저장 coefficient의 Taylor defect

step 시작 y를 고정한다. `F(h)=Φ(t+h,t,y)`를 exact ODE flow, `P(h)=Ψ_{t,h}(y)`를 (1)의 **지정한 실제 tableau/J/f_t를 사용하는 exact-real step**이라 한다. `F(0)=P(0)=y`, 둘이 h∈[0,h_n]에서 C⁶이고, 모든 reference stage/flow가 검증 tube 안에 있다고 하자. 다음 bounds를 구한다.

\[
d_j\ge\|F^{(j)}(0)-P^{(j)}(0)\|_n/j!,\quad j=1,...,5,
\]
\[
M_{F,6}\ge\sup_{0\le u\le h_n}\|F^{(6)}(u)\|_n,\quad
M_{P,6}\ge\sup_{0\le u\le h_n}\|P^{(6)}(u)\|_n.
\]

그때

\[
\tau_n:=\sum_{j=1}^5 d_j h_n^j+
\frac{h_n^6}{6!}(M_{F,6}+M_{P,6})
\ge\|\Phi(t_n+h_n,t_n,y)-\Psi_n(y)\|_n.
\tag{7}
\]

**증명.** 두 함수에 5차 Taylor 정리와 integral remainder를 적용하고 빼면 된다. remainder norm 각각은 `M h⁶/6!` 이하이다. □

**왜 유용한가.** 검증된 이상적 5차 tableau와 정확 Jacobian/f_t를 target으로 정하면 d_j=0을 order-condition 증명으로 닫을 수 있다. 그러나 저장 binary64 tableau를 exact-real 수로 해석하면서 order condition이 정확히 성립한다고 선언해서는 안 된다. 실제 저장 계수로 d_j를 interval Taylor arithmetic에서 구하면 낮은 차수의 coefficient perturbation도 잡는다. approximate J로 인해 order가 달라지는 경우도 d_j 또는 reference mismatch로 명시적으로 잡는다.

계산은 h에 대한 truncated jets로 수행할 수 있다. `W(h)Pstage_i(h)=rhs_i(h)`를 차수별 differentiate하면 W(0)=I인 Taylor 계수에서 triangular recurrence를 얻는다. h>0의 6차 remainder에는 uniform resolvent bound와 RHS derivatives가 필요하다. polynomial f에서 AD와 interval/Bernstein arithmetic으로 계산 가능하다. `M_F,6`은 extended field `(1,f)`의 반복 Lie derivatives로 표현할 수 있다. `M_P,6`을 단순 표본 미분값으로 대체하지 않는다.

### 4.2 defect integral의 대안

미분 가능한 reconstruction p(u)가 `p(0)=y`이고 `||p(h)-Ψ_n(y)||_n≤ε_end`를 만족한다. tube 내 one-sided Lipschitz 상수가 μ(u), defect bound가 `||p'(u)-f(t+u,p(u))||_n≤d(u)`이면

\[
\|\Phi(t_n+h_n,t_n,y)-\Psi_n(y)\|_n\le\tau_n,
\quad\tau_n:=\operatorname{up}\!\left[
\epsilon_{end}+
\operatorname{up}\!\int_0^{h_n}\exp\!\left(\int_u^{h_n}\mu(v)dv\right)d(u)du\right].
\tag{8}
\]

오차 방정식의 norm differential inequality와 variation-of-constants로 증명된다. `up integral`은 적분값 자체의 검증된 상계를 뜻한다. d(u)의 interval/Bernstein enclosure 및 integral remainder까지 인증한다. floating Simpson 수치값은 이 적분의 상계가 아니다. polynomial d≤Σ d_k u^k와 constant μ가 있으면 scalar exp/phi로 정확한 양의 integral formula를 만들 수 있다. 특히 μ≤0일 때 integral≤∫d로 충분하다.

reconstruction endpoint를 computed candidate로 잡아 **후보 전체 오차**를 직접 인증할 수도 있다. 그 경우 (5)의 contamination을 다시 더하지 않는다. 두 경로는 서로 다른 error accounting이다.

## 5. 정리 G3: metric 변화와 전역 ODE 정확도

accepted step grid `t_{n+1}=t_n+h_n`, n=0,...,N−1, h_n>0를 고정한다. 실제 adaptive 경로가 선택한 grid에도 사후 조건이 검증되면 정리가 적용된다. 튜브에는 true trajectory와 각 computed start의 exact flow 및 이들을 연결하는 선분이 들어 있어야 한다.

metric n에서 exact flow가 다음 검증된 perturbation bound를 만족한다고 하자.

\[
\|\Phi_n(x)-\Phi_n(y)\|_n\le\Lambda_n\|x-y\|_n,\qquad
\Lambda_n=\operatorname{up}\!\exp\left(\operatorname{up}\!\int_{t_n}^{t_{n+1}}\mu_n(t)dt\right).
\]

여기서 μ_n은 tube 전체에서 `μ_2(D_n J_f(t,y)D_n^-1)`의 상계다. 더 작은 Λ_n을 쓰려면 그 값 자체에 별도 flow-Lipschitz witness가 있어야 한다. metric transport를

\[
\kappa_n=\|D_{n+1}D_n^{-1}\|_2
=\max_j\frac{s_{n,j}}{s_{n+1,j}},\quad S_n=\kappa_n\Lambda_n
\tag{9}
\]

로 둔다. `E_n=||yhat_n-y(t_n)||_n`에 대해

\[
E_{n+1}\le S_n E_n+
\kappa_n\left[\tau_n+e_{out,n}+w_n^T(\rho_n+\xi_n+\omega_n)\right].
\tag{10}
\]

따라서 empty product를 1로 두면

\[
E_N\le\left(\prod_{k=0}^{N-1}S_k\right)E_0
 +\sum_{n=0}^{N-1}\beta_n
 \left[\tau_n+e_{out,n}+w_n^T(\rho_n+\xi_n+\omega_n)\right],
\]
\[
\beta_n=\kappa_n\prod_{k=n+1}^{N-1}S_k.
\tag{11}
\]

**증명.** computed output와 true solution 사이에 `Ψ_n(yhat_n)`과 `Φ_n(yhat_n)`를 삽입한다. 첫 차이는 G1, 둘째는 G2, 셋째는 exact-flow perturbation bound로 감싼다. metric을 (9)로 운반하면 (10). 이를 시간 순서로 전개하면 (11). numerical step-map의 Jacobian을 새로 상계할 필요가 없다. □

**정확도 승격의 실질적 의미.** (11)의 오른쪽이 요청한 E_total 이하이고 모든 bounds가 검증되면 그 finite trajectory의 global error를 promote할 수 있다. 이 수학 정리만으로 현 driver에 witness가 이미 연결되었다고 주장하지 않는다. `μ≤0`, fixed metric, uniform τ_n≤C h_n⁶이고 residual/fixed arithmetic contamination의 합이 budget 이내라면 `Σ h_n⁶≤T h_max⁵`이므로

\[
E_N\le E_0+CT h_{max}^5+E_{linear}+E_{arithmetic}
\tag{12}
\]

를 얻는다. 양의 μ≤M이면 `exp(MT)` factor를 추가한다. C가 stiffness에 독립인지 별도로 증명하지 않은 채 stiff-uniform fifth order라고 부르지 않는다.

**online 실행.** 미래 β를 모르면 이미 관측된 β를 예언값처럼 쓰지 않는다. 고정 metric/μ≤0인 regime에서는 `β≤1`이므로 각 accepted step에 `E_linear h_n/T`를 배분하면 합이 E_linear 이하다. 일반 regime에서는 미리 검증한 future amplification upper bound를 사용하거나 유한 window의 reserved budget을 쓴다. 입력 trajectory가 바뀌면 tube와 weights를 다시 감싸야 한다.

**flow tube의 구성.** f가 candidate 중심의 닫힌 convex ball `B(yhat_n,R)`의 열린 근방에서 C¹이고, input error ball의 반경 E_n과 bound `M≥sup_{t∈[t_n,t_n+h_n],z∈B}||f(t,z)||_n`에 대해 `E_n+h_n M<R`이면 input ball에서 출발한 exact flows는 이 step 동안 B 내부에 머문다. 첫 탈출 시각을 가정하여 적분식 `||y(t)-yhat_n||≤E_n+(t-t_n)M<R`을 적용하면 모순이다. compact tube와 local existence/continuation으로 full-step existence도 확보한다. 실제 stiff 예에서 이 단순 ball이 너무 크면 validated Picard boxes 또는 logarithmic-norm trajectory tube를 쓸 수 있지만, 그 새로운 enclosure도 증명되어야 한다. allocation을 바꾸기 전에 해당 residual budget으로 유도되는 모든 q_i와 E_n이 **동일하게 검증한 tube**에 들어가는지 확인한다.

## 6. 정리 G4: 보장된 작업량을 최소화하는 residual allocation

시간과 stage pair를 단일 index i=1,...,m으로 펼친다. (11)로부터 각 reducible residual의 영향도는

\[
a_i=\beta_n w_{n,j}\ge0.
\tag{13}
\]

initial error, truncation, RHS/operator/output arithmetic과 아래 iteration floor를 먼저 전역 budget에서 공제한다. 남은 B가 양수라고 하자. `a_i=0`인 solve는 출력 정확도 budget 때문에 반복할 필요가 없지만 stage existence/domain 등 별도 안전 조건은 유지한다.

선형 backend에 다음 **검증된** residual 감소가 있다고 가정한다.

\[
\rho_i(k)\le R_i q_i^k+\eta_i,\quad
R_i>0,\quad0<q_i<1,
\tag{14}
\]

iteration당 charged work c_i>0. `Σa_iη_i`는 B에서 이미 공제한다. 원하는 reducible residual target `0<r_i≤R_i`의 충분 반복 횟수는

\[
k_i=\left\lceil\frac{\log(R_i/r_i)}{-\log q_i}\right\rceil.
\tag{15}
\]

backend 자체가 solve success와 domain safety를 위해 최소 k_min을 요구하면 R_i를 그 mandatory prefix 이후 residual bound로 재정의하고 prefix work를 양쪽에 동일하게 charge한다.

`d_i=-log q_i`, `p_i=c_i/d_i`라 하자. 연속 relaxation

\[
\min_{0<r_i\le R_i}\;F(r)=\sum_i p_i\log(R_i/r_i),
\qquad\sum_i a_i r_i\le B
\tag{16}
\]

의 해는 B<Σa_iR_i일 때

\[
\boxed{r_i^*=\min\left(R_i,\frac{p_i}{\lambda a_i}\right)},
\quad
\sum_i a_i\min\left(R_i,\frac{p_i}{\lambda a_i}\right)=B.
\tag{17}
\]

B≥ΣaR이면 k_i=0이 해다. a_i=0이면 r_i=R_i로 둔다. active set이 모두 interior이면

\[
\lambda=\frac{\sum_i p_i}{B},\qquad
r_i^*=\frac{B p_i}{a_i\sum_jp_j}.
\tag{18}
\]

**증명.** objective의 Hessian은 diagonal `p_i/r_i²>0`이므로 strictly convex, constraint는 convex다. interior stationary equation은 `−p_i/r_i+λa_i=0`; upper cap에 부딪히면 r_i=R_i. B<ΣaR에서 λ>0이고 active residual budget equality가 성립한다. (17)의 왼쪽은 relevant domain에서 연속 단조 감소하여 λ를 정한다. KKT 충분조건으로 global optimum이다. □

**uniform 대비 정량 보장.** 같은 residual threshold `r_i=min(R_i,r_uni)`를 쓰는 모든 feasible 공통 정책은 (16)의 feasible point이므로 `F(r*)≤F(r_uni)`. 정수화 비용은

\[
W_{alloc}:=\sum_i c_i k_i < F(r^*)+\sum_i c_i
\tag{19}
\]

(각 비정수 위치에 대해서만 strict overhead; 전부 정수이면 W=F). allocator·weights·certificate의 추가 charged cost를 C_extra라 두면

\[
W_{uniform,scheduled}-F(r^*)-\sum_i c_i>C_{extra}
\tag{20}
\]

는 strict total-work reduction의 충분조건이다. 더 정확하게 각 ceil을 실제 계산해 `W_uniform−Σc_i k_i>C_extra`를 확인하면 된다. 정수 k별 feasibility는 마지막에 outward residual budget으로 확인한다. arbitrary floating `log`가 정확한 ceil을 준다고 가정하지 않는다.

**비교 대상의 범위.** 보장은 같은 (14)를 이용하는 backend의 **scheduled/certified iteration budget** 비교다. q_i bound가 보수적인 경우 adaptive uniform solver가 실제로 훨씬 빨리 끝날 수 있으므로, upper bound끼리 비교하여 measured speedup을 증명했다고 말하지 않는다. iteration count를 정확히 스케줄하여 실행하거나, residual 감소가 equality인 regime이면 실제 iteration work 비교도 된다. 가장 빠른 direct solver보다 낫다는 주장은 별도다.

### 6.1 (14)를 실제로 제공하는 backend

weighted operator B=DWD^-1가 symmetric positive definite이고 certified spectrum이 `[m_W,M_W]`, 0<m_W≤M_W라고 하자. Richardson relaxation `α=2/(m_W+M_W)`에서는

\[
r_{k+1}=(I-\alpha B)r_k,\qquad
q=(M_W-m_W)/(M_W+m_W)<1.
\tag{21}
\]

spectral theorem으로 (14), η=0이 따른다. `m_W=M_W`이면 q=0이며 exact arithmetic에서 한 Richardson step으로 끝난다. 이 경우는 (15)–(18)의 `log q` 공식을 호출하지 않고 one-step special case로 처리한다. finite precision에서는 그 한 step의 잔차 상계 ν를 확인하고 남은 budget에서 charge한다. 또는 필요하면 별도로 검증된 더 느슨한 `0<q<1` 상계로 일반 정리를 적용할 수 있다. 일반 q에 대해 finite precision에서 한 iteration의 residual perturbation norm이 ν 이하이면 `||r_{k+1}||≤q||r_k||+ν`이므로 η=ν/(1−q)로 (14)가 성립한다. 이 floor를 residual target 아래로 숨기면 안 된다. preconditioned polynomial cycle에 검증된 contraction bound가 있다면 한 cycle을 iteration으로 동일한 allocation을 쓸 수 있다. **GMRES observed history에서 q를 fit한 값은 검증된 (14)가 아니다.**

### 6.2 exact worked example: 68 → 36

m=4, `a=(4096,1,1,1)`, R_i=c_i=1, q_i=1/2, η_i=0, remaining B=1/16을 고정한다.

공통 threshold의 충분 최소 iteration은

\[
k_{uni}=\left\lceil\log_2\frac{4099}{1/16}\right\rceil=17,
\quad W_{uni}=4\cdot17=68.
\]

nonuniform optimum은

\[
r^*=\left(\frac1{262144},\frac1{64},\frac1{64},\frac1{64}\right),
\quad k=(18,6,6,6),\quad W=36.
\]

정확한 weighted sum은 `4096·2^-18+3·2^-6=1/16`이다. 모든 continuous optimal k가 정수이므로 이 예에서는 integer optimum이기도 하다. extra allocator/certificate cost가 iteration32회의 charged work보다 작으면 **총 산술 work도 엄밀히 감소**한다. 이 예는 현재 RODAS5P에서 측정한 결과가 아니다. 영향도 불균형이 있는 certified solves의 정확 구성 예이며 같은 backend의 best common threshold와 비교한다.

Wolfram exact evaluator와 별도 Python Fraction 계산으로 위 정수/분수값을 검산했다. 이는 수학 증명 외의 작은 검산이며 공개 scientific campaign 재실행이 아니다.

### 6.3 complexity 감소를 보장하는 family

m=2^r≥2, B=2^-b with integer b≥1, `a=(2^L,1,...,1)`, R=c=1, q=1/2, integer L≥r라 하자. uniform schedule과 optimal allocation의 exact work는

\[
W_{uni}=m(L+b+1),\qquad
W_{alloc}=L+m(r+b).
\tag{22}
\]

증명: `2^L<2^L+m−1<2^{L+1}`이고 (18)의 residual targets가 powers of two이므로 ceil이 위 값을 준다. 따라서 차이는 `(m−1)L+m(1−r)`. b 고정, L≫m log m이면 uniform은 Θ(mL), allocation은 Θ(L+m log m)이고 improvement factor는 m에 접근한다. interior 식의 allocator는 O(m), stage adjoint는 O(stage edges)이고 overhead가 이 절감보다 작다는 비용조건을 추가한다. bit-complexity model에서는 L비트 exponent/weight 처리비도 양쪽과 allocator에 charge한다.

이 family는 모든 ODE의 복잡도 하계 개선을 주장하지 않는다. current fixed eight-stage RODAS에 적용할 때 이득은 measured influence distribution과 available contraction backend에 따라 결정된다. 전역 m=N·s allocation을 하려면 미래 영향도 upper bounds를 사전에 확보해야 한다. 그렇지 않으면 step-local 또는 bounded-window theorem만 적용한다.

## 7. 코드 이식 가능한 구체 계약

새로운 production authority는 다음 세 구조를 기존 callback API에 얹는다. 이 문서에서 production 코드를 수정하지 않았다.

```rust
// Interface sketch, not existing APIs.
struct FrozenAccuracyTarget {
    // exact target identity: h, t, U-tableau, J/f_t model, metric, model epoch
}
struct StageTransferWitness {
    // G_i, Lipschitz tube bounds, nonnegative A, output weights w,
    // residual assembly/operator uncertainty; all outward upper bounds
}
struct GlobalAccuracyLedger {
    // verified truncation, exact-flow lognorm, metric transport,
    // accepted contamination and initial/time uncertainty
}
struct ResidualAllocation {
    // nonuniform rho_i, provable q_i (or polynomial-cycle contraction),
    // charged work, floor, validity domain and remaining output budget
}
```

**가장 작은 양의 이식 순서.**

1. fixed metric, autonomous polynomial/quadratic model, exact declared symmetric dissipative J 또는 declared block inverse에서 시작한다. G1 inverse witness·tube contract를 native stage caller에 연결한다. 일반 JVP-only authority를 새로 가정하지 않는다.
2. every accepted stage의 exact residual upper를 기록한다. floor/stall/fallback도 같은 ledger에 charge한다. 후보의 residual을 보고 이미 정한 η/B를 느슨하게 바꾸지 않는다.
3. scalar backward sweep (6)과 outward allocator를 구현한다. baseline은 같은 linear backend의 공통 residual budget. 추가 setup/certificate work를 explicit counter로 charge한다.
4. polynomial model의 h-jet/derivative enclosure로 G2를 구현한다. 낮은 차수 d_j까지 export한다. dense output sampling과 embedded error를 τ authority로 쓰지 않는다.
5. G3 ledger를 연결해 `CertifiedGlobalError {bound, metric, t, target}`를 반환한다. requested budget을 넘으면 축소/고정확도/fallback 등의 declared policy를 선택한다.
6. 별도 native holdout으로 domain validity, independent high-precision enclosure, bit/operation counters를 검증한다. 실제 elapsed-time promotion은 이론과 별도 gate다.

## 8. 실패 조건은 수학 정리의 적용 범위이지 결론의 회피가 아니다

이 정리는 local-to-global bound와 positive-work theorem을 실제로 증명했다. 구현 승격에는 다음 실재 값이 더 필요하다: current full-space G, actual model/tube binding, coefficient-aware τ, global flow stability, η floors, q contraction witness, measured/counted C_extra. 이 중 없는 값을 임의로 작게 설정해 얻은 숫자는 증명이 아니다.

보고 상태는 `derived`와 `numerically checked`다. production/global trajectory certificate 및 runtime speedup은 아직 `not implemented / not measured`다. 목표는 HOLD를 반복하는 것이 아니라, **어떤 최소 native objects가 채워지면 정리의 결론이 곧바로 적용되는지**를 위 식과 API 수준으로 고정하는 것이다.
