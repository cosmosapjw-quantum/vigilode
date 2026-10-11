# Homotopy 병렬화와 정확한 차원 축소: 조건부 양의 정리

작성: 2026-10-11. Source: `8ce9bda0d72d308facd615ac42ec137560fd68c6`.
범위: 이론 연구. `derived`; 새로운 native 실행·성능 측정·production 승격은 수행하지 않았다. 아래 정리들의 신규성은 주장하지 않는다.

## 0. 기존 결과와 이번 확장의 경계

확인한 source는 `block.rs`의 `nonlinear_remainder_snapshot`, `target_jacobian_matrix`, `homotopy.rs`의 `evaluate_partial_path`, `run_scheduled_homotopy_path`, 그리고 `outward_certificate.rs`의 causal/doubling 경로다. `MATHEMATICS_PORTING_KO.md` §7, Oct02 report §5–8과 Oct04 polynomial/parallel report를 읽었다. 이전 계산은 재실행하지 않았다.

다음은 이미 게시된 결과이므로 새 성과로 세지 않는다.

- 원래 λ=1 target에서의 endpoint certificate, candidate/authority 구분.
- strict-lower majorant의 유한 path sum, feasible-radius polynomial 및 Sturm 방식.
- action-first doubling과 diagonal component-blocked certificate.
- chart를 ODE가 아니라 stage residual에 적용하면 원래 root를 보존한다는 계약.
- 과거 q=2 비용 여유가 작고, 실제 client·dispatch·실패 비용을 포함해야 한다는 판단.

이번 확장은 (i) **exactly causal declared/reference stage target의 fold-free 구조 정리**, (ii) **독립 λ-window와 stage-corrector의 유한 오차·span 계약**, (iii) **affine invariant model에서 stage target을 그대로 보존하면서 실제 산술 복잡도를 낮추는 정리**, (iv) **실제 represented K coefficient leakage를 제거하지 않고 감싸는 near-causal 정리 H5**다. Homotopy는 모든 serial dependency를 없애는 장치가 아니다. 병렬 span 감소와 총 work 감소는 서로 다른 정리가 담당한다.

## 1. Source 수식과 exact causal reference의 구분

단계 수 s, 상태 차원 n, 동일한 frozen operator W, `D=I_s⊗W`를 둔다. C는 strict-lower stage block coupling이고, N_i(K)는 K_1,…,K_{i−1}에만 의존한다. `block.rs`에서 C는 h L⊗J, N은 현재 stage RHS에서 frozen affine 부분을 뺀 remainder다. 다음 식은 native K-form의 수식 형태다. 그러나 아래 C/N의 strict-lower 가정은 represented K coefficients에서 자동 성립하지 않으므로 다른 U-form의 계수와 섞거나 실제 native target의 성질이라고 바로 결론내리지 않는다.

\[
\eta=\theta+\lambda(1-\theta),\qquad
H_{\theta,\lambda}(K)=DK-b_0-\eta CK-\lambda hN(K).
\]

θ∈[0,1]을 한 경로에서 고정한다. λ∈[0,1]이며 λ=1의 target은 θ와 무관하게 원래 RODAS5P stage target이다. θ가 round마다 변하는 기존 scheduled engine은 이 고정 θ 곡선이 아니라 별도 nonstationary candidate generator다.

**독립 검토에서 수정한 source-binding 오류.** `coefficients.rs:189–193`은 generic `inverse(gamma_inv)` 뒤에 alpha/beta/l을 구성하고 `block.rs::stage_mix`는 모든 entries를 읽는다. 이미 게시된 `research/adversarial_reaudit_20261001_r3/homotopy/coefficient_structure.json`은 represented alpha와 l의 upper/diagonal leakage 및 alpha^8≠0, l^8≠0을 기록했다. 그 결과를 재사용하며 재실행하지 않았다. 따라서 **현재 native K target이 exact causal/no-fold라는 최초 초안의 blanket 주장은 철회**한다. H1/H2의 exact s-round 결론은 `EXACTLY_CAUSAL_REFERENCE_OR_DECLARED_TARGET`에 한정한다. Original U-form을 사용하려면 그 structural-zero contract를 실제로 확인해야 하며, K coefficients를 조용히 0으로 바꿔 동일 target이라 부르지 않는다. Actual target 적용은 H5가 담당한다.

`alpha`가 strict-lower이고 W가 변하지 않는다는 가정이 핵심이다. Mass matrix가 있어도 W가 가역이면 정리는 유지된다. Fully implicit stage, singular W, stage-dependent implicit diagonal N_i(K_i), 또는 별도 closure 방정식을 추가한 모델에는 그대로 적용하지 않는다.

