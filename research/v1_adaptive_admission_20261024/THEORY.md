# V1r scale enclosure and transactional trajectory

Target E²=sum((Le)_i/s_i)^2/33 for fixed PR79 +/-1 lift. s_i=atol_i+rtol_i*max(|Yold_i|,|Ytrial_i|), interpreting the supplied binary64 physical states/tolerances as exact reals. This does not certify upstream affine reconstruction rounding or actual ODE global error.

Under guarded round-to-nearest/gradual-underflow/no-fast-math, outward multiplication/addition enclose 0<slo<=s<=shi<infinity; unsupported scales refuse. Since d[(e/s)^2]/ds=-2e²/s³, E²(shi)<=E²(s)<=E²(slo). Accept only integer upper bound at slo<=1; Reject only integer lower bound at shi>1; otherwise hold. A floating estimate may propose h but cannot authorize commit.

For RL=I, F(Lz+b)=Lf(z), J_F L=L J_R and invertible shifted systems, W_F L=L W_R implies W_F^-1 L=L W_R^-1. Induction over raw-U stages proves u_F=L u_R in exact arithmetic, then affine endpoint and linear error correspondence. Float executions need not be bit-identical. This assumes stage relatedness and fixed chart, not arbitrary nonlinear coordinates or changing mass matrices.

The transaction binds accepted epoch, attempt id and represented h/next time. Begin validates forward clock and cumulative budget before work. Finish requires the same lease. Rejection/failure/Indeterminate preserves accepted time/state/epoch/accepted-controller history but charges attempted work. Rejection shrinks h; Indeterminate is sticky NeedsPrecision. Stale lease cannot replace the live trial. Budget survives caller yields, and true final arrival precedes budget refusal.

Manufactured family: z'_j=-lambda_j*z_j-alpha*z_j², exact positive solution z0 exp(-lambda*t)/(1+alpha*z0*(1-exp(-lambda*t))/lambda). Full33 redundant dynamics use negative sums for dependent rows. Two independently coded stage realizations use pinned eight-stage coefficients, not a production-crate build or actual BASS client. Autonomous dimensionless times/states; rates retain inverse-time interpretation.

Frozen finite experiment: linear and alpha4 families, rates1/20/300, tolerances1e-4/1e-7/1e-9, h0=.05, t=[0,1],20 fixed output landings, cap10000attempts. Physical30 certified, naive30 diagnostic and full33 point-WRMS arms. Observe endpoint/output errors against exact solutions; ceiling20tol is a finite test criterion, not local-to-global theorem. Same-step full/reduced difference target1e-12.

Wolfram checks: RL=I, Jacobian/shifted intertwining0, Riccati residual0, scale derivative -2u²/s³. SciSpace/primary arXiv2307.12677v3 support distinguishing controller behavior from estimator quality; that explicit-RK analysis does not prove stability of our simple clipped I proposal. No new-tableau novelty claim.
