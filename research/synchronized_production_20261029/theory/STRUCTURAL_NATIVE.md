# Native directional quadratic stage proposal: theorem and admission contract

This extension follows the user's explicit request for a structural mathematical speed improvement after allocation-only measurements failed to show broad speedup. It does not reset the inherited failures or the three consumed correction episodes. This is the first implementation of a newly requested algorithmic arm. Existing solver defaults and full-space polynomial witness types remain unchanged.

## N1. Exact directional quotient and its scope

Fix the exact-real linearization J at one RODAS attempt, scalar σ=hγ, and current raw U-form stage right-hand side f. Preserve the configured initial guess x0 (zero when absent) and set v=f−(I−σJ)x0. The candidate is x0+q_v below. The production client validates identity mass. The proposal receives finite caller-supplied scalars a,b; these are a hypothesis, not a certificate or an inferred full-space relation. Define

\[
e_v=J^2v-aJv-bv,\quad D=1-a\sigma-b\sigma^2,\quad
q_v=((1-a\sigma)v+\sigma Jv)/D.
\]

If D≠0, direct expansion, without spectral or diagonalizability assumptions, gives

\[
(I-\sigma J)((1-a\sigma)v+\sigma Jv)
=(1-a\sigma)v+a\sigma^2Jv-\sigma^2J^2v
=Dv-\sigma^2e_v,
\]

hence

\[
v-(I-\sigma J)q_v=\sigma^2e_v/D. \tag{N1}
\]

Only e_v=0 on the current v is required for exactness. No relation on another stage, new RHS, another operator epoch, or the full space follows. The implementation never promotes the supplied coefficients or a successful directional test into an operator identity or a full-space witness. It recomputes the proposal and the original residual on every stage using the live original shifted operator A=I−σJ and its current h_gamma(). The native numerator is (2−aσ)v−Av, algebraically equal to (1−aσ)v+σJv. Applying the shifted operator preserves its declared JVP provenance and partial-failure work. The kernel takes the live sigma from its caller; every acceptance still checks the original live A, so an inconsistent sigma can only create a rejected or independently acceptable proposal. This prevents relying on stale shift/J pairing without checking the current original residual.

At σ=0, D=1 and q_v=v; no division by σ or 1/σ is used. At D=0 the representation is refused before an action. This refusal does not imply singularity of I−σJ: supplied a,b may be wrong, and a true directional polynomial can include factors irrelevant to v. A finite-precision near-zero denominator guard is a conservative numerical selection rule, not a theorem that the system is singular. Degenerate b=0 and repeated roots require no special formula. A zero RHS can return a zero proposal, but still passes the same explicit residual check.

## N2. Original-residual acceptance and floating-point limits

The implementation forms a stored binary64 full candidate xhat=x0+qhat in private workspace and computes the live original shifted action through apply_counted. Its acceptance residual is exactly the existing GMRES-into algebra `rhat_i = f_i - shifted_action_i`, measured by safe_l2. The unchanged stage threshold is

\[
\tau=\max(\mathrm{linear\_atol},\mathrm{linear\_rtol}\,\|f\|_2).
\]

Input dimensions, finite RHS, finite nonnegative tolerances and finite threshold are validated before actions. A candidate is accepted only if every candidate component and its computed residual norm is finite and ||rhat||₂≤τ. Output is committed only after that test; rejection leaves it untouched and calls the existing GMRES-into fallback with the original warm start. Actual operator/callback errors propagate as errors, with attempts already charged; they are not reclassified as a harmless relation refusal. Algebraic range/denominator refusal or residual rejection is a normal fallback outcome. The hypothesis can be wrong yet yield an acceptable current direction, which is legitimate because the actual original residual is the acceptance authority.

The displayed inverse/error inequality is in L2; a WRMS use requires every operator/error norm in that inequality to use the same positive weighted norm. Action output scratch is poisoned with NaNs before every callback, and an incomplete/nonfinite action is an operator error.

This is the SAME floating residual criterion as the existing unstaged matrix-free GMRES solve, not bitwise equivalence to its iterate, not a rigorous physical error enclosure, and not a complete forward-stability proof. If the represented exact operator is A and the computed residual evaluation satisfies ||(f−A xhat)−rhat||₂≤η_eval, then ||f−A xhat||₂≤τ+η_eval by the triangle inequality. If additionally ||A⁻¹||₂≤K is established, ||A⁻¹f−xhat||₂≤K(τ+η_eval). Neither η_eval nor K is inferred from passing the residual test. Stage-to-output physical budgets remain separately owned; measured trajectory agreement does not complete those gates.

The algebra guard uses finite products aσ and bσ² and D, and refuses if |D|≤64ε(1+|aσ|+|bσ²|), with any nonfinite intermediate also refused. This guard is deliberately sufficient only for avoiding a visibly ill-scaled representation; acceptance never relies on it. An overflow or underflow in intermediate candidate arithmetic may lose an otherwise usable representation; nonfinite candidate arithmetic falls back without claiming a mathematical no-go.

