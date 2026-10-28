# A physical-error contract for the integrated polyalgorithm

## Scope, provenance and skill context

This is a constructive R3/R5/R9 result under physmath-research-loop 1.3.0,
using the parent's `../SKILL_CONTEXT.md`. Sources are read at PR93 commit
`384dba091a3d9e1aac0ecddec264958298239253`. No native solver experiment is
performed in this lane. The accompanying exact-rational checker is a finite
supporting check, not a proof of arbitrary-dimensional floating implementation.

The useful integration question is: **how can different admitted backends share
one physical accuracy contract without confusing a polynomial-action bound,
a stage-root residual, and a trajectory-error bound?** The construction below
answers it for explicitly supplied stability/domain/defect certificates. It
does not infer such certificates from an embedded error estimate or timing.

### Existing results reused

| Source | Already established and retained | Integration contribution here |
|---|---|---|
| `research/rigorous_performance_audit_20261027/theory/THEORY_AUDIT.md`, Theorems C/D | Causal residual-to-stage transfer, order-preserving algebraic tolerance under uniform hypotheses, physical-chart residual identity | Explicitly distinguish two valid trajectory induction anchors and their different local-error inputs |
| `research/derivative_free_causal_homotopy_20261027/theory/THEORY.md`, Sections 11.2/11.4 | Cubic nonexpansive induction; leftover error budget; sharp structural residual weights | Connect both to a single declared physical output contract without claiming they certify the same quantity |
| `research/production_solver_closure_20261027/structural/THEORY.md`, S1--S3 | Exact represented-stage residual, mean/lift/RHS rounding bounds | Use as an algebraic leaf; do not add the same lift/RHS errors a second time |
| `research/production_solver_closure_20261027/homotopy/THEORY.md`, H2 | Predictor-independent acceptance through original residual plus update rounding | Extend predictor independence to arbitrary backend/mode selection under the same target contract |
| `research/polyalgorithm_alternatives_20261028/homotopy/THEORY.md` | Cubic 1/12 local bound evaluated along the exact trajectory; rational decay sum | Preserve the correct discrete-map induction, including when this planner is composed with other candidate generators |
| `research/polyalgorithm_alternatives_20261028/geometry/THEORY.md`, G6--G9 | Exact two-channel discrete transport and exact-flow physical defect identity | General common-space and changing-norm contract; clock ownership and per-observation admission |
| `crates/vigilode/src/selection_evidence.rs` | Physical identity epochs and accuracy eligibility before cost comparison | A mathematical meaning for the accuracy eligibility being transported |
| `crates/rodas5p-core/src/polynomial_router.rs` | Private admission of a common represented fused phi action | Keep that action-level token distinct from a physical step certificate |
| `crates/vigilode-coupled/src/rhs_homotopy.rs` | `cubic_verified_step` adds residual/inverse-denominator and update displacement; `cubic_integrate` separately adds clock-label displacement | Preserve those units and separate owners when the planner is extended |
| `crates/vigilode-coupled/src/adaptive.rs` | Physical acceptance belongs to the driver; provisional fallback accepted/rejected counts are reset | One driver commits physical state after candidate admission; failed/rejected work remains a cost |

No new universal convergence or stiffness-uniform order theorem is claimed.
All statements below are derived directly; they are not attributed as novel
general numerical-analysis theorems. Their contribution is the explicit
cross-PR composition and the new code-ready consequences and counterexamples.

## E1. A selected backend need not have a smooth selector

Let the original ODE have a unique flow `Phi(t,s,x)` on a certified tube.
Fix a physical norm `||.||_*`. Accepted times `t_0 < ... < t_N` may depend on
the entire computed history. Rejected attempts do not advance physical state.
Let `y_n` be the exact physical solution and `yhat_n` the committed represented
state, interpreted as an exact real vector. At the *actual* accepted step,
assume a flow stability bound

\[
 \|\Phi(t_{n+1},t_n,x)-\Phi(t_{n+1},t_n,z)\|_*
 \le \Lambda_n\|x-z\|_* ,\qquad \Lambda_n\ge0,                 \tag{E1}
\]

