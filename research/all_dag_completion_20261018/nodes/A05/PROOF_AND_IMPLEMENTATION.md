# A05 — capability-bound generic continuous-residual acceptance

## Contract and result

The previous `Pr101Model::integrate_verified` was a model-specific driver. This node adds a single dimension-independent transaction in `generic_verified.rs`, shared by immutable PR101 and collective model owners, and open to additional **crate-owned** sealed providers. `DenseErrorControl::VerifiedResidual` is an explicit public request; arbitrary point-only `OdeProblem` calls return `UnsupportedUnboundModel`. The executable entry accepting an actual capability is `integrate_verified_residual`.

The endpoint candidate remains RODAS5P with the input `atol` and `rtol`. The factory-owned explicit Jacobian selects the direct stage solver; JVP-only owners select nonrecycled GMRES with the existing default linear tolerances. Continuous verification controls the final error regardless of candidate inner accuracy. A positive absolute tolerance is used for a stronger global acceptance contract. `initial_radius` denotes a Euclidean initial ball, not one radius per component. Finite-dimensional real state, identity mass, forward finite span, C1 state field and a complete outward interval/logarithmic-norm provider are assumptions. The represented time differences admitted by `Rodas5pQuarticSegment` are exact positive normal binary64 values. Subnormal/nonexact interval widths are refused rather than silently altering the clock.

No new literature-priority claim is made. The previously reviewed local defect comparison theorem, PR101 cancellation-preserving residual and collective interval provider are reused; the new result is their dimension-independent transactional composition and output contract.

## Lemma 0: cancellation-preserving interval extension

The real stored quartic admits the exact power representation

q(θ)=y₀+θ(a+θ(b+θ(c+θd))),
a=y₁−y₀+d₀, b=d₁−d₀, c=d₂−d₁, d=−d₂.

Its physical derivative is (a+θ(2b+θ(3c+4θd)))/h. Each coefficient is formed with outward arithmetic before Horner evaluation. Inclusion follows by induction on the nested additions/products. This is the same real quartic, with changed interval dependency only; the binary64 center evaluator is unchanged.

The former natural blend has width ≥O(|y₀|) on θ∈[0,1] even for constant q, because the two copies of θ are treated independently. The new expression returns exactly the constant for a=b=c=d=0 (subject to the directed primitive contract) and an O(|y₁−y₀|) width for affine q. For the explicit affine fixture y₀=10⁸,y₁=10⁸−1/8,h=1/8, the value width is below 0.126 and the derivative enclosure width below 10⁻¹² around −1. This removes a concrete h-refinement obstruction: the previous collective damping witness exhausted minimum step after 43 attempts; the repaired interval extension accepts 30 segments with 20 rejections at unchanged tolerance.

## Lemma 1: full-tube local error and signed gain

Let q(t) be the real quartic defined by the stored endpoints and dense coefficients, and let exact x solve x'=f(t,x). On the entire closed tube ||v-q(t)||₂≤R, require μ₂(D_y f(t,v))≤μ and ||q'(t)-f(t,q(t))||₂≤δ. If ||x(t₀)-q(t₀)||₂≤r, then the upper right derivative obeys

D⁺||x-q||₂ ≤ μ||x-q||₂+δ.

For nonzero error, this follows by differentiating its norm, applying the fundamental theorem of calculus to f(t,x)-f(t,q), and bounding the symmetric Jacobian quadratic form by μ. At zero error the upper Dini derivative is bounded by δ. Scalar comparison gives

b(s)=exp(μs)r+δ ψ(μ,s),   ψ(μ,s)=(exp(μs)-1)/μ,   ψ(0,s)=s.

b solves b'=μb+δ and is monotone, including μ<0 where it approaches -δ/μ. Thus max_{0≤s≤h}b(s)=max(r,b(h)). A verified `max(r,b(h))≤R` closes the tube by continuation/invariance on the compact tube. The model providers are smooth on a neighborhood of this tube; no point-sampled derivative substitutes for the full-tube premise.

`verify_continuous_residual_tube` uses signed directed exp and integral bounds; `Pr101Model::verify_segment` uses its polynomial/trigonometric defect enclosure before applying the same comparison. The generic adapter retains separate endpoint and uniform bounds. For the collective adapter endpoint is the outward sum of transported initial and defect terms, not the larger uniform radius. Consequently a contracting trajectory does not accumulate the larger uniform radius as a fictitious endpoint error.

Dimensions: μ has inverse time, δ has state/time, ψ has time, b and R have state. The limits μ→0 and δ=0 recover r+δs and exp(μs)r. For μ<0 the sign is retained, preserving damping.

## Lemma 2: continuous global induction

Assume segment k has certified start radius r_k and the returned bound E_k at its endpoint. Construction binds q_k(t_k)=the actual binary64 candidate start and q_k(t_{k+1})=the actual binary64 candidate endpoint. The driver commits that same endpoint and sets r_{k+1}=E_k. Lemma 1 therefore applies inductively to every committed interval, with no independent restart of the exact initial condition.

