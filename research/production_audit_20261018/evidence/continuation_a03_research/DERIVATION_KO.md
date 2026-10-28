# A03 local common-damping packet 계약 연구

상태: **derived, independent review pending**. 2026-10-18 continuation의
`A03-LOCAL-DAMPED-CONTRACT-01`이다. Rust 구현·테스트·빌드는 실행하지 않았다.
이 문서의 구현 계약은 제안이며 현재 공개 API로 인용하면 안 된다.

## 1. 입력 identity와 재사용 범위

읽은 checkout은 `/home/cosmosapjw/vigilode`, branch
`audit/production-readiness-20261018`, HEAD
`7e2873887ed6038b47b667c976435810851c5dba`, tree
`9275e35d3d5afe1ae4e40e1c0d9aca1034994ca6`이다. 연구 입력 소스는 HEAD와
일치한다. 다른 owner의 dirty A05/A02/A04 및 DAG/evidence는 그대로 보존했다.
전체 입력 hash는 `RESEARCH_DECISION.json`에 고정한다.

재사용하는 것은 (i) Loop12 `DAMPING_WINDOWS_KO.md`의 DW-A..F와 실제
`EXTENSION_REVIEW.json`이 승인한 조건부 수학, (ii) production audit T1/T3,
(iii) 현재 source hash가 과거 review와 일치하는 scalar forcing consumer,
(iv) A01-LCERT-01의 immutable owner/returned-center 원칙이다. DW 문서의
정규화 합계 1은 현재 저장 weights의 exact sum `S`로 일반화하여 아래에서
직접 유도한다. 이전 campaign이나 scalar 회귀를 재실행하지 않았다.

## 2. 실제 native 모델과 좌표계

현재 `CollectiveModel::rhs`는 **rotating-frame** 식

\[
 a_j'=(-\kappa+i\omega_j)a_j+g(m_a),\qquad
 \omega_j=\mathrm{width}\,\xi_j,\quad
 m_a=\sum_jw_ja_j,\quad g(z)=i\chi|z|^2z
 \tag{A3.1}
\]

을 생성한다. `carrier()`는 RHS/JVP에 들어가지 않는 metadata다. 새 packet도
이를 임의로 더하면 안 된다. 모든 파라미터는 받은 binary64 입력의 정확한
실수값으로 해석하고, `width*xi`의 nearest product를 exact frequency로
치환하지 않는다. `S=Σw_j>0`, `X=max|xi_j|`, `Ωmax=width*X`는 실제 저장
입력에서 outward 계산한다. 현재 admission의 `sum≈1`, `|xi|≈1` 검사는
이 proof에서 `S=1`, `X=1`을 뜻하지 않는다.

유한 N, w_j>0, κ≥0, 실수 χ, 공통 κ, 고정 ω_j, identity mass가 범위다.
불확실한 물리 파라미터나 서로 다른 κ_j는 별도 uniform witness 없이는
입장하지 않는다. κ,ω의 단위는 time^-1, χ는 amplitude^-2 time^-1이다.

한 local window의 시작값 z_j와 모듈러스 uncertainty η_j를 복사해 고정한다.
정확한 weighted norm은 `||v||_w²=Σw_j|v_j|²`이고, outward

\[
 E_+\ge\sum_jw_j(|z_j|+\eta_j)^2,\qquad
 e_{0,w}\ge\sqrt{\sum_jw_j\eta_j^2}
 \tag{A3.2}
\]

를 계산한다. 기존 unit-mean 적용범위를 계승하는 첫 native contract는
**outward S_hi E_+≤1**을 요구한다. 이 threshold를 바꾸어 성공시키지 않는다.

`a(s)=exp(-κs)b(s)`, `u=Σw_jb_j`, `ρ(s)=exp(-κs)`를 대입하면

\[
 b_j'=i\omega_jb_j+e^{-2\kappa s}g(u),\qquad
 E_b(s)=\sum_jw_j|b_j(s)|^2=E_b(0),\quad |u|^2\le S E_+.
 \tag{A3.3}
\]

에너지 증명은 `Re(Σw conj(b_j) iω_jb_j)=0`와
`Re(conj(u) iχ|u|²u)=0`이다. Σw=1을 쓰지 않는다. 계산에서 작아진 a에
exp(+κs)를 곱해 b를 복원하지 않고 b를 직접 표현한다.

