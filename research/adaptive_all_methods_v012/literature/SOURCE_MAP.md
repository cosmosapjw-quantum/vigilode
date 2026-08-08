# Source Map — Adaptive Step Control

## Primary / official anchors

1. SUNDIALS CVODE mathematics: variable-step, variable-order BDF 1–5; error-test failure response; step and order selection.
   - https://sundials.readthedocs.io/en/v6.6.1/cvode/Mathematics_link.html
2. LLNL CVODE project page: variable-step/variable-order multistep architecture and nonlinear solve choices.
   - https://computing.llnl.gov/projects/sundials/cvode
3. Hairer–Wanner RADAU5 source: fifth-order Radau IIA with step-size control, continuous output, Gustafsson predictive controller, Jacobian/LU reuse thresholds.
   - https://sources.debian.org/src/r-cran-desolve/1.14-1/src/radau5.f
4. RODAS5P construction paper: eight-stage Rosenbrock–Wanner 5(4) method and embedded error structure.
   - https://doi.org/10.1007/s10543-023-00967-x
5. Adaptive SDC paper: increment-based and dense-output-based estimators; adaptive \(\Delta t\) and iteration count.
   - https://doi.org/10.1007/s11075-024-01964-z
6. Parallel two-step W/Peer methods: stability and convergence for general step-size sequences.
   - https://doi.org/10.1137/S0036142902411057
7. Variable-step IMEX Peer methods preserving superconvergence under step changes.
   - https://arxiv.org/abs/1902.01161
8. Rosenbrock–Krylov method: time discretization and Krylov approximation are a single method; adaptive step control must not be reduced to linear-solver tolerance.
   - https://doi.org/10.1137/130923336
9. BOROK: adaptive subspace-size selection and basis extension.
   - https://arxiv.org/abs/1908.10531
10. Cost-aware adaptive control for exponential integrators where per-step iterative cost depends on step size.
    - https://doi.org/10.1016/j.camwa.2022.07.011
11. Adaptive linear multistep error models must depend on recent step ratios rather than a constant-step asymptotic model.
    - https://doi.org/10.1007/s11075-020-00900-1
12. Variable-step BDF2 stability on arbitrary grids under a modern stability definition; older zero-stability restrictions remain useful conservative implementation guards.
    - https://doi.org/10.1137/21M1462398

## Sources added during closeout

13. Current SUNDIALS CVODES mathematics, including the accepted/rejected local-error test and the `q+1` step-size exponent.
    - https://sundials.readthedocs.io/en/latest/cvodes/Mathematics_link.html
14. Direct RADAU5 Fortran source, including bounded step factors, error estimation, and the Gustafsson predictive controller.
    - https://sources.debian.org/src/r-cran-desolve/1.14-1/src/radau5.f
15. Variable-step BDF2/BDF3 stability analysis: BDF2 stability on arbitrary time grids under the paper's stability framework; BDF3 requires a ratio restriction for almost all adjacent steps.
    - https://doi.org/10.1137/21M1462398
16. Adaptive SDC version of record and 2025 correction: local-error estimators, `dt` adaptivity, and joint `dt`/iteration adaptivity.
    - https://doi.org/10.1007/s11075-024-01964-z
    - https://doi.org/10.1007/s11075-025-02191-w
