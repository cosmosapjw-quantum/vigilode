# SYM02: native affine accepted-segment event certificate

## Result and scope

For the scalar IVP y'=-lambda*y+b with constant parameters lambda in L=[l,u], l>0, b in B, a supplied incoming absolute error E0, and a fixed physical clock, the construction below encloses the original-equation defect of a native RODAS accepted dense segment over its entire slab. It provides state-error, strict event-sign and sufficient unique-crossing certificates. Native binary64 query output has an additional pointwise error bound computed from its actual returned bits.

This is an opt-in conditional affine model provider. Source identity binds a declared equation; the caller must establish that its supplied lambda,b and E0 enclose the intended IVP. Native numerical stages are candidates, not mathematical input proofs of that premise. The original VigilODE core DAG remains7/23; central REI remains28/88.

## R1–R3: source and assumptions

Base PR101 bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087; exact prior Fraction oracle PR100 8615b365c2812a126574f579328b2056ce5b03c1. The relevant native dense and directed-arithmetic blobs are unchanged between these two commits (survey/NATIVE_BLOBS.json).

Native path: FastDenseSegments stores accepted endpoint times, states, eight transformed stage rows and three H coefficient rows. It exposes these to an opt-in scalar-only adapter. No speculative stage, rejected transaction, controller default or replay is promoted.

Model, original source, clock, unit, event identity, epoch and segment index must agree. Every scalar/interval is finite and ordered. Time t1>t0. E0>=0. The same physical clock is used in the equation, polynomial and event. [lambda]=time^-1, [b]=[y]/time, [defect]=[y]/time, [E]=[y]. No relativistic metric enters this scalar numerical lemma; the project signature remains(-,+,+,+).

## R4–R5: exact polynomial and whole-slab defect

Let theta=(t-t0)/h in[0,1], with h=real(t1)-real(t0), and d_r=sum_j real(H_rj)*real(u_j). The target continuous curve is the exact real polynomial

p(theta)=(1-theta)y0+theta*(y1+(1-theta)*(d0+theta*(d1+theta*d2))).

Its power coefficients are a=(y0,y1-y0+d0,d1-d0,d2-d1,-d2). Thus the native source's old phrase third-degree was inaccurate when d2 is nonzero. The derivative is with respect to the actual physical clock, not normalized theta:

r(theta)=p_theta'(theta)/h+lambda*p(theta)-b.

All stored binary64 values denote exact real numbers. Enclose exact products/sums d_r, all a_j, the exact h difference, 1/h and residual coefficients using existing directed Interval operations. The degree-four Bernstein coefficients are

beta_k=sum_(j=0)^k r_j * choose(k,j)/choose(4,j), k=0,...,4.

Positive basis functions sum to1. Therefore delta=max_k max(abs(beta_k.lo),abs(beta_k.hi)) bounds |r| everywhere, including all lambda,b in their declared constant boxes. Dependency loss widens the interval but preserves inclusion. Endpoints use p(0)=y0 and p(1)=y1 exactly, preserving their cancellation identities.

For e=p-y, e'=-lambda*e+r and D^+|e|<=-l|e|+delta. Hence

|e(t0+s)|<=exp(-l*s)E0+delta/l*(1-exp(-l*s))=:E(s).

A positive-arithmetic endpoint upper can use the minimum of independently valid bounds E0+h_hi*delta; max(E0,delta/l); and E0/(1+l*h_lo)+delta*min(h_hi,1/l), evaluated outward. The last bound follows exp(-z)<=1/(1+z) and (1-exp(-z))/l<=min(s,1/l). Since E(s) is monotone, Eall=max(E0,Eh) encloses the entire slab. The coarser Eall=E0+h_hi*delta is also valid. The implementation's actual formula is recorded with its tests.

## Event signs and crossing

For g=y-threshold, the state Lipschitz constant is1. Strictly positive/negative outward intervals for y-threshold certify POS/NEG; equality or overlap returns UNRESOLVED. Endpoint certificates use E0/Eh and the exact stored endpoints. A whole-slab range follows the Bernstein hull of p enlarged by Eall. Enclosing -L*(P+[-Eall,Eall])+B gives the event derivative. A strictly signed derivative gives strict monotonicity. Opposite certified endpoint signs plus that derivative imply exactly one crossing by continuity and strict monotonicity. A strict whole-slab sign, or equal certified endpoint signs with strict monotonicity, certifies no crossing. A grazing/equality case remains unresolved.

## Native query rounding

For any finite represented query t in[t0,t1], enclose exact theta=(real(t)-real(t0))/h and exact p(theta) in I=[Ilo,Ihi]. Let q be the actual finite binary64 dense output. Set

rho=max_up(abs(q-Ilo),abs(Ihi-q)).

Then |q-p(theta)|<=rho for any q, including q outside I. Thus |q-y(t)|<=rho+E_t, with E_t=E0 at t0, Eh at t1, and Eall inside. This encloses actual query normalization, H accumulation, polynomial evaluation and endpoint-copy discrepancies pointwise. It does not infer a whole-query uniform rounding constant from sampled values. Query APIs require the current binding and reject stale source/clock/event/epoch; chained error carry additionally requires adjacent times, endpoint bits and segment index.

## Arithmetic environment

The reused directed arithmetic assumes IEEE754 binary64 nearest rounding and gradual underflow. The safe-only production crate preserves forbid(unsafe_code). Per-call black_box arithmetic probes discriminate the common rounding modes and flushed subnormals; they are observed arithmetic checks, not direct register inspection. Correct IEEE operations and an unchanged FP environment within a synchronous call remain platform premises. Overflow/nonfinite intervals refuse a certificate.

## Reuse and literature

The Dini/logarithmic-norm mechanism is established methodology; this unit's contribution is the concrete native accepted-segment/source/clock/query bridge and its adversarial validation. Neumaier, Global rigorous and realistic bounds for dissipative differential equations, Computing52(1994)315–336, original manuscript https://arnold-neumaier.at/ms/ode.pdf. Rust f64 directed-neighbor semantics: https://doc.rust-lang.org/stable/core/primitive.f64.html. SciSpace and Exa retrievals were used to locate primary rigorous-ODE literature; Wolfram independently expands the quartic and verifies the coefficient identity to0.

## Admission obligations

No new strong no-go theorem is proposed. NG1–NG7 therefore have no candidate to admit. Existing source-dependent BASS30 capsule, full nonlinear Jacobian/lognorm, source/clock uncertainty and production release qualifications remain separate project obligations. SYM02 completion requires implementation, actual focused native and independent exact arithmetic tests, negative controls and one independent review pass. Native/Rust tests are not replaced by structural metadata checks.
