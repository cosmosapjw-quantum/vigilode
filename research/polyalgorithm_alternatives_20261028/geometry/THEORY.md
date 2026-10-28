# Structural transport, causal closure, and geometric polyalgorithm routes

Date: 2026-10-28 UTC. Source identity:
7eb2dd0ffda39afb2871d76700a720986ff09fe0, the published PR92 head.

Scope: PhysMath Research Loop 1.3.0, R2 literature/reuse and R4 constructive
alternatives, with local derivations that support those stages. This note
does not claim a new ten-stage campaign, a native implementation, a runtime
speedup, or closure of PR-N03/PR-N05. The project remains derivative-free
first, retaining augmented causal Rosenbrock/RK and homotopy routes with
selective JVP only where justified.

Evidence labels used below:

- **Established/reused:** already proved or implemented in the cited source.
- **Derived:** the displayed assumptions and proof establish the stated
  mathematical result, not an implemented floating-point certificate.
- **Checked:** reserved for an actually executed, recorded finite check.
- **Candidate:** a constructive implementation proposal requiring the listed
  conditions and subsequent local validation.

## 1. Reuse audit: do not restart solved mathematical questions

| Existing source at the pinned head | Reused result | Remaining addition |
|---|---|---|
| crates/vigilode-coupled/src/structural_mean_field.rs, MeanFieldPlan::algebraic_residual_budget | Two scalar adjoint solves, exact fixed-step residual-to-output formula, sharp block infinity norm. The implementation explicitly uses ordinary f64, not outward certification. | Variable-step products, physical defects, endpoint and clock rounding, charged adaptive consumer. |
| research/production_solver_closure_20261027/structural/THEORY.md | Full represented-stage residual enclosure; lift, mean, RHS and scalar-solve rounding retained. | That stage enclosure alone does not certify the physical trajectory. |
| research/derivative_free_causal_homotopy_20261027/theory/THEORY.md, Theorems B/C | Unique causal root, finite propagation, nilpotent Lipschitz majorant without global contraction; 56-to-28 exact-arithmetic cache example. | Provider-valid full-space constants, actual temporal-order inheritance, and total work/span against forward substitution. |
| crates/vigilode-coupled/src/causal.rs | One shared projected diagonal factorization; forward, batched Picard, exact input and unchanged-prefix reuse; joined/charged failures. | Cheap verified endpoint amplification and residual-domain contracts; no repeated claim that caching removes all sequential dependence. |
| crates/rodas5p-integrators/src/quotient_flow.rs | Declared damped-rotation quotient; exact affine and harmonic actions; forward-semigroup Lawson-DOPRI5 with residual-only callbacks. | Wider declared finite-algebra providers and explicit routing prerequisites. Classical order five here is for fixed A and smooth residual, not uniform nonlinear stiff order. |
| research/production_audit_20261018/WOODBURY_THEORY.md | Existing finite-rank algebraic reduction work. | Reuse declared invariant subspaces for an exponential/polynomial split; account for leakage if the subspace is approximate. |

The original-space residual remains essential after a projected root solve.
A small projected residual cannot exclude a large residual orthogonal to the
projection basis. Conversely, invoking a generic contraction requirement for
a strictly causal stage DAG discards a stronger result the project already
has.

## 2. What the supplied geometric references actually support

