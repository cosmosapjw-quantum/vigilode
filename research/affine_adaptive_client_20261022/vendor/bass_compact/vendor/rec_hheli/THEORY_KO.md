# 유한 H/He/Li 결합 모형: 물리 루프 R1–R10

## R1–R3: 질문, 근거, 선택

목표는 기존 LRS geometry/particle owner를 유지하면서 실제 21상태 원자망이 만든 전자밀도가 광자 collision으로 들어가는 `z=20→2` preliminary sweep을 샌드박스에서 완주하는 것이다. 최신 source의 전체 목록과 실행 반환은 `provenance/UPSTREAM_SURVEY.json`, backup 대조는 `INPUT_SURVEY.json`에 있다. 완료한 warm729/175/237 또는 기존 검증 case는 새로운 원자 결합 실행으로 재명명하지 않는다.

비교한 후보는 tangent-only 재사용, 완전 thermal/photonuclear 폐쇄, 유한 photo/RR/DR number network 결합이다. 세 번째를 선택한다. 첫 번째는 새 핵종 범위를 해결하지 못하며, 두 번째에는 별도 energy/ionic-yield source 의존성이 남는다. 세 번째는 가정과 오차 미인증 범위를 명시하면서 현재 계수로 실제 방정식을 계산할 수 있다.

## R4: 정의와 범위

평균 scale factor `a`, `x=ln(a/a_ref)`, `a_ref=1/21`, `a_end=1/3`. `a_perp=a exp(-b)`, `a_parallel=a exp(2b)`, `s=db/dt`. Orthonormal frame와 signature `(-,+,+,+)`를 쓴다.

`sigma_ab=diag(-s,-s,2s)`, `pi_A,ab=diag(-p_A,-p_A,2p_A)`. 따라서

\[
\Sigma_F=\sqrt6|s|/H,\qquad
\Pi_{A,F}=\sqrt6\kappa_t|p_A|/H^2,\quad \kappa_t=8\pi G/c^2.
\]

`p_A`는 J m^-3이며 시간은 s이다. 시간단위 Einstein 계수는 `8πG/c²`다. `8πG/c⁴`를 SI energy density와 시간 `H`에 그대로 섞지 않는다. Half-contraction norm은 각 Frobenius norm의 `1/√2`다. signed 좌표 `S_F, P_m, P_r`를 먼저 계산한다. `Pi_total=|P_m+P_r|`; `|P_m|+|P_r|`는 별개 양이다.

원자 상태는 n; H1/D/T 각각 I–II; He3/He4 각각 I–III; Li6/Li7 각각 I–IV다. 전자는 `ne=nb q·Y`, `Y_i=n_i/nb`. 음이온·분자, 짧은 수명의 Li8/9 등 임의의 모든 핵공명·핵종을 포함한다고 주장하지 않는다. 이는 요청 그림의 우주론적 H–Li inventory에 Li6와 완전 이온화 상태를 더한 명시적 21상태 집합이다.

## R5: 실제 유도

기존 finite canonical-momentum quadrature의 수를 고정한다. `24×12` cells, 각 cell Gauss3, 총 2592 positive packets; `m/T_ref=30`, `fm=0.1` 비바리온 분율이다. `Q_perp=q_perp² exp(-2x+2b)`, `Q_parallel=q_parallel² exp(-2x-4b)`, `E=sqrt(m²+Q_perp+Q_parallel)`. 고정 양의 number weights로 rho, P_perp, P_parallel을 합한다. `p_m=(P_parallel-P_perp)/3`이며 직접 미분하면 `rho_b=-6p_m`이다. 기존 source equation, quadrature, cutoff는 바꾸지 않는다.

\[
H^2=H_0^2[\Omega_{dust}/a^3+\rho_{mass,norm}+\rho_{rad,norm}+\Omega_\Lambda]+s^2,
\quad \dot s+3Hs=\kappa_t(p_m+p_r).
\]

CMB photon과 massless neutrino의 energy/number multipoles는 기존 `L=8` 행렬을 각각 사용한다. 광자에만 `-c sigma_T ne D_l/H` 항을 넣는다. `pi_r=pi_gamma+pi_nu`. 지정 UV는 local-frame isotropic reservoir여서 `pi_UV=0`; 동적 CMB/ν와 동일한 photon population으로 합치지 않는다.

원자 반응은 column generator로 적는다.

\[
\dot Y=G_\gamma(T,J)Y+n_e R(T)Y+DY,
\qquad n_e=n_b q^T Y.
\]

각 photo 사건은 `q→q+1`, RR/DR는 `q→q-1`. 모든 off-diagonal은 비음수, 각 column의 합은 0, `A^T G=0`이다. 원자 연쇄별 핵종 수는 보존된다. 실제 캠페인은 n=T=0이므로 약붕괴 flux도 0이다. 별도 검증은 `n→HII+e`, `TI→He3II+e`, `TII→He3III+e`의 spectator-electron continuum-beta 근사를 검사한다. 핵붕괴 에너지의 완전 장부는 이 원자 모델에 포함하지 않는다.

정확한 finite-model abundance Jacobian은

\[
J_Y=G_\gamma+D+n_eR+n_b(RY)q^T.
\]

마지막 rank-one 항이 전자밀도의 되먹임이다. 이 항을 없애는 반례 검증이 오차를 검출한다. 각 isotope inventory `w_i`로 `v_i=Y_i/w_i`를 적분하고 `J_v=diag(w)^{-1}J_Ydiag(w)`를 사용한다. Li6의 작은 실제 abundance가 H 규모의 absolute tolerance에 지워지는 것을 막는다. n/T는 zero inventory지만 유한 reference scale을 사용하며 실제 abundance는 0이다.

