# Multiplicity-preserving observables: a bounded production route

Status: **theory completed; bounded native implementation recommended for parent
admission; no runtime edit or new numerical campaign by this theory worker**.

Base: PR96 `13f3298bd0213f121e07385903f9508c435ad6a1`, retaining PR94's
algebra/physical-error separation. Scope: R2/R3/R5/R7/R9 of
physmath-research-loop 1.3.0. The attachment is the current
`RVJ_NAG_PARALLEL_20261028` intake, with source identities in
`../intake/INPUTS.json`; it is not treated as new native VigilODE evidence.

## 1. Decision and evidence reuse

An explicitly requested sum of a polynomial over **all complex roots with
algebraic multiplicity** can be evaluated without finding any roots. For the
attached monic quartic, this remains a polynomial operation at its triple-root
discriminant. This gives a useful, small, derivative-free and Newton-free
provider. It does not solve a selected real closure, and must not be installed
as a fallback for a branch-selected closure request.

The immediate implementation should expose this observable identity, a
finite-precision enclosure, and work counts. A manufactured coupled ODE can
then exercise the observable inside a native solver. Passing that experiment
would establish this component and example, not general production readiness.

| Input | What is reusable | What is not implied |
|---|---|---|
| F6 derivation, source and symbolic results | Exact monic quartic; length-four quotient; Newton sums; polynomial-only query; rational reconstruction certificate | A selected real branch, a general multivariate flat family, native interval certification or a native speedup |
| F1 charts and review | Exact original residual; box-specific branch identity; chart-switch reconstruction cost | Coverage of a selected singular root: both charts share `4uv-1 != 0` |
| F4/F5 review and F5 derivation | Local singular structure; horizontal omitted contribution; multiplicities and stabilizer corrections | Complete global `116/58` or `120/60` totals; complete classification of `D=0` |
| PR94 algebra contract | A polynomial/quotient representation can replace repeated operations if it preserves the original target and all defect terms | That a trace of roots equals a raw-U stage solve, a selected algebraic branch, or a physical flow certificate |
| PR94 physical error contract | Separate algebraic/evaluation, discretization, clock, representation and interface errors | That an exact trace identity by itself controls a time trajectory |

Attachment evidence is recorded in `../intake/rvj/f6_trace/derivation.md`,
`trace_client.py`, `symbolic_results.json`, `results.json`, and
`../intake/rvj/reviews/INDEPENDENT_REVIEW.md`. Its 17 symbolic checks and old
timings are historical input; none were replayed here. In particular its
integration-only timing omits the dominant rational certificate work. A native
comparison must include integration, certification, preparation and failures.

Primary literature provides a useful independent algebraic connection:
Bostan, Flajolet, Salvy and Schost, *Fast Computation With Two Algebraic
Numbers*, INRIA RR4579 (2002), §2, Lemma 1 and §4.1, Lemma 6, relates power
sums to coefficient series and multiplication traces to multiplicity-weighted
root observables. The relevant sections, not only the abstract, were opened:
<https://algo.inria.fr/flajolet/Publications/BoFlSaSc02.pdf>.
The proofs and finite-precision contracts below are stated explicitly for the
present target rather than importing its large-degree algorithms.

## 2. R3: exact target and monic quotient theorem

### Theorem T1 — fixed-rank trace through a discriminant

Let

\[
 P(z)=z^d+c_1z^{d-1}+\cdots+c_d,\qquad d\geq1,
\]

be monic over a characteristic-zero coefficient field, and let `h` be a
polynomial. In the quotient `A = C[z]/(P)`, multiplication by `h` has trace

\[
 \operatorname{Tr}_A h=\sum_{r:P(r)=0}m_r h(r),
 \qquad \sum_r m_r=d.                                      \tag{T1}
\]

There is no square-free hypothesis. The quotient has basis
`1,z,...,z^(d-1)` for every coefficient specialization. If the coefficients
and `h` depend polynomially on parameters, the trace does too, including at
discriminants.