외부 lab 출력은 명시적으로 요청한 경우에만
`z_lab(t0+s)=exp(iΩs)a(s)`로 정의한다. 이 정의는 시작값 z_j가 t0의 lab
값일 때의 **local carrier reset**이다. 기존 global rotating 상태를
`exp(iΩt_global)`로 바꾸는 옵션은 origin과 입력 phase convention까지
다른 계약이다. 두 convention을 자동 교환하지 않는다. 비영 Ω의 verified
phase enclosure가 없는 현재 소스는 lab certificate를
`UnsupportedFrameCertificate`로 거절해야 한다. Rotating 출력은 carrier
metadata를 바꾸어도 같은 방정식/출력을 가진다.

## 3. DW homotopy와 필요한 정확한 forcing integral

`h>0`, `τ=s/h`, `ζ=κh`라 하고, 실제 μ_k=Σw xi^k, ν_k=Σw xi^k z_j로
F_p,K_p를 만든다. exact detuning polynomial을 쓰는 수학식은

\[
 K_p(s)=\sum_{k=0}^p\mu_k(i\,\mathrm{width}\,s)^k/k!,\quad
 F_p(s)=\sum_{k=0}^p\nu_k(i\,\mathrm{width}\,s)^k/k!.
 \tag{A3.4}
\]

`r=Ωmax h`, `A0=Σw|z_j|`이면 `||K_p||≤S exp(r)`,
`||F_p||≤A0 exp(r)`. DW8의 U_l를 같은 target λ=1에 대해 사용한다.
η_hom>1을 homotopy coefficient norm의 반경으로 고르면 충분조건은

\[
 M=A_0e^r+\eta_{hom}S|\chi|e^r\theta_\kappa(h)B^3<B,
 \quad q=3\eta_{hom}S|\chi|e^r\theta_\kappa(h)B^2<1.
 \tag{A3.5}
\]

이는 기존 DW-B 증명에서 kernel norm의 S만 복원한 것이다. 최종 U_Q는
exp(-2qκs)P_q(s)의 유한합이고 보수적 degree는
`D=(3Q+1)p+Q`, exponent index≤Q이다. 이 표현의 scalar precision/setup
비용을 κ와 무관하다고 하지 않는다. expanded coefficient의 κ^-k 상쇄를
피하려면 κ=0 branch와 **confluent integral graph 또는 동등하게 검증된
stable representation**이 필요하다. 단순 `if κ<epsilon { κ=0 }`은 금지다.

forcing g(U_Q)의 exponent index q는 0..3Q이며 integrand에는 추가
exp(-2κs)가 있으므로 필요한 rate는 **2κ(q+1)**이다. 요구 scalar 항은

\[
 Q_{r,d}(a,t)=\int_0^t(t-s)^r s^d e^{-as}\,ds
             =I_{d,r}(a,t),\quad a=2\kappa(q+1).
 \tag{A3.6}
\]

기존 consumer의 index 교환은 맞다. 예를 들어 r=0,d=1,a=2에서
`Q=(1-(1+2t)e^{-2t})/4`이고, 감쇠 지수를 반대 끝에 둔 I_{0,1}과 다르다.
그러나 이 consumer는 **전달된 단일 f64 a**의 exact integral을 감싼다.
`a=fl(2κ(q+1))`을 넘기면 원래 target integral의 인증이 되지 않는다.

가장 작은 해결은 interval A=[a_lo,a_hi]로 exact rate를 감싼 후

\[
 Q_{r,d}(a,t)\in[Q_{r,d}(a_{hi},t).lo,
                    Q_{r,d}(a_{lo},t).hi]
 \tag{A3.7}
\]

를 쓰는 것이다. 양의 integrand에서 ∂_a Q≤0이므로 증명된다. normalized
변수에서는 `a=2(q+1)ζ`, horizon τ로 쓰고 마지막 h factor/polynomial scale을
원래 식에 맞게 charge한다. exact τ=t/h가 interval T=[τ_lo,τ_hi]로 주어지면
Q는 horizon에 대해 비감소하므로 lower에 τ_lo, upper에 τ_hi를 사용한다.
τ_lo=0의 Q는 exact zero로 처리한다. 이는 양의 **시간 구간이 0으로
반올림되었다**는 경우와 구별한다. 모든 rate/time product, powers, beta,
coefficient multiply/sum와 endpoint hull도 outward다.

