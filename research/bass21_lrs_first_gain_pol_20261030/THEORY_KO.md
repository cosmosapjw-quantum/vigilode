# BASS21 source-owned LRS single-scattering Thomson polarization: 이론 유도

**범위/상태** 2026-10-30 KST. 원본 VigilODE PR127가 보존한 BASS owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`의 original trajectory 50×193을 **동일 사건**에서 결합한다. 새로운 기하/화학/편광 IVP는 전혀 적분하지 않는다. 새로운 값은 원본 *무편광* 광자 진화에 대한 **cold-electron first-gain (single-scattering source with exact extinction)** 후처리이며, 완전한 편광 Boltzmann 해, CMB E/B 관측량 또는 z=0 관측 광학깊이가 아니다. 모든 우주론 변수는 BASS의 정규 직교 normal tetrad 기준이다. Metric signature `(-,+,+,+)`, 물리 상수 c와 σ_T를 명시한다.

## A. source authority와 동일 이벤트 보장

원본 BASS21 파일에 저장된 `raw_state62`는 **같은** 62성분 `CoupledSystem`의 해다. `[1]`은 LRS scale shape `b`, `[4:13]`은 photon **energy** Legendre tower, `[13:22]`은 photon number tower, `[22:31]`은 neutrino energy tower, `[31:40]`은 neutrino number tower, `[40:61]`은 원자 abundance, `[61]`은 forward τ. `diagnostics`에는 같은 193 시점의 `H`(s⁻¹), `ne_m3`(m⁻³), `gamma_T_per_H`, `tau_T`, `b`가 있다. 50개 binary NPZ를 원본 Git/manifest SHA로 모두 확인한다. source의 b와 y[:,1], source τ와 y[:,61]의 정확한 동등성도 검사한다.

기본 방정식은 원본 frozen code:

- `research/rei_hheli_compact_pilot_20261022/vendor/rec_hheli/vendor/baseline/lrs_boltzmann.py` (Git blob `e15e1875fba7fcc4a42e257e291c54d9b540a3f0`)
- `research/rei_hheli_compact_pilot_20261022/vendor/rec_hheli/code/geometry.py` (Git blob `a6ba35fff4f62ef795628499f108705ab6b55ac1`)
- `research/rei_hheli_compact_pilot_20261022/vendor/rec_hheli/code/coupled.py` (source-owned full62 RHS/diagnostic: same BASS owner commit)

CE1-Q/Y의 6-cell collisionless Einstein–Vlasov data (다른 SHA, 다른 41-time clock)는 **이 계산의 입력이 아니며**, 동일 물리 사건이라는 가정도 사용하지 않는다. 이 선택은 이전 source/event blocker를 *BASS21 내부 경로에 대해서만* 피한다. CE1 41↔BASS21 193 직접 매핑의 본래 blocker는 여전히 미해결이다.

## B. LRS null characteristic과 screen basis

원본 LRS Bianchi-I orthonormal geometry:

\[
 ds^2=-c^2dt^2+a_\perp^2(dx_1^2+dx_2^2)+a_\parallel^2dx_3^2,
\quad a_\perp=a e^{-b},\ a_\parallel=a e^{2b},
\quad x=\ln(a/a_\mathrm{ref}),\quad {dx\over dt}=H>0.
\]

정규화된 광자 전파축의 cosine을 `μ=n·ẑ`로, 관측자 시점 xₒ의 방향을 μₒ로 잡는다. 세 공간 Killing 좌표의 공변 운동량 q_i는 null geodesic을 따라 보존된다. 이미 저장된 원본 b(x)만으로 방출점 x에서의 방향과 에너지 비율이 정해진다. δb=b(x)−b(xₒ)에 대해

\[
D^2(x;x_o,\mu_o)=(1-\mu_o^2)e^{2\delta b}+\mu_o^2e^{-4\delta b},\qquad
\mu_x={\mu_o e^{-2\delta b}\over D},\qquad
{E_o\over E_x}={a(x)\over a(x_o)}D^{-1}.
\]

차원 검사: D, μ, x, b는 무차원. \(\partial_b\mu=-3\mu(1-\mu^2)\), \(\partial_b\ln D=1-3\mu^2\)이며 원본 `lrs_boltzmann.py`의 angular Liouville characteristic과 일치한다. Wolfram 실제 `FullSimplify` 검산에서 이 두 미분 항등식과 아래 screen-factor 항등식의 잔차가 각각 정확히 0이다.

LRS 대칭으로 photon ray와 공간 대칭축의 meridional plane이 불변이다. 그 평면의 meridional/azimuthal screen axes를 local parallel-transport한 기저로 잡으면 이 경로의 기하 회전은 Stokes Q와 U를 혼합하지 않는다. 이 제한적인 무회전 속성은 일반 Type-II나 임의 비대각 Bianchi 계량으로 확장하지 않는다.

## C. 원본 energy tower와 Thomson polarization gain

Frozen BASS는

\[
F(\mu,x)={a(x)^4 I(x,\mu)\over a_\mathrm{ref}^4 I_\mathrm{ref}}
=\sum_{\ell=0}^{8}(2\ell+1)J_\ell^\gamma(x)P_\ell(\mu)
\]

를 저장하며, `J₂=raw_state62[:,6]`. 여기서 `I_ref`는 모형의 상대 기준 세기이며 임의로 Kelvin이나 열역학적 CMB temperature로 해석하지 않는다. 기존 BASS의 unpolarized Thomson 감쇠 \(\dot J_2|_C=-(9/10)(c\sigma_T n_e/H)J_2\)는 편광의 E₂ 역작용을 **포함하지 않는다**.

Cold, rest-comoving electron의 산란 이득은 screen tensor \(P(n)=\mathbf 1-nn^T\)와 \(J^{in}_{ij}=(F/2)P_{ij}\)를 사용해

\[
(KJ)_{ij}(n)={3\over8\pi}P_{ik}(n)\Big[\int_{S^2}J^{in}_{k\ell}(n')d\Omega'\Big]P_{\ell j}(n).
\]

원래 F의 higher multipoles ℓ≥4는 local Thomson *gain*에 기여하지 않는다. \(\int n_i n_j I\,d\Omega\)의 모멘트로부터 얻는 정확한 축대칭식은

\[
I_\mathrm{gain}=J_0+\tfrac12J_2P_2(\mu),\quad
Q_\mathrm{gain}=J_{\theta\theta}-J_{\phi\phi}
=-\tfrac34J_2(1-\mu^2),\qquad U_\mathrm{gain}=V_\mathrm{gain}=0.
\]

예: \(F=I_0[1+\alpha P_2(\mu)]\)이면 \(J_2=I_0\alpha/5\), 적도에서 \(Q_\mathrm{gain}=-3\alpha I_0/20\). 독립 GL28×56 photon-direction 구면적분으로 원본 세 케이스의 세 방향 모두 원식을 ~1e-14 수준까지 직접 확인했다.

## D. 정확한 source 이벤트의 방향별 collision-rate + line-of-sight Q

원본 source는 normal-comoving cold electron scattering을 사용하므로 한 ray에서

\[
g(x)=c\sigma_T n_e(x)/H(x)\,,\qquad\tau_\mathrm{fwd}(x)=\int_0^xg(u)\,du\,.
\]

`gamma_T_per_H`, `tau_T`는 같은 owner와 저장 solver가 산출한 값이다. 이 `τ_fwd`는 **z=20→2 positive forward optical depth**로, z=0 CMB 광학깊이가 아니다. 전자 상대속도 \(\beta_e\ne0\)일 경우 생기는 \(D_{e\leftarrow n}=\gamma_e(1-\beta_e\cdot n)\)은 이번 source에 없으므로 삽입하지 않는다.

주어진 unpolarized original `J₂`를 외력으로 고정하고, 생산된 Q가 다시 광자의 세기·편광 collision gain에 미치는 영향은 무시한다. 단일 획득 산란 source와 `-g Q` extinction을 포함한, **완결된 선형 후처리 해**는

\[
\boxed{\widetilde Q_o(\mu_o)=-\frac34\int_0^{x_o}\!dx\;g(x)\,J_2(x)\;e^{-[\tau(x_o)-\tau(x)]}
\,\frac{1-\mu_x^2}{D^4}}
\]

이고 \(\widetilde Q_o=Q_o/[I_\mathrm{ref}(a_\mathrm{ref}/a_o)^4]\)다. 따라서 `I`와 `Q`의 정규화를 일치시키면 \(Q/I=\widetilde Q/F(x_o,\mu_o)\). 안정적인 계산 형태는

\[
\frac{1-\mu_x^2}{D^4}=
\frac{(1-\mu_o^2)e^{2(b(x)-b_o)}}{[(1-\mu_o^2)e^{2(b(x)-b_o)}+\mu_o^2e^{-4(b(x)-b_o)}]^3}.
\]

이 식에서 μₒ=±1에서는 Q=0이고, xₒ=0에서도 Q=0. b가 일정하고 g,J₂가 상수인 제조해석 극한은 \(\widetilde Q_o=-\frac34J_2(1-\mu_o^2)(1-e^{-g x_o})\)가 된다. 저장 시점 사이를 새 IVP로 채우지 않고 원본 x_grid 위의 composite sampled-grid Simpson을 사용하며, 이는 quadrature의 표본 오차만 점검할 수 있다.

**근사 순서:** 원래 BASS L=8 photon energy moments와 n_e/b/H를 완전히 고정 → 선형 polarized gain 계산 → 해당 first-gain source를 unmodified scalar attenuation과 redshift screen transportation으로 적분. Polarized gain의 재산란 및 E_2다중극 역작용, 진정한 spectral/thermal Thomson regime, gas/UV feedback, 복사 스크린의 arbitrary global spin atlas는 해결되지 않았다. `Q/I`는 소규모 일관성 진단이지 정확한 full solution error bound가 아니다.

## E. matched pair 3-input attribution (exact algebra, not causal)

동일 (m,r)의 두 원본 이력 A: `s00`, B: `s04`에 대해 서로 다른 3개 입력군을 식별한다: (1) `b(x)` 기하, (2) `J₂(x)` 입사 사중극자, (3) `(g(x),τ(x))` 전자/기하 광학 clock. 각 항의 A/B 선택으로 2³=8번 위 boxed 식을 계산한다. `Shapley`의 6개 교환 순서를 평균하면 각 3군 attribution의 합은 **항상** \(\Delta Q=Q_B-Q_A\)를 재구성한다. 혼합 입력은 일반적으로 원래 Einstein–Boltzmann solution이 아니므로 **관측 가능한 독립적 인과 개입이 아니다**. 이전 optical depth의 geometry/chemistry 92%/8%와도 별개다.

## F. 판정

이번 bounded BASS21 동일 소스의 local photon→polarized gain 및 frozen-background line-of-sight Q는 **실제 BASS21 source-conditional model physics**다. 원본 원자·기하·광자들이 동일 물리 사건이라는 부분은 archive의 동일 saved 62-state로 닫혔다. CE1 6-cell을 결합한 full polarized CMB 계산의 source/event blocker, distinct-author R6/C9, original BASS21 global BDF convergence, ledger CI failure, production HOLD는 각각 별도로 남는다.

원전: Pontzen & Challinor, MNRAS 380 (2007) https://doi.org/10.1111/j.1365-2966.2007.12221.x ; Fleury, Pitrou & Uzan, PRD 91 (2015) https://doi.org/10.1103/PhysRevD.91.043511 ; original BASS owner Git blob above.