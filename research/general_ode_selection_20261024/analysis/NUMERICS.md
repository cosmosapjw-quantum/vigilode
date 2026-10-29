# Numerical analysis of the selected ordinary-ODE methods

## Scope and source identity

This note audits the native solver sources inherited from `057fecf0698fa262d881f101625e6b0bb93791cf`, whose solver code matches `1cdf2c3071498888f2d8ff3ddc0629cb3cf443ef`, plus the narrowly described changes in this branch. It concerns ordinary, identity-mass ODEs. Application-specific reductions, collision models, spatial discretizations and previously audited affine transformations are excluded. Published evidence is reused; this investigation does not rerun their proofs.

The selected methods are the seven-stage Dormand–Prince 5(4) explicit method and the **eight-stage** RODAS5P transformed Rosenbrock method. An explicit method performs no Newton iteration, Jacobian assembly or JVP. RODAS5P performs linear stage solves and requires derivative information; not assembling an extra Jacobian product in its transformed stage equations does not make the method derivative-free. A JVP-only GMRES route is matrix-free, but still uses derivatives. Finite differences approximate derivatives. These distinctions must remain visible in specifications and cost accounting.

## Constructive exact checks

Run `python research/general_ode_selection_20261024/analysis/verify_numerics.py`. The standard-library script parses the exact rational expressions in `dopri5.rs`, rather than maintaining a second independent tableau. It checks row sums, the strict triangular structure and the embedded difference weights. Unlabelled rooted trees of orders one through six have counts 1, 1, 2, 4, 9 and 20. For a tree τ, define its density γ(•)=1 and γ([τ₁,…,τₖ])=|τ|∏γ(τⱼ). Define stage weights Φ(•)=1 and Φᵢ([τ₁,…,τₖ])=∏ⱼ(AΦ(τⱼ))ᵢ. The exact order conditions are bᵀΦ(τ)=1/γ(τ).

All **17 endpoint conditions through order five** and **8 embedded conditions through order four** hold exactly. Consequently, for sufficiently smooth ODEs and exact arithmetic, the endpoint has local defect O(h⁶), and the embedded difference is O(h⁵). This explains the fifth-root step controller. It does not make the estimated local error an upper bound for the global solution error. Floating-point constants approximate those rationals; ordinary rounding and callback error are additional perturbations.

An independent Wolfram evaluation reconstructs the tableau and obtains

\[
R(z)=1+z+\frac{z^2}{2}+\frac{z^3}{6}+\frac{z^4}{24}+\frac{z^5}{120}+\frac{z^6}{600},
\]
\[
R(z)-\widehat R(z)=-\frac{z^5(97-39z+5z^2)}{120000}.
\]

The nonzero estimator roots have positive real part, (39±i√419)/10; this is a concrete discriminator for the scalar linear model, not a general reliability certificate. The exact endpoint logarithm starts as

\[
\log R(z)=z+\frac{z^6}{3600}-\frac{z^7}{2100}+\frac{z^8}{3200}+O(z^9).
\]

It reveals accumulated amplitude and phase effects that endpoint tolerance alone does not specify. The RODAS decimal snapshot has eight stages. Exact rational arithmetic applied to its *decimal literals* yields scalar-series coefficient discrepancies through degree five no larger than 5.53×10⁻¹⁵ and an exactly zero scalar infinite-stiffness limit. These are a coefficient transcription/structure check, **not a proof of all nonlinear RODAS5P order conditions or stiff accuracy under arbitrary approximate Jacobians**. The snapshot explicitly says that original exact higher-precision coefficients are unavailable. The intended classical method must be distinguished from numerical observations about its stored approximation.

## A usable accuracy bound for oscillatory explicit integration

Consider y′=iωy, a real constant ω, and positive variable step sizes hₙ summing to T. Assume |ω|h_max≤0.9. In exact arithmetic the implemented Dormand–Prince endpoint is contractive for this scalar rotation. Indeed,

\[
|R(i\theta)|^2-1=\theta^6\left(-\frac1{1800}+\frac{\theta^2}{1600}-\frac{\theta^4}{14400}+\frac{\theta^6}{360000}\right)\le0,
\]

