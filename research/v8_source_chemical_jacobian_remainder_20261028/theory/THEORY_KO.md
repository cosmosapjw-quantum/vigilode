# 실제 source chemical Jacobian의 전체 entry range와 quadratic remainder

## 1. 고정 단위와 증명 범위

이 문서는 UNIT_CONTRACT.md의 **V8_SOURCE_CHEMICAL_JACOBIAN_REMAINDER_20261028**을 정식화한 저자 이론이다. 물리 source는 BASS commit **cc602e209e7deeb511ab2a1f3b2268d55206c43c**, binding **8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16**이다. 차원은 reduced30 / physical33 / query31이고, 실제 chemical 좌표는 reduced30의 **23..28**이다.

목표는 실제 source의 upward/downward transition topology와 electron feedback를 사용하여 다음을 구성하는 것이다.

1. 실제 6변수 chemical polynomial과 정확한 Jacobian.
2. 독립된 12-rate interval box와 전체 physical chemical simplex의 곱 위에서 36개 Jacobian entry의 정확한 min/max 및 **72개 attaining witnesses**.
3. 고정 coefficients의 정확한 quadratic remainder와 Hessian.
4. 기존 native metric과 next7 matrix certificate를 재사용한 remainder upper.

이 결과는 whole-model Jacobian 또는 실제 해 enclosure가 아니다. Source 식은 선언된 represented coefficients에 real arithmetic을 적용한 수학적 식이다. 실제 callback의 floating-point 연산 오차와 coefficient preparation error는 별도 입력이다. Actual time/geometry 전구간에 대한 effective rate enclosure의 진실성 역시 별도다.

이 작성 단계에서 code를 실행하거나 과학 모듈을 import하지 않았다. RHS/Jacobian/ODE/NCP/native norm/lift/build 및 CAS 호출은 0이다. 기존 next7과 이전 모든 단위는 CLOSED를 유지한다. 중앙 관측은 **28/88**, 이 문서 작성 자체의 증가량은 0이다. 백업·게시 상태와 독립적인 Git 복구는 부모 기록을 따른다.

## 2. 실제 읽은 source와 일차 문헌

Actual source chain은 FrozenClient → AffineSystem → CompactSystem → CoupledSystem → AtomicNetwork·Geometry다. 다음 원문을 전체 읽고 Git blob을 확인했다.

| source | 핵심 내용 | 불변 blob |
|---|---|---|
| [atomic_network.py](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/research/rei_hheli_compact_pilot_20261022/vendor/rec_hheli/code/atomic_network.py) | population-dependent electrons와 analytic abundance Jacobian | e3a0603176304f64b953c0b2faf898d6b75eceb5 |
| [coupled.py](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/research/rei_hheli_compact_pilot_20261022/vendor/rec_hheli/code/coupled.py) | proper-time chemistry를 H로 나누는 clock, geometry coupling | 7d1a346081528f7f9449fdcb1032cf84a9aabb44 |
| [compact_system.py](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/research/rei_hheli_compact_pilot_20261022/code/compact_system.py) | isotope-fraction equality 및 full62→compact33 restriction | 4769767f1fb6a13856c86e0c36484a95282be570 |
| [affine_system.py](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/research/sandbox_affine_chemistry_20261022/code/affine_system.py) | 보존 total에 의한 neutral elimination과 기존 LIFT | baa8272d507375cdbcb11ec89af1a6282dca6b0b |
| [atomic_model.json](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/research/rei_hheli_compact_pilot_20261022/vendor/rec_hheli/data/atomic_model.json) | source inventory, isotope equality, n/T=0 조건 | 73285c1585f8edca92487effe95e68448f3163b8 |

Full raw와 추가 geometry/tangent source는 intake/actual_source_next8에 보존되어 있다. Source identity 확인과 과학적 interval enclosure는 같은 주장이 아니다.

