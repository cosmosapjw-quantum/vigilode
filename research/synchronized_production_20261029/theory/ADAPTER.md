# Adapter/switcher: ownership, competitive work and clock–fiber certificates

Source: PR101 `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`; audited 2026-10-29. This unit uses the research-loop 1.0.0 contract. R1–R10 evidence is indexed in `ADAPTER_WORKFLOW.json`. New mathematical claims below are derived under their explicit hypotheses. New native implementation evidence is recorded separately after theory admission.

## 1. Actual bottleneck and reuse

The reentrant comparison in PR101 compares **different policies**, not identical algorithms with and without a wrapper. `examples/support/production_return_cases.rs::config` maps `rhs-only` to derivative fallback Forbidden, and `observed-strict-sentinel` to competitive ProvidedOnly switching with strict integer comparisons and explicit return sentinels. The former uses `persistent_explicit`; the latter runs speculative entry/return replays and changes accepted trajectories. Its work ratio 24440/17113 and errors 6.136e-8/4.084e-8 therefore combine policy and numerical-path effects. PR101's exact baseline/candidate parity is *within each policy*. The 2.420 guest-instruction slope ratio is a whole-process diagnostic, not a certified minimal cost.

The current call path is `SwitchingSolver::solve → drive_windows → run → Rodas5p::solve_observed_bounded → routed/native driver`. Contiguous explicit tenure already preserves its controller, FSAL derivative and workspace through `Dopri5::with_session`. Committed implicit steps end that explicit tenure. Every implicit batch calls the facade afresh, rebuilding its workspace. The new safe candidate identified here is narrower: consumed return data are copied twice by the facade, independently of integration arithmetic.

PR90–93 already supply bounded work comparisons, proposal reuse and speculative-accounting safeguards. PR98 supplies guarded ownership exchange for the coupled controller; the same proof pattern applies to facade output ownership, with different live fields. PR100's signed defect and continuous-path certificates provide the general defect mechanism. This unit contributes (i) the exact consumed-output refinement obligation, (ii) the separation of local competitiveness from global cost, and (iii) a clock–fiber certificate specialized to the actual reentrant nonlinear benchmark. It does not restart the preceding proofs or reset their budgets.

| Observation | Classification | Code consequence |
|---|---|---|
| Routed `partial.clone()` and `run.observed().clone()`, followed by `observed.t/y.clone()` | Adapter/export ownership | Move owned fields while preserving external value observations |
| Repeated native implicit construction | Adapter/session design | Requires a separate epoch and live-workspace contract; do not silently carry a Jacobian/controller across starts |
| Extra successful/failed competitive probes | Requested switcher policy | Charge them; same-window winning cost does not bound total policy overhead |
| Large clock column in the manufactured Jacobian | Coordinate-sensitive numerical bound | Use clock–fiber comparison; retain clock defect instead of declaring whole-state contraction |
| Arnoldi/MGS work inside JVP solve | Algorithm/numerical kernel | Separate issue; no attribution to copies without component evidence |

## 2. Consuming ownership refinement theorem (A1)

Let a completed driver return an owned Rust value `R=(O,P,M)`. `O` contains the observed time vector, nested state vectors, message and scalar telemetry; `P` is an optional partial accepted state; `M` is routing metadata. Let a facade compute `F(R)` by cloning `O/P`, then computing status, choosing published rows, constructing Stats and locating events. Consider the transformed facade `F_move` which consumes those fields instead, with these obligations:

1. Driver execution, callbacks, configuration, guards and arithmetic occur before this transformation and are identical.
2. Every scalar/enum/string read needed to compute status and `StopRecord` is done before moving its owner, or from an equivalent remaining field.
3. For non-dense output, the exact same owned `t/y` rows are transferred. A partial row is appended iff the original predicate `last_t < partial.t` holds; its `y` is transferred. Empty/zero-span output and equal-time partial rows keep the original behavior.
4. Dense-extension storage remains live in the returned Solution; rows needed both by the extension and exported output still require distinct ownership. No borrowing lifetime is extended.
5. Event location runs with identical row values, extension, status and guard state. Callbacks may carry counters, recoverable/fatal behavior and other declared effects: they need not be pure. The same sequence of numerical input values from the same initial callback state must yield the same callback effects. Code deliberately conditioned on a slice's allocation address is outside this numerical equivalence contract. Allocator addresses/counts and allocation-failure/panic behavior are outside the successful-execution theorem; Rust moves can alter which allocation reaches an event callback.