## N3. Existing positive and adverse dynamical systems

The already present autonomous-jvp workload is a direct sum of identical rotations J=[[0,−3],[3,0]], with J²=−9I. Thus a=0,b=−9 is an exact full-space relation for this *specific* model (the generic API does not certify it). For real σ,

\[
D=1+9\sigma^2\ge1,\quad q_v=(v+\sigma Jv)/(1+9\sigma^2).
\]

Since Jᵀ=−J, the Euclidean logarithmic norm μ₂(J)=λmax((J+Jᵀ)/2)=0. Moreover

\[
(I-\sigma J)^T(I-\sigma J)=(1+9\sigma^2)I,
\quad \|(I-\sigma J)^{-1}\|_2=(1+9\sigma^2)^{-1/2}\le1.
\]

This is a concrete coercivity/resolvent proof, stronger than relying on eigenvalues alone for an arbitrary nonnormal J. It gives a conditional residual-to-stage-error bound once residual-evaluation error is separately bounded. It does not establish a global RODAS trajectory enclosure.

The existing time-jvp workload has 16 distinct real diagonal entries λ_i=−(2+i/8), i=0,…,15. The actual full-IVP benchmark uses a=−47/8 and b=−31/4, whose polynomial is

\[
p(z)=z^2+(47/8)z+31/4=(z+2)(z+31/8).
\]

These coefficients interpolate the two interval endpoints −2 and −31/8; they are not the rotation coefficients. A monic quadratic has at most two distinct roots. Thus for v with nonzero entries in more than two of these eigendirections, e_v=p(J)v cannot vanish in exact arithmetic: this workload remains an adverse control for any claim of exact degree-two termination. It is also a justified *approximate* quadratic proposal, rather than an arbitrary wrong-coefficient test.

Write L=15/8 for the spectral interval width and t=λ+31/8 in [0,L]. Then p(λ)=t(t−L), so |p(λ)|=t(L−t)=L²/4−(t−L/2)²≤L²/4=225/256. This is a scalar completion-of-the-square proof. Because this particular J is real diagonal (hence normal), its Euclidean operator norm satisfies

\[
\|p(J)\|_2=\max_i|p(\lambda_i)|\le225/256.
\]

The normality assumption is essential: spectral containment alone would not give this norm bound for an arbitrary nonnormal Jacobian. In fact the discrete 16-point grid has its closest points on opposite sides of the interval midpoint, giving the sharper bound 7/8, but the interval bound above is sufficient and applies continuously over the whole interval.

For σ≥0 the denominator and logarithmic norm give

\[
D=(1+2\sigma)(1+31\sigma/8)>0,\quad
\mu_2(J)=\lambda_{\max}(J)=-2,\quad
\|(I-\sigma J)^{-1}\|_2=1/(1+2\sigma).
\]

Consequently the exact-real candidate and its exact directional residual obey

\[
\|v-(I-\sigma J)q_v\|_2
\le \frac{225\sigma^2}{256D}\|v\|_2,\qquad
\|(I-\sigma J)^{-1}v-q_v\|_2
\le \frac{225\sigma^2}{256D(1+2\sigma)}\|v\|_2.
\]

This derives precisely why sufficiently small σ can pass a fixed residual threshold despite the absence of an exact quadratic relation. For the implemented floating candidate, coefficient/evaluation defects and original-residual evaluation error must still be included before turning these formulas into a certified forward-error bound. The implementation does not replace its original residual gate with this estimate or loosen its tolerance. Actual acceptance/refusal counts and refusal costs are reported. The fixed native wrong-coefficient test still uses a=b=0 on the rotation and a separated threshold, providing a genuine rejection control independent of this endpoint-based approximate proposal.

## N4. Algorithmic work and break-even

For a zero initial guess, an accepted nonzero-shift quadratic stage takes one shifted action Av to form the correction, one original shifted residual action on the full candidate, O(n) vector arithmetic and two retained n-vectors. A nonzero configured x0 adds its counted initial residual action, which the baseline also requires. It performs zero Arnoldi columns, zero MGS passes and zero small QR/least-squares solves. A zero-shift proposal needs no construction action, but the actual original shifted residual is still called. Every construction and verification action is counted before entry through the shared apply_counted API, including failed callbacks. Successful proposed stages increment linear_solves once; they do not fabricate Arnoldi iteration/cycle counters. A separate cumulative proposal ledger reports attempted, accepted, refused and callback/action work; it survives rejected outer-step attempts as work already performed.