The selected trial tube is at most atol/2. Its elapsed-time headroom schedule is merely an acceptance proposal and carries no error authority; it may be computed in ordinary binary64 because the resulting represented radius is checked by the outward verifier. It avoids spending the full global budget at the beginning of an expanding PR101 trajectory. If this schedule is insufficient, attempts reject and the returned run reports incomplete execution without certifying missing intervals.

For each stored segment, ||x(t)-q_k(t)||₂≤atol/2 at every real time in its span. This statement is established by the interval residual and log-norm proof. Extra midpoint samples are regression probes, not its justification.

## Lemma 3: returned-center enclosure at exact requested times

For requested represented time τ, compute binary64 center c=q̂(τ) and interval Q enclosing the real stored polynomial q(τ). Let d_i=mag(Q_i-[c_i,c_i]) and D=sqrt_up(sum_up mul_up(d_i,d_i)). The output radius is add_up(uniform_radius,D). Triangle inequality implies ||x(τ)-c||₂≤radius. The same scalar radius bounds every individual component. Requiring radius≤atol is stronger than the supplied conventional absolute-plus-relative tolerance for each component.

The API passes physical τ directly into both evaluators, avoiding a round-trip τ→θ→τ. Adjacent representable output times remain distinct. Exact endpoints use immutable stored state copies. Pure-relative tolerances are explicitly refused until a verified scale is implemented.

## Lemma 4: transaction and work conservation

The candidate `sequential_step` is stateless here (no accepted history or recycled Krylov state). Candidate endpoint status counters are normalized while every other RHS/JVP/Jacobian/factor/solve work item is retained. Curve construction, interval workspace, provider primitives, residual/gain operations and output evaluation all update the verification ledger before fallible work as applicable.

The transaction stages the candidate, full-tube certificate, endpoint vector, all outputs due in the interval, and next output index. A refusal leaves time, state, inherited radius, committed segments and committed outputs unchanged. Successful verification commits all of them together. Each attempt receives exactly one final accepted or rejected disposition. Allocator OOM abort behavior of ordinary Vec growth is unchanged from the surrounding crate and is not marketed as a recoverable scientific rejection.

## Code and capabilities

- `VerifiedContinuousProblem`: private `Box<dyn VerifiedModelProvider>`; factory owns problem and provider.
- `VerifiedModelProvider`: crate-private interface; `problem`, `validate_span`, `verify`; no external forged implementation can obtain authority.
- PR101 factory preserves its immutable λ,η target and cancellation provider. The old PR101 driver remains byte-for-byte in its algorithm; only candidate factory visibility becomes crate-private.
- Collective factory retains the same private `Arc<CollectiveModel>` in generated callbacks and interval provider. A crate-private accessor borrows its generated problem without exposing it to external callers.
- `VerifiedResidualSegment`: private real curve and radius; `evaluate` recomputes the returned-center enclosure.
- Public copied diagnostics and serialized outputs are evidence records; they cannot mint new segment owners.
- Existing generic drivers retaining `Off`, `ReportDefect`, `Report`, `Enforce` retain their existing semantics. This new capability route is opt-in.

## Cost and performance interpretation

For d state coordinates and m outputs, each attempt charges candidate direct algebra when an explicit Jacobian is owned, otherwise nonrecycled GMRES/JVP work, plus residual provider work, O(d) curve/evaluation arithmetic per output and construction. The collective interval box residual can overestimate through dependency and need smaller steps; a cancellation-preserving vector polynomial provider is a subsequent speed optimization, not a missing requirement for soundness. The current generic driver is intended as a usable verification baseline; no zero-allocation or measured speed claim is made. Test/native work and full verifier ledger must be included in comparisons with Off/Enforce.

## Closure criteria and next local execution

1. Original PR101 λ=-10⁶,η=10, [0,1],101 outputs,atol=10⁻¹¹,rtol=10⁻⁸ through the new generic route: all requested radii≤atol and observed sin-reference discrepancy within radius; between-output validity follows Lemmas 1–3.
2. A vector collective problem through the same generic route, with every coordinate checked against its simple analytic orbit.
3. Rejection leaves only the initial committed output and retains all work.
4. Invalid zero-span config, invalid model domain and pure-relative request refuse.
5. An arbitrary point `OdeProblem` cannot request verified authority.
6. Large-offset exact-affine enclosure and nonlinear unit-circle cubic orbit regressions.
7. Existing 15 A05 callback/zero-span contracts and original PR101 tests remain valid.

Commands and actual results are recorded in `RESULT.json` and `evidence/`. Independent review is separate from execution. General unknown models require a new sealed interval provider; their absence produces a typed capability refusal rather than a sampled certificate.
