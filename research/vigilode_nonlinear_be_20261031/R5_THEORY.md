# VigilODE nonlinear source-owned fast-first: constructive derivation (2026-10-31)

Authority: PR #152 HEAD `c89c639cb67e0528db733598fd654fa538ce8182` (unmerged draft), VigilODE's own RHS-only research lane. No BASS/REI model or original eight-stage RODAS admission. We model represented binary64 source coefficients as exact dyadic reals, without JVP, Jacobian, finite differences, AD, or Newton. The fast-first accelerator is a **predictor**; each accepted candidate is independently certified by the unchanged nonlinear source.

## Nonlinear source and source-owned contraction

For \(k>0,g\ge0,Y>0\), let \(y'=-\phi(y),\ \phi(y)=ky+g y^3\), over \([0,T]\), \(y(0)=Y\). The exact solution remains in \([0,Y]\). Let \(h=T/N>0\) and BE equation on the represented state \(v\) be

\[F_{h,v}(z)=(1+hk)z+hgz^3-v=0.\]

Write \(a=1+hk\), \(b=hg\), \(U_0=v/a\). On \(D=[0,U_0]\), the *nonlinear source-owned* preconditioned Picard operator is \(G(z)=U_0-(b/a)z^3\). This is exactly equivalent to BE, not a surrogate differential equation. With

\[q=3bU_0^2/a=3hgv^2/(1+hk)^3,\]

\(0\le q<1\) implies \(G(D)\subseteq D\): \(G(0)=U_0\), \(G(U_0)=U_0(1-q/3)\ge0\), and \(|G'(z)|\le q\) for all \(z\in D\). Thus the BE state root is unique there. The transformation removes the potentially stiff linear factor from the contraction criterion, while only evaluating source values. For represented finite candidates, a slightly enlarged representable interval \([0,U^+]\) is permitted **only if** interval checks also prove invariance and use the correct enlarged source bound \(q^+=3b(U^+)^2/a<1\).

A candidate \(\hat z\) is certified via its exact real BE source residual \(R=F_{h,v}(\hat z)\): because \(F'_{h,v}(z)=a+3bz^2\ge a>0\) for positive \(z\),

\[|\hat z-z_*|\le |R|/a.\]

This bound is *not* the f64 operational \(|G(\hat z)-\hat z|\). An Anderson trial can be accepted before Picard if either its source-certified residual is within the fixed stage budget or its exact residual satisfies the source-derived q envelope. No invented derivative callback exists in the solver.

## Whole-IVP physical error, rigorous scalar source theorem

Let the true solution be \(y(t)\), with exact grid instants \(t_n=nh\), and define \(e_n=\hat y_n-y(t_n)\). Because \(y'(t)=-\phi(y(t))\le -k y(t)\) and \(e^{-kh}\le(1+kh)^{-1}\),

\[0\le y(t_n)\le M_n=Y/(1+kh)^n.\]

For \(s\in[t_n,t_{n+1}]\), set \(P_n=kM_n+g M_n^3\), \(L_n=k+3gM_n^2\), bounding \(|y'|\le P_n\) and \(|\frac{d}{dt}\phi(y)|\le L_nP_n\). The exact-flow BE defect \(d_n=y(t_{n+1})+h\phi(y(t_{n+1}))-y(t_n)\) obeys

\[|d_n|\le D_n=h^2 L_nP_n/2\]

by integrating the endpoint quadrature remainder of \(\phi(y(t))\). Let the *actual represented candidate's exact-real original source residual* be \(R_n=\hat y_{n+1}+h\phi(\hat y_{n+1})-\hat y_n\). On nonnegative states, \(F_h(z)=z+h\phi(z)\) satisfies \(|F_h(u)-F_h(v)|\ge (1+hk)|u-v|\), hence

\[\boxed{|e_{n+1}|\le(|e_n|+D_n+|R_n|)/(1+hk)}.\]

For output at rounded time label \(\hat t_n=\mathrm{fl}(nh)\), add the time-displacement enclosure \(P_0|\hat t_n-nh|\). A conservative fixed source-owned residual budget is \(R_{\max}=(\varepsilon-\max_n[E_n^{\rm trunc}+P_0|\hat t_n-nh|])hk\), positive only if the chosen step grid can already certify the truncation remainder. Then for \(|R_j|\le R_{\max}\), \(E_n\le E_n^{\rm trunc}+R_{\max}/(hk)\le\varepsilon\). In particular, unlike affine fixed-point certification, this proves a **full trajectory physical-error bound on the model**. This theorem says nothing about arbitrary nonlinear RHS, adaptive grid changes, or production eight-stage RODAS5P.

Source params have dimensions \([k]=T^{-1}, [g]=T^{-1}Y^{-2}\). Thus \(q, hk\) are dimensionless, \(D_n,R_n,E_n\) have \([Y]\), and \(R_{\max}\) has \([Y]\). The exact solution is \(y(t)=Y e^{-kt}/\sqrt{1+(g/k)Y^2(1-e^{-2kt})}\) for \(k>0\), supplying an independent high-precision reference, not the source for the numerical solver.

## Direct numerical check and limitations

`validated_cubic_be.py` implements source-referenced `Fraction` residuals and *exact rational* whole-IVP recurrence for every represented source and state, while the candidate map remains f64. It verifies every certified bound against a separate 80-digit `mpmath` analytic solution. The two arms use the same source, step grid, physical epsilon, exact residual checks, and cost ledger; only baseline-first versus fast-first candidate order changes. Raw measurements are **Python algorithmic workloads**, not Rust or production speedups. Accepted positive trajectories, superfluous candidate costs, and noncontractive/truncation failures are preserved. Exact rational comparisons are trustworthy for the represented-source mathematical model; 80-digit transcendental reference remains a numerical crosscheck rather than a formal proof.

Historical official VigilODE DAG remains 7/23; REI 28/88 is informational; successor 0/12; C9 distinct author 0/1; production HOLD; auto-next false. Do not merge or change defaults/tolerances. Next coding target is native Rust with `rodas5p_core::directed::Interval` preserving proof dependency; full native CI has not yet been executed.