여기서 W는 **실제 선형 operator**다. Closure에 frozen되어 있다는 사실만으로 finite-difference JVP callback이 방향에 선형인 matrix가 되지는 않는다. 그런 callback에는 source-bound exact/reference linear W와 callback approximation-error enclosure가 필요하다. 이 계약 없이 H1의 determinant 또는 H4의 invariant identity를 callback 자체의 정리로 사용하지 않는다.

## H1. Exactly causal reference homotopy는 정칙한 단일 graph다

**가정.** W가 가역이고 모든 N_i가 필요한 domain에서 C^k (k≥1)이며 이전 stage만 읽는다. 각 λ에서 아래 재귀로 얻는 stage state가 그 domain 안에 있다.

**결론.** 각 λ에는 유일한 K*(λ)가 존재하며 C^k 함수다. 모든 유한 target point에서

\[
\det D_KH_{\theta,\lambda}=\det(W)^s\ne0.
\]

따라서 zero-set은 λ 위의 하나의 graph이고, λ projection에는 critical point/fold가 없다. 독립 window에서 얻은 certified root는 중간 root를 순차 추적하지 않아도 같은 branch다. N_i가 모든 실수 입력에서 정의되는 다항식이면 각 K_i*(λ)도 다항식이다.

**증명.** 첫 식은 W K_1=b_{01}+λ hN_1로 K_1을 유일하게 정한다. i−1까지 고정되었을 때

\[
K_i=W^{-1}\!\left[b_{0i}+\eta\sum_{j<i}C_{ij}K_j
+\lambda h N_i(K_1,\ldots,K_{i-1})\right]
\]

는 K_i를 유일하게 정한다. 유한 귀납으로 존재·유일성이 따른다. C^k 합성으로 정칙성이 따르고, Jacobian은 대각 block이 모두 W인 lower triangular matrix이므로 determinant 식이 성립한다. Implicit function theorem의 국소 graph들은 이 유일성에 의해 하나의 전역 graph로 접합된다. 다항식이면 합성·곱·상수 선형변환이 다항식을 보존한다. □

이때 algebraic geometry의 실제 이득은 일반 Gröbner basis나 continuation discriminant를 계산하는 데 있지 않다. W^{-1}를 상수 coefficient field에 포함하면 ideal을 `K_i−P_i(λ)` 꼴의 triangular graph ideal로 차례로 소거할 수 있다. 즉 branch label 검사와 fold 회피를 **모델 계약으로 제거**할 수 있다. Polynomial degree d에 대한 단순 상계는 D_1≤1, D_i≤1+d max_{j<i}D_j (d≥1)이므로 d>1에서 D_i≤(d^i−1)/(d−1)다. 이 상계는 커질 수 있어 exact full-branch polynomial을 무조건 전개하는 algorithm은 권하지 않는다.

유한 실수 구간 [0,1]에서 globally polynomial 모델의 exact root는 유한하다. 이는 binary64 overflow가 없거나 interval bound가 유용하다는 말이 아니다. General callback이 유한 domain만 가지면 domain exit는 여전히 검사한다. 기존 global algebraic-closure의 “critical locus/infinity escape” 경고는 그대로 유효하지만, 이 **정확히 causal하다고 검증된 declared/reference target**에는 순차 branch 추적을 강제할 이유가 없다. 현재 represented K target에는 H5의 leakage/uniqueness 계약이 별도로 필요하다.

**정확한 작은 예.** H_1=K_1−a, H_2=K_2−b−λcK_1², H_3=K_3−d−λeK_2²이면

\[
K_1=a,\quad K_2=b+\lambda ca^2,
\quad K_3=d+\lambda eb^2+2\lambda^2ebca^2+\lambda^3ec^2a^4.
\]

Jacobian determinant는 1이다. λ-window들을 독립 계산해도 다른 root에 도착할 수 없다. 이 예는 새 native benchmark가 아니라 정리의 기호적 instance다.

## H2. Contraction보다 강한 causal error certificate와 유한 종료

\[
\Phi_\lambda(K)=D^{-1}[b_0+\eta CK+\lambda hN(K)]
\]

를 둔다. 각 stage norm을 고정하고 candidate c 주위 product tube `T=∏_i B(c_i,r_i)`를 잡는다. Tube에서 다음의 **비음수 strict-lower** matrix B가 검증되었다고 하자.

\[
\|\Phi_i(x)-\Phi_i(y)\|_i
\le\sum_{j<i}B_{ij}\|x_j-y_j\|_j.
\]