**Theorem.** Under 1–5, `F(R)` and `F_move(R)` have identical serialized/public value observations: every floating-point bit, status, metadata, event result and callback trace. No rounding-error term is introduced.

**Proof.** A Rust move transfers the existing finite sequence of element bit patterns and does not apply floating arithmetic. Cloning those sequences gives an elementwise identical sequence. Thus at the first differing ownership operation both implementations satisfy the relation: equal values for all live scientific fields, potentially different allocation identities. Each subsequent branch predicate reads equal values, so branches agree. Appending the partial row is selected by the same predicate and appends equal elements. Scalar metadata and status construction therefore agree. Dense output is unaffected by 4. Event location receives equal value inputs and the same callback state. Induction over event calls, using5, preserves both callback effects and results. The driver callback history was already identical by1. Induction over the remaining statements proves the observation equality. This is a program refinement proof, not a sampled numerical approximation. ∎

**Savings.** For a routed non-dense success with m observed rows of dimension n, two existing deep clones transfer m×n state coordinates and m times each. Moving removes those copies and their allocated vector copies. Partial-state clone savings depend on whether a partial is present and appended. Current dense-output paths use the FastDense/Sequential plans, so no saving from a routed clone is attributed to those paths. Their extension-to-output copy remains because both representations are returned. Wall-time savings depend on the share of total solve cost; callback work and accepted steps must be unchanged.

**Implementation contract.** In `crates/vigilode/src/solve.rs`, first obtain driver scalars by borrowing the routed result; then consume `RoutedRun` with a match to extract its observed field and move `routed.partial`. Before output projection, construct status and stop metadata; the non-dense arm takes `run.observed.t/y` and moves a partial row only under the unchanged append condition. Do not add a public API or alter the router, method, norms, tolerances or callback ceilings.

## 3. Epoch and scheduling refinement (A2)

Model a numerical session state as `S=(x,t,h,c,f,w,e,q,b)`: accepted state/time, step proposal, controller history, FSAL/cache data, workspace, epochs, output schedule and remaining budget. A legal pause emits observations without mutating the state. With fixed model/provider/norm/tolerance epochs, identical schedule, callback behavior and floating-point environment, dividing a sequence of attempts into quotas leaves each transition unchanged. Induction on attempts gives the same accepted/rejected words and final scientific state. This is precisely the fixed-session claim used by `ExplicitSession::advance_to`.

A method switch is a different transition: an implicit accepted state invalidates the earlier explicit FSAL derivative/controller provenance. Therefore storage reuse can retain capacity, but **numeric cache reuse requires its own validity proof**. A persistent implicit implementation must either reset every dependency-sensitive cache as fresh construction does or verify a matching operator epoch and the driver's cache validity conditions. Merely matching dimension is insufficient. This unit implements output ownership only, where the theorem needs no new epoch mechanism.

## 4. Strict comparator theorem and total policy cost (A3)

Use the current checked integer objective

`W = N_rhs + N_jvp_vectors + N_ft + n N_jacobian_builds + N_linear_solves`.

All summands are nonnegative; overflow yields unavailable evidence, never a cheaper result. Let C be a completed admissible candidate's cost. Let L be an incumbent replay's already charged cost, and I the cost it would have if continued under the same deterministic driver. Then `L≤I`. If `C<L`, the candidate is strictly cheaper than that completion. A comparator may stop when its callback-only prefix first exceeds C; its full prefix cost is at least that callback prefix, hence `C<L≤I`. No partial incumbent state is used as an accepted endpoint. Ties retain the incumbent.

For entry, an already accepted explicit prefix with cost E can be challenged by a replay only while it may have C<E. A callback prefix L≥E makes that strict win impossible. The implementation's cap E−1 and charged crossing callback are exactly this integer boundary. These conclusions are local to identical physical start state, requested interval, cost unit and permission contract.

**Global accounting identity.** Let committed productive segment costs be P_i and discarded/failed speculative costs be D_j. The actual policy cost is `W_policy=ΣP_i+ΣD_j`, while a separate RHS-only solve has its own trajectory and cost W_rhs. Local comparisons only constrain selected P_i against their same-window comparators. They impose no inequality between `W_policy` and W_rhs. Consequently the preserved strict/RHS loss does not contradict the strict-comparator theorem. It identifies a scheduling/trajectory optimization problem.

