# Explicit parallel peer: EPPEER source audit and variable-step DQC certificate

**Date:** 2026-11-09  
**Source lock:** VigilODE research commit 569ce02629498fec133db9ce368eb303ba943b69, tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae  
**Status:** published four-stage DQC family reconstructed and exact-symbolically verified; EPPEER source inspected; no adapter or runtime qualification.  
**Execution policy:** CPU only. GPU is forbidden in every task and fallback.

This note separates two explicit-peer method identities:

- **EPPEER high-order family:** stage-parallel, variable-step explicit peers of formal order p=s, with DOPRI5 startup, a source-owned error indicator, and FSAL reuse for odd stage counts. This is the first high-order nonstiff candidate.
- **Variable-step DQC EPP family:** a published four-stage, order-two method with a proved asymptotic global-error estimator and order-three/order-four embedded peer formulas. It is an error-control reference and contract test, not a low-step-count production candidate.

The DQC estimator is not transferable to arbitrary explicit peer tableaus. In particular, it must not be attached to EPPEER EPP5f3 unless that exact tableau is independently shown to satisfy the DQC hypotheses.

## 1. Official EPPEER source audit

Official material:

- Project page: https://www.mathematik.uni-marburg.de/~schmitt/peer/
- Main module: https://www.mathematik.uni-marburg.de/~schmitt/peer/ivpepp.f90
- Runge–Kutta starter module: https://www.mathematik.uni-marburg.de/~schmitt/peer/ivprkp.f90
- Manual: https://www.mathematik.uni-marburg.de/~schmitt/peer/man_epp.pdf

The manual describes explicit two-step peer methods of orders 3 through 9. Each new peer stage is computed from the previous-step peer vector and previous-step RHS values. The method therefore exposes stage-level CPU concurrency without parallelizing the user's RHS internally.

Inspection of the actual Fortran source establishes:

1. PPSETCOEFF selects a method with mnr stages. The formal method order is p=mnr.
2. The private source variable named order is set to mnr+1. It is the local error-estimator exponent, not the formal integration order. Do not confuse the variable name with p.
3. For mnr=3,5,7,9, pfsal is true and one old endpoint RHS value is reused. The number of newly evaluated independent RHS stages is s-1.
4. For mnr=4,6,8, all s RHS stages are new.
5. EPP5f3 is a five-stage, fifth-order FSAL method. It has four fresh RHS evaluations per step and four independent stage tasks. EPP7f4 and EPP9f2 are higher-stage alternatives; their utility must be established at matched achieved error.
6. Startup is source-defined. The code selects DOPRI5 (id=5) from ivprkp.f90, computes the first endpoint, then constructs peer stage values at the selected nodes. This is more specific than a generic instruction to use a fifth-order starter.
7. EPPEER counts function evaluations including startup, rejected steps and accepted steps.
8. The Fortran RHS callback is declared PURE. A Rust callback that is Send+Sync is not automatically semantically reentrant if it uses interior mutability or unsynchronized side effects.

The source's error indicator is also method-specific. It forms a weighted combination of newly computed peer-stage RHS values using its PE vector, normalizes componentwise by atol + rtol*abs(y), takes the maximum, and multiplies by the step size. The next-step update uses exponent 1/(p+1), a 0.95 safety factor, a lower ratio bound of 0.2, and a method-specific upper ratio cap sigx. This is source-inspected controller behavior, not a proof that the estimate is an asymptotically exact global-error estimator for every ODE.

The implementation supports dense output with full method order; its output is continuous for the FSAL odd-stage methods.

### 1.1 Interpretation

The official examples show that method-level parallelism is useful when each RHS evaluation is expensive. They do not establish universal speedup: cheap RHS problems can fail to amortize synchronization, and some reported accuracy curves deviate materially from the requested tolerance.

Decision:

- use EPP5f3 as the first source-backed high-order parallel peer candidate;
- retain DOPRI5 as baseline and startup/reference;
- compare EPP7f4 and EPP9f2 only when RHS work can amortize extra stages;
- count startup calls, rejected-step work, accepted steps, wall time, achieved error and observed concurrent-stage count;
- do not infer a global-error guarantee from the fact that the source has an adaptive local indicator.

### 1.2 Reuse boundary in VigilODE

Do not port EPPEER as a second solver frontend or add a second thread pool.

| EPPEER responsibility | Existing VigilODE component |
|---|---|
| Bounded CPU stage execution | ParallelExecution |
| Ordered independent stage work and concurrency probe | stage_batch |
| IVP callback and state ownership | existing Problem API |
| Startup/reference method | existing DOPRI5 implementation and tests |
| Rejection, lifecycle and work accounting | existing session and WorkLedger contracts |
| Error/performance reporting | existing matched-achieved-error benchmark protocol |

