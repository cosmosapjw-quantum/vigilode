# A cheaper original-equation certificate for the cyclic RVJ closure

## Scope and theory admission

This is the bounded R2/R3/R5/R7/R9 closure lane under the supplied
`../SKILL_CONTEXT.md` (physmath-research-loop 1.3.0), based on PR96
`13f3298bd0213f121e07385903f9508c435ad6a1`. It is **theory before coding**.
No native campaign, old executable, new solver experiment or symbolic checker
was run in this lane. Python was used only to read existing JSON entries.
The proofs below are analytical derivations; implementation and measured
performance remain separate admission gates.

The main result is a fixed-degree, direct Bernstein certificate for exactly
the same original equations and exact-slope Hermite path as the attachment.
It eliminates generic polynomial conversion, exploits two exact endpoint
identities, and reuses endpoint slope enclosures. These changes target the
dominant certification cost. They do not alter the physical error cap or
replace all-state, whole-path accuracy by endpoint or scalar-output accuracy.

Primary project sources, read directly:

- `../intake/rvj/f23_native/{derivation.md,native.cpp,reference.py,results.json}`.
- `../intake/rvj/f1_atlas/derivation.md`.
- `../intake/rvj/reviews/native_secondary_review.json`.
- `../../integrated_polyalgorithm_20261028/error_contract/THEORY.md` (E1--E3).
- `../../../crates/rodas5p-core/src/directed.rs` (finite interval API).

The first attachment already provides a sound bounded-model construction.
Its limitations are useful boundaries, not evidence that the closure idea
fails. Its existing n=4, N=64 compiled case spends a recorded median
0.001870575 s certifying, against 0.000030555 s integrating; its certificate
fraction is 0.9840102768532503. These are **reused old measurements**, not
measurements of the proposed implementation.

## R2: retained problem and adversarial findings

For cyclic indices, retain without modification

\[
 G_i(x,q)=q_i+\tfrac14q_{i+1}^2-x_i,
 \qquad f_i(x,q)=-x_i+\tfrac1{16}(q_i+q_{i+1})+c_i,
 \quad c_i=\tfrac1{32}+\frac{i\bmod3}{256}.
\]

