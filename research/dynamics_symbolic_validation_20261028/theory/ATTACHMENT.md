# Attachment-to-PR97 theory review and symbolic-dynamics contract

Date: 2026-10-28. Reviewer: /root/attachment_theory. This is a theory-first, read-only review of attachment equations and the corresponding PR97 implementation at commit 87465835d9d21298d235917f036580b0b53ab645. It does not create fresh native campaign evidence or close the production gates.

## R1–R10 record

| Stage | Status | Concrete output |
|---|---|---|
| R1 | complete | Preserve original physical target, norm, clock, 1e-8 attachment cap, branch semantics and correction budgets. |
| R2 | reused | Read F1, F23, F4, F5, F6 derivations, F6 code, review record; compare PR97 CLOSURE.md, TRACE.md and native modules. |
| R3 | complete | Separate selected-branch cyclic DAE from all-complex multiplicity trace, and discrete routing words from continuous-flow symbolic models. |
| R4 | complete | Tested a cheaper rational residual bound as candidate; rejected it as a new contribution because PR97 already has a stronger safe envelope. |
| R5 | complete | Independently derive residual transport, rational envelope, guard-language induction and contractive finite abstraction condition below. |
| R6 | complete | Separate child /root/attachment_theory/bound_review derives the scalar envelope and checks the unsafe shared-denominator counterexample. Source-level mapping confirms PR97 already implements the stronger max-envelope. |
| R7 | complete, scoped | Current-turn Python executes five explicit discriminant cases in attachment_theory_checks.json. Child reports 1,040 Decimal160 checks, but raw receipt was not saved after ENOSPC; these are not promoted into replayable test evidence. |
| R8 | complete | Reuse existing PR97 radius implementation; route-language acceptance requires original-flow edge certificates; no new strong no-go claim. |
| R9 | complete | Coding contracts and exact formulas below. |
| R10 | complete, scoped | This source review is ready to merge into parent theory report. Native execution and full request completion remain parent obligations. |

Read the physmath skill and RESEARCH_POLICY.md, THEORY_LOOP.md, EVIDENCE_AND_HANDOFF.md and RUNTIME_AND_LEGACY.md. No prior runtime failure was inherited. Python executed successfully in this turn.

## Preserved attachment mathematics

F23 fixes G_i=q_i+q_(i+1)^2/4-x_i and x'_i=-x_i+(q_i+q_(i+1))/16+c_i on X=[-1/8,1/8]^n, Q=[-1/4,1/4]^n. Its contraction gives root sensitivity 8/7; reduced flow has contraction lambda=6/7; original residuals give delta=Df+Dg/7 and algebraic error at most (8/7)(E+Dg). These identities and the O(n) cyclic tangent circuit are already admitted in PR97 CLOSURE.md and cyclic_closure.rs. Reimplementing them would duplicate completed work.

F6's monic quartic P=z^4-2az^2+z+a^2-b defines a rank-four quotient for every complex a,b, including the discriminant. Newton sums calculate polynomial full-quotient traces with algebraic multiplicity. For example S2=4a, S4=4a^2+4b, S6=4a^3+12ab+3. PR97 trace_quotient.rs retains explicit all-complex-multiplicity semantics and rejects selected-real and real-only requests. The sum with multiplicities is the target itself; it cannot replace a selected physical branch under an unchanged target identity.

F1's u/v primitive charts have the same determinant d=4uv-1. Their transitions preserve branch identity only with the common-domain uniqueness/residual arguments. A global eliminant discriminant can vanish while the selected root is regular. Neither chart choice resolves a selected-root singularity.

F4 proves the stated local cuspidal-butterfly germ under positive square roots and explicit analytic IFT/front conditions. It supplies potential singularity-aware coordinates, not a complete solver for crossing arbitrary singular branches. F5's extra horizontal chart invalidates unconditional inherited 116/58 totals; 120/60 stays conditional on remaining chart completeness, and the full D=0 stratum remains open. Neither unresolved count is an input to the quartic trace algebra.