일례로 stage norm이 동일하고 G≥||W^{-1}||, N_i의 stage별 derivative upper bound가 L_ij이면 B_ij≥G(|η| ||C_ij||+|λh|L_ij)다. 가능한 경우 W^{-1}와 derivative의 합성 자체를 감싸 scalar G의 손실을 줄인다. 모든 operator/model identity와 uncertainty는 bound에 포함한다.

### H2a. Norm contraction 없이 root 포함

d_i≥||Φ_i(c)−c_i||, `d+B r≤r`이면 T에 유일한 exact root가 있고

\[
e(c)\le (I-B)^{-1}d=\sum_{j=0}^{s-1}B^j d.
\]

**증명.** H1의 causal 재귀를 사용한다. Stage 1의 error≤d_1≤r_1이다. 이전 exact stage들이 T에 포함되어 있으면 stage i error≤d_i+Σ_{j<i}B_ij e_j≤r_i다. 따라서 domain과 error inclusion을 함께 귀납 증명한다. `e≤d+B e`를 s번 치환하면 B^s=0에 의해 finite sum bound가 나온다. □

`||B||<1`은 필요 없다. 모든 eigenvalue가 0이라는 사실만으로 작은 bound를 얻는 것은 아니며, finite path weights가 크면 bound 역시 커진다. 계산은 새 dense `(sn)×(sn)` inverse가 아니라 이미 존재하는 causal majorant/action-first kernel의 확장 계약이다.

### H2b. Parallel Jacobi correction의 정확도와 round 수

각 round는 이전 K만 읽는다. `K^{m+1}=Φ(K^m)`이면

\[
e_m\le B^m e_0.
\]

따라서 exact arithmetic에서는 **s rounds 이내에 exact root**다. 더 정확히 stage i는 i번째 round 이후 exact다. 이것은 global contraction이나 작은 h 없이도 성립하되 callback이 필요한 점에서 정의되어야 한다.

실제 계산의 stage별 오차 bound를 ρ_m≥0라 하면

\[
e_m\le B^m e_0+
\sum_{j=0}^{m-1}B^{m-1-j}\rho_j.
\]

**증명.** 한 round에 Lipschitz inequality와 계산 오차를 적용하면 e_{m+1}≤B e_m+ρ_m다. 귀납으로 식이 나오며 exact arithmetic에서는 ρ=0, B^s=0이다. □

Tube에 대한 B를 사용하려면 exact root와 모든 iterates가 그 tube에 있어야 한다. 이는 H2a와 매-round enclosure, 또는 rounding까지 포함한 self-mapping `d+B r+ρ≤r`로 확인한다. e_0는 알려지지 않은 reference root에서 추정하지 않고 H2a로 감싼다.

Output `y_out=y+Σ_i b_i K_i`에 대해 같은 metric을 사용하면

\[
E_{\rm out}(m)\le |b|^T e_m+E_{\rm output\ rounding}.
\]

Embedded vector도 실제 별도 coefficient로 운반한다. λ=1에서 이 inequality가 budget을 만족하는 최초 m을 선택하면 사전에 최대 s를 약속하면서 유리한 regime에서는 m<s를 보증한다. 이 output contamination은 ODE truncation/global error certificate를 대체하지 않는다.

**양의 regime가 비어 있지 않다는 정량 예.** Weighted block maximum norm에서 ||B||≤1/4, initial error E_0≤32ε, round error ρ/(1−1/4)≤ε/2라면 m=3에서 e_3≤ε다. `(1/4)^3·32ε+ε/2=ε`이기 때문이다. s=8이면 8-stage serial target 대신 3-stage-batch waves로 같은 지정 stage error에 도달한다. 이는 조건을 만족하는 모델의 수학 보장이며 현재 repository client가 이 상계를 충족한다는 실행 결과는 아니다.

이 예의 ε는 stage error budget이다. Desired output budget τ에 적용하려면 `||b||_1 ε+E_output_rounding≤τ` (또는 더 타이트한 componentwise projection)를 먼저 충족시켜야 한다. ε를 ODE global tolerance로 직접 해석하지 않는다.

## H3. Common predictor atlas에서 independent windows와 엄밀한 비용 gate

H1 기반의 branch/finite-round 결론은 exact causal reference에 대한 것이다. 실제 all-entry K target에는 아래 H5의 uniform leakage/closure bound 또는 certified overlap으로 연결된 branch witness가 필요하다. H3b의 complex contraction 조건을 actual map에 직접 증명할 수 있으면 그 조건 자체가 disk 안의 유일성을 제공하지만, 이번에 그 native bound를 계산하지 않았다.

### H3a. Branch 인증을 병렬화하는 계약

