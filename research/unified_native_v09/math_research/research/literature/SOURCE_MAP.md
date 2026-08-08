# SOURCE MAP

- **S-001 — SUNDIALS documentation.** https://sundials.readthedocs.io/en/latest/  
  Official current documentation: CVODE uses Adams/BDF, IDA uses BDF for DAEs, common nonlinear/linear solver infrastructure.
- **S-002 — PETSc TSBDF.** https://petsc.org/release/manualpages/TS/TSBDF/  
  Official implementation documentation: implicit BDF for stiff ODE/DAE, selectable order and extrapolated nonlinear initial guess.
- **S-003 — PETSc TS framework.** https://petsc.org/release/manualpages/TS/  
  Official framework documentation: implicit methods use a common nonlinear solver infrastructure; ODE/DAE methods coexist in one TS layer.
- **S-004 — Rosenbrock–Krylov methods.** https://doi.org/10.1137/130923336  
  Primary paper: time discretization and Krylov approximation are co-designed; Krylov dimension is tied to temporal order.
- **S-005 — Parallel SDC 2025.** https://doi.org/10.1137/24M1649800  
  Primary paper: optimized diagonal/nonstationary sweepers, including nilpotent error-propagation products; order increases per sweep.
- **S-006 — Parallel two-step W/Peer.** https://doi.org/10.1137/S0036142902411057  
  Primary paper: s independent linear stage equations, order s-1 and variable-step stability.
- **S-007 — Exponential Rosenbrock.** https://doi.org/10.1137/080717717  
  Primary paper: exponential Rosenbrock methods use matrix functions and avoid standard linear solves; separate capability domain.
- **S-008 — RADAU5/PETSc wrapper.** https://petsc.org/main/manualpages/TS/TSRADAU5/  
  Official wrapper documentation: external RADAU5 has its own nonlinear solver, adaptivity and dense factorization; useful control but not same backend.
- **S-009 — PETSc TSIRK.** https://petsc.org/main/manualpages/TS/TSIRK/  
  Official IRK implementation uses Kronecker structure and supports ODE/DAE solving.
- **S-010 — Krawczyk-like algorithms.** https://doi.org/10.1137/0722048  
  Primary interval method paper for existence/uniqueness verification of nonlinear system roots.
- **S-011 — Krawczyk–Neumaier improved interval Newton.** https://doi.org/10.1016/0022-247X(86)90303-3  
  Primary interval-operator paper: existence and uniqueness of fixed intervals/solutions.
- **S-012 — Yamamoto sharp Newton error bounds.** https://doi.org/10.1007/BF01389624  
  Primary paper on computable sharp a posteriori Newton error bounds under Kantorovich assumptions.
- **S-013 — Akrivis–Makridakis–Nochetto RK a posteriori.** https://doi.org/10.1007/s00211-009-0254-2  
  Primary paper: optimal-order a posteriori estimates for RK collocation and cG methods for stiff nonlinear ODEs/PDEs.
- **S-014 — Patera–Rønquist output bounds.** https://doi.org/10.1142/S0218202501001057  
  Primary output-functional error bound framework for incomplete iteration in linear algebraic systems.
- **S-015 — Variable-step BDF2/BDF3 stability.** https://doi.org/10.1137/21M1462398  
  Primary modern analysis: BDF2 stable on arbitrary time grids; variable-step BDF3 needs restrictions.
- **S-016 — Radau IIA index-2 convergence.** https://doi.org/10.1137/S0036142995296539  
  Primary paper illustrating that DAE order behavior needs separate treatment even for Radau IIA.
- **S-017 — Large-scale DAE Krylov.** https://doi.org/10.1137/0915088  
  Primary paper: DASSL-type integration with inexact Newton and preconditioned GMRES for large DAEs.
