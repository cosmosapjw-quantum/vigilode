# VigilODE dynamics and guarded symbolic execution

Date: 2026-10-28 (Asia/Seoul). Scope: independent theory R1–R10, applied to PR97 tree `87465835` and its inherited switching implementation. This document makes no production code modification.

## Result and implementation contract

The concrete new optimization is ownership transfer of a completed protected-reference state snapshot in `crates/vigilode-coupled/src/switching.rs::SwitchState::observe`. After a successful protected solve, replace the second coordinate copy from `pending_state` to `baseline_state` by a buffer swap. This removes exactly `n` copied coordinates per successful protected-reference update, where `n` is the state dimension. The guarded routing language and all physical acceptance checks are preserved. The receipt must charge only the actual first copy performed in `prepare`; the additional `n` copy charge must be removed.

The existing PR97 contraction recurrence is already stronger than a tempting new positive-coefficient bound, so it is reused. The logarithmic-norm derivation below provides an independent semantic audit, not a claim of a new analytic result.

## R1: research location and contract

The user requested theory first, coding second, with ODE dynamical systems and symbolic dynamics actively connected to solver performance. The source contains a data-bearing guarded transducer, `SwitchState`, rather than a finite physical partition of ODE phase space. Its methods obey the protocol `prepare(h,y,epoch); skip(); attempt; observe(receipt)`. We optimize this actual protocol and preserve its observable routing outputs. Physical acceptance remains in the solver's residual/error checks and does not follow from a routing symbol.

The proposed code acceptance criteria are: bitwise equality of all future stored baseline states; unchanged skip/phase/decision/reference-status words for the same legal inputs; accurate copied-coordinate counts; unchanged physical trajectory acceptance; adversarial tests covering retries, candidate-only success, protected success, changes of epoch, step size, and state dimension. The performance claim is initially an exact work reduction. Wall-time improvement requires native matched-work measurement.

## R2: sources and reusable evidence

Read sources: PR97 `cyclic_closure.rs` (prepared endpoint identity, radius recipes, private prepared-state data), `trace_quotient.rs` (point/box certificate scope), and inherited `switching.rs` (all state updates and tests). Attached-source theory is coordinated with the attachment agent; no duplicate closure implementation is proposed.

Primary literature accessed using Exa:

* G. Söderlind, *On nonlinear difference and differential equations*, BIT 24 (1984), 667–680, DOI https://doi.org/10.1007/BF01934923. Nonlinear logarithmic norm/Dahlquist constant and error propagation.
* J. Banasiak, *Logarithmic norms and regular perturbations of differential equations*, DOI https://doi.org/10.17951/a.2019.73.2.5-19. Full retrieved text includes Dini comparison, logarithmic-norm properties, and dissipative perturbation estimates.
* G. Pola, A. Girard, P. Tabuada, *Approximately bisimilar symbolic models for nonlinear control systems*, https://arxiv.org/abs/0706.0246, Automatica 44 (2008), DOI https://doi.org/10.1016/j.automatica.2008.02.021. Incremental global asymptotic stability supports arbitrarily precise symbolic abstractions; bounded physical state space yields finiteness.
* A. Girard, G. Pola, P. Tabuada, *Approximately bisimilar symbolic models for incrementally stable switched systems*, https://arxiv.org/abs/0807.5022. Common incremental Lyapunov functions, or multiple functions with dwell time, supply assumptions for physical switched-system abstraction.

Exa's PDF fetch of 0706.0246 returned abstract/metadata and a reCAPTCHA marker; theorem-level quantization constants are therefore not claimed from that fetch. Raw service records are in `dynamics_service_evidence.json`. Wolfram was actually invoked; its derivative identities simplify to zero, while the first limit/series evaluations generated removable-singularity messages retained in the raw record.

## R3: claim/assumption alignment

Let `y'=f(t,y)` have a unique local solution and let a continuous piecewise differentiable represented path `v` have residual `r=v'-f(t,v)`. On a convex tube containing the segment between `y(t)` and `v(t)`, assume `f` is continuously differentiable in state, `mu(D_y f)<=mu_bar`, and `||r||<=delta`. The norm is fixed on the slab. These assumptions imply a scalar error majorant; a state-dependent error scale cannot silently replace that fixed norm.

