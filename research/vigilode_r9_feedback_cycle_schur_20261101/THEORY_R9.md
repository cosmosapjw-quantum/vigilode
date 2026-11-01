# VigilODE R9 | Exact cycle-core Schur reduction for original eight-stage RODAS5P

Date 2026-11-01 KST; stage R9 (formal mathematical result + computational contract). Immutable parent PR #184 HEAD e7e33fb77599be99f214009ab5a330ebaa3b62d2; pre-simulation scientific preregistration Git commit 4985060e598e8c039d759b9dd43e61a783cd2808. Only standalone original VigilODE G0, no applied BASS/REI. Original protected solver/source and 23-node DAG are unchanged.

## Theorem R9.1: finite DAG path plus small feedback Schur core

Let k_i>0, g_i>=0, c_ij>=0, c_ii=0 and tau=h gamma>0. Original cooperative source f_i(y)=-k_i y_i-g_i y_i^3+sum_(j !=i)c_ij(y_j-y_i) yields the original Rosenbrock-Wanner stage matrix W=I-tau Df(y_n), not the nonlinear backward-Euler root equation.

Let d_i=1+tau(k_i+sum_j c_ij+3g_i y_n,i^2)>0, D=diag(d_i), and T_ij=0 if i=j, tau*c_ij/d_i otherwise. Then W=D(I-T), T>=0, q=||T||_infty<1. For the original raw-U stage right side r_i, let b=D^-1 r_i.

Pick an explicitly declared subset B (feedback vertices), A=V\B, and **verify** the induced directed graph on A with edges j->i if T_ij != 0 is a DAG. No claim of a minimum feedback vertex set or efficient optimal search. Let L be the longest directed path length in A. Then the strictly acyclic T_AA obeys T_AA^(L+1)=0 and

    R_A=(I_A-T_AA)^-1=I_A+T_AA+...+T_AA^L.

The A-action R_A z can be evaluated by ONE topologically ordered sparse dynamic program, without enumerating all paths. Let

    Z_B = T_BB+T_BA R_A T_AB,
    S_B = I_B-Z_B,
    b'_B=b_B+T_BA R_A b_A.

If B is nonempty, solve

    u_B=S_B^-1 b'_B;
    u_A=R_A(b_A+T_AB u_B).

If B is empty, simply set u_A=R_A b_A. For |B|<=3 the core inverse is computed EXACTLY over the represented dyadic coefficient field with R8 Cayley-Hamilton/adjugate; otherwise this research prototype refuses and proposes the already protected original RODAS5P fallback.

### Proof and validity range

T is nonnegative and its row sums are q<1, so I-T has a convergent Neumann inverse with nonnegative entries. A is acyclic and each nonzero term of T_AA^p encodes a directed path of length p in A; no path exists of length L+1, proving nilpotency and the finite sum.

For the core put e_A=R_A T_AB 1_B. Since T_AA 1_A+T_AB 1_B<=q 1_A,

    T_AB 1_B <= (I_A-T_AA)1_A-(1-q)1_A,

so positivity of R_A implies

    0<=e_A<=1_A-(1-q) R_A 1_A<=1_A.

Therefore

    0<=Z_B 1_B=T_BB 1_B+T_BA e_A
      <=T_BB 1_B+T_BA 1_A <=q 1_B.

Thus Z_B is nonnegative with ||Z_B||_infty<=q<1 and S_B is itself an invertible M-matrix with nonnegative inverse. The block equations follow by substituting the finite A solve into the B equation. The resulting u is the *exact* solution of the original W u=r.

The standard original-source M-matrix row margin

    m=min_i(d_i-tau sum_j c_ij)
      =min_i(1+tau(k_i+3 g_i y_n,i^2))>1

gives for any proposed u_hat

    ||u_hat-u_*||_infty <= ||W u_hat-r||_infty/m,

by taking a component with maximal |u_hat-u_*| and applying reverse triangle inequality to W(u_hat-u_*). No positivity condition on the signed Rosenbrock stage vector or on r is required. A full original 8-stage candidate may use R6's lower-triangular exact-rational residual recursion to bound the first-step source output; that does not cover native roundoff or global ODE truncation.

### New nontrivial signed-H branch, NOT a positivity theorem

If original source cooperativity fails but W is strictly *absolute-row* diagonally dominant, the same residual bound with m_H=min_i(|W_ii|-sum_(j!=i)|W_ij|)>0 still holds. Graph elimination remains an algebraic method when all required inverses exist, but S_B need not be a positive M-matrix; its nonnegative inverse, homotopy positivity and bounded ||Z_B|| are NOT imported. Do not use the cooperative proof or source gate for signed terms without a new justified certificate.

### Source ownership and runtime semantics