Vertex 원리는 기존 수학을 재사용한다. Blanchini, Colaneri, Giordano, Zorzan, *Vertex results for the robust analysis of uncertain biochemical systems*, Journal of Mathematical Biology 85:35 (2022), DOI [10.1007/s00285-022-01799-z](https://doi.org/10.1007/s00285-022-01799-z)의 실제 회수된 서론과 §2–§2.3 시작 부분을 읽었다. 논문은 parameter vertex 원리와 stronger “totally multiaffine” matrix 조건을 구분한다. 본 단위는 scalar entry의 별도 affinity만 직접 증명한다. 모든 minors, determinant, eigenvalue 또는 spectral stability가 vertices에서 결정된다는 더 강한 명제를 가져오지 않는다. [읽은 일차 원문](https://giuliagiordano.dii.unitn.it/docs/papers/Journal/J055_2022_JoMB.pdf)

새 VigilODE branch **research/integrated-polyalgorithm-20261028**, HEAD **af7f3e9f55d652554042d2386bf07abb7d857d75**의 변경 9개 파일을 모두 읽었다. 그 [algebra_contract/THEORY.md](https://github.com/cosmosapjw-quantum/vigilode/blob/af7f3e9f55d652554042d2386bf07abb7d857d75/research/integrated_polyalgorithm_20261028/algebra_contract/THEORY.md)의 Theorem B, B6는 model-specific near-affine remainder를 요구한다. 아래 정확한 quadratic remainder는 해당 의무의 실제 source chemical component를 제공한다. 그 branch의 preregistration이나 prospective CHECK.py를 실행 결과로 취급하지 않는다.

## 3. 좌표, source weights, clocks

### 3.1 Chemical 좌표와 physical simplex

\[
z=(h,u,v,\ell_1,\ell_2,\ell_3)
=(\mathrm{HII},\mathrm{HeII},\mathrm{HeIII},
\mathrm{LiII},\mathrm{LiIII},\mathrm{LiIV})
\]

로 쓰자. 기존 affine source의 represented totals는

\[
T_H=\mathrm{0x1.fffffffffffffp{-}1}=1-2^{-53},\qquad
T_{\rm He}=T_{\rm Li}=1.
\tag{3.1}
\]

H total을 1로 교체하지 않는다. 실제 9-state fractions는

\[
(T_H-h,h;\ T_{\rm He}-u-v,u,v;\
T_{\rm Li}-\ell_1-\ell_2-\ell_3,\ell_1,\ell_2,\ell_3).
\tag{3.2}
\]

이는 기존 LIFT와 source neutral substitutions의 수학적 표시이며 새로운 native lift evaluator가 아니다.

Domain은

\[
\mathcal S=\mathcal S_H\times\mathcal S_{\rm He}\times\mathcal S_{\rm Li},
\]

\[
\mathcal S_H=[0,T_H],\quad
\mathcal S_{\rm He}=\{(u,v)\ge0:u+v\le T_{\rm He}\},
\]

\[
\mathcal S_{\rm Li}=\{\ell\ge0:\ell_1+\ell_2+\ell_3\le T_{\rm Li}\}.
\tag{3.3}
\]

각 block의 vertices는 2,3,4개이므로 \(\mathcal S\)는 정확히 24개의 product vertices를 갖는다. 각 점은 block별 neutral/charged fractions를 total로 나눈 barycentric weights의 곱으로 이 vertices의 convex combination으로 표현된다. 경계에서 일부 weight가 0이어도 이 표현은 그대로 유효하다.

### 3.2 Electron row와 isotope equality

Actual compact source는 H1/D, He3/He4, Li6/Li7의 charge fractions를 같게 두며 n/T inventory는 0이다. Source isotope scales를 \(s_A\)라 하고

\[
w_H=s_{H1}+s_D,\quad
w_{\rm He}=s_{He3}+s_{He4},\quad
w_{\rm Li}=s_{Li6}+s_{Li7}
\]

로 두면 electron fraction은

\[
E(z)=e^\mathsf Tz,\qquad
e=(w_H,w_{\rm He},2w_{\rm He},w_{\rm Li},2w_{\rm Li},3w_{\rm Li}).
\tag{3.4}
\]

Neutral charge는 0이므로 totals에서 추가 constant가 나오지 않는다. \(w_H,w_{\rm He},w_{\rm Li}>0\)는 exact frozen inputs이며 그 provenance를 기록한다. 실제 source는 isotope electronic equality를 명시적으로 선택한 finite model이다. 다른 isotope model, REI25, 다른 binding에는 이 식을 자동으로 이전하지 않는다.

### 3.3 Source-reconstructed RN inventory의 정확한 의미

원래 owner-prepared scale array와 원 CLIENT_SOURCE_MANIFEST 전체는 현재 회수되지 않았다. 별도 inventory witness가 source의 finite scalar preparation을 재구성하는 경우 다음 계산을 가정·기록한다.

1. Source numeric literals를 binary64 round-to-nearest, ties-to-even으로 읽어 represented ratio \(r_A\)를 얻는다.
2. Integer mass \(A_A\)와 ratio의 각 product \(p_A=\operatorname{RN}_{64}(A_A r_A)\)를 얻는다.
3. Source fsum의 의미를 exact sum of represented products의 한 번의 RN으로 고정하여 \(N=\operatorname{RN}_{64}(\sum_Ap_A)\)를 얻는다.
4. Positive isotope inventory를 \(s_A=\operatorname{RN}_{64}(r_A/N)\)로 만들고 모든 represented hex를 보존한다. Source의 zero-inventory fallback도 별도로 표시한다.
5. 위 represented \(s_A\)를 exact rational로 해석한 뒤 isotope sums와 integer charge factors로 \(e\)를 만든다. 여기서 exact real sums와 실제 callback dot-product roundoff는 다르다.

이 결과는 **SOURCE_RECONSTRUCTED_UNDER_RN**이다. 실제 owner array를 관측했다거나 source environment의 fsum·rounding behavior를 새로 입증했다는 뜻이 아니다. Source identity와 이 rounding semantics가 맞는다는 조건에서만 owner initialization의 represented coefficients와 연결된다. Synthetic weights는 diagnostic control로만 표시한다.

### 3.4 Effective rate intervals와 clock

Source의 독립변수는 \(x=\ln(a/a_{\rm ref})\), \(dx/dt=H\)다. Proper photo rate \(\Gamma\)와 recombination coefficient \(\alpha\)에서

\[
\gamma=\Gamma/H,\qquad \kappa=n_b\alpha/H
\tag{3.5}
\]

를 정의한다. 따라서 아래 식은 source clock의 derivative다. \(E(z)=n_e/n_b\)는 무차원이고 \(\gamma,\kappa\)는 inverse source-clock 단위를 갖는다.

Rate 순서는 계약대로 photo H0, He0, He1, Li0, Li1, Li2와 recombination H1, He1, He2, Li1, Li2, Li3이다. 모두 nonnegative rational closed intervals를 갖는다. 집합을 \(\mathcal R=\prod_{a=1}^{12}[\underline r_a,\overline r_a]\)라 쓴다.

이 independent box는 공통 H·T·UV에서 생기는 실제 correlation을 잃을 수 있다. 실제 clock cell과 geometry corridor에서 rates가 \(\mathcal R\) 안에 든다는 전제는 따로 필요하다. 점별 analytic Jacobian이나 rate positivity 검사만으로 이 전제가 충족되지 않는다.

## 4. 실제 source의 6변수 polynomial

Photo rates를 \(p_H,p_{H\!e0},p_{H\!e1},p_{L0},p_{L1},p_{L2}\), recombination rates를 \(r_H,r_{H\!e1},r_{H\!e2},r_{L1},r_{L2},r_{L3}\)라 표기한다. 여기서 \(r\)는 (3.5)의 effective \(\kappa\)이며 물리초 단위의 원 \(\alpha\)와 혼동하지 않는다.

Source의 각 upward flux는 \(p_q y_q\), downward flux는 \(E(z)r_q y_q\)다. 식 (3.2)를 대입하면

\[
F_H=p_H(T_H-h)-E r_H h,
\]

\[
F_{\rm He1}=p_{H\!e0}(T_{\rm He}-u-v)-p_{H\!e1}u
+E(-r_{H\!e1}u+r_{H\!e2}v),
\]

\[
F_{\rm He2}=p_{H\!e1}u-E r_{H\!e2}v,
\]

\[
F_{\rm Li1}=p_{L0}(T_{\rm Li}-\ell_1-\ell_2-\ell_3)
-p_{L1}\ell_1+E(-r_{L1}\ell_1+r_{L2}\ell_2),
\]

\[
F_{\rm Li2}=p_{L1}\ell_1-p_{L2}\ell_2
+E(-r_{L2}\ell_2+r_{L3}\ell_3),
\]

\[
F_{\rm Li3}=p_{L2}\ell_2-E r_{L3}\ell_3.
\tag{4.1}
\]

따라서

\[
\boxed{F(z)=b+Pz+(e^\mathsf Tz)Qz}
\tag{4.2}
\]

이며

\[
b=(p_HT_H,\ p_{H\!e0}T_{\rm He},0,\
p_{L0}T_{\rm Li},0,0)^\mathsf T,
\]

\[
P=\operatorname{diag}\!\left(
[-p_H],\
\begin{pmatrix}-p_{H\!e0}-p_{H\!e1}&-p_{H\!e0}\\p_{H\!e1}&0\end{pmatrix},\
\begin{pmatrix}
-p_{L0}-p_{L1}&-p_{L0}&-p_{L0}\\
p_{L1}&-p_{L2}&0\\
0&p_{L2}&0
\end{pmatrix}\right),
\tag{4.3}
\]

\[
Q=\operatorname{diag}\!\left(
[-r_H],\
\begin{pmatrix}-r_{H\!e1}&r_{H\!e2}\\0&-r_{H\!e2}\end{pmatrix},\
\begin{pmatrix}
-r_{L1}&r_{L2}&0\\
0&-r_{L2}&r_{L3}\\
0&0&-r_{L3}
\end{pmatrix}\right).
\tag{4.4}
\]

여기서 block-diag 표기는 block 사이 원소가 0인 이 coefficient matrices의 정확한 구조를 뜻한다. Actual full Jacobian이 block diagonal이라는 뜻은 아니다.

**Source restriction의 정당화.** AtomicNetwork의 real-arithmetic abundance 식은 \(f_Y=(G_0+D)Y+n_b(q^\mathsf TY)G_{\rm ne}Y\)다. Isotope electronic equality에서 각 isotope의 \(Y\)는 고정 inventory scale과 공통 charge fraction의 곱이다. 같은 isotope의 transition matrix 양변에서 그 scale이 상쇄된다. Electron density에는 모든 isotope scale이 (3.4)로 남는다. n/T=0 restricted subspace에서 source decay flux는 0이다. CoupledSystem이 H로 나누면 (3.5)의 rates가 되고, 실제 neutral elimination을 수행하면 (4.1)이다. 이 과정에서 isotope inventory, electron source 또는 neutral total을 다시 fit하지 않는다.

**보존과 boundary control.** 9-state flux 표현에서 한 adjacent transition은 source species에 \(-f\), destination에 \(+f\)를 더하므로 각 elemental sum derivative는 정확히 0이다. Nonnegative rates와 \(E(z)\ge0\)에서 어떤 population이 0이면 해당 species의 outflow는 0이고 inflow는 비음수다. Neutral face에서도 neutral derivative가 비음수이므로 charged-sum derivative는 0 이하이다. 이는 이 conditional chemical system의 정확한 boundary direction 성질이며 full physical ODE의 domain/continuation 인증을 대신하지 않는다.

## 5. Feedback Jacobian, Hessian, 정확한 remainder

**정리 5.1.** Frozen coefficients에서

\[
\boxed{J(z)=P+(e^\mathsf Tz)Q+(Qz)e^\mathsf T.}
\tag{5.1}
\]

또한 모든 signed increments \(d,h\in\mathbb R^6\)에 대해

\[
\boxed{D^2F[d,h]=(e^\mathsf Td)Qh+(e^\mathsf Th)Qd,}
\tag{5.2}
\]

\[
\boxed{F(z+d)-F(z)-J(z)d=(e^\mathsf Td)Qd.}
\tag{5.3}
\]

**증명.** \(E(z)Qz\)를 direction \(d\)로 미분하면 \((e^\mathsf Td)Qz+(e^\mathsf Tz)Qd\)다. 첫 항의 matrix는 \((Qz)e^\mathsf T\)이므로 (5.1)이다. 이 expression을 다시 \(h\)로 미분하면 (5.2)가 된다. 직접 전개

\[
(e^\mathsf T(z+d))Q(z+d)
=(e^\mathsf Tz)Qz+(e^\mathsf Td)Qz
+(e^\mathsf Tz)Qd+(e^\mathsf Td)Qd
\]

에서 constant와 linear terms를 빼면 (5.3)이다. \(\square\)

마지막 feedback term을 뺀 \(P+EQ\)는 fixed-electron derivative이지 실제 source derivative가 아니다. \(Q\)가 각 fixed-\(E\) transition generator에서 왔더라도 full \(J\)가 Metzler이거나 기존 l1 contractive 구조를 가진다고 가정할 수 없다.

이 polynomial identity는 signed increments에도 유효하다. 물리적 state claim에는 \(z,z+d\) 및 필요한 연결 선분의 domain membership이 별도로 필요하다. Polynomial continuation이 전체 physical source domain을 확장하는 허가가 아니다.

특히 \(R_z(d)=(e^\mathsf Td)Qd\)는 \(z\)에 의존하지 않으며

\[
R_z(2d)=4R_z(d),\qquad
R_z(d+h)-R_z(d)-R_z(h)=D^2F[d,h].
\tag{5.4}
\]

Source coefficients가 바뀌면 다른 polynomial이므로 이 동일한 remainder identity의 operands를 섞지 않는다.

## 6. 36 entry의 정확한 범위와 72 witnesses

\(j_{ij}(z,r)=J_{ij}(z;r)\)라 하자. 식 (5.1)에서 frozen rates \(r\)에 대해 \(j_{ij}\)는 **전체 state vector \(z\)**의 affine 함수다. Frozen state에 대해서는 \(P,Q\)가 rates의 linear function이므로 \(j_{ij}\)도 rates의 linear function이다. State별 quadratic RHS와 구별해야 하는 핵심 구조다.

**정리 6.1 — product-family entry extrema.** \(\mathcal V\)를 (3.3)의 24 vertices라 하자. 각 \(v\in\mathcal V\)에서

\[
j_{ij}(v,r)=c_{ij,0}(v)+\sum_{a=1}^{12}c_{ij,a}(v)r_a
\tag{6.1}
\]

로 쓴다. 현재 homogeneous rate parameterization에서는 \(c_{ij,0}=0\)이다. 다음을 정의한다.

\[
m_{ij}(v)=c_{ij,0}(v)+
\sum_a
\begin{cases}
c_{ij,a}(v)\underline r_a,&c_{ij,a}(v)\ge0,\\
c_{ij,a}(v)\overline r_a,&c_{ij,a}(v)<0,
\end{cases}
\]

\[
M_{ij}(v)=c_{ij,0}(v)+
\sum_a
\begin{cases}
c_{ij,a}(v)\overline r_a,&c_{ij,a}(v)\ge0,\\
c_{ij,a}(v)\underline r_a,&c_{ij,a}(v)<0.
\end{cases}
\tag{6.2}
\]

그러면

\[
\boxed{\min_{\mathcal S\times\mathcal R}J_{ij}
=\min_{v\in\mathcal V}m_{ij}(v),\qquad
\max_{\mathcal S\times\mathcal R}J_{ij}
=\max_{v\in\mathcal V}M_{ij}(v).}
\tag{6.3}
\]

각 endpoint는 선택된 state vertex와 (6.2)의 rate endpoints에서 실제로 attained된다.

**증명.** \(z=\sum_{v\in\mathcal V}\lambda_vv\), \(\lambda_v\ge0\), \(\sum_v\lambda_v=1\)로 쓰면 state affinity 때문에

\[
j_{ij}(z,r)=\sum_v\lambda_vj_{ij}(v,r).
\]

따라서 모든 \((z,r)\)의 값은 vertex별 최소의 최소와 최대의 최대 사이에 있다. 각 fixed \(v\)의 rate-linear expression은 독립 coordinate마다 sign에 맞는 endpoint를 선택하면 정확한 min/max를 갖는다. 마지막으로 그 값의 min/max를 주는 vertex가 finite set에 존재하므로 해당 pair가 전역 endpoint를 attained한다. \(\square\)

Zero coefficient의 rate endpoint 선택은 어느 쪽이어도 좋다. Deterministic serialization에서는 명시한 tie rule을 사용한다. Zero-width rate intervals, zero transitions 및 zero entry도 endpoint witnesses를 생략하지 않는다.

각 36개 entry에 min/max 두 개를 보존하므로 witness records는 **72개**다. 각 record는 최소한 entry index, lower/upper role, 정확한 value, state vertex, 12개 exact rate values, weights/totals/source identity, 실제 domain membership 및 독립 expression에서의 equality를 포함한다.

일반적인 모든 \(2^{12}\) rate corners를 각 entry마다 다 계산할 필요는 없다. 24 state vertices에서 각 entry의 12개 rate coefficient를 사용하면 된다. Resource cap 때문에 일부 계산을 생략한 결과를 exact range로 표시해서는 안 된다.

### 6.2 범위 exactness와 matrix-family correlation

여기서 exactness는 **declared \(\mathcal S\times\mathcal R\)의 각 scalar entry projection**에 관한 것이다. 모든 lower 또는 upper entries를 동시에 달성하는 공통 state/rate pair가 존재한다고 주장하지 않는다. 이 ranges를 center/radius matrix box로 포장하면 추가적인 entry correlation을 잃을 수 있다.

마찬가지로 실제 source의 공통 H·T·UV·density가 강제하는 rate correlations는 independent \(\mathcal R\)보다 좁은 subset을 이룰 수 있다. Actual family가 \(\mathcal R\)에 포함된다는 별도 증거가 있을 때만 (6.3)을 실제 family의 sufficient enclosure로 사용할 수 있다.

### 6.3 RHS 자체에는 같은 vertex 결론을 적용하지 않음

Actual retained topology 안에서도 quadratic RHS의 극값은 interior에서 생길 수 있다. 이를 보이는 diagnostic family를 다음처럼 고정한다: photo rates와 Li1 이외의 recombination rates는 0, \(r_{L1}=1\), \(w_H=w_{\rm He}=1/100,\ w_{\rm Li}=1\). H는 \(h=T_H\), He는 \(u=0,v=1\)로 두어

\[
c=(T_H+2)/100,\qquad 0<c<1
\]

의 electron contribution을 만든다. Li2를 0으로 두고 Li1+Li3=1인 face에서

\[
F_{\rm Li1}=-\ell_1(c+3-2\ell_1).
\]

그 최소는 \(\ell_1=(c+3)/4,\ \ell_3=(1-c)/4\)에서

\[
-(c+3)^2/8
\]

이다. 모든 product vertices에서의 최소는 \(-(c+1)\)이며 차이의 크기는 \((1-c)^2/8>0\)이다. 다른 H/He choices는 electron contribution을 더 크게 만들지 못하고, fixed Li1에서 남은 charge를 Li3에 두는 것이 electron contribution을 최대로 하므로 이 interior 값은 전체 domain에서도 최소다.

이는 analytic diagnostic control이며 이번 작성 단계에서 실행한 데이터가 아니다. 이 예는 vertex 증명의 실제 가정인 Jacobian entry affinity를 확인하게 한다. Determinants·eigenvalues·RHS extrema까지 같은 결론으로 넓히지 않는다.

## 7. 기존 native metric에서 고정-Q remainder를 감싸기

### 7.1 Named coefficient operator

\(I_\chi:\mathbb R^6\to\mathbb R^{30}\)를 reduced chemical indices 23..28에 넣는 coordinate injection, \(S_\chi=I_\chi^\mathsf T\)를 selection이라 하자. 고정 \(Q,e\)에 대해

\[
\bar Q=I_\chi Q S_\chi,\qquad \bar e=I_\chi e.
\tag{7.1}
\]

\(\bar Q\)는 **polynomial remainder coefficient operator**라는 명시된 수학적 객체다. 다른 좌표에서 0인 것은 그 operator의 정의이며 missing full-model Jacobian entries를 0으로 채운 것이 아니다. Output role 및 coverage를 분리해야 한다.

Existing fixed native metric은

\[
G=\frac1{33}L^\mathsf T\operatorname{diag}(u_j^{-2})L,\qquad u_j>0
\tag{7.2}
\]

다. 기존 next7 metric(u33) 및 inherited support_core 기반 inverse를 재사용하여 \(G,G^{-1}\)를 얻는다. 새로운 norm/lift evaluator를 만들지 않는다.

### 7.2 Dual factor와 coefficient-operator factor

\[
\sigma^2=\bar e^\mathsf TG^{-1}\bar e
\tag{7.3}
\]

로 두자. \(a=G^{-1}\bar e\)이면

\[
|\bar e^\mathsf Td|=|\langle a,d\rangle_G|
\le\sqrt{\sigma^2}\|d\|_G.
\tag{7.4}
\]

기존 generalized_upper에 \(T_Q=\bar Q^\mathsf TG\bar Q\), \(G,G^{-1}\)를 제공하여

\[
s_QG-T_Q\succeq0,\qquad \|\bar Q\|_G^2\le s_Q
\tag{7.5}
\]

의 exact PSD certificate를 얻는다. Initial bracket, zero-pivot-aware rational LDL, midpoint refinement 및 scalar square-root enclosure는 CLOSED next7의 소스를 그대로 재사용한다. 이 새 Q에 관한 실행은 새 단위의 실행 예산으로만 기록하고 이전 test suite를 재실행하지 않는다.

기존 scalar_sqrt_enclosure로

\[
\boxed{\Gamma\ge\sqrt{\sigma^2s_Q}}
\tag{7.6}
\]

를 얻는다. Exact square 및 Q=0인 경우는 기존 routine의 exact branch를 그대로 따른다.

**정리 7.1 — 고정 coefficient remainder/Hessian bound.** Frozen coefficient polynomial에 대해 모든 \(d,h\in\mathbb R^{30}\)에서

\[
\|\overline R(d)\|_G
\le\Gamma\|d\|_G^2,
\tag{7.7}
\]

\[
\|\overline{D^2F}[d,h]\|_G
\le2\Gamma\|d\|_G\|h\|_G.
\tag{7.8}
\]

여기서 \(\overline R(d)=I_\chi R(S_\chi d)\)이며 coefficients는 frozen 상태다.

**증명.** (5.3), (7.1)에 의해 \(\overline R(d)=(\bar e^\mathsf Td)\bar Qd\)다. 따라서

\[
\|\overline R(d)\|_G
\le\sqrt{\sigma^2}\|d\|_G\sqrt{s_Q}\|d\|_G
\le\Gamma\|d\|_G^2.
\]

Hessian은 \((\bar e^\mathsf Td)\bar Qh+(\bar e^\mathsf Th)\bar Qd\)이므로 각각에 같은 bound를 적용하고 더하면 (7.8)이다. \(\square\)

이는 \(M_2=2\Gamma\)인 Hessian upper와 Taylor remainder \(\frac12M_2\|d\|_G^2\)의 일치도 확인한다. Factor 2를 remainder에 다시 곱하지 않는다. \(\Gamma\)는 일반적으로 최적 상수가 아니다. Dual와 operator의 최대 방향이 다를 수 있고 generalized eigenvalue bracket과 scalar upper도 보수성을 더할 수 있다.

### 7.3 Independent metric controls

Unit physical scales에서 기존 H/He/Li Gram inverse blocks를 사용하면

\[
\sigma^2=\frac{33}{2}w_H^2+66w_{\rm He}^2+165w_{\rm Li}^2
=\frac{33}{2}(w_H^2+4w_{\rm He}^2+10w_{\rm Li}^2).
\tag{7.9}
\]

H block은 \(G_H=2/33\), He block inverse는 \(11\left(\begin{smallmatrix}2&-1\\-1&2\end{smallmatrix}\right)\), Li block inverse는 \(33(I-\mathbf1\mathbf1^\mathsf T/4)\)다. 각 electron vector \((w_H)\), \((w_{\rm He},2w_{\rm He})\), \((w_{\rm Li},2w_{\rm Li},3w_{\rm Li})\)를 대입하면 (7.9)가 된다.

Synthetic weights가 모두 1이면 \(\sigma^2=495/2\)다. Q가 H-only coefficient \(-\kappa E_{00}\)이면 \(\|\bar Q\|_G^2=\kappa^2\). 이 control은 모든 positive electron weights를 사용하므로 full \(\Gamma\)를 1D Hydrogen-only sharp constant와 혼동하지 않는다.

모든 physical scales를 \(u\mapsto t u\), \(t>0\)로 바꾸면 \(G\mapsto G/t^2,\ G^{-1}\mapsto t^2G^{-1}\), \(s_Q\)의 정확한 대상은 불변이고 \(\sigma^2\mapsto t^2\sigma^2,\ \Gamma_{\rm exact}\mapsto t\Gamma_{\rm exact}\)다. 따라서 norm은 \(1/t\)배, remainder upper \(\Gamma\|d\|_G^2\)도 \(1/t\)배가 된다. Numerical upper의 finite refinement/root rounding은 각 증거에서 따로 확인한다.

H-only increment의 별도 1D restriction에서는 \(\|d\|_G^2=(2/33)d_H^2\), \(\|\overline R(d)\|_G=\sqrt{2/33}\,\kappa w_Hd_H^2\)다. Unequal H neutral/charged scales에서는 \(G_H=(u_{\rm H0}^{-2}+u_{\rm H1}^{-2})/33\)로 바뀐다. 이 작은 oracle가 33 및 neutral-lift contribution의 누락을 드러낸다. 새로운 norm 구현으로 채택하지 않는다.

### 7.4 Interval rates는 별도 확장

이번 numerical remainder bridge는 **고정 supplied Q와 e**만 다룬다. Rate-box extrema의 성공이 uniform \(\Gamma\)의 구현 성공을 뜻하지 않는다. 향후 Q-family 전체에 같은 metric의 operator upper를 공급하면 (7.7)–(7.8)을 균일하게 적용할 수 있으나 그 family bound와 source clock enclosure를 별도로 구성·검토해야 한다.

## 8. 구체적인 independent controls

아래 식은 실행 전에 고정하는 mathematical controls이며 PASS 실행 결과가 아니다.

### 8.1 Hydrogen-only source oracle와 feedback omission

다른 charged fractions를 0으로 제한하면

\[
F_H(h)=\gamma(T_H-h)-\kappa w_Hh^2,\qquad
J_{HH}=-\gamma-2\kappa w_Hh.
\tag{8.1}
\]

Fixed-electron derivative는 \(-\gamma-\kappa w_Hh\)만 남겨 discrepancy \(-\kappa w_Hh\)를 만든다. \(\kappa,w_H,h>0\)이면 정확히 비영이다. \(h=T_H/2\)에서 이 discrepancy는 \(-\kappa w_HT_H/2\)다. Plot은 이 diagnostic mismatch와 exact quadratic scaling을 표시하며 actual source trajectory 또는 실제 측정된 native Jacobian 오차로 표시하지 않는다.

### 8.2 Cross-element entry와 diagonal endpoints

전체 family에서

\[
J_{H,\rm HeII}=-r_Hh\,w_{\rm He},
\]

따라서 exact range는 \([-\overline r_HT_Hw_{\rm He},0]\)다. Min은 \(h=T_H,r_H=\overline r_H\), max는 \(h=0\)에서 attained된다. HeIII column에는 charge factor 2가 붙는다.

H diagonal은

\[
J_{HH}=-p_H-r_H\{2w_Hh+w_{\rm He}(u+2v)
+w_{\rm Li}(\ell_1+2\ell_2+3\ell_3)\}.
\]

그러므로

\[
\min J_{HH}=-\overline p_H-\overline r_H
(2w_HT_H+2w_{\rm He}T_{\rm He}+3w_{\rm Li}T_{\rm Li}),
\]

\[
\max J_{HH}=-\underline p_H.
\tag{8.2}
\]

첫 식의 괄호는 \(-\overline r_H\)가 곱하는 전체 비음수 electron expression이다. Min witness는 HII, HeIII, LiIV vertices와 upper rates, max는 all-neutral state와 lower photo rate다.

또한

\[
J_{\rm HeII,H}=(-r_{H\!e1}u+r_{H\!e2}v)w_H
\]

는 양·음 값을 모두 가질 수 있다. 그 min/max는 각각 \(-\overline r_{H\!e1}T_{\rm He}w_H\), \(\overline r_{H\!e2}T_{\rm He}w_H\)이며 서로 다른 state/rate witnesses를 갖는다. 모든 rate coefficients의 sign을 state와 무관하게 한 번만 고정하면 안 됨을 보여준다.

### 8.3 독립 flux oracle와 전체 witness validation

독립 검증은 위 P/Q constructor를 재사용하기보다 9-state adjacent transition flux를 별도로 조립한다. Electron fraction과 그 derivative를 함께 계산하고 source neutral elimination을 적용하여 F/J를 비교한다. 각 witness마다 state simplex와 rate box membership, target entry의 exact equality를 이 oracle로 확인한다.

작은 rate-corner subset의 exhaustive comparison은 sign-selection implementation과 독립적으로 해야 한다. 전체 continuum 결론은 정리 6.1에서 나오며 interior samples의 PASS 횟수에서 나오지 않는다. Zero-width rates, zero transition, conservation boundaries, 실제 non-unit H total, signed increment identity와 polarization을 포함한다.

## 9. 최신 VigilODE 이론 및 이전 matrix bound에 연결

### 9.1 Integrated-polyalgorithm B6

새 branch의 Theorem B6는 fixed carrier \(J_0\)에서 \(F(z+d)-F(z)-J_0d\)의 original-model remainder를 요구한다. Chemical carrier가 정확히 \(J(z)\)라면 고정-coefficient leaf에 대해 \(\Gamma\rho^2\), \(\|d\|_G\le\rho\), 를 공급할 수 있다.

Carrier가 다르면

\[
F(z+d)-F(z)-J_0d
=(e^\mathsf Td)Qd+(J(z)-J_0)d.
\tag{9.1}
\]

따라서 같은 norm에서 verified mismatch upper \(\delta_J\)가 있으면 \(\Gamma\rho^2+\delta_J\rho\)를 사용한다. Current exact-point J라는 이름만으로 whole-domain mismatch를 0으로 선언하지 않는다. 전체 model의 omitted geometry/clock terms까지 이 chemical bound가 감싼다고 주장하지 않는다.

### 9.2 PR93 및 PR90

PR93 G13의 curvature remainder 입력에 (7.8)의 \(M_2=2\Gamma\)가 component별 근거가 될 수 있다. PR93 G11/G12의 full-space inverse·Lipschitz 및 stage-argument connecting paths는 다른 의무다.

PR90의 fixed-metric dissipativity·Cayley·Jacobian drift에는 완전한 해당 operator의 same-metric bound가 필요하다. 6×6 entry certificate만으로 full30 signed lognorm 또는 dissipativity를 발급하지 않는다. 이후 빠진 blocks와 coefficient/rounding enclosures를 갖춘 full assembly를 next7 matrix_bounds에 전달할 수 있다. Native norm, generalized PSD routine, native scaled residual factor와 cached/current margin의 구분은 기존 소스를 재사용한다.

### 9.3 Actual geometry/time와 source arithmetic 경계

실제 CoupledSystem의 chemistry RHS를 \(F_{\rm chem}=(dY/dt)/(\mathrm{scale}\,H)\)라 쓰면 geometry columns는

\[
J_{\rm chem,geo}=-F_{\rm chem}(\nabla_{\rm geo}\log H)^\mathsf T.
\tag{9.2}
\]

이는 OPEN_NOT_ZERO다. Geometry rows, photon Thomson feedback, optical-depth row도 OPEN_NOT_ZERO다. 전체 source-clock 변화에는 T/UV/n_b/H dependence 및 별도 partial-time/regularity 입력이 필요하다. Frozen coefficient polynomial의 signed increment를 geometry-varying actual increment로 대체하지 않는다.

기존 float-query membership과 affine tube certificate가 actual binding·domain에 적용된다는 증거가 있을 때 chemical simplex premise로 연결할 수 있다. 단순히 indices와 source name이 같다는 이유로 새로운 capsule/domain 포함을 가정하지 않는다.

## 10. Admission, exactness 및 보존 ceiling

발급 가능한 결과는 다음처럼 범위를 명시한다.

- Source-derived six-variable polynomial 및 electron-feedback identity.
- Declared independent rate box × physical simplex의 exact scalar entry ranges와 72 witnesses.
- Frozen-coefficient named Qbar operator에 관한 fixed-native-metric remainder/Hessian bound.

이를 실제 source 전체의 interval Jacobian, physical step, solution enclosure 또는 VerifiedModelProvider로 승격하지 않는다. Source-reconstructed RN weights, observed owner array, declared coefficient intervals, 실제 clock enclosure, exact algebra, callback floating error를 각각 기록한다.

RHS quadratic interior extrema의 예는 특정 잘못된 확장에 대한 구성적 control이다. 새로운 strong no-go, 문헌 novelty 또는 actual model instability를 주장하지 않는다.

실행은 부모의 preexecution review·고정 source·backup checkpoint·GO 이후 lower-tier implementation worker가 수행한다. 이 문서는 root가 확인한 독립 검토나 실제 실행 결과를 대신하지 않는다.

**보존 상태:** 중앙28/88·자체delta0, query_rounding_bound:null, actual_query_rounding_bound:null, whole_domain_certificate:false, actual_ode_solution_enclosure:false, VerifiedModelProvider:null, V7fullrestore:NOT_RUN, production:HOLD, globalphysicalcompletion:null, L4:false. REI25는 별도 binding이며 기존 CLOSED와 UNKNOWN_PRESERVED budget은 변하지 않는다.
