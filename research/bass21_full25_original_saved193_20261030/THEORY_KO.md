# VigilODE × BASS21: 원본 저장 이력 25쌍의 전자·기하학 광학깊이 분해

**범위:** 2026-10-30. `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`에 저장된 50개 서로 다른 원본 `trajectory.npz`의 193시점 데이터. 신규 initial-value problem 0회, 새 시공간 궤적 생성 0회. Source-native `AtomicNetwork.evaluate` 9650회. BASS가 처방한 T(x), UV(x), test-fluid 바리온 가정과 광이온화/복합적 전자 source만 다룬다. Metric convention은 기본 (-,+,+,+), Bianchi-I volume scale factor의 양의 평균 H를 사용한다.

## 1. 변수, 단위, clock, source

원본 BASS forward clock은 \(x=\ln[a/a_{\mathrm{start}}]\), \(a_{\mathrm{start}}=1/21\), \(x\in[0,\ln7]\), \(z=21e^{-x}-1\), 즉 z=20→2이다. Proper time t(s)에 대해 \(H=d\ln a/dt>0\), 따라서 \(dt=dx/H\). 부호 및 integration orientation은 앞으로 증가하는 x이다. 물리적으로 일반적으로 관측자 쪽으로 거꾸로 정의하는 재이온화 optical depth와 원점/부호가 다르므로 **원본 BASS 저장된 tau_T**로 일관되게 비교한다.

원본 BASS 21성분 baryon-normalized abundance \(Y\), 전하 \(q\), 질량수 \(A\)에서 \(x_e=q^\top Y\), \(n_e=n_b x_e\). 원본 source code에서 \(n_b,n_e\)는 m\(^{-3}\), \(H\)는 s\(^{-1}\), \(Y,x_e,x\)는 무차원이다. \(K=c\sigma_T=(2.99792458\times10^{10}\ \mathrm{cm/s})(6.6524587051\times10^{-25}\ \mathrm{cm^2})/10^6\approx1.994356946945426\times10^{-20}\ \mathrm{m^3/s}\)은 고정된 원본 상수다.

BASS의 이 저장 관측량은 \(g(x)=d\tau_T/dx=K n_e/H=K n_b x_e/H\), \(\tau_T(x)=\int_{0}^{x}g(u)du\), \(\tau_T(0)=0\). 부호, 단위 및 길이 검산: \([K n_e/H]=(\mathrm{m^3/s})(\mathrm{m^{-3}})/(\mathrm{s^{-1}})=1\); \(\tau_T\) 무차원, \(g\ge0\) if \(n_e\ge0\). 이는 source model의 공간 균일 전자 소산/산란 surrogate이며, 방향별 null geodesic 광경로, electron bulk peculiar velocity, polarization/Stokes hierarchy, 관측 CMB 광학깊이가 아니다.

## 2. 유한 matched-shear 차이의 정확한 항등식 (DERIVED)

A는 \(\Sigma_{F,\mathrm{init}}=0\), B는 0.6이고 다른 초기 \(P_{m,F},P_{r,F}\) 목표가 동일한 한 쌍이다. \(\Delta f=f_B-f_A\), \(\bar f=(f_A+f_B)/2\). \(v=H^{-1}, w=n_b, y=x_e\)로 놓으면 모든 실수 양의 H에서 \(\Delta(ab)=\bar a\Delta b+\bar b\Delta a\)가 정확하게 성립한다. 이를 \(n_e=wy\)와 \(g=K n_ev\)에 연이어 적용하면

\[
\boxed{\Delta g = K\bigl[\bar v\bar w\Delta y + \bar v\bar y\Delta w + \overline{n_e}\Delta v\bigr]}.\tag{1}
\]

차례로 **ionization (chemistry), baryon density, geometry**라는 algebraic 분할이다. \(\Delta n_b=0\)인 이번 원본 25쌍의 모든 193시점에서는 density term이 정확히 0이고, geometry/ionization 두 항이 \(\Delta g\)를 재구성한다. 여기서 사용된 우선순서는 먼저 \(n_e v\), 그 뒤 \(n_b x_e\)를 분해하는 것으로 고정된다. 대안 순서를 쓰면 교차항의 할당이 달라질 수 있으므로 **인과적 independent intervention**으로 부르지 않는다.

양변을 같은 저장 x그리드에서 적분하면 \(\Delta\tau_T=\Delta\tau_{\rm chem}+\Delta\tau_{\rm density}+\Delta\tau_{\rm geom}\). 연속식은 정확하지만 193점 \(\mathrm{cumulative\_simpson}\)과 저장 solver-BDF \(\tau_T\)의 값 차이는 quadrature/sample-history 오차로 별도 평가한다. 수치 residual이 0이 아니라고 항등식이 깨지는 것은 아니다.

