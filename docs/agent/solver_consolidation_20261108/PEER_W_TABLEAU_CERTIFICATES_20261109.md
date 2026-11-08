# Source-certified peer tableaus and variable-step constructions

**Date:** 2026-11-09  
**Source lock:** research/vigilode-c5-native-graph-schur-20261102, commit 569ce02629498fec133db9ce368eb303ba943b69, tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae  
**Status:** exact PTSW2C tableau reconstructed and symbolically verified; no Rust implementation or runtime qualification.  
**Execution policy:** CPU only; GPU forbidden in tasks, dependencies, fallbacks and benchmarks.

## 1. Parallel two-step W-peer form

With sigma_m=h_m/h_(m-1), a parallel two-step W method has
\[
Y_m=\mathbf1\otimes u_m+h_m(A_m\otimes I)K_{m-1},
\]
\[
(I_s\otimes W_m)K_m=F(t_m\mathbf1+h_mc,Y_m)
+h_m(\Gamma_m\otimes T_m)K_{m-1},\quad
W_m=I-h_m\gamma T_m,
\]
\[
u_{m+1}=u_m+h_m\{(b^T\otimes I)K_m+(v^T\otimes I)K_{m-1}\}.
\tag{1}
\]
Current stage equations depend only on committed previous-step data. With common gamma and T_m, all stage systems share W_m and differ only in RHS. This method is distinct from protected one-step RODAS5P.

## 2. Exact variable-step PTSW2C tableau

Podhaisky (2002) gives the two-stage method
\[
c=(-1,5/9)^T,\quad \gamma=12/25,\quad
b=(1/28,27/28)^T,\quad v=(0,0)^T.
\tag{2}
\]
For every positive step ratio sigma:
\[
A(\sigma)=
\begin{pmatrix}
(8-9\sigma)/28&9(\sigma-4)/28\\
-5(8+5\sigma)/252&5(36+5\sigma)/252
\end{pmatrix},
\tag{3}
\]
\[
\Gamma(\sigma)=\frac1{175}
\begin{pmatrix}
24-54\sigma&54(\sigma-2)\\
24+30\sigma&-108-30\sigma
\end{pmatrix}.
\tag{4}
\]
The common matrix is
\[
\boxed{W_m=I-\frac{12}{25}h_mT_m.}
\tag{5}
\]

### 2.1 Exact order proof

For this convention the variable-step conditions are
\[
C(q):\ \sum_j a_{ij}(c_j-1)^{\ell-1}
=\sigma^{\ell-1}c_i^\ell/\ell,
\]
\[
\Gamma(q):\ \sum_j\Gamma_{ij}(c_j-1)^{\ell-1}
=-\gamma\sigma^{\ell-1}c_i^{\ell-1},
\]
\[
B(p):\ \sum_j b_j\sigma^{\ell-1}c_j^{\ell-1}
+\sum_jv_j(c_j-1)^{\ell-1}
=\sigma^{\ell-1}/\ell.
\]
Exact rational simplification of (3)-(4) gives zero residuals for C(2) and Gamma(2), identically in sigma. For the output conditions:
\[
b^T1=1,\qquad b^Tc=1/2,\qquad b^Tc^{\odot2}=1/3,
\]
so B(3) also has zero residual identically in sigma. This was checked in Wolfram Language using exact rational arithmetic.

The source convergence theorem gives p_*=min(q+1,p), provided the coefficient matrices are uniformly bounded and the initial solution/stage errors satisfy epsilon_0=O(H^p) and nu_0=O(H^q). Here q=2, p=3. Since A(sigma), Gamma(sigma) are affine in sigma, they are bounded on 0<sigma<=sigma_max. Thus the source theorem proves variable-step global order three under its W-method assumptions and compatible startup.

### 2.2 Stability and startup

The dissertation classifies PTSW2C as A(81.1 degree)-stable with limiting stiff amplification spectral radius rho_infinity=0.6796. It is not L-stable. The sector classification is source-reported; the full complex stability sector has not been independently recomputed here.

