# VigilODE independent algorithm lane: source-owned contraction and directed fast-first residual

**Unit**: VIGILODE_RHS_SOURCE_CERTIFIED_FASTFIRST_DIAGONAL_V1. **Base**: PR150 immutable HEAD `27eae39802a1c9bedebc60486e17e83430989829`. **Branch**: `research/vigilode-source-certified-fastfirst-20261031`. Separate VigilODE research, not BASS/REI, RODAS5P or a production solver migration.

## R1–R3 scope and immutable source (derived)
Treat each **represented binary64 coefficient** as an exact real: `G_i(x)=a_i x_i+b_i` for nonempty dimension n, with finite coefficients and `q=max_i |a_i| < 1`. The object privately owns copied coefficient arrays; no caller-supplied callback or q can enter its methods. The domain is all R^n for the exact map; finite binary64 callbacks must fail closed on overflow. For a represented finite anchor c and each finite 0<lambda<=1, `H_lambda(x)=(1-lambda)c+lambda G(x)`. Its L2 Lipschitz constant is `lambda*q`; the source computes a directed upper q_lambda^+ and refuses if it rounds to >=1. All continuation stations reset acceleration history.

## R4–R5 constructive result
For an accepted represented candidate x, evaluate the **exact-real** residual `r_i(x)=(1-lambda)c_i+lambda(a_i*x_i+b_i)-x_i` via the already tested `rodas5p_core::directed::Interval` arithmetic. If r_i is enclosed in [l_i,u_i], let `m_i=min(|l_i|,|u_i|)` for sign-definite intervals and 0 otherwise, and `M_i=max(|l_i|,|u_i|)`. Compute directed nonnegative sums and square roots to obtain `L(x)<=||r(x)||_2<=U(x)`. All overflow/nonfinite directed operations are errors, not infinity-certified successes.

Given source-bound q_lambda^+<1, if an evaluated speculative Anderson candidate z satisfies

```
U(z) <= down(q_lambda^+ * L(x)),
```

then the actual exact-real residual obeys `||r(z)||<=q_lambda^+||r(x)||` and the original Picard residual need not be computed. The same conservative check applies to a rounded Picard fallback y=fl(H_lambda(x)); **the mathematical inequality for exact H_lambda(x) alone does not certify fl(H_lambda(x))**. If neither candidate can pass the bound and neither has an independently certified target root error, return `CERTIFICATE_INCONCLUSIVE` with charged work and completed station prefix, not a fabricated success.

For any represented candidate x, exact source contraction and the Banach fixed point theorem imply

```
||x - x_lambda^*||_2 <= U(x) / (1 - q_lambda^+).
```

Use `sub_down(1,q_lambda^+)` and `div_up(U,denom)` to certify a **caller-fixed root-error target**, rather than treating an operational residual norm as a physical error. Computed float Anderson gamma is an *untrusted predictor only*. Neither division by a tiny gamma denominator nor a large coefficient certifies anything; only original interval residual decides. Work is charged for every started source evaluation and every exact residual enclosure.

Independent closed-form target for comparison (Wolfram verified for n=2):
```
x_lambda,i^* = ((1-lambda)*c_i + lambda*b_i)/(1-lambda*a_i).
```
The rational represented-source identity can be independently checked with Python Fraction. Finite floating tests supplement but do not substitute for the above real inequalities.

## Binding cautions, scientific negative controls
- **Source-specific theorem**, not generic `RhsOnlyMap`, not cubic source, stiff IVP, RODAS5P, or a claim of physical trajectory accuracy.
- An immutable source object proves the location of q and the residual evaluation code, not the correctness of the caller's physical model selection. No post-hoc "q=.99" input accepted.
- Forward verified inverse-error upper bound can remain valid even if fast-first is slower; wall-time comparison at **matched certified root error** must include interval evaluations and failed trials.
- Same-author adversarial review (R6/C9-like) is NOT formal distinct-author C9.
- Existing `solve_rhs_homotopy` remains unchanged; standalone opt-in research module only. Finite `max_map_evaluations` and source/clock/candidate validation must fail closed.
- Previous provisional N4C/BASS/REI results not merged, deleted, or reclassified.

## Admission and stops
Current official VigilODE DAG 7/23, central REI 28/88 informational, successor admitted 0/12; different-author C9 0/1, C10 NOT_STARTED, production HOLD, merge=false. No automatic DAG advancement or tolerance relaxation.

Checkpoint R1–R5: this proof/construction is posted before implementation, and no native implementation PASS is claimed at this point. Next C5: integrate source-owned opt-in Rust module + original directed arithmetic; C6 targeted and negative tests; C7 independent rational source crosscheck; C8 paired full validation-cost/wall-time tests; C9 same-author adversarial bounded review; C10 handoff.
