# SYM02 mathematical contract and independent review

Reviewer: source_contract subagent. Date: 2026-10-29. Source-bound R5/R6 contribution; no IVP or benchmark executed. Source recon report: REPORT.md. Selected unit: one auxiliary SYM02 node; VigilODE core remains 7/23 scoped; REI central remains separate 28/88.

## 1. Reused source identity

All six relevant blobs are identical at PR100 8615b365c2812a126574f579328b2056ce5b03c1 and PR101 bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087. This was confirmed through GitHub fetch_file SHA metadata, not inferred from ancestry:

| Native path | Unchanged Git blob |
|---|---|
| rodas5p-integrators/src/rodas5p_fast_dense.rs | 34a5c7d847151d82767c2ca92194b8f583503984 |
| rodas5p-integrators/src/dopri5_dense.rs | eb6a103b290f85046003afb52f75269ced5efee8 |
| rodas5p-integrators/src/outward_certificate.rs | 484c363e54091fa4537b62c491716bb8be10bf68 |
| rodas5p-core/src/directed.rs | 89e2b3122b08db1849e4e56cc94f1f58c473b735 |
| vigilode/src/events.rs | 67bbb3526d03efb5533fe01e46f791ca61747694 |
| vigilode/src/solution.rs | a889e2604f7472fd08d1e19825fd2b38311dd9ee |

The PR100 Fraction oracle is a named exact-rational test dependency. Its general theorem 2B gives the logical event contract. It is not native-rounding evidence.

## 2. Exact target curve from stored RODAS data

Consider one scalar accepted FastDenseSegments segment with finite stored binary64 endpoints t0<t1, stored y0,y1, eight stored transformed stages u_j and coefficient snapshot H_rj (r=0,1,2). Interpret each stored binary64 bit pattern as an exact real number. Let

h=t1-t0 > 0,
d_r=sum_{j=0}^7 H_rj u_j,

where this definition is exact real arithmetic. The mathematical target curve in normalized theta=(t-t0)/h is

p(theta)=(1-theta)y0+theta*(y1+(1-theta)*(d0+theta*(d1+theta*d2))).

Expanding gives a quartic:

a0=y0,
a1=y1-y0+d0,
a2=d1-d0,
a3=d2-d1,
a4=-d2.

Thus p=sum_{k=0}^4 a_k theta^k; p(0)=y0 and p(1)=y1 exactly. The source comment saying third-degree is inconsistent with the general d2 term. Preserve all five coefficients. This is a documentation correction; changing the native evaluator is unnecessary.

Use existing directed Interval primitives to construct H_real enclosure by [t1,t1]-[t0,t0] and require lo>0. Independently enclose each exact d_r using interval point products/sums, then each a_k. The resulting interval coefficient family contains the mathematical target; its dependency overestimation is safe. It is not necessary to claim these intervals equal the rounded native d_r.

Read H_rj from the same pinned coefficient snapshot used by the accepted segment; a current/global coefficient table with a different identity is insufficient. Stored source/config/epoch bindings must attach to the capsule. This review does not prove provenance from an arbitrary string label.

## 3. Whole-slab original-equation defect

Assume one fixed physical clock and the scalar autonomous equation

y'=-lambda*y+b,

with constant but uncertain parameters lambda in L=[lambda_lo,lambda_hi], lambda_lo=kappa>0, b in B=[b_lo,b_hi]. These are parameter enclosures, not changing values chosen independently at every t. Independent interval relaxation still safely bounds every member of that constant family.

The physical-time residual is

r(theta)=p_theta'(theta)/h+lambda*p(theta)-b.

Let A_j enclose a_j and H enclose exact h. With A_5=[0,0], define interval power coefficients

R_j=(j+1)*A_{j+1}/H+L*A_j-B*1_{j=0},  j=0,...,4.

Every operation is outward. The fourth derivative term is zero, not an omitted quartic residual term. Division uses H.lo>0. Dimensions: [r]=[y]/[time], [lambda]=1/[time], [b]=[y]/[time]; theta is dimensionless. No h^j factor appears in this normalized-theta power-to-Bernstein conversion.

Degree-four Bernstein coefficient enclosures are

Beta_k=sum_{j=0}^k [C(k,j)/C(4,j)]*R_j, k=0,...,4.

Construct rational factors with outward division of small exact integers (or an exact pretabulation). They are nonnegative. Since Bernstein basis functions on [0,1] are nonnegative and sum to one,

