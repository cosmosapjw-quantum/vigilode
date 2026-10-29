# N01 robust original-residual theorem (PR106 child; 2026-10-29)
**Conditional research, not native production certification.**
For represented J0,b0, intended J*,b*, A*=I-sigma J*, sigma>=0, positive fixed w,p, and independently verified bounds ||J*-J0||_{w,inf}<=dJ, ||b*-b0||_{w,inf}<=db, ||y-(I-sigma J0)xhat||_{w,inf}<=eta, use
rho_bound=||b0-y||_{w,inf}+db+eta+sigma*dJ*||xhat||_{w,inf}.
mu_w(J*)<=mu_w(J0)+dJ. If sigma*(mu_w(J0)+dJ)<1 then
||xhat-x*||_{p,inf} <= max_i(w_i/p_i) * rho_bound/(1-sigma*(mu_w(J0)+dJ)).
Direct proof: r*=(b*-b0)+(b0-y)+(y-A0*xhat)+sigma*(J*-J0)*xhat; apply triangle inequality, logarithmic norm semigroup estimate and resolvent integral. In exact represented point mode, y=A0*xhat is evaluated with outward intervals and eta=0. The stage requires outward rho_up <= original tolerance AND outward forward_up <= physical epsilon_stage. Projected Givens and left-preconditioned residual are triggers only. Native `residual_scale` is WRMS (Some) or L2 (None), NOT the weighted-infinity theorem norm.
Wolfram exact 2x2 nonnormal example: J0=[[-2,8],[1/8,-3]], w=[8,1], p=[4,2], sigma=1/4, dJ=1/32; mu0=-1, K=128/159, C_wp=2. For xhat=[1/2,-1/4], source and callback perturbations, exact rho=157/64 <= 10065/4096, exact physical error 940/1311 <= 3355/848. Mathematica inequalities True, True.
Model/JVP/source/clock/epoch/norm and uncertainty-owner provenance NOT established by standalone Rust. Prior PR103 chemical6 and PR108 symbolic-cell results remain sibling conditional evidence. References: Saad & Schultz 1986 doi:10.1137/0907058; Simoncini & Szyld 2003 doi:10.1137/S1064827502406415; van den Eshof & Sleijpen 2004 doi:10.1137/S0895479802403459; Wathen 2007 doi:10.1080/00207160701355961.
