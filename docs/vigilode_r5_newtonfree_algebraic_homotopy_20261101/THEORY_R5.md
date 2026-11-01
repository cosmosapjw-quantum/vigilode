# R5 constructive theorem: global monotone homotopy for source-locked cooperative cubic stage
2026-11-01 KST. Only VigilODE standalone. Scope is the actual frozen WP1 source family and the analytically defined homogeneous identity-mass first Rosenbrock stage; not a general polynomial ODE theorem or a new production solver.

## Definitions and assumptions
For n>=1, tau>0, b_i>=0, k_i>0, g_i>=0, c_ij>=0 and c_ii=0, let s_i=sum_(j != i)c_ij and define
  F_{lambda,i}(z) = [1+tau(k_i+s_i)] z_i + tau g_i z_i^3
                    -lambda tau sum_(j != i) c_ij z_j -b_i,
with lambda in [0,1]. The model underlying F1 is exactly the original source
  f_i(y) = -k_i y_i -g_i y_i^3 + sum_(j != i) c_ij (y_j-y_i)
at a diagonal implicit RK/BE stage z=b+tau f(z). Intermediate F_lambda are only homotopy problems, not the original physical ODE and not the 8-stage RODAS5P equations. Coefficients and initial bits are read from original Rust declarations; no new model parameters are fitted.

## Theorem A: real algebraic branch, uniqueness, positivity, residual certificate
Set d_i=1+tau(k_i+s_i), k_min=min_i k_i, s_max=max_i s_i, B=max_i b_i/(1+tau k_min), and m_lambda=min_i [1+tau(k_i+(1-lambda)s_i)] >=1+tau k_min.

(1) For each lambda in [0,1], the polynomial system F_lambda(z)=0 has exactly one real solution z_lambda. If b>=0 then z_lambda>=0; if every b_i>0 then every z_lambda,i>0. Moreover 0<=z_lambda,i<=B.

(2) The solution is smooth in lambda; d z_lambda/dlambda >=0 componentwise, so the unique real positive branch cannot fold or collide with another real root in the admitted interval.

(3) For every real trial vector z, including trials outside the positive domain,
    ||z-z_lambda||_infinity <= ||F_lambda(z)||_infinity/m_lambda.
The denominator is a mathematical lower bound and must be evaluated from exact represented source data or conservatively rounded DOWN for a binary64 certificate; numerator must be bounded UP. An ordinary binary64 residual is not an outward certificate.

(4) For 0<=mu<=lambda<=1,
    0 <= z_lambda-z_mu componentwise,
    ||z_lambda-z_mu||_infinity <= (lambda-mu) tau s_max B/(1+tau k_min)
                             = (lambda-mu) tau s_max ||b||_infinity/(1+tau k_min)^2.

Proof.
For any r>=0, the scalar polynomial P_i(t)=d_i t+tau g_i t^3-r has exactly one nonnegative root. Define the nonlinear Jacobi map T_lambda(z)_i=P_i^{-1}(b_i+lambda tau sum_j c_ij z_j) for z>=0. It maps the cube [0,B]^n into itself: its input is at most ||b||_infinity+lambda tau s_i B, while P_i(B)>=[1+tau(k_min+s_i)]B >= the input. The scalar inverse is Lipschitz 1/d_i (mean-value polynomial inequality without runtime derivative evaluation). Therefore
    ||T_lambda(x)-T_lambda(y)||_infinity <= q_lambda ||x-y||_infinity,
    q_lambda=max_i lambda tau s_i / d_i <1.
Banach gives existence and uniqueness in the cube. For general real z,w, use the polynomial identity
    z_i^3-w_i^3=(z_i-w_i)(z_i^2+z_i w_i+w_i^2)
whose coefficient is nonnegative. F_lambda(z)-F_lambda(w)=A(z,w)(z-w), where A has diagonal d_i+tau g_i(z_i^2+z_i w_i+w_i^2) and off-diagonal -lambda tau c_ij. It is a strictly row diagonally dominant nonsingular M-matrix. Pick a component attaining ||z-w||_infinity; the reverse triangle inequality implies
    ||F_lambda(z)-F_lambda(w)||_infinity >= m_lambda ||z-w||_infinity.
Thus F_lambda is injective globally, and its positive root is the only real root. Setting w=z_lambda gives (3).

The real matrix DF_lambda at the solution is again a nonsingular M-matrix, so (finite-dimensional) implicit-function theorem applies and its inverse is entrywise nonnegative. Since partial_lambda F_lambda=-tau C z_lambda, differentiation gives
    dz_lambda/dlambda=DF_lambda(z_lambda)^(-1) tau C z_lambda>=0.
Apply (3) at parameter lambda to trial z_mu:
    ||z_lambda-z_mu|| <= (lambda-mu)tau||Cz_mu||/m_lambda
                     <= (lambda-mu)tau s_max B/(1+tau k_min).
This proves (2),(4). No assumption of symmetric C, normal Q, or strict irreducibility is used.