현 core의 order cap은 각 index≤64다. `p=8,Q=10`의 이론 예에서는 D=258,
forcing degree≤774이므로 현 scalar table이 이를 모두 지원한다고 하면
틀리다. 새 maximum degree 계약 없이 조용히 truncate하지 말고
`UnsupportedMomentOrder {required,supported}`를 반환한다.

### 폭이 넓은 large-rate recurrence의 안정한 대안

양의 normalized integrand를 dyadic cells [l,u]⊂[0,T]로 나눈다. δ=u-l,
고정 horizon T에서 cell의 polynomial mass는 정확히

\[
 P_{l,u}=\sum_{i=0}^r\sum_{j=0}^d
 {r\choose i}{d\choose j}(T-u)^{r-i}l^{d-j}
 \delta^{i+j+1}B(i+1,j+1)\ge0.
 \tag{A3.8}
\]

증명은 s=l+δv, T-s=(T-u)+δ(1-v)를 양의 이항 전개하여 적분하는 것이다.
cell integral은 `exp(-a u)P_{l,u}`와 `exp(-a l)P_{l,u}` 사이이다. interval
rate라면 lower에는 a_hi, upper에는 a_lo를 쓴다. 모든 합이 비음수여서
alternating cancellation과 1/a division이 없으며 a=0에서 beta integral로
돌아간다. 정확한 exp 값일 때 mesh→0이면 연속 integrand의 Darboux 상하합이
같은 적분으로 수렴한다. 실제 finite interval에서는 exp 폭과 arithmetic
floor가 남으므로 width gate/work cap 이후 typed refusal을 허용한다.

이 fallback은 **derived, not implemented**다. 먼저 기존 kernel을 사용하고
width가 고정 예산을 넘을 때만 적용한다. 임의 작은/큰 rate의 빠른 성공이나
zero-allocation은 아직 증명·실행하지 않았다. 현재 large-z kernel은 매 호출
`e_table=vec![...;j+k+1]`를 만든다. warmed query 계약에는 caller-owned scratch
또는 출력시각별 setup cache가 필요하며 allocation 양성 대조군이 필수다.

## 4. 실제 저장 곡선과 continuous residual

source identity를 가진 private owner가 각 mode의 **저장된** point polynomial
`P_j(τ)=Σ_l b_{j,l}τ^l`(b_j0=1), candidate 함수 U(τ), h,κ,w,ω,z를 소유한다.
`b_{j,l}`는 rounded f64 coefficient일 수 있다. candidate 생성의 반올림은
U 자체의 exact representation 의미와 아래 residual에 결속한다. 검증하지
않은 이상 theoretical U_Q와 bitwise 같다고 가정하지 않는다.
식 (A3.5)의 M,q는 exact basis의 수학적 충분조건이다. rounded basis/coefficients에
그 bound를 그대로 붙이지 말고 basis/representation discrepancy를 포함하거나,
actual U의 전 구간 tube와 actual residual을 직접 검증한다.

실제로 인증할 transformed 곡선을 다음 exact expression으로 정의한다.

\[
 Q_j(\tau)=P_j(\tau)z_j+
 h\int_0^\tau P_j(\tau-v)e^{-2\zeta v}g(U(v))\,dv,
 \quad M_Q=\sum_jw_jQ_j,\quad Y_j(\tau)=e^{-\zeta\tau}Q_j(\tau).
 \tag{A3.9}
\]

공유 forcing를 rounded coefficient로 바꾸었다면 그 차이를 enclosure에 넣거나
해당 rounded forcing 자체를 곡선 정의로 고정하여 아래 residual을 재계산한다.
반올림을 두 경로 사이에서 없어진 것으로 간주하지 않는다. 실제 저장 P의
weighted 합으로 K*,F*를 구성할 때

\[
 r_U=U-F^*-hK^**(e^{-2\zeta\cdot}g(U)),\qquad
 M_Q=U-r_U
 \tag{A3.10}
\]

이다. 별도 가정으로 candidate U와 M_Q를 교환하지 않는다. `|U|≤B`,
`|r_U|≤D`를 **전 구간** interval 계산으로 보이면 `|M_Q|≤B+D`이다.
더 좁은 M_Q bound를 actual curve에서 직접 얻어도 된다.

normalized basis defect를

\[
 A_j(\tau)=P_j'(\tau)-i(h\omega_j)P_j(\tau)
 \tag{A3.11}
\]

로 둔다. 먼저 원래 모델에 대한 **전체 physical residual**을 고정한다.