valid for `x=yhat_n`, `z=y_n` and the required tube. A selected candidate `b_n`
must supply

\[
 \|\widehat y_{n+1}-\Phi(t_{n+1},t_n,\widehat y_n)\|_*
 \le d_n.                                                   \tag{E2}
\]

**Theorem E1 (selection-independent physical induction).** With `E_0`
enclosing the initial error, the outward recurrence

\[
 E_{n+1}\ge\Lambda_n E_n+d_n                                  \tag{E3}
\]

encloses every committed physical error. The choice of `b_n` can be
discontinuous, history-dependent, or obtained by any finite collection of
attempts; no derivative or Lipschitz constant of the *selector* is required.

**Proof.** Add and subtract `Phi(t_{n+1},t_n,yhat_n)` in the physical error.
The first term is bounded by (E2) and the second by (E1). Induct along the
realized accepted sequence. The same original flow and realized times occur
on both sides; the proof never compares the selector applied to two states.
It consequently never differentiates the selector. QED.

This proves accuracy of *accepted results*, not termination, bounded work,
or discovery of an admissible candidate. Domain validity must hold on the
actual required tube; acceptance cannot infer it from sampled endpoint values.
Failed probes must be transactionally rolled back or their state changes
must enter (E2). Their work remains charged even when their physical defect
is zero because they are discarded.

The corresponding exact-real finite product bound is

\[
 \|\widehat y_N-y_N\|_*\le\Bigl(\prod_{j=0}^{N-1}\Lambda_j\Bigr)E_0
       +\sum_{n=0}^{N-1}\Bigl(\prod_{j=n+1}^{N-1}\Lambda_j\Bigr)d_n. \tag{E4}
\]

For an unknown future adaptive grid, (E3) supports current-prefix admission.
Future products in (E4) become an online reservation only when an enclosing
future-flow bound is independently supplied. An accepted prefix by itself
does not promise that the remaining resource/error budget can finish the run.

### A concrete RHS-only source of a flow-defect certificate

For a fixed Hilbert norm, suppose on the valid tube
`<f(t,x)-f(t,z),x-z> <= mu(t)||x-z||^2`. The squared-error differential
inequality gives (E1) with
`Lambda_n = exp(integral_(t_n)^(t_(n+1)) mu(s) ds)`. If a differentiable
reconstruction `v(t)` starts at `yhat_n`, ends at the computed endpoint,
and has residual `R(t)=v'(t)-f(t,v(t))`, the same comparison gives

\[
 d_n\le\int_{t_n}^{t_{n+1}}
       \exp\!\left(\int_s^{t_{n+1}}\mu(r)\,dr\right)
       \|R(s)\|_*\,ds.                                      \tag{E4a}
\]

The derivative of a polynomial reconstruction is algebraic and requires
no Jacobian/JVP/AD callback. The residual uses RHS evaluations; a rigorous
integral bound still requires a valid interval/domain residual enclosure,
not point sampling alone. Any mismatched reconstruction endpoint adds
its explicit endpoint edge. These are standard energy-estimate ingredients;
see Ranocha and Giesselmann, *Stability of step size control based on a
posteriori error estimates* (2024), Section 2.1, Lemma 1,
https://doi.org/10.1007/s44207-024-00001-0, read directly in this lane.
Their controller analysis also emphasizes that error reliability and
step-size-controller stability are separate properties. E1 concerns the
former and does not claim the latter. For the sealed scalar cubic, the
analytic one-sided constant is `-k`, without runtime differentiation.

### Application to the polyalgorithm

`b_n` can select a proved direct structural flow, Rosenbrock step with a
qualified causal root, or a qualified homotopy/RK step. Each must discharge
(E2) for the same original model, represented physical checkpoint, observation
time and norm. A phi-action or resolvent bound is generally only one term in
(E2). A successful local-tolerance-controlled DOPRI call has its existing
local-tolerance contract; that status is not converted into (E2) by this theorem.