기하/원자/광학깊이를 함께 적분하는 state는 40+21+1=62개다. BDF에 analytic block Jacobian을 제공한다. 전체 62×62 행렬은 작은 stiff Newton 문제를 위해서만 만들며, 각 RHS에 momentum grid 또는 데이터 표를 재구축하지 않는다.

## R6: 지정 T/UV와 열린 에너지 장부

이번 preliminary 입력은 `Q(z)=[1+tanh((7-z)/0.8)]/2`, `T(z)=100+11900Q K`, UV amplitude `Q`다. 5–200eV에서 number spectrum `E^-2.5`, threshold를 나눈 71개 bin, late `Gamma_HI=1e-12 s^-1`로 정규화한다. 이는 관측에 fit한 EoR 이력이 아니다. 원자 소스의 전자 Maxwellian 온도 영역 안에서 계산한다.

바리온 mixture는 common-T isotropic ideal gas다. `n_g=n_nuc+ne`, `p_b=n_g k_B T`, `u_th=3p_b/2`이고 `pi_b=0`. 따라서 이 finite closure의 전체 matter anisotropic stress는 별도 collisionless 성분의 stress다. 바리온 thermal/ionization energy와 UV energy는 Einstein source에 추가하지 않는 test-fluid 근사이며 실제 `trace_energy_fraction`을 저장한다. 그 작은 비를 자동 global dynamical error bound로 해석하지 않는다.

중성 기준 누적 ionization energy `I_i`와 atomic reaction derivative를 쓰면, n/T flux가 0인 본 실행에서 필요한 **순 gas input**은

\[
Q_{net}=\frac32 k_B n_g(\dot T+2HT)
+\frac32 k_BT R_e+\sum_i I_iR_i,
\quad R_i=n_b\dot Y_i,\quad R_e=\sum_iq_iR_i.
\]

`bath_minus_escape=Q_net-Q_photo_abs`도 저장한다. 이것은 heater와 escaping emission을 각각 결정하지 않는다. RR number coefficient로 재결합 전자의 운동에너지 loss를 임의로 추정하지 않는다. Li I 1s의 흡수는 primary electron `E-64.39eV`, ionic threshold `5.392eV`, 미해결 core excitation `64.39-5.392eV`로 나누어 기록하며 core energy를 즉시 heat로 넣지 않는다.

## R7: 외부 source 재사용

- Verner et al. (1996) ground-state photo fits, Verner–Yakovlev (1995) partial-shell fits 및 원 author Fortran: https://www.pa.uky.edu/~verner/photo.html . Li I의 64.39eV 위에서는 PR151 source의 PH1 1s+2s를 합한다. 그 아래는 PH2 2s다. 기존 PR151 저장 검증은 91.838eV까지이며 이번 200eV formula check는 새 검증이다.
- H/He RR: Verner–Ferland (1996) selected rows, https://www.pa.uky.edu/~verner/rec.html . Li RR/DR: pinned PR150 Badnell coefficient rows; https://amdpp.phys.strath.ac.uk/tamoc/DATA/RR/ 와 DR source. 단위 cm³/s는 m³/s로 `1e-6` 변환한다.
- 기존 Git source의 계수와 isotope identity를 재사용하지만 H/D/T, He3/He4, Li6/Li7의 electronic equality는 근사다. 실제 isotope-dependent fit이라고 하지 않는다.
- Li I inner-shell 흡수를 한 전자/한 charge-step에 대응시키는 것은 이 모형의 명시적 coarse-graining이다. 단면적 source가 전체 ionic/Auger/fluorescence yield까지 인증한 것은 아니다.

## R8–R9: 점검, 반례, 한계

Wolfram CAS는 generator column sums, trace-free tensor norm, cancellation counterexample와 `Q_net`의 대수 항등식을 확인했다. `atomic_verification.json`은 소스/계수·21종 Jacobian·양의 frozen generator·에너지 분해를 검사한다. `COUPLED_VERIFICATION.json`은 실제 geometry–ne coupling Jacobian과 gamma-only collision을 검사한다. 이는 ODE global error certificate가 아니다.

강한 물리적 no-go는 주장하지 않는다. 빠진 CI/CX/secondary electron/분자/complete thermal transfer는 현재 finite model 밖의 과정이다. 이를 포함하는 모델이 불가능하다는 뜻도, 생략 오차가 인증됐다는 뜻도 아니다. 200eV UV는 대상 primordial nucleus의 광분해 문턱보다 낮으므로 이 선택 source의 photonuclear channel은 활성화되지 않는다. 열핵반응/BBN network는 별도이며 본 실행은 BBN abundance 생성 계산이 아니다.

## R10: 실행 계약과 다음 단계

물리축은 실제 Frobenius 초기값의 직교격자다: `Sigma=[0,.003,.03,.15,.6]`, signed `Pm=[-.0005,-.00025,0,.0005,.001]`, `Pr=[-.0012,-.0006,0,.0006,.0012]`; 125개 case. source positivity에서 허용되는 `A_m,A_r`로 각 case를 역변환한다. 저장점 193개, 전구간 `x=0..ln7`. 같은 source/data/config hash의 완료 결과만 재사용한다.

다음 DAG는 실제 신규 coupled pilot, four-figure 산출, 독립 code/plot 감사다. `Physical HOLD`, `global convergence NOT_RUN`, `production NOT_APPROVED`, 기존 REC U5 HOLD와 `REI_activation=false`를 유지한다.
