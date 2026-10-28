# Exact work boundaries for production return selection

## Scope and reused premises

We reuse PR91's work functional

\[
W=N_f+N_{Jv}+N_{f_t}+nN_J+N_{\rm solve}\in\mathbb N_0.
\]

The same-start state, time, model and tolerance contract are fixed by the
adapter call site. Callbacks must be replay safe. A completed trajectory arm
retains the driver's native admissibility tests. An incomplete arm contributes
only charged work. These are algorithmic assumptions, independent of any
stiffness classification or forecasting accuracy.
Counter values in the stated execution scope are faithfully represented;
the u128 aggregation check addresses sum/product overflow, while native u64
counter saturation is a separate resource bound. The measured100000-attempt
cohort is far below that representational limit.

### Theorem R1: exact return and prefix dominance

Let E be the work of a completed explicit candidate on [a,b], and I the work
of the incumbent implicit replay from the identical checkpoint to b. Among
these completed arms, with ties retaining the incumbent, the strict
observed-work minimizer chooses explicit exactly when E<I. If the implicit
replay is incomplete and its nonfatal charged prefix is L, the condition E<L
is sufficient to prefer the completed explicit candidate to any completion
of that same deterministic replay.

**Proof.** The objective change for completed arms is E−I. Strict improvement
is equivalent to E−I<0. For an incomplete prefix, all remaining work counts
are nonnegative, so I≥L. Thus E<L≤I. The incomplete replay's state never enters
the admissibility argument. A fatal callback ends the solve before this cost
comparison, preserving the existing fatal-priority contract. ∎

The native implementation uses checked u128 multiplication/addition directly
on counters; comparison does not use binary64 display costs. Overflow refuses
cheapness evidence. Candidate completion and finite same-window identity are
checked before using the integer inequality.

### Theorem R2: exact comparator and challenger ceilings

Suppose a callback guard charges a call before stopping at callback work
L_cb>cap, and callback work lower-bounds total work.

* To prove a completed explicit candidate of cost E dominates the implicit
  comparator, use cap=E. At the crossing call E<L_cb≤I, so R1 applies.
* To search for an explicit challenger strictly cheaper than a completed
  implicit incumbent of cost I≥1, use cap=I−1. After a crossing, L_cb≥I, so
  no completion can have cost E<I. For I=0 a strict winner cannot exist; cap0
  is the smallest representable native cap.

**Proof.** Both assertions follow from integrality, monotone charged work and
the strict inequality in R1. The first boundary is E rather than E−1 because
equality is not a return win. The second boundary is I−1 because equality
already rules out a challenger. ∎

The u64::MAX native value means an unarmed guard. If a derived ceiling cannot
be represented below that sentinel, the optional early stop is declined;
the common caller-supplied attempt budget still bounds execution. This does
not convert an overflowed or truncated number into cost evidence.

### Theorem R3: productive sentinel overhead

A sentinel first completes an implicit window with work I, then runs an
explicit candidate with cap I−1. Assume each explicit DOPRI attempt invokes
at most seven RHS callbacks and carries no derivative or linear-solve work.
The completed implicit window remains productive whenever the explicit
challenger loses or fails nonfatally. If I≥1, total sentinel work obeys

\[
W_{\rm sentinel}=I+E\le 2I+6.
\]

If I=0, the general bound is I+max(I−1,0)+7=7. Native positive-span successful
implicit runs have positive work, but this zero case makes the arithmetic
contract explicit.

**Proof.** Before the last attempted DOPRI step its charged cost is at most
I−1; the next attempt costs at most seven. Therefore E≤I+6, including a final
completed but losing attempt. Summing with the productive incumbent gives
the result. A fatal explicit callback stops the run while retaining the
already completed implicit window and charging the failed call. ∎

The seven is the actual stage count of the native Dormand–Prince5 attempt.
After FSAL is available the observed per-attempt maximum is six, but the
uniform proof uses seven to cover initialization. The native session
constructor performs no callback, so no uncharged startup term is omitted.

