# D1: endpoint domain admission with unchanged stages

Fixed affine compact30/full33 domain D: all z_j>=0, z23<=1, z24+z25<=1/2, z26+z27+z28<=the exact real encoded by binary64(0.2). Exact group inequalities are evaluated by integer accumulation in units2^-1074. Three finite nonnegative inputs sum below2^2100, within33u64words. The actually returned reconstructed full33 state is separately checked for negative values. Validate all supplied compact/physical inputs before any early rejection. Signed zero is allowed without rewriting.

If z0 belongs to D, and every commit requires unchanged norm approval AND exact domain approval, induction guarantees each committed represented endpoint belongs to D and its returned full33 vector is nonnegative. This is not a theorem about internal stages, dense output, actual ODE error or finite retry termination. Rejected trials preserve accepted state/history and charge work. Halving h is a bounded heuristic, not an unconditional positivity method. No clipping/projection/new weights/exact-flow fallback.

A mechanism beyond roundoff: evaluate the canonical raw-U8-stage scalar linear recurrence using exact rational interpretations of stored binary64 coefficients. For x=h*lambda, u_i=gamma/(1+gamma*x)[-x(1+sum A_ij u_j)+sum C_ij u_j], R(-x)=1+sum b_i u_i. R(-4)>0 and R(-8)<0 with R(-8) approximately-0.023374915029575196. Signed bisection brackets a crossing near5.179489120755712. No global first-root claim is made. A tiny population can cross sign while its embedded absolute error remains acceptable.

Manufactured model z'=-lambda*z-alpha*z²,lambda>0,alpha>=0 has invariant nonnegative independent components and nondecreasing dependent populations. CAS checked group sums{1,1/2,c},zero boundary0,dependent derivative lambda*u+alpha*u²+lambda*v+alpha*v² and exact Riccati residual0. This is not a program proof.

Selected alternative is rejection instead of weight adjustment, logarithmic coordinates or clipping. Nusslein/Ranocha/Ketcheson arXiv2005.06268 uses adaptive RK weights and linear programming; contextual primary source, not a proof of this reject-only strategy.

Frozen new experiment:alpha0/4,tol1e-5/1e-8,legacy/nonnegative policies,rate1/20/300,t=[0,.8],outputlandings.1/.2/.4/.8,h0.05,cap10000. Eight new configurations, not a replay of old18campaign. All accepted endpoints and rollback are checked. Legacy policy is compared bitwise with the old Session on the same new input stream. MP80 output error ceiling20tol is a finite test, not a global certificate. No external solver speed comparison.
