# 보존형 리매핑: 구성·오차 예산·BASS/rei 연결

기준 코드: `7a63876a97fc17f640771ec06658269947049994`의 후속 구현. 이 문서의 정리는 아래 전제에서 직접 유도했다. 구현 위치는 `crates/rodas5p-integrators/src/conservative_remap.rs`이다. 현재 결과는 양의 셀 적분량에 대한 1차 보존형 remap kernel이다. 원본 ODE driver의 가변 차원 hook, 고차 재구성, 일반 구면 mesh의 교차 다각형 계산은 후속 통합 항목이다.

## 1. 보존하는 양과 측도를 먼저 고정한다

양의 측도 $d\mu=w(z)dz$를 갖는 유한 구간을 old cells $I_i$, new cells $J_j$로 분할한다. 둘 다 정확히 같은 영역을 덮고 셀 측도 $V_i=\mu(I_i)$, $W_j=\mu(J_j)$는 양수다. 입력은 값 $u_i$가 아니라 적분량 $Q_i=V_i u_i\ge0$이다. 누적 측도 좌표 $x(z)=\int^z w(s)ds$로 옮기면 셀 길이가 측도가 된다. 현재 API의 `source_edges`와 `destination_edges`가 바로 이 $x$ 좌표다. 따라서 음의 좌표는 허용하지만 0 이하 폭과 비유한 폭은 거절한다.

질량 없는 광자의 물리적 tetrad에서, 스칼라 occupation $f$가 한 편광당 정의되어 있고 $g$개의 동일한 편광을 합한다면

\[
 dn=\frac{g}{(2\pi\hbar)^3c^3}E^2 f(E,\hat e,t)\,dE\,d\Omega,
 \qquad du_\gamma=E\,dn.
\]

여기서 $dn$은 단위 proper volume당 photon number이고 $du_\gamma$는 에너지 밀도다. Total number에는 proper cell volume을 곱한다. Bianchi/tilted frame의 volume 및 flux 계약을 FLRW의 $a^3$만으로 대체하지 않는다. 편광 밀도행렬 $f_{AB}$의 trace를 사용하면 이미 합한 편광을 $g$로 다시 곱하지 않는다. 이 정의는 프로젝트의 photon variable normalization과 별도 대조해야 한다. photon number remap에는 $x=E^3/3$, energy remap에는 $x=E^4/4$가 대응한다. 두 경우 모두 상수 정규화와 각도 셀 측도는 일관되게 포함한다. 동일한 piecewise-constant $f$의 한 scalar 적분량만으로 photon number와 energy라는 두 독립 모멘트를 동시에 보존한다고 주장하지 않는다. scalar remap을 signed Stokes $Q,U,V$ 또는 PSTF 성분에 그대로 적용하지도 않는다.

BASS/rei의 일반 Bianchi 배경에서는 $\hat e\in S^2$의 두 각도 자유도가 필요하다. 축대칭이 입증된 부분 문제에서만 $(E,\theta)$가 충분하다. 구면 좌표에서 $d\Omega=d\phi\,d(\cos\theta)$이므로 raw $\Delta\theta$를 셀 면적으로 쓰면 보존량이 달라진다. 현재 1차원 kernel은 고정 각도 셀별 에너지 remap, 또는 적절한 측도 좌표의 separable angular sweep에 재사용할 수 있다. 일반 spherical mesh에는 면적 교차 연산자를 별도로 구현한다.

## 2. 정리 R1: 양성·보존·수축·선형 비용

정의:

\[
 R_{ji}=\frac{\mu(J_j\cap I_i)}{V_i},\qquad Q'_j=\sum_i R_{ji}Q_i.
\]

완전한 같은 영역 분할이면 $R_{ji}\ge0$이고 각 열합은 $\sum_jR_{ji}=1$이다. 그러므로 다음이 성립한다.

1. $Q_i\ge0\Rightarrow Q'_j\ge0$.
2. $\sum_jQ'_j=\sum_iQ_i$.
3. 같은 old/new grids의 두 적분량 벡터에 대해
   \[
   \|RQ-R\widetilde Q\|_1
   \le\sum_{ji}R_{ji}|Q_i-\widetilde Q_i|
   =\|Q-\widetilde Q\|_1.
   \]
   밀도 표현으로는 $\sum_j W_j|u'_j-\widetilde u'_j|\le\sum_i V_i|u_i-\widetilde u_i|$이다. 공간 가중 L1 수축은 시간 solver의 WRMS/Euclidean norm 수축으로 자동 변환되지 않는다.
4. 두 구간 분할의 sorted edges를 동시 순회하면 각 양의 overlap은 적어도 하나의 오른쪽 edge를 소모한다. 따라서 overlap 수는 $n+m-1$ 이하이고, 생성 및 적용 비용은 $O(n+m)$이다. dense remap matrix를 저장하지 않는다.

구현은 진단값을 얻기 위한 두 번째 순회까지 포함해 $O(n+m)$이다. workspace는 길이 $N_{max}$의 `f64` staging 및 보상합 배열 두 개, 즉 element storage $16N_{max}$ bytes다. `N_max=4096`이면 65536 bytes이며 Vec 객체와 caller 배열은 별도다. 생성 후 `remap` 코드에는 allocation/resize 또는 문자열 생성 경로가 없으며 output slice의 첫 변경은 모든 검사 후 `copy_from_slice` 한 번이다. 실패해도 caller output은 유지된다. 기존 driver 전체가 무할당이라는 결론은 이 kernel에서 도출되지 않는다.

정리의 보존·수축은 실수 연산 명제다. `f64` 구현은 보상합, 비유한 연산 거절, 양의 항의 0 underflow 거절, 사용자가 지정한 상대 질량차 guard를 사용한다. 보고되는 질량차와 아래 L1 값은 진단값이며 directed-rounding certificate는 아니다.

