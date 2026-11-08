# Rosenbrock stage transformations and CPU parallelism

This is a research addendum, not solver qualification. Source lock: research/vigilode-c5-native-graph-schur-20261102, commit 569ce02629498fec133db9ce368eb303ba943b69, tree a4b2f09521b197b37612ff7dc66a13fbb74a0bae. GPU execution is excluded.

## 1. Stage-dependency barrier

A source-owned raw-U RODAS stage has schematic form:
W_n U_i = h gamma f(t_n+c_i h, y_n+sum_(j<i) A_ij U_j) + gamma sum_(j<i) C_ij U_j + h^2 gamma gamma_i f_t,
with W_n=I-h gamma J_n. Exact coefficient names/signs must be read from the pinned tableau. Later current-step stage RHSs depend on earlier current-step stages. Computing them from stale/guessed values changes the method.

Separate one-step RODAS/ROW, two-step peer-W, affine/linear transformed stages and source-specific nonlinear closures. They are different method classes.

## 2. Main general candidate: two-step parallel Rosenbrock-W peer methods

Representative peer form:
Y_ni = U_n + h_n sum_j a_ij U_(n-1,j) + h_n sum_(j<i) a_tilde_ij U_(n,j)
(I-gamma h_n T_n) U_ni = F(Y_ni) + h_n T_n sum_j gamma_ij U_(n-1,j) + h_n T_n sum_(j<i) gamma_tilde_ij U_(n,j)
U_(n+1) = U_n + h_n sum_i (b_i U_ni + v_i U_(n-1,i))

Notation varies among primary papers. Recover exact coefficient tables and startup/output formulae before code. When same-step coupling coefficients a_tilde and gamma_tilde are zero, current stages use only prior-step peer data. Their RHS and linear solves can run concurrently. They share I-gamma*h*T_n, allowing one factorization with multiple RHS or independent/block Krylov solves.

Literature establishes parallel two-step W/peer methods: Schmitt-Weiner 2004 prove order s-1 for general step sequences and identify methods up to order 7. Podhaisky-Weiner-Schmitt 2005 construct Rosenbrock-type peer methods with stage order s-1, variable-step zero-stability and L(alpha)-stable members for s=4..8. These results do not qualify an arbitrary coefficient table or VigilODE adapter.

First candidates:
- PW6-P5: six independent peer stages, variable-step global order >=5.
- PW8-P5: eight independent stages, conservative order-five parity candidate.
- PW8-P7: exploratory only if a verified variable-step coefficient set, startup and estimator exist.

Do not report constant-step effective order as variable-step order. Startup must generate all peers at required order. Rejected steps, events, discontinuities and step-ratio changes need consistent restart. High stage order and stiff damping may reduce order reduction, but matched-error full-IVP measurements must establish low step counts.

## 3. Finite resonant normal-form lift

Near an equilibrium, consider analytic/polynomial autonomous system x_dot=A*x+F_ge2(x). A convergent Poincare-Dulac transformation x=Phi(z) gives z_dot=A*z+R(z), where z^mu e_j is resonant when mu dot lambda=lambda_j and |mu|>=2. In a Poincare domain (convex hull of spectrum excludes zero), the resonant family is finite under classical hypotheses.

Let phi_1(z),...,phi_r(z) be the finite resonant monomials and set w_i=phi_i(z). Their Lie derivatives close linearly on the resonant span. Augmented coordinate xi=(z,w) obeys a linear parent system xi_dot=B*xi. Original normal-form trajectories lie on algebraic invariant manifold M={w_i-phi_i(z)=0}. Evolve the parent from consistent lifted initial data, project to z, then through x=Phi(z).

This is local. The transform must converge and be conditioned; the trajectory must remain inside its chart; transformation/projection errors must be transported to original-state error. Resonant lift dimension may grow rapidly with stiffness ratios.

### 3.1 Linear parent stage-block identity

For a constant linear/affine parent B, group the raw-U stages as
Lcal*Uvec=rvec,
Lcal=I_s kron W - h*gamma*A_s kron B - gamma*C_s kron I,
W=I-h*gamma*B,
where A_s,C_s are the source-owned strictly lower triangular stage coefficient matrices.

