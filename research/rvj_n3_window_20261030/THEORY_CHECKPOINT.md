# RVJ N3: exact-real nonlinear error transport across two stiff slow/fast windows

**2026-10-30 KST. Research-only next node (N3) of RVJ loop12 follow-on.**

Parent source: [N1 PR135](https://github.com/cosmosapjw-quantum/vigilode/pull/135), [N2 PR136](https://github.com/cosmosapjw-quantum/vigilode/pull/136), frozen N2 HEAD \`50672d52260129767da58420490bdd145f1cec3a\`. N2 source ZIP SHA-256 \`5f7a77bfdd3b2e4ba031c2085d0f70be76b6a90872d975a85effb0ee5afb15db\`.

Exact model: \`a_j'=(-kappa_j+i omega xi_j)a_j + i sum_alpha chi_alpha phi_alpha,j |m_alpha|^2 m_alpha\`, \`m_alpha=sum_k w_k phi_alpha,k a_k\`. Real couplings/basis, nonnegative kappas, positive normalized rational weights, \`sum_k w_k phi_alpha,k^2 <= 1\`, both exact trajectories start with energy \`sum w_k |a_k|^2 <=1\`. Therefore exact \`E'=-2 sum_j w_j kappa_j |a_j|^2 <=0\`.

For two exact trajectories define nonnegative \`L_jk=3 sum_alpha |chi_alpha phi_alpha,j| w_k |phi_alpha,k|\`, \`Q_j(h)=min(h,1/kappa_j)\` (or h if kappa=0), \`q=max_j Q_j sum_k L_jk <1\`, initial differences \`d_j\`, \`D=max d_j\`. Then from the original Volterra mild form and cubic Lipschitz inequality, \`sup_{0<=s<=h} |a_j(s)-a'_j(s)|<=D/(1-q)\` and
\`\`\`
d_j(end) <= d_j/(1+kappa_j*h) + Q_j(h)*(sum_k L_jk)*D/(1-q).
\`\`\`
The rational exponential inequality \`exp(-x)<=1/(1+x)\` for x>=0 was also checked symbolically with Wolfram.

N2 exact-ideal one-Picard local error at each fresh accepted mathematical carrier is \`B_j=3R^2 Q_j sum_alpha |chi_alpha phi_alpha,j|(b_alpha + A_alpha*tau_p)\`, with \`tau_p=(|omega|h)^(p+1)/(p+1)!\`, \`A_alpha=sum w_j|phi_alpha,j|(|Re a_j(0)|+|Im a_j(0)|)\`, \`e=max_j(|Re a_j(0)|+|Im a_j(0)|)\`, \`R=max(e,A_alpha(1+tau_p))<=1\`, \`b_alpha=e^3 sum_beta|chi_beta|sum_jw_j|phi_alpha,j phi_beta,j|Q_j\`.

Therefore the **exact-real, ideal-arithmetic** causal window transport:
\`\`\`
d_j[next] <= B_j + d_j[old]/(1+kappa_j*h)
             + Q_j*(sum_k L_jk)*D_old/(1-q),    D_old=max_k d_k[old].
\`\`\`
This is a source-specific proof of error transport for **exact** Picard candidate states, NOT a bound for binary64 evaluated Rust states until outward rounding/representation/evaluation errors are established. For kappa_s=1/4, kappa_f=1000, chi=(1/20,1/50), phi1=1, phi2=(-1)^j, h=1/4, p=2, the exact contraction figure is \`q=21/400\`. The fast initial envelope factor is <=1/251, versus slow <=16/17, while no kappa_f^-1 microstep is required.

RVJ scoped DAG after N3 = **3/8**. Official VigilODE DAG **7/23**, central REI **28/88**, successor admitted **0/12**, C9 separate author **0/1 NOT_RUN**, C10 NOT_STARTED, production HOLD, auto next false. No original 8-stage nonlinear RODAS5P parallelization is claimed.

**Stage checkpoint:** R1-R5 and C1-C4 research contract/derivation recorded; R6 self adverse check and C5-C8 evidence in next commit. Full executable Rust source, reference and exact Fraction oracle live in separate source ZIP; GitHub text branch is not source-complete until exact bundle provider readback is linked.