## E2. Two valid induction anchors; one invalid combination

The physical-flow anchor in E1 is not the only valid route. Let
`Psi_{n,b}` denote the exact-arithmetic *discrete step recipe* for selected
backend/method `b`, and suppose the implemented result obeys

\[
 \|\widehat y_{n+1}-\Psi_{n,b_n}(\widehat y_n)\|_*\le a_n.       \tag{E5}
\]

All stored coefficients, frozen operator/carrier parameters, norm and chart
identities must be fixed when defining this recipe. In a comparison using
`Psi(y_n)`, they remain those of the selected recipe; silently rebuilding a
different carrier at `y_n` changes the function being compared.

There are two distinct sufficient contracts:

**F: exact-flow propagation at the numerical input.** A bound

\[
 \tau_n^{\rm num}\ge
 \|\Psi_{n,b_n}(\widehat y_n)-\Phi(t_{n+1},t_n,\widehat y_n)\|_*
\]

gives `d_n <= a_n + tau_n^num` and

\[
 \|\widehat y_{n+1}-y_{n+1}\|_*
 \le\Lambda_n E_n+a_n+\tau_n^{\rm num}.                       \tag{E6}
\]

**D: discrete-map propagation with truncation at the exact input.** If
`Psi_{n,b_n}` is `L_n`-Lipschitz between `yhat_n` and `y_n`, and

\[
 \tau_n^{\rm exact}\ge
 \|\Psi_{n,b_n}(y_n)-\Phi(t_{n+1},t_n,y_n)\|_* ,
\]

then

\[
 \|\widehat y_{n+1}-y_{n+1}\|_*
 \le L_nE_n+a_n+\tau_n^{\rm exact}.                          \tag{E7}
\]

**Proof.** For F, insert the discrete step at the numerical input into (E2).
For D, add and subtract `Psi(yhat_n)` and `Psi(y_n)` in the physical error.
Apply (E5), discrete Lipschitz continuity, and the exact-input truncation
bound. Both use the triangle inequality, but at different intermediate
points. The next implemented certificate rounds the appropriate right-hand
side upward. QED.

**Counterexample to mixing the anchors.** On the real line take the exact
flow step `Phi(x)=x/2`, discrete step `Psi(x)=x`, exact input `y_n=0`, and
numerical input `yhat_n=e>0`. There is no implementation error. The
exact-input truncation is zero, while the output error is `e`. Combining
the flow contraction `Lambda=1/2` with this exact-input truncation would
give the false upper bound `e/2`. Formula D uses `L=1` and is exact.
Formula F includes `tau^num=e/2` and is also exact.

**Why this matters for PR93.** The cubic `1/12 + rational decay` planner
evaluates its truncation estimate along the *exact decaying trajectory*.
Its valid propagation is the nonexpansiveness of the midpoint discrete
map, i.e. D with `L_n=1`. This is precisely the existing proof. It must
not be combined with a stronger flow-contraction factor while leaving
the truncation term unchanged. A new numerical-input bound or a justified
correction would be needed for that different formula. This counterexample
targets that possible integration error, not an error in the PR93 proof.

**Connection to the algebra compiler.** An endpoint polynomial compiled
from the original raw-U equations, together with a full residual and
finite causal transfer, supplies (E5) relative to the exact original
Rosenbrock stage solve at the same `yhat_n`. It does not supply either
truncation term automatically. Use D with a valid discrete stability and
exact-input truncation bound, or F with an actual physical-flow local defect.
This remains true if virtual stages need not be materialized.

## E3. Representation, norms and clocks have explicit owners

### A telescoping path prevents missing or duplicated error terms

Choose physical-space anchor values

\[
 x^{(0)}=\Phi(t_{n+1},t_n,\widehat y_n),\quad
 x^{(1)},\ldots,x^{(r)}=\widehat y_{n+1}.                      \tag{E8}
\]

Each edge must name its exact mathematical operands and an enclosure
`||x^(j+1)-x^(j)||_* <= epsilon_j`. Telescoping gives