The source startup avoids backward integration. With T={c_i-1} union {0}, T_1=min T and t_1=t_0-h_0T_1, integrate forward to the shifted targets t_0+h_0(T_i-T_1). For PTSW2C, T={-2,-4/9,0}, so t_1=t_0+2h_0, and the required old stage derivative times are
\[
t_1+(c_1-1)h_0=t_0,\qquad
t_1+(c_2-1)h_0=t_0+\frac{14}{9}h_0.
\tag{6}
\]
The theorem requires solution startup error O(H^3) and old stage derivative error O(H^2). DOPRI5 can serve as a high-order starter provided the intermediate output has sufficient accuracy; that still needs an implementation contract/test.

### 2.3 Exact embedded estimator for PTSW2C

The dissertation's method-specific embedded formula has
\[
b_e=(19/560,513/560)^T,\quad \epsilon=1/10,\quad
V_1^{-1}=\begin{pmatrix}-2/7&9/7\\-9/14&9/14\end{pmatrix}.
\]
Its coefficient construction reduces exactly to
\[
\boxed{v_e(\sigma)=
\left(-(8+27\sigma)/560,\ 9(4+3\sigma)/560\right)^T.}
\tag{7}
\]
The embedded endpoint is
\[
\widetilde u_{m+1}=u_m+h_m(b_e^TK_m+v_e(\sigma_m)^TK_{m-1}).
\tag{8}
\]
The source uses normalized RMS component errors, rejects when the estimate exceeds one, and scales the next step with a safety factor 0.85 E_emb^(-1/s), clipped to [0.5,2], with additional history/hysteresis. This estimator is specific to the selected tableau; it is not interchangeable with DOPRI's embedded pair or the separate 2011 W-peer global-error estimator.

## 3. Stiff damping versus variable-step order

The dissertation proves that a multi-stage stiffly accurate PTSW satisfying B(s), C(s), Gamma(s) cannot also satisfy B(s+1) for arbitrary variable-step grids. The additional condition reduces to
\[
\int_0^\sigma\phi(t+1)\,dt=\sigma\gamma\phi(\sigma+1)
\]
for all positive sigma. Differentiation forces gamma=1/(s+1) and phi(x)=(x-1)^s; distinct nodes then collapse, impossible for s>1. Thus fixed-step superconvergence cannot be presumed for variable-step L-stable stiffly accurate methods.

The v=0 family can retain B(s+1) on variable meshes, but PTSW2C trades away L-stability for A(alpha)-stability. This is a real method-family Pareto tradeoff.

Related source controls:
- PTSW2A: gamma=4/5, c=(23/9,1), b=(0,4/5), v=(-27sigma/140,1/5+27sigma/140). Its A and Gamma matrices are
\[
A=\begin{pmatrix}529\sigma/252&23/9-529\sigma/252\\9\sigma/28&1-9\sigma/28\end{pmatrix},\
\Gamma=\begin{pmatrix}-46\sigma/35&-4/5+46\sigma/35\\-18\sigma/35&-4/5+18\sigma/35\end{pmatrix}.
\]
It is L-stable, but B(3) is only fixed-step and variable-step order is two. The node 23/9 is beyond the nominal endpoint.
- PTSW3B is source-reported L-stable, A(84.8 degree), rho_infinity=0; full exact coefficients and estimator remain to be transcribed.
- PTSW3C is source-reported A(87.3 degree), rho_infinity about 0.8844, variable-step B(4), with a look-ahead node 3.28.
- The thesis reports five- and six-stage examples as sensitive to step-size changes and not competitive in its tested implementation. Generic S6 should be demoted until a specific table justifies it.

## 4. General variable-step explicit peer coefficient generator

Let c contain s distinct nodes and let B*1=1. Form
\[
V_-=[(c_i-1)^{q-1}]_{i,q=1}^s.
\]
For r=h_(n+1)/h_n>0, define R columnwise:
\[
R(r)_{:,q}=
\frac{r^q c^{\odot q}-B(c-\mathbf1)^{\odot q}}{qr},
\quad q=1,\ldots,s,
\qquad A(r)=R(r)V_-^{-1}.
\tag{9}
\]
Then the explicit peer update
\[
Y_{n+1}=B Y_n+h_{n+1}A(r_n)F_n
\tag{10}
\]
satisfies
\[
c^{\odot q}=B((c-\mathbf1)/r)^{\odot q}
+qA(r)((c-\mathbf1)/r)^{\odot(q-1)},\quad q=0,\ldots,s.
\tag{11}
\]
For q=0 this is B*1=1.

