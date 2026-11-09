# Exact source reconstruction: PTSW2B common-shift parallel W method

**Date:** 2026-11-09  
**Source lock:** research/vigilode-c5-native-graph-schur-20261102, commit 569ce02629498fec133db9ce368eb303ba943b69, tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae  
**Documentation branch:** docs/solver-consolidation-plan-20261108  
**Status:** source tableau reconstructed; exact order and stiff-limit identities independently checked; no solver implementation or runtime qualification.  
**Execution policy:** CPU only. GPU is prohibited in all tasks, dependencies, fallbacks, and benchmarks.

This note reconstructs the two-stage PTSW2B method from Helmut Podhaisky's 2002 dissertation. It is the first complete source-backed common-shift peer-W candidate in the current research package, including its variable-step coefficient transformation, startup prescription, embedded estimator, and step-size controller. The exact symbolic verification is provided by verify_ptsw2b.py.

## 1. Candidate decision

PTSW2B is a two-stage, order-two, stage-order-two, stiffly accurate, common-shift parallel two-step W method. Its current stages use only the previous-step peer derivatives. They are therefore independent and share one matrix
\[
W_m=I-h_m\gamma T_m,\qquad \gamma=\frac{3-\sqrt3}{2}.
\]
The dissertation classifies this method as L(90)-stable and reports
\[
\rho(M(\infty))=0.
\]
The exact reconstruction below independently proves that its constant-step stiff-limit amplification matrix is nilpotent of index three.

**Selection:** PTSW2B is now the first source-certified candidate for large, stiff method-of-lines systems where a Krylov solve is used. The dissertation reports that its nilpotent stiff-limit matrix gives substantially cheaper Krylov convergence than the PTSW2A family on its large MOL tests. PTSW2C remains the order-three variable-step comparator, while PTSW2A remains a direct-factorization control. PTSW2B has only order two, so it may require more accepted steps than protected RODAS5P or PTSW2C. Its lower order is a material risk for the original low-step-count objective, not something to hide behind a stage speedup.

## 2. Source construction and exact coefficients

The source is Podhaisky (2002), Chapter 2, equations (2.13)–(2.15), together with Chapter 3, Section 3.2 and Theorem 3.1. For q=p=s=2, define
\[
V_0=\begin{pmatrix}1&c_1\\1&c_2\end{pmatrix},\quad
P_2=\begin{pmatrix}1&1\\0&1\end{pmatrix},\quad
D=\operatorname{diag}(1,2),\quad
S(\sigma)=\operatorname{diag}(1,\sigma),\quad
C=\operatorname{diag}(c_1,c_2).
\]
The source order construction is
\[
A(\sigma)=CV_0S(\sigma)D^{-1}P_2V_0^{-1},
\]
\[
\Gamma(\sigma)=-\gamma V_0S(\sigma)P_2V_0^{-1},
\]
and stiff accuracy fixes
\[
b^T=\gamma e_2^T,\qquad v^T(\sigma)=e_2^T\beta(\sigma),\qquad
\beta=A+\Gamma.
\]

Theorem 3.1 reduces nilpotency of M(infinity) to roots of
\[
\pi(\gamma)=-\gamma^2+3\gamma-\frac32=0,
\]
\[
\phi^*(c,\gamma)=c^2+(2-4\gamma)c-3+4\gamma=0.
\]
The two gamma roots are \((3\mp\sqrt3)/2\). The larger root gives an impractically large first node. PTSW2B is the smaller-root choice:
\[
\boxed{\gamma=\frac{3-\sqrt3}{2}},\qquad
\boxed{c_1=3-2\sqrt3,\quad c_2=1}.
\]
Numerically, c1 is approximately -0.4641016151. The first stage is an extrapolatory node; the source's startup procedure handles it by integrating forward from t0, not backward.

Substitution into the source's Vandermonde formula gives the exact step-ratio matrices:
\[
A(\sigma)=\frac18
\begin{pmatrix}
3(5-3\sqrt3)\sigma&
24-16\sqrt3+3(-5+3\sqrt3)\sigma\\
-(1+\sqrt3)\sigma&
8+(1+\sqrt3)\sigma
\end{pmatrix},
\tag{2.1}
\]
\[
\Gamma(\sigma)=\frac14
\begin{pmatrix}
3(\sqrt3-2)\sigma&
-6+2\sqrt3+6\sigma-3\sqrt3\sigma\\
\sqrt3\sigma&
-6+2\sqrt3-\sqrt3\sigma
\end{pmatrix},
\tag{2.2}
\]
\[
b^T=\left(0,\frac{3-\sqrt3}{2}\right),
\qquad
v^T(\sigma)=\frac{\sqrt3-1}{8}\,(\sigma,\ 4-\sigma).
\tag{2.3}
\]
Every coefficient is exact algebraic data. No rounded coefficient in the source table is needed to reconstruct this method.