because dropping the negative fourth-power term and substituting 0.9 in the remaining positive terms leaves a strictly negative rational number. The Taylor remainder gives

\[
|R(i\theta)-e^{i\theta}|\le\frac{|\theta|^6}{3600}+\frac{|\theta|^7 e^{|\theta|}}{5040}.
\]

A telescoping product and |R(iθ)|≤1 therefore yield the constructive bound

\[
|y_N-e^{i\omega T}y_0|\le |y_0|T|\omega|^6h_{\max}^5
\left(\frac1{3600}+\frac{0.9e^{0.9}}{5040}\right).
\]

This provides a conservative step cap from a known frequency and requested global state-error budget. For a normal skew-Hermitian linear operator the same argument applies in its unitary eigenbasis with |ω| replaced by the largest frequency and the Euclidean norm. It is not licensed for nonlinear, nonnormal or empirically estimated-frequency problems without further bounds. It is a route for a future declared-frequency adapter; the current selector does not pretend to implement that contract. Its asymptotic leading amplitude loss is proportional to Tω⁶h⁵ and phase excess to Tω⁷h⁶ on uniform meshes. The exact inequality above, rather than those leading terms alone, is the usable guarantee.

## Accuracy-preserving switching theorem

Suppose every eligible one-step map Ψₘ,ₕ satisfies, uniformly on a common solution neighborhood,

\[
\|\Psi_{m,h}(u)-\Psi_{m,h}(v)\|\le(1+Lh)\|u-v\|,\qquad
\|\Psi_{m,h}(y(t))-y(t+h)\|\le C_mh^{p+1},
\]

for L≥0 and all permitted positive h. Let actual step n choose any eligible method mₙ, including a discontinuous history-dependent choice. Let ηₙ bound the norm of an additional state transfer, roundoff or inexact-solve perturbation at that step. Subtracting the exact state using the **same method actually chosen** gives

\[
E_{n+1}\le(1+Lh_n)E_n+C_{m_n}h_n^{p+1}+\eta_n.
\]

Iterating and using ∏(1+Lhₙ)≤e^{LT} proves

\[
E_N\le e^{LT}\left(E_0+C_{\max}Th_{\max}^{p}+\sum_n\eta_n\right).
\]

Thus switching itself need not lower fifth order when the two fifth-order methods satisfy the hypotheses and share an unchanged physical state. No smoothness of the *selection rule* is needed. This result does not certify the missing stability/defect hypotheses for every stiff problem, a changed Jacobian, changed state dimension, discontinuous RHS or remapping. A real switch must occur at an accepted state, preserve that state, invalidate method-specific caches and account for any new startup work. The new conservative selection API currently chooses a method before integration; it does **not** implement or claim automatic online switching.

## Persistent execution: a constructive work reduction

Fix an immutable deterministic RHS provider, identical tolerances, controller, landing schedule, arithmetic environment and initial state. Regard a native session's *entire* accepted state, stage cache, proposed step, controller history, counters and output cursor as its transition state. If a pause/inspection does not mutate it, the next numerical transition is identical to the corresponding uninterrupted transition. Induction proves identical states, callback arguments, step accept/reject decisions and counters, including across quotas that stop between attempts. This argument concerns a full state transition, not merely matching the current `(t,y)` pair. Changing providers or epochs requires fresh caches.

For DOPRI5 with FSAL enabled and A completed full trials, absent partial callback failures, the RHS count is exactly 1+6A. Rejected full trials can retain the first derivative at the unchanged base; accepted trials carry their final derivative to the next first stage. For K independent restarts with an otherwise identical attempted trajectory the corresponding count is K+6A, giving K−1 avoidable startup evaluations. In real adaptive restarted runs the trajectories and A may differ, so that difference is conditional; a wall-clock improvement must be measured. Reusing workspaces also removes repeated setup allocations, with no claim that every problem is thereby faster. New facade pause/resume tests and timing receipts are recorded separately by the session workstream.

## Adversarial implementation findings and repairs