**Proof.** Let v_q=(c-1)^(q-1), the q-th column of V_-. Equation (9) gives
A(r)v_q=[r^q c^q-B(c-1)^q]/(qr). As d=(c-1)/r, this is precisely q A(r)d^(q-1)=c^q-Bd^q. Taylor expansion around t_(n+1) gives local defect O(h^(s+1)). If B is power-bounded with a simple unit root and other eigenvalues strictly inside the unit disk, A(r) is bounded on the allowed ratio set, f is Lipschitz on the solution tube, and the initial peer-vector error is O(H^s), discrete Gronwall gives global order s. This constructs variable-step coefficients once a zero-stable B and nodes are chosen; it does not select a stability-optimal B.

## 5. EPPEER source audit and DQC boundary

The official EPPEER manual/source describes explicit two-step peer stages that use previous-step peer data only; methods with 3-9 stages; DOPRI5 startup; OpenMP parallelization across method stages; automatic step control; and optional full-order continuous output. Odd-stage FSAL methods reuse one RHS value and compute s-1 fresh independent RHS evaluations. The five-stage EPP5f3 path therefore exposes four fresh parallel RHS evaluations.

The source has its own variable-ratio coefficient transformation (pa0, pcv, ppv) and error-indicator/controller. The general generator (9) is a separate construction; exact coefficient equivalence must be tested before claiming it reproduces EPPEER's table.

Doubly-quasi-consistent (DQC) estimators apply to a specific coefficient family and its variable-step transform, not all explicit peers. The earlier three-stage interpolation witness fails its exact DQC condition, P delta_4=(4,4,4)^T !=0; do not attach the DQC estimator to it. Do not attach DQC estimates to EPP5f3 without checking its exact coefficients. Retain EPPEER's own estimator initially and independently establish its asymptotic scaling. The Fortran pure RHS callback contract also means Rust callbacks must be reentrant and free of unsynchronized side effects; Send+Sync alone does not imply this in the presence of interior mutability.

## 6. Exact rank-r common-W reuse

If the selected operator has the exact source-owned decomposition T_m=D_m+U_m V_m^T, with U,V in R^(N x r), let alpha=h_m gamma, M=I-alpha D and S=I_r-alpha V^T M^(-1) U. If M and S are nonsingular, then
\[
(I-\alpha T)^{-1}
=M^{-1}+\alpha M^{-1}U S^{-1}V^TM^{-1}.
\tag{12}
\]
This is exact Woodbury algebra. For PTSW2C, alpha=(12/25)h_m. Factor M and form M^(-1)U and the r-by-r Schur complement once, then process each independent stage RHS. The identity applies only if the selected operator really has this exact rank-r representation; it does not assert a generic Jacobian is rank two.

## 7. Finite resonant lift: explicit example

Consider z1'=z1 and z2'=2z2+a z1^2. The monomial z1^2 is resonant because 2 lambda_1=lambda_2. Set w=z1^2. Then
\[
\frac{d}{dt}\begin{pmatrix}z_1\\z_2\\w\end{pmatrix}
=
\begin{pmatrix}1&0&0\\0&2&a\\0&0&2\end{pmatrix}
\begin{pmatrix}z_1\\z_2\\w\end{pmatrix}.
\]
The exact solution is z1=z10 e^t, w=z10^2 e^(2t), z2=e^(2t)(z20+a t z10^2), and w-z1^2=0 exactly. More generally, on a convergent Poincare-Dulac chart in a Poincare domain, resonant monomials generated by Lie differentiation have fixed weight. A positive separating functional bounds their degree, yielding a finite invariant monomial span and finite linear parent. If the parent numerical solution converges at order p and the inverse chart is Lipschitz on the projected trajectory tube, original coordinates retain order p after projection. A retraction is not required merely to retain projected convergence order. Chart convergence, lift dimension, conditioning and trajectory containment remain separate gates.

## 8. Complete no-reentry proof for the earlier three-stage toy

