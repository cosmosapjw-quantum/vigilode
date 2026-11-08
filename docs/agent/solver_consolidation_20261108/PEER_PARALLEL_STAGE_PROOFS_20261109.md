# Constructive proofs for CPU-parallel peer integration

**Date:** 2026-11-09  
**Repository:** cosmosapjw-quantum/vigilode  
**Research source lock:** branch research/vigilode-c5-native-graph-schur-20261102, commit 569ce02629498fec133db9ce368eb303ba943b69, tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae  
**Documentation branch:** docs/solver-consolidation-plan-20261108  
**Status:** THEORY_NOTE_PUSHED; NO_SOLVER_IMPLEMENTATION; NOT_RELEASE_QUALIFIED  
**Execution policy:** CPU only. GPU is prohibited in all branches, tests, fallbacks and benchmarks.

This note records constructive results and negative compatibility findings obtained after the source-locked solver portfolio review. It does not change any Rust source, tableau, production default, tolerance, or release gate.

## 1. Results in one page

1. A three-peer explicit method is constructed with exact variable-step coefficients. It is third order, FSAL, stage-parallel, and optimally zero-stable. Its negative-real-axis stability interval is exactly bounded by
   \[
   z\in[-x_*,0],\qquad x_*=\frac{12-2\sqrt{15}}7\approx0.607719043940738.
   \]
   It is a proof witness, not a production candidate.

2. The proof witness does **not** satisfy the double-quasi-consistency (DQC) condition required by the published explicit-peer global-error estimator. Its failed condition is exact:
   \[
   P\delta_4=(4,4,4)^T\ne0.
   \]
   The DQC estimator must therefore not be attached to this tableau.

3. The actual EPPEER implementation is a strong nonstiff reference: it has explicit parallel peer methods of orders 3–9, uses DOPRI5 for starting values, includes automatic step-size control and full-order dense output, and executes method-level stages in parallel without requiring parallelization of the user RHS. Odd-stage FSAL methods use \(s-1\) new RHS evaluations; even-stage methods expose \(s\) independent stages. Speedup is workload-dependent and reported mainly for expensive RHS evaluations.

4. For stiff problems, a common-shift parallel two-step W-peer remains the first general candidate. Setting the same-step coupling matrices to zero makes all stages independent and gives them the same matrix \(W_m=I-h_m\gamma T_m\). If the existing collective operator is exactly rank-\(r\), the stage solves admit an exact shared Woodbury/Schur reduction.

5. The published 2011 W-peer error-control work already supplies local/global error estimation and automatic step-size control. Its extracted text confirms that the estimator systems reuse the integration matrix, but the mathematical symbols in the PDF extraction are incomplete. Exact estimator coefficients must be transcribed from the typeset source before implementation; they are not reconstructed by guesswork here.

6. A finite-dimensional linear parent lift is available for analytic Poincaré-Dulac normal forms in a Poincaré domain. Numerical drift away from the algebraic graph does not, by itself, destroy the order of the projected original coordinates. A nonlinear retraction is not required merely to retain convergence order.

## 2. Explicit parallel peer: order conditions

Consider the explicit two-step peer recurrence
\[
Y_{n+1}=P Y_n+h_{n+1}Q(r_n)F_n,\qquad
F_n=\bigl(f(t_n+c_jh_n,Y_{n,j})\bigr)_{j=1}^s,
\]
where \(r_n=h_{n+1}/h_n>0\), \(c\) is the new-step node vector, and \(Y_n\) contains the previous-step peer values. The previous nodes expressed in units of the new step and relative to \(t_{n+1}\) are
\[
d(r)=\frac{c-\mathbf1}{r}.
\]

For fixed \(r\), Taylor expansion of the exact solution around \(t_{n+1}\) gives the moment conditions
\[
c^{\odot q}
=
P\,d(r)^{\odot q}
+
qQ(r)d(r)^{\odot(q-1)},\qquad q=0,\ldots,p,
\tag{2.1}
\]
with the \(q=0\) equation interpreted as \(P\mathbf1=\mathbf1\). Here powers are componentwise. These conditions are sufficient for a local defect \(O(h^{p+1})\) for a sufficiently smooth exact solution. Global convergence also requires zero-stability, bounded coefficients on the admissible step-ratio set, and compatible starting peer values.