## 3. 정리 R2: 실제 remap perturbation과 누적 오차

old reconstruction $u(z)=Q_i/V_i$와 new reconstruction $u'(z)=Q'_j/W_j$의 L1 변화는

\[
 \delta=\int|u-u'|d\mu
 =\sum_{ij}\left|Q_i\frac{\mu(I_i\cap J_j)}{V_i}
                 -Q'_j\frac{\mu(I_i\cap J_j)}{W_j}\right|.
\]

각 overlap에서 두 밀도가 상수이므로 절댓값 적분을 직접 계산하면 된다. 이 표현은 `Q_i/V_i`의 잠재적 overflow를 피한다. `reconstruction_change_l1`가 이 식의 부동소수점 평가다. 이 값은 *표현 변경의 크기*이며 unknown exact solution에 대한 오차 estimator로 해석하지 않는다. refinement가 old constant cells를 그대로 나누면 $\delta=0$이고 coarse remap에서는 작은 scale 정보를 제거하는 비용이 드러난다.

동일한 물리 함수 공간의 norm으로, 한 step의 propagation이 $L_k$-Lipschitz이고 temporal+spatial defect 상계가 $\tau_k$, remap perturbation 상계가 $\delta_k$, arithmetic error 상계가 $\eta_k$라면 삼각부등식으로

\[
 e_{k+1}\le L_ke_k+\tau_k+\delta_k+\eta_k.
\]

귀납적으로

\[
 e_K\le\left(\prod_{k=0}^{K-1}L_k\right)e_0+
 \sum_{j=0}^{K-1}(\tau_j+\delta_j+\eta_j)
             \prod_{k=j+1}^{K-1}L_k.
\]

따라서 dissipative contractive setting $L_k\le1$에서는 remap 기여를 $\sum\delta_k$로 직접 배정할 수 있다. 실제 certificate를 만들 때는 floating diagnostic 대신 outward bound와 검증된 $L_k,\tau_k,\eta_k$가 필요하다. 기존 `global_budget.rs`는 고정 Euclidean norm과 sealed model/provider identity를 요구하므로 이 L1 수식의 숫자를 그 API에 임의 주입하지 않는다. 공통 물리 norm의 새 adapter 또는 증명된 norm 변환이 연결 의무다.

공간적으로 $L$-Lipschitz인 scalar density $u$를 폭 $h_j$의 셀 평균으로 바꾸면

\[
 \int_{J_j}|u-\overline u_j|dx
 \le\frac{L}{h_j}\int_{J_j}\int_{J_j}|x-y|dxdy
 =\frac{Lh_j^2}{3}.
\]

이 식은 mesh와 보간 오차를 연결하는 구성적 상계다. 정칙성이 없으면 이 상계를 사용하지 않고 실제 reconstruction 변화 또는 TV 기반 상계를 선택한다.

## 4. 단일 step과 5차의 정확한 연결

RODAS 계열은 remesh 후 multistep history를 이식할 필요가 없다는 구현 이점이 있다. 그러나 재투영 오차, 새 semidiscrete RHS의 변화, limiter regularity, stage solve 오차, tolerance norm 변경은 여전히 오차 budget에 들어간다. Remesh가 $O(1/h)$번 발생하는 고정 시간 구간에서 5차 global convergence를 유지하려면 안정도 가정하에 remap 누적 오차가 $O(h^5)$여야 한다. 충분조건은 매 remap의 perturbation이 $O(h^6)$인 것이다. 현재 piecewise-constant remap은 positivity와 conservation baseline이며 이러한 고차 조건을 자동 충족하지 않는다.

Accepted-step integration 계약은 다음과 같다: 시간 step을 먼저 수락하고 원래 상태를 보존한다. candidate mesh와 remapped state를 staging에 생성한다. 보존/양성/오차 budget을 확인한 후 mesh epoch·state·dimension을 함께 commit한다. Jacobian/preconditioner/recycled Krylov vectors 및 cached reconstruction은 새 epoch에서 무효화한다. controller의 error norm을 새 cell measure와 component scale에 맞추고 다음 h를 재평가한다. remap이 거절되면 원래 mesh/state로 계속하거나 더 작은 적응 작업을 제안한다. 기존 dense output은 원래 mesh identity를 유지하고 remesh를 가로지르는 query는 명시적 representation mapping을 요구한다.

## 5. 검증과 다음 구현 순서

독립적으로 계산할 수 있는 nonuniform refine/coarsen, crossing grids의 정확한 적분량과 L1 변화, 두 density의 weighted-L1 수축, invalid grid/capacity/shape, overflow/underflow rollback, 100회 차원 변경 후 storage identity를 standalone Rust tests로 확인했다. `evidence/REMAP_TESTS.log`를 참조한다.

다음 구현은 (1) 실제 BASS photon 변수와 측도 binding, (2) accepted-step transactional hook, (3) threshold-aligned log-energy mesh에서 고정격자 convergence, (4) number/energy two-moment conservative reconstruction, (5) limiter/positivity와 시간오차 budget 결합 순서다. 한 energy cell $[a,b]$의 비음수 photon measure가 주어진 number $N$와 energy $U$를 실현하려면 $aN\le U\le bN$이 필요하다. 이 조건은 두 끝점의 양의 가중치 $w_a=(bN-U)/(b-a)$, $w_b=(U-aN)/(b-a)$로 충분함도 보인다. 실제 smooth/FVM reconstruction은 이 realizability 조건을 유지하도록 설계하며 atom construction 자체를 물리 spectrum으로 채택하지 않는다.
