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