\[
 d_n\le\sum_{j=0}^{r-1}\epsilon_j.                          \tag{E9}
\]

For example, edges can represent a justified model transformation,
physical-clock versus engine-clock discrepancy, exact temporal method,
inexact original stage solve, and represented endpoint assembly. These
labels are not permission to omit an edge: the actual endpoints determine
which terms a certificate already contains. A coarse certificate covering
several consecutive edges is an alternative cover of that subpath. It is
not an additional mandatory charge on top of those same individual edges.

If a reconstruction map `R` is `K_R`-Lipschitz on a validated coordinate
ball, a coordinate discrepancy `eta` contributes at most `K_R eta` in
physical units, plus any separately enclosed reconstruction arithmetic.
For a linear chart this is its operator norm. A coordinate norm is not
itself a physical norm merely because both values have type `f64`.

In particular, for weighted infinity norm
`||v||_w = max_i |v_i|/w_i`, `w_i>0`,

\[
 \|v\|_\infty\le (\max_i w_i)\|v\|_w.                       \tag{E10}
\]

The constant is sharp: use a vector supported where `w_i` is maximal.
Between two such weights,
`||v||_v <= max_i(w_i/v_i) ||v||_w`, also sharply. Changing norm epochs
therefore requires this conversion or a new physical evaluation.

**PR92 consequence.** Its represented-stage residual enclosure already
contains RHS, computed mean and lift defects. When the causal inverse map
converts that residual to endpoint stage error, those same defects should
not be separately re-added as if missing. Endpoint accumulation not covered
by that certificate still needs its own edge. A direct physical-flow defect
can instead cover the entire path and should not be added to the algebraic
and temporal bounds that it replaces.

### Two consistent clock conventions

1. **Physical-label convention:** the canonical accepted times are the real
   values of stored `t_n`. The local defect compares with the flow over
   exactly `t_(n+1)-t_n`. If the engine uses another represented `h_n`,
   that mismatch belongs to the local clock edge in (E8).
2. **Internal-clock convention:** the mathematical integration times are
   exact `tau_n` (for example the exact sum of represented step lengths),
   and a stored output label `t_n` is reported separately. Transport the
   internal-state error `E_n^int`. At an observation, if the exact physical
   trajectory satisfies `||y'(s)||_* <= M_n` between the two times, report

\[
 E_n^{\rm obs}\le E_n^{\rm int}+M_n|t_n-\tau_n|.             \tag{E11}
\]

This follows from the integral of the exact derivative over the time gap.
The observation-only term is not an internal-state perturbation to be
re-injected at every later step. The next recurrence propagates the
internal certificate unless an actual state/time conversion took place.

The cubic implementation currently follows the second convention: it
propagates algebraic error, forms an exact-time interval, and adds the
current label displacement at output. The new planner must preserve this
ownership. For `y'=1` and exact internal states `yhat_n=tau_n`, every
internal error is zero and the error at label `t_n` is exactly
`|t_n-tau_n|`, not the sum of all earlier label discrepancies.

For nonautonomous engine start-time shifts, a bound on the final exact
solution's speed alone does not bound a whole shifted-vector-field step.
The relevant clock edge needs time-regularity and flow sensitivity over
the actual shifted interval, or an exact-flow enclosure. Merely counting
two endpoint timestamp ULPs is insufficient.

## E4. Exact finite-menu budget allocation instead of an arbitrary split

Fix one request, its domain tube, and an observation/error-weight contract.
Suppose component `i` has finitely many admissible options `j` with error
upper bounds `e_ij >= 0` and nonnegative costs `c_ij` in one declared unit.
These options may be polynomial degrees, different certified bases, root
stopping levels, or direct structural solves. Bounds must hold jointly on
the chosen request/tube; dependent components cannot be assigned unrelated
sample-based guarantees.

Let `W >= 0` map component errors into `p` required observation bounds,
let `r >= 0` contain already-owned error terms, and let `epsilon` be the
physical budgets. The finite planning problem is

