# D5: a strict callback boundary with explicit pointwise authority

## Result and meaning

An owned scratch buffer plus checked dispatch gives a conditional transactional
interface for a user RHS and Jacobian. It preserves the last published scientific
value when an evaluation fails, and it prevents a callback from receiving a
query outside a declared finite box. These are interface properties. A finite
matrix returned at a point does not establish that it is the derivative of the
RHS, nor an enclosure of derivatives on a solution tube. The existing native
immutable continuous model provider remains the authority for that distinct task.

## 1. Contract, state and publication

Let N>0, I=[a,b], B=product_i[l_i,u_i], with finite binary64 endpoints and
l_i<=u_i, a<=b. The query is q=(t,y) in I x B. Define the input predicate

    Q(q) = finite(t,y) and a<=t<=b and all l_i<=y_i<=u_i.

The closed box is an admitted evaluation domain; it is not automatically an
invariant region of an arbitrary ODE. It must be supplied by the model owner.
Domain ordering uses an integer key of finite binary64 bits, treating signed
zeros as numerically equal without changing the query bits. For a negative
finite value the key complements its bits; for a nonnegative value it flips
the sign bit. This orders all finite numerical values independently of FP
control modes, including subnormals. N and the output shapes
are enforced by the Rust array types, with N=0 refused at construction.

A witness W returns (model_epoch, environment_ok). The gate holds a fixed expected
epoch e. The operational contract requires W to return, to report honestly, and
the underlying model to have no unobserved concurrent mutation or epoch ABA.
The probe is a read-only observation, and the FP environment cannot change
unobserved during checked arithmetic or perform FP-state ABA.
The equality W.epoch=e is an observation under those assumptions; it is not a
lock or a proof that an arbitrary Arc/Mutex capture is an immutable snapshot.

For each callback, the sequence is:

1. Refuse an already terminal gate and a query failing Q.
2. Check W before dispatch, then charge the attempted callback counter.
3. Allocate owned full-shape scratch initialized to NaN and enter user code.
4. Check W after return/handled unwind; classify status and every output entry.
5. Return the owned finite output only on success.

The pair sequence keeps the RHS result private, performs the analogous Jacobian
evaluation, and returns Prepared(t,y,e,f,J) once both have succeeded. Caller
publication is the assignment of this returned value. A failed pair returns no
Prepared value. RHS/J counters are monotonic attempted work and pair count records
successfully returned Prepared values. It does not observe caller assignment
or solver acceptance. Checked increments refuse overflow.

## 2. Constructive interface lemmas

### L1 — query exclusion

If Q(q) is false, callback entry count for that request is zero. The only program
edge entering user RHS/J code follows Q and witness admission. A false predicate
returns before that edge. A mutable callback cannot alter q through its shared
array reference in safe Rust; aliasing/interior mutation of unrelated captures
does not change this statement about the supplied query value.

For a centered time stencil, both q_minus and q_plus are preflighted before the
first RHS entry. Thus an inadmissible second point does not consume the first
evaluation. This is stronger than discovering the second point's error after
evaluating the first. Per-call witness checks still occur at each actual entry.

### L2 — owned-output atomicity

Let P be a caller's previously published value and U the fresh scratch. The
callback can access U, not a mutable reference to P. Before the sole successful
return edge, all assignments affect U, gate counters or terminal status. If any
check fails, U is dropped and no Prepared value is returned; P is unchanged.
On the successful edge, the caller may assign the entire owned Prepared value.
This proves a strong guarantee for this owned output, not for arbitrary external
effects performed by the callback or a panic hook. Such effects remain visible.

The proof assumes returning destructors and a handled Rust unwind. Process abort,
unhandled foreign exceptions, allocation failure and a nonreturning callback
are outside this guarantee. In particular catch_unwind does not turn every
possible process failure into a Result.

### L3 — incomplete full-write rejection

