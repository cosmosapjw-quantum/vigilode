# Joint defect, carrier and source certificate

## Result and source boundary

The scoped new result is a **root-free invariant-radius admission rule** that
joins VigilODE's original-equation defect, fixed-metric carrier dissipation,
quadratic remainder, representation transfer and event-sign contracts. It gives
an explicit certificate consumer for existing PR95/99 bounds without requiring
that the native solver adopt a new nonlinear or Krylov method.

The source baseline is PR101 `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`.
The inspected native implementation is
`crates/vigilode-coupled/src/trace_certificate.rs`: fixed manufactured F6,
exact-original-slope Hermite path, degree-15 residual Bernstein hull,
outward decay enclosure, carried endpoint error and strict domain tube.
Its original source target and registered grids remain unchanged.

The construction reuses:

* PR97 `production_rvj_nag_20261028/theory/CLOSURE.md`, C1/C2/C12/C13:
  original residual, algebraic-root transfer, two-envelope propagation and
  whole-slab error; endpoint-only acceptance is insufficient.
* PR98 `dynamics_symbolic_validation_20261028/theory/DYNAMICS.md`:
  signed logarithmic norm, metric transport and controller word equivalence.
* PR100 `8615b365c2812a126574f579328b2056ce5b03c1`,
  `physical_symbolic_defect_20261029/THEORY_KO.md`, Theorem 2B:
  event Lipschitz/transversality conditions on an actual solution tube.
* PR99 `e46704cbc5a1356bd8fd199de09ad26b046acfd4`,
  `orchestration_sync_20261028/authority/numerics/FROZEN_CONTRACT.md`:
  supplied-rate-box chemical remainder `Gamma rho^2`, plus a separate
  same-norm carrier mismatch `deltaJ rho`. The six-component chemical
  coefficient operator and inherited physical metric are the scoped target.
  Actual rates/clock/corridor, full operator and owner binding remain required.

No actual BASS full30/full33 field enclosure is supplied by this document.
The new oracle is an exact-rational **consumer of declared scalar bounds**;
the mathematical theorem below states exactly what those bounds must mean.

## R1–R4: contract, diagnosis and selected construction

There are two different costs. A missing original-equation tube proof is an
algorithm/numerical qualification gap; recomputing a certificate for identical
endpoints/source identity is an implementation cost. The current trace path
computes both endpoint slopes in every slab (`4N` field enclosures), while the
PR97 endpoint-reuse construction needs `2(N+1)` for this two-field slope.
That optimization preserves the same bounds if the cache key includes endpoint
bits, source/rate/clock identity, norm and arithmetic contract. It is retained
as a native follow-up under exhausted inherited PR-N05, not implemented here.

The selected independent composition task addresses a more substantial gap:
how to turn the quadratic remainder and carrier mismatch into a *self-consistent*
whole-path radius, then transport it across solver changes. The proposed rule
tests one rational polynomial inequality and reuses the existing positive
decay-envelope arithmetic. It does not require finding the roots of that
polynomial numerically.

Fix one slab `[a,b]`, `h=b-a>0`, a finite-dimensional real state space and a
fixed norm `||.||`. Let `p` be a continuously differentiable represented
candidate. Let the target field `f(t,x)` be continuous in `t`, locally Lipschitz
in `x` uniformly on compact time-state neighborhoods, and defined on an open
neighborhood of the compact tube
`K_R={(t,p(t)+v):a<=t<=b, ||v||<=R}`. The norm may be the inherited fixed
physical metric norm; a changing metric requires its own derivative term.
The initial value has `||x(a)-p(a)||<=E0`.

Assume independently verified nonnegative `Gamma`, `deltaJ`, `epsilon`,
`delta_r`, a real `lambda_c`, and a carrier `A(t)` such that, on this tube,

\[
\begin{aligned}
\mu(A(t))&\le-\lambda_c,\\
f(t,p(t)+v)-f(t,p(t))&=A(t)v+L(t)v+N(t,v),\\
\|L(t)\|&\le\delta_J,\qquad
\|N(t,v)\|\le\Gamma\|v\|^2,\\
\|p'(t)-\widetilde f(t,p(t))\|&\le\delta_r,\qquad
\|\widetilde f(t,p(t))-f(t,p(t))\|\le\epsilon.
\end{aligned}                                                    \tag{D1}
\]

Here the last line separates candidate residual from provider/source error.
If residual is already evaluated against the exact original field, set
`epsilon=0`; adding it again would double count. Conversely, a rounded or
surrogate RHS needs its actual offset bound. Set