\[
 \min_{j_1,\ldots,j_s}\sum_i c_{i j_i}
 \quad\hbox{subject to}\quad
 r+W(e_{1j_1},\ldots,e_{sj_s})^T\le\epsilon.                 \tag{E12}
\]

**Theorem E4 (safe componentwise frontier).** Starting with the zero
error-vector/cost label, extend one component at a time. For each partial
label add the relevant column of `W` times the candidate error and its
cost. Remove a label only if another has no greater cost and no greater
error in every observation coordinate, with at least one strict inequality.
If all coordinates and cost are equal, retain at least one representative
rather than deleting both labels. At completion, the least-cost
feasible remaining label solves (E12) exactly.

**Proof.** Without pruning, Cartesian extension generates every complete
combination. Any removed partial label has a dominating retained replacement. Completing
both with the same future options preserves componentwise dominance
because future contributions are identical and nonnegative. Thus any
feasible complete solution has a retained replacement with no greater
cost. Induction over components establishes the result. QED.

This is a finite deterministic planner, not a claim of cheap universal
global optimization. Its resource ceiling is caller-owned. A measured
cost table makes (E12) optimal for that table; it does not turn the table
into a future wall-time theorem. Operation counts can instead provide an
exact named work objective. Preparation, failed attempts and validation
must appear in the modeled cost boundary, including shared costs.

**Concrete exact example.** Two kernels have weights `(2,1)` and budget
`3/10`. Kernel A offers `(error,cost)=(1/10,4),(1/20,7)`. Kernel B offers
`(1/5,1),(1/10,3),(1/20,6)`. The exact optimum is A's coarse option plus
B's middle option: total error `3/10`, cost `7`. Equal physical halves
would require A's fine and B's middle option, costing `10`. These are
illustrative declared work units, not native VigilODE timing observations.

### Shared setup and session reuse constrain dominance

For stateful costs, labels must also carry a continuation state `s` that
fully determines admissible future options and their costs/bounds.
Dominance pruning is valid within the same such state, or when a proved
simulation relation permits every continuation of the dominated state
at no greater error/cost from the dominating one. Different numerical
states are not automatically equivalent; a uniform tube contract or an
explicit relation is needed before their labels can be combined.

Counterexample: a cold state costs `0` now and `100` at the next action;
a warm state costs `1` now and `0` next. Pruning the warm state solely
because its current cost is higher discards the optimum `1` in favor
of `100`. This is why polynomial coefficient caches, factorization keys
and winning native sessions belong to the integrated plan rather than
being treated as free interchangeable details.

For future adaptive steps with unknown states/weights, use (E12) only
within a frozen request/tube or supplied look-ahead contract. Do not
promote an offline fixed-grid optimum to an online universal policy.

## E5. A proof-derived continuous initializer and precision feasibility

The finite menu above makes the final decision. An analytic relaxation
can generate useful tolerance proposals when a declared work model has
the form

\[
 C_i(\eta_i)=a_i\log(B_i/\eta_i),\qquad a_i>0,
 \quad\sum_i w_i\eta_i\le B,\quad w_i>0.                    \tag{E13}
\]

Here `B` is the remaining budget *after* already-owned error terms;
`B_i`, `a_i` describe that work model, not fitted accuracy multipliers.
Assume first the unconstrained optimum lies in the model's valid range.
Since each cost decreases in `eta_i`, the constraint is active. Strict
convexity and stationarity `a_i/eta_i=lambda*w_i` give the unique optimum

\[
 \eta_i=\frac{B a_i}{w_i\sum_j a_j}.                         \tag{E14}
\]

For valid positive bounds `0 < l_i <= eta_i <= u_i`, first require
`B >= sum_i w_i l_i`; otherwise this bounded relaxation is infeasible.
The corresponding KKT solution
is `clip(a_i/(lambda*w_i), l_i, u_i)` with the active budget equation;
if all upper caps already fit, take them. The actual integer degrees or
iterations still require the original outward error test and finite
comparison. Equation (E14) proves an allocation for the stated relaxation,
not a universal hardware cost law.