The source's coefficient transformation uses PA0, PCV, PPV and powers of the step ratio. A separately derived generic peer coefficient generator can be used as an independent oracle, but equality with EPPEER's coefficient family must be tested before substituting one for the other.

## 2. Published variable-step DQC EPP method

Primary source: R. Weiner, G. Yu. Kulikov and H. Podhaisky, Variable-stepsize doubly quasi-consistent parallel explicit peer methods with global error control, Halle Report No. 05 (2010); corresponding 2012 journal article, Applied Numerical Mathematics 62, 1591–1603, DOI 10.1016/j.apnum.2012.06.018.

The method is
\[
X_k=(B\otimes I)X_{k-1}
+\tau_k(A(\theta_k)\otimes I)F_{k-1},
\qquad
\theta_k=\frac{\tau_k}{\tau_{k-1}},
\tag{2.1}
\]
with no current-step explicit peer coupling. All stages are independent conditional on the previous-step peer vector and RHS.

The nodes and fixed history matrix are
\[
c=(0,\tfrac14,\tfrac12,1)^T,\qquad
B=\mathbf1 b^T,\qquad
b^T=(\tfrac16,\tfrac12,\tfrac16,\tfrac16).
\tag{2.2}
\]
Consequently,
\[
B^2=B,\quad \operatorname{rank}B=1,\quad
\operatorname{spec}(B)=\{1,0,0,0\}.
\tag{2.3}
\]
Thus B is power-bounded and its unit eigenvalue is simple.

The published variable-step coefficient matrix is
\[
A(\theta)=\begin{pmatrix}
\dfrac{1-24\theta+12\theta^2}{96\theta}&
\dfrac{5}{16\theta}&
\dfrac12&
\dfrac{29-24\theta-12\theta^2}{96\theta}\\[2mm]
\dfrac{-39+37\theta+62\theta^2+50\theta^3}{192\theta}&
\dfrac{2\theta+5}{16\theta}&
\dfrac{41-55\theta-92\theta^2-50\theta^3}{96\theta}&
\dfrac{17+97\theta+122\theta^2+50\theta^3}{192\theta}\\[2mm]
\dfrac{1-30\theta}{96\theta}&
\dfrac{4\theta+5}{16\theta}&
\dfrac14&
\dfrac{29+30\theta}{96\theta}\\[2mm]
\dfrac{1-42\theta-36\theta^2}{96\theta}&
\dfrac{8\theta+5}{16\theta}&
\dfrac18&
\dfrac{29+78\theta+36\theta^2}{96\theta}
\end{pmatrix}.
\tag{2.4}
\]
The numerator signs have been checked against the source and the exact moment identities.

## 3. Exact DQC identities

Let
\[
d(\theta)=\frac{c-\mathbf1}{\theta}.
\]
For every integer ell >= 0 define
\[
\delta_\ell(\theta)=
c^{\odot\ell}
-Bd(\theta)^{\odot\ell}
-\ell A(\theta)d(\theta)^{\odot(\ell-1)},
\tag{3.1}
\]
where the last term is zero at ell=0 and powers are componentwise.

For the tableau (2.2)–(2.4), exact rational-function simplification gives
\[
\delta_0(\theta)=0,\qquad \delta_1(\theta)=0,
\tag{3.2}
\]
and
\[
\delta_2(\theta)=\frac14(1,-1,1,1)^T.
\tag{3.3}
\]
The DQC identities are
\[
\boxed{
B\delta_2=0,\qquad
B\delta_3=0,\qquad
A(\theta)\delta_2=0
}
\quad\text{for all }\theta>0.
\tag{3.4}
\]
These are exact rational identities, not floating-point observations.

For a variable-step EPP method of order p, the DQC conditions are
\[
\delta_\ell=0\quad(0\le\ell<p),\quad
B\delta_p=0,\quad B\delta_{p+1}=0,\quad
A(\theta)\delta_p=0
\tag{3.5}
\]
for every positive step ratio. Here p=2, and (3.2)–(3.4) verify (3.5).

### 3.1 Admissible meshes are part of the theorem

The result is not claimed for arbitrary step sequences. With \(\tau=\max_k\tau_k\), require
\[
0<\omega_1\le\theta_k\le\omega_2<\infty,\qquad
\omega_1\le1\le\omega_2,
\tag{3.6}
\]
and
\[
\frac{\tau}{\tau_k}\le\Omega<\infty.
\tag{3.7}
\]
The first condition bounds adjacent step ratios. The second prevents the minimum step becoming arbitrarily small relative to the maximum. It also bounds A(theta) on the admissible mesh family.