필요한 parameter points λ_1,…,λ_M에 대해 common setup에서 predictor P(λ), 각 window의 tube, d(λ), B(λ)를 감싼다. 각 parameter point 또는 window에 대해 H2a의 closure를 검증하고, H2b에 따라 m_j round를 정한다. 이때 각 window는 이전 window의 corrected root를 입력으로 요구하지 않는다. H1 때문에 각각의 certificate가 유일한 동일 branch를 지정한다. Overlap은 approximation quality 관리에 유용하지만 root label을 결정하는 필수조건이 아니다.

일반 regular nonlinear closure에서는 independent atlas의 overlap/branch witness가 필요하다. 이 생략은 constant-W strictly causal target에만 해당한다. θ를 바꾸는 경우에도 같은 λ에서 H1은 성립하지만 λ<1 target은 바뀐다. λ=1에서의 공통 target identity를 별도로 고정한다.

한 endpoint만 필요하면 λ=1을 포함한 **한 window**만 실행하는 것이 우선이다. M개의 λ 값을 필요하지도 않은데 생성해 parallel speedup을 주장하지 않는다. 이미 source의 sequential continuation으로 얻은 predictors를 공짜 setup으로 계산해서도 안 된다.

### H3b. 공통 λ-jet predictor를 만드는 한 방법

θ=0에서 `K=K_0+λ Ψ(K)`, K_0=D^{-1}b_0, Ψ(K)=D^{-1}(CK+hN(K))다. N이 holomorphic이고 complex ball ||K−K_0||≤r에서

\[
\|\Psi(K_0)\|\le a,\quad
\|\Psi(x)-\Psi(y)\|\le b\|x-y\|,
\quad R(a+br)\le r,\quad Rb<1,\quad R>1
\]

이면 |λ|≤R에서 unique analytic branch가 있고 ||K(λ)−K_0||≤r다. Degree-p Taylor predictor에는

\[
\|K(\lambda)-P_p(\lambda)\|
\le \frac{r(|\lambda|/R)^{p+1}}{1-|\lambda|/R},\quad |\lambda|<R.
\]

특히 λ=1에서는 `r R^{-p}/(R−1)`다.

**증명.** λΨ는 closed ball을 자신으로 보내고 Lipschitz constant≤Rb<1이다. Uniform contraction의 fixed-point iterations는 holomorphic 함수들의 locally uniform limit이므로 branch가 holomorphic다. Cauchy coefficient estimate `||K_m||≤r R^{-m}`와 geometric tail sum으로 식이 나온다. Boundary regularity는 R보다 작은 contour에서 계산하고 극한을 취해 얻는다. □

Taylor coefficient는

\[
K_m=D^{-1}\!\left(CK_{m-1}
+h[\lambda^{m-1}]N\!\left(\sum_{j=0}^{m-1}K_j\lambda^j\right)\right)
\]

로 계산한다. 각 coefficient order 안의 s common-W solves는 병렬이며 order m들은 순차다. Directional/polynomial jet provider와 coefficient convolution 비용을 charge한다. 단순 JVP callback이 이 coefficient authority를 제공하지 않는다. Actual native floating jet에는 coefficient formation/solve/normalization roundoff도 더해야 한다. Polynomial root의 존재만으로 R>1 또는 유용한 r가 자동 확보되지는 않는다.

### H3c. 비용을 포함한 strict benefit theorem

다음은 지정된 sequential comparator와 같은 target·budget에서의 **counted-cost model**이다. Hardware time의 보편 정리는 아니다. p workers의 한 batch 비용은 stage task schedule의 최대 load, packing, synchronization을 포함해 실제 상계 L_p로 선언한다. 단순 work/p로 대신하지 않는다.

Single endpoint에 common setup A, m correction waves, final certificate C, 실패·폐기된 window/반경 비용 F, fallback 비용 R_f, dispatch D를 쓰면

\[
T_{\rm new}\le A+m L_p+C+F+R_f+D.
\]

동일 조건 sequential comparator의 확정 하계 T_seq,low에 대해

\[
A+mL_p+C+F+R_f+D<T_{\rm seq,low}
\]

를 검증하면 지정 cost model에서 strict latency improvement가 증명된다. Work는 별도로

\[
W_{\rm new}=W_A+\sum_{r,i}W_{ri}+W_C+W_F+W_{R_f}+W_D
\]

를 기록한다. Parallel method가 이 inequality를 만족해도 work가 작아지는 것은 아니다.