A useful future amortized guard is explicit: if a policy admits speculative episodes only while cumulative D≤βΣP+C0, then by direct addition `W_policy≤(1+β)ΣP+C0`. This bounds overhead relative to the policy's own productive trajectory; proving a competitive ratio against another solver additionally needs a relation between ΣP and its trajectory. Such a guard changes policy and is not introduced in the present value-preserving patch.

## 5. Clock–fiber structure of the reentrant ODE (A4)

The benchmark evolves an autonomous clock x_0 and physical components x_j:

`x_0'=1`, `x_j'=−λ(x_0) φ(x_j−g(x_0))+g'(x_0)`, `φ(e)=e+e³/2`.

For x_0(0)=0, define the moving-carrier variable `e_j=x_j−g(x_0)`. The chain rule gives exactly `e_j'=−λ(x_0)φ(e_j)`. This cancellation is code-specific: the RHS contains g′ and the clock derivative is one. It holds for the current smooth tanh/sine/cosine definitions and any C¹ carrier and positive continuous λ.

The benchmark's clock, time and state variables are normalized and dimensionless. For dimensional variables the cubic coefficient must carry inverse-state-squared units: write `φ(e)=e+γe³`, γ≥0, and replace B+B³/2 by B+γB³. λ and κ have inverse-time units, L is the rate derivative with respect to the clock and g′ is carrier per clock. Every term in the defect comparison then has state/time units.

For any real a,b,

`(φ(a)−φ(b))/(a−b)=1+(a²+ab+b²)/2 ≥1`,

with the diagonal defined by continuity. Since `a²+ab+b²=(a+b/2)²+3b²/4≥0`, this is a complete global monotonicity proof. Two fiber solutions driven by the same clock satisfy

`D⁺|e_1−e_2| ≤ −λ(x_0)|e_1−e_2|`.

If λ≥κ>0 on a slab, their separation contracts by exp(−κh). This does not imply contraction of the full autonomous state: its clock direction is neutral and can force fibers through λ and g. The correct comparison retains that direction.

### Defect and clock error

Let a C¹ numerical reconstruction `hat x` have original-coordinate defect `ρ=hat x'−F(hat x)`. Its transformed fiber satisfies

`hat e_j'=−λ(hat x_0)φ(hat e_j)+ρ_j−g'(hat x_0)ρ_0`.

Let c=hat x_0−x_0 and d_j=hat e_j−e_j. On a validated slab/tube assume:

* λ(x_0)≥κ>0;
* λ is Lipschitz with constant L on the clock interval containing x_0 and hat x_0;
* |g′|≤G there, and |hat e_j|≤B;
* |ρ_0|≤δ_0 and |ρ_j|≤δ_j throughout the slab;
* initial bounds |c(0)|≤C and |d_j(0)|≤E_j.

Then `|c(s)|≤C+sδ_0`. Subtract the fiber equations using λ(x_0) for the contraction term. The remaining parameter forcing is

`[λ(x_0)−λ(hat x_0)]φ(hat e_j)+ρ_j−g'(hat x_0)ρ_0`.

Since |φ(hat e_j)|≤Φ(B)=B+B³/2, scalar comparison gives the rigorous bound

`|d_j(s)| ≤ e^(−κs) E_j + A_j (1−e^(−κs))/κ + L Φ(B) δ_0 (κs−1+e^(−κs))/κ²`,

where `A_j=δ_j+Gδ_0+LΦ(B)C`. The formula follows by integrating the forcing bound `A_j+LΦ(B)δ_0 s` against the positive kernel exp(−κ(s−u)). Original-coordinate error obeys

`|hat x_j(s)−x_j(s)|≤|d_j(s)|+G(C+sδ_0)`.

This includes clock defects, carrier magnification and rate sensitivity. If δ_0=C=0 it reduces to the usual signed contractive scalar bound, which can be much smaller than applying a full-space absolute Jacobian norm dominated by the clock column. The improvement is a derived bound under tube conditions, not a change to the solver's acceptance test.

