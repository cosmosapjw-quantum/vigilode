# WP9B physically forced perturbation stability, actual observables and policy feedback

## Scope and definitions

Frozen source: original cooperative cubic RHS (Git blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482) and four WP1 fixture declarations (Git blob 1bfc17ec2ed4b902cc06073807388f6e332e31d6). The refined asymmetric_2 fixture is a resolution-equivalent source, NOT an independent physical model. Three independent model families and fourteen independent parameter-axis choices are studied. Two different integrators generate actual ODE trajectories at a new registered horizon 0.15 and +/-3% perturbations; an independent after-selection holdout uses horizon 0.20 and +/-1.8%.

Let Q(theta) be the row-conservative Metzler exchange generator, with Q_ij(theta)>=0 for i unequal j and Q 1=0; pi(theta)>0 is normalized and pi(theta)^T Q(theta)=0. Define

  f_theta(y) = Q(theta)y - k(theta)⊙y - g(theta)⊙y^3 + q(theta),

where all k_i>0, g_i>=0. The external constant forcing q is a different perturbed model from the zero-source baseline. Natural units are irrelevant because the input is a dimensionless source-conditional ODE. Define the observable W_theta(t)=pi(theta)^T y_theta(t).

## Derived result: weighted contraction and parameter-forcing stability

For two trajectories y_theta and v_theta of the SAME parameterized IVP model, let z=y_theta-v_theta. The exact identity

  y_i^3-v_i^3 = (y_i-v_i)(y_i^2+y_i v_i+v_i^2)

implies z'=Q(theta)z-D(t)z where D_ii(t)=k_i(theta)+g_i(theta)(y_i^2+y_i v_i+v_i^2)>=k_i(theta)>0 for real y,v. Positivity of the off-diagonal elements of Q gives componentwise Kato inequality

  d|z|/dt <= Q(theta)|z|-D(t)|z|

almost everywhere, including zeros by the upper Dini derivative. Multiply by stationary pi(theta)^T to obtain

  d/dt ||z||_{1,pi(theta)} <= -k_min(theta)||z||_{1,pi(theta)},

hence ||z(t)||_{1,pi(theta)} <= exp[-k_min(theta)t]||z(0)||_{1,pi(theta)}.

Compare two different parameter models, y_theta(t) and y_0(t), with the same initial state y_0(0). Let delta=y_theta-y_0 and R_theta(t)=[f_theta-f_0](y_0(t)). The same Kato argument using Q(theta) yields a parameter-forced stability inequality:

  ||delta(t)||_{1,pi(theta)} <= Integral_0^t exp[-k_min(theta)(t-s)] ||R_theta(s)||_{1,pi(theta)} ds.

Because all registered parameter perturbations enter f linearly, R_theta(s)=theta*S_axis(y_0(s)), with S_axis determined exactly by the source definition. No assumed Taylor expansion or frozen pi is needed.

For observable differences, distinguish the weight variation:

  |W_theta(t)-W_0(t)|
  <= ||delta(t)||_{1,pi(theta)}
     + Sum_i |pi_i(theta)-pi_i(0)| |y_0,i(t)|.

The second term is essential for non-symmetric exchange and cannot be discarded. This is a DERIVED analytic inequality, for the registered cooperative-cubic source class, not a claim of a new general theorem or formal HOL verification.

## Numerical check of derived inequality

The companion bound_check.py evaluates the forcing norm on the actual registered unperturbed ODE trajectory and integrates the exponential convolution with sampled Simpson quadrature for each measured signed perturbation. The resulting finite-grid bound is a NUMERICAL APPROXIMATION, not an interval-certified upper bound, and must not be labelled rigorous. The analytical inequality is exact under stated assumptions; the numerical quadrature and integration errors are not rigorously bounded in this package. All source bits are checked again before calculating witnesses.

## Adaptive search / realistic limitations

The correction is operational: every actual Radau+DOP853 experiment updates the measured physics reward, the posterior-shrunken axis means, nonuniform ProgramDatabase-backed genetic mutation probabilities and the MCTS-PUCT root priors before the next wave. Uniform control uses the same real source and numerical reward but retains fixed uniform priors. Each wave records before and after distributions and the actual ProgramDatabase insertion. Genuine ProgramDatabase is imported from the hash-locked local OpenEvolve ZIP. The search is offline and LLM calls=0. This is a root-expansion PUCT policy study, NOT full many-rollout MCTS or a trained neural policy.

Double counting safeguard: the refined asymmetric_2 and original share a single canonical physical reward and source-indexed integration cache. They may appear as different experimental control genotypes but do not contribute independent posterior observations. The original two-arm preregistration was committed and pushed before simulations.

A policy score difference accumulated on the same queried candidates is IN-SAMPLE and cannot establish better out-of-sample science. The post-selection holdout comparison is reported without promoting the result to a performance improvement when both arms select the same candidate. Cross-integrator agreement does not replace a rigorous global error interval; native BE-v-RODAS5P matched achieved-error on the same runtime, observational REI Bianchi IGM owner-IVP, HOL full theorem, and distinct-author C9 remain unrun.

Official DAG 7/23, admitted successor 0/12, production HOLD, no merge/default change/tolerance relaxation.