Attachment numerical counts and timing remain inherited evidence. The old F6 full timing ratio is approximately 1.29 in sequential batches, with certificate work dominant; it does not establish a robust general solver speedup. The old F23 representative oracle checked selected slabs, not every possible binary64 input or all platforms.

## Independent residual-envelope derivation and PR97 reuse decision

Assume a differentiable represented path, incoming radius E>=0, whole-slab residual delta>=0 and a logarithmic-norm bound mu(Df)<=-lambda<0 on the entire certified tube. Comparison gives

    b(t)=exp(-lambda*t) E + [1-exp(-lambda*t)] delta/lambda.

For r=lambda*h>=0, exp(-r)<=1/(1+r). Also exp(-r)>=(1-r/2)/(1+r/2): for r>=2 the right side is nonpositive; for 0<=r<2 differentiate the log of (1+r/2)/(1-r/2). Since b(t) is monotone toward delta/lambda,

    U1=(E+h*delta)/(1+r)
    U2=((1-r/2)*E+h*delta)/(1+r/2)
    Eend=max(U1,U2)
    Ewhole=max(E,Eend)

are valid. U1-U2=h*r*(lambda*E-delta)/[2(1+r)(1+r/2)], so max selects the correct exponential side without a floating comparison of E versus delta/lambda.

PR97 cyclic_closure.rs::physical_radii already implements exactly this Tight recipe with outward interval operations. For its dyadic h<=1 and lambda=6/7 all coefficients in U2 remain positive. Its zero_initial_radius_does_not_drop_positive_forcing test explicitly guards the important failure: E=0,lambda=h=delta=1 gives shared-denominator U1=0.5 whereas b(1)=1-exp(-1)>0.632.

An independently derived positive-coefficient bound

    E/(1+r) + h*delta/(1+r/2)

is also valid, since g(r)=r-2+(r+2)exp(-r) has g(0)=0 and g'(r)=1-(r+1)exp(-r)>=0. However it is weaker than max(U1,U2). It is retained here as independent analytical support, not introduced as a performance change.

The implementation must continue using whole-path residuals and outward arithmetic. A sampled residual, endpoint-only tube or scalar endpoint enclosure cannot replace those inputs. For lambda=0 the limit is E+h*delta; for growing/noncontractive systems this specific negative-lognorm recipe has no authority.

## Symbolic dynamics: two distinct constructions

### A. Guarded language of numerical route decisions

Let an event contain route, outcome, target identity, checkpoint identity, model/provider epoch, clock, physical norm, certificate identity and charged work. A finite control state records current route and whether an admissible trial may commit. Continuous numerical data remain guard parameters.

Use symbols such as PROPOSE, REJECT, ACCEPT, COMMIT, INVALIDATE. A word is admitted only if every COMMIT is preceded by an ACCEPT for the same immutable candidate and checkpoint, every ACCEPT has a valid original-flow certificate, and every epoch/target/clock change invalidates relevant reuse before another ACCEPT. REJECT retains its consumed work; no rejected state becomes the physical checkpoint.

Induction on the accepted word proves the same physical trajectory radius contract as individual certified edges: base case is the initial checkpoint enclosure; each accepted edge applies its defect propagation and jump/clock transfer; rejected trials leave this state unchanged. The selector may be arbitrary or heuristic because safety comes from the admitted edge, while the selector determines cost.

This is a concrete safety language for solver execution. It does not identify a Markov partition of the ODE, calculate physical topological entropy, or prove a bisimulation of a chaotic flow. Existing SwitchingSegment fields already distinguish probes, committed states, reached endpoint, success, work and typed evidence; an offline word validator can consume them without inventing physical stiffness labels. Reuse of a failed route can only be skipped on cost/scheduling grounds or exact matching input evidence; a failure at one checkpoint does not prove mathematical failure at the next.