delta=max_k max(abs(Beta_k.lo),abs(Beta_k.hi))

bounds sup over the entire slab, all admitted parameters, and enclosed dense coefficients of |r|. Sampling is not used in this continuum proof. A fixed-degree implementation should reject unsupported degree/shape rather than truncate silently.

## 4. Dini comparison and sound endpoint upper bounds

Suppose the incoming global error satisfies |p(0)-y(t0)|<=E0, with finite E0>=0. For each admitted lambda,b,

e'= -lambda*e+r,
D^+|e| <= -lambda*|e|+delta <= -kappa*|e|+delta.

Hence |e(s)|<=E(s), s=t-t0,

E(s)=E0*exp(-kappa*s)+(delta/kappa)*(1-exp(-kappa*s)).

This is a direct comparison result for the globally defined affine family; no first-exit nonlinear tube existence premise is missing in this scalar setting.

### 4A. Existing rational two-envelope bound is sound

Set z=kappa*h, F=h*delta and C=delta/kappa. For z>=0,

(1-z/2)/(1+z/2) <= exp(-z) <= 1/(1+z).

The lower inequality is trivial when z>=2; for 0<=z<2, log((1-z/2)/(1+z/2))=-2 atanh(z/2)<=-z. The upper follows exp(z)>=1+z.

If E0>=C, the coefficient of exp(-z) is nonnegative and the upper exponential bound yields
U1=(E0+F)/(1+z).
If E0<C, the lower exponential bound yields
U2=((1-z/2)E0+F)/(1+z/2).

Therefore Eh=max(U1,U2) is always sound. In fact,

U2-U1 = z*(F-z*E0) / [2*(1+z)*(1+z/2)],

so the max selects the proper sign branch in exact arithmetic. U1 alone fails for E0=0,kappa=h=delta=1. Implement both with outward intervals and take max of their upper endpoints; the upper numerator of U2 can be negative, so signed interval arithmetic is required.

The exact majorant E(s) is monotone, because E'(s)=exp(-kappa*s)*(delta-kappa*E0). Thus Eall=max(E0,Eh) bounds the entire slab even if Eh is only an upper bound.

### 4B. Simpler always-sound positive arithmetic option

Independently,

exp(-z)<=1/(1+z),
1-exp(-z)<=min(z,1),

imply

Eh_simple=E0/(1+kappa*h)+delta*min(h,1/kappa).

For H=[hlo,hhi], a sufficient implementation is

Eh_simple_up =
 div_up(E0, lower(1+kappa*hlo))
 +_up mul_up(delta, min(hhi, div_up(1,kappa))).

Here lower(1+kappa*hlo) must be positive and constructed downward, not nearest rounded up. Equivalently use a full outward interval for E0/(1+kappa*H), taking its upper endpoint. All other quantities are nonnegative. This simpler formula avoids exponential and the sign branch, and is tight in the large-z forcing limit, though two-envelope is often tighter for moderate z.

Also E(s)<=M=max(E0,div_up(delta,kappa)) for every s. Thus

Eh_up=min(Eh_two_up,Eh_simple_up,M)
Eall_up=max(E0,Eh_up)

is sound when all selected candidates were actually computed as sound uppers. Using only Eh_simple_up and M is a valid simpler first implementation. A failed arithmetic enclosure is a refusal; do not fabricate a finite candidate and then take min with it. A candidate may be skipped only with an explicit reason and another valid bound.

For delta=0, the simple bound retains contraction. For E0=0 and small h, it gives <=h*delta; for large h, the M cap gives <=delta/kappa. No tolerance relaxation is involved.

## 5. Event sign, derivative and crossing

Take a constant event threshold in a declared exact value or interval Theta. Mathematical event g(y)=y-Theta has Lipschitz constant 1.

Endpoint state enclosures are Y0=[y0,y0]+[-E0,E0] and Y1=[y1,y1]+[-Eh_up,Eh_up], using the exact endpoint identities rather than an independently summed uncertain polynomial. Subtract Theta outward. Lower>0 certifies POS; upper<0 certifies NEG; otherwise UNRESOLVED, including equality.

For a whole-slab state enclosure, compute Bernstein coefficient intervals of p itself and set P=[min lower, max upper]. Y=P+[-Eall_up,Eall_up]. Y-Theta strictly positive/negative certifies no crossing throughout.