**비어 있지 않은 explicit sufficient regime.** 위 s=8,m=3 예에서 p≥8, task+barrier wave upper bound L_p≤t, 모든 추가비용 A+C+F+R_f+D≤2t, strongest applicable registered sequential arm이 같은 비싼 causal callback 8개를 순차 수행하여 T_seq,low≥8t이면 T_new≤5t<8t다. 즉 이 cost model에서 최소 3t의 양의 여유와 최대 5/8의 latency ratio를 보증한다. Sequential arm이 affine/direct simplification, caller reuse 등으로 8t보다 싸지면 이 예의 하계를 사용할 수 없다. 24 stage tasks 대 8 tasks이므로 이 예 자체는 work reduction이 아니다.

G개의 independent parameter groups를 실제 client가 요청하고 P≥G·p이면 각 group을 동시에 수행하여 correction span이 max_j(m_j L_{p,j})로 줄어든다. Setup과 certificate가 공통이면 한 번만, group별이면 모두 계산한다. Processor 수가 부족하면 explicit assignment의 최대 load로 바꾼다.

Theorem premises가 closure와 finite arithmetic을 모두 보장하면 실패 F=R_f=0인 범위를 선언할 수 있다. 그렇지 않으면 bounded retry schedule의 최악 비용을 넣는다. 실패 후 전체 sequential fallback을 실행하는 경로에는 비용이 늘 수 있으므로, 측정되지 않은 성공확률을 이용해 deterministic speed guarantee를 만들지 않는다. **Preflight가 양의 여유를 증명하지 못하면 기존 protected path를 선택**하는 것이 정확한 router다.

## H4. Affine invariant quotient에서는 stage target을 보존하며 arithmetic complexity가 감소한다

H1–H3는 주로 span을 줄인다. 다음은 총 산술 작업을 줄이는 별도 양의 정리다. Low-rank approximation이 아니라 **exact invariant model declaration**이 필요하다.

Full-column-rank U∈R^{n×r}, r<n, 고정 y_ref를 두고 affine manifold M={y_ref+Uz}를 고려한다. 필요한 domain/time tube에서

\[
f(t,y_{\rm ref}+Uz)=U g(t,z),\quad
MU=U M_r,\quad JU=U J_r,\quad f_t=U g_t,
\]

를 가정한다. J는 실제 step에 frozen된 operator이며, approximate J라면 invariance identity를 별도로 확인한다. W=M−hγJ가 가역이고 W_r=M_r−hγJ_r도 가역이다. (Full W의 가역성과 U full rank, WU=UW_r만으로 W_r 가역성도 따른다.) 현재 상태 y_n=y_ref+Uz_n이다.

**결론.** 같은 tableau, h, time, frozen data를 쓰는 full stage와 reduced stage는 모든 λ에 대해

\[
K_i^*(\lambda)=U k_i^*(\lambda)
\]

이며, output/embedded vector도 동일하게 lift된다. 따라서 reduced stage solve는 full RODAS5P target을 바꾸지 않는다. Exact arithmetic에서 full solver와 identical discrete step이다.

**증명.** 먼저 H1의 exactly causal target 또는 H5로 동일 tube의 root uniqueness가 인증된 actual target을 사용한다. H5를 사용하는 경우 **선택한 reduced root의 lift가 full certified tube 안에 있음**을 추가로 검증한다. WU=UW_r이므로 W^{-1}U=UW_r^{-1}. Base rhs는 U b_{0,r}다. Stage state y_n+Σα_ij U k_j=y_ref+U(z_n+Σα_ij k_j)이므로 RHS identity와 JU=UJ_r에 의해 N_full(Uk)=U N_r(k)가 성립한다. Coupling도 C_full(I_s⊗U)=(I_s⊗U)C_r다. 그러므로 H_full((I_s⊗U)k)=(I_s⊗U)H_r(k). Reduced root의 lift가 full root이고 H1 또는 H5의 인증된 유일성으로 동일하다. Actual coefficients에 upper/diagonal leakage가 있어도 intertwining 항등식 자체는 성립하지만, 유일성·의도한 root 선택은 별도 조건이다. Output은 선형 stage combination이므로 같은 관계다. □

### 오차와 physical scale

Full norm을 `||v||_s=||D_s v||_2/√n`, reduced norm을 `||e||_r=||D_r e||_2/√r`로 두면

\[
\|Ue\|_s\le
\sqrt{r/n}\,\|D_s U D_r^{-1}\|_2\,\|e\|_r.
\]

Verified lift gain과 actual lifting/assembly roundoff를 포함하여 full physical budget을 검사한다. Reduced tolerance를 full tolerance와 숫자만 같게 설정하면 안 된다. Nonlinear ψ(z)의 manifold chart는 RODAS가 일반적으로 coordinate-equivariant하지 않으므로 이 정리를 상속하지 않는다. 여기서는 affine ψ에 한정한다.