The two-step W equations are
\[
Y_{m,i}=u_m+h_m\sum_{j=1}^2a_{ij}(\sigma_m)k_{m-1,j},
\]
\[
(I-h_m\gamma T_m)k_{m,i}
=f(t_m+c_i h_m,Y_{m,i})
+h_mT_m\sum_{j=1}^2\Gamma_{ij}(\sigma_m)k_{m-1,j},
\quad i=1,2,
\]
\[
u_{m+1}=u_m+h_m\sum_{i=1}^2
\left(b_i k_{m,i}+v_i(\sigma_m)k_{m-1,i}\right),
\qquad \sigma_m=\frac{h_m}{h_{m-1}}.
\tag{2.4}
\]
There is no same-step stage coupling. Both stage solves use exactly the same shifted matrix Wm. This is actual method parallelism, not a concurrent implementation of dependent RODAS5P stages.

## 3. Exact order and stiff-limit proofs

### 3.1 Variable-step order

For the convention in the source, the simplifying conditions are
\[
C(q):\quad
\sum_j a_{ij}(c_j-1)^{\ell-1}
=\frac{\sigma^{\ell-1}c_i^\ell}{\ell},
\]
\[
\Gamma(q):\quad
\sum_j\Gamma_{ij}(c_j-1)^{\ell-1}
=-\gamma\sigma^{\ell-1}c_i^{\ell-1},
\]
\[
B(p):\quad
\sum_j b_j\sigma^{\ell-1}c_j^{\ell-1}
+\sum_j v_j(c_j-1)^{\ell-1}
=\frac{\sigma^{\ell-1}}{\ell}.
\]
Using (2.1)–(2.3), exact SymPy simplification gives zero residual for C(2), Gamma(2), and B(2), identically in the symbolic variable sigma. The PTSW convergence theorem therefore gives p*=min(q+1,p)=2, provided:
- the coefficient matrices are uniformly bounded on the admissible step-ratio family;
- the initial solution error is O(H^2);
- the initial peer-stage derivative errors are O(H^2);
- the usual smoothness and Lipschitz assumptions hold on a solution tube.

Because A(sigma), Gamma(sigma), and v(sigma) are affine in sigma, they are bounded on every bounded ratio interval. The source step controller clips the proposed ratio to [0.5,2] and includes hysteresis to avoid consecutive growth. This supplies a practical bounded-ratio policy; it is not a proof of stability under arbitrary unbounded step changes.

### 3.2 Constant-step stiff limit

For constant step size, let z=h lambda and w=z/(1-gamma z). The source amplification matrix is
\[
M(z)=
\begin{pmatrix}
w\beta & w\mathbf1\\
b^T w\beta+v^T & 1+b^T w\mathbf1
\end{pmatrix}.
\]
At Re(z) tending to negative infinity, w tends to -1/gamma. Substituting sigma=1 in (2.1)–(2.3) gives
\[
M(\infty)=
\begin{pmatrix}
\sqrt3/4&3\sqrt3/4&-1-\sqrt3/3\\
-\sqrt3/12&-\sqrt3/4&-1-\sqrt3/3\\
0&0&0
\end{pmatrix}.
\tag{3.1}
\]
Exact arithmetic gives
\[
\det(\lambda I-M(\infty))=\lambda^3,
\]
\[
M(\infty)^2=
\begin{pmatrix}
0&0&-1-\sqrt3\\
0&0&(1+\sqrt3)/3\\
0&0&0
\end{pmatrix}\ne0,
\qquad
M(\infty)^3=0.
\]
Thus M(infinity) is nilpotent of index three and its spectral radius is exactly zero. This independently verifies the source's rho(M(infinity))=0 statement. The dissertation also classifies PTSW2B as L(90)-stable; this note does not replace the source's full complex-sector stability proof with a new interval proof.

### 3.3 Leading error coefficient

The source gives
\[
C_3=2\gamma^2-\frac72\gamma+\frac76
=\frac{23}{12}-\frac{5\sqrt3}{4}
\approx-0.24839684279443.
\]
Hence |C3| is approximately 0.2483968428, agreeing with the rounded table entry 0.25.