The original problem is \(x'=f(x,q),G(x,q)=0\), with x=q=0 at t=0,
t in [0,1], X=[-1/8,1/8]^n, Q=[-1/4,1/4]^n. The proposed implementation
initially admits n=4,8,16. The theorem itself holds for n>=2 with these
cyclic equations. The single branch is `cyclic-small-real-zero`.

| Existing item | Critical conclusion and coding consequence |
|---|---|
| Global fixed-point contraction in Q | Retain it. It proves the original branch and residual-to-root conversion, rather than a root-iteration stopping rule. |
| Finite cyclic circuit | Retain it with a denominator guard. Its O(n) operation count is not a generic sparse-system complexity theorem. |
| Exact endpoint slopes in the Hermite definition | Essential. Rounded-slope dense output is a different path and must include its representation displacement. |
| Old endpoint radius | Valid but loose. Do not replace it by `(E+h*delta)/(1+lambda*h)` alone: at E=0 that expression underestimates the forcing contribution. |
| Native interval point constructor | Old singleton construction accepts nonfinite inputs before later operations fail. The finite scoped campaign is not thereby disproved. A public module should reject nonfinite inputs immediately, using the existing finite interval API. |
| Krawczyk radius and Newton tolerance | They are candidate-generation/inclusion choices, not mathematical constants in the final physical certificate. The new certificate does not inherit their values. |
| Atlas extension | The two F1 charts share determinant coverage. Neither resolves a selected-root singularity at determinant zero. The cyclic box contains no such singularity. |
| Existing exact oracle | Its 27 representative slabs do not mean every old certified slab was independently checked. New validation must identify its actual coverage. |
| Work comparison | Shared certification improvements benefit compiled, Newton and Newton-plus-Krawczyk candidates. Their gain must not be attributed solely to the compiled root strategy. |

### Theorem C1: branch, original residual and physical transfer

For x in X, \(T_x(q)_i=x_i-q_{i+1}^2/4\) maps Q to
[-9/64,1/8]^n and contracts in the infinity norm with factor 1/8. Therefore
there is one q(x) in Q, and for x,z in X and qhat in Q,

\[
 \|q(x)-q(z)\|_\infty\le\tfrac87\|x-z\|_\infty,\qquad
 \|\widehat q-q(x)\|_\infty\le\tfrac87\|G(x,\widehat q)\|_\infty. \tag{C1}
\]

**Proof.** The derivative of one scalar square on [-1/4,1/4] has magnitude
at most 1/2; its coefficient 1/4 gives 1/8. Subtract the fixed-point
equations and bring this contraction term to the left. The same operation
applied to \(\widehat q-T_x(\widehat q)=G(x,\widehat q)\) proves the second
claim. Banach's theorem supplies existence and uniqueness. The signs of
the original slow field on each upper and lower face of X are respectively
at most -7/128 and at least 1/8. Thus the exact slow solution stays in X.

For F(x)=f(x,q(x)), write F=-x+Cq+c. Since \(\|C\|_\infty=1/8\), the
maximum-coordinate Dini inequality gives contraction rate
\(\lambda=1-(1/8)(8/7)=6/7\). For any differentiable candidate (Xhat,Qhat)
remaining in X cross Q, define the **original** residuals

\[
 R_f=\widehat X'-f(\widehat X,\widehat Q),\qquad
 R_g=G(\widehat X,\widehat Q).
\]

If their suprema on a slab are bounded by Df,Dg, the slow physical error e
obeys \(D^+e\le-(6/7)e+\delta\), where

\[
 \delta=D_f+D_g/7,\qquad
 \|\widehat Q-q(x_{\rm exact})\|_\infty
 \le\tfrac87(e+D_g).                                      \tag{C2}
\]

This uses the actual reconstruction residual at its numerical input, so
it has the exact-flow anchor of PR94 E1/E4a. It does not mix flow contraction
with a local truncation estimate evaluated only on an exact trajectory.

### Circuit denominator and endpoint tangency

Set \(b_i=q_{i+1}/2\), \((Av)_i=b_iv_{i+1}\), and \(P=\prod_i b_i\).
Then \(A^n=PI\) and

\[
 (I+A)\sum_{k=0}^{n-1}(-A)^k=(1-(-1)^nP)I.
\]

The attached two-pass cyclic recurrence computes v satisfying
\((I+A)v=f\). On Q its denominator obeys

\[
 d\ge1-8^{-n}>0,                                           \tag{C3}
\]

namely 4095/4096, 16777215/16777216 and
281474976710655/281474976710656 for n=4,8,16. The exact lifted field is
s(y)=(f,v). In exact arithmetic,

\[
 G_qv-f=0                                                   \tag{C4}
\]

**even at inconsistent endpoints with G not equal to zero**. This is an
identity of the rational circuit, not an assumption that the numerical
integrator preserves G. Interval evaluation must enclose this exact circuit
and reject a denominator enclosure containing zero. The exact identity
permits endpoint cancellation in the mathematical residual; an interval
re-evaluation of its cancelling expression need not return [0,0].

The circuit uses no numerical derivative, Newton iteration, JVP, AD callback
or dense matrix. Its coefficients encode the analytic derivative structure
of these original equations; this is a structural RHS-only implementation,
not a black-box derivative-free theorem for arbitrary equations.

## R3: direct Bernstein certification with exact constants

### Path identity and domain envelope

Interpret stored finite binary64 endpoints y0,y1 as exact dyadics. Let h be
the exact physical slab length and \(s_j=s(y_j)\) the exact rational field.
The **certified path definition** is the Hermite cubic with those endpoints
and those exact slopes. Its degree-three Bernstein controls are

\[
 Y_0=y_0,\quad Y_1=y_0+hs_0/3,\quad
 Y_2=y_1-hs_1/3,\quad Y_3=y_1.                              \tag{C5}
\]

All displayed non-dyadic coefficients are exact rationals, enclosed by
directed arithmetic in code. Componentwise hulls of the four interval
controls must be subsets of X and Q. This proves the entire candidate path
remains in the same branch domain. Endpoint membership alone is inadequate.

### Theorem C2: exact slow-residual endpoint elimination

Let Xj,Qj be the controls in (C5). For one slow component set

\[
 D_0=f_i(y_0),\quad D_1=3(X_2-X_1)/h,\quad D_2=f_i(y_1),
\]
\[
 r_1=D_0/3+2D_1/3-f_i(Y_1),\qquad
 r_2=2D_1/3+D_2/3-f_i(Y_2).                                \tag{C6}
\]

The degree-three Bernstein coefficients of its slow residual are exactly
(0,r1,r2,0), and

\[
 D_f\le\tfrac34\max_i\{|r_{1,i}|,|r_{2,i}|\}.              \tag{C7}
\]

**Proof.** The derivative's degree-two controls are D0,D1,D2. Elevating
once gives D0, D0/3+2D1/3, 2D1/3+D2/3, D2. Since f is affine,
f of the path has controls f(Yj). The endpoint differences vanish by the
exact-slope definition. Thus the residual is
\(3\theta(1-\theta)[(1-\theta)r_1+\theta r_2]\), whose absolute value is
bounded by \((3/4)\max(|r_1|,|r_2|)\). The constant 3/4 is sharp when r1=r2
have the same nonzero sign. No tunable safety factor is involved.

An algebraically equivalent formula may reduce cancellation or arithmetic:
with M=(x1-x0)/h and \(a_j=-f_i(y_j)+(v_i(y_j)+v_{i+1}(y_j))/16\),

\[
 r_1=2(M-f_0)-\tfrac23(f_1-f_0)-\tfrac h3a_0,\quad
 r_2=2(M-f_1)+\tfrac23(f_1-f_0)+\tfrac h3a_1.                \tag{C8}
\]

Both formulas represent the same exact quantities. First implementation
may use (C6), with (C8) reserved for a bounded measured comparison.

### Theorem C3: only three algebraic interior coefficients are necessary

For component i set \(d_j=Q_{i,j}-X_{i,j}\), \(z_j=Q_{i+1,j}\), and
\(g_0=G_i(y_0),g_1=G_i(y_1)\). Its degree-six algebraic residual has
Bernstein coefficients

\[
 (g_0,g_0,\beta_2,\beta_3,\beta_4,g_1,g_1),                 \tag{C9}
\]

where, entirely in exact rational arithmetic,

\[
 \begin{aligned}
 E_2&=(d_0+3d_1+d_2)/5,& S_2&=(2z_0z_2+3z_1^2)/5,\\
 E_3&=(d_0+9d_1+9d_2+d_3)/20,& S_3&=(z_0z_3+9z_1z_2)/10,\\
 E_4&=(d_1+3d_2+d_3)/5,& S_4&=(2z_1z_3+3z_2^2)/5,\\
 \beta_k&=E_k+S_k/4 &&(k=2,3,4).
 \end{aligned}                                             \tag{C10}
\]

**Proof.** Degree elevation of the cubic d supplies E2,E3,E4. The Bernstein
product rule supplies S2,S3,S4 for the square of the cubic z. Endpoints of G
are g0,g1. By (C4), the derivative in theta of G at either endpoint is
zero. A degree-six Bernstein polynomial has endpoint derivatives
\(6(\beta_1-\beta_0)\) and \(6(\beta_6-\beta_5)\), giving (C9). No
assumption of exactly consistent numerical endpoints was used.

Let g bound both endpoint magnitudes across components and m bound the
three middle coefficient magnitudes. Then the sharper direct bound is

\[
 D_g\le\max\{g,(7g+25m)/32\}.                             \tag{C11}
\]

Indeed the sum w of the three interior Bernstein basis functions is
\(15t^2-10t^3\), where \(t=\theta(1-\theta)\in[0,1/4]\). Its maximum
is 25/32. The residual magnitude is at most (1-w)g+wm; maximizing this
linear expression for 0<=w<=25/32 gives (C11). The constant is sharp in
this coefficient-information class, attained at theta=1/2 when all three
middle coefficients share the larger signed magnitude. Writing the bound
as a maximum of two positive expressions avoids subtractive branching.

The algebraic square needs six products for its three interior coefficients,
plus endpoint residual evaluations, rather than a generic 4-by-4 polynomial
convolution and a seven-row power-to-Bernstein transform. This is an exact
kernel work comparison, not a measured runtime ratio.

### Theorem C4: endpoint enclosure reuse

For N consecutive slabs of one immutable represented path, the shared
endpoint yk and the same model/provider identity define one exact s(yk) and
one exact G(yk). Reusing their finite interval enclosures preserves all the
above proofs. Therefore only N+1 slope enclosures are needed, rather than
2N. Endpoint original-residual enclosures can also be reused.

The admissible cache key includes **all endpoint bits, model/provider epoch,
branch/domain identity and arithmetic contract**. Equal dimensions, close
states or an unchanged time index do not suffice. This model is autonomous;
an extension to a nonautonomous provider must additionally key physical time.
No cache hit may suppress accounting for a failed earlier attempt. A
streaming implementation can retain only the last prepared endpoint, so
these savings do not require an unbounded cache or O(Nn) extra memory.

## Optional radius improvement: a valid rational forcing envelope

### Theorem C5: a two-sided rational exponential enclosure

For E,delta,h>=0 and lambda>0 let z=lambda*h and

\[
 U_1=\frac{E+h\delta}{1+z},\qquad
 U_2=\frac{(1-z/2)E+h\delta}{1+z/2},\qquad U=\max(U_1,U_2). \tag{C12}
\]

The physical endpoint and whole-slab slow radii can be set to

\[
 E_{\rm end}=U,\qquad E_{\rm whole}=\max(E,U).               \tag{C13}
\]

**Proof.** The scalar comparison solution is
\(T(h)=w+(E-w)e^{-z}\), w=delta/lambda. For z>=0,
\(e^{-z}\le1/(1+z)\). Also
\(e^{-z}\ge(1-z/2)/(1+z/2)\): for 0<=z<2 take logarithms and use
\(\frac{d}{dz}\log((1+z/2)/(1-z/2))=1/(1-z^2/4)\ge1\); for z>=2 the
right side is nonpositive. If E>=w the upper exponential bound yields U1;
if E<=w the lower exponential bound yields U2. Furthermore

\[
 U_1-U_2=\frac{hz(\lambda E-\delta)}{2(1+z)(1+z/2)},
\]

so taking the maximum selects the correct side without a numerical
comparison of E and delta/lambda. T(t) is monotone on a constant-defect
slab, proving (C13). In particular E=0 selects U2, not U1.

Both exact-real expressions are at most the old valid endpoint bound
\(E/(1+z)+h\delta\). The new whole bound is at most E+h*delta.
Outward floating implementations can have different rounding widths;
strict numerical domination is not promised without comparing actual
enclosures. For the present h<=1 and lambda=6/7, all coefficients in U2
are positive. All rational constants and both expressions must be evaluated
outward, followed by maximum of upper endpoints. No rounded exponential
library call or fitted factor is required.

The first admitted coding target may retain the old valid recurrence to
isolate the certificate-kernel change. C5 is an independently switchable,
proved tightening. For the F1 atlas, the same theorem applies with lambda=1/2,
delta=Df+Dg/2, root transfer 2, and its own domain/switch contracts. Its
residual coefficient formulas must be derived for its original equations;
the cyclic formulas cannot simply be relabeled.

## R5: alternatives and complete-work consequence

Direct interval substitution into an unstructured state box is admissible
but destroys the exact cancellations defining the small residual. A shrinking
tube or interval subdivision can recover a tighter range at additional work.
Direct fixed-degree Bernstein evaluation retains temporal correlations in
the polynomial and performs the work once per slab. For these fixed-degree
polynomial original equations it is the chosen first candidate.

A useful alternate enclosure follows from (C9). Subtract the cubic
\(L(\theta)=g_0+(g_1-g_0)(3\theta^2-2\theta^3)\). The remainder factors as
\(\theta^2(1-\theta)^2 p_2(\theta)\). The degree-six middle controls of L
are \((4g_0+g_1)/5,(g_0+g_1)/2,(g_0+4g_1)/5\); the quadratic Bernstein
controls of p2 are 15(beta2-L2),10(beta3-L3),15(beta4-L4). Thus
\(D_g\le\max(|g_0|,|g_1|)+(1/16)\max|p_{2,j}|\). This can exploit signed
endpoint information, but adds arithmetic and another enclosure route.
It is a focused optional alternative, not a reason to delay the simpler
direct certificate. No new alternative may get an unrecorded fourth retry.

### Conditional consequence for the old N=32 failures

Even the old generic Bernstein coefficient bound Df_old, if reused, can
be multiplied by 3/4 for the **same mathematical path**, because its two
endpoint slow-residual coefficients are exactly zero. Keep the old radius
recurrence and the old algebraic residual bounds. Starting from E0=0,
linearity and contraction of that recurrence give, over total duration T,

\[
 B_{\rm new}\le\tfrac34 B_{\rm old}
       +\tfrac27(1+T/7)G_{\rm old},                        \tag{C14}
\]

where B_old is an all-slab physical/root upper bound and G_old is the
all-slab maximum algebraic residual bound. To prove this, subtract 3/4 of
the old slow recurrence from the new one; the only added term per step is
h*Dg/28, whose accumulated contribution is at most T*G_old/28. Add the
remaining quarter of the algebraic residual in the 8/7 root transfer.

At T=1 the second coefficient is exactly 16/49. All nine old N=32
configurations have recorded B_old < 1.173e-8 and G_old < 1.7e-11.
These coarse bounds imply a theoretical tightened bound < 8.804e-9,
below the unchanged 1e-8 cap. This is a **conditional analytical
recertification implication of previously recorded upper bounds**. It is
not a newly run native result, and a new implementation must include its
own outward rounding envelope and validate the original-path interpretation.

This prediction is valuable because grid reduction from 64 to 32 would
reduce both candidate work and certificate work. It applies to every old
backend sharing this path contract. The full comparison must therefore
report matched-cap total time, certificate time, integration time, failed
attempt work and first passing tested grid separately. No speed ratio is
proved by the operation reduction or by (C14).

## Minimal native module contract

Use a fixed-model, opt-in module in `vigilode-coupled`; keep its public
claim distinct from generic solver certification. Reuse
`rodas5p_core::directed::Interval` and its fallible finite operations.

Suggested logical API (exact Rust names are an implementation choice):

1. `prepare_endpoint(model_epoch, endpoint_bits)` validates dimensions,
   finiteness and endpoint domain, encloses the circuit denominator and
   exact slopes, and encloses the original G at the endpoint. Its prepared
   fields are immutable/private and carry the identity used for reuse.
2. `certify_slab(left, right, exact_h, incoming_radius)` validates matching
   identities and finite nonnegative radius, builds (C5), checks the entire
   control hull, evaluates (C6),(C10),(C7),(C11), and propagates one declared
   radius recipe. Reject invalid domains or denominators; do not silently
   enlarge the problem's domain.
3. The returned certificate names the exact-slope Hermite path, original
   branch/model/domain, physical infinity norm, exact physical slab length,
   Df,Dg,Ewhole,Eend,Qwhole and validity status. A separate cap-admission
   operation requires the unchanged all-state cap and committed clock.
4. A streaming wrapper accepts a prepared right endpoint as the next left
   only for equal endpoint bits and epoch. It reports actual slope/residual
   evaluations and reuse counts.

The current dyadic clock uses t=k/N, h=1/N, N a power of two. It has no
hidden physical-time displacement. General non-dyadic or time-rescaled
clocks require the explicit PR94 clock edge; successful slab certification
does not infer that edge from matching labels.

### Arithmetic envelope and refusal rules

- Finite binary64 inputs are exact represented reals. Reject NaN, infinity,
  dimension mismatch, negative radius, h<=0 and unsupported path/epoch.
- Rational constants 1/3,1/5,1/7,1/10,1/20,6/7,8/7 are constructed by outward
  division of exactly represented integers. Dyadic constants are exact.
- Every control, coefficient, magnitude bound and radius accumulation is
  enclosed by the existing directed interval primitives. Taking the maximum
  of finite upper bounds does not need another rounding operation.
- The exact coefficient identities in C2/C3 justify zero/duplicate controls;
  they cannot be assumed for an arbitrary supplied vector field or a
  rounded-slope interpolant. There is no user-supplied identity flag.
- The rounding environment must meet the core directed-arithmetic contract:
  IEEE binary64, round-to-nearest and gradual underflow, no transformations
  that invalidate the error-free-transform arguments. Explicit `mul_add`
  used by that library is part of its proof; it is not a hidden derivative.
- Reject any failed finite arithmetic operation or denominator interval
  containing zero. Domain evidence must cover the full polynomial path.
- The certificate describes an exact mathematical curve. A floating dense
  query must either return an enclosure of this curve or charge evaluation
  and representation displacement separately. Merely returning RN Hermite
  values does not inherit the mathematical-curve cap automatically.

## R7/R9: original-equation oracle and bounded admission plan

The independent oracle interprets each endpoint double and h with exact
`Fraction.from_float`, evaluates the exact cyclic recurrence, forms the
original Hermite **power** coefficients, differentiates X and substitutes
into the original G and f. It independently converts those residuals to
Bernstein coefficients. It then checks C6/C9/C10 against those exact
coefficients and checks each native enclosure. Its use of the original
equations prevents the optimized coefficient kernel from certifying itself.
Dense sampling is diagnostic and is never the supremum oracle.

Parent-preregistered native validation should include:

- Every slab in the bounded n=4,8,16 and N=16,32,64,128 fixture campaign,
  or explicitly state any lesser coverage; all three old candidate families
  should share the same certification/cap contract where feasible.
- Exact coefficient checks on consistent and deliberately inconsistent
  endpoints in the admitted box; off-manifold C4 is essential.
- The endpoint-radius forcing counterexample E=0,delta>0 if C5 is enabled,
  plus upper/lower sign cases and a maximum-principle whole-slab check.
- Finiteness/domain/denominator failures, stale endpoint epoch, changed
  endpoint bits, invalid clock, h<=0 and malformed dimension failures.
- Evidence that certified paths retain the same physical cap, branch and
  original equations; query enclosures if dense query output is exposed.
- Separate kernel and total-work measurements, same candidate endpoints
  for a certifier comparison, and matched-cap grids for end-to-end work.

The bounded coding target is **C1--C4 plus the old valid radius**, optionally
C5 as a separately named recipe after its oracle checks. The first useful
deliverable is a checked certificate kernel and endpoint reuse API. It is
not a new adaptive stiff integrator, a universal atlas, or an automatic
proof of production readiness. Remaining theoretical work is arbitrary
closures, selected singularities, stiffness-uniform accepted dynamics and
continuous-output representation accounting. Remaining engineering work is
finite API integration, immutable endpoint identity, independent validation
and measured complete certification cost. These are separate obligations.