### 산술 복잡도와 엄격한 절감 조건

직접 reduced callback g와 J_r/M_r가 제공되어 full n-dimensional RHS/linearization을 내부에서 다시 계산하지 않는다고 가정한다. q개 step 동안 basis·identity 검증 비용 V, reduced per-step setup F_r, stage solve S_r, RHS G_r, physical lift L_{nr}, certificate C_r를 모두 포함하면

\[
W_{\rm red}\le V+q(F_r+sS_r+sG_r+L_{nr}+C_r).
\]

같은 조건 strongest applicable full comparator의 lower bound W_full,low에 대해 이 값이 작으면 **총 work 감소**가 보장된다. Dense generic comparator에서는 F_n=Θ(n³), S_n=Θ(n²), while F_r=Θ(r³), S_r=Θ(r²), L_nr=O(nr). Explicit invariant identity 한 번 검증이 O(n²r)이고 r와 s가 n에 무관하게 고정되며 reduced callback/certificate가 O(nr+r³)라면 한 step도 full Θ(n³) 대비 reduced O(n²)다. Symbolic model declaration을 epoch별 한 번 검증하여 V를 amortize하면 online work는 O(nr+r³)다. 이 asymptotic conclusion은 dense full factorization을 실제 필요로 하는 comparator에 대한 것이며 banded/sparse/이미 reduced인 최적 comparator와는 해당 실제 비용으로 비교한다.

**비어 있지 않은 모델군.** U=[I_r;L], y_ref=0, y=(z,w), define

\[
f(z,w)=\begin{pmatrix}g(z)\\ Lg(z)-A_\perp(w-Lz)\end{pmatrix}.
\]

w=Lz에서 f=Ug(z), exact J U=U Dg(z)다. A_perp는 임의의 stiff invertible transverse operator일 수 있다. Full model에는 stiff transverse modes가 실제 존재하지만 initial state가 exact manifold에 있으면 reduced computation이 동일한 full discrete step을 만든다. g를 직접 제공하면 전체 large transverse factorization을 제거한다. Near-manifold initial data를 이 manifold에 투영해 버리는 것은 허용하지 않는다. 그런 데이터는 transverse error dynamics와 별도 budget을 증명해야 한다.

대수기하적으로 affine constraints L_null(y−y_ref)=0의 ideal을 보존하는 정확한 quotient 계산이고, 미분기하적으로 invariant affine submanifold의 tangent dynamics다. 기술의 효용은 수학 명칭이 아니라 **n 차원의 factorization을 r 차원으로 교체하면서 원래 target equality를 증명한 것**에 있다.

## H5. 작은 noncausal leakage를 보존하는 near-causal rescue

이 정리는 represented alpha/l의 upper/diagonal entries를 지우지 않고 **실제 target**을 인증한다. Φ_actual의 actual defect를 d_i≥||Φ_actual,i(c)−c_i||라 두고, 정해진 convex product tube에서 그 derivative/Lipschitz majorant를

\[
A=B+E\ge0,\qquad B\text{ strict-lower},\quad E\ge0
\]

로 분해한다. E는 coefficient leakage에 의한 모든 영향을 감싼다. Nonlinear alpha leakage가 자기 stage와 이후 stage에 미치는 derivative도 포함하므로 coefficient 절댓값만 넣는 것으로 끝나지 않는다. Approximate JVP/model arithmetic uncertainty 역시 별도 defect/derivative budget에 포함한다.

\[
S=(I-B)^{-1}=\sum_{k=0}^{s-1}B^k,\quad T=SE,
\quad 0\le\|T\|\le q<1
\]

을 검증했다고 하자. Norm은 monotone absolute norm 또는 그 weighted version이다. 그러면

\[
V=(I-T)^{-1}S=\sum_{k=0}^{\infty}T^kS\ge0,
\qquad (I-A)^{-1}=V.
\]

**증명.** `(I−B)(I−SE)=I−B−E=I−A`다. ||T||<1이므로 Neumann inverse가 존재하고 비음수다. 역행렬의 곱 순서를 뒤집으면 주장한 식이 나온다. □

### H5a. 실제 root existence, uniqueness와 candidate error

δ>0를 골라 `r=V(d+δ1)`을 형식적으로 정의하고, 이 r의 verified upper enclosure에 대해 실제로

\[
d+A r\le r
\]

및 사용한 derivative tube의 self-consistency를 확인한다. Exact r은 `d+Ar=r−δ1`이라는 여유를 가진다. Outward upper enclosure를 대입하면 여유가 소실될 수 있으므로 마지막 부등식 재검사가 필수다.

