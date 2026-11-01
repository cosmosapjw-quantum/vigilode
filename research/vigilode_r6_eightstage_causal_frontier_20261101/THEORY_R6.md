# R6 theorem and hostile validation: exact 8-stage causal propagation and source-residual error

## Source and claim classification

Original frozen native RODAS5P raw-U identity (RODAS5P coefficient snapshot SHA-256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6, Rust implementation Git blob d68239d53977e36f98b186b627aa0c4c5c7de7cb) in autonomous identity-mass form:

  W U_i = H_i(U_<i)
  W = I - tau J_n, tau=h gamma>0,
  H_i(U_<i) = tau f(y_n + sum_(j<i) A_ij U_j) + gamma sum_(j<i) C_ij U_j.
  i = 0,...,7

A,C are the original strictly lower tableau matrices, not fitted alternatives. Every coefficient and source parameter used in the numerical unit is reconstructed as exact rational from its represented binary64 value for the exact-real residual checks. The protected native implementation may have different IEEE evaluation ordering; no byte-parity or native full-IVP error certificate is inferred.

### Theorem 1 (finite depth without global contraction)
Assume W is nonsingular and f is a deterministic map for fixed time, source owner and epoch; for each i, H_i depends *only* on U_<i. Define the simultaneous original-stage recurrence

  U_i^(q+1) = W^-1 H_i(U_0^(q),...,U_(i-1)^(q)).

For any initial block vector U^(0), after q updates the first q blocks agree with the unique exact sequential solution. In particular after 8 updates **all eight blocks are exact** in exact arithmetic; no bound on the operator norm or Lipschitz constant of the full stage map is needed. The proof is finite induction on i: stage 0 is independent of prior stages and becomes exact at the first update. If the first q stages were exact, stage q is exact on the next update, regardless of later-stage errors. Uniqueness follows from forward substitution W^-1. This extends to any nonlinear f, provided H_i remains explicitly causal, not an implicit coupling in U_i.

This is standard strict-lower-triangular finite propagation, not claimed as novel universal convergence theory. The distinct research question is whether an early q<7 candidate passes the **original** stage/physical error gate at lower *net wall/work/span* than the existing protected driver. Nonlinear stage-map global contraction is neither required for exact 8-sweep termination nor sufficient by itself to justify early q.

### Theorem 2 (algebraic-geometric path of the true 8-stage Rosenbrock system)
For a polynomial RHS f of total degree d and invertible W, define a *stage-coupling* homotopy

  G_i(lambda,U) = W U_i -
    tau f(y_n + lambda sum_(j<i) A_ij U_j) -
    lambda gamma sum_(j<i) C_ij U_j =0.

This has, for **every complex lambda**, exactly one U(lambda), constructed recursively. It is a polynomial vector in lambda, since each coordinate is obtained by a constant linear inverse W^-1 applied to a polynomial in lambda and earlier polynomial stages. The full 8n by 8n stage Jacobian dG/dU is block-lower-triangular with all diagonal blocks W. Thus det(dG/dU)=(det W)^8 !=0 for any lambda and any stage vector, so the algebraic solution path never branches or folds (including complex lambda), assuming W remains constant and nonsingular.

For d>=1, deg U_0=0 and deg U_i <= d(1+max_(j<i)deg U_j), hence for cubic source d=3:

  deg U_i <= (3^(i+1)-3)/2, i>=1,
  degrees upper [0,3,12,39,120,363,1092,3279].

The high algebraic degree shows why full symbolic homotopy expansion is not automatically computationally viable. For an affine f, deg U_i<=i; the lower-block dependence is linear and the simultaneous recurrence at parameter lambda is a finite Neumann/path-prefix polynomial, exact after 8 sweeps. For nonlinear f, q sweeps are NOT generally degree-q Taylor truncations. Wolfram Language directly checked [0,3,12,39] for an independent exact cubic example; algebraic-geometric degree growth is a mathematical structure, not new ODE accuracy or a fast solver.

### Theorem 3 (nonlinear source-specific a-posteriori *all eight stage* error bound)
Let original autonomous source f_i(z)=-k_i z_i-g_i z_i^3+sum_(j !=i) c_ij(z_j-z_i), with k_i>0, g_i>=0 and c_ij>=0. At frozen y_n let J_n=Df(y_n), W=I-tau J_n, tau>0. The matrix W is a strictly row-diagonally-dominant nonsingular M-matrix and has

  ||W^-1||_inf <= beta := 1 / min_i(1+tau(k_i+3 g_i y_n,i^2)).

For a *candidate* Uhat comprising all eight blocks, define the exact real source residual r_i = W Uhat_i - H_i(Uhat_<i). Let E_j be verified nonnegative stage error bounds for j<i. Set

  zhat_i=y_n+sum_(j<i) A_ij Uhat_j,
  D_i=sum_(j<i) |A_ij| E_j,
  R_i=||zhat_i||_inf+D_i,
  L_i=max_v (k_v + 2 sum_(w!=v)c_vw + 3 g_v R_i^2).