Factor:
Lcal=(I_s kron W)*(I-N),
N=h*gamma*A_s kron (W^-1*B)+gamma*C_s kron W^-1.
N is strictly lower block triangular, hence N^s=0. Therefore (I-N)^-1=I+N+...+N^(s-1). For s=8:
I+N+...+N^7=(I+N)(I+N^2)(I+N^4).
The polynomial identity was checked symbolically; the Wolfram difference equals zero. This is not a runtime result. Count power application, multishift/Krylov basis, setup and memory. If the factorization still performs eight serial N applications, stage span has not been reduced.

For nonlinear RHS, the exact block identity applies only after exact finite lift or to an affine part. Finite Picard/homotopy correction does not automatically preserve original RODAS stage values or estimator semantics.

## 4. Fourier involution and convolution

PP05/PP06 already implement a source-specific Fourier-polynomial path:
- sparse coefficients indexed by harmonic k and formal degree j;
- coefficient involution (k,j,c)->(-k,j,conjugate(c));
- exact noncyclic convolution for the source RHS;
- PP06 padded 2D FFT is only an untrusted predictor;
- PP05 acceptance recomputes original lambda=1 RHS with directed complex intervals.

This is native and nontrivial, but not a generic finite closure. Harmonic support can grow, aliasing can fold high modes into low modes, and the invariant leaf may be left.

Recorded failures:
- PP05 reference enclosure G1 passed, but source-parity ratio reached 1.011159 against registered 1.01 (G2 FAIL); cyclic-alias negative candidate was accepted because still accurate (G3 FAIL).
- PP06 FFT/direct discrepancy was at most 5.33e-16 on 48 calls, but harmful-alias gate failed.
- PP07 found no shifted solve in the actual Fourier client and correctly abstained.
- PP15 has no timing. At omega=40, RODAS achieved error 4.56e-12 exceeded Fourier bound 2.89e-12; at omega=1e4 the RODAS error was far above the Fourier bound. This is not matched-error performance evidence.

## 5. Three CPU parallelization nodes

P1 stage-internal: RHS row/component, JVP/operator, preconditioner, Krylov reductions/orthogonalization, multiple-RHS solves. Protected one-step RODAS stages remain sequential. parallel.rs and stage_batch.rs are infrastructure/probes, not proof of independent true RODAS stages.

P2 independent ensembles: generalize DOPRI5 Ensemble to method-agnostic IVP/parameter batches. Each member owns controller, norm, history, partial result, failure and work ledger. Never pool error norms or step-size history.

P3 spatial MOL CPU: separate sphere/2D/3D geometry providers from time method. Test harmonics/spin harmonics, sphere finite volume/cubed sphere, area-weighted norms, pole/chart overlap, Laplace-Beltrami modes; Cartesian anisotropic diffusion/advection/reaction, refinement and conservation. Parallelize CPU rows/cells/modes and count communication/reductions.

GPU is forbidden in every node and fallback.

## 6. Decision

General candidate: parallel peer-W. Local high-risk candidate: normal-form lift. Source-specific candidate: Fourier-Volterra. Protected RODAS5P remains the baseline and receives dependency-preserving stage-internal CPU optimizations only. No candidate is promoted on nominal steps, symbolic identities, synthetic kernel timing or unmatched achieved errors alone.


## 7. Existing algebraic and continuation infrastructure to reuse

The pinned source already exports `conditioned_polynomial` and `polynomial_resolvent`. They provide declared chart/operator identities, polynomial actions/refinement and outward residual infrastructure. They are not normal-form generators and do not establish the truth of caller-supplied declarations. The new resonant lift should reuse their operator/action and residual machinery where the algebra fits; new code should be limited to resonance enumeration, homological equations, parent-matrix construction and invariant-ideal proof.

The existing `fixed_point_continuation` module accepts a source-bound map of the form `T(x,lambda)=x-P F(x,lambda)`, with declared uniform contraction, inverse-action, center-displacement and closed-patch bounds. It explicitly does not certify an ODE trajectory. Reuse it only for a correction subproblem that satisfies those declarations; do not write another continuation engine and do not reinterpret a root certificate as a trajectory certificate.

The existing `vigilode-coupled::rhs_homotopy` provides an independent RHS-only map contract, finite evaluation budget and residual-screened acceleration. Use it for compatible map/root work, while retaining the exact callback and error-bound obligations. `actual_k_homotopy`, `nonlinear_homotopy`, `structural_mean_field`, `collective_rodas5p`, `quotient_flow` and `affine_transport` are further source-specific building blocks documented in `REUSE_FIRST_INVENTORY.md`.