\[
\alpha=\lambda_c-\delta_J,\qquad \delta=\delta_r+\epsilon. \tag{D2}
\]

For a quadratic chemical field `f(z)=b+Pz+(e^T z)Qz`, the exact same-time
state remainder is `(e^T v)Qv`. PR99 supplies a uniform `Gamma` for declared
rate values in a box and one fixed inherited metric. Rates may vary with time
within that box if D1 holds uniformly at each time; changing rates between the
two *state arguments at the same time* adds terms that this identity does not
cover. Missing geometry, optical-depth and clock terms are not zeros.

Units are `[alpha]=time^-1`, `[Gamma]=(state-norm*time)^-1`,
`[delta]=state-norm/time`, and `[E0]=[R]=state-norm`. All scalar inequalities
must use the same norm and clock.

## R5: proofs

### D1. Quadratic Dini majorant

With `e=x-p`, subtract the two original differential equations. The
logarithmic-norm differential inequality, the induced operator inequality and
the triangle inequality give, while in `K_R`,

\[
D^+\|e\|\le-\alpha\|e\|+\Gamma\|e\|^2+\delta
             =:q(\|e\|).                                      \tag{D3}
\]

This proof uses `mu(A)` with the signed diagonal contribution. Replacing it
by `||A||` generally loses the contraction exploited here. In particular the
carrier error `deltaJ` reduces damping; it is not a constant source offset.

### D2. Root-free closed invariant tube

Suppose

\[
0\le E_0\le R,\qquad
q(R)=\Gamma R^2-\alpha R+\delta\le0.                            \tag{D4}
\]

Then the original IVP has a unique solution throughout the slab and
`||x(t)-p(t)||<=R`. The compact tube must be contained in the field's open
validity domain; checking only `q(R)` cannot replace that premise.

For a strict inequality, the first-exit argument follows directly from D3.
For equality, merely asserting that a boundary derivative is nonpositive is
not a complete first-exit proof. One rigorous closure is as follows. Put the
equation in moving coordinates `v=x-p`, and radially project a nearby outside
point `v` onto `v_R=R v/||v||`. At the boundary the upper norm derivative is
at most `q(R)<=0`. The directional derivative of a norm uses supporting
functionals of dual norm one; these are the same at positive multiples of a
nonzero vector. Local Lipschitz continuity of the moving vector field then
gives

`D+ (||v||-R)_+ <= L (||v||-R)_+`

in a neighborhood of the compact tube, with a finite uniform `L`. Gronwall
from an initial positive-part distance of zero proves invariance. This also
covers a tangency where `q(R)=0`. Compact containment and the usual extension
theorem extend the unique local solution to `b`. For `R=0`, D4 implies
`E0=delta=0`; D1 then says `p` solves the original IVP exactly and uniqueness
gives `x=p`, without needing a positive decay constant.

For `Gamma>0`, the feasible radii exist only when `alpha>0` and
`alpha^2>=4 Gamma delta`, except the trivial zero-radius case. More exactly,
for positive radii and `alpha=0`, feasibility is possible only for
`Gamma=delta=0`. In the ordinary `Gamma>0, alpha>0` case the interval is

\[
R\in[\rho_-,\rho_+],\qquad
\rho_\pm=\frac{\alpha\pm\sqrt{\alpha^2-4\Gamma\delta}}{2\Gamma}.
                                                                  \tag{D5}
\]

The root formula characterizes feasibility; code can verify D4 using exact
rationals or directed arithmetic and a supplied radius, avoiding cancellation
in `rho_-`. `Gamma=0` reduces to `delta<=alpha R`.

### D3. Certified endpoint and whole-slab transport

Inside the invariant tube `E^2<=R E`, hence

\[
D^+E\le-\beta E+\delta,\qquad \beta=\alpha-\Gamma R.       \tag{D6}
\]

For `R>0`, D4 implies `beta R>=delta>=0`, so `beta>=0`.
When `beta>0`,

\[
E(t)\le w+(E_0-w)e^{-\beta(t-a)},\quad w=\delta/\beta.      \tag{D7}
\]

For `beta=0`, D4 forces `delta=0` and the constant bound `E(t)<=E0` follows.
The zero-radius exact case also returns zero. The following rational decay
enclosure matches the *stronger PR101* native formula, instead of replacing
it with the weaker PR97 `1/(1+z)` upper bound:

