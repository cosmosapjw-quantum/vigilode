# EPPEER EPP3f4: variable-step superconvergence and error-indicator audit

**Date:** 2026-11-09  
**Source:** official EPPEER ivpepp.f90, version 20121207; Weiner, Schmitt, Podhaisky & Jebens (2009), Superconvergent explicit two-step peer methods.  
**Repository research lock:** 569ce02629498fec133db9ce368eb303ba943b69 / tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae.  
**Status:** EPP3f4 coefficient algebra exact-symbolically checked; conditional smooth-mesh order extension derived and numerically checked; no Fortran/Rust runtime qualification.  
**Execution:** CPU only. GPU is forbidden.

## 1. Why the method identity matters

The official EPPEER code constructs a variable-step coefficient transformation from PA0, PCV, and PPV. It is not safe to replace that implementation with a generic peer coefficient generator merely because both satisfy the same nominal order conditions. The exact source transformation is reconstructed below for EPP3f4.

The key distinction is:

- arbitrary bounded variable-step ratios: the certified consistency/convergence order is p=s=3;
- constant steps: the extra superconvergence condition holds, giving p=s+1=4, subject to the higher-order startup requirement;
- smoothly varying steps with sigma_n-1=O(H): the same fourth order is recovered by a quantitative extension of the published superconvergence proof;
- abrupt step changes: that extra order is not guaranteed.

Thus the source method should not be described as an unconditional variable-step fourth-order solver.

## 2. Exact source reconstruction for EPP3f4

The PPSETCOEFF(3,...) branch in ivpepp.f90 supplies
\[
c=(0,\tfrac34,1)^T,
\]
and
\[
B=\begin{pmatrix}
0&0&1\\[1mm]
-\dfrac{12163}{17521920}&\dfrac{12163}{7200}&-\dfrac{60328969}{87609600}\\[2mm]
-\dfrac1{1440}&\dfrac{169}{100}&-\dfrac{4963}{7200}
\end{pmatrix}.
\tag{2.1}
\]
The code's runtime row-sum correction leaves these rational rows unchanged.

Let
\[
V=\begin{pmatrix}
1&c_1&c_1^2\\
1&c_2&c_2^2\\
1&c_3&c_3^2
\end{pmatrix},\qquad
D=\operatorname{diag}(1,2,3),\qquad
C=\operatorname{diag}(c_1,c_2,c_3).
\]
The source builds the lower Pascal matrix
\[
P_3=\begin{pmatrix}1&1&1\\0&1&2\\0&0&1\end{pmatrix}
\]
and then row-scales it by D^{-1}. Its exact post-Vandermonde matrices are
\[
PA0=(\mathbf1 e_3^T-B)\,CV\,D^{-1}V^{-1},
\qquad
PPV=D^{-1}P_3V^{-1}.
\tag{2.2}
\]
In particular,
\[
PA0=\begin{pmatrix}
0&0&0\\[1mm]
-\dfrac{19108073}{3153945600}&\dfrac{18451271}{78848640}&
\dfrac{203620783}{1051315200}\\[2mm]
-\dfrac{1571}{259200}&\dfrac{1517}{6480}&\dfrac{16741}{86400}
\end{pmatrix},
\]
\[
PPV=\begin{pmatrix}
0&0&1\\
1/6&-8/3&5/2\\
4/9&-16/9&4/3
\end{pmatrix}.
\tag{2.3}
\]
Here CV has columns c, c^(2), c^(3).

In the source loop, h0 is the previous accepted step and sigma=h_new/h_old. pa starts as h0*PA0; each subsequent column receives the corresponding power of sigma. Therefore the exact mathematical coefficient used with the **old** step is
\[
A_{\rm code}(\sigma)
=PA0+CV\operatorname{diag}(\sigma,\sigma^2,\sigma^3)PPV,
\tag{2.4}
\]
and the source recurrence is
\[
Y_{\rm new}=BY_{\rm old}+h_{\rm old}A_{\rm code}(\sigma)F_{\rm old}.
\tag{2.5}
\]
The new peer times are t_new+c_i h_new, while the old peer times are t_new+(c_j-1)h_old. In the usual peer-method notation, the coefficient multiplying h_new is A_m=A_code(sigma)/sigma.