## 4. Published startup procedure specialized to PTSW2B

A two-step peer method requires the starting solution value u1 and both old peer derivatives k0,i. The dissertation gives a forward-only startup:
1. Form the relative node set T={c1-1,c2-1,0} and remove duplicates.
2. Sort it as T1<...<Tq and set t1=t0-h0*T1.
3. Integrate the original IVP forward from t0 to every time t0+h0(Tj-T1).
4. At the target corresponding to zero, set u1 to the computed solution.
5. At a target Tj=ci-1, set k0,i to the computed RHS value.
6. Set h1=h0 for the first PTSW step.

For PTSW2B,
\[
T=\{2-2\sqrt3,0\},\qquad
t_1=t_0+(2\sqrt3-2)h_0.
\]
The required derivative times are
\[
t_1+(c_1-1)h_0=t_0,\qquad
t_1+(c_2-1)h_0=t_1.
\]
Thus k0,1=f(t0,u0), k0,2=f(t1,u1), and the startup only needs one forward integration from t0 to t1. The source uses RADAU for direct-factorization tests and ROWMAP for Krylov tests, but the starter is not mathematically tied to either. Existing DOPRI5 can serve as the forward starter if its dense output or substep sequence supplies u1 to the required O(H^2) error; this must be tested in the adapter. A low-quality initialization k0,i=u0' and u1=u0 is explicitly discouraged by the source because it introduces inaccurate starts and numerous step increases.

## 5. Published embedded estimator and step controller

The source uses an embedded formula of order s-1=1. Its generic construction specializes to
\[
b_e^T=0.95\,b^T
=\left(0,\frac{19}{20}\gamma\right),
\]
\[
v_e^T(\sigma)=
\left(
\frac{(8\sqrt3-11)\sigma}{80},\
\frac{(11-8\sqrt3)\sigma}{80}
+\frac{19\sqrt3-17}{40}
\right).
\tag{5.1}
\]
The expression for v_e follows exactly from the source's equation (4.7), with epsilon=0.1 and the PTSW2B V0, V1, D, S matrices. It was independently checked symbolically.

The embedded endpoint is
\[
\widetilde u_{m+1}
=u_m+h_m\left(b_e^T K_m+v_e^T(\sigma_m)K_{m-1}\right).
\]
The source error indicator is a componentwise scaled RMS:
\[
Err_{\rm emb}=
\sqrt{\frac1n\sum_{i=1}^n
\left(
\frac{u_{m+1,i}-\widetilde u_{m+1,i}}
{ATOL+RTOL\,|u_{m+1,i}|}
\right)^2}.
\]
The proposed ratio is
\[
\sigma_*=\min\left\{2,\max\left\{0.5,\ 0.85\,Err_{\rm emb}^{-1/s}\right\}\right\},
\qquad s=2.
\]
The source hysteresis keeps h*=hm when sigma*>1.05 and any of the last s ratios was greater than one; it also keeps hm when sigma* lies in [0.95,1.05]. Otherwise h*=hm sigma*. Accept the step if Err_emb<=1, commit (u_(m+1),K_m,h_m), and set the next ratio to h*/hm. If rejected, repeat from the last committed history with the smaller h*. The ratio is not a generic error estimate: it is a method-specific embedded indicator from the original PTSW implementation.

The source derivation and algorithm are literature-supported. The exact PTSW2B specialization (5.1) was independently symbolically checked. A Rust controller adapter and rejection/restart transaction are not implemented.

## 6. Workload-specific comparison and choice

The dissertation's numerical experiments make an important distinction:
- For direct LU on small/moderate systems, PTSW2A is often better than PTSW2B; matrix factorization can dominate and the higher stiff-limit damping of B does not automatically win.
- For large MOL systems with Krylov approximation, PTSW2B and PTSW3B have rho(M(infinity))=0 and the source reports substantially lower Krylov residual work than PTSW2A/3A.
- On the NILIDI 10,000-variable MOL example, the source reports maximum speedups 1.7, 2.5, and 3.4 on two, three, and four processors for its selected nilpotent PTSW parallel runs. For DIFFU2, reported speedups are about 1.5, 2.0, and 2.5 for 2, 3, and 4 stages. In DIFFU2, PTSW3B is the best after parallelization; PTSW2B is strongest at coarse tolerance. These are historical measurements on the source's HP/Convex system and are not modern CPU performance predictions.
- The source notes that startup is sequential and Krylov dimensions differ between stages. It also shows that if the per-stage Krylov dimension is small, a sequential RHS/startup cost can reduce net speedup.