\[
q_L=\max\{0,(1-z/2)/(1+z/2)\},\quad
q_U=(1+z+z^2/2)^{-1},\quad z=\beta h.                       \tag{D8}
\]

For `z>=0`, `qL<=exp(-z)<=qU`: the upper follows from the first three
nonnegative terms of `exp(z)`; for `z<2` the lower follows from
`log((1+z/2)/(1-z/2))>=z`, whose derivative is `1/(1-z^2/4)>=1`.
For `z>=2`, zero is a valid lower bound. Define

\[
U=\max_{q\in\{q_L,q_U\}}\{qE_0+(1-q)w\},\quad
E_h=\min(R,U),\quad E_{all}=\max(E_0,E_h).                  \tag{D9}
\]

The affine expression takes its maximum over an interval at an endpoint,
regardless of the sign of `E0-w`. Both `R` and `U` are proven upper bounds,
so their minimum is safe. The comparison solution D7 is monotone toward `w`,
so the whole slab is bounded by `max(E0,E_h)`. With exact D8 arithmetic and
D4, `E0<=R` and `w<=R`, so each convex combination already lies below `R`.
The minimum is therefore redundant in the exact oracle. It becomes a safe
tightening when outward arithmetic inflates an upper beyond the independently
established invariant radius.

`E_h` is monotone in incoming `E0` on `[0,R]`, since the two affine maps have
nonnegative slopes and `min(R,.)` preserves monotonicity. Thus a prior
endpoint upper can be carried into the next valid slab by induction. No
claim that refining slabs always improves this *particular rounded bound*
is needed.

### D4. Metric/representation transport and ordered composition

Suppose a boundary map has an exact physical action `Psi` and a represented
candidate action `hatPsi`, measured in the next norm. Verify

`||Psi(x)-Psi(p)||_new <= kappa ||x-p||_old`

on the certified old tube and `||hatPsi(p)-Psi(p)||_new<=eta`. Then

\[
E_{next,0}\le\kappa E_h+\eta.                              \tag{D10}
\]

This is the triangle inequality. A pure solver-route change with an unchanged
state representation has `kappa=1,eta=0`. A coordinate/norm change may have
`kappa!=1`, and copying or recentering a dense path may have `eta>0`.
The next tube must pass D4 with this transported error. For fixed diagonal
weighted infinity norms defined by `||x||_w=max_i |x_i|/w_i` with `w_i>0`,
`kappa=max_i(w_old,i/w_new,i)` for the identity
physical map. For a physical reset, the appropriate Lipschitz bound replaces
that simple factor.

Transport and flow cannot generally be reordered. For a flow upper
`F(E)=qE+d` and transfer `T(E)=kappa E+eta`,

`F(T(E))-T(F(E))=(q-1)eta+(1-kappa)d`.

Consequently the accepted route word must retain chronological order,
source/clock epochs and norm identities. Controller word equivalence alone
does not establish D1 or physical event labels.

### D5. Event symbols and crossings from the joint tube

If the original event observable `g(t,x)` has verified state Lipschitz bound
`Lg` in the same tube, then `|g(t,x)-g(t,p)|<=Lg E(t)`. A strict lower bound
on `g(t,p)-Lg E` proves positive sign; a strict upper bound on
`g(t,p)+Lg E` proves negative sign. For a whole-slab label use certified
continuous candidate range and `Eall`. Opposite certified endpoint signs
give at least one crossing. If the original Lie derivative
`partial_t g + D_x g f` is uniformly `>=nu>0` (or `<=-nu<0`) in the tube,
there is exactly one crossing. This is IVT plus strict monotonicity.
Equality/grazing remains unresolved. In a finite ordered covering,
common boundary identity and D10 let these labels form a certified physical
itinerary. They do not constitute a phase-space approximate bisimulation.

### D6. A quantitative bottleneck diagnostic

At a fixed candidate radius define its signed admission margin

\[
M(R)=\lambda_c R-\delta_J R-\Gamma R^2-\delta_r-\epsilon.   \tag{D11}
\]

The exact losses in this margin are `R * Delta(deltaJ)`,
`R^2 * Delta(Gamma)`, `Delta(delta_r)` and `Delta(epsilon)`.
They identify whether a failed admission is dominated by carrier drift,
nonlinear curvature, reconstruction defect or source error. Repeatedly
tightening a Krylov tolerance can only improve the defect portion that its
reconstruction actually controls; it cannot remove an independently fixed
source offset. Improving an adapter copy has no term in D11 at all.

