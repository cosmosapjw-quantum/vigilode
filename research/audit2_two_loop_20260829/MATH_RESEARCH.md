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

For a separately specified observable budget B: upper<=B means within budget; lower>B means outside; otherwise reference-unresolved. Without B, no accuracy PASS is inferred. The caller must also declare whether u is an asserted upper bound or only an estimate. Estimate-only uncertainty may produce the interval diagnostic but cannot yield either `WithinBudget` or `OutsideBudget`; it remains `ReferenceUnresolved`. An empirically estimated u is not automatically a proved bound. The additive Rust diagnostic makes this conditional nature explicit. It neither changes the old gate nor creates a freeze.

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

The official decimal tableau is not bit-exactly strict lower/constant diagonal after its C-form to Gamma-form conversion. Direct observation of the unprojected f64 data gives maximum forbidden alpha `5.577737968635803e-16`, maximum upper-Gamma `4.994632140352628e-16`, and maximum Gamma diagonal error `8.881784197001252e-16`. Therefore the literal exact-structure claim made by the first candidate was invalid for those raw coefficients.

The continuation defines a separate research target by projecting only those structurally forbidden entries with the fixed rule

\[
\tau_{\rm proj}=64\,\epsilon_{\rm f64}=1.4210854715202004\times10^{-14}.
\]

This tolerance and the projected entry pattern were fixed independently of correction residuals, state differences, campaign outcomes, and the historical54 rows. Leakage above the fixed tolerance is a typed projection failure; accepted leakage is set to the exact structural zero or common diagonal. Only this projected research target has bit-exact strict-lower/common-W structure. The official coefficients, production residual, historical gates, and production dispatch are unchanged.

The target below is the projected full nonlinear eight-stage block, not merely a single linear stage solve. With frozen M, J_n, h and invertible `W=M-h gamma J_n`, its residual is

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

One common-W factorization, eight vector solves and fourteen correction JVPs suffice for the implemented candidate. No full stage J_i or full 8n-by-8n Jacobian is assembled. When the supplied JVP is consistent with the Jacobian oracle, it computes the same linearized Newton correction as the projected full-target oracle, not a heuristic cheap substitute. For residual sign `R=lhs-rhs`, Newton updates `K <- K-z`. The fresh review independently confirmed this sign.

Algebraically, `J_R=D(I-L)` with D block diagonal W and L strictly lower block triangular. Thus `L^8=0` and `(I-L)^-1=sum_{k=0}^7 L^k`; no norm-less-than-one assumption is needed for this finite identity. `det(J_R)=det(W)^8`. Subject to well-defined stage evaluations, this causal target has a unique recursively constructed root for fixed W. Generic simple-fold/multiple-root rescue language is therefore not justified for this particular target while W stays invertible. Off-diagonal coupling may still make it very ill-conditioned or nonnormal. Domain failure, singular W or a different target are separate issues.

This sharpens the semi-Jacobian-free homotopy proposal: first remove the unnecessary full-target factorization while preserving its linearized certificate. Do NOT immediately demote the oracle to a sampled shadow lane or replace it with an unproved residual bound. A Newton correction remains only a local linearized error diagnostic: nonlinear remainders, physical domains and conditioning still require safeguards.

The Rust candidate shares the real crate APIs but is compiled only by the non-default `audit2-research` feature and has no production dispatch. `FullTargetOracle` is the default research backend; `CommonWBlockForward` requires explicit selection, and the comparison constructs one target snapshot so both arms see matching trial stage states. Full-oracle and candidate corrections are compared by independent backward errors and a condition-aware state-difference bound, not by equality of already tiny secondary errors. Nonlinear residuals before and after the proposed update are reported but are not converted into a validity flag.

The extended tests include the original12 identity-mass points and a nonsingular mass matrix with determinant2.7 plus strong nonnormal coupling ratio134.6667. The latter has full-target Frobenius condition estimate96.8247899149117 and common-W backward error5.476681809776196e-17. Across the original12 points the condition range is32.1868941892757 to2686.7976507838134, maximum state-relative difference is4.664692226645801e-15, and maximum independently applied common-W backward error is approximately4.6955771192625894e-17.

Zero RHS uses absolute zero residual/state criteria and leaves the undefined relative 0/0 state comparison absent. Missing, failing and inconsistent JVPs, singular and overflowed solves including later-stage overflow, NaN input, and malformed shapes are retained as typed outcomes with attempt/completion counts and partial progress. An intentionally inconsistent JVP is a domain counterexample: a finite correction is not promoted to oracle agreement when the independent full-target residual exposes the mismatch.

Setup/frozen-J costs are not claimed free. One successful common-W arm records one setup, one factorization, eight solves, fourteen correction JVPs, eight shifted diagnostic applies, fourteen off-diagonal diagnostic JVPs, and one nonlinear diagnostic. The comparison separately records two independent full-target validation applies. For the identity case this totals52 counted candidate JVP vectors under the existing counter semantics. Test-oracle and condition-number work are validation work, not part of a reported runtime speedup.

## 5. Hypothesis decisions and negative results

- **Supported, scoped:** separate accuracy budget from relative policy sensitivity; keep old evidence intact.
- **Supported by formula and probes:** quartic extension, local defect order five; stable global order five need not contradict “order-four extension.”
- **Rejected shortcut:** force clipped/dense JVP parity; all54 fail it and no theorem justifies it.
- **Research-only survivor:** exact block-forward correction for the explicitly projected target, behind a non-default feature. It is not an exactness claim for the unprojected decimal tableau or a production promotion.
- **Not established:** all54 outputs are accurate, a new threshold is valid, holdout/generalization passes, or RODAS5P beats BDF.
- **Not established:** using scalar error ratios alone proves stiffness order, and low-stage rank guarantees easy Krylov solves.

## 6. Claim ceiling and minimal next research

The ceiling is `EXPLORATORY_NONAUTHORITATIVE` research diagnostic only. Not admitted: unprojected exactness, a nonlinear certificate, an accuracy PASS, timing/ranking/speedup, BDF/CVODE performance, production activation, holdout/freeze claims, or scientific-publication admission.

Keep the calibration and holdout frozen. The bounded mass/nonnormal/domain/failure extension and explicit research entry are implemented, and the final aggregate11-test structured-correction suite passed. This scoped test result does not authorize a new accuracy/equal-error campaign: that requires a predeclared observable budget B and reference-uncertainty treatment chosen independently of the historical54 outcomes. BDF comparison requires a working production comparator, not current unavailable rows. These are scientific dependencies, not new package checks.

## Primary sources

1. G. Steinebach (2023), *Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package*, DOI 10.1007/s10543-023-00967-x. Supports the eight-stage method and continuous-extension construction, not this project's performance.
2. SUNDIALS CVODE Mathematics documentation, https://sundials.readthedocs.io/en/latest/cvode/Mathematics_link.html. Supports variable-order BDF and local error-control semantics, not a global accuracy guarantee.
3. SciPy Radau documentation, https://docs.scipy.org/doc/scipy/reference/generated/scipy.integrate.Radau.html. Fifth-order endpoint with cubic dense output illustrates that endpoint order and dense polynomial degree are different concepts.
4. The inspected repository's `global_error.rs`, `dense_output_v2.rs`, `block.rs`, `homotopy.rs`, official coefficient snapshot, and immutable `external_reaudit_bundle` are the direct evidence for implementation/data claims.