Each of the N RHS or N^2 Jacobian slots begins as NaN. If any slot is never
written, it remains non-finite and the all-finite predicate fails. Therefore a
successful full-write result cannot contain a never-written slot. The converse
is not an accuracy claim: the callback can write an arbitrary finite wrong value.
Writing a NaN explicitly also fails, as required by this strict contract.

This mechanism deliberately differs from native previous-matrix/fixed-sparsity
Jacobian storage. A sparse callback that relies on untouched zeros must explicitly
implement the new full-write interface before it can use this gate. No native
callback is silently reinterpreted and no existing sparse storage is poisoned.

### L4 — fail-stop and accounting

Charge-before-entry counts every actual user callback attempt, including Err,
non-finite output and handled panic. Preflight refusals are not callback work.
After a fatal callback, handled panic, witness violation, non-finite output or
counter overflow, terminal status blocks later dispatch. A recoverable callback
error does not publish output and permits the caller's explicit policy to retry.
There is no loop inside the gate. A pair dispatches at most one RHS and one J;
a time stencil dispatches at most two RHS calls.

Use the existing Session without reconstructing it. Every successful begin
consumes one of its M attempts. A failed D5 evaluation resolves the same lease
through abort_stage. A recoverable abort preserves Accepted(t,y,epoch,last_norm)
and reduces proposed h; a fatal abort halts that Session. Hence, conditional on
returning gate/probe/callback/destructor calls, the outer attempts remain <=M.
Internal RHS/J counts are separately measured; they are not the number of
accepted steps and do not imply a wall-time speedup or unconditional termination.

## 3. Manufactured transfer/loss model and its exact derivatives

Let x,y be nonnegative nondimensional species abundances and time be scaled by
a fixed reference time. With nonnegative lambda,mu,k0,eta, define

    k(t)=k0+eta*t^2,
    f1(t,x,y)=-lambda*x+k(t)*y^2,
    f2(t,x,y)= lambda*x-mu*y-k(t)*y^2.

The transfer terms cancel exactly: f1+f2=-mu*y. At x=0, f1=k(t)y^2>=0;
at y=0, f2=lambda*x>=0. Local polynomial regularity and these inward boundary
directions give nonnegativity for the exact solution. If x(0)+y(0)<=1, then
the exact solution remains in the unit simplex because its total decreases.
This statement concerns the exact manufactured ODE, not a numerical trajectory
or the larger box's invariance. D5 executes no new trajectory campaign.

The derivative matrix and time derivative are

    J = [[-lambda, 2*k(t)*y],
         [ lambda, -mu-2*k(t)*y]],
    partial_t f = [2*eta*t*y^2, -2*eta*t*y^2].

For a state increment d=(dx,dy), direct expansion gives

    f(t,y_state+d)-f(t,y_state)-J*d
      = [k(t)*dy^2, -k(t)*dy^2].

This exact remainder verifies J as the derivative for this stated model. It
also shows what is missing from merely checking that the callback matrix is
finite. The exact identities

    det(J)=lambda*mu,
    det(I-h*J)=1+h*(lambda+mu+2*k(t)*y)+h^2*lambda*mu

provide an algebraic sign/nonsingularity check for h>=0 and nonnegative parameters.
No new Rosenbrock solver, inverse error certificate or global ODE bound is
constructed from these identities in D5.

For the frozen grid, lambda=3,mu=1,k0=2,eta=1/2 and t,x,y=i/8. Every input is a
dyadic rational. Exact Fraction evaluation of the algebraic model and derivative
is the independent oracle; it does not use a second floating implementation as
truth. The conservation residual and J*(1,-1) add independent sign/row checks.

## 4. Represented time stencil

After a successful witness check before stencil arithmetic, let
t_minus=fl(t-eps), t_plus=fl(t+eps). Admission requires eps finite and >0,
t_minus<t<t_plus, both represented points in I, and a finite positive represented
denominator Delta=fl(t_plus-t_minus). The algorithm returns

    d_t = fl((f(t_plus,y)-f(t_minus,y))/Delta).