\[
 R_{Y,j}=Y_j'-[(-\kappa+i\omega_j)Y_j+i\chi|M_Y|^2M_Y]
 =e^{-\kappa s}\{Q_j'-i\omega_jQ_j
                  -i\chi e^{-2\kappa s}|M_Q|^2M_Q\},
 \quad M_Y=e^{-\kappa s}M_Q.
 \tag{A3.11b}
\]

여기서 Q'는 physical time 미분이다. 아래 𝓡에서는 τ 미분을 쓴다.
A_j는 **linear basis defect만** 뜻하며 nonlinear forcing defect를 포함하지
않는다. 전체 residual에는 다음 식의 마지막 nonlinear difference가 반드시 남는다.

\[
 \mathcal R_j(\tau)
 =A_j(\tau)z_j+h\int_0^\tau A_j(\tau-v)e^{-2\zeta v}g(U(v))dv
 +he^{-2\zeta\tau}[g(U(\tau))-g(M_Q(\tau))].
 \tag{A3.12}
\]

미분은 Leibniz rule과 P_j(0)=1을 사용한다. physical residual은
`R_Y(s)=exp(-κs) 𝓡(s/h)/h`이다. 실제 저장 coefficient에서 A를 outward
계산하므로 basis truncation과 생성 반올림이 모두 남는다. exact Taylor P인
특수 경우에만 `A_j=-(ihω_j)^(p+1)τ^p/p!`로 줄어든다. 마지막 nonlinear
차이는 `3|χ|(B+D)^2 D`로 보수적으로 감쌀 수 있다. coefficient/force,
integral, sum, norm 계산의 나머지 폭도 함께 포함한다.

`1/h`를 먼저 계산하면 tiny positive h에서 overflow할 수 있다. 실제 인증은
𝓡를 normalized 좌표에서 직접 계산하고 `r_scaled≥sup||𝓡||_w`를 사용한다.
쿼리 τ의 interval까지 포함한 전 prefix/cell을 검증하고 point/midpoint 표본을
continuous residual로 취급하지 않는다.

## 5. T1의 factor 1과 forcing의 factor 3

u=(x,y)에 대해 g의 real Jacobian은

\[
 Dg(u)=\chi\begin{pmatrix}-2xy&-x^2-3y^2\\3x^2+y^2&2xy\end{pmatrix},
 \quad H=\tfrac12(Dg+Dg^T)
 =\chi\begin{pmatrix}-2xy&x^2-y^2\\x^2-y^2&2xy\end{pmatrix}.
 \tag{A3.13}
\]

H²=χ²|u|⁴I로 symmetric eigenvalues는 ±|χ||u|²이다. 반면 Dg의 singular
values는 |χ||u|²와 3|χ||u|²다. 따라서 factor 3은 `|g(u)-g(v)|`의
Lipschitz/forcing bound에 속하고 weighted one-sided state gain의 계수가 아니다.

collective state perturbation v에서 δm=Σw_jv_j이면 nonlinear quadratic form은
`δm^T H δm`. `|δm|²≤S||v||_w²`와 각 detuning block의 weighted skew성을 쓰면

\[
 \mu_w(Df(a))=-\kappa+S|\chi||m_a|^2.
 \tag{A3.14}
\]

등호는 모든 mode perturbation을 H의 양의 고유벡터 방향으로 같게 두면 얻는다
(χ|m|=0인 퇴화 경우도 성립). 이 식은 채택된 T1과 동일하다.

true a와 actual curve Y 사이 segment의 mean은 convexity에 의해
`|m_segment(s)|≤exp(-κs) B_seg`,
`B_seg²=max(S E_+,(B+D)²)`로 감싼다. 이로부터

\[
 L_1=S|\chi|B_{seg}^2,\quad
 G(t,s)=\exp[-\kappa(t-s)+L_1(\theta_\kappa(t)-\theta_\kappa(s))],
 \quad\theta_\kappa(t)=\int_0^t e^{-2\kappa v}dv
 \tag{A3.15}
\]

를 얻는다. 두 **exact flows**의 시작 energy가 모두 E 이하이면
`B_seg²=S E`, 즉 gain은 정확히
`exp[-κh+S²|χ|E θκ(h)]`로 T1과 일치한다. candidate Y를 exact flow로
바꾸어 생각하여 E만 대입하면 안 된다.

𝓡의 전 구간 상한 r_scaled에 대해 actual curve의 local weighted radius는

