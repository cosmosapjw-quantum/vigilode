# Phase 7 — Scientific validation

| Check | Status | Evidence |
|---|---|---|
| dimensions/units | PASS | theta/lambda/q/WRMS dimensionless; h and state increments retain ODE units |
| sign/normalization | PASS | partial operator identity and affine target-Jacobian tests |
| analytic affine limit | PASS | q=7 endpoint recovers protected affine stages |
| noncommuting mass matrix | PASS | manufactured exact-RHS and commutator test |
| original output certificate | PASS WITH SCOPE | 0 false accepts in 8,208 one-step rows |
| global fifth order: sequential | PASS | observed 5.067 and 4.938 before roundoff |
| global fifth order: q7 oracle | PASS | observed 5.067 and 4.936 before roundoff |
| global fifth order: theta1 q2 c1 | PASS BUT EXPENSIVE | observed 4.963 and 4.847 |
| global fifth order: low-depth c0 | FAIL | q0/1/2 do not robustly retain order 5 |
| stiff-decay low-depth | FAIL / HOLD | strong stiffness acceptance and accuracy collapse |
| DAE constraint | NOT_TESTED | outside cycle-1 validity range |
| false acceptance | PASS | zero in canonical grid |
| equal-work SABR advantage | FAIL | no row dominates SABR even in RHS and JVP scalar-work metrics |

A FAIL in the low-depth order and economics gates prevents method promotion. The correctness
oracles remain promotable as research infrastructure.
