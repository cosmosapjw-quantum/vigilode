# Exact-constant physical-accuracy extension

User steering: scientific constants must follow from numerical/algebraic/analytic
proofs rather than arbitrary algorithmic hyperparameters. Existing heuristic
arms and results remain controls. This is an additive, model-specific consumer.

Frozen before execution:

- Model `y'=-y-.5 y^3`, `y(0)=1`, horizon1; requested physical absolute error
  budgets `1e-3,1e-4,1e-5`.
- Let `Phi(Y)=kY+gY^3`, `L=k+3gY^2`, `C=L^2 Phi(Y)/8`.
  Select the smallest dyadic N with exact represented h=T/N,
  `N C h^3/(1+hk/2)<epsilon`, and
  `h[k+3g(Y+epsilon)^2]/2<1`.
- The algebraic step error budget is the outward lower remainder
  `(epsilon-E_trunc)/N`; the initial operational residual target is that budget
  times the lower inverse denominator `1+hk/2`.
- Every returned step is independently checked with an outward interval of
  the original polynomial stage residual and the exact represented addition
  displacement. A monotone derivative-free bisection polish is used only when
  this verification requires it. All map and interval evaluations are charged.
- Compare Picard and new ResidualDominance, with zero and previous-stage seeds.
  ResidualDominance uses eta=1 and only finite/domain admissibility. The previous
  0.9 gain margin, 0.5 ratio, 1024 coefficient cap and two-rejection cap do not
  affect this mode.
- Direct final target lambda=1. No arbitrary intermediate station schedule is
  added: intermediate contraction does not repair the final target's factor.
- Resource caps 16384 time steps and1,000,000 polynomial evaluations are host
  execution limits, not scientific accuracy or convergence constants.
- Same physical accuracy budget and same derived plan in all four arms;
  report total work including interval verification and any failure.
- Check all returned grid states against the independently known exact ODE
  solution, and compare actual error with both the requested cap and computed
  outward error bound. No timing claim is made by this count experiment.

The shared production module receives a distinct ResidualDominance enum variant
and sealed model planner; generic map users are not assigned a fabricated q.
