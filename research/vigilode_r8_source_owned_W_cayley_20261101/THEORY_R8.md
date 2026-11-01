# R8 science decision and source-locked polynomial W-inverse construction

Scope: standalone original VigilODE G0 8-stage RODAS5P. Pre-registered parent PR182 at immutable HEAD 8fd2ac31d65df5e70e3bbb266d51b397562edfb5; R8 preregistration published at 71109f9ab908442cf3ba50222bc0af34bfacf74c before numerical study. No BASS/REI applied projects. The R8 objective is **claim adjudication** after real R7 work-precision, not automatic solver production promotion.

## Definition and constructive source-specific lemma

Assume autonomous identity-mass original cooperative cubic source

  f_i(y) = -k_i y_i - g_i y_i^3 + sum_(j !=i) c_ij (y_j-y_i),
  k_i>0, g_i>=0, c_ij>=0, c_ii=0, tau=h gamma>0.

Write s_i=sum_j c_ij, d_i=1+tau(k_i+s_i+3*g_i*y_n,i^2), D=diag(d_i).
The EXACT ORIGINAL Rosenbrock stage linear system is W U_i=r_i, W=I-tau Df(y_n) for the original 8-stage raw-U RHS r_i (do not substitute the nonlinear implicit Euler equation). Define T_ii=0 and T_ij=tau*c_ij/d_i for i!=j. Then

  W=D(I-T), T>=0, q=||T||_infty=max_i(tau*s_i/d_i)<1.

Thus the original W is a nonsingular M-matrix, det W>0, W^-1=(I-T)^-1 D^-1>=0. The proof is the nonnegative absolutely convergent matrix Neumann series, since ||T||_infty<1. The precise finite remainder after K terms is

  u_K=sum_(j=0)^K T^j D^-1 r,
  u_true-u_K = T^(K+1)(I-T)^-1 D^-1r,
  ||u_true-u_K||_infty <= q^(K+1)/(1-q)*||D^-1r||_infty.

The identity W u_K-r = -D T^(K+1)D^-1r gives a direct original-target residual. Moreover strict row diagonal dominance margin

  m=min_i(d_i - tau s_i)=min_i(1+tau(k_i+3*g_i*y_n,i^2))>1

implies for ANY signed candidate U

  ||U-W^-1 r||_infty <= ||WU-r||_infty / m.

Proof of the inverse-norm bound: choose index i attaining max |e_i|, then
|(We)_i| >= (|W_ii|-sum_(j!=i)|W_ij|)||e||_infty >=m||e||_infty.
No positivity assumption on e or on stage r is used. A stronger source-specific first-stage bound is available from exact rational coefficients and directed-up display. This bound is valid for original raw-U stage, unlike R5 nonlinear BE-only root bounds.

## Distinct algebraic-geometric realization (Cayley–Hamilton, dimension 2 or 3)

For original source-owned n=2,3 form exact represented-rational W and its characteristic polynomial

  p_W(z)=z^n+a_(n-1)z^(n-1)+...+a_1 z+a_0,
  a_0=(-1)^n det W !=0.

Cayley–Hamilton yields an exact FINITE inverse:

  W^-1 = -(W^(n-1)+a_(n-1)W^(n-2)+...+a_1 I)/a_0.

For 2x2 W=[[d1,-a],[-b,d2]], det=d1*d2-a*b>0 and
  U_1=(d2*r1+a*r2)/det, U_2=(b*r1+d1*r2)/det.

The original rho= WU-r is computed exactly on the represented binary64 output. HOL Light 3.1.0 (isolated sandbox OCaml5.3.0) proved TWO universal exact 2x2 stage-equation field identities conditional det !=0 and ONE scalar finite-Neumann algebra identity as actual named 'thm' kernel objects (3/3, no Error/Exception markers). The entire high-dimensional M-matrix/ROW theorem and native Rust implementation are NOT proven in HOL. Independent Wolfram evaluated the same symbolic 2x2 inverse, characteristic polynomial and identity W W^-1=I. Python SymPy exact Fraction cross-checks the 2x2 and 3x3 CH polynomial against adjugate and exact matrix multiplication on every original source.

The value-only stage proposal is JVP-callback-free and Newton-free because the **model owner supplies analytic frozen polynomial coefficients**. It does NOT eliminate mathematical differentiation (3*g_i*y_n,i^2) and is not a generic black-box derivative-free method. Matrix assembly and CH operations are small dense source-scoped work. Without model owner, structural/epoch identity or justified low n, do not use it.

## New nontrivial branch: signed H-matrix certificate

Drop cooperativity, retain only strict absolute row diagonal dominance:
 W_ii>0 and m_H=min_i(|W_ii|-sum_(j!=i)|W_ij|)>0.
