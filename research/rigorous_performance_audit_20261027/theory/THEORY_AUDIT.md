# Mathematical audit and constructive results for VigilODE

Date: 2026-10-27 (KST). Scope: general ODE solver theory, adapter economics, and original-target stage-error control. The theory lane inspected the published adapter integration source `dda8d2f16758ce7d3369ad249344b2ac8bbd08ef`, then reconciled findings with the newer checkpoint source `3064d453091af1dec7f99f0c8afabd98b447ea34`. The latter already has return sentinels and incomplete-comparator lower-bound reasoning; neither is reported here as a missing baseline feature. Parent-owned concurrent edits are not mistaken for baseline evidence.

`skill_context_used`: PhysMath Research Loop 1.3.0, `SKILL.md`, `references/POLICY_INTEGRATION.md`, and `references/THEORY_LOOP.md`; selected R2-R7/R9. No recursive review or child delegation. Existing causal-majorant and polynomial-resolvent proofs were reused after checking assumptions and source semantics.

## 1. Scientific judgment

The current sources do not justify a conclusion that the project's mathematical construction is fundamentally invalid. Exact carrier removal, polynomial resolvents and finite causal propagation are valid under their declared structure. Their important limitations are specific: the structure must hold on the full relevant domain; an original-target residual needs an inverse or propagation bound; a low-rank projected root cannot itself establish original-space accuracy; and conditioning of coordinate reconstruction matters. These obligations cannot be repaired by memory allocation optimization.

The latest code already distinguishes an operational residual from a certified forward error (`vigilode-coupled/src/solver.rs`, `accuracy_authority = operational-residual-only-not-certified`). Its `full_check` computes all original-space stage residuals. `rodas5p-integrators/src/causal_majorant.rs` has finite triangular propagation and radius closure. `conditioned_polynomial.rs` binds declared polynomial relations, coefficient defects and original-operator residuals. These are substantive mathematical safeguards and should be preserved.

Conversely, copying a stage vector into a new matrix before every unchanged LU solve, allocating a temporary matrix row merely to dot it, or completing a losing comparator after its monotone cost has already disproved competitiveness are implementation/scheduling costs. Their removal does not require a new integration formula or weakened error threshold.

## 2. Proposition A: stopping a comparator from a partial cost lower bound

Fix one accepted checkpoint, one final time, one original-space output and accuracy contract, and one common nonnegative cost measure. Let a completed explicit candidate satisfy the contract at cost at most `E`. Let `L_k` be the cumulative cost already spent by a competing implicit execution after checked unit k. Suppose costs are monotone, so every completion of that same execution would have cost `I >= L_k`. Let `0 < rho < 1`.

The native driver's success flag establishes its registered local-tolerance/output eligibility. It is not, by itself, an independently certified physical global-error bound. Where the stronger contract is claimed, its evidence must be supplied separately; the cost proposition does not upgrade the underlying accuracy evidence.

**Proposition.** If `E <= rho L_k`, every completion of the implicit execution costs at least `E/rho`. The completed explicit candidate may therefore be selected and the implicit execution stopped without computing its endpoint. If the implicit execution never completes, it supplies no competing admissible result, while the explicit result retains its own qualification.

**Proof.** Monotonicity gives `E <= rho L_k <= rho I` for every implicit completion. This argument uses no state from the incomplete implicit execution. The explicit candidate's numerical validity was established before the cost comparison. QED.

The proposition is already implicit in source 3064's incomplete-comparator handling. The new executable refinement is to stop as soon as the lower bound becomes decisive, give it a distinct evidence type, and evaluate the sufficient inequality conservatively.

For floating arithmetic, compute an upper enclosure of E and a lower enclosure of `rho L_k`; do not round a threshold toward early acceptance. With integral callback counters, a threshold may be converted conservatively and independently rechecked after stopping. A callback-only lower bound is sufficient even if the full work score also includes nonnegative factorization and solve costs. Mixing seconds and callback counts is not sufficient. With serial checking after a callback whose weight is at most U, the charged callback cost before stopping is at most the prescribed threshold plus U, plus any separately charged structural verification. This is not a wall-clock bound.