The exact W is formed from known polynomial coefficients, hence no runtime JVP/FD/AD/Newton callbacks are used in this construction; however the analytic derivative 3g_i*y_n,i^2 is mathematically required, so this is **source-owned derivative-callback-free**, not a universal unknown-RHS derivative-free solver. W and B/A factor structure are frozen at the original step's source epoch, clock, tableau and parameter identity. The original 8-stage Rosenbrock r_i(U_<i) remains unchanged. R5's positive nonlinear BE/BDF homotopy root is a distinct candidate method and cannot be substituted as a RODAS linear stage.

## Exact computational form and bottlenecks

- Setup: verify graph acyclicity (O(n+|E|)), source identity and coefficient signs; evaluate |B| sparse A topological propagations for R_A T_AB; assemble and factor |B|^2 Schur core (source-specific n<=3 CH).
- Each original stage: propagate R_A b_A once, solve the small core, combine the A solution and check true original W residual; repeat for eight original raw-U stages, and preserve rejected/diagnostic/fallback work.
- In a long n=24 directed ring, the A path depth is 22. Although only one cycle-core variable is needed, this method's strictly sequential topological span is long; it is **not automatically an improvement over native GMRES**. Favor short-depth wide DAGs, reused step matrices, and source-owner validation. Setup cost and communication can overwhelm small systems.
- Counterexamples/typed refusals: dense n=8 with declared |B|=3 has a remaining cycle, so REFUSED; four disjoint two-cycles need |B|>=4, so REFUSED by |B|<=3 scope; negative c_ij violates positivity and is REFUSED. Signed-H is a separate research branch.
- When |B|>3, time/cost model unfavorable, graph invalid or original error target fails, return a typed non-commit and execute the unchanged protected original RODAS5P fallback. No uncharged speculation.

## Observed source-bound evidence

R9 executed the original R6/R8-source frozen original WP1 cubic cohort for all first-step eight raw-U stages (4 cases, 3 independent original physics sources + resolution control) and 4 explicitly synthetic sparse graph controls (DAG n8, 1-core ring n12, two-core n14, 1-core ring n24), all with the original RODAS5P eight-stage lower tableau. Total 64 original-mathematical-form raw-U stage checks, not 64 native Rust full-IVP integrations. Exact Fraction feedback Schur and signed stage residuals matched the protected full-stage first-step mathematical reference up to 1.43e-16; original 4 additionally matched independent 90-digit source reference to within displayed float (exact first-step upper residual propagation <=1.80e-15). Synthetic n12/n14/n24 have different generated sources and cannot be counted as original physical data. Three negative controls were rejected without false-PASS. 15/15 same-author adversarial tests PASSED; full n-dimensional HOL kernel theorem and actual same-error native full-IVP speed NOT_RUN.

Prior R7 real-native performance remains binding evidence: the current default transactional q1/q2 median protected/transactional 0.21069269 at matched achieved global error on the 3 original small source systems, hence NO default promotion or general q2 advantage is inferred. The new graph algorithm has no measured native wall gain. Mathematical proof uses classical M-matrix/Schur/factorization literature, without an unsupported claim of novel universal mathematics.

## Source provenance and external literature comparison

- Original source: immutable Git blobs d68239d53977e36f98b186b627aa0c4c5c7de7cb (raw-U), d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482 (original cubic), 1bfc17ec2ed4b902cc06073807388f6e332e31d6 (original cases), original tableau SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6.
- Classical block Schur literature: Johnson and Smith, Closure of matrix classes under Schur complementation including singularities, DOI 10.1090/conm/419/08004; Eisenstat and Liu, Structural representations of Schur complements in sparse matrices (1993), DOI 10.1007/978-1-4613-8369-7_4.
- Fiedler (1980), Application of graphs to the Gaussian elimination method, DOI 10.1007/BF01296240; Bru, Corral and Giménez, Schur complements in H-matrices classes (2008). These establish established components; project-specific stage/source/cost selection is the present synthesis.
- Rodas5P construction: Steinebach (2023) DOI 10.1007/s10543-023-00967-x, original eight-stage linearly implicit method not eighth order.

## R9 stop/go and handoff

ACCEPT source-scoped derived path-core theorem and proof-ready computational form. KEEP an opt-in prototype route only; do NOT call it a native solver or full-IVP benchmark. Later C5 should integrate one declared source-owned feedback-core stage provider with immutable owner/clock/time/coupling graph and exact raw-U residual, charged retries and protected RODAS fallback, then measure true same-achieved-global-error wall time and 8-stage critical path on original and large nonnormal holdout. No generic speedup or stronger no-go. OFFICIAL DAG 7/23, successor 0/12 and production HOLD, no default/tolerance/main changes, C9 distinct author NOT_RUN.