For `Gamma>0`, `alpha>0`, and strict discriminant `D=alpha^2-4 Gamma delta>0`,
the lower feasible radius has

`d rho_minus / d delta = 1/sqrt(D)`.

Thus near a double root this particular uniform-radius test becomes highly
sensitive to residual/source uncertainty. This is a conditioning result for
the sufficient certificate, not a claim of instability of the original IVP.
When D4 fails, a finite-time comparison or shorter slab can still validate
the trajectory; the helper returns `RadiusRejected`, not a physical failure
or a new impossibility theorem. This diagnostic directs algorithm research
toward the actual dominant verified term instead of interpreting every
certificate refusal as switcher overhead.

## Exact controls and coding admission contract

1. A rational input example uses `alpha=2,Gamma=1,delta=1/8,R=1/4,E0=1/16`,
   `h=1/2`. D4 gives `q(R)=-5/16`; `beta=7/4`. D8/D9 can be checked with
   `Fraction` and compared to an independent high-precision diagnostic.
2. Dropping carrier mismatch: `lambda_c=2,deltaJ=3,Gamma=0,delta=1/8,R=1/4`
   is rejected by D4. Pretending `deltaJ=0` would admit it.
3. Dropping source offset: `alpha=2,Gamma=1,delta_r=0,epsilon=1,R=1/4`
   fails; setting `epsilon=0` would admit the declared radius.
4. Oversized radius: `alpha=2,Gamma=1,delta=1/2,R=2` has `q(R)=1/2>0`.
   A looser radius is not automatically safer for a nonlinear tube.
5. Boundary tangency: `alpha=2,Gamma=1,delta=1,R=1,E0<=1` is valid,
   including `E0=1`. This tests the non-strict invariance proof.
6. `R=E0=delta=0` returns exact zero for any finite carrier bound.
7. `alpha=Gamma=delta=0,R>0` yields constant error; it cannot be reported as
   strict contraction. Invalid negative `Gamma`, `delta`, `h`, `E0`, or
   `E0>R` is rejected.
8. Composition includes a norm/reset jump that makes the next slab fail D4;
   omitting the jump is an adversarial failure. A positive diagonal metric
   scale example checks the `old/new` ratio direction.
9. Event signs require strict separation; a grazing/equality case returns
   unresolved. Crossing uniqueness additionally requires transversality.

The implementation target is an isolated exact-rational helper under this
research node, plus tests. It validates scalar contracts and records source
identities; it does not issue a VerifiedModelProvider, change default solver
selection, touch inherited trace source, or add missing physical input values.
Native transplantation requires outward arithmetic for every bound and an
actual provider for D1; source constants must not be silently fitted to make
admission pass.

## R6–R10: evidence and status

Wolfram exact algebra independently returned the quadratic factorization
residual `0`, majorant difference `Gamma E(E-R)`, nonnegative decay under D4,
strict decay with positive delta, and the noncommuting transfer difference.
The first Wolfram query accidentally reused a function symbol for a scalar
jump term; its malformed jump expression is retained and corrected in the
second query. The analytic proofs above do not depend on that malformed row.

Exa on 2026-10-29 found primary work by Breden and Lessard,
*Polynomial interpolation and a priori bootstrap for computer-assisted proofs
in nonlinear ODEs*, [arXiv:1704.03128](https://arxiv.org/abs/1704.03128).
Its abstract supports comparison with polynomial validated IVP methods and
radii-polynomial/Newton-Kantorovich approaches. It does not independently
prove the particular VigilODE D1–D10 contract. The closed-form comparison and
source-connected composition here are derived explicitly; no literature
novelty or new strong no-go is claimed. A second candidate PDF URL failed
retrieval and supplies no evidence.

Independent nonauthor review by `state_audit` checked D1–D10 and found no
blocking mathematical defect. Its two requested clarifications are incorporated:
the norm weights divide coordinates, and spatial local Lipschitz bounds are
uniform on compact time-state neighborhoods. The review accepted the exact
oracle contract subject to those explicit definitions.

The parent independently read D1–D10 and admitted the exact research consumer
after these clarifications. R1–R10 are complete within this conditional theory
scope. The implementation is `../code/defect_composition.py`; eleven methods in
`../code/test_defect_composition.py` await published-source execution. Coding
C1–C10 and actual execution are tracked separately in
`DEFECT_CODING_WORKFLOW.json`. Native physical production remains unqualified.
The coding records will update evidence status without rewriting the retained
native results or cumulative correction budgets.