The base method's starting peer vector must have error O(tau_0^3). Under the source's smoothness, zero-stability, boundedness and admissible-grid hypotheses, the theorem establishes equality of the principal terms of the true global error and the local defect:
\[
\Delta X_k=L(T_k,x(t),\tau_k)+O(\tau_k^3).
\tag{3.8}
\]
Here L is the method defect, which agrees with the local error under the source convention.

This is an asymptotic identity. It is not a finite-step rigorous upper bound, and does not imply every chosen tolerance will be met without remainder effects.

### 3.2 Proof structure

The source derives
\[
\Delta X_k=
\left[B\otimes I+
\tau_k A(\theta_k)\otimes g_x(t_k,x(t_k))\right]\Delta X_{k-1}
+L_k+O(\tau_k^4).
\tag{3.9}
\]
The defect expansion is
\[
L_k=
\frac{\delta_2(\theta_k)}{2!}\tau_k^2x''(t_k)
+\frac{\delta_3(\theta_k)}{3!}\tau_k^3x'''(t_k)
+O(\tau_k^4).
\tag{3.10}
\]
Repeated substitution propagates the leading local-defect terms through the matrix product in (3.9). The three DQC identities in (3.4) cancel the terms that would otherwise change the leading global-error coefficient. The remaining terms are O(tau_k^3) on the admissible mesh family. This yields (3.8).

The cancellation is specific to this method. Stage independence and ordinary peer order do not imply DQC.

## 4. Embedded estimator and adaptive control

The report constructs embedded peers with the same nodes and fixed history matrix:
\[
B_{\rm emb}=B,\qquad c_{\rm emb}=c.
\tag{4.1}
\]
Let
\[
C=\operatorname{diag}(c),\quad D=\operatorname{diag}(1,2,3,4),
\]
\[
S(\theta)=\operatorname{diag}(1,\theta,\theta^2,\theta^3),
\]
\[
(V_0)_{ij}=c_i^{j-1},\qquad
(V_1)_{ij}=(c_i-1)^{j-1},\qquad e_4=(0,0,0,1)^T.
\]
For beta in R^4, the embedded coefficient matrix is
\[
A_{\rm emb}(\theta;\beta)=
\left(CV_0D^{-1}-\frac14\beta e_4^T\right)
S(\theta)V_1^{-1}
-\frac1\theta B(C-I_4)V_1D^{-1}V_1^{-1}.
\tag{4.2}
\]
The source's two choices are:

- beta=(1/40,1/40,1/40,1/40)^T: embedded order three;
- beta=0: embedded order four.

For the base p=2 method, the embedded-pair theorem requires
\[
\delta_\ell^{\rm emb}=0\quad(0\le\ell\le2),\qquad
B_{\rm emb}\delta_2=0.
\tag{4.3}
\]
Exact symbolic checks give, for the order-three embedded formula,
\[
\delta_0^{\rm emb}=\delta_1^{\rm emb}=\delta_2^{\rm emb}=0,\qquad
\delta_3^{\rm emb}=\beta.
\]
For the order-four embedded formula, all moments through ell=4 vanish. Since B_emb=B and B delta_2=0, the embedded-pair condition holds.

The leading global-error estimate is
\[
\Delta_1X_k=X_{k,\rm emb}-X_k
=(B_{\rm emb}-B)X_{k-1}
+\tau_k(A_{\rm emb}-A)F_{k-1}.
\tag{4.4}
\]
The first term vanishes because B_emb=B. No Jacobian evaluation is required.

The published numerical controller uses
\[
EST=\|X_{k,\rm emb}-X_k\|_\infty,
\]
\[
\tau_{\rm new}
=\min(1.5,\max(0.5,0.9\sqrt{TOL/EST}))\,\tau_k.
\tag{4.5}
\]
The step is accepted when EST <= TOL; otherwise it is retried at the reduced step. The report compares the p=2 DQC method with order-three and order-four embedded continuations and MATLAB ode23.

Interpretation: (4.4) estimates the principal term of the true global error asymptotically. Equation (4.5) is a published practical controller. Neither makes the computed error a rigorous finite-step enclosure. The higher derivatives of the exact solution and interval length affect the remainder. The report also records a test reaching its three-million-step cap at the strictest tolerance. This method is therefore an error-control reference, not the first low-step-count production candidate.

## 5. Why the earlier three-stage toy cannot use this estimator

The three-stage interpolation witness has a different fixed peer matrix P. At its third-order level, exact arithmetic gives
\[
\delta_4=(0,9/16,4)^T,\qquad P\delta_4=(4,4,4)^T\ne0.
\]
It is not DQC. Its third-order convergence and narrow negative-real stability interval remain valid, but attaching (4.4) or the DQC global-error theorem to it is invalid.

Likewise EPPEER EPP5f3 must not be assigned the DQC estimator merely because it is an explicit parallel peer method. EPPEER's coefficient arrays, variable-step transformation and PE error-indicator are a separate method identity. First verify the exact tableau's DQC conditions, embedded-pair conditions and startup requirements.

## 6. Experiment plan

### 6.1 First high-order nonstiff candidate: EPP5f3

- Five peer stages, formal order five, FSAL.
- Four fresh independent RHS evaluations per step.
- Reuse the official DOPRI5 starter contract where it matches the existing VigilODE DOPRI5 implementation.
- Reuse ParallelExecution, stage_batch, Problem, step session and WorkLedger.
- Port the source variable-step coefficient transformation exactly, or independently show equivalence with the generic generator.
- Retain EPPEER's own error indicator initially, with an independent achieved-error audit.

Compare with protected DOPRI5 on nonstiff IVPs. Match achieved error, not requested tolerance. Include startup calls, rejected-step work, stage synchronization and controller overhead.

### 6.2 DQC reference candidate

The four-stage p=2 DQC tableau is a contract test for:

- variable-step ratio guards and coefficient generation;
- exact DQC identities;
- embedded-pair conditions;
- estimated versus actual global error on known-solution IVPs;
- startup order and finite-tolerance remainder behavior.

It performs four fresh RHS evaluations per step. Do not present it as a speed competitor to EPP5f3 without matched-error evidence.

### 6.3 Required tests

1. Exact rational checker for B projector, moments and embedded conditions.
2. Constant ratio, alternating ratio extremes, ratio-bound violation, rejection/retry without corrupting previous peer state.
3. Startup peer error O(tau_0^3) for the DQC method.
4. Estimate versus actual global error on exact-solution IVPs, including long intervals and high derivatives.
5. Callback reentrancy and CPU stage concurrency.
6. At least seven randomized paired CPU runs at matched achieved error. GPU remains disabled.

## 7. Reproducible exact checker

The accompanying verify_dqc_peer.py uses SymPy exact rational-function arithmetic with positive symbolic theta. Its run returned:

~~~text
PASS: B is a rank-one projector (eigenvalues 1,0,0,0).
PASS: base EPP order-2 moments AB(0)=AB(1)=0.
PASS: AB(2)= [1/4, -1/4, 1/4, 1/4]
PASS: DQC identities B AB(2)=B AB(3)=A(theta) AB(2)=0 exactly.
PASS: order-3 embedded moments AB_emb(0..2)=0; AB_emb(3)=beta.
PASS: order-4 embedded moments AB_emb(0..4)=0.
PASS: embedded-pair condition B_emb AB_base(2)=0.
~~~

This verifies tableau identities only. It does not test a Rust implementation, floating-point error-control behavior, startup, adaptive restarts or performance.

## 8. Evidence and remaining gates

**Derived / exact-symbolically verified**
- B^2=B, rank-one zero-stability structure;
- base peer moments and DQC identities for all symbolic positive theta;
- order-three and order-four embedded moment conditions;
- embedded-pair consistency condition.

**Literature-supported**
- variable-step order-two DQC global-error asymptotics under admissible meshes;
- embedded estimator construction without Jacobian evaluation;
- EPPEER formal order, stage independence, FSAL structure, DOPRI5 startup and native controller.

**Implementation-inspected**
- EPPEER ivpepp.f90 and ivprkp.f90 coefficient and controller logic;
- Fortran PURE RHS callback contract;
- no VigilODE adapter has been written.

**Unresolved**
- Exact EPPEER coefficient identity with the generic variable-step generator;
- DOPRI5 starter port and peer initialization order for all candidate nodes;
- event/discontinuity/restart semantics;
- asymptotic accuracy of the EPPEER PE indicator on VigilODE target systems;
- CPU speedup and accepted-step performance;
- Rust builds/tests/benchmarks while the connected development machine is offline.

## 9. Primary sources

1. R. Weiner, G. Yu. Kulikov and H. Podhaisky, Variable-stepsize doubly quasi-consistent parallel explicit peer methods with global error control, Halle Report No. 05 (2010), https://www2.mathematik.uni-halle.de/reports/sources/2010/10-05report.pdf.
2. R. Weiner, G. Yu. Kulikov and H. Podhaisky, Applied Numerical Mathematics 62 (2012), 1591–1603, DOI https://doi.org/10.1016/j.apnum.2012.06.018.
3. B. A. Schmitt, Manual for explicit parallel peer code EPPEER (2012), https://www.mathematik.uni-marburg.de/~schmitt/peer/man_epp.pdf.
4. Official EPPEER source, ivpepp.f90, https://www.mathematik.uni-marburg.de/~schmitt/peer/ivpepp.f90.
5. Official DOPRI5 starter, ivprkp.f90, https://www.mathematik.uni-marburg.de/~schmitt/peer/ivprkp.f90.