This convention is important: using h_new*A_code, or shifting the old nodes by c rather than c-1, changes the method.

## 3. Base order and optimal zero-stability

The source matrix satisfies the exact identities
\[
B\mathbf1=\mathbf1,\qquad
v^TB=v^T,\qquad
v^T\mathbf1=1,\qquad
B^2=\mathbf1v^T,
\tag{3.1}
\]
where
\[
v^T=(-1/1440,\ 169/100,\ -4963/7200).
\tag{3.2}
\]
Consequently B has one simple unit eigenvalue and two zero eigenvalues; all powers B^j for j>=2 equal 1 v^T. This is the optimal zero-stability structure used by the peer superconvergence theorem.

For the source recurrence (2.5), define the scaled defect moments
\[
r_q(\sigma)=
\sigma^q c^{\odot q}
-B(c-\mathbf1)^{\odot q}
-qA_{\rm code}(\sigma)(c-\mathbf1)^{\odot(q-1)},
\tag{3.3}
\]
with the final term zero at q=0. Substitution of the exact source matrices gives, in exact rational arithmetic,
\[
r_0(\sigma)=r_1(\sigma)=r_2(\sigma)=r_3(\sigma)=0
\quad\text{for every }\sigma>0.
\tag{3.4}
\]
Thus EPP3f4 is consistent of order s=3 for variable steps. The source formula order=4 is not the assertion that (3.4) holds through q=4.

## 4. The precise superconvergence gate

Weiner et al. (2009), Section 3, Theorem 1, show that an optimally zero-stable s-stage peer method of consistency order s is superconvergent of order s+1 on variable steps if, in addition to a compatible initial error O(H^(s+1)), the condition
\[
v^TAB(s+1)=0
\tag{4.1}
\]
holds for all admissible step ratios. Their AB(s+1) is the next defect moment in the peer recurrence. In the source convention (2.5),
\[
r_{s+1}(\sigma)=\sigma^{s+1}AB(s+1;\sigma),
\]
so the zero condition is equivalent to v^T r_(s+1)(sigma)=0 because sigma>0.

For EPP3f4 the next residual is not zero as a vector. Its persistent, zero-stability projection is
\[
\boxed{
g(\sigma):=v^Tr_4(\sigma)
=-\frac{(\sigma-1)
(106845\sigma^3+79585\sigma^2-10727\sigma-10727)}
{691200}.
}
\tag{4.2}
\]
Hence
\[
g(1)=0,\qquad
g'(1)=-\frac{3437}{14400}\ne0,
\qquad
g(\tfrac12)\ne0.
\tag{4.3}
\]
This gives three distinct conclusions.

**Constant steps.** When every sigma_n=1, (4.1) holds at every step. With initial peer error O(H^4), the published theorem gives global order four.

**Arbitrary adaptive ratios.** Equation (4.2) is not identically zero. The order-four theorem therefore does not apply to arbitrary bounded ratios; the general source-backed variable-step order remains three.

**Smoothly varying ratios.** Since g has a simple zero at one, it is locally Lipschitz there and
\[
|g(\sigma)|\le C|\sigma-1|.
\tag{4.4}
\]
This permits a useful quantitative extension.

## 5. Derived smooth-mesh superconvergence lemma

Let H=max_n h_n. Assume all hypotheses of the variable-step superconvergence theorem except that (4.1) is replaced by the mesh regularity condition
\[
|\sigma_n-1|\le C_\sigma H^\alpha
\quad\text{uniformly in }n,\qquad \alpha>0,
\tag{5.1}
\]
with adjacent ratios confined to a compact subset of (0,infinity), the coefficient matrices uniformly bounded, and sufficiently accurate startup. Suppose the order-s defect expansion and the source theorem's error recurrence hold uniformly. The initial peer error is assumed O(H^(s+min(alpha,1))).

