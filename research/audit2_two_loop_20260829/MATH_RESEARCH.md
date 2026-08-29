# Audit 2: mathematical research loop

Status: exploratory. Source: `codex/vigilode-scientific-validity-v2`, commit `93fe348ce36859dd5f78b31267d771ea9c054677`, tree `a0a045226175e261664533a56188352ded1da75a`. This is NOT the old K0 continuation. Three user-supplied audit documents and the research harness were the seed; assertions in an external review are hypotheses until checked against code/data.

## 1. Claim–evidence adjudication

**Confirmed:** a relative clipped/dense policy-insensitivity test is not a general accuracy test. **Rejected as a description of the code:** the current gate is NOT `abs(Ec-Ed) <= .1 Ed`. `DualOutputPolicyEvidence::new` computes the direct trajectory distance in an immutable common reference WRMS basis. `classify_output_policy_dominance` applies `D <= .1 Ed`. The source even comments explicitly that D must not be inferred from scalar errors.

**Qualified:** the gate is not universally impossible. Identical correct trajectories pass. Two accurate trajectories with distinct leading errors can fail; two identical inaccurate trajectories can pass. It tests a different property. Changing `.1` to a larger fitted number is not a remedy.

**Rejected:** the second audit's proposed `abs(Jc-Jd) <= .05 Jd` is not an accuracy theorem. Clipping changes accepted steps and shifted systems, so legitimate work differences are expected. Applied to the existing data, it also rejects all 54 cases. Work differences must remain measured outcomes, not forced equality constraints.

**Corrected coverage:** the full compact evidence has six families, dimensions 96/384/1536, and tolerances 1e-4/1e-6/1e-8: 54 combinations. Only the selected full-state slice is n=96 at 1e-8. Six raw slices were independently recomputed against matching saved reference states; the remaining 48 have compact metrics, not complete saved trajectories in this repository.

**Qualified convergence:** error-versus-rtol response is not error-versus-h order. The second audit recognizes that global dense error may be O(h^5), but its final V1 requires slope four on a fixed-interval test. Those instructions are inconsistent. Its final proposed independent-error/triangle-bound direction is preferable to its earlier JVP-equality gate, provided an accuracy budget is justified independently of these observed data.

**Preserved:** historical 54/54 `output-policy-dominated` and the closed freeze/holdout remain unchanged. Their errors are not declared accurate merely because the gate interpretation was defective. SciPy Radau success and typed-unavailable CVODE cannot establish a BDF performance comparison.

## 2. Norms, error budgets, and uncertainty

Fix common positive weights S_j(t) from the same reference, and define

\[
\|v\|_* = \max_{t\in\mathcal T}\sqrt{n^{-1}\sum_j[v_j(t)/S_j(t)]^2}.
\]

Let `Ec=||yc-yref||*`, `Ed=||yd-yref||*`, and `D=||yc-yd||*`. Then

\[
|E_c-E_d|\le D\le E_c+E_d.
\]

The lower bound does not identify D or its cause. In particular, `yc-yref=+v` and `yd-yref=-v` have equal error norms and nonzero distance. Nor is D a pure interpolation defect: clipped and dense runs can follow different meshes.

If `||yref-ytrue||* <= u` is justified in this SAME norm, then for each arm

\[
\max(E-u,0)\le\|y-y_{true}\|_*\le E+u.
\]

For a separately specified observable budget B: upper<=B means within budget; lower>B means outside; otherwise reference-unresolved. Without B, no accuracy PASS is inferred. An empirically estimated u is not automatically a proved bound. The additive Rust diagnostic makes this conditional nature explicit. It neither changes the old gate nor creates a freeze.

These norms and budgets are dimensionless after reference scaling. They are not solver-local rtol/atol guarantees. No equality of secondary error estimates, digest equality across backends, or post-hoc tolerance widening is required.

## 3. Continuous extension: local versus global order

The implemented expression is

\[
P(\theta)=(1-\theta)y_n+\theta[y_{n+1}+(1-\theta)(d_0+\theta(d_1+\theta d_2))].
\]

It is quartic, with `d^4 P/dtheta^4 = -24 d2`, and exact endpoint interpolation. The existing “third-degree” comment is incorrect; the production formula is not changed here. Wolfram independently checked the polynomial degree, derivative and endpoints.

An order-four continuous extension has exact-start local defect O(h^5). A fifth-order endpoint method has local defect O(h^6). At a fixed final interval with a stable, smooth finite-dimensional problem, nodal error is O(h^5), while the interpolation defect at a query is injected once, not accumulated at every step. Thus global dense error can be O(h^5), not necessarily O(h^4).

Use the manufactured nonlinear problem

\[
y'=-\lambda(y-e^{-t})-e^{-t}+\nu(y-e^{-t})^2,\qquad y(0)=1.
\]

The exact solution is e^-t, `J=-lambda+2 nu(y-e^-t)` and fixed-y `f_t=(1-lambda)e^-t+2nu(y-e^-t)e^-t`. Wolfram checked both identities. With \(\delta=y-e^{-t}\), the exact off-trajectory flow is

\[
\delta(t+s)=\frac{\delta(t)e^{-\lambda s}}
 {1-(\nu/\lambda)\delta(t)(1-e^{-\lambda s})}.
\]

It permits the signed decomposition

\[
P(Y_n)-y(t_n+\theta h)
=[P(Y_n)-\phi_{\theta h}(Y_n)]
+[\phi_{\theta h}(Y_n)-y(t_n+\theta h)].
\]