Consequently the candidate portfolio should be workload-specific:
- PTSW2B: first stiff, large-MOL/Krylov proof-of-concept due exact tableau, L-stability and nilpotent stiff limit. Risk: only order two and two stages.
- PTSW2C: order-three variable-step comparison; A(81.1°)-stable but not L-stable.
- PTSW2A: direct-LU comparison, source reports better behavior on small problems.
- PTSW3B: performance target for larger CPU concurrency and Krylov, but still requires complete exact coefficients, embedded estimator, and startup transcription before implementation.

Do not promote PTSW2B on its nilpotent matrix alone. It must meet the preregistered accepted-step and matched-achieved-error gates against protected RODAS5P. The existing project target remains accepted steps <=1.10x baseline and median full-IVP CPU speedup >=1.15x on the preregistered subset, with at least seven randomized paired runs. Startup, embedded estimator, factorization/preconditioner setup, rejections, and step-ratio changes are charged.

## 7. Exact verification performed

The companion script verify_ptsw2b.py uses exact SymPy algebra to check:
- C(2), Gamma(2), B(2) residuals vanish as rational/algebraic identities for symbolic sigma;
- b and v enforce stiff accuracy;
- the main error coefficient equals (23-15 sqrt(3))/12;
- the embedded b_e and v_e formulas match the source construction;
- det(lambda I-M(infinity))=lambda^3;
- M(infinity)^3=0 and M(infinity)^2 is nonzero.

**Verification result:** PASS in the current Python/SymPy analysis environment. This is symbolic coefficient verification, not a Rust build, solver run, full sector-stability proof, or benchmark.

## 8. Reuse-first implementation map and remaining gates

Reuse existing:
- ParallelExecution and bounded CPU executor;
- stage_batch instrumentation and contracts;
- Problem and existing LinearOperator/LinearSolverConfig/direct/Krylov/preconditioner services;
- existing collective low-rank backend only when its source-owned operator identity exactly matches T_m;
- existing session, step controller, work ledger, and error accounting.

The new method core should be limited to the source-backed PTSW2B tableau adapter, two-step peer history/startup, and its embedded formula. No new frontend, executor, Krylov implementation, benchmark framework, homotopy, or WorkLedger.

Remaining gates:
1. Replay pinned Rust source identities and existing tests when the local development machine is reachable.
2. Implement PTSW2B as a distinct opt-in method plugin without changing protected RODAS5P.
3. Test startup order, accepted-step history commit, rejection rollback, ratio-dependent coefficient recomputation, and event/discontinuity restart.
4. Test direct and matrix-free stage solves; verify the same W matrix is actually reused for both stage right-hand sides.
5. Validate the algebraic residual of every stage solve and combine it with, but do not confuse it with, the truncation-error estimate.
6. Compare PTSW2B, PTSW2C, PTSW2A and PTSW3B at matched achieved error; record accepted steps, RHS calls, Jacobian/operator refreshes, factorizations, Krylov matvecs/iterations, stage overlap and CPU wall time.
7. Run at least seven randomized paired CPU full-IVP timings; GPU remains disabled.

**Implementation status:** not implemented. The connected development machine was offline during this research turn; no Rust build/test/benchmark was run and no local worktree was modified.

## 9. Primary references

1. H. Podhaisky, *Parallele Zweischritt-W-Methoden*, PhD thesis, Martin-Luther-Universität Halle-Wittenberg (2002), especially Chapter 2 equations (2.13)–(2.16), Chapter 3 Theorem 3.1 and Section 3.2, Chapter 4 Sections 4.3–4.5, and Chapter 5; https://doi.org/10.25673/3017. The full institutional dissertation is archived at https://webdoc.sub.gwdg.de/ebook/e/2002/pub/mathe/02H015/prom.pdf.
2. H. Podhaisky, B. A. Schmitt, R. Weiner, “Design, analysis and testing of some parallel two-step W-methods for stiff systems,” *Applied Numerical Mathematics* 42 (2002), 381–395, https://doi.org/10.1016/S0168-9274(01)00162-3.
3. B. A. Schmitt, R. Weiner, “Parallel Two-Step W-Methods with Peer Variables,” *SIAM Journal on Numerical Analysis* 42 (2004), 265–282, https://doi.org/10.1137/S0036142902411057.