**Proof.** Monic division gives a unique remainder of degree below `d`, so
the listed basis is free even before specializing parameters. Over `C`, use
the Chinese remainder decomposition into `C[z]/((z-r)^m_r)`. In the local
basis `1,(z-r),...,(z-r)^(m_r-1)`, multiplication by `h` is triangular with
diagonal `h(r)`; the nilpotent terms have zero diagonal. Its trace is
`m_r h(r)`. Add the local traces. Monic polynomial reduction uses additions
and multiplications of coefficient polynomials, proving polynomial parameter
dependence. This proof introduces no inverse of `P'(r)` and no root-label
continuation. The companion matrix is a proof device, not a runtime need.

For `S_k = Tr(z^k)`, `S_0=d`, Newton identities give

\[
 S_k=-k c_k-\sum_{j=1}^{k-1}c_jS_{k-j}\quad(1\leq k\leq d),
 \qquad
 S_k=-\sum_{j=1}^{d}c_jS_{k-j}\quad(k>d).                   \tag{T2}
\]

They follow by expanding the formal identity
`P'(z)/P(z) = sum_r m_r/(z-r)` at infinity and equating coefficients.
Alternatively, after the first `d` sums, multiply `P(r)=0` by `r^(k-d)`
and sum with multiplicities. Thus a matrix-free coefficient recurrence is
an exact implementation route, not an approximation to root coordinates.

### Corollary T1a — observable-specific compilation

Reduce `h` modulo `P` once, obtaining `h = QP+r`, `deg(r)<d`. Then
`Tr(h)=sum_(j<d) r_j S_j`. Several observables share the first `d` moments.
For a parameterized fixed model, reduction can be performed offline over its
exact parameter ring; the runtime only evaluates the resulting scalar
circuits. Repeated evaluation of all high moments is unnecessary when the
requested observable is fixed. Symbolic preparation, plan storage and any
coefficient conversion still belong in work and error accounting.

## 3. R3/R5: the actual quartic and exact counterexamples

The attachment's original equations are

\[
 q_1^2+q_2=a,\qquad q_2^2+q_1=b.
\]

Substitution `z=q_1`, `q_2=a-z^2` gives, without division,

\[
 P_{a,b}(z)=z^4-2az^2+z+a^2-b.                             \tag{T3}
\]

Every root with its local length is represented; there is no eliminated
denominator or saturation condition. Its Newton sums are

\[
 (S_0,S_1,S_2,S_3,S_4,S_5,S_6)
 =(4,0,4a,-3,4a^2+4b,-10a,4a^3+12ab+3),                  \tag{T4}
\]

and for `k>4`, `S_k=2a S_(k-2)-S_(k-3)-(a^2-b)S_(k-4)`.
The fixed-basis trace functional is therefore

\[
 \operatorname{Tr}(r_0+r_1z+r_2z^2+r_3z^3)
 =4r_0+4a r_2-3r_3.                                     \tag{T5}
\]

Useful exact invariant observables are

\[
 \operatorname{Tr}q_1=\operatorname{Tr}q_2=0,\quad
 \operatorname{Tr}q_1^2=4a,\quad
 \operatorname{Tr}q_2^2=4b,\quad
 \operatorname{Tr}(q_1q_2)=3.                             \tag{T6}
\]

For example, the last identity is `a S_1-S_3=3`; it is a constant circuit,
not four root multiplications. Also `Tr(q_1^2 q_2^2)=4ab+3` follows by
expanding `z^2(a-z^2)^2` and applying T4. Such identities may be compiled
only for the declared original equations and exact polynomial query. They
are not identities for individual branches.

### Exact triple root, with multiplicity retained

At `a=b=3/4`,

\[
 P=(z-1/2)^3(z+3/2),\qquad
 S_k=3(1/2)^k+(-3/2)^k.                                  \tag{T7}
\]

`S_0=4`, `S_1=0`, `S_2=3` and `S_3=-3`. Replacing the quotient by its
radical changes `S_0` to `2` and `S_1` to `-1`; even an apparently harmless
deduplication changes the requested model. The constant T6 also changes from
`3` to `5/2` if the two distinct points are counted once.

### Invalid branch-only query, including a regular branch

At `a=b=2`,