For `a=(1,3)`, `w=(2,1)`, `B=1/2`, the allocation is
`eta=(1/16,3/8)`. Both KKT multipliers equal `8`. With `B_i=1`, its
product `eta_1 eta_2^3=27/8192` is larger than the equal-weighted-budget
product `1/512`; hence its logarithmic-model cost is smaller. All budget
and stationarity arithmetic here is rational.

### What a rounding floor does and does not prove

Suppose an implemented certificate has a fixed arithmetic allowance
`r_i` and a reducible contribution `eta_i`. With positive weights, its
available allowance is

\[
 B=\epsilon-\sum_i w_i r_i.                                 \tag{E15}
\]

If this is nonpositive, the *chosen enclosure decomposition* cannot
certify a strictly positive remaining allowance. This does not prove
that the mathematical solution or every binary64 algorithm is incapable
of the requested accuracy: an upper bound is not a lower bound. Refine
the enclosure, use an exact structural case, change the operation order
with a new proof, or raise precision; do not tighten root iteration
indefinitely under the same unresolved certificate floor.

An actual representability impossibility requires a certified lower
distance to the allowed representable outputs. In a fixed local lattice
`(1/8) Z`, the target `1/3` has nearest value `3/8` and distance `1/24`.
Tolerance `1/100` is unattainable on that lattice. By contrast, a loose
upper allowance `1` for the exactly representable target `1/2` gives no
such impossibility. The companion checker keeps these two statements
distinct. Real binary floating lattices vary with binade; their local
spacing/domain must be established before using this argument.

The algebra lane's inexact near-inverse recurrence supplies one specific
arithmetic-floor upper bound. Equation (E15) explains how that local
bound enters the common physical contract without relabeling it as a
universal lower bound on attainable state error.

## Computational contract and independent validation obligations

The future driver can consume a physical-step certificate with these
semantic fields, without requiring every backend to share an implementation:

- Original model, provider permission, input state, clock convention,
  chart, norm, requested observation and epoch identity.
- `anchor = ExactFlowAtNumericalInput | DiscreteMapAtExactInput`.
- The selected exact recipe and proof of its domain/stability factor.
- Bound and ownership of each error edge; original residual-to-endpoint
  conversion when the certificate is algebraic.
- Current prefix error, committed output and all work/failed-attempt records.

This is a mathematical contract proposal, not an added runtime API. The
current bool `CostEvidence::admissible` remains a consumer declaration;
the proposed fields explain the distinct evidence required for a stronger
physical-error admission. Tokens for `PhiAction`, `ShiftedSolve`,
`OriginalStageRoot` and `PhysicalStep` must not be substituted for each other.

The independent PR should validate at least:

1. Switching among independently admitted candidates on the same physical
   request, with discontinuous selectors, injected failures and rollback;
   the accepted prefix must obey the declared induction and work accounting.
2. The anchor-mixing counterexample, correct PR93 cubic nonexpansive path,
   and a separately derived numerical-input flow-defect path if implemented.
3. Physical norm conversions and changed norm/chart epochs; deliberate
   coordinate-to-physical amplification must not be silently omitted.
4. Internal versus physical-label clocks, output-only displacement, nonexact
   `t+h`, endpoint assembly, subnormal/overflow and nonautonomous time shifts.
5. Original residual leaves from PR90--93, including full-state represented
   structural defects, and the algebra compiler's omitted-stage certificate.
6. Exact finite-menu optimum against exhaustive enumeration on small cases;
   cache/session states must prevent invalid pruning. Measured-cost optimality
   remains distinct from measured elapsed speedup.
7. A certificate floor that can be improved, a true local representability
   floor, and original-target rejection without arbitrary tolerance widening.

`CHECK.py` provides exact finite examples for E1--E5, the invalid anchor
combination, norm and clock accounting, allocation, and precision distinction.
It uses only the Python standard library and prints JSON. Its source is to
be registered by the parent before execution. Native outward arithmetic,
validity of model-specific tubes, callback side effects, nonlinear order
and deployment performance remain the subsequent implementation obligations.