A performance change based on this language must show the eliminated work on matched original target and accuracy. Validation overhead belongs in total cost. Merely adding an automaton is not a performance result.

### B. Finite abstraction of an actually contractive flow

For a fixed autonomous contractive flow on a compact forward-invariant domain, suppose

    ||Phi_h(x)-Phi_h(y)|| <= exp(-lambda*h)||x-y||.

Let a lattice quantizer have error eta/2 and let a numerical transition have a validated endpoint error nu. If a lattice successor p is selected from the numerical endpoint for lattice state q, then for ||x-q||<=epsilon,

    ||Phi_h(x)-p|| <= exp(-lambda*h)*epsilon + nu + eta/2.

Thus exp(-lambda*h)*epsilon+nu+eta/2<=epsilon preserves the relation by one step. This gives an implementable precision budget connecting continuous contraction, numerical certificate and symbolic quantization. A complete approximate-bisimulation claim additionally requires appropriate transitions and total matching in both directions; the displayed induction alone gives the stated forward simulation property.

This construction can support a future state-space exploration of the bounded cyclic or trace client. It does not automatically apply to arbitrary stiff, nonnormal or chaotic production targets. For timed/nonautonomous trace control, time is part of the state/transition contract; compactness and contraction must be checked for the actual sampled family.

## Coding handoff from this review

1. Keep PR97 Tight radii and all trace/selected-branch semantics unchanged unless a new independent error is found. No correction-budget attempt is consumed by the discarded duplicate derivation here.
2. Any new guarded selector must carry target/checkpoint/epoch/clock/norm through proposals and validate them at commit; rejected work is retained. Tests should falsify stale epoch, wrong observable, changed clock and committing a failed probe.
3. Any finite physical abstraction must expose epsilon, eta, h, nu and a proven lambda; check the displayed budget with outward arithmetic. Do not use empirical event frequencies as a surrogate for contraction.
4. Run performance comparisons only after the parent selects a concrete implementation. Use same original path target, physical cap and complete candidate+certificate+commit time. Keep source replay and current native runs distinct.
5. Preserve 23-node/34-edge inherited DAG, 8 suites, 73 paths, 10 interactions and Q-LEDGER open state independently of this local review.

## Literature and provenance

SciSpace was actually called with the full question about rigorous ODE error bounds, logarithmic norms, residual transport and finite symbolic abstractions. Its unmodified response is attachment_literature_raw.json. It is discovery evidence; theorem authority below comes from primary texts subsequently opened and inspected.

- Arnold Neumaier, Global, rigorous and realistic bounds for the solution of dissipative differential equations, Part I: Theory, Computing 52 (1994), DOI 10.1007/BF02276880. Author PDF https://arnold-neumaier.at/ms/ode.pdf, Corollary 4.5, formula (30), pp.22–23 of the PDF: residual and initial-error propagation through a logarithmic-norm bound. The project-specific rational majorants above are derived explicitly here and in PR97.
- Giordano Pola, Antoine Girard, Paulo Tabuada, Approximately bisimilar symbolic models for nonlinear control systems, Automatica 44 (2008), DOI 10.1016/j.automatica.2008.02.021. https://arxiv.org/pdf/0706.0246, Definition 3.2 and Theorem 4.1: incremental stability plus beta(epsilon,tau)+mu+eta/2<=epsilon supports the stated approximate-bisimulation construction. The solver-event language above claims its narrower induction contract.

Discovery initially returned an old university URL for Neumaier that was 404; an author-domain PDF initially timed out and then opened successfully. These access failures do not affect the subsequently inspected text. A transient ENOSPC failure prevented the child's raw numerical receipt and initial report write; its claimed count is not treated as saved evidence. Primary source formulas and analytical derivations remain independently inspectable.

 
## Addendum: source-specific lazy moment contract for coding (R5/R9)