\[
 P=(z-1)(z+2)(z^2-z-1).
\]

The branch through `(q_1,q_2)=(1,1)` is regular because `4q_1q_2-1=3`.
Its `q_1` observable is **1**, while `Tr(q_1)=0` and `Tr(q_1)/4=0`.
Therefore a request to continue this specified real branch cannot be
implemented by a trace or trace average even away from a singularity. If
that closure supplies an ODE velocity, substituting the trace already changes
the velocity by 1 at the initial state; the ensuing physical error is not
roundoff or an integration tolerance issue.

At the triple point the selected positive value `q_1=1/2` also differs from
the all-root trace. This is a second semantic control, not the sole reason
for refusal. An all-real-root filter is likewise a different query: at
`a=b=-1/4`,

\[
 P=(z+1/2)^2(z^2-z+5/4).
\]

The real-only multiplicity-weighted sum of `q_1^2` is `1/2`, whereas the
all-complex-root value is `S_2=-1`. The complex conjugate pair contributes
to the real trace and must not be discarded.

These examples reject a particular substitution between target types. They
do not establish a universal impossibility for branch-certified homotopy,
regular implicit charts or every derivative-free solver.

## 4. R7: representation and arithmetic enclosures

### Theorem T2 — coefficient-to-observable error without root separation

Let the actual monic coefficients be `c_j`, the represented coefficients
`c_hat_j`, with `|c_hat_j-c_j|<=eta_j`. Let represented Newton sums satisfy
their coefficient recurrence up to a measured or enclosed arithmetic residual
`epsilon_k`. Suppose `M_k>=|S_k|`. Then `E_0=0` and

\[
 E_k\leq k\eta_k+
 \sum_{j=1}^{k-1}(|\widehat c_j|E_{k-j}+\eta_jM_{k-j})
 +\epsilon_k\quad(1\leq k\leq d),                          \tag{T8}
\]

\[
 E_k\leq\sum_{j=1}^{d}(|\widehat c_j|E_{k-j}
                         +\eta_jM_{k-j})+\epsilon_k
 \quad(k>d)                                               \tag{T9}
\]

bound `|S_hat_k-S_k|`. A root-free a priori choice is
`M_k=d R^k`, `R=1+max_j |c_j|`, using an upper enclosure of the maximum.
If `h=sum_l h_l z^l`, with represented coefficients having errors `zeta_l`,
and its final arithmetic residual is at most `epsilon_T`, then

\[
 |\widehat T-T|\leq
 \sum_l(|\widehat h_l|E_l+\zeta_lM_l)+\epsilon_T.            \tag{T10}
\]

**Proof.** In the recurrence difference use
`c_hat_j S_hat-c_j S = c_hat_j(S_hat-S)+(c_hat_j-c_j)S`,
then take absolute values. The same decomposition proves T10. For the root
bound, if `r=|z|>1+C`, `C=max_j |c_j|`, then
`sum_j |c_j| r^(d-j) < C r^d/(r-1) < r^d`, which contradicts `P(z)=0`.
No root-distance or `P'(r)` appears. These bounds can be conservative for
high-degree moments; an absolute observable error remains meaningful when
relative error near a zero trace does not.

**Preferred small native realization.** Evaluate T2 or a compiled T4/T5
circuit using the existing `rodas5p-core::directed::Interval`. Inductively,
outward addition and multiplication enclose every exact recurrence value.
This avoids estimating a hidden rounding residual and covers cancellation.
Keep scalar candidate evaluation separate from its enclosing interval.
The enclosure must include non-dyadic observable coefficient conversion;
`1/3` represented by a point containing only its rounded binary64 value is
not an enclosure of the exact rational `1/3`.

Target inputs must be declared. A binary64 state interpreted as an exact
dyadic has no additional representation error relative to that exact input.
If it approximates an externally specified real parameter, provide the
parameter interval once. Correlated coefficients `-2a` and `a^2-b` must be
generated from that declared parameter enclosure or bounded explicitly;
changing them to unrelated point approximations changes the polynomial.
Over-enclosing their correlation is safe but can cost admission.

