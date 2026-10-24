# D2: pre-RHS domain safety

Fixed PR84 compact30/full33 domain checks precede every guarded RHS call. Endpoint positivity alone is insufficient. For scalar y'=-lambda*y, x=h*lambda and d=1+gamma*x, let u_i=N_i/d^(i+1), with indices starting0. Then P_i=d^i+sum_(j<i) A_ij*N_j*d^(i-j-1) and N_i=gamma[-x*d^i+sum_(j<i)(C_ij-A_ij*x)*N_j*d^(i-j-1)]. RHS input multiplier is P_i/d^i. Endpoint numerator is d^8+sum_i b_i*N_i*d^(7-i).

Exact Fraction interpretations of the pinned binary64 coefficients produce nonnegative Bernstein coefficients for P_i and d^i-P_i on[0,2], for all8stages, and for endpoint and its gap. Hence each multiplier lies in[0,1] in exact arithmetic on this scalar-linear interval. Actual coefficient certificates are in the companion STAGE_CERTIFICATE.json, not merely sampled signs.

At x=4 stage2 multiplier(1-2gamma*x)/(1+gamma*x)=-0.3764039563837087 while endpoint R(-4)=0.016497504408650183>0. This is the new pre-RHS witness. Wolfram separately checks the stage2 algebra/rational sign and Bernstein partition, not full Rust correctness.

Selected proposal-only cap:1.8/max_j(lambda_j+2alpha*z_j). For alpha0 this uses90%of the certified linear interval; for alpha4 the frozen-Jacobian scale is heuristic. Runtime checks remain authoritative, and compiled roundoff is not enclosed by the exact-rational linear theorem. No generic matrix/nonlinear/SSP/global-error claim.

Abort returns stage index and partial RHS/solve work. A matching pending lease is settled once; accepted time/state/controller history is preserved. Recoverable failure shrinks h; explicit fatal callback is sticky. No guarantee that repeated shrinking terminates at the boundary. Callback side effects are not rolled back. Internal stage checks do not certify a dense interpolant.

Scale/norm/tableau/tolerances are unchanged. The ODE is a manufactured autonomous diagonal/Riccati system using an analytic diagonal Jacobian. This is not production native integration or actual BASS. SciSpace and SUNDIALS recoverable-RHS literature are context, not the authority for new source correctness.