Then the exact target U* satisfies the recursive inequality

  ||Uhat_i-U*_i||_inf <=
     beta [ ||r_i||_inf + sum_(j<i) (tau L_i |A_ij|+|gamma C_ij|) E_j ].

The source-specific algorithm sets E_i to the exact right side (or a verified outward upper enclosure), one stage at a time; it has no global q-contraction prerequisite. Proof: subtract exact and candidate stage equations, use W^-1 and the polynomial difference

  x^3-y^3=(x-y)(x^2+xy+y^2)

to bound f(x)-f(y) on the verified coordinate tube |x|<=R_i. Since the bound E_j is already valid for j<i, the tube R_i contains all exact intermediates and no unverified bound is assumed. Induction proves all eight bounds. The endpoint y_n+sum_i bcode_i U_i then obeys

  ||yhat_next-y*_next||_inf <= sum_i |bcode_i| E_i.

This is a rigorous **source-conditioned exact-real** inequality for the specified represented coefficients. In the executable, exact Python Fraction evaluates each original stage residual, recursively computes rational conservative bounds, and rounds only final stored float bounds upward using nextafter. It does **not** enclose the native IEEE callback/Rust implementation or finite-step ODE truncation error; do not advertise it as full mathematical validation of an adaptive solver.

### Computational form and explicit work ledger

The protected serial stage baseline performs 8 W solves, serial dependency depth 8. The frontier schedule uses initial U=0; at sweep q+1 it preserves the first q verified causal blocks and recomputes only later blocks using the previous frozen sweep's values. For q=0..7 the counted cumulative W-solves are respectively

  [8,15,21,26,30,33,35,36],

while depth is [1,...,8]. At q2, three independent parallel rounds *could* reduce span from 8 to 3, but require 21 W solves instead of 8 and an honest residual/certificate charge. The pre-registered q2 here is an **unaccelerated synchronous causal-frontier protocol**, NOT the native `transactional_q1_q2` implementation (which has source-specific predictors, endpoint corrections and its own admission logic); the native transactional challenger is NOT_RUN in this unit, so no native q2 policy verdict is inferred. Without parallel execution this is an unfavorable work trade. No hardware speedup follows.

The same source's W has positive row diagonal and nonnegative offdiagonal inverse. A source-owned scalar Jacobi solver for each W U_i=rhs_i is a callback-Newton-free, analytic source-specific alternative with contraction upper q_inner=max_i(tau s_i/W_ii)<1. It is **not a generic derivative-free unknown-RHS solver**, because the polynomial form and its analytically derived 3g_i y_i^2 are available. The nonlinear BE root homotopy of R5 is a different object. Protected RODAS5P remains the fallback if source/epoch identity, target residual, original error budget, cost or inner solve fails.

### Falsifications and negative controls

1. The original A,C have large signed coefficients. Positive M-matrix W does not ensure the 8-stage Jacobi *outer* map is contractive. On all four original frozen fixtures, q2's original endpoint differs visibly from protected 90-decimal source reference. Intermediate q3/q4 errors can INCREASE rather than monotonically decrease. Never admit q2 solely because q=2.
2. Counterexample with scalar W=1 and H_i=1+2.5 U_(i-1) for i>0: outer coupling norm 2.5>1, early q2 fails badly, but 8 causal sweeps terminate in exact binary64 algebra because 2.5 and integer constants are exactly represented. No global contraction.
3. Source-equivalent asymmetric_2_refined differs only in original step count, not physical coefficients. Count three physical models, four fixture cases. A speed benchmark from these four cases is NOT a general stiff/oscillatory/nonnormal corpus.
4. If W source coefficients are unknown, source owner/epoch/stage clock changes during the step, mass M≠I, f_t≠0, or certified inverse-norm bound unavailable, the present theorem does not authorize the source-specific fast path. Use protected JVP/GMRES with exact original residual/fallback.
5. Fixed first-step source-oriented Rust formula was read from original code, but the experiment is Python reconstruction with high-precision mpmath and exact Fraction postconditions, **not a native Rust eight-stage parity test**, and not same-runner achieved-global-error benchmarking.

### Source and literature state

- Original Rodas5P ROW source and 8-stage linear equations: Steinebach, BIT Numerical Mathematics (2023), DOI 10.1007/s10543-023-00967-x.
- More general waveform relaxation / Parareal finite dependency ideas are established: wavefront/triangular structure and nonlinear relaxation are not novel by themselves. Botchev (2023) arXiv:2307.00276; Fan (2019) DOI 10.1007/S11075-018-0493-3. These are contextual rather than claims of novelty for this eight-stage result.
- Exact original A,C tableau, RODAS raw-U code and original cubic RHS are stronger evidence for the implementation-specific equations than a secondary description.

Outcome gates: no false acceptance, source identity strict, keep all work, independent-author C9 NOT_RUN, M1 full native achieved-error NOT_RUN. Official DAG 7/23; successor 0/12; production HOLD; no original source/default/tolerance/merge changes.