The rank-one persistent part of the defect forcing is controlled by v^T r_(s+1)(sigma_n). From (5.1) and (4.4) it is O(H^alpha). The leading projected defect is therefore
\[
v^T\Delta_n=O(H^{s+\alpha})+O(H^{s+1})
=O(H^{s+\min(\alpha,1)}).
\tag{5.2}
\]
In the global error recurrence the defect enters multiplied by h_n. Summing over the mesh uses sum_n h_n=T-t_0, so the persistent projected contribution remains
\[
O(H^{s+\min(\alpha,1)}).
\]
The non-persistent components are annihilated after at most s-1 powers of B, exactly as in the published proof, and contribute O(H^(s+1)). The usual stability and discrete Grönwall estimates for the remaining error-propagation terms then give
\[
\boxed{
\max_n\|\varepsilon_n\|
=O(H^{s+\min(\alpha,1)}).
}
\tag{5.3}
\]
For EPP3f4, this predicts:
- alpha=0: generic variable-step order three;
- 0<alpha<1: intermediate order 3+alpha;
- alpha>=1: order four.

A sufficient condition for alpha=1 is a smooth mesh generated by a positive C^1 step-density function, with h_n=H eta(t_n) and eta bounded away from zero and infinity. Then sigma_n=eta(t_n)/eta(t_(n-1))=1+O(H).

This is a **derived extension of the published proof**, not a theorem stated by the authors. It relies on the same uniform expansion, boundedness, zero-stability and startup assumptions. It does not cover abrupt rejection-induced step changes unless they also satisfy (5.1).

## 6. Numerical isolation experiments

The accompanying verify_eppeer_superconvergence.py reconstructs (2.1)-(2.5) and checks the moments in exact SymPy rational arithmetic. It then runs the peer recurrence in high-precision Python with **exact initial peer values**. This deliberately removes DOPRI5 startup error and isolates the recurrence.

The test ODE is
\[
y'=y^2+\phi(t),\qquad
y_*(t)=e^t+\sin t,\qquad
\phi(t)=e^t+\cos t-(e^t+\sin t)^2,
\]
so y_* is known exactly on [0,1/4].

Observed endpoint orders:
| Mesh | Step-ratio behavior | Observed asymptotic order |
|---|---|---:|
| Constant | sigma_n=1 | 4.00 |
| Smooth geometric | sigma_n=1+1/N | 4.00 |
| Abrupt periodic ratios | 1/2, 1, 1/3 repeated | 3.00 |

Representative raw errors:
- Constant, H=1.5528e-3: 2.2440e-12.
- Smooth geometric, H=6.1728e-4: 3.7481e-14.
- Abrupt periodic ratios, H=4.2614e-4: 8.2826e-14.

These are recurrence experiments, not EPPEER Fortran, VigilODE, DOPRI5-startup or performance tests.

## 7. The PE indicator: order and consequence for adaptivity

The source sets pe=ppv(stages,:). Since the last row of the row-scaled Pascal matrix is (1/s)e_s^T,
\[
PE=\frac1s e_s^TV^{-1}.
\tag{7.1}
\]
For EPP3f4,
\[
PE=(4/9,-16/9,4/3),\qquad
PE\,c^{\odot0}=PE\,c^{\odot1}=0,\qquad
PE\,c^{\odot2}=1/3.
\]
For a smooth scalar RHS sampled at the new peer nodes,
\[
h\,PE\cdot F
=\frac{h^3}{6}f''(t)+O(h^4).
\tag{7.2}
\]
More generally, (7.1) annihilates node monomials of degree 0,...,s-2, and PE*c^(s-1)=1/s, so the raw source indicator h|PE dot F| is O(h^s).

This matches the generic variable-step order p=s, but it does **not** exploit the order-s+1 superconvergence on a smooth mesh. In the recurrence experiment, the indicator/error ratio grows as the smooth mesh is refined (for EPP3f4 it reaches about 3.2e2 at H approximately 6.17e-4); the indicator remains conservative but becomes asymptotically one order larger than the true order-four solution error. On the abrupt order-three mesh, the indicator/error ratio instead approaches a finite value near 1.8.

The Fortran source sets its private controller variable order=s+1 and uses exponent 1/(s+1) in the proposed step-size multiplier. This variable is a **controller exponent**, not the variable-step convergence order. The PE indicator has leading h^s scaling. Using 1/(s+1) is a damped, conservative response rather than the usual 1/s scaling suggested by that leading term; it does not change the fixed point EST approximately TOL, but its impact on step count and transient response must be measured. Most importantly, the indicator is not a rigorous finite-step error bound and should not be assumed to realize the extra order on smooth meshes.