For the quartic and a segment contained in `D=[1/2,1]^2`, the mean-value
theorem gives exact simple representation constants:

\[
 |\Delta S_2|\leq4e_a,\quad
 |\Delta S_4|\leq8e_a+4e_b,\quad
 |\Delta S_6|\leq24e_a+12e_b.                              \tag{T11}
\]

The maxima are attained by the corresponding partial derivatives at domain
endpoints; these are proved constants, not fitted multipliers. T11 concerns
input representation only; it does not also certify arithmetic evaluation.

**Arithmetic assumptions and refusals.** Use validated finite intervals and
the existing directed-arithmetic execution assumptions. Refuse non-finite
inputs, overflowed enclosure arithmetic, unsupported query semantics and
resource/degree overflow. Do not turn an overflow or interval containing an
undefined operation into a successful point value. No normalization by an
unproved nonzero leading coefficient is implicit: the generic API receives
a monic polynomial. Any degree conversion must be exact or outward enclosed;
an unchecked integer-to-float cast is not a proof. Resource caps govern
work, not mathematical validity. FTZ/DAZ or compiler settings invalidating
the inherited directed arithmetic require their own environment gate.

## 5. R3/R7: shared nonlinear work in the actual F6 client

The historical F6 vector field is

\[
 G_1=-a+S_2/32+S_4/128+S_6/512,\qquad
 G_2=-b+S_2/64+S_4/64+S_6/256.
\]

Substituting T4 gives an exact shared circuit:

\[
 N(a,b)=\frac{4a^3+16a^2+12ab+16b+3}{512},
 \qquad G_1=-\frac78 a+N,\qquad G_2=-b+\frac1{16}a+2N.     \tag{T12}
\]

One factorization is
`N = (4*a*(a*a+4*a+3*b)+16*b+3)/512`. Another computes
`U=a*a+b`, `V=a*(a*a+3*b)` and `N=U/32+V/128+3/512`.
Both are exact; finite arithmetic/cost decides which circuit to use. Do not
claim that algebraic operation count alone determines wall time. The field
also satisfies

\[
 G_2-2G_1=\frac{29}{16}a-b.                               \tag{T13}
\]

This identity is an independent original-target control and can preserve
shared-error correlation. It is not a conserved quantity of every trajectory.
When the sole nonlinear circuit has evaluation error `e_N`, its contribution
to the vector field is `(e_N,2e_N)`, exactly. In the original infinity norm
it contributes at most `2|e_N|`, before errors in the surrounding linear
operations. Independently bounding the same shared error three times would
waste a real cancellation opportunity.

On `D`,

\[
 |\Delta N|\leq\frac7{64}e_a+\frac7{128}e_b.               \tag{T14}
\]

Indeed `N_a=a/16+(3a^2+3b)/128 <=7/64` and
`N_b=1/32+3a/128 <=7/128`. These derivative expressions are used to prove
model constants; the native RHS need not form a Jacobian or invoke AD/JVP.

This is a local output-compilation analogue of PR94's quotient-algebra
strategy: repeated work disappears because the requested output is known.
It does not inherit PR94's raw-U stage inverse certificate or change stage
semantics. A useful polyalgorithm dispatch is: declared invariant circuit,
declared compiled polynomial circuit, or general monic Newton recurrence.
All three must implement the **same all-root query**. Branch selection is a
different request type, not a slower backend of this dispatch.

For fixed degree, independent states/batches may run in parallel. T12 removes
the need for a long degree recurrence for this particular observable; it
does not prove that an arbitrary sequential recurrence or adaptive ODE time
loop has become parallel. Generic high-degree power projection is a separate
future route with its own preparation, arithmetic and memory tradeoffs.

## 6. R7: actual coupled path and physical ODE error

Use the attachment's exact controlled target, without silently substituting
a branch-only physical model:

\[
 p(t)=\frac34+\frac t8+\frac{t^5}{64},\quad
 p'(t)=\frac18+\frac{5t^4}{64},\qquad -1\leq t\leq1,
\]

\[
 y'=G(y)+p'(t)(1,1)-G(p(t),p(t)),\quad
 y(-1)=(39/64,39/64).                                    \tag{T15}
\]