## 8. Refined peer-family taxonomy and exact two-step W equations

The earlier schematic peer equation is not enough to select a method. The implementation must distinguish the linearly implicit **two-step W** form from the broader general-linear peer form. A source-locked two-step W method (Klinge 2020 dissertation, Eq. 2.1.2) is
[
Y_{m,i}=u_m+h_msum_{j=1}^{s}a_{ij}k_{m-1,j}
+h_msum_{j=1}^{i-1}\widetilde a_{ij}k_{m,j},
]
[
(I-h_mgamma T_m)k_{m,i}
=f(t_{m,i},Y_{m,i})+h_mT_msum_{j=1}^{s}\gamma_{ij}k_{m-1,j}
+h_mT_msum_{j=1}^{i-1}\widetilde\gamma_{ij}k_{m,j},
]
[
u_{m+1}=u_m+h_msum_{i=1}^{s}(b_i k_{m,i}+v_i k_{m-1,i}).
]
Here (k_{m,i}) approximates the time derivative at (t_m+c_i h_m), (T_m) is a W-matrix approximation to (f_y), and (sigma_m=h_m/h_{m-1}) enters variable-step coefficients. The strict lower-triangular matrices \(\widetilde A\) and \(\widetilde\Gamma\) couple current stages. **The actual parallel subclass sets both to zero.** Then all stage equations depend only on previous-step data and share (I-h_m\gamma T_m), so one direct factorization can serve the (s) independent right-hand sides.

The simplifying order conditions are (C(q)), \(\Gamma(q)\) for stage order and (B(p)) for the advancing output. The cited convergence statement is (p_* = \min(q+1,p)) for bounded stepsize ratio and compatible starting errors. The (q=p=s) design is therefore a genuine candidate, but its stability/step-ratio properties must be verified for the selected coefficients. Do not infer an (L(\alpha))-stable table from the existence theorem alone.

A second distinct candidate is **multi-implicit peer two-step W**. It uses only previous-step peer data but assigns different implicit system parameters to different stages. Its order-(s-1) subclass was designed for optimal stiff damping and uniform stability for realistic step-size sequences. This trades a shared LU for independent stage-specific shifted systems. Use existing Krylov/operator/preconditioner services, but count each solve and test whether any shifted-basis reuse is mathematically valid.

A third candidate is the fully implicit **parallel peer** method. Its stage nonlinear equations can be independent and it has an order-(s-1) zero-stable subclass for general step sequences. It is a fallback research branch because nonlinear solves may cost more than linearly implicit W stages.

Keep two sequential controls separate:
- Rosenbrock-type peer two-step methods with lower-triangular current peer coupling are not parallel unless the selected matrix is diagonal/block-independent. Stage order (s-1), variable-step zero-stability and (L(\alpha)) stability make them valuable accuracy/stability controls. Constant-step effective order (s) is not the variable-step order.
- A separate stiffly accurate two-step W construction with an added variable-step superconvergence condition can attain order (s+1), but the cited construction loses current-stage parallelism. It quantifies the accuracy/concurrency tradeoff.

## 9. Reuse the published peer error controller

Kulikov & Weiner (2011), DOI 10.1016/j.cam.2011.08.006, already develops local and global error estimation plus automatic step-size control for linearly implicit parallel two-step peer W methods, with stiff/very stiff/large-scale tests and a RODAS comparison. The correct next step is to fetch the estimator equations and determine whether they apply to the exact selected coefficient family. Do not write a generic embedded RK estimator first and hope it transfers: a two-step GLM's global error recurrence and startup history are part of its method identity.

## 10. Startup and restart are first-class method equations

Two-step W schemes need the initial stage derivatives (k_{0,i}); peer schemes need (s) starting peer values. The startup method must match the required stage/global order and be included in short-IVP cost. A published parallel start procedure for **explicit** peer methods is not automatically valid for stiff linearly implicit peer-W. On rejection or event/discontinuity, retain the last committed state, recompute step-ratio-dependent coefficients and reconstruct only the history required by the selected method. The step controller must enforce the proven step-ratio domain.
