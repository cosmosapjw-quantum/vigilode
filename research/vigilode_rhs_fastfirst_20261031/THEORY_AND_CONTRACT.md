# VigilODE native RHS-only fast-first contraction research | R4/R5

**Scope**: VigilODE's own generic derivative-free homotopy solver, not BASS/REI physics, not an alternative RODAS5P tableau. Original baseline: PR131 exact `b289d8e8165bf7d5b1d3255c9a02e89bc4a20b9e`, `crates/vigilode-coupled/src/rhs_homotopy.rs` Git blob `e56b00146d58845786f68c47d1adb50d7ec24558`. Opt-in additive API, no change to production defaults, no user tolerance changes, no JVP/Jacobian/FD.

## Mathematical result (R5 direct derivation)

Let a closed invariant set D contain all iterates, and assume the **exact** source-owned callback map G: D -> D is a contraction in the Euclidean norm with a caller-proved constant 0<=q<1. For any 0<lambda<=1, define H_lambda(x)=(1-lambda)a+lambda G(x), with H_lambda(D) subset D and q_lambda=lambda q. Let r_lambda(x)=H_lambda(x)-x.

The exact Picard baseline y=H_lambda(x) has
```
||r_lambda(y)||_2
= ||H_lambda(H_lambda(x))-H_lambda(x)||_2
<= q_lambda ||H_lambda(x)-x||_2
= q_lambda ||r_lambda(x)||_2.
```
Thus if a depth-one Anderson trial z in D is actually evaluated **before** evaluating H_lambda(y), and
```
||r_lambda(z)||_2 <= q_lambda ||r_lambda(x)||_2,
```
z is admitted without the extra baseline map call, and the same geometric residual contraction envelope holds. If the trial fails, evaluate the original Picard baseline and choose the smaller *actually evaluated* residual. Hence the accepted exact-real residual sequence satisfies ||r_{k+1}||<=q_lambda||r_k||, and the unique fixed-point error obeys ||x_k-x*||<=||r_k||/(1-q_lambda). For an affine scalar G(x)=ax+b, 0<a<1, depth-one Anderson's exact trial is b/(1-a); a new fast-first implementation needs three map callbacks where the old baseline-first implementation needs four.

**Critical distinction:** this is not a proof that the speculative trial dominates the uncomputed Picard residual; the *a priori contraction envelope* replaces that unnecessary comparison. If contraction is not actually established for the frozen map and domain, a numeric q argument is NOT a proof and the theorem is inapplicable. In particular, a locally observed rate or a contraction of an unrelated stage may not be transported. Binary64 norms, callback arithmetic, and evaluation rounding require separate outward enclosures before a physical certification claim. The library reports operational residuals only.

## Coding contract

Add a separate public `solve_rhs_homotopy_contraction_fast_first(map,anchor,cfg,q)`. Require `cfg.acceleration == ResidualDominance`, finite 0<=q<1, and existing config/source domain/epoch checks. At each station reset Anderson history; use lambda*q, not q. Only evaluate the speculative trial before Picard if a full two-evaluation budget remains. Retain failed work accounting, callback-fatal behavior, exact admissibility checks and partial station receipts. No current source file is changed until the separate C5 implementation commit. Default `solve_rhs_homotopy` and every caller remain untouched.

## Experiment and admission

1. Original-vs-opt-in affine scalar q=0.99, same actual stopping tolerance 1e-12, require 4->3 RHS calls, same root tolerance and no derivatives.
2. Nonlinear invariant-map case, independent bisection root, compare actual residual and callback counts; no universal work-speed claim.
3. Bad q, model mutation, foreign/out-of-domain trial, finite resource limit: must fail closed or preserve charged fallback.
4. Same runner/flags paired wall measurements with warmup and ABBA/BAAB blocks for n=1 and n=256. State/runtime and full callback costs included; no extrapolation to other sizes.

Literature provenance: Walker & Ni, SIAM J. Numer. Anal. 49 (2011), DOI 10.1137/10078356X; Toth & Kelley, SIAM J. Numer. Anal. 53 (2015), DOI 10.1137/130919398; Pollock & Rebholz, IMA JNA 41 (2021), DOI 10.1093/imanum/draa095. The fast-first budget screen above is **derived here**, not asserted to be a new published method.

## Phase checkpoint and stop
- R4/R5: equations and proof recorded; Wolfram affine scalar independent reduction returned {a/(a-1), b/(1-a), 0}. CAS scope scalar only.
- C1–C4: inherit source and acceptance, **not** admitted as new official DAG nodes.
- C5–C8: NOT_RUN at this theory checkpoint.
- Native subagents: NOT_RUN (host tool inventory has no native delegation).
- Official VigilODE DAG **7/23**, central REI **28/88** (informational only), successor **0/12**, C9 actual distinct author **0/1**, C10 NOT_STARTED. Production HOLD; no automatic merge, default change, tolerance relaxation or official advancement.