R3 bounds exploration relative to the **current observed implicit window**;
it does not claim regret relative to the cheaper unknown arm on every future
window. The existing probe spacing, entry pressure and negative-evidence
backoff remain operational choices. A counterexample to predictive optimality
needs only two replay-safe models whose histories match through a but whose
future RHS differ after a: current observations cannot fix that future cost.
We do not promote this elementary indistinguishability observation as a new
no-go theorem; the constructive response is a current-window sentinel.

## Nonlinear discriminating fixtures and an independent exact state

Augment the state by c'=1, c(0)=0. For each remaining component let

\[
y_j'= -\lambda(c)\left(e_j+\tfrac12e_j^3\right)+g'(c),
\qquad e_j=y_j-g(c),\quad e_j(0)=\tfrac14+\tfrac{j}{100}.
\]

With \(\Lambda(t)=\int_0^t\lambda(s)\,ds\), separating variables gives

\[
e_j(t)=\frac{e_j(0)e^{-\Lambda(t)}}{
\sqrt{1+\frac12e_j(0)^2(1-e^{-2\Lambda(t)})}}.
\]

**Derivation.** For positive e define z=e^{-2}; z'=2λz+λ. Multiplying by
exp(−2Λ) and integrating yields z(t)exp(−2Λ)=z(0)+(1−exp(−2Λ))/2. Inverting
the positive square root gives the formula. This independently tests every
state component and every returned time; it does not use another integrator
as the accuracy oracle.

The supplied JVP evaluates the analytic directional action of this augmented
autonomous model. Writing d=1+3e²/2,

\[
(Jv)_0=0,\quad
(Jv)_j=[-\lambda'(e+e^3/2)+\lambda d g'+g'']v_0-\lambda d v_j.
\]

No full Jacobian is assembled. RHS-only policies never invoke this provider.
The two transition rates use tanh, whose primitive is log cosh; the benchmark
computes log cosh through |x|+log1p(exp(−2|x|))−log2 to avoid overflow. These
prescribed problem constants are input cases, not fitted method constants.

## Alternatives and coding contracts

### Proposition R4: causal transport of an observed step proposal

For a nonfatal pilot with m>0 accepted steps from a to c>a, let
\(\bar h=(c-a)/m\). The next eligible pilot, rooted at a later already accepted
explicit checkpoint, may initialize from

\[
h_0^{\rm pilot}=\max(h_0,h_{\rm checkpoint},\bar h),
\]

clipped to its replay span and subject to the existing native minimum-step
validation. This operation changes no committed state, no equation and no
native error/residual threshold. Its proposal uses only past observations.
The next pilot remains an independent admissibility and strict-cost test.

**Proof.** The stored datum is a scalar positive finite average of times of
already accepted pilot states. The code passes the next physical checkpoint
state to a fresh driver. That driver applies its unchanged stage evaluations,
acceptance/rejection rules, callback work ceiling and attempt budget to every
attempt. The adapter still requires a completed qualified candidate and the
exact work comparison before changing routes. No earlier pilot state replaces
the checkpoint, and no future endpoint observation enters the proposal. ∎

R4 preserves the acceptance contract; it does not assert that a larger
proposal always succeeds or reduces total work. Native rejection can consume
the remaining attempt budget. Tests separately cover finite positive prefixes,
zero acceptance, negative/zero/nonfinite spans and a fatal callback. The
new measured cohort determines practical fitness under changing stiffness.

Exact ordinary probes retain a cheap historical scheduling screen and can
miss a newly profitable return. Exact sentinels replace that screen with a
productive current-window comparison. Their stronger evidence can cost more
when an explicit probe loses, making both alternatives useful controls.

Transferring an entire winning DOPRI session could avoid its next startup RHS
and allocation. The current `with_session` API confines that session to a
callback lifetime. A state+step snapshot omits FSAL/controller/workspace
identity and cannot establish equivalent continuation. A future PR should
expose an owned or continuation-based session transfer with epoch and resource
contracts; this iteration does not substitute an incomplete checkpoint.