## 8. EPP5f3 cross-check

The official EPP5f3 source branch has five nodes and five-stage coefficient data written as decimal literals. Reconstructing the source PPV matrix from those node literals gives a PE row that annihilates degrees 0,...,3 and has PE*c^4=1/5, consistent with the general O(h^5) indicator.

A decimal-rational reconstruction of its source B (with the same last-column row-sum correction used by the source) gives the superconvergence projection for order six approximately
\[
v^T r_6(1/2)\approx-1.0081\times10^{-2},\qquad
v^T r_6(2)\approx-26.2763.
\]
At sigma=1 it is approximately -8.4e-14, consistent with a fixed-step condition to the precision of the stored decimal table. This cross-check is **numerical with rounded source coefficients**, not an exact symbolic coefficient certificate. It supports the same method-selection warning: do not advertise variable-step order six for EPP5f3 from its fixed-step behavior.

## 9. Consequences for the VigilODE candidate

1. Keep EPP5f3 as the first high-order nonstiff candidate at variable-step order five, subject to exact startup/restart and matched-error benchmarks.
2. EPP3f4 is a useful minimal CPU-stage-parallel research target: arbitrary variable-step order three; order four under constant or sufficiently smooth mesh schedules.
3. The existing EPPEER PE indicator is conservative in the superconvergent regime and may mask the extra accepted-step benefit. An order-four estimator that is compatible with (4.1) on the smooth-mesh class remains a missing mathematical component.
4. A step-ratio regularizer imposing |sigma_n-1|<=C H is a candidate, not a default. It may preserve order four but can react too slowly to sharp error features and rejected steps. Test accepted steps and achieved error before timing.
5. Do not transfer the four-stage p=2 DQC embedded estimator to EPP3f4 or EPP5f3. The identities in the separate DQC audit are method-specific.
6. Reuse VigilODE's ParallelExecution, stage_batch, DOPRI5 starter/session, Problem API and WorkLedger. No second pool, frontend or benchmark harness.

## 10. Evidence status and next gates

**Exact-symbolically checked**
- EPP3f4 source B, PA0, PPV, and A_code(sigma);
- moments r_0,...,r_3;
- B^2=1*v^T, v^T B=v^T, and v^T 1=1;
- superconvergence projection (4.2), its simple zero at sigma=1, and derivative g'(1)=-3437/14400;
- PE vector moments.

**Numerically checked**
- constant-step and smooth geometric mesh convergence to order four;
- abrupt variable-step convergence to order three;
- conservatism of the source PE indicator in the superconvergent mesh regime.

**Literature-supported**
- variable-step peer order conditions and Theorem 1 superconvergence condition v^T AB(s+1)=0, Weiner et al. (2009), Section 3;
- official EPPEER source recurrence, startup, OpenMP stage execution and controller.

**Not established**
- DOPRI5 startup error O(H^4) for the EPP3f4 order-four mode;
- a reliable order-four adaptive estimator compatible with the smooth-mesh condition;
- error behavior at rejection, event and discontinuity restarts;
- performance or accepted-step advantage against DOPRI5;
- Rust implementation/build/test/benchmark.

## 11. Primary sources

1. B. A. Schmitt, R. Weiner, H. Podhaisky and S. Jebens, “Superconvergent explicit two-step peer methods,” Journal of Computational and Applied Mathematics 223 (2009), 753–764, DOI 10.1016/j.cam.2008.02.014, author-hosted PDF: https://www.math.tecnico.ulisboa.pt/~plima/projecto2009a/A3.pdf.
2. B. A. Schmitt, R. Weiner and S. Jebens, “Parameter optimization for explicit parallel peer two-step methods,” Applied Numerical Mathematics 59 (2009), 769–782, DOI 10.1016/j.apnum.2008.03.013.
3. B. A. Schmitt, “Manual for explicit parallel peer code EPPEER” (2012), official project and source: https://www.mathematik.uni-marburg.de/~schmitt/peer/.
4. Official IVPEPP.F90 source: https://www.mathematik.uni-marburg.de/~schmitt/peer/ivpepp.f90 and DOPRI5 starter IVPRKP.F90: https://www.mathematik.uni-marburg.de/~schmitt/peer/ivprkp.f90.
