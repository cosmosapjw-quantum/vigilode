# G2 PHYS–MATH–CODE AUDIT

## Equation-to-code map

| Mathematical object | Code |
|---|---|
| `exp(A)` projected oracle | `rodas5p_core::matrix_exp_pade13` |
| `φ_k(τA)v` dense oracle | `rodas5p_core::dense_phi_action` |
| matrix-free `φ_k(τJ)v` | `rodas5p_integrators::krylov_phi_action` |
| nonlinear remainder `F(y+u)-F(y)-Ju` | `nonlinear_remainder` |
| EXPRB2 | `exprb2_step` |
| EXPRB43 | `exprb43_step` |
| pexprb54s4 | `pexprb54s4_step` |
| exact coefficient record | `pexprb54s4_tableau`, `ExponentialCoefficientAuthority` |
| dependency-level scheduler | `ParallelExecution::map_ordered` |
| work accounting | `WorkCounters::phi_*`, JVP, orthogonalization counters |

## Code-path reality

- All primary exponential step functions call `linearize_matrix_free`; an explicit Jacobian callback cannot be used accidentally.
- Dense matrix exponential is restricted to the reduced Hessenberg matrix or test oracle.
- pexprb54s4 level one, level two, and endpoint tasks are actual local-Rayon jobs.
- An atomic test verifies concurrent JVP execution.
- Failed/unconverged φ actions return typed errors; they are not silently accepted.

## P0–P3 risk ledger

### P0

None found inside the bounded G2 foundation after the level-one concurrency correction.

### P1

1. `krylov_phi_action` uses nested-dimension action difference, not a KIOPS residual/restart certificate.
2. `phi_combination` launches independent Arnoldi processes for every term. This multiplies JVP and orthogonalization work.
3. Adaptive timestep rejection/rollback is not connected to the exponential methods.
4. Nonautonomous and mass-matrix requests fail closed rather than being implemented.

### P2

1. Full modified Gram–Schmidt with reorthogonalization may dominate large problems.
2. Current allocations are not yet workspace-reused.
3. Endpoint main/embedded combinations duplicate compatible φ work.
4. No fresh finite-difference JVP quality gate exists for black-box RHS users.

### P3

- Naming and report polish;
- SIMD/GPU/multirate extensions;
- rational/Leja alternative engines.

## Genuinely fixed in this cycle

- G0 Cargo/build lock was restored; no new runtime dependency remained.
- Published 3-level dependency graph now matches actual code order.
- BDF/Radau hashes stayed byte-identical.
- Physical-client tokens remain absent from generic production Rust.

## Final code verdict

Correctness/reference implementation is credible and reproducible. Performance architecture is intentionally unfinished; the next code node must fuse φ combinations and add adaptive error control before any wall-time claim.