Φ_actual은 이 closed product ball을 자신으로 보내므로 Brouwer theorem으로 root가 존재한다. 두 root의 stage distance vector e에는 `e≤Be+Ee`가 성립한다. 비음수 S를 곱하면 `e≤SEe=T e`, 따라서 ||e||≤q||e||이고 q<1이므로 e=0이다. Candidate error 또한

\[
e\le Vd,\qquad
\|e\|\le \frac{\|Sd\|}{1-q}
\]

로 감싸진다. 이것은 actual coefficient target의 certificate이며 projected causal target의 root를 몰래 정답으로 쓰지 않는다.

각 λ∈[0,1]에서 같은 tube에 대한 continuous C^1 actual map과 uniform closure/q<1을 증명하면 root가 그 tube에서 하나이며 derivative `I−DΦ_actual`도 가역이다. 실제로 null vector가 있으면 그 component norm에도 e≤Ae가 성립하여 e=0이 된다. Implicit function theorem과 유일성으로 λ=0에서 시작하는 동일 branch가 연결된다. 서로 다른 window의 tube를 사용할 경우 단순 geometric overlap만으로 충분하지 않고, certified shared root inclusion 또는 연결된 공통 tube witness가 필요하다.

### H5b. Reference candidate와 실제 target의 차이

Reference root c_ref가 실제 tube에 있고 `δ_ref≥|Φ_actual(c_ref)−c_ref|`라면

\[
\|K_{\rm actual}-c_{\rm ref}\|
\le\frac{\|S\delta_{\rm ref}\|}{1-q}.
\]

따라서 exact causal reference에서 빠른 candidate를 생성한 뒤, actual residual과 leakage amplification을 계산하여 원래 target에 admit할 수 있다. δ_ref를 계산할 때 rounding뿐 아니라 zeroed coefficient와 full coefficient의 차이를 포함한다. 작은 coefficient leakage가 큰 resolvent/derivative 때문에 커질 수 있으므로 “10^{-16}이니 무시”라는 판단은 금지한다.

### H5c. 양의 근사 복잡도 보장

Vd를 근사하는 m차 positive expansion `v_m=Σ_{j=0}^m T^j S d`의 누락 tail은

\[
\|Vd-v_m\|\le
\frac{q^{m+1}}{1-q}\|Sd\|.
\]

0<q<1, ε>0, ||Sd||>0에서 원하는 bound budget ε를 만족하는 충분 차수는

\[
m\ge\max\left(0,\left\lceil
\frac{\log(\|Sd\|/[(1-q)\epsilon])}{\log(1/q)}
\right\rceil-1\right)
\]

이며 q=0 또는 ||Sd||=0이면 tail은 즉시 0이다. Nilpotent B의 finite action은 기존 causal kernel을 재사용하고, q≪1인 leakage correction은 작은 m만 요구한다. 이 표기는 exact-real positive-action complexity이며 native outward operations와 stopping upper bound를 추가해야 한다. Dense full inverse와 비교한 strict cost reduction은 이 실제 action·E 구성·q 검증 비용까지 포함하여 판단한다.

Actual parallel Jacobi iteration에는 `e_m≤A^m e_0+ΣA^{m−1−j}ρ_j`가 적용된다. A는 일반적으로 nilpotent가 아니므로 **s-round exact termination 주장은 철회**한다. Reference candidate에 한 번의 actual endpoint certificate를 붙이는 경로는 이 한계를 우회할 수 있다.

Actual target의 positive convergence rate도 얻을 수 있다. `v=V1=(I−A)^{-1}1>0`라 두면 `Av=v−1≥0`이므로 v_i≥1이다. Weighted maximum norm `||x||_v=max_i |x_i|/v_i`에서

\[
\alpha=\max_i(1-1/v_i)\in[0,1),\qquad
\|A\|_v=\alpha.
\]

따라서 actual same-tube iteration은 `||e_m||_v≤α^m||e_0||_v+Σ_{j<m}α^{m−1−j}||ρ_j||_v`를 만족한다. Constant per-round error upper bound ρ_v에는 floor ρ_v/(1−α)가 따른다. 이 식은 finite termination 대신 actual leakage target의 geometric convergence를 보증한다. Native에서는 v의 근삿값을 그대로 쓰지 말고 positive v_bar와 `A v_bar≤α_bar v_bar`, α_bar<1을 outward로 검사하면 동일 논리가 적용된다. q가 작아도 이 particular weighted bound의 α가 작다는 보장은 없으므로 실제 wave 수를 별도로 계산한다.

