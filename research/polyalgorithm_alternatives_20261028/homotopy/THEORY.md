# Cubic planning, derivative-free continuation, and conditional residence

Date: 2026-10-28 UTC  
Baseline: 7eb2dd0ffda39afb2871d76700a720986ff09fe0, PR 92  
Owned follow-up tasks: PR-N02, PR-N04, PR-N06

## Scope and provenance

This is the homotopy/adapter lane of the selected PhysMath Research Loop 1.3.0
R2 (primary-source investigation) and R4 (constructive alternatives). Local
derivations and bounded checks support those stages; this document does not
claim execution of all ten theory or coding stages. No production Rust, default
route, public tolerance, or resource budget is changed here.

Status vocabulary:

- **Derived:** a mathematical statement with its assumptions and proof below.
- **Exploratory checked:** hand/CAS/BigInt checks performed before the supporting
  check registration; no holdout, native solver validation, or timing inference.
- **Planned:** a code contract or experiment that still needs implementation.
- **Open:** an assumption or bridge not supplied by the current callback API.

The inherited statements and negative evidence are retained from:

- [PR 92 homotopy theory](https://github.com/cosmosapjw-quantum/vigilode/blob/7eb2dd0ffda39afb2871d76700a720986ff09fe0/research/production_solver_closure_20261027/homotopy/THEORY.md).
- [PR 92 adapter theory](https://github.com/cosmosapjw-quantum/vigilode/blob/7eb2dd0ffda39afb2871d76700a720986ff09fe0/research/production_solver_closure_20261027/adapter/THEORY.md).
- [Current scalar root consumer](https://github.com/cosmosapjw-quantum/vigilode/blob/7eb2dd0ffda39afb2871d76700a720986ff09fe0/crates/vigilode-coupled/src/rhs_homotopy.rs).
- [Scoped explicit session](https://github.com/cosmosapjw-quantum/vigilode/blob/7eb2dd0ffda39afb2871d76700a720986ff09fe0/crates/vigilode/src/session.rs).
- [Switcher](https://github.com/cosmosapjw-quantum/vigilode/blob/7eb2dd0ffda39afb2871d76700a720986ff09fe0/crates/vigilode/src/switching.rs).

### Critical reading of the two nonlinear-algebra papers

[Robust Numerical Algebraic Geometry, arXiv:2403.18749](https://arxiv.org/pdf/2403.18749)
studies uncertain parameter families and nearby exceptional algebraic strata.
Its regularization may select different parameters, and its examples use
Newton/Jacobian machinery. The useful transfer here is explicit separation of
parameter uncertainty, branch structure, and numerical residual. Moving an ODE's
parameters to a nearby exceptional stratum changes the original target and must
not be an implicit solver acceptance rule.

[Numerical Nonlinear Algebra, arXiv:2302.08585v2](https://arxiv.org/html/2302.08585v2),
sections 2.2, 2.4 and 5, distinguishes regular parameter paths, path tracking,
and certification. Its predictor-corrector and Krawczyk constructions use
derivatives/matrices. Its generic complex continuation theorem does not certify
a prescribed real homotopy path or select the physical root. We reuse those
distinctions, not a claim that those algorithms are derivative-free. Tracking
every complex solution is unnecessary for a scalar stage already known to have
one real root.

The results below are elementary project-specific derivations. No novelty or
general impossibility theorem is asserted.

## PR-N02: a finite-step constant and a decaying defect sum

### Definitions

The sealed model is

\[
 z'=-\phi(z),\qquad \phi(x)=kx+gx^3,\qquad
 k>0,\quad g\ge0,\quad z(0)=Y\ge0.
\]

For a nonnegative starting value \(u\), set

\[
 L(u)=k+3gu^2,\quad M_2(u)=L(u)\phi(u),\quad
 C_{12}(u)=\frac{L(u)^2\phi(u)}{12},\quad
 D_h=1+\frac{hk}{2}.
\]

The exact implicit-midpoint map \(M_h(x)\) is defined by

\[
 M_h(x)-x+h\phi\!\left(\frac{x+M_h(x)}2\right)=0.
\]

The stage increment residual, whose actual outward enclosure remains the
acceptance authority, is

\[
 F_x(K)=K+h\phi(x+K/2).
\]

### Theorem N02.1: finite-step residual bound with constant 1/12

**Derived.** For every \(h>0\), the exact ODE endpoint \(v=z(t+h)\), when
\(u=z(t)\ge0\), satisfies

\[
 |F_u(v-u)|\le C_{12}(u)h^3,\qquad
 |M_h(u)-v|\le\frac{C_{12}(u)h^3}{D_h}.
 \tag{1}
\]

This improves the inherited constant \(L(u)^2\phi(u)/8\). It is a finite-step
inequality, not only the leading term of a small-\(h\) expansion.

**Proof.** On \(0\le s\le h\), the exact trajectory lies in \([v,u]\), decreases,
and is convex because

\[
 z''=\phi'(z)\phi(z),\qquad 0\le z''\le M_2(u).
\]

Let \(\ell(s)=u+(v-u)s/h\) be its chord and \(m=(u+v)/2\).
The elementary chord bound is

\[
 0\le \ell(s)-z(t+s)\le \frac{M_2(u)}2s(h-s).
\]

The upper bound follows by comparing second derivatives and the common zero
endpoint values of the chord error and the parabola. Define

\[
 A=\int_0^h[\phi(\ell(s))-\phi(z(t+s))]\,ds,\qquad
 B=\int_0^h\phi(\ell(s))\,ds-h\phi(m).
\]

Both are nonnegative: \(\phi\) is increasing and convex on \([0,u]\).
The ODE integral gives \(F_u(v-u)=A-B\). Hence

\[
 A\le L(u)\frac{M_2(u)}2\int_0^h s(h-s)\,ds
   =\frac{L(u)^2\phi(u)h^3}{12}.
\]

Cubic integration is exact:

\[
 B=\frac{g h m(u-v)^2}{4}
 \le \frac{g u\phi(u)^2h^3}{4}.
\]

Here \(m\le u\) and \(u-v=\int_0^h\phi(z(t+s))ds\le h\phi(u)\).
Furthermore

\[
 L(u)^2-3gu\phi(u)=k^2+3kgu^2+6g^2u^4\ge0.
\]

Consequently \(B\le L(u)^2\phi(u)h^3/12\), and
\(|A-B|\le\max(A,B)\) proves the first bound. Finally
\(F_u'(K)=1+(h/2)\phi'(u+K/2)\ge D_h\) on the entire real line,
so the mean value theorem proves the second bound. This derivative is used in
the analytic proof; the runtime stage needs only polynomial values. \(\square\)

The improvement comes from integrating the chord kernel exactly. Replacing
the kernel by its maximum would lose the factor \(2/3\). The proof also needs
the sign information \(A,B\ge0\); adding their separate magnitudes is weaker.

### Theorem N02.2: transport numerical error separately from exact decay

**Derived.** \(M_h\) is globally nonexpansive on the real line for every \(h>0\).

To see this, write \(m=x-(h/2)\phi(m)\). Its solution is unique, since
\(m+(h/2)\phi(m)\) is strictly increasing onto the real line. Differentiation
gives

\[
 M_h'(x)=
 \frac{1-\tfrac h2\phi'(m)}{1+\tfrac h2\phi'(m)},\qquad |M_h'(x)|\le1.
\]

Suppose an accepted represented update has certified local algebraic/update
error \(\alpha_j\):

\[
 |\widehat y_{j+1}-M_h(\widehat y_j)|\le\alpha_j.
\]

Then, comparing the numerical state with the exact ODE state \(z_j=z(jh)\),

\[
 E_{j+1}\le E_j+\frac{h^3 C_{12}(z_j)}{D_h}+\alpha_j.
 \tag{2}
\]

The constant in the truncation term is evaluated on the **exact ODE
trajectory**, not on an unverified decaying numerical trajectory. This is
legitimate because the nonexpansive term compares
\(M_h(\widehat y_j)\) with \(M_h(z_j)\) separately.

Thus the existing outward stage residual and actual update-rounding enclosure
can be retained. The existing time-label displacement term must also remain.

### Theorem N02.3: positive geometric sums without a transcendental library

**Derived.** From \(z'+kz=-gz^3\le0\),

\[
 0\le z_j\le Y e^{-kjh}\le Y q^j,\qquad q=(1+kh)^{-1}.
\]

The last inequality follows from \(e^x\ge1+x\). No evaluation of an
exponential is needed for the rational envelope. The polynomial expansion is

\[
 C_{12}(x)=\frac1{12}
 [k^3x+7k^2gx^3+15kg^2x^5+9g^3x^7].
\]

Every coefficient is nonnegative. Define
\(S_n(r)=\sum_{j=0}^{n-1}r^j\). A certified truncation envelope is therefore

\[
 {\cal T}_n=
 \frac{h^3}{12D_h}\left[
 k^3Y S_n(q)+7k^2gY^3 S_n(q^3)
 +15kg^2Y^5 S_n(q^5)+9g^3Y^7 S_n(q^7)
 \right].
 \tag{3}
\]

Equation (2) gives
\(E_n\le{\cal T}_n+\sum_{j<n}\alpha_j\), plus the inherited time-label term.
For planning, keep the existing dyadic grid and, for the current Picard
fallback, the existing condition

\[
 q_B=\frac h2[k+3g(Y+\epsilon)^2]<1.
\]

Choose the smallest admitted dyadic \(N\) with \({\cal T}_N<\epsilon\).
The unchanged uniform budget construction becomes

\[
 a_{\rm step}=\frac{\epsilon-{\cal T}_N}{N},\qquad
 r_{\rm stage}=D_h\,a_{\rm step},
\]

with all subtractions, divisions and products rounded in the required
directions. As before, rounding can make a positive real budget unrepresentable;
the planner then declines that grid.

For \(N=2^m\), the geometric sums need only positive operations. Starting with
\(p=r,s=1\), repeatedly apply

\[
 (p,s)\longmapsto(p^2,s(1+p)).
 \tag{4}
\]

After \(m\) doublings, \(p=r^N,s=S_N(r)\). This is \(O(\log N)\) planning
work and avoids cancellation in \((1-r^N)/(1-r)\). The case \(r=1\) is
already handled. An outward upper enclosure of \(q\), intersected with the
mathematically known interval \([0,1]\), is sufficient.

**Minimum-grid supporting result.** For the exact rational fixture
\(k=1,g=1/2,Y=T=1,\epsilon=1/100000\), an exploratory BigInt rational
calculation gave:

| Bound | N=128 | N=256 | N=512 | First admitted dyadic N |
|---|---:|---:|---:|---:|
| inherited constant 1/8 | 7.1247264105e-5 | 1.7846536915e-5 | 4.4659870427e-6 | 512 |
| constant 1/12 only | 4.7498176070e-5 | 1.1897691277e-5 | 2.9773246951e-6 | 512 |
| 1/12 and rational decay sum | 1.3623174036e-5 | 3.3853071299e-6 | 8.4376762648e-7 | 256 |

Pass/fail used exact rational comparisons; displayed decimals are diagnostics.
The contraction condition also holds on these grids. Earlier dyadics fail:
the constant bounds decrease with refinement, and for (3),
\(q_{h/2}^2\le q_h\) implies the fine-grid sum is at most twice the
coarse-grid sum, while the \(h^3/D_h\) factor makes the total strictly smaller.
The new bound halves the **theoretical selected step count** on this fixture.
It does not establish half the elapsed runtime or any production speedup.
The forthcoming local CHECK.py repeats this exact arithmetic after source
publication; this exploratory result is not relabeled as preregistered.

### Code contract and alternatives for PR-N02

The production implementation still needs:

1. Replace the planner's constant by the proved \(C_{12}\); implement (3)--(4)
   with the existing directed arithmetic.
2. Replace the prefix expression \(n\) times one constant local bound by a
   valid prefix envelope. A runtime accumulated outward prefix and the
   precomputed total are independently valid upper bounds, so their minimum
   is also valid. This can prevent different rounding orders from causing an
   artificial end-of-run overshoot.
3. Keep original-residual verification, update rounding, time-label rounding,
   physical tolerance, fatal failures, and all resource charges intact.
4. Verify cancellation, underflow, subnormal denominators, overflow, \(Y=0\),
   \(g=0\), and strict-boundary decisions with exact/high-precision oracles.
5. Compare old and new planners with the same rational/scalar root backend,
   so a grid improvement is not confused with a root-kernel improvement.

Additional constructive branches remain explicit alternatives:

- **Stronger decay envelope:** the known exact solution
  \(Y e^{-kt}/\sqrt{1+(gY^2/k)(1-e^{-2kt})}\) is tighter, but production use
  needs validated transcendental enclosures. Ordinary exp/sqrt results are
  not a proof.
- **Contraction-weighted transport:** on a certified domain
  \(|x|\le B\) with \(hL(B)/2<1\), the midpoint map has Lipschitz constant
  \(\rho=(1-hk/2)/(1+hk/2)<1\). Equation (2) can then use
  \(\rho E_j\). A planner must bound every prefix, not only the final error,
  and preserve the invariant-domain induction. This is derived but not
  implemented or measured here.
- **Backend-specific admission:** \(q_B<1\) is required by the current
  Picard fallback, not by existence/uniqueness of the cubic root or the
  global nonexpansivity proof. A separate monotone-bisection/inverse backend
  can be investigated without that gate. It must not silently call an
  uncertified noncontractive Picard fallback. Numerical positivity and
  monotone physical decay are separate from nonexpansivity; midpoint is
  not promoted as an L-stable method.
- **Exact model mode:** a sealed analytic-flow backend could avoid time
  stepping for this model, but it would be a separately identified
  model-specific solver with validated transcendental rounding. It is not
  evidence for a generic causal/Rosenbrock speedup.
- **Equilibrium certificate:** when \(Y=0\), the exact trajectory is zero.
  A specialized certified zero-work output path is possible if its output
  schedule and telemetry contract are defined. It is not a reason to loosen
  resource accounting for nonzero inputs.

## PR-N04: branch-safe continuation and full session reuse

### Theorem N04.1: scalar monotone bracketing does not need Newton

**Derived.** If continuous \(F:[a,b]\to\mathbb R\) has opposite certified
endpoint signs, the interval contains a root. If, in addition,

\[
 F(v)-F(u)\ge m(v-u)\quad(a\le u<v\le b),\qquad m>0,
\]

the root is unique and
\(|x-x_*|\le |F(x)|/m\). Existence is the intermediate value theorem;
uniqueness and the residual bound follow directly from the displayed
inequality. No derivative evaluation is required at runtime. The monotonicity
certificate must come from the model or a valid enclosure; finitely many
ordinary callback values alone do not establish it.

For the existing cubic increment,
\(F_y'(K)\ge1+hk/2\) is an analytic certificate, and the bracket endpoints
are \(K=0\) and \(K=-2y\):

\[
 F_y(0)=h\phi(y),\qquad F_y(-2y)=-2y.
\]

This preserves the original polynomial target. A candidate from a rational
inverse, algebraic inverse, polynomial predictor or warm seed is only a
proposal until the original residual enclosure passes.

**Branch counterexample.** For \(F(x;p)=x^2-p\) and \(p>0\), both
\(\pm\sqrt p\) have zero residual. A small residual cannot select the
positive physical branch. A bracket in \((0,\infty)\) does select it;
on \([a,b]\), \(a>0\), an analytic lower monotonicity constant is \(2a\).
As \(p\to0\), branch separation and that lower bound can collapse. A policy
must refine the branch certificate or report its loss. This example does
not prove that derivative-free continuation is generally impossible.

### Theorem N04.2: a value-only continuation station from an exact margin

Consider the actual project homotopy

\[
 G_\lambda(x)=(1-\lambda)a+\lambda G(x).
\]

Suppose \(G\) is \(q\)-Lipschitz on a closed ball
\(D=\overline B(c,r)\), and its values are defined there. At a current
\(\lambda\), let certified upper bounds be

\[
 e\ge\|G_\lambda(c)-c\|,\qquad b\ge\|G(c)-a\|.
\]

For \(\mu=\lambda+\delta\le1\), the ball is invariant if

\[
 (\lambda+\delta)qr+e+\delta b\le r. \tag{5}
\]

If also \(\mu q<1\), Banach's theorem supplies one root of
\(G_\mu(x)=x\) in \(D\), with residual-to-root bound
\(\|G_\mu(x)-x\|/(1-\mu q)\). Provided the numerator is positive and the
denominator is nonzero, the largest station increment permitted by this sufficient
condition is

\[
 \delta_{\max}
 =\frac{(1-\lambda q)r-e}{qr+b},\qquad
 0\le\delta\le\min(1-\lambda,\delta_{\max}). \tag{6}
\]

The separate strict condition \(\mu q<1\) must still be checked.
Zero-denominator cases are handled from (5), not by dividing zero.

**Proof.** For \(x\in D\),

\[
 \|G_\mu(x)-c\|
 \le\mu q\|x-c\|+\|G_\lambda(c)-c\|
      +\delta\|G(c)-a\|
 \le\mu qr+e+\delta b.
\]

This proves invariance. The Lipschitz factor is \(\mu q\), proving the
remaining statements. Solving (5) for \(\delta\) gives (6). \(\square\)

All constants have mathematical roles. Radius \(r\), Lipschitz bound \(q\),
and their validity region must be model-supplied or certified; they cannot be
renamed empirical tuning parameters. The policy can select the maximal
representable admissible station by directed rounding and strict checks.

When \(G\) is already contractive on a common convex invariant domain at
\(\lambda=1\), direct solution is admitted. Intermediate stations may then
cost more and require measured justification. When \(q\ge1\), continuation
can still reach some stations, but (6) does not certify the endpoint
\(\lambda=1\); no general endpoint theorem is claimed.

The current solver resets Anderson history when \(\lambda\) changes.
Retain that rule. Reuse vector capacity across stations, but do not reuse
residual secants as if different \(G_\lambda\) were one operator.
Across ODE steps, changes in base state, time, \(h\), norm, tolerance or
model binding also change the effective stage map.

### Theorem N04.3: what must move with a winning session

**Derived, conditional on deterministic callbacks and unchanged bindings.**
Let a native integrator be a deterministic transition system on a full state
\(S\). If a pause/resume operation preserves every component read by future
transitions, preserves future observation landings, and performs the same
arithmetic in the same order, its future accepted states, failures, and work
trace equal uninterrupted continuation. Proof is induction on transitions.

For this repository, the required state includes at least:

- accepted time/state, proposed next step and native stage buffers;
- FSAL derivative and its valid time/state/model binding;
- controller error history and rejection state;
- registered output schedule and current cursor;
- cumulative attempt/resource budget and callback/failure telemetry;
- method, dimension, model/provider/norm/tolerance epochs.

Current SessionEpochs and with_session already encode much of this contract.
SessionSnapshot contains telemetry; it is not a serialized full session.
Copying only \((t,y,h)\) cannot establish equivalent continuation.

**Preferred first implementation:** retain a productive winning explicit
probe inside the existing scoped with_session continuation, register the full
future landing schedule before construction, and continue the same native
session. This avoids introducing a self-referential owned callback merely to
transfer the session. If an owned-session API is later necessary, its identity,
lifetime and failure contracts require independent review.

**Savings contract:** moving an existing winning session can remove its next
reconstruction allocation and initial RHS refresh when FSAL remains valid.
This is conditional saved work, not a blanket speedup. If epoch, norm,
schedule or model identity changes, invalidate/reconstruct and charge actual
work. Cumulative budgets never reset and all probe work remains charged.

### Implementation and experiment split for PR-N04

There are two independent costs to measure:

| Layer | Candidate | Required experiment |
|---|---|---|
| adapter/session | retain winning native session | pause/resume trace parity; identical schedule/epochs; changed-epoch refusal before callbacks; allocation and startup RHS counts |
| root/workspace | retain vector capacity, reset operator history | same lambda/map parity; changed lambda/base-state/step identity; allocation count and charged map count |
| root/continuation | station rule (6) | certified toy domains and branch-collapse examples; compare direct lambda=1 with continuation; report extra station work |
| candidate ordering | rational, Picard, bisection, optional polynomial predictor | common original residual/budget; count rejected candidates and interval evaluations |

The inherited rational inverse has an exact nonnegative defect, so it is a
legitimate cheap proposal before expensive cube-root work. Its measured
advantage over the scalar route was not uniform in the earlier report.
A prediction order must retain per-instance accounting and adverse cases;
no unconditional scalar/rational winner is inferred.

## PR-N06: residence requires a temporal cost certificate

The existing exact integer same-window comparison proves which completed
route was cheaper on that replay target. An incomplete nonfatal prefix gives
a lower bound on its eventual work. Those results remain useful and are not
a theorem about future windows. The inherited productive-sentinel bound
\(W_{\rm sentinel}\le2I+6\) for \(I\ge1\) is likewise a current-window bound.

### Theorem N06.1: finite residence from discrete variation

Fix a counterfactual comparison protocol: same physical start checkpoint,
model binding, tolerance, target window, and explicitly specified controller/
session initialization for both routes. Let \(C_A(j),C_B(j)\) denote their
costs in one declared unit for future equal-comparison windows \(j=0,1,\ldots\).
Suppose a certificate valid for these windows supplies

\[
 C_B(0)-C_A(0)\ge d>0,\qquad
 |[C_B(j)-C_A(j)]-[C_B(j-1)-C_A(j-1)]|\le V.
\]

For \(j\ge1\), telescoping gives

\[
 C_B(j)-C_A(j)\ge d-jV. \tag{7}
\]

Thus route \(A\) stays strictly cheaper for \(m\) future windows if
\(mV<d\). Its cumulative saving against \(B\) is at least

\[
 \sum_{j=1}^m[C_B(j)-C_A(j)]
 \ge md-\frac{V\,m(m+1)}2. \tag{8}
\]

If switching, calibration and extra probes cost at most \(S\) in the same
unit, the action pays for itself when the right side of (8) exceeds \(S\).
The constants \(1/2\) and \(m(m+1)\) come from the exact arithmetic sum,
not a fitted residence multiplier. Integer/rational cross-products can test
both inequalities without floating-point square roots.

For \(V=0\), the smallest paying horizon is
\(\lfloor S/d\rfloor+1\), when such a horizon is available. For \(V>0\),
both the payback inequality and \(mV<d\) must hold; if no integer \(m\)
satisfies them, this certificate does not justify a residence commitment.

### Theorem N06.2: continuous variant and its scope

For certified cost rates \(c_A,c_B\) with
\(c_B(0)-c_A(0)\ge d>0\) and Lipschitz constant \(L\) for their difference,

\[
 c_B(t)-c_A(t)\ge d-Lt,\qquad
 \int_0^H(c_B-c_A)\,dt\ge dH-\frac{LH^2}2. \tag{9}
\]

For \(L>0\), \(H\le d/L\) gives nonnegative pointwise advantage, and
\(dH-LH^2/2>S\) pays the overhead. When \(L>0\), the maximum of this
quadratic on that interval is \(d^2/(2L)\); thus
\(d^2>2LS\) is necessary for this sufficient-bound family to certify a
strictly positive net gain. It is not a necessary condition for actual
performance to improve.

For \(L=0\), every \(H\ge0\) has the same certified positive rate
advantage, and strict payback is exactly \(dH>S\). No division by \(L\)
is used in this branch.

### Why the certificate is still an open production bridge

Smooth ODE coefficients do not automatically make adaptive solver work
Lipschitz. An error-test ratio crossing 1 can add a rejected attempt and a
whole stage batch. Controller changes, landing constraints, nonlinear
iteration thresholds and differing session histories can add further jumps.
A finite difference of recently measured work is therefore only an estimate
of \(V\) or \(L\).

The certificate must describe both counterfactual route states, not just one
observed numerical trajectory. Accepted states within a local tolerance can
still lead to different future adaptive traces. Either a proved common
state enclosure and trace/variation bound is supplied, or the residence
claim remains operational.

A possible next theory loop is **trace-margin certification**: interval
enclosures keep every acceptance and controller predicate strictly on its
observed side over a declared model/state neighborhood. A fixed trace then
has fixed charged work in that neighborhood. Establishing and propagating
those neighborhoods is additional research, and may cost more than direct
productive comparisons.

The data type should distinguish:

- observed same-window work from a certified future variation envelope;
- callback-count units from calibrated wall-time units;
- an estimate, a deterministic bound, and a statistical confidence statement;
- current state/model/norm/tolerance identity from the certificate's lifetime.

Without a valid temporal certificate, retain the existing same-window
productive comparator and finite caller-owned budgets. Do not relabel
backoff or a recent cost ratio as a proven regret guarantee.

## Adversarial controls and bounded future coding loops

The following outcomes are required evidence, not exclusions to hide:

- A smaller local-defect constant alone may leave the dyadic grid unchanged.
- The exact rational decay envelope can reduce a grid while extra planner
  arithmetic offsets some elapsed-time gain.
- A rational/inverse candidate can fail or overflow; the original interval
  residual and finite fallback budget still decide.
- A zero residual can occur on the wrong branch.
- A continuation certificate can expire before lambda=1.
- Session state or a cached derivative can become invalid after an epoch change.
- A cheaper current replay can become the more expensive future route.
- A mathematically valid future bound may be too expensive to obtain.

For each task, preserve the existing at-most-three correction-loop budget;
this supplement does not reset previously consumed loops. The first future
coding experiment should isolate one hypothesis. Only an observed concrete
failure justifies another modification. Native tests, compiler output,
release benchmarks, profiling, and broad production qualification remain
NOT_RUN for this supplement.

## Supporting-check record

Before the supporting protocol was committed, exploratory Wolfram checks
returned zero for the cubic chord identity and the exact-solution ODE residual,
verified the positive dominance polynomial
\(k^2+3kgu^2+6g^2u^4\), and integrated the chord kernel to \(h^3/12\).
One earlier symbolic call attempted to Clear the protected symbol N; it
emitted that warning while returning the stated identities. The subsequent
chord calculation used unprotected symbols and returned without that warning.
No claim depends on symbolic output alone; the proofs are above.

The exact BigInt fixture table above was also exploratory. CHECK.py provides
a dependency-free rational repetition plus invariant-ball and negative
controls. Its execution record is owned by the root campaign after the
script/protocol source commit.

## skill_context_used

- PhysMath Research Loop 1.3.0: selected R2 and R4; stage-local derivations.
- THEORY_LOOP.md: source provenance, definition-to-proof-to-code contracts,
  established/derived/conjectural separation, constructive alternatives.
- Parent-provided integrated policy: derivative-free-first identity,
  justified exact constants, retained negative evidence, no recursive
  subagents, one meaningful verification pass.
- No strong no-go or general solver-completeness claim is made.