The fixed-step characteristic polynomial for z=-x is
\[
p_x(w)=w^3+(-1+5x/2)w^2+(-3x/2+13x^2/12)w-x^2/12.
\]
Let x_*=(12-2 sqrt(15))/7 and x**=(12+2 sqrt(15))/7. The Schur factors prove stability for 0<x<x_*. To exclude later re-entry:
- A-B+C<0 for x_*<x<x**;
- A-C<0 for x**<=x<sqrt(12);
- at x=sqrt(12), the product of root moduli is one. If all roots were in the closed unit disk, all would have modulus one; a real cubic with all roots on the unit circle must have a real root +1 or -1. But p_x(1)=x(x+1) and p_x(-1)=x(x-4), both nonzero;
- for x>sqrt(12), the product of root moduli is x^2/12>1.
Thus the exact negative-real stability interval is [-x_*,0]. This toy remains only a proof witness because its interval is narrow and its DQC condition fails.

## 9. Implementation decisions and open gates

**First stiff adapter candidate:** PTSW2C, the smallest exact variable-step common-shift method with order three and a published embedded estimator. Reuse ParallelExecution, stage_batch, Problem, LinearOperator/LinearSolverConfig, the collective backend and existing work/error ledger. No new pool or frontend.

**Stiff controls:** PTSW2A is L-stable but variable-step order two. PTSW3B is an L-stable comparator but lacks a transcribed full array. PTSW3C supplies variable-step order four but loses L-stability and has a look-ahead node.

**First nonstiff candidate:** EPPEER five-stage FSAL EPP5f3, retaining DOPRI5 as startup/reference and using the existing bounded CPU executor. Before porting, verify exact order, variable-step transform, stage-reuse semantics and error-indicator scaling.

**Not qualified:**
1. Independent full-sector verification of PTSW2C A(81.1 degree) stability; current classification is source-reported.
2. Rust startup, rejection rollback, event/discontinuity and restart semantics.
3. EPPEER transform equivalence to (9) and asymptotic proof/test of its internal estimator.
4. Full PTSW3B coefficients and estimator.
5. Matched-achieved-error CPU tests against protected RODAS5P/DOPRI5.
6. Cost and concurrency of exact rank-r factorization on source-owned operators.
7. Rust build/tests/benchmarks were not run because the connected development machine was unavailable at the prior check.

No Rust source, production default, protected RODAS5P tableau, tolerance, benchmark receipt or prior failure record was changed.

## 10. Primary sources

1. H. Podhaisky, Parallele Zweischritt-W-Methoden, PhD thesis (2002), https://doi.org/10.25673/3017.
2. H. Podhaisky, B. A. Schmitt, R. Weiner, Design, analysis and testing of some parallel two-step W-methods for stiff systems, Applied Numerical Mathematics 42 (2002), 381-395, https://doi.org/10.1016/S0168-9274(01)00162-3.
3. B. A. Schmitt, R. Weiner, Parallel Two-Step W-Methods with Peer Variables, SIAM Journal on Numerical Analysis 42 (2004), 265-282, https://doi.org/10.1137/S0036142902411057.
4. G. Yu. Kulikov, R. Weiner, Global error estimation and control in linearly-implicit parallel two-step peer W-methods, Journal of Computational and Applied Mathematics 236 (2011), 1226-1239, https://doi.org/10.1016/j.cam.2011.08.006.
5. G. Yu. Kulikov, R. Weiner, Doubly quasi-consistent parallel explicit peer methods with built-in global error estimation, Journal of Computational and Applied Mathematics 233 (2010), 2351-2364.
6. G. Yu. Kulikov, R. Weiner, Variable-Stepsize Interpolating Explicit Parallel Peer Methods with Inherent Global Error Control, SIAM Journal on Scientific Computing 32 (2010), 1695-1723, https://doi.org/10.1137/090764840.
7. R. Weiner, G. Yu. Kulikov, H. Podhaisky, Variable-stepsize doubly quasi-consistent parallel explicit peer methods with global error control, Applied Numerical Mathematics 62 (2012), 1591-1603, https://doi.org/10.1016/j.apnum.2012.06.018.
8. B. A. Schmitt, Manual for explicit parallel peer code EPPEER (2012), official page/source https://www.mathematik.uni-marburg.de/~schmitt/peer/.