## 3. 미시물리 source와 proper time (DERIVED + IMPLEMENTATION-VERIFIED)

원본 \(dY/dt=F(Y,T,UV,n_b)\), \(J=\partial F/\partial Y\big|_{T,UV,n_b}\)는 원본 `AtomicNetwork.evaluate`를 9650개 저장된 물리점에 적용해 직접 얻었다. \(dx_e/dx=(q^\top F)/H\)이며, 한 쌍의 차이는

\[
\Delta\left(\frac{dx_e}{dx}\right)=\bar v\Delta(q^\top F)+\overline{q^\top F}\,\Delta v.\tag{2}
\]

이 역시 회계적 identity. 두 항이 클 때 고정 193점 quadrature의 소거 오차가 최종 극히 작은 \(\Delta x_e\)에 비해 커질 수 있어 **동역학 차이의 누적 적분으로 복원된 전자량**은 인증하지 않고, 이력을 직접 사용한 식 (1)의 \(\tau_T\) 적분만 핵심 물리 산출물로 인정한다.

이번 matched source의 모든 저장 시점에서 prescribed T/UV와 \(n_b\)가 동일하다. H/He/Li source-native 광이온화 *계수* 7종은 shear 0과 .6 사이 25×193×7=33775회 비교 모두 정확히 동일했다. 그러나 \(Y(x)\), \(H(x)\)가 다르므로 실제 광전자 수율과 복합/DR net electron source, \(\tau_T\)는 달라질 수 있다. 이는 특정 추가 source를 넣은 결과가 아니라 같은 미시물리와 다른 proper-time exposure의 source-conditional 차이다.

흡수 광자 에너지율 \(P_{\rm absorb}\)와 1차 photoelectron kinetic power는 서로 다르며, 이들 어느 것도 full self-consistent gas heating/thermal transfer로 승격하지 않는다.

## 4. 계산/증거 상태

- `derived`: 식 (1), (2); Wolfram `FullSimplify` 식 (1) difference 0. Python 정확한 `Fraction.from_float` 검사는 25쌍×4시점에서 H,n_b,Y 원본 binary64를 유리수로 해석하고 3항 identity를 100/100 검증.
- `numerically-checked`: 원본 50 binary SHA-256+Git blob+NPZ CRC, 9650 native F/J, 25×193 paired instantaneous/cumulative data. 저장 τ 대비 각 case Simpson 최대 차이 6.8131e-8, paired delta Simpson 최대 차이 5.2269e-10. 서로 다른 quadrature/solver history이므로 BDF global error estimate 아님.
- `implementation-verified`: 자체 집중 Python 20/20 (새 script/test), 과거 PR122 다섯쌍의 4개 적분/저장량 모두 exact numerical equality.
- `unresolved`: distinct-author independent R6/C9, full native RODAS source transaction, 실제 CMB 방향/편광 transfer, observational posterior, production. 이들은 별도 DAG admission.

## 5. 결과 해석 및 문헌 범위

25쌍 전체에서 \(\Delta\tau_T<0\), 평균 \(-1.2407565482797477\times10^{-4}\). 저장-grid Simpson 분해에서 평균 geometry 92.07015546%, chemistry 7.92984454%, density 0%. 전자 chemistry 항 절댓값의 약 99.10%가 BASS 전이구간 \(z=11.9106\to6.9373\)에서 축적되며, 최종 \(\Delta x_e\)가 약 \(10^{-12}\)이어도 과거 \(\Delta x_e(x)\)는 약 \(5.18\times10^{-4}\)에 달한다. 따라서 최종 endpoint의 chemistry 기여율 1.7×10^-8 수준과 역사적 chemistry 기여율 약 8%는 동일한 양이 아니다.

관련 독립 원문: Pontzen & Challinor, *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380 (2007), DOI https://doi.org/10.1111/j.1365-2966.2007.12221.x 는 실제 Bianchi polarized radiative transfer와 scattering multipole hierarchy를 다룬다. Fleury, Pitrou & Uzan, *Light propagation in a homogeneous and anisotropic universe*, PRD 91 (2015), DOI https://doi.org/10.1103/PhysRevD.91.043511 은 Bianchi I null geodesic/광학 shear를 다룬다. 이번 BASS homogeneous \(g(x)\)만으로 그 수준의 sky prediction을 생성하거나 관측 자료와 동치로 부르지 않는다.

**후속 조건:** 전단 효과의 물리적 claim을 확장하려면 처방 UV/T에서 벗어난 source closure, 방향/편광 transfer, 독립 R6/C9, source/epoch authentication이 각각 필요하다. 기존 Q-LEDGER, Q-API, Q-GIVENS, 8 suites, 10 interactions, PR117 causal RHS/value-only, old CI failure, tolerances와 resource budget은 수정하지 않는다.
