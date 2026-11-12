# Test/benchmark contract

A solver comparison is admissible only with a common reference identity and uncertainty, common output grid and common physical output metric. Compare only overlapping achieved global-error bands. Equal user rtol, equal step count, equal local tolerance or passing a fixed budget is not matched achieved error.

Pre-register calibration/holdout cases. Use at least 2 warmups and 7 randomized paired runs per method/case. Record median and paired ratio with bootstrap interval and per-case table. Retain every failure/rejection/fallback.

Charge and report RHS, JVP, explicit Jacobian, FD RHS, AD, operator apply, factorization, solve, Krylov matvec/iteration/reduction, block-Krylov, preconditioner setup/apply, nonlinear iteration, homotopy round, startup, dense output, events, rejected steps, fallback, allocation bytes, peak RSS, setup/wall/CPU/communication time and retained-state bytes. Do not equate stored-state bytes with peak RSS.

Spatial refinement, temporal refinement and remap error must be separated. Sphere/surface norms use declared quadrature/volume weights; Cartesian Euclidean WRMS is not assumed.

Official G0 numeric targets are frozen in SOURCE_LOCK.json. Critical-depth definition for inner Krylov/reduction depth needs owner decision before scoring new methods.