**Established literature, limited scope.** Kunkel and Mehrmann,
[arXiv:2205.08105](https://arxiv.org/pdf/2205.08105), Hypothesis 2.1,
Sections 3–4 and Theorems 4.2/4.4, construct inherent ODEs under derivative-array
rank and smoothness conditions. The geometric symmetry results concern
linear time-varying DAEs. Their reduction requires a valid smooth coordinate
choice, algebraic solvability, and the coordinate derivative. The paper
explicitly excludes applying its Section 4 construction to the nonlinear
pendulum example despite that example's self-adjoint Jacobian pair.
Its implementation uses derivatives/AD; it is not a generic RHS-only DAE
algorithm. In this paper, self-adjoint means E^T=-E and A^T=A+dot(E);
skew-adjoint means E^T=E and A^T=-A-dot(E). Those names should not be
confused with positive-definite solver classifications. A declared analytic
reduction is a valid reusable provider; arbitrary nonlinear DAE support does
not follow from this citation.

**Established literature, limited scope.** Habib Ammari,
[ETH Lecture 5](https://people.math.ethz.ch/~grsam/FS18/NAII/files/slides/ODE-Lecture5.pdf),
the sections on composition/splitting, leapfrog and Runge–Kutta methods,
gives composition of Hamiltonian subflows, separable
Störmer–Verlet, and the RK coefficient condition
b_i a_ij+b_j a_ji=b_i b_j for quadratic-invariant preservation and
symplecticity. These results require the appropriate structure and exact
stage relations. They do not identify an arbitrary adaptive solver,
polynomial approximation, or small inexact root residual as symplectic.

The propositions below are explicit derivations for this design task, with
reuse identified separately. No claim of broad novelty or a universal
impossibility theorem is made.

## 3. PR-N03: exact variable-step transport in the existing two-channel algebra

Let

\[
 P=\mathbf1\mathbf1^T/n,\quad Q=I-P,\qquad
 J=-dQ-(d-\kappa)P,\quad f(y)=b+Jy,
 \qquad d\ge\kappa\ge0.
\]

Write r_t=d and r_c=d-kappa. All identities in this section are over the
exact real values of the represented inputs and coefficients. The model,
P and Q remain fixed during the accepted sequence.

For step h_k>0, define scalar lower-triangular matrices

\[
 L_k(r)_{ii}=1+h_k\gamma r,\qquad
 L_k(r)_{ij}=h_k\gamma r a_{ij}-\gamma c_{ij}\quad(j<i).
 \tag{G1}
\]

Their diagonal entries are positive. Use the actual native endpoint weights
b_code, denoted beta below:

\[
 w_{k,r}=L_k(r)^{-T}\beta,\qquad
 \theta_{k,r}=L_k(r)^{-1}(h_k\gamma\mathbf1),\qquad
 \sigma_{k,r}=1-r\,\beta^T\theta_{k,r}.
 \tag{G2}
\]

The exact affine discrete step has linear part
S_k=sigma_(k,t) Q+sigma_(k,c) P. If represented stages have full original
raw-U residual vectors rho_(k,i), the error in their exact endpoint sum,
relative to the exact stages at the same input, is

\[
 a_k=\sum_i C_{k,i}\rho_{k,i},\qquad
 C_{k,i}=w_{k,r_t,i}Q+w_{k,r_c,i}P.
 \tag{G3}
\]

**Proof of (G3).** Subtraction of the affine stage equations gives two
independent triangular systems L_k(r) delta U=rho, one on each invariant
channel. Apply the endpoint row beta^T and transpose the scalar solve.
This is the existing scalar-adjoint argument at each accepted h. QED.

**Derived variable-step proposition.** Suppose the endpoint error obeys

\[
 e_{k+1}=S_ke_k+\xi_k+\sum_iC_{k,i}\rho_{k,i}, \tag{G4}
\]

where xi_k contains all other local defects in consistent state units.
For an accepted grid of N steps set

\[
 p_{k,r}=\prod_{\ell=k+1}^{N-1}\sigma_{\ell,r},\qquad
 p_{-1,r}=\prod_{\ell=0}^{N-1}\sigma_{\ell,r}.
\]

Empty products are one. Define the already-used exact block norm

\[
 \nu_n(x,z)=
 \left|x+\frac{z-x}{n}\right|
 +(n-1)\left|\frac{z-x}{n}\right|.
 \tag{G5}
\]

Then

\[
\begin{split}
 \|e_N\|_\infty\le&
 \nu_n(p_{-1,t},p_{-1,c})\|e_0\|_\infty\\
 &+\sum_k\nu_n(p_{k,t},p_{k,c})\|\xi_k\|_\infty\\
 &+\sum_{k,i}\nu_n(p_{k,t}w_{k,t,i},
                  p_{k,c}w_{k,c,i})\|\rho_{k,i}\|_\infty.
\end{split}\tag{G6}
\]

The last sum, with independently bounded stage residual blocks, is the
sharp induced norm from those weighted blocks to the final endpoint.
Sharpness is a property of this abstract residual operator, not a claim
that every residual sign pattern is produced by a real integrator.

**Proof.** P and Q are complementary idempotents, so products of S factors
multiply the two scalar coefficients separately. Unroll (G4) and use (G5).
Every row of each block has the same absolute row sum; the induced
block-maximum-to-vector-infinity norm is the sum of those row sums. A
common selected output row can attain it by independently choosing signs
of each residual component. QED.

This replaces fixed powers by products; it does not multiply already-loose
matrix norm bounds at each step. Changing h is allowed. Changing the
projector without a new derivation is not: P_(k+1)P_k need not equal P_k.
For an online algorithm with unknown future h, (G6) is an a posteriori
accepted-grid statement. Do not spend a future error budget using predicted
products as though the future grid were certified.

### 3.1 Turning a stage certificate into a physical certificate

One valid convention is
xi_k=Psi_(h_k)(Y(t_k))-Y(t_(k+1))+endpoint_rounding_k,
where Psi is the exact arithmetic discrete step and Y is the physical
solution. The first term includes both discretization and any difference
between the represented step h_k and the exact clock interval
Delta_k=t_(k+1)-t_k. Its bound needs a state enclosure or a valid defect
estimate; the existing stage enclosure supplies neither automatically.

For this particular affine model there is a simpler constructive option.
Define

\[
 F_\Delta(y)=e^{-d\Delta}Qy+
 e^{-(d-\kappa)\Delta}Py+
 \Delta\varphi_1(-d\Delta)Qb+
 \Delta\varphi_1(-(d-\kappa)\Delta)Pb,\qquad
 \varphi_1(z)=\frac{e^z-1}{z},\quad\varphi_1(0)=1.
 \tag{G7}
\]

**Derived physical defect proposition.** With
zeta_k=hat(y)_(k+1)-F_(Delta_k)(hat(y)_k),

\[
 e_N=e^{J(t_N-t_0)}e_0+
 \sum_{k=0}^{N-1}e^{J(t_N-t_{k+1})}\zeta_k. \tag{G8}
\]

Since the exponential has nonnegative entries and constant row sum,

\[
 \|e_N\|_\infty\le
 e^{-(d-\kappa)(t_N-t_0)}\|e_0\|_\infty+
 \sum_k e^{-(d-\kappa)(t_N-t_{k+1})}
          \|\zeta_k\|_\infty. \tag{G9}
\]

**Proof.** F_Delta is the exact affine flow; subtract physical and computed
endpoints and unroll. For t>=0, its transverse and collective factors obey
0<exp(-dt)<=exp(-(d-kappa)t). The resulting matrix entries are nonnegative,
so its infinity norm equals its collective factor. QED.

This directly includes stage, temporal, lifting, and endpoint errors in
one observable local defect. A cheap online upper bound follows by the
scalar recurrence E_(k+1)=exp(-(d-kappa)Delta_k)E_k+||zeta_k||.
The exact identity remains valid on any accepted adaptive grid; it makes
no symplecticity claim about the adaptive controller.

**Candidate/implementation gap.** Rigorous (G7)–(G9) requires outward
enclosures for exponential/phi functions, averaging, vector arithmetic,
and the represented clock interval. Calling ordinary libm exp/expm1 and
rounding the final answer upward is not a proof of transcendental enclosure.
If a reliable scalar enclosure is unavailable, keep (G6) algebraic and label
the physical estimate non-certified. Existing half-ULP addition/multiplication
enclosures can be reused, but they do not certify a transcendental library
call.

The polyalgorithm should also compare direct (G7) with eight-stage
Rosenbrock execution for a declared immutable affine model. When the exact
flow is cheaper, verifying a more expensive stage computation is a fallback,
not the preferred construction. This is a new mean-field flow candidate;
the damped-rotation flow implementation already supplies the architectural
precedent.

## 4. PR-N05: specialize the existing causal majorant to the actual residual

The current raw-U target is

\[
 WU_i=h\gamma f\left(y+\sum_{j<i}a_{ij}U_j\right)
       +\gamma\sum_{j<i}c_{ij}U_j,\qquad
 W=I-h\gamma J_0. \tag{G10}
\]

Existence and uniqueness by forward substitution require W invertible and
each recursively evaluated stage argument to remain inside the RHS domain.
They do not require a small Lipschitz constant. The existing finite
propagation theorem already proves this.

### 4.1 Derived code-specific full-residual majorant

Let U* solve (G10), let hat(U) be a candidate, and let rho_i be an upper
bound on its original-space residual norm. Assume
||W^-1||<=omega and that f has a certified Lipschitz bound L_i on the
segment between the exact and candidate stage arguments. Put

\[
 B_{ij}=\omega\left(h\gamma L_i|a_{ij}|+
                              |\gamma c_{ij}|\right)\quad(j<i).
\]

Then the block errors satisfy

\[
 e\le\omega(I-B)^{-1}\rho
    =\omega\sum_{\ell=0}^{s-1}B^\ell\rho,\qquad
 \|\delta y_{\rm endpoint}\|
 \le |\beta|^T\omega(I-B)^{-1}\rho. \tag{G11}
\]

**Proof.** Subtract (G10) at hat(U) and U*, apply W^-1, and use the
Lipschitz and triangle inequalities. This gives e<=omega*rho+B e.
Strict lower triangularity gives B^s=0. QED.

This is an instantiation of the project's existing Theorem C, not a newly
discovered no-contraction principle. It fixes the units and coefficients
needed by the raw-U implementation. For the mean-field family,

\[
 \omega=\frac{1}{1+h\gamma(d-\kappa)}
 \tag{G12}
\]

is the exact infinity inverse norm. More generally, an analytically
dissipative J_0 with Re<v,J_0v><=-alpha||v||^2 gives the Euclidean
bound omega<=1/(1+h gamma alpha), when that denominator is positive.
This follows from Cauchy–Schwarz applied to <v,Wv>. A few JVP samples do
not certify this dissipativity bound for every direction.

### 4.2 Carrier-aware refinement using nonlinear remainder bounds

Set
eta_i(U)=f(y+sum_j a_ij U_j)-f(y)-J_0 sum_j a_ij U_j.
Let A be the block lower-triangular affine stage matrix with diagonal W
and off-diagonal -gamma c_ij I-h gamma a_ij J_0. Suppose

\[
 D_{ij}\ge\|(A^{-1})_{ij}\|,\qquad
 \|\eta_i(U)-\eta_i(V)\|\le
       \Lambda_i\sum_{j<i}|a_{ij}|\|U_j-V_j\|.
\]

Define N_ij=h gamma Lambda_i |a_ij| for j<i. Then

\[
 e\le \left[\sum_{\ell=0}^{s-1}(DN)^\ell\right]D\rho,
 \qquad
 \|\delta y_{\rm endpoint}\|
 \le |\beta|^T\left[\sum_{\ell=0}^{s-1}(DN)^\ell\right]D\rho.
 \tag{G13}
\]

**Proof.** Rewrite the residual equation as A U=b+h gamma eta(U).
Subtract, apply A^-1 blockwise, and majorize to obtain e<=D rho+DN e.
D is lower triangular and N strictly lower triangular, hence DN is
strictly lower triangular and its s-th power vanishes. QED.

For a true frozen derivative J_0=f'(y), a supplied bound
||f'(y+v)-f'(y)||<=M_2||v|| on ||v||<=R gives Lambda_i<=M_2 R.
The constant follows from an integral of the derivative along a segment;
it is not a tuned safety multiplier. If J_0 is a different declared
carrier, add the supplied bound ||f'(y)-J_0||. Analytic bounds can be
provided without executing AD, but they must actually be justified by the
model. No arbitrary operator inferred from sampled trajectories inherits
the original Rosenbrock order automatically.

For restricted domains, use a stage-by-stage enclosure: previously bounded
errors determine the next stage-argument ball, and a provider must validate
the next Lipschitz bound on that ball. A whole-domain claim from an observed
pointwise slope is invalid.

### 4.3 Order and work are separate unresolved obligations

Assume a reference method is of order p and has the required stability on
the considered grid. If the added endpoint defect from inexact inner
solves is bounded by C h_k^(p+1), with uniform C on a finite interval, the
usual stable accumulation is O(h_max^p), because
sum h_k^(p+1)<=T h_max^p. Equations (G11)/(G13) turn this requirement into
a residual budget using the true tableau weights. They do not prove
order p of a changed frozen operator or nonlinear closure.

The exact-arithmetic finite-sweep result is an upper bound on dependency
propagation, not evidence of a speedup. With active RHS batch sizes a_l,
p workers, callback service time F, per-batch overhead B_l, serial
solve/projection/lifting time S_l, certification V, setup C, and M
successfully reused steps, a balanced-service accounting model is

\[
 T_{\rm batch}=\frac{C}{M}+
 \sum_\ell\{\lceil a_\ell/p\rceil F+B_\ell+S_\ell\}+V.
 \tag{G14}
\]

The model's break-even condition is exactly T_batch<T_forward at matched
accuracy and charged failures. Its constants are measured service costs
or justified bounds, not universal numerical-analysis constants. If they
are intervals, a justified choice needs upper(T_batch)<lower(T_forward);
otherwise the comparison is unresolved. Unequal callback times require
the actual schedule model in place of ceil(a_l/p)F.

For a chain of eight stages the existing reused worst-case example has
28 nonlinear evaluations against seven fresh RHS evaluations in forward
substitution. Caching cuts duplicate work but does not make this smaller
than forward substitution. Early certified termination or sufficiently
cheap parallel batches must pay the difference. Projected LU construction,
all basis work, full-space checks, rejected attempts, and epoch failures
belong in the cost receipt.

Replacing an m-vector Krylov solve by a degree-m polynomial still has m
dependent operator applications in an ordinary recurrence. It removes
orthogonalization/storage when justified; it does not itself parallelize
that recurrence. Useful independent axes are trajectories, RHS blocks,
known quotient channels, and independent stage tasks.

### 4.4 Derived affine scan opportunity, with its precise boundary

For predetermined affine steps y_(k+1)=S_k y_k+v_k, pairs compose by

\[
 (S_2,v_2)\circ(S_1,v_1)
 =(S_2S_1,S_2v_1+v_2). \tag{G15}
\]

Associativity follows from affine-map composition. A balanced prefix scan
has O(N) pair-compositions and O(log N) dependency depth. When S_k lies in
the two-projector algebra, matrix composition needs two scalar products;
vector shifts still cost O(n), and all stored outputs cost O(Nn).
The statement assumes the step sequence and all affine coefficients are
known independently of the evolving state. Ordinary adaptive acceptance
does not satisfy that premise. Speculative intervals require rollback and
charge discarded work. This is a limited parallel route, not a generic
parallel-in-time claim for nonlinear adaptive solves.

## 5. Geometric alternatives that deserve distinct polyalgorithm routes

### 5.1 Declared separable Hamiltonian: force-only splitting

For H(q,p)=T(p)+V(q), with supplied gradient actions and well-defined
subflows, velocity Verlet is

\[
 p_{1/2}=p-\frac h2\nabla V(q),\qquad
 q_+=q+h\nabla T(p_{1/2}),\qquad
 p_+=p_{1/2}-\frac h2\nabla V(q_+). \tag{G16}
\]

It uses force/kinetic evaluations, not Jacobian, JVP, or Newton callbacks.
The split must be declared; a generic black-box RHS does not identify
T and V. Its fixed-h exact-arithmetic map is symplectic because it is a
composition of Hamiltonian subflows.

For the oscillator q'=p, p'=-omega^2 q, the map is

\[
 M(h)=
 \begin{pmatrix}
 1-z^2/2&h\\
 -h\omega^2(1-z^2/4)&1-z^2/2
 \end{pmatrix},\qquad z=h\omega.
 \tag{G17}
\]

**Derived exact stability boundary.** det M=1 and trace M=2-z^2.
For omega>0 and a fixed positive h, 0<h omega<2 gives distinct
unit-modulus eigenvalues and a power-bounded map. At h omega=2 the map
has a nontrivial Jordan block and is not power bounded. Thus a certified
largest frequency supplies the strict fixed-step bound; a fitted safety
threshold is not the underlying theorem.

**Derived variable-step caution.** Each state-independent step map remains
symplectic, but local fixed-step stability does not imply product
stability. For omega=1, h_1=1 and h_2=19/10,

\[
 \det(M(h_2)M(h_1))=1,\qquad
 \operatorname{tr}(M(h_2)M(h_1))=-9661/4000<-2. \tag{G18}
\]

The periodic two-step map is hyperbolic although both individual steps
have h omega<2. This is a specific counterexample, not a prohibition of
all variable-step geometric methods.

State-dependent h can also destroy symplecticity itself. For the exact
free-particle flow, h(q,p)=epsilon q gives
(q_+,p_+)=(q+epsilon q p,p), whose Jacobian determinant is
1+epsilon p, generally not one. A state-dependent route switch can
introduce the same issue. A genuine adaptive geometric extension needs
its own extended-phase-space/time-transformation or another proved
construction.

### 5.2 Exact composition constants, and a dissipative boundary

Starting from a symmetric second-order method S_h, the triple composition
S_(a h) S_(b h) S_(a h) cancels the cubic local-log term when

\[
 2a+b=1,\qquad 2a^3+b^3=0,\qquad
 a=\frac1{2-2^{1/3}},\quad
 b=-\frac{2^{1/3}}{2-2^{1/3}}. \tag{G19}
\]

These are algebraic constants, not hyperparameters. The construction
assumes the required smooth formal expansion and reversible substeps.
Because b<0, it can require an unavailable or ill-conditioned backward
subflow for an irreversible semigroup. Positive-time Strang splitting
remains a separate second-order candidate for dissipative models; it
does not inherit fourth order from (G19).

### 5.3 Inexact implicit RK: residual-based invariant drift

Let I(y)=y^T C y with C symmetric, and let y^T C f(y)=0.
Consider computed stages Y_i, evaluated slopes
F_i=f(Y_i)+eta_i, and stage defects

\[
 \delta_i=Y_i-y-h\sum_j a_{ij}F_j,\qquad
 z=y+h\sum_i b_iF_i.
\]

**Derived identity.**

\[
\begin{split}
 I(z)-I(y)=&
 -h^2\sum_{i,j}
 (b_i a_{ij}+b_j a_{ji}-b_i b_j)F_i^TCF_j\\
 &-2h\sum_i b_i\delta_i^TCF_i+
 2h\sum_i b_iY_i^TC\eta_i.
\end{split}\tag{G20}
\]

**Proof.** Expand the quadratic endpoint increment. Substitute
y=Y_i-h sum_j a_ij F_j-delta_i into the linear term.
Symmetrize the double sum and use Y_i^T C f(Y_i)=0. QED.

For a tableau satisfying the symplectic RK coefficient identities, the
first term vanishes. Hence

\[
 |I(z)-I(y)|\le
 2h\|C\|\sum_i|b_i|
 \bigl(\|\delta_i\|\|F_i\|+\|Y_i\|\|\eta_i\|\bigr).
 \tag{G21}
\]

For the represented endpoint hat(z)=z+xi add
2||C|| ||z|| ||xi||+||C|| ||xi||^2. If represented tableau coefficients
do not satisfy the algebraic identity exactly, retain the first term of
(G20) with an enclosure. A rational/symbolic identity for ideal coefficients
does not erase the represented-tableau defect.

This creates a concrete residual budget for a derivative-free homotopy
stage solver used by a geometric RK route. The residual must be translated
to these stage-state units; a raw-U residual cannot simply be substituted.
An invariant-drift certificate is not a proof of exact symplecticity of the
inexact numerical map: the latter concerns its derivative with respect to
the input. No Jacobian is needed to evaluate (G20) when the invariant and
actions are supplied.

For a linear field A with A^T C+C A=0, inexact midpoint gives an even
simpler identity. If
q=(I-hA/2)y_+-(I+hA/2)y and m=(y_++y)/2, then

\[
 I(y_+)-I(y)=2m^TCq. \tag{G22}
\]

Indeed y_+-y=hAm+q and the A term cancels. This makes the geometric
meaning of the actual root residual explicit; it does not require a
universal solver tolerance.

### 5.4 Declared invariant subspace plus complementary polynomial

Let P be an orthogonal projector, Q=I-P, and AP=PA.
For every analytic matrix function g on the required spectrum,

\[
 g(A)v=g(A|_{\operatorname{range}P})Pv+
        g(A|_{\operatorname{range}Q})Qv. \tag{G23}
\]

The proof is invariance and block diagonality. A known low-dimensional
oscillatory or slow block can therefore use an exact quotient/rotation,
while a dissipative complement uses a certified Chebyshev/Laguerre action.
This can shrink the spectral interval and remove unnecessary polynomial
degree. It is a constructive alternative to asking one polynomial family
to cover incompatible spectral geometry.

If P is only approximately invariant, define A_0=PAP+QAQ and
delta=||A-A_0||=||[A,P]|| in Euclidean operator norm. The equality follows
by multiplication with the orthogonal involution 2P-I, up to sign.
If both A and A_0 generate semigroups bounded by exp(omega t), then

\[
 \|e^{tA}-e^{tA_0}\|\le t\,\delta e^{\omega t}. \tag{G24}
\]

**Proof.** Apply Duhamel's integral identity to the two exponentials and
bound its integrand by delta exp(omega(t-s))exp(omega s). QED.
For a dissipative A the same dissipation bound holds for its orthogonal
block-diagonal compression, providing a common omega.

The leakage term t delta exp(omega t)||v|| must be added to polynomial
and arithmetic error. A sampled commutator action is not an operator-norm
certificate. A supplied orthogonal projector with analytic identity or
certified leakage is a prerequisite. Approximate deflation must retain the
complement and its error; dropping it is a different model reduction.

### 5.5 Constrained reduction provider, not inferred generic DAE support

For E(t)x'=A(t)x+f(t), suppose a declared constraint C(t)x=g(t)
admits a smooth lift x=V(t)z+w(t), with C V=0 and C w=g.
Choose a test matrix Z for which Z^T E V is invertible. Any consistent
lifted solution must satisfy

\[
 z'=(Z^TEV)^{-1}Z^T
 \{A(Vz+w)+f-E(\dot Vz+\dot w)\}. \tag{G25}
\]

This is a direct substitution identity. Its sufficiency requires that the
discarded original equations follow from the declared constraints and
projected equation, or an independent original-residual check with a valid
equivalence theorem. Invertibility of the small projected matrix alone
does not establish DAE equivalence.

The connection terms dot(V)z+dot(w) are essential for a moving basis.
If the provider supplies the reduced RHS, reconstruction, and analytic
small solve or exact action, the integration core can remain RHS-only.
Constructing arbitrary derivative arrays, tangent bases and solves from a
black-box residual generally needs additional derivatives/linear algebra.
Canonical symplecticity also requires the correct pulled-back two-form;
an arbitrary nullspace basis is insufficient.

For a reconstructed state hat(x)=Vz+w+xi,
the constraint defect is C xi when the exact provider identities hold.
With approximate identities, retain (CV)z+(Cw-g)+C xi. This gives an
explicit model-supplied constraint budget, rather than an unqualified
DAE-production claim.

## 6. Concrete candidate and PR contracts

| Candidate | Priority / PR | Conditions | Computational consequence | Mandatory evidence before promotion |
|---|---|---|---|---|
| G-TRANSPORT | First, PR-N03 | Immutable two-projector affine family; each accepted h recorded; full residual enclosures. | Two scalar adjoints per new h; product-form transport avoids repeatedly inflated scalar norms. | Outward variable-step bounds, endpoint/clock terms, n=1 and d=kappa cases, epoch rejection, charged failures. |
| G-EXACT-FLOW | First, PR-N03 | Declared affine forcing and rates, reliable scalar exponential/phi enclosures. | One exact structural flow and O(n) work; reuse quotient-flow architecture. | Direct flow versus existing structural route at matched physical error; tiny and large arguments; no hidden setup charge. |
| G-CAUSAL-BUDGET | First, PR-N05 | Full-space residual, valid inverse/remainder constants and stage-domain enclosure. | Endpoint tolerance comes from a finite triangular majorant; no fitted contraction threshold. | (G11)/(G13) against independent high-precision roots, nonlinear domains, basis/epoch changes, exact target/order identity. |
| G-INVARIANT-SPLIT | Second, PR-N03/N05 extension | Declared invariant or leakage-certified P, known reduced action, complementary spectral certificate. | Exact small block plus smaller-degree polynomial on the complement. | Leakage, projector roundoff, total projection cost, no unreported discarded component. |
| G-GEOMETRIC-RK | Second, optional PR after N05 | Hamiltonian split or exact RK coefficient identity; invariant provider; explicit stepping contract. | Force-only Verlet/Strang or residual-budgeted derivative-free implicit RK. | Long-horizon invariant and phase error, inexact roots/RHS, step schedule counterexamples, honest symplecticity scope. |
| G-AFFINE-SCAN | Third, optional parallel PR | Predetermined affine grid and fixed algebra; sufficient output/storage budget. | O(log N) pair-composition span with O(N) algebra work. | Strong sequential baseline, output bandwidth, setup/storage costs, discarded speculation charges if attempted. |
| G-DAE-PROVIDER | Third, optional extension | Supplied regular reduction/reconstruction/equivalence and geometry, including basis derivatives. | Reuse existing ODE engines on a valid reduced problem. | Original DAE residual, constraint drift, rank/epoch invalidation, reduced-coordinate geometry. |

The implementation should distinguish construction from admission:
declared structure chooses which mathematically eligible candidates exist;
charged measured work then chooses among those candidates at a matched error
target. An adapter must not infer a global affine, Hamiltonian, normal, or
constant-rank property from a handful of cheap probes.

Suggested provider fields are identity/epoch, state and time domain,
structure kind, exact action or subflow callbacks, model-bound constants,
coefficient/rounding provenance, residual norm and units, and work receipts.
For an arbitrary nonlinear model, evaluating a polynomial of J requires
J actions; matrix-free is not automatically derivative-free. For a declared
affine f(v)=Av+b, A v=f(v)-f(0) is an exact real-arithmetic RHS construction,
but both RHS work and cancellation error must be counted.

## 7. One-pass local verification plan and limits

The standalone CHECK.py belongs to stage-local supporting derivation
verification. It is not a Rust/native test, benchmark, profile, or proof
of a production floating-point enclosure. The planned exact-rational checks
are:

1. Variable-step scalar-adjoint transfer against independently assembled
   full block matrices, including sharp accumulated row-sum norms.
2. Carrier-aware nilpotent majorant for a nonlinear triangular example
   whose unweighted majorant norm exceeds one.
3. Inexact midpoint quadratic-invariant identity including RHS and endpoint
   defects.
4. Verlet determinant and the rational two-step instability counterexample.

Execution waits for the parent campaign's preregistered check specification.
Results, when executed, must identify the script and preregistration commit.
No observed speed or robustness improvement is asserted by these identities.
Each coding task remains bounded by the inherited at-most-three correction
loops; new theory cannot retroactively convert older measurements into a
fresh benchmark.

## 8. Skill context and sources

skill_context_used: PhysMath Research Loop 1.3.0; supplied SKILL.md,
STAGE_GUIDE_KO.md, POLICY_INTEGRATION.md and SUBAGENT_MODE.md context;
R2/R4 only; no recursive delegation; exact-constant constructive derivations;
existing evidence reused with pinned identity; broad no-go language avoided.

Primary external sources actually read:

- Kunkel–Mehrmann, arXiv:2205.08105, Hypothesis 2.1, Sections 3–4,
  Theorems 4.2/4.4, Experiment 5.2 and conclusions:
  https://arxiv.org/pdf/2205.08105
- Habib Ammari, ETH Numerical Analysis II Lecture 5, the splitting,
  leapfrog and Runge–Kutta sections and
  the quadratic-invariant/symplectic-RK derivation:
  https://people.math.ethz.ch/~grsam/FS18/NAII/files/slides/ODE-Lecture5.pdf

Repository sources are the pinned paths in Section 1. Derived propositions
here do not depend on secondary web summaries or source benchmark claims.