For the actual reentrant case let `q_1(t)=tanh(200(t−1/4))`, `q_2(t)=tanh(200(t−3/4))`. Since tanh is increasing, q_1≥q_2 and `λ=1+(9999/2)(2−q_1+q_2)` satisfies `1≤λ≤10000` on the real line. Each `q_i′` lies in [0,200], so `|λ′|≤(9999/2)200=999900`; bounding their difference does not require a factor2. For `g(t)=cos t+(1/100)sin150t`, `|g′|≤1+3/2=5/2`. Thus κ=1, L=999900 and G=5/2 are explicit analytic choices. The exact initial positive fibers lie in [0.26,0.40] and decrease while positive, so the true fibers remain in that interval's nonnegative hull. A bound B for the reconstructed fiber still requires its own tube calculation. Tighter local rate/derivative bounds are useful near the transition-free regions and must use outward transcendental enclosures in a floating-point certificate.

### Slab composition and physical norm

At a method boundary use the published endpoint to set the next C and E_j. Define J_0 as the clock jump and J_j as the jump in the transformed fiber coordinate; add those bounds before the next recurrence. If available jumps D_j are instead in original x coordinates, use `J_j≤D_j+G D_0` by the mean-value theorem. For exact bit-preserving ownership transfer all jumps are zero. Repeating the monotone comparison map proves a bound for any legal sequence of explicit/implicit segments. With a fixed physical linear map A, `||A Δx||∞≤max_i Σ_j |A_ij| U_j`, where U_0=C+hδ_0 and U_j bound the coordinate errors above. For a weighted component norm use its actual fixed weights. An estimator alone is not δ: rigorous whole-slab defect, rate, carrier and rounding bounds are required to instantiate this theorem.

The time-dependent envelope need not attain its maximum at a sampled endpoint when terms compete. A safe whole-slab bound is `E_j + A_j h + LΦ(B)δ_0 h²/2` plus `G(C+hδ_0)`; a tighter certificate can bound the derivative or subdivide. This avoids using an endpoint error as a whole-slab bound.

## 6. Independent checks and scientific scope

Wolfram independently returned `(2+e²+ex+x²)/2` for the divided difference, zero for the constant-forcing envelope residual, and `(−1+exp(−κt)+κt)/κ²` for the clock-forcing convolution. The raw request/output is in `evidence/ADAPTER_WOLFRAM.json`. These symbolic checks support the algebra; the inequalities and composition are proved above. Independent agent `/root/state_audit` inspected the proof and actual source, confirmed A1/A3/A4 under their assumptions and independently checked the clock forcing sign and convolution. Its two clarifications—jump coordinate basis and actual reachable dense route—are incorporated above before code admission.

SciSpace found Söderlind's logarithmic-norm theory and Enright/Hayes' defect-control research. The Lund author record confirms the former's differential-inequality/error-bound setting. The original Enright/Hayes PDF confirms that interpolant defects and their roundoff behavior are distinct from ordinary local-estimator control. They contextualize A4; A4's exact skew product is derived from this repository's model. Source leads and abstracts are in `evidence/ADAPTER_SCISPACE.json`; no claim relies on an uninspected theorem from those papers.

Primary sources: [Söderlind 2006, author record](https://lup.lub.lu.se/search/publication/67c82d12-1002-46bd-8084-d81f7409792d), DOI10.1007/s10543-006-0069-9; [Enright and Hayes 2007, author PDF](https://www.cs.toronto.edu/~wayne/research/papers/enright.pdf), DOI10.1145/1206040.1206041. Search date2026-10-29. Existing papers are reused for context; this unit claims a repository-specific derivation and refinement, not a new general logarithmic-norm theorem or strong no-go result.

## 7. Coding admission and local extension obligations

Admit A1's consumed-output patch after independent review, with tests covering routed dense/JVP, partial max-step/fatal/ceiling outcomes, zero span, vector-atol direct route and dense-event preservation. Require exact parity of returned row bits, complete Stats serialization, status, callback counts and switching decisions under identical inputs. Allocation/timing must cover the same complete solve/export/drop scope in both arms and retain adverse measurements. The theorem predicts less copying, not fewer RHS/JVP calls.

A4 is a proved conditional certificate contract. A future native implementation must provide outward enclosures for tanh/sin/cos rates/carrier, the numerical reconstruction and whole-slab defects, including the clock. It can first be tested with polynomial/rational manufactured carrier and rate, using exact Fraction bounds. A5 implicit workspace reuse remains a separate implementation obligation: capacity retention alone is safe after complete reset; reuse of numeric factors or Krylov data requires operator validity evidence.