Then the exact same residual-to-error norm inequality holds by the reverse triangle argument, even if offdiagonal W_ij have mixed signs and W^-1 is not nonnegative. Numerical signed witness J=[[-4,-3],[2,-5]], tau=1/8 has W_12>0, not an M-matrix but passes the H-bound. This is an honest admissible extension of the *linear residual bound only*, NOT the cooperative positivity/homotopy, physically-positive semigroup or monotone root theorem. It is a proposed structural search node for future strongly nonnormal source families, not an observed generic speedup.

## Original source verification and practical meaning

The original R6/R5 cubic/Git-locked Rust source and original RODAS5P tableau are verified by their original blob/digest. Reconstructed the full first original 8-stage raw-U step at each of four frozen first-h source cohorts (3 physical models and one resolution control), total 32 W-stage solves. Exact rational Cayley–Hamilton inverse is used at every stage with the ORIGINAL lower tableau and original source RHS, then rounded back to represented float for the next stage. Results are compared to protected sequential native-source mathematical reimplementation and independent mpmath 90-decimal source reference. Source-conditioned *whole original eight-stage first-step* rigorous rational recursive endpoint upper bounds:

| Source | q_inf | first Neumann K with certified <=1e-12 error | exact-rational propagated endpoint upper |
| --- | ---: | ---: | ---: |
| asymmetric_2 | 0.0039497626 | 3 | 1.7959863122e-15 |
| asymmetric_2_refined (resolution control) | 0.0019808797 | 3 | 3.9653386946e-16 |
| coupled_3_cubic | 0.0055922869 | 3 | 6.4692486548e-16 |
| linear_exchange_control | 0.0009918151 | 2 | 1.9652110603e-16 |

All four full first-step source/coefficients reconstructions agree with independent 90-digit reference within 2.8e-17 in displayed float endpoints. Tested 40 first-stage Neumann values (K∈{0,1,2,3,4,5,6,8,10,12}) with exact rational geometric+residual error certificates. 12/12 focused same-author adversarial tests pass, including source poisoning, signed non-Metzler branch, negative g/c, dimension four refusal, and no false production promotion.

These are **original 8-stage first-step mathematics computations**, not actual new native full-IVP opt-in Rust code nor measured speed. A low mathematical Neumann K is a candidate computational advantage, not an empirical wall-clock guarantee. R6 finite causal exactness (8 sweeps at higher work) and R7 native real wall economics are retained. R5 nonlinear BE cubic homotopy remains a distinct source-specific root-finder for an implicit stage, not a substitute for original linear ROW W.

## R8 arbitration and critical-path next action

- ADMIT DERIVED: original source-owned M-matrix inverse and Neumann tail/residual inequalities under strict assumptions, CH exact n<=3 algebraic inverse, signed H residual bound under separate condition.
- NUMERICALLY CHECKED: 4x8 first-step original raw-U reconstruction and source-conditioned exact-rational full eight-stage endpoint enclosure; 40 first-stage probes.
- HOL-KERNEL VERIFIED ONLY: three local scalar/2x2 algebraic sublemmas, **not** entire n-dimensional source or adaptive IVP theorem.
- HOLD production q2 default: R7 measured three distinct original model IVPs at 3 tolerances and full dense outputs; native transactional q1/q2 uses many more JVPs and wall ratio protected/transactional median 0.21069269165. No blanket no-go for large/parallel systems.
- KEEP protected RODAS5P as default on original 2/3-dimensional model scope; original source-specific Neumann/CH provider is an **opt-in research candidate only**, blocked on native end-to-end same-error cost/clock/epoch work and true fallback.
- EXPERIMENTAL HYBRIDS: (1) CH algebraic source provider + original protected RODAS fallback, preserving each of the actual 8 stages and certified budget; (2) nonlinear BDF1/BE homotopy rootfinder + RODAS or E-K as *alternative methods*, not simultaneous substitution of different stage equations; (3) Bernstein continuous residual + fused exponential Krylov candidate for future equal-grid IVP comparison. No pair is automatically declared an operational integrated production solver.
- Strong no-go novelty and independent C9: NOT established. Existing M-matrix and Cayley–Hamilton theory are standard; project-specific original source application is the concrete novelty.
- G0 DAG 7/23, successor 0/12, production HOLD; no default, core source, tolerance, merge or external API mutation.

## Stop and research loop boundary

R8 is a decision stage: finish this proof-specific outcome and transition only on a new request. The next legitimate new coding research stage should integrate **one source-owner-authenticated CH or truncated-Neumann raw-U provider** into the existing native 8-stage and retry/rollback accounting with protected fallback, then measure against R7 identical original full-IVP output and broader stiff/nonnormal holdout. The pure symbolic/step calculation does not justify claiming that code work is already complete.
