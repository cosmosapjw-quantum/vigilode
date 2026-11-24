# Independent review of computational depth and structured alternatives

## Fixed RODAS5P and its forcing subproblem

For the identity-mass ODE, the native fast driver uses

$$
 W u_i=f(t+c_i h,y+\sum_{j<i}a_{ij}u_j)
          +\sum_{j<i}(C_{ij}/h)u_j + h\gamma_i f_t(t,y),\qquad
 W=I/(h\gamma)-J .
$$

For uniformly scaled state components, `W` has units of inverse time and `u_i` has units of the state. A fixed state Jacobian and a single matrix factorization are shared within a trial. The V05 exact graph at the archived tableau has seven nonzero subdiagonal entries in the RHS state dependence. For an arbitrary smooth oracle evaluated at each actual state, an evaluator missing a queried stage state can be distinguished by two right-hand sides differing by a bump supported at that point. This establishes the serial chain for generic distinct stage inputs in the classical explicit point-query implementation. A constant/affine oracle, a declared invariant subspace, an evaluable model description and an altered tableau change the question. The archive's sweep-growth measurements depend on the discretization and norm. Their scope is the stated test family.

An additional factorization of the **callback** is useful even for nonlinear state dependence. Suppose

$$
 f(t,y)=N(t,y)+g(t),
$$

where both terms reproducibly return the same values for a fixed model and `g` may be evaluated out of order. In the snapshot the abscissae are 0, 0.6358126895828704, 0.4095798393397535, 0.9769306725060716, 0.4288403609558664, 1, 1, 1. Generate `t` directly at stage 1 and `t+c_i h` by the native operation everywhere else. Six independent forcing rows suffice on distinct represented clocks. Equal clocks may be reused even if the nonlinear states differ. Adding the original forcing value to the original nonlinear RHS at each sequential stage, in the same floating-point order, gives equal stages by induction, equal embedded error, and the same controller decision. This statement permits arbitrary smooth, non-affine, time-dependent `N`. If a numerical partial time derivative requests off-grid clocks, recompute `g` there. An analytic callback must supply `N_t+g'`. A full model epoch change requires rebuilding the table. Errors at speculatively visited clocks must be delivered only if requested. Callback side effects invalidate the computational contract.

With `P` workers, per-row forcing cost `G`, state/solve cost `K` over a trial, and `q` off-grid derivative calls of cost `G`, a simple matched model gives `T_direct≈K+(8+q)G` and `T_pre≈K+(ceil(6/P)+q)G+H`, with overhead `H` including table initialization, copies, lookup and dispatch. This explains the separable effects of 8-to-6 deduplication and concurrent evaluation. Measurement must include both, and compare prefetch at one worker. Across rejected steps and changing trial grids, fallback evaluation adds real work. The implemented method prefetches one proposed grid. A persistent adaptive hook that fills rows before each actual trial requires a separate validation of partial evaluation/error order, clipping, and cache lifetime.

## Iteration certificates and a fold counterexample

For a nonlinear implicit equation `H(u)=0`, fix a bounded, injective approximate inverse `B` and define `T(u)=u-B H(u)`. On a closed norm ball centered at `u0`, establish the rigorous bounds `||BH(u0)|| ≤ Y` and `sup ||I-B DH(u)|| ≤ Z < 1` (or a direct Lipschitz bound on `T`). If `Y+ZR ≤ R`, Banach's theorem gives an enclosed unique root and `||u*-u0|| ≤ Y/(1-Z)`. Metrics and scaling must refer to this same ball, norm and approximation. A posteriori floating samples of a JVP or a ratio of successive residuals cannot bound the stated supremum. A derivative-free runtime corrector may accept a model-specific interval or analytic bound from its provider.

Exact nonnormal check: `J=[[-1,1000],[0,-1]]`, `W=I-J`, and residual `r=(0,0.001)`. Both eigenvalues of `J` equal -1, yet `W^{-1}r=(1/4,1/2000)`; `||W^{-1}r||∞/||r||∞=250`. These rational values were verified with Wolfram Language. A residual-only stopping threshold needs a bound on inverse action or a tighter validated observable error proposition.

For backward Euler applied to `u'=u²` from `u_n=1` with `h=1/2`, use `H(u,tau)=u-1-(tau/2)u²`. The discriminant is `1-2 tau`, the initial-connected branch reaches a simple fold at `(u,tau)=(2,1/2)`, and the terminal equation at `tau=1` has no real root. The ODE itself is finite at physical time 1/2. Pseudo-arclength can follow the zero set locally through the fold; it cannot create a real terminal root. An implicit solve needs branch identity, endpoint existence and a smaller-step recovery. The fold is diagnosed by `H=H_u=0`, `H_uu H_tau != 0`. In more than one dimension use a null/cokernel transversality test. Singular geometry has an actionable role at this point. This counterexample constrains extrapolation of residual certification and continuation to all implicit equations.

## Methods, novelty and discriminating experiments

