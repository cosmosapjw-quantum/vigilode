# Phase 5 — Isolated implementation

Implemented on branch `homotopy-numerical-experiments-v0.5` from frozen base `ce1cc510...`.

Delivered coherent patches:

- read-only structured nonlinear decomposition;
- exact nonlinear target Jacobian and output certificate;
- fixed partial-coupling path with Euler/AB2 and q-truncated inverse;
- transactional protected sequential fallback;
- deterministic canonical experiment CLI;
- noncommuting manufactured mass problem;
- global convergence-order screen;
- stdlib-only reproducible analysis.

No new production dependency, adaptive schedule, Anderson, Chebyshev/Padé, BDF/Radau, or
GCRO-DR tuning was added.