The exact solution is `(p,p)` and the exact terminal value is `(57/64,57/64)`.
Since `p'>0`, it passes through the triple point only at `t=0` and stays in
the interior of `D`. At diagonal parameters
`disc(P)=(4a-3)^3(4a+1)`, so this is an actual discriminant crossing.

### Theorem T3 — strongest constant from the declared unweighted row bound

On `D`, the off-diagonal entries of `DG` are nonnegative and the diagonals
are negative. The row sums increase in `a,b`, reaching at `(1,1)`

\[
 -91/128,\qquad -39/64.
\]

Consequently

\[
 \mu_\infty(DG)\leq-\lambda,\qquad\lambda=39/64.           \tag{T16}
\]

The time-only forcing in T15 does not change this bound. It is sharper than
the historical F6 implementation's valid but unused-slack choice `lambda=1/2`.
The constant is sharp for this row-sum proof/domain, since its second row
attains equality at `(1,1)`.

**Proof detail.** At `(1,1)`,
`DG=[[-49/64,7/128],[9/32,-57/64]]`; its two row sums are precisely those
above. Direct differentiation of T12 shows the required componentwise
monotonicity, signs and off-diagonal positivity on the complete box. For a
convex domain, integrating `DG` on the segment between two states yields
the same one-sided infinity-norm bound. This proves trajectory contraction
without calculating a Jacobian during integration.

An optional weighted-infinity analysis balances the two endpoint row sums
with `w=(1,(2*sqrt(79)-8)/7)`, giving
`lambda_w=(53-sqrt(79))/64`. This follows from the Perron eigenvector of
the displayed Metzler matrix; it is an exact algebraic constant. The
physical infinity-norm conversion introduces `max(w)/min(w)`, so improved
weighted decay does not automatically improve the requested physical bound.
The first native implementation should use T16 and avoid that extra state
and conversion complexity until a measured need exists.

### Theorem T4 — complete residual-to-path effect

Let `y_tilde` be an absolutely continuous declared candidate on one slab,
with incoming original-norm error at most `E`, original-field defect

\[
 \|\widetilde y'(t)-F(t,\widetilde y(t))\|_\infty\leq\delta
\]

on the entire slab, and a proved tube contained in `D`. Then, for
`0<=s<=h`,

\[
 \|y(t_i+s)-\widetilde y(t_i+s)\|_\infty
 \leq e^{-\lambda s}E+
          \frac{1-e^{-\lambda s}}{\lambda}\delta.           \tag{T17}
\]

**Proof.** The upper Dini derivative of the norm of the error is at most
`-lambda*error+delta` by T16. Integrating this scalar comparison equation
gives T17. A strict candidate-to-boundary margin greater than a proved
whole-slab bound gives a first-exit argument, so the assumption about the
true path is not circular.

If a computed defect uses an approximate vector field with a verified
uniform value error `eta_F`, add `eta_F` to that defect budget. This includes
trace arithmetic and actual coefficient representation at the candidate
input. If interval evaluation of the original `F` already encloses those
terms, do not add them again. State discretization error propagated by T17
must not also be charged as a new coefficient representation error at every
step. Clock errors, candidate derivative/reconstruction errors, switches and
interfaces remain distinct leaves as in PR94. A node comparison against
`p(t)` is useful validation but does not replace the whole-slab defect and
tube proof.

### T4a — sharper exponential-free propagation with no ambiguous decision

For `x=lambda*h>=0`, define the exact rational bounds

\[
 q_L=\max\{0,(1-x/2)/(1+x/2)\},\qquad
 q_U=\frac1{1+x+x^2/2}.                                  \tag{T18}
\]

Then `0<=q_L<=exp(-x)<=q_U<=1`. The upper bound follows from
`exp(x)>=1+x+x^2/2`. For `0<=x<2`,
`log((1+x/2)/(1-x/2))=2*atanh(x/2)>=x`; for `x>=2`, zero is a valid
lower bound. In `0<=x<=2`, cross multiplication also gives
`q_L<=q_U` because the leftover term is `x^3/4`.