For routing, use a guarded labeled transition system with control modes calibration, candidate, cooldown, reentry and terminal. Its data includes continuous work references, states, step sizes and model epochs. The finite mode graph is an abstraction of this data-bearing system. It is not an approximately bisimilar physical ODE model until the incremental-stability and state-partition assumptions of the cited literature have been supplied.

## R4–R5: constructive mathematical connection

### Signed logarithmic-norm transport

Write `e=y-v`. By the mean-value integral,

`e' = A(t)e-r`, with `A(t)=integral_0^1 Df(t,v+s e) ds`.

Convexity of the matrix measure and its Dini-derivative inequality give

`D+ ||e|| <= mu_bar ||e|| + delta`.

Consequently, for `s>=0`,

`E(s)=exp(mu_bar*s) E0 + delta*s*phi1(mu_bar*s)`,

where `phi1(z)=expm1(z)/z` and `phi1(0)=1`, is a valid error majorant. For `mu_bar=-lambda<0`, it becomes

`E(s)=a+(E0-a) exp(-lambda*s)`, `a=delta/lambda`.

It is monotone toward `a`, hence its whole-slab maximum is `max(E0,E(h))`. An endpoint certificate alone is insufficient when `E0>a`: the endpoint decreases but the largest slab error is at the incoming endpoint. A self-consistent tube of radius `R` is certified when `max(E0,E(h))<R`, together with the field/residual hypotheses on that tube; the standard first-exit argument closes the containment assumption.

For a fixed weighted infinity norm `||x||_w=max_i |x_i|/w_i`,

`mu_w(A)=max_i (a_ii + sum_{j!=i} |a_ij| w_j/w_i)`.

Negative diagonal damping must retain its sign. Replacing it by an absolute row sum can erase contractivity and greatly increase the certificate cost. Step-dependent weights require a transition factor `max_i w_old_i/w_new_i` when transporting an incoming radius.

### Audit of the PR97 two-envelope contraction formula

Set `r=lambda*h`, `F=h*delta`, and assume `r>0`. The exact endpoint is `a+(E0-a)q`, where `q=exp(-r)`. The elementary inequalities

`(1-r/2)/(1+r/2) <= q <= 1/(1+r)`

hold for every `r>=0` (the lower bound is negative for `r>2`, still valid). If `E0-a>=0`, use the upper bound; otherwise use the lower bound because multiplication reverses the inequality. This produces exactly the source formulas

`U1=(E0+F)/(1+r)`,

`U2=((1-r/2)E0+F)/(1+r/2)`,

with a safe branch-free upper envelope `max(U1,U2)`. Their difference is

`U2-U1 = r*(F-r*E0)/((1+r)*(2+r))`.

Thus the maximizing branch agrees with the correct sign of `E0-a`. Wolfram independently returned this factorization. At `r=0`, both equal `E0+F`. PR97's `max(E0,endpoint)` supplies the whole-slab bound. This result is reused without adding a weaker recurrence.

### Guarded words and ownership-transfer bisimulation

Let an input letter consist of the next prepared `(h,y,epoch)` and the subsequent physical outcome class. Useful outcome symbols are `P+` (protected accepted), `P-` (protected retry), `C+` (candidate accepted), `C-` (candidate local-error retry), `F+/-` (candidate failure with protected outcome), and `T` (terminal failure). All accepted-state, locality, epoch and score guards remain the actual source guards.

The state semantics include a baseline snapshot `B` and pending snapshot `P`. `prepare` assigns `P := exact_bits(y)`. On a successful protected reference update, the old implementation performs `B := copy(P)`; the new one swaps ownership of the two vectors. The old post-state is `(B=P_current,P=P_current)`, while the new post-state is `(B=P_current,P=B_old)`.

Define relation `R_post` after `observe` by equality of all control/reference fields and equality of `B`; ignore `P` because it is scratch at this boundary. Define `R_pre` after the next `prepare` by equality of those fields and equality of both `B` and `P`. `prepare` overwrites `P` before any later read, so `R_post -> R_pre`; `observe` reads the same baseline/guard data, emits the same routing decision, and either leaves the baseline unchanged or promotes the identical prepared `P`, so `R_pre -> R_post`. Induction over every legal finite input word proves routing equivalence. Coordinate-copy telemetry is intentionally different and reports the removed operation.