이번 이론 루프에서 actual represented K coefficients에 대한 B/E/q 또는 closure를 계산하지 않았다. 따라서 H5는 명시한 native source-binding 문제를 해결할 수 있는 구체적인 조건부 개발 정리이며, 현재 native promotion 결과가 아니다.

## 5. 구체적인 code 이식 순서

이번 파일은 설계이며 아래 API는 현재 존재한다고 주장하지 않는다.

1. `StructuredBlockSystem`에서 coefficient identity와 common-W identity를 immutable contract로 expose한다. `ExactlyCausalReference`와 실제 all-entry represented target의 `NearCausalBound {B,E}`를 구분한다. Generic inverse로 얻은 작은 upper/diagonal entries를 0으로 투영하지 않는다. Generic callback 전체의 smoothness/domain과 model binding은 별도 provider가 담당한다. 기존 validated θ/λ type을 사용한다.
2. 기존 `causal_majorant.rs` / `outward_certificate.rs`에 B, d, output weights, current tube의 source-bound view를 연결한다. 새 `(sn)²` dense certificate를 다시 만들지 않는다. 기존 action-first / diagonal blocked 방법은 reuse한다.
3. `ParallelExecution`과 `problem.rs::BatchRhsFn`으로 **frozen-old-iterate** Jacobi waves를 실행한다. In-place asynchronous stage updates는 H2의 round ledger와 다른 algorithm이다. Candidate type에 accepted flag를 두지 않는다.
4. `HomotopyAtlasPlan`은 common predictor setup, all failed preflights, degree/order, per-window r/B/d, worker assignment, max wave count, current target identity를 보유한다. Endpoint admission은 기존 original-target transactional seam에서 한다.
5. Exact invariant declaration `AffineInvariantModel`은 U/y_ref/epoch, direct reduced model, full/reduced operator relation, physical lift gain을 포함한다. Numeric SVD의 작은 singular value를 exact identity로 취급하지 않는다. Production 가속 후보로는 실제 r≪n client와 이 선언을 먼저 연결하는 것이 H3의 speculative windows보다 work 감소 근거가 강하다.

### 필요한 prospective tests (이번에 미실행)

- Triangular polynomial target의 independent exact root, Jacobian determinant, λ-window endpoint equivalence; diagonal self-coupling을 넣으면 contract reject.
- B의 norm≥1이지만 strict-lower인 사례와 B^s=0, positive roundoff convolution, tube exit/overflow reject.
- Same-round frozen reads와 workers 1/2/4/8의 결과/증거 계약; thread pool setup·barrier·failed windows를 모두 ledger에 기록.
- Full/reduced exact affine model의 same target equality, non-invariant J·mass·ft·stale basis/epoch·off-manifold input rejects.
- Full physical metric에서 lift enclosure, roundoff, tiny scales 및 large basis norm 검증.
- 모든 새로운 performance campaign은 실제 client와 full-cost gate를 새로 등록한다. 과거 R-NEXT-06 및 PP07 campaign은 다시 실행하지 않는다.

## 6. 판단

H1–H5는 조건을 명시한 수학적 양의 보장이다. H1은 exactly causal reference의 순차 branch-tracking requirement를 없애고, H2/H3은 해당 조건과 출력 예산·전체 overhead 하에서 m<s wave latency improvement를 보증한다. 실제 represented K target에는 H5의 leakage gate가 추가되며 이번에는 actual q나 closure를 계산하지 않았다. H4는 exact invariant client에서 n→r 축소로 총 산술 complexity를 낮춘다. 현재 native 경로가 이 새로운 모든 조건·cost bound를 구현했다는 주장은 하지 않는다. Theory를 production PROMOTE로 직접 바꾸지 않고, 조건을 증명 가능한 model contract와 현재 target certificate에 연결하는 다음 단계가 남는다.

새로운 `homotopy_exact_check.py`를 한 번 실행했다. 결과 `HOMOTOPY_EXACT_CHECK.json`은 (i) 고정 rational coefficients에서 모든 λ에 대한 cubic branch의 exact polynomial identity, (ii) 8-stage noncontracting nilpotence와 3-round positive error majorant, (iii) 새로운 3D/1D nonlinear invariant model의 4-stage target에서 θ=1/3, λ=0,1/2,1의 36개 component equalities를 exact Fraction으로 확인한다. 3-round 실제 majorant는 127/128≤1이다. 이는 새 작은 이론 진단이며 일반 정리의 증명·native RODAS5P 실행·timing을 대신하지 않는다. 기존 게시 실험의 재실행은 0회다.