Let `D_delta=delta/lambda`. A universally valid endpoint bound is

\[
 E_+=\max\{q_LE+(1-q_L)D_\delta,
           q_UE+(1-q_U)D_\delta\}.                       \tag{T19}
\]

The exact endpoint comparison is affine in `q=exp(-x)`; its maximum on
`[q_L,q_U]` occurs at an endpoint. Thus T19 avoids a fragile floating-point
test of `E` against `delta/lambda`. If using the equivalent sign branch,
choose `q_U` only when an outward comparison proves `E>=delta/lambda`,
choose `q_L` only when it proves the reverse, and use the maximum when the
comparison overlaps. Evaluate all bounds outward, clamp `q_L` below by 0
and `q_U` above by 1 only because these inequalities have been proved, and
enclose the final maximum upward. Here `lambda=39/64` is exactly dyadic.

The scalar comparison trajectory is monotone, so
`E_whole=max(E,E_+)` is valid on the whole slab. One may additionally take
the minimum with the independently valid bound `E+h*delta`; every operand
must already be an upper bound. A tube margin test still applies.

**Rejected shortcut.** `(E+h*delta)/(1+lambda*h)` is not a universal upper
bound. At `E=0`, `lambda*h=1`, it equals `delta/(2*lambda)`, below the exact
`(1-exp(-1))*delta/lambda`. It is valid in a different monotonicity regime,
which must be proved before use. The old F6 rule
`E/(1+lambda*h)+h*delta` is conservative and valid; T19 is a prospective
replacement with higher accuracy per scalar propagation, not a measured
native improvement yet.

## 7. F1/F4/F5: why geometry changes admission, but not quartic rank

The F1 primitive charts cover the same determinant domain. Their successful
switching cannot prove that a selected branch continues regularly through
`4uv-1=0`. F6 avoids this inverse only because its observable is a symmetric
polynomial trace. It does not cure F1's selected-branch singularity. F4's
local singularity classification likewise does not establish a generic
runtime continuation or physical closure model.

F5's original totals **116 ordered / 58 unordered are not complete**. The
current attachment constructs omitted horizontal collisions contributing
4 ordered / 2 unordered reduced points. The joint reflection/swap
stabilizer matters: multiplying each double-quotient point by four is wrong
on this stratum. The resulting **120 / 60 is conditional** on the retained
nonhorizontal atlas being complete and on other chart exclusions; it is
not a promoted all-chart global count.

The additional vertical transversality factor is

\[
 D=4\eta^3p+27\eta^2q+27pq.
\]

At `eta=1,p=-1/2,q=4/27`, the review records nonzero carrier regularity
quantities but `D=0`, and a reduced local equation with leading term
`(2080/729)*a^3`; the local quotient length is 3. Regular carrier membership
therefore does not imply reduced incidence. This is one proved local
stratum, not the complete `D=0` classification.

**Admission consequence.** T3 is monic of degree four and explicitly
isomorphic to the original two-equation quotient. Its rank-four proof is
unaffected by these unrelated global counts. For a future multivariate
trace provider, however, a claimed rank `116`, `120`, `58` or `60` cannot be
used as a substitute for a proved finite free algebra/basis on the admitted
parameter domain. Specify the full ideal, any localization/saturation,
the basis or flatness proof, local algebra lengths, and exactly which
group quotient defines the observable. Radicalizing, dropping charts,
deduplicating roots or applying an unproved orbit factor changes the trace.
No such generalized provider is admitted by this document.

## 8. R9: native API, validation and stop conditions

Read-only tracing located the existing outward arithmetic in
`crates/rodas5p-core/src/directed.rs`. No existing generic algebraic root
trace provider was identified in the current core API; the coupled solver's
iteration `TracePoint` is unrelated. A bounded new module can reuse
`Interval` and `CoreResult` without a generic matrix, Jacobian or root engine.

Recommended first surface:

```text
AllComplexRootsWithAlgebraicMultiplicity   // explicit request semantics
QuarticParameters { a: Interval, b: Interval }
QuarticPolynomialObservable / fixed monomial powers
TraceEvaluation { candidate, enclosure, work }
quartic_power_sums(parameters, requested_degree, caller_work_cap)
quartic_observable(parameters, exact_or_enclosed_coefficients, work_cap)
quartic_trace_field(parameters)             // T12, this declared model only
```

