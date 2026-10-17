# Collective cubic의 O(N) Rosenbrock 직접 기준해법

## 결과와 가정

`CollectiveModel`의 회전 좌표 모델을 포함하는 다음 복소 모드 family

\[
 z_j'=(-\kappa+i\omega_j)z_j+i\chi |m|^2m,
 \qquad m=\sum_{j=1}^N\pi_jz_j,
 \qquad \omega_j=\Omega+\Delta\xi_j
\]

를 고정된 상태에서 선형화한다. 현재 native `CollectiveModel::rhs/jvp`는 carrier를 메타데이터로 보관하고 실제 회전 좌표의 주파수는 **ω_j=Δξ_j**를 사용한다. 따라서 현재 adapter와 직접 연결할 때 이 reference의 carrier는0이어야 한다. Ω≠0인 reference cases는 물리 좌표 확장 검산이며 현재 source와의 동일성 검사로 세지 않는다. \(\pi_j>0,\sum\pi_j=1\), \(\kappa\ge0\), \(\chi,\omega_j\in\mathbb R\), stage shift \(s\ge0\)를 가정한다. 이 문서는 \(A=I-sJ\)의 정확한 직접해법이다. 프로젝트의 \(W=I/(\gamma h)-J\)에는 \(s=\gamma h\), \(A x=s b\)로 연결한다.

**정리 WB1.** 아래2×2 Schur matrix가 가역이면, \(A\)의 setup과 각 우변 해법은 O(N) scalar arithmetic, O(N) storage로 수행된다. R개의 우변을 모두 복원하는 비용은 O(N+RN)이다. 일반 dense LU의 O(N³) setup과 O(RN²) triangular solves를 이 모델에서 제거한다. 이 주장은 구조가 주어진 해당 stage 행렬에 대한 것으로, 이미 구조를 활용하는 다른 방법과의 우열이나 native runtime 속도비를 포함하지 않는다.

이 구성은 알려진 저랭크 inverse identity의 해당 모델 적용이다. 수학적 우선권을 주장하지 않는다. 구체적 기여는 현재 모델의 rank2 Jacobian 식, 코드 연결, 인증 조건 및 모든 우변 복원 비용을 포함한 exact reference다.

## 유도와 증명

\(z_j=(x_j,y_j)^T\), \(m=(u,v)^T\)로 실수화하면

\[
 D_j=\begin{pmatrix}-\kappa&-\omega_j\\\omega_j&-\kappa\end{pmatrix},
 \quad F(m)=\chi\begin{pmatrix}-(u^2+v^2)v\\(u^2+v^2)u\end{pmatrix},
\]
\[
 C=DF(m)=\chi\begin{pmatrix}-2uv&-(u^2+3v^2)\\3u^2+v^2&2uv\end{pmatrix}.
\]

\(U\)는 I₂를 N개 수직으로 쌓은2N×2 행렬, \(V=(\pi_1I_2,\ldots,\pi_NI_2)\)라 두면 chain rule로

\[
 J=\operatorname{diag}(D_j)+UCV.
\]

\(a=1+s\kappa\ge1,b_j=s\omega_j\),

\[
 B_j=I-sD_j=\begin{pmatrix}a&b_j\\-b_j&a\end{pmatrix},
 \quad P_j=B_j^{-1}=\frac{1}{a^2+b_j^2}
 \begin{pmatrix}a&-b_j\\b_j&a\end{pmatrix}
\]

는 항상 가역이다. \(B=\operatorname{diag}(B_j)\), \(H=\sum\pi_jP_j\),

\[
 S=I_2-sHC
\]

를 setup에 한 번 계산한다. 우변 \(b=(b_1,\ldots,b_N)\)에 대해

\[
 z_j=P_jb_j,\qquad r=\sum_j\pi_jz_j,
 \qquad q=S^{-1}r,\qquad w=sCq,
 \qquad x_j=z_j+P_jw. \tag{1}
\]

실제로 \(Vx=r+sHCq=q\)이며 \(Bx=b+UCq\,s\)이므로 \((B-sUCV)x=b\). 따라서 (1)은 해다. 역으로 \(Ax=0\)이면 \(Sq=0\) 및 \(x=sB^{-1}UCq\)이므로 S가 가역일 때 x=0이다. 또한 S가 특이하면 영이 아닌 q∈kerS에 대해 \(x=sB^{-1}UCq\)의 Vx=q라서 x≠0이고 Ax=0이다. 이로써 A와 S의 가역성은 동치다(s=0에는 둘 다 항등행렬).