| Route | Work made parallel or derivative-free | Qualification experiment |
|---|---|---|
| Fixed RODAS5P | Causal stage sequence, single linearization; explicit JVP avoids dense assembly in the matching backend. Declared forcing or spectral modes expose independent subproblems. | Same tableau, model, controller, accepted/rejected work, embedded estimator, achieved error and complete wall time. Include nonlinear, nonnormal and near-zero estimator cases. |
| BDF | Krylov residual solves can use directional products and preconditioners. BDF retains step/history dependence; a derivative-free fixed-point corrector needs a convergence bound. | Compare with CVODE-BDF at identical requested outputs, achieved accuracy, supplied structure and callback cost. Count Newton/nonlinear residuals, rejections, setup/solves and start-up. |
| Radau IIA | Stage-block preconditioners or basis transformations enable parallel inner work on structured linear/semi-linear problems. A real Schur form alone preserves triangular coupling; the choice of preconditioner matters. | Use the identical PDE discretization. Include transforms, complex or paired real stage blocks, Krylov residual evaluations and nonlinear outer iterations. |
| Peer W | Independent stage linear equations use the previous-step history. Starting and changing step sizes influence the full run. | Include initialization, retry, dense output and restart. G0 holdout needs to be repeated with a changed mechanism and fixed stopping rule. |
| DOPRI, Lawson, exponential | Explicit stages are naturally Newton-free; exponential/Krylov actions obtain different parallelism from sparse actions or independent lanes. Split time forcing is possible for known abscissae; identities differ by method. | Explicit stiffness/stability margins, Krylov or transform work, time derivative semantics, dense output, model/provider transitions. |
| Coupled nonlinear and homotopy | A secant/derivative-free correction may have a validated contraction region; explicit continuation permits adaptive chart switching near folds. | Compare normalized residual *and* independently bounded state error; include the simple fold above and random nonnormal perturbations. Fallback at uncertified radius, exhausted work or loss of endpoint branch. |
| Hybrid routing | A symbolic itinerary is obtained from actual sequences of guards and accepts/rejects. Deterministic hysteresis makes its transitions reproducible. | Replay guard transitions under threshold perturbations, include rejected exploratory calls, bound fallback loss and hold out unseen trajectories. |
| Spatial | A method-of-lines spatial dependency graph is distinct from the time-stage DAG. Explicit stencils exchange finite halos. An implicit diffusion resolvent generally has global support even from a sparse stencil. | Charge halo, preconditioner, boundary callbacks and reductions at equal spatial discretization error. Test sphere null mode, invariant measure and high spatial frequency. |

Stage-parallel peer methods, common stage linearizations, JVP/Krylov integration, diagonal SDC sweepers, and stage-parallel implicit RK already have prior art. A prospective contribution is a demonstrably better *conditional* work–precision envelope for explicit structural contracts, with accurate fallback and rejected-work accounting under one frontend. Evidence on one declared synthetic family can support implementation correctness and local pay-off. A numerical-method novelty or general superiority claim requires independent competing methods and realistic, locked holdouts.

## Primary references and reusable evidence

- Steinebach (2023), construction and accuracy/stability rationale for Rodas5P, including eight coefficients rows, order reduction and dense output. https://doi.org/10.1007/s10543-023-00967-x
- Schmitt & Weiner (2004), Parallel Two-Step W-Methods with Peer Variables. https://doi.org/10.1137/S0036142902411057
- Axelsson et al. (2024), stage-parallel preconditioners for fully implicit Radau RK on linear problems. https://doi.org/10.1002/nla.2532
- Speck (2018), parallel SDC iterations and diagonalization. https://arxiv.org/abs/1703.08079
- Improving Efficiency of Parallel Across the Method Spectral Deferred Corrections, new diagonal sweepers. https://doi.org/10.1137/24M1649800
- SUNDIALS CVODE numerical methods, multistep formula, nonlinear solvers, matrix-based and matrix-free linear algebra. https://sundials.readthedocs.io/en/latest/cvode/Mathematics_link.html
- Dickson, Kelley, Ipsen & Kevrekidis (2007), conditioning and transversality at simple folds for pseudo-arclength continuation. https://doi.org/10.1137/060654384
- Repository source paths: `fixtures/rodas5p_coefficients_snapshot.json`, `crates/rodas5p-integrators/src/rodas5p_fast.rs`, `research/production_loop_20261116/theory/THEORY.md` sections S1–S2, `research/production_loop_20261116/level_tableau/REPORT_KO.md`, `research/vigilode_r6_eightstage_causal_frontier_20261101/RESULTS.json`.

## Independent matched-problem check

The scalar nonlinear equation has exact solution `x(t)=1/4+sin(t)` on `[0,1]`, with `N(t,y)=-80 y-y³` and `g(t)=cos(t)+80x(t)+x(t)³`. The public Rust client and direct formulation have identical returned states. SciPy 1.17.0 BDF and Radau use the analytic Jacobian. The requested first step is 0.05 and absolute tolerance is 0.01 times relative tolerance in every arm. RODAS5P also receives the analytic partial time derivative. The table records the actual number of accepted steps and absolute endpoint error.

| Relative tolerance | VigilODE RODAS5P | SciPy BDF | SciPy Radau |
|---|---|---|---|
| 1e-4 | 4 / 6.80e-6 | 12 / 8.57e-8 | 3 / 8.20e-6 |
| 1e-6 | 13 / 1.33e-8 | 23 / 6.17e-9 | 7 / 6.35e-9 |
| 1e-8 | 42 / 2.96e-10 | 40 / 2.97e-10 | 22 / 2.60e-10 |

At tight tolerance all three attained errors agree within about 15%, with Radau taking the fewest accepted steps. At the loosest tolerance BDF overachieves endpoint accuracy substantially. Relative work and runtime must be assessed on the corresponding achieved-accuracy curves with each method's rejected steps, factorizations and derivative evaluation cost. `results/manufactured_comparators.py` generates the independent SciPy measurements. The Rust test `manufactured_nonlinear_solution_with_analytic_reference` computes the other column.
