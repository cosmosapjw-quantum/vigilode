# Source map — multiple-RHS Krylov and RODAS telemetry

## Block GMRES rank loss / deflation

- Q. Wu, L. Bao, Y. Lin, “Residual-Based Simpler Block GMRES for Nonsymmetric Linear Systems with Multiple Right-Hand Sides,” *Advances in Mathematical Physics* (2018), DOI `10.1155/2018/1369707`.
  - Relevance: detects linear dependence of residuals and near-rank deficiency in block Arnoldi; motivates rank-revealing/deflation safeguards.
- “A flexible and adaptive simpler block GMRES with deflated restarting for linear systems with multiple right-hand sides,” *Journal of Computational and Applied Mathematics* 282 (2015), DOI `10.1016/j.cam.2014.12.040`.
  - Relevance: adaptive simpler block GMRES, flexible preconditioning, deflated restart.
- D.-L. Sun et al., “A block GMRES method with deflated restarting for solving linear systems with multiple shifts and multiple right-hand sides,” *Numerical Linear Algebra with Applications* 25 (2018), DOI `10.1002/nla.2148`.
  - Relevance: BGMRES-DR and seed selection for nearly linearly dependent RHS.

## Rosenbrock–Krylov boundary

- P. Tranquilli and A. Sandu, “Rosenbrock–Krylov Methods for Large Systems of Differential Equations,” *SIAM Journal on Scientific Computing* 36 (2014), DOI `10.1137/130923336`.
  - Relevance: time discretization and Krylov approximation are jointly designed; this is a separate integrator family, not a renamed shared common-W backend.

## Project-use rule

The literature supports deflation/restart as a possible remedy if actual dispatched block solves exhibit rank-loss or restart-related stagnation. It does not justify implementing BGMRES-DR before the shadow dispatcher demonstrates that this failure mode is material on real SABR/homotopy stage traces.