In real arithmetic at those represented points, the model's secant is exactly

    [eta*(t_plus+t_minus)*y^2, -eta*(t_plus+t_minus)*y^2].

Its difference from partial_t f(t,y) is

    [eta*(t_plus+t_minus-2*t)*y^2, opposite].

Thus using the actual denominator avoids silently substituting an unrepresented
2*eps, but an asymmetric represented stencil can still be centered elsewhere.
The frozen 49 stencils use t=i/8 (i=1..7), eps=2^-p (p=3..9), x=1/4,y=3/4,
so the represented stencil is exactly symmetric. Remaining floating evaluation
error is compared to exact rational truth under the preregistered 64-epsilon
scaled gate. Boundary t=0 or1 and a sub-ulp step are refusal controls.

## 5. Pointwise admission, derivative correctness and dense authority

There are three distinct questions:

| Layer | What is established | Additional obligation |
|---|---|---|
| D5 boundary | Admitted query, checked returning status, complete finite owned output, witness agreement | Truthful witness, no unobserved mutation, returning execution |
| Model derivative | J=partial_y f for the stated model | Analytic/AD/independent derivative evidence with exact model and convention binding |
| Continuous certificate | Whole-interval residual/Jacobian enclosure and closed tube | Existing immutable provider, interval-domain bounds and closure evidence |

For example J_bad=J+I is a finite full-write matrix on the entire frozen grid.
It passes the interface predicate but differs from the true derivative on every
diagonal. This explicit counterexample refutes only 'finite full-write implies
correct derivative'. It is a familiar contract distinction, not a proposed
strong no-go or novel impossibility theorem. We retain its useful witness and
do not claim NG1–NG7 novelty.

Neumaier's rigorous enclosure construction requires bounds on the relevant
domain and a closed tube. Point samples of J do not supply those hypotheses.
The native generic_verified provider already encodes a separate immutable model
authority. D5 does not manufacture a new norm-only node, duplicate that provider,
or allow unbound callbacks to acquire continuous authority.

## 6. Source and code map

| Obligation | Existing/new concrete path |
|---|---|
| Per-callback domain and NaN pattern | D2 stage_safe reused as established pattern; new gateway.rs generalizes the boundary only |
| Native compatibility | native problem.rs eval_rhs_into/dense_jacobian_into/with_jacobian_into read at pinned frontier; unchanged |
| Epoch authority | native documented model_epoch promise; explicit D5 honest/no-ABA assumption |
| Accepted prefix, attempts and fatal/retry | Exact inherited admission.rs Session; new session_tests.rs calls it directly |
| FP environment | unchanged research fixture's MXCSR reader; new gateway/tests compiled with forbid unsafe |
| Full-write/atomic output | new gateway.rs owned arrays and Prepared return |
| Derivative and secant data | new probes.rs; saved JSONL checked by independent review_points.py |
| General dense acceptance | native generic_verified; unchanged UnsupportedUnboundModel boundary |

Primary sources: [Neumaier, 1993, sections3–4](https://arnold-neumaier.at/ms/ode.pdf),
[Rust catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html),
[Rust unsafety reference](https://doc.rust-lang.org/reference/unsafety.html).
The numerical toolchain is separately pinned to Rust1.94.1; the current web docs
are consulted for the stable API limitations, not reported as that compiler's
executed version. Source/PR evidence is linked in SOURCE_SURVEY_KO.md.

Status at initial writing: constructive derivation complete; new symbolic,
implementation and saved-output verification remain pending their actual logs.

## Cross-project pattern reuse established by the current survey

C21H already validates an RPC RHS/Jacobian before publishing owned values; C21I
already has cell-owned scratch, model-specific domain/provider checks and success-only
cache insertion. Those patterns are reused as evidence. D5 adds their explicit
composition for direct in-process generic strict full-write RHS/Jacobian, pair
publication and bilateral time-stencil admission. It is not the first staging or
domain guard and does not replace/replay either completed optical adapter.
See RELATED_ADDITIONAL_KO.md and evidence/REUSED_CR_PATTERN_PINS.json.