Fatal callback semantics remain terminal even if the explicit candidate is available: a fatal result observed before the cost stop cannot be relabeled as cost exhaustion. Callback replay must be allowed by the caller's purity/transaction contract. A partial implicit stage/state is never committed. This proposition compares qualified observations on the present interval; it proves neither future optimality nor a fraction saved in total runtime, because explicit construction, discarded work and transition costs have already been paid.

**Code map:** `crates/vigilode/src/switching.rs`, the same-window return comparator and its `run(... callback ceiling ...)` invocation; `selection_evidence.rs` remains the complete-candidate comparison surface. Parent adapter lane owns implementation and actual ODE tests.

## 3. Theorem B: optimally relaxed reuse of dissipative shifted systems

Let H be a finite-dimensional real or complex Hilbert space with a fixed positive-definite metric M. All norms and adjoints below refer to M. Let J be linear and dissipative:

\[
\operatorname{Re}\langle v,Jv\rangle_M\le0\quad\text{for all }v.
\]

Let `a0 > 0`, `a > 0` carry units of time and J units of inverse time. Native VigilODE's scaled systems are

\[
P=a_0^{-1}I-J,\qquad W=a^{-1}I-J,
\]

where `a = gamma h`. Set

\[
\omega_*={2a\over a+a_0},\quad q_*={|a-a_0|\over a+a_0},\quad
R_0=(I-a_0J)^{-1}.
\]

**Theorem.** P and W are invertible. The iteration

\[
x_{k+1}=x_k+\omega_*P^{-1}(b-Wx_k)
\]

contracts its error in the M norm by at most `q_* < 1`, for every positive shift ratio. Moreover `omega_*` minimizes the worst-case norm bound over all real scalar relaxations and all dissipative J in this class.

**Proof of invertibility.** For `Wv=0`, taking a real inner product gives `a^{-1}||v||_M^2 = Re<v,Jv>_M <=0`, hence v=0; finite dimensionality gives invertibility. The same argument applies to P.

**Proof of the contraction.** Write `v=(I-a0 J)u`. Then

\[
\|u+a_0Ju\|_M^2-\|u-a_0Ju\|_M^2
=4a_0\operatorname{Re}\langle u,Ju\rangle_M\le0.
\]

Thus the reflected resolvent `C=2 R0-I=(I+a0 J)(I-a0 J)^{-1}` has norm at most one. The product ordering is valid because these factors are polynomials/rational functions of the same J. Put `s=a0/a`. Direct algebra gives

\[
P^{-1}W=I+(s-1)R_0={1+s\over2}I+{s-1\over2}C,
\]

and therefore

\[
T_*=I-\omega_*P^{-1}W
={a-a_0\over a+a_0}\,C.
\]

The error satisfies `e_(k+1)=T_* e_k`, proving the bound.

**Proof of minimax optimality.** For `J=0`, the iteration norm is `|1-omega s|`. For the dissipative family `J=-lambda I`, as `lambda` tends to infinity the norm tends to `|1-omega|`. Any uniform bound must therefore be at least `max(|1-omega s|,|1-omega|)`. Its minimum over real omega is attained by balancing the two endpoint errors with opposite signs, at `omega=2/(1+s)` with value `|1-s|/(1+s)`. The preceding upper bound attains this value, so it is minimax sharp. QED.

This is a project-specific use of the standard contractivity of a reflected monotone resolvent, not a claim of a new general operator-theoretic theorem. The existing `rodas5p_lagged.rs` already implements unrelaxed Richardson and GMRES, with logarithmic-norm, diagonal and resolvent staleness variants. This theorem supplies an additional, metric-qualified relaxation candidate; it does not replace those variants or show that this candidate is faster.

### Jacobian drift and approximate actions

Suppose the cached J0 satisfies `Re<v,J0v>_M <= -alpha||v||_M^2` for `alpha>=0`, while the actual target uses `J=J0+Delta`. Then