The capacity/allocation trace is outside this semantic equivalence: after a swap, the next `prepare` reuses the old baseline's capacity. At the first reference update the old baseline is empty, so a later prepare may allocate its second buffer exactly as the copy implementation allocated a second buffer during the first observe. This changes allocation timing, not the mathematical reference state. A benchmark must include prepare and observe together.

The symbolic-dynamics contribution is this explicit word-preserving relation used to justify a concrete native cost reduction. It does not require claiming a topological conjugacy of ODE trajectories or building a new unused dispatcher.

## R6–R7: independent and discriminating checks

Analytic checks: the Dini argument gives the signed transport formula; differentiation verifies `E'=mu_bar E+delta`; the zero-parameter limit yields `E0+delta*h`; whole-slab monotonicity follows from the sign of `E0-delta/lambda`; the factorized envelope difference determines the correct rational branch. Wolfram returned zero derivative residuals and the exact envelope difference. Its `Power::infy` messages arise during removable-singularity evaluation and are preserved rather than classified as a failed mathematical identity.

The buffer equivalence proof tracks source reads and writes: `pending_state` is only read for reference promotion inside `observe`; every legal subsequent attempt calls `prepare`, which clears and extends it first. Calling `observe` twice without `prepare` is outside the internal protocol and must not be introduced by a future caller. Concrete tests should exercise complete words, not only one successful transition.

## R8: decisions

1. Reuse PR97's two-envelope recurrence and its dimension-specific hypotheses. The weaker positive-coefficient candidate is not adopted.
2. Adopt the guarded buffer-ownership optimization for coding subject to the stated protocol and native tests. The proven work saving is one `n`-coordinate copy per reference update. Actual elapsed-time savings remain unmeasured at this theory checkpoint.
3. Keep approximate symbolic abstraction of the physical ODE as a separately specified extension requiring incremental stability and quantization error control. No such theorem is needed for the current routing optimization.
4. No new strong no-go theorem is asserted; NG1–NG7 novelty/independent-review requirements are not invoked by an ordinary refinement proof.

## R9: code handoff

Target: `SwitchState::observe`, successful accepted protected-reference update guarded by `reference_guard.is_some()`. Replace the second state copy with `std::mem::swap(&mut self.baseline_state,&mut self.pending_state)`. Remove only its additional copied-coordinate charge. Retain the first copy in `prepare`, all guards, cooldown arithmetic, cost weights and acceptance logic. Update the receipt field documentation to charge prepare's coordinate copying. Tests must compare complete symbolic outcome words against source-contract baseline snapshots and include state growth, signed zero, retries, epoch invalidation, and candidate-only accepted steps.

Suggested matched-work benchmark: repeatedly execute prepare+observe for guarded protected accepted words at dimensions 2, 32, 4096, and 65536; randomize paired order, hold all inputs/outcome words fixed, compare copy counts and median wall time. Include a candidate-only word and no-guard control. Do not present these as end-to-end solver speedups.

## R10: checkpoint

Budget ownership: `LOCAL_DEVELOPMENT_PLAN.json` identifies exhausted R-ADAPT as the facade adapter theorem/implementation (`crates/vigilode/src/switching.rs`, E<I/prefix E<L/productive-sentinel/past proposal transport), with 3/3 corrections used. The proposed change is in the distinct coupled module `crates/vigilode-coupled/src/switching.rs`; it does not revise those theorems, route decisions, future-cost assumptions, or facade implementation. Its closest inherited owner is PR-N04 (owned workspace reuse, 0/3 at PR97), whose explicit obligation is to prove buffer reuse preserves operation/evaluation order. Applying that existing obligation to this coupled reference buffer is a bounded code-target extension; it does not complete PR-N04's session/FSAL/homotopy obligations. PR-N06 remains pending because no temporal residence/payback theorem is proved here. The parent must record this mapping and charge any corrective episode to inherited PR-N04; this document does not create or reset a budget.

Theory unit: complete scoped derivation and source-connected optimization contract. External services: Exa search/fetch and Wolfram context/evaluator executed. Production code: not changed by this theory unit. Native performance: pending coding unit. New review obligation: parent or independent reviewer checks the source protocol and the buffer-swap refinement before acceptance. Existing PR97 numerical evidence is preserved and does not count as a fresh native run.
