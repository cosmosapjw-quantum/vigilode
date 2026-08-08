# RODAS5P Rust Parity Port — Design

## Goal

Rebuild the previous RODAS5P research code in a layered Rust workspace so that direct,
GMRES, LGMRES, GCRO-DR, sequential RODAS5P, SABR5P, and Fair A/B instrumentation share
one compiler, one executable family, one vector-kernel layer, and one dependency lock.

## Scope

Included: coefficients, norms, dense/matrix-free operators, mass matrices, direct solves,
Jacobi preconditioning, GMRES, LGMRES, fixed-left GCRO-DR, protected sequential RODAS5P,
adaptive integration, structured block systems, SABR5P with certified fallback, deterministic
Fair A/B traces, state lifetimes, accounting, CLI, validation, and reproducible delivery.

Excluded: all homotopy or continuation algorithms, flexible preconditioning, GPU/MPI,
singular-mass DAE claims, and complete BDF/Radau ranking.

## Architecture

- `rodas5p-core`: immutable coefficients, numerical primitives, operators, counters, errors.
- `rodas5p-krylov`: common kernels and all iterative linear solvers.
- `rodas5p-integrators`: sequential/adaptive RODAS5P, block system, SABR5P.
- `rodas5p-fair-ab`: traces, policies, adapters, accounting, benchmark analysis.
- `rodas5p-cli`: validation, trace generation, and strict benchmarks.

All solver arms use the same row-major `DenseMatrix`, the same Rust vector kernels, and the
same `faer` factorization/eigendecomposition dependency. Algorithm-specific operations are
recorded rather than hidden.

## Scientific boundary

The official-decimal RODAS5P snapshot and ROW transform are immutable. A successful linear
solve is certified by an external unpreconditioned residual. Failed/rejected attempts cannot
commit recycle state. Homotopy symbols and code are prohibited in this cycle.
