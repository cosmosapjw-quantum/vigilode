# Next Cycle — Native Adaptive Production Families

The v0.12 reference layer is frozen before production optimization.

1. Add a controller-stress lane whose horizon is independent of requested-output spacing, or add verified dense output, so high-order methods are not output-clipping limited.
2. Implement variable-order variable-step BDF1–5 with Nordsieck history, LTE/order selection, rejection rollback, Jacobian/preconditioner lagging, and order-3–5 stability-limit detection.
3. Implement a native Radau IIA3/RADAU5 error estimator and predictive/Gustafsson-style controller; compare full and simplified Newton.
4. Implement adaptive parallel SDC with independent step and sweep/iteration control.
5. Add Peer/W startup and step-ratio adaptivity.
6. Add ROK/BOROK joint step-size and Krylov-subspace adaptivity.
7. Add exponential/Leja joint step-size, scaling, and interpolation-degree adaptivity.
8. Re-run RODAS5P, SABR, Homotopy and all new families under the v0.11 global-error/total-cost authority with randomized repeated timings.

Do not tune cross-family performance before the native estimator/controller and output-bias gates are closed.