### 2.1 A concrete three-peer witness

Take
\[
c=(0,\tfrac12,1)^T,\qquad
P=\begin{pmatrix}
0&0&1\\0&0&1\\0&0&1
\end{pmatrix},\qquad
d(r)=(-1/r,-1/(2r),0)^T.
\]
Choose \(Q(r)\) as the Lagrange-integral matrix
\[
Q_{ij}(r)=\int_0^{c_i}\ell_j(\theta;d(r))\,d\theta,
\]
where \(\ell_j\) are the Lagrange cardinal polynomials at the three old nodes. Direct integration gives
\[
Q(r)=
\begin{pmatrix}
0&0&0\\[1mm]
\frac{r(2r+3)}{24}&-\frac{r(r+3)}6&
\frac{2r^2+9r+12}{24}\\[2mm]
\frac{r(4r+3)}6&-\frac{2r(2r+3)}3&
\frac{4r^2+9r+6}{6}
\end{pmatrix}.
\tag{2.2}
\]
The step is
\[
Y_{n+1,1}=Y_{n,3},
\]
\[
Y_{n+1,2}=Y_{n,3}+h_{n+1}
 \left(Q_{21}F_{n,1}+Q_{22}F_{n,2}+Q_{23}F_{n,3}\right),
\]
\[
Y_{n+1,3}=Y_{n,3}+h_{n+1}
 \left(Q_{31}F_{n,1}+Q_{32}F_{n,2}+Q_{33}F_{n,3}\right).
\]
The first peer is a copied endpoint (FSAL). The other two new peer values depend only on the old step, so their RHS evaluations can be performed concurrently. The stage equation contains no current-step peer RHS.