## Algebraic-geometric interpretation and boundary
At lambda=0 each coordinate is the univariate polynomial
  a_i z_i^3 + d_i z_i - b_i =0,  a_i=tau g_i>=0, d_i>0.
When a_i>0 its algebraic discriminant is
  Disc_i=-a_i(4 d_i^3+27 a_i b_i^2)<0,
so there is one real and two complex conjugate roots. When a_i=0 the true equation has degree one, NOT a singular cubic that should be processed by Cardano. Along lambda in [0,1], all real Jacobians remain nonsingular, so no real discriminant-locus collision occurs; this does not exclude complex algebraic singularities and is not a complete complex path tracking theorem. Wolfram Language independently produced the symbolic discriminant -a(27 a b^2+4 d^3). The real multivariate M-matrix proof is the substantive result; no claim of literature novelty is made.

## Runtime Newton-free map and honest cost
At each fixed lambda use the original source coefficients and value-only operations:
  r_i = b_i +lambda tau sum_j c_ij z_j;
  z_i^next = unique nonnegative root of d_i s +tau g_i s^3=r_i.
Monotone bisection brackets s in [0,r_i/d_i]. No Jacobian, JVP, AD, FD or Newton **callback** is required. Linear g_i=0 is an exact division, not a cubic search. The fixed point converges for all admitted lambda with rate q_lambda; when q_lambda is near one, slow convergence is expected. Continuation may produce better initial values but not necessarily fewer function evaluations. No assertion of speedup follows from this theorem.

For each returned represented vector, compute exact-real F_lambda at binary64 input bits with Fraction and construct a conservative upper residual plus conservative lower m_lambda. In the trial script the raw exact Fraction quotient is independently bounded by float(nextafter) upper, but this does not certify native floating-point callback rounding or an entire ODE trajectory. If a source changes within a step or the original residual/error cap fails, preserve all attempted work and return a typed failure, then protected fallback (no experimental output commit).

## Theorem B: connection to original RODAS5P without conflating stage types
For a positive identity-mass original RODAS5P step h>0 with gamma>0 and frozen y_n, the exact source linearization is
  J_n = Q -diag(k_i+3g_i y_{n,i}^2), Q_ii=-s_i, Q_ij=c_ij.
The true original raw-U stage system has
  W U_i = r_i, W=I-h gamma J_n.
For first autonomous stage, r_1=h gamma f(y_n); subsequent stages include the **original** nonlinear RHS at previous U-stage states and historical tableau couplings. The W matrix has diagonal 1+h gamma(k_i+s_i+3g_i y_n,i^2), off-diagonal -h gamma c_ij, hence is an M-matrix with uniform row-dominance margin >=1+h gamma k_min. Thus for any signed candidate U,
  ||U-U_true||_infinity <= ||W U-r_i||_infinity/(1+h gamma k_min).
For source-owner-accessible polynomial coefficients one can evaluate W U with algebraic source operations but this *uses an analytically known mathematical derivative*: it is callback-JVP-free, not a general unknown-RHS derivative-free Rosenbrock method. The new scalar nonlinear root does NOT replace any true original RODAS5P U-stage; it is only an independent structure test and possible implicit-RK alternative. Any future causal q1/q2 or structured fast provider must pass the true raw-U target, embedded/physical error, time/epoch and full-IVP failure-cost checks.

Proof of Theorem B. Same row diagonal dominance argument as A with tau=h gamma and g_i term replaced by 3g_i y_n,i^2. No Newton root is hidden; it is a linear resolvent.

## Explicit adversarial counterexamples
- g<0: F(z)=2z-z^3 at lambda=1 has three real zeros {0,+sqrt(2),-sqrt(2)}. Strict M-matrix/unique-branch conclusions no longer hold.
- c_ij<0: loses the Metzler/M-matrix and positivity assumptions, and must be refused by source validation.
- lambda>1, tau<0, unbound structural rank, stale model epoch, nonautonomous time-carrier mismatch: unsupported; no fallback certificate from the present theorem.
- b negative: real uniqueness may remain for admissible coefficients, but the claimed nonnegative domain and monotone path need separate conditions.
- q_lambda approaching 1 and negative evidence where 5-station continuation requires more scalar evaluations than direct solve: keep baseline and protected fallback; homotopy is not automatically an accelerator.

## Computation and conclusion
The prospectively registered four original Rust fixture declarations give three independent physical families and one resolution control. Source-only Newton-free Jacobi plus positive lambda=0,.25,.5,.75,1 polynomial continuation and the source-conditioned first Rosenbrock U-stage were run. Same-author hostile tests and exact binary64-as-rational residual inspection accompany RESULTS.json. This is DERIVED + NUMERICALLY_CHECKED_SOURCE_SCOPED (not independent author, no formal HOL, no full native original eight-stage parity, no claimed measured wall-time acceleration).

Preserved official DAG 7/23 and successor 0/12; production HOLD; existing protected fallback remains first-class.