Names are a proposal, not an assertion that this API already exists. If only
fixed observables T4/T12 are implemented, reject unsupported queries instead
of adding a hidden root-enumeration fallback. Zero-cost invariants may have
constant enclosures. A generic monic recurrence can follow if this surface
would otherwise force users to encode the same proven operation repeatedly.
An all-root field evaluation can be supplied as an ordinary native RHS;
the stage solver's request, original residual and physical error contracts
remain in force.

Plan/query identity should bind original equations or provider identity,
observable coefficients, multiplicity/complex-root semantics, parameter
interpretation, norm when returning a physical certificate, model epoch and
arithmetic policy. Do not accept a cache entry simply because dimensions
match. Work counters should cover polynomial evaluation, outward arithmetic,
scalar preparation, refused attempts and fallback; root/JVP/AD counts remain
zero only if no such operations actually occurred.

### One bounded prospective validation set after parent admission

1. Exact-rational quotient/companion trace oracle versus T4/T5/T6 at ordinary,
   triple and double-complex parameters; include repeated roots with correct
   multiplicities and the regular branch-only counterexample at `(2,2)`.
2. Outward-enclosure tests using independent exact rational values of the
   represented binary64 inputs, including cancellation, endpoint boxes,
   non-dyadic coefficient enclosure, subnormal behavior under the inherited
   arithmetic policy, overflow and malformed input refusal. Do not validate
   an interval by comparing only with another binary64 rearrangement.
3. Original formula versus shared T12 circuit and exact identity T13; report
   any wider enclosure or preparation cost along with operation savings.
4. Native T15 integration compared with the exact `(p,p)` on the declared
   physical clock, including `t=0`. A physical-certificate claim additionally
   requires actual whole-slab residual, tube and rounding evidence; otherwise
   label the result node/reference validation only.
5. T19 against independent high-precision exponential comparison for both
   monotonicity regimes, exact equality and overlapping outward threshold
   intervals, plus the rejected-shortcut counterexample. Testing supports
   the proof; it does not replace the full-domain inequalities.
6. Matched-target timing: old native direct moment evaluation, compiled
   circuit and interval certificate costs separately and combined. If a
   root baseline is included, use the identical all-root observable and
   full complex multiplicity. Compare original physical error, not only
   requested tolerances. Profile candidate, certificate and orchestration
   separately so adapter/switcher costs are not called integrator costs.

No old expensive campaigns are requested. This research task has used
**zero new native correction runs**. Parent may register one bounded
implementation campaign and at most three reasoned correction loops under
the existing task budget; every failed variant remains attached to this
same task. Stop when this component is verified or a concrete remaining
gate is documented. The code change must not claim to close Q-API, actual
source model coverage, arbitrary nonnormal polynomial action, selected
singular continuation or whole-production admission.

## 9. Remaining theoretical questions, separated from implementation work

**Implementation-ready:** T1/T2 coefficient trace, T4 fixed moments, exact
invariant compilation, T12 shared field circuit, outward parameter/arithmetic
enclosure, and T16/T19 controlled-example propagation. These require careful
native engineering and measurement, not a new root-existence theorem.

**Requires further theory for broader scope:** a finite free multivariate
quotient for the actual full ideal and all allowed parameters; local length
and group-action corrections on all incidence strata; model-specific
selected-real-branch continuation at singular points; and physical closure
semantics when only a subset of roots has meaning. Rational observables
could use an inverse modulo `P` only after proving the denominator is a unit
and bounding its represented inverse; this is a distinct extension with
new conditioning/refusal cases. None of these open questions can be closed
by choosing a faster polynomial basis or replaying the quartic example.

The production polyalgorithm can benefit from this narrow exact route while
retaining honest target types. Its contribution is elimination of unnecessary
root work for an admissible observable, with a complete path to arithmetic
and physical error bounds; it is not evidence that every algebraic closure
or sequential integration loop admits the same elimination.