\[
 \|a(t)-Y(t)\|_w\le B_w(t):=\exp[-\kappa t+L_1\theta_\kappa(t)]
          [e_{0,w}+\tau\,r_{scaled}],\quad\tau=t/h.
 \tag{A3.16}
\]

증명은 T1 residual integral에
`R_Y=exp(-κs)𝓡/h`를 넣고 `θ(t)-θ(s)≤θ(t)`로 상계한 것이다.
scaled formulation은 작은 h에서 1/h 계산을 요구하지 않는다. θ는 normalized
Q_00 adapter 또는 validated expm1/φ1로 계산하고, tiny κ를 exact zero로
분기하지 않는다. `exp(-κt)`와 `exp(L1 θ)`를 각각 계산하여 0*Inf를 만드는
대신 **합친 exponent**를 outward 평가한다. overflow는 typed refusal다.
구현이 반환하는 base radius는 위 실수식 B_w의 outward **상한 이상**이어야 한다.

비교를 위해 기존 DW Volterra 증명은 true transformed mean과 U 사이에
`exp(3 S|χ|B² θ)`를 준다. 이는 memory equation에서 `|K|≤S`와 cubic
Lipschitz norm을 쓴 다른 부등식이다. T1 state-flow gain을 그 식으로
교체하지 않는다. 이번 native local output authority 계약은 (A3.16)이다.

## 6. 반환한 중심, mean, frame

outward evaluation boxes가 **바로 위 exact Y(t)**를 감싸고, 반환할 binary64
center c_j와의 displacement upper를 δ_eval,j로 계산한다. 반환 radius는
`B_w/sqrt(w_j)+δ_eval,j` 이상이다. mean center는 반환 mode centers의 실제
weighted sum 또는 그 outward evaluation으로 만들며, `sqrt(S)B_w`에
exact reconstructed mean과 반환 mean center 사이 displacement를 더한다.
mode disks를 weighted 합한 mean bound와 최소를 취할 때는 두 bound가 모두
같은 returned mean center를 감싸는지 확인한다.

반환 center를 위한 weighted radius는
`B_w+sqrt(Σw δ_eval,j²)` 이상이다. A01의 `weighted_radius()`가 base B_w를
뜻한다는 주의사항을 그대로 계승하며 evaluation displacement를 생략하지 않는다.
query/refusal은 owner의 accepted data를 바꾸지 않고 모든 시도 work를 누적한다.

lab phase enclosure `|c_phase-exp(iΩt)|≤ε_phase`를 제공할 때는 transformed
값의 arithmetic box와 위상곱을 outward 실행하거나 별도로
`|Y_j|ε_phase` 및 최종 multiply displacement를 더한다. phase가 rounding으로
0이 되었다는 이유로 exact unit phase라 하지 않는다. 아직 phase kernel이 없으므로
그 경로는 명시적 refusal 상태로 남는다.

## 7. 실행 가능한 다음 단위와 남은 의무

가장 작은 현재 구현 단위는 **A03-SCALAR-RATE-01**이다. 기존 scalar consumer에
정확한 rate/time interval endpoint hull, caller scratch, width/work gate를 붙이는
독립된 finite contract다. 수학 근거는 (A3.6)–(A3.8)이고 독립 검토 전에는
READY가 아니라 `READY_FOR_INDEPENDENT_CONTRACT_REVIEW`다. 과거 scalar scoped
PASS를 다시 실행할 이유는 새 adapter가 실제 kernel 의존성을 바꾼 경우뿐이다.

다음 local unit은 immutable candidate/curve의 stable confluent representation과
(A3.10)–(A3.16)의 실제 outward implementation이다. 현재 source에는 이것이 없다.
일반 DW p8,Q10 지원은 >64 order fallback 없이는 HOLD다. first bounded contract의
p2,Q2 fixture를 통과했다고 일반 degree 지원으로 확대하지 않는다.

local κ>0 inclusion은 A02의 multiwindow 구현을 선행조건으로 필요로 하지 않는다.
A01-LCERT의 소유권/실제 residual 원칙과 현재 scalar kernel을 재사용하여 독립
subnode로 진행할 수 있다. 그러나 **damped trajectory**는 local pass, A02의
accepted-state/weighted-seed/physical-time/rejection 계약을 κ>0로 확장한 검토와
native 실행을 모두 요구한다. κ=0 A02 pass만으로 damped multiwindow를 인정하지 않는다.
speed/automatic dispatch/전체 production 승격은 별도 gate에 남는다.
