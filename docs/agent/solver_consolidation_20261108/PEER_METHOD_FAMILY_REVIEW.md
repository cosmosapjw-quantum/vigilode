# Parallel Rosenbrock-family candidates: method-family review

**Scope:** evidence-led selection of stage-parallel linearly implicit two-step methods for VigilODE. This is a mathematical design review, not a claim that any new method has been implemented or qualified. GPU is forbidden. The source-code reuse obligations are in `REUSE_FIRST_INVENTORY.md`.

## 1. Do not collapse all "peer" methods into one family

The literature uses related names for several mathematically distinct classes. Their order, stage order, linearity, stage concurrency and shift structure differ.

| Family | Current-stage dependency | Linear/nonlinear solve | Shift structure | Intended role |
|---|---|---|---|---|
| Parallel two-step W-method | Independent if same-step lower-triangular couplings are set to zero | Linearly implicit | A common (I-h\gamma T_m) can be shared by all stages | First candidate: reuse one factorization for (s) independent RHS |
| Parallel peer / multi-implicit peer two-step W | Previous-step peer data only | Linearly implicit | Stage-dependent implicit parameters; distinct shifted systems may occur | First/second candidate: independent CPU Krylov or direct solves; better stage-specific stiff damping |
| Implicit parallel peer method | All stages can be computed independently from previous-step peer data | Nonlinear stage equations | Separate nonlinear equations; each may need Newton/Krylov | Secondary candidate only if linear W-method stability is insufficient |
| Rosenbrock-type peer two-step method | Depends on the actual lower-triangular same-step matrix; not parallel by name | Linearly implicit peer stages | Can have same-step triangular coupling | Sequential high-stage-order/stiff-stability control unless a selected table proves diagonal/independent coupling |
| Stiffly accurate two-step W with extra variable-step superconvergence constraint | Cited construction loses stage parallelism | Linearly implicit | Stronger order constraint changes coupling | Sequential Pareto control for accuracy versus concurrency |

The key design fork is not simply "Runge-Kutta versus peer". It is whether every current-step stage is a function only of data already committed at the previous step. Stage order alone does not grant parallelism.

## 2. Parallel two-step W-method: common-shift candidate

The linearly implicit two-step W-method described in Klinge's 2020 dissertation (Chapter 2, equation 2.1.2) has the following form. Let (u_m\approx y(t_m)), (k_{m,i}\approx y'(t_m+c_i h_m)), (t_{m,i}=t_m+c_i h_m), and (\sigma_m=h_m/h_{m-1}):

\[
Y_{m,i}=u_m
+h_m\sum_{j=1}^{s}a_{ij}k_{m-1,j}
+h_m\sum_{j=1}^{i-1}\widetilde a_{ij}k_{m,j},
\]

\[
(I-h_m\gamma T_m)k_{m,i}
=f(t_{m,i},Y_{m,i})
+h_mT_m\sum_{j=1}^{s}\gamma_{ij}k_{m-1,j}
+h_mT_m\sum_{j=1}^{i-1}\widetilde\gamma_{ij}k_{m,j},
\]

\[
u_{m+1}=u_m+h_m\sum_{i=1}^{s}
(b_i k_{m,i}+v_i k_{m-1,i}).
\]

(T_m) is an arbitrary W-matrix approximation to (f_y(t_m,u_m)); the order conditions are constructed to tolerate this approximation. The matrices (widetilde A) and (widetilde\Gamma) are strictly lower triangular. When both vanish, all current-step stage equations use only (k_{m-1,j}), so they are independent. They share the same shifted matrix (I-h_m\gamma T_m), allowing one direct factorization and (s) RHS solves, or a shared matrix-free/Krylov preconditioner.

For the method's formal order, the dissertation gives simplifying conditions (C(q)), (\Gamma(q)), (B(p)) and convergence order
\[
p_* = \min(q+1,p)
\]
under bounded step ratio and compatible starting errors. In particular, (q=p=s) is an available order-condition construction in that framework. This does **not** imply every coefficient choice is A/L-stable; stability region, stiff accuracy, W-perturbation robustness and the allowed step-ratio interval must be verified for the chosen table.

This family is the best first implementation because its common shifted matrix reuses the existing direct/Krylov services. Start by retrieving exact coefficient tables rather than inventing an array from the schematic equations.

