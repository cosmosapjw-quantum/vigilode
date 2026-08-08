# Homotopy Numerical Experiments Cycle 1 — Design Specification

Date: 2026-08-07
Base commit: `ce1cc51034186958da3e27f641f29aae777ceaf1`
Branch: `homotopy-numerical-experiments-v0.5`

## Goal

Implement the smallest nonlinear partial-coupling continuation experiment that can decide whether
low-depth nilpotent propagation (`q=0,1,2`) reaches the original RODAS5P output-defect budget in
fewer effective rounds than equal-work SABR/Picard, while preserving a protected sequential
fallback.

## Protected scientific object

For the frozen step context,

\[
(D-C)K=g+hN(K),
\qquad
D=I_8\otimes W,
\qquad
C=hL\otimes J_n,
\qquad
W=M-h\gamma J_n.
\]

The fixed-schedule partial-coupling path is

\[
H_\theta(K,\lambda)
=(D-\theta C)K-g-\lambda[(1-\theta)CK+hN(K)].
\]

The endpoint at \(\lambda=1\) is exactly the original structured RODAS5P stage equation.

## In scope

1. A public read-only nonlinear-remainder snapshot reusing `StructuredBlockSystem` semantics.
2. Matrix-free/reference applications of `D`, `C`, and `D-\eta C`.
3. Common-`W` finite inverse actions at `q=0,1,2,7`.
4. Uniform fixed schedules in `lambda`.
5. Euler and two-step secant/AB2 path predictors.
6. Zero or one frozen-linear residual correction at each path point.
7. Exact dense original-target Jacobian and output-weighted defect certificate for problems with
   explicit stage Jacobians.
8. A transactional `homotopy_step` with protected sequential RODAS5P fallback.
9. Canonical single-step screens:
   - affine and scalar-linear exact limits;
   - Prothero–Robinson stiffness/nonlinearity grid;
   - noncommuting mass/nonnormal manufactured-vector grid.
10. Equal-backend controls: sequential RODAS5P and SABR5P.
11. Machine-readable CLI output and durable work/timing ledgers.

## Out of scope

- adaptive `lambda` or `theta`;
- Anderson acceleration;
- Chebyshev or Padé path approximation;
- BDF/Radau ranking;
- GCRO-DR retuning;
- sparse/MPI/GPU claims;
- singular-mass or index-1 DAE promotion;
- production API stability.

## Algorithm

For a uniform schedule \(0=\lambda_0<\cdots<\lambda_m=1\):

1. Construct a start approximation by applying the `q`-truncated inverse of
   \(D-\theta C\) to \(g\).
2. At each \(\lambda_j\), evaluate \(N(K_j)\) with one batched stage RHS call.
3. Form the frozen-linear tangent right-hand side
   \[
   b_j=(1-\theta)CK_j+hN(K_j).
   \]
4. Apply the `q`-truncated inverse of \(D-\eta_j C\) to obtain a tangent approximation.
5. Predict \(K_{j+1}\) by Euler, or by AB2 after the first point.
6. At \(\lambda_{j+1}\), optionally apply one frozen-linear correction
   \[
   \Delta K=-P_q(\eta_{j+1})H_\theta(K,\lambda_{j+1}).
   \]
7. At the endpoint, compute the original target residual and
   \[
   E_{\rm out}
   =
   \|(b^T\otimes I)J_R^{-1}\mathcal R(K)\|_{\rm WRMS}.
   \]
8. Accept only if `E_out <= defect_budget_fraction` and the combined embedded plus algebraic
   budget passes. Otherwise call the protected sequential solver on the same step context.

## Fairness and accounting

Every method uses the same Rust executable, coefficients, explicit Jacobians, `faer` LU backend,
state, step size, tolerances, and exact solution. Record separately:

- RHS calls, batch calls, and stage evaluations;
- JVP/Jacobian constructions;
- common-`W` solve batches and vectors;
- path rounds and correction rounds;
- certificate factorizations and solves;
- fallback count;
- RHS, `W` solve, path, certificate, and total time.

Iteration count alone is not a ranking metric.

## Acceptance

Software:

- all pre-existing tests remain green;
- invalid `theta`, `q`, schedules, and non-finite states fail explicitly;
- no failed or rejected path commits external state;
- CLI result schema is stable and parseable.

Scientific:

- `q=7` reproduces affine and scalar-linear exact endpoints;
- accepted nonlinear steps have zero false acceptance against the exact dense output certificate;
- fallback step agrees with protected sequential RODAS5P;
- dimensions and signs match the block formulation.

Numerical:

- canonical output is deterministic apart from timing fields;
- the screen records, rather than suppresses, divergence and non-finite paths;
- fixed-step global-order and stiff-decay gates are designed but promotion may remain HOLD if the
  first screen does not support them.

## Promotion boundary

Cycle 1 may promote only the experiment kernel, certificate, ledger, and deterministic screen.
A homotopy fast path is promoted only after later holdout evidence establishes fifth order,
stiff decay, zero false accepts, and median wall speedup at least `1.15x`.