**Order proof.** The interpolation polynomial through the three old RHS samples is exact for every polynomial of degree at most two. For an exact solution \(y\), the RHS is \(y'\). Hence integrating the interpolant from \(0\) to \(c_i\) reproduces the Taylor terms \(y,y',y'',y'''\) at every new peer node. Equivalently, substitution into (2.1) gives zero moment residual for \(q=0,1,2,3\), so the local defect is \(O(h^4)\). Thus the method has classical order three.

**Zero-stability proof.** The output matrix satisfies
\[
P^2=P,\qquad \operatorname{spec}(P)=\{1,0,0\},\qquad
\sup_{n\ge0}\|P^n\|_\infty=1.
\]
The unit root is simple and all other roots are strictly inside the unit disk. Therefore the method is optimally zero-stable.

**Variable-step defect bound.** Let \(M_4\) bound \(\|y^{(4)}\|\) over the interpolation stencil and the new interval. The interpolation remainder for \(f=y'\) is
\[
f(t_{n+1}+h_{n+1}\theta)-p_2(\theta)
=
\frac{h_{n+1}^3y^{(4)}(\xi_\theta)}{3!}
(\theta+1/r)(\theta+1/(2r))\theta.
\]
After integration,
\[
\|\tau_{n+1,2}\|
\le \frac{h_{n+1}^4M_4(r+2)^2}{384r^2},
\qquad
\|\tau_{n+1,3}\|
\le \frac{h_{n+1}^4M_4(r+1)^2}{24r^2}.
\tag{2.3}
\]
The first-stage defect is zero because it is the copied endpoint.

**Global convergence proof.** Assume \(f\) is Lipschitz in \(y\) with constant \(L\) on a tube containing exact and numerical trajectories, the step ratios satisfy \(0<r_{\min}\le r_n\le r_{\max}<\infty\), the maximum step is \(H\), and the initial peer error is \(O(H^3)\). Then \(Q(r)\) is bounded on the compact ratio interval. With the maximum stage error \(E_n\),
\[
E_{n+1}
\le \left(1+Lh_{n+1}\|Q(r_n)\|_\infty\right)E_n+C(r_n)h_{n+1}^4.
\]
Discrete Grönwall and \(\sum_n h_n^4\le H^3\sum_n h_n\le H^3(T-t_0)\) imply
\[
\max_{t_n\le T}E_n=O(H^3).
\]
This proves order-three convergence under the stated assumptions. It does not prove a useful stability region for stiff modes.

## 3. Corrected negative-real-axis stability proof

At constant step size \(r=1\), the characteristic polynomial of the stability matrix \(P+zQ(1)\) is
\[
\chi(w,z)=w^3-(1+\tfrac52z)w^2
 +(\tfrac32z+\tfrac{13}{12}z^2)w-\frac{z^2}{12}.
\]
Set \(z=-x\), \(x\ge0\). Then
\[
p_x(w)=w^3+bw^2+cw+d,
\quad
b=-1+\tfrac52x,\quad
c=-\tfrac32x+\tfrac{13}{12}x^2,\quad
d=-\tfrac1{12}x^2.
\tag{3.1}
\]
The signs in (3.1) are important. An earlier informal presentation reversed the signs of the terms linear in \(x\); this note supersedes that presentation.

For \(|d|<1\), the first Schur reduction gives
\[
p_1(w)=\frac{p_x(w)-d\,w^3p_x(1/w)}{w}
=Aw^2+Bw+C,
\]
where
\[
A=1-d^2,\qquad B=b-cd,\qquad C=c-bd.
\]
Exact factorization yields
\[
A=\frac{144-x^4}{144},
\]
\[
A+B+C=\frac{x(x+1)(x^2+12)}{12},
\]
\[
A-B+C=-\frac{(x^2-12)(7x^2-24x+12)}{72},
\]
\[
A-C=-\frac{(x^2+6x-12)(x^2+24x+12)}{144},
\]
\[
A+C=\frac{144-216x+144x^2+30x^3-x^4}{144}.
\]
Let
\[
x_*=\frac{12-2\sqrt{15}}7<1.
\]
For \(0<x<x_*\), all five quantities above are strictly positive:
- \(A>0\) because \(x_*<1<\sqrt{12}\);
- \(A+B+C>0\) directly;
- \(A-B+C>0\) because \(x^2-12<0\) and \(7x^2-24x+12>0\) before its first positive root \(x_*\);
- \(A-C>0\) because \(x^2+6x-12<0\) for \(0\le x<1\), while \(x^2+24x+12>0\);
- \(A+C>0\) because \(144x^2-216x+144\) has negative discriminant and positive leading coefficient, and \(x^3(30-x)>0\) for \(0<x<1\).

These are the quadratic Schur conditions, so both roots of \(p_1\) lie strictly inside the unit disk. The first Schur reduction then gives the strict root condition for \(p_x\). At \(x=x_*\), \(A-B+C=0\); the reduced quadratic has \(w=-1\) as a root, and since \(|d|<1\), the original cubic also has \(w=-1\). At \(x=0\), the roots are \(1,0,0\), with the unit root simple. Hence the exact negative-real-axis stability interval is
\[
\boxed{z\in[-x_*,0],\qquad x_*\approx0.607719043940738.}
\tag{3.2}
\]
This is a narrow interval. The witness is not A-stable or L-stable and is not the proposed nonstiff production tableau.

## 4. DQC audit: a precise negative result

Kulikov and Weiner's explicit-peer DQC construction uses, in fixed-step notation,
\[
X_k=(B\otimes I)X_{k-1}
+\tau(A\otimes I)F_{k-1},
\]
with no current-step RHS matrix. Define its defect moments
\[
\delta_\ell
=c^{\odot\ell}
-B(c-\mathbf1)^{\odot\ell}
-\ell A(c-\mathbf1)^{\odot(\ell-1)}.
\tag{4.1}
\]
The zero-th condition is interpreted as \(B\mathbf1=\mathbf1\). Ordinary consistency of order \(p\) requires \(\delta_\ell=0\) for \(\ell=0,\ldots,p\), together with zero-stability. The stronger DQC conditions include
\[
B\delta_p=0,\qquad B\delta_{p+1}=0,\qquad A\delta_p=0,
\tag{4.2}
\]
in addition to the lower moment conditions.

For the three-stage witness above at \(r=1\), take \(B=P\), \(A=Q(1)\), and \(p=3\). Exact arithmetic gives
\[
\delta_0=\delta_1=\delta_2=\delta_3=0,\qquad
\delta_4=(0,\tfrac9{16},4)^T,
\]
and therefore
\[
P\delta_4=(4,4,4)^T\ne0.
\]
The method is third order but not doubly quasi-consistent. Its embedded global-error estimate cannot be justified by the DQC theorem.

For a DQC explicit-peer method, the published embedded estimator has the form
\[
\Delta_1 X_k
=(B_{\rm emb}-B)\otimes I\,X_{k-1}
+\tau(A_{\rm emb}-A)\otimes I\,F_{k-1},
\tag{4.3}
\]
where the embedded peer has the same classical order but suitable different local-error terms. Equation (4.3) is not a universal peer estimator: the DQC equations and the embedded-pair hypotheses are part of its proof. The fixed-step construction (2008) and its variable-step interpolating developments (2010/2012) must be treated as distinct coefficient families.

**Decision:** either adopt a published variable-step DQC peer tableau after checking its exact conditions, or construct a new embedded DQC pair. Do not attach (4.3) to arbitrary EPPEER methods or to the three-stage witness.

## 5. DOPRI5 and explicit peer: what can and cannot be transformed

The official EPPEER project page describes seven explicit parallel peer methods of orders 3–9. The starting peer values are produced by a Runge-Kutta starter; the manual specifies DOPRI5 and the source file ivprkp.f90. The solver includes automatic step-size control and optional continuous output of full order. Its 4-, 6-, and 8-stage methods expose all stages as independent work; the odd-stage FSAL methods expose \(s-1\) new RHS evaluations. The project warns that useful speedup requires sufficiently expensive RHS evaluations and targets modest CPU core counts. This is evidence for a method-family replacement, not a claim that DOPRI5's own lower-triangular tableau became parallel.

### 5.1 A restricted no-go theorem for universal linear stage mixing

Let \(S\) be an invertible constant matrix mixing the stage-state vector. Suppose a transformed stage scheme is to preserve componentwise black-box RHS evaluation for every nonlinear scalar function \(f\), so that
\[
\mathcal F(SY)=S\mathcal F(Y),\qquad
\mathcal F(Y)_i=f(Y_i).
\tag{5.1}
\]
Choose \(f(y)=y^2\). The \(i\)-th component of (5.1) is
\[
\left(\sum_jS_{ij}Y_j\right)^2
=\sum_jS_{ij}Y_j^2.
\]
For independent variables \(Y_j\), all cross-term coefficients \(2S_{ij}S_{ik}\) with \(j\ne k\) must vanish. Thus each row has at most one nonzero entry. The coefficient of its square must satisfy \(s^2=s\), so each nonzero entry is \(1\). Invertibility forces one 1 in every row and every column. Therefore \(S\) is a permutation matrix.

This theorem is deliberately narrow: it excludes a universal dense linear mixing that commutes with arbitrary componentwise nonlinear RHS evaluation. It does not exclude transformations for a specified linear, polynomial, equivariant, or otherwise structured vector field.

**Decision:** retain DOPRI5 as the established nonstiff reference, startup and fallback. Use an explicit peer family as the main stage-parallel candidate when its stability and estimator conditions fit the problem. Do not claim that a basis transform parallelizes the existing DOPRI5 stages.

## 6. Common-shift Rosenbrock-W peer and rank-r collective operators

The two-step W-peer form includes
\[
(I-h_m\gamma T_m)k_{m,i}
=f(t_{m,i},Y_{m,i})+
h_mT_m\sum_j\gamma_{ij}k_{m-1,j}
+h_mT_m\sum_{j<i}\widetilde\gamma_{ij}k_{m,j}.
\]
The stage abscissa also has old-step and current-step terms with coefficients \(A\) and \(\widetilde A\). The fully parallel subclass sets
\[
\widetilde A=0,\qquad\widetilde\Gamma=0.
\]
Then every right-hand side depends only on committed previous-step peer data, and all stages share
\[
W_m=I-\alpha_mT_m,\qquad\alpha_m=h_m\gamma.
\]
This is a different method from the protected one-step RODAS5P tableau. It must be implemented and validated as a distinct method.

### 6.1 Exact rank-r Woodbury factorization

Suppose the actual source-owned operator has an exact decomposition
\[
T_m=D_m+U_mV_m^T,\qquad U_m,V_m\in\mathbb R^{N\times r}.
\]
Let
\[
M_m=I-\alpha_mD_m,\qquad
S_m=I_r-\alpha_mV_m^TM_m^{-1}U_m.
\]
If \(M_m\) and \(S_m\) are nonsingular, then
\[
\boxed{
W_m^{-1}
=M_m^{-1}
+\alpha_mM_m^{-1}U_mS_m^{-1}V_m^TM_m^{-1}.
}
\tag{6.1}
\]
To prove (6.1), put \(K=V_m^TM_m^{-1}U_m\). Multiplication of the right-hand side by \(W_m=M_m-\alpha_mU_mV_m^T\) leaves the identity plus a correction proportional to
\[
S_m^{-1}-I_r-\alpha_mKS_m^{-1}
=(I_r-\alpha_mK)S_m^{-1}-I_r=0.
\]
Thus (6.1) is exact.

For one peer step, factor \(M_m\) once, compute \(Z_m=M_m^{-1}U_m\) and \(S_m^{-1}\) once, then solve all independent stage right-hand sides. For each stage RHS \(r_i\):
\[
x_i=M_m^{-1}r_i,\qquad
k_i=x_i+\alpha_mZ_mS_m^{-1}V_m^Tx_i.
\]
The stage corrections are independent. For the existing rank-two collective backend this applies with \(r=2\), but only when its decomposition is exact for the selected \(T_m\) and common to every stage. No approximation of a generic Jacobian as rank two is licensed by this identity.

### 6.2 Inexact linear solves and preservation of order

If \(W_mk_i=r_i\), an approximate stage \(\widetilde k_i\) has residual
\[
\rho_i=r_i-W_m\widetilde k_i.
\]
If \(\|W_m^{-1}\|\le\kappa\) uniformly, then
\[
\|\widetilde k_i-k_i\|\le\kappa\|\rho_i\|.
\]
The external update multiplies stage derivatives by \(h_m\). Thus a residual of size \(O(h_m^p)\), with bounded \(\kappa\), contributes \(O(h_m^{p+1})\) to the one-step solution defect. Under the method's zero-stability and compatible starting assumptions this does not lower a formal order-\(p\) method. A practical solver must still combine truncation and algebraic solve error in its acceptance logic; a residual certificate is not an ODE truncation-error estimator.

## 7. Published peer-W error control: reuse, do not reinvent

Kulikov and Weiner (2011) develop local and global error estimation and automatic step-size control for linearly implicit parallel two-step peer W methods. The extracted article text establishes the following structural facts:

- the estimator is based on the peer method's asymptotic defect expansion and an error-propagation recurrence;
- practical defect derivatives are approximated from divided differences of extrapolated numerical values;
- the local and global error equations use the same coefficient matrix as the integration equations;
- in the singly implicit specialization, one matrix factorization per mesh point can support step advancement and error estimation.

This is directly relevant to common-shift W-peer. However, the available PDF text extraction drops several mathematical symbols in equations (11)–(25). The exact recurrence, derivative coefficients, startup conditions and controller constants therefore remain **source-transcription blocked**. Do not substitute a guessed formula or an embedded RK estimator. Recover the typeset equations and match every assumption to the chosen coefficient family before implementation.

The original parallel two-step W literature also distinguishes:
- common-shift parallel W-peer, where one matrix can serve all stage RHS;
- multi-implicit peer W, where the stage equations remain independent but stage shifts differ, requiring separate solves or a proved multi-shift method;
- sequential Rosenbrock-type peer controls, which may retain lower-triangular current-stage coupling.

These are separate algorithms and benchmark arms.

## 8. Existing affine transport: parallelize the error envelope, not time stepping

For an explicit peer with Lipschitz RHS, the global-error bound has the form
\[
E_{n+1}\le a_nE_n+b_n,\qquad
a_n=1+Lh_{n+1}\|Q(r_n)\|,\quad b_n\ge0.
\]
Each step is an affine map \(T_n(E)=a_nE+b_n\). Composition is
\[
(a_2,b_2)\star(a_1,b_1)
=(a_2a_1,a_2b_1+b_2).
\]
This operation is associative because it is ordinary function composition; equivalently it is multiplication of upper-triangular matrices
\[
\begin{pmatrix}a_2&b_2\\0&1\end{pmatrix}
\begin{pmatrix}a_1&b_1\\0&1\end{pmatrix}.
\]
Therefore the existing affine-transport / associative-scan infrastructure can compute prefix bounds with \(O(N)\) work and \(O(\log N)\) span after the per-step \(a_n,b_n\) are known. This does not parallelize the sequential IVP trajectory. Reuse is exact only when the supplied recurrence and outward arithmetic contract match the existing implementation.

## 9. Finite resonant normal-form lift: corrected projection statement

Consider an analytic vector field near a hyperbolic equilibrium,
\[
\dot z=\Lambda z+R(z),\qquad
R_j(z)=\sum_{\nu\cdot\lambda=\lambda_j,\ |\nu|\ge2}
r_{\nu j}z^\nu,
\]
where all nonlinear monomials are resonant. Assume the eigenvalues lie in a Poincaré domain: there is a real linear functional \(\ell\) on the complex eigenvalue plane and \(\delta>0\) such that \(\ell(\lambda_j)\ge\delta\) for every \(j\).

For a monomial \(z^\mu\), its Lie derivative is
\[
\mathcal L_X(z^\mu)
=(\mu\cdot\lambda)z^\mu+
\sum_{j,\nu}\mu_jr_{\nu j}z^{\mu-e_j+\nu}.
\tag{9.1}
\]
Whenever a term in the sum is present, \(\nu\cdot\lambda=\lambda_j\), so
\[
(\mu-e_j+\nu)\cdot\lambda=\mu\cdot\lambda.
\]
Thus Lie differentiation preserves the monomial weight. For a coordinate monomial \(z_i\), every monomial generated has weight \(\lambda_i\). The Poincaré-domain functional gives
\[
|\mu|\delta\le \sum_j\mu_j\ell(\lambda_j)=\ell(\lambda_i),
\]
so only finitely many nonnegative integer multi-indices \(\mu\) can occur. The resonant monomial span generated from the coordinate functions is therefore finite-dimensional and invariant under \(\mathcal L_X\).

Augment the state by those monomials:
\[
\xi=(z_1,\ldots,z_n,z^{\mu_1},\ldots,z^{\mu_q})^T.
\]
Equation (9.1) gives a finite constant matrix \(B\) such that
\[
\dot\xi=B\xi.
\]
This is an exact finite-dimensional linear parent system on the lifted monomial coordinates, assuming the analytic normal-form transformation itself converges on the selected chart.

**Projection-order lemma.** Suppose a zero-stable order-\(p\) integrator is applied to the parent linear IVP with compatible initial lift, and the numerical parent solution satisfies \(\|\widetilde\xi_n-\xi(t_n)\|=O(H^p)\) on a fixed compact time interval. Let \(\pi_z\) select the original normal-form coordinates and let \(\Phi^{-1}\) be the inverse normal-form chart. If \(\Phi^{-1}\) is Lipschitz on a compact subset containing the exact and numerical projected trajectories, then
\[
\|\Phi^{-1}(\pi_z\widetilde\xi_n)-x(t_n)\|
\le \operatorname{Lip}(\Phi^{-1})\,
\|\pi_z(\widetilde\xi_n-\xi(t_n))\|
=O(H^p).
\]
Hence numerical drift away from the algebraic graph \(\xi_{\rm lift}=z^\mu\) does **not** by itself reduce the order of the extracted original coordinates. A retraction may be useful for constraint enforcement or conditioning, but it is not required merely to preserve convergence order.

The restrictions remain substantial: convergence of the analytic normal form, Poincaré-domain hypothesis, chart radius, inverse-chart conditioning, lift dimension, and stability/conditioning of the parent matrix \(B\). A finite-order formal normal form, a truncated nonconvergent series, or a source-specific root certificate is not an exact parent lift.

Existing conditioned-polynomial and polynomial-resolvent code can be reused for declared operator actions and outward residuals where identities match. Existing quotient-flow actions apply only to their declared source class. They do not generate the normal form or certify its existence.

## 10. Implementation and evidence decision

### Reuse-first integration map

| Research node | Existing project component | Minimum new mathematical core |
|---|---|---|
| Explicit peer, nonstiff | DOPRI5 starter; existing bounded CPU executor; step session/controller and work ledger | Verified peer tableau, variable-node history, DQC or compatible error estimator, restart logic |
| Common-shift W-peer, stiff | ParallelExecution, stage_batch, Problem/LinearOperator, existing Krylov/direct solves, collective rank-two backend | Published coefficient table, step-ratio transformation, peer startup/restart, published W-peer estimator adapter |
| Error-bound scan | affine_transport and associative scan | Adapter from a verified local defect/solve-residual recurrence |
| Finite resonant parent | conditioned_polynomial, polynomial_resolvent, quotient_flow where exact | Resonance enumeration, homological normal form, finite parent generator and invariant-ideal proof |
| Independent IVP ensemble | existing DOPRI5 explicit_ensemble | Method-agnostic session adapter without pooling member controllers or failure state |

No second pool, generic solver frontend, homotopy engine, Fourier engine, or benchmark framework is justified.

### Evidence status

- **Derived:** explicit-peer moment conditions, variable-step interpolation coefficients, defect bounds, global order-three proof, rank-r Woodbury identity, inexact-solve order transfer, affine-bound associativity, no-go for universal nonlinear stage mixing, finite resonant parent closure and projection-order lemma.
- **Symbolically checked:** all variable-step moment residuals vanish exactly for the three-stage witness; the fixed-step characteristic polynomial and Schur factors are exact rational polynomials.
- **Independently checked:** rational moment identities and characteristic polynomial were evaluated in SymPy; the moment identities and determinant polynomial were also evaluated in Wolfram Language.
- **Literature-supported:** EPPEER startup/controller/parallel architecture; explicit-peer DQC conditions and estimator; two-step W-peer independence/order and 2011 W-peer local/global error control; multi-implicit peer-W and Poincaré normal-form framework.
- **Not implemented / not runtime-verified:** no Rust source has been changed; no Cargo build/test, solver run, CPU benchmark, or local package validator was performed in this research turn.
- **Unresolved:** exact production W-peer tableau and its admissible variable-step domain; exact 2011 W-peer estimator equations and startup constants; DQC-compatible variable-step explicit peer tableau and embedded pair for the intended DOPRI5 replacement; event/rejection restart rules; matched-achieved-error efficiency.

### Acceptance gates before code

1. Lock exact source pages and coefficient table for a common-shift peer-W with the required variable-step order, zero-stability, stiff damping and step-ratio bounds.
2. Transcribe and independently verify its order, stability and W-perturbation conditions in exact arithmetic.
3. Transcribe the 2011 local/global estimator and startup assumptions, then prove compatibility with that exact tableau.
4. For nonstiff peer, obtain the EPPEER coefficient/starter implementation or a published DQC interpolating tableau; verify variable-step nodes and embedded estimator.
5. Replay existing repository tests and extend existing tests only. Verify accepted steps, RHS calls, linear solves, algebraic residuals and CPU concurrency before timing.
6. Benchmark against protected DOPRI5/RODAS5P at matched achieved error with startup, factorization, history, rejection and estimator overhead included. GPU remains disabled.

## 11. Primary references

1. H. Podhaisky, *Parallele Zweischritt-W-Methoden*, PhD thesis, Martin-Luther-Universität Halle-Wittenberg (2002), [institutional document](http://webdoc.sub.gwdg.de/ebook/e/2002/pub/mathe/02H015/of_index.htm).
2. H. Podhaisky, B. A. Schmitt, R. Weiner, “Design, analysis and testing of some parallel two-step W-methods for stiff systems,” Applied Numerical Mathematics 42 (2002), 381–395, [doi:10.1016/S0168-9274(01)00162-3](https://doi.org/10.1016/S0168-9274(01)00162-3).
3. B. A. Schmitt, R. Weiner, “Parallel Two-Step W-Methods with Peer Variables,” SIAM Journal on Numerical Analysis 42 (2004), 265–282, [doi:10.1137/S0036142902411057](https://doi.org/10.1137/S0036142902411057).
4. R. Weiner, B. A. Schmitt, H. Podhaisky, “Parallel ‘Peer’ two-step W-methods and their application to MOL-systems,” Applied Numerical Mathematics 48 (2004), 425–439, [doi:10.1016/j.apnum.2003.10.005](https://doi.org/10.1016/j.apnum.2003.10.005).
5. B. A. Schmitt, R. Weiner, H. Podhaisky, “Multi-Implicit Peer Two-Step W-Methods for Parallel Time Integration,” BIT Numerical Mathematics 45 (2005), 197–217, [doi:10.1007/s10543-005-2635-y](https://doi.org/10.1007/s10543-005-2635-y).
6. G. Yu. Kulikov, R. Weiner, “Doubly quasi-consistent parallel explicit peer methods with built-in global error estimation,” Institute of Mathematics, Halle, Report 18 (2008), [PDF](https://www2.mathematik.uni-halle.de/reports/sources/2008/08-18report.pdf).
7. G. Yu. Kulikov, R. Weiner, “Variable-Stepsize Interpolating Explicit Parallel Peer Methods with Inherent Global Error Control,” SIAM Journal on Scientific Computing 32 (2010), 1695–1723, [doi:10.1137/090764840](https://doi.org/10.1137/090764840).
8. G. Yu. Kulikov, R. Weiner, “Global error estimation and control in linearly-implicit parallel two-step peer W-methods,” Journal of Computational and Applied Mathematics 236 (2011), 1226–1239, [doi:10.1016/j.cam.2011.08.006](https://doi.org/10.1016/j.cam.2011.08.006).
9. R. Weiner, G. Yu. Kulikov, H. Podhaisky, “Variable-stepsize doubly quasi-consistent parallel explicit peer methods with global error control,” Applied Numerical Mathematics 62 (2012), 1591–1603, [doi:10.1016/j.apnum.2012.06.018](https://doi.org/10.1016/j.apnum.2012.06.018).
10. B. A. Schmitt, “Manual for explicit parallel peer code EPPEER” (2012), and the [official EPPEER project page and source archive](https://www.mathematik.uni-marburg.de/~schmitt/peer/).
11. B. A. Schmitt, R. Weiner, “Parallel start for explicit parallel two-step peer methods,” Numerical Algorithms 53 (2010), 363–381, [doi:10.1007/s11075-009-9267-2](https://doi.org/10.1007/s11075-009-9267-2).
12. A. Gaeta, “Poincaré normal and renormalized forms,” and related analytic normal-form results; see [arXiv:math-ph/0106017](https://arxiv.org/abs/math-ph/0106017).