The first term is interpolation defect from the numerical left endpoint; the second is propagated nodal error. The code records both and the decomposition residual. Differences of scalar maxima cannot make this separation.

All coefficient strings are the original rounded decimal snapshot, evaluated at 70 decimal digits; this is NOT a claim of new high-precision authoritative coefficients. For lambda=2, nu=.25, theta=.37 and h=1/4 through 1/128, local dense slopes are near five, local endpoint near six before rounded-input cancellation becomes dominant; global dense and endpoint near five. Linear interpolation deliberately substituted as a negative control converges near two. At lambda=1000, dense slopes near four over the sampled range are retained as nonuniform-stiff-regime evidence. No assertion of uniform order or extrapolation through the roundoff floor is made.

## 4. A stronger route to the original RODAS5P stage-cost problem

The target is the full nonlinear eight-stage block, not merely a single linear stage solve. With frozen M, J_n, h and invertible `W=M-h gamma J_n`, its residual is

\[
R_i(K)=Wk_i-hf(t_i,y_n+\sum_{j<i}\alpha_{ij}k_j)
-hJ_n\sum_{j<i}\Gamma_{ij}k_j-h^2\gamma_i f_{t,n}.
\]

Only earlier stages occur in the nonlinear evaluation. Therefore

\[
(\mathcal J_R)_{ii}=W,\qquad
(\mathcal J_R)_{ij}=-h(\alpha_{ij}J_i+\Gamma_{ij}J_n),\quad j<i.
\]

For `J_R z=r`, solve sequentially

\[
Wz_i=r_i+hJ_i\sum_{j<i}\alpha_{ij}z_j
             +hJ_n\sum_{j<i}\Gamma_{ij}z_j.
\]

One common-W factorization, eight vector solves and fourteen exact JVPs suffice for the implemented candidate. No full stage J_i or full 8n-by-8n Jacobian is assembled. It computes the SAME linearized Newton correction as the full target oracle, not a heuristic cheap substitute. For residual sign `R=lhs-rhs`, Newton updates `K <- K-z`.

Algebraically, `J_R=D(I-L)` with D block diagonal W and L strictly lower block triangular. Thus `L^8=0` and `(I-L)^-1=sum_{k=0}^7 L^k`; no norm-less-than-one assumption is needed for this finite identity. `det(J_R)=det(W)^8`. Subject to well-defined stage evaluations, this causal target has a unique recursively constructed root for fixed W. Generic simple-fold/multiple-root rescue language is therefore not justified for this particular target while W stays invertible. Off-diagonal coupling may still make it very ill-conditioned or nonnormal. Domain failure, singular W or a different target are separate issues.

This sharpens the semi-Jacobian-free homotopy proposal: first remove the unnecessary full-target factorization while preserving its linearized certificate. Do NOT immediately demote the oracle to a sampled shadow lane or replace it with an unproved residual bound. A Newton correction remains only a local linearized error diagnostic: nonlinear remainders, physical domains and conditioning still require safeguards.

The test-only Rust candidate shares the real crate APIs and has no production dispatch. Full-oracle and candidate corrections are compared by independent backward errors and a condition-aware state-difference bound, not by comparing their already tiny error estimates to unrealistic relative precision. Setup/frozen-J costs are not claimed free; test-oracle and condition-number work are validation work, not part of a reported runtime speedup.

## 5. Hypothesis decisions and negative results

- **Supported, scoped:** separate accuracy budget from relative policy sensitivity; keep old evidence intact.
- **Supported by formula and probes:** quartic extension, local defect order five; stable global order five need not contradict “order-four extension.”
- **Rejected shortcut:** force clipped/dense JVP parity; all54 fail it and no theorem justifies it.
- **Test-only survivor:** common-W exact block-forward correction. Production promotion is HOLD for fresh independent review and broader mass/nonnormal/domain cases.
- **Not established:** all54 outputs are accurate, a new threshold is valid, holdout/generalization passes, or RODAS5P beats BDF.
- **Not established:** using scalar error ratios alone proves stiffness order, and low-stage rank guarantees easy Krylov solves.

## 6. Minimal next research

Keep the calibration and holdout frozen. Independently review the correction identity and test candidate; extend only its mass-matrix, nonlinear and failure cases, then integrate behind an explicit opt-in research path. An actual accuracy/equal-error claim requires a predeclared observable budget and valid reference uncertainty. BDF comparison requires a working production comparator, not current unavailable rows. These are scientific dependencies, not new package checks.

## Primary sources

1. G. Steinebach (2023), *Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package*, DOI 10.1007/s10543-023-00967-x. Supports the eight-stage method and continuous-extension construction, not this project's performance.
2. SUNDIALS CVODE Mathematics documentation, https://sundials.readthedocs.io/en/latest/cvode/Mathematics_link.html. Supports variable-order BDF and local error-control semantics, not a global accuracy guarantee.
3. SciPy Radau documentation, https://docs.scipy.org/doc/scipy/reference/generated/scipy.integrate.Radau.html. Fifth-order endpoint with cubic dense output illustrates that endpoint order and dense polynomial degree are different concepts.
4. The inspected repository's `global_error.rs`, `dense_output_v2.rs`, `block.rs`, `homotopy.rs`, official coefficient snapshot, and immutable `external_reaudit_bundle` are the direct evidence for implementation/data claims.