1. **Stale output could conceal an incomplete RHS write.** The native `OdeProblem::eval_rhs_into` checks dimensions and finite output but initializes no fresh poison marker. The fast RODAS workspace reuses a stage vector; the first call also begins from finite zero storage. An `Ok` callback omitting one component could therefore supply zero or an old stage value without detection. This is an actual interface correctness defect, independent of a numerical convergence argument. `Guard::vector_call` now poisons each vector output, requires complete finite writes and converts malformed `Ok` into a terminal callback error. The same guard covers ordinary RHS, finite-difference RHS, explicit time derivative and JVP in the selected facade. It refuses nonfinite time/state before user entry; a JVP also checks its direction. A previously latched fatal error remains terminal. Documented sparse writes into zero-filled dense/banded Jacobians are preserved.
2. **Overflowed acceptance scales silently became infinity.** The fast RODAS norm formerly lacked the finite-scale check already present in DOPRI and the core norm helpers. With finite atol=10³⁰⁸, rtol=1 and |y|=10³⁰⁸, the floating-point scale overflows. A finite error then divided by infinity gives zero. This is a contract/controller inconsistency; it is **not claimed to prove an invalid step acceptance**, because the exact oversized denominator can still make the true ratio smaller than one. The scalar and vector tolerance branches now return `InvalidInput` if the scale is not finite/nonnegative. A native constant-state regression exercises both paths and a finite-scale positive control.
3. The RODAS implementation still sums squared normalized components directly, whereas DOPRI uses a scaled sum of squares. For realizable vector sizes, overflow of an individual squared normalized component already means rejection would be necessary. This is potential overly conservative shrinkage, not a demonstrated false acceptance. It is not elevated to a new blocking defect without measured impact.

The callback fix adds O(n) writes/scans per guarded vector callback and no successful-path heap allocation. Cheap-RHS timing must include this cost. Raw native APIs outside the facade do not automatically gain its malformed-output contract; their scope must not be overstated.

## Research-loop record and next decisions

The numerical loop fixed the source identity and reused prior evidence; separated explicit/implicit eligibility; enumerated rooted-tree conditions; derived scalar stability and estimator polynomials; obtained an independent Wolfram result; derived an oscillatory global bound; proved uniform-method switching; proved deterministic pause/resume equivalence; audited finite-state/callback boundaries; and assigned bounded implementation and validation actions. SciSpace was used for literature discovery, but its returned set did not establish the original DOPRI/RODAS references reliably and is preserved as discovery output rather than cited as validation. The Wolfram context supplies the official Dormand–Prince tableau documentation; the new evaluator output is an independent symbolic calculation.

The coding loop traced the facade-to-native callback path; constructed a stale-buffer counterexample; chose a shared guard; implemented RHS/ft/JVP/FD checks; preserved sparse Jacobian semantics; added malformed and finite-input regressions; repaired scale overflow in the fast norm; added scalar/vector boundary controls; handed serialization of Rust validation to the runtime/session worker; and handed integration plus selected-route review to the parent. No remote compute was used. The root-level verification receipts are authoritative for compiled test outcomes; this note alone does not assert that tests passed.

Next mathematical work should first bind an actual global error contract to declared problem information, rather than reinterpret an embedded tolerance as a global guarantee. Next numerical implementation work should prioritize common accepted-state online switching with cache invalidation, truthful derivative authorization, persistent implicit facade support, and root/event semantics. Derivative-free homotopy or polynomial replacements remain conditional candidates until a measured advantage and accuracy/stability contract beat the selected methods on matching problems. The present exact calculations justify the selected explicit tableau and constructive restricted guarantees; they do not establish unrestricted production readiness.

## Evidence files

- `NUMERICS_CHECK.json`: exact parsed-tableau rooted-tree checks and scalar coefficient diagnostics.
- `external/WOLFRAM_CONTEXT.json`, `external/WOLFRAM_CHECK_REQUEST.json`, `external/WOLFRAM_CHECK_RESPONSE.json`, `external/WOLFRAM_PHASE_REQUEST.json`, `external/WOLFRAM_PHASE_RESPONSE.json`: unmodified service responses and evaluator inputs.
- `external/SCISPACE_RAW.json`: full discovery response, including irrelevant hits; no claim that its metadata independently validates method order.
- Official tableau context: <https://reference.wolfram.com/language/tutorial/NDSolveExplicitRungeKutta>.