Let C_J be one JVP/shifted action cost, C_V the proposal vector work, C_K(k) the MGS/QR plus basis work for a k-column GMRES solve, and m_base its actual total counted operator actions (including true residuals and warm-start cost). For a nontrivial nonzero-shift candidate let m_accept=2 for a zero start and m_accept=3 for a nonzero start; initial-convergence and zero-shift shortcuts are counted separately. Accepted-proposal savings require (m_base−m_accept)C_J+C_K(k)>C_V. With k=2 this can save both actions and orthogonalization; with k=1 the proposal may lose. A refused nondegenerate proposal adds up to two actions for zero x0 or three for nonzero x0 plus vector work before the original fallback. With accepted probability p, total improvement requires p(C_base−C_accept)>(1−p)C_refusal. Thus no unconditional speedup or default routing is claimed. Existing mixed-spectrum adverse controls and the same full IVP controller must be timed, not only isolated exact polynomial solves.

## N5. Smallest source integration and verification plan

New standalone module `crates/rodas5p-integrators/src/directional_quadratic.rs`: finite validated proposal(a,b), two-vector caller-owned workspace, cumulative statistics, GmresIntoReport with zero iterations/cycles/least_squares_solves and truthful action counts, and `try_directional_quadratic` returning `CoreResult<Option<GmresIntoReport>>`. Inputs include the live original operator, sigma, coefficients, RHS, configured x0, GmresConfig and optional positive residual_scale; the strict native client uses L2 while the standalone kernel supports the same WRMS gate. `None` means unchanged-output algebraic/residual refusal. Accepting the live LinearOperator preserves original residual semantics even for other mass maps; identity-mass algebra is guaranteed only by the strict MF-fast client. The native routine is a proposal, not the existing declared full-space provider.

Root owns the shared module export and MF-fast integration. Add a private optional proposal/workspace in Rodas5pMfFastWorkspace, a validated setter/getter/statistics accessor, clone/initialization handling, and an opt-in call before ordinary GMRES-into stages only. No staged target route or other solver method is silently opted in. On accepted stage commit output, add zero-iteration solve-log entry and continue; on refusal retain all work counters and use existing GMRES call and warm start. A reusable workspace can run the full existing integrate_observed API; no facade public options struct or default route must change.

Tests must cover: exact rotation and independent original residual; supplied-coefficient error/refusal with output sentinel intact; denominator zero/near-zero refusal before callback; σ=0; nonfinite/invalid inputs; actual first and verification callback errors and charged attempts; callback epoch/operator replacement uses live operator; accepted-action count and zero Arnoldi work; strict tolerance requiring fallback. Full-IVP tests compare endpoint/time-grid diagnostics at the same tolerances and count actual JVP/MGS/QR work; benchmark compares enabled/disabled full integration using the existing autonomous rotation and 16-rate adverse control, retaining any slowdown. No additional correction iteration is assumed authorized by this plan.

Independent audit: the existing proof_audit reviewer accepted the scoped formula and original-residual gate, required callback propagation and honest zero-Arnoldi reporting; root required preserving x0 and using shifted actions for complete provenance. Those conditions are incorporated above. Existing K3 exact Fraction algebra and original-source literature receipts are reused; no new symbolic or native run has occurred at the theory-admission point.

### Initial convergence and emitted native audit records

The admitted implementation preserves the configured initial iterate. If its computed original residual already meets the same threshold, it returns that initial iterate without constructing a correction. For a linear operator A, A0=0 exactly. With a zero initial iterate the residual is therefore the RHS itself and no A*0 action is introduced, matching the existing solver's initial-residual algebra. Its default RecomputeFinal accounting can subsequently call A*0 diagnostically; this optional proposal does not repeat that diagnostic. Thus the zero shortcut has deliberately different callback work, while using the same exact-linear residual. With a nonzero initial iterate the one counted original action establishes the shortcut. This special case is explicitly distinguished from a newly accepted quadratic candidate, which receives a fresh full original residual evaluation.

The seventh contract test emits actual binary64 outputs in a `STRUCTURAL_NATIVE_BITS` JSON envelope (`structural-native-bits-v1`), with six accepted fixed rotation cases and two output-preserving refusals. Sigma includes zero, positive and negative values; dimension includes two and four; several records preserve nonzero configured starts. It reports input/output/tolerance/coefficient bits and charged actions. These records are kernel executions, not recorded ODE stages. An independent Fraction replay can compute the exact represented original rotation residual and exact solution from these emitted bits, compare squared residual against the stored squared threshold, and use the proven rotation resolvent identity. This is a finite set of independently checked native records, not a general binary64 residual-evaluation enclosure or a physical trajectory certificate.

Prerelease range audit: finite-input norm overflow with a finite threshold is a normal proposal refusal, allowing the existing solver to handle an already solved large warm start. Overflow in the finite-input subtraction f−Ax0 is also refused before any construction action; such an infinite intermediate is never passed to a callback. A nonfinite requested threshold remains the existing invalid-input error. These are static pre-first-delivery corrections, with no native run yet.