setup은 평균2성분, N개의2×2 inverses,2×2 weighted sum,2×2 multiplication/inversion이다. 각 우변은 N개 block actions,2성분 평균,2×2 inverse action, N개 block corrections다. 모두 O(N), 저장하는 block inverses와 하나의 우변/출력 workspace도 O(N)이다. R개 결과를 모두 보존하면 출력 자체의 O(RN) storage는 추가한다. 이 증명은 정수/유리수 bit complexity를 일정한 scalar operation으로 취급하지 않는다.

## 인증 가능한 역행렬 상계

**정리 WB2.** \(L=3|\chi||m|^2\), \(q_*=sL/a<1\)이면 A,S가 가역이고

\[
 \|S^{-1}\|_2\le\frac1{1-q_*},\qquad
 \|A^{-1}\|_{\pi,2}\le\frac1{a-sL},
 \quad \|x\|_{\pi,2}^2=\sum_j\pi_j\|x_j\|_2^2.
\]

증명: F(m)=χR|m|²m, R은90° 회전이다. 따라서 C=χR(|m|²I+2mmᵀ), \(\|C\|_2=3|\chi||m|^2\). B_j는 normal이며 \(\|P_j\|_2=(a^2+b_j^2)^{-1/2}\le1/a\). 양의 정규화 weights로 \(\|H\|_2\le1/a\)다. \(\|sHC\|\le q_*<1\)에 Neumann series를 적용하면 첫 상계가 나온다. 가중 norm에서 \(\|U\|=\|V\|=1\), \(\|B^{-1}\|\le1/a\)이므로 A=B(I−sB⁻¹UCV)의 Neumann inverse 상계가 둘째 식을 준다.

임의의 근사 stage 해 \(\hat x\)에 대해 실제 residual r=b−A\hat x의 norm을 outward하게 계산하면

\[
 \|x-\hat x\|_{\pi,2}\le\frac{\|r\|_{\pi,2}}{a-sL}.
\]

q*≥1에서도 S가 가역일 수 있다. 이 경우 다음 native 구현은 S의 각 계수를 outward interval로 계산하여 det(S)의 interval이0을 배제하는지 확인하고 adj(S)/det(S)로2×2 inverse enclosure를 구성할 수 있다. 현재 Fraction reference는 exact determinant nonzero를 판정하며 floating interval inverse는 구현하지 않았다. 전체 정확도 보증에는 상계 계산과 residual 산술의 outward rounding이 필요하다.

## 코드·실행

- 현재 source adapter: `crates/rodas5p-integrators/src/collective_model.rs`, `CollectiveModel::jvp`와 `to_ode_problem`.
- reference: `woodbury_reference.py`, stdlib `fractions.Fraction`만 사용한다.
- 독립 확인: 별도로 좌표 미분식으로 dense A를 작성하고 pivoted Gaussian elimination의 exact 답 및 A·x=b를 비교한다.
- 실행: `python3 research/production_audit_20261018/woodbury_reference.py --output research/production_audit_20261018/evidence/woodbury_checks.json`.
- N=1,2,4,8에서 native 회전 좌표(carrier0, nonlinear), 무감쇠/비영 carrier, 감쇠/음의χ, zero width/linear 네 family,64개 우변을 검산했다. 모든 계수·답·residual은 exact equality다.
- 추가 경계: q*=3이고 det(S)=4인 가역 case 및 det(S)=0인 singular case를 확인했다. 충분조건을 필요조건으로 혼동하지 않는다.
- 구현에 실제 삽입된 카운터는 모든 setup의 평균/Jacobian/Schur/bound 연산과 각 우변의 모든 모드 복원을 포함한다. 각 우변은 **18N+14** rational operations이다. unary sign, 입력 검증·파싱·할당·loop overhead는 카운터 단위 밖이며 wall time으로 환산하지 않는다.

다음 native 단위는 `CollectiveStageFactors`(P_j,H,C,S의 factor/inverse와 identity) 및 `solve_into` workspace다. 동일 frozen Jacobian·동일 stage RHS·동일 output grid에서 dense RODAS 결과와 residual enclosure를 비교한 뒤, packet/기존 matrix-free와 같은 물리 정확도의 전체 trajectory 비용을 측정한다. 한 stage의 선형 대수 개선을 전체 adaptive trajectory 속도 승격으로 바로 바꾸지 않는다.