The exact event derivative is g'=b-lambda*y. Compute D=B-L*Y outward. D.hi<0 proves strict decrease, D.lo>0 proves strict increase. If endpoint signs are opposite, continuity plus either strict derivative condition proves exactly one crossing for each admitted model. If endpoint signs agree and monotonicity holds, no crossing follows. Endpoint signs alone give only at least one crossing. D containing zero gives no uniqueness certificate, and a grazing event must remain unresolved unless another whole-slab sign proof settles it.

The Y tube and D interval can be conservative; no step-size or native solver tolerance should be silently changed to obtain a decision.

## 6. Actual native query is a separate rounding object

The native routine forms theta_native=RN(RN(t-t0)/RN(t1-t0)), forms each d_r with rounded multiply/add, evaluates complement=RN(1-theta_native), and evaluates the nested expression. Exact endpoints copy y0/y1; an interior theta_native==0 also returns y0. A native query may therefore differ from p at exact physical theta. Do not identify them.

At each finite represented physical query t in [t0,t1]:
1. Check membership in physical time first; forming/rounding theta is not the membership test.
2. Compute Theta_query = ([t,t]-[t0,t0])/H outward, enclosing exact physical theta.
3. Evaluate the mathematical target at Theta_query with the stored real input enclosures, using its nested source expression (or equivalent power polynomial).
4. Obtain q_native by the actual source routine or its explicitly identified imported evaluator and record the returned bits.
5. A sound observed query rounding radius is
rho_query=max(abs_up(q_native-I_math.lo),abs_up(I_math.hi-q_native)),
where I_math encloses exact target p(theta). q_native need not lie in I_math for this radius argument to be valid. The finite returned q_native plus interval subtraction proves the distance to every target value inside I_math.

This approach includes coefficient/time/evaluation rounding in the observed source query discrepancy without pretending a real-expression enclosure alone proves a small forward rounding error. It is pointwise for any admitted query; it does not manufacture a uniform all-query radius from a finite grid.

Original-state query enclosure is I_math+[-Equery,Equery]. Equivalently a conservative enclosure around q_native is [q_native,q_native]+[-(Equery+rho_query),Equery+rho_query]. Use E0 at t0, Eh at t1, Eall at interior queries. Endpoints must bit-match their source stored values; endpoint rounding radius can then be exactly zero.

If an actual FP event callback returns RN(q_native-threshold), its additional event-evaluation rounding also needs an enclosure before that returned scalar is treated as a certificate. Alternatively decide mathematical signs directly by outward interval subtraction of q_native/error/threshold.

## 7. Guards and negative controls

Protected guard fields: accepted-record identity, dense coefficient snapshot, scalar affine model identifier and lambda/b enclosure, source/clock/unit identity, incoming error identity and value, epoch/config and constant event identity. Resets/source switches/epoch changes invalidate the certificate; nonzero E0 must be carried, not replaced by zero. Event routing cannot consume a certificate from a different callback.

Existing directed arithmetic requires actual round-to-nearest and gradual-underflow semantics; check relevant platform FP control, preserve it, and refuse unsupported environments. All finite arithmetic overflow, nonfinite endpoint/coefficient, reversed interval, denominator touching zero or unsupported shape must refuse.

Critical controls:
- Nonzero d2 case catches accidental cubic truncation.
- lambda/b interval corner cases and signed b; interval coefficients and whole-slab Bernstein inclusion checked by an independent Fraction-from-bits reference.
- Huge origin/cancellation, small width, interior normalization rounding to 0 or 1, subnormal operations and overflow refusal.
- Incoming E0>0; lambda_lo>0; source/clock/unit/epoch/event mismatch; reset/source switch.
- Threshold equality, grazing D containing 0, same endpoint signs but an interior pair of crossings (for any generalized event logic).
- Deliberately omit query radius or replace exact real width by rounded width; independent reference must expose undercoverage in chosen adversarial examples.
- An actual source-query observation must not be labeled actual chemical/BASS provider admission.

## Review disposition

Mathematical R5/R6 contract above is derived and source-bound. Its exact arithmetic transfer, Dini comparison, Bernstein convex hull and event transversality are complete under the stated scalar-affine assumptions. Parent implementation and runtime outputs still require C6–C9 review. This review introduces no strong no-go theorem and no production/speedup claim.

Current-shell readback encountered one exec-server transport disconnect while comparing blobs. The comparison was completed through GitHub file metadata instead. This is a service failure of that attempted command, not a statement that Python/compiler capability is unavailable.