The pinned HEAD polynomial API evaluates all moments through S6 whenever coefficients.len()>2. This creates a concrete avoidable refusal: with a=2^400 and b=0, the exact degree-two trace S2=2^402 is finite in binary64, while the unused a^3 term of S6 overflows. The same issue affects a sparse S3=-3 query with an enormous finite a even though its answer is constant. The constant/linear shortcut in pinned HEAD solves only degree<=1 requests.

For coefficient intervals C_k and finite parameter box (a,b), the exact requested set is the full multiplicity trace

    Tr(sum_{k=0}^6 c_k z^k)=sum_{k=0}^6 c_k S_k(a,b), c_k in C_k.

Linearity of algebra trace proves this identity at simple and multiple roots. Define live support L={k: C_k != [0,0] and k != 1}. Singleton [0,0] coefficients contribute exactly zero; S1=0 contributes zero for every finite coefficient. An interval merely containing zero is live and may not be dropped. Validate every input coefficient before support pruning, retaining malformed-input refusals.

Needed exact dependency graph:

| Live requested moment | Required arithmetic |
|---|---|
| S0=4 | constant only |
| S1=0 | identically zero, no term |
| S2=4a | scalar/interval multiply |
| S3=-3 | constant only |
| S4=4(a^2+b) | a^2 and addition |
| S5=-10a | scalar/interval multiply |
| S6=4a^3+12ab+3 | a^2,a^3,ab and additions |

Compute the union of these dependencies, sharing a^2 only if S4 or S6 is live. Sum only live terms with outward arithmetic. For each realization of a,b,c_k, each retained interval encloses its exact term, and interval addition encloses the sum. Pruning exact zero terms therefore preserves the mathematical target while avoiding arithmetic with no dependency on it. Constants and multiplicities stay unchanged; this is a fixed quartic degree-six provider optimization, not a generic symbolic compiler.

Core regression: degree2 at a=2^400,b=0 succeeds and encloses 2^402; pure S3 at a=2^600 succeeds and encloses -3 without squaring a; pure S5 at a=2^600 succeeds and encloses -10*2^600 without squaring a; zero high-degree coefficients do not trigger S6. A nonzero S6 query still refuses unrepresentable required arithmetic. Validate arbitrary interval coefficients, explicit zero, signed zero, malformed intervals, all-zero polynomial, multiple roots and a genuinely required overflow. Work counters charge only arithmetic actually performed. Full-moment and field APIs continue to calculate their requested S6 and preserve their prior failure scope.

The guarantee is dependency-specific avoidance of unused overflow. It does not promise to recover every mathematically finite sum whose active individual terms overflow and cancel.

## Addendum: independent review of the dynamics agent's buffer refinement

Read theory_dynamics.md and pinned HEAD coupled switching.rs::prepare/observe, plus adaptive.rs call sites. The production while loop calls prepare before each attempt; every observe call either is followed by continue/break or is the last observation of that attempt. At the next attempt prepare clears and overwrites pending_state before its only semantic read for reference promotion. The runtime protocol therefore establishes the proposed R_post/R_pre induction.

With reference_guard enabled and accepted finite positive protected score, replacing baseline_state.clear()+extend_from_slice(pending_state) by a vector ownership swap preserves baseline bits, baseline epoch/h/clock, routing guards and decisions. The old pending buffer becomes scratch; no observed reference reads it before the next prepare overwrite. No-guard paths are unchanged. The proven cost reduction is one n-coordinate copy at each such reference update; copy-count telemetry should change deliberately. Allocation timing and dimension-growth behavior need prepare+observe tests and complete-loop performance measurement.

The preexisting unit tests that call observe repeatedly without prepare use the unguarded setup; they do not contradict the guarded production protocol. A future guarded caller invoking observe twice without prepare would violate the refinement contract, so the protocol comment and complete-word tests should remain explicit.

This independent source/proof review accepts the scoped optimization contract; native execution, differential-word results and measured speed remain coding evidence owned by the parent.