\[
\|P^{-1}\|_M\le {a_0\over1+a_0\alpha},\qquad
\|T\|_M\le q_*+\omega_*{a_0\|\Delta\|_M\over1+a_0\alpha}=:q.
\]

**Proof.** Cauchy-Schwarz and coercivity give the inverse bound. The perturbed iteration matrix is `T=T_*+omega_* P^{-1} Delta`; the triangle inequality gives the result. No commutativity of Delta and J0 is assumed. Require q<1 for this convergence guarantee.

If a numerical inverse action is R rather than exact P inverse, with `||R-P^{-1}||_M <= eps_R`, and the represented relaxation differs from omega star by `eps_omega`, a sufficient additional perturbation allowance is

\[
\varepsilon_\omega\|P^{-1}W\|_M+
|\widehat\omega|\varepsilon_R\|W\|_M.
\]

For fixed J, `||P^{-1}W||_M <= max(1,a0/a)`. Merely observing a favorable residual ratio is not a proof of either eps bound. Alternatively, keep the iteration heuristic and certify the final original-target residual with an independently valid inverse bound.

If actual J is known dissipative with margin alpha prime, its original scaled residual `r=b-W x` gives

\[
\|x-W^{-1}b\|_M\le {a\over1+a\alpha'}\|r\|_M.
\]

The factor a is essential: native W has units of inverse time. It must not be confused with the normalized matrix `I-aJ`, whose residual is in state units. This distinction also explains the current source's `|1-hP/h|` staleness formula.

### Stable computational form and experiment

Avoid computing `a+a0` or a potentially overflowing ratio. If `a<=a0`, put `r=a/a0`, then `omega=2r/(1+r)` and `q=(1-r)/(1+r)`. Otherwise put `r=a0/a`, then `omega=2/(1+r)` with the same q. A production certified implementation must also enclose parameter rounding and inverse-action errors. The supplied prototype intentionally does not claim directed certification.

`shifted_reuse_experiment.py` uses actual cached SciPy LU factors, original-target residual checks and manufactured known solutions. It runs 27 finite shifted systems: diagonal dissipative, damped nonnormal dissipative, and skew-adjoint matrices; ratios 0.1 through 10. All 27 operator-norm and exact-identity checks passed within the stated floating diagnostic tolerance. The strict residual threshold `1e-10` was met in 25/27 cells. Two cases at ratio 10 stalled near floating residual floors; both failures remain in `SHIFTED_REUSE_RESULTS.json`. The maximum observed forward error over all 27 cells was `2.4181321575052657e-11`; the maximum identity discrepancy was `4.719879393761364e-13`. An initial run had incorrectly asserted that all cells would meet the residual threshold and stopped on the first failed cell. The repair records both failures instead of changing the threshold; this is preserved in `EXECUTION_RECEIPT.json`.

`cayley_reuse.rs` supplies a standalone native-language primitive, including overflow-safe parameters, outward scalar bounds that include represented-omega error, the Jacobian-drift term, metric/model/domain identity refusal, and the original-scaled-residual-to-state-error conversion. Rust 1.94.1 compiled it and all 7 targeted tests passed (`RUST_RECEIPT.json`). Its mathematical inputs are declared full-domain bounds. It deliberately does not certify an external LU action, callback truth, or a floating residual evaluator. It is not connected to the production lagged solver or automatic selector.

The scalar enclosure argument assumes correctly rounded basic binary64 operations, gradual underflow and no compiler reassociation/fast-math alteration. Each upward primitive advances the rounded result toward positive infinity; downward denominators advance toward negative infinity and are bounded only by analytically valid nonnegative/one lower bounds. The positive ratio is enclosed before applying monotone parameter formulas. Overflow or an unresolvable strict contraction is refused. A model/metric/domain identifier prevents declared identity mismatch but does not prove that the declaration is true. The cached dissipation margin and actual-J margin in the residual conversion are separate hypotheses.

Unrelaxed iteration diverged in the diagonal ratio-0.1 and ratio-0.25 cases, while the new rule converged. However, the uniform minimax rule usually used more iterations in the strongly damped/nonnormal and skew families, because actual operator spectra can be much easier than the uniform class. Thus the justified conclusion is an enlarged guaranteed convergence region and a runnable candidate. A speed improvement for this theory candidate was not established, and activating it as a native default would be unjustified. Cheap unrelaxed iterations or direct refactorization can win. Parent mode-lane exact allocation changes are independent of this finding.

## 4. Theorem C: original-target stage certification with approximate inverses

Consider causal stage equations `W_i u_i=b_i(u_<i)` with candidate stages uhat_i and residuals `r_i=b_i(uhat_<i)-W_i uhat_i`. Let R_i be a fixed linear approximate inverse for stage i. Assume, in a fixed common norm,

\[
\|I-R_iW_i\|\le\delta_i<1,\quad
\|R_i r_i\|\le d_i,\quad
\|R_i[b_i(v)-b_i(w)]\|\le\sum_{j<i}K_{ij}\|v_j-w_j\|.
\]

The bounds must hold on a domain containing both candidate and true stage states; radius closure supplies this premise when available. Define

\[
E_i={d_i+\sum_{j<i}K_{ij}E_j\over1-\delta_i}.
\]

**Theorem.** Every stage error is at most E_i. For an output with coefficients beta_i, its added algebraic error is at most `sum_i |beta_i| E_i + reconstruction_error`. The finite recurrence needs no contraction assumption on the interstage matrix K.

**Proof.** Subtract exact and candidate equations, multiply by R_i, and rearrange to `(I-(I-R_iW_i))e_i = R_i r_i + R_i(b_i(u_<i)-b_i(uhat_<i))`, up to an irrelevant sign convention for r. The inverse norm of `I-(I-R_iW_i)` is at most `1/(1-delta_i)` by a convergent Neumann series. Induction over causal stage order gives the recurrence. Output reconstruction follows by the triangle inequality. QED.

This extends the already implemented causal path sum by absorbing each diagonal inverse-action defect before the triangular sweep. In particular, using an approximate polynomial inverse or a cached factor does not justify dropping delta_i. If delta_i is unavailable, the construction remains an operational estimate. A norm estimate from a handful of vectors does not certify its full-space operator norm.

For the ideal order-p time-stepping map with a stability bound `Lip(Phi_h)<=1+Lh`, ensure the added algebraic output error is `O(h^(p+1))`; the discrete Gronwall argument then preserves the order-p global error bound on a fixed interval. This conclusion is conditional on stiffness-uniform L and defect constants if stiffness-uniform order is claimed. An embedded pair alone does not prove these premises.

**Concrete counterexample.** Take `J=[[-1,K],[0,-1]]`, `a=1`, and an approximate zero solution of `(I-J)x=b` with `b=(0,epsilon)`. Every eigenvalue of J is -1, but the true solution is `(K epsilon/4,epsilon/2)`. For `K=10^8`, `epsilon=10^-8`, the residual is `10^-8` and the forward error is approximately 1/4. Wolfram exact rational evaluation returned `(1/4,1/200000000)`. A small residual and negative eigenvalues alone do not justify an original-state error certificate. This is a local counterexample to that inference, not a no-go for the algorithm.

## 5. Proposition D: error transport for a structural chart

Let `y=S(t)z+c(t)`, where S is invertible and continuously differentiable on the full interval. Define the exact transformed field

\[
g(t,z)=S^{-1}\{f(t,Sz+c)-\dot S z-\dot c\}.
\]

For a differentiable reconstructed curve yhat and residual `d_z=zhat'-g(t,zhat)`, direct differentiation gives

\[
d_y=\widehat y'-f(t,\widehat y)=S(t)d_z.
\]

For an implemented transformed field gtilde, define separately `dtilde_z=zhat'-gtilde(t,zhat)` and `eta=gtilde-g`. Then `d_y=S(dtilde_z+eta)`. A physical residual enclosure must therefore include the transform norm, structural-model defect and numerical reconstruction errors. A small transformed residual alone does not establish a small original-space error when S is ill-conditioned or large in the physical norm.

For a known flow sensitivity bound Gamma(T,t), variation of constants/comparison yields a physical output error bound by the propagated initial error plus `integral Gamma(T,t)||S(t)|| (||d_z||+||eta||) dt`, with additional accounted reconstruction jumps. For nonlinear f this requires a valid Lipschitz/logarithmic-norm tube bound, not endpoint sampling. These are exactly the domain/provider obligations pursued by the separate V8 research; they are not supplied automatically by an adapter speed benchmark.

The existing exponential-chirp/cofactor charts use declared analytic structure and explicitly distinguish phase evaluation from a directed phase enclosure. Their mathematics is valid for that declared family. There is no evidence here that arbitrary noncommuting carriers or arbitrary RHS callbacks satisfy the same formulas. Their extension requires deriving the correct time-ordered carrier and bounding its defect rather than changing a loop or caching trigonometric calls.

## 6. Primary-literature checks and limits

1. P. Tranquilli and A. Sandu, *Rosenbrock-Krylov Methods for Large Systems of Differential Equations*, SIAM J. Sci. Comput. **36**, A1313-A1338 (2014), DOI `10.1137/130923336`, arXiv `1305.5481`. Its order theory jointly treats time discretization and Krylov approximation. The full primary text, Section 1 following equations (2a)-(2b), distinguishes the exact-Jacobian/exact-stage premise of classical Rosenbrock methods from separately constructed W methods. Sections 3 and 3.4 treat order conditions and finite-difference errors. Source read: https://arxiv.org/html/1305.5481v2 . This does not justify arbitrary projected solves in an unchanged Rosenbrock tableau.
2. M. Narayanamurthi, P. Tranquilli, A. Sandu, and M. Tokman, *EPIRK-W and EPIRK-K Time Discretization Methods*, DOI `10.1007/s10915-018-0761-3`. The primary NSF-hosted manuscript returned by Exa discusses separate W/K order conditions for approximate Jacobians. Subsequent web open returned HTTP 502; the retrieved primary text supports only this contextual statement, not the new proofs above.
3. L. Liu, W. M. Moursi, and J. Vanderwerff, *Strongly nonexpansive mappings revisited: uniform monotonicity and operator splitting*, arXiv `2205.09040` (2022). Full primary text read: Section 2, Fact 2.5 and equation (14), gives the monotone-operator/firmly-nonexpansive-resolvent/nonexpansive-reflected-resolvent equivalence. Source: https://arxiv.org/html/2205.09040v1 . The finite-dimensional dissipative result above is proved directly; our sign convention maps the paper's monotone operator to `A=-a0 J`.

SciSpace supplied bibliographic discovery; Exa supplied primary-source discovery/text. The finite 2x2 Wolfram calculation checked Cayley and noncommuting-drift identities exactly. None of these tools is counted as a proof of native implementation correctness or a timing result.

## 7. Admission table and next concrete action

| Result | Evidence state | Native status | Remaining obligation |
|---|---|---|---|
| Prefix cost dominance | Derived, source-aligned; parent adapter tests separate | Existing reasoning reused; proactive stop patch owned by parent lane | Verify fatal semantics, full charging, conservative arithmetic and actual timing |
| Cayley optimal relaxation | Derived; exact finite CAS; 27 numerical norm checks | Research prototype only | Metric/provider proof, floating action enclosure, native performance selection |
| Inexact causal stage bound | Derived; existing triangular theorem extended | Existing majorant reusable; new delta handling is contract | Bind inverse defects, tube closure and all requested outputs |
| Physical chart residual transport | Derived standard identity | Declared charts exist | Whole-domain structure, reconstruction and flow bounds for each physical client |
| Fundamental invalidity of project | Not established | No broad rejection | Audit each concrete applicability claim |

The next mathematical implementation should preserve native target equations and add a declared dissipative-metric candidate to the existing lagged solver, compare it with current unrelaxed/GMRES/refactor paths, and admit it only when measured total cost wins at the same original-state error. The present numerical evidence explicitly argues against unconditional activation.