Primary sources:
- Podhaisky, Schmitt & Weiner (2002), *Design, analysis and testing of some parallel two-step W-methods for stiff systems*, Applied Numerical Mathematics 42, 381–395, DOI [10.1016/S0168-9274(01)00162-3](https://doi.org/10.1016/S0168-9274(01)00162-3). The publisher abstract reports parallel stage computation, (q=s), (p=s), stability generalizations and shared-memory Krylov comparisons.
- Schmitt & Weiner (2004), *Parallel Two-Step W-Methods with Peer Variables*, SIAM Journal on Numerical Analysis 42(1), 265–282, DOI [10.1137/S0036142902411057](https://doi.org/10.1137/S0036142902411057). The paper proves order (s-1) convergence/stability for general step sequences, derives stronger order conditions and identifies numerical candidates up to order 7.
- Weiner, Schmitt & Podhaisky (2004), *Parallel 'Peer' two-step W-methods and their application to MOL-systems*, Applied Numerical Mathematics 48, 425–439, DOI [10.1016/j.apnum.2003.10.005](https://doi.org/10.1016/j.apnum.2003.10.005). The paper reports stage-parallel linearly implicit methods with order/stage-order (s-1) or (s), optimal damping at infinity and shared-memory Krylov tests on MOL systems.

## 3. Multi-implicit peer two-step W: distinct-shift candidate

Schmitt, Weiner & Podhaisky (2005), *Multi-implicit peer two-step W-methods for parallel time integration*, BIT Numerical Mathematics 45, 197–217, DOI [10.1007/s10543-005-2635-y](https://doi.org/10.1007/s10543-005-2635-y), introduces a subclass of order (s-1) in which all stage calculations use information from the previous time step only. Different implicit system parameters are assigned to different stages to obtain optimal stiff damping. The authors report uniform stability for realistic step-size sequences, a superconvergence property and matrix-free Krylov experiments on shared-memory hardware.

This is a different cost model from common-shift TSW:
- all stage solves are independent and may overlap;
- the shifted operators need not be identical;
- one LU cannot simply be reused for all stages;
- CPU cost includes (s) independent Krylov processes or a verified shared-basis/multi-shift strategy, and possibly stage-specific preconditioner actions.

Do not invent a multi-shift Krylov implementation. First inspect the existing `rodas5p_krylov`, `LinearOperator`, `LinearSolverConfig` and preconditioner contracts. If a common Krylov basis is valid for the shift family, demonstrate it for the actual operator and record orthogonalization, storage, residual and breakdown costs. Otherwise use independent existing Krylov solves in the existing `ParallelExecution` pool.

## 4. Implicit parallel peer methods: nonlinear comparator

Schmitt, Weiner & Erdmann (2005), *Implicit parallel peer methods for stiff initial value problems*, Applied Numerical Mathematics 53, 457–470, DOI [10.1016/j.apnum.2004.08.019](https://doi.org/10.1016/j.apnum.2004.08.019), also uses (s) peer stages computable in parallel and proves an order-(s-1) zero-stable subclass for general step-size sequences. These are implicit peer equations rather than linearly implicit W equations. They are a useful comparator if W-perturbation or damping constraints block a good W table, but nonlinear root work and error certification need separate accounting. Reuse existing residual-first/homotopy primitives only where their map contract matches; do not turn them into a second generic Newton/homotopy engine.

## 5. Sequential peer and stiffly accurate controls

Two separate controls should be included:

1. **Rosenbrock-type peer two-step methods.** Podhaisky, Weiner & Schmitt (2005), DOI [10.1016/j.apnum.2004.08.021], report stage order (s-1), zero-stability for variable step sequences, and (L(\alpha))-stable methods for (s=4,\ldots,8). Their "order (s)" result is an effective-order result for constant step size. The generic peer representation has a lower-triangular current-stage matrix (G); therefore a particular table is not stage-parallel unless its actual (G) has independent/diagonal structure. Use it as a sequential high-stage-order control until verified otherwise.
2. **Stiffly accurate two-step W with an extra variable-step superconvergence condition.** Klinge's 2020 dissertation explicitly notes that this stronger condition can give order (p_*=s+1) on variable steps but the construction makes current stages no longer parallel. Keep it as a sequential accuracy control, not a candidate for the parallel mainline.

This creates a genuine Pareto frontier: stage independence and low work-span versus stage order/global order, stiff accuracy and variable-step superconvergence. We should not silently trade away the property that motivated the project.

## 6. Error estimation, startup and variable-step control already have literature

Do not invent a peer-W controller before reading the existing methods:
- Kulikov & Weiner (2011), *Global error estimation and control in linearly-implicit parallel two-step peer W-methods*, Journal of Computational and Applied Mathematics 236(6), 1226–1239, DOI [10.1016/j.cam.2011.08.006](https://doi.org/10.1016/j.cam.2011.08.006), develops local and global error estimators and automatic step-size control, with experiments on nonstiff, stiff, very stiff and large-scale problems and comparison with RODAS.
- Every two-step method requires consistent initial peer/stage values. The two-step W scheme explicitly needs (k_{0,i}); peer formulations require (U_{0,i}). The selected method's matching-order startup and rejection/restart behavior must be recovered from its original paper or companion code. The 2010 "parallel start" paper is explicitly for **explicit** peer methods; do not transplant its startup to a stiff linearly-implicit table without a derivation.
- Variable-step coefficients depend on (\sigma_m=h_m/h_{m-1}). The adaptive controller must enforce the theorem's step-ratio interval and recompute the correct coefficient family when (h_m) changes. Startup cost and any restart after event/discontinuity must be fully charged.

The immediate work is to fetch the original coefficient/estimator/startup tables and build an exact method certificate. Reuse the existing VigilODE step session, controller/work-ledger, local CPU executor, LU/Krylov and stage-batch measurement infrastructure.

## 7. Concrete next experiment

After the coefficient-source lock, use one small scalar/vector stiff test to validate the exact peer recursion and its amplification matrix, then a source-backed MOL workload. Compare:
- protected RODAS5P sequential;
- common-shift parallel TSW with one factorization/multiple RHS;
- multi-implicit peer-W with independent stage-specific shifted solves;
- sequential high-stage-order Rosenbrock-type peer control.

Measure accepted steps and achieved output error before timing. At least seven randomized paired CPU full-IVP runs are required for performance claims. Keep all GPU paths disabled.
